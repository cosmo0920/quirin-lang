# C01 — Validate existing richer custody summaries across calls — 2026-09-07

Regression closure for the **merged** W4 custody-summary subprofiles: an
inventory of what the gateway admits today, a per-acceptance-item coverage
verdict citing the tests that pin each claim, and ten new adversarial tests
closing the gaps found.

This card is `verify_or_extend` in its **verify** outcome. It reimplements no
inference, guesses no recursive fixed point, extends no admitted vocabulary,
weakens no gate and promotes no verdict. **No soundness defect was found.**
Only two test files changed; no production analysis code was touched.

## 1. Admitted subprofiles today

Read off `crates/qcore-c-gateway/src/legacy_summary.rs` and cross-checked
against the merged reports listed in §5. A subprofile is "admitted" when
`derive_custody_summaries` can publish a `ResourceSummaryRecord` carrying it
under `ContractTrust::InferredValidated`.

### Parameter modes

| Mode | Admitted shape | Derived at | Nullable variant |
|---|---|---|---|
| `Borrowed` | formal never bound as a resource; the default the search starts from | `probe_contract`, `parameter_mode` | n/a |
| `Owned{nullable:false}` | formal bound (body raised an unresolved resource identity for it) and **live at every exit** | `parameter_mode` | **not inferred** |
| `Released{nullable:false}` | bound formal **released at every exit** | `parameter_mode` | **not inferred** |
| `Consumed{nullable:false}` | bound formal **escaped at every exit** | `parameter_mode` | **not inferred** |
| `Stored{owner_parameter, nullable:false}` | bound formal **contained at every exit** in the aggregate of exactly one *other* caller-held formal, the **same one at every exit** | `stored_mode` | **not inferred** |
| `Out{nullable:false\|true}` | formal the body writes through (`*out = …`, or forwards as a modeled `Out` argument), holding a **distinct fresh non-null live** site at every exit | `out_parameter_candidates`, `out_mode` | `true` only when some exit stores a **null constant** |
| `Removed{owner_parameter, nullable:false\|true}` | child and owner are **direct formal names** of a modeled `Removed` call, child and owner distinct; validated live-and-detached on every non-null exit | `infer_removed_wrapper` | `true` only by weakening an already-validated contract and revalidating the whole body, including the new null paths |

### Result modes

| Mode | Admitted shape | Derived at |
|---|---|---|
| `None` | no exit returns a tracked resource | `propose_result` |
| `Owned{nullable}` | every returning exit hands back a **fresh escaped** site that is not a formal's own site; `nullable` is forced by any exit returning nothing or returning a possibly-null site | `propose_result` |
| `Reallocated{parameter, nullable:true}` | some exit hands back a fresh replacement and some exit leaves the old resource with the caller; **`nullable:false` is explicitly refused** (`result_matches`) because a non-nullable `Reallocated` describes no custody dependence | `reallocated_parameter`, `reallocated_contract` |

### Normalizations and routes

| Route | Admitted shape |
|---|---|
| Immutable output aliases | first local declarations copying an output formal through an **immutable, same explicit flat pointer type** alias chain; only exact modeled `Out` arguments and direct indirect stores may use them. Mutation, shadowing, escape, cast, branch-local alias, indirect read all reject |
| Contextual recursion | one **immutable explicit integer** control partitioned Zero/NonZero; NonZero may call already-validated literal-zero contexts; both contracts must be equal and revalidated over the disjoint combined CFG |
| Unsigned countdown | exact `control - 1` actuals on **explicit unsigned** controls, including mutual calls with different control slots/widths; strong induction over concrete natural ranks |
| Declared precedence | a declared `.qcore` contract always governs the call site; a validated inference that disagrees is recorded as a conflict, is **not published as a record**, and raises `SummaryConflict` (`legacy_c.rs:738-751`) |
| Certificate containment | `check_legacy_safety_certificate` **re-derives the whole summary vector** from the preprocessed source and the declared contracts and demands exact equality (`legacy_c.rs:553`), then demands every record be `InferredValidated` (`legacy_c.rs:556`) |

### Explicitly not admitted

General removal aliases and heap-link discovery; alias-selected removal;
mutable/escaping/branch-local out aliases; general recursive fixed points;
signed or non-decreasing recursive arithmetic; zero-context recursion; a formal
handed straight back to the caller; a formal that is both bound and an out
slot; cost summaries. Each leaves the call **uncontracted**, hence `Unknown`.

## 2. Coverage verdict per acceptance item

Verdict vocabulary: **already pinned** = an existing test asserts it;
**pinned by C01** = a gap this card closed.

### Item 1 — Out kills, preserved alternatives, owner identity

