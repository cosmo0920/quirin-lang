# E03-B-O — the three recorded composition residuals, dispositioned

2026-09-09. Baseline `0f6d66c` ("Consolidate wave 9: corpus re-pin, matrix
regeneration, ledger"). Wave 10, workstreams W3/W5, lane Evidence.

[E03-B](../remaining-e03-b/README.md) §10 recorded three residuals when it
repaired the four composition-completeness boundaries. This card disposes of
each of them, honestly and separately:

| Residual | E03-B's record | Disposition here |
|---|---|---|
| **E03-B-O1** | an absent provider's obligations are *refused, not derived* | **Repaired for five of six capabilities**, with the sixth's blocker measured per capability rather than asserted |
| **E03-B-O2** | composition is *not associative under its own output* | **Repaired**: a composed outcome is a legitimate composition input, and every grouping publishes the same composite (proved, tested, and pinned against conflict-hiding) |
| **E03-B-O3** | the bounds intersection is *argued and unit-tested only, for count kinds no derivation emits* | **Half corrected by measurement** (three production derivations *do* state `BoundedPaths`; two real ones are now composed), **half pinned** with a tripwire that fails the day a derivation states a schedule or operation count |

**Nothing is promoted.** Raw Cycle 19 units stay 2 Safe / 42 provisional Unsafe
/ 137 Unknown plus four data-only units on the wave-9 re-pinned baseline, CT
coverage stays 30/50 mapped, the contract corpus stays 60 modules, and full
language conformance stays false. **`qcore-schemas` is untouched at 8.0.0** — no
wire type, serde shape, protocol version or evidence version moved, and no
provider package content changed. The four pinned 185-unit artifacts were not
re-measured; the reason and the disjointness argument are in §9.2.

| Evidence | Path |
|---|---|
| Merge algebra, associativity and bounds table (24 tests, +7) | `crates/qcore-verifier-host/tests/composition_relation.rs` |
| Production dispatch route, absent-provider derivation (14 tests, +4) | `crates/qcore-verification/tests/composition_relation.rs` |
| Bounds against real derivations, and the vacuity tripwire (2 tests, new) | `crates/qcore-verification/tests/composition_bounds.rs` |
| Lean | `QCore.VerifierPlugin.Composition` in `proof/QCore/VerifierPlugin.lean`, audited by `proof/AuditCycle20.lean` (31 → 61 audited theorems) |

---

## 1. E03-B-O1 — deriving an absent provider's obligations

### Before

`dispatch_scope` skipped a provider the registry could not find *before*
deriving anything:

```rust
let Some(manifest) = registry.manifest(&selected.plugin_id).cloned() else {
    absent.push(selected.plugin_id.as_str().to_owned());
    continue;      // nothing derived, nothing stated, nothing in `required`
};
```

E03-B made the absence a refusal in the composed verdict (`QP002`), which is
fail-closed. But the scope's *inventory* still silently shrank to the providers
that answered: what the missing provider was supposed to discharge was never
stated anywhere, so a reader of the report could not tell a scope that lost one
provider's obligations from a scope that never had them.

### The measurement

Before deciding whether derivation is possible, the card measured what
derivation actually reads from a manifest. `derive_task`'s inputs are the
translation unit, the program, the `TargetProfile`, the core's contract
sources, the caller's assumptions, the selected provider id, and
`required_observation_hooks`. Of these, exactly **one** is manifest-derived
data (`required_observation_hooks`; the `TargetProfile`'s `library` also comes
from the manifest), and the per-capability picture is:

| Capability | Reads a manifest field? | Absent-derived obligation identifiers |
|---|---|---|
| `ResourceContracts` | no | **identical** to the installed derivation's |
| `TranslationUnitProjection` | no | **identical** |
| `ChunkLifecycle` | no | **identical** |
| `EventTraces` | no | identical (no derivation exists; the incompleteness record is reproduced exactly) |
| `Concurrency` | no | identical (same) |
| `ObservationHooks` | **yes** — `required_observation_hooks` | **unavailable**: the per-hook obligations *are* the declared hooks, and E03-B3 keys the `ObservationCoverageComplete` identifier by the resolved hook set |

The reason the first five are identical is a property of the identifier, not a
hope: `VerificationTask::obligation_id` hashes the task version, the unit
identity, the semantic profile, the capability, the property and the subject.
`target` is **not** in the pre-image, so a derivation that cannot name the
provider's library still computes the same identifiers. This is asserted, both
directions, in
`an_absent_derivation_reproduces_the_installed_derivation_except_for_observation_hooks`,
which runs every one of the six capabilities and compares the two derivations
identifier for identifier.

### After

The absent branch now derives, states and then refuses:

- One `CapabilityDispatch` row per requested capability, carrying the derived
  task, `dispatched: false`, and a new
  `derivation: DerivationBasis::ProviderAbsent` field that says what the
  derivation was able to read. Present providers' rows read
  `DerivationBasis::InstalledManifest`.
- Every stated identifier enters the scope's `required` set, which
  `ScopeDispatch` now publishes (new `required: BTreeSet<String>` field) so the
  inventory is readable rather than internal.
- The row's own outcome is `Unknown` under `QP002`, naming the provider and the
  number of obligations that have no evidence and for which none can be
  requested. **The absence is the refusal on the obligation**, which is what
  E03-O4 asked for and E03-B could not deliver.
- For `ObservationHooks` the inventory is **withheld**, not stated: the
  manifest-free derivation states no obligation (measured), and the code
  clears any it might state in future as a fail-closed guard. The task carries
  a `DerivationIncomplete` row whose `category` is
  `ReasonCategory::EnvironmentUnavailable` — a provider the environment does
  not offer is never a source defect (spec §L21.1) — and whose `reason` is the
  string `manifest_dependent_derivation` returns, so the recorded reason *is*
  the measured one (asserted).

### Why this cannot relax anything

1. The scope-level `QP002` refusal still runs **first**, before the
   empty-inventory and merge branches, so `composed` is unchanged for every
   scope with an absent provider.
2. The stated obligations only ever *grow* `required`, and
   `compose_verification_with_inventory` requires coverage to equal the
   required set exactly. A larger denominator can only refuse more.
3. The absent rows' `Unknown` outcomes are also pushed into the composition
   input list, so even if the scope-level absence branch were ever removed, the
   merge would still refuse under `QP011`. Defense in depth, asserted.
4. No request is built and no provider is contacted for an absent provider; the
   derived task never leaves the dispatcher. Its `TargetProfile` carries an
   empty library, which nothing matches and which no identifier depends on.

Pinned by `an_absent_providers_derivable_obligations_are_stated_and_then_refused`
(two capabilities stated, every identifier in `required`, disjoint from
everything covered, scope refuses under `QP002`),
`an_absent_providers_observation_inventory_is_withheld_with_its_measured_reason`,
and `stating_an_absent_providers_obligations_never_relaxes_the_scope` (the
present providers' task digests and verdicts are byte-identical to the control
run without the absent provider, and the required set strictly grows).

### What is still open

`ObservationHooks`. Deriving it needs a hook denominator, which today lives only
in the provider's manifest. Two routes exist and neither is in this card's
scope: a scope could declare its own denominator (a source-language change), or
the registry could retain the last verified manifest of an uninstalled provider
(a package-lifecycle change, and one that would need P01-F2's template binding
to say what "the manifest of a provider that is not installed" means). Recorded
as **E03-B-O1-HOOKS**.

---

## 2. E03-B-O2 — composition is associative under its own output

### Before

`compose_verification` accepted `VerificationOutcome`s and read
`outcome.certificate`. A multi-component composed outcome carries
`certificate: None` (E03-B1's repair — no component may be published as the
merge), so feeding a composed outcome back in refused with `QP014`, "a safe
plugin result is missing its core certificate". Fail-closed, but silently
non-associative: the refusal said the input was missing evidence when in fact
it carried more evidence than any single certificate.

### After

Composition is n-ary over *components*, and a composed outcome is taken apart
into the components it was built from:

```rust
match (outcome.composite, outcome.certificate) {
    (Some(composite), _) => certificates.extend_from_slice(composite.certificates()),
    (None, Some(certificate)) => certificates.push(certificate),
    (None, None) => return QP014,
}
```

Every grouping of one component sequence therefore reduces to the same
certificate list before a single `CompositeCertificate::compose` decides every
conflict rule over all of them. Associativity is not approximate: the whole
published outcome — verdict, diagnostics, composite, component list, premise
union, coverage union, per-obligation bounds, task-bounds envelope, provenance
sets and the (absent) representative — compares equal across the flat and both
grouped compositions, asserted with `assert_eq!` on the outcomes themselves in
`composing_a_composite_publishes_the_same_merge_as_composing_the_parts`.

### Why this admits nothing

- A `CompositeCertificate` has private fields, a private constructor, no public
  conversion and no `Deserialize`. Outside the crate the only way to hold one
  is to have received it from a composition of certificates the firewall
  already minted.
- Composition performs no admission of its own: it merges already-checked
  certificates. Flattening re-decides every conflict rule over exactly those
  certificates, so a regrouped composition is *the same decision* as the flat
  one, not a weaker one.
- The verdict gates run before any component is taken apart, so a composite
  attached to a non-`Safe` outcome never enters a merge (`QP010` for an
  allegation, `QP011` for an `Unknown`), and a `Safe` outcome with neither a
  certificate nor a composite is still `QP014`. All three are mutation rows in
  `only_a_safe_outcome_contributes_its_composite`.
- `regrouping_cannot_hide_a_conflict_from_the_merge_rules` drives three conflict
  families (`QP012` premise contradiction, `QP013` incomparable per-obligation
  bounds, `QP015` different artifact) through four groupings each. Every
  grouping refuses: with the conflicting pair split across groups the outer
  composition reports the conflict's own code; with the pair inside one group
  that group refuses and contributes an `Unknown`, which the outer composition
  refuses under `QP011`. Both shapes are asserted, and the test fails if any
  grouping returns anything else.
- `a_recomposed_composite_is_still_measured_against_the_required_set`: an
  obligation nobody discharged is still missing after any number of
  regroupings (`QP011`).
- `re_submitting_one_composite_adds_coverage_for_nobody`: `[composite,
  composite]` has the same coverage, premises, bounds, provenance and
  per-obligation bounds as `composite`, and still no representative.

---

## 3. E03-B-O3 — the bounds intersection, measured

### The residual, corrected by measurement

E03-B recorded that `BoundedPaths`, `schedules_explored` and `max_operations`
"have no derivation that emits them, so their minima are argued and
unit-tested, not measured against a real claim". A census of every production
`TaskBounds` construction in `crates/**/src` and `compiler/src` shows that is
true of two of the three kinds and **false of the first**:

| Kind | Production derivations that state it |
|---|---|
| `PathBounds::BoundedPaths` | **three**: `qcore-verification::source_effect` (2 paths for a unit-result body, 4 otherwise), `::call_use_source`, `::teardown_source` |
| `schedules_explored` | none |
| `max_operations` | none |

### The measured half

`crates/qcore-verification/tests/composition_bounds.rs`
(`the_bounds_envelope_of_two_real_derivations_is_the_tighter_path_count`) takes
one imported C translation unit with two admitted bodies — a scalar-result
reader and a unit-result setter — derives both through the real
`BooleanFieldSourceTask::derive`, mints both certificates through the real
`verify_v2` firewall with the production replay checker, and composes them:

- the derived bounds are measured, not chosen: `BoundedPaths { paths: 4 }` and
  `BoundedPaths { paths: 2 }`;
- both carry the same `UnitIdentity` (one imported source is one artifact), so
  the composition is legitimate rather than contrived;
- the published envelope is `BoundedPaths { paths: 2 }` — the weakest
  component's path space — in both component orders;
- the per-obligation bounds keep 4 and 2 respectively: the intersection is the
  envelope, never a rewrite of the parts.

So the `BoundedPaths` branch of `tighter_paths` is now exercised by two real
derived claims, not by fixture values.

### The pinned half

`no_production_derivation_states_a_schedule_or_operation_count` reads every
production `.rs` file under `crates/*/src` and `compiler/src` and asserts that
none constructs `schedules_explored: Some(..)` or `max_operations: Some(..)`,
and that the set of files naming `PathBounds::BoundedPaths` is exactly the four
above plus the intersection itself. Its failure message names E03-B-O3 and says
what to do: re-examine the intersection argument against what the new
derivation means by that count. **The vacuity is now a tripwire, not a
footnote.** The census is also pinned in both directions, so silently adding or
removing a `BoundedPaths` derivation fails the test.

The algebra itself is exercised through the real merge for all three kinds and
both directions in
`the_bounds_envelope_is_the_tightest_of_every_kind` (seven rows, each in both
component orders): two path counts take the minimum, a single path absorbs a
count, two schedule counts take the minimum, an absent count does not relax the
composite, two operation counts take the minimum, and `observed_runs_only` is
the disjunction.

---

## 4. Lean inventory

`QCore.VerifierPlugin.Composition` grows from **31 to 61** audited theorems, all
with `#print axioms` lines in `proof/AuditCycle20.lean`, all within the allowed
closure (`propext`, `Classical.choice`, `Quot.sound`; three depend on nothing).
**No new axiom, no `sorry`, no placeholder, and no theorem retired** — every one
of E03-B's 31 is kept verbatim.

Thirty new theorems:

| Theorem | Pins |
|---|---|
| `absent_derivation_states_the_installed_inventory` | O1: with no manifest field read, the absent inventory *is* the installed one |
| `absent_derivation_withholds_a_manifest_dependent_inventory` | O1: where the denominator is missing, nothing is stated |
| `the_measured_capability_table_is_not_vacuous` | both kinds of capability exist and both derivations are nonempty when they can be |
| `stated_absent_obligations_only_refuse` | a stated-but-undischarged obligation keeps the merge `unknown` |
| `absent_provider_scope_is_unknown_however_much_is_stated` | the scope refusal is unchanged by the wider inventory |
| `collect_safe`, `flatten_leaves` | O2 plumbing: components and outcomes agree |
| `regrouping_is_the_same_component_sequence` | O2: both regroupings flatten to the same components |
| `composition_is_associative` | O2, the statement itself |
| `regrouped_components_merge` | O2 non-vacuity: a regrouped composition still merges |
| `regrouping_cannot_hide_a_conflict` | O2 soundness: a split conflict refuses at both groupings |
| `re_submitting_a_composite_adds_no_coverage` | a repeat contributes nothing |
| `tighter_paths_is_a_lower_bound`, `tighter_count_is_a_lower_bound` | O3: each kind's minimum is a lower bound |
| `the_envelope_is_a_lower_bound` | the whole envelope claims no more than either component |
| `envelope_is_commutative`, `envelope_is_idempotent` | order and repetition do not move the envelope |
| `a_single_path_absorbs` | `SinglePath` is the least path space |
| `an_absent_count_does_not_relax` | `None` is "not applicable", never "unbounded" |
| `recorded_runs_are_contagious` | one recorded-run component makes the composite a recorded-run claim |
| `every_count_kind_takes_the_minimum` | non-vacuity, including the measured 4-and-2 row |
| `pathsAtMost_refl` / `countAtMost_refl` / `atMost_refl` | the order is reflexive |
| `pathsAtMost_trans` / `countAtMost_trans` / `atMost_trans` | and transitive |
| `fold_is_a_lower_bound` | the fold over any number of components bounds each of them |
| `the_published_envelope_bounds_every_component` | O3 in general: a composite is only as strong as its weakest part |
| `an_empty_composition_states_no_envelope` | `none` states none, never "unbounded" |

Model boundaries, unchanged from E03/E03-B: obligation identifiers, premise
names, provider identities, artifacts and denominators are natural numbers; the
Rust identifiers are core-computed strings and this model does not repeat their
derivation. `derive`, `manifestDependent`, `flatten`, `composeComponents` and
`tighter` are *models of the dispatch, flattening and intersection rules*, not
extractions of the Rust code; the measured per-capability table behind
`manifestDependent` is established in Rust (§1), not in Lean.

---

## 5. Mutation rows

| # | Mutation | Result |
|---|---|---|
| 1 | Attach a real composite to an `Unknown` outcome and compose | refused, `QP011` |
| 2 | Attach a real composite to an `Unsafe` (alleged) outcome and compose | refused, `QP010` |
| 3 | `Safe` outcome with neither certificate nor composite | refused, `QP014` (unchanged) |
| 4 | Premise contradiction split across two groups | refused, `QP012` at every grouping |
| 5 | One obligation under two bounds, split across groups | refused, `QP013` at every grouping |
| 6 | A component certifying a different artifact, split across groups | refused, `QP015` at every grouping |
| 7 | Conflict placed *inside* an inner group | inner group refuses; outer refuses `QP011` |
| 8 | Compose a composite against a required set it does not cover | refused, `QP011` |
| 9 | Submit one composite twice | merges, coverage unchanged, no representative |
| 10 | Scope naming an absent provider, obligations now stated | refused, `QP002`; stated ids covered by nobody |
| 11 | Absent provider asked for `ObservationHooks` | inventory withheld, `environment-unavailable` reason, `QP002` |
| 12 | Absent provider added to a scope that composed `Safe` | control still `Safe`; with the absence, `required` strictly grows and the scope refuses |
| 13 | A production site starts stating a schedule/operation count | tripwire test fails, naming E03-B-O3 |
| 14 | The set of `BoundedPaths` derivation sites changes | census assertion fails |

No mutation was admitted; no gate was weakened to make one pass.

---

## 6. P08 impact

[P08](../remaining-p08/README.md)'s rows were re-run against this change:

- `compiler/tests/composed_five_providers.rs` — 2 tests, pass, unchanged.
- `providers/conformance/tests/composed_five_providers.rs` — passes in the
  serial provider-workspace run (§9), unchanged.
- P08 §5's six cross-route `QP015` pairs are unaffected: they are plain
  single-provider outcomes, never composites, so O2's flattening never applies
  to them, and they differ in `source_digest`, so the identity gate still fires
  first.
- O1 touches `dispatch_scope`, which P08's rows do not use (they compose
  registry outcomes directly).

