# Wave 30 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave 30 close (2026-09-13): **six cards** merged one at a time under the
failure-gated chain — `DOCS-COMPACT-4`, `CT24-CLEANUP-PREEMPTED`,
`CT-PROOF-LEG-2`, `REGION-JOINS-LOAN`, `CT07-WRITE-REMAP` and the arbiter
`W4-RECURSIVE-SUMMARIES` — plus one salvage commit recovered during the
worktree sweep (the `check_lean_audits.py` exit-code hardening: a failure now
exits non-zero and a missing `lake` is a hard error naming the elan path).
The wave's head is the **second executed proof row**: CT37, chosen by a
measured survey of all 42 mapped cases — **proof 1 of 42 → 2 of 42** under
the pinned `PROOF_GATED_CASES` gate, its stage evidence superseding card
T00's frozen archive from its own record. CT25's whole scenario became
expressible (`REGION-JOINS-LOAN`: the counted read share expiring at the
join, the adoption `move NAME;`, `T08-COMPOSE-LOAN` **deliberately
discharged** by measurement as a stated `Q600` narrowing; constraint 15m
amended a second time). CT07's evidence was upgraded to the write-lend
two-write-authorities scenario with **no count moved** (42/50, 8 unmapped,
sanitized-native 7 of 42). `CT24-CLEANUP-PREEMPTED` re-attributed the
early-exit refusal from the exit rule's own loan backstop, refused set
proved unchanged (attribution only, constraint 1). The wave ran **one
arbiter** whose honest headline is a measured disposition: the
neutral-element unit was **already landed** (C03-NFR;
`W4-RECSUM-ALREADY-LANDED`), so it lands the design's §4.3 soundness
induction in Lean and **constraint 4 is amended** — the assume-guarantee
prohibition now reads "above the neutral element" with both load-bearing
premises stated; BEFORE byte-identical to the wave-29 baseline, AFTER
byte-identical to its own BEFORE (a determinism measurement), corpus rows
confirmed unmoved (leak **266**, `unknown_alias` 4,344, `unsupported`
49,760, nothing promoted), baseline moved to
`docs/reports/w4-recursive-summaries/artifacts/`. `DOCS-COMPACT-4` honoured
the cadence (44 constraints in/out, five drifts fixed; next pass ~wave 32).
Zero reserved words, zero new codes. Lean audits 118 → **121**. Merged gate
at the close tip: 4,466 workspace tests (8 ignored, `--no-fail-fast`), 122
provider (carried), 193 Python, both probes exit 0, fmt/clippy clean
(`docs/reports/wave30-close/gate.json`); the matrix regenerated through
`build_matrix.py --gate`, hand-kept sections carried, `T08-SRC-O`'s
follow-up annotated with the loan discharge, every tripwire green at the
close tip. Nothing is promoted; full language conformance stays **false**.

