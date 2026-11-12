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

## Second pass: the CI panic excerpts

The excerpts of CI run 43225703921 (main `5389f1d37`, before this card's
commits) confirmed the causes above for every binary in the first table:
the `size_t` redefinitions, the `abi-unreadable-declaration` and enumeration
withdrawals, `grp.h` missing, the `\`-joined supersession and entry-point
paths, and `failed to download libc v0.2.189 ... --offline` in both
dependency tests. `descriptor_cells`' sixth failure is the
`child_rule_38_off` child process, which fails on the same withdrawal and
passes with the first-pass fixture. The rest:

| Binary | CI failed | Cause (from the excerpt) | Repair | Linux after |
|---|---|---|---|---|
| compiler `bitwise_conditions`, `branch_arms` (3), `branch_statement`, `logical_connectives`, `system_trace_examples_windows`, `ct_corpus` | 8 | x64 `cl.exe /W4 /WX` raises C4127 (conditional expression is constant) on the fixtures' intentional constant conditions | `tests/support/windows_c.rs` passes `/wd4127` on every MSVC target, not only ARM64; `/W4 /WX` stay for every other warning | Windows-only code; Windows-target Clippy clean |
| gateway `cg30_dialects_flags_types` | 1 | the runner's LLVM is Clang 20.1.8: it accepts `-std=c2y` but has no `_Countof` (Clang 21) | b: the `_Countof` test returns early below Clang 21, as it already does below the C2y mode | 10/10 |
| gateway `member_effect_summary` | 1 | the UCRT's `malloc` declaration is unreadable to the declared-ABI reader (`abi-unreadable-declaration`), so `setnew` names the withdrawal instead of the member store | b: on Windows `setnew` asserts that named withdrawal (`Unknown`); `setnew2` (`obj_create`) asserts the member-store refusal on every target | 11/11 |
| audit `lib` (`route_units_key_linked_monkey_units_like_a_linked_run`), `current_cli` (`a_linked_monkey_run_...`) | 2 | c: `is_monkey_unit` tested the located relative path as text (`starts_with("mk_core/")`), which on Windows is `mk_core\...`, so no Monkey unit was found | `Path::starts_with` by components; the same decisions on Linux | lib 186/186, `current_cli` 19/19 |
| audit `current_cli` (`the_sidecar_reproduces_the_provider_selection`) | 1 | the CLI's current directory keeps `%TEMP%`'s 8.3 name (`RUNNER~1`); the test's expectation came from `canonicalize()` (`runneradmin`) | on Windows the expected root is the fixture's own spelling, which the child was started in | 19/19 |
| plugin-measure `lib` (`invalid_routes_are_refused`) | 1 | c: route validation used `is_absolute()`, and on Windows `/etc/passwd` is rooted but not absolute, so it was admitted as repository-relative | a root, a drive prefix or `..` component is refused; Linux decisions unchanged | 27/27 |

The two product fixes (c) change no analysis verdict: one selects which
compile-database entries are Monkey units, the other refuses a route input
path. Neither has a Lean counterpart. The UCRT `malloc` reading is a reader
limitation that fails closed, recorded as a follow-up, not widened here.

Still to be confirmed on the runner: every row of both tables, since
the excerpts predate this branch. `detach_lookup_membership`'s native build
of the respelled frame and `current_cli`'s linked Monkey run are those most
dependent on Windows behaviour that the Linux wrapper cannot show.

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
