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

import QCore.Transferability

/-! # Card `MUTEX-SHARE-CROSS-ARMS` (wave 35): two dead arms, justified

Card `STABILITY-REEVAL-1` (wave 34) measured four refusal arms in
`lower_declared_share` (`crates/qcore-frontend/src/source_mutex.rs`) that no
test drives, under a wave-32 section asserting that no such arm exists. Two of
the four are reachable from source and are driven by
`crates/qcore-frontend/tests/mutex_share_cross_arms.rs`. The other two are
**unreachable from any program**, and this module is why — residual
`MUTEX-SHARE-CROSS-UNEXERCISED-ARMS`.

The discipline is `RETURN-POSITION-ESCAPE-UNREACHABLE`'s: a dead fail-closed
backstop is defensible, but only on an *argument*, never on a green suite that
cannot tell the arm from its negation. So each arm gets two theorems, as
`QCore.ReturnPositionEscape` does:

* one saying the arm is unreachable **today**
  (`theProtocolResidueIsEmpty`, `theLookupAlwaysHits`), and
* one saying what the arm would catch if its premise dropped
  (`thePassThroughJudgmentWouldReachTheProtocolArm`,
  `theJudgmentCannotTellTwoKindsOfOneClassApart`) — which is why the arms are
  **kept** rather than simplified away.

The premise that could drop is not hypothetical bookkeeping: plan item
`FB-SYSTEM-EXTENSIONS` (lock order) would need nesting and a second mutex-bearing
kind, and the wave-34 finding records that it "makes both dead arms reachable,
with no test behind either".

What is **not** claimed here: nothing about the mutex protocol's soundness
(that is `QCore.MutexShareCross`), and nothing about the lowering's *local*
control flow, which is Rust and is argued in the card's report. This module
decides exactly the non-local half — what the judgment can hand the lowering. -/

namespace QCore.MutexShareCrossArms

open QCore.Transferability

/-! ### Arm C — `Ok(other)`, the protocol residue

The lowering matches `admits_capture(place, ShareUnderMutex, MutexShare)`, takes
`Ok(AdmittedTransfer::ShareUnderMutex(ShareProtocol::CheckedMutex))` as its
admission, and refuses any *other* `Ok` by name. The residue is empty: for this
mode the judgment's admitting arm constructs the protocol as a **literal**, so
an admitted capture is that very value for every kind there is. -/

/-- The result of a mutex capture is one of exactly two things, for every kind
in the vocabulary: the refusal the lowering's `Err` arm reads, or the exact
value its preceding `Ok` arm matches. Nothing is left for `Ok(other)`. -/
theorem theProtocolResidueIsEmpty :
    ∀ k ∈ allKinds,
      admitsCapture k .shareUnderMutex .mutexShare = none ∨
        admitsCapture k .shareUnderMutex .mutexShare
          = some (.shareUnderMutex .checkedMutex) := by
  decide

/-- The same fact in the shape the Rust arm is written in: any admitted mutex
capture *is* the preceding arm's pattern, so the `Ok(other)` binder can never
be bound. Stated for an arbitrary kind, with no membership side condition —
`kindInventoryIsComplete` supplies it. -/
theorem theOkOtherArmIsUnreachable (k : Kind) (t : Transfer)
    (h : admitsCapture k .shareUnderMutex .mutexShare = some t) :
    t = .shareUnderMutex .checkedMutex := by
  rcases theProtocolResidueIsEmpty k (kindInventoryIsComplete k) with h0 | h1
  · rw [h0] at h; simp at h
  · rw [h1] at h; exact (Option.some.inj h).symm

/-- The judgment as it stood **before** card `MUTEX-SHARE-CROSS`, quoted from
that card's own comment at `crates/qcore-core/src/transfer.rs`: *"Before this
card the arm took whatever protocol the kind named."* The protocol is a bound
variable here, not a literal — which is the whole difference. -/
def admitsPassThrough : Class → Option Transfer
  | .movedOnly (some prot) _ => some (.shareUnderMutex prot)
  | _ => none

/-- **Why arm C is kept.** Under the pass-through judgment the arm is live, and
it is live on the kind the card's sibling residual names: an `own<Buffer>`
would be admitted as a share under `countedReadShares`, which is not a
mutex-mediated crossing at all.

The third conjunct is the load-bearing one. `admitsCapture`'s received-kind
gate would **not** intercept this value — `receivedKind` reads the transfer's
constructor and not its protocol, so a pass-through admission arrives at the
lowering's match with the wrong protocol and the right delivered kind. Arm C is
the only thing between it and an unprotected crossing. -/
theorem thePassThroughJudgmentWouldReachTheProtocolArm :
    admitsPassThrough (transferable .ownedBuffer)
        = some (.shareUnderMutex .countedReadShares) ∧
      (Transfer.shareUnderMutex .countedReadShares)
        ≠ .shareUnderMutex .checkedMutex ∧
      receivedKind .ownedBuffer (.shareUnderMutex .countedReadShares)
        = .mutexShare := by
  refine ⟨by decide, by decide, by decide⟩

