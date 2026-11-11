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

/-!
Relations for the copy-only scoped-task execution profile. i32 is represented
by 32-bit patterns; signed decoding, source import, Rust checking and emitted
native execution are explicit correspondence boundaries, not project axioms.
The capture list is finite and positionally typed. Completion retains either
ordinary result or error until explicit creator join. Owned captures, loans,
synchronization and arbitrary C11 execution are outside these relations.
-/
namespace QCore.TaskExecution
inductive ScalarType where
  | unit
  | i32
  deriving DecidableEq, Repr
inductive Scalar where
  | unit
  | i32 (bits : Fin 4294967296)
  deriving DecidableEq, Repr

def scalarType : Scalar → ScalarType
  | .unit => .unit
  | .i32 _ => .i32

def copyCaptures (types : List ScalarType) (values : List Scalar) : Option (List Scalar) :=
  if values.map scalarType = types then some values else none

theorem checked_copy_preserves_ordered_values
    {types : List ScalarType} {before after : List Scalar}
    (checked : copyCaptures types before = some after) : after = before := by
  unfold copyCaptures at checked
  split at checked
  · exact (Option.some.inj checked).symm
  · contradiction

theorem checked_copy_has_exact_parameter_types
    {types : List ScalarType} {before after : List Scalar}
    (checked : copyCaptures types before = some after) : after.map scalarType = types := by
  unfold copyCaptures at checked
  split at checked
  · cases checked
    assumption
  · contradiction

theorem checked_copy_preserves_arity
    {types : List ScalarType} {before after : List Scalar}
    (checked : copyCaptures types before = some after) : after.length = types.length := by
  have same := congrArg List.length (checked_copy_has_exact_parameter_types checked)
  simpa using same

theorem i32_representation_is_bounded (bits : Fin 4294967296) : bits.val < 4294967296 :=
  bits.isLt

/-- Runtime error tags are exact strings. Spawn failure's distinguished string
is separate from each worker tag under the checked declaration premise. -/
abbrev ErrorTag := String
abbrev spawnError : ErrorTag := "SpawnError"
inductive Outcome where
  | normal (value : Scalar)
  | error (tag : ErrorTag)
  deriving DecidableEq, Repr

def outcomeTyped (resultType : ScalarType) (errors : List ErrorTag) : Outcome → Prop
  | .normal value => scalarType value = resultType
  | .error tag => tag ∈ errors

inductive HandleSlot where
  | absent
  | running (creator : Nat)
  | completed (creator : Nat) (outcome : Outcome)
  | joined
  deriving DecidableEq, Repr
abbrev Handles := Nat → HandleSlot

def put (before : Handles) (handle : Nat) (value : HandleSlot) : Handles :=
  fun other => if other = handle then value else before other

inductive CreateChoice where
  | failed
  | created
  deriving DecidableEq, Repr
inductive CreateResult where
  | failure (tag : ErrorTag)
  | handle (id : Nat)
  deriving DecidableEq, Repr

/-- Failure is state preserving; only successful native creation commits the
fresh handle. Joined identities are retained and cannot be reused. -/
def create (before : Handles) (creator handle : Nat) (choice : CreateChoice) :
    Option (Handles × CreateResult) :=
  if before handle = .absent then
    match choice with
    | .failed => some (before, .failure spawnError)
    | .created => some (put before handle (.running creator), .handle handle)
  else none

def complete (before : Handles) (handle : Nat) (outcome : Outcome) : Option Handles :=
  match before handle with
  | .running creator => some (put before handle (.completed creator outcome))
  | _ => none

/-- Join collects without propagation. Unexpected native join failure has no
returning transition here; backend abort/no-reclamation is a separate premise. -/
def join (before : Handles) (creator handle : Nat) : Option (Handles × Outcome) :=
  match before handle with
  | .completed owner outcome =>
    if creator = owner then some (put before handle .joined, outcome) else none
  | _ => none

def quiescent (handles : Handles) (leaving : List Nat) : Prop :=
  ∀ handle ∈ leaving, handles handle = .absent ∨ handles handle = .joined

