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

import QCore.GnuBuiltinReader

/-! Card **W44-LIBC-RESIDUALS**: the compiler's `__builtin_va_list`, which
`__gnuc_va_list` names, projects to the loader's parameter-only `va_list`
spelling, so glibc's `vsnprintf` and `vprintf` agree.

Five directions are audited. **Conservativity**: the widening only adds, and
a chain that does not reach the builtin reads exactly as before. **Exactness**: the builtin admits `va_list` and
nothing else, no earlier reading admits `va_list`, and a result or a pointer
still refuses. **Shadowing**: a unit that `typedef`s the builtin name is read
through its own typedef. **Custody**: `va_list` is not a `ptr` spelling, so
`Borrowed` is the only mode on it, and it is not a result spelling.
**Non-vacuity**: the measured `vsnprintf` signature, unreadable before, is
read now.

What this is not: a model of the token reader. That the words are the ones the
token stream carries is trust boundary 28's extraction premise. -/

-- `__builtin_va_list`: conservativity, exactness and where it still refuses.
#print axioms QCore.GnuBuiltinReader.the_va_list_widening_only_adds
#print axioms QCore.GnuBuiltinReader.the_va_list_widening_is_strict
#print axioms QCore.GnuBuiltinReader.only_va_list_is_admitted_for_the_builtin
#print axioms QCore.GnuBuiltinReader.the_builtin_is_not_its_x86_64_representation
#print axioms QCore.GnuBuiltinReader.a_va_list_result_refuses
#print axioms QCore.GnuBuiltinReader.a_pointer_to_a_va_list_refuses
#print axioms QCore.GnuBuiltinReader.a_pointer_projection_never_admits_va_list
#print axioms QCore.GnuBuiltinReader.no_earlier_reading_admits_va_list

-- Resolution and shadowing.
#print axioms QCore.GnuBuiltinReader.a_chain_to_the_builtin_was_unresolved
#print axioms QCore.GnuBuiltinReader.resolution_is_conservative
#print axioms QCore.GnuBuiltinReader.a_unit_typedef_of_the_builtin_name_is_followed
#print axioms QCore.GnuBuiltinReader.the_glibc_chain_reaches_the_builtin
#print axioms QCore.GnuBuiltinReader.a_typedef_pointer_to_the_builtin_stays_unresolved

-- Custody and the loader's vocabularies.
#print axioms QCore.GnuBuiltinReader.a_va_list_is_not_a_pointer_spelling
#print axioms QCore.GnuBuiltinReader.borrowed_is_the_only_mode_on_a_va_list
#print axioms QCore.GnuBuiltinReader.an_owned_result_of_a_va_list_is_rejected
#print axioms QCore.GnuBuiltinReader.va_list_is_a_parameter_spelling_only
#print axioms QCore.GnuBuiltinReader.the_result_vocabulary_lost_only_va_list

-- The comparison, with the measured `vsnprintf`.
#print axioms QCore.GnuBuiltinReader.vsnprintf_agrees_now_and_was_withdrawn_as_unreadable
#print axioms QCore.GnuBuiltinReader.a_pointer_spelling_of_the_va_list_disagrees
