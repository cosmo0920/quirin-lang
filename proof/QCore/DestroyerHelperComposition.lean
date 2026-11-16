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
# A destroyer's named helper, composed at its consumers (gateway rule 51)

Card RULE50-CONSUMER-COMPOSITION (contract spec C04.19.6 amended), folding
in MEMBER-PATH-HELPER-AT-A-CALL. A destroyer's declaration may name, for one
unguarded `#stale` path of the formal it releases, the unit helper that takes
the path's object (`resource_member_helper(P, TAG.PATH, H)`). Rule 50's body
reading counts the clause as covered (evidence `50`) only where it established
the take through exactly that helper.

At a consumer's direct call of the destroyer, after the destroyer's own
states, the helper is composed in two steps:

1. each member invocation the helper states is decided over the run's set
   (rule 12n). Only a closed empty set decides: it invokes nothing. An open
   set, a closed non-empty set, and a route without the run's facts refuse;
2. then the helper's states are read at the object the caller linked at the
   stale path, as a direct call of the helper on that object would read them
   (C04.19.6): `#stale` takes the object once, `#released` ends it, `#kept`
   leaves it.

A direct call of the helper destroyer itself (MEMBER-PATH-HELPER-AT-A-CALL)
is read in the same order: invocations first, then states. Only one frame is
composed. A helper never names a helper (the loader rejects it), so a deeper
frame refuses.

The consumer's row then rests on the helper's declaration, so the run's fold
(rule 12n) keeps the row only when the run's set is closed. That is why the
destroyer's callback-footprint premise can be removed. Over a closed empty
set, the composition equals the reading that assumed the helper's callback
invokes nothing, and every other set refuses at the call
(`the_composition_carries_the_footprint`).

## What this is not

