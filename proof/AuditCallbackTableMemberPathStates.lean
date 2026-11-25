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

import QCore.CallbackTableMemberPathStates

#print axioms QCore.CallbackTableMemberPathStates.run_one_is_count
#print axioms QCore.CallbackTableMemberPathStates.join_decided
#print axioms QCore.CallbackTableMemberPathStates.foldl_decided
#print axioms QCore.CallbackTableMemberPathStates.a_decided_join_is_every_candidate
#print axioms QCore.CallbackTableMemberPathStates.the_join_is_at_least_each_candidate
#print axioms QCore.CallbackTableMemberPathStates.gone_members
#print axioms QCore.CallbackTableMemberPathStates.stays_members
#print axioms QCore.CallbackTableMemberPathStates.foldl_cleared
#print axioms QCore.CallbackTableMemberPathStates.foldl_taking
#print axioms QCore.CallbackTableMemberPathStates.foldl_kept
#print axioms QCore.CallbackTableMemberPathStates.a_gone_table_leaves_exactly_once
#print axioms QCore.CallbackTableMemberPathStates.a_stays_table_never_leaves
#print axioms QCore.CallbackTableMemberPathStates.disagreeing_candidates_refuse
#print axioms QCore.CallbackTableMemberPathStates.the_gone_reading_assumes_an_invocation
#print axioms QCore.CallbackTableMemberPathStates.a_table_composition_after_a_release_is_the_c_order
#print axioms QCore.CallbackTableMemberPathStates.table_vectors_hold
