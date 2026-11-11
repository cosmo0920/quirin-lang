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

namespace QCore.CallbackBinding

/-! Bounded acceptance boundary for callback execution refinement.
This account proves scope equality, unique path selection, and retained
obligations. It does not prove the Rust evaluator, C memory semantics, native
instrumentation, authentication, or allocation provenance. -/

structure Scope where
  event : Nat
  base : Nat
  dispatcher : Nat
  thread : Nat
  argument : Nat
  descriptor : Int
  generation : Nat
  deriving DecidableEq, Repr

structure PathResult where
  guards : Bool
  memory : Bool
  calls : Bool
  contract : Bool
  deriving DecidableEq, Repr

def refines (path : PathResult) : Bool :=
  path.guards && path.memory && path.calls && path.contract

def selected (paths : List PathResult) : List PathResult := paths.filter refines

def accept (registered observed : Scope) (paths : List PathResult) : Bool :=
  decide (registered = observed) && decide ((selected paths).length = 1)

theorem accepted_scope_is_registered
    {registered observed : Scope} {paths : List PathResult}
    (accepted : accept registered observed paths = true) :
    registered = observed := by
  have facts : registered = observed ∧ (selected paths).length = 1 := by
    simpa [accept] using accepted
  exact facts.1

theorem accepted_has_exactly_one_refining_path
    {registered observed : Scope} {paths : List PathResult}
    (accepted : accept registered observed paths = true) :
    (selected paths).length = 1 := by
  have facts : registered = observed ∧ (selected paths).length = 1 := by
    simpa [accept] using accepted
  exact facts.2

theorem selected_path_retains_all_obligations
    {paths : List PathResult} {path : PathResult}
    (member : path ∈ selected paths) :
    path.guards = true ∧ path.memory = true ∧
      path.calls = true ∧ path.contract = true := by
  have valid := (List.mem_filter.mp member).2
  simpa [refines, Bool.and_eq_true, and_assoc] using valid

theorem changed_scope_is_rejected
    (registered observed : Scope) (paths : List PathResult)
    (different : registered ≠ observed) :
    accept registered observed paths = false := by
  simp [accept, different]

def unknownPaths (total : Nat) (checked : List Nat) : List Nat :=
  (List.range total).filter (fun index => !checked.contains index)

theorem uncovered_path_remains_unknown
    {total index : Nat} {checked : List Nat}
    (bounded : index < total) (uncovered : index ∉ checked) :
    index ∈ unknownPaths total checked := by
  simp [unknownPaths, bounded, uncovered]

theorem checked_path_is_not_unknown
    {total index : Nat} {checked : List Nat} (covered : index ∈ checked) :
    index ∉ unknownPaths total checked := by
  simp [unknownPaths, covered]


/-! Typed memory/symbol binding operations. The model uses mathematical Int
values; the Rust LP64 profile additionally rejects out-of-range arithmetic. -/

structure TypedValue where
  typeTag : String
  value : Int
  deriving DecidableEq, Repr

structure MemoryState where
  live : Nat → Bool
  memory : Nat → String → Option TypedValue
  symbols : List TypedValue

-- A source read can bind only the exact value of a live, typed snapshot cell.
def bindRead (state : MemoryState) (object : Nat) (field : String)
    (observed : TypedValue) : Option MemoryState :=
  if state.live object && decide (state.memory object field = some observed) then
    some { state with symbols := state.symbols ++ [observed] }
  else none

theorem accepted_read_binds_live_typed_cell
    {state next : MemoryState} {object : Nat} {field : String} {value : TypedValue}
    (accepted : bindRead state object field value = some next) :
    state.live object = true ∧ state.memory object field = some value := by
  unfold bindRead at accepted
  split at accepted
  · rename_i valid
    simpa only [Bool.and_eq_true, decide_eq_true_eq] using valid
  · contradiction

theorem accepted_read_appends_exact_symbol
    {state next : MemoryState} {object : Nat} {field : String} {value : TypedValue}
    (accepted : bindRead state object field value = some next) :
    next.symbols = state.symbols ++ [value] := by
  unfold bindRead at accepted
  split at accepted
  · cases accepted
    rfl
  · contradiction

-- Explicit producer lookup: absent, future, or differently typed symbols fail.
def bindSymbol (state : MemoryState) (index : Nat) (typeTag : String) : Option Int :=
  match state.symbols[index]? with
  | some value => if value.typeTag = typeTag then some value.value else none
  | none => none

theorem absent_symbol_is_rejected (state : MemoryState) (index : Nat) (typeTag : String)
    (absent : state.symbols[index]? = none) :
    bindSymbol state index typeTag = none := by
  simp [bindSymbol, absent]

def storeCell (state : MemoryState) (object : Nat) (field : String)
    (value : TypedValue) : MemoryState :=
  { state with memory := fun other member =>
      if other = object ∧ member = field then some value else state.memory other member }

def bindWrite (state : MemoryState) (object : Nat) (field : String)
    (value : TypedValue) : Option MemoryState :=
  if state.live object then
    match state.memory object field with
    | some old => if old.typeTag = value.typeTag then some (storeCell state object field value) else none
    | none => none
  else none

theorem store_changes_exact_cell (state : MemoryState) (object : Nat)
    (field : String) (value : TypedValue) :
    (storeCell state object field value).memory object field = some value := by
  simp [storeCell]

