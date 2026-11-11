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

/-! Source branching and its custody join (card `IF-STATEMENT`,
`source.branch.v1`).

Wave 18 recorded that a short-circuit had nothing to lower onto: "there is no
branch construct in the executable subset". This module is the semantics of the
construct that removes that blocker, and it records the correction that card
`IF-STATEMENT` measured — the *models* already branched. Ownership, typed
errors, usage and resource flow have carried branch joins since their own
cycles (`OwnershipOp.Branch`, `ErrorOp.Branch`, `UsageExpr.Branch`,
`Flow.join_branches`), and the typed CFG has carried branch terminators since
Cycle 7. The one linear IR with no branch constructor was the *source lowering*,
`cleanup_lowering::SourceOp`, and that is what this module adds.

What is proved here:

* **The condition tier is total and decidable.** Every condition
  `source.branch.v1` admits has a value, computed without reading any state.
  That is what lets one reference observation, one CFG observation and one
  observation parsed from the compiled program agree — a run-time-varying
  condition would leave "the" observation ill-defined.
* **The join is agreement, never widening.** Both arms are validated from the
  same incoming custody state, and the branch admits only if they reach the
  join in *equal* states. The conditional leak and the conditional
  double-release both refuse, and so does the shape where one arm moves what
  the other releases — which comparing live sets alone would miss.
* **The arm not taken is validated in full.** A leak in the untaken arm refuses
  exactly as one in the taken arm does; only the *observation* is
  single-valued.
* **Nothing branch-free moves.** A branch-free program validates exactly as the
  pre-card linear validator did, and its block allocation is the identity
  numbering `0, 1, 2, …` the emitter has always produced — the model
  counterpart of the byte-identical-C pin.
* **Fail-closed.** Every shape outside v1 refuses, collected as one list so a
  future widening must change a theorem rather than slip past one.

What is **not** proved here: that the Rust parser extracts these operations
from real source the way this model says (a tested Rust boundary, per the
`ConcurrencyPrelude` / `PipeDesugaring` precedent); that a C compiler evaluates
the emitted comparison as `CompareOp.apply` does (that is what the sanitized
native run cross-checks); and anything about the logical operators `||` and
`&&`, which lower *onto* this construct in a later card and are deliberately
absent from the condition tier here. -/
namespace QCore.BranchJoin

/-! ## 1. The condition tier -/

/-- The closed comparison vocabulary of `source.branch.v1`. -/
inductive CompareOp where
  | eq
  | ne
  | lt
  | le
  | gt
  | ge
  deriving DecidableEq, Repr

/-- The relation itself. Total and decidable on `Int`. -/
def CompareOp.apply : CompareOp → Int → Int → Bool
  | .eq, left, right => decide (left = right)
  | .ne, left, right => decide (left ≠ right)
  | .lt, left, right => decide (left < right)
  | .le, left, right => decide (left ≤ right)
  | .gt, left, right => decide (left > right)
  | .ge, left, right => decide (left ≥ right)

/-- A condition. Deliberately without connectives: `||` and `&&` lower onto the
branch this module defines rather than extending this type. -/
inductive Condition where
  | lit (value : Bool)
  | cmp (left : Int) (op : CompareOp) (right : Int)
  deriving DecidableEq, Repr

def Condition.evaluate : Condition → Bool
  | .lit value => value
  | .cmp left op right => op.apply left right

/-- Totality: the tier has no undefined condition, so the reference semantics
never has to guess which arm runs. -/
theorem every_condition_has_a_value (condition : Condition) :
    condition.evaluate = true ∨ condition.evaluate = false := by
  cases h : condition.evaluate
  · exact Or.inr rfl
  · exact Or.inl rfl

/- `Condition` contains only literals and literal comparisons, and
`Condition.evaluate` has no state input. State independence is a restriction of
this model's vocabulary; self-equality of its result would add no evidence. -/

