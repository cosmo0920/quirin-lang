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

import QCore.RemovedSelection

#print axioms QCore.RemovedSelection.negative_selects_nothing
#print axioms QCore.RemovedSelection.nonnegative_constant_is_the_lookup
#print axioms QCore.RemovedSelection.selected_is_a_member
#print axioms QCore.RemovedSelection.the_result_is_the_selection
#print axioms QCore.RemovedSelection.no_selection_moves_nothing
#print axioms QCore.RemovedSelection.the_selected_child_leaves_parent_custody
#print axioms QCore.RemovedSelection.every_other_child_is_preserved
#print axioms QCore.RemovedSelection.detach_does_not_allocate_or_release
#print axioms QCore.RemovedSelection.detach_inventory_is_preserved
#print axioms QCore.RemovedSelection.delete_releases_exactly_the_selection
#print axioms QCore.RemovedSelection.delete_without_selection_changes_nothing
#print axioms QCore.RemovedSelection.delete_leaves_no_obligation_for_the_child
#print axioms QCore.RemovedSelection.delete_preserves_every_other_object
#print axioms QCore.RemovedSelection.materialization_needs_every_fact
#print axioms QCore.RemovedSelection.materialize_vectors_hold
#print axioms QCore.RemovedSelection.accepted_detach_exit_returns_the_selection_or_nothing
#print axioms QCore.RemovedSelection.accepted_release_exit_keeps_no_selection
#print axioms QCore.RemovedSelection.exit_vectors_hold
#print axioms QCore.RemovedSelection.selection_vectors_hold
