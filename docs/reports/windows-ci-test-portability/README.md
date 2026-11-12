# WINDOWS-TEST-PORTABILITY: the first full Windows CI suite

Card `WINDOWS-TEST-PORTABILITY`, base `5389f1d37`. CI's first full Windows
run ("Test full suite with MSVC and explicit Clang installation", run
43215923769, `windows-2022`, main `73c39ffcf`) failed 72 tests in 28 binaries;
the Linux suite passed. This card repairs the tests' target assumptions. It
loosens no analyzer, loader or ABI check, turns no `Unknown` into `Safe`, and
keeps every Linux assertion. The 2026-09-23 report
[`windows-test-portability`](../windows-test-portability/README.md) is a
different, frozen record and is left untouched.

## Causes

1. **LP64 fixture types.** Clang's default Windows target is
   `x86_64-pc-windows-msvc` (LLP64). Under MSVC compatibility it predeclares
   `size_t` as `unsigned long long`, so a prelude's `typedef unsigned long
   size_t;` is a redefinition error, and `long` is 4 bytes, which changes
   sized builtins (`__sync_fetch_and_add_4`) and record layouts.
2. **LP64 contracts withdrawn against LLP64 declarations.** A contract that
   states a C `long` as `i64` meets a 32-bit `long`, whose reading the ABI
   reader refuses (`abi-unreadable-declaration`). A contract that states an
   enumeration without a negative value as `u32` (the System V reading) meets
   Windows' `int` enumeration (`abi-parameter-type-disagreement`,
   `abi-return-type-disagreement`). Both are the analyzer failing closed.
3. **glibc headers.** One binary includes glibc's own headers.
4. **Host path spelling.** Two tests compared `/`-spelled paths with
   Windows-spelled ones.
5. **Offline metadata.** `cargo metadata --locked --offline` reads every
   package of the lockfile; a Windows build never downloads the
   `cfg(unix)`-only `libc`, so the two dependency-closure tests could not run.

## Repairs

Repair classes are the brief's: (a) a target-neutral fixture that asserts
the same thing on Linux, (b) a gate or a per-target arm naming the real
condition, (c) a product fix. Each (a) uses Clang's predefined
`__SIZE_TYPE__`, `__PTRDIFF_TYPE__`, `__INT64_TYPE__` or `__UINT64_TYPE__`,
which preprocess to the type the fixture spelled on LP64 (`long int`,
`long unsigned int`), so the Linux unit is the same unit. Each (b) arm on
Windows asserts the fail-closed outcome by name, following
`abi_enum_const_pp`'s per-target arms.

"Before" is CI's count (failed/total) or, with the `x86_64-pc-windows-msvc`
Clang wrapper on Linux, the local count. "Linux" is the plain Clang 21
after the repair. "Wrapper" is the local Windows-target Clang after the
repair; a `cfg!(windows)` arm cannot fire in a Linux-hosted test, so those
arms were run by temporarily substituting `true` for the predicate (bytes
restored and checked after each run), marked "flip".

| Binary | CI failed | Cause | Repair | Linux after | Wrapper after |
|---|---|---|---|---|---|
| gateway `loop_measure` | 15 | 1 (`size_t`, `uint64_t`) | a | 15/15 | 15/15 (before 0/15) |
| gateway `builtin_expect_null_test` | 5 | 1 (`size_t`, `ssize_t`) | a | 5/5 | 5/5 (before 0/5) |
| gateway `atomic_weak_memory_events` | 1 | 1 (`long` names `__sync_fetch_and_add_4`) | a: `long long` names `_8` on both | 7/7 | 7/7 (before 6/7) |
| gateway `descriptor_cells` | 6 | 2 (`time_t`/`long nsec` unreadable) | a | 14/14 | 14/14 (before 9/14) |
| gateway `event_removal_wrapper` | 2 | 2 (`unsigned long` against `u64`) | a | 4/4 | 4/4 (before 2/4) |
| gateway `retaining_call_minted_name` | 1 | 2 (`long` against `i64`) | a | 17/17 | 17/17 (before 16/17) |
| gateway `union_opaque_writes` | 2 | 1 (`long` layout) | a | 6/6 | 6/6 (before 4/6) |
| gateway `detach_lookup_membership` | 6 | 1 (retained LP64 frame) | b: frame respelled on Windows | 8/8 | flip 7/8 (before 2/8) |
| gateway `detach_wrapper_composition` | 4 | 1 (retained LP64 frame) | b, as above | 6/6 | flip 6/6 (before 2/6) |
| gateway `rule33_switch` | 5 | 1 (retained LP64 frame) | b, as above | 5/5 | flip 5/5 (before 2/5) |
| gateway `cfl_mpack_external_declarations` | 1 | 2 (enumerations as `u32`) | b: Windows arm, new census test | 10/10 (+1 test) | flip 10/10 (before 8/9) |
| gateway `librdkafka_header_inlines` | 1 | 2 (enumeration as `u32`) | b: Windows arm | 8/8 | flip 8/8 (before 7/8) |
| gateway `libc_gap_declarations` | 5 | 3 | b: `#![cfg(all(target_os = "linux", target_env = "gnu"))]` | 6/6 | not compiled on Windows |
| audit `contract_supersession` | 1 | 4 (by inspection) | a: `Path::ends_with` | 4/4 | needs Windows |
| audit `rule_switch_entry_points` | 1 | 4 (by inspection) | a: components joined with `/` | 4/4 | needs Windows |
| audit `dependency_graph` | 1 | 5 (reproduced) | CI: `cargo fetch --locked` before the suite | 2/2 | needs Windows |
| plugin `dependencies` | 1 | 5 (reproduced) | as above | 1/1 | needs Windows |

