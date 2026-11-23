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

import QCore.CastBeforeAmpersand

/-!
# Family S keeps both readings of an undecided cast (card RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST, rule 74h)

Family S of `legacy_return_family` (W45-RETURN-EXPRESSION-FAMILIES) reads a
single `*` or `&` by the token before it. At the base a `)` made it unary only
when the declaration reader resolved the group as a type (`is_cast_group`);
every other `)` read as a binary operator (`oldUnary`). Rule 74h keeps the
resolved reading and asks rule 74f's shared detector
(`ampersand_runs::may_close_a_cast`, `QCore.CastBeforeAmpersand.Group`) about
every other `)`. A group the detector rules out is an operand; every other
group is **undecided** (`operatorAt`).

An undecided operator has two readings, and family S has three tests on it.
None of the three readings contains the other, so the tip keeps what **any**
reading C may take (`Admissible`) would raise:

* **the `&` refusal** fires when some reading takes an address
  (`address_keeps_every_reading_refusal`);
* **the starred base** of an untracked root is the union of the readings'
  bases (`starred_keeps_every_reading_base`). The base missed it for
  `(T) *p` with `T` unresolved (`old_reading_misses_the_base`);
* **a may-bind root** is admitted only when every reading counts exactly one
  dereference (`starred_admission_needs_every_reading`), and a member or
  subscript root only when every reading counts none
  (`member_admission_needs_every_reading`). The count is monotone in the
  reading (`count_mono`), so the operand reading and the cast reading bound
  every mixed one. The cast reading alone would admit `(T) *q`, which the
  base refused (`the_cast_reading_alone_would_admit`); the tip keeps that
  refusal.

Every test refuses wherever the base refused, and every base the base read
the tip still reads (`the_fix_only_refuses_more_*`, `old_starred_kept`). No
admission moves toward `Safe`. `familyVectors` binds the token reader.

## What this is not

A model of C's grammar or of the token scan. The class of each `)` is the
Rust readers' output (trust boundary 28), as are the importer's tokens and the
completeness of rule 74f's declared-object set.
-/

namespace QCore.ReturnFamilyStarCast

open QCore.CastBeforeAmpersand (Group)

/-- The token before a single `*` or `&`, as family S classifies it. -/
inductive Before where
  /-- Nothing, an operator or an opening group. -/
  | notOperandEnd
  /-- A name, a number or `]`. -/
  | operandEnd
  /-- A `)` whose group the declaration reader resolves as a type. -/
  | resolvedCast
  /-- Any other `)`, classed by the shared detector. -/
  | close (g : Group)
  deriving DecidableEq, Repr

inductive Operator where
  | unary
  | binary
  | undecided
  deriving DecidableEq, Repr

/-- Rule 74h's reading (`operator_at`). -/
def operatorAt : Before → Operator
  | .notOperandEnd => .unary
  | .operandEnd => .binary
  | .resolvedCast => .unary
  | .close g => if g.mayBeCast then .undecided else .binary

/-- The base's `unary_position`: an unresolved `)` read as binary. -/
def oldUnary : Before → Bool
  | .notOperandEnd => true
  | .operandEnd => false
  | .resolvedCast => true
  | .close _ => false

/-- An operator under one choice for the undecided ones (`true`: unary). -/
def Operator.under : Operator → Bool → Bool
  | .unary, _ => true
  | .binary, _ => false
  | .undecided, cast => cast

/-- A reading C may take: unary where the operator is unary, binary where it
is binary, either where it is undecided. -/
def Admissible (r : Before → Bool) : Prop :=
  ∀ b, (operatorAt b = .unary → r b = true) ∧ (operatorAt b = .binary → r b = false)

/-- The operand reading of every undecided operator. -/
def operandReading (b : Before) : Bool := (operatorAt b).under false

/-- The cast reading of every undecided operator. -/
def castReading (b : Before) : Bool := (operatorAt b).under true

theorem operand_below (r : Before → Bool) (hr : Admissible r) (b : Before)
    (h : operandReading b = true) : r b = true := by
  unfold operandReading at h
  cases ho : operatorAt b <;> simp_all [Operator.under]
  exact (hr b).1 ho

theorem cast_above (r : Before → Bool) (hr : Admissible r) (b : Before)
    (h : r b = true) : castReading b = true := by
  unfold castReading
  cases ho : operatorAt b <;> simp [Operator.under]
  have := (hr b).2 ho
  simp_all

