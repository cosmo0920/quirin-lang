# qcore-schemas 15.0.0

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

## Reactor program projection (5.0.0)

`reactor_program` adds the portable projection and proposal records for one
admitted reactor callback family: `ReactorProgramInput` (a bounded operation
vocabulary, with no derived result), the `ReactorOperationRecord` /
`CallbackContextRecord` / `DeleteModeRecord` vocabulary it is built from, and
`ReactorCallbackWitness`. The proposal it carries is the *existing*
`reactor::ReactorCallbackProposal` at its unchanged version 1; those records and
their bytes did not move.

Two source-incompatible additions accompany the new module — a
`SemanticWitness::ReactorCallback` variant and the required
`VerificationRequestV2::reactor_program` field — so this is a major crate
version, consistent with the exhaustive-variant precedent in 2.0.0 and 4.0.0.

**Why the additions are forced.** The witness enum is the only carrier of a
replayed proposal, and no existing variant can hold a callback assignment
inventory: `ContractObservation` is a recorded-run record set with decimal
observation values, and reusing it would misdescribe what the witness is and
what its acceptance means. The program projection could technically have
travelled as JSON inside a `NormalizedFact::subject`, the way `LifecycleSiteFact`
does, but that encoding has no `deny_unknown_fields`, no schema-version field
and no `validate()` rule — it would be a strictly weaker boundary than the
typed field the counter and Boolean routes already established. No
`ObligationProperty` variant was added: the route reuses the reserved
`event_generation_discharged` and `callback_quiescent_at_teardown` properties,
which now have a derivation and a witness kind.

Existing wire bytes are unchanged: `reactor_program` is `Option`, defaults to
absent and is skipped when serializing, exactly as `source_effect` and
`counter_effect` are. `tests/reactor_request_wire.rs` pins that a request
without a projection serializes with none of the three keys and round-trips.
A request carrying a reactor projection together with either source projection
is rejected, and a reactor projection is admitted only with exactly one
`event_generation_discharged` and one `callback_quiescent_at_teardown`
obligation, both over the whole unit. The quiescence property is the marker
that requires a projection; the generation property alone does not, because it
predates this route and other exchanges still name it.

| Boundary | Current value | Meaning |
|---|---|---|
| Rust schema crate | 6.0.0 | Public API and serde compatibility |
| Plugin protocol | 1 | Unchanged envelope |
| Evidence | 1 and 2 | Unchanged negotiation and admission rules |
| Reactor callback proposal | 1 | Unchanged proposal records |
| Reactor program input | 1 | Required program-projection schema |
| Reactor semantic profile | `qcore.reactor_callback_program.v1` | Projection verification task |
| Reactor input digest key | `qcore:reactor-program-input:v1` | Complete projection identity in the task |
| Libevent provider package | 0.2.0 | Adds the reactor callback capability route |

The projection carries no proposal, and a proposal that matches the core's own
exported inventory still discharges nothing on its own: the core runs its own
reactor model checker over its own program before any obligation is covered.

## Reason categories for uncovered obligations (6.0.0)

`reason` publishes the closed seven-category vocabulary of
`docs/QCORE_SPEC_r0_3.md` §L21.1 — `unsupported-syntax`, `missing-premise`,
`incomplete-obligation-coverage`, `stale-evidence`, `provider-failure`,
`residual-custody`, `environment-unavailable` — and
`obligation::DerivationIncomplete` gains a **required** `category` field drawn
from it. `ReasonCategory::subject` partitions the seven into
`AnalysedSource`, `Analyser` and `Environment`, with
`environment-unavailable` the sole environment member, so §L21.1 rule 1 — an
environment failure is never readable as a statement about the analysed source
— holds by construction rather than by review.

**Why this is a major crate version, on two independent grounds.**

1. *Source incompatibility.* `DerivationIncomplete` is a plain struct, so every
   exhaustive construction of it stops compiling. This is the same
   exhaustive-construction precedent as 2.0.0, 4.0.0 and 5.0.0.
2. *A tightened validation rule.* The record is `deny_unknown_fields` and
   `category` has **no** serde default, so a `DerivationIncomplete` recorded
   before this version is now rejected, and a record written now is rejected by
   a 5.0.0 consumer. That refusal is the point, not a side effect:
   `DerivationIncomplete` is an element of `VerificationTask`, whose SHA-256 over
   canonical JSON travels in every evidence-v2 request and every witness, so a
   witness recorded against a task whose marker list is non-empty must not
   validate against a task derived after the change. `tests/reason_codes.rs`
   pins the refusal in both directions.

