# Wave 32 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

Wave 32, card `TRACE-ALPHABET-LEASE-BOUNDARY` (2026-09-14): the first card run
under the new `STABILITY-REEVAL` discipline, paying back the honesty debt that
discipline's first method step was written around. Wave 31's integration had
resolved a semantic conflict between `WRITE-LEND-TASKS` and
`CT03-PROOF-ALPHABET` inside `reference_trace` as a union — the Buffer write
encoded, the two range-write-lease events still refused — and reported it
verified on two green suites that never drove the arm; the residual
`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` recorded that as **vacuous
greenness** and named three alternative discharges. Measured first, chosen
after: the arm is **reachable today** — card `WRITE-LEND-TASKS`' own admitted
flagship produces a fourteen-event reference run whose index 2 is a
`BufferLeased` — so discharge (c), *show it unreachable*, is **false**;
`derive` is the only route by which a lease could reach an
`AdmittedCausalOrder`, the other two public routes (`parse_observed_trace`,
`declare`) being unable to express one, the first by its tag table and the
second at the type level; and — the finding the residual did not anticipate —
**dropping the lease events instead of refusing them would have been an
*admitting* mistake, not a lossy one**: the flagship's ten-event lease-free
remainder is a trace `AdmittedCausalOrder::declare` **accepts**, as a causal
order for a run in which two workers held outstanding write authority the order
never mentions. The card therefore took (a) and (b) **together**.
`proof/QCore/TaskTraceLeaseBoundary.lean` (new, registered in
`AuditTaskTraceLeaseBoundary.lean`, **125 → 126** audits, no project axiom)
proves the boundary on the *observation* alphabet where CT03's theorems cover
only the observable one: `succeeds_iff_no_event_is_refusable` (exact in both
directions — no lease slips through on any shape of run, and the refusal blocks
nothing else), `encoding_is_positionwise` with `encoding_preserves_length` (the
formal content of "never silently dropped"), `refusal_points_at_a_refused_event`
with `refusal_is_the_first` (the Rust pin `index: 2` is an instance, not a
coincidence), and `the_dropped_encoding_would_have_been_admitted`.
`compiler/tests/trace_alphabet_lease_boundary.rs` (new, five tests) drives a
real compiled lease-carrying reference run into the arm, with a read-share twin
as the control; each test was shown non-vacuous by **mutating the arm** three
ways — admit-as-loan, silent drop, wrong index — catching 3, 3 and 2 of 5. The
production diff is **documentation only**: the arm's behaviour is
byte-identical, no refusal is weakened, no `Q` code allocated, no word
reserved, and **no arbiter ran** (no gateway, contract, audit or provider file
is touched, so the corpus rows are carried). AGENTS.md gains standing
constraint **15n**. The residual is **fully discharged and retirable**; its
forward-looking half is replaced by `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED` —
whether the observable alphabet should grow to carry leases is **undecided**,
because CT03's method needs a rule shown non-vacuous in both directions and
that half is unavailable while a worker *holds* but does not *exercise* its
lease (`WRITE-LEND-TASKS-EXERCISE`), with the operative consequence recorded as
a measured boundary rather than a defect: write-lend programs sit outside F02
relational comparison by construction, their native evidence being sanitizer
and allocation-count. Evidence:
`docs/reports/trace-alphabet-lease-boundary/README.md`.

