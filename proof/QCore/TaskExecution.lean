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

end QCore.TaskExecution
