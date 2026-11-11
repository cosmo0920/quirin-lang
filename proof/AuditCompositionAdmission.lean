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

import QCore.CompositionAdmission

-- Card `P04-COMPOSE`: the composition vocabulary — the fifth transported source
-- vocabulary — and the admission of a transported composition proposal, whose
-- reading is a function of *other capabilities'* certificates. Model of
-- `qcore_c_gateway::composition_source`'s projector inputs and of
-- `qcore_verification::composition_source`'s `WitnessChecker::check` body.
--
-- Whether a mapping was torn down, and the `munmap`/sync/logging ABI bindings,
-- are outside every statement below and stay recorded in
-- `docs/reports/remaining-p04-compose/README.md`.

-- Prerequisite revalidation is a **total** decided function: it answers on
-- every (context, claim) pair, refuses a foreign or incomplete claim, refuses a
-- prerequisite whose own contract evaluation refuses, and is non-vacuous. Each
-- of the three identity fields is load-bearing on its own.
#print axioms QCore.CompositionAdmission.revalidation_is_total
#print axioms QCore.CompositionAdmission.a_foreign_claim_is_refused
#print axioms QCore.CompositionAdmission.a_refusing_prerequisite_is_never_established
#print axioms QCore.CompositionAdmission.an_incomplete_claim_is_refused
#print axioms QCore.CompositionAdmission.the_cores_own_identity_is_established
#print axioms QCore.CompositionAdmission.the_profile_field_is_load_bearing
#print axioms QCore.CompositionAdmission.the_task_digest_field_is_load_bearing
#print axioms QCore.CompositionAdmission.the_obligation_field_is_load_bearing

-- Fail-closed directions of the admission rule. Every conjunct the core
-- decides for itself refuses on its own, before anything a producer sent is
-- compared: the outer task binding, each of the three prerequisites, the
-- contract-derived record profile, both body ties, the triple's completeness
-- and distinctness, and the core's own composition.
#print axioms QCore.CompositionAdmission.a_refusing_core_admits_nothing
#print axioms QCore.CompositionAdmission.an_unbound_witness_is_refused
#print axioms QCore.CompositionAdmission.a_refusing_call_use_prerequisite_refuses
#print axioms QCore.CompositionAdmission.a_refusing_cleanup_prerequisite_refuses
#print axioms QCore.CompositionAdmission.a_refusing_counter_prerequisite_refuses
#print axioms QCore.CompositionAdmission.a_profile_that_is_not_the_declaration_is_refused
#print axioms QCore.CompositionAdmission.a_wrapper_body_the_prerequisite_did_not_check_is_refused
#print axioms QCore.CompositionAdmission.a_prerequisite_body_the_prerequisite_did_not_check_is_refused
#print axioms QCore.CompositionAdmission.an_incomplete_triple_is_refused
#print axioms QCore.CompositionAdmission.a_repeated_triple_is_refused
#print axioms QCore.CompositionAdmission.a_foreign_triple_is_refused
#print axioms QCore.CompositionAdmission.a_proposal_that_differs_is_refused

-- What an admission means: it is backed by work the core did, an admitted
-- proposal IS the core's own reading, two admitted proposals cannot disagree,
-- a changed core reading retires the witness, and every prerequisite was
-- re-established.
#print axioms QCore.CompositionAdmission.admission_implies_every_conjunct
#print axioms QCore.CompositionAdmission.an_admitted_proposal_is_the_cores_own_reading
#print axioms QCore.CompositionAdmission.two_admitted_proposals_cannot_disagree
#print axioms QCore.CompositionAdmission.a_changed_core_reading_retires_the_witness
#print axioms QCore.CompositionAdmission.an_admission_establishes_all_three

-- Non-vacuity: the core's own composition over its own prerequisites is
-- admitted, and the completeness and distinctness rules really discriminate.
#print axioms QCore.CompositionAdmission.the_cores_own_composition_is_admitted
#print axioms QCore.CompositionAdmission.the_distinctness_rule_is_decidable
#print axioms QCore.CompositionAdmission.the_completeness_rule_is_decidable

-- No composed result travels: the transported projection determines no
-- reading at all, and the same projection is refused by a core that did not
-- compose. A producer's proposal can never be evidence for itself.
#print axioms QCore.CompositionAdmission.the_projection_determines_no_reading
#print axioms QCore.CompositionAdmission.the_same_projection_is_refused_by_a_refusing_core

-- The chain: a prerequisite that is itself a dependent re-establishes its own
-- prerequisite first, so an unestablished link refuses the whole chain rather
-- than being absorbed by it.
#print axioms QCore.CompositionAdmission.an_unestablished_inner_prerequisite_refuses_the_chain
#print axioms QCore.CompositionAdmission.an_established_chain_is_established

-- Card `P04-COMPOSE-A`: the nested-prerequisite relation, where a composition's
-- own prerequisite is another composition. The two-level re-establishment is a
-- total decided function with no partial case; a forged **leaf** prerequisite
-- refuses under an honest **middle** identity and a succeeding nested
-- composition, so no level can be skipped by naming the level above it
-- honestly; and the outer route's answer is independent of the order in which
-- its three or four obligations are discharged.
--
-- Whether a file was unlinked, a descriptor closed or a mapping torn down is
-- outside every statement below and stays recorded in
-- `docs/reports/remaining-p04-compose-a/README.md`.
#print axioms QCore.CompositionAdmission.two_level_revalidation_is_total
#print axioms QCore.CompositionAdmission.a_forged_leaf_prerequisite_refuses_under_an_honest_middle_identity
#print axioms QCore.CompositionAdmission.a_refusing_leaf_call_use_refuses_the_nested_composition
#print axioms QCore.CompositionAdmission.a_refusing_leaf_cleanup_refuses_the_nested_composition
#print axioms QCore.CompositionAdmission.a_refusing_nested_composition_is_never_established
#print axioms QCore.CompositionAdmission.a_foreign_nested_identity_is_refused
#print axioms QCore.CompositionAdmission.an_honest_two_level_chain_is_established
#print axioms QCore.CompositionAdmission.all_id_is_permutation_invariant
#print axioms QCore.CompositionAdmission.establishing_the_obligations_is_order_independent
#print axioms QCore.CompositionAdmission.establishing_the_obligations_is_total
#print axioms QCore.CompositionAdmission.a_forged_leaf_refuses_the_whole_outer_route
#print axioms QCore.CompositionAdmission.the_deleting_arm_must_agree_in_both_directions
#print axioms QCore.CompositionAdmission.an_honest_outer_route_is_established
#print axioms QCore.CompositionAdmission.the_outer_distinctness_rule_is_decidable
#print axioms QCore.CompositionAdmission.the_file_close_question_nests_exactly_twice
