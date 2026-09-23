---
title: Concurrency
description: 'Separate suspension, scheduling, and parallel execution.'
section: Advanced
order: 8
---

## Async functions

An async function prefixes `fn` with `async`. Its declared return type describes the logical result.

```vut
async fn answer() -> int:
  42

async fn main():
  value = await answer()
  out("$value")
```

## Await

`await` is a prefix expression and is valid inside an async body. It is not `.await` syntax or a special `await(...)` function. Async main is driven by the runtime.

## Spawned work

[Vutcon](/docs/advanced/vutcon/) supplies scheduled lightweight execution units with typed handles. The current runtime schedules cooperatively across multiple worker threads. Calling an async function and spawning a task are different operations: use `vut` when work should be independently scheduled. [Channels](/docs/advanced/channels/) transfer values between tasks.

## Error handling

Awaiting a result-returning operation yields its result. A following `?` then unwraps or propagates the error using the ordinary result rules.
