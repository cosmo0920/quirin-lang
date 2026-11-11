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

import QCore.CallArgumentAccess

/-! Audited Lean companion to card W44-INDIRECT-CALL-ARGUMENT-ACCESS. -/

#print axioms QCore.CallArgumentAccess.a_custody_position_never_admits
#print axioms QCore.CallArgumentAccess.an_uncontracted_callee_never_admits
#print axioms QCore.CallArgumentAccess.an_excess_actual_without_a_tail_never_admits
#print axioms QCore.CallArgumentAccess.a_statement_the_gate_did_not_refuse_keeps_its_lowering
#print axioms QCore.CallArgumentAccess.a_descriptor_read_keeps_the_refusal
#print axioms QCore.CallArgumentAccess.an_uncontracted_call_keeps_the_refusal
#print axioms QCore.CallArgumentAccess.a_declined_probe_keeps_the_refusal
#print axioms QCore.CallArgumentAccess.a_used_actual_is_at_an_admitting_position
#print axioms QCore.CallArgumentAccess.a_custody_position_is_never_used
#print axioms QCore.CallArgumentAccess.the_reads_precede_the_call
#print axioms QCore.CallArgumentAccess.every_used_base_is_obligated
#print axioms QCore.CallArgumentAccess.the_call_sees_the_state_before_the_statement
#print axioms QCore.CallArgumentAccess.the_typed_read_changes_no_binding
#print axioms QCore.CallArgumentAccess.a_descriptor_value_refuses
#print axioms QCore.CallArgumentAccess.a_pointer_read_through_an_owner_of_sub_custody_refuses
#print axioms QCore.CallArgumentAccess.a_pointer_read_of_a_bound_place_refuses
#print axioms QCore.CallArgumentAccess.an_unbound_name_is_never_refused_by_the_second_obligation
#print axioms QCore.CallArgumentAccess.a_released_base_refuses_and_is_accused
#print axioms QCore.CallArgumentAccess.resolved_reads_add_no_diagnostic
#print axioms QCore.CallArgumentAccess.a_refused_read_is_never_safe
#print axioms QCore.CallArgumentAccess.the_brief_example_is_admitted
#print axioms QCore.CallArgumentAccess.the_brief_example_adds_no_diagnostic
#print axioms QCore.CallArgumentAccess.a_releasing_position_keeps_the_refusal
#print axioms QCore.CallArgumentAccess.a_linked_owner_refuses_the_pointer_read
#print axioms QCore.CallArgumentAccess.an_unlinked_live_owner_admits_the_pointer_read
#print axioms QCore.CallArgumentAccess.a_scalar_read_of_a_descriptor_carrier_refuses
