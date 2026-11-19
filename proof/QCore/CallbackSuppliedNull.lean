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

import QCore.CallbackContracts

/-!
# A supplied actual that may be NULL (card CALLBACK-SUPPLIED-NULL-ACTUAL)

The model of contract spec C04.19 item 2 as amended (gateway rule 13u) for
the actual word `supplied_nullable`: a value the callee supplies, as `_`,
that the library's page says may be a null pointer
(`crates/qcore-c-gateway/src/callback_contracts.rs`, `classify` and
`decide`).

`resource_owned(nonnull)` is a requirement of a live non-null object. At `_`
the callee meets it with what it supplies; at `supplied_nullable` it may
not, so a candidate requiring it there is refused (`callback-supplied-null`).
`resource_borrowed` demands nothing and `resource_owned(nullable)` accepts
NULL, so both are admitted there as at `_`.

* `a_nonnull_requirement_at_a_nullable_actual_is_not_admitted` and
  `decide_refuses_a_nonnull_requirement`: the refusal.
* `an_admitted_candidate_accepts_null`: an admitted candidate accepts NULL at
  every nullable actual.
* `marking_never_admits_more`: turning `_` actuals into `supplied_nullable`
  keeps the class or refuses; never the other way.
* `without_a_nullable_actual_the_reading_is_unchanged`: a list with no
  nullable actual classifies exactly as `CallbackContracts.classifyModes`.

`classifyVectors` and `decisionVectors` are `NULLABLE_CLASSIFY_VECTORS` and
`NULLABLE_DECISION_VECTORS` of `callback_contracts.rs`, bound by its unit
tests.
-/

namespace QCore.CallbackSuppliedNull

/-- A candidate's mode at one position, nullability kept. -/
inductive Mode where
  | borrowed
  | ownedNonnull
  | ownedNullable
  | released
  | other
  deriving DecidableEq, Repr

/-- What the callback receives at one position. -/
inductive Actual where
  | parameter (i : Nat)
  | supplied
  | suppliedNullable
  | element
  deriving DecidableEq, Repr

/-- The class of a candidate at one invocation. -/
inductive Class where
  | identity
  | releasesElement
  | effectful
  | requiresNonNull
  deriving DecidableEq, Repr

/-- Whether one position is admitted at all (as at `_`). -/
def admitted : Mode × Actual → Bool
  | (.borrowed, _) => true
  | (.ownedNonnull, _) => true
  | (.ownedNullable, _) => true
  | (.released, .element) => true
  | _ => false

/-- A requirement of a live non-null object where the callee may pass NULL. -/
def nonnullAtNullable : Mode × Actual → Bool
  | (.ownedNonnull, .suppliedNullable) => true
  | _ => false

/-- Whether a position releases. -/
def releases (p : Mode × Actual) : Bool := p.1 == .released

/-- `classify` of `callback_contracts.rs` over a plain contract of the right
arity, restricted to these actuals. -/
def classify (ms : List (Mode × Actual)) : Class :=
  if !ms.all admitted then .effectful
  else if ms.any nonnullAtNullable then .requiresNonNull
  else if ms.any releases then .releasesElement
  else .identity

/-- The decision of one invocation, as `decide` of `callback_contracts.rs`
reads these classes (`elements`: a registry invocation over its elements). -/
def decide (isOpen : Bool) (cs : List Class) (elements : Bool) :
    QCore.CallbackContracts.Decision :=
  if isOpen || cs.isEmpty then .refused
  else if cs.any (fun c => c == .effectful || (!elements && c == .releasesElement)) then .refused
  else if cs.any (· == .requiresNonNull) then .refused
  else if cs.all (· == .identity) then .identity
  else if cs.all (· == .releasesElement) then .releasesElements
  else .refused

/-! ## The refusal -/

/-- A `nonnull` requirement at a nullable actual is never admitted. -/
theorem a_nonnull_requirement_at_a_nullable_actual_is_not_admitted
    (ms : List (Mode × Actual)) (m : (Mode.ownedNonnull, Actual.suppliedNullable) ∈ ms) :
    classify ms = .effectful ∨ classify ms = .requiresNonNull := by
  have h : ms.any nonnullAtNullable = true := List.any_eq_true.mpr ⟨_, m, rfl⟩
  unfold classify
  by_cases ha : (!ms.all admitted) = true
  · exact Or.inl (ite_eq_left ha)
  · right
    rw [ite_eq_right ha, ite_eq_left h]

/-- A candidate requiring a live object where the callee may pass NULL
refuses the call. -/
theorem decide_refuses_a_nonnull_requirement (o : Bool) (cs : List Class) (e : Bool)
    (m : Class.requiresNonNull ∈ cs) : decide o cs e = .refused := by
  have h : cs.any (· == .requiresNonNull) = true := List.any_eq_true.mpr ⟨_, m, by decide⟩
  unfold decide
  by_cases h1 : (o || cs.isEmpty) = true
  · rw [ite_eq_left h1]
  · rw [ite_eq_right h1]
    by_cases h2 : cs.any (fun c => c == .effectful || (!e && c == .releasesElement)) = true
    · rw [ite_eq_left h2]
    · rw [ite_eq_right h2, ite_eq_left h]

