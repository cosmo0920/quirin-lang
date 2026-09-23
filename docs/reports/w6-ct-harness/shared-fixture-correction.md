# Correction note: CT02 and CT12 fixture bytes are shared

Card `CT-SHARED-FIXTURE-DISCLOSURE` wrote this note on 2026-09-23. It sits beside
the frozen [report](README.md), which stays unchanged (constraint 21).

This report mapped CT02 and CT12. Two of their fixtures have since become
byte-identical to files owned by other cases. The report could not have
recorded that, because the identities were created later:

- `CT12/case.qcore` (sha256 `131f635d…5e909c`) is no longer the program this
  report mapped. Commit `59f5c097` (`w6-owned-source-tasks`) rewrote it and
  created `CT01/case.qcore` with the same bytes. `CT14/case.qcore`
  (`w6-completed-child-custody`) followed. This is a `shared_artifact` group.
  CT12's creation-failure scenario comes from its native harness data, not
  from this program.
- `CT12/owned_transfer.h` (sha256 `b5e74449…26cfc63`) matches CT01's header. This
  file came from `w6-owned-source-tasks`, not from this report.
- `CT02/case.qcore` (sha256 `c775f721…208a7b`) is byte-identical to
  `CT14/read_before_join.qcore`, which is one of CT14's rejection controls. This
  is a `case_as_control` group. The control is a separate file. Editing CT02
  does not change CT14's control, but it silently ends the identity between
  them.

The identity is now pinned by
`compiler/tests/ct_corpus.rs::ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure`
against `artifact_identity.shared_fixture_groups` in
`docs/reevaluation/ct_manifest.json`. An edit to either side now fails that test.
No mapping status, stage, proof or verdict changes.
[Measurement and decisions](../ct-shared-fixture-disclosure/README.md).
