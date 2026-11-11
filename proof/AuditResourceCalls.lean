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

import QCore.ResourceCalls

#print axioms QCore.ResourceCalls.read_loan_permits_read
#print axioms QCore.ResourceCalls.read_loan_refuses_write
#print axioms QCore.ResourceCalls.write_loan_permits_write
#print axioms QCore.ResourceCalls.scalar_never_permits_resource_access
#print axioms QCore.ResourceCalls.duplicate_reads_are_compatible
#print axioms QCore.ResourceCalls.owned_and_read_alias_refuses
#print axioms QCore.ResourceCalls.write_and_read_alias_refuses
#print axioms QCore.ResourceCalls.compatible_repeated_aliases_are_reads
#print axioms QCore.ResourceCalls.one_move_consumes_the_slot
#print axioms QCore.ResourceCalls.a_second_move_refuses
#print axioms QCore.ResourceCalls.drop_after_move_refuses
#print axioms QCore.ResourceCalls.read_borrow_cannot_escape_as_owned
#print axioms QCore.ResourceCalls.write_borrow_cannot_escape_as_owned
#print axioms QCore.ResourceCalls.admitted_extent_discharges_the_minimum
#print axioms QCore.ResourceCalls.too_short_actual_refuses
#print axioms QCore.ResourceCalls.propagated_extent_discharges_callee_access
#print axioms QCore.ResourceCalls.self_call_refuses
#print axioms QCore.ResourceCalls.mutual_call_refuses
#print axioms QCore.ResourceCalls.callee_mutation_is_transitive
#print axioms QCore.ResourceCalls.explicit_empty_refuses_indirect_mutation
#print axioms QCore.ResourceCalls.zero_owner_failure_exit_is_clean
#print axioms QCore.ResourceCalls.a_live_owner_refuses_allocation_failure
