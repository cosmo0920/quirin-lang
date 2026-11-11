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

import QCore.IndirectCalls

/-!
# Callback contracts (card CG22-CALLBACK-REGISTRY-CONTRACT)

The model of contract spec C04.19 and gateway rules 13u–13x
(`crates/qcore-c-gateway/src/callback_contracts.rs`).

* **Synchronous invocation and table dispatch (rules 13u, 13v).** A callee
  calls one of its candidates zero or more times. The call reads as its custody
  contract exactly when every candidate's own contract is the identity on the
  caller's state; then any number of invocations, of any candidates, in any
  order, is the identity (`invocations_of_identities_are_identity`), so the
  callee's loop needs no fixed point. An open candidate set, a candidate with
  no contract in force, and an effectful candidate are refused, never assumed
  benign (`decide_*`).
* **Registries and element custody (rules 13w, 13x).** `registryTrace` follows
  one holder and one tracked element. A registry is used only under a
  registration in force (the `callbackTrace` premise of
  `QCore.IndirectCalls`: an invocation needs the registration in force); an
  element added to a releasing registry is the registry's, released by its
  invocation; one dropped or left at the exit without that invocation is lost
  (`none`); every other use is refused.

`decisionVectors` and `registryVectors` are `DECISION_VECTORS` and
`REGISTRY_VECTORS` of `callback_contracts.rs`, bound by its unit tests.
-/

namespace QCore.CallbackContracts

/-! ## Loop-safe composition -/

/-- What one candidate's contract is to an invocation. -/
inductive CandidateClass where
  | missing
  | identity
  | releasesElement
  | effectful
  deriving DecidableEq, Repr

/-- The decision of one invocation clause. -/
inductive Decision where
  | identity
  | releasesElements
  | refused
  deriving DecidableEq, Repr

/-- `decide` of `callback_contracts.rs`: `isOpen` an open candidate set,
`elements` a registry invocation over its elements. -/
def decide (isOpen : Bool) (cs : List CandidateClass) (elements : Bool) : Decision :=
  if isOpen || cs.isEmpty then .refused
  else if cs.any (· == .missing) then .refused
  else if cs.any (fun c => c == .effectful || (!elements && c == .releasesElement)) then .refused
  else if cs.all (· == .identity) then .identity
  else if cs.all (· == .releasesElement) then .releasesElements
  else .refused

/-- One run of the callee: the effects of the invocations it makes, in order. -/
def run {σ : Type} (s : σ) (effects : List (σ → σ)) : σ :=
  effects.foldl (fun s e => e s) s

/-- **Any number of invocations of identities is the identity.** -/
theorem invocations_of_identities_are_identity {σ : Type} (s : σ) (effects : List (σ → σ))
    (h : ∀ e ∈ effects, ∀ t, e t = t) : run s effects = s := by
  induction effects generalizing s with
  | nil => rfl
  | cons e es ih =>
    simp only [run, List.foldl_cons]
    rw [h e (List.mem_cons_self ..) s]
    exact ih s (fun e' m => h e' (List.mem_cons_of_mem _ m))

/-- An open candidate set is refused. -/
theorem decide_refuses_an_open_set (cs : List CandidateClass) (e : Bool) :
    decide true cs e = .refused := by
  simp [decide]

/-- An empty set is refused. -/
theorem decide_refuses_an_empty_set (o e : Bool) : decide o [] e = .refused := by
  simp [decide]

/-- A candidate with no contract in force is refused. -/
theorem decide_refuses_a_missing_contract (o : Bool) (cs : List CandidateClass) (e : Bool)
    (m : CandidateClass.missing ∈ cs) : decide o cs e = .refused := by
  unfold decide
  have : cs.any (· == .missing) = true := List.any_eq_true.mpr ⟨_, m, by decide⟩
  split
  · rfl
  · simp_all

/-- An effectful candidate is refused. -/
theorem decide_refuses_an_effectful_candidate (o : Bool) (cs : List CandidateClass) (e : Bool)
    (m : CandidateClass.effectful ∈ cs) : decide o cs e = .refused := by
  unfold decide
  have : cs.any (fun c => c == .effectful || (!e && c == .releasesElement)) = true :=
    List.any_eq_true.mpr ⟨_, m, by simp⟩
  split
  · rfl
  · split
    · rfl
    · simp_all

