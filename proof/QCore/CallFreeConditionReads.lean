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

import QCore.ConditionCallDesugar

/-! W48-BOTTOM-UP-BATCH4. The events of the existing Boolean tree are
checked read instructions, rather than calls. Existing leaf typing and flow
checks supply each event. No scalar value, liveness or nullness is assumed
by composition. Source extraction and Rust correspondence remain premises. -/
namespace QCore.CallFreeConditionReads

open ConditionCallDesugar

def keep (typed elaborated checkedRead : Bool) : Bool :=
  typed || elaborated || checkedRead

def vectors : List Bool := [false, true, true, true, true, true, true, true]

def evaluatedVectors : List Bool :=
  [false, true].flatMap fun a => [false, true].flatMap fun b =>
  [false, true].map fun c => keep a b c

theorem vectors_match : evaluatedVectors = vectors := by decide

theorem old_admissions_survive (a b c : Bool) (h : a || b = true) :
    keep a b c = true := by
  simp only [keep, Bool.or_eq_true] at *
  exact Or.inl h

theorem a_checked_read_keeps_the_complete_plan (a b : Bool) :
    keep a b true = true := by simp [keep]

theorem a_plan_without_a_reason_refuses : keep false false false = false := by decide

/-- Check each read in trace order. All complete states and diagnostics are
    retained; no notion of successful caller safety is introduced. -/
def checkTrace {α σ ε : Type} (step : α → σ → σ × List ε) :
    List α → σ → σ × List ε
  | [], state => (state, [])
  | event :: rest, state =>
      let first := step event state
      let later := checkTrace step rest first.1
      (later.1, first.2 ++ later.2)

/-- Every assignment of leaf values has the same checked-read events and
    final Boolean answer in the assembled graph as in C's Boolean tree. -/
theorem the_graph_preserves_checked_read_order {α : Type}
    (v : Nat → Bool) (tree : Cond α) :
    run v (desugar tree) = eval v tree :=
  the_desugaring_is_c_evaluation v tree

/-- Equality of event traces yields equality of the complete checked state
    and diagnostic list, for an arbitrary (possibly refusing) read checker. -/
theorem the_graph_preserves_state_and_all_read_errors {α σ ε : Type}
    (step : α → σ → σ × List ε) (state : σ)
    (v : Nat → Bool) (tree : Cond α) :
    checkTrace step (run v (desugar tree)).1 state =
      checkTrace step (eval v tree).1 state := by
  rw [the_graph_preserves_checked_read_order]

theorem an_evaluated_read_error_is_not_erased {α σ ε : Type}
    (step : α → σ → σ × List ε) (event : α) (rest : List α)
    (state : σ) (error : ε) (h : error ∈ (step event state).2) :
    error ∈ (checkTrace step (event :: rest) state).2 := by
  simp only [checkTrace]
  exact List.mem_append_left _ h

/-- A skipped right operand supplies no read, while an evaluated one does. -/
theorem short_circuit_reads {α : Type} (v : Nat → Bool) (a b : Cond α) :
    (run v (desugar (.and a b))).1 =
      (eval v a).1 ++ (if (eval v a).2 then (eval v b).1 else []) := by
  rw [the_graph_preserves_checked_read_order]
  exact and_evaluates_its_right_operand_only_when_the_left_holds v a b

end QCore.CallFreeConditionReads