**The full five-provider completion gate stays OPEN.** This card adds no
capability route, no provider evidence and no new certificate checker, so
P08 §7's ordered requirements are exactly where P08 left them. No P08 row
moved; nothing in P08's report needed correcting.

---

## 7. Versions and compatibility

- **`qcore-schemas` unchanged at 8.0.0.** No wire type, serde shape, protocol
  version or evidence version moved. All five provider packages were rebuilt
  and verified anyway (§9) because both lockfiles record the two crate bumps
  below; every package's manifest and executable digest is unchanged in
  content by this card.
- `qcore-verifier-host` **0.2.0 → 0.3.0**: `compose_verification`'s admitted
  input set changed (a composed outcome is now a component), which is a
  behavioural change to a public function.
- `qcore-verification` **0.2.0 → 0.3.0**: `CapabilityDispatch` gains a required
  public field (`derivation`), `ScopeDispatch` gains a required public field
  (`required`), and `DerivationBasis` / `manifest_dependent_derivation` are new
  public items. Source-incompatible for literal construction.
- Dependents (`compiler`, `qcore-audit`, `qcore-verification`) updated to match;
  `qcore-compiler` stays 0.3.0.
- No diagnostic code was added, changed or downgraded. `QP001`–`QP016` and
  `Q700`–`Q703` are untouched; `QP002`'s scope-level message text is reworded
  to stay accurate now that the obligations *are* derived (its code, its
  position before every other branch, and the existing `codes == ["Q701"]` pin
  are unchanged).

