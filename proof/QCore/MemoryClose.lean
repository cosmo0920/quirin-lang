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

namespace QCore.MemoryClose

/-! Ordered abstract effects corresponding to the source-derived memory-close
composition. Live/disjoint ownership, link membership, checked dependencies,
exclusive access and C/ABI correspondence remain premises. -/
structure State where
  context : Nat
  count : Nat
  reset : Bool := false
  backend : Bool := true
  primary : Bool := true
  secondary : Bool := true
  name : Bool := true
  chunk : Bool := true
  counted : Bool := true
  deriving DecidableEq, Repr
inductive Op where
  | reset
  | backend
  | primary
  | secondary
  | name
  | chunk
  | counter (savedContext : Nat)
  deriving DecidableEq, Repr

def step (state : State) : Op → Option State
  | .reset => if state.chunk && !state.reset then some {state with reset := true} else none
  | .backend => if state.reset && state.chunk && state.backend then some {state with backend := false} else none
  | .primary => if !state.backend && state.chunk && state.primary then some {state with primary := false} else none
  | .secondary => if !state.backend && state.chunk && state.secondary then some {state with secondary := false} else none
  | .name => if !state.backend && !state.primary && !state.secondary && state.chunk && state.name then some {state with name := false} else none
  | .chunk => if !state.backend && !state.primary && !state.secondary && !state.name && state.chunk then some {state with chunk := false} else none
  | .counter saved => if saved = state.context ∧ state.chunk = false ∧ state.counted = true ∧ 0 < state.count then some {state with count := state.count - 1, counted := false} else none

def run : State → List Op → Option State
  | state, [] => some state
  | state, op :: rest => (step state op).bind (fun next => run next rest)

def memoryTrace (context : Nat) : List Op :=
  [.reset, .backend, .primary, .secondary, .name, .chunk, .counter context]

theorem source_order_discharges_memory_chunk (context count : Nat) (positive : 0 < count) :
    run {context, count} (memoryTrace context) = some {
      context, count := count - 1, reset := true, backend := false,
      primary := false, secondary := false, name := false, chunk := false,
      counted := false } := by
  simp [run, memoryTrace, step, positive]

theorem accepted_counter_uses_saved_context {before after : State} {saved : Nat}
    (accepted : step before (.counter saved) = some after) : saved = before.context := by
  simp only [step] at accepted
  split at accepted
  next premises => exact premises.1
  next => contradiction

theorem accepted_counter_requires_chunk_discharge {before after : State} {saved : Nat}
    (accepted : step before (.counter saved) = some after) : before.chunk = false := by
  simp only [step] at accepted
  split at accepted
  next premises => exact premises.2.1
  next => contradiction

theorem accepted_counter_decrements_once {before after : State} {saved : Nat}
    (accepted : step before (.counter saved) = some after) :
    after.count + 1 = before.count ∧ after.counted = false := by
  simp only [step] at accepted
  split at accepted
  next premises =>
    cases accepted
    simp only [and_true]
    omega
  next => contradiction

theorem duplicate_counter_rejected {before after : State} {saved : Nat}
    (accepted : step before (.counter saved) = some after) : step after (.counter saved) = none := by
  have discharged := (accepted_counter_decrements_once accepted).2
  simp [step, discharged]

theorem empty_trace_preserves_state (state : State) : run state [] = some state := rfl

end QCore.MemoryClose
