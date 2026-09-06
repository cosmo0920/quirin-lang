# qcore-schemas 1.0.0

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
