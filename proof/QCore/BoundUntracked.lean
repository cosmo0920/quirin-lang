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

import QCore.IndirectChainAccess

/-!
# Bound but untracked roots (card CG10-BOUND-UNTRACKED-ROOTS, gateway rule 11d)

The abstract rule implemented by `crates/qcore-c-gateway/src/bound_untracked.rs`
(`decide`, `eligible_root`), `legacy_bound_untracked.rs` (the facts read off one
flow state) and the `bound_untracked` arm of `indirect_chain_access::chain_owner`.

Rule 10t lowers a mixed place on a pointer root to the chain obligation with the
never-bound premise off, so `chain_owner` reads the root only as one live
identity. A root the body may bind but that carries no tracked identity was
refused at every access. This rule adds a **separate, weaker-scoped** premise:
the root is read as an untracked, borrowed object at an access when

* it is *eligible*: a pointer declared once as a parameter or automatic local,
  every reference naming that declaration, never address-taken;
* its binding in the state entering the access is absent or a known,
  non-null, target-free value in a state that widened nothing;
* no place rooted at it is bound;
* no path reaching the access was refused or left an unresolved effect.

The last conjunct is what makes the premise per path: in the flow every
custody-taking position that receives an object with no tracked identity reports
a refusal, and so does every call to an uncontracted function, and the flag
records it on the path (joined by disjunction). The event model below states
those arms of the flow; the Rust tests bind them to the gateway (`EVENT_VECTORS`).

## What this is not

A model of the **decision** and of the flow arms it relies on. That an object
reached through a name with no binding is disjoint from every identity the frame
tracks (a contract's `None` result carries no custody; rule 10k's stated premise
that a never-bound root reaches no tracked place, here per path) is a premise,
not proved; faithful Clang extraction is trust boundary 28.
-/

namespace QCore.BoundUntracked

open QCore.IndirectScalarAccess (Binding Derivation)
open QCore.IndirectChainAccess (Owner)

/-! ## The decision -/

inductive BindingClass where
  | absent
  | targetFree
  | nullable
  | unresolved
  | tracked
  deriving DecidableEq, Repr

inductive Premise where
  | holds
  | ineligible
  | bound
  | rootedPlace
  | pathRefused
  deriving DecidableEq, Repr

/-- `bound_untracked::decide`. -/
def premise (eligible : Bool) (binding : BindingClass) (rooted refused : Bool) : Premise :=
  if !eligible then .ineligible
  else if !(binding == .absent || binding == .targetFree) then .bound
  else if rooted then .rootedPlace
  else if refused then .pathRefused
  else .holds

/-- `legacy_bound_untracked::binding_class`. -/
def bindingClass (widened : Bool) : Option Binding → BindingClass
  | none => .absent
  | some b =>
      if b.targets ≠ [] then .tracked
      else if b.mayBeNull then .nullable
      else if b.unknown && !widened && b.derivation != .mixed then .targetFree
      else .unresolved

/-- The decision vectors, spelled as `bound_untracked::DECISION_VECTORS`. -/
def decisionVectors : List (Bool × BindingClass × Bool × Bool × Premise) :=
  [ (true, .absent, false, false, .holds)
  , (true, .targetFree, false, false, .holds)
  , (false, .absent, false, false, .ineligible)
  , (true, .nullable, false, false, .bound)
  , (true, .unresolved, false, false, .bound)
  , (true, .tracked, false, false, .bound)
  , (true, .absent, true, false, .rootedPlace)
  , (true, .absent, false, true, .pathRefused)
  , (true, .targetFree, false, true, .pathRefused)
  , (false, .tracked, true, true, .ineligible) ]

theorem decision_vectors_hold :
    decisionVectors.all (fun (e, b, r, f, p) => premise e b r f == p) = true := by
  decide

theorem the_premise_holds_exactly_on_an_untracked_unrefused_root
    (e : Bool) (b : BindingClass) (r f : Bool) :
    premise e b r f = .holds ↔
      e = true ∧ (b = .absent ∨ b = .targetFree) ∧ r = false ∧ f = false := by
  cases e <;> cases b <;> cases r <;> cases f <;> simp [premise]

/-- Unlisted forms are refused: an ineligible root never holds. -/
theorem an_ineligible_root_never_holds (b : BindingClass) (r f : Bool) :
    premise false b r f = .ineligible := by
  simp [premise]

