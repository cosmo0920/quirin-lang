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

import QCore.TypedConstPointerValue

/-! Audited abstract companion to W47-TYPED-CONST-POINTER-VALUE (gateway rule 10m):
qualifier erasure of a `T *const` value preserves P2's statement lowering and P3's
return and position decisions; unlisted qualifiers refuse; the `void *` bit-cast arm
lowers to the inner path's own borrowed read, and every other cast refuses. -/

#print axioms QCore.TypedConstPointerValue.the_rule_only_adds
#print axioms QCore.TypedConstPointerValue.an_unqualified_value_is_decided_as_before
#print axioms QCore.TypedConstPointerValue.a_non_pointer_value_refuses
#print axioms QCore.TypedConstPointerValue.an_admitted_value_erases_to_an_old_admission
#print axioms QCore.TypedConstPointerValue.erasure_keeps_the_decision
#print axioms QCore.TypedConstPointerValue.an_unlisted_qualifier_refuses
#print axioms QCore.TypedConstPointerValue.a_volatile_value_refuses
#print axioms QCore.TypedConstPointerValue.a_restrict_value_refuses
#print axioms QCore.TypedConstPointerValue.an_atomic_value_refuses
#print axioms QCore.TypedConstPointerValue.a_repeated_const_refuses
#print axioms QCore.TypedConstPointerValue.erasure_preserves_the_shape
#print axioms QCore.TypedConstPointerValue.erasure_preserves_the_statement_lowering
#print axioms QCore.TypedConstPointerValue.erasure_preserves_the_borrowed_read
#print axioms QCore.TypedConstPointerValue.erasure_preserves_the_return
#print axioms QCore.TypedConstPointerValue.erasure_preserves_the_position_decision
#print axioms QCore.TypedConstPointerValue.a_const_pointer_return_hands_nothing_back
#print axioms QCore.TypedConstPointerValue.a_const_pointer_return_from_a_tracked_root_refuses
#print axioms QCore.TypedConstPointerValue.a_const_pointer_read_binds_no_custody
#print axioms QCore.TypedConstPointerValue.an_unlisted_qualifier_refuses_in_every_position
#print axioms QCore.TypedConstPointerValue.an_unlisted_qualifier_has_no_lowering
#print axioms QCore.TypedConstPointerValue.an_uncast_value_is_decided_as_before
#print axioms QCore.TypedConstPointerValue.the_bit_cast_arm_preserves_every_obligation
#print axioms QCore.TypedConstPointerValue.a_bit_cast_read_binds_no_custody
#print axioms QCore.TypedConstPointerValue.an_explicit_cast_refuses
#print axioms QCore.TypedConstPointerValue.any_other_implicit_conversion_refuses
#print axioms QCore.TypedConstPointerValue.a_cast_to_a_function_pointer_refuses
#print axioms QCore.TypedConstPointerValue.a_cast_to_a_void_pointer_refuses
#print axioms QCore.TypedConstPointerValue.a_cast_from_an_object_pointer_refuses
#print axioms QCore.TypedConstPointerValue.a_non_pointer_destination_refuses
#print axioms QCore.TypedConstPointerValue.a_cast_of_anything_but_a_typed_path_refuses
#print axioms QCore.TypedConstPointerValue.a_refused_conversion_has_no_lowering
#print axioms QCore.TypedConstPointerValue.a_cast_in_the_middle_of_a_path_refuses
#print axioms QCore.TypedConstPointerValue.a_const_pointer_value_is_admitted
#print axioms QCore.TypedConstPointerValue.the_old_reader_refused_it
#print axioms QCore.TypedConstPointerValue.a_const_volatile_value_is_refused
#print axioms QCore.TypedConstPointerValue.the_void_bit_cast_is_admitted
#print axioms QCore.TypedConstPointerValue.the_same_value_under_an_explicit_cast_is_refused
