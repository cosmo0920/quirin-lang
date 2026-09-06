# qcore-schemas

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

The plugin trust boundary is currently the compile-time crate boundary:
providers are separate crates speaking these types in-process (decision
recorded 2026-09-06; the out-of-process migration in the plan is deferred,
not abandoned — these types stay process-portable either way).
