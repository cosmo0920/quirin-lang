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

import Std

/-! Bounded scalar-route precedence. Extraction and Rust correspondence are
premises. Both readers denote the same scalar effect; their obligation lists
are independent. A typed syntax candidate alone cannot be selected. -/
namespace QCore.ScalarPrecedence

structure Effect where
  destination : Option String
  frame : Option (List String)
  deriving DecidableEq, Repr

structure Plan where
  effect : Effect
  scalarOnly : Bool
  obligations : List Bool
  aliases : List Bool
  deriving DecidableEq, Repr

def discharged (p : Plan) : Bool := p.obligations.all id && p.aliases.all id

/-- Old alias obligations survive even where the typed reader has fewer. -/
def prepared (old typed : Plan) : Plan :=
  { typed with aliases := typed.aliases ++ old.aliases }

/-- Missing facts, different effects and pointer effects never install a plan. -/
def candidate (old : Plan) (typed : Option Plan) : Option Plan :=
  typed.bind fun t =>
    if t.scalarOnly && decide (t.effect = old.effect) then some (prepared old t) else none

/-- Every attempted plan is checked on the same incoming state. A failed probe
has no state/log/diagnostic effects; the exact old plan executes once. -/
def select (old : Plan) (typed : Option Plan) : Plan :=
  if discharged old then old else
    match candidate old typed with
    | some t => if discharged t then t else old
    | none => old

theorem no_candidate_keeps_the_old_plan (old : Plan) : select old none = old := by
  simp [select, candidate]

theorem refused_probe_keeps_the_old_plan (old : Plan) (typed : Option Plan)
    (t : Plan) (hc : candidate old typed = some t) (hd : discharged t = false) :
    select old typed = old := by
  simp [select, hc, hd]

/-- No older admitting route, its diagnostics or its premise inventory changes. -/
theorem every_old_admission_stands (old : Plan) (typed : Option Plan)
    (ho : discharged old = true) : select old typed = old := by
  simp [select, ho]

theorem selection_preserves_the_effect (old : Plan) (typed : Option Plan) :
    (select old typed).effect = old.effect := by
  by_cases ho : discharged old = true
  · rw [every_old_admission_stands old typed ho]
  · cases typed with
    | none => rw [no_candidate_keeps_the_old_plan]
    | some t =>
      by_cases ht : t.scalarOnly = true ∧ t.effect = old.effect
      · have hc : candidate old (some t) = some (prepared old t) := by
          simp [candidate, ht.1, ht.2]
        rw [select, ite_eq_right ho, hc]
        dsimp only
        by_cases hd : discharged (prepared old t) = true
        · rw [ite_eq_left hd]
          exact ht.2
        · rw [ite_eq_right hd]
      · have hc : candidate old (some t) = none := by
          simpa [candidate] using ht
        simp [select, ho, hc]

/-- A different outcome cannot be selected by syntax admission alone. -/
theorem a_changed_selection_is_fully_discharged (old : Plan) (typed : Option Plan)
    (hc : select old typed ≠ old) : discharged (select old typed) = true := by
  unfold select at hc ⊢
  by_cases ho : discharged old = true
  · simp [ho] at hc
  · simp only [ho] at hc ⊢
    cases ht : candidate old typed with
    | none => simp [ht] at hc
    | some t =>
      simp only [ht] at hc ⊢
      by_cases hd : discharged t = true
      · simp [hd]
      · simp [hd] at hc

/-- Composition needs each reader's own soundness premise; the old reader's
failed obligation is never taken as evidence for the independent reader. -/
theorem independent_readers_compose (SoundRead : Plan → Prop) (old : Plan)
    (typed : Option Plan)
    (oldSound : discharged old = true → SoundRead old)
    (typedSound : ∀ t, candidate old typed = some t → discharged t = true → SoundRead t)
    (hd : discharged (select old typed) = true) : SoundRead (select old typed) := by
  by_cases ho : discharged old = true
  · rw [every_old_admission_stands old typed ho]
    exact oldSound ho
  · unfold select at hd ⊢
    simp only [ho] at hd ⊢
    cases hc : candidate old typed with
    | none => simp [hc, ho] at hd
    | some t =>
      simp only [hc] at hd ⊢
      by_cases ht : discharged t = true
      · simp only [ht, ↓reduceIte]
        exact typedSound t hc ht
      · simp [ht, ho] at hd

theorem selected_typed_plan_retains_old_aliases (old typed : Plan)
    (a : Bool) (ha : a ∈ old.aliases) : a ∈ (prepared old typed).aliases := by
  exact List.mem_append_right _ ha

theorem a_failed_original_alias_refuses_the_prepared_plan (old typed : Plan)
    (ha : false ∈ old.aliases) : discharged (prepared old typed) = false := by
  simp only [discharged, prepared, Bool.and_eq_false_iff]
  right
  simp only [List.all_eq_false]
  exact ⟨false, List.mem_append_right _ ha, by decide⟩

/-- Effect execution is performed after selection, once; a probe never executes it. -/
def execute (write : Effect → Nat → Nat) (state : Nat) (old : Plan) (typed : Option Plan) : Nat :=
  write (select old typed).effect state

theorem effects_execute_once (write : Effect → Nat → Nat) (state : Nat)
    (old : Plan) (typed : Option Plan) :
    execute write state old typed = write old.effect state := by
  simp [execute, selection_preserves_the_effect]

end QCore.ScalarPrecedence