def propagateError (handles : Handles) (leaving : List Nat) (tag : ErrorTag) :
    Option Outcome :=
  if (leaving.all fun handle =>
      decide (handles handle = .absent ∨ handles handle = .joined)) then
    some (.error tag)
  else none

theorem failed_create_changes_no_handles
    {before after : Handles} {creator handle : Nat} {result : CreateResult}
    (checked : create before creator handle .failed = some (after, result)) :
    after = before ∧ result = .failure spawnError := by
  unfold create at checked
  split at checked
  · cases checked
    exact ⟨rfl, rfl⟩
  · contradiction

theorem failed_create_produces_no_handle
    (before : Handles) (creator handle returnedHandle : Nat) (after : Handles) :
    create before creator handle .failed ≠ some (after, .handle returnedHandle) := by
  intro checked
  have wrong := (failed_create_changes_no_handles checked).2
  cases wrong

theorem successful_create_commits_fresh_handle
    {before after : Handles} {creator handle : Nat} {result : CreateResult}
    (checked : create before creator handle .created = some (after, result)) :
    before handle = .absent ∧ after handle = .running creator ∧ result = .handle handle := by
  unfold create at checked
  split at checked
  · cases checked
    exact ⟨by assumption, by simp [put], rfl⟩
  · contradiction

theorem completion_retains_exact_outcome
    {before after : Handles} {handle creator : Nat} {outcome : Outcome}
    (running : before handle = .running creator)
    (checked : complete before handle outcome = some after) :
    after handle = .completed creator outcome := by
  simp [complete, running] at checked
  cases checked
  simp [put]

theorem completed_result_cannot_be_overwritten
    {before : Handles} {handle creator : Nat} {original replacement : Outcome}
    (completed : before handle = .completed creator original) :
    complete before handle replacement = none := by
  simp [complete, completed]

theorem unrelated_handle_write_preserves_completion
    {before : Handles} {handle other creator : Nat} {outcome : Outcome}
    (completed : before handle = .completed creator outcome)
    (distinct : handle ≠ other) (value : HandleSlot) :
    put before other value handle = .completed creator outcome := by
  simp [put, distinct, completed]

theorem explicit_creator_join_collects_exact_outcome
    {before : Handles} {handle creator : Nat} {outcome : Outcome}
    (completed : before handle = .completed creator outcome) :
    join before creator handle = some (put before handle .joined, outcome) := by
  simp [join, completed]

theorem noncreator_cannot_collect_completion
    {before : Handles} {handle creator caller : Nat} {outcome : Outcome}
    (completed : before handle = .completed creator outcome)
    (different : caller ≠ creator) : join before caller handle = none := by
  simp [join, completed, different]

theorem checked_join_consumes_handle_once
    {before after : Handles} {handle creator : Nat} {outcome : Outcome}
    (checked : join before creator handle = some (after, outcome)) :
    after handle = .joined ∧ ∀ caller, join after caller handle = none := by
  unfold join at checked
  split at checked <;> try contradiction
  split at checked <;> try contradiction
  cases checked
  constructor
  · simp [put]
  · intro caller
    simp [join, put]

theorem checked_join_preserves_result_type
    {before after : Handles} {handle creator owner : Nat} {stored result : Outcome}
    {resultType : ScalarType} {errors : List ErrorTag}
    (completed : before handle = .completed owner stored)
    (typed : outcomeTyped resultType errors stored)
    (checked : join before creator handle = some (after, result)) :
    outcomeTyped resultType errors result := by
  simp only [join, completed] at checked
  split at checked
  · cases checked
    exact typed
  · contradiction

theorem checked_propagation_cannot_strand_outstanding_handles
    {handles : Handles} {leaving : List Nat} {tag : ErrorTag} {result : Outcome}
    (checked : propagateError handles leaving tag = some result) :
    quiescent handles leaving ∧ result = .error tag := by
  classical
  unfold propagateError at checked
  split at checked
  · cases checked
    exact ⟨by simpa [quiescent, List.all_eq_true] using ‹(leaving.all fun handle =>
      decide (handles handle = .absent ∨ handles handle = .joined)) = true›, rfl⟩
  · contradiction

