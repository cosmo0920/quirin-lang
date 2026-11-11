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

import QCore.ObservedWaitComposition

#print axioms QCore.ObservedWaitComposition.observer_flags_round_trip
#print axioms QCore.ObservedWaitComposition.unsupported_thread_sanitizer_refuses
#print axioms QCore.ObservedWaitComposition.unpaired_dependency_output_refuses
#print axioms QCore.ObservedWaitComposition.source_obligations_match_observation_sequence

#print axioms QCore.ObservedWaitComposition.projected_lifecycle_is_legal
#print axioms QCore.ObservedWaitComposition.model_generation_preserves_native_generation
#print axioms QCore.ObservedWaitComposition.fresh_name_is_injective_on_selected_domain
#print axioms QCore.ObservedWaitComposition.accepted_replay_requires_exact_join_assumptions
#print axioms QCore.ObservedWaitComposition.accepted_projection_retains_callback_identity
#print axioms QCore.ObservedWaitComposition.substituted_thread_assumption_refuses
#print axioms QCore.ObservedWaitComposition.invalid_manifest_identity_refuses
#print axioms QCore.ObservedWaitComposition.a_missing_identity_fact_refuses
#print axioms QCore.ObservedWaitComposition.an_unequal_identity_fact_refuses
#print axioms QCore.ObservedWaitComposition.accepted_identity_policy_has_exact_record_bound
#print axioms QCore.ObservedWaitComposition.a_substituted_policy_digest_refuses
#print axioms QCore.ObservedWaitComposition.a_missing_callback_identity_refuses
#print axioms QCore.ObservedWaitComposition.a_mutated_record_clause_refuses_the_eleven_record_profile
#print axioms QCore.ObservedWaitComposition.deleting_the_self_binding_clause_cannot_bypass_stale_context
