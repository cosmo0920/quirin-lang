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

import QCore.MemberAddress

/-!
# The declaration reader that classifies an addressed member

Card **ABI-TYPEDEF-ADDRESS-OF-MEMBER**. `QCore.MemberAddress` (card O04-B09B10)
models the *decision* that admits `& BASE - > FIELD` at a declared `Borrowed`
position, and takes the member's classification — `nonPointer`, `pointer`,
`refusedShape` — as given. This module models the **reader** that produces that
classification from the analysed translation unit's own declarations, and the
one shape this card taught it to commit to.

## What was refused, and why it is a boundary rather than a disagreement

Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF measured production
`flb_input_chunk_create` down to three statements and named the binding refusal
`ABI-TYPEDEF-ADDRESS-OF-MEMBER`. Two of them are

```c
pthread_mutex_lock(&in->metrics_chunk_lock);   /* pthread_mutex_t  */
msgpack_packer_init(&ic->mp_pck, ic, cb);      /* msgpack_packer   */
```

and both members are declared over a typedef that **defines** its own aggregate
— `typedef union { … } pthread_mutex_t;`, `typedef struct msgpack_packer { … }
msgpack_packer;`. The reader abandoned such a typedef, so the member classified
`refusedShape` and the address was refused before any contract was consulted.
The C declaration is present and faithful; the reader simply would not read it.

## What is proved

* **The widening only adds.** Every member the earlier reader classified
  `nonPointer` is classified `nonPointer` now, and nothing that was
  `nonPointer` stopped being so — pointwise over every member shape. Since
  `QCore.MemberAddress.admitsAddress` reads the classification and nothing else
  about the member, no address that was refused for another reason can have
  moved.
* **The widening is strict**, with a witness on each new shape, so no branch is
  vacuous.
* **A pointer is never addressable, by any spelling.** A declarator star, a
  pointer typedef, a function-pointer typedef and an array typedef are each
  refused. This is the laundering shape the whole rule exists to refuse: the
  address of a pointer member is a handle with which a callee could overwrite a
  link this projection maintains.
* **An addressable member is never a linkable one.** The two member sets are
  disjoint by construction, which is the disjointness premise
  `QCore.MemberAddress` argues in prose, stated here as a theorem.
* **The new reading is invisible to the classifier.** The aggregate a typedef
  defines is recorded in a place `classify` does not read, so every other
  premise keyed on `classify` — the scalar no-op rule, the borrowable,
  single-pointer, `void *` and record-pointer member sets, the chain walker and
  the declared types of locals — decides exactly what it decided before this
  card. That is the containment statement, proved rather than argued.
* **The shape recogniser is exact.** Only `struct|union|enum [TAG] { … } NAME`
  is read; a `*`, a second declarator and an array declarator each keep the
  absent reading they had.
* **Resolution is fail-closed**: an absent name, an exhausted alias budget and a
  name recorded twice with disagreeing readings are each unresolved, and
  unresolved is never addressable.

## What this is not

A model of C semantics. That the reader extracts these declaration facts
faithfully from the preprocessed token stream, that C's object model keeps
distinct members of a `struct` disjoint, and that a declared `Borrowed` mode
really borrows are premises supplied from outside — the same premises
`QCore.MemberAddress` already names. Nothing here is a claim about arbitrary C
memory safety, and nothing here says a declaration is faithful to a C body.
-/

namespace QCore.MemberAddressReader

open QCore.BoundFieldRead
open QCore.MemberAddress

/-- The classification `typed_field_access::classify_specifiers` produces, as
    far as the premises that read it separate them. `opaque` is the answer for
    everything the reader will not commit to. -/
inductive DeclaredKind where
  | scalar
  | record
  | opaque
  deriving DecidableEq, Repr

/-- The specifier list of one member declaration, as far as this reader
    separates them.

The typedef cases record what the *unit's* `typedef` table holds for that name:
`aggregateTypedef` is this card's new reading, `aliasTypedef` the reading the
table already held (a typedef with no declarator at all), and the remaining
three are the shapes the table records unresolved. -/
inductive Spec where
  /-- An arithmetic specifier list — `int`, `unsigned long`, `_Bool`. -/
  | arith
  /-- `struct TAG`. -/
  | structTag
  /-- `union TAG`. -/
  | unionTag
  /-- `enum TAG`, tagged or not. -/
  | enumTag
  /-- The literal one-token `void`, which is not an object type. -/
  | voidSpec
  /-- `typedef union { … } t;` / `typedef struct tag { … } t;` — the aggregate
      the typedef itself defines. **This card's new reading.** -/
  | aggregateTypedef
  /-- `typedef T t;` with no declarator at all: the table holds `T`. -/
  | aliasTypedef (target : Spec)
  /-- `typedef struct s *p;` — the declarator carries a star, so the table
      records nothing. -/
  | pointerTypedef
  /-- `typedef int (*f)(void *);` — likewise. -/
  | functionPointerTypedef
  /-- `typedef int a[4];` — likewise. -/
  | arrayTypedef
  /-- No typedef of this name in the unit, or one recorded twice with
      disagreeing readings. -/
  | unresolvedName
  deriving DecidableEq, Repr

