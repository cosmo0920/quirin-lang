# E03-B — Composition completeness boundaries, repaired

2026-09-08. Baseline `f8dbb2f` ("Consolidate the final O01/O02 observation
increment into the status ledgers"). Workstreams W3/W5, lane Evidence.

This card closes the four boundaries [E03](../remaining-e03/README.md) measured
and recorded but did not repair: **E03-B1** (the published representative is not
the composite), **E03-B2** (capability-scoped unit identities never merge),
**E03-B3** (the unit-scoped coverage obligation is shared, not
provider-indexed), and **E03-B4** (`dispatch_scope` continues past an
unregistered provider). Its open items **E03-O1** … **E03-O4** are closed with
them.

**Nothing is promoted.** Raw Cycle 19 units stay 2 Safe / 42 provisional Unsafe
/ 137 Unknown plus four data-only units, CT coverage stays 20/50 mapped, the
contract corpus stays 60 modules, and full language conformance stays false.
**`qcore-schemas` is untouched at 5.0.0** — no wire type, no serde shape, no
protocol or evidence version changed, and no provider package is affected. The
four pinned 185-unit artifacts were not re-measured and were not touched
(CORPUS-REPIN remains open); the repaired code links no analysis, gateway or
audit path — `derive_task` is reached only from `verify_dispatch`, which the
C-gateway audit routes never call.

| Evidence | Path |
|---|---|
| Merge algebra (17 tests, +4) | `crates/qcore-verifier-host/tests/composition_relation.rs` |
| Production dispatch route (10 tests, +4) | `crates/qcore-verification/tests/composition_relation.rs` |
| Lean | `QCore.VerifierPlugin.Composition` in `proof/QCore/VerifierPlugin.lean`, audited by `proof/AuditCycle20.lean` |

---

## 1. Boundary 1 — the published composite

### Before

`compose_verification` published `certificates.first()` as the composed
outcome's `certificate`. That certificate is attributed to **one** plugin, one
capability and one core-derived task, so its `coverage`, `obligation_bounds`,
`plugin_id`, `witness` and `task` were one component's, not the merge's. E03
had already repaired the premise half by *refusing* any pair whose premise sets
were not equal (`QP012`, "would understate its assumptions"), at a recorded
completeness cost (E03-O1).

### After

`CompositeCertificate` is now the composite, not just a certificate collection,
and `VerificationOutcome` carries it in a new `composite: Option<…>` field that
**only** `compose_verification` / `compose_verification_with_inventory` ever
fill. The composed outcome's `certificate` is populated only when every
component is the *same* certificate (one provider, or the same evidence
submitted more than once) — `CompositeCertificate::representative`. With two
distinct components it is `None`, so no caller reading `.certificate` can
mistake one component for the merge.

### Soundness argument, per field

The verdict contract (`AGENTS.md` invariant 2) requires every published result
to carry `(artifact, property, semantic profile, scope, bounds, assumptions,
evidence version)`. The composite is exactly the conjunction of its components'
scoped obligations, so:

| Field | Direction | Why that direction is the sound one |
|---|---|---|
| premises | **union** | The composite holds only where every component's premises hold. A larger premise set is a *weaker* claim, so publishing the union never widens the published claim; publishing a subset would drop an assumption the merge rests on. A name two components bind to different values is not a union but a contradiction and still refuses (`QP012`). |
| coverage | **union** | Each identifier in it was discharged by some component's core-checked replay, and this is exactly the set `compose_verification_with_inventory` compares to the core's required set. Intersection would be wrong in both roles: as a denominator it under-reports what was required (unsound), as a numerator it discards real discharges (incomplete). |
| obligation bounds | **union, conflicts refused** | Each entry is the bound the discharging component actually checked that obligation under. One identifier under two bounds is `QP013` — never silently widened to the looser one. This check moved from `compose_verification_with_inventory` into the composite construction, so `compose_verification` and `compose_certificates` now refuse it too: strictly more conservative. |
| task bounds | **intersection (tightest envelope)** | A composite is only as strong as its weakest component. `SinglePath` absorbs `BoundedPaths`; two counts take the minimum; `observed_runs_only` is the **disjunction**, so a composite containing one recorded-run component is a recorded-run claim and can never be presented as static source safety. `None` for a count means "not applicable"/"structural", never "unbounded", so an explicit count constrains the composite and its absence does not relax it. If any component carries no task, the composite publishes **no** bounds at all (`bounds() == None`) rather than inventing one. |
| provenance | **sets** | `plugin_ids`, `capabilities` and `task_digests` are the whole sets. One component's attribution is never presented as the merge's. |

### The one direction that became less refusing

The `QP012` "understate" refusal is **removed** and replaced by the premise
union. This is the only verdict-direction change in this card, and it is E03-O1
executed as recorded: two providers whose premise sets differ without
conflicting now merge, and the published result names every premise either of
them declared. It is not a weakened gate — the refusal existed *because* the
publication was dishonest, and the publication is now honest. Every soundness
gate is intact and two are stricter (bounds conflicts in `compose_verification`;
no representative for a multi-component merge).

Pinned by `a_composite_publishes_the_union_of_its_components_premises` (both
orders, superset and subset, plus the unchanged conflicting-value control),
`a_composite_publishes_the_union_of_its_components_coverage_and_provenance`,
`a_composite_publishes_the_tightest_bounds_envelope`, and
`a_single_component_composite_still_publishes_its_certificate`.

---

## 2. Boundary 2 — per-capability build digests

### Before

`derive_task` built `unit.build_digest` from
`"qcore-derivation/v1␟{semantic_profile}␟{translation_unit}"`. Two capabilities
of one source program therefore certified different `UnitIdentity` values and
`compose_verification` refused every multi-capability scope with `QP015`.

### After

The digest is `"qcore-derivation/v1␟{translation_unit}"`. A unit identity names
the **artifact**, and one source program is one artifact however many
capabilities are asked about it.

### Soundness argument

Nothing that the capability-keyed digest was carrying is lost, because the
capability was never *only* there:

- `VerificationTask::obligation_id` hashes `semantic_profile` **and**
  `capability` directly, so two capabilities' obligation identifiers stay
  distinct without the unit identity's help — proved in Lean and asserted in
  `one_program_has_one_unit_identity_across_capabilities`.
- `task_digest` is the SHA-256 of the whole task, which includes
  `semantic_profile` and the obligation list, so the two tasks still have
  different digests.
- `check_evidence_v2` binds every witness to `request.task_digest`, so a
  witness minted for one capability's task still cannot be replayed against
  another's.

And the identity check itself is untouched: different source bytes or a
different translation unit still change the digest and still refuse with
`QP015` (`a_real_artifact_difference_still_refuses_the_merge`, and the
`certificates_of_two_capabilities_over_one_unit_merge` test's own negative
control). The positive control is at the host, where two capabilities over one
unit now compose over a two-identifier required set.

This changes the derived `build_digest` bytes on the QCore source-dispatch
route, and therefore its task digests and obligation identifiers. That route
has no archived artifact: it is exercised only by `w3_evidence_v2`, the
composition tests and the CT/source pipeline, all of which recompute. One
pinned fixture had to follow — `compiler/tests/fixtures/w3/provider` compares
the complete host-generated request bytes against a recorded `request.bin`, and
the test helper that produces `request.bin` now derives under the same provider
identity the dispatcher uses (§3).

---

## 3. Boundary 3 — distinct observation obligation ids

### Before

`ObservationCoverageComplete` carries `ObligationSubject::Unit`, whose canonical
rendering is the constant `"unit"`. Two providers over one unit therefore
derived **one** identifier for two different claims ("every hook *I* require was
checked"), so a two-provider scope's required set had three members, not four.
E03 recorded that this was sound only because `compose_verification` requires
every component to be `Safe` — a property of the merge rule, not of the
obligation.

### After

That obligation's identifier is additionally keyed by the core-computed
denominator the replay actually decides it against: the provider the derivation
is for, and the resolved `required_hooks` set. `DerivationInputs` gains
`verifier: Option<String>`; `dispatch_scope` fills it with the selected plugin
id.

The identifier is computed by `scoped_obligation_id`, which hashes
`VerificationTask::obligation_id`'s own output together with the denominator
under a `␟` separator. Consequences:

- Every **unkeyed** obligation's bytes are unchanged — the base identity
  function is untouched and the separator makes the keyed pre-image
  unambiguous, so no other obligation family moves.
- Nothing here is provider-supplied. The denominator comes from the manifest
  the registry holds and from the core's own contract sources.
- **No schema change.** `ObligationSubject` still renders `Unit`; the repair is
  in the identifier, which is core-computed and opaque to every provider.

### Soundness argument

`ContractObservationReplay` decides this obligation by quantifying over
`self.inventory.required_hooks` — the same map the identifier now names. The
identifier and the checked denominator are therefore bound together, so a
certificate discharging "coverage complete over `{resize}` for
`example.left`" can no longer be credited for "coverage complete over `{grow}`
for `example.right`". The soundness no longer rests on the all-`Safe`
coincidence, and that coincidence is kept as a regression pin: the E03 test that
asserted `required.len() == 3` now asserts `4`, with the pre-repair number
recorded in the comment.

The adversarial direction is
`one_providers_certificate_cannot_discharge_anothers_coverage_obligation`: with
one provider silent, the honest provider's certificate leaves **two**
obligations uncovered where it previously left one, and the counts are pinned.
`two_providers_derive_two_distinct_coverage_obligations` pins that each half of
the key matters independently.

---

## 4. Boundary 4 — absent-provider dispatch

### Before

`dispatch_scope` pushed a `Q701` diagnostic and `continue`d when
`registry.manifest()` returned `None`. That provider's obligations were never
derived, so they never entered `required`, and `composed.verdict` alone could
read `Safe` off the providers that did answer. `ScopeDispatch::verified()` was
false and `driver.rs` consumes `report.diagnostics()`, so every actual consumer
failed closed — but a consumer reading `composed` would have been wrong.

### After

Absence is recorded and becomes a refusal in the composed verdict itself,
before the empty-inventory and merge branches:

```
QP002  plugin `X` is not installed; its obligations could be neither derived
       nor discharged, so this scope has no composite
```

**No new QP code.** The inventory was checked first: `QP002` is already the
registry's own code for "plugin `X` is not installed" (`verify` and `verify_v2`
both use it), and this is the same fact reported one level up. `QP001`–`QP016`
are unchanged; nothing was added, renamed or downgraded.

The scope-level `Q701` is now emitted by the existing composed-refusal branch
rather than by the absent branch directly, so its message carries the `QP002`
detail and the existing E03 pin (`codes == ["Q701"]`, message contains the
absent plugin id) holds **unchanged** — the repair is visible in
`composed`, which is where it was missing.

### What is still open

The obligations of an absent provider still cannot be *derived*: without its
manifest there is no target profile and no required-hook denominator. E03-O4
asked for derivation-then-requirement; what is delivered is a first-class
refusal, which is the fail-closed answer available without inventing a
manifest. Recorded below as **E03-B-O1**.

Pinned by `an_absent_provider_refuses_the_composed_verdict_itself`, which
carries a positive control: the same scope without the absent provider composes
`Safe`, so the refusal is attributable to the absence alone, and both present
providers are still dispatched and still `Safe`.

---

## 5. Lean inventory

`QCore.VerifierPlugin.Composition` grows from 15 audited theorems to **31**, all
with `#print axioms` lines in `proof/AuditCycle20.lean`, all depending only on
`propext` and `Quot.sound` (several on nothing at all). No new axiom, no
`sorry`, no placeholder.

Thirteen of E03's fifteen are retained verbatim. Two are **deliberately
retired**, because they state the pre-repair relation and are false of the
repaired one:

| Retired | Why |
|---|---|
| `distinct_premise_sets_are_unknown` | Unequal but compatible premise sets now merge (§1). |
| `safe_premises_are_uniform` | A merged `safe` no longer publishes one uniform premise set; it publishes the union. |

Eighteen new theorems:

| Theorem | Pins |
|---|---|
| `conflicting_premise_values_are_unknown` | a name bound to two values still refuses (`QP012`) |
| `unequal_premise_sets_merge` | non-vacuity for §1's completeness half: a subset/superset pair now merges |
| `uniform_premises_still_merge` | the retained uniform case still merges — the relation widened, it did not move |
| `composite_publishes_every_component_premise` | nothing a component declared is dropped from the union |
| `composite_premises_come_from_components` | and nothing is invented |
| `safe_premises_are_consistent` | a merged `safe` binds each premise name once |
| `representative_is_every_component` | a published representative *is* every component |
| `distinct_components_have_no_representative` | two distinct components publish none |
| `capabilities_of_one_artifact_share_a_unit` | E03-B2 completeness: two capabilities, one unit identity |
| `distinct_artifacts_have_distinct_units` | E03-B2 soundness: a real artifact difference still differs |
| `obligation_keys_stay_capability_distinct` | nothing is lost: the obligation key still carries the capability |
| `distinct_providers_have_distinct_coverage_keys` | E03-B3: two providers, two coverage obligations |
| `distinct_denominators_have_distinct_coverage_keys` | the denominator half of the key matters too |
| `one_providers_certificate_leaves_the_others_key_uncovered` | the composed consequence: the merge stays `unknown` |
| `absent_provider_scope_is_unknown` | E03-B4: an uninstalled provider refuses the scope verdict |
| `present_scope_is_the_merge` | control: with none absent the scope is exactly the merge |
| `honest_scope_still_composes` / `honest_scope_with_an_absent_provider_refuses` | the decidable pair: same outcomes, one absent provider, opposite verdicts |

Model boundaries, unchanged from E03: obligation identifiers, premise names,
provider identities, artifacts and denominators are natural numbers; the Rust
identifiers are core-computed strings and this model does not repeat their
derivation. `Derivation`/`coverageKey`/`composeScope` are *models of the
identifier and dispatch rules*, not extractions of the Rust code. The
`premisesAgree` definition is retained unused so `uniform_premises_still_merge`
can name the pre-repair relation explicitly.

---

## 6. Version and compatibility

- **`qcore-schemas` is unchanged at 5.0.0.** No wire type, serde shape,
  protocol version or evidence version moved, and `providers/` needed no edit.
  Its 58 tests pass unchanged.
- `qcore-verifier-host` **0.1.0 → 0.2.0**: `VerificationOutcome` gains a public
  field (source-incompatible for literal construction) and
  `VerificationOutcome::unknown` becomes public. `CompositeCertificate` gains
  fields and accessors; its private constructor keeps it unforgeable from
  outside the crate.
- `qcore-verification` **0.1.0 → 0.2.0**: `DerivationInputs` gains a required
  public field.
- Dependents' requirements (`compiler`, `qcore-audit`, `qcore-verification`)
  updated to match; `qcore-compiler` stays 0.2.0.
- Ten literal `VerificationOutcome` construction sites across production and
  tests were updated with `composite: None`. Of the four `DerivationInputs`
  construction sites, two take `verifier: None`, one — the w3 process fixture's
  helper — takes the provider identity the dispatcher uses (§2), and one was
  rewritten into a parameterized test helper.

---

## 7. Pins updated deliberately

| Pin | Before | After | Why |
|---|---|---|---|
| `two_providers_discharge_one_scopes_whole_obligation_inventory` required-set size | 3 | 4 | E03-B3: the coverage obligation is provider-indexed. The old number is recorded in the test comment. |
| the same test's published result | `composed.certificate` carries one component | `composed.certificate` is `None`; `composed.composite` carries the merge | E03-B1 |
| `capability_scoped_unit_identities_do_not_merge_across_capabilities` | asserted the build digests differ | replaced by `one_program_has_one_unit_identity_across_capabilities` + `a_real_artifact_difference_still_refuses_the_merge` | E03-B2; the refusal direction is kept as its own test |
| `a_composite_may_not_understate_its_premise_set` | `QP012` refusal | replaced by `a_composite_publishes_the_union_of_its_components_premises`, keeping the conflicting-value refusal | E03-B1/E03-O1 |
| `AuditCycle20.lean` | 15 Composition theorems | 31, with two retired by name and a comment saying why | §5 |
| `compiler/tests/fixtures/w3/provider` request bytes | derived with no provider identity | derived under `example.conformance` | E03-B3; the fixture pins the exact host-generated request |
| `docs/reports/remaining-e03/README.md` | four boundaries open | superseding note added at the boundary section | this card |
| `docs/reports/remaining-p08/README.md` | two prose statements about E03 repair 2 and E03-B2 | corrected in place with a dated note | P08's **tests are unchanged and all pass**: its six cross-route `QP015` pairs differ in `source_digest` too, so E03-B2 does not reach them |

Nothing else was re-pinned. No diagnostic code was added, changed or
downgraded. `AGENTS.md`, `CLAUDE.md`, `README.md`, `DEVELOPMENT.md`,
`docs/reevaluation/ct_manifest.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are untouched.

---

## 8. Validation

Full standing gate on this worktree, 2026-09-08, Linux, Clang available:

| Gate | Before | After |
|---|---|---|
| `cargo test --workspace` | 1473 passed, 0 failed, 1 ignored | **1481 passed, 0 failed, 1 ignored** (+8) |
| `cargo fmt --all -- --check` | clean | clean |
| `cargo clippy --all-targets -- -D warnings` | clean | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 | **exit 0** |
| `cd proof && lake build` | success | **79 jobs, success** |
| `python3 -B scripts/check_lean_audits.py` | 34 audits | **34 audits passed**, no new axiom |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | 58 passed | **58 passed, 0 failed** |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | clean | clean |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets -- -D warnings` | clean | clean |
| `python3 -B -m unittest discover -s scripts/tests` | 95 passed | **95 passed** |

`lake` is not on the default `PATH` on this host; it was run from
`~/.elan/bin`. The Libevent submodule required
`git -c protocol.file.allow=always submodule update --init --recursive`; the
pin test passes at `d82464a2`.

### 8.1 Honest failure note

`compiler/tests/cycle20_verifier_plugins.rs`
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets` asserts a **500 ms
wall-clock** budget for an 8 MiB write to a closed provider stdin. It failed
twice while another `cargo` build was running on the same host, and passed
**6/6** consecutive runs once the machine was idle; the baseline commit
`f8dbb2f`, checked out in a scratch worktree, passed **3/3** under the same
idle conditions. Nothing on that path is touched by this card (the test drives
`ProcessProvider` directly and never constructs a `VerificationOutcome`). It is
recorded here as load-flaky, in the same class as the P02 `RLIMIT_NPROC` note
in `docs/reports/remaining-p03/README.md` §9.1. **The assertion was not
weakened.**

### 8.2 Not re-measured

- The four pinned 185-unit Cycle 19 artifacts. The profile pins Fluent Bit at
  `66910c10` and the developer checkout has moved off it, so the audit's own
  pin gate refuses to run. They were not touched: `derive_task` is called only
  from `verify_dispatch`, which no C-gateway or configured-audit route reaches,
  and the host change is confined to composition. CORPUS-REPIN remains open and
  now also names E03-B as a card that did not discharge it.
- The canonical CMetrics configured audit, which needs a prepared Clang CMake
  build tree this worktree does not have. Its route does not call `derive_task`
  or `compose_verification`.

---

## 9. Evidence boundary

What this card establishes, and nothing more:

- The merge now **publishes** the composite it computes: premise union, coverage
  union, per-obligation bounds, tightest task-bounds envelope, full provenance;
  and it publishes a component certificate only when that component is the whole
  merge.
- A multi-capability composition over one program is **representable**. It is
  not thereby *reachable* end to end: `ChunkLifecycle` derivation is
  unconditionally `DerivationIncomplete`, so `ObservationHooks` is still the
  only capability that can reach `Safe` on the QCore dispatch route, and the
  multi-capability positive control is a host fixture. P08's finding that the
  shipped packages cannot co-certify a unit stands on its second, independent
  gate (the core's model-library binding).
- The provider-indexed coverage obligation removes a *latent* unsoundness, not
  a demonstrated one: no admitted `Safe` is withdrawn by this card, because the
  all-`Safe` merge rule had been masking it.
- Certificate **minting** is out of scope, exactly as in E03. The host tests use
  a test-double witness checker; the production checkers' replay gates are
  E01/E02's subject.
- Observation replay remains a **recorded-run** claim under the declared
  `observations_trusted` premise with `observed_runs_only` bounds. The
  production-route `Safe` in these tests is a synthetic QCore source fixture and
  is not evidence about any audited C source.

---

## 10. Open items

- **E03-B-O1** — an absent provider's obligations are refused, not derived.
  Deriving them would need the manifest the registry does not have; a scope
  could instead carry its own declared denominator, which is a source-language
  change.
- **E03-B-O2** — `compose_verification` is not associative under its own
  output: a composed outcome has `certificate: None` for a multi-component
  merge, so feeding it back into composition refuses with `QP014`. That is
  fail-closed and no caller does it, but a genuine n-ary composition would take
  `CompositeCertificate` as an input.
- **E03-B-O3** — the task-bounds intersection is exercised only on
  `observed_runs_only` and `SinglePath`; `BoundedPaths`, `schedules_explored`
  and `max_operations` have no derivation that emits them, so their minima are
  argued and unit-tested, not measured against a real claim.
- Composition of more than two providers, of process providers under the
  `linux-rlimit-v1` / `linux-confined-v1` profiles, and of capabilities other
  than `ObservationHooks` on the production route remain unexercised (carried
  over from E03).

---

## 11. Integrator notes

Proposed, not applied:

- Status-ledger paragraph:

> Composition completeness boundaries (E03-B): the four boundaries E03 recorded
> are repaired. `compose_verification` now publishes a real composite — premise
> union, coverage union, per-obligation bounds, the tightest task-bounds
> envelope and full provenance sets — and publishes a component certificate only
> when that component is the whole merge, so E03's `QP012` "understate" refusal
> is superseded by an honest publication rather than by a weaker gate. The
> derived unit identity no longer folds in the capability, so a multi-capability
> composition over one program is representable while a real artifact difference
> still refuses with `QP015`. The unit-scoped `ObservationCoverageComplete`
> obligation is keyed by the provider and its resolved hook denominator, so its
> soundness no longer rests on every component being `Safe`; the two-provider
> required set moves from three obligations to four and the old count is kept as
> a regression pin. An unregistered provider is now a refusal in the composed
> verdict itself under the registry's existing `QP002`, so no caller reading
> `composed` alone can see `Safe`. No diagnostic code was added or changed and
> `qcore-schemas` stays 5.0.0; `qcore-verifier-host` and `qcore-verification`
> move to 0.2.0 for two source-incompatible public-struct additions. The
> `QCore.VerifierPlugin.Composition` namespace moves from 15 to 31 audited
> theorems, two of them deliberately retired by name because they stated the
> pre-repair relation. Nothing is promoted and the four pinned 185-unit
> artifacts were not re-measured (CORPUS-REPIN). Evidence:
> `docs/reports/remaining-e03-b/README.md`.

- Acceptance matrix: E03-B closes E03-B1 … E03-B4 and E03-O1 … E03-O4, carrying
  two Rust test files and eighteen new Lean theorems as its evidence classes.
  P08's "Composition" completion row is narrowed (§7); the five-provider gate
  stays open.
- Workspace test count moves 1473 → 1481; Lean audits stay at 34; provider
  tests stay at 58; Python tests stay at 95.
- No CT case is mapped or unmapped. No contract-corpus count moves. No raw unit
  is promoted.
