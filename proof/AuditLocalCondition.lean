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

import QCore.LocalCondition
#print axioms QCore.LocalCondition.unknown_never_waives_a_prerequisite
#print axioms QCore.LocalCondition.active_never_assumes_a_prerequisite
#print axioms QCore.LocalCondition.inactive_waives_only_its_own_prerequisite
#print axioms QCore.LocalCondition.unknown_covers_every_concrete_branch
#print axioms QCore.LocalCondition.an_unknown_undischarged_prerequisite_refuses
#print axioms QCore.LocalCondition.a_join_fact_was_established_on_both_paths
#print axioms QCore.LocalCondition.no_common_fact_leaves_no_join_facts
#print axioms QCore.LocalCondition.an_unmatched_instance_is_unresolved
#print axioms QCore.LocalCondition.shared_vectors_hold

#print axioms QCore.LocalCondition.contradictory_facts_never_discharge_a_condition
#print axioms QCore.LocalCondition.leaving_scope_never_exports_a_fresh_fact

#print axioms QCore.LocalCondition.a_matching_conditional_entry_precondition_discharges_the_body_use

#print axioms QCore.LocalCondition.a_bound_condition_is_not_visible_to_an_unaware_consumer
#print axioms QCore.LocalCondition.a_condition_block_is_not_visible_to_an_unaware_consumer

#print axioms QCore.LocalCondition.a_guarded_interface_never_publishes_an_ordinary_summary
#print axioms QCore.LocalCondition.shared_publication_vectors_hold