/-- Outside a registry invocation, a releasing candidate is effectful. -/
theorem decide_never_releases_outside_a_registry (o : Bool) (cs : List CandidateClass) :
    decide o cs false ≠ .releasesElements := by
  unfold decide
  split
  · simp
  · split
    · simp
    · split
      · simp
      · split
        · simp
        · rename_i _ _ h3 _
          intro hr
          split at hr
          · rename_i hall
            cases cs with
            | nil => simp at *
            | cons c rest =>
              have hc : c = .releasesElement := by
                have := List.all_eq_true.mp hall c (List.mem_cons_self ..)
                simpa using this
              apply h3
              exact List.any_eq_true.mpr ⟨c, List.mem_cons_self .., by simp [hc]⟩
          · exact absurd hr (by simp)

/-- The identity decision needs a closed, non-empty set of identities. -/
theorem decide_identity_needs_identities {o : Bool} {cs : List CandidateClass} {e : Bool}
    (h : decide o cs e = .identity) :
    o = false ∧ cs ≠ [] ∧ ∀ c ∈ cs, c = .identity := by
  unfold decide at h
  split at h
  · cases h
  · rename_i h0
    simp only [Bool.or_eq_true, not_or] at h0
    split at h
    · cases h
    · split at h
      · cases h
      · split at h
        · rename_i hall
          refine ⟨by simpa using h0.1, ?_, ?_⟩
          · intro hn; subst hn; simp at h0
          · intro c m
            simpa using List.all_eq_true.mp hall c m
        · split at h <;> cases h

/-- **Soundness of rule 13u's reading.** When the decision is `identity`, every
run of invocations whose effects are the candidates' (each identity candidate
having the identity effect) leaves the caller's state as it was: the call is
its custody contract alone. -/
theorem an_identity_decision_is_the_custody_reading {σ : Type} {o : Bool}
    {cs : List CandidateClass} {e : Bool} (effect : CandidateClass → σ → σ)
    (sound : ∀ t, effect .identity t = t) (h : decide o cs e = .identity)
    (picks : List CandidateClass) (within : ∀ p ∈ picks, p ∈ cs) (s : σ) :
    run s (picks.map effect) = s := by
  have ids := (decide_identity_needs_identities h).2.2
  apply invocations_of_identities_are_identity
  intro f mf t
  obtain ⟨p, mp, rfl⟩ := List.mem_map.mp mf
  rw [ids p (within p mp)]
  exact sound t

/-! ## Reading a candidate's contract

`classify` and `requirements` of `callback_contracts.rs` over a plain contract
of the right arity: each position's mode against what the callback receives
there. `resource_owned` is a requirement (a live non-null object), not an
effect: the callee meets it with what it supplies, and at a parameter position
the call itself carries it (`requirementsOf`), as a direct call of the
candidate with that actual would. -/

inductive Mode where
  | borrowed
  | owned
  | released
  | other
  deriving DecidableEq, Repr

inductive Actual where
  | parameter (i : Nat)
  | supplied
  | element
  deriving DecidableEq, Repr

/-- Whether one position is admitted, and whether it releases. -/
def admitted : Mode × Actual → Bool
  | (.borrowed, _) => true
  | (.owned, _) => true
  | (.released, .element) => true
  | _ => false

def classifyModes (ms : List (Mode × Actual)) : CandidateClass :=
  if !ms.all admitted then .effectful
  else if ms.any (fun p => p.1 == .released) then .releasesElement
  else .identity

def requirementsOf : List (Mode × Actual) → List Nat
  | [] => []
  | (.owned, .parameter i) :: rest => i :: requirementsOf rest
  | _ :: rest => requirementsOf rest