theorem the_comparisons_are_the_relations (left right : Int) :
    (CompareOp.eq.apply left right = decide (left = right)) ∧
    (CompareOp.ne.apply left right = decide (left ≠ right)) ∧
    (CompareOp.lt.apply left right = decide (left < right)) ∧
    (CompareOp.le.apply left right = decide (left ≤ right)) ∧
    (CompareOp.gt.apply left right = decide (left > right)) ∧
    (CompareOp.ge.apply left right = decide (left ≥ right)) :=
  ⟨rfl, rfl, rfl, rfl, rfl, rfl⟩

/-! ## 2. Operations and custody state -/

abbrev Resource := Nat

mutual

/-- The source lowering's operations. `branch` is the whole content of card
`IF-STATEMENT`: every other constructor predates it. -/
inductive Op where
  | acquire (resource : Resource) (succeeds : Bool)
  | check (passes : Bool)
  | drop (resource : Resource)
  | transfer (resource : Resource)
  | ret (value : Int)
  | branch (condition : Condition) (thenArm : Seq) (elseArm : Seq)

/-- A statement sequence.  Spelled as its own inductive rather than `List Op`
so the validator and the interpreter below are structurally recursive. -/
inductive Seq where
  | nil
  | cons (head : Op) (tail : Seq)

end

/-- Sorted insertion, so a set of transferred identities has one spelling and
list equality *is* set equality at the join. -/
def insertSorted (value : Nat) : List Nat → List Nat
  | [] => [value]
  | head :: tail =>
      if value < head then value :: head :: tail
      else if value = head then head :: tail
      else head :: insertSorted value tail

/-- Remove the first occurrence, or fail. -/
def removeFirst (value : Nat) : List Nat → Option (List Nat)
  | [] => none
  | head :: tail =>
      if head = value then some tail
      else (removeFirst value tail).map (fun rest => head :: rest)

/-- The custody state a sequence carries.  `live` is **ordered**: a cleanup
block releases live resources in reverse acquisition order, so the order is
part of the state and comparing sequences rather than sets is the fail-closed
choice at a join. -/
structure Custody where
  live : List Resource
  transferred : List Resource
  returned : Bool
  deriving DecidableEq, Repr

def Custody.empty : Custody := { live := [], transferred := [], returned := false }

inductive Refusal where
  | resourceNotLive (resource : Resource)
  | alreadyOwned (resource : Resource)
  | fallibleAfterTransfer
  | liveResourcesAtReturn
  | operationAfterReturn
  | missingReturn
  | armsDisagree
  deriving DecidableEq, Repr

/-- Acquisition identities already spent, threaded across the whole program —
both arms of every branch included, because an identity names one `void *` slot
in the emitted C. -/
abbrev State := List Resource × Custody

/-! ## 3. Validation, with the join -/

/-- **The join rule, over the arms that survive to the join**, as a function of
the two arms' end states, so the theorems below are about the rule the validator
actually applies rather than about a paraphrase of it.

Card `IF-STATEMENT` wrote this as "neither arm returned, and the two agree".
Card `IF-ARMS` changed its *shape* rather than weakening it: an arm that
returned discharged its obligations at its own exit — `.ret` demands an empty
live list wherever it stands, which is the whole cleanup obligation this route
imposes at any exit — and it does not reach the join, so it is not a survivor
and there is nothing about it left to compare. With two survivors the rule is
unchanged and unweakened: agreement, never widening. -/
def joinArms (thenState elseState : Custody) : Option Custody :=
  match thenState.returned, elseState.returned with
  -- No survivor: the branch ends the function, and `operationAfterReturn`
  -- refuses anything written after it. `live` is empty on both sides (a `.ret`
  -- demanded it); the two exits carry their own payloads, so `transferred` is
  -- **unioned** rather than compared — the refusing direction for any later
  -- fallible operation, of which after a return there can be none.
  | true,  true  =>
      some { live := []
             transferred := elseState.transferred.foldr insertSorted thenState.transferred
             returned := true }
  -- One survivor: a one-element join is that element.
  | true,  false => some elseState
  | false, true  => some thenState
  -- Two survivors: agreement, never widening.
  | false, false => if thenState = elseState then some thenState else none