/-- A tracked, possibly-null or unresolved binding never holds: the root keeps
the tracked reading, or the refusal. -/
theorem a_bound_root_never_holds (e r f : Bool) (b : BindingClass)
    (hb : b = .nullable ∨ b = .unresolved ∨ b = .tracked) : premise e b r f ≠ .holds := by
  rcases hb with h | h | h <;> subst h <;> cases e <;> simp [premise]

/-! ## Eligibility -/

/-- The static facts `eligible_root` reads from the Clang AST. -/
structure Root where
  declaredOnce : Bool
  formalOrAutomatic : Bool
  pointer : Bool
  foreignReference : Bool
  addressTaken : Bool
  deriving DecidableEq, Repr

def eligible (r : Root) : Bool :=
  r.declaredOnce && r.formalOrAutomatic && r.pointer && !r.foreignReference && !r.addressTaken

/-- Class (d): an address-taken root stays refused. -/
theorem an_address_taken_root_is_refused (r : Root) (h : r.addressTaken = true)
    (b : BindingClass) (rooted f : Bool) : premise (eligible r) b rooted f = .ineligible := by
  simp [eligible, h, premise]

/-- A static or file-scope object, a shadowed spelling or a non-pointer is refused. -/
theorem an_unlisted_root_is_refused (r : Root)
    (h : r.declaredOnce = false ∨ r.formalOrAutomatic = false ∨ r.pointer = false ∨
      r.foreignReference = true)
    (b : BindingClass) (rooted f : Bool) : premise (eligible r) b rooted f = .ineligible := by
  rcases h with h | h | h | h <;> simp [eligible, h, premise]

/-! ## The root reading of `chain_owner` with rule 11d -/

/-- The flow facts one state gives the premise. -/
structure Facts where
  widened : Bool
  rooted : Nat → Bool
  refused : Bool

structure State where
  chain : IndirectChainAccess.State
  facts : Facts

/-- `ChainView::bound_untracked` on the flow state. -/
def holds (s : State) (root : Nat) : Bool :=
  premise true (bindingClass s.facts.widened (s.chain.base.binding root))
    (s.facts.rooted root) s.facts.refused == .holds

/-- An obligation with the classifier's eligibility. -/
structure Obligation where
  base : IndirectChainAccess.Obligation
  eligible : Bool

/-- Whether `chain_owner` asks the premise: a dereferenced, eligible root the
never-bound premise does not cover. -/
def asks (s : State) (c : Obligation) : Bool :=
  c.base.dereferencedRoot && !c.base.rootNeverBound && c.eligible && holds s c.base.root

/-- The root reading: where the premise holds, the root is read exactly as the
never-bound reading reads it; everywhere else, exactly as before. -/
def rootOwner (s : State) (c : Obligation) : Option Owner :=
  if asks s c then IndirectChainAccess.rootOwner s.chain { c.base with rootNeverBound := true }
  else IndirectChainAccess.rootOwner s.chain c.base

def resolves (s : State) (c : Obligation) : Bool :=
  match rootOwner s c with
  | some owner => (IndirectChainAccess.walk s.chain c.base 0 c.base.links owner).isSome
  | none => false

/-! ### The soundness interlock -/

/-- The never-bound answer is unchanged for every root. -/
theorem the_never_bound_answer_is_unchanged (s : State) (c : Obligation)
    (h : c.base.rootNeverBound = true) :
    rootOwner s c = IndirectChainAccess.rootOwner s.chain c.base := by
  simp [rootOwner, asks, h]

/-- An ineligible root is read exactly as before. -/
theorem an_ineligible_root_is_read_as_before (s : State) (c : Obligation)
    (h : c.eligible = false) :
    rootOwner s c = IndirectChainAccess.rootOwner s.chain c.base := by
  simp [rootOwner, asks, h]

/-- A root with a tracked identity keeps the tracked reading. -/
theorem a_tracked_root_keeps_the_tracked_reading (s : State) (c : Obligation) (b : Binding)
    (hb : s.chain.base.binding c.base.root = some b) (ht : b.targets ≠ []) :
    rootOwner s c = IndirectChainAccess.rootOwner s.chain c.base := by
  have : holds s c.base.root = false := by
    simp [holds, hb, bindingClass, ht, premise]
  simp [rootOwner, asks, this]

/-- A record-object root is never read by this rule. -/
theorem a_record_object_root_is_read_as_before (s : State) (c : Obligation)
    (h : c.base.dereferencedRoot = false) :
    rootOwner s c = IndirectChainAccess.rootOwner s.chain c.base := by
  simp [rootOwner, asks, h]