theorem completed_unjoined_child_prevents_propagation
    {handles : Handles} {leaving : List Nat} {handle creator : Nat}
    {stored : Outcome} (member : handle ∈ leaving)
    (completed : handles handle = .completed creator stored) (tag : ErrorTag) :
    propagateError handles leaving tag = none := by
  classical
  have busy : ¬ quiescent handles leaving := by
    intro quiet
    have impossible := quiet handle member
    simp [completed] at impossible
  unfold quiescent at busy
  simp [propagateError, List.all_eq_true, busy]

theorem normal_and_error_outcomes_are_distinct (value : Scalar) (tag : ErrorTag) :
    Outcome.normal value ≠ .error tag := by
  intro impossible
  cases impossible

theorem worker_error_tag_is_preserved (tag : ErrorTag) (different : tag ≠ spawnError) :
    Outcome.error tag ≠ .error spawnError := by
  intro impossible
  exact different (Outcome.error.inj impossible)

/-- Outcome discrimination consumes a Result slot and preserves the exact error
string; selecting an error arm does not replace it by one guessed static tag. -/
def tryOutcome : Option Outcome → Option (Option Outcome × Sum Scalar ErrorTag)
  | none => none
  | some (.normal value) => some (none, .inl value)
  | some (.error tag) => some (none, .inr tag)

theorem outcome_discrimination_preserves_normal_value (value : Scalar) :
    tryOutcome (some (.normal value)) = some (none, .inl value) := rfl

theorem outcome_discrimination_preserves_exact_error (tag : ErrorTag) :
    tryOutcome (some (.error tag)) = some (none, .inr tag) := rfl

theorem consumed_result_cannot_be_discriminated_again : tryOutcome none = none := rfl

theorem create_preserves_other_handle
    {before after : Handles} {creator handle other : Nat} {choice : CreateChoice}
    {result : CreateResult} (distinct : other ≠ handle)
    (checked : create before creator handle choice = some (after, result)) :
    after other = before other := by
  unfold create at checked
  split at checked
  · cases choice <;> cases checked <;> simp [put, distinct]
  · contradiction

theorem complete_preserves_other_handle
    {before after : Handles} {handle other : Nat} {outcome : Outcome}
    (distinct : other ≠ handle)
    (checked : complete before handle outcome = some after) :
    after other = before other := by
  unfold complete at checked
  split at checked <;> try contradiction
  cases checked
  simp [put, distinct]

theorem join_preserves_other_handle
    {before after : Handles} {creator handle other : Nat} {outcome : Outcome}
    (distinct : other ≠ handle)
    (checked : join before creator handle = some (after, outcome)) :
    after other = before other := by
  unfold join at checked
  split at checked <;> try contradiction
  split at checked <;> try contradiction
  cases checked
  simp [put, distinct]

/-- Project a finite execution onto operations that have not collected the
watched handle. Same-handle completion/creation are separately shown to reject
once the result exists. Other handles may create, finish or be joined. -/
inductive UncollectedStep (watched : Nat) : Handles → Handles → Prop where
  | create {before after creator handle choice result}
      (different : watched ≠ handle)
      (checked : TaskExecution.create before creator handle choice = some (after, result)) :
      UncollectedStep watched before after
  | complete {before after handle outcome}
      (different : watched ≠ handle)
      (checked : TaskExecution.complete before handle outcome = some after) :
      UncollectedStep watched before after
  | join {before after creator handle outcome}
      (different : watched ≠ handle)
      (checked : TaskExecution.join before creator handle = some (after, outcome)) :
      UncollectedStep watched before after
  | idle (before) : UncollectedStep watched before before

inductive UncollectedRun (watched : Nat) : Handles → Handles → Prop where
  | nil (before) : UncollectedRun watched before before
  | cons {before middle after}
      (first : UncollectedStep watched before middle)
      (rest : UncollectedRun watched middle after) : UncollectedRun watched before after

