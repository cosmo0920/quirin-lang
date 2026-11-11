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

namespace QCore.Concurrency

/-! Cycle 14's proof boundary for the restricted concurrent event checker.
Each access record carries evidence computed from conserved permissions or
explicit trusted guard premises and the supported happens-before edges. Lock
and destruction capabilities are indexed by identity and must be unique. -/

abbrev TaskId := Nat
abbrev MutexId := Nat
abbrev ResourceId := Nat

/-! Executable lifetime projection of the Rust event checker. Ordering of prior
accesses is supplied by its vector-clock calculation; this projection does not
prove source-to-trace refinement or manufacture a mutex invariant. -/
namespace Lifetime

structure State where
  knownTasks : List TaskId := [0]
  terminalTasks : List TaskId := []
  destroyedResources : List ResourceId := []
  parents : List (TaskId × TaskId) := [] -- child, parent
  custody : List (ResourceId × TaskId) := []
  permissionHolders : List (ResourceId × TaskId) := [] -- positive shares only
  guardHolders : List TaskId := []
  deriving DecidableEq, Repr

def State.Active (state : State) (task : TaskId) : Prop :=
  task ∈ state.knownTasks ∧ task ∉ state.terminalTasks

def State.Live (state : State) (resource : ResourceId) : Prop :=
  resource ∉ state.destroyedResources

def State.OtherAuthorityQuiescent (state : State) (task : TaskId)
    (resource : ResourceId) : Prop :=
  (state.permissionHolders.all fun holder =>
    decide (holder.1 ≠ resource ∨ holder.2 = task ∨ holder.2 ∈ state.terminalTasks)) = true ∧
  (state.guardHolders.all fun holder => decide (holder = task)) = true

def State.ChildQuiescent (state : State) (child : TaskId) : Prop :=
  child ∉ state.guardHolders ∧
  (state.parents.all fun relation =>
    decide (relation.2 ≠ child ∨ relation.1 ∈ state.terminalTasks)) = true

instance (state : State) (task : TaskId) : Decidable (state.Active task) := by
  unfold State.Active; infer_instance
instance (state : State) (resource : ResourceId) : Decidable (state.Live resource) := by
  unfold State.Live; infer_instance
instance (state : State) (task : TaskId) (resource : ResourceId) :
    Decidable (state.OtherAuthorityQuiescent task resource) := by
  unfold State.OtherAuthorityQuiescent; infer_instance
instance (state : State) (child : TaskId) : Decidable (state.ChildQuiescent child) := by
  unfold State.ChildQuiescent; infer_instance

inductive Event where
  | access (task : TaskId) (resource : ResourceId)
  | transfer (sender to : TaskId) (resource : ResourceId) (synchronized : Bool)
  | destroy (task : TaskId) (resource : ResourceId) (priorAccessesOrdered : Bool)
  | spawn (parent child : TaskId)
  | join (parent child : TaskId)
  | lock (task : TaskId)
  | unlock (task : TaskId)
  | taskAction (task : TaskId)
  deriving DecidableEq, Repr

def step (state : State) : Event → Option State
  | .access task resource =>
      if state.Active task ∧ state.Live resource then some state else none
  | .transfer sender to resource synchronized =>
      if state.Active sender ∧ state.Active to ∧ state.Live resource ∧
          (resource, sender) ∈ state.custody ∧ synchronized = true then
        some { state with custody := (resource, to) ::
          state.custody.filter (fun entry => decide (entry.1 ≠ resource)) }
      else none
  | .destroy task resource priorAccessesOrdered =>
      if state.Active task ∧ state.Live resource ∧ (resource, task) ∈ state.custody ∧
          state.OtherAuthorityQuiescent task resource ∧ priorAccessesOrdered = true then
        some { state with
          destroyedResources := resource :: state.destroyedResources
          custody := state.custody.filter (fun entry => decide (entry.1 ≠ resource)) }
      else none
  | .spawn parent child =>
      if state.Active parent ∧ child ∉ state.knownTasks then
        some { state with
          knownTasks := child :: state.knownTasks
          parents := (child, parent) :: state.parents }
      else none
  | .join parent child =>
      if state.Active parent ∧ state.Active child ∧ (child, parent) ∈ state.parents ∧
          state.ChildQuiescent child ∧ parent ∉ state.guardHolders then
        some { state with terminalTasks := child :: state.terminalTasks }
      else none
  | .lock task =>
      if state.Active task then some { state with guardHolders := task :: state.guardHolders }
      else none
  | .unlock task =>
      if state.Active task ∧ task ∈ state.guardHolders then
        some { state with guardHolders := state.guardHolders.erase task }
      else none
  | .taskAction task => if state.Active task then some state else none

