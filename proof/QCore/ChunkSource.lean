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

namespace QCore.ChunkSource

/-! Executable finite-state semantics for the bounded Boolean-field source
importer. Clang typing and AST-to-IR translation remain trusted boundaries;
these theorems do not prove unrestricted C or a full ChunkIO teardown. -/

inductive Expr where
  | literal (value : Int)
  | field
  | not (value : Expr)
  | negate (value : Expr)
  | equal (left right : Expr)
  | notEqual (left right : Expr)
  deriving Repr, DecidableEq

def boolInt (value : Bool) : Int := if value then 1 else 0

def Expr.eval (field : Bool) : Expr → Int
  | .literal value => value
  | .field => boolInt field
  | .not value => boolInt (value.eval field == 0)
  | .negate value => -(value.eval field)
  | .equal left right => boolInt (left.eval field == right.eval field)
  | .notEqual left right => boolInt (left.eval field != right.eval field)

inductive Stmt where
  | empty
  | sequence (first rest : Stmt)
  | branch (condition : Expr) (yes no : Stmt)
  | assign (value : Expr)
  | ret (value : Expr)
  | frameAssign (field : String) (value : Int)
  deriving Repr, DecidableEq

structure State where
  value : Bool
  writes : List Bool := []
  branches : List Bool := []
  frameWrites : List (String × Int) := []
  deriving Repr, DecidableEq

def Stmt.eval : Stmt → State → Option (State × Option Int)
  | .empty, state => some (state, none)
  | .sequence first rest, state => do
      let (next, result) ← first.eval state
      match result with
      | some value => some (next, some value)
      | none => rest.eval next
  | .branch condition yes no, state =>
      let taken := condition.eval state.value != 0
      let next := { state with branches := state.branches ++ [taken] }
      if taken then yes.eval next else no.eval next
  | .assign expression, state =>
      let value := expression.eval state.value
      if value = 0 ∨ value = 1 then
        let next := value == 1
        let writes := state.writes ++ [next]
        some ({ state with value := next, writes := writes }, none)
      else none
  | .ret value, state => some (state, some (value.eval state.value))
  | .frameAssign field value, state =>
      some ({ state with frameWrites := state.frameWrites ++ [(field, value)] }, none)

structure Case where
  state : State
  result : Int
  deriving Repr, DecidableEq

def run (statement : Stmt) (initial : Bool) : Option Case := do
  let (state, result) ← statement.eval { value := initial }
  let value ← result
  pure { state, result := value }

structure Summary where
  whenFalse : Case
  whenTrue : Case
  deriving Repr, DecidableEq

def summarize (statement : Stmt) : Option Summary := do
  let whenFalse ← run statement false
  let whenTrue ← run statement true
  pure { whenFalse, whenTrue }

def Summary.select (summary : Summary) (initial : Bool) : Case :=
  if initial then summary.whenTrue else summary.whenFalse

theorem accepted_summary_covers_every_boolean_prestate
    {statement : Stmt} {summary : Summary}
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

def checkContract (summary : Summary) (post : Bool → Case → Bool) : Bool :=
  post false summary.whenFalse && post true summary.whenTrue

theorem accepted_contract_holds_on_every_source_path
    {statement : Stmt} {summary : Summary} {post : Bool → Case → Bool}
    (imported : summarize statement = some summary)
    (checked : checkContract summary post = true) (initial : Bool) :
    run statement initial = some (summary.select initial) ∧
      post initial (summary.select initial) = true := by
  refine ⟨accepted_summary_covers_every_boolean_prestate imported initial, ?_⟩
  have both : post false summary.whenFalse = true ∧ post true summary.whenTrue = true := by
    simpa [checkContract] using checked
  cases initial
  · exact both.1
  · exact both.2

/-! Finite path portion of the source-effect witness firewall. The imported
summary is produced by `summarize`; the provider cannot replace it. Equality
of `Summary` includes final values, returns, branches, ordered field writes and
ordered frame writes. The explicit labels also prevent omitted, duplicated or
swapped Boolean prestates. `post` represents the pinned contract under its
declared live/exclusive/Boolean-domain premises.

Rust additionally checks the task and source/build/profile/helper identities.
Those checks, Clang decoding, contract-text interpretation and runtime caller
provenance are not proved by this finite-state theorem. -/