theorem the_old_reading_is_the_operand_reading (b : Before) :
    oldUnary b = operandReading b := by
  cases b with
  | close g => cases hg : g.mayBeCast <;> simp [oldUnary, operandReading, operatorAt, hg, Operator.under]
  | _ => rfl

/-! ## The `&` refusal -/

/-- Rule 74h: refused when some reading takes an address. -/
def refusesAddress (b : Before) : Bool := castReading b

theorem address_keeps_every_reading_refusal (r : Before → Bool) (hr : Admissible r)
    (b : Before) (h : r b = true) : refusesAddress b = true :=
  cast_above r hr b h

theorem address_old_refusal_kept (b : Before) (h : oldUnary b = true) :
    refusesAddress b = true := by
  rw [the_old_reading_is_the_operand_reading] at h
  cases ho : operatorAt b <;>
    simp_all [refusesAddress, castReading, operandReading, Operator.under]

/-! ## The starred base of an untracked root -/

/-- The bases one reading gives the untracked root `p` after its `*`. -/
def basesUnder (r : Before → Bool) (b : Before) (p : String) : List String :=
  if r b then [p] else []

/-- Rule 74h: the union of the readings' bases. -/
def starredBases (b : Before) (p : String) : List String := basesUnder castReading b p

theorem starred_keeps_every_reading_base (r : Before → Bool) (hr : Admissible r)
    (b : Before) (p q : String) (h : q ∈ basesUnder r b p) : q ∈ starredBases b p := by
  unfold basesUnder at h
  unfold starredBases basesUnder
  cases hrb : r b
  · simp [hrb] at h
  · simp_all [cast_above r hr b hrb]

theorem old_starred_kept (b : Before) (p q : String) (h : q ∈ basesUnder oldUnary b p) :
    q ∈ starredBases b p := by
  have hold : Admissible oldUnary := by
    intro b'
    rw [the_old_reading_is_the_operand_reading]
    constructor <;> intro ho <;> simp [operandReading, ho, Operator.under]
  exact starred_keeps_every_reading_base oldUnary hold b p q h

/-- `return (T) *p;` with `T` unresolved: the base made `p` no base. -/
theorem old_reading_misses_the_base :
    basesUnder oldUnary (.close .undecided) "p" = [] ∧
      starredBases (.close .undecided) "p" = ["p"] := by
  decide

/-! ## A may-bind root's dereference count -/

/-- `stars_before`: the `*`s before a root, nearest first, counted while the
reading reads them unary. -/
def count (r : Before → Bool) (stars : List Before) : Nat := (stars.takeWhile r).length

theorem takeWhile_length_mono {α : Type} (p q : α → Bool) (h : ∀ x, p x = true → q x = true) :
    ∀ l : List α, (l.takeWhile p).length ≤ (l.takeWhile q).length
  | [] => by simp
  | x :: xs => by
    by_cases hp : p x = true
    · have hq := h x hp
      have ih := takeWhile_length_mono p q h xs
      simp [hp, hq, ih]
    · simp [List.takeWhile_cons, hp]

/-- The count is monotone in the reading: the operand reading's is the least,
the cast reading's the greatest. -/
theorem count_mono (r : Before → Bool) (hr : Admissible r) (stars : List Before) :
    count operandReading stars ≤ count r stars ∧ count r stars ≤ count castReading stars :=
  ⟨takeWhile_length_mono _ _ (operand_below r hr) stars,
   takeWhile_length_mono _ _ (cast_above r hr) stars⟩

/-- Rule 74h: a starred may-bind root, exactly one dereference under both
readings. -/
def admitsStarred (stars : List Before) : Bool :=
  count operandReading stars == 1 && count castReading stars == 1

/-- Rule 74h: a member or subscript root, no `*` under the cast reading. -/
def admitsMember (stars : List Before) : Bool := count castReading stars == 0

theorem starred_admission_needs_every_reading (r : Before → Bool) (hr : Admissible r)
    (stars : List Before) (h : admitsStarred stars = true) : count r stars = 1 := by
  simp only [admitsStarred, Bool.and_eq_true, beq_iff_eq] at h
  have := count_mono r hr stars
  omega

