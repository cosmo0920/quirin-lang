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

import QCore.AbiEnumConstPointer

/-! Card **W43-ABI-ENUM-AND-CONST-PP** (gateway rules 12u, 12v): an
enumeration's integer projection is Clang's, exactly; a pointer to a pointer
to const has its own spelling, and a qualified slot still refuses.

Audited directions. **Exactness**: a projection exists only for one
compatible standard type of the probed signedness, and its width and
signedness are the probed ones. **Never assumed**: no `isize`/`usize`, no
`i32` for an unsigned enumeration, and no projection without Clang's facts.
**Pointers**: the widening only adds, the new spelling is exactly the
innermost-const two-level pointer, a qualified slot refuses, and
`resource_out` never admits it. **Controls**: the measured shapes.

What this is not: a model of the token reader or of Clang (trust boundary
28's extraction premise). -/

-- Enumerations: exactness and refusal.
#print axioms QCore.AbiEnumConstPointer.ofWidth_exact
#print axioms QCore.AbiEnumConstPointer.ofWidth_never_pointer_width
#print axioms QCore.AbiEnumConstPointer.a_projection_is_exactly_the_probed_width_and_signedness
#print axioms QCore.AbiEnumConstPointer.an_enumeration_never_reads_as_a_pointer_width
#print axioms QCore.AbiEnumConstPointer.an_unsigned_enumeration_is_never_int
#print axioms QCore.AbiEnumConstPointer.no_unique_compatible_type_refuses
#print axioms QCore.AbiEnumConstPointer.a_signedness_disagreement_refuses
#print axioms QCore.AbiEnumConstPointer.without_clang_the_reader_refuses
#print axioms QCore.AbiEnumConstPointer.enumeration_controls
#print axioms QCore.AbiEnumConstPointer.two_compatible_types_refuse

-- Pointers to pointers to const.
#print axioms QCore.AbiEnumConstPointer.the_pointer_widening_only_adds
#print axioms QCore.AbiEnumConstPointer.the_new_spelling_is_exactly_two_levels_with_a_const_innermost_pointee
#print axioms QCore.AbiEnumConstPointer.a_qualified_slot_refuses
#print axioms QCore.AbiEnumConstPointer.pointer_to_pointer_and_pointer_to_pointer_to_const_are_distinct
#print axioms QCore.AbiEnumConstPointer.resource_out_never_admits_a_pointer_to_a_pointer_to_const