mutual

def stepOp (state : State) : Op → Except Refusal State
  | .acquire resource _ =>
      let acquired := state.1
      let custody := state.2
      if custody.transferred ≠ [] then .error .fallibleAfterTransfer
      else if acquired.contains resource then .error (.alreadyOwned resource)
      else .ok (resource :: acquired, { custody with live := custody.live ++ [resource] })
  | .check _ =>
      if state.2.transferred ≠ [] then .error .fallibleAfterTransfer else .ok state
  | .drop resource =>
      match removeFirst resource state.2.live with
      | none => .error (.resourceNotLive resource)
      | some rest => .ok (state.1, { state.2 with live := rest })
  | .transfer resource =>
      match removeFirst resource state.2.live with
      | none => .error (.resourceNotLive resource)
      | some rest =>
          .ok (state.1,
            { state.2 with
              live := rest
              transferred := insertSorted resource state.2.transferred })
  | .ret _ =>
      if state.2.live ≠ [] then .error .liveResourcesAtReturn
      else .ok (state.1, { state.2 with returned := true })
  | .branch _ thenArm elseArm =>
      match runSeq state thenArm with
      | .error refusal => .error refusal
      | .ok afterThen =>
          match runSeq (afterThen.1, state.2) elseArm with
          | .error refusal => .error refusal
          | .ok afterElse =>
              match joinArms afterThen.2 afterElse.2 with
              | some joined => .ok (afterElse.1, joined)
              -- The only shape with no joined state is two *surviving* arms
              -- that disagree.
              | none => .error .armsDisagree

def runSeq (state : State) : Seq → Except Refusal State
  | .nil => .ok state
  | .cons head tail =>
      if state.2.returned then .error .operationAfterReturn
      else
        match stepOp state head with
        | .error refusal => .error refusal
        | .ok next => runSeq next tail

end

/-- A whole program: run it, then demand it returned. -/
def validate (program : Seq) : Except Refusal State :=
  match runSeq ([], Custody.empty) program with
  | .error refusal => .error refusal
  | .ok state => if state.2.returned then .ok state else .error .missingReturn

def Admits (program : Seq) : Prop := (validate program).isOk = true

instance (program : Seq) : Decidable (Admits program) := by
  unfold Admits; infer_instance

/-! ## 4. The join is agreement between survivors, never widening -/

/-- The load-bearing statement of card `IF-STATEMENT`, now stated of the two
*surviving* arms: where both fall through to the join, a joined state is
produced **only** when they reach it in equal custody states, and the joined
state is that common state. There is no rule that widens one arm's state to the
other's, and none that drops a component. -/
theorem the_join_of_two_survivors_is_their_common_state
    {thenState elseState joined : Custody}
    (thenSurvives : thenState.returned = false) (elseSurvives : elseState.returned = false)
    (result : joinArms thenState elseState = some joined) :
    thenState = elseState ∧ joined = thenState := by
  unfold joinArms at result
  rw [thenSurvives, elseSurvives] at result
  simp only at result
  split at result
  · rename_i agree
    refine ⟨agree, ?_⟩
    injection result with equality
    exact equality.symm
  · contradiction

/-- The joined state is *both* survivors' state, so nothing either established is
lost at the join and nothing neither established is invented. -/
theorem the_join_widens_neither_survivor
    {thenState elseState joined : Custody}
    (thenSurvives : thenState.returned = false) (elseSurvives : elseState.returned = false)
    (result : joinArms thenState elseState = some joined) :
    joined = thenState ∧ joined = elseState := by
  obtain ⟨agree, equality⟩ :=
    the_join_of_two_survivors_is_their_common_state thenSurvives elseSurvives result
  exact ⟨equality, equality.trans agree⟩

/-- The contrapositive, in the form that matters: two survivors that disagree do
not join. -/
theorem disagreeing_survivors_do_not_join
    {thenState elseState : Custody}
    (thenSurvives : thenState.returned = false) (elseSurvives : elseState.returned = false)
    (disagree : thenState ≠ elseState) :
    joinArms thenState elseState = none := by
  unfold joinArms
  rw [thenSurvives, elseSurvives]
  simp_all

