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

import QCore.ReturnFamilyDerefCast

/-!
# A dereference outside a starred root's parentheses (card RETURN-FAMILY-STAR-OUTSIDE-ROOT-PARENTHESES, rule 74s)

Family S of `legacy_return_family` admits a name the body may bind as the
root of exactly one dereference. For a starred root, `stars_before` reads back
from the name through `(` and `*` and stops at anything else: a binary
operator (`* ( 1 + * q )`) or a `*` beyond the root's own parentheses
(`* ( * q + 1 )`, the `deep` item of `QCore.ReturnFamilyDerefCast`). The tokens
after the root are read only up to its own parentheses
(`ends_one_dereference`), so `( * q + 1 ) [ 0 ]` passes too. Each of these
counted one `*`, and `q` was admitted, although the outer `*`, `[` or `->`
dereferences a pointer computed from the value `*q` reads: a second
dereference (`old_admits_a_second_dereference`).

Rule 74s keeps rule 74m's counts and adds the groups enclosing the root
(`Enclosing`). A group is dereferenced under a reading when a `[` or `->`
follows it or the `*` before it is unary under that reading. A starred root
is admitted only when no enclosing group may be dereferenced, that is, under
the cast reading, the larger one (`admits`).

* Every admission was one at the base (`the_fix_only_refuses_more`).
* Under every reading C may take of rule 74h's undecided groups, an admitted
  root has no dereferenced enclosing group
  (`admitted_has_no_outside_dereference`), so its whole count, rule 74m's
  `through` plus the dereferenced enclosing groups, is exactly one
  (`admitted_is_one_dereference`).
* `( * q )`, `* q + 1`, `( T ) * q` and `n * ( * q + 1 )` keep their admission
  and `* ( T * ) * q` its refusal (`outside_vectors_hold`).

## What this is not

