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

import QCore.DeclaredAbiAgreement

/-!
# The typedef reader that card LOADER-ABI's ABI comparison resolves through

Card **CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF**. `QCore.DeclaredAbiAgreement` models
the comparison of a declared contract's ABI spellings against the analysed
translation unit's own C declaration, and takes the C side's *projection* as
given. This module models the reader that produces that projection through a
`typedef`, and the two shapes this card taught it to commit to.

## What was refused, and why it is a boundary rather than a disagreement

Card CONSTRUCTOR-RESIDUALS (§4.2) measured the whole remaining residual of
production `flb_input_chunk_create` down to one family: three callees whose
parameter types the reader would not commit to, withdrawn at every call site
with `abi-unreadable-declaration`. Two shapes:

* a `struct`/`union`/`enum` **defined by the typedef itself**
  (`typedef union { … } pthread_mutex_t;`), which the reader abandoned at the
  aggregate's own opening brace;
* a **function-pointer typedef**
  (`typedef int (*msgpack_packer_write)(void *, const char *, size_t);`).

## What is proved

* **The widening only adds.** Every reading the earlier reader committed to is
  unchanged, pointwise over targets and pointer depths — so no projection, and
  therefore no comparison, can have moved for a shape that was already read.
* **The widening is strict**, with a witness on each of the two shapes, so
  neither branch is vacuous.
* **An untagged aggregate projects exactly as a tagged one does**, at every
  pointer depth. That is the whole admission: the pointer projection reads no
  tag and no member, so `union tag *` and `typedef union { … } t; t *` differ in
  nothing this vocabulary can spell. As a **value** an aggregate still refuses.
* **A function pointer is its own projection.** `fnptr` is refused for every
  object pointer and every integer, and every other spelling is refused for a
  function pointer — the LP64 eight-byte coincidence buys nothing.
* **No custody mode is writable on it.** The loader's two gates key on the
  leading word `ptr`, and `fnptr` is not one, so `Borrowed` is the only mode a
  declaration may carry there and an owned result of it is rejected. This is why
  the admission is in the *refusing* direction for exactly what card LOADER-ABI
  exists to stop.
* **`ABI-FNPTR-DEPTH1`**: a pointer *to* a function pointer refuses, in both the
  direct and the aliased shape.
* **Resolution is fail-closed**: an absent name, an exhausted depth budget and a
  name recorded twice with different readings are all `unresolved`.
* **The tagged definition composes.** A tagged aggregate definition is recorded
  as the specifier pair it equals, so a unit that also carries the plain
  `typedef struct tag tag;` keeps one reading instead of collapsing to
  `unresolved` — a regression this card would otherwise have introduced.

## What this is not

A model of a **decision**, exactly as `QCore.DeclaredAbiAgreement` is. That the
token stream read is the translation unit, that the LP64 data model holds on the
audited targets, and C's own type system are premises supplied from outside.
Nothing here says a declaration is faithful to a C *body*.
-/

namespace QCore.TypedefProjection

open QCore.DeclaredAbiAgreement

/-- What one `typedef` name stands for, as far as this reader commits.

`object` covers every target that names an object type without defining one —
a tag (`struct cio_chunk`), a chain to another such name — carrying the pointer
depth the declarator folded. `scalar` covers an arithmetic target with its
admissible spellings. The last three are this card's subject. -/
inductive Target where
  /-- A named object type under `stars` pointer levels. -/
  | object (stars : Nat)
  /-- An arithmetic target and the spellings it admits. -/
  | scalar (spellings : List AbiType)
  /-- A `struct`/`union`/`enum` **defined by the typedef** and given no tag,
  under `stars` pointer levels. -/
  | aggregate (stars : Nat)
  /-- A pointer to a function, at exactly one pointer level. -/
  | functionPointer
  /-- Not committed to. -/
  | unresolved
  deriving DecidableEq, BEq, Repr

/-- The pointer projection of `stars` levels over a pointee that is or is not
`const`-qualified. Three levels and up have no spelling here. -/
def pointerProjection : Nat → Bool → Projection
  | 1, false => some [.ptrMut]
  | 1, true => some [.ptrConst]
  | 2, false => some [.ptrMutMut]
  | _, _ => none

/-- The projection of one typedef target under `outer` further pointer levels.

This is the widened reader. -/
def project (t : Target) (outer : Nat) (constPointee : Bool) : Projection :=
  match t with
  | .object k => pointerProjection (k + outer) constPointee
  | .scalar s => if outer = 0 then some s else pointerProjection outer constPointee
  | .aggregate k => pointerProjection (k + outer) constPointee
  -- `ABI-FNPTR-DEPTH1`: a pointer *to* a function pointer is an object pointer
  -- and would project into a custody-bearing spelling. Refused, not admitted.
  | .functionPointer => if outer = 0 then some [.fnptr] else none
  | .unresolved => none

