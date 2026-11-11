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

import QCore.TypedefProjection

/-! Card **CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF**: the typedef reader that card
LOADER-ABI's ABI comparison resolves through, widened to commit to two shapes it
previously refused — a `struct`/`union`/`enum` **defined by the typedef itself**
and a **function-pointer typedef**.

`QCore.DeclaredAbiAgreement` takes the C side's projection as given. This module
supplies it, and audits five directions.

**Conservativity.** The widening only *adds*: every reading the earlier reader
committed to is the same reading now, pointwise over targets and pointer depths,
and nothing that was read became unreadable. No comparison can therefore have
moved for a shape that was already read.

**Non-vacuity.** Each of the two new shapes has its own witness that it was
unreadable and is now read — `pthread_mutex_t *` and `msgpack_packer_write` —
and neither new target is identified with an existing one.

**The aggregate admission, as an equality rather than as prose.** An untagged
aggregate projects *exactly* as a tagged one does at every pointer depth,
because the pointer projection reads no tag and no member. As a value it still
refuses, exactly as a `struct tag` value does, and the two- and three-level
depth boundaries are the pointer table's own.

**The function-pointer admission, in the refusing direction.** `fnptr` is its
own projection: only that spelling is admitted there, an object pointer and an
integer are both refused (the LP64 eight-byte coincidence buys nothing), and no
object pointer or arithmetic projection admits `fnptr` in return. The half that
makes it sound is that no custody mode is writable on it — the loader's gates
key on the leading word `ptr`, `fnptr` is not one, `Borrowed` is proved to be
the only admissible mode there, and an owned result of one is rejected by the
rule that was already present. The gate is proved still to admit object pointers,
so nothing was weakened to make that true. `ABI-FNPTR-DEPTH1` — a pointer *to* a
function pointer — refuses at every depth and through an alias.

**Fail-closed resolution, and one regression closed by construction.** An absent
name, an exhausted depth budget, an unresolved link and a name recorded twice
with different readings are each `unresolved`. A **tagged** aggregate definition
is recorded as the specifier pair it equals, so a unit that also carries the
plain `typedef struct tag tag;` keeps one reading; recording it as an untagged
aggregate instead is proved to collapse that pair to `unresolved`, which is the
withdrawal this card would otherwise have introduced.

What this is not: a model of a decision, exactly as `QCore.DeclaredAbiAgreement`
is. That the token stream read is the translation unit, that the LP64 data model
holds on the audited targets, and C's own type system are premises supplied from
outside. Nothing here says a declaration is faithful to a C body. -/

-- Conservativity: the widening only adds.
#print axioms QCore.TypedefProjection.the_widening_only_adds
#print axioms QCore.TypedefProjection.the_widening_loses_no_reading

-- Non-vacuity: each new shape, with its measured witness.
#print axioms QCore.TypedefProjection.an_untagged_aggregate_pointer_was_unreadable_and_is_now_read
#print axioms QCore.TypedefProjection.a_function_pointer_was_unreadable_and_is_now_read
#print axioms QCore.TypedefProjection.a_function_pointer_is_not_any_other_target
#print axioms QCore.TypedefProjection.an_untagged_aggregate_is_its_own_target

-- The aggregate admission, as an equality.
#print axioms QCore.TypedefProjection.an_untagged_aggregate_projects_as_a_tagged_one
#print axioms QCore.TypedefProjection.an_aggregate_value_refuses
#print axioms QCore.TypedefProjection.a_named_object_value_refuses
#print axioms QCore.TypedefProjection.an_aggregate_under_two_pointers_is_the_out_slot
#print axioms QCore.TypedefProjection.an_aggregate_under_three_pointers_refuses

-- The function-pointer admission, in both directions.
#print axioms QCore.TypedefProjection.only_a_function_pointer_spelling_is_admitted
#print axioms QCore.TypedefProjection.a_function_pointer_is_not_an_integer
#print axioms QCore.TypedefProjection.a_function_pointer_is_not_an_object_pointer
#print axioms QCore.TypedefProjection.a_pointer_projection_never_admits_a_function_pointer
#print axioms QCore.TypedefProjection.no_object_pointer_admits_a_function_pointer
#print axioms QCore.TypedefProjection.no_untagged_aggregate_admits_a_function_pointer
#print axioms QCore.TypedefProjection.no_arithmetic_projection_admits_a_function_pointer
#print axioms QCore.TypedefProjection.a_pointer_to_a_function_pointer_refuses

-- No custody mode is writable on it, and the gate is not vacuous.
#print axioms QCore.TypedefProjection.a_function_pointer_is_not_a_pointer_spelling
#print axioms QCore.TypedefProjection.borrowed_is_the_only_mode_on_a_function_pointer
#print axioms QCore.TypedefProjection.every_custody_mode_is_refused_on_a_function_pointer
#print axioms QCore.TypedefProjection.an_owned_result_of_a_function_pointer_is_rejected
#print axioms QCore.TypedefProjection.the_custody_gate_still_admits_object_pointers
#print axioms QCore.TypedefProjection.the_result_gate_still_admits_object_pointers

-- Resolution is fail-closed.
#print axioms QCore.TypedefProjection.an_exhausted_depth_budget_is_unresolved
#print axioms QCore.TypedefProjection.an_absent_name_is_unresolved
#print axioms QCore.TypedefProjection.a_chain_to_a_function_pointer_keeps_its_level
#print axioms QCore.TypedefProjection.an_alias_to_a_pointer_to_a_function_pointer_is_unresolved
#print axioms QCore.TypedefProjection.an_alias_folds_an_aggregate_like_an_object
#print axioms QCore.TypedefProjection.an_unresolved_link_is_unresolved

-- Recording a name twice, and the regression closed by construction.
#print axioms QCore.TypedefProjection.a_name_recorded_twice_alike_keeps_its_reading
#print axioms QCore.TypedefProjection.a_name_recorded_twice_differently_is_unresolved
#print axioms QCore.TypedefProjection.a_tagged_definition_composes_with_its_plain_typedef
#print axioms QCore.TypedefProjection.an_untagged_definition_would_not_have_composed
