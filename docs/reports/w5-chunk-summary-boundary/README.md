# W5 chunk lifecycle and summary boundary

The chunk lifecycle model and its tests move unchanged from the compiler to
`qcore-core`. Both files retain exact byte parity with main `6a66f99`;
`parity.json` records hashes. Compiler module and root exports remain valid.
Core has no runtime or workspace dependencies. Existing Lean model definitions
are unchanged; this relocation introduces no new refinement theorem.

`qcore-schemas` 0.2.0 adds Boolean-field/path and frame summary records. This is
an additive package-version change under the schema compatibility policy;
plugin protocol versions and admitted witnesses are unchanged. Records are
serializable untrusted proposals, with unknown fields rejected. No raw record
can construct the opaque, serialize-only source or frame summary. Read-only
`Deref` exposes inspection without a mutation or promotion route.

`qcore-contract::chunk_contract` now evaluates declared path records without a
provider dependency. The evaluator requires both Boolean initial cases,
consistent write/final values, and rejects a second contradictory write channel
for the modeled fields. Its result is explicitly a policy evaluation under
premises, not a source-import certificate. The compiler compatibility entry
points still require importer-created opaque summaries and issue a separate
private-constructor certificate. All four inherent certificate accessors are
preserved, including qualified calls/function pointers. A compile-fail example
checks that a raw record cannot become a source summary.

The three new record tests cover serialization, policy disagreement, missing
or contradictory Boolean paths, contradictory frame writes and unsupported
fields. Existing typed-source importer and source-bound policy tests remain.
This is a boundary split, not full interpreter unification: frame-helper ABI
and Boolean policy vocabulary remain bounded, and provider source import and
full lifecycle witness admission remain open.

A full 185-TU audit using persistent `build/cycle19/monkey-build` inputs has
byte-identical resource diagnostics, source projection and chunk-contract JSON
against the revision-0.3 integration baseline: 3 Safe / 98 provisional Unsafe /
80 Unknown / 4 data-only. The audit preceded the additional raw-record frame
consistency guard and byte-identical concurrency relocation; those do not alter
imported paths. Combined-main validation is recorded separately when later
agent branches join. Raw production units remain unpromoted.

Gate logs in this directory cover the completed workspace test suite, format,
Clippy, both reevaluation probes, Lean build and all 19 Lean audits. They include
the already merged direct Removed custody increment. The first schema-version
run correctly rejected a stale locked standalone conformance fixture; its
lockfile was refreshed (only the schema package version changed), then the
workspace gate was rerun. No failure was waived.
