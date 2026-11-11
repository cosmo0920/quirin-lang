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

import QCore.MsvcAbiReader

/-! Exact readings, preserved disagreements and refusing boundaries.
Extraction and implementation correspondence remain stated premises. -/

#print axioms QCore.MsvcAbiReader.the_native_unsigned_size_reading
#print axioms QCore.MsvcAbiReader.explicit_and_default_signed_readings
#print axioms QCore.MsvcAbiReader.accepted_words_are_exact
#print axioms QCore.MsvcAbiReader.a_missing_or_repeated_sized_keyword_refuses
#print axioms QCore.MsvcAbiReader.mixed_arithmetic_words_refuse
#print axioms QCore.MsvcAbiReader.repeated_or_conflicting_signs_refuse
#print axioms QCore.MsvcAbiReader.signedness_and_width_cannot_be_bought
#print axioms QCore.MsvcAbiReader.no_sized_integer_reading_is_a_pointer
#print axioms QCore.MsvcAbiReader.a_scalar_size_carries_no_owned_result
#print axioms QCore.MsvcAbiReader.borrowed_is_the_only_mode_on_a_native_size
#print axioms QCore.MsvcAbiReader.cdecl_preserves_the_entire_declaration
#print axioms QCore.MsvcAbiReader.default_declarations_keep_their_comparison
#print axioms QCore.MsvcAbiReader.cdecl_keeps_all_contract_disagreements
#print axioms QCore.MsvcAbiReader.another_convention_refuses
#print axioms QCore.MsvcAbiReader.repeated_cdecl_refuses
#print axioms QCore.MsvcAbiReader.earlier_type_readings_are_preserved
#print axioms QCore.MsvcAbiReader.an_unambiguous_cdecl_type_preserves_its_reading
#print axioms QCore.MsvcAbiReader.a_target_dependent_long_result_refuses
#print axioms QCore.MsvcAbiReader.a_target_dependent_long_parameter_refuses
#print axioms QCore.MsvcAbiReader.the_native_memcmp_signature_agrees
#print axioms QCore.MsvcAbiReader.the_same_native_declaration_refuses_a_forged_width
#print axioms QCore.MsvcAbiReader.the_same_native_declaration_refuses_a_forged_qualifier
#print axioms QCore.MsvcAbiReader.a_cdecl_pointer_result_cannot_be_hidden_as_unit
