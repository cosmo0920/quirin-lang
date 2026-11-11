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

namespace QCore.TypedErrors

abbrev ErrorName := Nat
abbrev ResourceId := Nat
abbrev Ledger := List ResourceId

inductive Effect where
  | io
  | alloc
  | mutation
  | unsafety
  | throws (error : ErrorName)
  deriving DecidableEq

/-- A predicate representation of a finite effect row. The executable model
    uses an ordered finite set; the proofs need only membership and removal. -/
abbrev EffectRow := Effect → Prop

namespace EffectRow

def empty : EffectRow := fun _ => False
def insert (effect : Effect) (row : EffectRow) : EffectRow :=
  fun candidate => candidate = effect ∨ row candidate
def remove (effect : Effect) (row : EffectRow) : EffectRow :=
  fun candidate => candidate ≠ effect ∧ row candidate
def union (left right : EffectRow) : EffectRow :=
  fun effect => left effect ∨ right effect

theorem inserted (effect : Effect) (row : EffectRow) :
    insert effect row effect := by
  exact Or.inl rfl

theorem removed_absent (effect : Effect) (row : EffectRow) :
    ¬remove effect row effect := by
  intro present
  exact present.1 rfl

end EffectRow

inductive ExitTag where
  | normal
  | error (error : ErrorName)
  deriving DecidableEq

structure ExitState where
  ledger : Ledger
  effects : EffectRow

/-- A path-sensitive map: each tag retains every alternative state. -/
abbrev ExitMap := ExitTag → List ExitState

namespace ExitMap

def empty : ExitMap := fun _ => []

def singleton (tag : ExitTag) (state : ExitState) : ExitMap :=
  fun candidate => if candidate = tag then [state] else []

def append (left right : ExitMap) : ExitMap :=
  fun tag => left tag ++ right tag

def raise (error : ErrorName) (state : ExitState) : ExitMap :=
  singleton (.error error)
    { state with effects := EffectRow.insert (.throws error) state.effects }

theorem raise_has_no_normal_exit (error : ErrorName) (state : ExitState) :
    raise error state .normal = [] := by
  simp [raise, singleton]

theorem raise_preserves_error_ledger (error : ErrorName) (state : ExitState) :
    ∀ raised ∈ raise error state (.error error), raised.ledger = state.ledger := by
  intro raised member
  simp [raise, singleton] at member
  subst raised
  rfl

theorem raise_records_throw_effect (error : ErrorName) (state : ExitState) :
    ∀ raised ∈ raise error state (.error error), raised.effects (.throws error) := by
  intro raised member
  simp [raise, singleton] at member
  subst raised
  exact EffectRow.inserted (.throws error) state.effects

/-- `?` forwards the callee's declared normal and error post-states exactly. -/
def propagate (callee : ExitMap) : ExitMap := callee

theorem propagation_preserves_post_state (callee : ExitMap) (tag : ExitTag)
    (state : ExitState) (member : state ∈ callee tag) :
    state ∈ propagate callee tag := by
  exact member

/-- Sequential composition evaluates the continuation only on normal exits. -/
def seq (first : ExitMap) (next : ExitState → ExitMap) : ExitMap :=
  fun tag =>
    (first .normal).flatMap (fun state => next state tag) ++
      match tag with
      | .normal => []
      | .error error => first (.error error)

theorem seq_preserves_prior_error (first : ExitMap) (next : ExitState → ExitMap)
    (error : ErrorName) (state : ExitState) (member : state ∈ first (.error error)) :
    state ∈ seq first next (.error error) := by
  simp only [seq, List.mem_append]
  exact Or.inr member

/-- Ordinary branch merges require every normal alternative to have one
    common ownership ledger. Error alternatives need not agree. -/
def NormalJoinable (left right : ExitMap) : Prop :=
  ∃ ledger, ∀ state,
    state ∈ left .normal ∨ state ∈ right .normal → state.ledger = ledger

def branchJoin (left right : ExitMap) : ExitMap := append left right

theorem branch_join_preserves_normal_ledger (left right : ExitMap)
    (joinable : NormalJoinable left right) :
    ∃ ledger, ∀ state ∈ branchJoin left right .normal, state.ledger = ledger := by
  rcases joinable with ⟨ledger, agrees⟩
  refine ⟨ledger, ?_⟩
  intro state member
  apply agrees state
  simpa [branchJoin, append, List.mem_append] using member

theorem branch_retains_distinct_error_paths (left right : ExitMap)
    (error : ErrorName) (leftState rightState : ExitState)
    (inLeft : leftState ∈ left (.error error))
    (inRight : rightState ∈ right (.error error)) :
    leftState ∈ branchJoin left right (.error error) ∧
      rightState ∈ branchJoin left right (.error error) := by
  constructor
  · simp [branchJoin, append, inLeft]
  · simp [branchJoin, append, inRight]

def removeHandled (error : ErrorName) (state : ExitState) : ExitState :=
  { state with effects := EffectRow.remove (.throws error) state.effects }

/-- Cycle 5's cleanup-free catch turns matching errors into normal recovery
    paths. Cycle 6 generalizes this operation with cleanup capabilities. -/
def catchError (error : ErrorName) (body : ExitMap) : ExitMap :=
  fun tag => match tag with
    | .normal => body .normal ++ (body (.error error)).map (removeHandled error)
    | .error candidate => if candidate = error then [] else body (.error candidate)