theorem uncollected_step_retains_handle
    {watched : Nat} {before after : Handles}
    (step : UncollectedStep watched before after) : after watched = before watched := by
  cases step with
  | create different checked => exact create_preserves_other_handle different checked
  | complete different checked => exact complete_preserves_other_handle different checked
  | join different checked => exact join_preserves_other_handle different checked
  | idle => rfl

theorem completion_survives_until_explicit_collection
    {watched creator : Nat} {before after : Handles} {outcome : Outcome}
    (completed : before watched = .completed creator outcome)
    (run : UncollectedRun watched before after) :
    after watched = .completed creator outcome := by
  induction run with
  | nil => exact completed
  | cons first _ ih =>
    exact ih ((uncollected_step_retains_handle first).trans completed)

theorem completed_handle_cannot_be_recreated
    {before : Handles} {handle creator caller : Nat} {outcome : Outcome}
    (completed : before handle = .completed creator outcome) (choice : CreateChoice) :
    create before caller handle choice = none := by
  simp [create, completed]

/-! Explicit creation-result handlers. This is the pure-copy handle relation:
there are no owned moves, loans, capture usage or SSA refinements to restore.
The finite handler trace records actual collect operations; failure itself
cannot manufacture such an operation or erase an earlier sibling. -/

theorem failed_create_preserves_prior_sibling
    {before after : Handles} {creator newHandle sibling : Nat} {result : CreateResult}
    (checked : create before creator newHandle .failed = some (after, result)) :
    after sibling = before sibling := by
  rw [(failed_create_changes_no_handles checked).1]

theorem failed_create_does_not_join_or_cancel_running_sibling
    {before after : Handles} {creator newHandle sibling owner : Nat} {result : CreateResult}
    (checked : create before creator newHandle .failed = some (after, result))
    (running : before sibling = .running owner) :
    after sibling = .running owner ∧ after sibling ≠ .joined ∧ after sibling ≠ .absent := by
  rw [failed_create_preserves_prior_sibling checked, running]
  simp

theorem failed_create_retains_completed_sibling_for_explicit_join
    {before after : Handles} {creator newHandle sibling owner : Nat} {result : CreateResult}
    {outcome : Outcome}
    (checked : create before creator newHandle .failed = some (after, result))
    (completed : before sibling = .completed owner outcome) :
    after sibling = .completed owner outcome ∧
    join after owner sibling = some (put after sibling .joined, outcome) := by
  have preserved := (failed_create_preserves_prior_sibling checked).trans completed
  exact ⟨preserved, explicit_creator_join_collects_exact_outcome preserved⟩

theorem creation_cannot_mark_an_unjoined_sibling_joined
    {before after : Handles} {creator newHandle sibling : Nat} {choice : CreateChoice}
    {result : CreateResult}
    (checked : create before creator newHandle choice = some (after, result))
    (joined : after sibling = .joined) : before sibling = .joined := by
  cases choice with
  | failed =>
    rw [(failed_create_changes_no_handles checked).1] at joined
    exact joined
  | created =>
    by_cases same : sibling = newHandle
    · subst sibling
      have running := (successful_create_commits_fresh_handle checked).2.1
      rw [running] at joined
      cases joined
    · exact (create_preserves_other_handle same checked).symm.trans joined

theorem completion_cannot_mark_an_unjoined_sibling_joined
    {before after : Handles} {handle sibling : Nat} {outcome : Outcome}
    (checked : complete before handle outcome = some after)
    (joined : after sibling = .joined) : before sibling = .joined := by
  unfold complete at checked
  split at checked <;> try contradiction
  cases checked
  by_cases same : sibling = handle
  · simp [put, same] at joined
  · simpa [put, same] using joined

