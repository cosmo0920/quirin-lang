# Discharge note: §3.1 `CT-SHARED-FIXTURE-DISCLOSURE`

Card `CT-SHARED-FIXTURE-DISCLOSURE` wrote this note on 2026-09-23. It sits beside
the frozen [report](README.md), which stays unchanged (constraint 21).

§3.1 counted 43 distinct artifacts behind 45 mapped cases. That count belonged to
its checkpoint. At 50 of 50 mapped, the cases' `source_fixture` files have **48**
distinct SHA-256 digests. The only shared primary artifact is still the
CT01/CT12/CT14 program (`131f635d…`). The same figure holds after stripping
comments and whitespace.

The discharge is §3.1's second option, in a checked form. `ct_manifest.json` now
carries `artifact_identity`, which records `mapped_cases: 50`,
`distinct_source_fixture_digests: 48` and every byte-identical fixture group
with its digest. `compiler/tests/ct_corpus.rs` recomputes all of it, so the
manifest cannot present one program as three without failing the gate.
[Measurement, decisions and gate](../ct-shared-fixture-disclosure/README.md).
