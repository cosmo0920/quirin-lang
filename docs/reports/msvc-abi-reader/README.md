# MSVC default-C ABI declarations

The caller fixtures for UCRT `memcmp`, `strncmp` and `strnlen` now require
`Safe` and independent certificate replay against the real preprocessed
headers. No library body or contract is substituted. The native test declares
`MainSourceDefinitions`: it certifies its caller, while unrelated UCRT inline
helpers remain outside that recorded target scope. This is conditional custody
safety under the loaded libc contracts, not string-bounds safety or a proof of
the CRT implementation. [Prediction](PREDICTION.md) was committed before code
as `af37f1e5` (base `53ffdbd3`).

The lexical declaration reader interprets one function-prefix `__cdecl` as
the default C convention, removing it before counting result pointer levels.
It reads exactly one `__int64` plus at most one `signed` or `unsigned` word,
in either order, as the existing signed/unsigned 64-bit projections. `isize`
and `usize` retain the existing 64-bit-pointer premise. Ordinary `long` still
has the reader's LP64 premise; this is not general LLP64 or x86 support.
A newly readable cdecl result or parameter involving ordinary `long` stays
unreadable, including through typedef chains, to avoid inheriting a misleading
LP64 reading on MSVC. Explicit `long long` keeps its 64-bit reading.
Other conventions, repeated annotations, mixed sized specifiers, wrong widths,
wrong signs and unproven source effects retain their refusals. No AST is used
in the ABI comparison; derivation, validation and certificate replay all
recompute from retained tokens and contracts.

`QCore.MsvcAbiReader` models the two decisions and audits 23 theorems:
exact word counts, signed/unsigned readings, no pointer/custody laundering,
preservation of existing comparisons, unreadable-convention refusal, native
query agreement and width/qualifier/hidden-pointer-result disagreements.
Compiler language facts are external premises; extraction and Lean-to-Rust
correspondence remain the review-maintained premises of [boundary 28](../../agents/evidence.md#constraint-28).

The `loader_abi` admission table adds 18 held-out MSVC rows; an additional
caller control refuses an allocator contract that hides a cdecl pointer result.
The real-header negative control deliberately declares MSVC `memcmp`'s size
as `u32` and must return `Unknown` with `abi-parameter-type-disagreement`.
Prior failure logs show why whole-header certification is not claimed: the
caller becomes Safe but unrelated UCRT inline bodies keep the whole expanded
unit Unknown until the test selects its intended main-source scope.

The additional package-inheritance fixture reconstructs a published LF parent
only when those exact bytes match the inventory's existing SHA-256. Raw bytes
are retained when they already match. This repairs checkout CRLF differences
without changing a pin or making the production bundle verifier newline-insensitive;
changed/missing-parent controls still refuse. Its eight tests pass.

Validation on Windows x86-64 used Rust artifacts at `0a22a998`, eight binary
workers and four libtest threads. The full no-fail-fast run completed **408 units,
421 summaries: 6,762 passed, 1 failed, 11 ignored**, exit **101**. The failure was
`checkpoint_cli::interrupted_run_preserves_completed_units_for_resume`: no first
unit checkpoint within its 30-second deadline under the parallel gate. The
unchanged control passed alone (1 passed, exit 0), and the entire checkpoint
binary passed alone with the same four threads (7 passed, exit 0). All supplied
Windows failures passed their full-gate suites, including SDK linkage, FFI
refusals, package identities, allocator fixtures and real UCRT queries.

`scripts/run_test_binaries.py` now schedules `checkpoint_cli` in its exclusive
phase. Its deadline, assertions, captured arguments and environment are unchanged.
The Python suite covers exclusive planning and absence of overlapping units;
after this scheduling edit it exits 0 (365 tests, 14 skipped). The full workspace
was not repeated after this Python-only scheduling edit; its original exit 101
remains recorded rather than being relabelled a clean gate.

Commands and private logs (Python uses the bundled interpreter):

| Command | Exit and result | Log under `private/msvc-abi-20261001/` |
| --- | --- | --- |
| `python scripts/run_test_binaries.py --jobs 8 --log-dir private/msvc-abi-workspace-final-20261001` | 101; counts above | `workspace-final.log`; per-unit logs and `summary.json` in the named log directory |
| `cargo test -p qcore-audit --test checkpoint_cli interrupted_run_preserves_completed_units_for_resume -- --exact --test-threads=1` | 0; 1 passed, 6 filtered | `checkpoint-isolated.log` |
| `cargo test -p qcore-audit --test checkpoint_cli -- --test-threads=4` | 0; 7 passed | `checkpoint-binary-isolated.log` |
| `cargo test -p qcore-c-gateway --test loader_abi --test w46_libc_event_contract_gaps` | 0; 16 + 5 passed | `focused-guarded.log` |
| `cargo test -p qcore-compiler --test contract_inherit_qcore_provider` | 0; 8 passed | `package-fixture.log` |
| `cargo fmt --all -- --check` | 1; Windows command-length error 206 | `fmt-reference.log` |
| Edition-aware formatting batches of Cargo metadata target roots, then changed files | 0; all 435 roots and later changes checked | `fmt-batched.log`, `fmt-latest.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | `clippy-guarded.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `probe-guarded.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `followup-guarded.log` |
| `python scripts/check_lean_audits.py` | 0; 262 audits | `lean-guarded.log` |
| `python -m unittest discover -s scripts/tests` | 0; 365 tests, 14 skipped | `python-checkpoint-scheduling.log` |

Exit files accompany the logs. Lean used the pinned 4.34.0 toolchain; native
Clang is recorded in `clang-version.txt`. Earlier failed diagnostic attempts
and interrupted workspace runs remain under their private run directories;
they are not passing gate evidence. No pinned GNU/Linux corpus measurement or
broader coverage increase is claimed here.

Primary facts: [Microsoft `__int64`](https://learn.microsoft.com/en-us/cpp/cpp/int8-int16-int32-int64?view=msvc-170)
and [`__cdecl`](https://learn.microsoft.com/en-us/cpp/cpp/cdecl?view=msvc-170).