/-- Every step uses the existing checked operation. The list records only the
explicit creator joins that actually occurred, in their execution order. -/
inductive HandlerRun : Handles → Handles → List Nat → Prop where
  | nil (before) : HandlerRun before before []
  | create {before middle after creator handle choice result joins}
      (checked : TaskExecution.create before creator handle choice = some (middle, result))
      (rest : HandlerRun middle after joins) : HandlerRun before after joins
  | complete {before middle after handle outcome joins}
      (checked : TaskExecution.complete before handle outcome = some middle)
      (rest : HandlerRun middle after joins) : HandlerRun before after joins
  | collect {before middle after creator handle outcome joins}
      (checked : TaskExecution.join before creator handle = some (middle, outcome))
      (rest : HandlerRun middle after joins) : HandlerRun before after (handle :: joins)

theorem handler_cannot_discharge_sibling_without_explicit_join
    {before after : Handles} {sibling : Nat} {joins : List Nat}
    (run : HandlerRun before after joins)
    (unjoined : before sibling ≠ .joined)
    (joined : after sibling = .joined) : sibling ∈ joins := by
  induction run with
  | nil => exact False.elim (unjoined joined)
  | create checked _ ih =>
    exact ih (fun middleJoined => unjoined
      (creation_cannot_mark_an_unjoined_sibling_joined checked middleJoined)) joined
  | complete checked _ ih =>
    exact ih (fun middleJoined => unjoined
      (completion_cannot_mark_an_unjoined_sibling_joined checked middleJoined)) joined
  | @collect before middle after creator handle outcome joins checked _ ih =>
    by_cases same : sibling = handle
    · simp [same]
    · have middleUnjoined : middle sibling ≠ .joined := by
        intro middleJoined
        exact unjoined ((join_preserves_other_handle same checked).symm.trans middleJoined)
      exact List.mem_cons_of_mem handle (ih middleUnjoined joined)

/-- A handler may return normally or rethrow the original creation/worker error.
Both outcomes use the same finite inventory check. No joins are inserted here. -/
def finishHandler (handles : Handles) (leaving : List Nat) (outcome : Outcome) : Option Outcome :=
  if (leaving.all fun handle =>
      decide (handles handle = .absent ∨ handles handle = .joined)) then
    some outcome
  else none

theorem checked_handler_exit_is_quiescent_and_preserves_outcome
    {handles : Handles} {leaving : List Nat} {outcome returned : Outcome}
    (checked : finishHandler handles leaving outcome = some returned) :
    quiescent handles leaving ∧ returned = outcome := by
  unfold finishHandler at checked
  split at checked
  · cases checked
    exact ⟨by simpa [quiescent, List.all_eq_true] using ‹(leaving.all fun handle =>
      decide (handles handle = .absent ∨ handles handle = .joined)) = true›, rfl⟩
  · contradiction

theorem checked_normal_and_error_handlers_both_require_quiescence
    {normalHandles errorHandles : Handles} {leaving : List Nat}
    {value : Scalar} {tag : ErrorTag}
    (normal : finishHandler normalHandles leaving (.normal value) = some (.normal value))
    (error : finishHandler errorHandles leaving (.error tag) = some (.error tag)) :
    quiescent normalHandles leaving ∧ quiescent errorHandles leaving := by
  exact ⟨(checked_handler_exit_is_quiescent_and_preserves_outcome normal).1,
    (checked_handler_exit_is_quiescent_and_preserves_outcome error).1⟩

theorem handler_join_preserves_exact_worker_error
    {before after : Handles} {creator sibling : Nat} {tag : ErrorTag} {collected : Outcome}
    (completed : before sibling = .completed creator (.error tag))
    (checked : join before creator sibling = some (after, collected)) :
    collected = .error tag ∧ after sibling = .joined := by
  rw [explicit_creator_join_collects_exact_outcome completed] at checked
  cases checked
  exact ⟨rfl, by simp [put]⟩

theorem failed_create_then_handler_join_preserves_worker_error
    {before failed after : Handles} {creator newHandle sibling : Nat}
    {tag : ErrorTag} {result : CreateResult} {collected : Outcome}
    (creation : create before creator newHandle .failed = some (failed, result))
    (completed : before sibling = .completed creator (.error tag))
    (collection : join failed creator sibling = some (after, collected)) :
    result = .failure spawnError ∧ collected = .error tag ∧ after sibling = .joined := by
  have preserved := (failed_create_preserves_prior_sibling creation).trans completed
  exact ⟨(failed_create_changes_no_handles creation).2,
    handler_join_preserves_exact_worker_error preserved collection⟩

