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

import QCore.ReturnFamilyUntrackedArithAccess

/-!
# Rule 74w's walk stops at a conditional's arm (card RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL, rule 82)

Family S of `legacy_return_family` refuses an expression when rule 74w's
walk accesses an untracked root through an additive group it cannot
classify (`Outcome.unclassified`). At the base the walk read a parenthesised
conditional as the additive group around a later term of an arm's sum: in
`( c ? 1 + p - n : a ) - > m` and `( c ? a : n + p ) [ 0 ]` it crossed the
conditional's parentheses, found `?` among the other operands and refused
(`old_refuses_the_arm`), although rule 74x reads that arm.

Rule 82 keeps the walk's path and its old result, and reads each crossed
group a second way: a conditional group whose arm holds the span is
classified **within the arm**, the arm's `?` or `:` standing in for the
parentheses (`Crossing.arm`); a root in a condition, or a group with no
top-level `?`, has no arm reading. When the old result is `unclassified`,
some crossing was an arm (`delimited`) and every crossing is classified
under the arm reading (`inArms`), the walk returns `Walk.arm`, and family S
refuses unless rule 74x made the root a base (`Root.condBase`).

* **The only refusal removed** is that one, and the root it covered is a
  rule 74x base (`the_only_refusal_removed`, `an_arm_was_unclassified`).
  No refusal is added (`no_new_refusal`).
* **Everything else stays unclassified**: a root in a condition and any
  other unclassified group keep the refusal
  (`a_condition_keeps_its_refusal`, `another_group_keeps_its_refusal`).
* **Fail closed.** Every old base and `->` root is kept (`old_base_kept`,
  `old_arrow_kept`); where the base admitted, the tip admits only what the
  base admitted (`where_the_base_admitted_nothing_admits_more`); where it
  did not, every relaxed root is a discharged base
  (`a_relaxed_root_is_discharged`). And nothing admits more than a reading
  in which both rules read the arm, 74w classifying it as a sum term and
  74x making its roots bases (`nothing_admits_more_than_both`).

## What this is not

A model of the token scan or of the flow check. Each crossing's two
classifications, the context and rule 74x's verdict on the root are the
Rust reader's output (trust boundary 28), bound by `walkVectors`, which the
Rust test checks against `accessed_through_arithmetic` and `scalar_read`.
Obligations are abstracted as predicates the flow check decides.
-/

namespace QCore.ReturnFamilyArithWalkConditional

open QCore.ReturnFamilyUntrackedArithAccess (Layer Ctx Outcome arith)

