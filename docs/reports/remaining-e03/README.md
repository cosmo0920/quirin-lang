# E03 — Bounded multi-provider coverage composition

2026-09-08. Baseline `a4492da` ("Consolidate the wave into the status ledger
and repair the repository map"). Workstreams W3/W5, `verify_or_extend`,
lane Evidence.

This unit audits the **composition relation**: what happens when several
core-checked certificates are merged against the core's own required obligation
set. Its dependencies are [E02](../remaining-e02/README.md) (replay/cache
mutations, 52 rows, 0 repairs), [E01](../remaining-e01/README.md) (obligation
inventory completeness) and
[`w3-evidence-v2`](../w3-evidence-v2/README.md) (the design this implements).
Those cards audit what happens *below* composition — how one certificate is
minted. This card starts where they stop.

It promotes nothing. No raw Cycle 19 unit changes class, no pinned 185-unit
artifact is touched, CT coverage is unchanged at 11/50 mapped, the contract
corpus is unchanged at 59 modules, and no schema, protocol or evidence version
is bumped.

## Subject

| Field | Value |
|---|---|
| Merge relation | `compose_verification`, `compose_verification_with_inventory`, `compose_certificates` (`crates/qcore-verifier-host/src/lib.rs`) |
| Claim aggregation | `dispatch_scope` (`crates/qcore-verification/src/verify_dispatch.rs`) |
| Lean model | `QCore.VerifierPlugin.Composition` (`proof/QCore/VerifierPlugin.lean`), audited by `proof/AuditCycle20.lean` |
| New evidence | `crates/qcore-verifier-host/tests/composition_relation.rs` (13 tests), `crates/qcore-verification/tests/composition_relation.rs` (6 tests) |

Two test files, because the relation has two honest views:

- The **host** file audits the merge algebra itself. It mints every composed
  certificate through the real `verify_v2` firewall — nothing there fabricates
  a `CoreCertificate`, which is impossible from outside the crate anyway — but
  the witness checker it mints them with is a deliberately small test double.
  That double is core-side code no provider can reach, it decides its
  quantifiers against the core's own obligation list, and it never compares
  provider data against provider data; it is not one of the production
  checkers. Using it isolates *composition* from replay, which is E01/E02's
  subject and is not re-audited here.
- The **verification** file runs two genuinely independent providers through
  one real `verify scope`, with the production `ContractObservationReplay`
  checker, the real registry and the real `dispatch_scopes` orchestration.
  A two-provider scope composes `Safe` there, so the relation is exercised in
  production shape and not only in a fixture.

## Result

**Verified, with two repairs.** Of the 19 new rows, 17 pass against the
baseline code unchanged and pin gates that already held; 2 failed and are
repaired. Both repairs are in `compose_verification`, both are fail-closed
(they turn a `Safe` into an `Unknown` and never the reverse), and both were
confirmed load-bearing by re-running the suite with the guard removed:

```
a_composite_may_not_understate_its_premise_set   left: Safe   right: Unknown
composition_cannot_republish_an_unchecked_unsafe_label  left: Unsafe  right: Unknown
```

### Repair 1 — composition may not republish an unchecked violation

`compose_verification` returned an `Unsafe` component outcome *verbatim* as the
composite verdict, before any identity, premise or coverage check. Both
firewalls already downgrade a provider's `Unsafe` to `Unknown` under `QP010`
("evidence v2 has no counterexample checker"), so no registry route can produce
one — but `compose_verification` is a public API, exported from the host and
re-exported by `qcore-compiler`, and any caller-supplied `Unsafe` became the
composite verdict of the whole scope. That is exactly the shape a provisional
source-unit finding has, and republishing it as the composite verdict would
relabel it as a checked counterexample.

The composition now downgrades it the same way the firewalls do: `Unknown`
under `QP010`, with the alleging outcome's own diagnostics retained verbatim so
nothing is hidden. The composed result carries no certificate, and
`compose_certificates` still returns `Err`.

### Repair 2 — a composite may not understate its premise set

`compose_verification` merged component premises into a local map only to
detect *conflicts* (same key, different value → `QP012`), then discarded the
merged map and published `certificates.first()` as the representative. Two
components with non-conflicting but **unequal** premise sets therefore composed
`Safe` while the published certificate carried only one component's
`assumptions_used`: a premise the composite genuinely rests on was silently
dropped from the published result. Hard invariant 2 requires every published
result to carry its assumptions.

Composition cannot fix this by fabricating a merged certificate: a
`CoreCertificate` is attributed to one plugin and one task, so a representative
carrying another provider's premises would replace one misstatement with
another. The repair is therefore a refusal, not a fabrication: when the
components do not all declare the same premise set, composition returns
`Unknown` under `QP012` with a distinct message ("plugins declare different
premise sets; the published composite would understate its assumptions"). The
existing conflicting-value message and its existing test are unchanged, so the
two `QP012` cases stay distinguishable. The check is symmetric — both orders of
the same pair refuse — and the test carries an equal-premise control that
composes `Safe`, so the refusal is attributable to the premise set alone.

This costs completeness: two providers whose premise sets differ but do not
conflict are no longer mergeable. Widening this to a first-class composite
premise union — which needs somewhere to put a premise set that belongs to no
single provider — is recorded below as open item **E03-O1**, not done here.

No diagnostic code was added or changed; `QP010` and `QP012` already own these
rejections. No provider, schema, protocol or evidence version is affected.

## Acceptance

### 1 — requested nonempty obligations must be covered; missing or failed providers leave theirs `Unknown`

Enforced by the baseline code; pinned, not patched.

| Row | Shape | Outcome |
|---|---|---|
| 1.1 | two providers, disjoint coverage, honest | `Safe`; the union of certificate coverages equals the core's required set exactly |
| 1.2 | one provider answers `Unknown` | `QP011`; its obligations stay in `coverage_for` and uncovered |
| 1.3 | one provider fails (`ProviderFailure`) | `QP006` at the firewall, then `QP011` at the merge |
| 1.4 | one provider proposes `Safe` with a witness that records nothing its obligations need | `QP009` at the firewall, then `QP011` — refused before the merge, not by it |
| 1.5 | a required provider is simply absent from the outcome list | `QP011`; "nothing reported" is never read as "nothing required" |
| 1.6 | required set empty | `QP016`; a zero-obligation derivation is not promoted |
| 1.7 | no outcomes at all | `QP010` |
| 1.8 | coverage strictly wider than the required set | `QP011` |

On the production route: a two-provider scope where the second answers
`Unknown`, fails, or proposes *the other provider's* hook leaves the scope
unverified with `Q701`, the failing provider still dispatched, its obligations
still in the core inventory, and — for the foreign-hook row — `QP009` from the
core's own replay rather than silent credit to the honest provider. A scope
with no providers, and a scope naming a provider absent from the registry, both
report `Q701` and compose `QP016`/`Unknown` rather than verifying vacuously.
(`resolve_source_scopes` already rejects an uninstalled plugin with `Q700`
before dispatch; the row pins the dispatcher's own defensive branch.)

### 2 — reject incompatible identities, profiles and premises; detect conflicts without counting duplicate evidence twice

| Row | Shape | Outcome |
|---|---|---|
| 2.1 | one certificate submitted twice against a two-obligation required set | `QP011`, "1 core-derived obligation(s) are not covered" — the repeat contributes one identifier, not two |
| 2.2 | the same duplication against its own exact required set | `Safe` — duplication neither adds nor destroys coverage |
| 2.3 | certificates over different `UnitIdentity` values | `QP015` |
| 2.4 | conflicting premise values (`observations_trusted` true vs false) | `QP012`, "conflicting assumptions" |
| 2.5 | non-conflicting but unequal premise sets, both orders | `QP012`, "understate" — **repaired here** |
| 2.6 | equal premise sets (control for 2.5) | `Safe`; the published certificate carries that premise set |
| 2.7 | one obligation identifier discharged under two different `TaskBounds` | `QP013`, "incomparable bounds" |

Row 2.7 is worth naming precisely: `VerificationTask::obligation_id` hashes the
task version, unit identity, semantic profile, capability, property and
canonical subject — **not** the bounds. Two tasks that differ only in bounds
therefore produce the *same* core-computed identifier with different bounds and
different task digests, and the merge catches that on the bounds map rather
than on the identifier.

### 3 — a checked violation stays scoped to its supported property

| Row | Shape | Outcome |
|---|---|---|
| 3.1 | provider proposes `Unsafe` through the registry | `QP010` `Unknown`, provider diagnostic retained, no certificate; composition `QP011` |
| 3.2 | caller-supplied `Unsafe` outcome reaches composition, in three positions | `QP010` `Unknown` — **repaired here**; no certificate, `compose_certificates` errors |
| 3.3 | `Safe` verdict with no certificate | `QP014` |

On the production route, one provider's allegation raises `Q703` ("alleges a
violation the core cannot validate") *and* leaves the scope `Q701` unverified
with an `Unknown` composite and no published certificate. The allegation is
neither promoted to a scope-wide violation nor discarded.

The separate half of acceptance 3 — that source-unit provisional diagnostics
are not relabelled — was checked by reading rather than by patching: the
`qcore-audit` resource route never feeds a unit verdict into
`compose_verification`, its source-contract route calls `verify_v2` per
function and reports `SourceContractError::Admission` instead of touching the
unit verdict, and the report scope strings still say "Unsafe is provisional".
`ProposedVerdict::Unsafe` is constructed nowhere in `crates/`, `compiler/src`
or `providers/` outside the two firewall match arms that downgrade it. Repair 1
closes the one place where a caller could have injected one.

## Measured boundaries (recorded, not repaired)

- **E03-B1 — the published representative is not the composite.**
  `compose_verification` returns `certificates.first()`; `compose_certificates`
  returns the full collection. After repair 2 the representative no longer
  understates the *premises*, but its `coverage`, `obligation_bounds`,
  `plugin_id`, `witness` and `task` are still one component's. That direction
  is conservative — a caller comparing the representative's coverage to the
  required set sees an under-report and fails closed — and row 1.1 asserts the
  composite coverage through `compose_certificates`, which is the sanctioned
  accessor. E03-O1 below is the non-conservative fix.
- **E03-B2 — capability-scoped unit identities never merge.**
  `derive_task` builds `unit.build_digest` from the capability's semantic
  profile, so two capabilities of one source program certify different
  `UnitIdentity` values and `compose_verification` refuses them with `QP015`.
  Multi-capability scopes therefore cannot be verified in this increment. This
  costs completeness, never soundness, and is pinned by a test. It is currently
  unobservable end-to-end for a second reason: `ChunkLifecycle` derivation is
  unconditionally `DerivationIncomplete` ("source lifecycle effect/ABI import
  is unavailable"), so `ObservationHooks` is the only capability that can reach
  `Safe` on the QCore dispatch route today.
- **E03-B3 — the unit-scoped coverage obligation is shared, not
  provider-indexed.** `ObservationCoverageComplete` has subject `Unit`, so two
  providers over one unit derive the *same* obligation identifier for it. The
  union of a two-provider scope's required set is therefore three identifiers,
  not four, and the shared one is discharged by both providers under their own
  inventories. This is sound only because `compose_verification` requires
  *every* component to be `Safe`, so no provider's obligation is ever covered
  by another's certificate alone. The test pins the count and the joint
  discharge so the property cannot drift silently.
- **E03-B4 — `dispatch_scope`'s `required` set skips a provider that is absent
  from the registry**, because it `continue`s before deriving that provider's
  task. The scope is still unverified (`Q701`, and `ScopeDispatch::verified()`
  is false), and `driver.rs` consumes `report.diagnostics()` rather than
  `composed`, so every consumer fails closed; but `composed.verdict` alone can
  read `Safe` in that state. Pinned by a test; a consumer that reads `composed`
  without `verified()` would be wrong.

## Lean inventory

`proof/QCore/VerifierPlugin.lean` gains a `Composition` namespace modelling the
repaired relation: certificates reduced to `(unitId, premises, coverage,
bounds)`, outcomes as `safe cert | alleged | unknown`, and
`composeInventory outcomes required`. Obligation identifiers and premise names
are natural numbers — the Rust identifiers are core-computed strings and this
model does not repeat their derivation. The pre-existing `check`, `compose` and
their six theorems are unchanged.

Fifteen audited theorems, all with `#print axioms` lines in
`proof/AuditCycle20.lean`, all depending only on `propext` and `Quot.sound`;
no new axiom, no `sorry`, no placeholder:

| Theorem | Pins |
|---|---|
| `disjoint_certificates_merge` | non-vacuity: two providers over one unit with one premise set and disjoint coverage do merge |
| `duplicate_certificate_still_merges` | non-vacuity for duplication: a repeat neither widens nor destroys exact coverage |
| `composition_is_never_a_violation` | the composite verdict is never `rejected` (repair 1) |
| `alleged_component_is_unknown` | an allegation anywhere in the list makes the merge `unknown` |
| `unknown_component_is_unknown` | a missing or failed provider blocks the merge |
| `no_outcomes_is_unknown` | an empty outcome list is not a vacuous success |
| `empty_inventory_is_unknown` | a zero-obligation derivation is not promoted |
| `safe_covers_every_required_obligation` | a merged `safe` covers every required identifier |
| `uncovered_obligation_is_unknown` | an obligation no certificate discharges keeps the merge `unknown` |
| `duplicate_certificate_adds_no_coverage` | coverage is a set: `c :: c :: rest` and `c :: rest` have the same members |
| `duplicate_evidence_cannot_cover_a_missing_obligation` | two copies of one certificate cannot discharge what it does not cover |
| `distinct_unit_identities_are_unknown` | identity mismatch refuses (`QP015`) |
| `distinct_premise_sets_are_unknown` | any premise-set difference refuses, conflicting value or omitted key (repair 2, `QP012`) |
| `safe_premises_are_uniform` | a merged `safe` publishes one premise set every component declared |
| `incomparable_bounds_are_unknown` | one identifier under two bounds refuses (`QP013`) |

Supporting, unaudited: `composable_of_safe`, `collect_alleged_is_none`,
`collect_unknown_is_none`.

The two non-vacuity theorems are the guard against a model that proves every
refusal because it accepts nothing.

## Evidence boundary

What this card establishes, and nothing more:

- The merge relation over already-core-checked certificates is exact against a
  core-computed required set, duplicate-insensitive, and refuses mismatched
  identities, premise sets and bounds. It cannot publish a violation.
- Certificate *minting* is out of scope. The host tests use a test-double
  witness checker; the production checkers' replay gates are E01/E02's subject.
  A sound merge over unsound certificates would still be unsound, and this card
  does not re-establish that they are sound.
- Observation replay remains a **recorded-run** claim under the declared
  `observations_trusted` premise with `observed_runs_only` bounds. The
  production-route `Safe` in these tests is a synthetic QCore source fixture and
  is not evidence about any audited C source.
- No universal whole-program safety claim follows from a composed `Safe`: the
  composite is exactly the conjunction of its components' scoped obligations,
  under the premise set every component declared.
- `qcore-audit`'s resource verdicts are untouched. A provisional `Unsafe` unit
  is still provisional and is still not an upstream defect.

## Open items

- **E03-O1** — first-class composite premises. Repair 2 refuses unequal but
  compatible premise sets rather than merging them, because a `CoreCertificate`
  is attributed to one plugin and one task and there is nowhere honest to put a
  premise union. Carrying a `CompositeCertificate` (or a premise union) on
  `VerificationOutcome` would restore that completeness; it changes a public
  host struct and about a dozen literal construction sites across production
  and tests, so it is a separate unit.
- **E03-O2** — E03-B1's remaining representative fields (coverage, bounds,
  provenance). Subsumed by E03-O1.
- **E03-O3** — E03-B2: capability-independent unit identity, so a scope may
  request more than one capability. Blocked behind a capability other than
  `ObservationHooks` having a complete derivation and a core checker
  (`ChunkLifecycle` is unconditionally incomplete today).
- **E03-O4** — E03-B4: `dispatch_scope` should derive and require the
  obligations of a provider it cannot find, rather than `continue`ing before
  the derivation. Currently masked by `resolve_source_scopes`' `Q700`.
- Composition of more than two providers, of process providers under the
  `linux-rlimit-v1` profile, and of capabilities other than `ObservationHooks`
  remain unexercised.

## Validation

Full standing gate on this worktree, 2026-09-08, Linux:

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1016 passed, 0 failed** (baseline 997 + 19 new) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `cd proof && lake build` | 64 jobs, success |
| `python3 -B scripts/check_lean_audits.py` | **21 audits passed** |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **47 passed, 0 failed** |
| `scripts/tests/test_*.py` | **43 tests, 0 failures** |

`git submodule update --init --recursive` was run once for this worktree; the
`third_party/libevent` pin test passes at `d82464a2`.

The pinned 185-unit Cycle 19 artifacts were **not re-measured and not
modified**: this change touches only the verifier host's composition function,
its tests and the Lean model, and links no analysis, gateway or audit code
path. `docs/reviews/r0_3/acceptance_matrix.json`, `acceptance.json` and
`docs/reevaluation/ct_manifest.json` are owned by the integrator and are
untouched; suggested matrix updates are in the handoff note below.

## Integrator notes

Proposed, not applied:

- `docs/reviews/r0_3/acceptance_matrix.json`: E03 closes as *verified with two
  repairs*; the evidence classes it carries are the two Rust test files and the
  fifteen `QCore.VerifierPlugin.Composition` theorems.
- AGENTS.md / CLAUDE.md status ledger: the workspace test count moves from 997
  to 1016 and the Lean audit count stays at 21. No verdict, CT (11/50) or
  contract-corpus (59) figure changes; `qcore-schemas` stays 3.0.0.
- No CT case is mapped or unmapped by this card.
