# Vut Standard Library — `math`

## 1. Purpose

`math` provides scalar numeric utilities: mathematical constants, `float`
transcendental functions, integer algorithms, and a small pseudo-random number
generator.

It is an official stdlib module (`import math`), not a VPM package.

`math` is a **scalar** module. It does not allocate managed aggregates and does
not define collection algorithms. `random` lives under `math` as
`math.rng(...)` / `math.random*` (see §6); there is no separate `random` module
in the initial surface.

---

## 2. Scope

In scope:

```text
constants (float)
float: basic, rounding, powers/roots, exp/log, trigonometry, hyperbolic,
       classification, sign/utility
int:   gcd, lcm, is_power_of_two
random: rng, random, random_int, random_float, random_bool
```

Out of scope (not part of this module):

```text
f32 math (deferred until f32 methods are specified)
complex numbers
arbitrary-precision integers
statistics / distributions beyond uniform
cryptographically secure randomness
matrix / linear algebra
```

Integer scalar helpers (`abs`, `min`, `max`, `clamp`, `pow`) are the existing
numeric **methods** (`x.abs()`, `a.min(b)`, `x.clamp(low, high)`, `x.pow(n)`).
They are not duplicated as `math.*` functions because Vut has no overloading: a
single `math.abs` cannot accept both `int` and `float`.

---

## 3. Module Layout

```text
std/math/
├── mod.vut        public API (constants + wrappers)
├── _native.vut    extern "C" declarations for vut_rt_math_* primitives
├── _float.vut     pure-Vut float helpers (sign, fract, radians, degrees, powi)
└── _integer.vut   pure-Vut integer algorithms (gcd, lcm, is_power_of_two)
```

Native primitives are implemented in
`vut-stdlib/native/vut-runtime/src/math.rs`.

Public functions are thin wrappers. When the operation already exists as a
numeric method (`sqrt`, `pow`, `abs`, `floor`, ...), the wrapper delegates to the
method rather than reimplementing it.

---

## 4. Constants

All constants are IEEE-754 binary64 (`float`) literals:

```text
PI        TAU        E          SQRT_2
FRAC_1_PI FRAC_2_PI  FRAC_PI_2  FRAC_PI_3  FRAC_PI_4
LN_2      LN_10      LOG2_E     LOG10_E
```

Constants are module-level `NAME = value` declarations. They are immutable
scalars with no addressable storage; the compiler inlines the initializer at each
reference (including cross-module `math.NAME`). Reassigning a constant is a
compile error.

---

## 5. Float API

`float` is IEEE-754 binary64. Every function below is defined on `f64`
semantics.

Basic and classification:

```text
abs(value)          min(left, right)    max(left, right)
clamp(value, low, high)                  sign(value)
floor(value)        ceil(value)          round(value)     trunc(value)
fract(value)
is_nan(value)       is_finite(value)     is_inf(value)
```

Powers, roots and utilities:

```text
sqrt(value)                    cbrt(value)
pow(base, exponent)            powi(base, exponent: int)
copysign(magnitude, sign)      hypot(left, right)
fma(a, b, c)
```

Exponential, logarithm and trigonometry (radians):

```text
exp(x)   exp2(x)   log(x)   log2(x)   log10(x)
sin(x)   cos(x)    tan(x)   asin(x)   acos(x)   atan(x)   atan2(y, x)
sinh(x)  cosh(x)   tanh(x)
radians(degrees)   degrees(radians)
```

`round` rounds half away from zero (Rust `f64::round`), not half to even.
`copysign`, `fma`, and `sqrt`/`abs`/`floor`/`ceil`/`trunc` delegate to the
corresponding numeric methods, which the compiler lowers to native instructions.

`min`/`max` ignore NaN (Rust `f64::min`/`f64::max`). `sign(NaN)` is NaN and
`sign(±0.0)` is `0.0`. `fract` preserves the sign of the input.

---

## 6. Randomness

Randomness is **pseudo-random and not cryptographically secure**. It must never
be used for keys, tokens, or security decisions.

Design:

```text
engine        xoshiro256** (state held in the native runtime)
seeding       splitmix64 from a 64-bit seed
global source thread-local, seeded from OS entropy on first use
random_int    uniform over an inclusive range via rejection sampling
random_float  53-bit mantissa: (next_u64() >> 11) * 2^-53
```

`math.rng(seed: int) -> Rng` returns an independent, explicitly seeded generator.
`Rng` is a `resource` with methods `random`, `random_int`, `random_float`,
`random_bool`. The global functions (`math.random`, `math.random_int`,
`math.random_float`, `math.random_bool`) use the thread-local generator.

`random_int(min, max)` is inclusive of both bounds; `random_float(min, max)` is
half-open `[min, max)`.

Sequences are reproducible for a given seed across platforms.

---

## 7. Native ABI

Native symbols follow `vut_rt_math_<operation>_v1` and take/return `f64` (or
`i64` for integer exponents). `float` is not FFI-safe, so `_native.vut`
declarations use `f64`; wrappers convert with `.to_f64()` / `.to_float()` (both
identity casts at codegen).

The bridge delegates to the platform libm through Rust `f64` methods. No libm
symbol is referenced directly by the stdlib; only `vut_rt_math_*` symbols cross
the ABI.

The compiler may lower an operation to a native instruction instead of the
runtime call when the semantics are identical (for example `sqrt` → Cranelift
`sqrt`). It must not lower an operation to an instruction whose semantics differ
(for example `round` must not use round-half-to-even `nearest`).

---

## 8. Semantics

```text
float       IEEE-754 binary64; domain errors yield NaN, overflow yields ±Inf,
            underflow yields 0
int         signed 64-bit; arithmetic wraps (two's complement) on overflow
usize       target pointer width
```

`int` arithmetic does not trap on overflow; `pow` saturates and `abs` wraps, as
specified for the numeric methods.

`gcd`/`lcm` use the absolute values of their arguments; `lcm(0, x) == 0`.
`is_power_of_two` is false for `<= 0`.

No operation in `math` traps on a value-domain error: invalid float inputs
produce IEEE results (NaN/±Inf/0). Panics/traps are reserved for the language's
own trap mechanisms (for example explicit numeric conversions).

---

## 9. Errors

`math` has no `result`-returning API. All operations are total over their input
domain under IEEE-754 and two's-complement semantics.

---

## 10. Testing

`math` requires:

```text
exact-value tests for constants, rounding, sign, and integer algorithms
tolerance tests for transcendentals (abs(error) < epsilon)
classification tests for NaN / ±Inf / -0.0
determinism tests for a fixed RNG seed
```

Tests live as stdlib E2E tests that compile and run real Vut programs against the
native runtime.