/-- One additive group rule 74w's walk crosses, innermost first: its
classification as a whole group (`group`, the base's), and, when it is a
conditional whose arm holds the span, its classification within that arm. -/
structure Crossing where
  group : Bool
  arm : Option Bool
  deriving DecidableEq, Repr

def Crossing.layer (c : Crossing) : Layer := .sum c.group

/-- The crossing's classification with an arm's delimiters standing in for a
conditional group's parentheses. -/
def Crossing.inArm (c : Crossing) : Bool := c.arm.getD c.group

def layers (cs : List Crossing) : List Layer := cs.map Crossing.layer

/-- Some crossed group was an arm's conditional. -/
def delimited (cs : List Crossing) : Bool := cs.any (fun c => c.arm.isSome)

/-- Every crossed group is classified under the arm reading. -/
def inArms (cs : List Crossing) : Bool := cs.all Crossing.inArm

/-- The tip's walk: the old outcome, or `arm` (with whether `->` accesses). -/
inductive Walk where
  | old (o : Outcome)
  | arm (arrow : Bool)
  deriving DecidableEq, Repr

/-- The relaxed case: the old refusal, caused only by the arm delimiter. -/
def relaxed (cs : List Crossing) (ctx : Ctx) : Bool :=
  arith (layers cs) ctx == .unclassified && delimited cs && inArms cs

/-- `accessed_through_arithmetic` at the tip. -/
def walkOf (cs : List Crossing) (ctx : Ctx) : Walk :=
  if relaxed cs ctx then .arm (ctx == .arrow) else .old (arith (layers cs) ctx)

theorem walk_off_the_arm (cs : List Crossing) (ctx : Ctx) (h : delimited cs = false) :
    walkOf cs ctx = .old (arith (layers cs) ctx) := by
  simp [walkOf, relaxed, h]

theorem an_arm_was_unclassified (cs : List Crossing) (ctx : Ctx) (a : Bool)
    (h : walkOf cs ctx = .arm a) :
    arith (layers cs) ctx = .unclassified ∧ delimited cs = true ∧ inArms cs = true := by
  unfold walkOf at h
  cases hr : relaxed cs ctx with
  | false => simp [hr] at h
  | true =>
    simp only [relaxed, Bool.and_eq_true, beq_iff_eq] at hr
    exact ⟨hr.1.1, hr.1.2, hr.2⟩

theorem a_condition_keeps_its_refusal (cs : List Crossing) (ctx : Ctx) (a : Bool)
    (c : Crossing) (hc : c ∈ cs) (hn : c.arm = none) (hg : c.group = false) :
    walkOf cs ctx ≠ .arm a := by
  intro h
  have := (an_arm_was_unclassified cs ctx a h).2.2
  simp only [inArms, List.all_eq_true] at this
  have := this c hc
  simp [Crossing.inArm, hn, hg] at this

theorem another_group_keeps_its_refusal (cs : List Crossing) (ctx : Ctx) (a : Bool)
    (c : Crossing) (hc : c ∈ cs) (hn : c.arm = some false) :
    walkOf cs ctx ≠ .arm a := by
  intro h
  have := (an_arm_was_unclassified cs ctx a h).2.2
  simp only [inArms, List.all_eq_true] at this
  have := this c hc
  simp [Crossing.inArm, hn] at this

/-! ## One root -/

/-- One untracked, non-scalar root: its crossings and context, the readings
that do not change (the direct tests and rules 74r's and 74t's walks), and
rule 74x's (`conditional_roots`). -/
structure Root where
  name : String
  crossings : List Crossing
  ctx : Ctx
  direct : Bool
  directArrow : Bool
  condBase : Bool
  condArrow : Bool
  deriving DecidableEq, Repr

def Root.old (r : Root) : Outcome := arith (layers r.crossings) r.ctx
def Root.walk (r : Root) : Walk := walkOf r.crossings r.ctx

/-- The base's reading of the root (rules 74w and 74x). -/
def oldRefuses (r : Root) : Bool := r.old == .unclassified
def oldBase (r : Root) : Bool := r.direct || r.old != .none || r.condBase
def oldArrow (r : Root) : Bool := r.directArrow || r.old == .arrow || r.condArrow

/-- The tip's: an arm refuses unless rule 74x made the root a base. -/
def newRefuses (r : Root) : Bool :=
  match r.walk with
  | .old o => o == .unclassified
  | .arm _ => !r.condBase
def newBase (r : Root) : Bool := r.direct || r.walk != .old .none || r.condBase
def newArrow (r : Root) : Bool :=
  r.directArrow || r.walk == .old .arrow || r.walk == .arm true || r.condArrow

/-- Both rules reading the arm: rule 74w classifies the arm's sum term (an
access of the root), and rule 74x makes its roots bases. -/
def Root.both (r : Root) : Outcome :=
  if relaxed r.crossings r.ctx then (if r.ctx == .arrow then .arrow else .element) else r.old
def bothRefuses (r : Root) : Bool := r.both == .unclassified
def bothBase (r : Root) : Bool := r.direct || r.both != .none || r.condBase
def bothArrow (r : Root) : Bool := r.directArrow || r.both == .arrow || r.condArrow

theorem old_refuses_the_arm (r : Root) (a : Bool) (h : r.walk = .arm a) : oldRefuses r = true := by
  simp [oldRefuses, Root.old, (an_arm_was_unclassified _ _ a h).1]

theorem no_new_refusal (r : Root) (h : newRefuses r = true) : oldRefuses r = true := by
  unfold newRefuses at h
  cases hw : r.walk with
  | old o =>
    simp only [hw, beq_iff_eq] at h
    unfold Root.walk walkOf at hw
    split at hw
    · cases hw
    · cases hw; simp [oldRefuses, Root.old, h]
  | arm a => exact old_refuses_the_arm r a hw

