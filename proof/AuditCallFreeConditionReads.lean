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

import QCore.CallFreeConditionReads
#print axioms QCore.CallFreeConditionReads.vectors_match
#print axioms QCore.CallFreeConditionReads.old_admissions_survive
#print axioms QCore.CallFreeConditionReads.a_checked_read_keeps_the_complete_plan
#print axioms QCore.CallFreeConditionReads.a_plan_without_a_reason_refuses
#print axioms QCore.CallFreeConditionReads.the_graph_preserves_checked_read_order
#print axioms QCore.CallFreeConditionReads.the_graph_preserves_state_and_all_read_errors
#print axioms QCore.CallFreeConditionReads.an_evaluated_read_error_is_not_erased
#print axioms QCore.CallFreeConditionReads.short_circuit_reads