/-- An identity candidate only borrows or requires. -/
theorem identity_only_borrows_or_requires (ms : List (Mode × Actual))
    (h : classifyModes ms = .identity) : ∀ p ∈ ms, p.1 = .borrowed ∨ p.1 = .owned := by
  unfold classifyModes at h
  split at h
  · cases h
  · rename_i hall
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hall
    split at h
    · cases h
    · rename_i hrel
      intro p m
      have ha := List.all_eq_true.mp (by simpa using hall) p m
      obtain ⟨mode, actual⟩ := p
      cases mode <;> cases actual <;> simp_all [admitted]

/-- A release is admitted only at an `element` position. -/
theorem a_release_is_only_of_an_element (ms : List (Mode × Actual))
    (h : classifyModes ms ≠ .effectful) (a : Actual) (m : (Mode.released, a) ∈ ms) :
    a = .element := by
  unfold classifyModes at h
  split at h
  · exact absurd rfl h
  · rename_i hall
    have ha := List.all_eq_true.mp (by simpa using hall) _ m
    cases a <;> simp_all [admitted]

/-- The requirements are exactly the `owned` parameter positions. -/
theorem requirements_are_owned_parameters (ms : List (Mode × Actual)) (i : Nat) :
    i ∈ requirementsOf ms ↔ (Mode.owned, Actual.parameter i) ∈ ms := by
  induction ms with
  | nil => simp [requirementsOf]
  | cons p rest ih =>
    obtain ⟨mode, actual⟩ := p
    cases mode <;> cases actual <;> simp_all [requirementsOf, eq_comm]

/-- `CLASSIFY_VECTORS`: positions and the class. -/
def classifyVectors : List (List (Mode × Actual) × CandidateClass) := [
  ([(.borrowed, .parameter 0), (.borrowed, .supplied), (.borrowed, .parameter 2)], .identity),
  ([(.owned, .supplied), (.owned, .supplied)], .identity),
  ([(.owned, .parameter 0), (.borrowed, .supplied)], .identity),
  ([(.released, .element)], .releasesElement),
  ([(.released, .supplied)], .effectful),
  ([(.released, .parameter 0)], .effectful),
  ([(.other, .supplied)], .effectful),
  ([], .identity)
]

theorem classify_vectors_hold :
    classifyVectors.all (fun (ms, c) => classifyModes ms == c) = true := by
  decide

/-! ## Registries and element custody -/

/-- What a registration in force holds, for element custody. -/
inductive RegistryClass where
  | none
  | neutral
  | releasing
  | undecided
  deriving DecidableEq, Repr

/-- `registry_class` of `callback_contracts.rs`. -/
def registryClass (cs : List CandidateClass) : RegistryClass :=
  if cs.isEmpty then .none
  else if cs.all (· == .identity) then .neutral
  else if cs.all (· == .releasesElement) then .releasing
  else .undecided

/-- One event of a registry's life on a path. -/
inductive Event where
  | register (c : RegistryClass)
  | add
  | invoke
  | clear
  | exit
  deriving DecidableEq, Repr

/-- The element's state. -/
inductive Element where
  | owned
  | moved
  | retained
  | released
  | absent
  deriving DecidableEq, Repr

/-- `registry_trace` of `callback_contracts.rs`: `some true` nothing raised,
`some false` refused, `none` an element lost. -/
def go (reg : Option RegistryClass) (el : Element) : List Event → Option Bool
  | [] => some true
  | .register c :: es => go (some c) el es
  | .add :: es =>
    if el != .absent then some false
    else match reg with
      | some .releasing => go reg .moved es
      | some .none | some .neutral => go reg .retained es
      | _ => some false
  | .invoke :: es =>
    match reg with
    | some .releasing => go reg (if el == .moved then .released else el) es
    | some .none | some .neutral => go reg el es
    | _ => some false
  | .clear :: es =>
    match reg with
    | Option.none => some false
    | some _ =>
      if el == .moved then Option.none
      else go Option.none (if el == .retained then .owned else el) es
  | .exit :: _ =>
    match el with
    | .moved => Option.none
    | .retained => some false
    | _ => some true

def registryTrace (t : List Event) : Option Bool := go Option.none .absent t

/-- **No registry use without a registration in force.** -/
theorem an_invocation_needs_a_registration (rest : List Event) :
    registryTrace (.invoke :: rest) = some false := rfl

