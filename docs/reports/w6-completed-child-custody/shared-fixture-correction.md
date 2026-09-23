# Correction note: CT14's and CT37's fixture bytes are shared

Card `CT-SHARED-FIXTURE-DISCLOSURE` wrote this note on 2026-09-23. It sits beside
the frozen [report](README.md), which stays unchanged (constraint 21).

This report's commit added four CT14 and CT37 files that are byte-identical to
files in other cases. The report does not say so:

- `CT14/case.qcore` (sha256 `131f635d…5e909c`) is the program in `CT01/case.qcore`
  and `CT12/case.qcore`. This is a `shared_artifact` group. CT14's scenario (the
  child finishes but the parent has not joined) comes from the native adapter,
  not from the program. `PROOF-LEG-BATCH` withdrew a CT14 proof row on this
  identity.
- `CT14/read_before_join.qcore` (sha256 `c775f721…208a7b`) is byte-identical to
  `CT02/case.qcore`. This is a `case_as_control` group: CT14's rejection control is
  CT02's own refused program. The control is a separate file, so an edit to CT02
  does not reach CT14. It silently ends the identity instead.
- `CT14/completion_before_join.h` and `CT37/completion_before_join.h` (sha256
  `bf65bbc7…3d3926`) are the same one-line `#include` shim. Both point to the
  shared `task_worker_allocations/completion_before_join.h`, as §"Evidence and
  reproduction" states. This is a `shared_artifact` group of headers. CT37 is
  proof-bound, so its side is not edited.

The identity is now pinned by
`compiler/tests/ct_corpus.rs::ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure`
against `artifact_identity.shared_fixture_groups` in
`docs/reevaluation/ct_manifest.json`. No mapping status, stage, proof or verdict
changes. [Measurement and decisions](../ct-shared-fixture-disclosure/README.md).