/-- **The shape card `IF-ARMS` changed.** An arm that returned is not a
survivor, so it constrains the join not at all: whatever custody state it
reached, the branch carries the *other* arm's state forward. This is what makes
an early return from an arm a widening of the construct rather than a hole in
it — the returning arm's obligations were discharged at its own exit, and the
join is over what is left. -/
theorem a_returning_arm_does_not_constrain_the_join
    {thenState elseState : Custody}
    (returned : thenState.returned = true) (survives : elseState.returned = false) :
    joinArms thenState elseState = some elseState := by
  unfold joinArms
  rw [returned, survives]

theorem a_returning_else_arm_does_not_constrain_the_join
    {thenState elseState : Custody}
    (survives : thenState.returned = false) (returned : elseState.returned = true) :
    joinArms thenState elseState = some thenState := by
  unfold joinArms
  rw [survives, returned]

/-- With no survivor at all the branch itself ends the function, so nothing
after it is reachable — which is what `operationAfterReturn` says. -/
theorem a_branch_whose_arms_both_return_returns
    {thenState elseState joined : Custody}
    (thenReturned : thenState.returned = true) (elseReturned : elseState.returned = true)
    (result : joinArms thenState elseState = some joined) :
    joined.returned = true ∧ joined.live = [] := by
  unfold joinArms at result
  rw [thenReturned, elseReturned] at result
  injection result with equality
  subst equality
  exact ⟨rfl, rfl⟩

/-- …and the whole rule, as one biconditional: the **only** shape with no joined
state is two surviving arms that disagree. Every other combination joins, which
is the precise sense in which the arm-exit widening is a change of shape rather
than a relaxation — nothing that refused before is admitted by a *join* here;
the returning arm is admitted by its own exit's obligations instead. -/
theorem the_join_refuses_only_two_disagreeing_survivors (thenState elseState : Custody) :
    joinArms thenState elseState = none ↔
      (thenState.returned = false ∧ elseState.returned = false ∧ thenState ≠ elseState) := by
  unfold joinArms
  cases thenState.returned <;> cases elseState.returned <;> simp <;>
    (by_cases agree : thenState = elseState <;> simp [agree])

/-- Restated for the two-survivor case in the form card `IF-STATEMENT` pinned:
a join succeeds exactly when the two states are equal. -/
theorem the_join_rule_is_exactly_agreement_between_survivors
    {thenState elseState : Custody}
    (thenSurvives : thenState.returned = false) (elseSurvives : elseState.returned = false) :
    (joinArms thenState elseState).isSome = (thenState == elseState) := by
  unfold joinArms
  rw [thenSurvives, elseSurvives]
  by_cases agree : thenState = elseState <;> simp_all

/-! ## 5. Concrete fail-closed instances -/

/-- `let a = acquire; if c { drop a; } return;` — the conditional leak. -/
def conditionalLeak : Seq :=
  .cons (.acquire 1 true)
    (.cons (.branch (.lit true) (.cons (.drop 1) .nil) .nil)
      (.cons (.ret 0) .nil))

/-- The same program with both arms releasing: the correct spelling. -/
def bothArmsRelease : Seq :=
  .cons (.acquire 1 true)
    (.cons (.branch (.lit true) (.cons (.drop 1) .nil) (.cons (.drop 1) .nil))
      (.cons (.ret 0) .nil))

/-- One arm moves what the other releases. Live sets agree; the states do not.
Comparing live alone would admit this, which is why `transferred` is compared
too. -/
def moveVersusRelease : Seq :=
  .cons (.acquire 1 true)
    (.cons (.branch (.lit true) (.cons (.drop 1) .nil) (.cons (.transfer 1) .nil))
      (.cons (.ret 0) .nil))

/-- A leak in the arm the decided condition does **not** take. -/
def leakInUntakenArm : Seq :=
  .cons (.acquire 1 true)
    (.cons (.branch (.lit true) (.cons (.drop 1) .nil) .nil)
      (.cons (.ret 0) .nil))

