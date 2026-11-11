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

import QCore.TypedRootDeclarations

/-!
The never-bound premise reads every assignment, and the typed positions serve
the join of the two closures (W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT, gateway
rule 10o). The token extraction and the Rust-to-Lean correspondence remain
review premises (standing constraint 28). This module proves:

* **the gap**: a closure computed over only the events the old statement
  reading saw (the first top-level `=` of each run) can be seeded and settled
  for that view while a name bound in the model is outside it — the old
  never-bound answer was unsound, witnessed by a nested assignment;
* **the superset reading is monotone and sound**: the model over the old view
  is contained in the model over every event, and a name outside a set that
  is seeded and settled for every event has no binding;
* **the join over-approximates each closure**: a root the joined answer calls
  never bound is outside the lexical set and, wherever the AST reader answers,
  outside the AST set; so if either set is seeded and settled for the body, it
  has no binding in the model;
* **rule 10l is kept and only strengthened**: where the lexical reader does not
  answer, the joined answer is rule 10l's; where the AST reader refuses the
  function, the lexical answer stands; the joined answer implies rule 10l's
  everywhere, and the pointer-root premise keeps both `may_bind` halves;
* **P3's return and chain theorems hold under the join**: a pointer return or
  a pointer actual with a root either closure binds refuses, and a P2 mixed
  path whose root either closure binds refuses.
-/

namespace QCore.LexicalClosureNestedAssignment

open QCore.TypedRootDeclarations
open QCore.TypedAccessPathPositions

/-! ## The model is monotone in the events it reads -/

/-- `Bound` only grows with the body it closes over. -/
theorem bound_mono {small large : List BindingEvent} (hsub : ∀ e ∈ small, e ∈ large)
    {n : Nat} (h : Bound small n) : Bound large n := by
  induction h with
  | seed he hn => exact .seed (hsub _ he) hn
  | opaqueAssign he => exact .opaqueAssign (hsub _ he)
  | copied he hm _ ih => exact .copied (hsub _ he) hm ih

/-- An event tagged with whether the old statement reading read it (the first
top-level `=` of a `;{}` run, or a seed it spells). -/
structure Tagged where
  event : BindingEvent
  oldReads : Bool
  deriving DecidableEq, Repr

/-- What the old reading saw. -/
def oldView (body : List Tagged) : List BindingEvent :=
  (body.filter (·.oldReads)).map (·.event)

/-- Every event: what rule 10o reads. -/
def fullView (body : List Tagged) : List BindingEvent :=
  body.map (·.event)

theorem the_old_view_is_contained (body : List Tagged) :
    ∀ e ∈ oldView body, e ∈ fullView body := by
  intro e he
  simp only [oldView, fullView, List.mem_map, List.mem_filter] at he ⊢
  obtain ⟨t, ⟨ht, _⟩, rfl⟩ := he
  exact ⟨t, ht, rfl⟩

/-- **Monotone.** Every binding the old reading found is a binding of the
superset reading's model. -/
theorem the_superset_reading_keeps_every_old_binding (body : List Tagged) {n : Nat}
    (h : Bound (oldView body) n) : Bound (fullView body) n :=
  bound_mono (the_old_view_is_contained body) h

/-- A set seeded and settled for every event is seeded and settled for the
old view too: the superset reading's set is a valid old-reading set. -/
theorem a_full_set_is_an_old_set (body : List Tagged) (s : List Nat)
    (hs : Seeded (fullView body) s) (ht : Settled (fullView body) s) :
    Seeded (oldView body) s ∧ Settled (oldView body) s :=
  ⟨fun e he => hs e (the_old_view_is_contained body e he),
    fun d o names he => ht d o names (the_old_view_is_contained body _ he)⟩

/-- **Soundness of the superset reading.** A name outside a set seeded and
settled for every event has no binding in the model. -/
theorem an_unlisted_name_is_never_bound_in_the_full_model {body : List Tagged}
    {s : List Nat} {n : Nat} (hs : seededB (fullView body) s = true)
    (ht : settledB (fullView body) s = true) (hn : n ∉ s) : ¬ Bound (fullView body) n :=
  an_unlisted_name_is_never_bound hs ht hn

/-! ## The gap, witnessed -/

