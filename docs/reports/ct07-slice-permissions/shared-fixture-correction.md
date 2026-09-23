# Correction note: overlapping_write.qcore is CT08's artifact

Card `CT-SHARED-FIXTURE-DISCLOSURE` wrote this note on 2026-09-23. It sits beside
the frozen [report](README.md), which stays unchanged (constraint 21).

`compiler/tests/fixtures/slice_permissions/overlapping_write.qcore` is
byte-identical to `compiler/tests/fixtures/ct/CT08/case.qcore` (sha256
`29f2f013…538fa9`). `CT-WIRE-LANE-2` mapped CT08 onto a copy of this program. This
is a `shared_with_non_ct_fixture` group. The files are independent copies, so
either could diverge silently.

The identity is now pinned by
`compiler/tests/ct_corpus.rs::ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure`.
An edit to either side fails that test until the manifest's reviewed
disclosure changes. [Measurement and decisions](../ct-shared-fixture-disclosure/README.md).
