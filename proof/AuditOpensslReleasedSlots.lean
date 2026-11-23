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

import QCore.OpensslReleasedSlots

#print axioms QCore.OpensslReleasedSlots.the_admission_only_grows
#print axioms QCore.OpensslReleasedSlots.the_admission_grows_only_by_released_slot_invocations_alone
#print axioms QCore.OpensslReleasedSlots.another_object_mode_still_rejects
#print axioms QCore.OpensslReleasedSlots.a_released_slot_beside_another_clause_rejects
#print axioms QCore.OpensslReleasedSlots.the_reading_is_the_object_before_the_release
#print axioms QCore.OpensslReleasedSlots.the_route_after_the_release_reads_a_released_object
#print axioms QCore.OpensslReleasedSlots.a_member_effect_beside_a_release_refuses
#print axioms QCore.OpensslReleasedSlots.a_use_after_the_release_refuses
#print axioms QCore.OpensslReleasedSlots.a_non_null_requirement_strengthens_the_release
#print axioms QCore.OpensslReleasedSlots.applied_only_over_closed_identity_slots
#print axioms QCore.OpensslReleasedSlots.nothing_reads_the_object_after_the_release
#print axioms QCore.OpensslReleasedSlots.the_premises_cover_exactly_what_remains
#print axioms QCore.OpensslReleasedSlots.vectors_as_landed
#print axioms QCore.OpensslReleasedSlots.vectors_drifted_refuse
#print axioms QCore.OpensslReleasedSlots.the_decision_holds
#print axioms QCore.OpensslReleasedSlots.vectors_reading
