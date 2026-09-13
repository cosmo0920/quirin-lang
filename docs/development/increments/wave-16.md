# Wave 16 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave-16 delegated increment (2026-09-10): seven bounded cards landed through
delegated worktrees, merged onto main under the verified failure-gated chain
discipline — which caught one predicted semantic conflict and forced an
explicit composed-tree reconciliation — and closed by one integrator corpus
consolidation. This paragraph supersedes nothing above and each card's record
keeps its own boundaries. It is also the first increment written under the
compacted convention: `AGENTS.md` and `CLAUDE.md` now carry only the ledger,
the invariants, the standing constraints and the gate, and this file is the
only place per-increment history lives.
- Agent-doc compaction (DOCS-COMPACT): `AGENTS.md` 2,423 lines to 361 and
  `CLAUDE.md` 2,150 to 86, with every unpreserved paragraph moved here
  verbatim by line range and 28 of 28 moved segments byte-verified present.
  It changed **no fact**, but re-read every headline number from its source
  rather than carrying the prose, correcting three that had drifted (the
  repository map still said 60 contract modules and schemas 3.0.0; the
  reevaluation bullet still said CT 39/50 unmapped). The central obligation
  was the constraint sweep: 87 candidate imperative sentences deduplicated
  into 25 numbered standing subsystem constraints in `AGENTS.md`, each
  carrying the card that established it, with six working-practice rules left
  in `CLAUDE.md`. The sweep is regex-driven plus a manual pass — thorough,
  not provably exhaustive — and the complete original text remains in git
  history and, for every unpreserved paragraph, verbatim here. Evidence:
  `docs/reports/wave16-docs-compact/README.md`.
- Ledger and tooling hygiene (HYGIENE-16): three tally-neutral follow-ons
  close and none moves a number. `D02-BUCKETS-STALE` was measured **already
  repaired** — HYGIENE-14 made the counted set structural and CT-LABELS made
  the label derived, so the committed acceptance record is a fixed point of
  its own writer — and the card supplied the regression a tree-reading test
  cannot show: the literal sum-22 shape re-inflated into a scratch record and
  refused by field name. Extending the same check to the companion
  `newly_decidable_not_mapped_reasons` map, in both derivations, immediately
  found one live staleness (the matrix still explained why CT39 was unmapped,
  four waves after wave 8 mapped it), dropped here.
  `CT-LABELS-T00-REVERT` restores card T00's frozen `coverage.json` to its
  pre-CTWIN bytes byte-exactly, including the original missing final newline;
  CT40/CT41/CT42's current labels come from the wave-9 lane through
  CT-LABELS' dual publication and T00's original text is visible beside them
  instead of overwritten — verified neutral three ways, one of them a proof.
  `DIFF-TOOL-CONTRACTS` was measured **worse than recorded**: `contracts.json`
  was not diffed at all, in any archived diff document. The fifth artifact now
  itemizes contract-module adds, removals and content changes, both embedded
  JSON documents and every remaining top-level key; both recorded blind-spot
  cases reproduce on committed bytes and the four pre-existing entries are
  byte-identical to the two archived documents the pre-change tool produced.
  14 audited theorems (`QCore.BucketTally`); Lean judged not applicable for
  the diff tool with the reasoning recorded. One further finding fell out of
  regenerating: `build_matrix.py`'s writer had never emitted the committed
  matrix's byte form, so every "adopt the regenerated matrix" commit
  re-serialised its output by hand; the writer now emits the committed form.
  Evidence: `docs/reports/wave16-hygiene/README.md`.
- Chained tail actuals (NEIGHBOURS-CHAINED-TAIL): the one refusal wave 15
  isolated falls. A tail actual at a **declared**
  `resource_variadic_data_tail()` position that is exactly one admitted
  bounded typed access chain is data (clause T5) — the same predicate, over
  the same declaration facts, that already decides a declared `Borrowed`
  actual, so the position is the only difference and the position is read
  from the callee, never from the actual. Three measured corrections to the
  card premise: the cause was **not** a pointer-typed final field (a
  depth-one *scalar* tail actual refused identically) but premise T2 plus the
  lexical gate firing ahead of it; the requested closure over the derived
  value is structurally unnecessary because a tail actual binds no name,
  while the closure over the **base** is what refuses a released or
  body-acquired root; and the pinned NEIGHBOURS headline moved in the
  *admitting* direction, re-pinned deliberately with all three load-bearing
  negative halves kept and a fourth added. Corpus effect: exactly one leaf
  removed, `flb_input_chunk_create` L19581, the line NEIGHBOURS predicted.
  28 audited theorems (`QCore.ChainedTailActual`); CHAINED-TAIL-NO-LIVENESS
  is recorded as inherited, not created. Evidence:
  `docs/reports/remaining-neighbours-chained-tail/README.md`.
