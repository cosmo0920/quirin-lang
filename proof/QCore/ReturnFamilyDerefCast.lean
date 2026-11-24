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

import QCore.ReturnFamilyStarCast

/-!
# A starred may-bind root counts through cast groups (card RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST, rule 74m)

Family S of `legacy_return_family` admits a name the body may bind as the
root of exactly one dereference. For a starred root, `stars_before` read the
tokens before the name back to the nearest cast group and stopped there
(`inner`). So `* ( long * ) * q` counted one `*`, and `q` was admitted as a
dereference base, although the outer `*` dereferences the value `*q` reads,
which is a second dereference (`old_admits_a_second_dereference`).

Rule 74m keeps that count and adds one that a cast group does not stop
(`through`, rule 12h's transparency, which member and subscript roots already
had). A starred root is admitted only when both counts are one, under both of
rule 74h's readings (`admits`).

* `through` is never less than `inner` (`inner_le_through`), so every
  admission was one at the base (`the_fix_only_refuses_more`).
* `through` is monotone in the reading (`through_mono`). An admitted root
  therefore counts exactly one dereference through every cast group under
  every reading C may take (`admitted_is_one_dereference`).
* `* ( T * ) q` dereferences the converted value, not `q`'s pointee alone. Its
  `inner` is 0, so it keeps its refusal (`the_converted_value_keeps_its_refusal`).
* `( T ) * q` keeps its admission (`a_cast_of_one_dereference_is_unchanged`).

## What this is not

This is not a model of the token scan. Each item's class is the Rust reader's
output (trust boundary 28): a `*` with the token before it, a group
`cast_semantics::type_group_open` reads as a type, and anything that stops the
count. That last class includes a `*` that rule W45-SR5's parenthesis bound
places outside the root's parentheses (`deep`), which stops both counts at the
same point. `derefVectors` binds the reader.
-/

namespace QCore.ReturnFamilyDerefCast

open QCore.ReturnFamilyStarCast (Before Admissible operandReading castReading operand_below cast_above)

/-- One step back from the root, nearest first. -/
inductive Item where
  /-- A `*`, classed by the token before it. -/
  | star (b : Before)
  /-- A `)` closing a group `type_group_open` reads as a type. -/
  | cast
  /-- A `*` beyond more opening parentheses than close after the root. -/
  | deep
  /-- Anything else: the count stops. -/
  | other
  deriving DecidableEq, Repr

/-- The count up to the nearest cast group (the base's `stars_before` with a
finite `closing`). -/
def inner (r : Before → Bool) : List Item → Nat
  | .star b :: rest => if r b then inner r rest + 1 else 0
  | _ => 0

