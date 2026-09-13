# Historical guidance: task identity, source transport and diagnostics

[Development table of contents](../../../../DEVELOPMENT.md) · [History index](../README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
#### R02 terminal task state, adversarial verification (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 841-860 — fuller variant.*

R02 terminal task state, adversarial verification (2026-09-07): V02 re-examined
"a joined child can still emit events" and found no soundness failure; no
production code changed. Evidence was the gap. Four new tests pin the full
terminal-state x event-kind matrix in `crates/qcore-core`: each joined-child
event kind now asserts its exact `TerminalTask`/`UnknownJoin`/`DuplicateTask`
diagnostic and the absence of any authority diagnostic, proving the terminal
decision precedes every custody, guard and permission check. A refused join is
not an implicit completion: exact diagnostic vectors show the child keeps its
authority and duties and stays outstanding, matching CT15 and the native
abort-before-reclamation path. A completed-but-unjoined child's captures stay
in `Custodian::Capsule` and all ten ledger operations reject with
`InactiveTask` in both `Completed` and `Joined` phases, leaving the ledger
unchanged. Nineteen new Lean theorems give the `Lifetime` model per-constructor
terminal rejection, post-join corollaries and refused-join retention, with no
new axioms or placeholders. All 823 workspace tests, 45 provider tests, 43
Python tests, 20 Lean audits and the standing gates pass; CT coverage remains
11/50 and all Cycle19 artifacts are untouched. Evidence:
`docs/reports/remaining-v02/README.md`. Source-to-trace refinement, checked
mutex invariants, loans, general owned graphs and Rust/Lean/native
correspondence remain open.

*Moved verbatim from `CLAUDE.md` lines 615-627 — condensed variant, subsumed by the above but kept verbatim.*

R02 terminal task state, adversarial verification (2026-09-07): V02 re-examined
"a joined child can still emit events" and found no soundness failure; no
production code changed. Four new `qcore-core` tests pin the full terminal-state
x event-kind matrix with exact diagnostics (and the absence of authority
diagnostics), the refused-join direction with exact diagnostic vectors, and the
`Completed`/`Joined` ledger phases rejecting all ten operations with
`InactiveTask` while a completed child's captures stay in `Custodian::Capsule`.
Nineteen new Lean theorems give the `Lifetime` model per-constructor terminal
rejection, post-join corollaries and refused-join retention, with no new axioms.
All 823 workspace tests, 45 provider tests, 43 Python tests, 20 Lean audits and
the standing gates pass; CT coverage remains 11/50 and Cycle19 artifacts are
untouched. Evidence: `docs/reports/remaining-v02/README.md`. Source-to-trace
refinement, checked mutex invariants, loans and native correspondence remain open.

#### Compile-task identity (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 862-887 — byte-identical in CLAUDE.md 629-654.*

Compile-task identity (2026-09-07): the configured audit now records a complete
identity for every accepted compile task — Clang program/version/target, the
declared driver, the compile working directory, the main source path, the
complete ordered argument vector, the dialect, the ordered transported
preprocessing options and analyzed scope, the raw source bytes, the effective
preprocessing input, and named contract/profile/database/rule dependency
digests. A used header, a generated configuration header, include order, an
unread admitted macro option or a contract/profile change now retires the
recorded identity even when the selected `.c` file, the compilation database and
the preprocessed token stream are unchanged. The evidence-v2 task digest already
bound the same inputs; a stale witness is now shown to be refused in both
directions. One configuration is one (working directory, complete command) pair:
repeated identical records are one task, conflicting records reject. A read-only
census records that the pinned Cycle 19 compatibility adapter still chooses
silently between two configurations for `src/flb_input_chunk.c` and among four
for each of 27 Monkey/libevent units; repairing that changes the pinned
artifacts and remains open. Ten new tests; all 829 workspace tests, 45 provider
tests, 43 Python tests, 20 Lean audits and the standing gates pass, and the
configured CMetrics audit still reports one checked function contract, six
Unknown functions and a provisional Unsafe raw unit with identical verdicts,
diagnostics and source digests. The four pinned 185-unit artifacts could not be
re-measured: the upstream Fluent Bit checkout has moved off its pin and the
audit's own pin gate refuses to run. Evidence:
`docs/reports/remaining-g01/README.md`. Header enumeration, compiler
equivalence, build hermeticity and the compatibility adapter's ambiguity remain
open; no verdict changes and no raw unit is promoted.

#### Windows Clang discovery and test portability (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 929-938 — byte-identical in CLAUDE.md 696-705.*

Windows Clang discovery and test portability (2026-09-07): discovery orders
vswhere results newest first and falls back to standard VS 2026 (`18`) and
VS 2022 (`2022`) edition directories, including missing/empty locator cases.
The Windows tests use the discovered executable in compile databases, preserve
CRLF mutation checks, and run POSIX LP64 cleanup fixtures in isolated child
tests through that Clang with an explicit Linux target. Native Windows analysis
retains its native target; no checker rule or production cleanup ABI is widened.
See `docs/reports/windows-vs2026/README.md` for local validation and limits.
Windows validation: 828 tests, formatting, Clippy and both reevaluation probes
pass. The Lean gate was not run because `lake` is not installed on this host.

#### Diagnostic attribution and ownership (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 964-986 — variant carrying the V04/V07 occurrence detail and the spec reconciliation.*

Diagnostic attribution and ownership (2026-09-07): W7 repairs the two real
attribution defects recorded by V04 and V07. `locate` tested an inclusive bound
against an exclusive `FileRange::end`, so a diagnostic at byte 0 of a non-first
input file was blamed on the previous file at a line past its end; all three of
V07's confirmed occurrences now name the offending file, with its controls
unchanged. The unconditional dimension/axis check no longer reports
`Span { 0, 0 }`: slice provenance carries the offending declaration's own span,
and a parse failure in the reconstructed buffer now reports the caller's byte
offsets (V04-F2, fixed rather than deferred). `StaticEnvironment::check_located`
is additive; `check` delegates and the accepted/rejected set is unchanged.
No diagnostic code changed — attribution is not a code change. `Q700`–`Q703` are
reconciled in the spec against the code, which is the authority; `Q020` (task)
and `Q021` (sequential) stay separate by decision because their rejection
coverage differs. `docs/QCORE_SPEC_r0_3.md` §L21 is the canonical per-code
ownership table and §L21.1 specifies a closed reason-code vocabulary for
uncovered obligations, with `environment-unavailable` separated so no
environment failure can be relabeled a source defect. That vocabulary is
specified only: carrying it on `DerivationIncomplete` would change every task
digest, and carrying it on audit-report units would change all four pinned
185-unit artifacts. Three new tests; 890 workspace tests, standing gates and
20 Lean audits pass. The 185-unit audit was not re-run — no prepared Clang build
tree here — and parity is argued by verified disjointness from the C gateway.
Evidence: `docs/reports/remaining-d01/README.md`.

*Moved verbatim from `CLAUDE.md` lines 731-752 — variant carrying the file paths, the empty-`contract { }` asymmetry and the `DerivationIncomplete` constraint.*

Diagnostic attribution and ownership (2026-09-07): W7 repairs the two real
attribution defects recorded by V04 and V07 without changing a single diagnostic
code. `crates/qcore-verification/src/driver.rs` `locate` tested an inclusive
bound against an exclusive `FileRange::end`, so a diagnostic at byte 0 of a
non-first input file was blamed on the previous file; the repair selects the
containing range and keeps a fallback for the separator newline and
end-of-input. `crates/qcore-frontend/src/frontend.rs` no longer emits
`Span { 0, 0 }` for unconditional dimension/axis failures: per-slice provenance
maps back to the offending declaration's original span, and parse failures
report the caller's byte offsets instead of offsets into the reconstructed
buffer. `StaticEnvironment::check_located` is purely additive. When touching
these sites, keep attribution and rejection separate: fixing where a diagnostic
points is not a code change, and changing a code is semantic. `Q020` (task
route) and `Q021` (sequential route) both refuse unenforced source contracts and
must stay separate until a consumer lands — the task gate also refuses an empty
`contract { }` and the sequential gate accepts it, so unifying them would change
rejection coverage. `docs/QCORE_SPEC_r0_3.md` §L21 is now the canonical per-code
ownership table (a semantic pass owns a range; `source_diagnostic` is the
authoritative map) and §L21.1 specifies the reason-code vocabulary, which is
prose only — do not add a category field to `DerivationIncomplete` without
regenerating every task digest, provider package and witness in the same change.
Evidence: `docs/reports/remaining-d01/README.md`.

#### Configured second source set (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 988-1007 — byte-identical in CLAUDE.md 754-773.*

Configured second source set (2026-09-07): the new `chunkio-core` profile
audits 13 pinned ChunkIO translation units through the existing `qcore-audit`
route from `profile.json` and `audit.json` alone, over CMake's unmodified
Clang compile database and the existing resource-contract manifest. No
production Rust changed: the whole `crates/` diff is one new test and one new
test fixture, and `compiler/` is untouched. The run reports 0 Safe /
2 provisional Unsafe / 11 Unknown across 13 units (1 Safe / 2 Unsafe /
120 Unknown functions, 3 unprojected); no raw unit is promoted. Changed pins,
changed audited digests, unsupported compiler flags and absent provider
packages each reject with no report, against both the real inputs and hermetic
fixtures, while unmodeled source forms stay Unknown with keyed diagnostics.
Report and assumption keys match the CMetrics and synthetic-demo routes
exactly, except the `source_contracts` block only a configured provider route
emits; both canonical CMetrics artifacts are byte-identical. Two generalization
gaps are recorded and deliberately not repaired: a source set whose C sources
sit at the checkout root cannot be expressed (`source_roots[].root` rejects
`.`), and a second real CMake project's unmodified database rejects because
only four frontend flags are transported. All 824 workspace tests, 45 provider
tests, 43 Python tests, 20 Lean audits and the standing gates pass. Evidence:
`docs/reports/remaining-g03/README.md`.

#### Replay mutation matrix (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 1008-1040 — byte-identical in CLAUDE.md 774-806.*

Replay mutation matrix (2026-09-07): the ChunkIO Boolean-field capability now
has a systematic 52-row mutation matrix over source-bound witness replay and
cached summaries. All 52 rows reject; no mutation was admitted, so no gate was
changed and no repair was required. Newly pinned: analyzed function scope and a
widened imported function set retire the task and refuse its witness; task,
obligation, request and source-projection version fields are each bound, and an
unsupported evidence version rejects in both directions; mismatched obligation
identifiers reject with the right count, as a foreign vector, as a permutation
and as another function's real identifier, with the core checker independently
refusing a task it did not derive; one witness never covers two obligations,
whether its coverage set claims both or the same certificate is submitted twice
at composition; a forged record rebound to a real donor task digest is caught by
the outermost binding; a well-formed path set that belongs to another function
and satisfies the same contract is refused while the obligation stays Unknown,
as is a proposal that echoes assertion text, the Lean theorem family and the
verdict label instead of computed paths; oversized and degenerate proposals fail
bounded without panicking. Legacy protocol v1 is preserved as non-certifying:
Unknown flows verbatim with no injected rejection, a complete v1 Safe envelope
still refuses terminally and an Unsafe allegation still downgrades. No summary
or evidence cache exists on this route; that is recorded as the current safe
state and pinned behaviorally. Boundaries recorded, not repaired: witness
admission conveys no provider authority, because acceptance terminates in the
core re-evaluating its own sealed import; and this capability's checker returns
its whole task coverage set, sound only because it derives exactly one
obligation and refuses every other task, now guarded by a test. Eleven new
tests; all 861 workspace tests, 46 provider tests, 43 Python tests, 20 Lean
audits and the standing gates pass, and the configured CMetrics audit still
reports one checked function contract, six Unknown functions and a provisional
Unsafe raw unit. No production source file changed, so the pinned 185-unit
artifacts, contract corpus counts and CT coverage are untouched. Evidence:
`docs/reports/remaining-e02/README.md`. Other capabilities' matrices,
out-of-process transport bounds and E01-B1's enumeration gap remain open; no
verdict changes and no raw unit is promoted.

#### Source-filename-sensitive transport (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 1041-1067 — variant carrying the macro/detection detail and the G01 merge record.*

Source-filename-sensitive transport (2026-09-07): the configured audit now hands
Clang the compilation database's own input word from its own directory, so
`__FILE__`, `__FILE_NAME__`, `__BASE_FILE__` and the includer-directory rule for
quoted includes answer as the recorded compile command does; the synthesized
source-parent quote path is removed. Declared source bytes must equal the file's.
`CSourceInput` makes the input identity explicit: an anonymous text rejects
file-name macros that would expand to `<stdin>`, and `__builtin_FILE`/
`__builtin_FILE_NAME` reject in every declared mode because the syntax and typed
AST phases always read preprocessed standard input. Detection scans preprocessed
output, so header-borne constructs are seen too. A CLI regression reproduces a
`Safe` verdict decided by the audit's working directory, where a shadowing header
replaced the real one; it now reports Unknown from any directory. Path spelling is
preserved, not normalized, and no equivalence with a direct one-shot compilation
is claimed. The pinned Cycle19 compatibility route explicitly retains its archived
standard-input projection and both exemptions; its 185-unit artifacts were not
re-measured. Recorded identities bind the declared input under
`qcore.declared-input.v1:`; standard-input units keep historical identity bytes.
Merged with G01's compile-task identity: `CompileTaskIdentity` now also binds
`source_input`, so the transport is not invisible to the recorded identity, and
G01's synthesized-quote-include pin is replaced by an explicit `-iquote` plus
`source_input` assertions. A pre-existing `PluginLimits::additional` build break
on the integration branch is repaired. All 894 workspace tests, 47 provider
tests, 43 Python tests and 20 Lean audits pass with the standing gates. The CMetrics audit is unchanged: one checked
function, six Unknown, a provisional Unsafe raw unit, byte-identical diagnostics,
with ten identity-digest fields moved because 12 `__FILE__` expansions became the
real source path. Evidence: `docs/reports/remaining-g02/README.md`. Compatibility
route migration, single-invocation frontend transport and raw promotion remain open.

*Moved verbatim from `CLAUDE.md` lines 807-824 — variant carrying the `CSourceInput` do-not rules and the `HistoricalStdin` re-measurement rule.*

Source-filename-sensitive transport (2026-09-07): the configured audit hands
Clang the compilation database's own input word from its own directory, so
file-name macros and the includer-directory rule for quoted includes answer as
the recorded compile command does; the synthesized source-parent quote path is
removed and declared source bytes must equal the file's. `CSourceInput` makes the
input identity explicit; anonymous texts reject `<stdin>` file-name expansions and
`__builtin_FILE`/`__builtin_FILE_NAME` reject in every declared mode. When
touching this transport, do not normalize the spelled input path (that changes
`__FILE__`), do not let a declared identity survive onto derived text, and do not
move the pinned Cycle19 compatibility route off `HistoricalStdin` without
re-measuring its 185-unit artifacts. A CLI regression pins that a shadowing header
in the audit's working directory can no longer buy a `Safe` verdict. G01's
`CompileTaskIdentity` binds this transport as `source_input`: keep it there, or a
standard-input run and a declared-file run record the same identity for different
preprocessing. All 894 workspace tests, 47 provider tests, 43 Python tests and 20
Lean audits pass; the CMetrics audit is unchanged at one checked function/six
Unknown with byte-identical diagnostics. Evidence:
`docs/reports/remaining-g02/README.md`.

<!-- migrated-section:end -->
