# Correction note: CT04's control is CT03's program

Card `CT-SHARED-FIXTURE-DISCLOSURE` wrote this note on 2026-09-23. It sits beside
the frozen [report](README.md), which stays unchanged (constraint 21).

The §3 fixture table describes `CT04/write_after_joins.qcore` as "**control**:
same write after both joins". The file is also byte-identical to
`CT03/case.qcore` (sha256 `7c75577b…634be6`). CT04's accepted control is CT03's
own admitted program. This is a `case_as_control` group. The report does not
say so, and wave 35's `CT-CTWIN-LANE` found it
(`CT-FIXTURE-CROSS-CASE-DUPLICATES`).

The reuse is deliberate and is kept: another mapped case's admitted program is a
strong control. CT03 is bound to its model proof through the F02 trace/digest
relation, so neither side is edited. The two files are separate. An edit to
CT03 does not change what CT04's control tests. It silently ends the identity
instead.

The identity is now pinned by
`compiler/tests/ct_corpus.rs::ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure`
against `artifact_identity.shared_fixture_groups` in
`docs/reevaluation/ct_manifest.json`. No mapping status, stage, proof or verdict
changes. [Measurement and decisions](../ct-shared-fixture-disclosure/README.md).