A proof about Monkey's C bodies, Clang's AST, the run's store reading
(rule 12n's premise P-CG22-MEMBER-CLOSED-WORLD), or the tracker's links. The
Rust correspondence is reviewed (trust boundary 28) and pinned by
`tests/rule50_consumer_composition.rs`.
-/

namespace QCore.DestroyerHelperComposition

/-- A member's set as a consumer reads it over the run's facts. -/
inductive RunSet where
  /-- The route reads no run facts (no pre-pass). -/
  | unread
  /-- The run's set is open. -/
  | opened
  /-- The run's set is closed, with these candidates. -/
  | closed (candidates : List String)
  deriving DecidableEq, Repr

/-- Rule 51 (and rule 12n at a direct call over the run's facts): an
invocation decides, invoking nothing, only over a closed empty set. -/
def invokesNothing : RunSet → Bool
  | .closed [] => true
  | _ => false

/-- The run's fold (rule 12n with the pre-pass): a row resting on a member
invocation is kept exactly when the run's set is closed. -/
def foldKeeps : RunSet → Bool
  | .closed _ => true
  | _ => false

/-- One object the caller tracks: live, and taken through a stale path. -/
structure Obj where
  live : Bool
  taken : Bool
  deriving DecidableEq, Repr

/-- What a helper's state leaves at a path (C04.19.6, no set: a set refuses
at a direct call, and the loader admits no guard on a helper's state). -/
inductive Fate where
  | stale
  | released
  | kept
  deriving DecidableEq, Repr

/-- The direct-call reading of one state at one linked object: a take needs
one live object; `#stale` takes it (released, the member still designating
it), `#released` ends it, `#kept` leaves it. `none` refuses. -/
def applyState : Fate → Obj → Option Obj
  | .kept, o => some o
  | .stale, o => if o.live then some ⟨false, true⟩ else none
  | .released, o => if o.live then some ⟨false, false⟩ else none

/-- The composition at a call in frame `depth` (1: the consumer's own call
of the destroyer or of the helper): every invocation decides first, then the
states apply to the linked objects. `none` refuses. -/
def compose (depth : Nat) (sets : List RunSet) (links : List (Fate × Obj)) :
    Option (List Obj) :=
  if depth ≠ 1 then none
  else if sets.all invokesNothing then links.mapM (fun l => applyState l.1 l.2)
  else none

/-- The reading the callback-footprint premise stood for: the states alone,
the callback assumed to invoke nothing. -/
def statesOnly (links : List (Fate × Obj)) : Option (List Obj) :=
  links.mapM (fun l => applyState l.1 l.2)

/-- A later release of a tracked object: a taken object refuses (a double
release), a live one ends. -/
def release (o : Obj) : Option Obj :=
  if o.taken then none else if o.live then some ⟨false, false⟩ else none

/-- A composed call decided every invocation over a closed empty set. -/
theorem composed_only_over_closed_empty_sets (d : Nat) (sets : List RunSet)
    (links : List (Fate × Obj)) (r : List Obj) (h : compose d sets links = some r) :
    ∀ s ∈ sets, s = .closed [] := by
  intro s hs
  unfold compose at h
  split at h
  · cases h
  · split at h
    · rename_i hall
      have := List.all_eq_true.mp hall s hs
      cases s with
      | unread => simp [invokesNothing] at this
      | opened => simp [invokesNothing] at this
      | closed c => cases c with
        | nil => rfl
        | cons _ _ => simp [invokesNothing] at this
    · cases h

/-- An open set refuses, whatever the states. -/
theorem an_open_set_refuses (d : Nat) (sets : List RunSet) (links : List (Fate × Obj))
    (h : RunSet.opened ∈ sets) : compose d sets links = none := by
  cases hc : compose d sets links with
  | none => rfl
  | some r =>
    have := composed_only_over_closed_empty_sets d sets links r hc _ h
    cases this

/-- A closed non-empty set refuses: its candidates are not composed at a
consumer (RULE50-NONEMPTY-RUN-SET is not this card). -/
theorem a_non_empty_set_refuses (d : Nat) (sets : List RunSet) (links : List (Fate × Obj))
    (c : String) (cs : List String) (h : RunSet.closed (c :: cs) ∈ sets) :
    compose d sets links = none := by
  cases hc : compose d sets links with
  | none => rfl
  | some r =>
    have := composed_only_over_closed_empty_sets d sets links r hc _ h
    cases this

/-- A route without the run's facts refuses. -/
theorem no_run_facts_refuses (d : Nat) (sets : List RunSet) (links : List (Fate × Obj))
    (h : RunSet.unread ∈ sets) : compose d sets links = none := by
  cases hc : compose d sets links with
  | none => rfl
  | some r =>
    have := composed_only_over_closed_empty_sets d sets links r hc _ h
    cases this

/-- A deeper frame is never composed. -/
theorem a_deeper_frame_refuses (d : Nat) (sets : List RunSet) (links : List (Fate × Obj))
    (h : d ≠ 1) : compose d sets links = none := by
  simp [compose, h]

/-- The order: an invocation that does not decide leaves no state applied,
whatever the states would have done. -/
theorem a_refused_invocation_applies_no_state (sets : List RunSet)
    (h : sets.all invokesNothing = false) (links : List (Fate × Obj)) :
    compose 1 sets links = none := by
  simp [compose, h]

/-- Over closed empty sets the composition is exactly the reading the
callback-footprint premise stood for, and the fold keeps every such row: the
premise adds nothing the composition and the fold do not check. -/
theorem the_composition_carries_the_footprint (sets : List RunSet)
    (h : ∀ s ∈ sets, s = .closed []) (links : List (Fate × Obj)) :
    compose 1 sets links = statesOnly links ∧ sets.all foldKeeps = true := by
  have hall : sets.all invokesNothing = true := by
    apply List.all_eq_true.mpr
    intro s hs
    rw [h s hs]
    rfl
  constructor
  · simp [compose, statesOnly, hall]
  · apply List.all_eq_true.mpr
    intro s hs
    rw [h s hs]
    rfl

/-- Whatever the consumer's call decides, the fold keeps too: the call is the
stronger check, and the fold never promotes. -/
theorem a_decided_invocation_is_kept_by_the_fold (s : RunSet)
    (h : invokesNothing s = true) : foldKeeps s = true := by
  cases s with
  | unread => simp [invokesNothing] at h
  | opened => simp [invokesNothing] at h
  | closed c => rfl

/-- The helper's `#stale` takes a linked object once; releasing it again
refuses. -/
theorem a_release_after_the_helpers_take_refuses (o o' : Obj)
    (h : applyState .stale o = some o') : release o' = none := by
  cases hl : o.live with
  | true =>
    simp [applyState, hl] at h
    subst h
    rfl
  | false => simp [applyState, hl] at h

/-- A take of an object that is not one live object refuses. -/
theorem a_take_needs_a_live_object (f : Fate) (o : Obj) (hf : f ≠ .kept)
    (hl : o.live = false) : applyState f o = none := by
  cases f with
  | stale => simp [applyState, hl]
  | released => simp [applyState, hl]
  | kept => exact absurd rfl hf

/-- `#kept` leaves the linked object exactly as it was. -/
theorem kept_leaves_the_object (o : Obj) : applyState .kept o = some o := rfl

/-- Rule 50's evidence for a declaration naming its helper: the reading
admits only the named helper, so an established take was made through it. -/
def admitted (named : Option String) (callee : String) : Bool :=
  match named with
  | none => true
  | some h => h == callee

/-- An established take through an admitted helper is the named one. -/
theorem evidence_only_through_the_named_helper (h callee : String)
    (a : admitted (some h) callee = true) : callee = h := by
  simp [admitted] at a
  exact a.symm

/-- The scheduler's case: `mk_thread.cb_destroy`'s closed empty run set and
no tracked object linked at `parent`: the consumer's call composes, and the
result is the destroyer's own reading. -/
theorem the_schedulers_case_composes :
    compose 1 [.closed []] [] = some [] := by decide

end QCore.DestroyerHelperComposition