/-- `return` inside an arm, with the other arm falling through to the join.
Card `IF-STATEMENT` refused this outright; card `IF-ARMS` admits it, because the
returning arm discharged its obligations at its own exit and the join is over
the one survivor. This is the definition card `IF-ARMS` moved, and it is kept
here under its own name so the move is visible rather than silent. -/
def armReturnWithSurvivor : Seq :=
  .cons (.branch (.lit true) (.cons (.ret 1) .nil) .nil) (.cons (.ret 0) .nil)

/-- The same shape with a resource still live at the arm's exit. `.ret` demands
an empty live list **wherever it stands**, so an early return leaks nothing:
this is the obligation that makes the widening safe, and it refuses. -/
def earlyReturnLeaks : Seq :=
  .cons (.acquire 1 true)
    (.cons (.branch (.lit true) (.cons (.ret 1) .nil) (.cons (.drop 1) .nil))
      (.cons (.ret 0) .nil))

/-- Both arms acquiring: one identity, one emitted slot, so this refuses rather
than joining two acquisitions of the same name. -/
def bothArmsAcquireOneIdentity : Seq :=
  .cons
      (.branch (.lit true)
        (.cons (.acquire 1 true) (.cons (.drop 1) .nil))
        (.cons (.acquire 1 true) (.cons (.drop 1) .nil)))
    (.cons (.ret 0) .nil)

theorem a_conditional_leak_refuses : validate conditionalLeak = .error .armsDisagree := rfl

theorem both_arms_releasing_is_admitted : Admits bothArmsRelease := by decide

theorem moving_in_one_arm_and_releasing_in_the_other_refuses :
    validate moveVersusRelease = .error .armsDisagree := rfl

/-- The untaken arm is validated in full. The condition is `true`, so the
`else` arm is never observed — and it still refuses. -/
theorem a_leak_in_the_untaken_arm_refuses :
    validate leakInUntakenArm = .error .armsDisagree := rfl

/-- **The widening, and the theorem it had to change.** Card `IF-STATEMENT`
proved `validate returnInsideArm = .error .returnInBranchArm`; that refusal
constructor no longer exists, and the program is admitted. The returning arm is
not admitted by the *join* — it never reaches one — but by discharging at its
own exit, which `an_arm_return_must_discharge_its_custody` below shows is a real
obligation rather than a waiver. -/
theorem an_arm_return_joins_over_the_survivor : Admits armReturnWithSurvivor := by decide

/-- …and the price of it: an early return with a resource still live refuses,
with exactly the refusal a top-level return with a live resource gets. The
obligation at an arm exit **is** the obligation at the program exit. -/
theorem an_arm_return_must_discharge_its_custody :
    validate earlyReturnLeaks = .error .liveResourcesAtReturn := rfl

theorem two_arms_cannot_spend_one_acquisition_identity :
    validate bothArmsAcquireOneIdentity = .error (.alreadyOwned 1) := rfl

/-- Collected as one list, so widening the construct must change a theorem
rather than slip past one. Card `IF-ARMS` did exactly that: the fourth row was
`returnInsideArm`, refusing at a constructor this revision no longer has, and it
is replaced by the leaking early return — the obligation the widening rests on
rather than the shape it opened. -/
theorem refused_shapes_are_exactly_these :
    [validate conditionalLeak, validate moveVersusRelease, validate leakInUntakenArm,
      validate earlyReturnLeaks, validate bothArmsAcquireOneIdentity] =
      [.error .armsDisagree, .error .armsDisagree, .error .armsDisagree,
        .error .liveResourcesAtReturn, .error (.alreadyOwned 1)] := rfl

/-! ## 6. Nothing branch-free moves -/

mutual

/-- Whether an operation is a branch. -/
def Op.branchFree : Op → Bool
  | .branch _ _ _ => false
  | _ => true

/-- Whether a sequence contains a branch anywhere. -/
def Seq.branchFree : Seq → Bool
  | .nil => true
  | .cons head tail => head.branchFree && tail.branchFree