theorem an_element_needs_a_registration (rest : List Event) :
    registryTrace (.add :: rest) = some false := rfl

theorem a_clear_needs_a_registration (rest : List Event) :
    registryTrace (.clear :: rest) = some false := rfl

/-- The same premise in `QCore.IndirectCalls`: an invocation with no
registration in force is never `safe`. -/
theorem the_callback_trace_agrees :
    QCore.IndirectCalls.callbackTrace [.invoke] ≠ .safe := by decide

/-- An undecided registration decides no element and no invocation. -/
theorem an_undecided_registration_refuses (rest : List Event) :
    registryTrace (.register .undecided :: .add :: rest) = some false ∧
      registryTrace (.register .undecided :: .invoke :: rest) = some false :=
  ⟨rfl, rfl⟩

/-- **Element custody.** An element a releasing registry took is released by
the registry's invocation, and the trace is safe; dropping it, or leaving it
at the exit, without that invocation loses it. -/
theorem a_moved_element_is_released_by_its_registry :
    registryTrace [.register .releasing, .add, .invoke, .clear, .exit] = some true := rfl

theorem a_moved_element_left_at_the_exit_is_lost :
    registryTrace [.register .releasing, .add, .exit] = Option.none := rfl

theorem a_moved_element_dropped_without_invocation_is_lost :
    registryTrace [.register .releasing, .add, .clear, .exit] = Option.none := rfl

/-- A retained element is the function's again once the registry drops it;
still retained at the exit, it is refused (C04.14's exit rule). -/
theorem a_retained_element_returns_on_clear :
    registryTrace [.register .none, .add, .clear, .exit] = some true := rfl

theorem a_retained_element_at_the_exit_is_refused :
    registryTrace [.register .none, .add, .exit] = some false := rfl

/-- A releasing class needs every candidate to release its element. -/
theorem releasing_needs_every_candidate (cs : List CandidateClass)
    (h : registryClass cs = .releasing) : ∀ c ∈ cs, c = .releasesElement := by
  unfold registryClass at h
  split at h
  · cases h
  · split at h
    · cases h
    · split at h
      · rename_i hall
        intro c m
        simpa using List.all_eq_true.mp hall c m
      · cases h

/-! ## Shared vectors -/

/-- `DECISION_VECTORS`: (open?, candidate classes, elements?, decision:
0 identity, 1 releases elements, 2 refused). -/
def decisionVectors : List (Bool × List CandidateClass × Bool × Nat) := [
  (true, [.identity], false, 2),
  (false, [], false, 2),
  (false, [.identity], false, 0),
  (false, [.identity, .identity], false, 0),
  (false, [.identity, .missing], false, 2),
  (false, [.effectful], false, 2),
  (false, [.releasesElement], false, 2),
  (false, [.releasesElement], true, 1),
  (false, [.releasesElement, .identity], true, 2),
  (false, [.identity], true, 0)
]

def Decision.kind : Decision → Nat
  | .identity => 0
  | .releasesElements => 1
  | .refused => 2

theorem decision_vectors_hold :
    decisionVectors.all (fun (o, cs, e, k) => (decide o cs e).kind == k) = true := by
  decide

theorem decision_vectors_count : decisionVectors.length = 10 := by decide

/-- `REGISTRY_VECTORS`: a trace and its verdict. -/
def registryVectors : List (List Event × Option Bool) := [
  ([.register .releasing, .add, .invoke, .clear, .exit], some true),
  ([.register .releasing, .add, .exit], Option.none),
  ([.register .releasing, .add, .clear, .exit], Option.none),
  ([.add, .exit], some false),
  ([.register .undecided, .add, .exit], some false),
  ([.register .none, .add, .exit], some false),
  ([.register .none, .add, .clear, .exit], some true),
  ([.register .neutral, .invoke, .clear, .exit], some true),
  ([.invoke, .exit], some false),
  ([.register .undecided, .invoke, .exit], some false)
]

theorem registry_vectors_hold :
    registryVectors.all (fun (t, r) => registryTrace t == r) = true := by
  decide

theorem registry_vectors_count : registryVectors.length = 10 := by decide

end QCore.CallbackContracts
