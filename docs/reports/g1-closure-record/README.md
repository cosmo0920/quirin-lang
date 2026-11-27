# P0-G1-CLOSURE-RECORD: gate G1 (certificate containment), evaluated

Card P0-G1-CLOSURE-RECORD (round 41; USER DECISION 2026-11-27: "the G1
closure record and CONNECT-LIBC-EFFECT-CLAIMS are still open. We need to
launch this card"). It discharges the follow-up P0-G1-CLOSURE-RECORD,
which PRIORITY0-ATTRIBUTION registered in
[the priority-0 remeasurement](../priority0-remeasurement/README.md#g1-certificate-containment).
Base `9903b027a`. No analyzer, contract or gateway semantic change. The
card adds two firewall control tests, the `G1` records vocabulary and its
validator-checked formulas.

## Verdict

**G1 is met, within the scope stated below.** Every conjunct of the
criterion holds for each of WM01, WM02, WM03, VP01 and VP02, with an
executed negative control, an executed positive control and an executed
profile control. The formula `g1_gate_met_when` holds over the case table.
The scope is the certificates and profiles listed under
[Scope](#scope-certificates-and-profiles). The
[open items](#open-items-this-record-does-not-cover) are outside it. They
include the two the specification names: capability-specific witnesses for
capabilities that have no core checker, and third-party `Unsafe` witness
validation. G2 through G6 are not touched.

`docs/REEVALUATION_SPEC.md` §9 defines G1 this way.

- Deliverable: "WM01–WM03 and VP01–VP02 fixes; explicit profile
  restrictions".
- Criterion: "Invalid candidates and insufficient evidence cannot obtain
  the affected certificates; positive controls survive".

The 2026-09-05 six-finding repair recorded the fixes. This record adds what
no earlier record had: an evaluation of the profile-restriction deliverable
and of the criterion, per finding, against controls executed at `9903b027a`.

## The criterion as formulas

The terms are the `G1` block of the C04.18.9 typed rendering
(`RENDERED_TERMS`, block `G1`; sorts `g1_finding` and `g1_case`; contract
spec C04.18.9,
[certificate-containment gate block](../../QCORE_CONTRACT_SPEC_r0_3.md#certificate-containment-gate-block-g1)).
`crates/qcore-frontend/tests/g1_closure_formulas.rs` (`FORMULAS`) checks
every formula with the premise-formula validator `guarded_proposition`. The
same test decides each formula over the case table below (`CASES`), and
checks that this page states each formula verbatim and lists each case. The
prose after each formula is only a gloss.

Per case:

- `g1_negative_controls_contained`:
  `forall (#c: g1_case) { ((#g1_invalid(#c) || #g1_insufficient(#c)) => !#g1_certified(#c)) }`
  (an invalid candidate, or insufficient evidence, obtains no certificate).
- `g1_positive_controls_survive`:
  `forall (#c: g1_case) { (#g1_positive(#c) => #g1_certified(#c)) }`.
- `g1_positive_is_valid_and_sufficient`:
  `forall (#c: g1_case) { (#g1_positive(#c) => (!#g1_invalid(#c) && !#g1_insufficient(#c))) }`.
- `g1_certificate_inside_profile`:
  `forall (#c: g1_case) { (#g1_certified(#c) => #g1_in_profile(#c)) }`
  (a certificate only inside the finding's explicit profile restriction).
- `g1_outside_profile_is_unknown`:
  `forall (#c: g1_case) { (!#g1_in_profile(#c) => #g1_unknown(#c)) }`
  (fail-closed: an unmatched profile is refused with a named reason).
- `g1_unknown_is_uncertified`:
  `forall (#c: g1_case) { (#g1_unknown(#c) => !#g1_certified(#c)) }`.

Per finding, and the gate:

- `g1_finding_closed_needs_controls`:
  `forall (#f: g1_finding) { (#g1_closed(#f) => ((exists (#n: g1_case) { (#g1_of(#n,#f) && (#g1_invalid(#n) || #g1_insufficient(#n))) } && exists (#p: g1_case) { (#g1_of(#p,#f) && #g1_positive(#p)) }) && exists (#o: g1_case) { (#g1_of(#o,#f) && !#g1_in_profile(#o)) })) }`
- `g1_finding_closed_needs_every_case`:
  `forall (#f: g1_finding) { (#g1_closed(#f) => forall (#c: g1_case) { (#g1_of(#c,#f) => (((#g1_executed(#c) && ((#g1_invalid(#c) || #g1_insufficient(#c)) => !#g1_certified(#c))) && (#g1_positive(#c) => #g1_certified(#c))) && ((#g1_certified(#c) => #g1_in_profile(#c)) && (!#g1_in_profile(#c) => #g1_unknown(#c))))) }) }`
- `g1_finding_closed_when`:
  `forall (#f: g1_finding) { ((((exists (#n: g1_case) { (#g1_of(#n,#f) && (#g1_invalid(#n) || #g1_insufficient(#n))) } && exists (#p: g1_case) { (#g1_of(#p,#f) && #g1_positive(#p)) }) && exists (#o: g1_case) { (#g1_of(#o,#f) && !#g1_in_profile(#o)) }) && forall (#c: g1_case) { (#g1_of(#c,#f) => (((#g1_executed(#c) && ((#g1_invalid(#c) || #g1_insufficient(#c)) => !#g1_certified(#c))) && (#g1_positive(#c) => #g1_certified(#c))) && ((#g1_certified(#c) => #g1_in_profile(#c)) && (!#g1_in_profile(#c) => #g1_unknown(#c))))) }) => #g1_closed(#f)) }`
- `g1_gate_met_needs_every_finding`:
  `(#g1_gate_met() => forall (#f: g1_finding) { #g1_closed(#f) })`
- `g1_gate_met_when`:
  `(forall (#f: g1_finding) { #g1_closed(#f) } => #g1_gate_met())`

Refusals (each leaves the finding, or the gate, open):

- `g1_refuse_no_negative_control`:
  `forall (#f: g1_finding) { (!exists (#n: g1_case) { (#g1_of(#n,#f) && (#g1_invalid(#n) || #g1_insufficient(#n))) } => !#g1_closed(#f)) }`
- `g1_refuse_no_positive_control`:
  `forall (#f: g1_finding) { (!exists (#p: g1_case) { (#g1_of(#p,#f) && #g1_positive(#p)) } => !#g1_closed(#f)) }`
- `g1_refuse_no_profile_control`:
  `forall (#f: g1_finding) { (!exists (#o: g1_case) { (#g1_of(#o,#f) && !#g1_in_profile(#o)) } => !#g1_closed(#f)) }`
- `g1_refuse_unexecuted`:
  `forall (#c: g1_case) { (!#g1_executed(#c) => forall (#f: g1_finding) { (#g1_of(#c,#f) => !#g1_closed(#f)) }) }`
- `g1_refuse_open_finding`:
  `(exists (#f: g1_finding) { !#g1_closed(#f) } => !#g1_gate_met())`

`#g1_certified(#c)` means one of two things. For WM, `check_c11_execution`
returned `Ok` (a `C11ExecutionCertificate`). For VP, the outcome carries a
`CoreCertificate`. `#g1_in_profile(#c)` means the case's input lies inside
the profile restriction listed for its finding below.

## Per-finding evaluation

| Finding | Affected certificate | Negative control (cannot obtain it) | Positive control (survives) | Profile restriction in force | Formula conjunct | Holds? |
|---|---|---|---|---|---|---|
| WM01 (plain store/load accepted `AcquireRelease`) | `C11ExecutionCertificate` from `qcore_core::weak_memory::check_c11_execution` | probe `WM01_acq_rel_plain_operations` (rejected); `wm01_plain_load_and_store_order_legality_table`: every illegal order on a store or a load, each separately, gets `InvalidMemoryOrder` | probe `CONTROL_relaxed_plain_operations` (accepted); the same test's legal orders, SC included, accepted | R-WM1, R-WM2 (`invalid_order`, Lean `Kind.orderValid`), R-WM3 | `g1_finding_closed_when` at WM01 | yes |
| WM02 (reversed same-thread coherence accepted) | as WM01 | probe `WM02_reversed_write_coherence`; `wm02_each_coherence_direction_rejects_reversal_and_accepts_forward_order`, reversed half (write/write, write/read, read/write, read/read) | the same test's forward half, including equal-valued distinct writes | R-WM1, R-WM3; `validate_coherence` (Lean `Execution.coherent`) | `g1_finding_closed_when` at WM02 | yes |
| WM03 (SC store buffering 0/0 accepted) | as WM01 | probe `WM03_sc_store_buffering_zero_zero`; `wm03_all_sc_store_buffering_orders_and_outcomes_match_serial_execution`: of all 24 supplied S orders by 4 outcomes, those the serial-schedule oracle forbids; Lean `sb_seq_cst_weak_outcome_refused`, run through the Rust checker by `cg28_lean_litmus_vectors_match_the_rust_checker` | the oracle-permitted rows; `relaxed_store_buffering_weak_outcome_is_permitted`; Lean vector `sbRelaxed` (`[]`) through the same binding test | R-WM1, R-WM3; `validate_sc` (Lean `Execution.scValid`) | `g1_finding_closed_when` at WM03 | yes |
| VP01 (an `ApiRecognized` fact obtained a `ResourceContracts` certificate) | `CoreCertificate` from `VerifierRegistry::verify` (v1) and `verify_v2` (v2) | probe `VP01_recognition_is_not_resource_proof` (`Unknown`, `certificate: false`); `vp01_predicate_source_profile_and_capability_mutations_cannot_mint_safety` (every first-party manifest and capability, 7 predicates by 2 sources by 2 profiles by 2 revisions, built-in and echoing provider); `vp01_vp02_process_proposals_reach_the_firewall_and_cannot_mint_safety`, non-empty half (QP009 at the firewall) | v2 certificates through a core witness replay: `only_a_registry_certificate_makes_a_plugin_safe`; `a_provider_cannot_remove_or_substitute_a_requested_obligation`, honest half (trusted-core source-effect checker, Clang 21.1.8); this card's positive halves | R-VP1 to R-VP5 | `g1_finding_closed_when` at VP01 | yes |
| VP02 (empty claims and coverage obtained a certificate) | as VP01 | probe `VP02_empty_evidence_firewall` (`Unknown`, `certificate: false`); `vp02_empty_in_process_evidence_is_rejected_by_the_core` (QP009 "no-obligations"); process half of the VP01 test, empty request; **new** `g1_vp02_empty_core_inventory_cannot_obtain_a_v2_certificate` (QP016 at the v2 firewall); `malicious_v2_proposals_cannot_change_the_denominator_or_replay` (cleared or phantom coverage, cleared runs) | as VP01 | R-VP1 to R-VP5 | `g1_finding_closed_when` at VP02 | yes |

The new v2 empty-inventory control is a genuine negative control. With the
firewall's empty-inventory refusal disabled (`required.is_empty() && false`
in `check_evidence_v2`, a temporary local edit that was reverted and not
committed), the test failed at its first assertion: `left: Safe, right:
Unknown`. A certificate would have been minted for the empty scope. The
mutation run's log is `private/g1/mutation.log`.

## Case table

Each row is one value of `g1_case`. The row's classes and observations are
`CASES` in `crates/qcore-frontend/tests/g1_closure_formulas.rs`. A row is
exactly one of three kinds: negative (invalid or insufficient), positive, or
profile (`!#g1_in_profile`). Every row was executed at this record's
revision and met its expectation (see [Checks](#checks)).

| Case | Findings | Kind | Certified | Evidence |
|---|---|---|---|---|
| `g1_case_probe_wm01_acq_rel_plain_operations` | WM01 | invalid | no | `reevaluation_probe` entry `WM01_acq_rel_plain_operations` |
| `g1_case_test_wm01_illegal_orders` | WM01 | invalid | no | `crates/qcore-core/tests/unit/weak_memory.rs` `wm01_plain_load_and_store_order_legality_table` |
| `g1_case_probe_control_relaxed_plain_operations` | WM01 | positive | yes | probe entry `CONTROL_relaxed_plain_operations` |
| `g1_case_test_wm01_legal_orders` | WM01 | positive | yes | `wm01_plain_load_and_store_order_legality_table` |
| `g1_case_probe_wm02_reversed_write_coherence` | WM02 | invalid | no | probe entry `WM02_reversed_write_coherence` |
| `g1_case_test_wm02_reversed_coherence` | WM02 | invalid | no | `wm02_each_coherence_direction_rejects_reversal_and_accepts_forward_order` |
| `g1_case_test_wm02_forward_coherence` | WM02 | positive | yes | the same test |
| `g1_case_probe_wm03_sc_store_buffering_zero_zero` | WM03 | invalid | no | probe entry `WM03_sc_store_buffering_zero_zero` |
| `g1_case_test_wm03_serial_oracle_forbidden` | WM03 | invalid | no | `wm03_all_sc_store_buffering_orders_and_outcomes_match_serial_execution`; `wm03_sc_modification_order_and_mixed_load_sources` |
| `g1_case_test_cg28_lean_vector_sb_seq_cst` | WM03 | invalid | no | Lean `sb_seq_cst_weak_outcome_refused`; `cg28_lean_litmus_vectors_match_the_rust_checker` |
| `g1_case_test_wm03_serial_oracle_permitted` | WM03 | positive | yes | `wm03_all_sc_store_buffering_orders_and_outcomes_match_serial_execution` |
| `g1_case_test_relaxed_store_buffering_permitted` | WM03 | positive | yes | `relaxed_store_buffering_weak_outcome_is_permitted` |
| `g1_case_test_cg28_lean_vector_sb_relaxed` | WM03 | positive | yes | Lean `litmusVectors` row `sbRelaxed` (`litmus_vectors_hold`); `cg28_lean_litmus_vectors_match_the_rust_checker` |
| `g1_case_test_atomic_route_refusals` | WM01, WM02, WM03 | profile | no | `crates/qcore-c-gateway/tests/unit/atomic_source_route.rs` `cg19_refusals_name_the_form_and_card` (consume RMW and load, `__sync`, by-pointer, unlisted, unevaluated weak flag) |
| `g1_case_probe_vp01_recognition_is_not_resource_proof` | VP01 | invalid | no | probe entry `VP01_recognition_is_not_resource_proof` |
| `g1_case_test_vp01_mutations` | VP01 | invalid | no | `compiler/tests/cycle20_verifier_plugins.rs` `vp01_predicate_source_profile_and_capability_mutations_cannot_mint_safety` |
| `g1_case_test_vp01_process_nonempty` | VP01 | invalid | no | `vp01_vp02_process_proposals_reach_the_firewall_and_cannot_mint_safety` |
| `g1_case_probe_vp02_empty_evidence_firewall` | VP02 | insufficient | no | probe entry `VP02_empty_evidence_firewall` |
| `g1_case_test_vp02_empty_in_process` | VP02 | insufficient | no | `vp02_empty_in_process_evidence_is_rejected_by_the_core` |
| `g1_case_test_vp02_process_empty` | VP02 | insufficient | no | `vp01_vp02_process_proposals_reach_the_firewall_and_cannot_mint_safety` |
| `g1_case_vp02_v2_empty_inventory` | VP02 | insufficient | no | **new** `crates/qcore-verifier-host/tests/plugin_firewall.rs` `g1_vp02_empty_core_inventory_cannot_obtain_a_v2_certificate` |
| `g1_case_test_v2_malicious_proposals` | VP01, VP02 | insufficient | no | `compiler/tests/w3_evidence_v2.rs` `malicious_v2_proposals_cannot_change_the_denominator_or_replay` |
| `g1_case_test_v2_registry_certificate` | VP01, VP02 | positive | yes | `plugin_firewall.rs` `only_a_registry_certificate_makes_a_plugin_safe` |
| `g1_case_test_v2_source_effect_honest_coverage` | VP01, VP02 | positive | yes | `crates/qcore-verification/tests/source_effect_obligations.rs` `a_provider_cannot_remove_or_substitute_a_requested_obligation` |
| `g1_case_vp02_v2_positive_control` | VP01, VP02 | positive | yes | **new**, positive halves of both new `plugin_firewall.rs` tests |
| `g1_case_vp_v2_unmatched_target` | VP01, VP02 | profile | no | **new** `plugin_firewall.rs` `g1_unmatched_target_profile_cannot_obtain_a_v2_certificate` (profile, library, dialect, backend and capability each outside the manifest: QP005) |
| `g1_case_test_v2_task_envelope_rebound` | VP01, VP02 | profile | no | `w3_evidence_v2.rs` `task_envelope_cannot_be_rebound_and_incomplete_derivations_stay_unknown` (target revision and unit rebound against the core task: QP001) |

## Profile restrictions in force

Each restriction below confines an affected certificate and fails closed:
an input outside it is refused with a named reason and publishes no
certificate.

| Id | Restriction | Where | Fail-closed? |
|---|---|---|---|
| R-WM1 | One memory model: C11 as published (N1570), the atomic fragment (plain loads and stores, RMWs, compare-exchange, thread and signal fences, release sequences, S, non-atomic races, mutex histories, spawn edges). It is not C++20, RC11 or LWG 2130. There is no consume order, no dependency ordering and no mixed-size access. No model parameter exists, so nothing can request another model. | `crates/qcore-core/src/weak_memory.rs` module doc; `QCore.WeakMemory` ("One standard") | Yes, by construction. A form the model lacks has no `MemoryEventKind` and cannot be submitted. |
| R-WM2 | Per-operation order legality. A store is relaxed, release or SC; a load is relaxed, acquire or SC; a compare-exchange's failure order is admitted by `casOrdersValid`. | `invalid_order` (`InvalidMemoryOrder`); Lean `Kind.orderValid`, `cas_failure_no_stronger` | Yes. `Err`, no certificate. |
| R-WM3 | The C atomic source route (gateway rules 11i and 11t) routes only represented forms and orders. It refuses consume on a load, RMW or compare-exchange; the `__sync` builtins; generic by-pointer forms; unlisted builtins; and unevaluated orders or weak flags. A failed or undecided check is `Unknown` and carries the single-graph assumption. | `crates/qcore-c-gateway/src/atomic_source_route.rs` (`classify_builtin`, `event_order_represented`, `SINGLE_GRAPH_ASSUMPTION`); Lean `QCore.AtomicSourceRoute` (`routed_order_is_represented`, `consume_load_refused`, `consume_order_refused`, `legacy_sync_refused`, `by_pointer_refused`, `unlisted_refused`, `failed_publication_is_unknown`, `undecided_publication_is_unknown`) | Yes |
| R-VP1 | Protocol v1 issues no certificate. `check_evidence` replays the envelope, then returns `Err` unconditionally ("protocol v1 lacks core-derived obligations and capability-specific semantic witnesses"), in every profile. Empty facts, claims or coverage are refused first. | `crates/qcore-verifier-host/src/lib.rs` `check_evidence` (QP009); `BuiltInProvider::verify` answers `Unknown`; Lean `QCore.VerifierPlugin.invalid_evidence_is_unknown`, `safe_without_evidence_is_unknown` | Yes |
| R-VP2 | Capability and target match. The manifest must declare the requested capability, and one `SupportedTarget` must match library, revision, profile, dialect, platform and backend. Protocol and evidence versions must also be declared. | `VerifierRegistry::verify` and `dispatch_v2` (QP004, QP005); `PluginManifest::supports`, `SupportedTarget::matches` | Yes. QP005 comes before the provider is consulted (new test). |
| R-VP3 | v2 task binding. The request's target (the profile included), unit, assumptions, evidence version and obligations must equal the core-derived task's (`task_digest`, which also binds `semantic_profile`), and every source projection must be bound to the task. | `dispatch_v2` (QP001) | Yes |
| R-VP4 | A registered trusted-core `WitnessChecker` for exactly the requested capability. No incomplete derivation for it. A non-empty core inventory. | `dispatch_v2` (QP016, incomplete derivation); `check_evidence_v2` (QP016: no checker, checker capability mismatch, empty inventory) | Yes. Empty inventory: new test. |
| R-VP5 | The replay must discharge exactly the core inventory, and the proposal's coverage must equal it. | `check_evidence_v2` (QP009) | Yes |

One restriction is **not** relied upon. The first-party manifests
(`contract/profiles/*/providers/qcore.*.json`) declare wildcard revisions,
profiles, platforms and backends and `evidence_versions: [1]`. As the
six-finding repair says, they describe configuration routing only. They are
contained by R-VP1, not by their targets: no v1 proposal obtains a
certificate whatever its target.

## Scope: certificates and profiles

The closure covers the following.

- **WM01–WM03.** `C11ExecutionCertificate` from `check_c11_execution`, for
  one supplied candidate graph inside R-WM1, and the gateway atomic-route
  verdicts built on it inside R-WM3. It establishes that the checker refuses
  the three invalid candidate shapes, and their regression families, while
  the controls are accepted. It does not establish exploration of all
  executions. The atomic-publication route's all-executions claim is CG29's,
  within its stated bounds.
- **VP01–VP02.** `CoreCertificate` from `VerifierRegistry::verify`
  (protocol v1: none can be issued) and `VerifierRegistry::verify_v2`
  (evidence v2: only through a registered trusted-core witness replay that
  discharges a non-empty core inventory, inside R-VP2 to R-VP5). This
  includes the W45 plugin-claim classification, whose `CertifiedSafe` is
  exactly a `verify_v2` certificate
  (`the_classification_never_disagrees_with_the_registry_on_safe`). The
  capabilities with registered core checkers are `ResourceContracts`
  (source-effect, counter, frame, call-use, cleanup-replay, composition and
  teardown checkers in `crates/qcore-verification`), `ChunkLifecycle` and
  `ObservationHooks`.

## Open items this record does not cover

| Item | Status | Tracking |
|---|---|---|
| Capability-specific witnesses for `EventTraces`, `Concurrency` and `TranslationUnitProjection` | No core checker. Every request is `Unknown` (QP016), so this is contained, not verified. | **new** `G1-EXCLUDED-CAPABILITIES-WITHOUT-CHECKER` |
| Third-party `Unsafe` witness validation | The registry's v1 and v2 paths keep every `Unsafe` an allegation (QP010). The W45 plugin-claim path validates one only through a `CounterexampleChecker`, and no trusted-core one exists. | `W45-PLUGIN-COUNTEREXAMPLE-CHECKER`, `PLUGIN-UNSAFE-MISBOUND-EVIDENCE-PENDING` |
| Legitimate empty scopes | No independently checked no-obligations argument exists, so a genuinely empty scope is `Unknown` (v1 QP009, v2 QP016). This is a precision gap, not a containment gap. | **new** `G1-EXCLUDED-EMPTY-SCOPE-ARGUMENT` |
| The memory-model identity on the WM certificate | `C11ExecutionCertificate` names no model or profile. The model is fixed by R-WM1, but §1's verdict tuple asks a published result to name its semantic profile. | **new** `G1-WM-CERTIFICATE-MODEL-IDENTITY` |
| Mixed-size, union and bit-field atomic accesses at the source route | They are outside R-WM1 (no event kind). This record executed no control showing that the source route refuses an overlapping or differently sized access to an atomic object, rather than reading it as a represented event. | **new** `G1-WM-MIXED-SIZE-ROUTE-CONTROL` |
| WM04 and the G2 deliverables: the matched-oracle litmus suite, the evidence-mutation corpus, core-derived plugin obligations | Gate G2, not G1 | `docs/REEVALUATION_SPEC.md` §9 |
| Rust-to-Lean correspondence of the weak-memory checker; the Lean `VerifierPlugin` model is an evidence-flag model (FM01) | A stated review premise (`QCore.WeakMemory` doc). The Lean vectors bind the listed graphs only. | §4 FM01, gate G5 |
| Package identity, process deadlines, source integration (VP03–VP05) | Not G1 findings. The follow-up probes VP04, VP05 and VP07–VP08 pass in `reevaluation_followup`, but this record does not evaluate them. | §4 and `docs/REEVALUATION_FOLLOWUP.md`; gates G3 and G5 |

A closure here is not a claim of G2–G6. It is not a claim that any
application is safe, and not a claim that the probes exhaust the invalid
candidates. Zero observed acceptances on a finite set of controls is not a
soundness theorem (§7).

One weakness is noted, not changed. `reevaluation_probe` scores VP01 and VP02
on the verdict alone (`pass = verdict == Unknown`). It prints `certificate`
but does not gate on it. The certificate's absence is asserted by the
cycle-20 tests and by both new tests. In this run the probe printed
`certificate: false` for both. The probe source stays unchanged, as the
six-finding repair records it.

## Probe runs

Both examples ran once through `scripts/gate_slot.py build`, with the
test-build configuration of `scripts/test_link_config.py`. The script is
`private/g1/probes.sh` and the receipt `private/g1/probes-receipt.json`.

| Example | Exit | `failed_expectations` | Probes | Log |
|---|---|---|---|---|
| `reevaluation_probe` | 0 | 0 | 8 (WM01–WM04, VP01, VP02, two controls) | `private/g1/probe.json` |
| `reevaluation_followup` | 0 | 0 | 11 (VP04–VP08, LC01, WM05 x2, three controls) | `private/g1/followup.json` |

## Checks

Recorded in the hand-back and in `private/g1/tests/` (one log per command,
exits in `private/g1/tests/exits.txt`).
