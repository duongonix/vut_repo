# Diagnostics and testing

Diagnostics must cover:

- invalid `map` arity in type position;
- invalid map key type;
- key type mismatch;
- value type mismatch;
- empty map without expected type;
- wrong key/value passed to methods;
- map as FFI-safe extern value;
- invalid mutation during iteration when static prevention is unavailable.

Required tests:

- type tests for `map(str, i32)`, nested maps, map in data/list/result, and invalid arity;
- literal tests for one entry, multiple entries, empty with expected type, and empty without context;
- API tests for len/is_empty/get/set/contains_key/remove/clear/reserve/capacity;
- growth and collision tests;
- ownership tests for insert/replace/remove/clear/drop/move/COW;
- native executable E2E tests.

Regression coverage must keep `list(T)`, `array(T,N)`, `@(...)`, `array(...)`, `result(T,E)`, async/await, FFI, data, methods, for loops, formatter, and LSP behavior intact.
