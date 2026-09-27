# Source and platform records, September 8–9

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
### Quant delimiter alias `[| |]` (2026-09-09)

`quant[| ... |]` is now an accepted delimiter alias for `quant< ... >` on
closed quantitative index types, over the byte-identical interior grammar.
`[|` and `|]` are new single lexer symbols occupying previously rejected
syntax space (a bare `|` was and remains a lexer error, so no existing
program changes meaning); the opener fixes the required closer, and a mixed
pair such as `quant[| ... >` is refused, never repaired. Both spellings
resolve to the same `SourceQuantIndexType`/`ResolvedQuantIndexType`, source
spellings round-trip verbatim, and the `quant<access ...>` exclusion applies
symmetrically to both openers. The alias exists because the angle form's
closing `>` sits beside where-clause comparison operators; it adds no type,
no diagnostic code and no semantic surface, so no Lean change is required
(the resolver and everything downstream are untouched). Five new tests pin
lexing beside plain brackets, identical elaboration of both spellings, the
bracket twin of the pinned `quantified_indices.qcore` fixture in parameter,
return and contract-clause position (the corpus file itself is untouched),
and mixed/unterminated-delimiter refusals. The implemented-grammars G04
entry, spec prose and the Emacs mode's `|` punctuation class are updated in
the same change. Validation: 1752 workspace tests passed (5 new),
3 ignored, 0 failed; formatting, Clippy, both reevaluation probes, all
42 Lean audits and the 9 Emacs ERT checks pass.

### Sequential C emission indentation (2026-09-08)

`qcore_core::c_emit::emit_c11`'s runtime helper bodies (`q_trace`,
`q_acquire`, `q_release`) and `main`'s result/trace report block were written
with Rust string-literal line continuations, which strip the following line's
leading whitespace, so those regions of every emitted sequential translation
unit landed flush left with visually mis-nested braces. The literals now carry
the intended four-space-per-level indentation; emitted bytes change only in
leading whitespace, the token stream is identical, and a re-emitted unit
compiles under `-std=c11 -Wall -Wextra -Werror` with byte-identical program
output. A new unit test pins the repaired lines and refuses any column-zero
statement outside preprocessor lines, file-scope declarations, braces and goto
labels. The task, mutex and extern emitters already placed indentation before
their continuations and are untouched, so the two pinned F02 emission digests
do not move (re-emission of all four thread examples verified byte-identical).
This is a formatting repair of generated C, not a semantic, verdict or
coverage change. Validation: 1652 workspace tests passed (one new), 1 ignored;
formatting, Clippy and both reevaluation probes pass; the Lean gate could not
run on this host because `lake` is not installed, and no Lean-relevant
semantics changed.

### Core concurrency prelude (2026-09-08)

Explicit concurrency imports and core prelude (2026-09-08): source files opt in
with `#import std.concurrency;`; the concurrency context is unavailable by default.
The core owns exact bundled-module resolution; the frontend rejects unknown
modules (`Q014`) and missing imports (`Q015`) before checking each file, so imports
do not leak across compilation inputs. The task profile retains its separate
`concurrency checked;` marker. `qcore_core::prelude::concurrency` exposes existing
core models and backend APIs through compatible Rust imports; frontend lowering
and the verification driver consume it. `QCore.ConcurrencyPrelude` exposes 14
existing bounded properties and imports six new module-selection theorems.
Thread examples and current source fixtures opt in explicitly, with a paired
Rust custody example. Arbitrary user/package imports, implicit imports, combined
task/mutex semantics and general native correctness remain outside this change.
Validation: 1387 workspace tests passed, 1 ignored; all 32 Lean audits,
formatting, Clippy, both reevaluation probes and four native examples pass.
Evidence: `docs/reports/core-concurrency-prelude/README.md`.