theorem member_admission_needs_every_reading (r : Before → Bool) (hr : Admissible r)
    (stars : List Before) (h : admitsMember stars = true) : count r stars = 0 := by
  simp only [admitsMember, beq_iff_eq] at h
  have := count_mono r hr stars
  omega

/-- The base's count, `unary_position` read back. -/
def oldCount (stars : List Before) : Nat := count oldUnary stars

theorem old_count_is_the_operand_count (stars : List Before) :
    oldCount stars = count operandReading stars := by
  unfold oldCount count
  have : oldUnary = operandReading := funext the_old_reading_is_the_operand_reading
  rw [this]

theorem the_fix_only_refuses_more_starred (stars : List Before)
    (h : admitsStarred stars = true) : oldCount stars = 1 := by
  simp only [admitsStarred, Bool.and_eq_true, beq_iff_eq] at h
  rw [old_count_is_the_operand_count]
  exact h.1

theorem the_fix_only_refuses_more_member (stars : List Before)
    (h : admitsMember stars = true) : oldCount stars = 0 := by
  simp only [admitsMember, beq_iff_eq] at h
  have hle := takeWhile_length_mono _ _
    (operand_below castReading (by
      intro b
      constructor <;> intro ho <;> simp [castReading, ho, Operator.under]) ) stars
  rw [old_count_is_the_operand_count]
  unfold count at h ⊢
  omega

/-- `return (T) *q;` with `T` unresolved and `q` may-bind: the cast reading
alone would admit a dereference base; the tip keeps the base's refusal. -/
theorem the_cast_reading_alone_would_admit :
    count castReading [.close .undecided] = 1 ∧ oldCount [.close .undecided] = 0 ∧
      admitsStarred [.close .undecided] = false := by
  decide

/-- `return (long) *q;`: a resolved cast is one dereference, as at the base. -/
theorem a_resolved_cast_is_unchanged :
    admitsStarred [.resolvedCast] = true ∧ oldCount [.resolvedCast] = 1 := by
  decide

/-- `return (T) * *q;`: one dereference as an operand, two as a cast. The
base admitted it; the tip refuses. -/
theorem an_undecided_second_star_refuses :
    oldCount [.notOperandEnd, .close .undecided] = 1 ∧
      admitsStarred [.notOperandEnd, .close .undecided] = false := by
  decide

/-! ## The token reader's vectors -/

/-- `(spelling, token before the `*` of the untracked root `p`, is `p` a
base)`. The Rust unit test reads the spellings and the last field. -/
def starredVectors : List (String × Before × Bool) := [
  ("( T ) * p", .close .undecided, true),
  ("( T * ) * p", .close .undecided, true),
  ("( long ) * p", .resolvedCast, true),
  ("* p", .notOperandEnd, true),
  ("( a ) * p", .close .object, false),
  ("( n + 1 ) * p", .close .expression, false),
  ("n * p", .operandEnd, false)
]

/-- `(spelling, token before a single `&`, refused)`. -/
def addressVectors : List (String × Before × Bool) := [
  ("( T ) & n", .close .undecided, true),
  ("( long ) & n", .resolvedCast, true),
  ("( a ) & n", .close .object, false),
  ("( n > > 8 ) & 0xff", .close .expression, false),
  ("n & 1", .operandEnd, false)
]

/-- `(spelling, the root is a member root, the `*`s before the may-bind `q`
nearest first, admitted)`. -/
def rootVectors : List (String × Bool × List Before × Bool) := [
  ("( T ) * q", false, [.close .undecided], false),
  ("( long ) * q", false, [.resolvedCast], true),
  ("* q", false, [.notOperandEnd], true),
  ("( T ) * * q", false, [.notOperandEnd, .close .undecided], false),
  ("( a ) * * q", false, [.notOperandEnd, .close .object], true),
  ("( T ) * q - > len", true, [.close .undecided], false),
  ("( a ) * q - > len", true, [.close .object], true)
]

def familyVectors : Bool :=
  starredVectors.all (fun (_, b, base) => (starredBases b "p" == ["p"]) == base) &&
  addressVectors.all (fun (_, b, refused) => refusesAddress b == refused) &&
  rootVectors.all (fun (_, member, stars, admitted) =>
    (if member then admitsMember stars else admitsStarred stars) == admitted)

theorem family_vectors_hold : familyVectors = true := by decide

end QCore.ReturnFamilyStarCast