/-- Reading a root as never bound only adds: every owner the old reading gives,
the never-bound reading gives too. -/
theorem the_never_bound_reading_only_adds (s : IndirectChainAccess.State)
    (c : IndirectChainAccess.Obligation) (o : Owner)
    (h : IndirectChainAccess.rootOwner s c = some o) :
    IndirectChainAccess.rootOwner s { c with rootNeverBound := true } = some o := by
  obtain ⟨root, dereferencedRoot, rootNeverBound, firstPlace, links⟩ := c
  unfold IndirectChainAccess.rootOwner at h ⊢
  cases dereferencedRoot <;> cases rootNeverBound <;> simp only at h ⊢
  all_goals first
    | exact h
    | simp at h
    | skip
  · cases hb : s.base.binding root with
    | none => simp [hb] at h
    | some b =>
      simp only [hb] at h ⊢
      obtain ⟨targets, unknown, mayBeNull, derivation⟩ := b
      cases mayBeNull <;> cases targets <;> cases unknown <;> simp_all

/-- The rule only adds: every chain the old reading resolves still resolves. -/
theorem every_old_admission_stands (s : State) (c : Obligation)
    (h : IndirectChainAccess.resolves s.chain c.base = true) : resolves s c = true := by
  unfold IndirectChainAccess.resolves at h
  unfold resolves rootOwner
  cases ho : IndirectChainAccess.rootOwner s.chain c.base with
  | none => simp [ho] at h
  | some o =>
    rw [ho] at h
    by_cases ha : asks s c = true
    · simp only [ha, ↓reduceIte, the_never_bound_reading_only_adds s.chain c.base o ho]
      exact h
    · simp only [ha, Bool.false_eq_true, ↓reduceIte]
      exact h

/-! ### Per-hop sufficiency under the premise -/

/-- Under the premise an unbound root is untracked, and every link of it resolves:
the access carries exactly the per-hop obligations of an untracked owner. -/
theorem under_the_premise_an_unbound_root_resolves (s : State) (c : Obligation)
    (hd : c.base.dereferencedRoot = true) (hn : c.base.rootNeverBound = false)
    (he : c.eligible = true) (hb : s.chain.base.binding c.base.root = none)
    (hr : s.facts.rooted c.base.root = false) (hf : s.facts.refused = false) :
    rootOwner s c = some .untracked ∧ resolves s c = true := by
  have hh : holds s c.base.root = true := by simp [holds, hb, bindingClass, hr, hf, premise]
  have ha : asks s c = true := by simp [asks, hd, hn, he, hh]
  have ho : rootOwner s c = some .untracked := by
    simp [rootOwner, ha, IndirectChainAccess.rootOwner, hd, hb]
  exact ⟨ho, by simp [resolves, ho, IndirectChainAccess.an_untracked_owner_walks_untracked]⟩

/-- A known, non-null, target-free root (a borrowed alias, a null-tested derived
result of an untracked base) resolves under the premise. -/
theorem under_the_premise_a_target_free_root_resolves (s : State) (c : Obligation)
    (hd : c.base.dereferencedRoot = true) (hn : c.base.rootNeverBound = false)
    (he : c.eligible = true) (d : Derivation) (hmix : d ≠ .mixed)
    (hb : s.chain.base.binding c.base.root = some ⟨[], true, false, d⟩)
    (hw : s.facts.widened = false)
    (hr : s.facts.rooted c.base.root = false) (hf : s.facts.refused = false) :
    resolves s c = true := by
  have hh : holds s c.base.root = true := by
    simp [holds, hb, bindingClass, hw, hmix, hr, hf, premise]
  have ha : asks s c = true := by simp [asks, hd, hn, he, hh]
  have ho : rootOwner s c = some .untracked := by
    simp [rootOwner, ha, IndirectChainAccess.rootOwner, hd, hb]
  simp [resolves, ho, IndirectChainAccess.an_untracked_owner_walks_untracked]