| Sub-claim | Verdict | Evidence |
|---|---|---|
| Out relationship killed by **reassignment** of the formal | already pinned | `legacy_summary.rs::reassigned_out_formals_do_not_describe_the_callers_slot` (3 assignments); `forwarding_does_not_accept_reassigned_escaping_or_unwritten_slots` (25 bodies, incl. `out = other`, `out++`, `--out`, `out = 0`, `q_make(&out)`) |
| Out relationship killed by **alias change** | already pinned | `out_alias_normalization_rejects_mutation_shadow_escape_and_type_changes` (21 bodies: alias mutation, shadowing, escape, cast, `volatile`, `static`, branch-local, deferred assignment, indirect read, short-circuit) |
| **Stale sanctioned-slot reuse rejects** | already pinned | `formal_identity_guards_keep_scalar_updates_but_reject_resource_mutation`; `resource_aliases_do_not_hide_formal_pointer_mutation`; `a_store_through_a_local_pointer_stays_unmodeled`; caller side: `forwarded_output_callers_keep_leak_and_slot_alias_obligations`, `aliased_out_callers_still_discharge_distinct_fresh_outputs` (`fill(&p,&p)`) |
| Null/success/failure alternatives preserved at **inference** | already pinned | `a_wrapper_returning_an_unresolved_acquisition_is_nullable`; `a_nullable_returning_wrapper_forces_a_null_check`; `an_out_slot_nulled_on_a_failure_path_is_nullable`; `a_possibly_null_out_store_without_a_callee_side_test_yields_no_summary`; `forwarded_outputs_keep_nullability_and_dont_invent_unresolved_handback` |
| A summary **cannot collapse a possibly-null return to non-null** at the applied/certificate boundary | **pinned by C01** | `custody_summary_regression.rs::a_possibly_null_owned_result_cannot_be_collapsed_to_non_null` (both directions, plus the non-null control); `a_nullable_out_slot_cannot_be_collapsed_to_a_guaranteed_one` |
| `Removed` **owner identity** preserved across application | already pinned | `removal_summary_composes_through_reordered_direct_formals`; `removed_formal_identity_and_all_exit_dispositions_are_rechecked` (13 bodies); `removed_certificate_rederives_owner_and_child_effects` (owner index tampering) |
| `Stored` **owner identity** — same formal at every exit | **pinned by C01** | `legacy_summary.rs::a_stored_owner_must_be_the_same_caller_held_formal_at_every_exit` (reordered-formal positive; 5 negatives incl. branch-dependent owner, partial store, double store, self-store) |
| `Stored` owner identity survives certificate rederivation | **pinned by C01** | `custody_summary_regression.rs::stored_owner_identity_survives_certificate_rederivation` (5 tampers) |
| `Reallocated` **parameter identity** and its null alternative | **pinned by C01** | `custody_summary_regression.rs::reallocated_result_identity_and_its_null_alternative_are_rechecked` (4 tampers) |

### Item 2 — Declared contracts win, `SummaryConflict`, never silent

| Sub-claim | Verdict | Evidence |
|---|---|---|
| Agreeing declaration produces no conflict | already pinned | `an_explicit_contract_agreeing_with_inference_is_preserved` |
| Disagreeing declaration raises `SummaryConflict`, unit stays `Unknown` | already pinned | `an_explicit_contract_disagreeing_with_inference_is_a_diagnostic` |
| The conflicting **inferred** summary is not published as evidence | already pinned | `a_conflicting_summary_is_not_published_as_evidence`; `removed_summary_keeps_declared_precedence_and_records_conflicts` (also asserts the environment keeps the *declared* contract) |
| Precedence holds on the richer subprofiles | already pinned | `out_forwarding_respects_declared_contracts_and_certificate_rederivation`; `aliased_out_inference_preserves_declared_precedence`; `contextual_recursion.rs::declared_conflict_cannot_be_overridden_by_recursive_inference` |
| Precedence holds for a **middle** wrapper in a chain, and the inner wrapper's own summary is untouched | **pinned by C01** | `legacy_summary.rs::a_declared_middle_wrapper_governs_its_callers_and_publishes_no_record` |
| Nested wrapper summaries undergo **independent reanalysis** (certificate recheck re-derives them) | already pinned | `extended_summary_certificates_are_rederived_and_reject_tampering`; `removed_certificate_rederives_owner_and_child_effects`; `real_c_alias_certificate_rederives_identity_and_rejects_changed_source_or_summary`; `contextual_recursion.rs::certificate_recheck_rejects_source_and_summary_context_substitution`, `countdown_certificate_rechecks_rank_guards_and_summary_identity` |

### Item 3 — Mutated callee summaries invalidate caller evidence

