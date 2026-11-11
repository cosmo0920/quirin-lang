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

/-! Explicit assertion positions. Predicates and execution correspondence are
inputs; declaring a condition proves neither its truth nor termination. -/
namespace QCore.QuantCondition
inductive Selector where
  | entry | normal | error (name : String) | any
  deriving DecidableEq, Repr
inductive Exit (State : Type u) where
  | normal (state : State)
  | error (name : String) (state : State)
/-- `none` means no terminating exit of this execution, not missing evidence.
Actual execution interpretation and completeness require independent inputs. -/
structure Execution (State : Type u) where
  entryState : State
  exitState : Option (Exit State)
def Satisfies (selector : Selector) (predicate : State → Prop)
    (execution : Execution State) : Prop :=
  match selector with
  | .entry => predicate execution.entryState
  | .normal => match execution.exitState with
    | some (.normal state) => predicate state
    | _ => True
  | .error expected => match execution.exitState with
    | some (.error actual state) => actual = expected → predicate state
    | _ => True
  | .any => match execution.exitState with
    | some (.normal state) | some (.error _ state) => predicate state
    | none => True

theorem any_covers_normal (predicate : State → Prop) (execution : Execution State)
    (allExits : Satisfies .any predicate execution)
    (returned : execution.exitState = some (.normal state)) : predicate state := by
  simpa [Satisfies, returned] using allExits

theorem any_covers_error (predicate : State → Prop) (execution : Execution State)
    (allExits : Satisfies .any predicate execution)
    (raised : execution.exitState = some (.error name state)) : predicate state := by
  simpa [Satisfies, raised] using allExits

theorem error_selection_is_exact (predicate : State → Prop) (execution : Execution State)
    (raised : execution.exitState = some (.error actual state)) (different : actual ≠ expected) :
    Satisfies (.error expected) predicate execution := by
  simp [Satisfies, raised, different]

theorem normal_assertion_does_not_supply_entry :
    Satisfies .normal (fun n : Nat => n = 1) ⟨0, some (.normal 1)⟩ ∧
    ¬ Satisfies .entry (fun n : Nat => n = 1) ⟨0, some (.normal 1)⟩ := by
  simp [Satisfies]

theorem entry_assertion_does_not_supply_normal :
    Satisfies .entry (fun n : Nat => n = 0) ⟨0, some (.normal 1)⟩ ∧
    ¬ Satisfies .normal (fun n : Nat => n = 0) ⟨0, some (.normal 1)⟩ := by
  simp [Satisfies]

def HasExit (execution : Execution State) : Prop := ∃ exit, execution.exitState = some exit

theorem exit_conditions_do_not_prove_termination (before : State) (predicate : State → Prop) :
    Satisfies .any predicate ⟨before, none⟩ ∧ ¬ HasExit (⟨before, none⟩ : Execution State) := by
  simp [Satisfies, HasExit]
/-- Graph-v4 roots retain all explicit entry assertions. Group membership only
controls standalone proposition metadata; it cannot discharge an entry root. -/
def CanonicalRoots {Key : Type u} (entry formula grouped roots : Key → Prop) : Prop :=
  ∀ key, roots key ↔ entry key ∨ (formula key ∧ ¬ grouped key)

theorem every_explicit_entry_is_required {Key : Type u}
    (entry formula grouped roots : Key → Prop)
    (canonical : CanonicalRoots entry formula grouped roots) (key : Key)
    (explicit : entry key) : roots key :=
  (canonical key).mpr (Or.inl explicit)

theorem regrouping_preserves_explicit_entry {Key : Type u}
    (entry formula firstGroups nextGroups firstRoots nextRoots : Key → Prop)
    (first : CanonicalRoots entry formula firstGroups firstRoots)
    (next : CanonicalRoots entry formula nextGroups nextRoots) (key : Key)
    (explicit : entry key) : firstRoots key ∧ nextRoots key :=
  ⟨(first key).mpr (Or.inl explicit), (next key).mpr (Or.inl explicit)⟩

end QCore.QuantCondition