theorem store_preserves_other_cells (state : MemoryState) (object other : Nat)
    (field member : String) (value : TypedValue)
    (different : ¬(other = object ∧ member = field)) :
    (storeCell state object field value).memory other member = state.memory other member := by
  simp [storeCell, different]

theorem write_rejects_absent_cell (state : MemoryState) (object : Nat)
    (field : String) (value : TypedValue) (absent : state.memory object field = none) :
    bindWrite state object field value = none := by
  simp [bindWrite, absent]

def releaseObject (state : MemoryState) (object : Nat) : MemoryState :=
  { state with
    live := fun other => if other = object then false else state.live other
    memory := fun other field => if other = object then none else state.memory other field }

def bindRelease (state : MemoryState) (object : Nat) : Option MemoryState :=
  if state.live object then some (releaseObject state object) else none

theorem release_clears_all_object_cells (state : MemoryState) (object : Nat) (field : String) :
    (releaseObject state object).memory object field = none := by
  simp [releaseObject]

theorem release_preserves_other_objects (state : MemoryState) (object other : Nat)
    (field : String) (different : other ≠ object) :
    (releaseObject state object).memory other field = state.memory other field ∧
      (releaseObject state object).live other = state.live other := by
  simp [releaseObject, different]

theorem release_prevents_subsequent_read (state : MemoryState) (object : Nat)
    (field : String) (value : TypedValue) :
    bindRead (releaseObject state object) object field value = none := by
  simp [bindRead, releaseObject]

theorem release_prevents_subsequent_write (state : MemoryState) (object : Nat)
    (field : String) (value : TypedValue) :
    bindWrite (releaseObject state object) object field value = none := by
  simp [bindWrite, releaseObject]

theorem release_prevents_duplicate_release (state : MemoryState) (object : Nat) :
    bindRelease (releaseObject state object) object = none := by
  simp [bindRelease, releaseObject]

structure CallObservation where
  function : String
  arguments : List TypedValue
  result : TypedValue
  sequence : Nat
  deriving DecidableEq, Repr

-- Matching consumes the head, so effect records cannot be silently reordered.
def bindCall (expected : CallObservation) (remaining : List CallObservation) :
    Option (List CallObservation) :=
  match remaining with
  | observed :: rest => if expected = observed then some rest else none
  | [] => none

theorem accepted_call_consumes_exact_next_record
    {expected : CallObservation} {remaining next : List CallObservation}
    (accepted : bindCall expected remaining = some next) :
    remaining = expected :: next := by
  cases remaining with
  | nil => simp [bindCall] at accepted
  | cons observed rest =>
    by_cases same : expected = observed
    · simp [bindCall, same] at accepted
      subst next
      simp [same]
    · simp [bindCall, same] at accepted

theorem accepted_write_preserves_binding
    {state next : MemoryState} {object : Nat} {field : String} {value : TypedValue}
    (accepted : bindWrite state object field value = some next) :
    ∃ old, state.live object = true ∧ state.memory object field = some old ∧
      old.typeTag = value.typeTag ∧ next = storeCell state object field value := by
  cases live : state.live object with
  | false => simp [bindWrite, live] at accepted
  | true =>
    cases cell : state.memory object field with
    | none => simp [bindWrite, live, cell] at accepted
    | some old =>
      by_cases same : old.typeTag = value.typeTag
      · simp [bindWrite, live, cell, same] at accepted
        exact ⟨old, rfl, rfl, same, accepted.symm⟩
      · simp [bindWrite, live, cell, same] at accepted

theorem accepted_release_requires_live_object
    {state next : MemoryState} {object : Nat}
    (accepted : bindRelease state object = some next) :
    state.live object = true ∧ next = releaseObject state object := by
  unfold bindRelease at accepted
  split at accepted
  · rename_i live
    cases accepted
    exact ⟨live, rfl⟩
  · contradiction

/-! Evidence origin is retained independently of path refinement. Exercising a
path with a controlled failure cannot count as an unmodified native call result.
Origins are supplied by the observer; these facts do not authenticate it. -/
inductive EvidenceOrigin where
  | native
  | injectedFailure
  deriving DecidableEq, Repr

structure CheckedExecution where
  path : Nat
  origin : EvidenceOrigin
  deriving DecidableEq, Repr

def nativePaths (executions : List CheckedExecution) : List Nat :=
  (executions.filter (fun execution => decide (execution.origin = .native))).map CheckedExecution.path

def observedPaths (executions : List CheckedExecution) : List Nat :=
  executions.map CheckedExecution.path

theorem native_path_has_native_execution
    {executions : List CheckedExecution} {path : Nat}
    (member : path ∈ nativePaths executions) :
    ∃ execution ∈ executions, execution.path = path ∧ execution.origin = .native := by
  obtain ⟨execution, present, same⟩ := List.mem_map.mp member
  have facts := List.mem_filter.mp present
  exact ⟨execution, facts.1, same, by simpa using facts.2⟩

theorem injected_path_is_observed_but_not_native (path : Nat) :
    path ∈ observedPaths [{ path, origin := .injectedFailure }] ∧
    path ∉ nativePaths [{ path, origin := .injectedFailure }] := by
  simp [observedPaths, nativePaths]

theorem native_path_is_observed
    {executions : List CheckedExecution} {path : Nat}
    (member : path ∈ nativePaths executions) : path ∈ observedPaths executions := by
  obtain ⟨execution, present, same, _⟩ := native_path_has_native_execution member
  exact List.mem_map.mpr ⟨execution, present, same⟩

end QCore.CallbackBinding
