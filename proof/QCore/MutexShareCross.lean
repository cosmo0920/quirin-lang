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

import QCore.MutexInvariant

/-! # Card `MUTEX-SHARE-CROSS` (wave 32): CT46's soundness statement

CT46 reads *"Known shared wrapper with non-Sync but Send payload"*, expected
**"Accept only through verified mutex protocol"**. The Rust side of the card
supplies the composition: a `protected N` payload crosses into a child task
under a declared generation, admitted by the transferability judgment's fifth
row and gated on that generation existing and protecting this very payload.

This module supplies the **soundness statement for the admission**, and it is
one sentence: *a payload crossing under this route is, at every point of every
interleaving, reachable only through the protocol.*

## What is proved, and what carries it

Everything here is proved against `QCore.MutexInvariant`'s **existing**
executable model — the same `step`/`run` semantics that has modelled this
route since card T03, unedited by this card. That is the point rather than an
economy: the card's discipline is to *compose* the checked mutex model rather
than restate its rules beside the judgment, and a soundness proof that had to
add a rule to the model would be evidence that the composition had not
happened.

Two features of that model do all the work, and neither is new:

* `canAccessUnguarded` requires `(s.protectorOf f).isNone` — **there is no
  unguarded access to a protected footprint at all**, and no fallback arm.
* `canAccessGuarded` requires the named guard to be held by the accessing task
  *and* its own generation to be that footprint's protector — so a guard over
  some other mutex authorizes nothing here.

`step` is the one-step transition of the whole model, and its `access` case is
total over `Option GuardId`, so "every point of every interleaving" is not a
figure of speech: a schedule is a list of events, `run` folds `step` along it,
and a statement quantified over *all* states `s` with `step s _ = some _`
holds at every position of every admitted schedule whatever the interleaving
put before it. The trace-level theorem below says exactly that, with
`run_append` supplying the decomposition.

## What is **not** proved here