/-- The declared-type classification every *other* premise of the model reads
    (`typed_field_access::classify_specifiers`).

`aggregateTypedef` answers `opaque` here, exactly as an unrecorded name did
before this card: the new reading lives beside this classifier, not inside
it. -/
def classify : Spec → DeclaredKind
  | .arith => .scalar
  | .structTag => .record
  | .enumTag => .scalar
  | .aliasTypedef t => classify t
  | _ => .opaque

/-- The classifier as it stood before this card. The two are equal — which is
    the point: the reading this card adds is recorded where `classify` does not
    look. -/
def classifyBase : Spec → DeclaredKind
  | .arith => .scalar
  | .structTag => .record
  | .enumTag => .scalar
  | .aliasTypedef t => classifyBase t
  | _ => .opaque

/-- Whether the specifier list names an **object type that is not a pointer
    type** (`typed_field_access::is_non_pointer_object`).

Deliberately weaker than `classify`: the disjointness argument needs to know
only that the member is not itself a pointer, never what shape it is. -/
def isNonPointerObject : Spec → Bool
  | .arith => true
  | .structTag => true
  | .unionTag => true
  | .enumTag => true
  | .aggregateTypedef => true
  | .aliasTypedef t => isNonPointerObject t
  | _ => false

/-- One member declaration. -/
structure Member where
  /-- Declarator pointer levels. -/
  stars : Nat
  /-- The declarator carries an array modifier. -/
  array : Bool
  /-- The specifier list carries `volatile` or `_Atomic`. -/
  qualified : Bool
  spec : Spec
  deriving DecidableEq, Repr

/-- The member premise this card decides: a member declared with no `*`, no
    array modifier and no rejecting qualifier, over a specifier list this reader
    committed to as a non-pointer object type. -/
def addressable (m : Member) : Bool :=
  m.stars == 0 && !m.array && !m.qualified && isNonPointerObject m.spec

/-- The same premise as it stood before this card: the specifier side was the
    `record`/`scalar` classification, so a `union`-typed member and every typedef
    the table would not resolve — an aggregate a typedef defines above all —
    failed it. -/
def addressableBase (m : Member) : Bool :=
  m.stars == 0 && !m.array && !m.qualified
    && (classifyBase m.spec == DeclaredKind.record || classifyBase m.spec == DeclaredKind.scalar)

/-- The classification this card produces, in `QCore.MemberAddress`'s own
    vocabulary. -/
def classifyMember (m : Member) : AddressedMember :=
  if addressable m then AddressedMember.nonPointer
  else if m.stars == 0 then AddressedMember.refusedShape
  else AddressedMember.pointer

/-- The classification as it stood before this card. -/
def classifyMemberBase (m : Member) : AddressedMember :=
  if addressableBase m then AddressedMember.nonPointer
  else if m.stars == 0 then AddressedMember.refusedShape
  else AddressedMember.pointer

/-! ## The new reading is invisible to the classifier

Every premise of the model that is keyed on `classify` — the scalar no-op rule,
the borrowable, single-pointer, `void *` and record-pointer member sets, the
bounded chain walker, and the declared types of a function's own locals —
therefore decides byte-identically to the tree before this card. -/

theorem the_classifier_is_unchanged (s : Spec) : classify s = classifyBase s := by
  induction s with
  | aliasTypedef t ih => simp [classify, classifyBase, ih]
  | _ => rfl

theorem an_aggregate_definition_is_still_opaque_to_the_classifier :
    classify Spec.aggregateTypedef = DeclaredKind.opaque := by rfl

theorem an_aggregate_definition_behind_an_alias_is_still_opaque :
    classify (Spec.aliasTypedef Spec.aggregateTypedef) = DeclaredKind.opaque := by rfl

/-! ## The widening only adds -/

