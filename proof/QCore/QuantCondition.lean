/-! Explicit assertion positions. Predicates and execution correspondence are
inputs; declaring a condition proves neither its truth nor termination. -/
namespace QCore.QuantCondition
inductive Selector where
  | entry | normal | error (name : String) | any
  deriving DecidableEq, Repr
inductive Exit (State : Type u) where
  | normal (state : State)
  | error (name : String) (state : State)
/-- `none` means no terminating exit of this execution, not missing evidence.
Actual execution interpretation and completeness require independent inputs. -/
structure Execution (State : Type u) where
  entryState : State
  exitState : Option (Exit State)
def Satisfies (selector : Selector) (predicate : State → Prop)
    (execution : Execution State) : Prop :=
  match selector with
  | .entry => predicate execution.entryState
  | .normal => match execution.exitState with
    | some (.normal state) => predicate state
    | _ => True
  | .error expected => match execution.exitState with
    | some (.error actual state) => actual = expected → predicate state
    | _ => True
  | .any => match execution.exitState with
    | some (.normal state) | some (.error _ state) => predicate state
    | none => True

theorem any_covers_normal (predicate : State → Prop) (execution : Execution State)
    (allExits : Satisfies .any predicate execution)
    (returned : execution.exitState = some (.normal state)) : predicate state := by
  simpa [Satisfies, returned] using allExits

theorem any_covers_error (predicate : State → Prop) (execution : Execution State)
    (allExits : Satisfies .any predicate execution)
    (raised : execution.exitState = some (.error name state)) : predicate state := by
  simpa [Satisfies, raised] using allExits

theorem error_selection_is_exact (predicate : State → Prop) (execution : Execution State)
    (raised : execution.exitState = some (.error actual state)) (different : actual ≠ expected) :
    Satisfies (.error expected) predicate execution := by
  simp [Satisfies, raised, different]

theorem normal_assertion_does_not_supply_entry :
    Satisfies .normal (fun n : Nat => n = 1) ⟨0, some (.normal 1)⟩ ∧
    ¬ Satisfies .entry (fun n : Nat => n = 1) ⟨0, some (.normal 1)⟩ := by
  simp [Satisfies]

theorem entry_assertion_does_not_supply_normal :
    Satisfies .entry (fun n : Nat => n = 0) ⟨0, some (.normal 1)⟩ ∧
    ¬ Satisfies .normal (fun n : Nat => n = 0) ⟨0, some (.normal 1)⟩ := by
  simp [Satisfies]

def HasExit (execution : Execution State) : Prop := ∃ exit, execution.exitState = some exit

theorem exit_conditions_do_not_prove_termination (before : State) (predicate : State → Prop) :
    Satisfies .any predicate ⟨before, none⟩ ∧ ¬ HasExit (⟨before, none⟩ : Execution State) := by
  simp [Satisfies, HasExit]
end QCore.QuantCondition