W4-RECURSIVE-SUMMARIES (2026-09-13, wave 30): the wave's one arbiter card. Its
brief asked for the design's bounded first unit — the neutral-element
fixed-point start for recursive components
(`docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md` §4.2–§4.3) — and the measured
disposition is that the unit was **already landed**: C03-NFR implemented units
U1/U2/U3/U7 (`legacy_neutral.rs`, the descent to the greatest post-fixed
point, certificate rederivation inside `check_legacy_safety_certificate`,
self-dependent record hygiene), and W4-EXPR-POS plus REACH-GATE carried its
corpus reach to the measured frontier — **1 of 46** recursive components
(`digits10`), closed at the sound-admission frontier (`C03-NFR-REACH`),
recorded here as `W4-RECSUM-ALREADY-LANDED` so the ledger does not
double-count the route. The card lands the one leg the design still owed: the
§4.3 soundness induction itself, `QCore.NeutralPrefixPreservation` (six
audited declarations, Lean audits 118 → **119**) — per-frame validation under
the all-neutral assumption map propagates to **every finite execution
prefix**, completed and still-running inner calls alike
(`neutral_prefix_preservation`, the N-prefix preservation argument), while the
same per-frame discipline for an assumed *effect* constrains no divergent
prefix (`an_assumed_effect_appears_in_no_divergent_prefix`, the measured
`purge`/`mint` counterexample compositionally) and a returned prefix does
discharge it (probe row 17 in the model) — and proposes the constraint-4
amendment recording the neutral-element exception with its two load-bearing
premises (no precondition, so no failure path can be deleted; negative
prefix-closed facts, so soundness is a prefix induction rather than a rank),
for the integrator to adopt (`W4-RECSUM-CONSTRAINT-4`). The arbiter runs, on a
fresh Clang pin: BEFORE byte-identical to
`docs/reports/w29-corpus-contracts/artifacts/` in `diagnostics.json` and
`chunk-contracts.json` (raw and canonical, SHA-256 `8662337d…` /
`c4f438e4…`), AFTER byte-identical to BEFORE in **all five artifacts** — the
card changes no analysis input, so the two counted runs are a run-to-run
determinism measurement of the pinned route — leak **266 unmoved** (no fifth
move), `unknown_alias` **4,344 unmoved**, `unsupported` **49,760 unmoved**,
zero unit and zero function status moves, zero leaves added or removed,
nothing promoted; in particular no recursive component's function flips
`Unknown` → `Safe`, because no new component validates. Contract corpus stays
74; no Rust source, contract, manifest, schema or test pin moved; no new Q
code. Gate: **4,445 workspace tests, 0 failed, 8 ignored** across 218
binaries under `--no-fail-fast` (a first concurrent attempt was killed by the
environment mid-suite at 54 binaries, every completed binary green —
discarded, recorded, not counted), fmt/clippy clean, both probes exit 0,
**Passed 119 Lean audits**, Python 193 with the one disclosed derived-facts
tripwire (`lean_audits: 118 != 119`, constraint 25's usual wave-close
reconciliation). Evidence: `docs/reports/w4-recursive-summaries/README.md`.

