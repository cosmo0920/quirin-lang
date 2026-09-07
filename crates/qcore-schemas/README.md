# qcore-schemas 4.0.0

The published, versioned schema surface of the QCore verifier ecosystem
(layer L0 of `docs/GENERALIZATION_PLAN.md` §3.1). This crate holds the data
types that cross the core/provider boundary: the verifier-plugin protocol v1
manifest/request/response types, the resource-contract data model, the
library-mechanism vocabulary, and source/build identity records.

## Rules

- **Serde types only, no analysis logic.** Everything here must be
  serializable; format validation of a schema value (e.g.
  `PluginManifest::validate`) belongs here, semantic analysis never does.
- **Providers depend on this crate only.** Provider crates and profile data
  are validated against this surface; they never link core internals. The
  core never depends on any provider.
- **No workspace dependencies.** This crate must not depend on any other
  crate in the workspace (dependencies point strictly downward); a test in
  `qcore-compiler` (`tests/workspace_layering.rs`) enforces this.

## Compatibility policy

The crate is semver-versioned independently of the compiler:

- **Breaking (major bump, a compatibility event with the same discipline as
  protocol-version negotiation):** removing or renaming a public type, field,
  or variant; changing a serde representation (field name, tag, transparent
  wrapper); tightening a validation rule so previously valid data is
  rejected; changing the meaning of an existing field.
- **Additive (minor bump):** new types, new optional fields with defaults,
  new enum variants on non-exhaustive wire enums, new capabilities announced
  through the existing negotiation.
- Data files (contracts, profiles) validated against this crate are *trusted
  assumptions*, not verified facts; certificates must record which ones were
  assumed (plan invariant 6).

## Boolean path records (introduced in 0.2.0)

`chunk_summary` adds portable Boolean-field and frame summary records. These
records deserialize untrusted proposed paths. They carry no source-import
certificate, and successful policy evaluation proves only the supplied paths
under declared premises. The compiler's source importer retains opaque,
serialize-only wrappers; raw records cannot construct those wrappers or the
source-checked compatibility certificate.

The original additive path-record surface introduced crate version 0.2.0.
That addition did not change plugin protocol version 1 or evidence versions
1 and 2. Later source-program exchange requires the compatibility event below.

## Source-program compatibility event (1.0.0)

Version 1.0.0 makes the breaking Rust/schema transition explicit. During the
source-provider work, `BooleanFieldSourceInput` changed from a required
`summary: ChunkSourceSummaryRecord` field to required `schema_version`,
`function`, `source_revision`, `field`, `program`, `dependencies`, and
`profile_assumptions` fields. It now sends typed statements and expressions
without expected execution paths. Reusing 0.2.0 for that shape violated the
major-version policy above; all workspace consumers and standalone providers
now require 1.0.0 and their lockfiles record it.

Consumers constructing the old input must migrate to the version-2 statement
input. Trusted callers obtain it from the opaque C gateway import's
`source_input()` method. A raw deserialized input still conveys no source
identity authority. Old result-bearing inputs are rejected, including inputs
that attach a `summary` field to the new shape; there is no silent conversion.
Providers derive output paths independently, and trusted replay compares them
with the original source import before evaluating the pinned contract.

These version numbers describe separate boundaries:

| Boundary | Current value | Meaning |
|---|---|---|
| Rust schema crate | 1.0.0 | Public API and serde compatibility |
| Plugin protocol | 1 | Existing manifest/request/response envelope |
| Evidence | 1 and 2 | Existing negotiation and admission rules |
| Boolean source input | 2 | Required statement-program schema |
| Boolean source profile | `qcore.c_boolean_field_contract.v2` | Program-input verification task |
| Input digest key | `qcore:boolean-field-source-input:v2` | Complete program identity in the task |
| ChunkIO provider package | 0.3.0 | Independent program execution introduced before this crate-version repair |

This crate bump does not renumber protocol or evidence versions, change
existing data-profile versions, or grant new witness authority. Provider
package versions remain independent; rebuilding records exact artifact
identity through the package manifest and digest.

## Explicit result ABI (2.0.0)

`BooleanFieldResult` distinguishes I32 from Unit in source input and proposed
summary records. New Rust fields and an exhaustive statement-enum variant are
source-incompatible, so this is a major crate-version change. Existing I32 wire
records omit the default field and retain their bytes and version-2 semantics.
Unit records explicitly serialize `result_kind: "unit"`, use source input version
3 and `ReturnUnit`, and bind `qcore:boolean-field-source-input:v3`. A mismatched
input version/result kind rejects. Unknown enum variants remain rejected.

## Declared process limits (3.0.0)

`PluginLimits` gained the public `additional` map of declared limit kinds, so
existing exhaustive struct construction no longer compiles: a major crate
version under the policy above. A manifest that declares no kind serializes
byte-identically, and no protocol, evidence or data-profile version moved.

## Counter source projection (4.0.0)

`counter_summary` adds the portable projection and proposal records for the
unsigned unit-counter capability: `CounterSourceInput` (a typed path, with no
derived result), `CounterUpdateRecord` (a proposed structural reading) and
`CounterContractWitness`. Three source-incompatible additions accompany them —
a `SemanticWitness::CounterContract` variant, an
`ObligationProperty::CounterUpdateContract` variant, and the required
`VerificationRequestV2::counter_effect` field — so this is a major crate
version, consistent with the exhaustive-variant precedent in 2.0.0.

Existing wire bytes are unchanged: `counter_effect` is `Option`, defaults to
absent and is skipped when serializing, exactly as `source_effect` is. A
request carrying both projections is rejected, and a counter projection is
admitted only with exactly one `counter_update_contract` obligation whose
subject function and translation unit match it.

| Boundary | Current value | Meaning |
|---|---|---|
| Rust schema crate | 4.0.0 | Public API and serde compatibility |
| Plugin protocol | 1 | Unchanged envelope |
| Evidence | 1 and 2 | Unchanged negotiation and admission rules |
| Counter source input | 1 | Required path-projection schema |
| Counter semantic profile | `qcore.c_counter_contract.v1` | Projection verification task |
| Counter input digest key | `qcore:counter-source-input:v1` | Complete projection identity in the task |
| ChunkIO provider package | 0.5.0 | Adds the counter capability route |

The projection carries no expected reading, and a proposal that matches the
core's own reading still discharges nothing on its own: the core evaluates the
pinned counter contract itself before any obligation is covered.

The unit semantic profile is `qcore.c_boolean_field_contract.v3`: two initial
Boolean states, no scalar result, a nominal record from the opaque import, and
`field`/`old_field` predicates. A path's numeric return slot is a zero placeholder
only; no unit contract can inspect it. The admitted checker is
`qcore.boolean_field_contract.v2`. Version-1 checkers remain usable for old I32
contexts. No protocol-v1 or ordinary evidence-v2 envelope changes, no new claim of
source authenticity, and no change to unrelated witness admission follows.