structure SourceEffectWitness where
  falseInitial : Bool
  trueInitial : Bool
  paths : Summary
  deriving Repr, DecidableEq

def sourceEffectWitnessAgrees (imported : Summary) (proposed : SourceEffectWitness) : Bool :=
  decide (proposed.falseInitial = false ∧ proposed.trueInitial = true ∧
    proposed.paths = imported)

def checkSourceEffectWitness (imported : Summary) (proposed : SourceEffectWitness)
    (post : Bool → Case → Bool) : Bool :=
  sourceEffectWitnessAgrees imported proposed && checkContract imported post

theorem accepted_source_effect_witness_has_exact_paths
    {imported : Summary} {proposed : SourceEffectWitness} {post : Bool → Case → Bool}
    (accepted : checkSourceEffectWitness imported proposed post = true) :
    proposed.falseInitial = false ∧ proposed.trueInitial = true ∧
      proposed.paths = imported := by
  have gates : sourceEffectWitnessAgrees imported proposed = true ∧
      checkContract imported post = true := by
    simpa [checkSourceEffectWitness] using accepted
  simpa [sourceEffectWitnessAgrees] using gates.1

theorem accepted_source_effect_witness_covers_every_boolean_prestate
    {statement : Stmt} {imported : Summary} {proposed : SourceEffectWitness}
    {post : Bool → Case → Bool}
    (sourceImport : summarize statement = some imported)
    (accepted : checkSourceEffectWitness imported proposed post = true) (initial : Bool) :
    run statement initial = some (proposed.paths.select initial) ∧
      post initial (proposed.paths.select initial) = true := by
  have exactPaths := accepted_source_effect_witness_has_exact_paths accepted
  have gates : sourceEffectWitnessAgrees imported proposed = true ∧
      checkContract imported post = true := by
    simpa [checkSourceEffectWitness] using accepted
  rw [exactPaths.2.2]
  exact accepted_contract_holds_on_every_source_path sourceImport gates.2 initial

theorem changed_source_effect_paths_reject
    {imported : Summary} {proposed : SourceEffectWitness} (post : Bool → Case → Bool)
    (changed : proposed.paths ≠ imported) :
    checkSourceEffectWitness imported proposed post = false := by
  simp [checkSourceEffectWitness, sourceEffectWitnessAgrees, changed]

theorem wrong_source_effect_prestate_labels_reject
    {imported : Summary} {proposed : SourceEffectWitness} (post : Bool → Case → Bool)
    (wrong : proposed.falseInitial ≠ false ∨ proposed.trueInitial ≠ true) :
    checkSourceEffectWitness imported proposed post = false := by
  rcases wrong with wrong | wrong
  · simp [checkSourceEffectWitness, sourceEffectWitnessAgrees, wrong]
  · simp [checkSourceEffectWitness, sourceEffectWitnessAgrees, wrong]

theorem matching_source_effect_witness_cannot_hide_contract_failure
    {imported : Summary} (proposed : SourceEffectWitness) {post : Bool → Case → Bool}
    (failed : checkContract imported post = false) :
    checkSourceEffectWitness imported proposed post = false := by
  simp [checkSourceEffectWitness, failed]

abbrev ChunkFields := Nat → Bool × Bool

def bindBusy (state : ChunkFields) (identity : Nat) (after : Bool) : ChunkFields :=
  fun other => if other = identity then (after, (state other).2) else state other

theorem busy_effect_preserves_lock
    (state : ChunkFields) (identity : Nat) (after : Bool) :
    (bindBusy state identity after identity).2 = (state identity).2 := by
  simp [bindBusy]

/-- Historical provider-specific name retained for existing audits. -/
theorem busy_effect_preserves_chunkio_lock
    (state : ChunkFields) (identity : Nat) (after : Bool) :
    (bindBusy state identity after identity).2 = (state identity).2 :=
  busy_effect_preserves_lock state identity after

theorem bound_effect_preserves_other_identities
    (state : ChunkFields) (identity other : Nat) (after : Bool)
    (different : other ≠ identity) :
    bindBusy state identity after other = state other := by
  simp [bindBusy, different]

def releaseBusy : Stmt :=
  .sequence (.branch (.equal .field (.literal 0)) (.ret (.literal (-1))) .empty)
    (.sequence (.assign (.literal 0)) (.ret (.literal 0)))

