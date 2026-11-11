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

import QCore.IndexedCollection

/-! Audited Lean companion to card W43-INDEXED-RESOURCE-COLLECTIONS. -/

#print axioms QCore.IndexedCollection.step_term_preserves_value
#print axioms QCore.IndexedCollection.the_step_preserves_every_element
#print axioms QCore.IndexedCollection.subst_term_preserves_value
#print axioms QCore.IndexedCollection.the_exit_substitution_preserves_every_element
#print axioms QCore.IndexedCollection.the_fill_form_is_the_filled_prefix
#print axioms QCore.IndexedCollection.one_fill_iteration_keeps_the_invariant
#print axioms QCore.IndexedCollection.the_stepped_fill_is_the_fill_form
#print axioms QCore.IndexedCollection.the_fill_exit_is_the_whole_range
#print axioms QCore.IndexedCollection.one_release_iteration_releases_exactly_element_j
#print axioms QCore.IndexedCollection.the_stepped_sweep_is_the_sweep_form
#print axioms QCore.IndexedCollection.the_sweep_exit_owes_nothing
#print axioms QCore.IndexedCollection.the_cleanup_starts_from_the_failed_fill
#print axioms QCore.IndexedCollection.the_partial_cleanup_discharges_exactly_the_prefix
#print axioms QCore.IndexedCollection.a_body_fact_orders_the_bounds
#print axioms QCore.IndexedCollection.a_known_zero_evaluates_its_bounds
#print axioms QCore.IndexedCollection.a_clamped_zero_bounds_nonpositive_offsets
#print axioms QCore.IndexedCollection.an_exit_at_zero_clamps_the_bound
#print axioms QCore.IndexedCollection.a_zero_bound_admits_no_iteration
#print axioms QCore.IndexedCollection.a_second_release_is_a_double_release
#print axioms QCore.IndexedCollection.only_a_live_element_is_released
#print axioms QCore.IndexedCollection.an_unfilled_element_is_never_released
#print axioms QCore.IndexedCollection.reading_a_released_element_is_a_use_after_release
#print axioms QCore.IndexedCollection.an_unfilled_element_is_never_read
#print axioms QCore.IndexedCollection.overwriting_a_live_element_is_a_leak
#print axioms QCore.IndexedCollection.a_possibly_null_live_element_refuses_a_store
#print axioms QCore.IndexedCollection.freeing_the_array_with_live_elements_leaks_them
#print axioms QCore.IndexedCollection.an_escaped_array_carries_its_elements
#print axioms QCore.IndexedCollection.an_array_owing_nothing_is_discharged
#print axioms QCore.IndexedCollection.unknown_ranges_claim_nothing
#print axioms QCore.IndexedCollection.a_never_filled_array_owes_nothing
#print axioms QCore.IndexedCollection.the_joined_form_describes_both_sides
#print axioms QCore.IndexedCollection.the_fill_entry_collapses_to_the_fill_form
#print axioms QCore.IndexedCollection.the_sweep_entry_collapses_to_the_sweep_form
#print axioms QCore.IndexedCollection.an_admitted_statement_keeps_its_lowering
#print axioms QCore.IndexedCollection.a_refusal_without_an_element_shape_is_kept
#print axioms QCore.IndexedCollection.an_unresolved_array_is_exactly_the_fallback
#print axioms QCore.IndexedCollection.a_refusal_is_never_safe
#print axioms QCore.IndexedCollection.a_candidate_is_unsafe
#print axioms QCore.IndexedCollection.a_borrow_of_an_owing_collection_refuses
#print axioms QCore.IndexedCollection.a_reallocation_of_an_owing_collection_refuses
#print axioms QCore.IndexedCollection.releasing_or_consuming_the_array_is_admitted
#print axioms QCore.IndexedCollection.three_filled_elements_are_live
#print axioms QCore.IndexedCollection.eval_const
#print axioms QCore.IndexedCollection.eval_var
#print axioms QCore.IndexedCollection.only_heap_elements_are_collected
#print axioms QCore.IndexedCollection.a_descriptor_read_keeps_the_fallback
#print axioms QCore.IndexedCollection.a_non_heap_element_keeps_the_fallback
