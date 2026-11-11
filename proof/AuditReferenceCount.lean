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

import QCore.ReferenceCount

/-! Audited companion to REFCOUNT-RESOURCE-KIND: the reference-count resource kind (gateway rules 13b, 13c). -/

#print axioms QCore.ReferenceCount.disagreeing_counts_are_undecided
#print axioms QCore.ReferenceCount.a_mint_needs_an_unbound_name
#print axioms QCore.ReferenceCount.a_final_drop_is_the_last_reference
#print axioms QCore.ReferenceCount.an_unheld_drop_is_refused
#print axioms QCore.ReferenceCount.an_unmodeled_exit_never_leaks
#print axioms QCore.ReferenceCount.a_balanced_exit_owes_nothing
#print axioms QCore.ReferenceCount.a_safe_path_is_balanced
#print axioms QCore.ReferenceCount.a_missing_drop_is_never_safe
#print axioms QCore.ReferenceCount.a_kept_reference_never_dropped_leaks
#print axioms QCore.ReferenceCount.setName_refused
#print axioms QCore.ReferenceCount.step_keeps_refused
#print axioms QCore.ReferenceCount.foldl_keeps_refused
#print axioms QCore.ReferenceCount.a_refused_path_is_never_safe
#print axioms QCore.ReferenceCount.a_violation_is_never_safe
#print axioms QCore.ReferenceCount.a_statistics_counter_is_never_read_as_a_reference
#print axioms QCore.ReferenceCount.a_counter_changes_no_family
#print axioms QCore.ReferenceCount.the_neutral_reading_is_not_admitted
#print axioms QCore.ReferenceCount.nameOf_map_invalidate
#print axioms QCore.ReferenceCount.an_alias_use_after_a_possibly_final_drop_is_refused
#print axioms QCore.ReferenceCount.a_use_after_the_last_drop_is_refused_never_accused
#print axioms QCore.ReferenceCount.an_over_drop_of_an_acquired_family_is_a_violation
#print axioms QCore.ReferenceCount.an_over_drop_on_a_refused_path_is_not_accused
#print axioms QCore.ReferenceCount.a_drop_past_a_borrowed_familys_last_drop_is_refused
#print axioms QCore.ReferenceCount.reference_vectors_hold
#print axioms QCore.ReferenceCount.reference_trace_vectors_hold
