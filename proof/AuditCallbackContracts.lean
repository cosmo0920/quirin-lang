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

import QCore.CallbackContracts

/-! Audited companion to CG22-CALLBACK-REGISTRY-CONTRACT: callback contracts (gateway rules 13u-13y): loop-safe composition of synchronous invocations and table dispatch, registries decided only under a registration in force, and element custody. -/

#print axioms QCore.CallbackContracts.invocations_of_identities_are_identity
#print axioms QCore.CallbackContracts.decide_refuses_an_open_set
#print axioms QCore.CallbackContracts.decide_refuses_an_empty_set
#print axioms QCore.CallbackContracts.decide_refuses_a_missing_contract
#print axioms QCore.CallbackContracts.decide_refuses_an_effectful_candidate
#print axioms QCore.CallbackContracts.decide_never_releases_outside_a_registry
#print axioms QCore.CallbackContracts.decide_identity_needs_identities
#print axioms QCore.CallbackContracts.an_identity_decision_is_the_custody_reading
#print axioms QCore.CallbackContracts.an_invocation_needs_a_registration
#print axioms QCore.CallbackContracts.an_element_needs_a_registration
#print axioms QCore.CallbackContracts.a_clear_needs_a_registration
#print axioms QCore.CallbackContracts.the_callback_trace_agrees
#print axioms QCore.CallbackContracts.an_undecided_registration_refuses
#print axioms QCore.CallbackContracts.a_moved_element_is_released_by_its_registry
#print axioms QCore.CallbackContracts.a_moved_element_left_at_the_exit_is_lost
#print axioms QCore.CallbackContracts.a_moved_element_dropped_without_invocation_is_lost
#print axioms QCore.CallbackContracts.a_retained_element_returns_on_clear
#print axioms QCore.CallbackContracts.a_retained_element_at_the_exit_is_refused
#print axioms QCore.CallbackContracts.releasing_needs_every_candidate
#print axioms QCore.CallbackContracts.decision_vectors_hold
#print axioms QCore.CallbackContracts.decision_vectors_count
#print axioms QCore.CallbackContracts.registry_vectors_hold
#print axioms QCore.CallbackContracts.registry_vectors_count
#print axioms QCore.CallbackContracts.identity_only_borrows_or_requires
#print axioms QCore.CallbackContracts.a_release_is_only_of_an_element
#print axioms QCore.CallbackContracts.requirements_are_owned_parameters
#print axioms QCore.CallbackContracts.classify_vectors_hold