`PROOF-LEG-BATCH` (2026-09-13, wave 32) set out to grow the proof axis
**3 of 43 → 5 of 43** by binding CT01's already-proved procedure
`QCore.Ct01OwnedTransfer.dischargesOwnedTransfer` to CT12's and CT14's own
artifacts, writing no new Lean, on `CT-PROOF-LEG-2`'s recorded premise that
their default reference traces are event-for-event CT01's `roundTrip`. The
premise was measured rather than assumed and it holds — for a stronger reason
than the survey recorded, and **that reason withdraws both rows**:
`compiler/tests/fixtures/ct/{CT01,CT12,CT14}/case.qcore` are one
**byte-identical file** (sha256 `131f635d…`), the only duplicate among the
corpus's forty case fixtures and a triple. All three derive the same seven-event
reference run carrying the same kernel-proved digest `13023433800416048247`, so
the domain question answers *not different in any event, and not by
coincidence* — one program, one trace term, one theorem. The proof stage, as all
three landed legs define it, is a function of the fixture's bytes alone, so CT12
and CT14 could publish only CT01's proof with a different path in the `artifact`
field: no new theorem, term, digest, property or refusal, constraint 23's gate
satisfied **vacuously** because "the case's own artifact" cannot distinguish
them from CT01, and `proof 5 of 43` reporting five rows over **three** distinct
artifacts against invariant 2's `(artifact, property, …)` identity. Each case's
own scenario was measured out of reach rather than assumed so: the one scripted
arm putting a creation failure into a reference run refuses at
`derive` with `Violation(UnattributableEvent { index: 1 })` — `CreationFailed`
has `actor() = None`, so no declared order can contain the event CT12 is about —
and CT14's distinguishing content is a native-adapter ordering plus `Q600`
controls, not a trace. The card's deliverable is therefore the withdrawal,
pinned: `compiler/tests/ct12_ct14_proof_reach.rs`, two host-unconditional tests
holding fixture/trace/digest identity and the unattributable-creation-failure
refusal, **tripwires in constraint 24's sense** whose two failure modes are
exactly the conditions under which the rows genuinely reopen (a fixture
diverging; the alphabet growing to attribute `CreationFailed`, on
`CT03-PROOF-ALPHABET`'s deliberate pattern). Nothing semantic changed — `git
diff` over `crates/`, `compiler/src/` and `proof/` is empty; no manifest,
matrix, `acceptance.json`, `ct_corpus.rs` or `EVIDENCE_FILES` edit; no Lean
audit added, so `check_lean_audits.py` is unchanged; **no arbiter ran** and no
corpus row moves. Headline unmoved: proof stays **3 of 43** (CT01, CT37, CT03),
CT coverage 43/50. Verified for the reopening: CT12's and CT14's stages live in
card T00's frozen archive `docs/reports/remaining-t00/coverage.json`, the
`supersedes` target a future flip would name. Follow-ons
`CT-PROOF-CREATION-FAILURE-ALPHABET` (the measured condition that would make
CT12, and CT13/CT15 with it, real candidates) and
`CT-SHARED-FIXTURE-DISCLOSURE` (three manifest rows with three profiles over one
file — legitimate for the per-regime native stages, misleading for any stage
that is a function of the bytes), plus `CT-PROOF-CONTROL-ARM-TIER` for the
deliberate version of the row this card declined to land by side effect. Gate at
the card tip: 224 suite summaries, **4,512 passed / 1 failed / 8 ignored**, the
one failure constraint 16's `RLIMIT_NPROC` row under five concurrent worktree
gates (load to 65) and **re-verified green in isolation**, same binary, nothing
edited; fmt, clippy and both probes clean; `Passed 125 Lean audits`; `scripts/tests`
fully green at 200. Report: `docs/reports/proof-leg-batch/README.md`.

<!-- migrated-section:start -->
## Increment records

<!-- migrated-section:end -->

`DOCS-COMPACT-5` — fifth recurring compaction of `AGENTS.md`, `CLAUDE.md` and
`README.md` (2026-09-13, after wave 31; **no arbiter run, no corpus row
touched**). The standing-constraint inventory is **45 in, 45 out, zero dropped
and zero renumbered** (DOCS-COMPACT-4's 44 plus `C-GATEWAY-WAVE`'s 10b),
verified mechanically along with the six hard invariants, every evidence-pointer
slug, every residual name and every diagnostic code; four bodies were reworded
and each word-diffed — 14 (waves 30–31 appended to the census ledger, the
"does not engage" sentences merged), 15h (a **pure insert**), 23 and 25 (stale
numerators only). Every headline number was re-read from its source and
**eight drifts** fixed: contract corpus 74 → **76**, the gateway's "four"
load-bearing modules → **six** (`legacy_neutral.rs` and
`legacy_joined_provenance.rs`, already named by constraints 4 and 10b), named
follow-ons 39 → **40**, constraint 23's numerator pointer → 7 of 43 and 3 of 43
at the wave-31 close, constraint 25's live corpus → 76, constraint 14's census
ledger and the Q-space note both stopped at wave 29 (now waves 26–31, thirteen
uses on nine bands, three non-instances named), and three diagnostics rows
behind §L21's wave-28/30/31 widenings (`Q020`/`Q021`, `Q302`, `Q600`). Those
last four share **one root cause**, recorded as
`DOCS-COMPACT-5-RECONCILIATION-LIST`: `WRITE-LEND-TASKS`' explicit integrator
reconciliation list (§12) was applied for the headline numbers only, so its
proposed **constraint 15h append** — the range write lease crossing the spawn
boundary, with `ownership::axes_overlap` called rather than re-implemented, the
three asymmetries, and `WRITE-LEND-TASKS-EXERCISE` — had no home in the three
files at all; it is now landed as an insert that changes not one existing word.
The restructure's seams were checked mechanically: all **149** path references
and every `#fragment` across the three files and `DEVELOPMENT.md` resolve, the
sync rule and the increment index agree on "one paragraph, newest first, in the
current record file the index names", the split's one duplicated instruction
(update the index / do not append to the root table of contents) was merged back
into the sync rule, and `README.md`'s pointer for the verdict vocabulary and the
hard invariants was repointed from `DEVELOPMENT.md` — now a table of contents
that carries neither — to `AGENTS.md`. The new `STABILITY-REEVAL` step produced
three findings: the CT ledger row now states CT03's **observed** side beside its
proved one (`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED`, the arm no test drives,
bounded by direction alone), 15h's append carries
`WRITE-LEND-TASKS-EXERCISE` in the card's own words so it cannot read as a
two-threads-mutating claim, and `DOCS-COMPACT-5-EXAMPLES-HOST-GATE` is
**flagged and deliberately not reworded** — measured, `examples_suite.rs` is
`#![cfg(any(target_os = "linux", target_os = "macos"))]`, so the unconditional
"built and run under the sanitizers" sentences are a scope omission already
covered by the `*-WINDOWS-RUN` residual family. `CLAUDE.md` gains
`WRITE-LEND-TASKS-WINDOWS-RUN` and records the next pass as due around wave 34.
Byte deltas: `AGENTS.md` +5,379 (+5.2%), `CLAUDE.md` +419 (+3.9%), `README.md`
+259 (+2.2%) — growth again, and for DOCS-COMPACT-4's recorded reason. Gate at
the card tip: **4,511 workspace tests (0 failed, 8 ignored, 223 binaries,
`--no-fail-fast`), no constraint-16 row tripped**, fmt/clippy clean, both probes
exit 0, `Passed 125 Lean audits`, 200 Python tests OK. Nothing is promoted, no
count moved, and full language conformance stays **false**.
`docs/reports/docs-compact-5/README.md`.

`CT-COMPOSED-NATIVE` (2026-09-13) supplies the runner that CT22's and CT25's
own `unassessed` rows had been naming since wave 29: `run_composed_native`
compiles each admitted `composed_epoch` case's **own** emitted C with the
platform C compiler under ASan/UBSan — plus LSan on Linux, where alone it
runs — and compares its standard output **byte for byte** with
`render_task_observation` of the reference execution of that same module's
**own checked task half**, the interpreter's, since a composed module
publishes no reference observation of its own. Both cases pass, and CT25's
transcript carries the scenario in the open — loan at the spawn, return at the
join, and only then the `QCORE_BUFFER_DROP` the discharge decided, with LSan's
silence observing that the buffer really is reclaimed, which is the sense in
which *cleanup destroys owner* is now **executed** rather than decided in a
model. It is an **observation, never a proof**: one scheduling of one program
on one host with one compiler on the all-success script, saying nothing about
inputs, schedules or hosts it did not take, nothing about the credit
quantities (the region and the plan erase from emission, so credit 5, effort 2
and the abandoned remainder 3 remain check-time folds no byte of the run
touches), and nothing that moves either `proof` stage off `unassessed`. Every
arm was **falsified before it was believed** — the pinned transcript, the
native byte comparison and LSan each made to fail on this tree and restored —
because an arm no test drives is a stated boundary rather than a verified
rule. CT23 is deliberately excluded and the harness asserts the exclusion: it
is the route's refusal, never reaches the backend, and keeps its
`not_applicable` rows for that stated reason instead of being swept in.
**No stage record and no CT number move in this card**: mapped stays 43/50,
sanitized-native **7 of 43**, proof 3 of 43, and the three rows the runner
backs still read `unassessed` — the manifest now *understating* what the
harness executes, which is the safe direction and never the claiming one,
because a status is checked against the manifest for exact equality and the
derivation and the rows must move in one change the card's boundary reserves
to the integrator. That coupled edit is published verbatim in
`docs/reports/ct-composed-native/README.md` §4 (harness arm,
`SANITIZED_GATED_CASES` 7 → 9, the six manifest rows, `stages_executed`,
the `acceptance.json` regeneration), residual `CT-COMPOSED-NATIVE-ROWS`;
applying it reads sanitized-native **9 of 43**. `RUNNER_REGISTRY` grows by one
deliberately, `COMPOSED_NATIVE_CASES` lands pinned and fail-closed in both
directions, and `crates/`/`compiler/src/` are untouched — no checker rule, no
diagnostic code, no reserved word. Nothing is promoted: the wave ran no
arbiter and this card did not run the corpus, so the Cycle 19 rows are carried
(2 Safe / 42 provisional Unsafe / 137 Unknown plus four data-only, contract
corpus 76 modules, `qcore-schemas` 14.0.0). Residuals named:
`CT-COMPOSED-NATIVE-ROWS`, `CT-COMPOSED-NATIVE-SCRIPT` (the failure edges have
no native run), `CT-COMPOSED-NATIVE-APPLE` (the macOS arm is written and gated
but unrun here, and makes no reclamation claim by construction).
