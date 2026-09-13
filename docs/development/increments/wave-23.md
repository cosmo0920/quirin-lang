# Wave 23 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave 23 close (2026-09-11): six cards merged one at a time under the
failure-gated chain, each under its own green gate. `NATIVE-LIBRARY-SOURCE-BINDING`
gave a source file its own manifest selection — `extern library NAME;` resolves
that file against the one selected manifest declaring the name, an unbound file
is unchanged and still resolves selection-wide, and a binding only ever narrows,
with a refusal owed exactly where it removes a resolution (band `Q065`–`Q067`,
`NativeLibrary`'s fourth range, zero reserved words, the identity still a stated
pin); it ships `docs/design/STD_HTTP_BINDING_V1.md` beside it, design-only, with
no library named as chosen. `PIPE-DROP-STAGE` (with `PIPE-INTRINSIC-EXAMPLE`)
admitted `drop` as `source.pipe.v1`'s third statement-stage head and its only
releasing one, so the allocating chain wave 22 measured as unreleasable is
spellable end to end with the pipeline releasing the temporary it invented;
custody flows through the unedited checkers, no code and no reserved word were
added, and no other keyword-shaped head joined on a two-part census criterion.
`STRING-SLICE` provided `std.string` and `source.string.v1` — immutable UTF-8
`{ptr, len}` views of statically stored literal bytes, an exact O(1) length and
an allocation-free copy-free checked slice, reference-executed and lowered to C11
with per-stream byte agreement under sanitizers — on band `Q068`–`Q073` under a
new `StandardString` pass, relocated at integration from the `Q065`–`Q070` it
allocated. `COMPOSE-EVENT-LINES` put `print`, `log` and `flush` and the one value
form `let NAME = dispatch ready E;` inside an event entry without moving anything
of `Q058`–`Q062`, and declared no reordering because none is needed: the
`OBS-REORDERING` live-set walk, generalised to a dispatch turn, reads its
emitters off the single-threaded reactor profile, so every record is a barrier
and the existing byte comparison is shown to be the right one rather than
replaced — landed as standing constraint 15c, on relocated code `Q074`.
`NATIVE-LIBRARY-HEADER-DECORATION` made a decorated exported header readable by
*deriving* the export macro from declared, hash-identified include texts,
evaluating none of the four conditional arms it is defined in: a manifest says
which texts may be read and the reader says what they mean, Libevent's census
goes 0 → 264 of 423 read on one declared text, declaring `event2/util.h` refuses
seven headers as whole files because it carries `#error`, and a silently ignored
prefix decoration is repaired — with no new diagnostic code, `Q063` extending
instead. `AOM-DESTROY-SIBLINGS` then decided the two undeclared sibling
destructors in opposite directions: `flb_input_chunk_destroy_direct_routes` is
declared with a **nullable** release where its neighbour carries `nonnull`, and
`flb_input_chunk_destroy_corrupted` is read in full and declined at measured
payoff zero. Its payoff moves the leak pin **272 → 266**, constraint 7's fourth
deliberate move and the first of more than one, entirely by withdrawal-by-refusal
at the preceding call sites rather than by discharge, so the declaration's
discharging arm stays unexercised corpus-wide; `CONSTRUCTOR-RESIDUALS-LEAK-271`
is decided at a measured cost of five withdrawn claims rather than the six wave
17 recorded, its sixth leaf L19544 standing on the success path, which is why the
function keeps its provisional `Unsafe` and the pin reads 266 and not 265. The
contract corpus grows to 73 modules (manifest-selected 18, v1 declarations 39);
unit verdicts stay 2 `Safe` / 42 provisional `Unsafe` / 137 `Unknown` / 4
data-only and the function histogram stays 150 / 84 / 4,600 with zero status
moves in either direction, and the corpus baseline
`docs/reports/wave23-c-operations/artifacts/` is deliberately **not** re-pinned.
At the close, spec §L21's duplicated `Q050` and `Q063`–`Q064` rows — which the
wave's own merges made worse, one stale copy each carrying pre-wave-22 text —
were deduplicated to one current row apiece with no row lost, closing
`L21-DUPLICATE-ROWS` as a documentation repair. Measured close gate: **3,776
workspace tests, 0 failed, 8 ignored**, `fmt` and `clippy` clean, both
reevaluation probes exit 0, **193 Python tests**, **99 Lean audits** (95 → 99),
122 provider tests carried unchanged with `qcore-schemas` still 14.0.0 and no
package rebuilt. Nothing is promoted: no raw Cycle 19 unit is promoted, no
function moved status, every provisional `Unsafe` stays a provisional claim and
never an upstream defect claim, and full language conformance stays false.
Evidence: `docs/reports/native-library-source-binding/README.md`,
`docs/reports/pipe-drop-stage/README.md`, `docs/reports/string-slice/README.md`,
`docs/reports/compose-event-lines/README.md`,
`docs/reports/native-library-header-decoration/README.md`,
`docs/reports/aom-destroy-siblings/README.md`.