/-- The reader as it stood before this card: the two new shapes were not
recorded at all, so every declaration using one was unreadable. -/
def projectBase (t : Target) (outer : Nat) (constPointee : Bool) : Projection :=
  match t with
  | .aggregate _ => none
  | .functionPointer => none
  | other => project other outer constPointee

/-! ### The widening only adds -/

/-- Pointwise: every reading the earlier reader committed to is the same
reading now. No comparison can have moved for a shape that was already read. -/
theorem the_widening_only_adds (t : Target) (outer : Nat) (constPointee : Bool)
    {x : List AbiType} (h : projectBase t outer constPointee = some x) :
    project t outer constPointee = some x := by
  cases t <;> simp_all [projectBase]

/-- And nothing that was read became unreadable. -/
theorem the_widening_loses_no_reading (t : Target) (outer : Nat) (constPointee : Bool)
    (h : (projectBase t outer constPointee).isSome = true) :
    (project t outer constPointee).isSome = true := by
  cases hb : projectBase t outer constPointee with
  | none => simp [hb] at h
  | some x => simp [the_widening_only_adds t outer constPointee hb]

/-! ### The widening is strict, on each shape separately -/

/-- `pthread_mutex_t *`: an untagged `union` definition behind one pointer. -/
theorem an_untagged_aggregate_pointer_was_unreadable_and_is_now_read :
    projectBase (.aggregate 0) 1 false = none
      ∧ project (.aggregate 0) 1 false = some [.ptrMut] := by decide

/-- `msgpack_packer_write`: a function-pointer typedef at a parameter. -/
theorem a_function_pointer_was_unreadable_and_is_now_read :
    projectBase .functionPointer 0 false = none
      ∧ project .functionPointer 0 false = some [.fnptr] := by decide

/-! ### An untagged aggregate projects exactly as a tagged one

The pointer projection reads no tag and no member, so the two differ in nothing
this vocabulary can spell. This is the whole admission, stated as an equality
rather than argued in prose. -/

theorem an_untagged_aggregate_projects_as_a_tagged_one (k outer : Nat) (constPointee : Bool) :
    project (.aggregate k) outer constPointee = project (.object k) outer constPointee := by
  rfl

/-- As a **value** an aggregate still refuses: this vocabulary has no spelling
for one, exactly as it has none for a `struct tag` value. -/
theorem an_aggregate_value_refuses (constPointee : Bool) :
    project (.aggregate 0) 0 constPointee = none := by
  cases constPointee <;> rfl

theorem a_named_object_value_refuses (constPointee : Bool) :
    project (.object 0) 0 constPointee = none := by
  cases constPointee <;> rfl

/-- Two levels reach the out-slot projection, and three reach nothing — the
depth boundary is the pointer table's, not this card's. -/
theorem an_aggregate_under_two_pointers_is_the_out_slot :
    project (.aggregate 1) 1 false = some [.ptrMutMut] := by decide

theorem an_aggregate_under_three_pointers_refuses :
    project (.aggregate 2) 1 false = none := by decide

/-! ### A function pointer is its own projection, in both directions -/

/-- Only `fnptr` is admitted there. -/
theorem only_a_function_pointer_spelling_is_admitted (t : AbiType) (constPointee : Bool) :
    admits (project .functionPointer 0 constPointee) t = true ↔ t = .fnptr := by
  cases constPointee <;> cases t <;> decide

/-- The LP64 eight-byte coincidence buys nothing: a function pointer is not an
integer reading, so a `usize` declaration of one is a disagreement. -/
theorem a_function_pointer_is_not_an_integer :
    admits (project .functionPointer 0 false) .usize = false
      ∧ admits (project .functionPointer 0 false) .u64 = false := by decide

/-- Nor an object pointer, which is the direction that matters: admitting one
would let a declaration write a custody mode on a function designator. -/
theorem a_function_pointer_is_not_an_object_pointer :
    admits (project .functionPointer 0 false) .ptrMut = false
      ∧ admits (project .functionPointer 0 false) .ptrConst = false
      ∧ admits (project .functionPointer 0 false) .ptrMutMut = false := by decide

/-- And the refusal runs the other way. Every pointer projection is a list of
`POINTER_ABI_TYPES` spellings, none of which is `fnptr`. -/
theorem a_pointer_projection_never_admits_a_function_pointer (k : Nat) (constPointee : Bool) :
    admits (pointerProjection k constPointee) .fnptr = false := by
  unfold pointerProjection
  split <;> decide