/-- The only refusal the tip removes is rule 74w's arm-delimited one, and
the root it covered is a rule 74x base. -/
theorem the_only_refusal_removed (r : Root) (ho : oldRefuses r = true)
    (hn : newRefuses r = false) : (∃ a, r.walk = .arm a) ∧ r.condBase = true := by
  unfold newRefuses at hn
  cases hw : r.walk with
  | old o =>
    simp only [hw, beq_eq_false_iff_ne, ne_eq] at hn
    unfold Root.walk walkOf at hw
    split at hw
    · cases hw
    · cases hw; simp [oldRefuses, Root.old] at ho; exact absurd ho hn
  | arm a =>
    simp only [hw, Bool.not_eq_false'] at hn
    exact ⟨⟨a, rfl⟩, hn⟩

theorem old_base_kept (r : Root) (h : oldBase r = true) : newBase r = true := by
  simp only [oldBase, newBase, Bool.or_eq_true, bne_iff_ne, ne_eq] at h ⊢
  rcases h with (h | h) | h
  · exact Or.inl (Or.inl h)
  · refine Or.inl (Or.inr ?_)
    unfold Root.walk walkOf
    split
    · simp
    · simpa [Root.old] using h
  · exact Or.inr h

theorem old_arrow_kept (r : Root) (h : oldArrow r = true) : newArrow r = true := by
  simp only [oldArrow, newArrow, Bool.or_eq_true, beq_iff_eq] at h ⊢
  rcases h with (h | h) | h
  · exact Or.inl (Or.inl (Or.inl h))
  · refine Or.inl (Or.inl (Or.inr ?_))
    have ho : arith (layers r.crossings) r.ctx = .arrow := h
    have hr : relaxed r.crossings r.ctx = false := by simp [relaxed, ho]
    show walkOf r.crossings r.ctx = .old .arrow
    simp only [walkOf, hr, Bool.false_eq_true, ite_false]
    exact congrArg Walk.old ho
  · exact Or.inr h

theorem both_refusal_kept (r : Root) (h : bothRefuses r = true) : newRefuses r = true := by
  unfold bothRefuses Root.both at h
  unfold newRefuses Root.walk walkOf
  cases hr : relaxed r.crossings r.ctx with
  | true => simp only [hr, ite_true] at h; split at h <;> simp at h
  | false => simpa [hr, Root.old] using h

theorem both_base_kept (r : Root) (h : bothBase r = true) : newBase r = true := by
  simp only [bothBase, newBase, Bool.or_eq_true, bne_iff_ne, ne_eq] at h ⊢
  rcases h with (h | h) | h
  · exact Or.inl (Or.inl h)
  · refine Or.inl (Or.inr ?_)
    unfold Root.both at h
    unfold Root.walk walkOf
    cases hr : relaxed r.crossings r.ctx with
    | true => simp
    | false => simpa [hr, Root.old] using h
  · exact Or.inr h

theorem both_arrow_kept (r : Root) (h : bothArrow r = true) : newArrow r = true := by
  simp only [bothArrow, newArrow, Bool.or_eq_true, beq_iff_eq] at h ⊢
  rcases h with (h | h) | h
  · exact Or.inl (Or.inl (Or.inl h))
  · unfold Root.both at h
    cases hr : relaxed r.crossings r.ctx with
    | true =>
      refine Or.inl (Or.inr ?_)
      have hc : r.ctx = .arrow := by
        cases hc : r.ctx <;> simp_all
      show walkOf r.crossings r.ctx = .arm true
      simp only [walkOf, hr, ite_true]
      simp [hc]
    | false =>
      refine Or.inl (Or.inl (Or.inr ?_))
      have ho : arith (layers r.crossings) r.ctx = .arrow := by
        simpa [hr, Root.old] using h
      show walkOf r.crossings r.ctx = .old .arrow
      simp only [walkOf, hr, Bool.false_eq_true, ite_false]
      exact congrArg Walk.old ho
  · exact Or.inr h

/-! ## The read -/

/-- Family S's result over the roots: `none` refuses; else the bases and the
`->` roots. `refused` is every other refusal (rule 74x's, a call, ...),
which this card does not touch. -/
def read (refuses base arrow : Root → Bool) (refused : Bool) (roots : List Root) :
    Option (List String × List String) :=
  if refused || roots.any refuses then none
  else some ((roots.filter base).map (·.name), (roots.filter arrow).map (·.name))

def oldRead := read oldRefuses oldBase oldArrow
def newRead := read newRefuses newBase newArrow
def bothRead := read bothRefuses bothBase bothArrow

/-- The flow check admits only when every base's obligation `ok` and every
`->` root's C02-P-ALIAS obligation `alias` discharge. -/
def admits (ok alias : String → Bool) : Option (List String × List String) → Bool
  | none => false
  | some (bases, arrows) => bases.all ok && arrows.all alias

theorem names_kept (old new : Root → Bool) (h : ∀ r, old r = true → new r = true)
    (roots : List Root) (name : String)
    (hn : name ∈ (roots.filter old).map (·.name)) : name ∈ (roots.filter new).map (·.name) := by
  simp only [List.mem_map, List.mem_filter] at hn ⊢
  obtain ⟨r, ⟨hm, ho⟩, he⟩ := hn
  exact ⟨r, ⟨hm, h r ho⟩, he⟩

/-- A reading whose refusals, bases and `->` roots the tip's include admits
whatever the tip admits. -/
theorem admits_weaker (ok alias : String → Bool) (refused : Bool) (roots : List Root)
    (rf bs ar : Root → Bool)
    (hrf : ∀ r ∈ roots, rf r = true → newRefuses r = true)
    (hbs : ∀ r, bs r = true → newBase r = true)
    (har : ∀ r, ar r = true → newArrow r = true)
    (h : admits ok alias (newRead refused roots) = true) :
    admits ok alias (read rf bs ar refused roots) = true := by
  unfold newRead read at h
  unfold read
  cases hr : refused with
  | true => simp [hr, admits] at h
  | false =>
    cases hn : roots.any newRefuses with
    | true => simp [hr, hn, admits] at h
    | false =>
      have ho : roots.any rf = false := by
        rw [List.any_eq_false] at hn ⊢
        intro r hm hrr
        exact hn r hm (hrf r hm hrr)
      simp only [hr, hn, ho, Bool.or_false, Bool.false_eq_true, ite_false, admits,
        Bool.and_eq_true, List.all_eq_true] at h ⊢
      obtain ⟨hb, hw⟩ := h
      exact ⟨fun name hm => hb name (names_kept _ _ hbs roots name hm),
        fun name hm => hw name (names_kept _ _ har roots name hm)⟩

/-- Where the base admitted the expression's shape (no root refused), the
tip admits only what the base admitted: its obligations include the base's. -/
theorem where_the_base_admitted_nothing_admits_more (ok alias : String → Bool) (refused : Bool)
    (roots : List Root) (hb : roots.any oldRefuses = false)
    (h : admits ok alias (newRead refused roots) = true) :
    admits ok alias (oldRead refused roots) = true := by
  refine admits_weaker ok alias refused roots _ _ _ ?_ old_base_kept old_arrow_kept h
  intro r hm hr
  rw [List.any_eq_false] at hb
  exact absurd hr (hb r hm)

/-- Nothing admits more than a reading in which both rules read the arm. -/
theorem nothing_admits_more_than_both (ok alias : String → Bool) (refused : Bool)
    (roots : List Root) (h : admits ok alias (newRead refused roots) = true) :
    admits ok alias (bothRead refused roots) = true :=
  admits_weaker ok alias refused roots _ _ _ (fun r _ => both_refusal_kept r)
    both_base_kept both_arrow_kept h

/-- Where the tip admits an expression the base refused for a root, that
root's refusal was the arm's, it is a rule 74x base, and its obligation
discharged: no access is left unobligated. -/
theorem a_relaxed_root_is_discharged (ok alias : String → Bool) (refused : Bool)
    (roots : List Root) (h : admits ok alias (newRead refused roots) = true)
    (r : Root) (hm : r ∈ roots) (ho : oldRefuses r = true) :
    (∃ a, r.walk = .arm a) ∧ r.condBase = true ∧ ok r.name = true := by
  unfold newRead read at h
  cases hr : refused with
  | true => simp [hr, admits] at h
  | false =>
    cases hn : roots.any newRefuses with
    | true => simp [hr, hn, admits] at h
    | false =>
      have hnr : newRefuses r = false := by
        rw [List.any_eq_false] at hn
        simpa using hn r hm
      obtain ⟨harm, hc⟩ := the_only_refusal_removed r ho hnr
      simp only [hr, hn, Bool.or_false, Bool.false_eq_true, ite_false, admits,
        Bool.and_eq_true, List.all_eq_true] at h
      refine ⟨harm, hc, h.1 r.name ?_⟩
      simp only [List.mem_map, List.mem_filter]
      exact ⟨r, ⟨hm, by simp [newBase, hc]⟩, rfl⟩

/-! ## The readings -/

/-- `( c ? 1 + p - n : a ) - > m`: the base refused; the tip makes `p` a base
and a `->` root through rule 74x. -/
theorem the_arm_is_read :
    let r : Root := ⟨"p", [⟨false, some true⟩], .arrow, false, false, true, true⟩
    oldRefuses r = true ∧ newRefuses r = false ∧ newBase r = true ∧ newArrow r = true := by
  decide

/-- `( 1 + p - n ? a : b ) - > m`: `p` is the condition; the refusal stays. -/
theorem the_condition_still_refuses :
    newRefuses ⟨"p", [⟨false, none⟩], .arrow, false, false, false, false⟩ = true := by
  decide

/-- `( c ? n - p + 1 : a ) - > m`: rule 74w's arm reading classifies the
term, but rule 74x refuses a subtracted pointer, so `p` is no rule 74x
base and the refusal stays. -/
theorem an_arm_74x_refuses_still_refuses :
    newRefuses ⟨"p", [⟨false, some true⟩], .arrow, false, false, false, false⟩ = true := by
  decide

/-! ## The token reader's vectors -/

/-- `(spelling, crossings, context, direct, direct ->, 74x base, 74x ->,
old refusal, arm, base, -> root, refused)` for the first untracked `p`;
`refused` is family S's result, every other refusal included (`other`).
The Rust unit test reads the spelling and the last five fields, checking
`old refusal` and `arm` against `accessed_through_arithmetic` and the last
three against `scalar_read`. -/
def walkVectors :
    List (String × List Crossing × Ctx × Bool × Bool × Bool × Bool × Bool ×
      Bool × Bool × Bool × Bool × Bool) := [
  ("( n ? 1 + p - n : a ) - > m", [⟨false, some true⟩], .arrow, false, false, true, true, false,
    true, true, true, true, false),
  ("( n ? a : n + p ) [ 0 ]", [⟨false, some true⟩], .element, false, false, true, false, false,
    true, true, true, false, false),
  ("* ( n ? a : n + p - 1 )", [⟨false, some true⟩], .element, false, false, true, false, false,
    true, true, true, false, false),
  ("( ( n ? 1 + p - n : a ) + 1 ) - > m", [⟨false, some true⟩, ⟨true, none⟩], .arrow,
    false, false, true, true, false, true, true, true, true, false),
  ("( n ? 1 + ( long * ) p - n : a ) [ 0 ]", [⟨false, some true⟩], .element,
    false, false, true, false, false, true, true, true, false, false),
  ("( n ? a : n > 1 ? b : 1 + p - n ) - > m", [⟨false, some true⟩], .arrow,
    false, false, true, true, false, true, true, true, true, false),
  ("( n ? 1 + p - n : a ) - > m + p - > m", [⟨false, some true⟩], .arrow,
    false, false, true, true, false, true, true, true, true, false),
  ("( n ? 1 + p - n : a ) != 0", [⟨false, some true⟩], .none, false, false, false, false, false,
    false, false, false, false, false),
  ("( 1 + p - n ? a : b ) - > m", [⟨false, none⟩], .arrow, false, false, false, false, false,
    true, false, true, false, true),
  ("( n ? n > 1 + p - n ? a : b : b ) [ 0 ]", [⟨false, none⟩], .element,
    false, false, false, false, false, true, false, true, false, true),
  ("( n ? 1 + p - a : b ) - > m", [⟨false, some false⟩], .arrow, false, false, false, false, true,
    true, false, true, false, true),
  ("( n ? 1 + p / 2 : a ) - > m", [⟨false, some false⟩], .arrow, false, false, false, false, true,
    true, false, true, false, true),
  ("( ( n ? 1 + p - n : a ) - b ) - > m", [⟨false, some true⟩, ⟨false, none⟩], .arrow,
    false, false, false, false, true, true, false, true, false, true),
  ("( n ? n - p + 1 : a ) - > m", [⟨false, some true⟩], .arrow, false, false, false, false, true,
    true, true, true, true, true),
  ("( n ? 1 + p - n : q ) - > m", [⟨false, some true⟩], .arrow, false, false, false, false, true,
    true, true, true, true, true),
  ("( n ? 1 + ( n ? 1 + p - n : a ) - n : b ) - > m", [⟨false, some true⟩, ⟨false, some true⟩],
    .arrow, false, false, false, false, true, true, true, true, true, true),
  ("f ( n ? 1 + p - n : a ) - > m", [⟨false, some true⟩], .arrow, false, false, true, true, true,
    true, true, true, true, true)
]

/-- A row's refusal: the root's, or every other (`other`). -/
def rowAgrees (row : String × List Crossing × Ctx × Bool × Bool × Bool × Bool × Bool ×
    Bool × Bool × Bool × Bool × Bool) : Bool :=
  let (name, cs, ctx, d, da, cb, ca, other, oldR, arm, b, a, refused) := row
  let r : Root := ⟨name, cs, ctx, d, da, cb, ca⟩
  oldRefuses r == oldR && (match r.walk with | .arm _ => true | .old _ => false) == arm &&
    newBase r == b && newArrow r == a && (other || newRefuses r) == refused

def walkVectorsAgree : Bool := walkVectors.all rowAgrees

theorem walk_vectors_hold : walkVectorsAgree = true := by decide

end QCore.ReturnFamilyArithWalkConditional