/-- Without the premise the same root is refused (rule 10t's reading). -/
theorem a_refused_path_keeps_the_refusal (s : State) (c : Obligation)
    (hd : c.base.dereferencedRoot = true) (hn : c.base.rootNeverBound = false)
    (hb : s.chain.base.binding c.base.root = none) (hf : s.facts.refused = true) :
    resolves s c = false := by
  have hh : holds s c.base.root = false := by
    cases hr : s.facts.rooted c.base.root <;> simp [holds, hb, bindingClass, hr, hf, premise]
  simp [resolves, rootOwner, asks, hh, IndirectChainAccess.rootOwner, hd, hn, hb]

theorem a_rooted_place_keeps_the_refusal (s : State) (c : Obligation)
    (hd : c.base.dereferencedRoot = true) (hn : c.base.rootNeverBound = false)
    (hb : s.chain.base.binding c.base.root = none) (hr : s.facts.rooted c.base.root = true) :
    resolves s c = false := by
  have hh : holds s c.base.root = false := by simp [holds, hb, bindingClass, hr, premise]
  simp [resolves, rootOwner, asks, hh, IndirectChainAccess.rootOwner, hd, hn, hb]

/-- A possibly-null root stays refused under the premise too. -/
theorem a_nullable_root_stays_refused (s : State) (c : Obligation) (b : Binding)
    (hd : c.base.dereferencedRoot = true)
    (hb : s.chain.base.binding c.base.root = some b) (hnull : b.mayBeNull = true) :
    resolves s c = false := by
  have hold : IndirectChainAccess.rootOwner s.chain c.base = none := by
    simp [IndirectChainAccess.rootOwner, hd, hb, hnull]
  have hnew : IndirectChainAccess.rootOwner s.chain { c.base with rootNeverBound := true } = none := by
    simp [IndirectChainAccess.rootOwner, hd, hb, hnull]
  unfold resolves rootOwner
  by_cases ha : asks s c = true <;> simp [ha, hold, hnew]

/-- An unbound root the never-bound premise does not cover resolves only where
it is eligible and the premise holds: nothing else admits it. -/
theorem an_unbound_root_resolves_only_under_the_premise (s : State) (c : Obligation)
    (hd : c.base.dereferencedRoot = true) (hn : c.base.rootNeverBound = false)
    (hb : s.chain.base.binding c.base.root = none) (h : resolves s c = true) :
    c.eligible = true ∧ holds s c.base.root = true := by
  by_cases ha : asks s c = true
  · simp only [asks, Bool.and_eq_true] at ha
    exact ⟨ha.1.2, ha.2⟩
  · have hold : IndirectChainAccess.rootOwner s.chain c.base = none := by
      simp [IndirectChainAccess.rootOwner, hd, hn, hb]
    unfold resolves rootOwner at h
    simp [ha, hold] at h

/-! ## Rule 10k(3) with rule 11d: a pointer read handed to a `Borrowed` actual -/

/-- The classifier's answer for a pointer-valued actual at a `Borrowed` or
declared data-tail position (`typed_access_positions::argument`): a never-bound
root as before, or an eligible chain root left to the flow; an address `&PATH`
keeps rule 10n's never-bound premise. -/
def actualClassified (neverBound eligible address : Bool) : Bool :=
  neverBound || (eligible && !address)

theorem every_never_bound_actual_stands (e a : Bool) : actualClassified true e a = true := by
  simp [actualClassified]

theorem an_address_actual_keeps_the_never_bound_premise (e : Bool) :
    actualClassified false e true = false := by
  simp [actualClassified]

theorem an_ineligible_actual_is_refused (a : Bool) : actualClassified false false a = false := by
  simp [actualClassified]

/-- The flow's reading of a deferred actual's root (`root_untracked_only`):
only an untracked owner; rule 10k(3)'s premise is not relaxed to a tracked
root. -/
def actualOwner (s : State) (c : Obligation) : Option Owner :=
  match rootOwner s c with
  | some .untracked => some .untracked
  | _ => none

theorem a_deferred_actual_never_reads_a_tracked_root (s : State) (c : Obligation) (site : Nat) :
    actualOwner s c ≠ some (.tracked site) := by
  unfold actualOwner
  split <;> simp

theorem a_deferred_actual_is_read_under_the_premise (s : State) (c : Obligation)
    (hd : c.base.dereferencedRoot = true) (hn : c.base.rootNeverBound = false)
    (he : c.eligible = true) (hb : s.chain.base.binding c.base.root = none)
    (hr : s.facts.rooted c.base.root = false) (hf : s.facts.refused = false) :
    actualOwner s c = some .untracked := by
  simp [actualOwner, (under_the_premise_an_unbound_root_resolves s c hd hn he hb hr hf).1]

/-! ## The flow arms the premise relies on (per path) -/

/-- One event on a path, as the flow meets the root. -/
inductive Custody where
  | consumed
  | released
  | stored
  | owned
  deriving DecidableEq, Repr

inductive Event where
  /-- The root at a `Borrowed` position: no report, no binding change. -/
  | borrowed
  /-- The root at a custody-taking position. -/
  | custody (mode : Custody)
  /-- A call to an uncontracted function (any actuals). -/
  | uncontracted
  /-- `R = f(…)` with a contracted result that carries no custody. -/
  | rebindNone
  /-- `R = f(B)` with a result the contract declares derived from an untracked
  base (`derive_pointer` of an absent base). -/
  | rebindDerived
  /-- `R = f(…)` with an owned result: a fresh tracked identity. -/
  | rebindOwned (site : Nat)
  /-- The non-null edge of a null test of `R`. -/
  | nullTest
  /-- An admitted store binds a place rooted at `R`. -/
  | memberStore
  deriving DecidableEq, Repr

/-- The root's slice of the flow state on one path. -/
structure PathState where
  binding : Option Binding
  rooted : Bool
  refused : Bool
  deriving DecidableEq, Repr

/-- The flow's arms, as `legacy_c::transfer` applies them to the root: a custody
position on a root with no target reports (`apply_resource_action`,
`require_live_resource`), an uncontracted call is a refused statement; on a
tracked root a custody position changes the site, not the binding. -/
def step (s : PathState) : Event → PathState
  | .borrowed => s
  | .custody _ =>
      match s.binding with
      | some b => if b.targets = [] then { s with refused := true } else s
      | none => { s with refused := true }
  | .uncontracted => { s with refused := true }
  | .rebindNone => { s with binding := none }
  | .rebindDerived => { s with binding := some ⟨[], true, true, .derived⟩ }
  | .rebindOwned site => { s with binding := some ⟨[site], false, false, .identity⟩ }
  | .nullTest =>
      match s.binding with
      | some b => { s with binding := some { b with mayBeNull := false } }
      | none => s
  | .memberStore => { s with rooted := true }

def run (s : PathState) : List Event → PathState
  | [] => s
  | e :: rest => run (step s e) rest

def premiseOf (s : PathState) : Premise :=
  premise true (bindingClass false s.binding) s.rooted s.refused

/-- The path starting at an eligible root with no binding. -/
def start : PathState := ⟨none, false, false⟩

theorem the_refusal_is_monotone (s : PathState) (e : Event) (h : s.refused = true) :
    (step s e).refused = true := by
  cases e with
  | custody m =>
    simp only [step]
    split
    · split <;> simp [h]
    · simp
  | nullTest =>
    simp only [step]
    split <;> simp [h]
  | _ => simp [step, h]

theorem a_refusal_is_kept_on_every_later_path :
    ∀ (rest : List Event) (s : PathState), s.refused = true → (run s rest).refused = true := by
  intro rest
  induction rest with
  | nil => intro s h; exact h
  | cons e rest ih => intro s h; exact ih _ (the_refusal_is_monotone s e h)

theorem a_refused_path_never_holds (s : PathState) (h : s.refused = true) :
    premiseOf s ≠ .holds := by
  intro hp
  have := (the_premise_holds_exactly_on_an_untracked_unrefused_root _ _ _ _).mp hp
  rw [h] at this
  simp at this

/-- A `Borrowed` actual preserves the premise. -/
theorem a_borrowed_actual_preserves_the_premise (s : PathState) :
    premiseOf (step s .borrowed) = premiseOf s := rfl

/-- A custody-taking actual on an untracked root ends the premise on every path
after it. -/
theorem a_custody_actual_ends_the_premise (s : PathState) (m : Custody)
    (hu : s.binding = none ∨ ∃ b, s.binding = some b ∧ b.targets = [])
    (rest : List Event) : premiseOf (run (step s (.custody m)) rest) ≠ .holds := by
  apply a_refused_path_never_holds
  apply a_refusal_is_kept_on_every_later_path
  rcases hu with h | ⟨b, h, ht⟩
  · simp [step, h]
  · simp [step, h, ht]

/-- An uncontracted call ends the premise on every path after it. -/
theorem an_uncontracted_call_ends_the_premise (s : PathState) (rest : List Event) :
    premiseOf (run (step s .uncontracted) rest) ≠ .holds :=
  a_refused_path_never_holds _ (a_refusal_is_kept_on_every_later_path rest _ rfl)

/-- A result the contract declares to carry no custody satisfies the premise. -/
theorem a_none_result_satisfies_the_premise (s : PathState)
    (hr : s.rooted = false) (hf : s.refused = false) :
    premiseOf (step s .rebindNone) = .holds := by
  simp [premiseOf, step, bindingClass, hr, hf, premise]

/-- A derived result of an untracked base satisfies it once null-tested, and is
refused (possibly null) before. -/
theorem a_derived_result_of_an_untracked_base_satisfies_the_premise (s : PathState)
    (hr : s.rooted = false) (hf : s.refused = false) :
    premiseOf (step (step s .rebindDerived) .nullTest) = .holds ∧
      premiseOf (step s .rebindDerived) = .bound := by
  constructor <;> simp [premiseOf, step, bindingClass, hr, hf, premise]

/-- An owned result is a tracked identity: the tracked reading decides. -/
theorem an_owned_result_keeps_the_tracked_reading (s : PathState) (site : Nat) :
    premiseOf (step s (.rebindOwned site)) = .bound := by
  simp [premiseOf, step, bindingClass, premise]

/-- The join of two paths (`AbstractState::join`): the refusal is a disjunction,
so a refusal on either path is kept. -/
def join (a b : PathState) : PathState :=
  { binding :=
      match a.binding, b.binding with
      | none, none => none
      | some x, none | none, some x => some { x with unknown := true, mayBeNull := true }
      | some x, some y =>
          some ⟨x.targets ++ y.targets, x.unknown || y.unknown, x.mayBeNull || y.mayBeNull,
            if x.derivation = y.derivation then x.derivation else .mixed⟩
    rooted := a.rooted || b.rooted
    refused := a.refused || b.refused }

theorem the_join_keeps_a_refusal (a b : PathState) (h : a.refused = true ∨ b.refused = true) :
    premiseOf (join a b) ≠ .holds := by
  apply a_refused_path_never_holds
  rcases h with h | h <;> simp [join, h]

/-- A root bound on one path and unbound on the other is possibly null after the
join: never read as untracked. -/
theorem a_one_sided_binding_is_refused_after_the_join (a b : PathState) (x : Binding)
    (ha : a.binding = some x) (hb : b.binding = none) : premiseOf (join a b) ≠ .holds := by
  simp [premiseOf, join, ha, hb, bindingClass, premise]
  split <;> simp

/-- The event vectors, spelled as `bound_untracked::EVENT_VECTORS`. -/
def eventVectors : List (List Event × Premise) :=
  [ ([], .holds)
  , ([.borrowed], .holds)
  , ([.borrowed, .borrowed], .holds)
  , ([.custody .released], .pathRefused)
  , ([.custody .consumed], .pathRefused)
  , ([.custody .stored], .pathRefused)
  , ([.custody .owned], .pathRefused)
  , ([.uncontracted], .pathRefused)
  , ([.custody .released, .borrowed], .pathRefused)
  , ([.uncontracted, .rebindNone], .pathRefused)
  , ([.rebindNone], .holds)
  , ([.rebindDerived], .bound)
  , ([.rebindDerived, .nullTest], .holds)
  , ([.rebindOwned 7], .bound)
  , ([.memberStore], .rootedPlace) ]

theorem event_vectors_hold :
    eventVectors.all (fun (events, p) => premiseOf (run start events) == p) = true := by
  decide

/-! ## Non-vacuity -/

/-- Root 0 of `IndirectChainAccess.example_state` is unbound; read as a formal
the body may bind, on an unrefused path its chain of depth three is admitted,
and after a refusal on the path it is not. -/
def exampleState (refused : Bool) : State :=
  { chain := IndirectChainAccess.example_state
    facts := { widened := false, rooted := fun _ => false, refused := refused } }

theorem an_unbound_bound_root_is_admitted_on_an_unrefused_path :
    resolves (exampleState false) ⟨⟨0, true, false, 99, [10, 11, 12]⟩, true⟩ = true := by
  decide

theorem the_same_root_is_refused_after_a_refusal :
    resolves (exampleState true) ⟨⟨0, true, false, 99, [10, 11, 12]⟩, true⟩ = false := by
  decide

theorem the_same_root_is_refused_when_ineligible :
    resolves (exampleState false) ⟨⟨0, true, false, 99, [10, 11, 12]⟩, false⟩ = false := by
  decide

end QCore.BoundUntracked