/-- An admitted candidate accepts NULL at every nullable actual:
`resource_borrowed` or `resource_owned(nullable)` there. -/
theorem an_admitted_candidate_accepts_null (ms : List (Mode × Actual))
    (h : classify ms = .identity ∨ classify ms = .releasesElement)
    (m : Mode) (mem : (m, Actual.suppliedNullable) ∈ ms) :
    m = .borrowed ∨ m = .ownedNullable := by
  unfold classify at h
  by_cases ha : (!ms.all admitted) = true
  · rw [ite_eq_left ha] at h
    rcases h with h | h <;> cases h
  · rw [ite_eq_right ha] at h
    by_cases hn : ms.any nonnullAtNullable = true
    · rw [ite_eq_left hn] at h
      rcases h with h | h <;> cases h
    · have hall : ms.all admitted = true := by
        cases hx : ms.all admitted <;> simp_all
      have hm := List.all_eq_true.mp hall _ mem
      have hnm : nonnullAtNullable (m, .suppliedNullable) ≠ true :=
        fun hc => hn (List.any_eq_true.mpr ⟨_, mem, hc⟩)
      cases m
      · exact Or.inl rfl
      · exact absurd rfl hnm
      · exact Or.inr rfl
      · exact absurd hm (by decide)
      · exact absurd hm (by decide)

/-! ## Marking never admits more -/

/-- Read `_` as `supplied_nullable`. -/
def mark : Actual → Actual
  | .supplied => .suppliedNullable
  | a => a

/-- Mark every `_` of a list. -/
def markAll (ms : List (Mode × Actual)) : List (Mode × Actual) :=
  ms.map (fun p => (p.1, mark p.2))

theorem admitted_mark (p : Mode × Actual) : admitted (p.1, mark p.2) = admitted p := by
  obtain ⟨m, a⟩ := p
  cases m <;> cases a <;> rfl

theorem releases_mark (p : Mode × Actual) : releases (p.1, mark p.2) = releases p := rfl

theorem nonnull_mark (p : Mode × Actual) (h : nonnullAtNullable p = true) :
    nonnullAtNullable (p.1, mark p.2) = true := by
  obtain ⟨m, a⟩ := p
  cases m <;> cases a <;> simp_all [nonnullAtNullable, mark]

theorem all_admitted_markAll (ms : List (Mode × Actual)) :
    (markAll ms).all admitted = ms.all admitted := by
  induction ms with
  | nil => rfl
  | cons p rest ih =>
    simp only [markAll, List.map_cons, List.all_cons] at *
    rw [admitted_mark, ih]

theorem any_releases_markAll (ms : List (Mode × Actual)) :
    (markAll ms).any releases = ms.any releases := by
  induction ms with
  | nil => rfl
  | cons p rest ih =>
    simp only [markAll, List.map_cons, List.any_cons] at *
    rw [releases_mark, ih]

theorem any_nonnull_markAll (ms : List (Mode × Actual)) (h : ms.any nonnullAtNullable = true) :
    (markAll ms).any nonnullAtNullable = true := by
  obtain ⟨p, mem, hp⟩ := List.any_eq_true.mp h
  exact List.any_eq_true.mpr ⟨(p.1, mark p.2), List.mem_map.mpr ⟨p, mem, rfl⟩, nonnull_mark p hp⟩

/-- **Marking never admits more.** Reading a candidate's `_` actuals as
`supplied_nullable` keeps its class or refuses it as requiring a live
object there. -/
theorem marking_never_admits_more (ms : List (Mode × Actual)) :
    classify (markAll ms) = classify ms ∨ classify (markAll ms) = .requiresNonNull := by
  unfold classify
  rw [all_admitted_markAll, any_releases_markAll]
  by_cases ha : (!ms.all admitted) = true
  · left
    rw [ite_eq_left ha, ite_eq_left ha]
  · rw [ite_eq_right ha, ite_eq_right ha]
    by_cases hm : (markAll ms).any nonnullAtNullable = true
    · right
      rw [ite_eq_left hm]
    · have ho : ¬ ms.any nonnullAtNullable = true := fun hc => hm (any_nonnull_markAll ms hc)
      left
      rw [ite_eq_right hm, ite_eq_right ho]

/-- In particular an admitted marked candidate was admitted unmarked. -/
theorem a_marked_identity_was_an_identity (ms : List (Mode × Actual))
    (h : classify (markAll ms) = .identity) : classify ms = .identity := by
  rcases marking_never_admits_more ms with e | e
  · rw [← e, h]
  · rw [h] at e
    cases e

/-! ## Without a nullable actual, the reading is unchanged -/