- Opaque aggregate members (ALIAS-AGGREGATE): the twelve `unresolved
  resource identity` residuals NEIGHBOURS left on production
  `flb_input_chunk_create` are measured to have **no aliasing cause at all**
  — each is the reaching state of an earlier refusal, and the twelve have
  four roots, attributed by withdrawal experiments rather than argued. The
  root of eight of them **corrects card O04-B08's own record**: production
  declares `void *chunk`, not the reduced fixture prelude's `struct cio_chunk
  *chunk`, so that card's flagship store never reached production at all —
  `NEIGHBOURS-FIXTURE-PROXY` caught on the statement it was named for. A
  third member class lands, admitted as a store destination and **refused as
  a bound read in every position**, with the exact-tag value premise replaced
  by "a declared record pointer"; the asymmetry is the whole soundness
  argument. `legacy_c.rs` is byte-unchanged, so `C02-P-ALIAS-LEAK-ORDER` is
  undisturbed by construction. The constructor moves 21 to 12 diagnostics;
  corpus reach is five statements in four units, with **all four added leaves
  refusals replacing refusals**. Fourteen new audited theorems across two
  existing namespaces, none retired. Evidence:
  `docs/reports/remaining-alias-aggregate/README.md`.
- Declared/C ABI agreement (LOADER-ABI): a declared contract whose parameter
  or result ABI projections the analysed translation unit's own C declaration
  contradicts is now **withdrawn for that unit** — every call site refuses
  with one of eight named keys and the name leaves the environment the
  derivation seeds — closing the fail-open NEIGHBOURS recorded, where the
  loader checked those spellings against its own table, against nothing else,
  and then discarded them while its admissibility rules were keyed on exactly
  what it discarded. Both sides are read from what a certificate already
  holds, so W4 derivation, W4 validation and certificate rechecking reproduce
  the decision; withdrawal is proved monotone in the refusing direction. The
  sweep found **eight unfaithful declarations**: `flb_sds_create_len`'s `len`
  declared `usize` against C's `int` (the only production finding, corrected
  in `contract/qcore`) and seven fixture declarations, the last of which
  (`flb_error` at a call site's arity against `(const char *, ...)`) closes
  `O04-ARMS-VARIADIC` and was load-bearing for five cards' published fixture
  results. 38 audited theorems (`QCore.DeclaredAbiAgreement`); spec §C04.1
  gains the agreement requirement. **The composed-tree reconciliation is part
  of the record**: the failure-gated chain caught eleven pins, and on the
  composed tree all eleven revert to their pre-LOADER-ABI values — now
  carried by a *faithful* declaration, because CHAINED-TAIL's clause T5
  admits the same statement honestly at the declared data tail the correction
  creates. Neither ground is weakened by the other, four further theorems tie
  the two namespaces, and the two probes composition made undiscriminating
  are retired **by name** rather than left to pass vacuously.
  `O04-TRACE-REACH` still stands: no production claim is restored. Evidence:
  `docs/reports/remaining-loader-abi/README.md` (§10 for the reconciliation).
- Publication custody (CTWIN-PUBLISH): the wave-9 `CT-CTWIN-PUBLISH` hazard
  is reproduced and closed — a store of an owned pointer into a file-scope
  object was `Safe` with no diagnostic, and so was publishing an
  already-released one, because the importer modelled the file-scope object
  as one more local name. The repair **refuses rather than escapes**, and the
  brief's escape proposal is **disproved by measurement**: escape discharges
  the residual, so the publish-without-release body — measured `Unsafe` at
  base — would have been *promoted* to `Safe`. Both directions are withdrawn
  instead, the leak claim retired through the existing unresolved-effect
  mechanism, which is a downgrade of a provisional `Unsafe` and never a
  promotion. Destinations are file-scope objects the body does not shadow
  plus its own `static` locals (added by measurement, not in the brief); the
  value half fires only on an identity the body itself holds, so borrowed
  publication, null stores and scalar stores stay admitted, each argued.
  Corpus reach is **zero**, with a toggled-rule probe measuring *which*
  conjunct makes it so: with the value conjunct disabled, six librdkafka
  units move at real static-storage destinations, so the destination scan is
  not vacuous. `CT-CTWIN-SYNC` is converted from an omission into an asserted
  decision — C11 `volatile` is not a synchronization primitive, so a
  projection deriving no synchronization fact must answer identically, pinned
  on the whole verdict in both directions. 30 audited theorems
  (`QCore.PublicationCustody`); CT40 candidate rows recorded without touching
  the manifest, with `_Atomic` measured *not* load-bearing. Evidence:
  `docs/reports/remaining-ctwin-publish/README.md`.
- Filesystem-close composition (P04-COMPOSE-FS): the fourth composition
  module moves through the process boundary — the first with **two derived
  units in one question** and the first whose **every** prerequisite is
  itself a composition, checked over two different translation units. The
  measured consequence is recorded rather than papered over: this family
  cannot carry the memory family's single-unit base tie, so the unit tie is
  pinned in *both* directions and the backend prerequisite is tied by
  identity, revision and prototype with linker resolution published as a
  premise (`P04-COMPOSE-FS-CROSS-UNIT`). No composed result and no
  prerequisite conclusion travels at any of the three levels; parity with the
  retained in-process route is byte-identical on the serialized certificate,
  and the two routes now call one decision procedure so they cannot drift. A
  118-row mutation matrix, an 18-row provider degeneracy matrix and 9
  request-gate rows all refuse. `nested()` returns a vector because one slot
  could not publish two nested questions honestly; that and three enum
  variants make `qcore-schemas` **13.0.0** with existing wire bytes and every
  existing task digest unchanged. ChunkIO package 0.12.0; 35 audited theorems
  (`QCore.TwoDerivationAdmission`). `unsupported_close` is now composable-now
  and honestly sized out as `P04-COMPOSE-UNSUPPORTED` — a fifth proposal
  record carrying the symbolic other-backend class, not an extension of this
  one. Evidence: `docs/reports/remaining-p04-compose-fs/README.md`.
- Corpus consolidation: one integrator measurement
  (`docs/reports/wave16-corpus/`) re-pins the baseline. The diagnostics diff
  is 4 added / 14 removed across four units and is **triangulated in both
  directions** against the two gateway cards' archived AFTER measurements —
  each was taken without the other's diff — with zero unattributed leaves:
  the union of CHAINED-TAIL's one leaf and ALIAS-AGGREGATE's seventeen equals
  the composed measurement exactly, and the composed set minus either card's
  is the other card's. The composed `flb_input_chunk_create` count is
  **measured at 11** (ALIAS-AGGREGATE's 12 minus CHAINED-TAIL's L19581), and
  it stays `Unknown`. `chunk-contracts.json` is byte-identical; the one
  changed contract module is LOADER-ABI's correction, verified through
  HYGIENE-16's new itemizer rather than by hand; the 36 `source-effects`
  leaves are LOADER-ABI's 33 identity digests plus P04-COMPOSE-FS's three
  `provider_version` leaves, with no verdict, coverage shape or bounds value
  moving; and the `source.json` residue is 14 `__FILE__` literals and their
  derived extents. CTWIN-PUBLISH's, HYGIENE-16's and DOCS-COMPACT's
  byte-identity claims all hold on the composed tree. `named_followups` moves
  25 to 26 and `AGENTS.md`'s standing constraints 25 to 28 (the LOADER-ABI
  withdrawal rule, the fixture-prelude rule ALIAS-AGGREGATE's correction
  forces, and the publication-store withdrawal rule), with constraints 21 and
  22 rewritten because HYGIENE-16 closed what they described. One recorded
  generator defect fired at the regeneration and was handled rather than
  absorbed: `build_matrix.py` drops `newly_decidable_not_mapped_reasons` on
  every run (`D02-GEN-CTREASONS`), so the map is carried forward by a script
  that **refuses** a row describing a case the manifest reports as mapped —
  the staleness HYGIENE-16 found by hand, now caught mechanically at the one
  point a regeneration would lose it. The follow-on stays open: that is a
  guard at the carry, not a derivation.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-16 baseline, with zero unit and zero function status moves and
leak unchanged at 271; the contract corpus stays 67 modules and full language
conformance stays false. CT coverage stays 32/50 mapped. `qcore-schemas` is
13.0.0; `qcore-compiler`, `qcore-verifier-host` and `qcore-verification` stay
0.4.0. Merged gate at this checkpoint: 2,746 workspace tests (8 ignored),
115 provider tests, 161 Python tests, 72 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean in both workspaces, and all five
provider packages rebuilt and verified at schemas 13.0.0 / ChunkIO 0.12.0.
Neither recorded load-flake (`vp04_vp07_vp08`, the confined-fixture write
test) was hit at the consolidation gate; both assertions stay unweakened.
Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

<!-- migrated-section:end -->
