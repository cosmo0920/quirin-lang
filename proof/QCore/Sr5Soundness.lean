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

import QCore.VerdictSemigroup

/-!
# The refusing repairs of card W45-SR5-SOUNDNESS

STABILITY-REEVAL-5 registered findings whose one-off probe this card measured
as admitting (`Safe` where it should not be) or as a lowered verdict
(`Pending` where `Unknown` is due). The interprocedural member release is
`QCore.MemberReleaseCallee` and the reserved prefix `QCore.ReservedSpelling`;
this module carries the other five repairs. Each only refuses.

## What is proved

* **A released container root discharges no publication**
  (`PUBLICATION-ROOT-RELEASED`, gateway 10d): a publication discharges only into
  a root the body never hands to a non-`Borrowed` position, and a local
  initialised from the formal alone shares its release.
* **An element bound over a formal publishes the caller's object**
  (`CONTAINER-OF-FORMAL-PUBLICATION-LAUNDER`, 10d with 5l(2)): it takes the
  formal entry's rule, an unresolved identity, never the borrowed reading.
* **A counted-user event C may skip, or a call through a call's result,
  undecides the counts** (`COUNTED-USER-CONDITIONAL-OPERAND`, 10e): no clause
  holds over it; and **a declared neutrality of a body defined here applies
  only when the body verifies it** (`DECLARED-NEUTRAL-UNVERIFIED-AT-CALL`).
* **A scalar-result read admits exactly one dereference of a name the body may
  bind**, counting stars through parentheses
  (`SCALAR-READ-PARENTHESISED-DOUBLE-DEREF`, 5j).
* **A unit either scope gate withholds from `Safe` is `Unknown`, never
  `Pending`** (`PENDING-BYPASSES-UNIT-SAFE-GATES`, §L21.1).

## What this is not

The lexical readers that decide the inputs (which positions are `Borrowed`,
which tokens C may skip, how many stars apply) are trust boundary 28.
-/

namespace QCore.Sr5Soundness

/-! ## A released container root (10d) -/

/-- The container facts of one body: the stable roots (unrebound formals and
locals derived from them), the aliases `L = P`, and the names the body hands
whole to a position whose contract is not `Borrowed`. -/
structure Roots where
  stable : List Nat
  aliases : List (Nat × Nat)
  handed : List Nat

/-- A root is released when it, or the other name of its object, is handed on. -/
def released (r : Roots) (root : Nat) : Bool :=
  r.handed.contains root
    || r.aliases.any (fun (l, p) => (l == root && r.handed.contains p)
      || (p == root && r.handed.contains l))

/-- A publication into a container reached from `root` discharges only into a
stable root the body does not release. -/
def discharges (r : Roots) (root : Nat) : Bool := r.stable.contains root && !released r root

/-- **A released root discharges no publication.** -/
theorem a_released_root_discharges_nothing (r : Roots) (root : Nat) (h : released r root = true) :
    discharges r root = false := by
  simp [discharges, h]

