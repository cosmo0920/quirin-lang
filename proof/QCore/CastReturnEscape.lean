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

/-!
# A cast in `return` position (card W44-LEAK-PIN-AUDIT, `RETURN-CAST-PLACE-NOT-ESCAPED`)

Before this card the gateway's `return` parser escaped `return p;` but read
`return (T *) p;` as returning **no place**, so the site `p` held stayed live and
`report_leaks` accused it at that exit: a false leak for a pointer the function
hands to its caller (`mk_string_copy_substr`'s `return (char *) buffer;`), and
the reason W43-DESCRIPTOR-RESOURCE-TRACKING withdrew its `dlopen` declaration
(`return (rd_dl_hnd_t *) handle;`).

The rule this module models (`crates/qcore-c-gateway/src/legacy_cast_return.rs`)
decides `return ( TYPE ) OPERAND ;` by the **carrier** the cast converts between:

* a pointer target (`TYPE` ends in `*`; a chain of pointer casts counts as one)
  over an operand that is one place, is not a declared scalar and carries no
  tracked integer identity is a pointer-to-pointer conversion, which leaves the
  object unchanged (C11 6.3.2.3p7): the return is decided **as the uncast
  return of that place** (`asOperand`), by whatever the existing rules answer
  for it — an escape, an admitted place-less exit, or a refusal. The cast adds
  no admission of its own;
* `(int)` over a plain name that may hold a tracked descriptor converts `int` to
  `int`: it is decided as the uncast return too, so the descriptor stays
  custody;
* any other cast of a possible descriptor, and a non-pointer cast of an operand
  the reader commits to as a pointer, changes the carrier of a possibly tracked
  value: it is **refused**, never an escape;
* everything else keeps its pre-card answer (`preCard`), which escapes nothing.

## What is proved

* Delegation happens only on a carrier-preserving conversion, and a pointer
  target delegates only a place.
* An integer↔pointer crossing of a possibly tracked value is refused, in both
  directions; untracked scalar data keeps its pre-card answer.
* The cast is the uncast return: a delegated answer reports exactly what the
  uncast return reports.
* The exit report only withdraws: whatever the rule answers, it reports about
  the returned site at most what the pre-card place-less exit reported, given
  that the uncast return itself escapes, withdraws by refusal (the
  refused hand-on rule, `QCore.RefusedHandOnCustody`) or is place-less.
* The corpus false leak is gone, and every answer is reached (non-vacuity).

## What is a premise

That the declaration reader classifies operands faithfully and that
`DescriptorNames` over-approximates the names holding a tracked descriptor are
the gateway's standing extraction premises (trust boundary 28). The
Rust–Lean correspondence is maintained by review, not proved.
-/

namespace QCore.CastReturnEscape

/-- The target type of the cast, as the parser reads its tokens. -/
inductive Target where
  /-- The type name ends in `*` (every link of a cast chain does). -/
  | pointer
  /-- Exactly `int`. -/
  | int
  /-- Any other scalar type, or a typedef name the reader does not resolve. -/
  | other
  deriving DecidableEq, Repr

/-- What the parser knows about the operand. -/
structure Operand where
  /-- The operand may hold a tracked integer identity (`DescriptorNames`). -/
  descriptor : Bool
  /-- The declaration reader commits to a scalar type for the operand. -/
  declaredScalar : Bool
  /-- The declaration reader commits to a pointer type for the operand. -/
  declaredPointer : Bool
  /-- `canonical_place` spells the operand as one place. -/
  place : Bool
  deriving DecidableEq, Repr

/-- The parser's answer for `return ( TYPE ) OPERAND ;`. -/
inductive Answer where
  /-- Decide the return as `return OPERAND ;`. -/
  | asOperand
  /-- A refusal: an `Unsupported` node that reads the operand as its value. -/
  | refuse
  /-- The pre-card answer: the exit returns no place. -/
  | preCard
  deriving DecidableEq, Repr

/-- The decision. -/
def decide (t : Target) (o : Operand) : Answer :=
  if o.descriptor then
    (if t = .int ∧ o.place then .asOperand else .refuse)
  else if o.declaredScalar then .preCard
  else match t with
    | .pointer => if o.place then .asOperand else if o.declaredPointer then .refuse else .preCard
    | _ => if o.declaredPointer then .refuse else .preCard

/-! ## Delegation preserves the carrier -/

/-- Delegation happens only for a pointer-to-pointer conversion of a place that
    is neither a declared scalar nor a possible descriptor, or for `(int)` over
    a possible descriptor. -/
theorem delegation_preserves_the_carrier (t : Target) (o : Operand)
    (h : decide t o = .asOperand) :
    o.place = true ∧
      ((t = .pointer ∧ o.descriptor = false ∧ o.declaredScalar = false) ∨
        (t = .int ∧ o.descriptor = true)) := by
  unfold decide at h
  cases hd : o.descriptor <;> cases hs : o.declaredScalar <;> cases t <;>
    cases hp : o.declaredPointer <;> cases hpl : o.place <;> simp_all

/-- A tracked descriptor returned through `(int)` is decided as `return fd;`:
    it stays custody. -/
theorem a_descriptor_through_int_is_the_uncast_return (o : Operand)
    (h : o.descriptor = true) (hp : o.place = true) :
    decide .int o = .asOperand := by
  simp [decide, h, hp]

/-! ## Integer and pointer crossings of a tracked value refuse -/

/-- Integer to pointer: a possible descriptor cast to a pointer type refuses. -/
theorem a_descriptor_cast_to_a_pointer_refuses (o : Operand) (h : o.descriptor = true) :
    decide .pointer o = .refuse := by
  simp [decide, h]

/-- A possible descriptor cast to any integer type but `int` refuses. -/
theorem a_descriptor_cast_to_another_integer_refuses (o : Operand)
    (h : o.descriptor = true) : decide .other o = .refuse := by
  simp [decide, h]

/-- Pointer to integer: an operand committed to as a pointer, cast to a
    non-pointer type, refuses. -/
theorem a_pointer_cast_to_an_integer_refuses (t : Target) (o : Operand)
    (ht : t ≠ .pointer) (hd : o.descriptor = false) (hs : o.declaredScalar = false)
    (hp : o.declaredPointer = true) : decide t o = .refuse := by
  cases t with
  | pointer => exact absurd rfl ht
  | int => simp [decide, hd, hs, hp]
  | other => simp [decide, hd, hs, hp]

/-- No integer-to-pointer or pointer-to-integer crossing of a possibly tracked
    value is delegated. -/
theorem no_crossing_is_delegated (t : Target) (o : Operand)
    (crossing : (o.descriptor = true ∧ t ≠ .int) ∨
      (o.descriptor = false ∧ o.declaredScalar = false ∧ o.declaredPointer = true ∧
        t ≠ .pointer)) :
    decide t o ≠ .asOperand := by
  intro h
  unfold decide at h
  rcases crossing with ⟨hd, ht⟩ | ⟨hd, hs, hp, ht⟩ <;> cases t <;>
    cases hpl : o.place <;> simp_all

/-- Untracked scalar data keeps its pre-card answer whatever the target. -/
theorem untracked_scalar_data_is_unchanged (t : Target) (o : Operand)
    (hd : o.descriptor = false) (hs : o.declaredScalar = true) :
    decide t o = .preCard := by
  simp [decide, hd, hs]

/-- A pointer cast of something that is not one place is not delegated. -/
theorem a_pointer_cast_of_an_expression_is_not_delegated (o : Operand)
    (hpl : o.place = false) : decide .pointer o ≠ .asOperand := by
  unfold decide
  cases hd : o.descriptor <;> cases hs : o.declaredScalar <;>
    cases hp : o.declaredPointer <;> simp [hpl]

/-! ## The exit report -/

/-- What the existing rules answer for the uncast `return OPERAND ;`. -/
inductive Uncast where
  /-- `EscapeIfOwned` of a canonical place. -/
  | escape
  /-- A place-less exit (the pointer-chain widening, or no place). -/
  | noop
  /-- A refusal, which the refused hand-on rule withdraws the operand on. -/
  | refuse
  deriving DecidableEq, Repr

/-- What one exit reports about the returned site, given whether it is live
    before the return. A refusal reports nothing because the refused-escape
    rule withdraws the identity the refused return hands on
    (`QCore.RefusedHandOnCustody.a_refused_hand_on_is_withdrawn`). -/
def reportedUncast (u : Uncast) (live : Bool) : Bool :=
  match u with
  | .escape => false
  | .noop => live
  | .refuse => false

def reported (a : Answer) (u : Uncast) (live : Bool) : Bool :=
  match a with
  | .asOperand => reportedUncast u live
  | .refuse => false
  | .preCard => live

/-- The cast is the uncast return. -/
theorem the_cast_is_the_uncast_return (t : Target) (o : Operand) (u : Uncast) (live : Bool)
    (h : decide t o = .asOperand) : reported (decide t o) u live = reportedUncast u live := by
  rw [h]; rfl

/-- The card's rule never adds an accusation about the returned site: whatever
    it answers, it reports at most what the pre-card place-less exit reported. -/
theorem the_rule_only_withdraws (t : Target) (o : Operand) (u : Uncast) (live : Bool)
    (h : reported (decide t o) u live = true) : reported .preCard u live = true := by
  cases hdec : decide t o <;> cases u <;> simp_all [reported, reportedUncast]

/-- The false leak is gone: `return (char *) buffer;` over a live heap buffer,
    whose uncast return escapes it, reports nothing. -/
theorem the_cast_return_leak_is_withdrawn :
    reported (decide .pointer ⟨false, false, true, true⟩) .escape true = false := by
  rfl

/-! ## Non-vacuity -/

theorem every_answer_is_reached :
    decide .pointer ⟨false, false, true, true⟩ = .asOperand ∧
      decide .other ⟨false, false, true, true⟩ = .refuse ∧
      decide .other ⟨false, true, false, true⟩ = .preCard ∧
      decide .int ⟨true, true, false, true⟩ = .asOperand ∧
      decide .pointer ⟨true, true, false, true⟩ = .refuse ∧
      decide .pointer ⟨false, false, false, false⟩ = .preCard := by
  refine ⟨rfl, rfl, rfl, rfl, rfl, rfl⟩

end QCore.CastReturnEscape
