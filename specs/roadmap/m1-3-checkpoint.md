# M1.3 handoff checkpoint

## Audited baseline

The handoff starts from `b00de8e` (fs), after `d1b2057` (io). The previous
agent left 15 modified files and the untracked codegen `numeric.rs` module.
All changes were preserved. The checkpoint is numeric core/conversions
(execution step B), before unit, io/fs migration, process or final hardening.
Workspace checking and the five existing numeric execution tests passed at
takeover; this did not establish feature completeness.

## Takeover changes and evidence

- Corrected negative signed-to-unsigned conversion checks and conversions
  between f32 and the float alias; used target pointer width for float conversion
  bounds; removed codegen debug output.
- Accepted contextual signed minimum literals while retaining positive overflow
  rejection. Added execution regressions for unsigned arithmetic/display and
  every numeric conversion destination.
- Added semantic `unit`, distinct from `void`: size zero, alignment one, Copy,
  no drop or allocation. Internal function ABI carries a constant token;
  raw C signatures reject unit. Result, data and enum zero-sized payload accesses
  skip reads/writes. Optional, array and frame access paths also skip zero-sized
  payload access; shared collection element loads materialize the constant token.
  Function/generic/Result/data/enum execution tests pass. Remaining storage paths
  still require a full audit before unit is considered Complete.
- Migrated payload-less io/fs APIs from result[bool,E] to result[unit,E].
  All 14 existing io/fs execution tests pass on Windows.
- Fixed type-parameter discovery to compare source identity as well as offsets,
  preventing generated display functions from inheriting unrelated parameters.
- Corrected narrow numeric data-field stores and floating data-field loads.
- Retargeted generic constructors to concrete instance symbols in MIR call
  tables; generic functions returning parameterized data no longer use template
  layouts for their payloads. All ten existing generic execution tests pass.

Seven numeric tests, two unit tests and workspace Clippy with `-D warnings`
passed. A full `cargo test --workspace --all-targets --no-fail-fast` run passed
every target except `vut-runtime --test bytes_contract`: its out-of-bounds access
test aborts the test process (`bytes.at: index 99 out of range`, Windows exit
0xc0000409), rather than returning the old zero fallback expected by the test.
The failure also reproduced when that target ran alone; the subsequent confirmed
numeric-semantics continuation reconciled the test with the bounds spec as
described below. A subsequent full all-target workspace run (with
`--no-fail-fast`) passed, including existing HTTP targets and bytes_contract.
Final M1.3 feature-completeness gates remain unmet; a green workspace does not
establish completion of process/time/env/os/path. Linux/macOS execution has not
been verified in this takeover.

## Confirmed numeric semantics (2026-09-18)

The user confirmed finite-source validation, then truncation toward zero, then
destination-range validation. `02-type-system.md` now records this normative
order. Codegen follows it and reuses `NUMERIC_PANIC`; no new panic ABI was added.
Cranelift float-to-integer lowering now converts through I64 before reducing
checked narrow destinations, fixing an x64 backend panic for i8/i16.

`numeric_conversion_boundaries.rs` exercises every requested integer destination
through real native executables: signed lower boundaries, unsigned upper
boundaries, fractional truncation, negative fractional unsigned conversion,
upper/lower rejection and NaN/positive/negative infinity traps. It also checks
f32/f64 source conversion and adjacent representable f64 values at 64-bit limits.
All three numeric boundary execution tests pass. Direct integer-to-f32 conversion
avoids double rounding through f64 and has a dedicated regression. Shared
collection temporary stores use the semantic machine width rather than the
width of an uncoerced literal. Ten collection execution tests and two unit tests
pass after these changes. Formatter and workspace all-target Clippy with
`-D warnings` pass.
The final rerun of `cargo test --workspace --all-targets` on these code changes
also passed on Windows (including numeric boundary, io/fs, ownership, HTTP and
runtime bytes tests). No Linux/macOS verification is claimed.

The bytes failure was a pre-existing test/runtime mismatch: `b00de8e` already
traps explicit OOB at/set, matching memory-model §56, while the test expected
the removed zero fallback. `bytes_contract` now tests both traps in isolated
subprocesses, preserving strict checking and first/last fallback assertions.
The successful UTF-8 conversion test also releases its returned string; buffer
tests serialize global allocation accounting.

## Process lifetime decision

The user confirmed that dropping Child closes its owned native handles and
remaining pipe endpoints, without implicit kill or wait/reap. Process lifetime
is independent of handle lifetime. Moved pipe endpoints have independent owners;
dropping a pipe only closes that endpoint. The normative standard-library and
memory-model specs now record this policy.

Command/Child/ExitStatus/Output/Stdio and pipe operations have focused native and
Vut modules using the canonical resource owner. Native archive build and eight
process execution/layout tests pass on Windows: typed failed spawn, early-return
Child drop with a surviving process, explicit kill/wait, successful status,
binary Output, moved stdout surviving Child drop, all three pipe endpoint moves,
stdin drop signalling EOF, pending try_wait, environment configuration ordering,
and 16 repeated spawn/wait/cleanup executions. Clean exits
also exercise the runtime allocation leak gate. `Stdio.null_device` denotes the
null device (`null` is a reserved language keyword).

Both resumed workspace all-target runs passed, including bytes_contract and
HTTP regression targets. The eight process tests were also rerun separately;
the later explicit-return test adjustment passed its targeted rerun. Formatter
and workspace all-target Clippy with `-D warnings` passed. Native handle stress
beyond the 16-run smoke test, cross-platform verification and final M1.3 gates
remain required. Process is not yet marked Complete.

## Remaining execution steps

1. Finish numeric literal/operator/conversion/FFI coverage, pointer-width literal
   validation and constant-folding audit with the confirmed fractional rule.
2. Finish unit diagnostics and coverage of all value-storage paths.
3. Migrate io/fs numeric counts and timestamps to exact ABI types; remove
   temporary numeric buffer workarounds where direct ABI is supported.
4. Re-run Step 0–3 gates after the complete migration.
5. Finish process try_wait, configuration/error-path and repeated resource
   hardening coverage. The typed binary Output and resource-owned Child/pipes
   replace the obsolete text-output/encoded-status wrappers.
6. Complete time, typed env errors/map enumeration, typed os errors and path
   platform verification.
7. Run HTTP regression, examples, resource/leak tests and all final gates.
8. Reconcile canonical specs/tracker with verified implementation.

M1.3 remains **Partial**. M1.4 has not been started. No takeover commit has been
created; changes remain in the working tree.
