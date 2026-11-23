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

import QCore.CallbackRequirementFold

#print axioms QCore.CallbackRequirementFold.the_old_fold_dropped_a_non_null_requirement
#print axioms QCore.CallbackRequirementFold.the_fold_never_drops_a_non_null_requirement
#print axioms QCore.CallbackRequirementFold.the_old_fold_differed_by_form
#print axioms QCore.CallbackRequirementFold.the_fold_agrees_with_the_slot_fold
#print axioms QCore.CallbackRequirementFold.the_slot_fold_is_one_requirement
#print axioms QCore.CallbackRequirementFold.the_fold_agrees_where_the_old_one_folded
#print axioms QCore.CallbackRequirementFold.the_fold_only_strengthens
#print axioms QCore.CallbackRequirementFold.a_non_null_position_is_a_fixed_point
#print axioms QCore.CallbackRequirementFold.a_nullable_requirement_changes_no_held_position
#print axioms QCore.CallbackRequirementFold.out_and_removed_refuse
#print axioms QCore.CallbackRequirementFold.fold_vectors_hold