def ZeroLocalObligations (baseline : Ledger) (error : ErrorName)
    (body : ExitMap) : Prop :=
  ∀ state ∈ body (.error error), state.ledger = baseline

theorem catch_requires_and_preserves_baseline (baseline : Ledger)
    (error : ErrorName) (body : ExitMap)
    (zero : ZeroLocalObligations baseline error body)
    (caught : ExitState) (member : caught ∈ body (.error error)) :
    removeHandled error caught ∈ catchError error body .normal ∧
      (removeHandled error caught).ledger = baseline := by
  constructor
  · simp only [catchError, List.mem_append]
    exact Or.inr (List.mem_map.mpr ⟨caught, member, rfl⟩)
  · simpa [removeHandled] using zero caught member

theorem catch_removes_handled_error_exit (error : ErrorName) (body : ExitMap) :
    catchError error body (.error error) = [] := by
  simp [catchError]

theorem catch_removes_handled_throw_effect (error : ErrorName) (state : ExitState) :
    ¬(removeHandled error state).effects (.throws error) := by
  exact EffectRow.removed_absent (.throws error) state.effects

def NormalLedger (exits : ExitMap) (ledger : Ledger) : Prop :=
  ∀ state ∈ exits .normal, state.ledger = ledger

/-- Derive the joined ledger from the two incoming path classes. Neither
premise mentions the result of `catchError`: ordinary normal exits agree, and
matching error exits have discharged their local obligations to that baseline. -/
theorem validated_catch_join_has_one_normal_ledger (error : ErrorName)
    (body : ExitMap) (ledger : Ledger)
    (normal : NormalLedger body ledger)
    (recovered : ZeroLocalObligations ledger error body) :
    ∀ state ∈ catchError error body .normal, state.ledger = ledger := by
  intro state member
  simp only [catchError, List.mem_append, List.mem_map] at member
  rcases member with ordinary | ⟨caught, matching, rfl⟩
  · exact normal state ordinary
  · exact recovered caught matching

/-- Both premises are necessary, not just sufficient. The catch retains the
ordinary paths and the ledgers of the recovered paths. -/
theorem catch_normal_ledger_iff_incoming_ledgers (error : ErrorName)
    (body : ExitMap) (ledger : Ledger) :
    NormalLedger (catchError error body) ledger ↔
      NormalLedger body ledger ∧ ZeroLocalObligations ledger error body := by
  constructor
  · intro joined
    constructor
    · intro state member
      exact joined state (by simp [catchError, member])
    · intro state member
      apply joined (removeHandled error state)
      exact List.mem_append_right _ (List.mem_map.mpr ⟨state, member, rfl⟩)
  · rintro ⟨normal, recovered⟩
    exact validated_catch_join_has_one_normal_ledger error body ledger normal recovered

/-- A nonempty ordinary path and a nonempty recovery path really join. -/
theorem matching_incoming_ledgers_join (error : ErrorName) (ledger : Ledger) :
    let state : ExitState := ⟨ledger, EffectRow.empty⟩
    let body := append (singleton .normal state) (raise error state)
    NormalLedger (catchError error body) ledger := by
  intro state body
  apply validated_catch_join_has_one_normal_ledger
  · simp [NormalLedger, body, append, singleton, raise, state]
  · simp [ZeroLocalObligations, body, append, singleton, raise, state]

/-- The recovery premise cannot be dropped: a matching error retains a
resource absent from the otherwise consistent normal baseline. -/
theorem a_mismatched_recovery_ledger_does_not_join (error : ErrorName) :
    let ordinary : ExitState := ⟨[], EffectRow.empty⟩
    let caught : ExitState := ⟨[0], EffectRow.empty⟩
    let body := append (singleton .normal ordinary) (raise error caught)
    NormalLedger body [] ∧ ¬NormalLedger (catchError error body) [] := by
  simp [NormalLedger, catchError, append, singleton, raise, removeHandled]

/-- The normal premise cannot be dropped either, even when recovery has no
remaining obligation. -/
theorem a_mismatched_ordinary_ledger_does_not_join (error : ErrorName) :
    let ordinary : ExitState := ⟨[0], EffectRow.empty⟩
    let caught : ExitState := ⟨[], EffectRow.empty⟩
    let body := append (singleton .normal ordinary) (raise error caught)
    ZeroLocalObligations [] error body ∧ ¬NormalLedger (catchError error body) [] := by
  simp [ZeroLocalObligations, NormalLedger, catchError, append, singleton, raise,
    removeHandled]

def AcceptedErrors (declared : ErrorName → Prop) (allowed : EffectRow)
    (exits : ExitMap) : Prop :=
  ∀ error state, state ∈ exits (.error error) →
    declared error ∧ allowed (.throws error)

theorem accepted_error_is_declared (declared : ErrorName → Prop)
    (allowed : EffectRow) (exits : ExitMap)
    (accepted : AcceptedErrors declared allowed exits)
    (error : ErrorName) (state : ExitState)
    (member : state ∈ exits (.error error)) : declared error := by
  exact (accepted error state member).1

theorem accepted_error_is_in_effect_row (declared : ErrorName → Prop)
    (allowed : EffectRow) (exits : ExitMap)
    (accepted : AcceptedErrors declared allowed exits)
    (error : ErrorName) (state : ExitState)
    (member : state ∈ exits (.error error)) : allowed (.throws error) := by
  exact (accepted error state member).2

end ExitMap
end QCore.TypedErrors
