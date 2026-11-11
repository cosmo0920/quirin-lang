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

import QCore.CallExits

/-! Return-position correctness (card GAPO-RETURN-CORRECTNESS, gateway rules
15x, 15y and 15z): a model of
`crates/qcore-c-gateway/src/legacy_probe_state.rs` (the probe state a
declined classification hands back), of `legacy_no_return_paths.rs` (a
declared no-return call ends its path at every lowered position) and of the
return-position reading of rule 12h (`legacy_cast_semantics.rs`).

Bound mechanically: the unit tests `legacy_c::probe_state::tests` pin the
probe-state field list against the parser's declaration;
`tests/gapo_return_correctness.rs` runs the route-G controls; the decision a
no-return node takes is `QCore.CallExits.decide`, the table rule 11o's
shared-vector test already binds. The Clang reading of `noreturn` and of a
cast's class stay the review premises of rules 11o and 12h.

Stated here:
* **15x, state restoration.** A declined probe leaves the probe state equal to
  before (`declined_restores_probe_state`); a probe that only reads monotone
  facts leaves the whole state equal to before
  (`declined_without_new_facts_is_identity`); monotone facts only grow
  (`declined_keeps_facts`); so the next site a statement mints is the one it
  would have minted had the probe never run (`declined_probe_moves_no_site`),
  and rule 12h's replacing classification leaves no gap
  (`replacement_from_before_has_no_gap`, against `replacement_after_first_gaps`).
* **15y, path end at every position.** A declared no-return call never
  continues, at any position (`no_return_never_continues`); after settlement
  every node holding one has no successor or is refused
  (`settled_ends_or_refuses`); a path end is created only where rule 11o's
  premises hold (`settled_path_end_only_when_established`); a node without
  such a call, or already refused, is unchanged (`settle_identity`,
  `settle_keeps_refusal`).
* **15z, the uncast actual at the return position.** Reading a cast actual as
  its place is at least as strict as reading it as no place
  (`uncast_at_least_as_strict`), so the replacement never admits a call the
  original refused, and it refuses the use after release the cast hid
  (`cast_actual_hid_a_released_place`). -/
namespace QCore.ReturnCorrectness

/-! ## 15x: probe state -/

/-- The parser state: the probe state (a site counter, an element counter,
the consultation log and the rest of the scratch, abstracted as `scratch`)
and the monotone facts. -/
structure State where
  nextSite : Nat
  elements : Nat
  scratch : List Nat
  consultations : List String := []
  facts : List String
  deriving DecidableEq, Repr

/-- The probe-state part of a state (Rust `ParserProbeState`). -/
def State.probe (s : State) : Nat × Nat × List Nat × List String :=
  (s.nextSite, s.elements, s.scratch, s.consultations)

/-- A classification probe: an answer and the state it leaves. -/
abbrev Probe (α : Type) := State → α × State

/-- A well-formed probe only grows the monotone facts. -/
def Grows {α : Type} (p : Probe α) : Prop := ∀ s f, f ∈ s.facts → f ∈ (p s).2.facts

/-- Hand back the probe state (Rust `restore_probe_state`); the facts keep
what the probe grew them to. -/
def restore (before after : State) : State :=
  { after with nextSite := before.nextSite, elements := before.elements,
               scratch := before.scratch, consultations := before.consultations }

/-- A declined probe (Rust `Parser::probe` with `Probe::Decline`). -/
def declined {α : Type} (p : Probe α) : Probe α := fun s =>
  let r := p s
  (r.1, restore s r.2)

theorem declined_restores_probe_state {α : Type} (p : Probe α) (s : State) :
    (declined p s).2.probe = s.probe := by
  simp [declined, restore, State.probe]

theorem declined_restores_consultations {α : Type} (p : Probe α) (s : State) :
    (declined p s).2.consultations = s.consultations := by
  simp [declined, restore]

theorem declined_keeps_facts {α : Type} (p : Probe α) (hp : Grows p) (s : State)
    (f : String) (hf : f ∈ s.facts) : f ∈ (declined p s).2.facts := by
  simpa [declined, restore] using hp s f hf

theorem declined_without_new_facts_is_identity {α : Type} (p : Probe α) (s : State)
    (h : (p s).2.facts = s.facts) : (declined p s).2 = s := by
  cases s
  simp_all [declined, restore]

theorem declined_answer {α : Type} (p : Probe α) (s : State) :
    (declined p s).1 = (p s).1 := rfl

