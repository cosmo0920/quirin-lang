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

import QCore.CollectorIdentity
#print axioms QCore.CollectorIdentity.selection_keeps_the_exact_key
#print axioms QCore.CollectorIdentity.selection_returns_an_actual_entry
#print axioms QCore.CollectorIdentity.the_first_match_wins_with_duplicate_keys
#print axioms QCore.CollectorIdentity.an_empty_list_has_no_selected_record
#print axioms QCore.CollectorIdentity.selection_uses_the_exact_owner_list
#print axioms QCore.CollectorIdentity.another_key_cannot_supply_the_selected_record
#print axioms QCore.CollectorIdentity.equal_keys_do_not_identify_records
#print axioms QCore.CollectorIdentity.changing_the_list_can_change_the_selected_record
#print axioms QCore.CollectorIdentity.construction_keeps_the_exact_instance
#print axioms QCore.CollectorIdentity.the_threaded_snapshot_uses_that_workers_loop
#print axioms QCore.CollectorIdentity.the_nonthreaded_snapshot_uses_the_config_actual
#print axioms QCore.CollectorIdentity.a_different_config_is_not_the_instance_config
#print axioms QCore.CollectorIdentity.construction_does_not_prove_the_future_loop
#print axioms QCore.CollectorIdentity.a_changed_flag_does_not_reconstruct_the_loop
#print axioms QCore.CollectorIdentity.replay_requires_all_identity_and_stability_obligations
#print axioms QCore.CollectorIdentity.an_unstable_list_supplies_no_cross_call_identity
#print axioms QCore.CollectorIdentity.an_unstable_record_supplies_no_persistent_loop_identity
#print axioms QCore.CollectorIdentity.an_unstable_flag_supplies_no_conditional_reuse

#print axioms QCore.CollectorIdentity.list_selection_alone_does_not_prove_the_instance_member
#print axioms QCore.CollectorIdentity.a_separate_membership_invariant_establishes_the_instance

#print axioms QCore.CollectorIdentity.a_disjoint_subobject_frame_preserves_both_outer_members
