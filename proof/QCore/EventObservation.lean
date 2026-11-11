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

namespace QCore.EventObservation

/-! Identity-bearing refinement for the first Cycle 18 runtime-observation
profile.  The profile is deliberately a single dispatcher timed wait.  It has
no synchronization edge: accepting one would assert an order this execution
does not establish. -/

structure Identity where
  id : Nat
  generation : Nat
  deriving DecidableEq, Repr

inductive Kind where
  | baseCreate
  | eventNew
  | eventAdd
  | dispatchBegin
  | activateTimeout
  | callbackEnterTimeout
  | callbackExit
  | dispatchEnd
  | eventFree
  | argumentEnd
  | baseFree
  deriving DecidableEq, Repr

structure Record where
  run : Nat
  buffer : Nat
  localSequence : Nat
  dispatcher : Nat
  base : Identity
  event : Option Identity
  argument : Option Identity
  callback : Option String
  kind : Kind
  deriving DecidableEq, Repr

structure Context where
  run : Nat
  buffer : Nat
  dispatcher : Nat
  base : Identity
  event : Identity
  argument : Identity
  callback : String
  deriving DecidableEq, Repr

def mkRecord (c : Context) (sequence : Nat) (kind : Kind)
    (event : Option Identity) (argument : Option Identity)
    (callback : Option String) : Record :=
  { run := c.run
    buffer := c.buffer
    localSequence := sequence
    dispatcher := c.dispatcher
    base := c.base
    event
    argument
    callback
    kind }

def record (c : Context) (sequence : Nat) : Kind → Record
  | .baseCreate => mkRecord c sequence .baseCreate none none none
  | .eventNew => mkRecord c sequence .eventNew (some c.event)
      (some c.argument) (some c.callback)
  | .eventAdd => mkRecord c sequence .eventAdd (some c.event) none none
  | .dispatchBegin => mkRecord c sequence .dispatchBegin none none none
  | .activateTimeout => mkRecord c sequence .activateTimeout (some c.event) none none
  | .callbackEnterTimeout => mkRecord c sequence .callbackEnterTimeout
      (some c.event) (some c.argument) (some c.callback)
  | .callbackExit => mkRecord c sequence .callbackExit
      (some c.event) (some c.argument) (some c.callback)
  | .dispatchEnd => mkRecord c sequence .dispatchEnd none none none
  | .eventFree => mkRecord c sequence .eventFree (some c.event) none none
  | .argumentEnd => mkRecord c sequence .argumentEnd none (some c.argument) none
  | .baseFree => mkRecord c sequence .baseFree none none none

def expected (c : Context) : List Record :=
  [record c 1 .baseCreate,
   record c 2 .eventNew,
   record c 3 .eventAdd,
   record c 4 .dispatchBegin,
   record c 5 .activateTimeout,
   record c 6 .callbackEnterTimeout,
   record c 7 .callbackExit,
   record c 8 .dispatchEnd,
   record c 9 .eventFree,
   record c 10 .argumentEnd,
   record c 11 .baseFree]

structure Trace where
  records : List Record
  synchronizationEdges : List (Nat × Nat)
  deriving DecidableEq, Repr

def Valid (trace : Trace) (context : Context) : Prop :=
  trace.records = expected context ∧ trace.synchronizationEdges = []

instance (trace : Trace) (context : Context) : Decidable (Valid trace context) := by
  unfold Valid
  infer_instance

structure Certificate where
  trace : Trace
  context : Context
  deriving DecidableEq, Repr

def check (trace : Trace) (context : Context) : Option Certificate :=
  if Valid trace context then some { trace, context } else none

private theorem check_valid {trace : Trace} {context : Context}
    {certificate : Certificate}
    (result : check trace context = some certificate) : Valid trace context := by
  unfold check at result
  split at result
  · assumption
  · contradiction

theorem accepted_trace_is_exact {trace : Trace} {context : Context}
    {certificate : Certificate}
    (result : check trace context = some certificate) :
    trace.records = expected context :=
  (check_valid result).1

theorem accepted_trace_has_no_invented_synchronization
    {trace : Trace} {context : Context} {certificate : Certificate}
    (result : check trace context = some certificate) :
    trace.synchronizationEdges = [] :=
  (check_valid result).2

theorem accepted_callback_binds_expected_event_generation
    {trace : Trace} {context : Context} {certificate : Certificate}
    (result : check trace context = some certificate) :
    ((trace.records.drop 5).head?).bind Record.event = some context.event := by
  rw [accepted_trace_is_exact result]
  simp [expected, record, mkRecord]

theorem changing_the_event_generation_changes_the_callback_record
    (context : Context) (replacement : Identity)
    (different : replacement ≠ context.event) :
    (record { context with event := replacement } 6 .callbackEnterTimeout) ≠
      record context 6 .callbackEnterTimeout := by
  intro equal
  have same := congrArg Record.event equal
  dsimp [record, mkRecord] at same
  exact different (Option.some.inj same)

end QCore.EventObservation