/-- The release of a formal reaches a local initialised from it alone. -/
theorem an_alias_shares_the_release (r : Roots) (l p : Nat) (ha : (l, p) ∈ r.aliases)
    (hp : r.handed.contains p = true) : released r l = true := by
  have hp' : p ∈ r.handed := by simpa using hp
  simp only [released, Bool.or_eq_true, List.any_eq_true]
  right
  exact ⟨(l, p), ha, by simp [hp']⟩

/-- The measured row: `mk_list_add(&e->_head, &q->items); q_free(q);`. -/
theorem the_measured_root_release_discharges_nothing :
    discharges { stable := [0], aliases := [], handed := [0] } 0 = false := by decide

/-- Its control, the publication alone, still discharges. -/
theorem the_publication_alone_still_discharges :
    discharges { stable := [0], aliases := [], handed := [] } 0 = true := by decide

/-! ## An element bound over a formal (10d with 5l(2)) -/

/-- What a publication does with an entry that denotes no tracked identity. -/
inductive EntryOutcome where
  | unresolvedIdentity
  | borrowedReading
  deriving DecidableEq, Repr

/-- A formal's entry, or an element rule 5l(2) bound over a formal, reports an
unresolved identity; any other untracked entry keeps the borrowed reading. -/
def untrackedEntry (formal overFormal : Bool) : EntryOutcome :=
  if formal || overFormal then .unresolvedIdentity else .borrowedReading

/-- **An element bound over a formal is never laundered as borrowed.** -/
theorem an_element_over_a_formal_is_never_borrowed (formal : Bool) :
    untrackedEntry formal true = .unresolvedIdentity := by
  cases formal <;> rfl

/-! ## Counted users (10e) -/

/-- A tracked count: known, or undecided. -/
inductive Count where
  | known (n : Int)
  | undecided
  deriving DecidableEq, Repr

/-- One event of an expression: a call or a counter write, with the step it
applies, whether C may skip it (behind `?:`, `&&`, `||`), and whether its
callee is opaque (a call through a call's result, `f(a)(b)`). -/
structure Event where
  step : Int
  skippable : Bool
  throughResult : Bool

/-- The walker's transfer: an event C may skip, or an opaque one, undecides. -/
def apply (c : Count) (e : Event) : Count :=
  match c with
  | .undecided => .undecided
  | .known n => if e.skippable || e.throughResult then .undecided else .known (n + e.step)

def run (c : Count) (es : List Event) : Count := es.foldl apply c

theorem run_undecided (es : List Event) : run .undecided es = .undecided := by
  induction es with
  | nil => rfl
  | cons e es ih => simpa [run, List.foldl, apply] using ih

/-- A clause holds only over a known count equal to its value. -/
def holds (c : Count) (value : Int) : Bool :=
  match c with
  | .known n => n == value
  | .undecided => false

/-- **No clause holds over a skippable or opaque event.** -/
theorem a_skippable_or_opaque_event_holds_no_clause (c : Count) (pre post : List Event)
    (e : Event) (h : e.skippable = true ∨ e.throughResult = true) (value : Int) :
    holds (run c (pre ++ e :: post)) value = false := by
  have hmid : apply (run c pre) e = .undecided := by
    cases hc : run c pre with
    | undecided => rfl
    | known n => rcases h with h | h <;> simp [apply, h]
  simp only [run, List.foldl_append, List.foldl_cons] at *
  rw [hmid]
  have := run_undecided post
  simp only [run] at this
  rw [this]
  rfl

/-- The measured row: `u_ref(t); c ? u_unref(t, 0) : (void)0; return 0;`
under `restored` held before the repair (the drop applied on every path). -/
theorem the_measured_conditional_row_holds_no_clause :
    holds (run (.known 0) [⟨1, false, false⟩, ⟨-1, true, false⟩]) 0 = false := by decide

/-- A call's counted-user effect as far as neutrality goes. -/
inductive CallEffect where
  | neutral
  | undecides
  deriving DecidableEq, Repr

/-- A declared-neutral callee: with no body here the declaration governs
(UC-FRAME); with a body here it applies only when the body verifies it. -/
def neutralEffect (bodyHere verified : Bool) : CallEffect :=
  if !bodyHere || verified then .neutral else .undecides

/-- **An unverified declared neutrality of a body defined here is opaque.** -/
theorem an_unverified_neutral_body_is_opaque : neutralEffect true false = .undecides := rfl

/-- A verified one, or one with no body here, is neutral. -/
theorem a_verified_or_bodiless_neutral_is_neutral (bodyHere : Bool) :
    neutralEffect bodyHere true = .neutral ∧ neutralEffect false false = .neutral := by
  cases bodyHere <;> exact ⟨rfl, rfl⟩

/-! ## A scalar-result read (5j) -/

/-- One read of a name the body may bind: the unary `*`s applied to it
(through parentheses), whether it is followed by one `->` or `[…]`, and
whether a further `->`, `[` or call follows once parentheses close. -/
structure Read where
  stars : Nat
  selected : Bool
  continues : Bool

/-- The number of dereferences the read performs. -/
def dereferences (r : Read) : Nat :=
  r.stars + (if r.selected then 1 else 0) + (if r.continues then 1 else 0)

/-- The reader's admission, as repaired. -/
def admits (r : Read) : Bool :=
  !r.continues && ((r.selected && r.stars == 0) || (!r.selected && r.stars == 1))

/-- **An admitted read performs exactly one dereference.** -/
theorem an_admitted_read_dereferences_once (r : Read) (h : admits r = true) :
    dereferences r = 1 := by
  obtain ⟨stars, selected, continues⟩ := r
  cases selected <;> cases continues <;> simp_all [admits, dereferences]

/-- The measured shapes `(*q)->len`, `(*q)[0]` and `**q` are refused. -/
theorem the_parenthesised_shapes_are_refused :
    admits ⟨1, false, true⟩ = false ∧ admits ⟨2, false, false⟩ = false
      ∧ admits ⟨0, true, true⟩ = false := by decide

/-! ## A `Pending` unit and the `Safe` gates (§L21.1) -/

open QCore.VerdictSemigroup

/-- The unit verdict: the fold of its functions, joined with `Unknown` when a
scope gate (an unread selected definition, an unestablished certificate
target, an inventory that withholds `Safe`) refuses. -/
def unitVerdict (functions : V) (gateRefuses : Bool) : V :=
  if gateRefuses then V.join functions .unknown else functions

/-- **A unit a scope gate refuses is neither `Safe` nor `Pending`.** -/
theorem a_gated_unit_is_never_pending (functions : V) :
    unitVerdict functions true ≠ .pending ∧ unitVerdict functions true ≠ .safe := by
  cases functions <;> decide

/-- A `Pending` unit passed every scope gate. -/
theorem a_pending_unit_passed_its_gates (functions : V) (gateRefuses : Bool)
    (h : unitVerdict functions gateRefuses = .pending) : gateRefuses = false := by
  cases gateRefuses with
  | false => rfl
  | true => exact absurd h (a_gated_unit_is_never_pending functions).1

/-- The measured row: a held call beside an `#if 0` definition was `Pending`. -/
theorem the_measured_gated_row_is_unknown : unitVerdict .pending true = .unknown := by decide

end QCore.Sr5Soundness
