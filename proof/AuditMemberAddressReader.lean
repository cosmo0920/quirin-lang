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

import QCore.MemberAddressReader

/-! Card **ABI-TYPEDEF-ADDRESS-OF-MEMBER**: the declaration reader that
classifies the member whose address a callee is handed, widened to commit to one
shape it previously refused — a `struct`/`union`/`enum` **defined by its own
typedef**, and the `union`-typed member the same classifier had refused for the
same reason.

`QCore.MemberAddress` (card O04-B09B10) models the *decision* that admits
`& BASE - > FIELD` at a declared `Borrowed` position and takes the member's
classification as given. This module supplies that classification, and audits
five directions.

**Conservativity.** The widening only *adds*: every member the earlier reader
classified `nonPointer` is `nonPointer` now, pointwise over every member shape,
and no member moved into the `pointer` class. Since `admitsAddress` reads the
classification and nothing else about the member, no address refused for another
reason can have moved.

**Non-vacuity.** Each new shape has its own witness that it was refused and is
now addressable — the measured production `pthread_mutex_t` member, the same
reading behind an intervening plain typedef, and a `union`-typed member.

**A pointer is never addressable, by any spelling.** A declarator star, a
pointer typedef, a function-pointer typedef and an array typedef are each
refused, as are an array declarator, a `volatile`/`_Atomic` member, a `void`
member and an unresolved name. This is the laundering shape the whole rule
exists to refuse: the address of a pointer member is a handle with which a
callee could overwrite the very link this projection maintains. The refusal is
proved to hold under **both** readers, so the widening cannot manufacture it.

**Disjointness, as a theorem rather than as prose.** An addressable member is
never a *linkable* one and a linkable member is never addressable: the two sets
are disjoint by construction, because a link is recorded only on a member with
exactly one declarator star and an addressable member has none.

**Containment.** The new reading is invisible to `classify`, so every other
premise keyed on it — the scalar no-op rule, the borrowable, single-pointer,
`void *` and record-pointer member sets, the bounded chain walker and the
declared types of a function's own locals — decides exactly what it decided
before this card.

**Fail-closed reading.** The shape recogniser reads
`struct|union|enum [TAG] { … } NAME` and nothing else: a `*`, a second
declarator and an array declarator each keep the absent reading they had. A name
recorded twice with disagreeing readings retires to the absent reading — the
case of a unit carrying both `typedef struct tag { … } tag;` and
`typedef struct tag tag;` is proved to retire rather than to be admitted
silently — and an unresolved name is never addressable.

What this is not: a model of C semantics. That the reader extracts these
declaration facts faithfully from the preprocessed token stream, that C's object
model keeps distinct members of a `struct` disjoint, and that a declared
`Borrowed` mode really borrows are premises supplied from outside — the same
premises `QCore.MemberAddress` already names. Nothing here says a declaration is
faithful to a C body. -/

-- Containment: the new reading is invisible to the classifier.
#print axioms QCore.MemberAddressReader.the_classifier_is_unchanged
#print axioms QCore.MemberAddressReader.an_aggregate_definition_is_still_opaque_to_the_classifier
#print axioms QCore.MemberAddressReader.an_aggregate_definition_behind_an_alias_is_still_opaque

-- Conservativity: the widening only adds.
#print axioms QCore.MemberAddressReader.a_classified_specifier_is_a_non_pointer_object
#print axioms QCore.MemberAddressReader.the_premise_only_adds
#print axioms QCore.MemberAddressReader.an_addressable_member_has_no_declarator_star
#print axioms QCore.MemberAddressReader.the_widening_only_adds
#print axioms QCore.MemberAddressReader.the_widening_moves_no_member_into_the_pointer_class

-- Non-vacuity, one witness per new shape.
#print axioms QCore.MemberAddressReader.an_aggregate_typedef_member_was_refused_and_is_now_addressable
#print axioms QCore.MemberAddressReader.an_aliased_aggregate_typedef_member_is_addressable
#print axioms QCore.MemberAddressReader.a_union_typed_member_was_refused_and_is_now_addressable

-- A pointer is never addressable, by any spelling.
#print axioms QCore.MemberAddressReader.a_declared_pointer_member_is_never_addressable
#print axioms QCore.MemberAddressReader.a_pointer_typedef_member_is_never_addressable
#print axioms QCore.MemberAddressReader.a_function_pointer_typedef_member_is_never_addressable
#print axioms QCore.MemberAddressReader.an_array_typedef_member_is_never_addressable
#print axioms QCore.MemberAddressReader.a_pointer_typedef_behind_an_alias_is_never_addressable
#print axioms QCore.MemberAddressReader.an_array_member_is_never_addressable
#print axioms QCore.MemberAddressReader.a_qualified_member_is_never_addressable
#print axioms QCore.MemberAddressReader.an_unresolved_name_is_never_addressable
#print axioms QCore.MemberAddressReader.a_void_member_is_never_addressable

-- Disjointness: an addressable member is never a linkable one.
#print axioms QCore.MemberAddressReader.an_addressable_member_is_never_linkable
#print axioms QCore.MemberAddressReader.a_linkable_member_is_never_addressable

-- The admission relation inherits every refusal unchanged.
#print axioms QCore.MemberAddressReader.no_custody_position_admits_a_read_member
#print axioms QCore.MemberAddressReader.a_refused_base_never_admits_a_read_member
#print axioms QCore.MemberAddressReader.an_incomplete_record_never_admits_a_read_member
#print axioms QCore.MemberAddressReader.a_pointer_member_refuses_under_both_readers

-- The shape recogniser is exact.
#print axioms QCore.MemberAddressReader.only_a_bare_name_after_the_body_is_read
#print axioms QCore.MemberAddressReader.a_pointer_declarator_after_the_body_is_not_read
#print axioms QCore.MemberAddressReader.a_second_declarator_after_the_body_is_not_read
#print axioms QCore.MemberAddressReader.an_array_declarator_after_the_body_is_not_read
#print axioms QCore.MemberAddressReader.the_aggregate_definition_is_read
#print axioms QCore.MemberAddressReader.a_plain_typedef_keeps_its_reading

-- Recording a name twice is fail-closed.
#print axioms QCore.MemberAddressReader.a_name_recorded_twice_alike_keeps_its_reading
#print axioms QCore.MemberAddressReader.a_name_recorded_twice_differently_is_unresolved
#print axioms QCore.MemberAddressReader.an_aggregate_definition_and_a_plain_typedef_of_one_name_retire_it
#print axioms QCore.MemberAddressReader.a_retired_name_is_never_addressable