**Existing wire bytes.** A task whose `derivation_incomplete` list is *empty*
serializes byte-identically to 5.0.0 — `"derivation_incomplete":[]` does not
mention the element shape — so its `task_digest` is unchanged. This was
measured, not assumed; the measurement and the exact digests that do move are
recorded in `docs/reports/remaining-reason-codes/README.md`. Nothing else in
the crate changed: protocol v1, evidence versions 1 and 2, every data-profile
version, and every other record's bytes are untouched.

The category **refines** a row and replaces nothing (§L21.1 rule 2): no source
span, `property`, semantic profile, scope, bound, assumption or obligation
identifier is reachable from it, and `reason` keeps its exact text.
`ReasonCategory::classify` is the documented free-text mapping for string-only
rows; it is fail-closed, refusing an unrecognised string rather than defaulting
it — in particular never onto a category whose subject is the analysed source.

| Boundary | Current value | Meaning |
|---|---|---|
| Rust schema crate | 6.0.0 | Public API and serde compatibility |
| Plugin protocol | 1 | Unchanged envelope |
| Evidence | 1 and 2 | Unchanged negotiation and admission rules |
| Reason vocabulary | L21.1, closed at 7 | Adding a category is a compatibility event |

The unit semantic profile is `qcore.c_boolean_field_contract.v3`: two initial
Boolean states, no scalar result, a nominal record from the opaque import, and
`field`/`old_field` predicates. A path's numeric return slot is a zero placeholder
only; no unit contract can inspect it. The admitted checker is
`qcore.boolean_field_contract.v2`. Version-1 checkers remain usable for old I32
contexts. No protocol-v1 or ordinary evidence-v2 envelope changes, no new claim of
source authenticity, and no change to unrelated witness admission follows.

## Teardown source projections (7.0.0)

Version 7.0.0 adds `teardown_summary`: a portable projection of imported typed
paths for two bounded ChunkIO teardown families — the intrusive-list unlink
(an entry function together with the complete body of the single helper it
calls) and the nullable-field backend destructor — plus the proposed readings
a producer returns for them.

**Why the bump is forced.** Two of the three additions are source-incompatible
for exhaustive matches and struct literals, exactly as in 4.0.0 and 5.0.0:

| Addition | Kind |
|---|---|
| `SemanticWitness::TeardownContract` | new variant on an exhaustively matched enum |
| `ObligationProperty::{ListUnlinkContract, FieldDestructorContract}` | new variants on an exhaustively matched enum |
| `VerificationRequestV2::teardown_source` | new required struct field (skipped when absent) |

No existing variant was reused. `ContractObservation` is a recorded-run record
set and would misdescribe both what this witness is and what accepting it
means; `CounterContract` carries a single-path unsigned update and cannot hold
a multi-path release enumeration. The projection could technically have
travelled as JSON inside a `NormalizedFact::subject`, but that encoding has no
`deny_unknown_fields`, no schema-version field and no `validate()` rule, so it
would be a strictly weaker boundary than the typed field the three earlier
projections already established.

**Existing wire bytes are unchanged.** `teardown_source` is an `Option` that
defaults to absent and is skipped on serialization, exactly like the three
projections before it; `crates/qcore-schemas/tests/teardown_request_wire.rs`
pins that a request carrying none of the four projections serializes with none
of the four keys and round-trips.

`PathBounds::BoundedPaths` is now emitted for the first time: the destructor
family is a multi-path body, and the exact path count is part of its task, so a
re-import with a different path set is a different task. The variant, its serde
representation and its meaning are unchanged; only the doc comment saying no
derivation emits it was corrected.

| Boundary | Before | After | Why |
|---|---|---|---|
| Rust schema crate | 6.0.0 | **7.0.0** | Exhaustive-variant precedent from 2.0.0, 4.0.0 and 5.0.0 |
| Plugin protocol | 1 | 1 | Unchanged |
| Evidence version | 1, 2 | 1, 2 | Unchanged |
| Teardown source input | — | 1 | New |
| List unlink semantic profile | — | `qcore.c_list_unlink_contract.v1` | New |
| Field destructor semantic profile | — | `qcore.c_field_destructor_contract.v1` | New |
| Teardown input digest key | — | `qcore:teardown-source-input:v1` | New |
| ChunkIO package | 0.5.0 | **0.6.0** | Adds the two capability routes |

A projection carries the *question* only. For the destructor family that
includes the owned-field inventory the pinned contract declares, because the
inventory selects which nullable states must be enumerated; the trusted core
re-reads that inventory from the contract itself and refuses a task whose
projection names any other set. No derived reading — no case list, no write
list, no release order — ever travels in a request.

## Call-result def/use projections (8.0.0)