theorem no_object_pointer_admits_a_function_pointer (k outer : Nat) (constPointee : Bool) :
    admits (project (.object k) outer constPointee) .fnptr = false :=
  a_pointer_projection_never_admits_a_function_pointer (k + outer) constPointee

theorem no_untagged_aggregate_admits_a_function_pointer (k outer : Nat) (constPointee : Bool) :
    admits (project (.aggregate k) outer constPointee) .fnptr = false :=
  a_pointer_projection_never_admits_a_function_pointer (k + outer) constPointee

theorem no_arithmetic_projection_admits_a_function_pointer
    (s : List AbiType) (outer : Nat) (constPointee : Bool)
    (h : (s.any (· == AbiType.fnptr)) = false) :
    admits (project (.scalar s) outer constPointee) .fnptr = false := by
  show admits (if outer = 0 then some s else pointerProjection outer constPointee)
      AbiType.fnptr = false
  by_cases ho : outer = 0
  · rw [ite_eq_left ho]; simpa [admits] using h
  · rw [ite_eq_right ho]
    exact a_pointer_projection_never_admits_a_function_pointer outer constPointee

/-! ### `ABI-FNPTR-DEPTH1` -/

/-- A pointer *to* a function pointer refuses, at every further depth. -/
theorem a_pointer_to_a_function_pointer_refuses (outer : Nat) (constPointee : Bool) :
    project .functionPointer (outer + 1) constPointee = none := by
  simp [project]

/-! ### No custody mode is writable on a function pointer

The loader admits a custody mode only on a declared type whose leading word is
`ptr`, and `resource_out` only at `ptr<mut, ptr<mut, u8>>`; a non-`None` result
likewise requires a pointer return type. `fnptr` is not a `ptr` spelling, so
those rules — which were already there — make every custody mode on it a
load-time rejection. That is why this admission is in the refusing direction. -/

/-- Whether a spelling is one of `POINTER_ABI_TYPES`. -/
def isPointerSpelling : AbiType → Bool
  | .ptrMut | .ptrConst | .ptrMutMut => true
  | _ => false

/-- The v1 parameter modes. -/
inductive Mode where
  | borrowed | owned | consumed | released | out | stored | removed
  deriving DecidableEq, BEq, Repr

def allModes : List Mode :=
  [.borrowed, .owned, .consumed, .released, .out, .stored, .removed]

/-- The loader's gate, as written: everything but `Borrowed` needs a pointer
spelling, and `out` needs the pointer-to-pointer one. -/
def modeAdmissible (m : Mode) (t : AbiType) : Bool :=
  match m with
  | .borrowed => true
  | .out => t == .ptrMutMut
  | _ => isPointerSpelling t

theorem a_function_pointer_is_not_a_pointer_spelling :
    isPointerSpelling .fnptr = false := by decide

/-- `Borrowed` is the only mode a declaration may carry on an `fnptr`
parameter. -/
theorem borrowed_is_the_only_mode_on_a_function_pointer (m : Mode) :
    modeAdmissible m .fnptr = true ↔ m = .borrowed := by
  cases m <;> decide

theorem every_custody_mode_is_refused_on_a_function_pointer :
    (allModes.filter (fun m => modeAdmissible m .fnptr)) = [.borrowed] := by decide

/-- The loader's *result* gate, the second of the two: a non-`None` result
requires a pointer return type, decided on the spelling exactly as the parameter
gate is. `unit` is a result type only and carries no custody either. -/
def ownedResultAdmissible (t : AbiType) : Bool := isPointerSpelling t

/-- So an owned or reallocated result of a function pointer is rejected, by the
rule that was already there. -/
theorem an_owned_result_of_a_function_pointer_is_rejected :
    ownedResultAdmissible .fnptr = false := by decide

/-- Both gates are non-vacuous: the object-pointer spellings still carry
custody, so nothing was weakened to make the function-pointer half true. -/
theorem the_custody_gate_still_admits_object_pointers :
    modeAdmissible .released .ptrMut = true
      ∧ modeAdmissible .out .ptrMutMut = true
      ∧ modeAdmissible .out .ptrMut = false := by decide

theorem the_result_gate_still_admits_object_pointers :
    ownedResultAdmissible .ptrMut = true
      ∧ ownedResultAdmissible .ptrMutMut = true
      ∧ ownedResultAdmissible .usize = false := by decide

/-! ### Resolution: chains, depth and duplicate recordings -/