Windows Clang fixture repair (2026-09-08): O04's reduced prelude uses
Clang target integer types, and the generic consuming sink accepts object
pointers without incompatible C calls. Fixture-local Git attributes preserve
the four upstream excerpts' exact LF bytes and existing hashes. Both the O04
and counter-source negative contract tests normalize only their local mutation
baselines and still assert that substitution occurred. All 14 O04 assertions
pass through native Visual Studio discovery; both affected targets (20 tests)
pass with explicit `QCORE_CLANG`. Windows CI covers both routes. Strict C type checks
also pass for Windows x64 and Linux x64 targets. Checker/Lean semantics,
production verdicts and pinned upstream contents are unchanged. The completed
Windows workspace gate records 1,316 passed, zero failed and one ignored,
superseding the earlier fixture-failure limitation. Formatting, workspace
Clippy, both reevaluation probes, the rebased Lean build and all 31 audits
pass. Validation:
`docs/reports/windows-clang-fixtures/README.md`.

Windows second-source-set fixture (2026-09-08): compile databases now name the
gateway-selected Clang and omit Unix-only build flags on Windows. All five
tests pass through Visual Studio discovery and explicit `QCORE_CLANG`; CI also
runs this target with Clang removed from PATH. Failed audit assertions include
status, stdout and stderr. Production checker and proof semantics are unchanged.
Formatting, workspace Clippy, both reevaluation probes and the pinned Lean build
pass. Workspace validation still fails in the untouched `o04_constructor_caller`
and `counter_source` targets.

Windows executable examples (2026-09-08): `examples/threads/` adds two copied
workers, an owned-buffer round trip with independent parent work, and two
shared readers followed by an exclusive parent write. `build-windows.ps1`
emits C and builds executables under `target/thread-examples/`; `-Run` executes
them. All three run under MSVC with `/std:c11 /W4 /WX` and an unavailable
`QCORE_CLANG`, producing main results 7, 42 and 7 respectively. The Windows
MSVC core CI job runs the same script. The guide explains developer-shell
setup, compile/link commands, process exit codes and the buffered trace output.
Compiler, runtime and proof semantics are unchanged. Formatting, workspace
Clippy, both reevaluation probes, the pinned Lean build and all 31 audits pass.
The aggregate Windows workspace run records 1,306 passed, ten existing fixture
failures and one ignored test. The failures are two in `second_source_set`,
seven in `o04_constructor_caller` and one in `counter_source`. These failures
are outside the example changes; the run started before the concurrent
audit-fixture repair was finished.

Lean build performance (2026-09-08): TaskTraceRelation's finite-domain proofs
expand D3's generated inputs once before kernel evaluation; theorem statements,
domains, digests and allowed axioms are unchanged. The affected-module rebuild
passes in 47.5 seconds locally; the next incremental build takes 0.13 seconds.
`python3 scripts/check_lean_audits.py` now builds from the pinned proof directory,
reports progress, enforces finite time limits and cleans up workers on timeout
or cancellation. All 31 audits, eight runner tests and the standing gates pass.
Evidence: `docs/reports/lean-build-performance/README.md`.

Windows native concurrency (2026-09-08): the shared emitted C11 thread subset
now includes a self-contained Windows implementation adapted from Monkey
winpthreads. Joinable start records preserve signed int results; failed native
creation frees unpublished storage, and failed wait/close retains the record
and result until successful join. Plain non-recursive mutexes use exclusive
SRW locks. Source task and mutex routes admit Windows; MSVC tests cover native
reference agreement, failure recovery, completed-but-unjoined custody and
mutex contention. Unix sanitizer fixtures and CT50's Linux mapping observation
retain their target scope; CT mappings and checker/Lean semantics are unchanged.
This is the emitted subset, not full pthread/C11-threads conformance.
The pinned Lean build and all 31 audits pass; the aggregate workspace gate
retains ten failures in three untouched audit targets.
See `docs/reports/windows-c11-threads/README.md` for provenance, build commands,
validation and platform limits.


The dated entries below are archived in the order they landed. Each one keeps
its own checkpoint, counts and boundaries; a later entry does not supersede an
earlier one unless it says so.

<!-- migrated-section:end -->
