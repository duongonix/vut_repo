---
title: Vutcon
description: 'Schedule lightweight concurrent work with typed handles.'
section: Advanced
order: 2
---

## Spawn and await

A Vutcon is a lightweight execution unit, not an OS thread. Create it inside an async function using `vut` and a parameterless, non-capturing callable.

```vut
fn calculate() -> int:
  21

async fn main():
  job: vutcon(int) = vut(() => calculate() * 2)
  answer = await job
  out("Answer: $answer")
```

## Ownership

`await job` consumes the handle and moves out its result. Awaiting the same handle again is invalid. Dropping an unawaited handle safely destroys the task, cancelling it if suspended; it does not guarantee that the task never started.

## Scheduling limits

The current specified scheduler is cooperative and single-threaded. Spawning schedules immediately; waiting does not start the task. Tasks make progress when the running task suspends. There is no parallel-execution guarantee, worker pool, channel, or mutex API in this phase.

## Related concepts

An `async fn` may suspend; `await` waits; `vut` schedules work; `vutcon(T)` is its typed handle. These are distinct responsibilities. See [Concurrency](/docs/advanced/concurrency/).