/-- And the two judgments really do differ, so the preceding theorem is about a
weakening this project could suffer rather than about a distinction without a
difference. -/
theorem theCardsLiteralIsWhatEmptiesTheResidue :
    ∃ k ∈ allKinds, admitsPassThrough (transferable k) ≠ admits k .shareUnderMutex := by
  decide

/-! ### Arm D — the payload lookup that cannot miss

Having admitted, the lowering looks the payload name up in the very map whose
`contains_key` decided the kind it just fed the judgment. The Rust half of that
argument — that `place` is `ProtectedResource` only inside the branch guarded by
`state.resource_of.contains_key(resource_name)` — is three lines of local
control flow and is argued in the card's report. The half that is *not* local is
here: which kinds the judgment admits at all. -/

/-- **Arm D, the non-local half.** No kind but the protected payload reaches the
lookup, so the name the lowering is about to look up is one `contains_key` has
already answered for. -/
theorem theLookupAlwaysHits :
    ∀ k ∈ allKinds,
      (admitsCapture k .shareUnderMutex .mutexShare).isSome = true →
        k = .protectedResource := by
  decide

/-- **Non-vacuity**, for both arms at once. `theLookupAlwaysHits` and
`theOkOtherArmIsUnreachable` are implications, and an implication whose
antecedent nothing satisfies says nothing at all: a judgment that refused every
kind would prove both and would also mean the admitting path is dead, which is
the opposite of what is being claimed. It is not — the capture admits, and the
value it admits with is the one the lowering's preceding arm matches. -/
theorem theAdmittingCaseExists :
    admitsCapture .protectedResource .shareUnderMutex .mutexShare
      = some (.shareUnderMutex .checkedMutex) := by
  decide

/-- The judgment restated over classes rather than kinds, so that "a kind the
vocabulary does not have yet" is expressible. Mirrors `admits` arm for arm. -/
def admitsOfClass (c : Class) (m : Mode) : Option Transfer :=
  match m with
  | .write => none
  | _ =>
    match c with
    | .never _ => none
    | .duplicated => match m with
      | .copy => some .duplicate
      | _ => none
    | .movedOnly p q => match m with
      | .move => some .moveCustody
      | .lend => match p with
        | some .countedReadShares => some (.lendShare .countedReadShares)
        | _ => none
      | .shareUnderMutex => match p with
        | some .checkedMutex => some (.shareUnderMutex .checkedMutex)
        | _ => none
      | .lendWrite => match q with
        | some .disjointWriteLeases => some (.lendWriteRange .disjointWriteLeases)
        | _ => none
      | _ => none

/-- The class form is the judgment, decided whole over both vocabularies. A
weakening of `admits` that this restatement did not follow would show up here
rather than in a proof that quietly ranged over the wrong function. -/
theorem theClassFormIsTheJudgment :
    ∀ k ∈ allKinds, ∀ m ∈ allModes, admitsOfClass (transferable k) m = admits k m := by
  decide

/-- **Why arm D is kept.** The admitting decision reads the kind's *class* and
nothing else, so it cannot tell two kinds of one class apart. Arm D is therefore
dead by **uniqueness alone** — `theMutexProtocolIsReachableForExactlyOneKind`'s
first conjunct, that `protectedResource` is the only kind sharing under the
checked mutex — and by nothing structural.

A second mutex-bearing kind, which is what `FB-SYSTEM-EXTENSIONS`' lock-order
work would introduce, admits exactly as the payload does while being a kind the
lowering's `resource_of` map knows nothing about. The lookup then misses, and
arm D is the refusal that catches it. -/
theorem theJudgmentCannotTellTwoKindsOfOneClassApart :
    transferable .protectedResource = .movedOnly (some .checkedMutex) (some .checkedMutex) ∧
      admitsOfClass (transferable .protectedResource) .shareUnderMutex
        = some (.shareUnderMutex .checkedMutex) ∧
      (∀ k ∈ allKinds, transferable k = transferable .protectedResource →
        admitsOfClass (transferable k) .shareUnderMutex
          = admitsOfClass (transferable .protectedResource) .shareUnderMutex) := by
  refine ⟨by decide, by decide, ?_⟩
  intro k _ h
  rw [h]

/-- The uniqueness arm D leans on, restated here so the dependency is visible in
this module rather than only in the report: exactly one kind carries the class,
today. -/
theorem armDLeansOnUniqueness :
    allKinds.filter (fun k => transferable k = transferable .protectedResource)
      = [.protectedResource] := by
  decide

end QCore.MutexShareCrossArms