theorem create_preserves_nonabsent_sibling
    {before after : Handles} {creator newHandle sibling : Nat} {choice : CreateChoice}
    {result : CreateResult}
    (checked : create before creator newHandle choice = some (after, result))
    (live : before sibling ≠ .absent) : after sibling ≠ .absent := by
  cases choice with
  | failed => simpa [(failed_create_changes_no_handles checked).1] using live
  | created =>
    by_cases same : sibling = newHandle
    · subst sibling
      rw [(successful_create_commits_fresh_handle checked).2.1]
      intro impossible
      cases impossible
    · rw [create_preserves_other_handle same checked]
      exact live

theorem complete_preserves_nonabsent_sibling
    {before after : Handles} {handle sibling : Nat} {outcome : Outcome}
    (checked : complete before handle outcome = some after)
    (live : before sibling ≠ .absent) : after sibling ≠ .absent := by
  unfold complete at checked
  split at checked <;> try contradiction
  cases checked
  by_cases same : sibling = handle
  · simp [put, same]
  · simpa [put, same] using live

theorem join_preserves_nonabsent_sibling
    {before after : Handles} {creator handle sibling : Nat} {outcome : Outcome}
    (checked : join before creator handle = some (after, outcome))
    (live : before sibling ≠ .absent) : after sibling ≠ .absent := by
  by_cases same : sibling = handle
  · subst sibling
    rw [(checked_join_consumes_handle_once checked).1]
    intro impossible
    cases impossible
  · rw [join_preserves_other_handle same checked]
    exact live

theorem handler_cannot_erase_existing_handle
    {before after : Handles} {sibling : Nat} {joins : List Nat}
    (run : HandlerRun before after joins)
    (live : before sibling ≠ .absent) : after sibling ≠ .absent := by
  induction run with
  | nil => exact live
  | create checked _ ih => exact ih (create_preserves_nonabsent_sibling checked live)
  | complete checked _ ih => exact ih (complete_preserves_nonabsent_sibling checked live)
  | collect checked _ ih => exact ih (join_preserves_nonabsent_sibling checked live)

theorem handler_exit_requires_explicit_join_of_every_prior_sibling
    {before after : Handles} {sibling : Nat} {joins leaving : List Nat}
    {outcome returned : Outcome}
    (run : HandlerRun before after joins)
    (live : before sibling ≠ .absent)
    (unjoined : before sibling ≠ .joined)
    (member : sibling ∈ leaving)
    (exit : finishHandler after leaving outcome = some returned) : sibling ∈ joins := by
  have quiet := (checked_handler_exit_is_quiescent_and_preserves_outcome exit).1
  have afterJoined : after sibling = .joined := by
    cases quiet sibling member with
    | inl absent => exact False.elim ((handler_cannot_erase_existing_handle run live) absent)
    | inr joined => exact joined
  exact handler_cannot_discharge_sibling_without_explicit_join run unjoined afterJoined

/-- CT13's handle obligation: after a second creation fails, a handler that
leaves the scope must contain an explicit join of each pre-existing sibling.
This holds for either a normal return or a rethrown error outcome. -/
theorem failed_creation_handler_exit_joins_prior_sibling
    {before failed after : Handles} {creator newHandle sibling : Nat}
    {result : CreateResult} {joins leaving : List Nat} {outcome returned : Outcome}
    (creation : create before creator newHandle .failed = some (failed, result))
    (handler : HandlerRun failed after joins)
    (live : before sibling ≠ .absent)
    (unjoined : before sibling ≠ .joined)
    (member : sibling ∈ leaving)
    (exit : finishHandler after leaving outcome = some returned) : sibling ∈ joins := by
  have same := (failed_create_changes_no_handles creation).1
  exact handler_exit_requires_explicit_join_of_every_prior_sibling handler
    (by simpa [same] using live) (by simpa [same] using unjoined) member exit

