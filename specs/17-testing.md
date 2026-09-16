

# Vut Testing

## 1. Purpose

This document defines testing conventions for:

* Vut projects
* Vut libraries
* compiler development
* standard library development
* VPM packages

Project tests are executed through:

```text
vpm test
```

---

## 2. Testing Philosophy

Testing should be:

* simple
* compiler-integrated
* deterministic
* fast
* suitable for CI
* suitable for library and application projects

Vut should avoid requiring a heavyweight external test framework for basic tests.

---

## 3. Test Directory

Project-level tests live in:

```text
tests/
```

Example:

```text
project/
├── src/
│   ├── main.vut
│   └── math.vut
├── tests/
│   ├── math.vut
│   └── parser.vut
├── vpm.toml
└── vpm.lock
```

---

## 4. Test Files

Test files use the normal:

```text
.vut
```

extension.

Testing should not introduce a second source-language syntax.

---

## 5. Unit Tests

Vut should support tests close to the implementation where appropriate.

The exact test declaration syntax must be standardized before parser implementation.

The intended direction is a minimal dedicated test construct rather than external annotations or macros.

Do not invent Rust-style attributes such as:

```text
#[test]
```

unless explicitly specified later.

---

## 6. Test Declaration Placeholder

The final syntax for declaring an individual Vut test is not yet locked.

Implementation must not guess between constructs such as:

```text
test "name":
test name:
@test
```

before the syntax is formally added to the grammar.

Until then, compiler infrastructure may build test discovery generically without hard-coding unsupported syntax.

---

## 7. `vpm test`

Command:

```text
vpm test
```

Responsibilities:

```text
resolve dependencies
compile project test configuration
discover tests
run tests
collect results
print summary
return appropriate exit status
```

---

## 8. Compiler Reuse

VPM must use the normal Vut compiler pipeline for tests.

Do not create a separate test-only parser/type checker.

Test code follows the same:

* syntax
* type system
* interfaces
* modules
* diagnostics

as production code.

---

## 9. Test Isolation

Individual tests should be isolated sufficiently that one test failure does not prevent unrelated tests from executing unless compilation itself fails.

Compile errors block execution.

Runtime assertion failures should normally affect only the corresponding test.

---

## 10. Assertions

The standard testing API should eventually provide operations such as:

```text
assert
assert_eq
assert_ne
```

Exact surface syntax/API belongs to the standard testing library specification.

Assertions should produce useful source diagnostics.

---

## 11. Assertion Failure

A failed equality assertion should show useful values.

Conceptual:

```text
test failed: vector_add

  --> tests/vector.vut:12:3

expected:
  20

found:
  19
```

Avoid output such as:

```text
assertion failed
```

without context when more information is available.

---

## 12. Test Names

Every test should have a stable human-readable name.

Names should appear in:

```text
test output
filters
failure reports
CI output
```

---

## 13. Test Filtering

The test runner should eventually support filtering.

Conceptual:

```text
vpm test <filter>
```

Exact CLI syntax may be refined.

Filtering should be deterministic and documented.

---

## 14. Test Summary

Example successful summary:

```text
test result: ok. 24 passed; 0 failed
```

Failure:

```text
test result: FAILED. 22 passed; 2 failed
```

The exact formatting may evolve while retaining the same essential information.

---

## 15. Exit Status

All tests pass:

```text
0
```

Any failed test:

```text
non-zero
```

Compilation error:

```text
non-zero
```

Internal test-runner/compiler error:

```text
non-zero
```

---

## 16. Output Capture

The test runner may capture stdout/stderr for individual tests.

Successful tests should not flood normal output with unnecessary logs.

Failed tests may display captured output.

A verbose/no-capture option may be introduced.

---

## 17. Parallel Tests

Independent tests may run in parallel.

Parallel execution must not:

* corrupt output
* mix test diagnostics unpredictably
* make final ordering nondeterministic

Tests requiring shared global state may eventually need explicit serialization controls.

---

## 18. Deterministic Results

Test reports should be deterministic even when execution occurs in parallel.

Sort/report by stable test identity rather than completion timing where practical.

---

## 19. Project Dependencies

Tests have access to the project's normal resolved dependencies.

Test-only dependencies may be introduced in the manifest specification.

Conceptual future section:

