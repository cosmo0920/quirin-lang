-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

import QCore.CleanupReplayAgreement

-- Card `P04-B2`: the symbolic cleanup-replay vocabulary — the fourth
-- transported source vocabulary — and the admission of a transported proposal.
-- Model of `qcore_c_gateway::cleanup_replay_source`'s projector and of
-- `qcore_verification::cleanup_replay_source`'s `WitnessChecker::check` body.
--
-- Whether a descriptor was released, a mapping torn down or a file deleted, and
-- the `close`/`munmap`/`unlink` ABI bindings, are outside every statement below
-- and stay recorded in `docs/reports/remaining-p04-b2/README.md`.

-- The value lattice is closed: exactly nine distinct constructors, a complete
-- enumeration with no catch-all, and a truth function that refuses rather than
-- guessing on every symbolic value.
#print axioms QCore.CleanupReplayAgreement.the_lattice_has_nine_constructors
#print axioms QCore.CleanupReplayAgreement.the_nine_constructors_are_distinct
#print axioms QCore.CleanupReplayAgreement.the_lattice_enumeration_is_complete
#print axioms QCore.CleanupReplayAgreement.symbolic_values_have_no_truth
#print axioms QCore.CleanupReplayAgreement.the_decided_truths_are_decided

-- The step vocabulary is closed: a bare value-free return, a scan loop, a
-- termination, an out-of-vocabulary expression and an undeclared callee each
-- have no projection, and one refused step refuses the whole path.
#print axioms QCore.CleanupReplayAgreement.a_bare_return_has_no_projection
#print axioms QCore.CleanupReplayAgreement.a_scan_loop_has_no_projection
#print axioms QCore.CleanupReplayAgreement.a_termination_has_no_projection
#print axioms QCore.CleanupReplayAgreement.an_expression_outside_the_vocabulary_has_no_projection
#print axioms QCore.CleanupReplayAgreement.an_undeclared_callee_has_no_projection
#print axioms QCore.CleanupReplayAgreement.a_refused_step_refuses_the_path
#print axioms QCore.CleanupReplayAgreement.a_refused_tail_refuses_the_path

-- Non-vacuity of the projector: a real cleanup shape projects, and the same
-- shape with the one refused terminal form does not.
#print axioms QCore.CleanupReplayAgreement.a_real_cleanup_shape_projects
#print axioms QCore.CleanupReplayAgreement.the_same_shape_with_a_bare_return_does_not_project

-- The sampled prestate domains are fixed by the family, duplicate-free and
-- disjoint, so no case of one family can be read as a case of the other.
#print axioms QCore.CleanupReplayAgreement.the_outcome_family_samples_four_prestates
#print axioms QCore.CleanupReplayAgreement.the_delete_family_samples_five_prestates
#print axioms QCore.CleanupReplayAgreement.each_sampled_domain_is_duplicate_free
#print axioms QCore.CleanupReplayAgreement.the_two_domains_are_disjoint

-- Fail-closed directions of the admission rule. Each of the five inputs the
-- core supplies for itself — the task, the declared profile, the declared
-- family, the re-established prerequisite and its own replay — refuses on its
-- own, before anything a producer sent is read.
#print axioms QCore.CleanupReplayAgreement.a_refusing_core_admits_nothing
#print axioms QCore.CleanupReplayAgreement.a_profile_that_is_not_the_declaration_is_refused
#print axioms QCore.CleanupReplayAgreement.a_family_that_is_not_the_declared_one_is_refused
#print axioms QCore.CleanupReplayAgreement.a_cross_task_witness_is_refused
#print axioms QCore.CleanupReplayAgreement.a_foreign_task_is_refused
#print axioms QCore.CleanupReplayAgreement.a_proposal_over_another_domain_is_refused

-- The dependent-capability boundary: a prerequisite is re-established by the
-- core, never accepted from a request. Every field of the identity is
-- load-bearing, a refusing prerequisite can never be established, and a
-- dependent over one is never admitted.
#print axioms QCore.CleanupReplayAgreement.a_foreign_prerequisite_is_refused
#print axioms QCore.CleanupReplayAgreement.a_refusing_prerequisite_is_never_established
#print axioms QCore.CleanupReplayAgreement.a_dependent_over_a_refusing_prerequisite_is_refused
#print axioms QCore.CleanupReplayAgreement.every_prerequisite_field_is_load_bearing
#print axioms QCore.CleanupReplayAgreement.a_prerequisite_does_not_cross_tasks

-- Agreement discharges nothing: every admission is backed by a replay the core
-- ran itself, an admitted proposal *is* that replay, two admitted witnesses
-- cannot disagree, and a changed core replay retires the witness.
#print axioms QCore.CleanupReplayAgreement.an_admission_is_the_cores_own_replay
#print axioms QCore.CleanupReplayAgreement.an_admitted_witness_has_a_core_replay
#print axioms QCore.CleanupReplayAgreement.only_the_core_replay_is_admitted
#print axioms QCore.CleanupReplayAgreement.admitted_witnesses_never_disagree
#print axioms QCore.CleanupReplayAgreement.a_changed_core_replay_retires_the_witness

-- An admitted proposal answers exactly its family's whole sampled domain, so
-- it can never answer a smaller question than the one the core evaluates.
#print axioms QCore.CleanupReplayAgreement.an_admitted_proposal_answers_its_whole_domain
#print axioms QCore.CleanupReplayAgreement.an_admitted_proposal_has_its_domains_case_count
#print axioms QCore.CleanupReplayAgreement.an_admitted_outcome_proposal_has_four_cases
#print axioms QCore.CleanupReplayAgreement.an_admitted_delete_proposal_has_five_cases

-- Non-vacuity of the admission rule: the core's own replay, over its own
-- declared profile and family and its own derived prerequisite identity, is
-- admitted — so the fail-closed theorems above are not theorems about a route
-- that admits nothing.
#print axioms QCore.CleanupReplayAgreement.the_core_replay_is_admitted
#print axioms QCore.CleanupReplayAgreement.the_domain_rule_is_not_vacuous