theorem destroyed_resource_rejects_access (state : State) (task : TaskId)
    (resource : ResourceId) (dead : resource ∈ state.destroyedResources) :
    step state (.access task resource) = none := by
  simp [step, State.Live, dead]

theorem destroyed_resource_rejects_transfer (state : State) (sender to : TaskId)
    (resource : ResourceId) (synchronized : Bool)
    (dead : resource ∈ state.destroyedResources) :
    step state (.transfer sender to resource synchronized) = none := by
  simp [step, State.Live, dead]

/-- Destruction is not idempotent: the dead generation rejects a repeat
destruction before any custody or quiescence question is asked. -/
theorem destroyed_resource_rejects_destruction (state : State) (task : TaskId)
    (resource : ResourceId) (ordered : Bool)
    (dead : resource ∈ state.destroyedResources) :
    step state (.destroy task resource ordered) = none := by
  simp [step, State.Live, dead]

/-- Only the current custodian ends a generation; an outstanding capture in
another task leaves the parent without destruction authority. -/
theorem foreign_custody_rejects_destruction (state : State) (task : TaskId)
    (resource : ResourceId) (ordered : Bool)
    (foreign : (resource, task) ∉ state.custody) :
    step state (.destroy task resource ordered) = none := by
  simp [step, foreign]

theorem terminal_task_rejects_access (state : State) (task : TaskId)
    (resource : ResourceId) (terminal : task ∈ state.terminalTasks) :
    step state (.access task resource) = none := by
  simp [step, State.Active, terminal]

theorem terminal_task_rejects_action (state : State) (task : TaskId)
    (terminal : task ∈ state.terminalTasks) :
    step state (.taskAction task) = none := by
  simp [step, State.Active, terminal]

theorem destruction_invalidates_generation {state next : State} {task : TaskId}
    {resource : ResourceId} {ordered : Bool}
    (accepted : step state (.destroy task resource ordered) = some next) :
    resource ∈ next.destroyedResources := by
  simp only [step] at accepted
  split at accepted
  · cases Option.some.inj accepted
    simp
  · contradiction

theorem accepted_destruction_requires_quiescence {state next : State} {task : TaskId}
    {resource : ResourceId} {ordered : Bool}
    (accepted : step state (.destroy task resource ordered) = some next) :
    state.OtherAuthorityQuiescent task resource ∧ ordered = true := by
  simp only [step] at accepted
  split at accepted
  · rename_i h
    exact h.2.2.2
  · contradiction

theorem join_terminates_child {state next : State} {parent child : TaskId}
    (accepted : step state (.join parent child) = some next) :
    child ∈ next.terminalTasks := by
  simp only [step] at accepted
  split at accepted
  · cases Option.some.inj accepted
    simp
  · contradiction

theorem destruction_prevents_later_access {state next : State} {task reader : TaskId}
    {resource : ResourceId} {ordered : Bool}
    (accepted : step state (.destroy task resource ordered) = some next) :
    step next (.access reader resource) = none :=
  destroyed_resource_rejects_access next reader resource
    (destruction_invalidates_generation accepted)

theorem destruction_prevents_later_transfer {state next : State} {task sender to : TaskId}
    {resource : ResourceId} {ordered synchronized : Bool}
    (accepted : step state (.destroy task resource ordered) = some next) :
    step next (.transfer sender to resource synchronized) = none :=
  destroyed_resource_rejects_transfer next sender to resource synchronized
    (destruction_invalidates_generation accepted)

theorem destruction_prevents_repeat_destruction {state next : State} {task second : TaskId}
    {resource : ResourceId} {ordered again : Bool}
    (accepted : step state (.destroy task resource ordered) = some next) :
    step next (.destroy second resource again) = none :=
  destroyed_resource_rejects_destruction next second resource again
    (destruction_invalidates_generation accepted)

theorem join_prevents_child_action {state next : State} {parent child : TaskId}
    (accepted : step state (.join parent child) = some next) :
    step next (.taskAction child) = none :=
  terminal_task_rejects_action next child (join_terminates_child accepted)