/-- Forget nullability: `QCore.CallbackContracts`' modes and actuals. -/
def eraseMode : Mode → QCore.CallbackContracts.Mode
  | .borrowed => .borrowed
  | .ownedNonnull => .owned
  | .ownedNullable => .owned
  | .released => .released
  | .other => .other

def eraseActual : Actual → QCore.CallbackContracts.Actual
  | .parameter i => .parameter i
  | .supplied => .supplied
  | .suppliedNullable => .supplied
  | .element => .element

def erase (p : Mode × Actual) : QCore.CallbackContracts.Mode × QCore.CallbackContracts.Actual :=
  (eraseMode p.1, eraseActual p.2)

/-- Read this module's class as `QCore.CallbackContracts`' one. -/
def Class.embed : Class → QCore.CallbackContracts.CandidateClass
  | .identity => .identity
  | .releasesElement => .releasesElement
  | .effectful => .effectful
  | .requiresNonNull => .effectful

theorem admitted_erase (p : Mode × Actual) :
    QCore.CallbackContracts.admitted (erase p) = admitted p := by
  obtain ⟨m, a⟩ := p
  cases m <;> cases a <;> rfl

theorem releases_erase (p : Mode × Actual) :
    ((erase p).1 == QCore.CallbackContracts.Mode.released) = releases p := by
  obtain ⟨m, a⟩ := p
  cases m <;> rfl

theorem all_admitted_erase (ms : List (Mode × Actual)) :
    (ms.map erase).all QCore.CallbackContracts.admitted = ms.all admitted := by
  induction ms with
  | nil => rfl
  | cons p rest ih =>
    simp only [List.map_cons, List.all_cons]
    rw [admitted_erase, ih]

theorem any_releases_erase (ms : List (Mode × Actual)) :
    (ms.map erase).any (fun p => p.1 == QCore.CallbackContracts.Mode.released)
      = ms.any releases := by
  induction ms with
  | nil => rfl
  | cons p rest ih =>
    simp only [List.map_cons, List.any_cons]
    rw [releases_erase, ih]

/-- A list with no nullable actual classifies exactly as
`CallbackContracts.classifyModes` reads it. -/
theorem without_a_nullable_actual_the_reading_is_unchanged (ms : List (Mode × Actual))
    (h : ∀ p ∈ ms, p.2 ≠ .suppliedNullable) :
    (classify ms).embed = QCore.CallbackContracts.classifyModes (ms.map erase) := by
  have hnn : ¬ ms.any nonnullAtNullable = true := by
    intro hc
    obtain ⟨p, mem, hp⟩ := List.any_eq_true.mp hc
    obtain ⟨m, a⟩ := p
    have hne := h _ mem
    cases m <;> cases a <;> first | exact absurd rfl hne | cases hp
  unfold classify QCore.CallbackContracts.classifyModes
  rw [all_admitted_erase, any_releases_erase]
  by_cases ha : (!ms.all admitted) = true
  · rw [ite_eq_left ha, ite_eq_left ha]
    rfl
  · rw [ite_eq_right ha, ite_eq_right ha, ite_eq_right hnn]
    by_cases hr : ms.any releases = true
    · rw [ite_eq_left hr, ite_eq_left hr]
      rfl
    · rw [ite_eq_right hr, ite_eq_right hr]
      rfl

/-! ## The shared vectors -/

/-- `NULLABLE_CLASSIFY_VECTORS`: positions and the class. -/
def classifyVectors : List (List (Mode × Actual) × Class) := [
  ([(.borrowed, .parameter 0), (.ownedNonnull, .suppliedNullable)], .requiresNonNull),
  ([(.borrowed, .parameter 0), (.ownedNullable, .suppliedNullable)], .identity),
  ([(.borrowed, .suppliedNullable), (.borrowed, .supplied)], .identity),
  ([(.ownedNonnull, .supplied), (.ownedNonnull, .parameter 1)], .identity),
  ([(.released, .suppliedNullable)], .effectful),
  ([(.other, .suppliedNullable)], .effectful),
  ([(.ownedNonnull, .suppliedNullable), (.released, .supplied)], .effectful),
  ([(.ownedNonnull, .suppliedNullable), (.released, .element)], .requiresNonNull)
]

theorem classify_vectors_hold :
    classifyVectors.all (fun (ms, c) => classify ms == c) = true := by
  decide

/-- `NULLABLE_DECISION_VECTORS`: (open?, classes, elements?, kind: 0
identity, 1 releases elements, 2 refused). -/
def decisionVectors : List (Bool × List Class × Bool × Nat) := [
  (false, [.requiresNonNull], false, 2),
  (false, [.identity, .requiresNonNull], false, 2),
  (false, [.identity, .identity], false, 0),
  (false, [.requiresNonNull], true, 2),
  (true, [.identity], false, 2)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun (o, cs, e, k) => (decide o cs e).kind == k) = true := by
  decide

end QCore.CallbackSuppliedNull
