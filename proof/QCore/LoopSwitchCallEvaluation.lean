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

namespace QCore.LoopSwitchCallEvaluation

/-! Card `B03-LOOP-AND-SWITCH`: evaluation of one modeled contract call in a
C controlling expression.

This additional gateway route requires an externally declared callee with no
definition in the translation unit; provisional local summaries are excluded.
The gateway admits a call only after a lexical check establishes that it is
unconditionally evaluated within the controlling expression and the remaining
expression is custody-neutral. This module models where the call instruction
sits in the CFG. It distinguishes normal condition termination from an abrupt
`break`/`return`, because an abrupt body exit skips the next condition test.

This is an abstract rule theorem. Faithful C extraction and correspondence to
the Rust CFG builder remain review-maintained premises (constraint 28).
-/

inductive LoopKind where
  | whileLoop
  | forLoop
  | doWhileLoop
  deriving DecidableEq, Repr

inductive ExitKind where
  | conditionFalse
  | abruptBodyExit
  deriving DecidableEq, Repr

/-- Number of condition evaluations on a path that enters the body `entries`
times. A pre-test loop evaluates before each entry and once more only when its
condition terminates the loop. A post-test loop evaluates after each normally
completed body; an abrupt final body exit skips that last evaluation. -/
def evaluations (kind : LoopKind) (exit : ExitKind) (entries : Nat) : Nat :=
  match kind, exit with
  | .whileLoop, .conditionFalse => entries + 1
  | .forLoop, .conditionFalse => entries + 1
  | .whileLoop, .abruptBodyExit => entries
  | .forLoop, .abruptBodyExit => entries
  | .doWhileLoop, .conditionFalse => entries
  | .doWhileLoop, .abruptBodyExit => entries - 1

/-- One effect record per visit to the modeled call node. -/
def effectTrace (effect : Nat) (kind : LoopKind) (exit : ExitKind)
    (bodyEntries : Nat) : List Nat :=
  List.replicate (evaluations kind exit bodyEntries) effect

theorem pre_test_normal_exit_has_one_final_evaluation
    (kind : LoopKind) (isPre : kind ≠ .doWhileLoop) (entries : Nat) :
    evaluations kind .conditionFalse entries = entries + 1 := by
  cases kind <;> simp_all [evaluations]

theorem pre_test_abrupt_exit_has_no_final_evaluation
    (kind : LoopKind) (isPre : kind ≠ .doWhileLoop) (entries : Nat) :
    evaluations kind .abruptBodyExit entries = entries := by
  cases kind <;> simp_all [evaluations]

theorem post_test_normal_exit_evaluates_after_every_body
    (entries : Nat) :
    evaluations .doWhileLoop .conditionFalse entries = entries := by
  rfl

theorem post_test_abrupt_exit_skips_the_last_evaluation
    {entries : Nat} (entered : 0 < entries) :
    evaluations .doWhileLoop .abruptBodyExit entries + 1 = entries := by
  simp [evaluations, Nat.sub_add_cancel (by omega : 1 ≤ entries)]

theorem every_condition_visit_applies_the_modeled_effect
    (effect : Nat) (kind : LoopKind) (exit : ExitKind) (entries : Nat) :
    (effectTrace effect kind exit entries).length = evaluations kind exit entries := by
  simp [effectTrace]

/-- The condition node remains a two-way branch. Modeling the call changes its
instruction, never its exit/back-edge policy. -/
def conditionSuccessors (bodyEntry continuation : Nat) : List Nat :=
  [bodyEntry, continuation]

theorem a_modeled_call_never_removes_the_loop_exit
    (bodyEntry continuation : Nat) :
    continuation ∈ conditionSuccessors bodyEntry continuation := by
  simp [conditionSuccessors]

theorem a_modeled_call_keeps_the_loop_body_edge
    (bodyEntry continuation : Nat) :
    bodyEntry ∈ conditionSuccessors bodyEntry continuation := by
  simp [conditionSuccessors]

/-- The CFG route taken by `continue`. A `for` loop executes its increment
node before revisiting the condition call; `while` and `do`/`while` revisit the
condition call directly. -/
def continueRoute (kind : LoopKind) (incrementNode conditionNode : Nat) : List Nat :=
  match kind with
  | .forLoop => [incrementNode, conditionNode]
  | .whileLoop | .doWhileLoop => [conditionNode]

theorem every_continue_route_revisits_the_call
    (kind : LoopKind) (incrementNode conditionNode : Nat) :
    conditionNode ∈ continueRoute kind incrementNode conditionNode := by
  cases kind <;> simp [continueRoute]

theorem a_for_continue_runs_increment_before_the_call
    (incrementNode conditionNode : Nat) :
    continueRoute .forLoop incrementNode conditionNode =
      [incrementNode, conditionNode] := by
  rfl

/-- Switch dispatch evaluates its controlling expression exactly once; case
count and fallthrough do not create an edge back to the dispatch call. -/
def switchEffectTrace (effect : Nat) : List Nat := [effect]

theorem switch_dispatch_applies_the_effect_once (effect : Nat) :
    (switchEffectTrace effect).count effect = 1 := by
  simp [switchEffectTrace]

end QCore.LoopSwitchCallEvaluation
