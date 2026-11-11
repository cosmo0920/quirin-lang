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

import QCore.BranchJoin

/-! Arm exits, and the continuation hoisting they require (card `IF-ARMS`).

Card `IF-STATEMENT` landed `source.branch.v1` with an arm subset that refused a
`return`: "a return is the last operation of a program, and an early exit from
an arm has no position in this lowering". This module is the semantics of
removing that restriction, built on `QCore.BranchJoin`'s own operations,
validator and interpreter — so the theorems here are about the model that card
proved rather than about a paraphrase of it.

Two things had to be true, and both are proved here rather than asserted.

* **An arm exit carries the program exit's obligations.** `Op.ret` demands an
  empty live list wherever it stands; it is one case of one step function, and a
  branch arm reaches it through the same case the entry body does. The returning
  arm therefore discharges at its own exit, which is what lets
  `QCore.BranchJoin.joinArms` leave it out of the join — see
  `the_arm_exit_is_the_program_exit` and, in `BranchJoin` itself,
  `a_returning_arm_does_not_constrain_the_join` and
  `an_arm_return_must_discharge_its_custody`.

* **The front end may move the continuation without changing the program.** Four
  further models join a branch — resource flow, ownership, typed errors and
  usage — and none of them can express "this arm exited". So the driver does not
  hand them a one-sided join: the statements written *after* the branch are
  lowered inside the arm that survives to reach them, which is where they run.
  `the_hoist_preserves_the_observation` is that rewrite's correctness, by cases
  on the decided condition, and
  `the_hoist_moves_statements_without_duplicating_them` is the property that
  makes it a *move*: the hoisted program has exactly as many operations as the
  program it came from, so no acquisition, call or demand is doubled. That is
  also why the shape needing the continuation twice is refused rather than
  duplicated — `hoist` has no case that produces it.