Version 8.0.0 adds `call_use_summary`: the **third** transported source
vocabulary, for the bounded ChunkIO filesystem cleanup-attempt family, plus the
proposed def/use reading a producer returns for it and the identity a
*dependent* capability may carry for its prerequisite.

**Why it is its own vocabulary.** This capability's evidence is which call
results a body ignores and which it guards, in operation order — a def/use
reading, not a path replay. The Boolean and counter projections carry a path so
the receiver can evaluate it to a field state, which discards the ordering a
def/use reading needs. The teardown projection has no `ReturnValue` and no
`LocalStart`, and its call table is closed to one co-transported helper or
exactly `free`; widening those enums would weaken a family whose closed table is
the whole point. So this module shares no type with either.

**Why the bump is forced.** Three of the four additions are source-incompatible
for exhaustive matches and struct literals, exactly as in 4.0.0, 5.0.0 and
7.0.0:

| Addition | Kind |
|---|---|
| `SemanticWitness::CallResultUse` | new variant on an exhaustively matched enum |
| `ObligationProperty::CallResultUseContract` | new variant on an exhaustively matched enum |
| `VerificationRequestV2::call_use_source` | new required struct field (skipped when absent) |
| `call_use_summary` module | purely additive |

**Existing wire bytes are unchanged.** `call_use_source` is an `Option` that
defaults to absent and is skipped on serialization, exactly like the four
projections before it; `crates/qcore-schemas/tests/call_use_request_wire.rs`
pins that a request carrying none of the five projections serializes with none
of the five keys and round-trips.

| Boundary | Before | After | Why |
|---|---|---|---|
| Rust schema crate | 7.0.0 | **8.0.0** | Exhaustive-variant precedent from 2.0.0, 4.0.0, 5.0.0 and 7.0.0 |
| Plugin protocol | 1 | 1 | Unchanged |
| Evidence version | 1, 2 | 1, 2 | Unchanged |
| Call-use source input | — | 1 | New |
| Call-use semantic profile | — | `qcore.c_call_result_use.v1` | New |
| Call-use input digest key | — | `qcore:call-use-source-input:v1` | New |
| Prerequisite identity digest key | — | `qcore:prerequisite-identity:v1` | New |
| ChunkIO package | 0.6.0 | **0.7.0** | Adds the call-result def/use route |

The projection carries the *question* only, and this one carries less than the
teardown projections did: the contract's policy selection — which callee is
counted, which second callee, which signature — never leaves the trusted core,
because the reading reports every call of every path. No derived reading — no
call inventory, no use lists, no fact vector, no verdict — ever travels in a
request.

`PrerequisiteIdentity` is the only thing a dependent capability may carry for a
prerequisite it consumes: a semantic profile, a task digest and an obligation
identifier, all of which the trusted core recomputes from its own import before
re-running the prerequisite's own evaluation. It carries no verdict, no
certificate and no reading, and a `deny_unknown_fields` test pins that.

## Symbolic cleanup-replay projections (9.0.0)

Version 9.0.0 adds `cleanup_replay_summary`: the **fourth** transported source
vocabulary, for the two bounded ChunkIO cleanup-outcome families — the native
cleanup outcome (`close`/`munmap`) and the guarded delete (`unlink`) — plus the
proposed symbolic replay a producer returns for them.

**Why it is its own vocabulary.** This capability's evidence is neither a path
evaluation nor a def/use fold. The reading a producer must compute — which
single path each sampled prestate selects, the return value, the call counts and
the record's field transition (`field_writes`, `fields_preserved`,
`fields_reset`) — is a function of a symbolic *field state* evolved by a
nine-constructor value lattice over four (respectively five) sampled prestates,
with the syscall's argument identity checked against the initial field values.
That is a symbolic replay evaluator. By P04-NEXT's own rule, a producer
reimplementing an algorithm over a new transported vocabulary is a separate
capability definition, not a widening of an existing one, so this module shares
no type with the other three — its step vocabulary differs too (no bare
`Return`: a cleanup path without a terminal `ReturnValue` is not a complete
path in either family).

**Closed by construction.** `CleanupLatticeValue` has exactly the nine
constructors the trusted checker's own value domain has, with no catch-all;
`CleanupReplayOperationRecord` has exactly the eight step forms the replay
executes; and `CleanupPrestateRecord`'s two variants fix each family's complete
sampled domain, so a producer cannot answer over a smaller or differently
sampled one.

**Why the bump is forced.** Three of the four additions are source-incompatible
for exhaustive matches and struct literals, exactly as in 4.0.0, 5.0.0, 7.0.0
and 8.0.0:

| Addition | Kind |
|---|---|
| `SemanticWitness::CleanupReplay` | new variant on an exhaustively matched enum |
| `ObligationProperty::CleanupReplayContract` | new variant on an exhaustively matched enum |
| `VerificationRequestV2::cleanup_replay_source` | new required struct field (skipped when absent) |
| `cleanup_replay_summary` module | purely additive |

**Existing wire bytes are unchanged.** `cleanup_replay_source` is an `Option`
that defaults to absent and is skipped on serialization, exactly like the five
projections before it; `crates/qcore-schemas/tests/cleanup_replay_request_wire.rs`
and the extended `call_use_request_wire.rs` pin that a request carrying none of
the six projections serializes with none of the six keys and round-trips.

| Boundary | Before | After | Why |
|---|---|---|---|
| Rust schema crate | 8.0.0 | **9.0.0** | Exhaustive-variant precedent from 2.0.0, 4.0.0, 5.0.0, 7.0.0 and 8.0.0 |
| Plugin protocol | 1 | 1 | Unchanged |
| Evidence version | 1, 2 | 1, 2 | Unchanged |
| Cleanup-replay source input | — | 1 | New |
| Cleanup-replay semantic profile | — | `qcore.c_cleanup_replay.v1` | New |
| Cleanup-replay input digest key | — | `qcore:cleanup-replay-source-input:v1` | New |
| ChunkIO package | 0.7.0 | **0.8.0** | Adds the symbolic cleanup-replay route |

The projection carries the *question* only: the complete typed path set, the
record profile the replay is defined over, and the prerequisite's identity
triple. No derived reading — no case vector, no field state, no return value,
no call count, no verdict — ever travels in a request, and neither does the
contract text or the policy name that selects it.

The transported `CleanupProfileRecord` is an **input** to the replay and
simultaneously a **claim**: the trusted core builds its own profile from the
pinned contract's `cleanup_vocabulary_v1` declaration (card P04-B1) and refuses
unless the transported record is exactly that. A producer therefore cannot
choose the record shape it answers over. The policy name is deliberately not a
member of the record, because which declaration the core reads is the core's own
selection.

## Contracts awaiting human review (15.0.0)

Card W45-CONTRACT-DEFERRED-REVIEW (contract specification C06.1). A declared
resource-v1 contract, or one clause of it, may be marked
`resource_review_pending`; the loader validates it and withholds it.

| Addition | Kind |
|---|---|
| `ContractTrust::PendingHumanReview` | new variant on an exhaustively matched enum |
| `ReasonCategory::AwaitingHumanReview` (`awaiting-human-review`, subject: analyser) | new variant on an exhaustively matched enum; the closed vocabulary grows from seven to eight |
| `PendingHumanReview`, `ReviewDeferral`, `ReviewScope` | new types |
| `ResourceContracts::pending_review` | new private field with accessors, skipped when empty |

The two new variants are source-incompatible for exhaustive matches, so this
is a major bump by the exhaustive-variant precedent of 2.0.0 through 9.0.0.
**Existing wire bytes are unchanged**: an environment with no marker
serializes without `pending_review`, no existing reason text classifies
differently, and no `DerivationIncomplete` or task digest moves (no producer
names the new category). Plugin protocol 1 and evidence versions 1 and 2 are
unchanged.

## Contract revision modes (16.0.0)

Card W45-CONTRACT-REVISION-MODE (contract specification C06.2). A declared
resource-v1 contract, or one clause of it, may say `resource_revision(pinned)`
or `resource_revision(flexible)`; pinned is the default. An audit withholds a
pinned contract whose lock the audited source does not match.

| Addition | Kind |
|---|---|
| `ReasonCategory::ContractPinMismatch` (`contract-pin-mismatch`, subject: analyser) | new variant on an exhaustively matched enum; the closed vocabulary grows from eight to nine |
| `RevisionMode`, `RevisionScope`, `RevisionMarker`, `DeclaredRevision`, `PinMismatchReason`, `PinMismatch` (module `contract_revision`) | new types |
| `ResourceContracts::revision` (`declared_revisions`, `revision_mode`) and `ResourceContracts::pin_mismatch` (`withhold_for_pin_mismatch`) | new private fields with accessors, each skipped when empty |

The new reason variant is source-incompatible for exhaustive matches, so this
is a major bump by the exhaustive-variant precedent. **Existing wire bytes are
unchanged** for data without a marker or a withheld pin: such an environment
serializes without either field, and no existing reason text classifies
differently (the projection summary's pin component is named only when
positive). Plugin protocol 1 and evidence versions 1 and 2 are unchanged.