Native observation repair (2026-09-11): native sequential, extern, task, mutex, pipeline and CLI checks explicitly select typed diagnostic sinks after quiet emission, preserving their reference comparisons; CT03/CT17 retain both quiet and instrumented sanitizer controls, and example tests distinguish source-I/O projections from their explicitly published traces. Quiet extern C now includes `<stddef.h>` for `NULL`; mutex observations retain read endpoints and complete declared growth records; two task-emitter Clippy errors are repaired without changing emitted bytes. The compiler facade exposes the existing diagnostic adapter so the CLI keeps its established dependency boundary. F02's instrumented-C pins are deliberately renewed with fresh native causal-relation evidence beside the frozen archives. The Windows task-observation companion now applies that same explicit sink and the same instrumented-C pins; all four native MSVC thread tests pass, including the owned-error and read-loan reference comparisons. The full workspace run passed 3,579 tests with 8 ignored and one temporary CLI layering failure; removing that dependency and recompiling yielded passing reruns of all 31 bitwise-condition, 11 layering, 4 F02 and 13 CLI tests, covering 3,580 distinct workspace tests with no unresolved failures. Formatting, Clippy, both reevaluation probes, 193 Python tests and 95 Lean audits pass. Providers and the frozen wave-23 corpus are carried unchanged, with no new package or corpus run. Evidence and the explicit full-run/rerun distinction: `docs/reports/native-observation-repair/README.md`.

Windows quiet-event test repair (2026-09-11): the `std.event` null-sink regression fixture now removes its logger import and `log_level` statement without assuming Unix newlines, so a CRLF checkout tests quiet emission instead of accidentally retaining the explicit stdout adapter. The focused `std_event` integration suite passes unchanged on the emitted-program contract; the Windows system-trace harness also spells its optional fixture list as an explicit branch for the current Clippy gate, and the Unix-only extern native-run helper is no longer imported into the Windows test build. No compiler semantics, verdicts, provider packages or frozen corpus artifacts change.

Quiet-emission and explicit-system-trace increment (2026-09-11): the C11 backends now retain sequential, extern, task, mutex and event observations behind one typed six-level diagnostic logger whose ordinary sink is null, while `log_level LEVEL;` (gated by `std.logger`, recorded and rechecked, and carrying `Io`) explicitly installs the maintained hosted stdout adapter; source `log` remains a separately composed stderr stream, `Streams` now models stdout/stderr pointwise composition, and the event, thread, pipeline, pipe, declared-extern and branching examples explicitly select `log_level trace;` and publish native-checked transcripts, while unsafe linkage demonstrates caller-authored trace markers without presenting an unchecked return as a modeled value. The English normative amendment and implementation report are `docs/design/QUIET_EMISSION_STREAMS_AMENDMENT.md` and `docs/reports/quiet-emission-streams/README.md`; Rust/Lean coverage pins the quiet default, six levels, import/effect/recheck rules, reactor-projection neutrality and stream algebra. Targeted validation passed: `cargo check --workspace`, `cargo test -p qcore-compiler --no-run`, `cargo test -p qcore-compiler --test std_event` (27), `cargo test -p qcore-compiler --test system_trace_examples_windows` (5 native MSVC transcript tests), the focused `std_modules` log-level test, `cargo fmt --all -- --check`, `lake build QCore.StdIo QCore.StdEvent`, and direct `lake env lean AuditStdIo.lean` / `AuditStdEvent.lean` under elan's pinned Lean 4.33.1 toolchain; both new logger theorems report no axioms, and all other reported axioms remain within the allowed `propext` / `Classical.choice` / `Quot.sound` surface. At the user's direction the full standing gate, clippy, reevaluation probes and all-audit runner were not run, so this increment makes no full-gate claim; providers and the frozen Cycle 19 corpus were untouched.

