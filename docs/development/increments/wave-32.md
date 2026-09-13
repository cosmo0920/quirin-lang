# Wave 32 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

Wave 32, card `CT-WIRE-LANE-4` (2026-09-14): the CT mapping lane, **CT44 and
CT47 mapped, 43/50 → 45/50**, both on the `task` route under the `reject`
verdict mode, and both on the **nearest expressible shape with the scenario's
one inexpressible word published as a boundary** in its own semantic profile —
the CT06 precedent, whose *"same pointer"* scenario already carries *"pointer
values the language gives no type remain outside this route"*. CT47 (*"Raw
pointer is Copy but no transfer/share contract exists"*, expected *Reject
automatic cross-thread acceptance*) is refused by the **transferability
judgment at the crossing construct** — `Q600`, naming the missing contract and
enumerating the three this kind carries — on card `CT47-COPY-CAPTURE`'s wave-29
readiness statement; **all three** enumerated contracts are supplied by accepted
controls that each admit the program (`move` → `ScopedOwnedTasksV1`,
`read … share N` → `ScopedReadSharesV1`, `lend … LOW..HIGH` →
`ScopedWriteLeasesV1`), and the *automatic* half is pinned as the grammar's own
`Q000`, the modeless capture form not existing at all. Its boundary is stated
twice over: no pointer type exists, and **no QCore type is both freely
duplicable and resource-bearing**, so the dangerous combination the scenario
names is prevented by the type system rather than caught by a checker — the
fixture demonstrates the rule that makes it impossible, and *"is Copy"* has a
referent at the **mode** level and not the **type** level. CT44 (*"Stack capture
expires or environment moves before join"*, expected *Reject*) is refused by the
checked task ledger's own `LoanOutstanding { resource: 1, outstanding: 1024,
borrowers: [2] }`, attributed to a **written** `drop buffer;` whose executed
meaning the admitting control demonstrates; both disjuncts are pinned and the
construct is separated from its position **in both directions** (the identical
program with the destruction after the join checks; so does the identical
destruction in the identical position with a scalar capture instead of the
buffer's share). Two of card T00's archived blockers for CT44 are **measured
obsolete and said so**: the available rejection is about the *capture* — it
names the borrowing task — rather than the handle, and *"environment moves
before join"* has the source form T00 recorded it lacked (true at that
checkpoint, where share captures did not lower at all). Its boundary: the
captured storage is a **heap** buffer bound to a parent-frame local, there being
no stack-allocated capturable resource, so `thread scope` being the
scoped-thread construct is what carries the hazard. **Proved vs. observed**
(`STABILITY-REEVAL` discipline): every claim above is *driven* by a named test —
`ct_corpus.rs::ct_manifest_and_fixture_set_agree_and_mapped_cases_execute`
compiles both cases and all eight controls and compares the exact ordered
diagnostic vectors and the accepted controls' profiles — except CT47's
mode-vs-kind reading, which rests on card `CT47-COPY-CAPTURE`'s audited Lean
(`copyOfTheOwnedBufferIsRefusedForItsMode`, `theBoundaryIsDecidedExactlyOnce`);
that is a theorem about the **model's** judgment, not about this fixture, and no
proof stage is read from it. The **constraint-25 trap was measured, not
assumed** (`measure/constraint25_probe.py`, on card `CT-WIRE-LANE-3`'s
precedent): mapping CT47 moves the derived
`newly_decidable_not_mapped_reasons` key set `["CT24", "CT47"]` → `["CT24"]` and
reddens three rows, one of them the committed matrix's, while mapping CT44
touches none of it (CT44 is not in card T00's newly-decidable list); the matrix
is therefore regenerated through its standing generator with the **committed
gate file passed by its relative spelling**, so
`validation.measurement_provenance` cannot be downgraded from
`integrator_supplied (docs/reports/wave31-close/gate.json)` to the
carried-and-unmeasured warning — measured outcome: `validation` **byte-identical**
and every hand-maintained section unchanged except `named_followups`, to which
this lane **appends** its two residuals through that section's own documented
mechanism, editing no existing row. The remaining mover is
`dimensions.named_followups` **39 → 42**: two of the three are those new rows,
and the **third is a pre-existing defect the regeneration repairs** — the
committed count read 39 against **40** rows already on disk, commit `1c7d8e45`
having added one by hand without moving the derived count beside it. Published
as the repair `D02-DIMENSIONS-FOLLOWUP-COUNT`, and measured both ways so the
repair is separable from this lane's additions (39 → 40 unmodified, then
40 → 42). **No retirement
is declared** for either case and that is the mechanism working: both were
archive-held, so no lane record carries a row to retire. Nothing semantic
changed — `git diff` over `crates/` and `compiler/src/` is empty; the only Rust
is harness wiring (`MAPPED_CASES` 43 → 45, `SOURCE_REJECT_CASES` grown by both
ids, `EVIDENCE_FILES` 18 → 19 in both derivations, one new arm in the
`execution_profile` label map) — no checker rule, no diagnostic code, no
lowering, no model, no reserved word, and **this wave ran no arbiter**, so the
Cycle 19 corpus rows and the leak pin are carried by construction. Neither case
carries native, sanitizer or proof evidence — both fixtures are refusals that
reach no backend — so **sanitized-native stays 7 of 45 and proof 3 of 45**: a
grown denominator is mapping, not conformance. Evidence:
`docs/reports/ct-wire-lane-4/README.md`.

<!-- migrated-section:start -->
<!-- migrated-section:end -->