No data-race freedom claim about C, no source-to-native refinement, no
statement about layout, and no claim that the source lowering produces the
trace it intends (that is the Rust side's, tested). Nor is *liveness* touched:
this says a reach is impossible without the protocol, not that any reach ever
happens. Deadlock-freedom in particular is **not** established — the lock-rank
discipline's own note already says a passing rank check is not a
deadlock-freedom proof, and nothing here strengthens that. -/

namespace QCore.MutexShareCross

open QCore.Concurrency.MutexInvariant

/-! ### 1. The soundness statement, one step -/

/-- **The card's theorem.** Any admitted access to a footprint a live
generation protects — by *any* task, at *any* state — names a guard, and that
guard is one the accessing task holds over the very generation that protects
the footprint.

This is the whole of "reachable only through the protocol", and the three
conjuncts are the three ways the property could fail: reaching without naming
a guard, naming a guard someone else holds, and naming a guard over a
different mutex. The model refuses all three, and it refuses them for every
task without distinguishing a creator from a child — which is why the crossing
needs no rule of its own. -/
theorem aSharedPayloadIsReachableOnlyThroughTheProtocol
    {s next : State} {task : TaskId} {f : Footprint} {kind : AccessKind}
    {g : Option GuardId} {m : MutexId}
    (protectedBy : s.protectorOf f = some m)
    (admitted : step s (.access task f kind g) = some next) :
    ∃ guard record, g = some guard ∧ s.guardOf guard = some record ∧
      record.holder = task ∧ s.protectorOf f = some record.mutex := by
  cases g with
  | none =>
      exfalso
      have refused : canAccessUnguarded s task f kind = false := by
        simp [canAccessUnguarded, protectedBy]
      simp [step, refused] at admitted
  | some guard =>
      have reachable : canAccessGuarded s task f kind guard = true := by
        cases h : canAccessGuarded s task f kind guard
        · simp [step, h] at admitted
        · rfl
      cases record : s.guardOf guard with
      | none => simp [canAccessGuarded, State.guardSat, record] at reachable
      | some gr =>
          unfold canAccessGuarded at reachable
          rw [State.guardSat, record] at reachable
          simp only [Option.elim, Bool.and_eq_true, beq_iff_eq,
            decide_eq_true_eq] at reachable
          -- `canAccessGuarded`'s second conjunct is exactly the two facts
          -- this theorem claims: the guard's holder is the reaching task, and
          -- the guard's own generation is this footprint's protector.
          exact ⟨guard, gr, rfl, record, reachable.2.1.1.1, reachable.2.1.1.2⟩

/-! ### 2. The same statement at every point of every interleaving -/

/-- The trace-level lift. For **any** schedule that reaches an access to the
shared payload — whatever else any task did before it — the access is governed
by the rule above at the state it actually runs in.

`pre` is an arbitrary prefix: it may interleave the creator's statements and
the child's in any admitted order, may open and close any number of critical
sections, and may spawn and join. So this is the card's sentence verbatim —
*at every point of every interleaving* — rather than a statement about one
schedule. -/
theorem everyReachAlongAnAdmittedTraceHoldsTheProtocol
    {s mid final : State} {pre post : List Event}
    {task : TaskId} {f : Footprint} {kind : AccessKind} {g : Option GuardId}
    {m : MutexId}
    (prefixRuns : run s pre = some mid)
    (protectedBy : mid.protectorOf f = some m)
    (whole : run s (pre ++ .access task f kind g :: post) = some final) :
    ∃ guard record, g = some guard ∧ mid.guardOf guard = some record ∧
      record.holder = task ∧ mid.protectorOf f = some record.mutex := by
  rw [run_append, prefixRuns] at whole
  simp only [Option.bind_some, run] at whole
  cases stepped : step mid (.access task f kind g) with
  | none => rw [stepped] at whole; exact absurd whole (by simp)
  | some next =>
      exact aSharedPayloadIsReachableOnlyThroughTheProtocol protectedBy stepped

/-! ### 3. Why the protocol is worth crossing under: mutual exclusion -/

/-- The generation admits one holder at a time. `canLock` requires the
generation to be unheld, so a second acquisition of a live generation is
refused at every state — which is what makes "the protocol orders mutation"
true rather than aspirational.

Together with §1 this is CT46's expected answer in full: the payload is
reachable only through the generation (§1), and the generation admits one
reacher at a time (here). -/
theorem aSecondAcquisitionOfALiveGenerationIsRefused
    {s : State} {task : TaskId} {mutex : MutexId} {guard held : GuardId}
    (busy : s.mutexSat mutex (fun r => r.holder == some held) = true) :
    step s (.lock task mutex guard) = none := by
  simp only [step]
  have unheld : canLock s task mutex guard = false := by
    simp only [canLock, State.mutexSat] at busy ⊢
    cases record : s.mutexOf mutex with
    | none => simp
    | some r =>
        simp only [record, Option.elim] at busy ⊢
        have : r.holder = some held := of_decide_eq_true (by simpa using busy)
        simp [this]
  simp [unheld]

/-- And the release is the holder's alone, so a reacher cannot be displaced by
another task shortening its window. -/
theorem onlyTheHolderReleasesTheGeneration
    {s : State} {task : TaskId} {guard : GuardId} {record : GuardRecord}
    (live : s.guardOf guard = some record) (other : record.holder ≠ task) :
    step s (.unlock task guard) = none := by
  simp only [step]
  have refused : canRelease s task guard = false := by
    simp only [canRelease, State.guardSat, live, Option.elim]
    have : (record.holder == task) = false := by
      cases h : (record.holder == task)
      · rfl
      · exact absurd (of_decide_eq_true (by simpa using h)) other
    simp [this]
  simp [refused]

/-! ### 4. The crossing itself

§1 quantifies over the accessing task, so a child reaches the payload under
exactly the rule its creator does. The two corollaries below say that in the
shapes CT46 names, so the reading is recorded rather than left to be
re-derived. -/

/-- A child task spawned by the creator is held to the identical rule: nothing
about the crossing weakens the protocol for the party that crossed.

The spawn hypothesis is named and **deliberately unused**, and that is the
content rather than an oversight: the conclusion does not depend on the
crossing having happened, because §1 never asks which task is reaching. A
proof that *needed* the spawn would mean the crossing had bought the child
some authority, which is precisely what this route does not do — a mutex share
parks nothing with the child and grants it only the right to acquire. -/
theorem aSpawnedChildReachesThePayloadOnlyThroughTheProtocol
    {s spawned next : State} {parent child : TaskId} {f : Footprint}
    {kind : AccessKind} {g : Option GuardId} {m : MutexId}
    (_crossing : step s (.spawn parent child) = some spawned)
    (protectedBy : spawned.protectorOf f = some m)
    (reach : step spawned (.access child f kind g) = some next) :
    ∃ guard record, g = some guard ∧ spawned.guardOf guard = some record ∧
      record.holder = child ∧ spawned.protectorOf f = some record.mutex :=
  aSharedPayloadIsReachableOnlyThroughTheProtocol protectedBy reach

/-- The rule does not read the task at all beyond "the guard's holder is the
reacher", so creator and child are governed by one rule rather than two. This
is the formal content of the card's claim that the crossing composes the
unedited checker instead of adding a rule beside it: swapping which task
reaches changes only which task must hold the guard. -/
theorem theCreatorAndTheChildAreHeldToOneRule
    (s : State) (creator child : TaskId) (f : Footprint) (kind : AccessKind)
    (guard : GuardId) :
    (canAccessGuarded s creator f kind guard = true ↔
      (s.act creator && !s.destroyedResources.contains f.resource && f.valid &&
        s.guardSat guard (fun g => g.holder == creator &&
          decide (s.protectorOf f = some g.mutex) &&
          s.mutexSat g.mutex (fun m => m.protects.any (fun p => p.contains f)) &&
          (kind == .read || g.phase == .opened))) = true) ∧
    (canAccessGuarded s child f kind guard = true ↔
      (s.act child && !s.destroyedResources.contains f.resource && f.valid &&
        s.guardSat guard (fun g => g.holder == child &&
          decide (s.protectorOf f = some g.mutex) &&
          s.mutexSat g.mutex (fun m => m.protects.any (fun p => p.contains f)) &&
          (kind == .read || g.phase == .opened))) = true) :=
  ⟨Iff.rfl, Iff.rfl⟩

/-! ### 5. The obligation the judgment's fifth row carries

`QCore.Transferability`'s `Backed` records which proved rule answers for each
admitting arm. This is the rule that answers for `shareUnderMutex
checkedMutex`, in the same record-which-rule-answers discipline as the move,
read-lend and write-lease arms. -/

/-- The obligation a `shareUnderMutex checkedMutex` admission carries: a
payload a live generation protects is reachable only by a task holding a guard
of that same generation — at any state, hence at any point of any
interleaving. -/
def MutexShareObligation : Prop :=
  ∀ {s next : State} {task : TaskId} {f : Footprint} {kind : AccessKind}
    {g : Option GuardId} {m : MutexId},
    s.protectorOf f = some m →
    step s (.access task f kind g) = some next →
    ∃ guard record, g = some guard ∧ s.guardOf guard = some record ∧
      record.holder = task ∧ s.protectorOf f = some record.mutex

/-- Discharged entirely by §1 — the unedited model's own access rules. -/
theorem mutexShareObligationHolds : MutexShareObligation :=
  fun protectedBy admitted =>
    aSharedPayloadIsReachableOnlyThroughTheProtocol protectedBy admitted

end QCore.MutexShareCross
