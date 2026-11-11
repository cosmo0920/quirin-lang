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

import QCore.ResidualAccessReadings

/-! Audited companion to CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (gateway rules 14a-14d). -/

#print axioms QCore.ResidualAccessReadings.a_borrowed_call_changes_no_custody
#print axioms QCore.ResidualAccessReadings.a_read_is_decided_alike_on_either_side_of_the_call
#print axioms QCore.ResidualAccessReadings.a_releasing_call_does_not_commute
#print axioms QCore.ResidualAccessReadings.scalar_call_vectors_hold
#print axioms QCore.ResidualAccessReadings.two_calls_are_never_read
#print axioms QCore.ResidualAccessReadings.an_admitted_call_takes_no_custody
#print axioms QCore.ResidualAccessReadings.interior_vectors_hold
#print axioms QCore.ResidualAccessReadings.a_bound_root_is_never_read
#print axioms QCore.ResidualAccessReadings.the_interior_alias_is_the_pointer_reads
#print axioms QCore.ResidualAccessReadings.a_release_through_the_alias_discharges_nothing
#print axioms QCore.ResidualAccessReadings.the_alias_is_never_known_non_null
#print axioms QCore.ResidualAccessReadings.address_vectors_hold
#print axioms QCore.ResidualAccessReadings.a_member_keeps_rule_10n
#print axioms QCore.ResidualAccessReadings.an_element_is_read_only_when_scalar
#print axioms QCore.ResidualAccessReadings.nothing_else_is_read
#print axioms QCore.ResidualAccessReadings.offset_vectors_hold
#print axioms QCore.ResidualAccessReadings.a_bound_name_is_never_read
#print axioms QCore.ResidualAccessReadings.a_statement_reads_only_a_path_operand