/-! Empty-error-row elimination is not propagation. It consumes one Result and
returns its typed scalar without introducing a phantom error edge or performing
any handle operation. Native scalar extraction relies on the opaque checked
module and on correspondence with this typed outcome relation. -/

def unwrapInfallible (errors : List ErrorTag) : Option Outcome → Option (Option Outcome × Scalar)
  | some (.normal value) => if errors = [] then some (none, value) else none
  | _ => none

theorem empty_declared_error_row_rules_out_error (resultType : ScalarType) (tag : ErrorTag) :
    ¬ outcomeTyped resultType [] (.error tag) := by
  simp [outcomeTyped]

theorem typed_empty_error_row_has_normal_scalar
    {resultType : ScalarType} {outcome : Outcome}
    (typed : outcomeTyped resultType [] outcome) :
    ∃ value, outcome = .normal value ∧ scalarType value = resultType := by
  cases outcome with
  | normal value => exact ⟨value, rfl, typed⟩
  | error tag => exact False.elim (empty_declared_error_row_rules_out_error resultType tag typed)

theorem typed_infallible_extraction_preserves_value_and_consumes_result
    {resultType : ScalarType} {outcome : Outcome}
    (typed : outcomeTyped resultType [] outcome) :
    ∃ value, outcome = .normal value ∧ scalarType value = resultType ∧
      unwrapInfallible [] (some outcome) = some (none, value) := by
  obtain ⟨value, normal, scalar⟩ := typed_empty_error_row_has_normal_scalar typed
  exact ⟨value, normal, scalar, by simp [normal, unwrapInfallible]⟩

theorem checked_infallible_extraction_requires_empty_declared_errors
    {errors : List ErrorTag} {slot consumed : Option Outcome} {value : Scalar}
    (checked : unwrapInfallible errors slot = some (consumed, value)) :
    errors = [] ∧ consumed = none ∧ slot = some (.normal value) := by
  unfold unwrapInfallible at checked
  split at checked <;> try contradiction
  split at checked <;> try contradiction
  cases checked
  exact ⟨by assumption, rfl, rfl⟩

theorem infallible_extraction_does_not_accept_actual_error (errors : List ErrorTag) (tag : ErrorTag) :
    unwrapInfallible errors (some (.error tag)) = none := rfl

theorem consumed_infallible_result_cannot_be_extracted_again (errors : List ErrorTag) :
    unwrapInfallible errors none = none := rfl

/-! ### Cleanup-preemption attribution (card CT24-CLEANUP-PREEMPTED, wave 30)

The scope obligation refuses an early exit while the scope holds an unjoined
child, and the exit rule carries its own loan backstop: a frame may not leave
while one of its resources stays lent to an unjoined borrower. The preempting
refusal is now *attributed* from that same loan judgment when it holds — the
frame's cleanup would destroy a lent owner before the borrower's join, the
construct CT24 names. These relations pin what makes that move incapable of
changing any verdict: whether the exit refuses is decided by the scope
obligation alone, before and after; the judgment only selects the message. -/

/-- The two renderings a refused scope exit can carry. -/
inductive ExitAttribution where
  | scopeBusy
  | cleanupPreempted (resource : Nat)
  deriving DecidableEq, Repr

/-- The exit rule's own loan judgment: the first frame resource with an
outstanding read share, exactly the search the exit backstop performs. -/
def outstandingLentResource (lent : Nat → Nat) : List Nat → Option Nat
  | [] => none
  | resource :: rest =>
    if lent resource > 0 then some resource else outstandingLentResource lent rest

/-- A scope exit under the scope obligation: `some` attribution is a refusal,
`none` is the admitted exit. The loan judgment is consulted only after the
obligation has already refused. -/
def scopeExit (busy : Bool) (lent : Nat → Nat) (resources : List Nat) :
    Option ExitAttribution :=
  if busy then
    some (match outstandingLentResource lent resources with
      | some resource => .cleanupPreempted resource
      | none => .scopeBusy)
  else none