---

## 8. Pins updated deliberately

| Pin | Before | After | Why |
|---|---|---|---|
| `an_absent_provider_refuses_the_composed_verdict_itself` capability-row count | 2 | **3** | E03-B-O1: the absent provider has a row of its own. The load-bearing halves — composed `Unknown` under `QP002`, no composite, both present providers still `Safe` and still dispatched — are kept and strengthened (they are now asserted per `DerivationBasis`). |
| `AuditCycle20.lean` | 31 Composition theorems | **61** | §4; nothing retired |
| `qcore-verifier-host`, `qcore-verification` | 0.2.0 | **0.3.0** | §7 |

Nothing else was re-pinned. `AGENTS.md`, `CLAUDE.md`, `README.md`,
`DEVELOPMENT.md`, `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are untouched.

---

## 9. Validation

Full standing gate on this worktree, 2026-09-09, Linux, Clang available:

| Gate | Before (wave-9 checkpoint) | After |
|---|---|---|
| `cargo test --workspace` | 1888 passed, 4 ignored | **1900 passed, 1 failed (load-flaky, §9.1), 4 ignored** — 1901 tests, +13 |
| `cargo fmt --all -- --check` | clean | **clean** |
| `cargo clippy --all-targets -- -D warnings` | clean | **clean** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 | **exit 0** |
| `python3 -B scripts/check_lean_audits.py` | 48 audits | **48 audits passed**, no new axiom |
| `cargo test --manifest-path providers/Cargo.toml --workspace -- --test-threads=1` | 75 passed | **75 passed, 0 failed** |
| `python3 -B -m unittest discover -s scripts/tests` | 125 passed | **125 passed** |
| `providers/package.py build` + `verify` × 5 | verified | **all five built and verified** |

`lake` is not on the default `PATH` on this host; it was run from
`~/.elan/bin`. The Libevent submodule was initialised with
`git submodule update --init --recursive`; the pin test passes at `d82464a2`.

### 9.1 Honest failure notes

- `compiler/tests/cycle20_verifier_plugins.rs`
  `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` asserts a 500 ms
  wall-clock budget for an 8 MiB write to a closed provider stdin. It failed in
  the loaded full-workspace run and passed **3/3** consecutive runs in
  isolation on an idle machine. This is the load-flakiness already recorded in
  `docs/reports/remaining-e03-b/README.md` §8.1 and
  `docs/reports/remaining-p03/README.md` §9.1. Nothing on that path is touched
  by this card (the test drives `ProcessProvider` directly). **The assertion
  was not weakened.**
- `crates/qcore-verifier-host/tests/standalone_host.rs`
  `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
  failed once in the same loaded run and passed **3/3** in isolation. Same
  family, same disposition.
