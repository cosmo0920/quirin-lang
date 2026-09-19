namespace QCore.LoopSwitchCallEvaluation

/-! Card `B03-LOOP-AND-SWITCH`: evaluation of one modeled contract call in a
C controlling expression.

The gateway admits a call only after a lexical check establishes that it is
unconditionally evaluated within the controlling expression and the remaining
expression is custody-neutral. This module models the other half of the rule:
where the resulting call instruction sits in the CFG. A `while`/`for` call is
the loop header, a `do`/`while` call is the post-body header, and a `switch`
call is the one dispatch node. Thus back edges revisit the call while switch
arms do not, and both loop successors remain present.

This is an abstract rule theorem. Faithful C extraction and correspondence to
the Rust CFG builder remain review-maintained premises (constraint 28).
-/

inductive Position where
  | preTestLoop
  | postTestLoop
  | switchDispatch
  deriving DecidableEq, Repr

/-- Number of controlling-expression evaluations for an execution with the
given number of completed loop-body iterations. A post-test execution has at
least one body iteration; that premise is stated where the formula is used. -/
def evaluations : Position → Nat → Nat
  | .preTestLoop, bodyIterations => bodyIterations + 1
  | .postTestLoop, bodyIterations => bodyIterations
  | .switchDispatch, _ => 1

/-- The modeled contract effect appears once per controlling-expression
evaluation; it is never erased into a neutral branch node. -/
def effectTrace (effect : Nat) (position : Position) (bodyIterations : Nat) :
    List Nat :=
  List.replicate (evaluations position bodyIterations) effect

theorem a_pre_test_loop_evaluates_before_entry_and_after_each_iteration
    (iterations : Nat) :
    evaluations .preTestLoop iterations = iterations + 1 := by
  rfl

theorem a_post_test_loop_evaluates_after_every_completed_iteration
    {iterations : Nat} (oneOrMore : 0 < iterations) :
    evaluations .postTestLoop iterations = iterations := by
  cases iterations with
  | zero => cases Nat.not_lt_zero 0 oneOrMore
  | succ n => rfl

theorem a_switch_evaluates_exactly_once (armsTaken : Nat) :
    evaluations .switchDispatch armsTaken = 1 := by
  rfl

theorem every_visit_applies_the_modeled_effect
    (effect : Nat) (position : Position) (iterations : Nat) :
    (effectTrace effect position iterations).length =
      evaluations position iterations := by
  simp [effectTrace]

/-- The loop header retains both alternatives. The condition call changes the
instruction at the header, not its exit/back-edge rule. -/
def loopSuccessors (bodyEntry continuation : Nat) : List Nat :=
  [bodyEntry, continuation]

theorem a_modeled_call_never_removes_the_loop_exit
    (bodyEntry continuation : Nat) :
    continuation ∈ loopSuccessors bodyEntry continuation := by
  simp [loopSuccessors]

theorem a_modeled_call_keeps_the_loop_back_edge
    (bodyEntry continuation : Nat) :
    bodyEntry ∈ loopSuccessors bodyEntry continuation := by
  simp [loopSuccessors]

/-- A continue in every loop kind targets the controlling-expression node.
For a post-test loop this is after the body; for a pre-test loop it is before
the next possible body execution. -/
def continueTarget (conditionNode : Nat) : Nat := conditionNode

theorem continue_revisits_the_call (conditionNode : Nat) :
    evaluations .preTestLoop (continueTarget conditionNode + 1) =
      conditionNode + 2 := by
  simp [continueTarget, evaluations]

/-- Switch dispatch has no edge from an arm back to the call node. Its one
evaluation is independent of the number of case labels. -/
theorem case_count_cannot_repeat_switch_evaluation (caseCount : Nat) :
    (effectTrace 0 .switchDispatch caseCount).length = 1 := by
  simp [effectTrace, evaluations]

end QCore.LoopSwitchCallEvaluation
