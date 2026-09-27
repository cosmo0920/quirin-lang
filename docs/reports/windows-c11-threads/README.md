# Windows C11 thread emission

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Date: 2026-09-08.

## Scope and build

The task and checked-mutex emitters embed
`crates/qcore-core/src/c11_threads_compat.h`. Windows now implements the exact
subset those emitters use: `thrd_t`, `thrd_start_t`, `thrd_create`, `thrd_join`,
`thrd_equal`, `thrd_yield`, `mtx_t`, `mtx_plain`, `mtx_init`, `mtx_lock`,
`mtx_unlock` and `mtx_destroy`. Linux keeps its native `<threads.h>` branch and
macOS keeps its existing pthread branch. Both source routes admit Windows.

From a Visual Studio x64 developer shell at the repository root:

```powershell
cargo run -p qcorec -- emit compiler/tests/fixtures/ct/CT13/case.qr -o generated.c
cl /nologo /std:c11 /W4 /WX generated.c /Fe:generated.exe
.\generated.exe
```

There is no extra source file, include path, link flag or installed pthread
library. MSVC/Windows SDK supply the CRT and Win32 primitives. Windows Vista
or newer primitives are required; local execution is Windows x64. The test
observers that include `<stdatomic.h>` additionally use MSVC's
`/experimental:c11atomics`; emitted task code itself does not need that flag.

## Provenance and changes

This is a small adapted subset of the user-supplied Fluent Bit Monkey code,
not a vendored or linked copy of the entire pthread library. Sources retrieved
at repository revision `f78472443d32691c319c424c75d9ea2343dbc8f7`:

| Source | SHA-256 (UTF-8 source bytes) |
|---|---|
| [winpthreads.c](https://github.com/fluent/fluent-bit/blob/f78472443d32691c319c424c75d9ea2343dbc8f7/lib/monkey/mk_core/external/winpthreads.c) | `b313c538cc9484b3b5e2425c64bf7707577e724fefc235ae613c4aca6b19b186` |
| [winpthreads.h](https://github.com/fluent/fluent-bit/blob/f78472443d32691c319c424c75d9ea2343dbc8f7/lib/monkey/include/monkey/mk_core/external/winpthreads.h) | `f19f439b6b309d02902171cc5f2d0cc7e188a8a48f0e9b45e4e245ba39d5ff15` |

The Lockless copyright, redistribution conditions and disclaimer are retained
in the embedded header and therefore in emitted C. Binary redistributors must
retain that notice in accompanying materials. The core crate's SPDX expression
now includes BSD-3-Clause for this component.

The adapter uses `_beginthreadex` with a retained record. A worker writes its
signed `int` result directly into that record and never reads the handle that
the creator publishes. This accommodates a worker completing before creation
returns, without an unsynchronized handle-publication polling loop. There is
no cancellation, TLS allocation or detached-thread cleanup path.

If either allocation or native creation fails, output thread storage stays
unchanged and unpublished storage is freed. Join checks for `WAIT_OBJECT_0`,
then a successful `CloseHandle`, before copying the result and freeing the
record. On either failure it returns `thrd_error` with record and output result
intact. The emitted task runtime's existing policy aborts before reclaiming
the environment after any such failure. This differs from upstream's unchecked
wait/close and unreleased record after `_beginthreadex` failure. The required
wait-result distinction follows [Microsoft's wait API contract](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject).

Only `mtx_plain` is accepted. Exclusive SRW locks implement the supported
non-recursive mutex operations and need no separately allocated kernel handle.
Unsupported kinds return an error. Recursive use is outside the checked
profile and [SRW exclusive acquisition is non-recursive](https://learn.microsoft.com/en-us/windows/win32/sync/slim-reader-writer--srw--locks).

## Evidence and boundaries

`compiler/tests/windows_threads.rs` and its C fixture test signed extrema,
null result output, completion before creation returns, allocation/create
failures, unsuccessful wait and close with successful retry, unchanged result
storage, balanced allocation/OS handle counts, invalid mutex kind, and 8,000
protected increments from four actual threads. Separate fixed native traces
cover read loans, read shares and owned-error task emission. Two injected worker
allocation failures return the original Buffer through an owned error and real
creator join; reference traces and per-address single releases agree.

The existing source-task and ordinary CT native/fault tests now use MSVC on
Windows. They cover failed creation and sibling recovery, failed join with no
premature reclamation, owned transfers and worker allocations, and real
completed-but-unjoined custody. The completion fixture has a Windows timeout
instead of POSIX `alarm`. Mutex source/model runs and a CLI check/emit/native
recovery test use the same Windows runner. Its output normalization removes
only CRT CRLF line endings. Failure-hook compilations suppress MSVC C4702
because constant failure deliberately makes success arms unreachable.

CT03/CT17 and CT26 keep their Unix sanitizer configurations scoped to Unix;
Windows still checks and emits those fixtures, with native Windows observations
recorded separately. CT50's Linux mapping-address observation remains Linux-only.
No CT mapping, diagnostic code, checked semantic rule, Lean definition or
verification admission rule changes. Native OS synchronization remains an
implementation premise; these tests do not prove Rust/Lean/C refinement or
arbitrary schedules. This is not full C11 `<threads.h>` or POSIX conformance:
condition variables, timed/recursive locks, sleep, detach, thread-local storage,
once initialization and cancellation are absent.

## Validation

Windows x64 / Visual Studio 2026 MSVC results:

- All 18 targeted tests in `source_tasks`, `checked_mutex_guards` and
  `windows_threads` pass, including native AddressSanitizer execution. Windows
  UBSan/LSan and a race detector are not claimed.
- All four CT harness tests pass. Target-scoped exclusions are described above;
  the manifest is unchanged.
- All four CLI tests pass with `QCORE_CLANG` pointing to an intentionally absent
  executable. The 17 then-current thread/mutex tests also pass with this override
  (the added owned-error failure test was subsequently run under ASan).
- The three core mutex-emission tests pass, including all fixed model/native
  trace comparisons. These are ordinary MSVC runs on Windows despite the
  historical sanitizer wording in the test name.
- Standalone emitted CT13 task C compiles and executes with
  `cl /std:c11 /W4 /WX`, without the observer's atomics switch or extra includes.
  CT13 and CT26 both compile and execute with native Windows Clang using
  `-std=c11 -Wall -Wextra -Werror`; CI now checks these builds.
- `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets -- -D warnings` pass.
  Both reevaluation examples exit 0.
- Elan installed the repository-pinned Lean 4.33.1 toolchain from its official
  release. `lake build` passes all 74 jobs and `scripts/check_lean_audits.py`
  passes all 30 audits, including the whole-project axiom closure. The audits
  use the installed pinned toolchain's `bin` on PATH to avoid elan's online
  self-update probe in the restricted execution environment.
- The existing Libevent checkout matches the gitlink exactly at
  `d82464a277d0f42703702c4dfd9af6af38595a83`; its workspace test passes.
  The submodule initialization command itself encountered the host Git shell's
  missing-helper PATH error, but this checkout was already initialized.
- The pre-existing Apple and native-C11 preprocessor branch bodies match HEAD
  exactly. Native Linux/macOS execution was not repeated on this Windows host.

The full workspace command was run with `--no-fail-fast -- --test-threads=8`.
That run recorded **1,283 passed, 11 failed, 1 ignored**; it preceded the final
additional owned-error test. One failure was the generic-driver test's stale
Windows-rejection expectation: it is now updated, and that target's five tests
pass. The remaining **10 failures in three untouched targets** prevent claiming
a green aggregate workspace gate:

| Target | Failures | Observed failure |
|---|---:|---|
| `qcore-audit --test second_source_set` | 2 | Audit subprocess returns failure in the nested-root fixtures. |
| `qcore-c-gateway --test o04_constructor_caller` | 7 | CRLF-sensitive byte/mutation checks, incompatible `size_t` declarations and an incompatible-pointer fixture. |
| `qcore-verification --test counter_source` | 1 | A multiline mutation does not change the CRLF contract fixture. |

These targets, their fixtures and contract bytes were not changed here. The
aggregate suite was not repeated after the expectation-only repair. Local logs
are in `target/windows-threads-workspace.log`,
`target/windows-threads-probe.log`, `target/windows-threads-followup.log` and
`target/windows-threads-lean-audits.log`. No production audit verdict is promoted.

## Integration with incoming main

Before pushing, this commit was rebased onto `87ef606`, which adds the F02
trace relation and status consolidation. Both appended status records are
retained. F02's generated-C pins move because the embedded header changes;
[f02-emission-parity.json](f02-emission-parity.json) records that restoring
only the old header recovers each old pin exactly. The non-Windows branch
bodies remain identical. A Windows test checks both new pins and native
renderings through the incoming relation, with a missing-fact rejection control.
The previous Linux executable records remain historical; Linux native
revalidation of the new pins is left to CI. This does not widen F02's relation.

After integration, all 23 targeted compiler tests (threading, CT and mutex)
and all 19 core trace-relation tests pass. Formatting and workspace Clippy are
clean. The merged Lean build passes all 75 jobs, and all 31 audits pass; the
audit log is `target/windows-threads-rebase-lean-audits.log`. The full workspace
suite was not rerun; its previously recorded failures remain the aggregate
validation limitation.