- `providers/conformance/tests/configured_audit.rs`
  `configured_helper_is_imported_from_its_real_pinned_compile_entry` failed in
  the *parallel* provider-workspace run (reporting 0 checked function contracts
  instead of 1) and passes both alone and in the full serial run, which is the
  number recorded above. The route it exercises calls neither
  `compose_verification` nor `dispatch_scopes`. Recorded as an observed
  parallel-execution flake of the configured-audit fixtures, not repaired here.

### 9.2 Not re-measured

- **The four pinned 185-unit Cycle 19 artifacts.** The profile pins Fluent Bit
  at `66910c10` and no prepared Clang build tree exists in this worktree, so
  the audit's own pin gate refuses to run. Parity is argued by measured
  disjointness rather than asserted: the only production callers of
  `compose_verification` are `dispatch_scope` and, through it, `driver.rs`'s
  QCore *source* compilation route; a workspace-wide search finds no call to
  `dispatch_scopes` or `compose_verification` from `crates/qcore-audit`,
  `compiler/examples` or `crates/qcorec`. The C-gateway and configured-audit
  routes are exercised hermetically by `crates/qcore-audit/tests/*` in the
  workspace run above, all passing unchanged.
- The canonical CMetrics configured audit, which needs the same prepared build
  tree.

---