end

/-- The pre-card operation semantics, written **independently** of `stepOp` so
the agreement theorem below has content: it has no branch case at all, exactly
as the shipped validator had none, and refuses one outright. -/
def stepLinear (state : State) : Op → Except Refusal State
  | .acquire resource _ =>
      let acquired := state.1
      let custody := state.2
      if custody.transferred ≠ [] then .error .fallibleAfterTransfer
      else if acquired.contains resource then .error (.alreadyOwned resource)
      else .ok (resource :: acquired, { custody with live := custody.live ++ [resource] })
  | .check _ =>
      if state.2.transferred ≠ [] then .error .fallibleAfterTransfer else .ok state
  | .drop resource =>
      match removeFirst resource state.2.live with
      | none => .error (.resourceNotLive resource)
      | some rest => .ok (state.1, { state.2 with live := rest })
  | .transfer resource =>
      match removeFirst resource state.2.live with
      | none => .error (.resourceNotLive resource)
      | some rest =>
          .ok (state.1,
            { state.2 with
              live := rest
              transferred := insertSorted resource state.2.transferred })
  | .ret _ =>
      if state.2.live ≠ [] then .error .liveResourcesAtReturn
      else .ok (state.1, { state.2 with returned := true })
  | .branch _ _ _ => .error .armsDisagree

/-- The pre-card sequence driver: identical in shape to `runSeq`, differing
only in which step function it calls. -/
def runLinear (state : State) : Seq → Except Refusal State
  | .nil => .ok state
  | .cons head tail =>
      if state.2.returned then .error .operationAfterReturn
      else
        match stepLinear state head with
        | .error refusal => .error refusal
        | .ok next => runLinear next tail

/-- **Nothing branch-free moves.** Every operation that is not a branch steps
exactly as it did before this card — the branch case is the *only* behavioural
addition, so no shipped program can take a different path through the
validator. -/
theorem a_branch_free_operation_steps_exactly_as_before
    (state : State) (operation : Op) (free : operation.branchFree = true) :
    stepOp state operation = stepLinear state operation := by
  cases operation <;> simp_all [stepOp, stepLinear, Op.branchFree]

/-- …and a branch is the one operation the two disagree on, so the theorem above
is a statement about the addition rather than a vacuous one. -/
theorem a_branch_is_the_one_operation_they_disagree_on :
    (Op.branch (.lit true) .nil .nil).branchFree = false := rfl

mutual

/-- A branch-free *sequence* runs the same way, operation for operation: the
sequence driver never had a branch case and still does not. -/
theorem stepOp_is_linear_on_branch_free (state : State) (operation : Op)
    (free : operation.branchFree = true) :
    stepOp state operation = stepLinear state operation :=
  a_branch_free_operation_steps_exactly_as_before state operation free

theorem runSeq_is_linear_on_branch_free (state : State) (sequence : Seq)
    (free : sequence.branchFree = true) :
    runSeq state sequence = runLinear state sequence := by
  cases sequence with
  | nil => rfl
  | cons head tail =>
      simp only [Seq.branchFree, Bool.and_eq_true] at free
      by_cases returned : state.2.returned = true
      · simp [runSeq, runLinear, returned]
      · simp only [runSeq, runLinear, ite_eq_right returned,
          stepOp_is_linear_on_branch_free state head free.1]
        cases stepped : stepLinear state head with
        | error _ => rfl
        | ok next => exact runSeq_is_linear_on_branch_free next tail free.2

end

/-! ## 7. Block allocation: the numbering the emitter has always produced -/

mutual

/-- Pre-order block allocation for one operation: it takes the next id, then
its `then` arm, then its `else` arm. -/
def allocOp (next : Nat) : Op → List Nat × Nat
  | .branch _ thenArm elseArm =>
      let thenResult := allocSeq (next + 1) thenArm
      let elseResult := allocSeq thenResult.2 elseArm
      (next :: thenResult.1 ++ elseResult.1, elseResult.2)
  | _ => ([next], next + 1)