Wave 23 fixed scalar C queries (2026-09-11): the existing-C gateway admits
typed scalar `sizeof`/`_Alignof` queries in six expression positions and scalar
assignments/initializers. Operand markers never determine values or CFG edges;
assignments forget old scalar facts, and sibling mutations, VLA/opaque operands
and resource accesses retain their refusals. Six new regression tests and nine
Lean theorems cover admission, erasure and scalar-fact forgetting. The pinned
185-unit BEFORE/AFTER comparison removes 27 unsupported diagnostics across
seven units, with no verdict changes, no added diagnostics, and leaks unchanged
at 272; the other four artifacts are byte-identical. The closing gate passes
3,575 workspace tests (8 ignored), 193 Python tests, 95 Lean audits, both probes,
fmt and clippy; 122 provider tests are carried from the unchanged prior package
checkpoint. The shared branch's `1a7846dd` includes the implementation; the
closing evidence records the recovered workspace gate and measured baseline.
This is a bounded first gateway slice, with fixed objects, scalar RHS/mutation
lowering and the other gateway families still named follow-ons. Evidence and
boundaries: `docs/reports/wave23-c-operations/README.md`.

Windows native-event increment (2026-09-11): `EVENT-V1-WINDOWS-AFUNIX`
replaces `std.event`'s unconditional Windows loopback construction with the
pinned Libevent 2.2.2-alpha shape: compile-time `<afunix.h>` detection,
run-time `AF_UNIX`/`SOCK_STREAM` probing, a UTF-8 pathname-backed
listen/connect/accept pair whose connector is separately bound for peer
validation, attempted rollback on every pre-publication failure, and the
existing AF_INET loopback route only for an unsupported family/type/protocol
probe. Other probe failures refuse. The source surface, reactor/custody models, certificate,
diagnostics, model identities and observation bytes are unchanged. The
26-test `std_event` target now runs on Windows through the shared native-C
harness and forces AF_UNIX, explicit loopback and no-header fallback builds;
all three reproduce the reference trace, while an intentional early failure
checks ordinary-exit pathname cleanup. The same harness repairs
`bitwise_conditions`' Windows native cross-check, which previously tried to
launch an absent Unix `cc`, and the two CRLF-sensitive fixture mutations are
newline-agnostic. Evidence: `docs/reports/windows-std-event-af-unix/README.md`.

Windows branch-arm gate repair (2026-09-11): the three native observation
comparisons in `branch_arms` now use the established MSVC harness on Windows
instead of trying to launch the absent Unix `cc`; non-Windows hosts retain the
existing ASan/UBSan/LSan route. The emitted programs, reference observations,
branch semantics, diagnostics and fixtures are unchanged. The three exact
formerly failing tests and the complete 35-test target pass on Windows.

Windows logical-connective gate repair (2026-09-11): the native observation
comparison in `logical_connectives` now uses the established MSVC harness on
Windows instead of trying to launch the absent Unix `cc`; non-Windows hosts
retain the existing ASan/UBSan/LSan route. The emitted program, reference
observation, connective semantics, diagnostics and fixtures are unchanged.
The exact formerly failing test and the complete 25-test target pass on
Windows.

Windows native-library fixture repair (2026-09-11): the two malformed-manifest
rows that remove the library or manifest declaration now use the existing
checked token mutation instead of matching a Unix newline. This makes the
intended `Q039` refusal newline-agnostic without changing the loader, accepted
manifest surface, diagnostics or evidence. The exact formerly failing test
and the complete 16-test `native_library` target pass on Windows.

Windows workspace newline gate repair (2026-09-11): a complete
`cargo test --workspace --no-fail-fast` sweep exposed four remaining failing
targets. Manifest-header and composed-I/O fixture mutations now match semantic
tokens instead of Unix newlines, README fenced-block extraction normalises
checkout line endings, and the task/I/O emission comparison normalises both
sides before comparing synthesized lines. Unix-only native-evidence imports
are target-gated, removing the two Windows warning clusters found by the same
sweep. No loader, source-language, emission or diagnostic semantics changed;
all four repaired targets and the complete workspace sweep pass on Windows.

Post-IF-ARMS gate repair (2026-09-11): the `bitwise_conditions` suite now pins
the existing `Q044` refusal for an arm-local check-time constant binding on its
concrete ground — the binding has no position after the join — rather than on
the superseded pending-owner label. The same gate run narrowed the
`execution_quota` fixture's script helper from Unix to Linux, matching every
call site and restoring warning-free macOS clippy without changing the
provider assertions. The refusal code, span, accepted language, emitted C,
corpus baseline and headline counts are unchanged; the recorded load-sensitive
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets` row remains unchanged
and passes in isolation.

<!-- migrated-section:end -->
