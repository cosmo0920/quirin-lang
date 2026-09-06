# W3 design: obligation inventory, evidence schema v2, and verify-scope dispatch

Date: 2026-09-06. Status: **design only** — this is the
[generalization plan](../GENERALIZATION_PLAN.md) §5 review checkpoint "after
the W3 evidence-schema v2 design (before implementation)". No Rust, test, or
Lean change accompanies this document. Implementation may begin only after
this design is reviewed and accepted; every schema type below becomes a
compatibility surface the moment it lands in `qcore-schemas`.

Governing constraints, restated as binding on this design:

- **Fail-closed everywhere** (plan §2.1). Nothing below converts an `Unknown`
  to `Safe` as a side effect. Every new path that cannot complete its check
  yields `Unknown`.
- **The v1 `Safe` refusal is permanent for v1.** The firewall's terminal
  rejection for evidence version 1 — including empty scopes and proposals
  whose Boolean claims exactly match the request — is retained verbatim.
  v2 acceptance is a new, separately keyed path; it does not touch the v1 arm.
- **Evidence-contract rule** (`REEVALUATION_SPEC.md` §6): *"The core MUST
  derive an obligation inventory from the normalized program. Plugins may
  propose witnesses, not the list of obligations whose absence would make
  their own proof succeed."* This sentence is the specification this design
  implements, not merely quotes.
- **Gate ordering** (`REEVALUATION_SPEC.md` §9): G1 certificate containment
  is recorded complete (2026-09-05 repair); this design is G2 work
  ("core-derived plugin obligations", semantic witness checkers). Cycle 19's
  existing analysis routes stay until G4 parity; nothing here removes or
  reroutes them. Raw Cycle 19 units remain **unpromoted**.
- **Verdict contract** (plan §2.4): every result carries
  `(artifact, property, semantic profile, scope, bounds, assumptions,
  evidence version)`; the `Safe`/`Unsafe`/`Unknown` vocabulary is exact.
- **Layering** (plan §3): the types below land in `qcore-schemas` (L0);
  obligation derivation and the per-capability witness checkers are trusted
  core code; the dispatch loop lands in the new `qcore-verifier-host` (L2)
  crate, which orchestrates but never decides.

Code studied for this design: `compiler/src/verifier_plugin.rs` (registry,
process provider, `check_evidence`, QP001–QP015), `crates/qcore-schemas/src/`
(`plugin.rs`, `contract.rs`, `identity.rs`, `mechanism.rs`),
`compiler/src/driver.rs:179-209` (Q700 scope resolution, Q701 rejection —
finding VP05), `compiler/src/chunk_lifecycle.rs`
(`check_chunk_program`, `execute_chunk_prefix`),
`crates/qcore-contract/src/contract_observation.rs`
(`check_contract_observation`), `crates/qcore-contract/src/contract_loader.rs`,
`compiler/examples/reevaluation_probe.rs` (VP01/VP02),
`compiler/examples/reevaluation_followup.rs` (VP04–VP08, LC01, WM05),
`docs/QCORE_SPEC_r0_2.md` §54 (the Q700–Q712 table) and the verify-scope
grammar note near §5.1.

---

## 1. Obligation inventory model (destined for `qcore-schemas`)

### 1.1 The versioned verification task (SV-LIB direction)

The reevaluation literature row for SV-LIB 1.0 directs: *version the complete
verification task and its semantics, not just the provider response envelope.*
Protocol v1 versions only the request/response pair. v2 introduces a
first-class task record. All types are serde types with
`deny_unknown_fields`, no analysis logic, in a new `qcore-schemas` module
`obligation.rs` (plus `witness.rs`, §2).

```text
VerificationTask {
    task_version: u16,                  // starts at 1; bumped with semantics
    unit: UnitIdentity,                 // raw + normalized source/build digests
    target: TargetProfile,              // library, revision, profile, dialect,
                                        // platform, backend
    semantic_profile: String,           // named checker profile, e.g.
                                        // "qcore.chunk_lifecycle.v1"
    contract_sources: BTreeMap<String, String>,
                                        // path -> sha256 of each trusted
                                        // contract/profile input assumed
    bounds: TaskBounds,                 // see 1.3
    assumptions: BTreeMap<String, bool>,
    evidence_version: u16,              // schema version witnesses must speak
    obligations: Vec<Obligation>,       // the core-derived inventory
}
```

The task is digested (`task_digest`: SHA-256 over the canonical JSON
serialization; all maps are `BTreeMap`, so serialization is deterministic) and
that digest travels in every v2 request and every witness. A witness bound to
a different `task_digest` than the one the core computed is rejected before
any capability-specific checking. The cache-key rule of reevaluation §6
("cache keys MUST include every field affecting semantics") is satisfied by
keying on `task_digest`; a missing input identity is `Unknown`, never an
empty-string default.

`VerificationRequest` gains (in its v2 form; see §3.4 for compatibility):
`task_digest: String` and `obligations: Vec<Obligation>`. The existing
`facts: Vec<NormalizedFact>` field is retained for envelope compatibility but
is not the obligation carrier in v2; the obligations are.

### 1.2 Obligation identity