/-- `a` (name 1) is written from a call at the top level; `b` (name 2) is
copied from `a` inside a condition, `if ((b = a))`, which the old reading
skipped. -/
def gapBody : List Tagged :=
  [⟨.assign 1 true [], true⟩, ⟨.assign 2 false [1], false⟩]

/-- The old reading's set `[1]` is seeded and settled for what it saw… -/
theorem the_old_set_is_settled_for_the_old_view :
    seededB (oldView gapBody) [1] = true ∧ settledB (oldView gapBody) [1] = true := by
  decide

/-- …yet `b` is bound in the model: the old never-bound answer for `b` was
unsound. -/
theorem the_nested_copy_is_bound : Bound (fullView gapBody) 2 :=
  .copied (m := 1) (o := false) (names := [1]) (by decide) (by decide)
    (.opaqueAssign (d := 1) (names := []) (by decide))

theorem the_old_set_misses_a_binding :
    2 ∉ ([1] : List Nat) ∧ Bound (fullView gapBody) 2 :=
  ⟨by decide, the_nested_copy_is_bound⟩

/-- The superset reading's set contains it, and is seeded and settled. -/
theorem the_full_set_is_settled :
    seededB (fullView gapBody) [1, 2] = true ∧ settledB (fullView gapBody) [1, 2] = true := by
  decide

/-! ## The join over-approximates each closure -/

/-- The joined never-bound answer at the set level: outside the lexical set,
and outside the AST set wherever the AST reader answers (`none`: it refused
the function). -/
def joinedOut (lexical : List Nat) (ast : Option (List Nat)) (n : Nat) : Prop :=
  n ∉ lexical ∧ ∀ a, ast = some a → n ∉ a

theorem a_joined_root_is_outside_the_lexical_set {l : List Nat} {a : Option (List Nat)}
    {n : Nat} (h : joinedOut l a n) : n ∉ l :=
  h.1

theorem a_joined_root_is_outside_the_ast_set {l a : List Nat} {n : Nat}
    (h : joinedOut l (some a) n) : n ∉ a :=
  h.2 a rfl

/-- **Over-approximation.** If either closure's set is seeded and settled for
the body, a root the join calls never bound has no binding in the model. -/
theorem a_joined_root_has_no_binding {body : List BindingEvent} {l : List Nat}
    {a : Option (List Nat)} {n : Nat} (h : joinedOut l a n)
    (hsound : (seededB body l = true ∧ settledB body l = true) ∨
      (∃ s, a = some s ∧ seededB body s = true ∧ settledB body s = true)) :
    ¬ Bound body n := by
  rcases hsound with ⟨hs, ht⟩ | ⟨s, hsa, hs, ht⟩
  · exact an_unlisted_name_is_never_bound hs ht h.1
  · exact an_unlisted_name_is_never_bound hs ht (h.2 s hsa)

/-- Where the AST reader refuses the function, the lexical answer stands. -/
theorem where_the_ast_refuses_the_lexical_answer_stands (l : List Nat) (n : Nat) :
    joinedOut l none n ↔ n ∉ l := by
  constructor
  · exact fun h => h.1
  · exact fun h => ⟨h, fun _ hn => by cases hn⟩

/-! ## The served answers -/