| Sub-claim | Verdict | Evidence |
|---|---|---|
| Two-level composition is bottom-up | already pinned | `nested_wrappers_compose_bottom_up`; `components_are_returned_callees_first` |
| A changed callee **body** invalidates a caller's certificate | already pinned for a direct callee | `removed_certificate_rederives_owner_and_child_effects` (`q_remove` → `unknown_remove`); `real_c_alias_certificate_rederives_identity_and_rejects_changed_source_or_summary` |
| **A → B → C, mutate C**: the certificate is refused, not reused | **pinned by C01** | `custody_summary_regression.rs::a_changed_transitive_callee_body_invalidates_the_whole_certificate` (3 mutations, each asserted against its exact rejection reason) |
| **A → B → C, mutate C**: B's published summary and A's obligation both follow, forwards | **pinned by C01** | `custody_summary_regression.rs::a_changed_transitive_callee_redirects_the_analysis_of_every_caller`; inference-level twin `legacy_summary.rs::a_changed_transitive_callee_redirects_every_derived_summary` (non-null → nullable → unsummarizable) |

### Item 4 — A hand-constructed `InferredValidated` record must be rejected

| Sub-claim | Verdict | Evidence |
|---|---|---|
| A record whose **contract** the analyzer would not derive is rejected | already pinned | `extended_summary_certificates_are_rederived_and_reject_tampering` and the three other certificate tests above |
| An **added** record for a function that has none is rejected | **pinned by C01** | `fabricated_validated_summaries_are_rejected_at_the_recheck_boundary` (a) |
| A record fabricating a **release** the wrapper does not perform is rejected | **pinned by C01** | same test (b) |
| A **deleted** record is rejected (a verdict cannot hide what it rests on) | **pinned by C01** | same test (c) |
| A record **relabelled** `ContractTrust::Declared` is rejected | **pinned by C01** | same test (d) |
| A record **reattached to another function name** is rejected | **pinned by C01** | same test (e) |

## 3. Tests added

Ten tests, all passing, no production code changed.

**New file** `crates/qcore-c-gateway/tests/custody_summary_regression.rs` —
public-API only (`analyze_unrestricted_c11`, `check_legacy_safety_certificate`,
`LegacySafetyCertificate`, `ResourceSummaryRecord`), in the style of the
existing `contextual_recursion.rs` integration suite. 7 tests, 23 tamper cases:

1. `fabricated_validated_summaries_are_rejected_at_the_recheck_boundary` — 5 cases
2. `a_changed_transitive_callee_body_invalidates_the_whole_certificate` — 3 cases
3. `a_changed_transitive_callee_redirects_the_analysis_of_every_caller`
4. `a_possibly_null_owned_result_cannot_be_collapsed_to_non_null` — 3 cases
5. `a_nullable_out_slot_cannot_be_collapsed_to_a_guaranteed_one` — 3 cases
6. `stored_owner_identity_survives_certificate_rederivation` — 5 cases
7. `reallocated_result_identity_and_its_null_alternative_are_rechecked` — 4 cases

**Extended** `crates/qcore-c-gateway/tests/unit/legacy_summary.rs` — 3 tests
needing the crate-internal `derive_custody_summaries` so that *absence* of a
record can be asserted directly, not merely inferred from a non-`Safe` verdict:

8. `a_stored_owner_must_be_the_same_caller_held_formal_at_every_exit`
9. `a_changed_transitive_callee_redirects_every_derived_summary`
10. `a_declared_middle_wrapper_governs_its_callers_and_publishes_no_record`

### Non-vacuity

Every certificate rejection is asserted against its **exact reason string**,
not `is_err()`, so a case cannot pass by failing for an unrelated cause (a
fixture that stopped parsing, a Clang change). The reasons were **measured
before being pinned**, by temporarily printing each result: of the 23 tamper
cases, 22 are refused with
`inferred custody summaries do not match the preprocessed source`
(`legacy_c.rs:553`), and one — C acquiring a second undischarged resource,
which leaves the published summaries intact — with
`program summary does not match the preprocessed source` (`legacy_c.rs:563`).
That single case is the reason the reason-strings are asserted individually
rather than accepted as a class: the two containment gates are distinct, and a
test that conflated them would not notice one of them regressing.

The source-mutation cases additionally assert that the substring they replace
is actually present in the preprocessed source, so a silently-missed
replacement cannot masquerade as a rejection.

### One honest observation, not a defect