/-- A statement that mints `k` sites from the current counter. -/
def mint (k : Nat) (s : State) : List Nat × State :=
  ((List.range k).map (· + s.nextSite), { s with nextSite := s.nextSite + k })

theorem declined_probe_moves_no_site {α : Type} (p : Probe α) (s : State) (k : Nat) :
    (mint k (declined p s).2).1 = (mint k s).1 := by
  simp [mint, declined, restore]

/-- A classification that mints `k` sites. -/
def classify (k : Nat) : Probe Unit := fun s => ((), (mint k s).2)

/-- Rule 12h before the card: the uncast classification ran after the first,
which kept its sites. -/
def replacementAfterFirst (k₁ k₂ : Nat) (s : State) : State :=
  (classify k₂ (classify k₁ s).2).2

/-- Rule 12h with the card: the uncast classification starts from the state
the first one started from. -/
def replacementFromBefore (k₁ k₂ : Nat) (s : State) : State :=
  (classify k₂ (restore s (classify k₁ s).2)).2

theorem replacement_from_before_has_no_gap (k₁ k₂ : Nat) (s : State) :
    (replacementFromBefore k₁ k₂ s).nextSite = s.nextSite + k₂ := by
  simp [replacementFromBefore, classify, mint, restore]

theorem replacement_after_first_gaps (k₁ k₂ : Nat) (s : State) :
    (replacementAfterFirst k₁ k₂ s).nextSite = s.nextSite + k₁ + k₂ := by
  simp [replacementAfterFirst, classify, mint, Nat.add_assoc]

/-! ## 15y: a declared no-return call ends its path at every position -/

/-- Where route G can lower a call. -/
inductive Position where
  | statement
  | laterStatementRoute
  | returnEvaluation
  | condition
  | forClause
  | nestedActual
  deriving DecidableEq, Repr

/-- The decision a lowered call takes, at any position: rule 11o's own
decision for a contracted call, with the declaration read from the
environment. A declared callee never continues. -/
def decideAt (_pos : Position) (declared : Bool) (r : CallExits.Resolution) :
    CallExits.Decision :=
  CallExits.decide true declared r

theorem no_return_never_continues (pos : Position) (r : CallExits.Resolution) :
    decideAt pos true r ≠ .continue := by
  cases r <;> simp [decideAt, CallExits.decide]

/-- One lowered node: the callees it calls, whether it is exactly the
discarded whole call of its first callee, its successors, whether it carries
a loop exit edge, and whether it is a refusal. -/
structure Node where
  calls : List String
  discardedWhole : Bool
  succ : List Nat
  loopExit : Bool
  refused : Bool
  deriving DecidableEq, Repr

/-- The refusal of a node (Rust `Unknown`, unbounded writes): no call, no
claim; its successors stay. -/
def Node.refuse (n : Node) : Node := { n with calls := [], refused := true }

/-- The one-call case of [`settle`]: a path end exactly where rule 11o's
premises hold for the discarded whole call, a refusal otherwise. -/
def settleCall (declared : String → Bool) (resolve : String → CallExits.Resolution)
    (n : Node) (c : String) : Node :=
  if n.discardedWhole = true ∧ n.loopExit = false ∧
      CallExits.decide true (declared c) (resolve c) = .pathEnd
  then { n with succ := [] } else n.refuse

/-- Rust `settle_no_return_node`: a node calling a declared callee that still
has a successor becomes a path end when it is exactly that discarded whole
call, carries no loop exit and the callee resolves `noreturn`; otherwise it
is refused. -/
def settle (declared : String → Bool) (resolve : String → CallExits.Resolution)
    (n : Node) : Node :=
  if n.calls.any declared = false ∨ n.succ = [] then n
  else
    match n.calls with
    | [c] => settleCall declared resolve n c
    | _ => n.refuse

theorem settle_identity (declared : String → Bool) (resolve : String → CallExits.Resolution)
    (n : Node) (h : n.calls.any declared = false) : settle declared resolve n = n := by
  simp [settle, h]

theorem settle_call_cases (declared : String → Bool) (resolve : String → CallExits.Resolution)
    (n : Node) (c : String) :
    settleCall declared resolve n c = { n with succ := [] } ∨
      settleCall declared resolve n c = n.refuse := by
  unfold settleCall
  by_cases h : n.discardedWhole = true ∧ n.loopExit = false ∧
      CallExits.decide true (declared c) (resolve c) = .pathEnd
  · left; rw [ite_eq_left h]
  · right; rw [ite_eq_right h]

