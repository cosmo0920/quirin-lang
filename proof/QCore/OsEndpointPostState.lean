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

/-!
# Target-qualified OS endpoint failure post-states

The model of card O03's relation for **one** endpoint operation on **one**
target: `int close(int fd)` on Linux. The specification and its primary sources
are `docs/design/OS_FAILURE_POSTSTATE_PROFILE.md`; the deciding implementation
is `crates/qcore-core/src/os_endpoint.rs`.

Three identities are separated, because one liveness bit cannot express "the
descriptor is gone **and** nothing was discharged": the descriptor slot, the
open file description (released only with its last reference), and the deferred
write-back duty (which `close` never discharges).

Five descriptor post-states are distinguished. `attempted` is an *analysis* gap
— a better analysis resolves it — and `indeterminate` is a *documentation* gap,
which only pinning the target can resolve. They carry identical conservative
duties, and neither permits any recovery.

**`QCore.DescriptorCustody` now depends on this module** (card O03-U4, closing
O03's follow-on): its `TargetedClose` record is admitted only when
`closePostState` derives exactly what the record claims, and its legacy
four-field `Close` record has had its "returns on failure" arm withdrawn rather
than left contradicting the relation. Nothing in this module changed for that
repair; the dependency runs one way only.

This is a model of the *decisions*, not of kernel semantics. Which arm a real
call took, and whether the documentation is faithful to the kernel, are premises
supplied from outside; nothing here proves either. No verdict, no durability
claim, and no universal close or unmap policy follows.
-/

namespace QCore.OsEndpointPostState

/-! ## Vocabulary -/

/-- The platform whose documented semantics the relation is read against. -/
inductive Target where
  /-- Any Linux ABI target; the behaviour is the kernel's. -/
  | linux
  /-- A strictly POSIX.1-2024 conforming implementation. Linux is documented as
  non-conforming here, which is why both are kept. -/
  | posix2024
  /-- No target pinned. -/
  | unpinned
  deriving DecidableEq, Repr

/-- Post-state of the descriptor slot. -/
inductive DescriptorState where
  /-- No call was issued on this path. -/
  | notAttempted
  /-- A call was issued and the analysis cannot map the outcome to a terminal
  state. Resolvable by a better analysis. -/
  | attempted
  /-- The epoch is known live after the call. -/
  | retained
  /-- The epoch ended; the number may be rebound by a fresh generation. -/
  | released
  /-- The documentation does not determine the fate. Not resolvable by
  analysis. -/
  | indeterminate
  deriving DecidableEq, Repr

/-- The two states carrying the conservative residual duty set. -/
def DescriptorState.isPoisoned : DescriptorState → Bool
  | .attempted => true
  | .indeterminate => true
  | _ => false

/-- Post-state of the open file description. -/
inductive DescriptionState where
  | retained
  | released
  | unknown
  deriving DecidableEq, Repr

/-- Post-state of the deferred write-back duty. `discharged` exists so that the
relation can be shown never to derive it. -/
inductive WriteBack where
  | failed
  | unknown
  | discharged
  deriving DecidableEq, Repr

/-- The `errno` observed on a `-1` return. -/
inductive Errno where
  /-- The one documented error on which implementations do not close. -/
  | ebadf
  | eintr
  /-- The deferred write-back family: caused by an earlier write, reported
  here. -/
  | eio
  | enospc
  | edquot
  | other
  /-- The failure branch never read `errno`. -/
  | unobserved
  deriving DecidableEq, Repr

/-- Whether this errno is in the deferred write-back family. -/
def Errno.isDeferredIO : Errno → Bool
  | .eio => true
  | .enospc => true
  | .edquot => true
  | _ => false

/-- One observed outcome of one call. Total over what a consumer can observe. -/
inductive Outcome where
  | notAttempted
  | resultUnobserved
  /-- The success test is `ret == 0`, spelled: not `ret != -1`, and not a
  nullability test. -/
  | zero
  | minusOne (e : Errno)
  deriving DecidableEq, Repr

/-- The call-site pre-state. Every field is a premise the relation consumes;
none is established by it. -/
structure Premises where
  /-- A2 — the analysis holds a live epoch and `fd` names it. -/
  epochLive : Bool
  /-- A3 — no concurrent close or reuse of the number. -/
  soleCustodian : Bool
  /-- A4 — the result is bound and comparable against zero. -/
  resultObserved : Bool
  /-- A5 — `errno` is read on the failure branch and not clobbered. -/
  errnoObserved : Bool
  /-- A6 — this epoch is the last reference to its description. -/
  lastReference : Bool
  /-- A7 — the process holds no POSIX record locks on the file. -/
  noRecordLocks : Bool
  deriving DecidableEq, Repr

/-- The residual obligations a post-state carries. -/
structure Duties where
  releaseEpoch : Bool
  noUse : Bool
  noRetryClose : Bool
  noTreatNumberAsFree : Bool
  keepInDenominator : Bool
  reportUnresolved : Bool
  reportPossibleDataLoss : Bool
  deriving DecidableEq, Repr

def Duties.none : Duties :=
  ⟨false, false, false, false, false, false, false⟩

/-- The conservative residual duty set carried by both poisoned states. It
forbids both directions: the number may neither be used nor retried nor
recycled, and the endpoint stays in the live denominator. -/
def Duties.poisoned : Duties :=
  ⟨false, true, true, true, true, true, false⟩

/-- The not-attempted and retained duty set: the epoch is live and owed. -/
def Duties.live : Duties :=
  { Duties.none with releaseEpoch := true, keepInDenominator := true }

def Duties.withDataLoss (d : Duties) : Duties :=
  { d with reportPossibleDataLoss := true }

/-- Pointwise union, used by the join. -/
def Duties.union (a b : Duties) : Duties :=
  ⟨a.releaseEpoch || b.releaseEpoch,
   a.noUse || b.noUse,
   a.noRetryClose || b.noRetryClose,
   a.noTreatNumberAsFree || b.noTreatNumberAsFree,
   a.keepInDenominator || b.keepInDenominator,
   a.reportUnresolved || b.reportUnresolved,
   a.reportPossibleDataLoss || b.reportPossibleDataLoss⟩

/-- The derived post-state of one call. `mayClose` is the whole permitted
recovery set: `close` is the only recovery operation the relation ever
considers. -/
structure PostState where
  target : Target
  descriptor : DescriptorState
  description : DescriptionState
  writeback : WriteBack
  duties : Duties
  mayClose : Bool
  deriving DecidableEq, Repr

/-! ## The relation -/

private def poisonedState (t : Target) (s : DescriptorState) : PostState :=
  ⟨t, s, .unknown, .unknown, Duties.poisoned, false⟩

/-- The open file description is freed only when the closed descriptor was its
last reference: a `dup`, a forked duplicate and an in-flight syscall in another
thread all keep it alive. -/
private def descriptionAfterRelease (p : Premises) : DescriptionState :=
  if p.lastReference then .released else .retained

/-- `R_close[target] : Premises → Outcome → PostState`.

`none` is outside the declared domain: the not-attempted arm has no epoch to
describe. Fail-closed by construction — every arm the documentation does not
determine is poisoned, and no arm permits an automatic retry. -/
def closePostState (t : Target) (p : Premises) : Outcome → Option PostState
  -- A7 is consumed by the whole relation: closing removes every POSIX record
  -- lock the process holds on the file, whichever descriptor took it. That
  -- footprint escapes all three modelled identities.
  | o =>
    if p.noRecordLocks = false then
      some (poisonedState t .indeterminate)
    else
      match o with
      | .notAttempted =>
          if p.epochLive then
            some ⟨t, .notAttempted, .retained, .unknown, Duties.live, true⟩
          else
            none
      -- A discarded result cannot be separated from EBADF, the one documented
      -- arm that does not release. An analysis gap, not a documentation gap.
      | .resultUnobserved => some (poisonedState t .attempted)
      | .zero =>
          if p.resultObserved = false then
            some (poisonedState t .attempted)
          -- A3 is load-bearing: a successful close proves that *some* epoch on
          -- that number was closed. Only sole custody identifies it with the
          -- tracked one. The dangerous double close does not report EBADF at
          -- all — on a recycled number it closes a foreign epoch and returns 0.
          else if p.soleCustodian = false then
            some (poisonedState t .indeterminate)
          else
            -- A successful close does not guarantee the data reached the disk:
            -- success discharges the descriptor, never the data.
            some ⟨t, .released, descriptionAfterRelease p, .unknown, Duties.none, false⟩
      | .minusOne e =>
          if p.resultObserved = false then
            some (poisonedState t .attempted)
          else if e = .unobserved || p.errnoObserved = false then
            some (poisonedState t .attempted)
          else if e = .ebadf then
            -- EBADF is a refutation, not information: under A2 the analysis
            -- believed the epoch live and this says it was not. Not `retained`,
            -- because the thing that would be retained is what was refuted.
            some (poisonedState t .indeterminate)
          else if e = .eintr then
            match t with
            -- The Linux kernel releases the descriptor early; the steps that
            -- can fail happen later. Retrying may close a number another thread
            -- has already reused.
            | .linux =>
                some ⟨t, .released, .unknown, .failed,
                      Duties.none.withDataLoss, false⟩
            -- POSIX.1-2024 standardized the behaviour in which the descriptor
            -- stays open and close must be called again.
            | .posix2024 =>
                some ⟨t, .retained, .retained, .failed,
                      Duties.live.withDataLoss, true⟩
            -- The targets do not merely differ in what is known: they prescribe
            -- opposite recovery. One requires the retry the other forbids.
            | .unpinned =>
                some ⟨t, .indeterminate, .unknown, .failed,
                      Duties.poisoned.withDataLoss, false⟩
          else if e.isDeferredIO then
            match t with
            | .linux =>
                some ⟨t, .released, descriptionAfterRelease p, .failed,
                      Duties.none.withDataLoss, false⟩
            | _ =>
                some ⟨t, .indeterminate, .unknown, .failed,
                      Duties.poisoned.withDataLoss, false⟩
          else
            match t with
            | .linux => some ⟨t, .released, .unknown, .unknown, Duties.none, false⟩
            | _ => some (poisonedState t .indeterminate)

/-- Join two post-states reaching the same program point.

Agreement is preserved exactly; disagreement widens to `indeterminate` with the
poisoned duty set, dropping `releaseEpoch` because it contradicts
`noRetryClose`. `none` refuses a join across targets. -/
def join (a b : PostState) : Option PostState :=
  if a.target ≠ b.target then none
  else
    let description := if a.description = b.description then a.description else .unknown
    let writeback :=
      if a.writeback = b.writeback then a.writeback
      else if a.writeback = .failed || b.writeback = .failed then WriteBack.failed
      else WriteBack.unknown
    if a.descriptor = b.descriptor then
      some ⟨a.target, a.descriptor, description, writeback,
            a.duties.union b.duties, a.mayClose && b.mayClose⟩
    else
      some ⟨a.target, .indeterminate, description, writeback,
            { Duties.poisoned.union (a.duties.union b.duties) with releaseEpoch := false },
            false⟩

/-! ## Endpoint identity

Contract spec §C10: an endpoint is identified by kind, acquisition generation
and lifetime, never by a number alone. Numeric reuse after an epoch has ended is
permitted and must not merge the epochs. -/

inductive EndpointKind where
  | descriptor
  deriving DecidableEq, Repr

structure Epoch where
  kind : EndpointKind
  number : Nat
  generation : Nat
  deriving DecidableEq, Repr

/-- The next epoch on the same number. -/
def Epoch.successor (e : Epoch) : Epoch :=
  { e with generation := e.generation + 1 }

/-! ## Exhaustive domain -/

def allTargets : List Target := [.linux, .posix2024, .unpinned]

def allErrnos : List Errno :=
  [.ebadf, .eintr, .eio, .enospc, .edquot, .other, .unobserved]

def allOutcomes : List Outcome :=
  [.notAttempted, .resultUnobserved, .zero] ++ allErrnos.map Outcome.minusOne

private def allBools : List Bool := [false, true]

def allPremises : List Premises :=
  allBools.flatMap fun a =>
    allBools.flatMap fun b =>
      allBools.flatMap fun c =>
        allBools.flatMap fun d =>
          allBools.flatMap fun e =>
            allBools.map fun f => ⟨a, b, c, d, e, f⟩

theorem mem_allTargets (t : Target) : t ∈ allTargets := by
  cases t <;> decide

theorem mem_allOutcomes (o : Outcome) : o ∈ allOutcomes := by
  cases o with
  | minusOne e => cases e <;> decide
  | _ => decide

theorem mem_allPremises (p : Premises) : p ∈ allPremises := by
  obtain ⟨a, b, c, d, e, f⟩ := p
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> cases f <;> decide

/-- The whole `3 × 64 × 10` domain of the relation. -/
def domainAll (P : Target → Premises → Outcome → Bool) : Bool :=
  allTargets.all fun t => allPremises.all fun p => allOutcomes.all fun o => P t p o

/-- Bridge from a decided exhaustive check to a universally quantified fact. -/
theorem of_domainAll {P : Target → Premises → Outcome → Bool}
    (h : domainAll P = true) (t : Target) (p : Premises) (o : Outcome) :
    P t p o = true := by
  have ht := (List.all_eq_true.mp h) t (mem_allTargets t)
  have hp := (List.all_eq_true.mp ht) p (mem_allPremises p)
  exact (List.all_eq_true.mp hp) o (mem_allOutcomes o)

/-! ## Standing invariants over the whole domain -/

private def dischargesNothing (t : Target) (p : Premises) (o : Outcome) : Bool :=
  match closePostState t p o with
  | none => true
  | some s => s.writeback != WriteBack.discharged

/-- `close` never discharges the data duty, on any target, under any premises,
for any outcome. Only `fsync` could, and this relation does not model it. -/
theorem no_outcome_ever_discharges_the_data_duty
    (t : Target) (p : Premises) (o : Outcome) (s : PostState)
    (h : closePostState t p o = some s) : s.writeback ≠ WriteBack.discharged := by
  have hd : dischargesNothing t p o = true := of_domainAll (by decide) t p o
  unfold dischargesNothing at hd
  rw [h] at hd
  simpa using hd

private def poisonedPermitsNothing (t : Target) (p : Premises) (o : Outcome) : Bool :=
  match closePostState t p o with
  | none => true
  | some s => !s.descriptor.isPoisoned || s.mayClose == false

/-- **Acceptance item 3.** No poisoned post-state prescribes a retry, anywhere
in the domain. -/
theorem a_poisoned_state_permits_no_recovery
    (t : Target) (p : Premises) (o : Outcome) (s : PostState)
    (h : closePostState t p o = some s)
    (hp : s.descriptor.isPoisoned = true) : s.mayClose = false := by
  have hd : poisonedPermitsNothing t p o = true := of_domainAll (by decide) t p o
  unfold poisonedPermitsNothing at hd
  rw [h] at hd
  simp [hp] at hd
  exact hd

private def poisonedKeepsDuties (t : Target) (p : Premises) (o : Outcome) : Bool :=
  match closePostState t p o with
  | none => true
  | some s =>
      !s.descriptor.isPoisoned ||
        (s.duties.noUse && s.duties.noRetryClose && s.duties.noTreatNumberAsFree &&
          s.duties.keepInDenominator && s.duties.reportUnresolved &&
          !s.duties.releaseEpoch)

/-- A poisoned epoch carries the complete conservative residual: it may not be
used, retried or recycled, it stays in the live denominator, and it is never
also owed a plain release. -/
theorem a_poisoned_state_carries_the_conservative_residual
    (t : Target) (p : Premises) (o : Outcome) (s : PostState)
    (h : closePostState t p o = some s)
    (hp : s.descriptor.isPoisoned = true) :
    s.duties.noUse = true ∧ s.duties.noRetryClose = true ∧
      s.duties.noTreatNumberAsFree = true ∧ s.duties.keepInDenominator = true ∧
      s.duties.reportUnresolved = true ∧ s.duties.releaseEpoch = false := by
  have hd : poisonedKeepsDuties t p o = true := of_domainAll (by decide) t p o
  unfold poisonedKeepsDuties at hd
  rw [h] at hd
  simp [hp] at hd
  exact ⟨hd.1.1.1.1.1, hd.1.1.1.1.2, hd.1.1.1.2, hd.1.1.2, hd.1.2, hd.2⟩

private def onlyLiveMayClose (t : Target) (p : Premises) (o : Outcome) : Bool :=
  match closePostState t p o with
  | none => true
  | some s =>
      !s.mayClose ||
        (s.descriptor == DescriptorState.notAttempted ||
          s.descriptor == DescriptorState.retained)

/-- Only a state that still holds a live epoch may close it. In particular a
released epoch permits nothing on its own generation. -/
theorem only_a_live_epoch_may_be_closed
    (t : Target) (p : Premises) (o : Outcome) (s : PostState)
    (h : closePostState t p o = some s) (hc : s.mayClose = true) :
    s.descriptor = DescriptorState.notAttempted ∨
      s.descriptor = DescriptorState.retained := by
  have hd : onlyLiveMayClose t p o = true := of_domainAll (by decide) t p o
  unfold onlyLiveMayClose at hd
  rw [h] at hd
  simp [hc] at hd
  rcases hd with hd | hd
  · exact Or.inl hd
  · exact Or.inr hd

private def releasedNeedsLastReference (t : Target) (p : Premises) (o : Outcome) : Bool :=
  match closePostState t p o with
  | none => true
  | some s => s.description != DescriptionState.released || p.lastReference

/-- Descriptor release is not description release: the open file description is
reported released only under the last-reference premise. -/
theorem the_description_is_released_only_with_the_last_reference
    (t : Target) (p : Premises) (o : Outcome) (s : PostState)
    (h : closePostState t p o = some s)
    (hr : s.description = DescriptionState.released) : p.lastReference = true := by
  have hd : releasedNeedsLastReference t p o = true := of_domainAll (by decide) t p o
  unfold releasedNeedsLastReference at hd
  rw [h] at hd
  simp [hr] at hd
  exact hd

private def deniedLocksNeverRelease (t : Target) (p : Premises) (o : Outcome) : Bool :=
  match closePostState t p o with
  | none => true
  | some s => p.noRecordLocks || s.descriptor == DescriptorState.indeterminate

/-- A7 denied is refused, not ignored: closing removes process-wide record locks
on the file, an effect outside every modelled identity. -/
theorem denied_record_locks_are_never_released
    (t : Target) (p : Premises) (o : Outcome) (s : PostState)
    (h : closePostState t p o = some s) (hl : p.noRecordLocks = false) :
    s.descriptor = DescriptorState.indeterminate := by
  have hd : deniedLocksNeverRelease t p o = true := of_domainAll (by decide) t p o
  unfold deniedLocksNeverRelease at hd
  rw [h] at hd
  simp [hl] at hd
  exact hd

/-! ## The arms the documentation determines -/

/-- A premise vector granting everything, for the pointwise arms below. -/
def granted : Premises := ⟨true, true, true, true, true, true⟩

/-- On Linux a descriptor interrupted mid-close is nevertheless released, and
the data duty has failed. -/
theorem linux_eintr_releases_the_descriptor :
    closePostState .linux granted (.minusOne .eintr) =
      some ⟨.linux, .released, .unknown, .failed, Duties.none.withDataLoss, false⟩ := by
  decide

/-- POSIX.1-2024 keeps the descriptor open on EINTR and requires another
`close`. -/
theorem posix2024_eintr_retains_the_descriptor :
    closePostState .posix2024 granted (.minusOne .eintr) =
      some ⟨.posix2024, .retained, .retained, .failed, Duties.live.withDataLoss, true⟩ := by
  decide

/-- The two targets prescribe **opposite** recovery for the same errno: one
forbids the retry the other requires. This is why the profile is
target-qualified rather than portable. -/
theorem the_targets_prescribe_opposite_recovery_on_eintr :
    (∃ s, closePostState .linux granted (.minusOne .eintr) = some s ∧ s.mayClose = false) ∧
    (∃ s, closePostState .posix2024 granted (.minusOne .eintr) = some s ∧ s.mayClose = true) := by
  refine ⟨⟨_, linux_eintr_releases_the_descriptor, by decide⟩,
          ⟨_, posix2024_eintr_retains_the_descriptor, by decide⟩⟩

/-- With no target pinned, EINTR is indeterminate and permits nothing: the
fail-closed answer is to carry the residual, not to retry and not to assume a
release. -/
theorem an_unpinned_eintr_is_indeterminate_and_permits_no_retry :
    closePostState .unpinned granted (.minusOne .eintr) =
      some ⟨.unpinned, .indeterminate, .unknown, .failed,
            Duties.poisoned.withDataLoss, false⟩ := by
  decide

/-- On Linux the deferred write-back family releases the descriptor and fails
the data duty. -/
theorem linux_deferred_io_releases_and_fails_the_data_duty (e : Errno)
    (he : e.isDeferredIO = true) :
    closePostState .linux granted (.minusOne e) =
      some ⟨.linux, .released, .released, .failed, Duties.none.withDataLoss, false⟩ := by
  cases e <;> simp [Errno.isDeferredIO] at he <;> decide

private def ebadfNeverReleases (t : Target) (p : Premises) : Bool :=
  match closePostState t p (.minusOne .ebadf) with
  | none => true
  | some s => s.descriptor != DescriptorState.released

/-- EBADF is the one documented error on which nothing was closed, so it never
yields a release on any target. -/
theorem ebadf_never_releases (t : Target) (p : Premises) (s : PostState)
    (h : closePostState t p (.minusOne .ebadf) = some s) :
    s.descriptor ≠ DescriptorState.released := by
  have hd : ebadfNeverReleases t p = true := by
    have := of_domainAll (P := fun t p _ => ebadfNeverReleases t p) (by decide) t p .zero
    exact this
  unfold ebadfNeverReleases at hd
  rw [h] at hd
  simpa using hd

private def releaseNeedsAnObservedResult (t : Target) (p : Premises) (o : Outcome) : Bool :=
  match closePostState t p o with
  | none => true
  | some s =>
      s.descriptor != DescriptorState.released ||
        (p.resultObserved && o != Outcome.resultUnobserved)

/-- A discarded result can never conclude a release: it does not exclude the
EBADF arm, the one documented error on which nothing was closed. This is what
makes checking the return value of `close` a soundness requirement of the
profile rather than hygiene advice. -/
theorem a_release_requires_an_observed_result
    (t : Target) (p : Premises) (o : Outcome) (s : PostState)
    (h : closePostState t p o = some s)
    (hr : s.descriptor = DescriptorState.released) :
    p.resultObserved = true ∧ o ≠ Outcome.resultUnobserved := by
  have hd : releaseNeedsAnObservedResult t p o = true := of_domainAll (by decide) t p o
  unfold releaseNeedsAnObservedResult at hd
  rw [h] at hd
  simp [hr] at hd
  exact ⟨hd.1, hd.2⟩

private def failedReleaseNeedsAnObservedErrno (t : Target) (p : Premises) (o : Outcome) : Bool :=
  match o with
  | .minusOne e =>
      (match closePostState t p o with
       | none => true
       | some s =>
           s.descriptor != DescriptorState.released || (p.errnoObserved && e != Errno.unobserved))
  | _ => true

/-- An unread `errno` on the failure branch can never conclude a release
either, for the same reason. -/
theorem a_failed_close_releases_only_with_an_observed_errno
    (t : Target) (p : Premises) (e : Errno) (s : PostState)
    (h : closePostState t p (.minusOne e) = some s)
    (hr : s.descriptor = DescriptorState.released) :
    p.errnoObserved = true ∧ e ≠ Errno.unobserved := by
  have hd : failedReleaseNeedsAnObservedErrno t p (.minusOne e) = true :=
    of_domainAll (by decide) t p (.minusOne e)
  unfold failedReleaseNeedsAnObservedErrno at hd
  rw [h] at hd
  simp [hr] at hd
  exact ⟨hd.1, hd.2⟩

private def successNeedsSoleCustody (t : Target) (p : Premises) : Bool :=
  match closePostState t p .zero with
  | none => true
  | some s => p.soleCustodian || s.descriptor != DescriptorState.released

/-- A successful close proves only that *some* epoch on that number was closed.
Without sole custody it does not identify the tracked epoch, so it releases
nothing. -/
theorem success_without_sole_custody_never_releases
    (t : Target) (p : Premises) (s : PostState)
    (h : closePostState t p .zero = some s) (hc : p.soleCustodian = false) :
    s.descriptor ≠ DescriptorState.released := by
  have hd : successNeedsSoleCustody t p = true := by
    have := of_domainAll (P := fun t p _ => successNeedsSoleCustody t p) (by decide) t p .zero
    exact this
  unfold successNeedsSoleCustody at hd
  rw [h] at hd
  simp [hc] at hd
  simpa using hd

/-- The not-attempted arm has no epoch to describe when the caller has already
said none is live. -/
theorem the_not_attempted_arm_requires_a_live_epoch
    (t : Target) (p : Premises) (hl : p.noRecordLocks = true) (he : p.epochLive = false) :
    closePostState t p .notAttempted = none := by
  obtain ⟨a, b, c, d, e, f⟩ := p
  subst hl
  subst he
  cases t <;> cases b <;> cases c <;> cases d <;> cases e <;> decide

/-! ## Non-vacuity -/

/-- All five descriptor states are actually derived. The taxonomy is not
decorative. -/
theorem every_descriptor_state_is_reachable :
    (closePostState .linux granted .notAttempted).map PostState.descriptor =
        some DescriptorState.notAttempted ∧
    (closePostState .linux granted .resultUnobserved).map PostState.descriptor =
        some DescriptorState.attempted ∧
    (closePostState .posix2024 granted (.minusOne .eintr)).map PostState.descriptor =
        some DescriptorState.retained ∧
    (closePostState .linux granted .zero).map PostState.descriptor =
        some DescriptorState.released ∧
    (closePostState .linux granted (.minusOne .ebadf)).map PostState.descriptor =
        some DescriptorState.indeterminate := by
  decide

/-- `attempted` and `indeterminate` are distinct states carrying identical
duties and the same empty recovery: they differ in whether a better analysis
could resolve them, not in what is owed. Both literal post-states below are the
same except for the descriptor field. -/
theorem attempted_and_indeterminate_differ_only_in_provenance :
    closePostState .linux granted .resultUnobserved =
        some ⟨.linux, .attempted, .unknown, .unknown, Duties.poisoned, false⟩ ∧
    closePostState .linux granted (.minusOne .ebadf) =
        some ⟨.linux, .indeterminate, .unknown, .unknown, Duties.poisoned, false⟩ ∧
    DescriptorState.attempted ≠ DescriptorState.indeterminate := by
  decide

/-! ## Joins -/

/-- Disagreeing paths widen to `indeterminate`, permit nothing, and drop the
plain release duty, which contradicts the no-retry duty. -/
theorem a_join_of_disagreeing_states_is_poisoned (a b : PostState) (s : PostState)
    (h : join a b = some s) (hne : a.descriptor ≠ b.descriptor) :
    s.descriptor = DescriptorState.indeterminate ∧ s.mayClose = false ∧
      s.duties.noRetryClose = true ∧ s.duties.releaseEpoch = false := by
  unfold join at h
  by_cases ht : a.target ≠ b.target
  · simp [ht] at h
  · simp [ht, hne] at h
    subst h
    exact ⟨rfl, rfl, by simp [Duties.poisoned, Duties.union], rfl⟩

/-- A join never produces a permitted recovery that both sides did not
permit. -/
theorem a_join_never_invents_a_permitted_recovery (a b : PostState) (s : PostState)
    (h : join a b = some s) (hc : s.mayClose = true) :
    a.mayClose = true ∧ b.mayClose = true := by
  unfold join at h
  by_cases ht : a.target ≠ b.target
  · simp [ht] at h
  · by_cases hd : a.descriptor = b.descriptor
    · simp [ht, hd] at h
      subst h
      simpa [Bool.and_eq_true] using hc
    · simp [ht, hd] at h
      subst h
      simp at hc

/-- A join across targets is refused rather than silently mixing two
platforms. -/
theorem a_join_across_targets_is_refused (a b : PostState) (h : a.target ≠ b.target) :
    join a b = none := by
  unfold join
  simp [h]

/-! ## Endpoint identity -/

/-- Numeric reuse never merges epochs: a successor on the same number is a
different identity. -/
theorem a_reused_number_is_a_distinct_epoch (e : Epoch) :
    e.successor ≠ e ∧ e.successor.number = e.number := by
  refine ⟨?_, rfl⟩
  intro h
  have : e.generation + 1 = e.generation := congrArg Epoch.generation h
  omega

/-! ## The distinction the card exists to draw -/

/-- A wrapper that returns `-1`/EIO without reaching the kernel necessarily
leaves the descriptor open — the POSIX.1-2024 post-state, not the Linux one.
Reading the project's suppressed-syscall fixtures as evidence about real Linux
close failures would invert the answer. -/
theorem the_suppressed_wrapper_post_state_is_not_the_linux_post_state :
    (closePostState .linux granted (.minusOne .eio)).map PostState.descriptor =
        some DescriptorState.released ∧
    (closePostState .posix2024 granted (.minusOne .eintr)).map PostState.descriptor =
        some DescriptorState.retained ∧
    DescriptorState.released ≠ DescriptorState.retained := by
  decide

end QCore.OsEndpointPostState
