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

namespace QCore.CallbackTaskHandoff

inductive ArgumentLifetime where
  | owned | shared | borrowedFrame
  deriving DecidableEq, Repr

inductive TaskState where
  | spawned | running | cancelRequested | completed | cancelled | joined
  deriving DecidableEq, Repr

inductive DeferredState where
  | registered | running | completed
  deriving DecidableEq, Repr

def canRetainAcrossReturn : ArgumentLifetime → Bool
  | .owned | .shared => true
  | .borrowedFrame => false

def canClaimArgument (existing : Option ArgumentLifetime) (live : Bool)
    (incoming : ArgumentLifetime) : Bool :=
  match existing with
  | none => true
  | some prior => live && prior == .shared && incoming == .shared

def canDropArgument (known live activeOrRetained : Bool) : Bool :=
  known && live && !activeOrRetained

def remainingCustody (lifetime : ArgumentLifetime) (live : Bool) : Bool :=
  live && lifetime != .borrowedFrame

def taskStep : TaskState → TaskState → Bool
  | .spawned, .running => true
  | .running, .completed => true
  | .spawned, .cancelRequested => true
  | .running, .cancelRequested => true
  | .cancelRequested, .cancelled => true
  | .completed, .joined => true
  | .cancelled, .joined => true
  | _, _ => false

def deferredStep : DeferredState → DeferredState → Bool
  | .registered, .running => true
  | .running, .completed => true
  | _, _ => false

structure TeardownState where
  callbackExited : Bool
  taskStates : List TaskState
  deferredStates : List DeferredState
  deriving DecidableEq, Repr

def quiescent (state : TeardownState) : Bool :=
  state.callbackExited &&
  state.taskStates.all (· == .joined) &&
  state.deferredStates.all (· == .completed)

theorem borrowed_frame_cannot_be_retained :
    canRetainAcrossReturn .borrowedFrame = false := by decide

theorem dropped_argument_identity_cannot_be_reused (lifetime incoming : ArgumentLifetime) :
    canClaimArgument (some lifetime) false incoming = false := by
  cases lifetime <;> cases incoming <;> decide

theorem owned_argument_identity_is_exclusive :
    canClaimArgument (some .owned) true .owned = false := by decide

theorem shared_argument_identity_can_be_shared :
    canClaimArgument (some .shared) true .shared = true := by decide

theorem unknown_argument_cannot_be_dropped (live retained : Bool) :
    canDropArgument false live retained = false := by
  cases live <;> cases retained <;> decide

theorem undropped_owned_argument_remains_explicit_custody :
    remainingCustody .owned true = true := by decide

theorem returned_frame_borrow_is_not_owned_custody :
    remainingCustody .borrowedFrame true = false := by decide

theorem cancellation_request_is_not_joinable :
    taskStep .cancelRequested .joined = false := by decide

theorem cancellation_acknowledgement_can_be_joined :
    taskStep .cancelRequested .cancelled = true ∧
      taskStep .cancelled .joined = true := by decide

theorem running_task_blocks_teardown (deferred : List DeferredState) :
    quiescent ⟨true, [.running], deferred⟩ = false := by simp [quiescent]

theorem registered_deferred_work_blocks_teardown (tasks : List TaskState) :
    quiescent ⟨true, tasks, [.registered]⟩ = false := by simp [quiescent]

theorem post_return_join_and_deferred_completion_are_quiescent :
    quiescent ⟨true, [.joined], [.completed]⟩ = true := by decide

theorem callback_exit_alone_does_not_discharge_handoff :
    quiescent ⟨true, [.completed], []⟩ = false := by decide

theorem spawned_task_cannot_skip_start :
    taskStep .spawned .completed = false := by decide

theorem registered_deferred_work_cannot_skip_start :
    deferredStep .registered .completed = false := by decide

theorem cancellation_completion_race_is_outside_profile :
    taskStep .cancelRequested .completed = false := by decide

/- A keyed executable trace model mirrors the admission-changing structure of
the Rust checker.  Lists are finite maps with single-use identities; `none` is
the fail-closed diagnostic result. -/

abbrev CallbackId := Nat
abbrev TaskId := Nat
abbrev DeferredId := Nat
abbrev ArgumentId := Nat

structure CallbackRecord where
  argument : ArgumentId
  lifetime : ArgumentLifetime
  running : Bool
  deriving DecidableEq, Repr

structure TaskRecord where
  owner : CallbackId
  retainsArgument : Bool
  state : TaskState
  deriving DecidableEq, Repr

structure DeferredRecord where
  owner : CallbackId
  retainsArgument : Bool
  state : DeferredState
  deriving DecidableEq, Repr

structure ArgumentRecord where
  lifetime : ArgumentLifetime
  live : Bool
  deriving DecidableEq, Repr

structure TraceState where
  callbacks : List (CallbackId × CallbackRecord) := []
  tasks : List (TaskId × TaskRecord) := []
  deferred : List (DeferredId × DeferredRecord) := []
  arguments : List (ArgumentId × ArgumentRecord) := []
  tornDown : List CallbackId := []
  joined : List TaskId := []
  deferredCompleted : List DeferredId := []
  dropped : List ArgumentId := []
  deriving DecidableEq, Repr