/-- Outside the identity case the settled node is a path end of the one call
or the refusal. -/
theorem settle_cases (declared : String → Bool) (resolve : String → CallExits.Resolution)
    (n : Node) (hc : ¬ (n.calls.any declared = false ∨ n.succ = [])) :
    settle declared resolve n = n.refuse ∨
      ∃ c, n.calls = [c] ∧ settle declared resolve n = settleCall declared resolve n c := by
  unfold settle
  rw [ite_eq_right hc]
  cases hcalls : n.calls with
  | nil => left; rfl
  | cons c rest =>
    cases rest with
    | nil => right; exact ⟨c, rfl, rfl⟩
    | cons d more => left; rfl

theorem settle_keeps_refusal (declared : String → Bool) (resolve : String → CallExits.Resolution)
    (n : Node) (h : n.refused = true) : (settle declared resolve n).refused = true := by
  by_cases hc : n.calls.any declared = false ∨ n.succ = []
  · unfold settle; rw [ite_eq_left hc]; exact h
  · rcases settle_cases declared resolve n hc with hs | ⟨c, _, hs⟩
    · rw [hs]; rfl
    · rw [hs]
      rcases settle_call_cases declared resolve n c with hk | hk
      · rw [hk]; exact h
      · rw [hk]; rfl

theorem settled_ends_or_refuses (declared : String → Bool)
    (resolve : String → CallExits.Resolution) (n : Node)
    (h : (settle declared resolve n).calls.any declared = true) :
    (settle declared resolve n).succ = [] ∨ (settle declared resolve n).refused = true := by
  by_cases hc : n.calls.any declared = false ∨ n.succ = []
  · have hn : settle declared resolve n = n := by unfold settle; rw [ite_eq_left hc]
    rw [hn] at h ⊢
    rcases hc with hc | hc
    · rw [hc] at h; exact absurd h (by decide)
    · left; exact hc
  · rcases settle_cases declared resolve n hc with hs | ⟨c, _, hs⟩
    · right; rw [hs]; rfl
    · rw [hs]
      rcases settle_call_cases declared resolve n c with hk | hk
      · left; rw [hk]
      · right; rw [hk]; rfl

theorem settled_path_end_only_when_established (declared : String → Bool)
    (resolve : String → CallExits.Resolution) (n : Node) (hs : n.succ ≠ [])
    (h : (settle declared resolve n).succ = [])
    (hr : (settle declared resolve n).refused = n.refused) (hn : n.refused = false) :
    ∃ c, n.calls = [c] ∧ declared c = true ∧ resolve c = .noReturn ∧
      n.discardedWhole = true ∧ n.loopExit = false := by
  by_cases hc : n.calls.any declared = false ∨ n.succ = []
  · have hn' : settle declared resolve n = n := by unfold settle; rw [ite_eq_left hc]
    rw [hn'] at h
    exact absurd h hs
  · rcases settle_cases declared resolve n hc with hsr | ⟨c, hcalls, hsc⟩
    · rw [hsr] at hr
      simp [Node.refuse] at hr
      rw [hn] at hr
      exact absurd hr (by decide)
    · rw [hsc] at h hr
      unfold settleCall at h hr
      by_cases hk : n.discardedWhole = true ∧ n.loopExit = false ∧
          CallExits.decide true (declared c) (resolve c) = .pathEnd
      · have ⟨_, hdecl, hres⟩ := (CallExits.path_end_iff _ _ _).mp hk.2.2
        exact ⟨c, hcalls, hdecl, hres, hk.1, hk.2.1⟩
      · rw [ite_eq_right hk] at hr
        simp [Node.refuse] at hr
        rw [hn] at hr
        exact absurd hr (by decide)

/-! ## 15z: the uncast actual at the return position -/

/-- A `Borrowed` actual as the lexical classifier reads it: a place, or no
place (a cast actual before rule 12h). -/
def borrowedCheck (live : List Nat) : Option Nat → Bool
  | none => true
  | some p => live.contains p

theorem uncast_at_least_as_strict (live : List Nat) (p : Nat) :
    borrowedCheck live (some p) = true → borrowedCheck live none = true :=
  fun _ => rfl

/-- `t = alloc(); free(t); return observe((void *) t);`: the cast actual read
as no place passes, the uncast place does not. -/
theorem cast_actual_hid_a_released_place :
    borrowedCheck ([1].erase 1) none = true ∧
      borrowedCheck ([1].erase 1) (some 1) = false := by
  decide

end QCore.ReturnCorrectness
