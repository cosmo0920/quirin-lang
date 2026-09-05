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

theorem validated_catch_join_has_one_normal_ledger (error : ErrorName)
    (body : ExitMap) (ledger : Ledger)
    (validated : NormalLedger (catchError error body) ledger) :
    ∀ state ∈ catchError error body .normal, state.ledger = ledger := by
  exact validated

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