What is **not** proved here: that the Rust driver performs this rewrite the way
the model says (a tested Rust boundary, per the `ConcurrencyPrelude` /
`PipeDesugaring` / `BranchJoin` / `Connectives` precedent); that the four models
the hoist exists for accept the rewritten program (they are outside this
module's vocabulary — the Rust suite measures it); and anything about an
acquisition inside an arm, which card `IF-ARMS` deliberately keeps refused
(follow-on `IF-ARM-ACQUIRE`). -/
namespace QCore.BranchArms

open QCore.BranchJoin

/-! ## 1. Termination, and appending a continuation -/

/-- Sequence concatenation.  `Seq` is its own inductive rather than `List Op`,
so it needs its own append. -/
def appendSeq : Seq → Seq → Seq
  | .nil, rest => rest
  | .cons head tail, rest => .cons head (appendSeq tail rest)

@[simp] theorem appendSeq_nil (rest : Seq) : appendSeq .nil rest = rest := rfl

@[simp] theorem appendSeq_cons (head : Op) (tail rest : Seq) :
    appendSeq (.cons head tail) rest = .cons head (appendSeq tail rest) := rfl

theorem appendSeq_nil_right : ∀ sequence : Seq, appendSeq sequence .nil = sequence
  | .nil => rfl
  | .cons head tail => by rw [appendSeq_cons, appendSeq_nil_right tail]

mutual

/-- Whether one operation ends the function on every path through it: a
`return`, or a branch both of whose arms do. -/
def opTerminates : Op → Bool
  | .ret _ => true
  | .branch _ thenArm elseArm => seqTerminates thenArm && seqTerminates elseArm
  | _ => false

/-- Whether a sequence ends the function on every path through it.  A validated
sequence puts its exit last — an operation after one is `operationAfterReturn` —
so this reads the last operation.  The Rust counterpart is
`cleanup_lowering::sequence_returns`. -/
def seqTerminates : Seq → Bool
  | .nil => false
  | .cons head .nil => opTerminates head
  | .cons _ (.cons second rest) => seqTerminates (.cons second rest)

end

/-- A bare `return` terminates, and it is the base case the whole recursion
rests on. -/
theorem a_return_terminates (value : Int) :
    seqTerminates (.cons (.ret value) .nil) = true := rfl

/-- A branch terminates exactly when **both** arms do — the mixed shape, one arm
returning and one falling through, is precisely what the continuation hoisting
below exists to remove. -/
theorem a_branch_terminates_when_both_arms_do (condition : Condition) (thenArm elseArm : Seq) :
    seqTerminates (.cons (.branch condition thenArm elseArm) .nil) =
      (seqTerminates thenArm && seqTerminates elseArm) := rfl

/-- An empty sequence does not terminate, so an `if` with no `else` has a
non-terminating arm however its `then` arm ends: the missing arm is the empty
arm, and the empty arm falls through. -/
theorem the_missing_arm_falls_through : seqTerminates .nil = false := rfl

/-! ## 2. An arm exit is the program exit -/

/-- **The obligation is the same one.** `stepOp` has exactly one `ret` case, and
a branch arm runs it through `runSeq` exactly as the entry body does: a live
resource at the exit refuses with `liveResourcesAtReturn` and an empty one
succeeds, wherever the `return` stands.  This is the whole reason a returning
arm may be left out of the join — its obligations were discharged, not
deferred. -/
theorem the_arm_exit_is_the_program_exit (state : State) (value : Int) :
    stepOp state (.ret value) =
      (if state.2.live ≠ [] then .error .liveResourcesAtReturn
       else .ok (state.1, { state.2 with returned := true })) := rfl

/-- Stated as the two halves, so neither direction is left implicit. -/
theorem an_exit_with_live_custody_refuses
    (state : State) (value : Int) (live : state.2.live ≠ []) :
    stepOp state (.ret value) = .error .liveResourcesAtReturn := by
  simp [stepOp, live]

theorem a_discharged_exit_succeeds
    (state : State) (value : Int) (discharged : state.2.live = []) :
    stepOp state (.ret value) = .ok (state.1, { state.2 with returned := true }) := by
  simp [stepOp, discharged]

/-! ## 3. The continuation hoisting -/

/-- **The front end's rewrite** (card `IF-ARMS`).

`hoist condition thenArm elseArm cont` is what the driver produces for
`if (condition) thenArm else elseArm` followed by `cont`:

* neither arm returns — the continuation follows the join, which is where it
  already was.  This is the pre-card program, unchanged;
* exactly one arm returns — the other arm is the only one that reaches `cont`,
  so `cont` is appended **to that arm**.  If the result still does not
  terminate, the branch is still mixed and there is no second continuation to
  place: `none`;
* both arms return — nothing after the branch is reachable, so a non-empty
  `cont` is refused rather than dropped.

`none` is a refusal, never a fallback: the driver reports it at `Q044`. -/
def hoist (condition : Condition) (thenArm elseArm cont : Seq) : Option Seq :=
  match seqTerminates thenArm, seqTerminates elseArm with
  | true, true =>
      match cont with
      | .nil => some (.cons (.branch condition thenArm elseArm) .nil)
      | .cons _ _ => none
  | true, false =>
      if seqTerminates (appendSeq elseArm cont) then
        some (.cons (.branch condition thenArm (appendSeq elseArm cont)) .nil)
      else none
  | false, true =>
      if seqTerminates (appendSeq thenArm cont) then
        some (.cons (.branch condition (appendSeq thenArm cont) elseArm) .nil)
      else none
  | false, false => some (.cons (.branch condition thenArm elseArm) cont)

/-- Nothing moves for a branch neither of whose arms returns: the hoist is the
identity there, which is the model counterpart of the byte-identity claim for
every program card `IF-STATEMENT` already admitted. -/
theorem the_hoist_is_the_identity_without_an_arm_exit
    (condition : Condition) (thenArm elseArm cont : Seq)
    (thenFalls : seqTerminates thenArm = false) (elseFalls : seqTerminates elseArm = false) :
    hoist condition thenArm elseArm cont =
      some (.cons (.branch condition thenArm elseArm) cont) := by
  unfold hoist
  rw [thenFalls, elseFalls]

/-- A mixed branch with **no** continuation to place is refused: there is
nothing for the surviving arm to continue into, so some path would reach no
`return` at all. -/
theorem a_mixed_branch_with_no_continuation_refuses
    (condition : Condition) (thenArm elseArm : Seq)
    (thenReturns : seqTerminates thenArm = true) (elseFalls : seqTerminates elseArm = false) :
    hoist condition thenArm elseArm .nil = none := by
  unfold hoist
  rw [thenReturns, elseFalls, appendSeq_nil_right, elseFalls]
  rfl

/-! ## 4. The hoist preserves the observation -/

/-- A one-operation sequence observes exactly what its operation observes. -/
theorem interp_single (machine : Machine) (operation : Op) :
    interpSeq machine (.cons operation .nil) = interpOp machine operation := by
  show (match interpOp machine operation with
        | (next, none) => interpSeq next .nil
        | (next, some observation) => (next, some observation)) = interpOp machine operation
  cases result : interpOp machine operation with
  | mk next observation => cases observation <;> rfl

/-- The defining step of the sequence interpreter, named so the proofs below can
rewrite with it rather than re-deriving it. -/
theorem interp_cons (machine : Machine) (head : Op) (tail : Seq) :
    interpSeq machine (.cons head tail) =
      (match interpOp machine head with
       | (next, none) => interpSeq next tail
       | (next, some observation) => (next, some observation)) := rfl

/-- Interpreting a concatenation is interpreting the front and, if it fell
through, continuing into the back. -/
theorem interp_append : ∀ (machine : Machine) (front back : Seq),
    interpSeq machine (appendSeq front back) =
      (match interpSeq machine front with
       | (next, none) => interpSeq next back
       | (next, some observation) => (next, some observation))
  | machine, .nil, back => rfl
  | machine, .cons head tail, back => by
      rw [appendSeq_cons, interp_cons, interp_cons]
      cases result : interpOp machine head with
      | mk next observation =>
          cases observation with
          | none => simpa using interp_append next tail back
          | some value => rfl

mutual

/-- A terminating operation always produces an observation: it either exits with
one of its own or reaches an arm that does. -/
theorem a_terminating_operation_observes (machine : Machine) (operation : Op)
    (terminating : opTerminates operation = true) :
    (interpOp machine operation).2.isSome = true := by
  cases operation with
  | acquire _ _ => simp [opTerminates] at terminating
  | check _ => simp [opTerminates] at terminating
  | drop _ => simp [opTerminates] at terminating
  | transfer _ => simp [opTerminates] at terminating
  | ret _ => rfl
  | branch condition thenArm elseArm =>
      simp only [opTerminates, Bool.and_eq_true] at terminating
      by_cases decided : condition.evaluate = true
      · have selected : interpOp machine (.branch condition thenArm elseArm) =
            interpSeq machine thenArm := by simp [interpOp, decided]
        rw [selected]
        exact a_terminating_sequence_observes machine thenArm terminating.1
      · simp only [Bool.not_eq_true] at decided
        have selected : interpOp machine (.branch condition thenArm elseArm) =
            interpSeq machine elseArm := by simp [interpOp, decided]
        rw [selected]
        exact a_terminating_sequence_observes machine elseArm terminating.2

/-- …and so does a terminating sequence. This is what makes the mixed-arm
rewrite sound: the returning arm never falls out of its branch, so appending the
continuation to the *other* arm loses nothing. -/
theorem a_terminating_sequence_observes (machine : Machine) (sequence : Seq)
    (terminating : seqTerminates sequence = true) :
    (interpSeq machine sequence).2.isSome = true := by
  cases sequence with
  | nil => simp [seqTerminates] at terminating
  | cons head tail =>
      cases tail with
      | nil =>
          rw [interp_single]
          exact a_terminating_operation_observes machine head terminating
      | cons second rest =>
          rw [interp_cons]
          cases result : interpOp machine head with
          | mk next observation =>
              cases observation with
              | none =>
                  simpa using
                    a_terminating_sequence_observes next (.cons second rest) terminating
              | some value => rfl

end

/-- **The rewrite is correct.** Whatever the decided condition selects, the
hoisted program observes exactly what the branch followed by its continuation
observes. Moving the continuation into the surviving arm is a change of *shape*
made for the four models that cannot express an exiting arm, not a change of
program. -/
theorem the_hoist_preserves_the_observation
    (machine : Machine) (condition : Condition) (thenArm elseArm cont hoisted : Seq)
    (rewritten : hoist condition thenArm elseArm cont = some hoisted) :
    interpSeq machine hoisted =
      interpSeq machine (.cons (.branch condition thenArm elseArm) cont) := by
  unfold hoist at rewritten
  rw [interp_cons]
  cases thenTerminates : seqTerminates thenArm <;> cases elseTerminates : seqTerminates elseArm <;>
    rw [thenTerminates, elseTerminates] at rewritten <;> dsimp only at rewritten
  -- Neither arm returns: the hoist is the identity.
  case false.false =>
    injection rewritten with equality
    rw [← equality, interp_cons]
  -- The else arm returns; the then arm carries the continuation.
  case false.true =>
    split at rewritten
    · injection rewritten with equality
      rw [← equality, interp_single]
      by_cases decided : condition.evaluate = true
      · have selected : interpOp machine (.branch condition (appendSeq thenArm cont) elseArm) =
            interpSeq machine (appendSeq thenArm cont) := by simp [interpOp, decided]
        have original : interpOp machine (.branch condition thenArm elseArm) =
            interpSeq machine thenArm := by simp [interpOp, decided]
        rw [selected, original, interp_append]
      · simp only [Bool.not_eq_true] at decided
        have selected : interpOp machine (.branch condition (appendSeq thenArm cont) elseArm) =
            interpSeq machine elseArm := by simp [interpOp, decided]
        have original : interpOp machine (.branch condition thenArm elseArm) =
            interpSeq machine elseArm := by simp [interpOp, decided]
        have observed := a_terminating_sequence_observes machine elseArm elseTerminates
        rw [selected, original]
        cases arm : interpSeq machine elseArm with
        | mk next observation =>
            cases observation with
            | none => rw [arm] at observed; simp at observed
            | some value => rfl
    · exact absurd rewritten (by simp)
  -- The then arm returns; the else arm carries the continuation.
  case true.false =>
    split at rewritten
    · injection rewritten with equality
      rw [← equality, interp_single]
      by_cases decided : condition.evaluate = true
      · have selected : interpOp machine (.branch condition thenArm (appendSeq elseArm cont)) =
            interpSeq machine thenArm := by simp [interpOp, decided]
        have original : interpOp machine (.branch condition thenArm elseArm) =
            interpSeq machine thenArm := by simp [interpOp, decided]
        have observed := a_terminating_sequence_observes machine thenArm thenTerminates
        rw [selected, original]
        cases arm : interpSeq machine thenArm with
        | mk next observation =>
            cases observation with
            | none => rw [arm] at observed; simp at observed
            | some value => rfl
      · simp only [Bool.not_eq_true] at decided
        have selected : interpOp machine (.branch condition thenArm (appendSeq elseArm cont)) =
            interpSeq machine (appendSeq elseArm cont) := by simp [interpOp, decided]
        have original : interpOp machine (.branch condition thenArm elseArm) =
            interpSeq machine elseArm := by simp [interpOp, decided]
        rw [selected, original, interp_append]
    · exact absurd rewritten (by simp)
  -- Both arms return: the continuation is empty, so the two programs are one.
  case true.true =>
    cases cont with
    | nil =>
        injection rewritten with equality
        rw [← equality, interp_cons]
    | cons head tail => exact absurd rewritten (by simp)

/-! ## 5. The hoist moves statements; it never duplicates them -/

mutual

def opSize : Op → Nat
  | .branch _ thenArm elseArm => 1 + seqSize thenArm + seqSize elseArm
  | _ => 1

def seqSize : Seq → Nat
  | .nil => 0
  | .cons head tail => opSize head + seqSize tail

end

theorem appendSeq_size : ∀ front back : Seq,
    seqSize (appendSeq front back) = seqSize front + seqSize back
  | .nil, back => by simp [seqSize]
  | .cons head tail, back => by
      rw [appendSeq_cons, seqSize, seqSize, appendSeq_size tail back, Nat.add_assoc]

/-- **The hoist is a move.** The rewritten program has exactly as many
operations as the program it came from, so the continuation appears once
wherever it appears — no acquisition is spent twice, no call takes two positions
in the flat call sequence, and no usage demand is doubled. This is why the shape
that would need the continuation in *two* arms is refused (`hoist` has no case
that produces it) rather than served by duplication. -/
theorem the_hoist_moves_statements_without_duplicating_them
    (condition : Condition) (thenArm elseArm cont hoisted : Seq)
    (rewritten : hoist condition thenArm elseArm cont = some hoisted) :
    seqSize hoisted = seqSize (.cons (.branch condition thenArm elseArm) cont) := by
  unfold hoist at rewritten
  cases thenTerminates : seqTerminates thenArm <;> cases elseTerminates : seqTerminates elseArm <;>
    rw [thenTerminates, elseTerminates] at rewritten <;> dsimp only at rewritten
  case false.false =>
    injection rewritten with equality
    rw [← equality]
  case false.true =>
    split at rewritten
    · injection rewritten with equality
      rw [← equality]
      simp only [seqSize, opSize, appendSeq_size]
      omega
    · exact absurd rewritten (by simp)
  case true.false =>
    split at rewritten
    · injection rewritten with equality
      rw [← equality]
      simp only [seqSize, opSize, appendSeq_size]
      omega
    · exact absurd rewritten (by simp)
  case true.true =>
    cases cont with
    | nil =>
        injection rewritten with equality
        rw [← equality]
    | cons head tail => exact absurd rewritten (by simp)

/-! ## 6. Concrete programs -/

/-- `let a = acquire; if (c) { drop a; return 1; } drop a; return 0;` — the
canonical early return, as the driver hoists it: both arms release and exit. -/
def earlyReturnHoisted : Seq :=
  .cons (.acquire 1 true)
    (.cons
      (.branch (.lit true)
        (.cons (.drop 1) (.cons (.ret 1) .nil))
        (.cons (.drop 1) (.cons (.ret 0) .nil)))
      .nil)

/-- The same source before the hoist: the branch's else arm is empty and the
release and exit follow the join.  Card `IF-STATEMENT` could not have written
this program either, because the `then` arm returns. -/
def earlyReturnUnhoisted : Seq :=
  .cons (.acquire 1 true)
    (.cons (.branch (.lit true) (.cons (.drop 1) (.cons (.ret 1) .nil)) .nil)
      (.cons (.drop 1) (.cons (.ret 0) .nil)))

theorem the_early_return_is_admitted : Admits earlyReturnHoisted := by decide

/-- Its unhoisted spelling is admitted too — by `QCore.BranchJoin`'s general
join-over-survivors rule, which is what the core model implements.  The driver
hoists anyway, for the four models that are not in this vocabulary. -/
theorem the_unhoisted_spelling_is_admitted_by_the_survivor_join :
    Admits earlyReturnUnhoisted := by decide

/-- …and the two observe the same thing, which is the concrete instance of
`the_hoist_preserves_the_observation`. -/
theorem the_hoisted_program_observes_what_its_source_does :
    (interpSeq { live := [], trace := [] } earlyReturnHoisted).2 =
      (interpSeq { live := [], trace := [] } earlyReturnUnhoisted).2 := rfl

/-- One acquisition and one release on the taken path, exiting with the arm's
own value: the early return really returns early. -/
theorem the_early_return_observes_its_own_exit :
    (interpSeq { live := [], trace := [] } earlyReturnHoisted).2 =
      some { outcome := some 1, trace := [Event.acquire 1, Event.release 1] } := rfl

/-- Both arms returning: the branch ends the function, and a statement after it
is unreachable rather than joined. -/
def bothArmsReturnThenMore : Seq :=
  .cons (.branch (.lit true) (.cons (.ret 1) .nil) (.cons (.ret 0) .nil))
    (.cons (.ret 2) .nil)

theorem an_operation_after_a_returning_branch_refuses :
    validate bothArmsReturnThenMore = .error .operationAfterReturn := rfl

/-- A mixed branch whose surviving arm leaks: the join is over the survivor, so
the survivor's live resource reaches the program's own exit and refuses there.
The widening moved which exit reports a leak, never whether it is reported. -/
def mixedBranchSurvivorLeaks : Seq :=
  .cons (.acquire 1 true)
    (.cons (.branch (.lit true) (.cons (.drop 1) (.cons (.ret 1) .nil)) .nil)
      (.cons (.ret 0) .nil))

theorem a_leaking_survivor_refuses :
    validate mixedBranchSurvivorLeaks = .error .liveResourcesAtReturn := rfl

/-- Collected as one list, so widening the arm subset further must change a
theorem rather than slip past one. -/
theorem refused_arm_shapes_are_exactly_these :
    [validate earlyReturnLeaks, validate mixedBranchSurvivorLeaks,
      validate bothArmsReturnThenMore] =
      [.error .liveResourcesAtReturn, .error .liveResourcesAtReturn,
        .error .operationAfterReturn] := rfl

/-- The positive control beside them: the shapes the card admits really are
admitted, so the list above is a statement about a live gate rather than about a
model that refuses everything. -/
theorem admitted_arm_shapes_are_exactly_these :
    Admits earlyReturnHoisted ∧ Admits earlyReturnUnhoisted ∧ Admits armReturnWithSurvivor := by
  refine ⟨?_, ?_, ?_⟩ <;> decide

end QCore.BranchArms
