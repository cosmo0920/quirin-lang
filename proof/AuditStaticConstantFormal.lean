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

import QCore.StaticConstantFormal
#print axioms QCore.StaticConstantFormal.fixed_needs_internal_linkage
#print axioms QCore.StaticConstantFormal.an_escape_refuses
#print axioms QCore.StaticConstantFormal.a_block_scope_declaration_refuses
#print axioms QCore.StaticConstantFormal.no_call_refuses
#print axioms QCore.StaticConstantFormal.every_call_agrees
#print axioms QCore.StaticConstantFormal.a_variable_actual_refuses
#print axioms QCore.StaticConstantFormal.mixed_truths_refuse
#print axioms QCore.StaticConstantFormal.selection_is_cs_choice
#print axioms QCore.StaticConstantFormal.selected_arm_is_taken_at_every_call
#print axioms QCore.StaticConstantFormal.selection_only_removes_edges
#print axioms QCore.StaticConstantFormal.no_reading_is_the_identity
#print axioms QCore.StaticConstantFormal.a_zero_actual_keeps_the_zero_arm
#print axioms QCore.StaticConstantFormal.a_read_constant_fits