theorem release_busy_error_path_preserves_state :
    run releaseBusy false = some {
      state := { value := false, writes := [], branches := [true] }, result := -1 } := by
  decide

theorem release_busy_success_path_clears_busy :
    run releaseBusy true = some {
      state := { value := false, writes := [false], branches := [false] }, result := 0 } := by
  decide

/-! A validated frame helper is lowered only after its complete C body has
been checked to write ordinary, disjoint metadata fields. This abstract account
assumes that source/layout check and does not prove cross-unit C ABI equality. -/
theorem frame_assignment_preserves_boolean_state
    (state : State) (field : String) (value : Int) :
    (Stmt.frameAssign field value).eval state =
      some ({ state with frameWrites := state.frameWrites ++ [(field, value)] }, none) := rfl

def unlockWithReset : Stmt :=
  .sequence (.frameAssign "error_n" 0) releaseBusy

theorem unlock_reset_error_path_retains_metadata_write :
    run unlockWithReset false = some {
      state := { value := false, writes := [], branches := [true], frameWrites := [("error_n", 0)] }, result := -1 } := by
  decide

theorem unlock_reset_success_path_clears_lock :
    run unlockWithReset true = some {
      state := { value := false, writes := [false], branches := [false], frameWrites := [("error_n", 0)] }, result := 0 } := by
  decide

def bindLock (state : ChunkFields) (identity : Nat) (after : Bool) : ChunkFields :=
  fun other => if other = identity then ((state other).1, after) else state other

theorem lock_effect_preserves_wrapper_busy
    (state : ChunkFields) (identity : Nat) (after : Bool) :
    (bindLock state identity after identity).1 = (state identity).1 := by
  simp [bindLock]

theorem lock_effect_preserves_other_identities
    (state : ChunkFields) (identity other : Nat) (after : Bool)
    (different : other ≠ identity) :
    bindLock state identity after other = state other := by
  simp [bindLock, different]

/-! Generic frame-helper admission. `modeledFields` is the complete inventory of
modeled fields for the selected record. Rust obtains it from the loaded profile
and separately checks record/prototype/ordered field metadata, borrowed parameter
identity, source/build snapshots and bounded i32 constants. Cross-unit concrete
ABI equality remains an external premise; field type names alone do not prove
recursive record layout equality. These definitions do not infer those C facts. They make the disjoint-field premise of `frameAssign` explicit. -/
abbrev FieldStore := Nat → String → Int

def writeFrame (before : FieldStore) (identity : Nat) (field : String)
    (value : Int) : FieldStore :=
  fun object key => if object = identity ∧ key = field then value else before object key

def applyFrameWrites (before : FieldStore) (identity : Nat) : List (String × Int) → FieldStore
  | [] => before
  | (field, value) :: rest => applyFrameWrites (writeFrame before identity field value) identity rest

def frameWritesAllowed (modeledFields : List String) (writes : List (String × Int)) : Prop :=
  writes ≠ [] ∧ ∀ write ∈ writes, write.1 ∉ modeledFields

instance (modeledFields : List String) (writes : List (String × Int)) :
    Decidable (frameWritesAllowed modeledFields writes) := inferInstanceAs
      (Decidable (writes ≠ [] ∧ ∀ write ∈ writes, write.1 ∉ modeledFields))

def checkedFrameWrites (before : FieldStore) (identity : Nat)
    (modeledFields : List String) (writes : List (String × Int)) : Option FieldStore :=
  if frameWritesAllowed modeledFields writes then some (applyFrameWrites before identity writes) else none

theorem frame_writes_preserve_unwritten_field
    (before : FieldStore) (identity object : Nat) (field : String)
    (writes : List (String × Int)) (unwritten : ∀ write ∈ writes, write.1 ≠ field) :
    applyFrameWrites before identity writes object field = before object field := by
  induction writes generalizing before with
  | nil => rfl
  | cons write rest ih =>
    rcases write with ⟨key, value⟩
    have different := unwritten (key, value) (by simp)
    have tail : ∀ write ∈ rest, write.1 ≠ field := by
      intro write member
      exact unwritten write (by simp [member])
    simp only [applyFrameWrites, ih _ tail]
    simp [writeFrame, Ne.symm different]

