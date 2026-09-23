---
title: Time
description: 'Separate wall-clock timestamps from elapsed-time measurement.'
section: Standard library
order: 23
---

## Responsibility

The specified time module covers timestamps, durations, monotonic measurement, and sleeping. Wall-clock time can change independently of elapsed time; use a monotonic source for measuring duration.

## Duration

The design calls for explicit duration units and checked conversions when a representation could overflow. Proposed constructors must follow the language features actually supported by the toolchain.

## Measure elapsed time

```vut
import time

fn main():
  start = time.instant()
  time.sleep(time.milliseconds(5))
  duration = start.elapsed()
  out(duration.as_milliseconds())
```

`sleep(Duration)` is synchronous in the current public module. It is not an awaitable timer; do not write `await time.sleep(...)`. Actual elapsed time can exceed the requested duration.

## Constructors and methods

Duration constructors accept `i64`: `nanoseconds`, `microseconds`, `milliseconds`, `seconds`, `minutes`, and `hours`. Internally spans use signed nanoseconds; checked scaling and arithmetic reject overflow.

Duration methods include `as_nanoseconds`, `as_microseconds`, `as_milliseconds`, `as_seconds`, `add`, and `subtract`. Coarser-unit conversions use integer division.

`now()` returns a wall-clock `Timestamp`; `from_unix_seconds` and `from_unix_milliseconds` construct one. Timestamp methods are `unix_seconds`, `unix_milliseconds`, `add`, `subtract`, and `duration_since`.

`instant()` returns a monotonic `Instant`. Use `start.elapsed()` or `elapsed(start, finish)` for measurement, not subtraction of wall-clock timestamps. An instant is not a persistent calendar timestamp.
