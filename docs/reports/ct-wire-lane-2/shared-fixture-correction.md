# Correction note: CT08 and CT09 are copies of capability-card fixtures

Card `CT-SHARED-FIXTURE-DISCLOSURE` wrote this note on 2026-09-23. It sits beside
the frozen [report](README.md), which stays unchanged (constraint 21).

§2.2 and §2.3 say where CT08's and CT09's programs came from. They do not say
that the CT fixtures are byte copies:

- `CT08/case.qcore` is byte-identical to
  `compiler/tests/fixtures/slice_permissions/overlapping_write.qcore` (sha256
  `29f2f013…538fa9`), which `compiler/tests/slice_permissions.rs` reads.
- `CT09/case.qcore` is byte-identical to
  `compiler/tests/fixtures/source_record/point.qcore` (sha256 `9a793c46…10f86`),
  which `compiler/tests/source_record.rs` reads.

Both are `shared_with_non_ct_fixture` groups. The copies are independent files,
so either side could diverge from the other silently. §2.2's description of
CT07's control being CT08's case no longer describes the tree:
`CT-LANE-5` remapped CT07, and its directory no longer holds that control.

The identity is now pinned by
`compiler/tests/ct_corpus.rs::ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure`
against `artifact_identity.shared_fixture_groups` in
`docs/reevaluation/ct_manifest.json`. No mapping status, stage, proof or verdict
changes. [Measurement and decisions](../ct-shared-fixture-disclosure/README.md).
