# Correction note: CT10's controls are the route fixtures

Card `CT-SHARED-FIXTURE-DISCLOSURE` wrote this note on 2026-09-23. It sits beside
the frozen [report](README.md), which stays unchanged (constraint 21).

CT10's two admitted controls are byte copies of this card's own route fixtures:

- `ct/CT10/distinct_units.qcore` ≡ `bitfield_concurrency/distinct_units.qcore`
  (sha256 `57c3072d…dc17`)
- `ct/CT10/addressable.qcore` ≡ `bitfield_concurrency/addressable.qcore`
  (sha256 `d91828fc…ac1c`), also read by `crates/qcorec/tests/examples_cli.rs`

Both are `shared_with_non_ct_fixture` groups. They are independent files, so
either side could diverge silently. CT10's own case program has no copy
elsewhere in the tree.

The identity is now pinned by
`compiler/tests/ct_corpus.rs::ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure`
against `artifact_identity.shared_fixture_groups` in
`docs/reevaluation/ct_manifest.json`. No mapping status, stage, proof or verdict
changes. [Measurement and decisions](../ct-shared-fixture-disclosure/README.md).