/-! Terminal state is decided per event kind before any custody, guard,
permission or parentage check, so retained identity history never restores
execution authority. -/

theorem terminal_task_rejects_lock (state : State) (task : TaskId)
    (terminal : task ∈ state.terminalTasks) :
    step state (.lock task) = none := by
  simp [step, State.Active, terminal]

theorem terminal_task_rejects_unlock (state : State) (task : TaskId)
    (terminal : task ∈ state.terminalTasks) :
    step state (.unlock task) = none := by
  simp [step, State.Active, terminal]

theorem terminal_sender_rejects_transfer (state : State) (sender to : TaskId)
    (resource : ResourceId) (synchronized : Bool)
    (terminal : sender ∈ state.terminalTasks) :
    step state (.transfer sender to resource synchronized) = none := by
  simp [step, State.Active, terminal]

theorem terminal_recipient_rejects_transfer (state : State) (sender to : TaskId)
    (resource : ResourceId) (synchronized : Bool)
    (terminal : to ∈ state.terminalTasks) :
    step state (.transfer sender to resource synchronized) = none := by
  simp [step, State.Active, terminal]

theorem terminal_task_rejects_destroy (state : State) (task : TaskId)
    (resource : ResourceId) (ordered : Bool)
    (terminal : task ∈ state.terminalTasks) :
    step state (.destroy task resource ordered) = none := by
  simp [step, State.Active, terminal]

theorem terminal_parent_rejects_spawn (state : State) (parent child : TaskId)
    (terminal : parent ∈ state.terminalTasks) :
    step state (.spawn parent child) = none := by
  simp [step, State.Active, terminal]

theorem terminal_parent_rejects_join (state : State) (parent child : TaskId)
    (terminal : parent ∈ state.terminalTasks) :
    step state (.join parent child) = none := by
  simp [step, State.Active, terminal]

theorem terminal_child_rejects_join (state : State) (parent child : TaskId)
    (terminal : child ∈ state.terminalTasks) :
    step state (.join parent child) = none := by
  simp [step, State.Active, terminal]

theorem join_prevents_child_lock {state next : State} {parent child : TaskId}
    (accepted : step state (.join parent child) = some next) :
    step next (.lock child) = none :=
  terminal_task_rejects_lock next child (join_terminates_child accepted)

theorem join_prevents_child_unlock {state next : State} {parent child : TaskId}
    (accepted : step state (.join parent child) = some next) :
    step next (.unlock child) = none :=
  terminal_task_rejects_unlock next child (join_terminates_child accepted)

theorem join_prevents_child_transfer {state next : State} {parent child to : TaskId}
    {resource : ResourceId} {synchronized : Bool}
    (accepted : step state (.join parent child) = some next) :
    step next (.transfer child to resource synchronized) = none :=
  terminal_sender_rejects_transfer next child to resource synchronized
    (join_terminates_child accepted)

theorem join_prevents_child_receiving_custody {state next : State} {parent child sender : TaskId}
    {resource : ResourceId} {synchronized : Bool}
    (accepted : step state (.join parent child) = some next) :
    step next (.transfer sender child resource synchronized) = none :=
  terminal_recipient_rejects_transfer next sender child resource synchronized
    (join_terminates_child accepted)

theorem join_prevents_child_destruction {state next : State} {parent child : TaskId}
    {resource : ResourceId} {ordered : Bool}
    (accepted : step state (.join parent child) = some next) :
    step next (.destroy child resource ordered) = none :=
  terminal_task_rejects_destroy next child resource ordered (join_terminates_child accepted)

theorem join_prevents_child_spawn {state next : State} {parent child grandchild : TaskId}
    (accepted : step state (.join parent child) = some next) :
    step next (.spawn child grandchild) = none :=
  terminal_parent_rejects_spawn next child grandchild (join_terminates_child accepted)

theorem join_cannot_be_repeated {state next : State} {parent child : TaskId}
    (accepted : step state (.join parent child) = some next) :
    step next (.join parent child) = none :=
  terminal_child_rejects_join next parent child (join_terminates_child accepted)

/-! A refused join is not an implicit completion. -/

theorem nonquiescent_child_rejects_join (state : State) (parent child : TaskId)
    (busy : ¬ state.ChildQuiescent child) :
    step state (.join parent child) = none := by
  simp [step, busy]