```toml
[dev-dependencies]
```

Do not finalize syntax until `specs/vpm/manifest.md` defines it.

---

## 20. Package Testing

A package version should be testable before publication.

Typical:

```text
vpm test
```

from the package project.

Tests are not required for consuming the dependency at application compile time.

---

## 21. Compiler Tests

Compiler development should contain distinct categories.

Recommended:

```text
lexer
parser
resolver
type checker
interface checker
diagnostics
codegen
end-to-end
regression
```

Compiler implementation tests are Rust-side tests and are separate from user-facing Vut project tests.

---

## 22. Valid Compilation Tests

Compiler suites should contain Vut programs expected to compile successfully.

Example categories:

```text
variables
data
functions
interfaces
loops
imports
packages
```

---

## 23. Compile-Fail Tests

Compiler suites must also contain intentionally invalid programs.

Examples:

```text
type mismatch
missing colon
invalid indentation
private import
unknown module
invalid interface
constant reassignment
invalid for condition
```

A compiler is not sufficiently tested if only valid programs are covered.

---

## 24. Diagnostic Golden Tests

Diagnostics should have snapshot/golden tests.

A test should verify:

```text
error code
message
primary span
related spans
expected/found
help
plain-text rendering
```

This prevents accidental degradation of error quality.

---

## 25. Regression Tests

Every fixed compiler bug should receive a regression test whenever practical.

Structure may conceptually contain:

```text
tests/compiler/regressions/
```

Exact repository layout is implementation-specific.

---

## 26. Parser Fuzzing

Lexer/parser should support fuzz testing with:

* arbitrary UTF-8
* malformed indentation
* malformed strings
* malformed numeric literals
* deeply nested expressions
* random tokens

No arbitrary input should crash the compiler.

---

## 27. Runtime Tests

Runtime tests should validate:

* strings
* lists
* allocation
* interface dispatch
* dynamic values
* error paths
* platform behavior

Memory-safety-sensitive runtime components require targeted tests.

---

## 28. Standard Library Tests

Every public standard-library module should include tests for:

* normal behavior
* boundary values
* invalid input
* error handling
* platform-specific behavior

---

## 29. Cross-Platform Testing

Platform-sensitive components should run CI tests on supported platforms.

At minimum as support grows:

```text
Windows
Linux
macOS
```

Platform-specific failures must not be hidden by tests that only run on one OS.

---

## 30. Optimization Equivalence

Where practical, test suites should compare debug/unoptimized and release/optimized behavior.

Optimization must not change program semantics.

---

## 31. Test Performance

Large compiler/stdlib suites should avoid unnecessary process creation.

Where architecture permits, reusable compiler APIs should be called directly from test harnesses.

End-to-end CLI tests remain necessary but should not replace focused unit tests.

---

## 32. Benchmarks

Benchmarks are separate from correctness tests.

Compiler benchmarks may measure:

```text
lex
parse
type-check
incremental rebuild
codegen
```

Runtime benchmarks may measure:

```text
collections
strings
numeric workloads
dispatch
allocation
```

Performance regressions should be tracked where practical.

---

## 33. No Network in Ordinary Tests

Normal deterministic unit tests should avoid real network dependencies.

Provider/network VPM tests should prefer:

* local fixtures
* mock HTTP servers
* recorded fixtures

A smaller integration suite may test actual providers separately.

---

## 34. Temporary File Safety

Tests involving files/packages should use isolated temporary directories.

Tests must not modify the user's:

```text
~/.vpm
```

unless explicitly running a dedicated integration environment.

---

## 35. Testing Principles

Vut testing follows these principles:

1. `vpm test` is the project test command.
2. Tests use normal `.vut` syntax.
3. The test declaration syntax must be explicitly standardized before implementation.
4. Tests use the normal compiler.
5. Compilation failures are reported with normal diagnostics.
6. Assertion failures show useful context.
7. Tests should be independently runnable.
8. Parallel execution may be used while retaining deterministic reporting.
9. Compile-fail tests are first-class compiler tests.
10. Diagnostic behavior requires golden tests.
11. Compiler bugs should receive regression tests.
12. Fuzzing should protect parser/frontend robustness.
13. Runtime and stdlib require dedicated testing.
14. Tests must not mutate real user package stores.
15. CI should eventually cover every supported platform.