```text
Obligation {
    obligation_id: String,       // deterministic; see below
    capability: VerifierCapability,
    property: ObligationProperty, // per-capability enum, 1.4
    subject: ObligationSubject,   // what artifact the property is about
    scope: ObligationScope,       // which part of the unit is covered
    bounds: TaskBounds,           // may narrow the task-level bounds
    assumptions: BTreeMap<String, bool>,
    evidence_version: u16,
}
```

Every obligation therefore carries the full verdict-contract tuple:
`subject` + `scope` = artifact and scope, `property` = property,
`VerificationTask.semantic_profile` = semantic profile, `bounds` = bounds,
`assumptions` = assumptions, `evidence_version` = evidence version.

`obligation_id` is computed by the core, never accepted from a provider:
`sha256(task-identity fields ++ capability ++ property ++ canonical subject)`,
truncated to a fixed hex length with the capability name as a readable prefix
(e.g. `chunk_lifecycle:3fa9c2…`). Determinism requirements: two derivations
over the same normalized unit and profile produce identical IDs; any change to
the source digest, profile, or derivation version changes the task digest.
IDs are unique within a task (a schema `validate()` enforces this, mirroring
`VerificationRequest::validate`'s duplicate-fact check).

```text
ObligationSubject =
    Function { symbol, translation_unit }
  | AllocationSite { function, site_index, resource_kind }
  | ChunkEntity { entity: Context(id) | Stream(id) | Chunk(id) }
  | ObservedContract { function, contract_module }   // path into contract/qcore
  | MutexIdentity { … } | TaskIdentity { … }          // reserved, 1.4.4
  | Unit                                              // whole-TU subjects

ObligationScope =
    WholeUnit
  | Functions { symbols: BTreeSet<String> }
  | ObservedRuns { run_ids: BTreeSet<String> }        // observation capability
```

### 1.3 Bounds

```text
TaskBounds {
    path_exploration: PathBounds,     // e.g. SinglePath | BoundedPaths(n)
    schedules_explored: Option<u64>,  // concurrency; None = not applicable
    observed_runs_only: bool,         // true for observation obligations
    max_operations: Option<u64>,      // trace-length bound for replay
}
```

Bounds are part of the claim, not metadata: a `Safe` on an obligation with
`observed_runs_only: true` is "checked for the recorded runs", per the
"Observed agreement" row of reevaluation §1 — it is never presented as
static source safety, and it can never discharge an obligation whose bounds
say otherwise (the firewall compares bounds field-by-field, §3.3).

### 1.4 Per-capability obligation derivation

The derivation functions are **trusted core code** (they extend the existing
checkers; they do not move to the host or to any provider). What follows
specifies, per capability, what the core derives from a normalized
translation unit (C path) or typed program (QCore path). Derivation itself is
fail-closed: a construct the deriver cannot classify produces a
`DerivationIncomplete` marker on the task (not a silent omission), and a task
with that marker can never reach `Safe` for the affected capability.

**1.4.1 Resource contracts** (`resource_contracts`). Input: the normalized
unit's per-function CFGs (the existing `legacy_c` import) plus the loaded
`ResourceContracts` environment (W1 loader; its exact source texts enter
`contract_sources`). Derived obligations:

- per contracted acquire site reachable in an analyzed function: a
  `CustodyDischarged` obligation (subject `AllocationSite`) — every path from
  the site releases, escapes, stores under a live owner, or returns the
  resource;
- per function parameter with a declared mode: a `ParameterModeRespected`
  obligation;
- per function analyzed: a `NoUnknownCallEffect` obligation — every call is
  contracted, summarized, or the obligation is undischargeable (stays
  `Unknown`), matching today's fail-closed call handling.

This capability initially has **no v2 witness kind** (see §2.5): the
obligations exist so inventories are complete and denominators honest, but
requests for them return `Unknown` exactly as today. W4's validated summaries
are the intended discharge route; v2 providers do not get a shortcut around
LC01's open boundary.

**1.4.2 Chunk lifecycle** (`chunk_lifecycle`). Input: the core-imported
lifecycle call inventory of the unit — the set of call sites (with file,
span, callee, and argument identity) whose callees appear in the core's
lifecycle-relevant API table (from the W1/W2 contract and profile data, not
from the provider). Derived obligations, phrased against the
`chunk_lifecycle` model's terminal conditions:

- per created context: `ContextDischarged` (context destroyed, chunk counter
  zero — the `ContextStillLive` / `ChunkCounterMismatch` terminal checks);
- per created stream: `StreamDischarged` (`StreamStillLive`);
- per opened chunk: `ChunkDischarged` (chunk closed, backend resources and
  name released, list links removed — `ChunkStillLive`,
  `BackendResourceStillLive`, `ListLinkStillLive`);
- per transaction begun: `TransactionSettled`;
- one `LifecycleProgramComplete` obligation for the unit: the checked
  operation trace accounts for **every** lifecycle-relevant call site the
  core discovered, exactly once.

Where the core cannot yet build this inventory (unsupported construct,
indirect call to a lifecycle API, unresolvable argument identity), the task
carries `DerivationIncomplete` and the capability stays `Unknown` for the
unit. Entity identities in subjects are core-assigned indices over the core's
own call-site enumeration — a provider never names an entity the core did not
already number.

**1.4.3 Contract observations** (`observation_hooks`). Input: the contract
modules selected for the unit (their digests in `contract_sources`) and the
manifest/profile-declared required observation hooks. Derived obligations:

- per `extern fn … contract { requires …; ensures normal …; }` declaration
  whose hook is required: an `ObservationChecked` obligation (subject
  `ObservedContract`), with `bounds.observed_runs_only = true` and scope
  `ObservedRuns` filled in at check time from the witness's run identities;
- one `ObservationCoverageComplete` obligation: every required hook has at
  least the declared minimum number of observations.

Discharge of these obligations claims **observed agreement only** — the
"Recorded executions and an observation relation" row of reevaluation §1.
It never discharges a `resource_contracts` obligation and is never a
universal statement about the function.

**1.4.4 Event traces and concurrency** (`event_traces`, `concurrency`).
Obligation shapes are reserved in the enum now so the inventory model does
not need a breaking change later: per-event generation-lifecycle discharge,
callback-quiescence-at-teardown, per-mutex guard release, join-before-exit.
No derivation and no witness kind in this design's first increment; requests
return `Unknown` with the existing diagnostics. Defining them now costs one
enum variant each and prevents a v3 schema bump for the Cycle 18/W5
extraction.

**1.4.5 Translation-unit projection** (`translation_unit_projection`).
One `ProjectionAccounted` obligation per unit: every discovered function is
classified projected or explicitly unprojected, with the projection
inventory recomputed by the core from its own Clang-gateway discovery.
Projection coverage remains distinct from safety coverage; discharging this
obligation moves nothing out of the `Unknown` denominator.

---

## 2. Evidence schema v2: semantic witnesses the core replays

### 2.1 Versioning and negotiation

`qcore-schemas` gains `PLUGIN_EVIDENCE_VERSION_V2: u16 = 2`. The protocol
frame format, `PLUGIN_PROTOCOL_VERSION = 1`, and the four-byte length
framing are unchanged (plan §6: "evidence moves to v2 within the existing
negotiation"). Manifests advertise v2 by listing `2` in `evidence_versions`;
the existing QP004 mismatch check already gates this. A provider that
declares only v1 can never be asked for, nor supply, a v2 witness.

`ProposedEvidence` is extended with one optional field:

```text
witness: Option<SemanticWitness>     // required iff evidence_version == 2
```

serialized with `#[serde(default, skip_serializing_if = Option::is_none)]`
so every existing v1 frame remains byte-compatible. Firewall keying:

- `evidence_version == 1` → the existing `check_evidence` path runs
  unchanged and still ends in its terminal rejection. **v1 `Safe` proposals
  remain refused, including empty scopes.** If a v1 envelope smuggles a
  `witness` field, the envelope is rejected (QP009) — a witness under the
  wrong version is not evidence.
- `evidence_version == 2` → the envelope checks of v1 (identity, coverage,
  assumptions echo, reproducibility, claim matching) run first and remain
  **necessary but never sufficient**; then the capability-specific witness
  checker runs. Only the checker's acceptance can produce a
  `CoreCertificate`.
- `witness` absent under v2, or a witness whose kind does not match the
  request capability, or a capability with no registered core checker
  (resource contracts, event traces, concurrency, projection in this
  increment) → `Unknown` (new reason code QP016, "no core witness checker
  for this capability/evidence version" — invalid evidence, reported
  distinctly per reevaluation §1's reason-code rule).

In v2, `coverage` refers to `obligation_id`s from the core inventory, not
submitted fact counts — implementing reevaluation §6's "coverage must refer
to the core inventory" sentence. A `Safe` verdict requires coverage equal to
the full obligation set of the request; partial coverage is `Unknown`.

### 2.2 The anti-vacuousness design rule

VP01/VP02's root cause was evidence that restated the request: equality
between provider-echoed fields and request fields was the entire check, and
an empty `.all(...)` was vacuously true. v2 adopts one structural rule,
checked in review for every witness kind:

> **No acceptance path may terminate in an equality comparison with
> request-supplied or provider-supplied data. Every acceptance path must
> terminate in a core recomputation over core-derived inputs, and the empty
> case of every quantifier must be explicitly decided against a
> core-computed denominator.**

Concretely: obligation sets come from the core's derivation (never the
witness); completeness is checked against the core's own call-site or hook
inventory (never the witness's list of what it covered); the semantic
verdict comes from re-executing a trusted core checker
(`check_chunk_program`, `check_contract_observation`) on the witness data;
and "nothing to check" is representable only as the core's own
`DerivationIncomplete`/empty-inventory record, never as an empty witness
(§3.5, open question 1).

### 2.3 Witness kind 1: `ChunkLifecycleWitness`

Serves `VerifierCapability::ChunkLifecycle`, replayed against
`chunk_lifecycle::check_chunk_program` — a checker the trusted core already
owns, which is why this capability is first: v2 `Safe` promotion is real
from the first provider without writing a new trusted checker.

```text
ChunkLifecycleWitness {
    task_digest: String,
    schedule: Vec<BoundOperation>,
}

BoundOperation {
    operation: ChunkOperationRecord,   // schema-side mirror of ChunkOperation
                                       // (serde enum; the compiler keeps its
                                       // internal type and converts)
    site: SourceSite,                  // file, byte span, callee symbol,
                                       // argument identity index
}
```

**What the provider supplies:** the ordered schedule — its genuine expertise
is mapping the library's C API surface onto the lifecycle model (which callee
is an open, which teardown cascades, which argument names which entity) and
choosing the order in which the unit's call sites execute.

**What the core recomputes (all of it):**

1. *Site validity.* Every `site` must name a call site in the core's own
   lifecycle call inventory (1.4.2), with the callee and argument-identity
   the core recorded. A site the core did not discover → reject.
2. *Completeness.* Every call site in the core inventory appears in the
   schedule exactly once. Omission or duplication → reject. The denominator
   is the core's inventory, so the empty case is decided by the core: an
   empty schedule is acceptable only when the core's inventory is empty, and
   that case is governed by §3.5, not by the witness.
3. *Operation faithfulness.* The `operation` claimed for each site must be
   the one the core's API table maps that callee to, with entity identities
   consistent with the core's argument-identity numbering. A provider cannot
   relabel a close as an open or retarget it at a different chunk.
4. *Order admissibility.* The schedule's per-function subsequences must embed
   into a single path of that function's core-built CFG (first-increment
   bound: `PathBounds::SinglePath`; multi-path units are `Unknown`, recorded
   as an explicit bound, see open question 5). Cross-function order is
   admissible only along the core's observed call structure.
5. *Semantic replay.* The core runs `check_chunk_program` on the converted
   schedule. Model-rule violations (`InvalidTransition`, counter mismatch,
   …) or any terminal diagnostic → reject.
6. *Obligation discharge.* Each obligation in the inventory is mapped to its
   terminal condition over the replayed final state; all must discharge, and
   the witness's `coverage` must equal the full obligation-ID set.

**Vacuousness argument** ("could a malicious provider make the core's check
vacuous?"): the provider chooses only an ordering among core-verified
operations at core-discovered sites. It cannot shrink the check by omitting
sites (completeness is against the core inventory), cannot pad it with
fictitious discharging operations (site validity + faithfulness), cannot
reorder across what the CFG permits (order admissibility), and cannot
influence which properties are checked (obligations and terminal conditions
are core-derived). The residual provider freedom — choosing among admissible
orders — is exactly the freedom the claim is scoped to: the certificate's
bounds say "the checked schedule(s)", and a unit whose admissible orders the
core cannot bound stays `Unknown`. An empty witness proves nothing because
the empty-schedule case is decided by the core's own inventory, not by the
witness (the VP02 lesson made structural).

### 2.4 Witness kind 2: `ContractObservationWitness`

Serves `VerifierCapability::ObservationHooks`, replayed against
`check_contract_observation` in `qcore-contract`.

```text
ContractObservationWitness {
    task_digest: String,
    runs: Vec<ObservedRun>,
}

ObservedRun {
    run_id: String,
    binding: NativeRunBinding {        // reevaluation §6 native-observation set
        emitted_source_digest, compiler_invocation, executable_digest,
        exit_status, seed_or_input, profile, instrumentation_version,
    },
    observations: Vec<ContractObservation>,
}

ContractObservation {
    function: String,
    hook: String,                       // the required-hook name it satisfies
    values: BTreeMap<String, i128>,     // the recorded fact values
}
```

**What the provider supplies:** recorded runtime facts (`values`) and the run
identity binding. That is all.

**What the core recomputes (all of it):** the contract text comes from the
core's trusted `contract_sources` (digest-checked against the task), never
from the witness; per observation, the core evaluates every `requires` and
`ensures normal` predicate with its own interpreter — unsupported predicate
syntax, missing variables, and overflow all fail closed inside `evaluate`;
verdict mapping is core-side: a false `ensures` is an observed contract
violation (the obligation cannot discharge and the response is at best a
provider `Unsafe` allegation, still downgraded per §3.3); a false `requires`
is `OutsidePreconditions` and does **not** count toward coverage; hook
coverage (`ObservationCoverageComplete`) is counted by the core against the
manifest/profile-declared required-hook set, with `OutsidePreconditions`
observations excluded from the count.

**Vacuousness argument:** the provider cannot choose or weaken the predicates
(they are parsed from digest-pinned trusted contract sources), cannot skip a
required hook (coverage is counted by the core against the declared hook
set, and preconditions-dodging observations do not count), and cannot make
evaluation trivially succeed (missing observation variables are an error,
not a skip — the interpreter evaluates all clauses before testing
`requires`, so an unsupported predicate cannot hide behind a false
precondition). What the provider *can* do is fabricate `values`; the design
answers this honestly rather than pretending otherwise: recorded
observations are a **declared trusted assumption** (plan invariant §2.6),
so the certificate carries the full `NativeRunBinding` and an explicit
assumption `observations_trusted: true`, and the obligation's bounds carry
`observed_runs_only: true` with scope `ObservedRuns{run_ids}`. The core's
check is therefore never vacuous — it genuinely evaluates core-chosen
predicates over the supplied facts and genuinely counts core-required
coverage — and the part that is trust rather than proof is named in the
certificate instead of being laundered into `Safe`-without-qualification.
Discharge here claims observed agreement for the bound runs; it is not, and
is never composed as, static source safety.

### 2.5 Capabilities without a v2 witness kind

`resource_contracts`, `event_traces`, `concurrency`, and
`translation_unit_projection` have obligation shapes (§1.4) but no witness
kind in this increment. A v2 `Safe` proposal for them hits QP016 and stays
`Unknown`. This is deliberate: resource-contract discharge must come from
W4's validated interprocedural inference (LC01's boundary), not from a
provider-proposed custody story the core cannot yet recheck; event/
concurrency witnesses need the Cycle 18 model extraction (W5) first.
Extending `SemanticWitness` with a new kind is a minor, additive schema
change; changing an existing kind's meaning requires an evidence-version
bump.

---

## 3. Dispatch semantics for resolved verify scopes (closes VP05)

### 3.1 Pipeline

Today `driver.rs:198-209` rejects every resolved scope with Q701
("configuration-only"). W3 replaces that rejection with dispatch, in the new
`qcore-verifier-host` crate (absorbing `verifier_plugin.rs`'s registry/
process/composition halves; the firewall's per-capability witness checkers
stay on the trusted core side of the crate boundary):

```text
resolved scope (resolve_source_scopes — unchanged, still fail-closed)
  → per (provider use, capability):
      core derives the ObligationInventory / VerificationTask   (trusted core)
      → empty-or-incomplete derivation handling                  (§3.5)
      → VerificationRequest (protocol 1, evidence_version 2,
        task_digest, obligations, bounded source facts)
      → registry.verify: manifest gates QP001–QP005 (unchanged)
      → provider runs (in-process adapter or bounded process — unchanged
        spawn/deadline/output machinery; VP04/VP07/VP08 behavior untouched)
      → firewall: v1/v2 keying (§2.1), witness checker, QP006–QP016
  → per scope: compose_verification over all (provider, capability) outcomes,
    plus the required-set check (§3.3)
  → compile diagnostics attached to the compilation artifact
```

Dispatch happens during `compile_files_with_registry` at the point of
today's Q701 rejection; `compile_files` (no registry) continues to resolve
scopes against the default registry and, for any scope needing an
unavailable capability, fails closed with the same diagnostics.

### 3.2 Exact behavior per failure mode

| Condition | Protocol outcome | Scope outcome |
|---|---|---|
| Provider absent / not installed | QP002 `Unknown` | scope unverified: Q701 (§4 meaning: undischarged obligations), carrying the QP diagnostics |
| Provider disabled | QP003 `Unknown` | same |
| Protocol/evidence version mismatch | QP004 `Unknown` | same |
| Capability/target undeclared | QP005 `Unknown` | same |
| Spawn failure, crash, timeout, output limit, malformed frame | QP006 `Unknown` (existing `ProviderFailure` mapping; deadline and process-group behavior unchanged) | same |
| Provider verdict `Unknown` | passed through as `Unknown` with provider diagnostics (allegations) | same |
| Provider verdict `Unsafe` | **QP010 downgrade retained**: v2 ships no counterexample checker, so an unchecked `Unsafe` proposal remains an allegation and the outcome is `Unknown` with the allegation diagnostics attached | scope unverified; the allegation is surfaced, never presented as a validated violation |
| Provider `Safe`, evidence v1 | refused (existing terminal rejection) → `Unknown` | same |
| Provider `Safe`, evidence v2, no/mismatched witness or no core checker | QP009/QP016 `Unknown` | same |
| Provider `Safe`, evidence v2, witness rejected by core replay | QP009 `Unknown` with the replay diagnostic | same |
| Provider `Safe`, evidence v2, witness accepted, all obligations covered | `Safe` + `CoreCertificate` for that (capability, unit) | contributes to scope success |
| Any obligation of the scope without a `Safe` outcome | — | scope unverified: Q701 |
| Conflicting assumptions / claims across providers | QP012/QP013 `Unknown` (existing composition) | scope unverified: Q702 (§4) |

There is no retry, no partial promotion, and no optimistic path: a scope is
verified only when **every** obligation of **every** requested capability is
discharged by a core-checked certificate, and the composed certificate set
covers the scope's full required capability set.

### 3.3 Aggregation and composition

`compose_verification`'s existing conflict rules (QP011–QP015) are kept and
extended with the reevaluation §6 requirements that v1 could not express:

- **Required-set validation:** composition takes the scope's obligation
  inventory as an explicit input and verifies the union of certificate
  coverages equals the full obligation-ID set. A missing required provider
  or capability is therefore detectable from the inventory, not inferred
  from an absent certificate (FM02's noted gap).
- **Assumption and bounds preservation:** the composed result carries every
  component's assumptions and bounds; conflicting assumptions stay QP012
  `Unknown`; incomparable bounds (e.g. one `observed_runs_only` certificate
  and one path-bounded certificate over the same obligation) do not merge —
  each obligation must be discharged under a single coherent bound.
- **Verdict precedence:** unchanged from the follow-up containment — an
  unchecked `Unsafe` never takes precedence (it is already downgraded before
  composition); a validated negative witness is future work.
- The composite result is the `CompositeCertificate` collection; no leading
  certificate is presented as certifying the whole composition.

### 3.4 Compatibility and regression rules

- **v1 `Safe` stays refused — regression-tested.** The v1 arm of
  `check_evidence`, including its final
  `Err("protocol v1 lacks core-derived obligations and capability-specific
  semantic witnesses")`, is preserved verbatim. VP01 and VP02 keep passing
  against the same probe source. No refactor may route a v1 envelope into
  the v2 checker.
- **VP05's accepted boundary advances, not flips.** The followup probe
  passes on "dispatch happens (`invocations > 0`) **or** explicit Q701".
  After W3, the counting provider is invoked (its `Unsafe` is downgraded,
  the scope stays unverified with Q701), so the probe passes on the dispatch
  arm without edits. The probe source is not modified by W3.
- **VP04/VP07/VP08** exercise process deadline, frame-budget agreement, and
  encoding-failure containment; W3 does not change framing, limits, or the
  process loop, so these hold by construction.
- **VP06** (unsupported `Unsafe` proposal → `Unknown`) is preserved by the
  QP010 retention above.
- **Q700 scope-resolution diagnostics** (unresolved scopes) are unchanged.
- Existing Cycle 19 analysis routes and the Cycle 19 audit example are not
  rerouted through dispatch (G4 parity gate); v2 dispatch serves `verify
  scope` declarations in QCore source first.

### 3.5 Empty and incomplete inventories

Reevaluation §6: "an empty scope is safe only with an independently checked
no-obligations argument." Under v2 the core derives the inventory itself, so
an empty inventory *is* core-established — but only as strong as the
derivation's coverage. First increment, deliberately conservative:

- `DerivationIncomplete` on the task → the capability's scope outcome is
  `Unknown`, always.
- A complete derivation with zero obligations → **still `Unknown`** in the
  first implementation, with a dedicated diagnostic ("no obligations
  derived; empty-scope promotion requires a derivation-coverage witness"),
  and no request is sent to any provider (nothing to discharge means nothing
  for a provider to do). Promotion of legitimately empty scopes is deferred
  until the derivation records a checkable coverage artifact (open
  question 1). This costs completeness, never soundness.

---

## 4. Resolving the Q700/Q701 collision with spec §54 (W7 first bullet)

**The collision.** Spec §54 allocates `Q700`–`Q712` to Libevent-specific
event/callback custody diagnostics ("event/base lifecycle transition is
invalid or stale", "used after free", …). Cycle 20 independently uses `Q700`
(scope-resolution failure, `driver.rs:189`, spec §5.1's verify-scope note)
and `Q701` (configuration-only rejection, `driver.rs:204`). Two unrelated
meanings share two codes.

**Proposal: the core keeps Q7xx; §54's Libevent table moves to the provider
namespace.** Rationale: plan W7 already states the rule — "the core
allocates no numbers for library-specific conditions", provider diagnostic
namespaces belong to provider manifests, and the firewall *already enforces*
that every provider diagnostic code is prefixed `<plugin_id>.`
(`check_response`'s envelope check). The §54 table is library-specific by
its own wording, so it is the half that moves.

Concrete allocation:

- **Core `Q700`–`Q719` — verify-scope resolution and dispatch** (reserved
  once, in one changelog entry):
  - `Q700` — verify scope failed to resolve against the registry
    (unchanged meaning).
  - `Q701` — verify scope unverified: one or more core-derived obligations
    were not discharged by core-checked evidence (the W3 successor of
    today's "configuration-only" message; same code, strictly stronger
    system behind it, so no pinned expectation weakens and VP05's Q701 arm
    remains truthful during migration).
  - `Q702` — verify scope unverified: providers returned conflicting
    evidence or assumptions (surfaces QP012/QP013 at the language level).
  - `Q703` — verify scope carries an unverified provider allegation
    (surfaces a QP010-downgraded `Unsafe` proposal as an allegation
    attached to the compilation result).
  - `Q704`–`Q719` — reserved, unallocated.
- **Libevent §54 diagnostics become `qcore.libevent.L00`–`L12`**, owned by
  the Libevent provider manifest, preserving their order and one-line
  meanings (`Q700`→`L00` "event/base lifecycle transition is invalid or
  stale", … `Q712`→`L12` "observed Libevent trace does not refine the
  checked event model"). The mapping table ships in the provider's manifest
  documentation; archived Cycle 18 reports are **not** rewritten (they
  retain their dates and gain a superseding-link note, per the
  reevaluation's historical-record rule).

**Required spec-changelog entry** (`CHANGELOG_r0_2.md`, verbatim proposal):

> §54 amendment (W7/W3): the `Q700`–`Q712` identifiers previously listed for
> Libevent event/callback diagnostics are reassigned to the provider
> namespace as `qcore.libevent.L00`–`L12`; provider manifests own
> library-specific diagnostic namespaces and the core allocates no numbers
> for library-specific conditions. `Q700`–`Q719` are reserved for core
> `verify scope` resolution and dispatch diagnostics; `Q700` (scope
> resolution failure) and `Q701` (scope obligations undischarged) are
> allocated. §54's diagnostic guidance sentence is unchanged. This is a
> deliberate renumbering of a pinned table, recorded here per the
> pinned-expectations invariant.

The spec §54 table itself is edited in the same change that lands the
renumbering (an implementation task, not part of this design's file set).

---

## 5. Test plan (plan §W3 task 4) and Lean direction (task 5)

### 5.1 Rust tests

**Derivation determinism and identity.**
- Same normalized unit + profile derived twice → identical obligation IDs,
  identical `task_digest`.
- Any single change to source digest, contract source, profile, bounds, or
  derivation version → different `task_digest`.
- Duplicate obligation IDs rejected by task validation.
- `DerivationIncomplete` present → capability outcome `Unknown` regardless
  of any provider response.

**Dispatch (upgrading the VP05 boundary).**
- Nonempty obligations + provider absent / disabled / version-mismatched /
  crashing / timing out → scope unverified with Q701 plus the mapped QP
  diagnostic; no optimistic fallback.
- Counting provider proves dispatch happens: `verify scope` compilation
  invokes the provider (invocations > 0) and, with an `Unknown` response,
  the scope stays unverified (per reevaluation VP05's own instruction to
  test with an `Unknown`-returning provider, not just manifest resolution).
- Conflicting two-provider evidence → Q702; allegation surfacing → Q703.
- Empty derived inventory → `Unknown` with the §3.5 diagnostic and zero
  provider invocations.

**v2 witness round-trip and mutation corpus (per capability).**
- Chunk lifecycle, positive: in-process provider returns a valid schedule
  for a fixture unit → `Safe`, a `CoreCertificate` whose coverage equals the
  inventory, and a verified scope. Mutations, each → `Unknown` at the
  firewall with a distinct reason: omitted site; duplicated site; phantom
  operation at an undiscovered site; relabeled operation (close as open);
  retargeted entity identity; order not embeddable in the CFG path; schedule
  valid but a terminal diagnostic remains (e.g. a stream left live); wrong
  `task_digest`; witness under `evidence_version: 1`; v2 envelope with
  `witness: None`; coverage missing one obligation ID.
- Contract observations, positive: witness with complete hook coverage and
  passing predicates → `Safe` with `observed_runs_only` bounds and the run
  bindings in the certificate. Mutations: missing required hook; observation
  with a missing variable; failing `ensures` (allegation path, still not
  `Safe`); all observations `OutsidePreconditions` (coverage not met);
  tampered contract-source digest; predicate-syntax injection via values
  keys (must be inert — values are data).
- Cross-capability: a v2 `Safe` for `resource_contracts` (no checker) →
  QP016 `Unknown`.

**Regression: v1 `Safe` stays refused.** The existing probe sources
(`reevaluation_probe.rs`, `reevaluation_followup.rs`) run unmodified with
exit 0; additionally a direct test pins that a v1 envelope whose Boolean
claims exactly match the request still ends in the terminal v1 rejection
after the v2 code lands (no special-casing of probe identifiers).

**Out-of-process conformance provider.** A new test-only binary crate (e.g.
`crates/qcore-conformance-provider`, excluded from default install
artifacts) whose **only workspace dependency is `qcore-schemas`** (plus
serde/serde_json), proving the schema crate suffices to implement a
provider. It speaks length-delimited JSON over stdio: reads one framed
`VerificationRequest`, writes one framed `VerificationResponse` carrying an
evidence-v2 `ChunkLifecycleWitness` built purely from the request's
obligations and bounded source facts. The integration test builds it,
writes a `verifier-plugin.json` with the computed package digest, registers
it via `discover_packages`, dispatches a chunk-lifecycle scope, and expects
`Safe` with a core certificate; variants cover a malformed frame, an
over-budget response, and a deadline overrun (reusing the VP04-style
harness). A guard test parses the conformance crate's `Cargo.toml` and fails
if any workspace dependency other than `qcore-schemas` appears (extending
the W0 layering guard downward to the provider side of the boundary).

### 5.2 Lean direction (`proof/QCore/VerifierPlugin.lean`)

Today the model proves the right *shape* (checked safety has valid evidence;
missing/invalid evidence and envelopes stay unknown; a missing composition
member prevents safety) over Boolean evidence flags — exactly FM01's
narrowness. W3 extends it for the first capability so the theorem is about
the checked relation, not a flag:

- Model the obligation derivation as a function
  `deriveObligations : NormalizedUnit → List Obligation` (trusted,
  executable over the bounded chunk model already in the proof tree) and the
  witness as a schedule over bound operations.
- Define the checker executably:
  `checkWitness : NormalizedUnit → Witness → Bool` composed of the four
  replay stages (site validity, completeness, faithfulness + order, chunk
  state replay against the existing chunk-model definitions) — mirroring
  §2.3, not re-axiomatizing it.
- Theorems: (a) `checkedSafe u w → ∀ o ∈ deriveObligations u, discharged o
  (replay u w)` — an accepted witness entails every derived obligation's
  property (the §6 "accepted witness entails the capability's property"
  requirement, for this capability's bounded model); (b) a witness omitting
  or duplicating a site is not accepted; (c) v1 (flag-only) evidence still
  cannot reach `checkedSafe`; (d) composition with any undischarged
  obligation is not safe — strengthening the existing composition theorem
  from member-presence to obligation coverage (also discharging FM02's
  required-set note in the model).
- `AuditCycle20.lean` continues to print the axiom surface;
  placeholder-free, no project axioms, per the standing gate. The
  Rust-to-Lean bridge remains an explicitly open boundary: these theorems
  are about the modeled relation; the claim inventory must keep saying so.

---

## 6. Open questions for the review checkpoint

1. **Empty-inventory promotion.** Should a complete derivation with zero
   obligations ever yield a verified scope? *Recommendation:* not in the
   first implementation (§3.5: `Unknown` with a dedicated diagnostic).
   Revisit once the derivation emits a checkable coverage artifact
   ("analyzed constructs" inventory) that the certificate can bind — that
   artifact is the "independently checked no-obligations argument" §6
   demands, and designing it now would delay the two real witness kinds.

2. **Capability slot for contract observations.** This design uses the
   existing `observation_hooks` capability rather than adding a new variant.
   *Recommendation:* keep `observation_hooks`; its manifest field
   `required_observation_hooks` already exists and the semantics match. If
   review prefers a distinct `contract_observations` capability, that is an
   additive enum variant and must be decided before implementation (it is
   wire-visible).

3. **Where the schedule's order bound stops.** §2.3 restricts v2.0 to
   schedules embeddable in a single CFG path per function.
   *Recommendation:* accept the restriction; a bounded multi-path extension
   (`PathBounds::BoundedPaths(n)`, one witness schedule per path, all must
   discharge) is additive later. Anything the core cannot order-check is
   `Unknown`, never approximated.

4. **`Unsafe` counterexample witnesses.** Should v2 include a
   counterexample witness kind so a provider `Unsafe` can be validated
   rather than QP010-downgraded? *Recommendation:* defer. A validated
   negative needs its own replay checker per capability (reevaluation §6:
   "a negative verdict needs its own validated witness"), and shipping it
   half-done risks presenting allegations as findings. Q703 surfaces
   allegations honestly in the meantime.

5. **Crate placement of the witness checkers.** Plan W3 task 3 puts
   registry/process/composition in `qcore-verifier-host` and "the firewall's
   per-capability checkers … the trusted core side". The chunk checker lives
   in `qcore-compiler` (pending W5's move to `qcore-core`) and the
   observation interpreter in `qcore-contract`. *Recommendation:* the v2
   firewall trait (`WitnessChecker`) is defined in the host, implemented by
   trusted-core crates, and wired by the compiler — the host never contains
   a checker implementation, so no provider-reachable crate can influence
   checking. Dependency direction stays downward (plan §2.7).

6. **Task-digest canonicalization.** SHA-256 over serde_json of BTreeMap-
   only types is deterministic within one serde_json version but is not a
   formal canonical form. *Recommendation:* pin the rule "all task types use
   BTreeMap/BTreeSet/Vec, no floats, no HashMap" as a schema-crate test, and
   record the serializer in `Reproducibility`-style metadata; a full
   canonical-JSON spec is not needed while both digesting ends are the core.

7. **Q7xx renumbering ripple.** The §54 renumbering touches the spec table,
   the Libevent provider docs, and Cycle 18 report references.
   *Recommendation:* one deliberate change (spec table + changelog entry of
   §4 + provider manifest docs), archived reports untouched with a
   superseding note; land it in the same change-set as the Q701 message
   update so the collision never half-exists. This is a deliberate pinned-
   table update and must say so in its commit message, per the
   pinned-expectations invariant.

8. **Protocol version.** Should dispatch bump `PLUGIN_PROTOCOL_VERSION`?
   *Recommendation:* no. Frames, framing, limits, and process behavior are
   unchanged; only the evidence schema and two optional request fields move,
   and `evidence_versions` negotiation plus `deny_unknown_fields` on the
   response side give the compatibility gate. Plan §6 records the same
   direction ("protocol v1 framing stays"). If review disagrees, the change
   is mechanical but must be decided before the conformance provider is
   written. Note one consequence to confirm in review: adding optional
   fields to `VerificationRequest` is visible to v1-only process providers
   that enforce `deny_unknown_fields` on requests; the request builder must
   therefore omit the v2 fields entirely when negotiating evidence v1.

---

## 7. What this design explicitly does not do

- It does not weaken the v1 `Safe` refusal, the QP010 `Unsafe` downgrade,
  or any QP001–QP015 fail-closed path; it adds QP016 and the v2 arm beside
  them.
- It does not promote any pinned Cycle 19 translation unit; raw units
  remain unpromoted, and the Cycle 19 audit routes are untouched until G4
  parity.
- It does not give `observation_hooks` discharge any static-safety meaning:
  observed agreement stays a separate, bounded claim class.
- It does not design witness kinds for resource contracts, event traces,
  concurrency, or projection; those capabilities keep returning `Unknown`
  under v2 until their trusted checkers exist (W4/W5).
- It does not modify probe sources, pinned tables, or archived reports; the
  one deliberate pin update it requires (spec §54 renumbering) is named in
  §4 and carries its own changelog entry.
