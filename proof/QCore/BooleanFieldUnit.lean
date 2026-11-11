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

import QCore.ChunkSource

namespace QCore.ChunkSource.UnitResult

/-! Typed void-result extension. Completion is a control flag, never a scalar
result. Clang/ABI validation and AST translation remain explicit trusted inputs;
the existing integer-result model is unchanged. -/

inductive Statement where
  | empty
  | sequence (first rest : Statement)
  | branch (condition : Expr) (yes no : Statement)
  | assign (value : Expr)
  | ret
  | frameAssign (field : String) (value : Int)
  deriving Repr, DecidableEq

def Statement.eval : Statement → State → Option (State × Bool)
  | .empty, state => some (state, false)
  | .sequence first rest, state => do
      let (next, stopped) ← first.eval state
      if stopped then some (next, true) else rest.eval next
  | .branch condition yes no, state =>
      let taken := condition.eval state.value != 0
      let next := { state with branches := state.branches ++ [taken] }
      if taken then yes.eval next else no.eval next
  | .assign value, state => do
      let (next, _) ← (Stmt.assign value).eval state
      some (next, false)
  | .ret, state => some (state, true)
  | .frameAssign field value, state =>
      some ({ state with frameWrites := state.frameWrites ++ [(field, value)] }, false)

def run (statement : Statement) (initial : Bool) : Option State := do
  let (state, _) ← statement.eval { value := initial }
  pure state

structure Summary where
  whenFalse : State
  whenTrue : State
  deriving Repr, DecidableEq

def summarize (statement : Statement) : Option Summary := do
  let whenFalse ← run statement false
  let whenTrue ← run statement true
  pure { whenFalse, whenTrue }

def Summary.select (summary : Summary) (initial : Bool) : State :=
  if initial then summary.whenTrue else summary.whenFalse

theorem accepted_summary_covers_both_prestates
    {statement : Statement} {summary : Summary}
    (accepted : summarize statement = some summary) (initial : Bool) :
    run statement initial = some (summary.select initial) := by
  unfold summarize at accepted
  cases hf : run statement false with
  | none => simp [hf] at accepted
  | some f =>
    cases ht : run statement true with
    | none => simp [hf, ht] at accepted
    | some t =>
      simp [hf, ht] at accepted
      subst summary
      cases initial <;> simp [Summary.select, hf, ht]

def checkContract (summary : Summary) (post : Bool → State → Bool) : Bool :=
  post false summary.whenFalse && post true summary.whenTrue

theorem accepted_contract_holds_on_every_prestate
    {statement : Statement} {summary : Summary} {post : Bool → State → Bool}
    (imported : summarize statement = some summary)
    (checked : checkContract summary post = true) (initial : Bool) :
    run statement initial = some (summary.select initial) ∧
      post initial (summary.select initial) = true := by
  refine ⟨accepted_summary_covers_both_prestates imported initial, ?_⟩
  have both : post false summary.whenFalse = true ∧ post true summary.whenTrue = true := by
    simpa [checkContract] using checked
  cases initial
  · exact both.1
  · exact both.2

theorem void_fallthrough_preserves_state (initial : Bool) :
    run .empty initial = some { value := initial } := rfl

theorem return_stops_all_later_effects (rest : Statement) (state : State) :
    (Statement.sequence .ret rest).eval state = some (state, true) := rfl

theorem setting_true_has_no_scalar_result (initial : Bool) :
    run (.assign (.literal 1)) initial =
      some { value := true, writes := [true] } := by
  simp [run, Statement.eval, Stmt.eval, Expr.eval]

theorem invalid_boolean_store_rejects (initial : Bool) :
    run (.assign (.literal 2)) initial = none := by
  simp [run, Statement.eval, Stmt.eval, Expr.eval]

theorem metadata_write_preserves_selected_field (name : String) (value : Int)
    (state : State) : (Statement.frameAssign name value).eval state =
      some ({ state with frameWrites := state.frameWrites ++ [(name, value)] }, false) := rfl

theorem integer_fallthrough_is_still_rejected (initial : Bool) :
    QCore.ChunkSource.run .empty initial = none := rfl

end QCore.ChunkSource.UnitResult