inductive Operation where
  | callbackEnter (callback : CallbackId) (argument : ArgumentId)
      (lifetime : ArgumentLifetime)
  | spawnTask (callback : CallbackId) (task : TaskId) (retainsArgument : Bool)
  | startTask (task : TaskId)
  | completeTask (task : TaskId)
  | requestCancel (callback : CallbackId) (task : TaskId)
  | acknowledgeCancel (task : TaskId)
  | joinTask (callback : CallbackId) (task : TaskId)
  | registerDeferred (callback : CallbackId) (work : DeferredId)
      (retainsArgument : Bool)
  | startDeferred (work : DeferredId)
  | completeDeferred (work : DeferredId)
  | callbackExit (callback : CallbackId)
  | dropArgument (argument : ArgumentId)
  | teardown (callback : CallbackId)
  deriving DecidableEq, Repr

def lookup (id : Nat) : List (Nat × α) → Option α
  | [] => none
  | (key, value) :: rest => if key = id then some value else lookup id rest

def replace (id : Nat) (value : α) : List (Nat × α) → List (Nat × α)
  | [] => []
  | (key, old) :: rest =>
      if key = id then (key, value) :: rest else (key, old) :: replace id value rest

def claimedArgument (records : List (ArgumentId × ArgumentRecord))
    (argument : ArgumentId) (lifetime : ArgumentLifetime) : Option (List (ArgumentId × ArgumentRecord)) :=
  match lookup argument records with
  | none => some ((argument, ⟨lifetime, true⟩) :: records)
  | some record =>
      if !record.live then none
      else if record.lifetime != lifetime then none
      else if lifetime != .shared then none
      else some records

def ownerArgument (state : TraceState) (owner : CallbackId) : Option ArgumentId :=
  (lookup owner state.callbacks).map (·.argument)

def taskRetains (state : TraceState) (argument : ArgumentId) : Bool :=
  state.tasks.any fun (_, task) =>
    task.retainsArgument && task.state != .joined && ownerArgument state task.owner == some argument

def deferredRetains (state : TraceState) (argument : ArgumentId) : Bool :=
  state.deferred.any fun (_, work) =>
    work.retainsArgument && work.state != .completed && ownerArgument state work.owner == some argument

def step (state : TraceState) : Operation → Option TraceState
  | .callbackEnter callback argument lifetime =>
      if (lookup callback state.callbacks).isSome then none else
      match claimedArgument state.arguments argument lifetime with
      | none => none
      | some arguments => some { state with
          callbacks := (callback, ⟨argument, lifetime, true⟩) :: state.callbacks
          arguments := arguments }
  | .spawnTask callback task retains =>
      match lookup callback state.callbacks with
      | some cb =>
          if !cb.running || (lookup task state.tasks).isSome ||
              (retains && cb.lifetime == .borrowedFrame) then none
          else some { state with tasks := (task, ⟨callback, retains, .spawned⟩) :: state.tasks }
      | none => none
  | .startTask task =>
      match lookup task state.tasks with
      | some record => if record.state != .spawned then none else
          some { state with tasks := replace task { record with state := .running } state.tasks }
      | none => none
  | .completeTask task =>
      match lookup task state.tasks with
      | some record => if record.state != .running then none else
          some { state with tasks := replace task { record with state := .completed } state.tasks }
      | none => none
  | .requestCancel callback task =>
      match lookup task state.tasks with
      | some record =>
          if record.owner != callback ||
              !(record.state == .spawned || record.state == .running) then none
          else some { state with tasks := replace task { record with state := .cancelRequested } state.tasks }
      | none => none
  | .acknowledgeCancel task =>
      match lookup task state.tasks with
      | some record => if record.state != .cancelRequested then none else
          some { state with tasks := replace task { record with state := .cancelled } state.tasks }
      | none => none
  | .joinTask callback task =>
      match lookup task state.tasks with
      | some record =>
          if record.owner != callback ||
              !(record.state == .completed || record.state == .cancelled) then none
          else some { state with
            tasks := replace task { record with state := .joined } state.tasks
            joined := state.joined ++ [task] }
      | none => none
  | .registerDeferred callback work retains =>
      match lookup callback state.callbacks with
      | some cb =>
          if !cb.running || (lookup work state.deferred).isSome ||
              (retains && cb.lifetime == .borrowedFrame) then none
          else some { state with deferred :=
            (work, ⟨callback, retains, .registered⟩) :: state.deferred }
      | none => none
  | .startDeferred work =>
      match lookup work state.deferred with
      | some record => if record.state != .registered then none else
          some { state with deferred := replace work { record with state := .running } state.deferred }
      | none => none
  | .completeDeferred work =>
      match lookup work state.deferred with
      | some record => if record.state != .running then none else
          some { state with
            deferred := replace work { record with state := .completed } state.deferred
            deferredCompleted := state.deferredCompleted ++ [work] }
      | none => none
  | .callbackExit callback =>
      match lookup callback state.callbacks with
      | some record => if !record.running then none else
          some { state with callbacks := replace callback { record with running := false } state.callbacks }
      | none => none
  | .dropArgument argument =>
      match lookup argument state.arguments with
      | some record =>
          let active := state.callbacks.any fun (_, cb) => cb.argument == argument && cb.running
          if !record.live || record.lifetime == .borrowedFrame || active ||
              taskRetains state argument || deferredRetains state argument then none
          else some { state with
            arguments := replace argument { record with live := false } state.arguments
            dropped := state.dropped ++ [argument] }
      | none => none
  | .teardown callback =>
      match lookup callback state.callbacks with
      | some record =>
          let outstandingTask := state.tasks.any fun (_, task) =>
            task.owner == callback && task.state != .joined
          let outstandingDeferred := state.deferred.any fun (_, work) =>
            work.owner == callback && work.state != .completed
          if state.tornDown.contains callback || record.running || outstandingTask || outstandingDeferred then none
          else some { state with tornDown := state.tornDown ++ [callback] }
      | none => none