theorem locked_parent_rejects_join (state : State) (parent child : TaskId)
    (locked : parent ∈ state.guardHolders) :
    step state (.join parent child) = none := by
  simp [step, locked]

/-- Refusing a join leaves the child exactly as authorized as before: its own
actions stay enabled and no terminal state is recorded. -/
theorem refused_join_preserves_child_authority (state : State) (parent child : TaskId)
    (busy : ¬ state.ChildQuiescent child) (active : state.Active child) :
    step state (.join parent child) = none ∧
      step state (.taskAction child) = some state ∧ child ∉ state.terminalTasks :=
  ⟨nonquiescent_child_rejects_join state parent child busy, by simp [step, active], active.2⟩

def run (state : State) : List Event → Option State
  | [] => some state
  | event :: rest => (step state event).bind (fun next => run next rest)

theorem step_preserves_destroyed {state next : State} {event : Event}
    {resource : ResourceId} (dead : resource ∈ state.destroyedResources)
    (accepted : step state event = some next) : resource ∈ next.destroyedResources := by
  cases event <;> simp only [step] at accepted
  all_goals split at accepted
  all_goals first | contradiction | (cases Option.some.inj accepted; simp_all)

theorem step_preserves_terminal {state next : State} {event : Event}
    {task : TaskId} (terminal : task ∈ state.terminalTasks)
    (accepted : step state event = some next) : task ∈ next.terminalTasks := by
  cases event <;> simp only [step] at accepted
  all_goals split at accepted
  all_goals first | contradiction | (cases Option.some.inj accepted; simp_all)

theorem run_preserves_destroyed {events : List Event} {state next : State}
    {resource : ResourceId} (dead : resource ∈ state.destroyedResources)
    (accepted : run state events = some next) : resource ∈ next.destroyedResources := by
  induction events generalizing state with
  | nil =>
      simp only [run] at accepted
      cases Option.some.inj accepted
      exact dead
  | cons event rest ih =>
      cases transition : step state event with
      | none => simp [run, transition] at accepted
      | some intermediate =>
          simp only [run, transition, Option.bind_some] at accepted
          exact ih (step_preserves_destroyed dead transition) accepted

theorem run_preserves_terminal {events : List Event} {state next : State}
    {task : TaskId} (terminal : task ∈ state.terminalTasks)
    (accepted : run state events = some next) : task ∈ next.terminalTasks := by
  induction events generalizing state with
  | nil =>
      simp only [run] at accepted
      cases Option.some.inj accepted
      exact terminal
  | cons event rest ih =>
      cases transition : step state event with
      | none => simp [run, transition] at accepted
      | some intermediate =>
          simp only [run, transition, Option.bind_some] at accepted
          exact ih (step_preserves_terminal terminal transition) accepted

example : run { custody := [(7, 0)] }
    [.destroy 0 7 true, .access 0 7] = none := by decide
example : (run { custody := [(7, 0), (8, 0)] }
    [.destroy 0 7 true, .access 0 8]).isSome = true := by decide
example : run {} [.spawn 0 1, .join 0 1, .taskAction 1] = none := by decide
example : run { custody := [(7, 0)] }
    [.spawn 0 1, .lock 1, .destroy 0 7 true] = none := by decide
example : run {} [.spawn 0 1, .lock 1, .join 0 1] = none := by decide
example : (run {} [.spawn 0 1, .taskAction 1, .join 0 1, .taskAction 0]).isSome = true := by decide
example : run { custody := [(7, 0)] }
    [.destroy 0 7 true, .destroy 0 7 true] = none := by decide
example : run { custody := [(7, 0)] }
    [.destroy 0 7 true, .transfer 0 0 7 true] = none := by decide
example : run { custody := [(7, 0)], permissionHolders := [(7, 1)] }
    [.destroy 0 7 true] = none := by decide
example : (run { custody := [(7, 0)], permissionHolders := [(7, 1)], terminalTasks := [1] }
    [.destroy 0 7 true]).isSome = true := by decide
example : run { custody := [(7, 1)] } [.destroy 0 7 true] = none := by decide
example : run { custody := [(7, 0)] } [.destroy 0 7 false] = none := by decide
/-- Distinct generations are independent: destroying one never touches the
other, whatever numeric address the model input reuses. -/
example : (run { custody := [(7, 0), (8, 0)] }
    [.destroy 0 7 true, .destroy 0 8 true]).isSome = true := by decide

