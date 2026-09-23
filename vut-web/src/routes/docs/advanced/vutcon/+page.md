---
title: Vutcon
description: 'Schedule lightweight concurrent work with typed handles.'
section: Advanced
order: 2
---

## Spawn and await

A Vutcon is a lightweight execution unit, not an OS thread. Create it inside an async function using `vut` and a parameterless lambda. Captures are supported and follow ordinary closure ownership rules.

```vut
fn calculate() -> int:
  21

async fn main():
  job: vutcon[int] = vut(() => calculate() * 2)
  answer = await job
  out("Answer: $answer")
```

## Ownership

`await job` consumes the handle and moves out its result. Awaiting the same handle again is invalid. Dropping an unawaited handle safely destroys the task, cancelling it if suspended; it does not guarantee that the task never started.

## Scheduling limits

The current scheduler uses multiple worker threads with work stealing. The caller is worker zero. `VUT_MAXPROCS` selects a positive worker count; otherwise the runtime uses available parallelism. Scheduling is cooperative, not preemptive: a long-running task that never suspends can occupy a worker.

Spawning schedules immediately; awaiting is not what starts the task. Independent tasks can run in parallel, but their execution and output order are not deterministic. Native blocking work has a separate pool. Use [channels](/docs/advanced/channels/) for typed communication.

## Captured work

```vut
async fn main():
  name = "Vut"
  job = vut(fn():
    out("Hello, $name")
  )
  await job
```

An `async fn():` lambda may itself await asynchronous work. The current `vut` surface requires a lambda literal; do not substitute an arbitrary stored callable. Its environment stays alive until completion or cleanup.

## Related concepts

An `async fn` may suspend; `await` waits; `vut` schedules work; `vutcon[T]` is its typed handle. These are distinct responsibilities. See [Concurrency](/docs/advanced/concurrency/).
