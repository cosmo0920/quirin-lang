# W2 profile-manifest schema — design for review

Date: 2026-09-06. Status: **design only** — this is the
[`GENERALIZATION_PLAN.md`](../GENERALIZATION_PLAN.md) §5 review checkpoint
"after the W2 manifest schema (before mass migration)". No Rust code, tests,
or existing documents change with this document. No verdict moves: every raw
unit that is unpromoted today remains unpromoted; the design only changes
*where the declared trusted assumptions live* (plan invariant 6 — moving a
table from Rust into data makes it declarable, diffable, and per-source-set,
never verified).

Audience: the Opus-class reviewer named in W2's agent guidance, plus the
Sonnet-class migration agents that will execute W2 tasks 2–4 after review.

---

## 0. Scope and inputs

W2 task 1 asks for a versioned *profile manifest* format (schema types in
`qcore-schemas`, files under `contract/profiles/<source-set>/`) carrying what
is today hardcoded. The concrete hardcoded surfaces this schema must be able
to express, as surveyed in the current tree:

| # | Hardcoded site (current tree) | What it encodes |
|---|---|---|
| H1 | `compiler/src/external_source.rs` (whole file) | Pinned upstream paths with versions baked into path strings (`:9-19`), closed `AuditedLibrary` dispatch (`:38-64`), source "contracts" as `contents.contains(fragment)` over exact upstream substrings with attached mechanism summaries (`:103-237`) |
| H2 | `compiler/src/external_integration.rs:458-912` | 3-variant `AuditedLibrary` enum (`:461-465`), per-library `permits` arms over `LibraryMechanismSummary` (`:470-497`), the pinned-revision equality gates (`:532`, `:675`, `:811`), the operation → summary-name table `required_chunk_summary` (`:643-669`), the closed `MonkeyCallback` list and compile-time-constant audited callback summaries (`:712-754`), the Monkey composition profile booleans and their audit checks (`:764-898`) |
| H3 | `compiler/src/verifier_plugins/libevent/event_driven.rs` | `PINNED_LIBEVENT_COMMIT` (`:14`), the 48-entry API disposition table `LIBEVENT_API_PROFILE` (`:108-157`), the accepted build-profile matrix (`:83-95`), the pinned-commit + clean-tree + 12-file × ~40-needle source audit `verify_pinned_libevent_source` (`:1482-1612`) |
| H4 | `compiler/src/verifier_plugins/libevent/monkey_{source,binding,observation}.rs` | Closed 3-callback list `CALLBACKS` and 15-wrapper list `WRAPPERS` (`monkey_source.rs:12-29`), the typed-import profile: record names, sizeof types, per-function signature constraints, callee allowlist (`monkey_source.rs:63-108`), binding/observation contract text constants (`monkey_binding.rs:115`, `monkey_observation.rs:11`) |
| H5 | `compiler/src/verifier_plugins/chunkio/chunk_source.rs` | Exactly two modeled Boolean fields on one record: `ChunkSourceField::{Busy,Locked}` (`:19-23`) with the record-layout check pinned to `cio_chunk` (`:133-168`); `PINNED_FLUENT_BIT_REVISION` in `chunkio/mod.rs:22` |
| H6 | `compiler/src/verifier_plugin.rs:991-1076` | First-party manifest literals: the plugin id / library / capability / theorem-family table `first_party_manifests` + `unextracted_first_party_manifests` + `first_party_registry` (the plan's pre-relocation reference `verifier_plugin.rs:1436-1521` maps to these lines in the current tree) |
| H7 | `compiler/examples/cycle19_fluent_bit_libraries_audit.rs` | `PINNED_FLUENT_BIT_COMMIT` (`:27`), the clean-tree path list for the audited subtrees (`:120-145`), the per-library iteration/selection table (`:165-240`), per-check revision plumbing throughout |

Existing published surface the schema must reuse (and its constraint):
`crates/qcore-schemas` is L0, serde types only, and a test forbids workspace
dependencies. Already published there and reused verbatim by this design:

- `mechanism::LibraryMechanismSummary` (closed 8-variant vocabulary),
- `identity::ExternalSourceBundle` (revision + file map),
- `contract::{ResourceContracts, ResourceFunctionContract, …}` (W1 loader
  output),
- `plugin::{PluginManifest, PluginId, VerifierCapability, SupportedTarget,
  TargetProfile, …}` (protocol v1 manifest model, `deny_unknown_fields`
  everywhere).

Precedents this design deliberately follows: the W1 resource-contract
manifest (`cycle19-resource-contracts.json`: JSON, integer
`schema_version`, total-or-reject loading) and protocol v1's day-one
versioning discipline.

---

## 1. Manifest format

### 1.1 On-disk layout

One directory per *source set* (a pinned, audited corpus — possibly spanning
several libraries and nested checkouts):

```
contract/profiles/<source-set>/
  profile.json                     # the ProfileManifest root (required)
  providers/<plugin-id>.json       # one PluginManifest per provider bound to
                                   # this source set (replaces the H6 literals;
                                   # same schema as verifier-plugin.json today)
```

Concretely for the current corpus:

```
contract/profiles/fluent-bit/
  profile.json                     # pins: fluent-bit @66910c1…; libraries:
                                   # cmetrics, ctraces, librdkafka, cfl,
                                   # chunkio, monkey, fluent_bit (input-chunk)
  providers/qcore.chunkio.json
  providers/qcore.cmetrics.json
  providers/qcore.ctraces.json
  providers/qcore.librdkafka.json
contract/profiles/libevent/
  profile.json                     # pin: libevent @d82464a…
  providers/qcore.libevent.json
```

Design choices:

- **JSON, single `profile.json` per source set.** Matches the W1 manifest and
  `verifier-plugin.json`; one file keeps the load unit atomic (total-or-reject
  is per source set, like the W1 loader is per manifest). File digests are
  inline — the current tables are small (tens of files); a split-out digest
  file can be added in a later schema version if a corpus outgrows this.
- **A source set may declare several pins.** Fluent Bit vendors its libraries
  at one commit; Libevent is a separate pinned checkout
  (`third_party/libevent`). Monkey-over-Libevent composition references both
  pins, so pins are named and libraries reference a pin by name.
- **Contracts stay where W1 put them.** The profile does not restate
  `.qcore` contracts; it *references* the W1 resource-contract manifest so a
  profile directory + a contract directory is the complete definition of a
  source set (W2's pluggability deliverable).
- **Relative paths only**, validated with the same `safe_relative_path` rule
  `PluginManifest` already enforces (no `..`, no absolute paths, no roots).

### 1.2 Proposed serde types (destined for `crates/qcore-schemas/src/profile.rs`)

All structs use `#[serde(deny_unknown_fields)]` and
`#[serde(rename_all = "snake_case")]`, matching the existing schema-crate
convention. Everything is `Serialize + Deserialize`, no analysis logic, no
workspace dependencies — the L0 rules. Sketch (field-by-field commentary
below):

```rust
pub const PROFILE_MANIFEST_VERSION: u32 = 1;

/// Root of `contract/profiles/<source-set>/profile.json`.
pub struct ProfileManifest {
    /// Must equal PROFILE_MANIFEST_VERSION exactly; any other value is a
    /// load rejection (fail-closed), never a best-effort parse.
    pub schema_version: u32,
    /// Source-set name; must match the directory name (checked at load).
    pub source_set: String,
    /// Named pinned checkouts. Keys are referenced by `LibraryProfile::pin`.
    pub pins: BTreeMap<String, SourcePin>,
    /// One entry per audited library in the set. Keys are the library names
    /// used in `SupportedTarget::library` / `TargetProfile::library`.
    pub libraries: BTreeMap<String, LibraryProfile>,
    /// Relative path to the W1 resource-contract manifest that supplies
    /// `ResourceContracts` for this source set (e.g.
    /// "../../cycle19-resource-contracts.json").
    pub resource_contract_manifest: String,
    /// Provider manifests bound to this source set: relative paths under the
    /// profile directory (e.g. "providers/qcore.chunkio.json"). Loaded as
    /// `PluginManifest` through the existing package-discovery types;
    /// registry ordering is the sorted plugin-id order (deterministic).
    pub provider_manifests: Vec<String>,
}

/// A pinned checkout (H1 revision constants, H3 commit pin, H7 pin check).
pub struct SourcePin {
    /// Full lowercase hex commit hash. Exact equality; no ranges, no "*".
    pub git_commit: String,
    /// Paths (relative to the checkout root) that must be free of tracked
    /// modifications. Empty list = the whole tree must be clean (the H3
    /// `--untracked-files=no` porcelain check); a nonempty list reproduces
    /// the H7 `git diff --quiet HEAD -- <paths>` subtree check.
    pub clean_paths: Vec<String>,
    /// Human identity line carried into certificates/reports
    /// (e.g. "fluent-bit 66910c10a4d7…", "libevent d82464a2…").
    pub description: String,
}

/// Everything today's closed `AuditedLibrary` arms + provider tables say
/// about one library.
pub struct LibraryProfile {
    /// Which pin this library's files are checked against.
    pub pin: String,
    /// Where the library lives inside the pinned checkout and how audit
    /// units are selected (H7 iteration table).
    pub source_roots: Vec<SourceRootSelection>,
    /// The audited-file inventory with digests (H1 path tables, H3 file list).
    pub audited_files: Vec<AuditedFile>,
    /// Construct witnesses binding mechanism summaries to source constructs
    /// (H1 fragment tables, H3 needle audit). See §1.3 digest-vs-needle rule.
    pub source_witnesses: Vec<SourceWitness>,
    /// Replaces the `permits` match arm (H2 :470-497). A summary outside
    /// this set is `UnsupportedSummary` → the import fails → Unknown.
    pub permitted_mechanisms: BTreeSet<LibraryMechanismSummary>,
    /// Replaces `required_chunk_summary` (H2 :643-669): abstract operation
    /// kind → required summary name. Keys come from the closed
    /// `OperationKind` vocabulary below; values are the summary-name strings
    /// the import profile must contain (e.g. "cio_chunk_open").
    pub operation_summaries: BTreeMap<OperationKind, String>,
    /// Replaces the 48-entry `LIBEVENT_API_PROFILE` (H3 :108-157).
    /// Absence from the map is *not* representable as accepted: symbols not
    /// listed are Unknown, exactly like `classify_libevent_symbol` today.
    pub api_dispositions: BTreeMap<String, ApiDisposition>,
    /// Accepted build/backend/thread combinations (H3 :83-95). Empty = no
    /// build gating for this library.
    pub accepted_builds: Vec<BuildProfileSelector>,
    /// Required evidence/observation kinds: what must be *present* for the
    /// bounded checks to run at all (dispatcher observation, native
    /// allocation ledger, …). Names come from the closed
    /// `RequiredEvidenceKind` vocabulary below.
    pub required_evidence: BTreeSet<RequiredEvidenceKind>,
    /// Typed-source import surface (H4): which functions may be imported
    /// and under what syntactic bounds. Optional; only Monkey uses it today.
    pub typed_import: Option<TypedImportProfile>,
    /// Declared audited callback effect summaries (H2 :712-754). Optional.
    pub callback_summaries: BTreeMap<String, CallbackEffectSummary>,
    /// Boolean single-field source models (H5): record × field pairs the
    /// chunk-source importer may model. Optional; ChunkIO uses it today.
    pub boolean_field_models: Vec<BooleanFieldModel>,
    /// Provider binding for this library (H6): which plugin id serves it,
    /// which diagnostic namespace and Lean theorem family it declares.
    pub provider: ProviderBinding,
}

pub struct SourceRootSelection {
    /// Root relative to the pin checkout (e.g. "lib/chunkio/src").
    pub root: String,
    /// How compile-database units under the root are selected.
    pub selection: UnitSelection,
}

pub enum UnitSelection {
    /// Only `.c` files whose parent is exactly the root (H7 cmetrics/ctraces).
    TopLevel,
    /// Every built unit under the root, recursively (H7 librdkafka/chunkio).
    Recursive,
    /// One exact file (H7 `src/flb_input_chunk.c`, Monkey units).
    Exact { file: String },
}

pub struct AuditedFile {
    /// Path relative to the pin checkout root.
    pub path: String,
    /// SHA-256 of the file's bytes at the pinned revision, lowercase hex.
    /// Mandatory: every audited file is digest-pinned (§1.3).
    pub sha256: String,
}

/// One named construct witness: "this mechanism summary is justified by this
/// construct being present in this file". Replaces `SourceContract` (H1).
pub struct SourceWitness {
    /// Stable name, e.g. "librdkafka-refcount"; unique within the library.
    pub name: String,
    /// Must name a path present in `audited_files` (checked at load).
    pub file: String,
    /// The check that establishes the construct (§1.3).
    pub check: WitnessCheck,
    /// Mechanism summaries this witness contributes when the check passes.
    pub mechanisms: BTreeSet<LibraryMechanismSummary>,
}

pub enum WitnessCheck {
    /// Preferred: the construct's presence is implied by the audited file's
    /// digest (already pinned in `audited_files`); the witness records the
    /// human-readable construct description for the report.
    Digest { construct: String },
    /// Exact-substring needles (today's `contents.contains` checks), allowed
    /// only with a rationale for why a digest cannot express the check
    /// (§1.3). Empty needles or an empty rationale reject the manifest.
    Substrings { needles: Vec<String>, rationale: String },
}

/// Closed operation vocabulary for `operation_summaries` keys — the abstract
/// `ChunkOperation` kinds of H2 :643-669, name-stable and serializable.
pub enum OperationKind {
    CreateContext, CreateStream, OpenChunk, ResidencyUp, ResidencyDown,
    Lock, Unlock, SetBusy, TaskCustody, RouteCustody,
    BeginTransaction, CommitTransaction, RollbackTransaction,
    CloseChunk, DeleteStreamContent, DestroyStream, DestroyContext,
}

pub enum ApiDisposition {
    /// Inside the bounded model. Note: no `Safe` value exists here on
    /// purpose — disposition is model coverage, never a verdict.
    Accepted,
    /// Explicitly recorded as outside the model (documents *why* a symbol
    /// is excluded, e.g. "openssl", "dns"); classification-wise identical
    /// to absence: Unknown.
    Unknown { reason: String },
}

pub struct BuildProfileSelector {
    pub backend: String,   // "epoll" | "kqueue" | "select" | "poll" | "iocp"
    pub threads: String,   // "pthreads" | "windows_threads"
    pub forbid: BTreeSet<String>, // e.g. {"openssl"}
}

/// Closed evidence-kind vocabulary (extending it is a schema-version event).
pub enum RequiredEvidenceKind {
    DispatcherObservation,     // H2 MonkeyComposition observation trace
    EventMapCustody,           // H2 profile booleans :832-843
    NullableBaseCreation,
    NullableEventCreation,
    RegistrationFailurePath,
    NativeAllocationLedger,    // Cycle 19 native memory observations
    OsEndpointLedger,          // Cycle 19 close/unmap/delete endpoints
    ControlFlowComplete,       // H2 ChunkImportProfile :678
}

pub struct TypedImportProfile {
    /// Function name → role. Replaces CALLBACKS/WRAPPERS (H4 :12-29).
    pub functions: BTreeMap<String, ImportRole>,
    /// Record names the importer may reference (H4 :64-70).
    pub records: BTreeSet<String>,
    /// Types permitted under sizeof (H4 :71-79).
    pub sizeof_types: BTreeSet<String>,
    /// Per-function exact signature constraints (H4 :80-84).
    pub signature_constraints: BTreeMap<String, SignatureConstraint>,
    /// The callee allowlist: calls stay opaque, but only these names may
    /// appear (H4 :85-107). Anything else fails the import → Unknown.
    pub restricted_calls: BTreeSet<String>,
}

pub enum ImportRole { Callback, Wrapper }

pub struct SignatureConstraint {
    pub result: String,
    pub parameters: Vec<String>,
}

/// Declared audited callback effects (H2 :719-754). These are *declared
/// trusted assumptions* (invariant 6): the checker still verifies the
/// imported body against them; a mismatch is a diagnostic → Unknown.
pub struct CallbackEffectSummary {
    pub argument_borrowed: bool,
    pub writes_fired_array: bool,
    pub releases_event_map: bool,
    pub closes_descriptor: bool,
    pub no_effect: bool,
}

/// H5: one modeled Boolean field on one record.
pub struct BooleanFieldModel {
    pub record: String,          // "cio_chunk"
    pub field: String,           // "busy" | "locked"
    /// Which OperationKind a write to this field maps to.
    pub set_operation: OperationKind,
    pub clear_operation: OperationKind,
}

pub struct ProviderBinding {
    /// Must match a plugin id from `provider_manifests` (checked at load).
    pub plugin_id: String,
    /// The provider's diagnostic namespace (today the `QP***` space per
    /// Cycle 20 / W7; the core allocates no numbers for library-specific
    /// conditions).
    pub diagnostic_namespace: String,
    /// Lean theorem family, mirroring `PluginManifest::lean_theorem_family`
    /// (cross-checked against the referenced provider manifest at load;
    /// disagreement rejects the profile).
    pub lean_theorem_family: String,
}
```

Field-by-field rationale highlights:

- **`permitted_mechanisms` + `source_witnesses` reproduce today's two-sided
  check.** `derive_library_summary_profile` (H1) only *grants* summaries whose
  witnesses pass; `check_imported_library_custody` (H2) additionally rejects
  any summary outside the `permits` arm and any *used* summary not granted.
  The manifest keeps both sides: witnesses grant, `permitted_mechanisms`
  bounds, `required_library_summaries` (unchanged Rust) demands. All three
  failures remain Unknown.
- **`operation_summaries` values stay strings** (the `"cio_chunk_open"`
  names) because they are compared against `ChunkImportProfile::summaries:
  BTreeSet<String>` supplied by the audit; the *keys* become a closed enum so
  a manifest cannot silently name an operation the checker does not model.
- **`ApiDisposition` has no accept-by-default and no `Safe`.** The map is an
  allowlist for model coverage only; symbol absence and explicit
  `Unknown{reason}` classify identically. The `reason` field preserves the
  documentation value of the current 8 explicit-Unknown rows (DNS, HTTP,
  OpenSSL, listeners, allocators/locks, fork repair).
- **`provider_manifests` replaces H6** by routing first-party manifests
  through the same `PluginManifest` JSON the executable package loader
  already reads (`verifier-plugin.json` schema, unchanged). Built-in
  entrypoints remain `BuiltIn` with their `builtin:<id>` digest rule;
  deterministic registry ordering is by sorted plugin id, which matches
  today's fixed registration order once ids are sorted.
- **Everything is data the loader can hash.** The audit certificate must
  record the profile file's own digest plus `schema_version` as part of its
  assumption set (invariant 6: certificates record which trusted inputs were
  assumed). This mirrors what the W1 loader already does with the contract
  manifest sidecar (`.contracts.json` records manifest text + source
  assumptions).

### 1.3 Digest-preferred rule (needles need a rationale)

Every `audited_files` entry carries a mandatory SHA-256. Given that, a
digest-pinned file at a pinned revision already implies "this exact construct
is present", so the default witness form is `WitnessCheck::Digest` — it names
the construct for the human report and relies on the file digest.

`WitnessCheck::Substrings` is retained **only** for cases a digest cannot
express, and requires a non-empty `rationale`. The two legitimate cases found
in the survey:

1. **Cross-revision durability of a specific declaration** — e.g. the H3
   header needles (`"int event_del_noblock(struct event *ev);"`) are intended
   to keep working as a *construct check* if the pin is deliberately updated:
   the digest changes on any byte, the needle only when the declaration
   changes. Rationale text: "construct must survive deliberate pin updates
   without re-deriving the whole-file digest baseline".
2. **Fragment-level provenance in reports** — the H1 fragment lists double as
   human-auditable evidence of *which* lines justify a mechanism summary
   (e.g. `"TAILQ_REMOVE(&rkq->rkq_q, rko, rko_link)"` justifying
   `IntrusiveList`/`TypedQueue`). Where the review wants that provenance kept,
   the witness keeps its needles with rationale "fragment-level provenance
   retained for the archived report".

A needle without a rationale, an empty needle list, or a rationale that is
whitespace rejects the whole manifest at load (fail-closed; see §3). The
migration default (§4) is: convert every current fragment table to
`Digest{construct}` witnesses plus the file digest, and keep needles only
where one of the two rationales above genuinely applies.

---

## 2. Versioning and compatibility policy

Day-one versioning, protocol-v1 style:

- **One integer `schema_version` at the root**, first field checked. Version
  1 is defined by this document. A manifest declares exactly one version
  (unlike plugin manifests, which negotiate over sets — a data file has no
  negotiation counterpart; it either is v1 or it is not readable).
- **Unknown version ⇒ rejection ⇒ Unknown.** The loader refuses the whole
  profile; nothing is partially imported. Never "parse what we recognize".
- **Unknown fields ⇒ rejection ⇒ Unknown.** All types are
  `deny_unknown_fields` (the schema crate's existing convention). This is
  deliberate: a field this loader does not understand could be a *narrowing*
  the manifest author intended (e.g. a new exclusion list); ignoring it would
  be an optimistic fallback, violating invariant 1.
- **What is a breaking change (⇒ version bump).** Under
  total-or-reject + `deny_unknown_fields`, effectively *every* schema change
  is breaking and takes a new `schema_version`:
  - adding, removing, or renaming any field (even optional ones — an old
    loader must not silently ignore a new field, and a new loader must not
    guess defaults for an old file unless the bump defines them);
  - extending any closed vocabulary (`LibraryMechanismSummary`,
    `OperationKind`, `ApiDisposition`, `RequiredEvidenceKind`,
    `ImportRole`, `UnitSelection`, `WitnessCheck`);
  - changing the meaning, units, or matching semantics of any existing field
    (e.g. substring → regex would be a new field in a new version, not a
    reinterpretation).
- **Loader support window.** The loader supports an explicit set of versions
  (initially `{1}`). When version 2 exists, v1 files remain loadable only if
  the v2 change record defines an exact, total v1→v2 interpretation; if any
  v1 construct is ambiguous under v2 semantics, v1 support is dropped and
  v1 files are rejected (fail-closed beats convenience).
- **Vocabulary lives in `qcore-schemas` and is semver-coupled.** Extending
  `LibraryMechanismSummary` (or any closed enum above) is simultaneously a
  schema-crate compatibility event (W0 task 3 policy) and a profile
  `schema_version` bump. String-typed open vocabularies were considered and
  rejected: an unknown mechanism string that deserializes successfully could
  flow into a permitted set and weaken the `UnsupportedSummary` gate.
- **The manifest never widens Rust-side gates.** A v1 manifest cannot express
  "accept all symbols", "skip the pin check", "any revision", or a `Safe`
  disposition — those non-features are part of the compatibility contract:
  adding any of them would be a schema change subject to the same review this
  document is receiving.

---

## 3. Fail-closed semantics table

For every load/validation failure mode, the resulting behavior. In all rows
the affected scope's verdict contribution is **Unknown** — never a default
profile, never a partial import, never a skipped check. Diagnostics are given
structurally (Rust enum variants, serialized into reports); numeric code
assignment is deferred to W7 (see open question Q4).

| # | Failure mode | Detection point | Diagnostic (structural) | Resulting verdict effect |
|---|---|---|---|---|
| F1 | No profile directory / `profile.json` missing for a requested source set | Loader entry | `ProfileNotFound { source_set }` | Source set unmatched: every unit that would have used it stays Unknown (identical to today's "unmatched profile" invariant) |
| F2 | `profile.json` unreadable / not valid JSON | Loader parse | `ProfileUnreadable { path, error }` | Whole profile rejected; all dependent scopes Unknown |
| F3 | Unknown `schema_version` | First field check | `UnsupportedSchemaVersion { found, supported }` | Whole profile rejected; Unknown |
| F4 | Unknown field / structurally invalid content (serde `deny_unknown_fields`) | Loader parse | `ProfileUnreadable { path, error }` | Whole profile rejected; Unknown |
| F5 | Internal reference broken (library → unknown pin name; witness → file not in `audited_files`; `ProviderBinding::plugin_id` not among `provider_manifests`; duplicate witness names; `source_set` ≠ directory name) | Post-parse validation | `ProfileInconsistent { detail }` | Whole profile rejected; Unknown |
| F6 | Substring witness with empty needles or empty/whitespace rationale | Post-parse validation | `NeedleWithoutRationale { witness }` | Whole profile rejected; Unknown |
| F7 | Pin commit mismatch (checkout HEAD ≠ `git_commit`) | Pin verification | `SourceRevisionMismatch { pin, expected, actual }` | All libraries on that pin: Unknown (today's `SourceRevisionMismatch` / `WrongCommit`) |
| F8 | Dirty checkout on `clean_paths` (or whole tree) | Pin verification | `DirtyCheckout { pin, paths }` | All libraries on that pin: Unknown (today's `DirtyCheckout` / H7 tracked-modification error) |
| F9 | Audited file missing on disk | File audit | `MissingFile { path }` | Library profile not derived: Unknown (today's `MissingFile`) |
| F10 | File digest mismatch | File audit | `DigestMismatch { path, expected, actual }` | Library profile not derived: Unknown |
| F11 | Substring needle absent from its file | Witness check | `WitnessMismatch { witness, file }` | The witness's mechanisms are **not granted**; any imported program requiring them then fails with `MissingSummary` → Unknown (today's `ContractMismatch` path). Note today's H1 makes *any* contract mismatch reject the whole library certificate; migration preserves that: any failed witness → library Unknown |
| F12 | Imported program uses a mechanism outside `permitted_mechanisms` | Import check (unchanged Rust) | `UnsupportedSummary { mechanism }` | Unknown (today's behavior, data-driven bound) |
| F13 | Operation whose `OperationKind` has no `operation_summaries` entry, or whose named summary is absent from the supplied import profile | Import check | `MissingSummary { name }` | Unknown (today's behavior) |
| F14 | API symbol absent from `api_dispositions`, or present with `Unknown{..}` | Symbol classification | none (classification result) | Symbol classified Unknown, exactly like `classify_libevent_symbol` today — absence is never accepted |
| F15 | Build profile outside `accepted_builds` | Build gating | `UnknownBuildProfile` | Unknown (today's `accepted()` false path) |
| F16 | Required evidence kind absent (e.g. no dispatcher observation, `control_flow_complete` false) | Composition check | `MissingEvidence { kind }` | Unknown (today's `MissingCallback`/`Missing*Path`/`IncompleteControlFlow` family) |
| F17 | Callback summary mismatch vs. `callback_summaries` declaration | Composition check | `InvalidCallbackSummary { callback }` | Unknown (today's behavior) |
| F18 | Referenced provider manifest missing/invalid (`PluginManifest::validate` failure) or `ProviderBinding` disagrees with it (theorem family, plugin id) | Provider load | `ProviderManifestInvalid { path, error }` | Provider not registered from this profile; every capability request that would route to it gets today's provider-absence handling: Unknown |
| F19 | Referenced W1 contract manifest missing/rejected | Contract load | W1 loader's existing error | Unknown for all resource-projection scopes (W1's existing total-or-reject behavior, unchanged) |

Two properties worth the reviewer's attention:

- **No row downgrades to a warning.** Every load-time row (F1–F6) rejects the
  entire profile: a profile that is 95% valid is 0% valid. This matches the
  W1 loader ("total-or-reject") and prevents a truncated manifest from
  silently narrowing the witness set while keeping the permitted set.
- **No row can produce `Safe` or `Unsafe`.** Profile loading only ever
  gates existing checks; verdicts other than Unknown continue to require the
  same discharged obligations as today. `Unsafe` remains reserved for
  validated reachable violations and is never a manifest outcome.

---

## 4. Migration inventory (W2 tasks 2–4, per invariant 3)

For each hardcoded site: the manifest section that replaces it, and the
parity artifact required **before** the Rust table may be deleted. Until each
parity record is archived, the Rust table stays and remains authoritative
(gate ordering: Cycle 19 routes stay until parity; raw units remain
unpromoted throughout — this table moves representation, not verdicts).

| Hardcoded site | Replacing manifest section | Parity artifact required before deletion |
|---|---|---|
| H1 `external_source.rs:9-19` path constants | `libraries.*.audited_files[].path` (fluent-bit profile) | `docs/reports/w2-profile-fluent-bit/README.md`: byte-identical `LibrarySourceCertificate` fields (checked files, checked contracts, granted summaries) for librdkafka/CFL/ChunkIO from manifest-driven derivation vs. current arms, plus the negative cases (mutated file, missing file, wrong revision) still rejecting |
| H1 `external_source.rs:38-64` `required_library_source_paths` dispatch + `:60-64` per-library contract dispatch | `libraries` map keyed by library name; `source_witnesses` per library | Same record as above |
| H1 `external_source.rs:103-237` `SourceContract` fragment tables | `source_witnesses` with `Digest{construct}` default; `Substrings{needles, rationale}` only per §1.3 | Same record; where a fragment table becomes a digest witness, the report must list each dropped needle and the digest that supersedes it |
| H2 `external_integration.rs:461-465` `AuditedLibrary` enum + `:470-497` `permits` arms | `libraries.*.permitted_mechanisms` | `docs/reports/w2-profile-mechanisms/README.md`: for each library, the permitted set loaded from data equals the current arm; `UnsupportedSummary` still raised for each out-of-set mechanism (test evidence) |
| H2 `:532`,`:675`,`:811` `PINNED_FLUENT_BIT_REVISION` equality gates | `pins.fluent-bit.git_commit`; check functions take the expected revision from the loaded profile | Same mechanisms record; mismatch case demonstrated to still reject |
| H2 `:643-669` `required_chunk_summary` | `libraries.chunkio.operation_summaries` (+ `fluent_bit` input-chunk entries: `SetBusy`, task/route custody) | `docs/reports/w2-profile-chunk-summaries/README.md`: table equality (all 17 operation kinds → identical summary strings) and `MissingSummary` still raised when an entry is removed |
| H2 `:712-754` `MonkeyCallback` + `MonkeyCallbackSummary::audited` | `libraries.monkey.callback_summaries` | `docs/reports/w2-profile-monkey/README.md`: composition check on the pinned corpus yields identical certificates and identical diagnostics on each seeded mismatch |
| H2 `:764-898` `MonkeyCompositionProfile` boolean requirements | `libraries.monkey.required_evidence` (EventMapCustody, Nullable*, RegistrationFailurePath, DispatcherObservation) | Same Monkey record |
| H3 `event_driven.rs:14` `PINNED_LIBEVENT_COMMIT` | `pins.libevent.git_commit` (libevent profile) | `docs/reports/w2-profile-libevent/README.md`: `verify_pinned_libevent_source` outcome identical (accept on pinned clean tree; each error class reproduced) with data-driven pin/needles; the existing submodule-revision test keeps passing untouched |
| H3 `:83-95` `accepted()` build matrix | `libraries.libevent.accepted_builds` | Same libevent record: acceptance matrix truth table diffed exhaustively (5 backends × 2 thread modes × openssl flag) |
| H3 `:108-157` `LIBEVENT_API_PROFILE` (48 entries) | `libraries.libevent.api_dispositions` | Same libevent record: all 48 classifications identical **and** a not-listed symbol still Unknown |
| H3 `:1482-1612` file × needle audit | `libraries.libevent.audited_files` (12 files, digests) + `source_witnesses` (needles retained per §1.3 case 1) | Same libevent record |
| H4 `monkey_source.rs:12-29` CALLBACKS/WRAPPERS | `libraries.monkey.typed_import.functions` (3 Callback + 15 Wrapper) | Monkey record: imported function set and per-function `MonkeySourceResult`s identical on the pinned units |
| H4 `monkey_source.rs:63-108` `SourceImportProfile` (records, sizeof types, signature constraints, callee allowlist) | `libraries.monkey.typed_import.{records,sizeof_types,signature_constraints,restricted_calls}` | Monkey record: identical import results; a call outside the allowlist still fails the import |
| H4 `monkey_binding.rs`/`monkey_observation.rs` contract text constants | Stay in Rust for W2 (they are checker semantics, not source-set data); revisit at W5 provider extraction | n/a in W2 — explicitly out of scope (see Q5) |
| H5 `chunk_source.rs:19-23` field pair + `:133-168` `cio_chunk` layout gate | `libraries.chunkio.boolean_field_models` (2 entries) | `docs/reports/w2-profile-chunk-fields/README.md`: identical `ChunkSourceSummary` output on the pinned units; a third, undeclared field still rejected |
| H6 `verifier_plugin.rs:991-1076` first-party manifest literals | `provider_manifests` files under `contract/profiles/*/providers/` loaded through the existing package-discovery path | `docs/reports/w2-profile-providers/README.md`: `first_party_manifests()` output (all five manifests) byte-identical after serialization round-trip; registry ordering deterministic; `qcorec plugins list`/`inspect` output unchanged |
| H7 `cycle19_…audit.rs:27` pin, `:120-145` clean-path list, `:165-240` iteration table | `pins.fluent-bit.{git_commit,clean_paths}`, `libraries.*.source_roots` | `docs/reports/w2-profile-audit-driver/README.md`: full audit example rerun with profile-driven selection; verdict/coverage JSON diffed against the current archived Cycle 19 report — byte-identical or every difference explained and recorded. This is the W2 exit gate; the deeper orchestration move is W5 task 1 |

Deletion order discipline: within each row, land (a) schema type + loader +
tests, (b) dual-run parity harness, (c) archived report, (d) deletion — as
separate reviewable changes, keeping `cargo test --workspace` green at each
step. No row's deletion may precede its report. The `qcore_contract_corpus`
pinned tables are untouched by W2 (profiles are not `.qcore` contracts); if
any migration step does touch the canonical contract corpus, its pin tables must be
updated deliberately and the change must say so.

Placement note (invariant §2.7): the loader and validation logic land in
`crates/qcore-contract` (L1, alongside the W1 loader) — not in the
`qcore-compiler` grab-bag; the serde types land in `crates/qcore-schemas`
(L0). Providers and the audit driver consume loaded profiles only through
schema types.

---

## 5. Open questions for the review checkpoint

**Q1. Granularity: one profile per source set, or one per library?**
Proposed: per source set (one `profile.json` for all of Fluent Bit's vendored
libraries, sharing one pin), because the pin, the clean-path list, and the
audit iteration are corpus-level facts, and cross-library composition (Monkey
over Libevent) needs a place to reference two pins. Cost: a large-ish file
(est. 400–700 lines of JSON for fluent-bit). **Recommendation: per source
set, revisit only if the file exceeds review-ability; the schema already
permits splitting later by adding a versioned `include` in v2 — do not add
includes in v1.**

**Q2. Digest baseline provenance.** Who computes the `sha256` values and how
is the reviewer convinced they match the pinned revision? Proposed: a
generator mode in the audit driver emits the digest table *from the pinned
checkout after the commit/clean check passes*, and the parity report archives
the generated table. **Recommendation: accept, with the rule that generated
digests are committed only alongside the parity record that validated them —
a digest table with no archived provenance is treated as hand-written and
gets extra review.**

**Q3. How many H3 needles survive as `Substrings` witnesses?** §1.3 gives two
admissible rationales. Strictest reading: with mandatory per-file digests,
*zero* needles are necessary at the current pins, and every needle converts
to `Digest{construct}`. Keeping needles preserves behavior-relevant checks
across deliberate pin updates but is exactly the over-fitting W2 is removing.
**Recommendation: convert all H1 fragment tables to digest witnesses
(rationale 2 satisfied by the archived report instead); keep only the H3
public-header declaration needles (rationale 1), since those express API
presence the event model genuinely depends on. Record the kept set in the
libevent parity report.**

**Q4. Diagnostic numbering for profile-load failures.** The `Q700/Q701`
collision (W7) makes new `Q7xx` allocation unwise, and `QP***` is
provider-scoped. **Recommendation: ship W2 with structural diagnostics only
(serialized enums in reports, as §3 defines), and let W7 assign a numeric
namespace once the collision is resolved — numbering is presentation, and
deferring it avoids a second renumbering event.**

**Q5. Do the Monkey binding/observation contract text constants
(`CALLBACK_BINDING_CONTRACTS`, `DESCRIPTOR_OBSERVATION_CONTRACTS`) belong in
the v1 manifest?** They read like data but function as documentation of
checker semantics bound to specific Rust replay logic. Moving text without
moving the logic would create a manifest field the loader cannot validate
against anything (a fail-open temptation). **Recommendation: out of scope for
W2 (kept in the migration inventory as explicit non-items); revisit at W5
when the Monkey trio moves to a provider crate and the text can travel with
the logic that interprets it.**

**Q6. Should `SupportedTarget` revision wildcards tighten when profiles
land?** Today's first-party manifests declare `revisions: {"*"}` (H6). Once
provider manifests are on-disk next to a profile with exact pins, `"*"` is a
latent over-claim. Narrowing them to the pinned revision would, however,
change `supports()` matching behavior — which is provider routing, not
representation. **Recommendation: keep `"*"` byte-identical through W2 parity
(the H6 row demands byte-identical manifests), and file the narrowing as a
follow-up decided at the W3/W5 boundary where target routing is actually
exercised.**

**Q7. Verdict-contract binding.** Invariant 4 requires every published result
to carry `(artifact, property, semantic profile, scope, bounds, assumptions,
evidence version)`. Proposed: the loaded profile contributes to the
*assumptions* component as `(source_set, schema_version, profile.json
digest, pin commits)`, recorded by the audit driver in the report sidecar the
same way W1 records the contract manifest. **Recommendation: accept; make the
sidecar extension part of the H7 parity work so the first profile-driven
audit already records its profile assumption, and no report ever presents a
profile-derived result without naming the profile it trusted.**

---

## 6. What this design explicitly does not do

- It does not move any verdict: 2 Safe / 98 provisional Unsafe / 81 Unknown
  stands, raw units remain **unpromoted**, and no manifest field can express
  a promotion.
- It does not weaken any fail-closed gate: every current rejection path has a
  named row in §3 mapping to Unknown, and several (F5, F6) are new rejections
  that only a data format needs.
- It does not touch the Cycle 19 fail-closed acceptance gates (per-path
  identity imports, observation evidence) — those remain Cycle 19's own work.
- It does not design provider routing, obligation dispatch (W3), or the
  orchestration move (W5 task 1); it only ensures the manifest carries what
  those consumers will need (source roots, provider bindings, evidence
  kinds).
- It does not make profile data verified truth. A profile is a declared
  trusted assumption; certificates and reports must say which profile, at
  which digest, was assumed (Q7).
