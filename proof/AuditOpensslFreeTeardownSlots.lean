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

import QCore.OpensslFreeTeardownSlots

#print axioms QCore.OpensslFreeTeardownSlots.eval_builtin
#print axioms QCore.OpensslFreeTeardownSlots.eval_loaded
#print axioms QCore.OpensslFreeTeardownSlots.eval_free
#print axioms QCore.OpensslFreeTeardownSlots.eval_released
#print axioms QCore.OpensslFreeTeardownSlots.teardown_premises_cover_the_free_statement
#print axioms QCore.OpensslFreeTeardownSlots.an_uncovered_function_needs_the_free_statement
#print axioms QCore.OpensslFreeTeardownSlots.released_statement_entails_free_statement
#print axioms QCore.OpensslFreeTeardownSlots.free_statement_does_not_entail_released
#print axioms QCore.OpensslFreeTeardownSlots.the_teardown_formulas_are_well_formed
#print axioms QCore.OpensslFreeTeardownSlots.a_left_item_keeps_the_premise
#print axioms QCore.OpensslFreeTeardownSlots.nothing_left_drops_the_premise
#print axioms QCore.OpensslFreeTeardownSlots.a_running_free_names_the_teardown_premises
#print axioms QCore.OpensslFreeTeardownSlots.the_stated_slots_are_the_audited
#print axioms QCore.OpensslFreeTeardownSlots.a_released_free_states_no_slot
#print axioms QCore.OpensslFreeTeardownSlots.an_unaudited_release_refuses
#print axioms QCore.OpensslFreeTeardownSlots.a_source_entry_admits_only_the_release_read
#print axioms QCore.OpensslFreeTeardownSlots.a_documented_entry_is_cited_nowhere
#print axioms QCore.OpensslFreeTeardownSlots.vectors_as_landed
#print axioms QCore.OpensslFreeTeardownSlots.vectors_drifted_refuse
#print axioms QCore.OpensslFreeTeardownSlots.vectors_inner_frees
#print axioms QCore.OpensslFreeTeardownSlots.the_handshake_statement_never_beside_the_free_statement
#print axioms QCore.OpensslFreeTeardownSlots.vector_source_entries