/-- The attribution cannot move the refusal: whether the exit refuses is the
scope obligation's own decision, whatever the loan judgment says. -/
theorem attribution_never_moves_the_refusal
    (busy : Bool) (lent : Nat → Nat) (resources : List Nat) :
    (scopeExit busy lent resources).isSome = busy := by
  cases busy <;> simp [scopeExit]

/-- An admitted exit is never attributed: the judgment is read only inside a
refusal the obligation already made. -/
theorem an_admitted_exit_is_never_attributed (lent : Nat → Nat) (resources : List Nat) :
    scopeExit false lent resources = none := rfl

/-- A named owner is a genuinely lent frame resource: the judgment cannot
invent a resource, and what it names carries an outstanding share. -/
theorem a_named_owner_is_a_lent_frame_resource
    {lent : Nat → Nat} {resources : List Nat} {resource : Nat}
    (found : outstandingLentResource lent resources = some resource) :
    resource ∈ resources ∧ lent resource > 0 := by
  induction resources with
  | nil => cases found
  | cons head rest ih =>
    unfold outstandingLentResource at found
    by_cases lentHead : lent head > 0
    · rw [ite_eq_left lentHead] at found
      cases found
      exact ⟨List.mem_cons_self, lentHead⟩
    · rw [ite_eq_right lentHead] at found
      obtain ⟨member, positive⟩ := ih found
      exact ⟨List.mem_cons_of_mem head member, positive⟩

/-- The preempting gate reads the backstop's own judgment: the refined
attribution appears exactly where that judgment resolves a resource. -/
theorem the_preempting_gate_reads_the_backstops_own_judgment
    (lent : Nat → Nat) (resources : List Nat) (resource : Nat) :
    scopeExit true lent resources = some (.cleanupPreempted resource) ↔
      outstandingLentResource lent resources = some resource := by
  unfold scopeExit
  cases found : outstandingLentResource lent resources with
  | none => simp
  | some judged => simp

/-- A frame with no lent resource keeps the plain refusal byte for byte: a
busy scope whose unjoined child holds no loan is not the CT24 construct, and
nothing is attributed to it. -/
theorem an_unlent_frame_keeps_the_plain_refusal
    {lent : Nat → Nat} {resources : List Nat}
    (unlent : ∀ resource ∈ resources, lent resource = 0) :
    scopeExit true lent resources = some .scopeBusy := by
  have empty : outstandingLentResource lent resources = none := by
    induction resources with
    | nil => rfl
    | cons head rest ih =>
      unfold outstandingLentResource
      rw [ite_eq_right (by simp [unlent head List.mem_cons_self])]
      exact ih fun resource member => unlent resource (List.mem_cons_of_mem head member)
  simp [scopeExit, empty]

/-- CT24's cell: a refused exit whose frame holds a lent owner is always
attributed to the destroy-before-join, and the owner it names really is a
lent resource of the exiting frame. -/
theorem ct24s_cell_is_attributed_to_the_destroy_before_join
    {lent : Nat → Nat} {resources : List Nat} {witness : Nat}
    (member : witness ∈ resources) (outstanding : lent witness > 0) :
    ∃ resource, scopeExit true lent resources = some (.cleanupPreempted resource) ∧
      resource ∈ resources ∧ lent resource > 0 := by
  have found : ∃ resource, outstandingLentResource lent resources = some resource := by
    induction resources with
    | nil => cases member
    | cons head rest ih =>
      unfold outstandingLentResource
      by_cases lentHead : lent head > 0
      · exact ⟨head, by rw [ite_eq_left lentHead]⟩
      · rw [ite_eq_right lentHead]
        cases List.mem_cons.mp member with
        | inl atHead => exact absurd (atHead ▸ outstanding) lentHead
        | inr inRest => exact ih inRest
  obtain ⟨resource, judged⟩ := found
  exact ⟨resource,
    (the_preempting_gate_reads_the_backstops_own_judgment lent resources resource).mpr judged,
    a_named_owner_is_a_lent_frame_resource judged⟩

end QCore.TaskExecution