## 10. Open boundaries

What this card establishes, and nothing more:

- **E03-B-O1-HOOKS** — `ObservationHooks` obligations still cannot be derived
  for a provider that is not installed, because the hook denominator exists
  only in its manifest (§1). Two candidate routes are named there; neither is
  attempted here.
- **E03-B-O3** stays *named*, not closed, for `schedules_explored` and
  `max_operations`: their minima remain unexercised by any real derivation, and
  the tripwire exists precisely so the argument is re-examined when that
  changes.
- Associativity is proved of the merge relation. It says nothing about
  *evidence minting*: the host tests use a test-double witness checker, and the
  production checkers' replay gates remain E01/E02's subject. The
  `composition_bounds.rs` measurement does use a production checker
  (`BooleanFieldSourceTask`) and a real import.
- The absent-provider inventory is a statement about *obligations*, not about
  the source: a stated obligation with no evidence is `Unknown`, and no
  reachable path turns it into anything else.
- Composition of more than two providers is now exercised (three components,
  three groupings), but composition of process providers under the
  `linux-rlimit-v1` / `linux-confined-v1` profiles, and of capabilities other
  than `ObservationHooks` on the production QCore route, remain unexercised
  (carried over from E03/E03-B).
- Observation replay remains a **recorded-run** claim under the declared
  `observations_trusted` premise with `observed_runs_only` bounds. The
  production-route `Safe` in these tests is a synthetic QCore source fixture and
  is not evidence about any audited C source.