/-- What the table holds for one name. `alias` is `typedef OTHER *^stars NAME;`,
the one shape whose reading has to be followed. -/
inductive Entry where
  | direct (t : Target)
  | alias (name : Nat) (stars : Nat)
  | absent
  deriving Repr

abbrev Table := Nat → Entry

/-- Follow one name to the target this reader commits to, folding the pointer
depth of every link. `fuel` is the reader's `TYPEDEF_DEPTH_LIMIT`. -/
def resolve (table : Table) : Nat → Nat → Target
  | _, 0 => .unresolved
  | n, fuel + 1 =>
      match table n with
      | .absent => .unresolved
      | .direct t => t
      | .alias m k =>
          match resolve table m fuel with
          | .object j => .object (j + k)
          | .aggregate j => .aggregate (j + k)
          | .scalar s => if k = 0 then .scalar s else .object k
          | .functionPointer => if k = 0 then .functionPointer else .unresolved
          | .unresolved => .unresolved

theorem an_exhausted_depth_budget_is_unresolved (table : Table) (n : Nat) :
    resolve table n 0 = .unresolved := by rfl

theorem an_absent_name_is_unresolved (table : Table) (n fuel : Nat)
    (h : table n = .absent) : resolve table n (fuel + 1) = .unresolved := by
  simp [resolve, h]

/-- A name for a function pointer keeps its one pointer level through an alias
that adds none. -/
theorem a_chain_to_a_function_pointer_keeps_its_level (table : Table) (n m fuel : Nat)
    (hn : table n = .alias m 0) (hm : resolve table m fuel = .functionPointer) :
    resolve table n (fuel + 1) = .functionPointer := by
  simp [resolve, hn, hm]

/-- And an alias that adds one is `ABI-FNPTR-DEPTH1` again. -/
theorem an_alias_to_a_pointer_to_a_function_pointer_is_unresolved
    (table : Table) (n m fuel k : Nat)
    (hn : table n = .alias m (k + 1)) (hm : resolve table m fuel = .functionPointer) :
    resolve table n (fuel + 1) = .unresolved := by
  simp [resolve, hn, hm]

/-- An aggregate reached through an alias folds its depth exactly as a tagged
object type does — the same equality the projection has. -/
theorem an_alias_folds_an_aggregate_like_an_object (table : Table) (n m fuel k : Nat)
    (hn : table n = .alias m k) (j : Nat) (hm : resolve table m fuel = .aggregate j) :
    resolve table n (fuel + 1) = .aggregate (j + k) := by
  simp [resolve, hn, hm]

/-- An unresolved link poisons the whole chain. -/
theorem an_unresolved_link_is_unresolved (table : Table) (n m fuel k : Nat)
    (hn : table n = .alias m k) (hm : resolve table m fuel = .unresolved) :
    resolve table n (fuel + 1) = .unresolved := by
  simp [resolve, hn, hm]

/-! ### Recording a name twice -/

/-- The reader's own rule: a second reading that differs makes the name
unreadable. -/
def record (existing new : Target) : Target :=
  if existing = new then existing else .unresolved

theorem a_name_recorded_twice_alike_keeps_its_reading (t : Target) :
    record t t = t := by simp [record]

theorem a_name_recorded_twice_differently_is_unresolved {a b : Target} (h : a ≠ b) :
    record a b = .unresolved := by simp [record, h]

/-- The regression this card would otherwise have introduced, closed by
construction: a **tagged** aggregate definition is recorded as the specifier
pair it equals — `.object 0`, the same reading `typedef struct tag tag;`
produces — so a unit carrying both keeps one reading. Recording it as
`.aggregate 0` instead would have collapsed the pair to `unresolved` and
withdrawn a declaration that agreed before this card. -/
theorem a_tagged_definition_composes_with_its_plain_typedef :
    record (.object 0) (.object 0) = .object 0 := by decide

theorem an_untagged_definition_would_not_have_composed :
    record (.aggregate 0) (.object 0) = .unresolved := by decide

/-! ### The two new readings are distinguishable from every old one

Non-vacuity in the other direction: nothing here quietly identifies the new
targets with an existing one. -/

theorem a_function_pointer_is_not_any_other_target :
    (.functionPointer : Target) ≠ .object 1
      ∧ (.functionPointer : Target) ≠ .aggregate 1
      ∧ (.functionPointer : Target) ≠ .unresolved := by decide

theorem an_untagged_aggregate_is_its_own_target (k : Nat) :
    (Target.aggregate k) ≠ .object k := by
  intro h
  exact Target.noConfusion h

end QCore.TypedefProjection