theorem frame_writes_preserve_other_object
    (before : FieldStore) (identity object : Nat) (field : String)
    (writes : List (String × Int)) (different : object ≠ identity) :
    applyFrameWrites before identity writes object field = before object field := by
  induction writes generalizing before with
  | nil => rfl
  | cons write rest ih =>
    rcases write with ⟨key, value⟩
    simp [applyFrameWrites, ih, writeFrame, different]

theorem checked_frame_writes_preserve_every_modeled_field
    {before after : FieldStore} {identity : Nat} {modeledFields : List String}
    {writes : List (String × Int)}
    (checked : checkedFrameWrites before identity modeledFields writes = some after)
    (field : String) (modeled : field ∈ modeledFields) (object : Nat) :
    after object field = before object field := by
  unfold checkedFrameWrites at checked
  split at checked
  next allowed =>
    cases checked
    apply frame_writes_preserve_unwritten_field
    intro write member equal
    exact (allowed.2 write member) (equal ▸ modeled)
  next => contradiction

theorem checked_frame_writes_preserve_every_other_object
    {before after : FieldStore} {identity : Nat} {modeledFields : List String}
    {writes : List (String × Int)}
    (checked : checkedFrameWrites before identity modeledFields writes = some after)
    (object : Nat) (different : object ≠ identity) (field : String) :
    after object field = before object field := by
  unfold checkedFrameWrites at checked
  split at checked
  · cases checked
    exact frame_writes_preserve_other_object before identity object field writes different
  · contradiction

theorem modeled_field_write_rejects
    (before : FieldStore) (identity : Nat) (modeledFields : List String)
    (writes : List (String × Int)) (write : String × Int)
    (member : write ∈ writes) (modeled : write.1 ∈ modeledFields) :
    checkedFrameWrites before identity modeledFields writes = none := by
  have denied : ¬ frameWritesAllowed modeledFields writes := by
    intro allowed
    exact allowed.2 write member modeled
  simp [checkedFrameWrites, denied]

theorem empty_frame_helper_rejects
    (before : FieldStore) (identity : Nat) (modeledFields : List String) :
    checkedFrameWrites before identity modeledFields [] = none := by
  simp [checkedFrameWrites, frameWritesAllowed]

def frameConstant : Expr → Bool
  | .literal _ => true
  | .negate value => frameConstant value
  | _ => false

theorem admitted_frame_constant_is_prestate_independent
    (expression : Expr) (admitted : frameConstant expression = true)
    (first second : Bool) : expression.eval first = expression.eval second := by
  induction expression with
  | literal value => rfl
  | field => simp [frameConstant] at admitted
  | not value ih => simp [frameConstant] at admitted
  | negate value ih =>
    exact congrArg Neg.neg (ih admitted)
  | equal left right il ir => simp [frameConstant] at admitted
  | notEqual left right il ir => simp [frameConstant] at admitted

def frameProgram : List (String × Int) → Stmt
  | [] => .empty
  | (field, value) :: rest => .sequence (.frameAssign field value) (frameProgram rest)

theorem frame_program_retains_exact_ordered_writes
    (writes : List (String × Int)) (state : State) :
    (frameProgram writes).eval state =
      some ({ state with frameWrites := state.frameWrites ++ writes }, none) := by
  induction writes generalizing state with
  | nil => simp [frameProgram, Stmt.eval]
  | cons write rest ih =>
    rcases write with ⟨field, value⟩
    simp [frameProgram, Stmt.eval, ih, List.append_assoc]

theorem frame_program_preserves_boolean_and_control_state
    (writes : List (String × Int)) (state next : State) (result : Option Int)
    (evaluated : (frameProgram writes).eval state = some (next, result)) :
    next.value = state.value ∧ next.writes = state.writes ∧
      next.branches = state.branches ∧ result = none := by
  rw [frame_program_retains_exact_ordered_writes] at evaluated
  cases evaluated
  exact ⟨rfl, rfl, rfl, rfl⟩

theorem another_modeled_field_cannot_be_hidden_as_metadata
    (before : FieldStore) (identity : Nat) :
    checkedFrameWrites before identity ["active", "locked"] [("locked", 0)] = none := by
  apply modeled_field_write_rejects before identity ["active", "locked"] [("locked", 0)] ("locked", 0)
  · simp
  · simp

end QCore.ChunkSource