/-- Every specifier list the classifier committed to as an object type is one
    the widened premise accepts. This is the whole conservativity argument: the
    new predicate is weaker, pointwise. -/
theorem a_classified_specifier_is_a_non_pointer_object (s : Spec)
    (h : classify s = DeclaredKind.record ∨ classify s = DeclaredKind.scalar) :
    isNonPointerObject s = true := by
  induction s with
  | aliasTypedef t ih => exact ih (by simpa [classify] using h)
  | arith => rfl
  | structTag => rfl
  | enumTag => rfl
  | aggregateTypedef => rfl
  | unionTag => exact absurd h (by decide)
  | voidSpec => exact absurd h (by decide)
  | pointerTypedef => exact absurd h (by decide)
  | functionPointerTypedef => exact absurd h (by decide)
  | arrayTypedef => exact absurd h (by decide)
  | unresolvedName => exact absurd h (by decide)

theorem the_premise_only_adds (m : Member) (h : addressableBase m = true) :
    addressable m = true := by
  simp only [addressableBase, Bool.and_eq_true, beq_iff_eq, Bool.or_eq_true,
    Bool.not_eq_eq_eq_not, Bool.not_true] at h
  obtain ⟨⟨⟨zero, notArray⟩, notQualified⟩, kind⟩ := h
  have spec : isNonPointerObject m.spec = true := by
    refine a_classified_specifier_is_a_non_pointer_object m.spec ?_
    rw [the_classifier_is_unchanged]
    exact kind
  simp [addressable, zero, notArray, notQualified, spec]

/-- Pointwise: every member the earlier reader called `nonPointer` is
    `nonPointer` now. `QCore.MemberAddress.admitsAddress` reads the
    classification and nothing else about the member, so no address that was
    admitted before this card can have been withdrawn by it. -/
theorem the_widening_only_adds (m : Member)
    (h : classifyMemberBase m = AddressedMember.nonPointer) :
    classifyMember m = AddressedMember.nonPointer := by
  by_cases hb : addressableBase m = true
  · simp [classifyMember, the_premise_only_adds m hb]
  · simp only [classifyMemberBase, ite_eq_right hb] at h
    split at h <;> exact absurd h (by decide)

/-- And no member became a *pointer* that was not one: the star count decides
    that branch in both readers. -/
theorem the_widening_moves_no_member_into_the_pointer_class (m : Member)
    (h : classifyMember m = AddressedMember.pointer) :
    classifyMemberBase m = AddressedMember.pointer := by
  by_cases zero : (m.stars == 0) = true
  · exfalso
    unfold classifyMember at h
    by_cases ha : addressable m = true
    · rw [ite_eq_left ha] at h
      exact absurd h (by decide)
    · rw [ite_eq_right ha, ite_eq_left zero] at h
      exact absurd h (by decide)
  · have notBase : ¬ addressableBase m = true := by
      intro base
      exact zero (by
        simp only [addressableBase, Bool.and_eq_true] at base
        exact base.1.1.1)
    simp only [classifyMemberBase, ite_eq_right notBase, ite_eq_right zero]

/-! ## The widening is strict, on each new shape separately -/

/-- `pthread_mutex_t metrics_chunk_lock;` — an untagged `union` the typedef
    defines. The measured production case. -/
theorem an_aggregate_typedef_member_was_refused_and_is_now_addressable :
    classifyMemberBase ⟨0, false, false, .aggregateTypedef⟩ = AddressedMember.refusedShape
      ∧ classifyMember ⟨0, false, false, .aggregateTypedef⟩ = AddressedMember.nonPointer := by
  decide

/-- `msgpack_packer mp_pck;` reached through an intervening plain typedef: the
    chain keeps the reading. -/
theorem an_aliased_aggregate_typedef_member_is_addressable :
    classifyMember ⟨0, false, false, .aliasTypedef .aggregateTypedef⟩
      = AddressedMember.nonPointer := by decide

/-- `union tag member;` — a `union`-typed member of a `struct`. Its storage is
    disjoint from its siblings exactly as any other member's is; the reader had
    refused it only because the *classifier* answers `opaque` for a `union`. -/
theorem a_union_typed_member_was_refused_and_is_now_addressable :
    classifyMemberBase ⟨0, false, false, .unionTag⟩ = AddressedMember.refusedShape
      ∧ classifyMember ⟨0, false, false, .unionTag⟩ = AddressedMember.nonPointer := by
  decide

/-! ## A pointer is never addressable, by any spelling

This is the laundering shape the rule exists to refuse: the address of a pointer
member is a handle with which a callee could overwrite the very link this
projection maintains. -/

