# Correction note: CT01's fixture bytes are shared

Card `CT-SHARED-FIXTURE-DISCLOSURE` wrote this note on 2026-09-23. It sits beside
the frozen [report](README.md), which stays unchanged (constraint 21).

- `CT01/case.qcore` (sha256 `131f635d…5e909c`) is byte-identical to
  `CT12/case.qcore` and `CT14/case.qcore`. This is a `shared_artifact` group. This
  report's commit (`59f5c097`) created the identity: it rewrote CT12's program
  and added CT01's with the same bytes. CT14's copy arrived later with
  `w6-completed-child-custody`. CT01's F02 proof binding is derived from these
  bytes, so no side is edited. `PROOF-LEG-BATCH` measured this identity and withdrew the
  CT12 and CT14 proof rows because of it.
- `CT01/owned_transfer.h` and `CT12/owned_transfer.h` (sha256 `b5e74449…26cfc63`),
  both added by this report's commit, are byte-identical. This is a
  `shared_artifact` group of headers. This report's archived
  `sanitizer-create-failure.h` and `sanitizer-round-trip.h` carry the same bytes.
  They are frozen evidence and fall outside the pinning check.

The identity is now pinned by
`compiler/tests/ct_corpus.rs::ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure`
against `artifact_identity.shared_fixture_groups` in
`docs/reevaluation/ct_manifest.json`. No mapping status, stage, proof or verdict
changes. [Measurement and decisions](../ct-shared-fixture-disclosure/README.md).