/-- One root's answers under rule 10o: rule 10l's (`base`, whose
`lexicalNeverBound` is now the superset closure's), and the AST closure's
spelling-level answers, `none` where it refuses the function. -/
structure Joined where
  base : Answers
  astBinds : Option Bool
  astNamesMayBind : Option Bool
  deriving DecidableEq, Repr

/-- Rule 10o: where the lexical reader answers, its answer unless the AST
closure binds the spelling; elsewhere rule 10l's served answer. -/
def joinedNeverBound (j : Joined) : Bool :=
  if j.base.lexicalReadable then j.base.lexicalNeverBound && j.astBinds != some true
  else servedNeverBound j.base

/-- Rule 5b's premise under rule 10o. -/
def joinedPointerRoot (j : Joined) : Bool :=
  if j.base.lexicalReadable then
    joinedNeverBound j && !j.base.lexicalMayBind && j.astNamesMayBind != some true
  else servedPointerRoot j.base

theorem a_joined_root_is_lexically_never_bound (j : Joined) (hr : j.base.lexicalReadable = true)
    (h : joinedNeverBound j = true) : j.base.lexicalNeverBound = true := by
  simp only [joinedNeverBound, hr, ite_true, Bool.and_eq_true] at h
  exact h.1

theorem a_joined_root_is_not_ast_bound (j : Joined) (hr : j.base.lexicalReadable = true)
    (h : joinedNeverBound j = true) : j.astBinds ≠ some true := by
  simp only [joinedNeverBound, hr, ite_true, Bool.and_eq_true, bne_iff_ne, ne_eq] at h
  exact h.2

/-- A root the AST closure binds is never served, in any readable function. -/
theorem an_ast_bound_root_is_not_served (j : Joined) (hr : j.base.lexicalReadable = true)
    (hb : j.astBinds = some true) : joinedNeverBound j = false ∧ joinedPointerRoot j = false := by
  simp [joinedPointerRoot, joinedNeverBound, hr, hb]

theorem where_the_ast_refuses_the_function_the_lexical_answer_is_served (j : Joined)
    (hr : j.base.lexicalReadable = true) (ha : j.astBinds = none) :
    joinedNeverBound j = j.base.lexicalNeverBound := by
  simp [joinedNeverBound, hr, ha]

theorem an_unreadable_function_keeps_rule_10l (j : Joined) (hr : j.base.lexicalReadable = false) :
    joinedNeverBound j = servedNeverBound j.base ∧ joinedPointerRoot j = servedPointerRoot j.base := by
  simp [joinedNeverBound, joinedPointerRoot, hr]

/-- **Only strengthened.** The joined answer implies rule 10l's, everywhere. -/
theorem the_join_implies_rule_10l (j : Joined) (h : joinedNeverBound j = true) :
    servedNeverBound j.base = true := by
  unfold joinedNeverBound at h
  cases hr : j.base.lexicalReadable
  · simpa [hr] using h
  · simp only [hr, ite_true, Bool.and_eq_true] at h
    simp [servedNeverBound, hr, h.1]

theorem the_joined_pointer_root_implies_rule_10l (j : Joined) (h : joinedPointerRoot j = true) :
    servedPointerRoot j.base = true := by
  unfold joinedPointerRoot at h
  cases hr : j.base.lexicalReadable
  · simpa [hr] using h
  · simp only [hr, ite_true, Bool.and_eq_true, Bool.not_eq_true'] at h
    have hl := a_joined_root_is_lexically_never_bound j hr h.1.1
    simp [servedPointerRoot, hr, hl, h.1.2]

/-- Rule 5b's two `may_bind` halves are kept: a joined pointer root is named by
neither, in every function the AST reader answers. -/
theorem a_joined_pointer_root_is_named_by_no_may_bind (j : Joined)
    (hr : j.base.lexicalReadable = true) (h : joinedPointerRoot j = true) :
    j.base.lexicalMayBind = false ∧ j.astNamesMayBind ≠ some true := by
  simp only [joinedPointerRoot, hr, ite_true, Bool.and_eq_true, Bool.not_eq_true', bne_iff_ne,
    ne_eq] at h
  exact ⟨h.1.2, h.2⟩

theorem a_joined_pointer_root_is_never_bound (j : Joined) (h : joinedPointerRoot j = true) :
    joinedNeverBound j = true := by
  unfold joinedPointerRoot at h
  cases hr : j.base.lexicalReadable
  · have := a_pointer_root_is_never_bound j.base (by simpa [hr] using h)
    simpa [joinedNeverBound, hr] using this
  · simp only [hr, ite_true, Bool.and_eq_true] at h
    exact h.1.1

/-! ## P3's return and chain theorems, and P2's mixed path, under the join -/

def withJoinedRoots (c : Candidate) (roots : List Joined) : Candidate :=
  { c with rootsUntracked := roots.all joinedPointerRoot }

/-- A pointer return with a root the AST closure binds refuses, in a function
the lexical reader answers (where rule 10l served the lexical answer alone). -/
theorem a_return_through_an_ast_bound_root_refuses_everywhere (c : Candidate)
    (roots : List Joined) (hp : c.position = .returnExpression) (hv : c.pointerValue = true)
    (r : Joined) (hr : r ∈ roots) (hread : r.base.lexicalReadable = true)
    (hb : r.astBinds = some true) :
    admits (withJoinedRoots c roots) = false := by
  apply a_returned_pointer_from_a_tracked_root_refuses
  · simpa [withJoinedRoots] using hp
  · simpa [withJoinedRoots] using hv
  · simp only [withJoinedRoots, List.all_eq_false]
    exact ⟨r, hr, by simp [(an_ast_bound_root_is_not_served r hread hb).2]⟩

/-- A pointer return with a root the (superset) lexical closure binds refuses. -/
theorem a_return_through_a_lexically_bound_root_refuses_everywhere (c : Candidate)
    (roots : List Joined) (hp : c.position = .returnExpression) (hv : c.pointerValue = true)
    (r : Joined) (hr : r ∈ roots) (hread : r.base.lexicalReadable = true)
    (hb : r.base.lexicalNeverBound = false) :
    admits (withJoinedRoots c roots) = false := by
  apply a_returned_pointer_from_a_tracked_root_refuses
  · simpa [withJoinedRoots] using hp
  · simpa [withJoinedRoots] using hv
  · simp only [withJoinedRoots, List.all_eq_false]
    refine ⟨r, hr, ?_⟩
    simp [joinedPointerRoot, joinedNeverBound, hread, hb]

/-- The chain theorem: a pointer actual with a root the AST closure binds
refuses. -/
theorem a_pointer_actual_through_an_ast_bound_root_refuses (c : Candidate)
    (roots : List Joined) (hp : c.position = .callActual) (hv : c.pointerValue = true)
    (r : Joined) (hr : r ∈ roots) (hread : r.base.lexicalReadable = true)
    (hb : r.astBinds = some true) :
    admits (withJoinedRoots c roots) = false := by
  apply a_pointer_actual_from_a_tracked_root_refuses
  · simpa [withJoinedRoots] using hp
  · simpa [withJoinedRoots] using hv
  · simp only [withJoinedRoots, List.all_eq_false]
    exact ⟨r, hr, by simp [(an_ast_bound_root_is_not_served r hread hb).2]⟩

/-- An admitted return still hands nothing back, whatever roots are served. -/
theorem a_joined_return_hands_nothing_back (s : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement)
    (r : IndirectChainAccess.State × List QCore.IndirectScalarAccess.Diagnostic × Exit)
    (h : lowerReturn s p = some r) : r.2.2 = .handsNothingBack :=
  a_typed_return_hands_nothing_back s p r h

/-- P2: a mixed path whose root the joined answer does not serve refuses. -/
theorem a_mixed_path_through_a_joined_bound_root_refuses (p : TypedAccessPathStatements.Shape)
    (j : Joined) (hm : p.mixedPath = true) (hroot : p.untrackedRoot = joinedNeverBound j)
    (hn : joinedNeverBound j = false) : TypedAccessPathStatements.admits p = false :=
  TypedAccessPathStatements.mixed_path_with_tracked_root_refuses p hm (hroot.trans hn)

/-! ## Non-vacuity -/

/-- An untouched root of a readable function the AST reader answers. -/
def untouchedJoined : Joined :=
  { base := { untouched with lexicalReadable := true, lexicalNeverBound := true },
    astBinds := some false, astNamesMayBind := some false }

theorem an_untouched_root_is_still_served :
    joinedNeverBound untouchedJoined = true ∧ joinedPointerRoot untouchedJoined = true := by
  decide

theorem the_same_root_bound_by_the_ast_is_not_served :
    joinedNeverBound { untouchedJoined with astBinds := some true } = false := by
  decide

theorem the_same_root_named_by_the_ast_may_bind_is_no_pointer_root :
    joinedPointerRoot { untouchedJoined with astNamesMayBind := some true } = false ∧
      joinedNeverBound { untouchedJoined with astNamesMayBind := some true } = true := by
  decide

theorem an_ast_refused_function_serves_the_lexical_answer :
    joinedNeverBound { untouchedJoined with astBinds := none } = true := by
  decide

theorem a_joined_root_in_the_gap_example : joinedOut [1, 2] (some [1, 2]) 0 ∧
    ¬ Bound (fullView gapBody) 0 :=
  ⟨⟨by decide, fun a h => by cases h; decide⟩,
    a_joined_root_has_no_binding (body := fullView gapBody) (l := [1, 2]) (a := some [1, 2])
      ⟨by decide, fun a h => by cases h; decide⟩ (Or.inl the_full_set_is_settled)⟩

end QCore.LexicalClosureNestedAssignment