def run (operations : List Operation) : Option TraceState :=
  operations.foldlM step {}

structure TraceCertificate where
  callbacksTornDown : List CallbackId
  tasksJoined : List TaskId
  deferredCompleted : List DeferredId
  argumentsDropped : List ArgumentId
  remainingArgumentCustody : List ArgumentId
  deriving DecidableEq, Repr

def insertArgumentId (value : ArgumentId) : List ArgumentId → List ArgumentId
  | [] => [value]
  | head :: tail => if value ≤ head then value :: head :: tail
      else head :: insertArgumentId value tail

def sortArgumentIds (values : List ArgumentId) : List ArgumentId :=
  values.foldl (fun sorted value => insertArgumentId value sorted) []

def finish (state : TraceState) : Option TraceCertificate :=
  let incomplete := state.callbacks.any fun (callback, record) =>
    record.running || !state.tornDown.contains callback
  if incomplete then none else some {
    callbacksTornDown := state.tornDown
    tasksJoined := state.joined
    deferredCompleted := state.deferredCompleted
    argumentsDropped := state.dropped
    remainingArgumentCustody := sortArgumentIds <| state.arguments.filterMap fun (argument, record) =>
      if record.live && record.lifetime != .borrowedFrame then some argument else none }

def checkTrace (operations : List Operation) : Option TraceCertificate :=
  (run operations).bind finish

def taskfulAcceptedTrace : List Operation := [
  .callbackEnter 1 100 .owned,
  .spawnTask 1 10 true,
  .registerDeferred 1 20 true,
  .callbackExit 1,
  .startTask 10,
  .requestCancel 1 10,
  .acknowledgeCancel 10,
  .joinTask 1 10,
  .startDeferred 20,
  .completeDeferred 20,
  .dropArgument 100,
  .teardown 1]

theorem taskful_trace_constructs_exact_certificate :
    checkTrace taskfulAcceptedTrace = some ⟨[1], [10], [20], [100], []⟩ := by decide

theorem duplicate_callback_identity_refuses :
    run [.callbackEnter 1 100 .owned, .callbackEnter 1 101 .owned] = none := by decide

theorem duplicate_task_identity_refuses :
    run [.callbackEnter 1 100 .owned, .spawnTask 1 10 true,
      .spawnTask 1 10 true] = none := by decide

theorem duplicate_deferred_identity_refuses :
    run [.callbackEnter 1 100 .owned, .registerDeferred 1 20 true,
      .registerDeferred 1 20 true] = none := by decide

theorem wrong_task_owner_refuses :
    run [.callbackEnter 1 100 .owned, .callbackEnter 2 200 .owned,
      .spawnTask 1 10 true, .requestCancel 2 10] = none := by decide

theorem retained_argument_cannot_drop :
    run [.callbackEnter 1 100 .owned, .spawnTask 1 10 true,
      .callbackExit 1, .dropArgument 100] = none := by decide

theorem borrowed_frame_cannot_drop_after_return :
    run [.callbackEnter 1 100 .borrowedFrame, .callbackExit 1,
      .dropArgument 100, .teardown 1] = none := by decide

theorem teardown_requires_join_and_completion :
    run [.callbackEnter 1 100 .owned, .spawnTask 1 10 true,
      .registerDeferred 1 20 true, .callbackExit 1, .teardown 1] = none := by decide

theorem undropped_argument_is_in_constructed_certificate :
    checkTrace [.callbackEnter 1 100 .owned, .callbackExit 1, .teardown 1] =
      some ⟨[1], [], [], [], [100]⟩ := by decide

theorem remaining_custody_is_canonical_by_argument_identity :
    checkTrace [.callbackEnter 1 200 .owned, .callbackExit 1, .teardown 1,
      .callbackEnter 2 100 .owned, .callbackExit 2, .teardown 2] =
      some ⟨[1, 2], [], [], [], [100, 200]⟩ := by decide

end QCore.CallbackTaskHandoff