The retained cJSON frames (`fixtures/detach-*`) are Linux preprocessor
output and stay byte for byte; the new
[`support/retained_frame.rs`](../../../crates/qcore-c-gateway/tests/support/retained_frame.rs)
respells their single `typedef long unsigned int size_t;` as
`unsigned long long` on Windows only (the library's
`checked_indexed_child_lookup_keeps_borrowed_result_summary` is the
precedent), asserts the typedef is present exactly once, and for the two
include-path tests writes the respelled headers to a fresh directory. The
mutation control that makes `size_t` signed now mutates the target's
spelling. The one wrapper failure left in `detach_lookup_membership` is the
native build's `<assert.h>`, which the wrapper cannot find without a
Windows SDK.

In `cfl_mpack_external_declarations`, the new
`the_enumeration_contracts_follow_the_target_abi` reads the declared-ABI
census for `mpack_writer_flag_error`, `mpack_tag_type` and
`mpack_reader_destroy`: `Agreed` on System V targets (a new Linux
assertion), `Withdrawn` with the named parameter or return disagreement on
Windows. The accepting test then requires `Unknown` for those three rows on
Windows and `Safe` everywhere else. The offline-metadata cause was
reproduced from Linux with a fresh `CARGO_HOME`: after `cargo fetch --target
x86_64-pc-windows-msvc`, `cargo metadata --locked --offline` failed on
`libc v0.2.189`; after a plain `cargo fetch` it succeeded; a Linux-only
fetch needs nothing more.

## Pending the CI excerpts or a Windows host

Main `5389f1d37` prints each failed unit's panic into the job log; until
that run's excerpts are read, these are unrepaired or unconfirmed:

- gateway `member_effect_summary` (1), `cg30_dialects_flags_types` (1), and
  the sixth `descriptor_cells` failure: locally `cg30` passes, and
  `member_effect_summary` fails only on the wrapper's missing `<stdlib.h>`;
  with a stand-in header holding the UCRT's preprocessed `malloc`, `calloc`,
  `realloc` and `free` declarations it passes 11/11, so the CI failure lies
  elsewhere in the real UCRT headers;
- compiler `bitwise_conditions` (1), `branch_arms` (3), `branch_statement`
  (1), `logical_connectives` (1), `system_trace_examples_windows` (1),
  `ct_corpus` (1): native MSVC builds through `tests/support/windows_c.rs`;
- audit `lib` (1), `current_cli` (2), and plugin-measure `lib` (1);
- confirmation of the two path-spelling repairs and the fetch step.

## Commands

- Linux, plain Clang 21: the fifteen gateway binaries above, 0 failures;
  `contract_supersession` 4/4, `rule_switch_entry_points` 4/4.
- `cargo fmt --all -- --check`: exit 0.
- `cargo clippy --target x86_64-pc-windows-msvc --workspace --all-targets
  --keep-going -- -D warnings`: exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `python3 -m unittest discover -s scripts/tests`: two runs of 756 tests,
  each with 1 failure, a different timing-sensitive gate-slot status test
  each time (`test_each_waiter_shows_its_queue_position`,
  `test_a_backing_off_drain_and_a_job_of_an_older_client_are_told_apart`);
  each passed in isolation.
