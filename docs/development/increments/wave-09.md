# Wave 9 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave-9 delegated increment (2026-09-09): six bounded follow-on cards landed
through delegated worktrees with per-card reports, merged onto main one at a
time under the standing gate, and closed by one integrator corpus
consolidation; this paragraph supersedes nothing above and each card's record
keeps its own boundaries.
- Raw tokenizer repair (E01-F1-RAW): a `#` that is not the first preprocessing
  token of a logical line is no longer a directive introducer (C11 phase-2/
  6.10 semantics; stringization and pasting survive); the raw scan and the
  wave-8 enumeration now agree, with rdkafka.c's absent_from_raw_scan moving
  146 to 15. The verdict surface does not move. The 15-row residual is a
  second, distinct cause — conditional-arm brace imbalance — named
  E01-F1-RAW-COND and pinned, not repaired. 31 audited Lean theorems
  (`QCore.RawDirectiveLexing`) with an exhaustive two-sided lexing checksum
  tie. Evidence: `docs/reports/remaining-e01-f1-raw/README.md`.
- Aggregate stores (O04-B08): a conjunctive typed admission for storing a
  tracked resource into a field of a live, body-acquired aggregate
  (`ic->chunk = chunk`) as a field-link custody edge — deliberately shallow
  (P-B08-SHALLOW: escape carries live linked children, release does not
  cascade, overwrite never discharges) — plus a bounded null-test branch form.
  C5 discharges: five of six constructor exits settle, and only the
  out-of-scope success exit remains, gated on O04-B08-READ-BOUND-BASE. A
  C02-P fail-closed pin caught and corrected the first implementation rather
  than being weakened. Corpus diagnostics move fail-closed in 38 units with
  zero verdict movement and leak unchanged at 271. 42 audited Lean theorems
  (`QCore.AggregateStore`). Evidence:
  `docs/reports/remaining-o04-b08/README.md`.
- Guard-local join (T03-GUARD-JOIN): the soundness argument closes only for a
  rule narrower than proposed — the joined subtree must name no mutex at all,
  making deadlock structurally impossible rather than analyzed — landed as
  the opt-in `thread scope c under g` admission (eight checked conditions)
  with zero deleted checker lines, no diagnostic change, and 24 new Lean
  theorems. The T03-F asymmetry (a child ending its parent's borrow) is
  refused by G7. CT28/CT32 cascades verified unchanged. Evidence:
  `docs/reports/remaining-t03-guard-join/README.md`.
- C-twin CT lane (CT-CTWIN): the reserved `legacy_c` route becomes real — the
  C twin runs through the actual gateway with the whole verdict pinned
  (unit result, verbatim reason, complete function map, ordered diagnostic
  inventory) and tool-gated stage records. CT43 and CT45 map; CT40/CT41/CT42
  stay unmapped on measured invisibility/vacuity/loader-refusal grounds. CT
  coverage moves 28/50 to 30/50; sanitized-native stays 3 of 30 and proof
  0 of 30. One admitted mutation (M20, the inert declared-entry check) is
  recorded and asserted rather than hidden. Evidence:
  `docs/reports/wave9-ctwin/README.md`.
- Call-result vocabulary (P04-NEXT-AB): file_attempts travels as the third
  transported source vocabulary (`call_use_summary`) — policy-free, so which
  callee the contract counts never leaves the core — with byte-identical
  in-process parity on all five bodies and a 122-row mutation matrix, all
  refused. The dependent pair is corrected by measurement into P04-NEXT-B1
  (no contract-side referent: the vocabulary must be declared first) and
  P04-NEXT-B2 (the symbolic-replay fourth vocabulary);
  PrerequisiteIdentity/revalidate_prerequisite land with no production
  consumer yet. `qcore-schemas` is 8.0.0; ChunkIO package 0.7.0; 28 audited
  Lean theorems (`QCore.CallUseAdmission`). Evidence:
  `docs/reports/remaining-p04-next-ab/README.md`.
- Generator and reason residuals (GEN-REASON): the acceptance-matrix
  generator's every section is now derived, integrator-supplied (via
  `--gate`, else labelled carried) or carried verbatim from the committed
  matrix, with a published section_ownership block — whole-file regeneration
  no longer clobbers later reconciliations. REASON-CODES-B closes on its
  reachable surface: B2 types ChunkSourceResult's Unknown reason
  byte-neutrally, B1 is measured at zero occurrences in the pinned corpus
  with the three reachable conversions named, and B3's sidecar reason_code
  lands with the pinned bytes deliberately left for the re-pin. 15 audited
  Lean theorems (`QCore.ReasonCarrier`). Evidence:
  `docs/reports/wave9-gen-reason/README.md`.
- Corpus consolidation: one integrator measurement on the fully merged tree
  through the canonical pinned route re-pins the 185-unit baseline
  (`docs/reports/wave9-corpus/`): 2 Safe / 42 provisional Unsafe /
  137 Unknown / 4 data-only, zero verdict movement, and every changed leaf
  attributed to the union of the three archived per-card diffs plus the two
  known residue classes (path-literal provenance, package identity). The
  REASON-CODES-B3 tripwire test is flipped per its own design; the matrix is
  regenerated whole-file for the first time under the new generator.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-9 baseline, the contract corpus stays 60 modules and full
language conformance stays false. CT coverage advances to 30/50 mapped, by
executed-stage evidence only. `qcore-schemas` is 8.0.0; `qcore-compiler`
stays 0.3.0. Merged gate at this checkpoint: 1888 workspace tests
(4 ignored), 75 provider tests, 125 Python tests, 48 Lean audits, both
reevaluation examples exit 0, fmt and clippy clean, and all five provider
packages rebuilt and verified. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

<!-- migrated-section:end -->