The dedicated trust gate at `legacy_c.rs:556-562` ("certificate carries a
custody summary that was not validated") is **unreachable from this path**: a
relabelled record is already refused by the vector-equality check one line
earlier, because a fresh derivation only ever emits `InferredValidated`. The
gate is defense in depth for any future producer of that vector, and is
recorded as such in the test comment. Nothing was removed or weakened.

## 4. Gate results

This branch's checkpoint is `19fab3f` (the V01–V07/B00–B02/P01/T00/C00 wave).

**Baseline.** The baseline was measured on this same checkout by running the
full workspace suite with exactly the ten new test names skipped
(`logs/clean_baseline.log`), which avoids trusting a separately-timed run:

```
passed: 834   failed: 0   filtered out: 10
```

834 + 10 = 844, so the ten filtered tests are precisely the delta this card
introduces. The card's ">819" expectation is consistent with the 834 baseline.

| Gate | Command | Expected | Observed | Exit |
|---|---|---|---|---|
| Workspace tests | `cargo test --workspace` | 834 + 10 | **844 passed / 0 failed** | 0 |
| Format | `cargo fmt --all -- --check` | clean | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 | exit 0 | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 | exit 0 | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | 45–46 | **46 passed / 0 failed** | 0 |
| Provider fmt | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | clean | clean | 0 |
| Provider clippy | `cargo clippy --locked --offline --manifest-path providers/Cargo.toml --all-targets -- -D warnings` | clean | clean | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | 43 | **43 ran, OK** | 0 |
| Lean build | `cd proof && lake build` | clean | **62 jobs, completed successfully** | 0 |
| Lean audits | `python3 -B scripts/check_lean_audits.py` | 20 | **20 audits passed** | 0 |

Provider tests are **46**, not the 45 recorded under card B01: the CMetrics
unit-result provider increment added one. That is a pre-existing count on this
branch, unaffected by C01 — no provider file was touched.

Raw logs are under [`logs/`](logs/): `clean_baseline.log`, `gate_test.log`,
`gate_clippy.log`, `gate_probe.log`, `gate_followup.log`,
`gate_providers_test.log`, `gate_providers_clippy.log`, `gate_python.log`,
`gate_lean_audits.log`.

### Pinned artifacts

The four pinned 185-unit audit artifacts are **untouched by construction**. The
complete diff of this card is two test files:

- `crates/qcore-c-gateway/tests/custody_summary_regression.rs` (new)
- `crates/qcore-c-gateway/tests/unit/legacy_summary.rs` (3 tests appended)

No file under `crates/*/src/`, `compiler/src/`, `contract/qcore/`, `providers/`
or `proof/` was modified, so no analysis input, contract corpus count, verdict,
projection or diagnostic can differ. No Lean parity work was required: no
semantics changed.

## 5. Sources for the inventory

- `crates/qcore-c-gateway/src/legacy_summary.rs` (module header and the
  `parameter_mode` / `out_mode` / `stored_mode` / `propose_result` /
  `reallocated_contract` / `observations_match_contract` chain)
- `crates/qcore-c-gateway/src/legacy_c.rs:529-600`
  (`check_legacy_safety_certificate`) and `:738-751` (`SummaryConflict`)
- `crates/qcore-schemas/src/contract.rs:106-150` (`ContractTrust`,
  `ResourceSummaryRecord`)
- `docs/reports/remaining-c00/README.md` and `inventory.json`
- `docs/reports/w4-custody-inference`, `w4-custody-extensions`,
  `w4-removed-custody`, `w4-nullable-removal`, `w4-out-forwarding`,
  `w4-indirect-access`, `w4-immutable-out-aliases`, `w4-contextual-recursion`,
  `w4-unsigned-countdown`, `w4-gateway-crate`

## 6. Scope and non-claims

- **No raw unit is promoted.** The Cycle 19 corpus remains 2 Safe / 42
  provisional Unsafe / 137 Unknown plus four data-only units, unpromoted.
- **No defect was found.** All four acceptance items hold on the merged
  implementation; six of the sub-claims simply had no test pinning them, which
  this card supplied. Absence of a pinning test was never evidence of a bug,
  and finding none is a valid `verify_or_extend` outcome.
- These tests establish that the gateway **refuses** unvalidated custody claims
  at the boundaries exercised. They do not establish that the admitted
  subprofiles are complete, that inference is maximally precise, or that any
  upstream C is correct.
- The C fixtures are synthetic `q_*` contract fixtures — trusted declared test
  assumptions, not verified facts about any library.
- No gate was weakened, no pinned expectation edited, no diagnostic downgraded,
  no `Unknown` flipped to `Safe`.
- CT coverage is unchanged at **11/50 mapped**; the contract corpus remains 59
  modules.
- Clang-dependent cases skip explicitly when Clang is unavailable and assert
  that an explicit `QCORE_CLANG` was not silently ignored.