theorem an_addressable_member_has_no_declarator_star (m : Member)
    (h : addressable m = true) : m.stars = 0 := by
  simp only [addressable, Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.1

theorem a_declared_pointer_member_is_never_addressable (m : Member) (h : 0 < m.stars) :
    classifyMember m ≠ AddressedMember.nonPointer := by
  intro admitted
  have notAddressable : ¬ addressable m = true := by
    intro positive
    have := an_addressable_member_has_no_declarator_star m positive
    omega
  have zero : ¬ (m.stars == 0) = true := by
    simp only [beq_iff_eq]
    omega
  rw [classifyMember, ite_eq_right notAddressable, ite_eq_right zero] at admitted
  exact absurd admitted (by decide)

theorem a_pointer_typedef_member_is_never_addressable :
    classifyMember ⟨0, false, false, .pointerTypedef⟩ = AddressedMember.refusedShape := by decide

theorem a_function_pointer_typedef_member_is_never_addressable :
    classifyMember ⟨0, false, false, .functionPointerTypedef⟩
      = AddressedMember.refusedShape := by decide

theorem an_array_typedef_member_is_never_addressable :
    classifyMember ⟨0, false, false, .arrayTypedef⟩ = AddressedMember.refusedShape := by decide

theorem a_pointer_typedef_behind_an_alias_is_never_addressable :
    classifyMember ⟨0, false, false, .aliasTypedef .pointerTypedef⟩
      = AddressedMember.refusedShape := by decide

theorem an_array_member_is_never_addressable (s : Spec) :
    classifyMember ⟨0, true, false, s⟩ = AddressedMember.refusedShape := by
  simp [classifyMember, addressable]

theorem a_qualified_member_is_never_addressable (s : Spec) :
    classifyMember ⟨0, false, true, s⟩ = AddressedMember.refusedShape := by
  simp [classifyMember, addressable]

theorem an_unresolved_name_is_never_addressable :
    classifyMember ⟨0, false, false, .unresolvedName⟩ = AddressedMember.refusedShape := by decide

theorem a_void_member_is_never_addressable :
    classifyMember ⟨0, false, false, .voidSpec⟩ = AddressedMember.refusedShape := by decide

/-! ## An addressable member is never a linkable one

`QCore.MemberAddress` argues in prose that the address handed to the callee
denotes storage disjoint from every member this projection *links*. Here is the
half of that which is the reader's: the two member sets are disjoint by
construction, because a link is only ever recorded on a member with exactly one
declarator star and an addressable member has none. -/

/-- The members the aggregate-store rule may record a link on: exactly one
    declarator star over a resolved specifier list (`single_pointer_fields`) or
    the literal `void *` (`void_pointer_fields`). -/
def linkable (m : Member) : Bool :=
  m.stars == 1 && !m.array && !m.qualified
    && (classify m.spec != DeclaredKind.opaque || m.spec == Spec.voidSpec)

theorem an_addressable_member_is_never_linkable (m : Member)
    (h : classifyMember m = AddressedMember.nonPointer) : linkable m = false := by
  have positive : addressable m = true := by
    by_cases ha : addressable m = true
    · exact ha
    · rw [classifyMember, ite_eq_right ha] at h
      split at h <;> exact absurd h (by decide)
  have zero : m.stars = 0 := an_addressable_member_has_no_declarator_star m positive
  simp [linkable, zero]

theorem a_linkable_member_is_never_addressable (m : Member) (h : linkable m = true) :
    classifyMember m ≠ AddressedMember.nonPointer := by
  intro admitted
  rw [an_addressable_member_is_never_linkable m admitted] at h
  exact absurd h (by decide)

/-! ## The admission relation inherits every refusal unchanged

`QCore.MemberAddress.admitsAddress` is not touched by this card: the reader
decides the `member` field and nothing else. These re-state, over the reader's
own classification, that no custody-bearing position and no refused base can
reach an admitted address. -/

theorem no_custody_position_admits_a_read_member (a : Address) (m : Member)
    (_member : a.member = classifyMember m) (custody : a.position ≠ Position.borrowed) :
    admitsAddress a = false :=
  a_custody_bearing_position_never_admits_an_address a custody

theorem a_refused_base_never_admits_a_read_member (a : Address) (m : Member)
    (_member : a.member = classifyMember m) (refused : a.route = BaseRoute.refused) :
    admitsAddress a = false :=
  a_refused_base_never_admits_an_address a refused

theorem an_incomplete_record_never_admits_a_read_member (a : Address) (m : Member)
    (_member : a.member = classifyMember m) (incomplete : a.recordComplete = false) :
    admitsAddress a = false :=
  an_incomplete_record_never_admits_an_address a incomplete

/-- And the reader can only ever *withdraw* a refusal, never manufacture an
    admission on a member the earlier reader called a pointer. -/
theorem a_pointer_member_refuses_under_both_readers (a : Address) (m : Member)
    (member : a.member = classifyMember m) (pointer : 0 < m.stars) :
    admitsAddress a = false := by
  have notNonPointer : a.member ≠ AddressedMember.nonPointer := by
    rw [member]; exact a_declared_pointer_member_is_never_addressable m pointer
  cases hm : a.member
  case nonPointer => exact absurd hm notNonPointer
  case pointer => exact a_pointer_member_address_is_never_admitted a hm
  case refusedShape => exact a_refused_member_shape_never_admits_an_address a hm

/-! ## The shape recogniser is exact

`read_aggregate_definition` reads `struct|union|enum [TAG] { … } NAME` and
nothing else. Every other declarator keeps the absent reading it had. -/

/-- The declarator that follows the aggregate body. -/
inductive Declarator where
  /-- `} NAME ;` — the bare typedef name. -/
  | bareName
  /-- `} * NAME ;`. -/
  | pointerName
  /-- `} NAME [ 4 ] ;`. -/
  | arrayName
  /-- `} a , * b ;`. -/
  | severalNames
  deriving DecidableEq, Repr

/-- What one `typedef` declaration records for the name it declares. -/
inductive Reading where
  | aggregate
  | specifiers (s : Spec)
  | absent
  deriving DecidableEq, Repr

/-- The reader. `definesAggregate` is true when the declaration's specifier part
    is a `struct`/`union`/`enum` **with a body**. -/
def readTypedef (definesAggregate : Bool) (d : Declarator) (plain : Spec) : Reading :=
  if definesAggregate then
    match d with
    | .bareName => .aggregate
    | _ => .absent
  else
    match d with
    | .bareName => .specifiers plain
    | _ => .absent

theorem only_a_bare_name_after_the_body_is_read (d : Declarator) (plain : Spec)
    (h : d ≠ Declarator.bareName) : readTypedef true d plain = Reading.absent := by
  cases d <;> simp_all [readTypedef]

theorem a_pointer_declarator_after_the_body_is_not_read (plain : Spec) :
    readTypedef true .pointerName plain = Reading.absent := by rfl

theorem a_second_declarator_after_the_body_is_not_read (plain : Spec) :
    readTypedef true .severalNames plain = Reading.absent := by rfl

theorem an_array_declarator_after_the_body_is_not_read (plain : Spec) :
    readTypedef true .arrayName plain = Reading.absent := by rfl

theorem the_aggregate_definition_is_read :
    readTypedef true .bareName Spec.arith = Reading.aggregate := by rfl

/-- The plain shape is untouched: a typedef with no aggregate body records the
    specifier reading it always did. -/
theorem a_plain_typedef_keeps_its_reading (plain : Spec) :
    readTypedef false .bareName plain = Reading.specifiers plain := by rfl

/-! ## Recording a name twice is fail-closed -/

/-- The table's own rule: agreement keeps the reading, disagreement retires it. -/
def record : Option Reading → Reading → Reading
  | none, fresh => fresh
  | some existing, fresh => if existing = fresh then existing else Reading.absent

theorem a_name_recorded_twice_alike_keeps_its_reading (r : Reading) :
    record (some r) r = r := by simp [record]

theorem a_name_recorded_twice_differently_is_unresolved (r s : Reading) (h : r ≠ s) :
    record (some r) s = Reading.absent := by simp [record, h]

/-- The case the card has to get right: a unit carrying both
    `typedef struct tag { … } tag;` and `typedef struct tag tag;` records two
    disagreeing readings, so the name retires to the absent reading — exactly
    the refusal it had before this card, never a silent admission. -/
theorem an_aggregate_definition_and_a_plain_typedef_of_one_name_retire_it :
    record (some Reading.aggregate) (Reading.specifiers Spec.structTag) = Reading.absent := by
  decide

/-- And an unresolved name is never addressable, so the retirement is a
    refusal. -/
theorem a_retired_name_is_never_addressable :
    classifyMember ⟨0, false, false, .unresolvedName⟩ ≠ AddressedMember.nonPointer := by decide

end QCore.MemberAddressReader
