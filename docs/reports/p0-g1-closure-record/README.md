# P0-G1-CLOSURE-RECORD: G1 (certificate containment) evaluated against §9

Base main `76c1f9cfc` (r77). Discharges the named follow-up
`P0-G1-CLOSURE-RECORD` (registered by PRIORITY0-ATTRIBUTION,
[G1 finding](../priority0-remeasurement/README.md#g1-certificate-containment)).
No analyzer, contract, rule or Lean change. One test-only commit adds two
controls (below). No corpus figure can move, so no prediction and no
Fluent Bit measurement were made.

**Verdict: G1 is met**, for the scope stated in
[`docs/REEVALUATION_SPEC.md` §9](../../REEVALUATION_SPEC.md#9-ordered-implementation-and-acceptance-gates),
whose dated closure note links this page. The open items below are
excluded from G1 by the specification's own text, not waived.

## What G1 requires

§9, G1 row: deliverable "WM01–WM03 and VP01–VP02 fixes; explicit profile
restrictions"; criterion "Invalid candidates and insufficient evidence
cannot obtain the affected certificates; positive controls survive".

The affected certificates are the two the findings name:

- the C11 candidate certificate, `C11ExecutionCertificate`, returned only by
  `check_c11_execution` (`crates/qcore-core/src/weak_memory.rs`) on
  acceptance (the §1 claim "Candidate consistent": one supplied graph, one
  named model, not exploration);
- the plugin core certificate, `CoreCertificate`
  (`crates/qcore-verifier-host/src/lib.rs`), constructed in production at
  exactly two sites: `check_response` (evidence v1, after `check_evidence`)
  and `check_response_v2` (after `check_evidence_v2`). The third
  construction is the `#[cfg(test)]` composition fixture
  `conflicting_assumptions_make_composition_unknown`, which the spec's repair
  record already names as not issuable through the firewall.

## Evidence table, criterion by criterion

Fixes were made in `3827db1c0` (2026-09-05, "Fix weak-memory audit
findings and contain plugin certificates") and hardened in `895689b91`
(same day, follow-up repair); the code has since moved crates
(`e4ed3df5b` moved the weak-memory tests, `9f845c878` extracted the
verifier host). Current locations are given.

| Finding | Fix (commit; current file and function) | Negative control: the invalid candidate / insufficient evidence gets no certificate | Positive control that survives |
|---|---|---|---|
| WM01 (plain store/load with `AcquireRelease` accepted) | `3827db1c0`; `weak_memory.rs` `invalid_order`: explicit legality table per operation kind (store: relaxed, release, seq_cst; load: relaxed, acquire, seq_cst) | probe `WM01_acq_rel_plain_operations` (rejected, `InvalidMemoryOrder(1)`, `InvalidMemoryOrder(2)`); `weak_memory::tests::wm01_plain_load_and_store_order_legality_table`: every order on a store and on a load separately, each illegal one rejected with `InvalidMemoryOrder` at that event | probe `CONTROL_relaxed_plain_operations` (accepted); the same WM01 test accepts each legal order, seq_cst included |
| WM02 (`init < x=2 < x=1` against same-thread `x=1; x=2` accepted) | `3827db1c0`; `validate_coherence` (with `validate_modification_order`): write/write, write/read, read/write, read/read coherence against happens-before and mo | probe `WM02_reversed_write_coherence` (rejected); `wm02_each_coherence_direction_rejects_reversal_and_accepts_forward_order`: each of the four directions reversed, rejected with `CoherenceViolation` | the same test's forward order for each direction (accepted, with equal-valued distinct writes so identity, not value, decides) |
| WM03 (SC store buffering, both loads read 0, accepted) | `3827db1c0`; `validate_sc`: SC order constrained by mo, each SC load's source by the last preceding SC write | probe `WM03_sc_store_buffering_zero_zero` (rejected); `wm03_all_sc_store_buffering_orders_and_outcomes_match_serial_execution`: all 24 supplied SC orders x 4 outcomes against a serial-schedule oracle; `wm03_sc_modification_order_and_mixed_load_sources` (SC order against mo, and an SC load reading a write HB-before the last SC write) | the same oracle test accepts exactly the serially realisable (order, outcome) pairs; `relaxed_store_buffering_weak_outcome_is_permitted` (the relaxed 0/0 control); the mixed-source test's unordered non-SC source |
| VP01 (one `ApiRecognized` fact for a nonexistent file and unaudited revision obtains a `ResourceContracts` certificate) | `3827db1c0` + `895689b91`; `qcore-verifier-host` `check_evidence` ends in a terminal `Err("protocol v1 lacks core-derived obligations and capability-specific semantic witnesses")` for every v1 proposal; built-ins return `Unknown` | probe `VP01_recognition_is_not_resource_proof` (`Unknown`, no certificate); `cycle20_verifier_plugins::vp01_predicate_source_profile_and_capability_mutations_cannot_mint_safety` (every first-party manifest x capability x 7 predicates x source x profile x revision, built-in and echoing provider); `builtin_assertions_cannot_establish_safety`; `forged_safe_evidence_is_downgraded_to_unknown`; `vp01_vp02_process_proposals_reach_the_firewall_and_cannot_mint_safety` (an executable provider, rejected at the firewall with QP009, not by a transport failure) | evidence v1 has no legitimate positive control by design (the repair record: the old Boolean-fact certificates "were not legitimate semantic positive controls"). The positive path that survives is evidence v2: `w3_evidence_v2::complete_scope_dispatch_replays_observations_and_retains_claim_bounds` certifies a witness whose core replay discharges every core-derived obligation, and `plugin_firewall::only_a_registry_certificate_makes_a_plugin_safe` |
| VP02 (custom provider's `Safe` with empty claims and coverage for an empty request certified) | `3827db1c0` + `895689b91`; `check_evidence` rejects empty facts, claims or coverage first ("no core-established no-obligations proof"); v2: `check_evidence_v2` rejects an empty core inventory (QP016) | probe `VP02_empty_evidence_firewall` (`Unknown`, no certificate); probe `CONTROL_builtin_empty_is_unknown`; `vp02_empty_in_process_evidence_is_rejected_by_the_core` (QP009, "no-obligations"); the process test above with empty facts; **new** `w3_evidence_v2::g1_empty_core_inventory_cannot_mint_a_v2_certificate` (QP016, "no-obligations"); `composition_requires_exact_nonempty_inventory` (an empty or mismatched inventory composes to `Unknown`) | as VP01; the new test carries its own positive control (the unmodified task certifies) |

WM04 is G2's deliverable ("WM04 fix/profile rule"), not G1's; it is fixed
too (probe `WM04_c11_release_sequence` accepted; `wm04_*` tests).

### Runs at base `76c1f9cfc`

All through `sh private/toolchain/env.sh` with
`scripts/test_link_config.py`'s config; logs under the worktree's
`private/g1/` (driver `private/g1/run.sh`).

| Command | Exit | Counts |
|---|---|---|
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `failed_expectations` 0; 8 of 8 pass (WM01-WM04, VP01, VP02, two controls). [Output](artifacts/reevaluation_probe.json) |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | 11 of 11 pass (VP04-VP08, LC01, WM05, controls). [Output](artifacts/reevaluation_followup.json) |
| `cargo test -p qcore-core --lib -- weak_memory::tests` | 0 | 12 passed, 0 failed (1106 filtered) |
| `cargo test -p qcore-compiler --test cycle20_verifier_plugins` | 0 | 20 passed, 0 failed |
| `cargo test -p qcore-compiler --test w3_evidence_v2` | 0 | 8 passed, 0 failed |
| `cargo test -p qcore-verifier-host --test plugin_firewall` | 0 | 6 passed, 0 failed |
| same `w3_evidence_v2` at the card tip (`f5127016c`) | 0 | 10 passed, 0 failed (the 8 plus the two new controls) |

## Explicit profile restrictions in force

"Profile" is read as the §1 verdict-contract field: what each certificate
is restricted to, and what it refuses outside that. Each is enforced in
code and driven by a test.

| Restriction | What is refused | Enforced | Tested |
|---|---|---|---|
| Memory model: C11 as published (N1570 §5.1.2.4, §7.17), one standard; not C++20 release sequences, RC11 or LWG 2130 | Orders N1570 forbids on an operation (plain store acquire/acq_rel; plain load release/acq_rel; a compare-exchange failure order that is release, acq_rel or stronger than success) | `invalid_order`, `compare_exchange_failure_admitted` (`weak_memory.rs`; module doc states the standard) | `wm01_*`; `cg28_cas_order_validity_matches_lean`; `cg28_lean_litmus_vectors_match_the_rust_checker` |
| Represented event subset | `memory_order_consume` and mixed-size or overlapping accesses: not representable in `MemoryOrder` / `MemoryEventKind`, so no candidate using them can be accepted | the event API itself | structural (no value to test) |
| One supplied candidate | Acceptance certifies consistency of the supplied graph only, never exploration of all executions | `check_c11_execution` contract; exploration is a separate route (`execution_coverage`, CG29) | the spec's §1 claim table; not a G1 test subject |
| Plugin evidence v1 | Every `Safe` proposal, whatever its predicates (`ResourceBalanced`, `ConcurrencySafe` included), source, profile, revision or capability; every empty proposal | `check_evidence` terminal rejection; empty check first | VP01/VP02 rows above |
| Plugin `Unsafe` (v1 and v2) | Every `Unsafe` allegation becomes `Unknown` (QP010): no counterexample checker exists | `check_response`, `check_response_v2` | `vp06_unverified_unsafe_is_unknown_and_cannot_dominate_composition`; the v2 mutation `r.verdict = Unsafe`; `plugin_firewall::an_unsafe_claim_is_pending_unless_a_counterexample_checker_validates_it` (Pending in the firewall classification, never a certificate) |
| Plugin evidence v2: core checker | `Safe` without a trusted-core witness checker for the requested capability (QP016); with an incomplete derivation (QP016); with an empty core inventory (QP016); with coverage other than the core inventory, a witness bound to another task, or a replay that does not discharge every obligation (QP009) | `dispatch_v2`, `check_evidence_v2` | `malicious_v2_proposals_cannot_change_the_denominator_or_replay` (incl. no checker); `task_envelope_cannot_be_rebound_and_incomplete_derivations_stay_unknown`; `explicit_observation_distrust_cannot_dispatch_or_mint_a_certificate`; **new** `g1_empty_core_inventory_cannot_mint_a_v2_certificate`; the providers/conformance `no_checker` QP016 cases |
| Target profile (manifest) | A request whose capability or target (library, revision, profile, dialect, platform, backend) the manifest does not declare is refused before dispatch (QP005). Manifest declarations and `*` wildcards are routing only: they are necessary, never sufficient, for a certificate | `PluginManifest::supports` in `dispatch_v2` and v1 dispatch; scope resolution refuses an undeclared profile | **new** `g1_undeclared_target_profile_cannot_dispatch_or_mint_a_certificate`; `composed_five_providers` (every off-diagonal cell QP005) |
| Premises carried, not discharged | A v2 certificate for observation hooks carries `observations_trusted` and `observed_runs_only` bounds; explicit distrust refuses before dispatch | `derive_task`, `check_evidence_v2` | `complete_scope_dispatch_…` (asserts both in the certificate); `replay_requires_declared_trust_and_observed_run_bounds` |

Before this card, two of these refusal arms had no test that reached them:
the v2 empty-inventory arm of `check_evidence_v2` (VP02's v2 counterpart:
the existing empty-coverage mutation is refused by the coverage-equality
arm, against a non-empty inventory) and the undeclared-target-profile arm of QP005 (the
five-provider matrix exercises capability/library mismatches). Commit
`f5127016c` adds one test for each in `compiler/tests/w3_evidence_v2.rs`,
each asserting the specific code (QP016 with "no-obligations"; QP005 with
the provider never called) and carrying its positive control. Test-only;
no semantic change, so no Lean change.

## Open items excluded from G1, and why

| Item | Why it is outside G1 (text cited) | Tracking |
|---|---|---|
| Capability-specific witnesses; core-derived inventories, including legitimate empty scopes | §9 G2 row: "core-derived plugin obligations"; repair record: "This is containment of the two reported false-safe cases, not completion of G2–G6"; `check_evidence`'s comment: "Re-enable only with a core-derived inventory, … and a versioned capability-specific witness checker (reevaluation G2)" | G2; evidence v2 (W3 design) is that work |
| Memory-model relation, matched-oracle litmus suite, exploration completeness, WM04 profile rule | §9 G2 row | G2 |
| Third-party `Unsafe` witness validation | The spec names it "a separate open obligation" and assigns no gate. G1's criterion is that insufficient evidence cannot obtain a certificate; an unvalidated `Unsafe` obtains none (QP010 to `Unknown`, and it cannot dominate composition), so containment holds. Validating one is a new capability, not containment | `W45-PLUGIN-COUNTEREXAMPLE-CHECKER`, `PLUGIN-UNSAFE-MISBOUND-EVIDENCE-PENDING` |
| Source integration (VP05), CT stage mapping | §9 G3 row ("provider invocation, source/CFG/native evidence links") | G3 |
| Package identity (VP03), process-tree isolation (VP04) | Listed open in the repair record; not part of G1's deliverable (the VP01-VP02 fixes); VP04 and VP07-VP08 have followup-probe controls that pass | not assigned a gate by the spec |
| Lean implementation bridges (FM01) | §9 G5 row ("Implementation bridges, Lean CI"); the repair claims no Lean change | G5 |
| Cycle 19 route removal | §9: "Do not remove Cycle 19's existing analysis routes until G4 parity is recorded" | G4 |

What this record does not claim: soundness of `check_c11_execution` as
an implementation of N1570 (no bridge theorem; constraint 28), exploration
of any program's executions, any plugin `Safe` reachable through evidence
v1, or that a green suite proves the absence of an untested acceptance
path. The criterion is met by the controls named above; a later change
that adds a certificate construction site must bring its own negative and
positive controls.

`docs/design/W3_OBLIGATIONS_EVIDENCE_V2.md` already said "G1 certificate
containment is recorded complete (2026-09-05 repair)"; no record evaluated
the deliverable at the time. This page is that evaluation; the design text
is left as written.

## Gate

Recorded in [gate.json](gate.json) after the run; see the card reply for
commands, exits and counts.
