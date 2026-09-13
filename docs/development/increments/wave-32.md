# Wave 32 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

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