This is not a model of the token scan. Each item's class, each enclosing
group, the `*` reached back from its `(` and what follows its `)` are the Rust
reader's output (trust boundary 28), bound by `outsideVectors`. The reader
cannot tell whether a dereferenced group's value is the pointer computed from
the root (`* ( p + * q )` with a pointer `p` dereferences `p`'s object), so
every dereferenced enclosing group refuses: the fail-closed reading.
-/

namespace QCore.ReturnFamilyStarOutside

open QCore.ReturnFamilyStarCast (Before Admissible operandReading castReading operand_below cast_above)
open QCore.ReturnFamilyDerefCast (Item through inner)

/-- What follows an enclosing group's `)`. -/
inductive After where
  /-- A `[`: the group is subscripted. -/
  | subscript
  /-- A `->`: a member of the group's pointee is selected. -/
  | arrow
  /-- Anything else, `.` and the end included. -/
  | other
  deriving DecidableEq, Repr

def After.derefs : After → Bool
  | .subscript => true
  | .arrow => true
  | .other => false

/-- A parenthesised group enclosing the root, innermost first. -/
structure Enclosing where
  /-- The `*` the reader reaches back from the group's `(` through
  parenthesised groups, classed by the token before it, if any. -/
  star : Option Before
  after : After
  deriving DecidableEq, Repr

/-- The group is dereferenced under the reading `r`. -/
def Enclosing.deref (r : Before → Bool) (g : Enclosing) : Bool :=
  g.after.derefs || (match g.star with
    | some b => r b
    | none => false)

/-- `group_dereferenced`: under either reading (`may_be_unary` is the cast
reading). -/
def Enclosing.mayDeref (g : Enclosing) : Bool := g.deref castReading

theorem deref_below_may (r : Before → Bool) (hr : Admissible r) (g : Enclosing)
    (h : g.deref r = true) : g.mayDeref = true := by
  unfold Enclosing.mayDeref Enclosing.deref at *
  cases hs : g.star with
  | none => simp_all
  | some b =>
    simp only [hs, Bool.or_eq_true] at h ⊢
    rcases h with h | h
    · exact Or.inl h
    · exact Or.inr (cast_above r hr b h)

/-- The dereferenced enclosing groups under `r`. -/
def outside (r : Before → Bool) (gs : List Enclosing) : Nat :=
  (gs.filter (Enclosing.deref r)).length

/-- The root's whole count under `r`: rule 74m's `through` and every
dereferenced enclosing group. -/
def total (r : Before → Bool) (l : List Item) (gs : List Enclosing) : Nat :=
  through r l + outside r gs

/-- Rule 74s: rule 74m's admission, and no enclosing group may be
dereferenced. -/
def admits (l : List Item) (gs : List Enclosing) : Bool :=
  QCore.ReturnFamilyDerefCast.admits l && gs.all (fun g => !g.mayDeref)

theorem the_fix_only_refuses_more (l : List Item) (gs : List Enclosing)
    (h : admits l gs = true) : QCore.ReturnFamilyDerefCast.admits l = true := by
  simp only [admits, Bool.and_eq_true] at h
  exact h.1

theorem admitted_has_no_outside_dereference (r : Before → Bool) (hr : Admissible r)
    (l : List Item) (gs : List Enclosing) (h : admits l gs = true) : outside r gs = 0 := by
  simp only [admits, Bool.and_eq_true, List.all_eq_true, Bool.not_eq_true'] at h
  unfold outside
  rw [List.length_eq_zero_iff, List.filter_eq_nil_iff]
  intro g hg hd
  have := deref_below_may r hr g hd
  simp_all

/-- An admitted root is one dereference, counting every enclosing group,
under every reading C may take of the undecided groups. -/
theorem admitted_is_one_dereference (r : Before → Bool) (hr : Admissible r)
    (l : List Item) (gs : List Enclosing) (h : admits l gs = true) :
    total r l gs = 1 := by
  have h74m := the_fix_only_refuses_more l gs h
  have hone := (QCore.ReturnFamilyDerefCast.admitted_is_one_dereference r hr l h74m).1
  have hout := admitted_has_no_outside_dereference r hr l gs h
  simp [total, hone, hout]

/-- `* ( * q + 1 )`: the base admitted a second dereference; it counts two and
the tip refuses it. -/
theorem old_admits_a_second_dereference :
    QCore.ReturnFamilyDerefCast.admits [.star .notOperandEnd, .deep] = true ∧
      total castReading [.star .notOperandEnd, .deep]
        [⟨some .notOperandEnd, .other⟩] = 2 ∧
      admits [.star .notOperandEnd, .deep] [⟨some .notOperandEnd, .other⟩] = false := by
  decide

/-- `( * q )`: one dereference, admitted as at the base. -/
theorem a_parenthesised_dereference_is_unchanged :
    admits [.star .notOperandEnd] [⟨none, .other⟩] = true := by
  decide

/-! ## The token reader's vectors -/

/-- `(spelling, the items before the may-bind `q` nearest first, the groups
enclosing it innermost first, admitted at the base, admitted)`. The Rust unit
test reads the spellings and the last field; the fourth field is pinned by
`old_vectors_hold`. -/
def outsideVectors : List (String × List Item × List Enclosing × Bool × Bool) := [
  ("* ( * q + 1 )", [.star .notOperandEnd, .deep], [⟨some .notOperandEnd, .other⟩],
    true, false),
  ("* ( 1 + * q )", [.star .notOperandEnd, .other], [⟨some .notOperandEnd, .other⟩],
    true, false),
  ("* ( n + * q )", [.star .notOperandEnd, .other], [⟨some .notOperandEnd, .other⟩],
    true, false),
  ("* ( * q - 1 )", [.star .notOperandEnd, .deep], [⟨some .notOperandEnd, .other⟩],
    true, false),
  ("* ( ( long * ) * q + 1 )", [.star .resolvedCast, .cast, .deep],
    [⟨some .notOperandEnd, .other⟩], true, false),
  ("* ( long * ) ( * q + 1 )", [.star .notOperandEnd, .cast, .deep],
    [⟨some .notOperandEnd, .other⟩], true, false),
  ("* ( T * ) ( * q + 1 )", [.star .notOperandEnd, .cast, .deep],
    [⟨some .notOperandEnd, .other⟩], true, false),
  ("* ( ( * q + 1 ) )", [.star .notOperandEnd, .deep],
    [⟨none, .other⟩, ⟨some .notOperandEnd, .other⟩], true, false),
  ("( * q + 1 ) [ 0 ]", [.star .notOperandEnd], [⟨none, .subscript⟩], true, false),
  ("( * q + 1 ) - > f", [.star .notOperandEnd], [⟨none, .arrow⟩], true, false),
  ("( a ) * ( n + * q )", [.star .notOperandEnd, .other],
    [⟨some (.close .undecided), .other⟩], true, false),
  ("* q", [.star .notOperandEnd], [], true, true),
  ("( * q )", [.star .notOperandEnd], [⟨none, .other⟩], true, true),
  ("( ( * q ) )", [.star .notOperandEnd], [⟨none, .other⟩, ⟨none, .other⟩], true, true),
  ("* q + 1", [.star .notOperandEnd], [], true, true),
  ("( * q + 1 )", [.star .notOperandEnd], [⟨none, .other⟩], true, true),
  ("n * ( * q + 1 )", [.star .notOperandEnd, .other], [⟨some .operandEnd, .other⟩],
    true, true),
  ("( long ) * q", [.star .resolvedCast, .cast], [], true, true),
  ("( long ) ( * q )", [.star .notOperandEnd, .cast], [⟨none, .other⟩], true, true),
  ("* ( long * ) * q", [.star .resolvedCast, .cast, .star .notOperandEnd], [], false, false)
]

def outsideVectorsAgree : Bool :=
  outsideVectors.all (fun (_, items, groups, _, admitted) => admits items groups == admitted)

def oldVectorsAgree : Bool :=
  outsideVectors.all (fun (_, items, _, old, _) =>
    QCore.ReturnFamilyDerefCast.admits items == old)

theorem outside_vectors_hold : outsideVectorsAgree = true := by decide

theorem old_vectors_hold : oldVectorsAgree = true := by decide

end QCore.ReturnFamilyStarOutside