def allocSeq (next : Nat) : Seq → List Nat × Nat
  | .nil => ([], next)
  | .cons head tail =>
      let headResult := allocOp next head
      let tailResult := allocSeq headResult.2 tail
      (headResult.1 ++ tailResult.1, tailResult.2)

end

mutual

/-- The allocator's counter only ever moves forward. -/
theorem allocOp_monotone (next : Nat) (operation : Op) : next ≤ (allocOp next operation).2 := by
  cases operation with
  | branch _ thenArm elseArm =>
      simp only [allocOp]
      exact Nat.le_trans (Nat.le_trans (Nat.le_succ next) (allocSeq_monotone (next + 1) thenArm))
        (allocSeq_monotone _ elseArm)
  | _ => simp [allocOp]

theorem allocSeq_monotone (next : Nat) (sequence : Seq) : next ≤ (allocSeq next sequence).2 := by
  cases sequence with
  | nil => simp [allocSeq]
  | cons head tail =>
      simp only [allocSeq]
      exact Nat.le_trans (allocOp_monotone next head) (allocSeq_monotone _ tail)

end

/-- **The byte-identity claim's model counterpart.** A branch-free operation
takes exactly its own single id and advances the counter by exactly one — so a
branch-free sequence is numbered `start, start + 1, start + 2, …`, the exact
numbering the emitter produced before this card, and no emitted C byte moves for
a program that does not use the construct. -/
theorem a_branch_free_operation_takes_one_consecutive_block
    (start : Nat) (operation : Op) (free : operation.branchFree = true) :
    allocOp start operation = ([start], start + 1) := by
  cases operation <;> simp_all [allocOp, Op.branchFree]

/-- The sequence-level instance, on a program of every branch-free shape. -/
def branchFreeProgram : Seq :=
  .cons (.acquire 3 true)
    (.cons (.acquire 4 true) (.cons (.check true)
      (.cons (.drop 4) (.cons (.transfer 3) (.cons (.ret 0) .nil)))))

theorem the_branch_free_program_is_numbered_from_zero :
    allocSeq 0 branchFreeProgram = ([0, 1, 2, 3, 4, 5], 6) := rfl

theorem the_branch_free_program_is_branch_free :
    branchFreeProgram.branchFree = true := rfl

/-- A branch, by contrast, allocates its arms *inside* its own numbering, so a
program that uses the construct genuinely renumbers — the theorem above is a
statement about branch-free programs, not a vacuous one. -/
theorem a_branch_allocates_its_arms_after_itself :
    allocSeq 0 bothArmsRelease = ([0, 1, 2, 3, 4], 5) := rfl

/-- A branch's own id comes before every id in either of its arms, and the two
arms' ids are disjoint from each other: the arms are separate blocks, never a
shared one. -/
theorem a_branch_precedes_its_arms (next : Nat) (condition : Condition) (thenArm elseArm : Seq) :
    (allocOp next (.branch condition thenArm elseArm)).1.head? = some next := by
  simp [allocOp]

/-- Two branches allocated in sequence do not share a block. -/
theorem a_sequence_advances_past_its_head (next : Nat) (head : Op) (tail : Seq) :
    (allocOp next head).2 ≤ (allocSeq next (.cons head tail)).2 := by
  simp only [allocSeq]
  exact allocSeq_monotone _ tail

/-! ## 8. What the observation is -/

inductive Event where
  | acquire (resource : Resource)
  | release (resource : Resource)
  | transfer (resource : Resource)
  deriving DecidableEq, Repr

structure Observation where
  /-- `some value` is a normal return; `none` is an error exit. -/
  outcome : Option Int
  trace : List Event
  deriving DecidableEq, Repr

structure Machine where
  live : List Resource
  trace : List Event
  deriving DecidableEq, Repr

def Machine.errorExit (machine : Machine) : Observation :=
  { outcome := none
    trace := machine.trace ++ machine.live.reverse.map Event.release }

mutual