end Lifetime

structure AccessCheck where
  authorized : Bool
  conflictsOrdered : Bool
  deriving DecidableEq, Repr

structure ConcurrentState where
  accesses : List AccessCheck
  lockCapabilities : List MutexId
  destructionCustody : List ResourceId
  outstandingTasks : List TaskId
  quiescent : Bool
  lifetimeInitial : Lifetime.State := {}
  lifetimeEvents : List Lifetime.Event := []
  usedGuardAuthority : Bool := false
  usedSynchronizedHandoff : Bool := false
  deriving DecidableEq, Repr

def ConcurrentState.Valid (state : ConcurrentState) : Prop :=
  state.accesses.all (fun access => access.authorized) = true ∧
  state.accesses.all (fun access => access.conflictsOrdered) = true ∧
  state.lockCapabilities.Nodup ∧
  state.destructionCustody.Nodup ∧
  state.outstandingTasks = [] ∧
  state.quiescent = true ∧
  (Lifetime.run state.lifetimeInitial state.lifetimeEvents).isSome = true

instance (state : ConcurrentState) : Decidable state.Valid := by
  unfold ConcurrentState.Valid
  infer_instance

inductive ModelProfile where
  | trustedExplicitTrace
  deriving DecidableEq, Repr

inductive Assumption where
  | initialAuthorityAndUniqueLifetimeIds
  | guardFootprintAuthority
  | synchronizedCustodyEdges
  deriving DecidableEq, Repr

def assumptions (state : ConcurrentState) : List Assumption :=
  [.initialAuthorityAndUniqueLifetimeIds] ++
  (if state.usedGuardAuthority then [.guardFootprintAuthority] else []) ++
  (if state.usedSynchronizedHandoff then [.synchronizedCustodyEdges] else [])

structure Certificate where
  checked : ConcurrentState
  profile : ModelProfile
  premises : List Assumption
  deriving DecidableEq, Repr

def check (state : ConcurrentState) : Option Certificate :=
  if state.Valid then some {
    checked := state, profile := .trustedExplicitTrace, premises := assumptions state
  } else none

private theorem check_valid {state : ConcurrentState} {certificate : Certificate}
    (result : check state = some certificate) : state.Valid := by
  unfold check at result
  split at result
  · assumption
  · contradiction

theorem accepted_accesses_are_authorized
    {state : ConcurrentState} {certificate : Certificate}
    (result : check state = some certificate) :
    state.accesses.all (fun access => access.authorized) = true :=
  (check_valid result).1

/-- Accepted traces contain no conflicting unordered modeled access. -/
theorem accepted_program_is_race_free
    {state : ConcurrentState} {certificate : Certificate}
    (result : check state = some certificate) :
    state.accesses.all (fun access => access.conflictsOrdered) = true :=
  (check_valid result).2.1

/-- A mutex capability cannot be duplicated in an accepted state. -/
theorem accepted_lock_capabilities_are_linear
    {state : ConcurrentState} {certificate : Certificate}
    (result : check state = some certificate) :
    state.lockCapabilities.Nodup :=
  (check_valid result).2.2.1

theorem accepted_destruction_custody_is_unique
    {state : ConcurrentState} {certificate : Certificate}
    (result : check state = some certificate) :
    state.destructionCustody.Nodup :=
  (check_valid result).2.2.2.1

theorem accepted_scope_is_quiescent
    {state : ConcurrentState} {certificate : Certificate}
    (result : check state = some certificate) :
    state.outstandingTasks = [] ∧ state.quiescent = true :=
  ⟨(check_valid result).2.2.2.2.1, (check_valid result).2.2.2.2.2.1⟩




theorem accepted_lifetimes_replay
    {state : ConcurrentState} {certificate : Certificate}
    (result : check state = some certificate) :
    (Lifetime.run state.lifetimeInitial state.lifetimeEvents).isSome = true :=
  (check_valid result).2.2.2.2.2.2

theorem accepted_certificate_names_trusted_profile_and_assumptions
    {state : ConcurrentState} {certificate : Certificate}
    (result : check state = some certificate) :
    certificate.profile = .trustedExplicitTrace ∧ certificate.premises = assumptions state := by
  unfold check at result
  split at result
  · cases Option.some.inj result
    exact ⟨rfl, rfl⟩
  · contradiction

end QCore.Concurrency