Wave 30, card `CT07-WRITE-REMAP` (2026-09-13): the CT lane wave 29's
write-lend card named — CT07's evidence upgraded from the read-loan half to
the real two-write-authorities scenario, **no count moved** (mapped 42/50,
unmapped 8, sanitized-native 7 of 42, proof 1 of 42 all unchanged; evidence
quality, not coverage, constraint 23). The case's artifact is now
`SLICE-PERMISSIONS-WRITE-LEND`'s own published disjoint-write-halves program,
**moved** into `compiler/tests/fixtures/ct/CT07/` with
`compiler/tests/write_lend.rs` repointed at it (the
CT23/`epoch_credit_error_path.rs` precedent), taking `StandardBufferV1` where
the wave-28 case took `StandardStringV1`. The manifest's semantic profile
replaces the read-loan/retained-write boundary sentence with the stronger true
one — **two write authorities over disjoint ranges of one buffer, both held by
one sequential entry's bindings**, interleaved stores each admitted because
each goes through the loan that owns its byte, and the remaining boundary
stated: **not two threads**, `WRITE-LEND-TASKS` the residual. The bounds proof
moves to the lend surface's own `Q053` in both layers (the lend's range,
`[Q053]`; the loan's own index, `[Q054, Q053]` — the IR's rebased containment
beside the route's bound), the disjointness proof to the `Q054` exclusivity
refusal of the one-shared-byte control, and the wave-28 read-loan case is
**kept as the accept control**, measuring that the weaker configuration stays
admitted beside the stronger one; the wave-28 `Q078`/`Q077` controls are
dropped as no longer earning rows here (the former is CT08's own case, still
pinned there). The sanitized-native leg is the same one gated
`run_sequential_sanitized` run, now of the write program's own emitted C, with
ASan trapping the write direction (a displaced-pointer store's off-by-one is
an out-of-bounds **write**). Record mechanics per constraints 21/23/25:
`docs/reports/ct07-write-remap/ct_stage_evidence.json`'s CT07 entry declares
`supersedes` of the wave-28 `ct-wire-lane-2` record — whose bytes are
untouched — the new record is appended to both `EVIDENCE_FILES` lists
(`ct_corpus.rs`'s pinned array grown 14 → 15 deliberately, constraint 5) and
to `acceptance.json`'s `derived_from`, and
`scripts/ct_stage_summary.py --write`/`--check` exit 0 at mapped 42 /
unmapped 8 with no generated field moved; no proof row, `PROOF_GATED_CASES`
entry or CT01 evidence touched. Zero new codes, zero reserved words, no crate
source changed. Report `docs/reports/ct07-write-remap/README.md`.

REGION-JOINS-LOAN (2026-09-13, wave 30): the loan half of CT25, giving *loan
expiry* the referent the wave-29 readiness statement said it lacked — CT25's
whole scenario (*"cleanup destroys owner after joins and loan expiry"*,
expected Accept) is now expressible and accepted end to end. Two admissions,
each composing unedited checkers: **read-loan captures** on
`source.epoch.tasks.v1` (`read PLACE share N` at the spawn; the place-free
`read;` plan clause), **deliberately discharging `T08-COMPOSE-LOAN`** because
its stated premise — a residency shared with the lender that the flat
accounting cannot represent — does not hold on this route, where the lender
is always the creator (workers are leaf functions) and creator-scope bytes
are outside the epoch allocation (`REGION-JOINS-EPOCH-AXIS`, re-measured:
certificates byte-identical with and without the loan); and the **adoption**
`move NAME;` in the composed region body, taking a live entry owner's cleanup
duty into the sealed pack — an acquisition from the pack's side, erasing
completely (the owner's acquisition already stands in the entry's text) with
the discharge's `drop NAME;` written into the blanks after every join.
"After joins and loan expiry" is then three unedited rules composed: the
share returns at the join (task ledger), joins precede the region (wave 28's
`Q600` span rule), and the erased release therefore follows the expiry — while
a destruction under a live share refuses at the ledger's own `LoanOutstanding`
(`Q600`), measured as a written drop and as a region moved above the join. An
adopted non-buffer refuses at the task route's own `Q500`; an acquisition
beside an adopted owner keeps `REGION-JOINS-MULTI-OWNER` (`Q600`). An
acquired owner's name stays the region's own; an adopted owner's name must be
the entry's own (`Q302` both ways). Zero new codes (one `Q302` widening, one
stated deliberate `Q600` narrowing, §L21), zero reserved words (census
vacuous: `read`/`move` already clause words), no model, driver, schema,
manifest, matrix or corpus file touched; the wave-29 `CT25/` fixtures stand
unedited. Rust: 11 new end-to-end tests (`compiler/tests/region_joins_loan.rs`,
hand-written-twin byte comparison and native counting-allocator run included)
on 7 fixtures under `compiler/tests/fixtures/epoch_credit/CT25_LOAN/`, 3 new
frontend unit tests (24 total); Lean: `QCore.RegionJoinsLoan`, 16 audited
theorems consuming `ReadLoan`/`EpochSourceLowering`/`RegionJoinsDuties`
unedited, audits 118 → **119**. Constraint 15m amendment proposed in the card
report §2 for the integrator. Evidence:
`docs/reports/region-joins-loan/README.md`.