---

## 11. Integrator notes

Proposed, not applied:

- Status-ledger paragraph:

> Composition residuals (E03-B-O): E03-B's three recorded residuals are
> dispositioned. An absent-but-configured provider's obligations are now
> derived and stated for five of six capabilities — measured, because
> `obligation_id` never hashes the target, so the manifest-free derivation
> produces the installed derivation's identifiers — and the absence becomes the
> refusal on them under the existing `QP002`, with the sixth (`ObservationHooks`)
> withholding its inventory and recording an `environment-unavailable` reason
> rather than stating a vacuous empty-denominator claim; the scope's required
> set is now published and can only grow. `compose_verification` is associative
> under its own output: a composed outcome is taken apart into its components,
> so every grouping publishes the same composite, and three conflict families
> are pinned refused at every grouping. The bounds-intersection vacuity is
> corrected by census — three production derivations do state `BoundedPaths`,
> and two real ones now compose to the tighter envelope — while
> `schedules_explored` and `max_operations` keep a tripwire that fails the day a
> derivation states one. `QCore.VerifierPlugin.Composition` moves from 31 to 61
> audited theorems with none retired and no new axiom; `qcore-schemas` stays
> 8.0.0 while `qcore-verifier-host` and `qcore-verification` move to 0.3.0. No
> diagnostic code changed, P08's rows are unmoved and its five-provider gate
> stays open, and nothing is promoted. Evidence:
> `docs/reports/remaining-e03-b-o/README.md`.

- Acceptance matrix: E03-B-O1 (partly), E03-B-O2 and E03-B-O3 close at the
  boundaries above; **E03-B-O1-HOOKS** is the new named follow-on. P08's
  five-provider gate row is unchanged.
- Workspace test count moves 1888 → 1901; Lean audits stay at 48; provider
  tests stay at 75; Python tests stay at 125.
- No CT case is mapped or unmapped. No contract-corpus count moves. No raw unit
  is promoted.
