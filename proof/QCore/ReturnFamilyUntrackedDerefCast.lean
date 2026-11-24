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
# An untracked root dereferenced through a cast group is a base (card RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST, rule 74r)

Family S of `legacy_return_family` admits any use of an untracked root, but a
root that a `*` dereferences is a **base** of the read node, so the flow check
stands behind the syntactic premise: a possibly-null binding gives
`unknown_alias`. At the base the root was a starred base only when the token
right before it was a `*` that may be unary (`oldStarred`, rule 74h's union).
So `* ( long * ) p` made no base, and
`long w(void) { const long *p = 0; return *(long *)p; }` was `Safe`
(`old_misses_the_base`).

Rule 74r keeps that test and adds one that reads back from the root through
opening parentheses and through every group `cast_semantics::type_group_open`
reads as a type (rule 12h's transparency), to the first `*`, which counts when
it may be unary (`through`). The root is a base when either test says so
(`starred`). A word of a cast's type name (`long` in `* ( long * ) p`) is no
root, so the new test does not apply to it (`typeWord`, the reader's
`cast_type_word`; `a_type_word_keeps_the_old_reading`).

* `oldStarred` implies `starred`: every base the base read is kept
  (`old_starred_kept`), and family S refuses exactly where it refused, since
  the change touches no refusal.
* A base only adds an obligation, so a result the tip admits the base admitted
  too (`the_fix_only_refuses_more`): no reading admits more than at the base.
* Under every reading C may take of rule 74h's undecided groups, a root some
  reading dereferences through the groups is a base
  (`starred_keeps_every_reading_base`).
* A `*` that is binary under every reading (`n * ( long ) p`) still makes no
  base (`a_multiplication_is_no_base`).

## What this is not

A model of the token scan or of the flow check. Each item's class is the Rust
reader's output (trust boundary 28), bound by `untrackedVectors`; a base's
obligation is abstracted as a predicate `ok` that the flow check decides.
-/

namespace QCore.ReturnFamilyUntrackedDerefCast

open QCore.ReturnFamilyStarCast (Before Admissible operandReading castReading cast_above)
open QCore.CastBeforeAmpersand (Group)

/-- One token step back from the untracked root, nearest first. -/
inductive Item where
  /-- A `*`, classed by the token before it. -/
  | star (b : Before)
  /-- A `(`. -/
  | opening
  /-- A `)` closing a group `type_group_open` reads as a type. -/
  | cast
  /-- Anything else: the reading stops. -/
  | other
  deriving DecidableEq, Repr

/-- The base's starred test: the token right before the root is a `*` that
may be unary (rule 74h's union, `may_be_unary`). -/
def oldStarred : List Item → Bool
  | .star b :: _ => castReading b
  | _ => false

/-- The reading through parentheses and type groups under one reading `r` of
the undecided operators. -/
def throughUnder (r : Before → Bool) : List Item → Bool
  | .star b :: _ => r b
  | .opening :: rest => throughUnder r rest
  | .cast :: rest => throughUnder r rest
  | _ => false

/-- Rule 74r's reading: the first `*` through the groups, when it may be
unary. -/
def through (l : List Item) : Bool := throughUnder castReading l

/-- Rule 74r's starred test: either reading; the new one only for a root
that is no word of a cast's type name. -/
def starred (typeWord : Bool) (l : List Item) : Bool :=
  oldStarred l || (!typeWord && through l)

theorem old_starred_kept (typeWord : Bool) (l : List Item) (h : oldStarred l = true) :
    starred typeWord l = true := by
  simp [starred, h]

/-- A word of a cast's type name keeps the base's reading exactly. -/
theorem a_type_word_keeps_the_old_reading (l : List Item) : starred true l = oldStarred l := by
  simp [starred]

/-- The old test is the first step of the new one, so for a root the union is
the new reading alone. -/
theorem starred_is_through (l : List Item) : starred false l = through l := by
  cases l with
  | nil => rfl
  | cons i rest => cases i <;> simp [starred, oldStarred, through, throughUnder]

theorem through_mono (p q : Before → Bool) (h : ∀ b, p b = true → q b = true) :
    ∀ l : List Item, throughUnder p l = true → throughUnder q l = true
  | [] => by simp [throughUnder]
  | .star b :: _ => by simpa [throughUnder] using h b
  | .opening :: rest => by simpa [throughUnder] using through_mono p q h rest
  | .cast :: rest => by simpa [throughUnder] using through_mono p q h rest
  | .other :: _ => by simp [throughUnder]

/-- Every reading C may take of the undecided groups that dereferences the
root through the groups makes it a base. -/
theorem starred_keeps_every_reading_base (r : Before → Bool) (hr : Admissible r)
    (l : List Item) (h : throughUnder r l = true) : starred false l = true := by
  rw [starred_is_through]
  exact through_mono r castReading (fun b hb => cast_above r hr b hb) l h

/-! ## Bases only add obligations -/

/-- The bases of the untracked names under a starred test; each name carries
whether it is a type word and the items before it. -/
def basesOf (test : Bool → List Item → Bool) (roots : List (String × Bool × List Item)) :
    List String :=
  (roots.filter (fun root => test root.2.1 root.2.2)).map Prod.fst

theorem old_bases_kept (roots : List (String × Bool × List Item)) (name : String)
    (h : name ∈ basesOf (fun _ => oldStarred) roots) : name ∈ basesOf starred roots := by
  simp only [basesOf, List.mem_map, List.mem_filter] at h ⊢
  obtain ⟨root, ⟨hm, ht⟩, he⟩ := h
  exact ⟨root, ⟨hm, old_starred_kept _ _ ht⟩, he⟩

/-- Family S's result over one expression: `none` refuses; otherwise the
flow check admits only when every base's obligation `ok` discharges. -/
def admits (ok : String → Bool) : Option (List String) → Bool
  | none => false
  | some bases => bases.all ok

/-- The change only adds bases and touches no refusal, so a result the tip
admits the base admitted too. -/
theorem the_fix_only_refuses_more (ok : String → Bool) (refused : Bool)
    (roots : List (String × Bool × List Item))
    (h : admits ok (if refused then none else some (basesOf starred roots)) = true) :
    admits ok (if refused then none else some (basesOf (fun _ => oldStarred) roots)) = true := by
  cases refused with
  | true => simp [admits] at h
  | false =>
    simp only [admits, List.all_eq_true, Bool.false_eq_true, ite_false] at h ⊢
    exact fun name hn => h name (old_bases_kept roots name hn)

/-! ## The readings -/

/-- `* ( long * ) p`: the base missed the dereference; the tip makes `p` a
base, and not a type word in the same place. -/
theorem old_misses_the_base :
    oldStarred [.cast, .star .notOperandEnd] = false ∧
      starred false [.cast, .star .notOperandEnd] = true ∧
      starred true [.cast, .star .notOperandEnd] = false := by
  decide

/-- `n * ( long ) p`: a multiplication of the converted value is no
dereference. -/
theorem a_multiplication_is_no_base :
    starred false [.cast, .star .operandEnd] = false := by
  decide

/-- `( a ) * ( long ) p` with `( a )` undecided: the cast reading
dereferences, so the root is a base, as rule 74h's union asks. -/
theorem an_undecided_star_through_a_cast_is_a_base :
    oldStarred [.cast, .star (.close .undecided)] = false ∧
      starred false [.cast, .star (.close .undecided)] = true ∧
      throughUnder operandReading [.cast, .star (.close .undecided)] = false := by
  decide

/-! ## The token reader's vectors -/

/-- `(spelling, the items before the untracked `p` nearest first, a base at the
base, a base)`. The Rust unit test reads the spellings and the last field;
the third field is pinned by `old_vectors_hold`. -/
def untrackedVectors : List (String × List Item × Bool × Bool) := [
  ("* p", [.star .notOperandEnd], true, true),
  ("* ( long * ) p", [.cast, .star .notOperandEnd], false, true),
  ("* ( long * ) ( char * ) p", [.cast, .cast, .star .notOperandEnd], false, true),
  ("* ( ( long * ) p )", [.cast, .opening, .star .notOperandEnd], false, true),
  ("* ( long * ) ( p )", [.opening, .cast, .star .notOperandEnd], false, true),
  ("* ( p )", [.opening, .star .notOperandEnd], false, true),
  ("* ( p + 1 )", [.opening, .star .notOperandEnd], false, true),
  ("* ( T * ) p", [.cast, .star .notOperandEnd], false, true),
  ("( int ) * ( long * ) p", [.cast, .star .resolvedCast], false, true),
  ("( a ) * ( long ) p", [.cast, .star (.close .undecided)], false, true),
  ("* ( long * ) * p", [.star .resolvedCast, .cast, .star .notOperandEnd], true, true),
  ("( long ) * p", [.star .resolvedCast], true, true),
  ("( T ) * p", [.star (.close .undecided)], true, true),
  ("sizeof ( long ) * p", [.star .resolvedCast], true, true),
  ("n * ( long ) p", [.cast, .star .operandEnd], false, false),
  ("( n ) * ( long ) p", [.cast, .star (.close .object)], false, false),
  ("* ( long * const ) p", [.cast, .star .notOperandEnd], false, true),
  ("( long ) p", [.cast], false, false)
]

def untrackedVectorsAgree : Bool :=
  untrackedVectors.all (fun (_, items, _, base) => starred false items == base)

def oldVectorsAgree : Bool :=
  untrackedVectors.all (fun (_, items, old, _) => oldStarred items == old)

theorem untracked_vectors_hold : untrackedVectorsAgree = true := by decide

theorem old_vectors_hold : oldVectorsAgree = true := by decide

end QCore.ReturnFamilyUntrackedDerefCast