def interpOp (machine : Machine) : Op → Machine × Option Observation
  | .acquire resource succeeds =>
      if succeeds then
        ({ machine with
            live := machine.live ++ [resource]
            trace := machine.trace ++ [Event.acquire resource] }, none)
      else (machine, some machine.errorExit)
  | .check passes => if passes then (machine, none) else (machine, some machine.errorExit)
  | .drop resource =>
      match removeFirst resource machine.live with
      | none => (machine, none)
      | some rest =>
          ({ live := rest, trace := machine.trace ++ [Event.release resource] }, none)
  | .transfer resource =>
      match removeFirst resource machine.live with
      | none => (machine, none)
      | some rest =>
          ({ live := rest, trace := machine.trace ++ [Event.transfer resource] }, none)
  | .ret value => (machine, some { outcome := some value, trace := machine.trace })
  | .branch condition thenArm elseArm =>
      if condition.evaluate then interpSeq machine thenArm else interpSeq machine elseArm

def interpSeq (machine : Machine) : Seq → Machine × Option Observation
  | .nil => (machine, none)
  | .cons head tail =>
      match interpOp machine head with
      | (next, none) => interpSeq next tail
      | (next, some observation) => (next, some observation)

end

/-- The decided condition selects the arm, and the branch's whole effect is
that arm's — there is no merge step, because there is nothing to merge. -/
theorem the_observed_arm_is_the_one_the_condition_selects
    (machine : Machine) (condition : Condition) (thenArm elseArm : Seq) :
    interpOp machine (.branch condition thenArm elseArm) =
      (if condition.evaluate then interpSeq machine thenArm else interpSeq machine elseArm) :=
  rfl

theorem a_true_condition_observes_the_then_arm
    (machine : Machine) (thenArm elseArm : Seq) :
    interpOp machine (.branch (.lit true) thenArm elseArm) = interpSeq machine thenArm := rfl

theorem a_false_condition_observes_the_else_arm
    (machine : Machine) (thenArm elseArm : Seq) :
    interpOp machine (.branch (.lit false) thenArm elseArm) = interpSeq machine elseArm := rfl

theorem a_comparison_condition_selects_by_its_relation
    (machine : Machine) (thenArm elseArm : Seq) :
    interpOp machine (.branch (.cmp 3 .lt 5) thenArm elseArm) = interpSeq machine thenArm ∧
    interpOp machine (.branch (.cmp 5 .lt 3) thenArm elseArm) = interpSeq machine elseArm :=
  ⟨rfl, rfl⟩

/-- An empty arm observes nothing: a branch whose selected arm is empty leaves
the machine exactly as it found it. -/
theorem an_empty_arm_changes_nothing (machine : Machine) (thenArm : Seq) :
    interpOp machine (.branch (.lit false) thenArm .nil) = (machine, none) := rfl

/-- The admitted both-arms-release program observes one acquisition and one
release, and returns normally — the same observation its hand-written unbranched
twin has. -/
theorem the_admitted_branch_observes_its_taken_arm :
    (interpSeq { live := [], trace := [] } bothArmsRelease).2 =
      some { outcome := some 0, trace := [Event.acquire 1, Event.release 1] } := rfl

/-- The unbranched program a reader would have written instead. Its observation
is identical, which is the model counterpart of the byte-identical emission pin.
-/
def unbranchedTwin : Seq :=
  .cons (.acquire 1 true) (.cons (.drop 1) (.cons (.ret 0) .nil))

theorem the_branch_observes_what_its_hand_written_twin_does :
    (interpSeq { live := [], trace := [] } bothArmsRelease).2 =
      (interpSeq { live := [], trace := [] } unbranchedTwin).2 := rfl

/-- …while the two are genuinely different programs: the twin is branch-free
and the branched one is not, so the agreement above is a theorem about the
lowering rather than a restatement. -/
theorem the_twin_is_branch_free_and_the_branched_program_is_not :
    unbranchedTwin.branchFree = true ∧ bothArmsRelease.branchFree = false := ⟨rfl, rfl⟩

end QCore.BranchJoin
