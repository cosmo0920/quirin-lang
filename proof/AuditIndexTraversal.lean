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

import QCore.IndexTraversal

/-! Audited Lean companion to card W43-INDEX-TRAVERSAL. -/

#print axioms QCore.IndexTraversal.mem_all
#print axioms QCore.IndexTraversal.join_comm_all
#print axioms QCore.IndexTraversal.join_idem_all
#print axioms QCore.IndexTraversal.join_assoc_all
#print axioms QCore.IndexTraversal.join_comm
#print axioms QCore.IndexTraversal.join_idem
#print axioms QCore.IndexTraversal.join_assoc
#print axioms QCore.IndexTraversal.mixed_absorbs
#print axioms QCore.IndexTraversal.an_unstepped_derived_result_still_joins_the_identity_to_mixed
#print axioms QCore.IndexTraversal.the_header_joins_the_identity_and_its_step_to_any
#print axioms QCore.IndexTraversal.stepped_and_unstepped_families_stay_apart
#print axioms QCore.IndexTraversal.a_step_is_never_the_identity
#print axioms QCore.IndexTraversal.a_step_of_a_non_mixed_binding_is_interior
#print axioms QCore.IndexTraversal.a_backward_step_may_be_at_the_start
#print axioms QCore.IndexTraversal.const_dropped_survives_a_step
#print axioms QCore.IndexTraversal.admits_of_covers
#print axioms QCore.IndexTraversal.join_covers_all
#print axioms QCore.IndexTraversal.join_admits_left
#print axioms QCore.IndexTraversal.join_admits_right
#print axioms QCore.IndexTraversal.le_admits
#print axioms QCore.IndexTraversal.a_forward_step_is_sound
#print axioms QCore.IndexTraversal.an_unknown_step_is_sound
#print axioms QCore.IndexTraversal.a_move_is_sound
#print axioms QCore.IndexTraversal.every_iteration_from_the_header_is_covered
#print axioms QCore.IndexTraversal.a_post_fixed_point_covers_every_iteration
#print axioms QCore.IndexTraversal.the_walk_header_is_a_post_fixed_point
#print axioms QCore.IndexTraversal.releasing_a_stepped_pointer_never_releases_its_base
#print axioms QCore.IndexTraversal.a_forward_stepped_release_is_accusable_and_never_at_the_start
#print axioms QCore.IndexTraversal.a_pointer_that_may_be_at_the_start_is_never_accused
#print axioms QCore.IndexTraversal.the_walk_header_is_never_accused
#print axioms QCore.IndexTraversal.a_mixed_binding_is_never_accused_or_credited
#print axioms QCore.IndexTraversal.a_step_changes_no_status
#print axioms QCore.IndexTraversal.a_step_keeps_its_targets
#print axioms QCore.IndexTraversal.an_unbound_pointer_stays_unbound
#print axioms QCore.IndexTraversal.losing_the_only_name_of_a_live_object_refuses
#print axioms QCore.IndexTraversal.a_step_that_loses_nothing_adds_no_diagnostic
#print axioms QCore.IndexTraversal.stepping_the_only_name_refuses
#print axioms QCore.IndexTraversal.stepping_a_copy_is_admitted
#print axioms QCore.IndexTraversal.a_one_sided_scalar_fact_joins_to_no_fact
#print axioms QCore.IndexTraversal.the_scalar_join_is_sound
#print axioms QCore.IndexTraversal.no_fact_admits_every_count
#print axioms QCore.IndexTraversal.an_unbound_typed_copy_is_unbound_and_admitted
#print axioms QCore.IndexTraversal.an_untyped_unbound_copy_keeps_its_refusal
#print axioms QCore.IndexTraversal.a_bound_copy_is_unchanged
#print axioms QCore.IndexTraversal.a_typed_condition_changes_no_binding
#print axioms QCore.IndexTraversal.a_typed_condition_keeps_its_obligations
#print axioms QCore.IndexTraversal.an_unsequenced_stepped_name_refuses
#print axioms QCore.IndexTraversal.a_double_step_refuses
#print axioms QCore.IndexTraversal.an_index_write_outside_the_shape_refuses
#print axioms QCore.IndexTraversal.the_access_precedes_its_steps
#print axioms QCore.IndexTraversal.a_full_stepped_candidate_is_admitted
#print axioms QCore.IndexTraversal.a_fixed_formal_keeps_the_refusal
#print axioms QCore.IndexTraversal.an_exposed_formal_keeps_the_refusal
#print axioms QCore.IndexTraversal.a_checked_formal_step_preserves_custody_and_caller_slots
#print axioms QCore.IndexTraversal.a_read_only_unbound_formal_step_is_admitted
#print axioms QCore.IndexTraversal.formal_step_vectors_hold
