# Typed Channels (`channel[T]`) — runtime done, compiler integration pending

## Status

**Core complete.** Runtime, type system, MIR lowering, and codegen are in place.
Direct use in an `async fn` works end to end for scalar, managed, and collection
elements, including buffered FIFO, close, and close-then-`null`. Cross-task
suspension works: a producer Vutcon that captures the channel can send or close
while the consumer awaits, and the scheduler wakes the consumer. This required
fixing `vut(...)` closure capture (see "Closure capture" below).

## Audit summary

- **Vutcon** is a poll task (`AsyncHandle { op, poll_fn, drop_fn, Mutex<State>,
  TaskStorage }`); `vut(...)` eagerly schedules it on the global scheduler.
- **Scheduler** (`crates/vut-runtime/src/scheduler/`): M:N worker pool,
  per-worker local queues + injector + work stealing, `schedule`/`wake`/
  `pop_or_park`, blocking-pool offload, deferred reclamation.
- **Suspension** reuses the native async-op model: a task awaits a child op via
  `vut_rt_async_await_child_v1`; a `Pending` poll suspends the task (frame state
  machine), and `vut_rt_async_signal_v1` from any thread wakes it.
- **Ownership**: managed values are reference-counted with atomic counters and
  copy-on-write; moves/retains/drops are compiler-managed.
- **Type checker**: parameterized builtin types (`list`, `map`, `result`,
  `vutcon`) are resolved in `checker/analyze.rs`; builtin methods are resolved by
  `(receiver type, name)` in `checker/builtins.rs`.
- **Async context**: `await`/`vut(...)` require `context.in_async`; async bodies
  are lowered to poll state machines by the MIR `future` pass.

## Runtime representation

```text
ManagedChannel {
  references: AtomicUsize,
  capacity: usize,            // 0 = unbuffered/rendezvous
  element_size: usize,
  element_release: Option<fn(*mut u8)>,
  inner: Mutex<Inner { buffer: VecDeque<Vec<u8>>,
                       senders: VecDeque<*mut SendOp>,
                       receivers: VecDeque<*mut RecvOp>,
                       closed: bool }>,
}
```

`send`/`recv` are native poll tasks created with `new_handle`. Each op retains
the channel, so the channel cannot be dropped while a waiter exists. Values are
owned byte buffers; ownership transfers sender → channel/buffer → receiver, and
managed elements are released exactly once on op drop or channel drop.

## Scheduler-aware suspension

```text
ch.recv() with no value
  → create RecvOp, register in channel.receivers, poll returns Pending
  → task awaits the op → task suspended, worker freed

ch.send(value)
  → if a receiver waits: deliver, vut_rt_async_signal_v1(receiver.handle)
  → else if buffer has room: enqueue, Ready
  → else register SendOp, Pending

close
  → drain receivers/senders and signal each handle
```

No busy loop, no sleep, no per-channel thread, no worker blocking. Verified by
`recv_suspends_task_and_a_sender_thread_wakes_it`, which drives the real
executor and wakes the task from another OS thread.

## Semantics

- `capacity == 0` → unbuffered rendezvous; `capacity > 0` → FIFO buffer.
- `recv` → `T?`: present while open with data; absent when closed and drained.
- `send` on a closed channel traps; double close traps.
- `close` drains buffered values before reporting absence and wakes all waiters.

## Runtime ABI

```text
vut_rt_channel_new_v1(capacity, elem_size, elem_align, retain, release)
vut_rt_channel_retain_v1 / vut_rt_channel_release_v1
vut_rt_channel_close_v1
vut_rt_channel_send_v1(channel, value) -> *mut AsyncHandle   (result: unit)
vut_rt_channel_recv_v1(channel, presence: *mut u8) -> *mut AsyncHandle
vut_rt_channel_live_count_v1
```

`recv`'s await result is the raw `T`; presence is a separate byte the compiler
combines into `T?`.

## Remaining compiler work

Implemented:

1. `Type::Channel(TypeId)`, `channel[T]` resolution, layout/ownership
   (`OwnershipKind::RcChannel`), display/LSP.
2. Checker: `channel[T](capacity:)` constructor (default 0, non-negative `int`);
   `send(value: T) -> unit`, `recv() -> T?`, `close() -> unit`; channel ops
   outside `async fn` / `vut(...)` are rejected (`E6011`). A function/lambda
   whose body contains `send`/`recv` is marked suspendable (`current_function`
   → `async_symbols`).
3. MIR: `ChannelNew/Send/Recv/Close` runtime calls, `AwaitChannelRecv` marker,
   `PollChannelRecv`, and `OptionalFromValue`. The async pass rewrites the recv
   marker into a poll site that keeps the value address and presence in the
   continuation (no presence across suspension needed).
4. Codegen: `layouts.channels`, `ChannelSend` (element pointer), `ChannelNew`,
   `ChannelRecv`, `ChannelClose`, `PollChannelRecv`, `OptionalFromValue`.
5. Runtime `recv` writes the element at offset 0 and the presence byte at
   `element_size`, so the runtime never constructs `T?`.

### Closure capture (fixed)

A `vut(...)` callable may now capture enclosing bindings. A capturing callable
is a tagged closure value whose environment lives inside the reference-counted
closure block; the spawned task takes ownership of that block so the captures
outlive the parent scope.

- **Sync capturing callable**: `Instruction::Spawn` passes the tagged closure as
  the task op; the poll thunk derives the environment (`base + 32`) and passes it
  as the body's hidden leading parameter; the task drop thunk calls
  `CLOSURE_RELEASE(op)`.
- **Async capturing callable**: the environment (`base + 32`) is stored into the
  frame's parameter slot 0 at spawn; the frame-based body reads it as usual; the
  frame drop thunk recovers the tagged closure (`(env - 32) | TAG`) and releases
  it after the in-flight child, before freeing the frame.
- **Non-capturing callable**: unchanged fast path (`op = 0` for sync; a bare
  frame for async).

Ownership is a transfer, not a retain: the callable is always a lambda literal
(the checker rejects other callables via `E6020`), so the environment is a fresh
reference the task owns and releases exactly once, on completion or cancellation.

A related fix: an implicitly-suspendable function/lambda (marked in
`async_symbols` because its body contains `send`/`recv`) now propagates to MIR
`is_async`, so it becomes a real poll state machine. Previously only explicitly
`async` bodies were converted, and a capturing producer's `send` was driven to
completion inline instead of suspending.

E2E coverage: `channel_e2e` (buffered FIFO, `str`, `list[int]`, close→`null`,
recv→producer-send→wake, recv→close→`null`, `E6011`), `vutcon_capture_e2e`
(scalar/`str`/`list`/`async` captures, environment outliving the parent scope,
environment release after completion, and dropping a pending capturing task).

### Not yet done

- Channel iteration `for value in ch:` (recv loop with a null check).
- Transitive implicit async (a sync helper containing a channel op called from an
  async function) — currently only direct containment is marked.
- `crates/vut-codegen/src/cranelift/futures.rs` is 534 lines; the thunk generators
  are cohesive but should be split into a `futures/` module tree if it grows.


## Tests already in place (runtime)

`crates/vut-runtime/tests/channel_contract.rs`: buffered FIFO; unbuffered
rendezvous; buffered send suspends when full and wakes on recv; close wakes a
waiting receiver with absence; closed buffered channel drains then reports
absence; dropping a channel releases buffered values exactly once;
scheduler-aware recv suspension woken by a sender thread.

## Not in scope

select, mutex/rwlock, semaphore, actor, broadcast, async stream, direction
types, distributed channels, Go-API parity.