/-- The count through every cast group (rule 74m's `through`). -/
def through (r : Before → Bool) : List Item → Nat
  | .star b :: rest => if r b then through r rest + 1 else 0
  | .cast :: rest => through r rest
  | _ => 0

theorem inner_le_through (r : Before → Bool) : ∀ l : List Item, inner r l ≤ through r l
  | [] => by simp [inner, through]
  | .star b :: rest => by
    have ih := inner_le_through r rest
    by_cases h : r b = true <;> simp [inner, through, h, ih]
  | .cast :: rest => by simp [inner]
  | .deep :: _ => by simp [inner]
  | .other :: _ => by simp [inner]

theorem inner_mono (p q : Before → Bool) (h : ∀ b, p b = true → q b = true) :
    ∀ l : List Item, inner p l ≤ inner q l
  | [] => by simp [inner]
  | .star b :: rest => by
    have ih := inner_mono p q h rest
    by_cases hp : p b = true
    · simp [inner, hp, h b hp, ih]
    · simp [inner, hp]
  | .cast :: _ => by simp [inner]
  | .deep :: _ => by simp [inner]
  | .other :: _ => by simp [inner]

theorem through_mono (p q : Before → Bool) (h : ∀ b, p b = true → q b = true) :
    ∀ l : List Item, through p l ≤ through q l
  | [] => by simp [through]
  | .star b :: rest => by
    have ih := through_mono p q h rest
    by_cases hp : p b = true
    · simp [through, hp, h b hp, ih]
    · simp [through, hp]
  | .cast :: rest => by simpa [through] using through_mono p q h rest
  | .deep :: _ => by simp [through]
  | .other :: _ => by simp [through]

/-- The base's admission of a starred may-bind root (rule 74h): one `*` up to
the nearest cast group, under both readings. -/
def oldAdmits (l : List Item) : Bool :=
  inner operandReading l == 1 && inner castReading l == 1

/-- Rule 74m: one `*` up to the nearest cast group and one through every cast
group, under both readings. -/
def admits (l : List Item) : Bool :=
  oldAdmits l && through operandReading l == 1 && through castReading l == 1

theorem the_fix_only_refuses_more (l : List Item) (h : admits l = true) :
    oldAdmits l = true := by
  simp only [admits, Bool.and_eq_true] at h
  exact h.1.1

/-- An admitted root is one dereference through every cast group, and one up
to the nearest, under every reading C may take of the undecided groups. -/
theorem admitted_is_one_dereference (r : Before → Bool) (hr : Admissible r)
    (l : List Item) (h : admits l = true) : through r l = 1 ∧ inner r l = 1 := by
  simp only [admits, oldAdmits, Bool.and_eq_true, beq_iff_eq] at h
  have t1 := through_mono _ _ (operand_below r hr) l
  have t2 := through_mono _ _ (cast_above r hr) l
  have i1 := inner_mono _ _ (operand_below r hr) l
  have i2 := inner_mono _ _ (cast_above r hr) l
  omega

/-- `* ( long * ) * q`: the base admitted a second dereference; the tip
refuses it. -/
theorem old_admits_a_second_dereference :
    oldAdmits [.star .resolvedCast, .cast, .star .notOperandEnd] = true ∧
      through castReading [.star .resolvedCast, .cast, .star .notOperandEnd] = 2 ∧
      admits [.star .resolvedCast, .cast, .star .notOperandEnd] = false := by
  decide

/-- `* ( long * ) q`: the dereference of the converted value keeps its refusal. -/
theorem the_converted_value_keeps_its_refusal :
    oldAdmits [.cast, .star .notOperandEnd] = false ∧
      admits [.cast, .star .notOperandEnd] = false := by
  decide

/-- `( long ) * q`: one dereference, admitted as at the base. -/
theorem a_cast_of_one_dereference_is_unchanged :
    admits [.star .resolvedCast, .cast] = true := by
  decide

/-! ## The token reader's vectors -/

/-- `(spelling, the items before the may-bind `q` nearest first, admitted at
the base, admitted)`. The Rust unit test reads the spellings and the last
field; the third field is pinned by `old_vectors_hold`. -/
def derefVectors : List (String × List Item × Bool × Bool) := [
  ("* ( long * ) * q", [.star .resolvedCast, .cast, .star .notOperandEnd], true, false),
  ("* ( long * ) ( long * ) * q",
    [.star .resolvedCast, .cast, .cast, .star .notOperandEnd], true, false),
  ("* ( long * ) ( * q )", [.star .notOperandEnd, .cast, .star .notOperandEnd], true, false),
  ("* ( ( long * ) * q )", [.star .resolvedCast, .cast, .star .notOperandEnd], true, false),
  ("* ( long * ) q", [.cast, .star .notOperandEnd], false, false),
  ("( long ) * q", [.star .resolvedCast, .cast], true, true),
  ("( int ) ( long ) * q", [.star .resolvedCast, .cast, .cast], true, true),
  ("* q", [.star .notOperandEnd], true, true),
  ("( * q )", [.star .notOperandEnd], true, true),
  ("* ( T * ) * q", [.star (.close .undecided), .cast, .star .notOperandEnd], false, false)
]

def derefVectorsAgree : Bool :=
  derefVectors.all (fun (_, items, _, admitted) => admits items == admitted)

def oldVectorsAgree : Bool :=
  derefVectors.all (fun (_, items, old, _) => oldAdmits items == old)

theorem deref_vectors_hold : derefVectorsAgree = true := by decide

theorem old_vectors_hold : oldVectorsAgree = true := by decide

end QCore.ReturnFamilyDerefCast