Wave 30, card `DOCS-COMPACT-4` (2026-09-13): the fourth recurring
documentation compaction over `AGENTS.md`, `CLAUDE.md` and `README.md`, on the
cadence `CLAUDE.md` schedules (precedents DOCS-COMPACT, DOCS-COMPACT-2,
DOCS-COMPACT-3; next due around wave 32), with waves 27–29 as its material.
Every headline number was re-read from its source and the ledger's wave-29
figures all agree (74 contract modules, 118 Lean audits, 4,445 workspace
tests, CT 42/50 mapped with sanitized-native 7 and proof 1 of 42, 39 named
follow-ons, leak 266 / `unknown_alias` 4,344 / `unsupported` 49,760 at the
`w29-corpus-contracts` baseline). Five drifts were found and fixed, none a
close error in the ledger itself: the repository map still said 73 contract
modules; constraint 25's aside still said "a live 73"; constraint 23 still
carried the wave-26 CT numerators ("5 of 36 and proof 0 of 36") as if current;
the diagnostics-table summary rows for `Q051`–`Q057`, `Q077` and `Q302` were
behind §L21's recorded wave-28/29 condition widenings
(`SLICE-PERMISSIONS-WRITE-LEND`, `REGION-CREDIT-JOINS`,
`REGION-JOINS-DUTIES`); and constraint 14's census ledger and the Q-space
note stopped at wave 27, while `CLAUDE.md`'s platform records lacked
`WRITE-LEND-WINDOWS-RUN` beside its wave-27 sibling. The standing-constraint
inventory is **44 before, 44 after — zero dropped, zero renumbered** (41
bodies byte-identical; 14, 23 and 25 reworded, each preserving every count,
keyword, census figure and rejected candidate), six hard invariants
byte-identical, and no evidence pointer, residual name or diagnostics code
lost (verified mechanically; one residual name gained,
`WRITE-LEND-WINDOWS-RUN`, and constraint 14's pointer list now also cites the
three wave-28/29 census reports, which 15h and 15m already cited). Constraints 15h and 15m, both superseded
and amended since DOCS-COMPACT-3, needed no further reconciliation: no
sentence in the three files still states the old "no write lend" or
"duty-free pack" premises outside the amendments themselves. `README.md` was
refreshed rather than compacted: records, the write lend, the ranged
`borrow_utf8`, the quantitative error region (alone and composed after
joins), the epoch error arm and the gateway's original-`file:line`
presentation. Net sizes moved +2.9% / +4.0% / +7.5% (`AGENTS.md` 92,530 →
95,189 B, `CLAUDE.md` 7,752 → 8,063 B, `README.md` 10,441 → 11,223 B) — this
pass was synchronisation-heavy, the accretion being exactly the waves-28/29
material the files had not yet absorbed, offset only partly by trims (the
Q-space note's per-wave runs merged, a duplicated baseline pointer removed).
`EXAMPLES-STYLE-SLICE-FANOUT` remains an open user decision, flagged in
`examples/README.md` and untouched. No crate source, no spec, no frozen
archive, no manifest and no matrix file changed; corpus-neutral by
construction. Evidence: `docs/reports/docs-compact-4/README.md`.

Wave 30, card `CT-PROOF-LEG-2` (2026-09-13): the **second proof-stage
capability**, on the CT01 precedent — delivered without flipping the row: the
manifest, the acceptance matrix, `acceptance.json` and `ct_corpus.rs` are
untouched, proof still reads 1 of 42, and the exact proposed flip terms
(manifest record, `PROOF_GATED_CASES` row, stage-evidence record with its
`supersedes` target — card T00's frozen `coverage.json`, the record currently
holding the case) are in the report's §7
(`docs/reports/ct-proof-leg-2/README.md`). The case is **CT37** (*child
returns allocation and finishes early; allocation remains live in completion
capsule*), chosen by a measured survey of all 42 mapped cases: it is the one
remaining accept-mode case whose **default reference run both derives into
the F02 alphabet and itself exhibits its scenario** — CT03's run carries
writes the alphabet refuses, CT12/CT14's default traces are event-for-event
CT01's own, the failure-arm cases (CT13/CT15) and error-outcome cases
(CT16/CT17/CT20) have no admitted rendering of their defining events, CT49's
ObsReordering sibling has no case transcription, and CT50's address-reuse crux
is invisible to the reference alphabet (each rejection stated with its ground
in the report §1). The Lean side (`proof/QCore/Ct37CompletionCapsule.lean`,
audit `AuditCt37CompletionCapsule`, Lean audits 118 → **119**) proves the
scenario property on the case's own trace terms: a total decision procedure
`dischargesCompletionCapsule` with meaning theorems for arbitrary traces (the
child's whole program order is allocate-then-finish carrying the very
generation its only allocation created; the moved generation is released
exactly once, by the parent, after the join and after the allocation's own
payload is read back out of the capsule), the kernel-checked acceptance of the
new `referenceRun` transcription, the **symbolic closure**
`relate referenceRun observed = 0 → dischargesCompletionCapsule observed =
true`, six named refusal mutants and the transcription pin
`mixTrace referenceRun = 17239871955803798867`. The genuinely new proof work
over CT01: the scenario's defining fact — the child's completion **precedes
the parent's join** — crosses actors, so the procedure carries a positional
clause (`returnPrecedesTheJoin`) and the closure extracts `checkOrder`'s
completion-before-join conjunct from the relation's own order admission,
restating the sixteen-flag list definitionally (`rfl`) rather than editing
`TaskTraceRelation`. The named host-unconditional runner
(`compiler/tests/ct37_proof_leg.rs::ct37_artifact_is_the_lean_proved_completion_capsule_shape`,
plus the six-mutant refusing twin) compiles CT37's own fixture through the
real pipeline, checks the derived events against the transcription event for
event, recomputes the digest with wrapping `u64` arithmetic, runs the same
procedure and requires the production `relate` to admit the artifact's
rendering. Boundaries stated, not implied: **no native refinement is
claimed** (the emitted C stays covered only by the `native_task_scenarios`
observations; the profile's "native refinement remains open" stands), and the
Lean–Rust tie is F02's checked-digest transcription agreement, not a verified
extraction. Axiom surface within `propext`/`Classical.choice`/`Quot.sound`.
`git diff` over `crates/` and `compiler/src/` is empty, zero new codes, zero
reserved words; gate at the card tip 4,447 workspace tests (baseline 4,445 +
2; constraint 16's wall-clock row tripped under external load ~45 and was
re-verified green in isolation at load ~32, both facts recorded), fmt/clippy
clean, both probes exit 0, `Passed 119 Lean audits`, Python 192/193 with the
standing lean-count tripwire disclosed for the integrator's regeneration.
Evidence: `docs/reports/ct-proof-leg-2/README.md`.

Wave 30, card `CT24-CLEANUP-PREEMPTED` (2026-09-13): the preempted-cleanup
refusal CT24 names is now attributable to its own construct. The wave-26 CT
lane measured CT24's distinguishing fixture unwritable — an early exit (`?` or
`return`) inside a thread scope while a child holds a read share of the frame's
owner refuses first on the task obligation, `Q600` `ScopeBusy(5)` bare, so the
refusal was never attributable to a cleanup destroying an owner before the
child's join (`docs/reports/ct-wire-lane/README.md` §3.5). The judgment that
names that construct already existed: the reference execution's exit rule
carries a loan backstop ("return/error leaves resource N lent to an unjoined
child"), recorded since card T04 as redundant because the scope rule preempts
it, its result discarded for the bare debug rendering. On the wave-29 CT47
precedent this card re-emits the preempting refusal **from that judgment**
(`task_execution.rs::scope_exit_refusal` over the factored
`outstanding_lent_resource`, the same reader `exit` keeps): the message keeps
its `ScopeBusy(..)` prefix and now names the owner the preempted cleanup would
destroy and the borrower whose join it precedes. **Attribution-only, and the
weaker half of constraint 1's clause**: zero codes allocated, zero codes
moved, zero conditions widened, same span, same pass, and the refused set is
unchanged — proved, not asserted, by the new attribution relations in
`QCore.TaskExecution` (`attribution_never_moves_the_refusal`: refusal is the
scope obligation's decision alone; `the_preempting_gate_reads_the_backstops_own_judgment`,
`a_named_owner_is_a_lent_frame_resource`, `an_unlent_frame_keeps_the_plain_refusal`,
`ct24s_cell_is_attributed_to_the_destroy_before_join`), registered in the
existing `AuditTaskExecution` (audits stay 118). A busy scope with no lent
frame resource — CT13's shape — keeps the bare rendering byte for byte, the
explicit-drop shape keeps `LoanOutstanding` byte-identical, and joining the
borrower and releasing the owner ahead of the `?` is admitted whole; all four
shapes plus the early-return twin are pinned by the new
`crates/qcore-frontend/tests/cleanup_preempted.rs`, with the two standing
`ScopeBusy` substring pins in `read_loans.rs`/`read_shares.rs` moved
deliberately to the refined messages. CT24 stays **unmapped** — its manifest
and derived-matrix rows belong to a mapping lane (constraint 25); this card
delivers the attribution, the readiness statement and the updated hand-kept
`acceptance.json` reason, with the non-widening stated in §L21's `Q6xx` row.
Zero reserved words, no gateway/contract/audit/provider file touched (corpus
rows carried, leak pin 266 untouched by construction). Evidence:
`docs/reports/ct24-cleanup-preempted/README.md`.

<!-- migrated-section:end -->
