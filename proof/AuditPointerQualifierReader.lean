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

import QCore.PointerQualifierReader

/-! Card **W43-LIBC-RESULT-ALIAS**, discharging
`W43-LIBC-POINTER-QUALIFIER-READER`: the declared-ABI reader drops the
qualifiers after a parameter declarator's last `*`, which qualify the parameter
object and are dropped for type compatibility (C11 6.7.6.3p15).

Four directions are audited. **Conservativity**: every parameter the earlier
reader projected is projected identically, and nothing that was read became
unreadable. **The rule as an equation**: any run of trailing qualifiers reads as
its absence. **Exactly one position**: a qualifier between two `*` still
refuses, a declarator with no `*` is unchanged, and the step is idempotent.
**Non-vacuity**: glibc's `const char *__restrict`, measured unreadable, now
projects to `ptr<const, u8>` and keeps its pointee's constness.

What this is not: a model of the token reader. That the word list is the
declarator C declares is trust boundary 28's extraction premise, and result
declarators are deliberately not widened. -/

-- Conservativity.
#print axioms QCore.PointerQualifierReader.the_widening_only_adds
#print axioms QCore.PointerQualifierReader.the_widening_loses_no_reading

-- The rule, as an equation, and where it applies.
#print axioms QCore.PointerQualifierReader.the_widening_reads_the_unqualified_type
#print axioms QCore.PointerQualifierReader.a_declarator_without_a_star_is_unchanged
#print axioms QCore.PointerQualifierReader.drop_is_idempotent
#print axioms QCore.PointerQualifierReader.a_qualifier_between_two_stars_still_refuses

-- Non-vacuity, with the measured glibc witness.
#print axioms QCore.PointerQualifierReader.the_glibc_format_parameter_was_unreadable
#print axioms QCore.PointerQualifierReader.the_glibc_format_parameter_is_now_read
#print axioms QCore.PointerQualifierReader.the_widening_is_strict
#print axioms QCore.PointerQualifierReader.the_pointee_constness_is_kept
#print axioms QCore.PointerQualifierReader.a_const_pointer_to_mutable_bytes_is_mutable
#print axioms QCore.PointerQualifierReader.a_restrict_between_two_stars_refuses

/-! Card **W43-LIBC-RESTRICT-PROTOTYPES**: a qualifier inside an array
parameter's bound (C11 6.7.6.3p7) is read as the adjusted pointer's, and so
dropped (p15); an array of qualified pointers now refuses as its pointer
spelling does. -/

-- The rule, as an equation, and its conservativity.
#print axioms QCore.PointerQualifierReader.the_array_parameter_is_the_adjusted_pointer
#print axioms QCore.PointerQualifierReader.a_bound_read_before_is_read_now
#print axioms QCore.PointerQualifierReader.the_array_widening_only_adds

-- The one narrowing, and exactly where it is.
#print axioms QCore.PointerQualifierReader.an_array_of_qualified_pointers_refuses
#print axioms QCore.PointerQualifierReader.the_array_and_pointer_spellings_agree_on_it
#print axioms QCore.PointerQualifierReader.the_earlier_reader_read_it_as_two_levels

-- What still refuses.
#print axioms QCore.PointerQualifierReader.a_static_bound_still_refuses
#print axioms QCore.PointerQualifierReader.a_star_bound_still_refuses
#print axioms QCore.PointerQualifierReader.a_qualifier_after_the_size_refuses
#print axioms QCore.PointerQualifierReader.a_multi_word_size_refuses

-- Non-vacuity, with the measured glibc witness.
#print axioms QCore.PointerQualifierReader.the_regexec_match_array_was_unreadable
#print axioms QCore.PointerQualifierReader.the_regexec_match_array_is_now_read
#print axioms QCore.PointerQualifierReader.an_array_element_constness_is_kept
