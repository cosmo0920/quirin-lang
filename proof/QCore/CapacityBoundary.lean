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
# The file-backed capacity grow boundary

The reduced model of card O02's relation for **one** boundary family: growing a
file-backed chunk, and the kernel refusing that growth for want of capacity.
The deciding implementation is `crates/qcore-core/src/capacity_boundary.rs`; the
recorded native evidence and its result boundary are
`docs/reports/remaining-o02/README.md`.

Four quantities are separated, because one success bit cannot express "the file
is longer **and** nothing more is backed":

* the file **length**, moved by every grow call;
* the reserved **backing**, moved only by an allocating call — `ftruncate`
  leaves a hole, and a store into that hole may fault rather than the call;
* the **mapping** epoch, which may be longer than the file and therefore
  witnesses no capacity past its end;
* the **logical** content length, which no grow call ever advances.

Three reductions are deliberate, and are the reason this is a model rather than
a transcription. Sizes are small natural numbers rather than the recorded byte
counts; backing is measured in bytes rather than the 512-byte units `st_blocks`
reports; and at most one mapping epoch is live at a time, which is what the
recorded scenarios do and not a general fact. The exhaustive statements below
quantify over the whole premise and outcome vocabulary but over a
*representative* family of measurements, named in `allObservations`.

This is a model of the *decisions*. Which arm a real call took, and whether the
documentation is faithful to the kernel, are premises supplied from outside.
No durability claim, no filesystem model and no verdict follows.
-/

namespace QCore.CapacityBoundary

/-! ## Vocabulary -/

/-- The platform whose documented semantics the relation is read against. -/
inductive Target where
  /-- A Linux ABI target. -/
  | linux
  /-- No target pinned. -/
  | unpinned
  deriving DecidableEq, Repr

/-- The call used to grow the file. -/
inductive GrowCall where
  /-- `ftruncate(2)`: moves the length, reserves nothing. -/
  | ftruncate
  /-- `fallocate(2)`: reserves, reports through `errno`. -/
  | fallocate
  /-- `posix_fallocate(3)`: reserves, reports the error as its **return value**
  and does not set `errno`. -/
  | posixFallocate
  /-- A write that extends the file, and may move part of its request. -/
  | writeCall
  deriving DecidableEq, Repr

/-- Whether a successful call reserves backing for the range it covers. -/
def GrowCall.reservesBacking : GrowCall → Bool
  | .ftruncate => false
  | _ => true

/-- Whether a failure of this call reports through `errno`. -/
def GrowCall.reportsThroughErrno : GrowCall → Bool
  | .posixFallocate => false
  | _ => true

/-- Whether this call can move part of its request. -/
def GrowCall.mayTransferPartially : GrowCall → Bool
  | .writeCall => true
  | _ => false

/-- Whether a *failed* call may nevertheless have changed the length.
`ftruncate(2)` documents no change; the allocating calls leave a partial effect
unspecified. -/
def GrowCall.failureMayChangeLength : GrowCall → Bool
  | .ftruncate => false
  | _ => true

/-- The error observed on a failing call. -/
inductive Errno where
  | enospc
  | edquot
  | efbig
  | eintr
  | eio
  | other
  /-- The failure branch did not observe the error. -/
  | unobserved
  deriving DecidableEq, Repr

/-- The capacity class: the errors that say the space is not available. -/
def Errno.isCapacityClass : Errno → Bool
  | .enospc => true
  | .edquot => true
  | .efbig => true
  | _ => false

/-- What the call was observed to do. -/
inductive Outcome where
  | notAttempted
  | resultUnobserved
  | granted
  /-- Part of the request moved. -/
  | partialTransfer (moved : Nat)
  | refused (e : Errno)
  deriving DecidableEq, Repr

/-- The measurements taken around one call. Every field is an observation. -/
structure Observation where
  call : GrowCall
  requestLen : Nat
  priorLen : Nat
  observedLen : Nat
  /-- Backed bytes after the call. The Rust implementation reads this from
  `st_blocks` in 512-byte units; the model uses bytes directly. -/
  observedBacked : Nat
  /-- The length of the live mapping, if any. -/
  mappingLen : Option Nat
  deriving DecidableEq, Repr

/-- Pre-state facts the relation consumes and never establishes. -/
structure Premises where
  resultObserved : Bool
  errnoObserved : Bool
  lengthObserved : Bool
  backingObserved : Bool
  soleWriter : Bool
  signalHandled : Bool
  deriving DecidableEq, Repr

/-- The post-state of the file length. -/
inductive FileLength where
  | unchanged (n : Nat)
  | grownTo (n : Nat)
  | indeterminate
  deriving DecidableEq, Repr

/-- The post-state of the reserved backing. -/
inductive Backing where
  | reserved (n : Nat)
  | notReserved
  | unknown
  deriving DecidableEq, Repr

/-- Whether this post-state claims reserved backing at all. -/
def Backing.isReserved : Backing → Bool
  | .reserved _ => true
  | _ => false

/-- What the caller must do with a live mapping. -/
inductive MappingDuty where
  | noMapping
  | remapBeforeUse (toLen : Nat)
  | retainExisting
  | poisoned
  deriving DecidableEq, Repr

/-- What may be attempted next. -/
inductive Recovery where
  | none
  | retrySameRequest
  | retryFromObservedLength
  | retryAfterCapacityChange
  deriving DecidableEq, Repr

/-- Residual obligations carried out of the call. -/
structure Duties where
  remapBeforeUse : Bool
  noStoreBeyondReserved : Bool
  keepLogicalLength : Bool
  reobserveLength : Bool
  releaseEpochs : Bool
  reportUnresolved : Bool
  deriving DecidableEq, Repr

/-- The conservative set carried by every undetermined arm. -/
def Duties.poisoned : Duties := ⟨false, true, true, true, true, true⟩

/-- The decided post-state of one grow call. -/
structure PostState where
  fileLength : FileLength
  backing : Backing
  mapping : MappingDuty
  duties : Duties
  recovery : Recovery
  /-- Bytes of *content* the call authorises. A grow call authorises zero. -/
  logicalAdvance : Nat
  deriving DecidableEq, Repr

/-- A post-state that determines nothing. -/
def PostState.isPoisoned (s : PostState) : Bool := s.duties.reportUnresolved

private def poisonedState : PostState :=
  ⟨.indeterminate, .unknown, .poisoned, Duties.poisoned, .none, 0⟩

private def mappingDutyOnGrowth (o : Observation) (toLen : Nat) : MappingDuty :=
  match o.mappingLen with
  | none => .noMapping
  | some len => if len < toLen then .remapBeforeUse toLen else .retainExisting

private def remapRequired : MappingDuty → Bool
  | .remapBeforeUse _ => true
  | _ => false

/-! ## The relation

`growPostState[target] : Premises → Observation → Outcome → Option PostState`.

`none` is outside the declared domain: a request that does not exceed the prior
length is not a grow, and a mapping of length zero is not a mapping.
Fail-closed by construction — every arm the documentation or the observation
leaves open is poisoned, and no poisoned arm permits a recovery. -/
def growPostState (t : Target) (p : Premises) (o : Observation) (out : Outcome) :
    Option PostState :=
  if o.requestLen ≤ o.priorLen then none
  else if o.mappingLen = some 0 then none
  else if p.soleWriter = false then some poisonedState
  -- Left at its default disposition, the boundary signal terminates the
  -- process, so no post-state is observable at all.
  else if o.call = .writeCall && p.signalHandled = false then some poisonedState
  -- `errno` read from the call that does not set it.
  else if p.errnoObserved && o.call.reportsThroughErrno = false then some poisonedState
  else if t = .unpinned then some poisonedState
  else
    match out with
    | .notAttempted =>
        some ⟨.unchanged o.priorLen, .unknown,
              (match o.mappingLen with | none => .noMapping | some _ => .retainExisting),
              ⟨false, true, true, false, true, false⟩, .retrySameRequest, 0⟩
    | .resultUnobserved => some poisonedState
    | .granted =>
        if p.resultObserved = false || p.lengthObserved = false then
          some poisonedState
        else if o.observedLen < o.requestLen then
          some poisonedState
        else if o.call.reservesBacking &&
            (p.backingObserved = false || o.observedBacked < o.requestLen) then
          some poisonedState
        else
          let duty := mappingDutyOnGrowth o o.observedLen
          some ⟨.grownTo o.observedLen,
                (if o.call.reservesBacking then .reserved o.observedLen else .notReserved),
                duty,
                ⟨remapRequired duty, !o.call.reservesBacking, true, false, true, false⟩,
                .none, 0⟩
    | .partialTransfer moved =>
        if o.call.mayTransferPartially = false || moved = 0 ||
            o.requestLen ≤ o.priorLen + moved then
          some poisonedState
        else if p.resultObserved = false || p.lengthObserved = false ||
            o.observedLen ≠ o.priorLen + moved then
          some poisonedState
        else
          let duty := mappingDutyOnGrowth o o.observedLen
          some ⟨.grownTo o.observedLen, .reserved o.observedLen, duty,
                ⟨remapRequired duty, true, false, true, true, false⟩,
                .retryFromObservedLength, moved⟩
    | .refused e =>
        if p.resultObserved = false then some poisonedState
        else if e = .unobserved || (o.call.reportsThroughErrno && p.errnoObserved = false) then
          some poisonedState
        else if p.lengthObserved = false then some poisonedState
        else if o.observedLen ≠ o.priorLen && o.call.failureMayChangeLength = false then
          some poisonedState
        else
          some ⟨(if o.observedLen = o.priorLen then .unchanged o.priorLen
                 else .grownTo o.observedLen),
                (if o.call.reservesBacking then .unknown else .notReserved),
                .retainExisting,
                ⟨false, true, true, o.call.reservesBacking, true, false⟩,
                (if e.isCapacityClass then .retryAfterCapacityChange
                 else if e = .eintr then .retryFromObservedLength
                 else .none),
                0⟩

/-- What a repeat of the identical request may conclude. -/
inductive RetryVerdict where
  | refused
  | sameClassExpected
  | undetermined
  deriving DecidableEq, Repr

/-- A repeat decides nothing new until the available capacity changes. -/
def retryVerdict (s : PostState) (capacityChanged : Bool) : RetryVerdict :=
  match s.recovery with
  | .none => .refused
  | .retryAfterCapacityChange => if capacityChanged then .undetermined else .sameClassExpected
  | _ => .undetermined

/-! ## What a mapping witnesses -/

structure Witness where
  usable : Nat
  beyondEndOfFile : Nat
  deriving DecidableEq, Repr

/-- A mapping never witnesses capacity it does not have: not past the end of
the file, and not inside a hole. -/
def mappingWitness (mappingLen fileLen : Nat) (b : Backing) : Witness :=
  let within := min mappingLen fileLen
  let backed := match b with
    | .reserved n => min within n
    | _ => 0
  ⟨backed, mappingLen - fileLen⟩

/-! ## Exhaustive domain -/

def allTargets : List Target := [.linux, .unpinned]

def allCalls : List GrowCall := [.ftruncate, .fallocate, .posixFallocate, .writeCall]

def allErrnos : List Errno :=
  [.enospc, .edquot, .efbig, .eintr, .eio, .other, .unobserved]

def allOutcomes : List Outcome :=
  [.notAttempted, .resultUnobserved, .granted, .partialTransfer 1, .partialTransfer 2] ++
    allErrnos.map Outcome.refused

private def allBools : List Bool := [false, true]

def allPremises : List Premises :=
  allBools.flatMap fun a =>
    allBools.flatMap fun b =>
      allBools.flatMap fun c =>
        allBools.flatMap fun d =>
          allBools.flatMap fun e =>
            allBools.map fun f => ⟨a, b, c, d, e, f⟩

/-- Four representative measurements per call: a grant that reached the request
with backing and without, a call that changed nothing, and one that moved part
of the request with no mapping live. The prior length is `2` and the request
`4` throughout. -/
def allObservations : List Observation :=
  allCalls.flatMap fun c =>
    [⟨c, 4, 2, 4, 4, some 2⟩, ⟨c, 4, 2, 4, 2, some 2⟩,
     ⟨c, 4, 2, 2, 2, some 2⟩, ⟨c, 4, 2, 3, 3, none⟩]

theorem mem_allTargets (t : Target) : t ∈ allTargets := by
  cases t <;> decide

theorem mem_allPremises (p : Premises) : p ∈ allPremises := by
  obtain ⟨a, b, c, d, e, f⟩ := p
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> cases f <;> decide

/-- The checked domain: every target, every premise assignment, every
representative measurement and every outcome shape. -/
def domainAll (P : Target → Premises → Observation → Outcome → Bool) : Bool :=
  allTargets.all fun t =>
    allPremises.all fun p =>
      allObservations.all fun o => allOutcomes.all fun out => P t p o out

theorem of_domainAll {P : Target → Premises → Observation → Outcome → Bool}
    (h : domainAll P = true) (t : Target) (p : Premises) (o : Observation) (out : Outcome)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes) :
    P t p o out = true := by
  have ht := (List.all_eq_true.mp h) t (mem_allTargets t)
  have hp := (List.all_eq_true.mp ht) p (mem_allPremises p)
  have hobs := (List.all_eq_true.mp hp) o ho
  exact (List.all_eq_true.mp hobs) out hout

/-! ## Standing invariants over the checked domain

Each invariant below is a predicate over the same domain, and each one starts by
computing `growPostState` at the point. Deciding them one at a time enumerated
the domain — and recomputed every post-state — once per invariant. `allInvariants`
conjoins the six checks behind a single `growPostState`, so `all_invariants_hold`
decides the domain once and `invariants_at` hands the result to each invariant's
own pointwise lemma. The domain, the six checks and every theorem statement are
unchanged; a check that drifted away from its copy here would fail its pointwise
lemma rather than pass silently. -/

private def allInvariants
    (t : Target) (p : Premises) (o : Observation) (out : Outcome) : Bool :=
  match growPostState t p o out with
  | none => true
  | some s =>
      (s.duties.noStoreBeyondReserved || s.backing.isReserved)
        && (!s.isPoisoned ||
              (s.recovery == Recovery.none && s.duties == Duties.poisoned &&
                s.backing == Backing.unknown && s.logicalAdvance == 0))
        && (o.call.mayTransferPartially || s.logicalAdvance == 0)
        && (t != Target.unpinned || s.isPoisoned)
        && (p.soleWriter || s.isPoisoned)
        && (p.resultObserved || out == Outcome.notAttempted || s.isPoisoned)

private theorem all_invariants_hold : domainAll allInvariants = true := by decide

private theorem invariants_at (t : Target) (p : Premises) (o : Observation) (out : Outcome)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes) :
    allInvariants t p o out = true :=
  of_domainAll all_invariants_hold t p o out ho hout

private def neverAuthorisesAnUnreservedStore
    (t : Target) (p : Premises) (o : Observation) (out : Outcome) : Bool :=
  match growPostState t p o out with
  | none => true
  | some s => s.duties.noStoreBeyondReserved || s.backing.isReserved

private theorem never_authorises_unreserved_store_holds
    (t : Target) (p : Premises) (o : Observation) (out : Outcome)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes) :
    neverAuthorisesAnUnreservedStore t p o out = true := by
  have hi := invariants_at t p o out ho hout
  unfold allInvariants at hi
  unfold neverAuthorisesAnUnreservedStore
  cases hg : growPostState t p o out with
  | none => rfl
  | some s =>
      rw [hg] at hi
      simp only [Bool.and_eq_true] at hi
      exact hi.1.1.1.1.1

/-- No arm drops the store duty without having reserved the range. A length a
call merely reported is never writable authority. -/
theorem no_arm_authorises_an_unreserved_store
    (t : Target) (p : Premises) (o : Observation) (out : Outcome) (s : PostState)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes)
    (h : growPostState t p o out = some s)
    (hd : s.duties.noStoreBeyondReserved = false) : s.backing.isReserved = true := by
  have hall : neverAuthorisesAnUnreservedStore t p o out = true :=
    never_authorises_unreserved_store_holds t p o out ho hout
  unfold neverAuthorisesAnUnreservedStore at hall
  rw [h] at hall
  simp [hd] at hall
  exact hall

private def poisonedPermitsNothing
    (t : Target) (p : Premises) (o : Observation) (out : Outcome) : Bool :=
  match growPostState t p o out with
  | none => true
  | some s =>
      !s.isPoisoned ||
        (s.recovery == Recovery.none && s.duties == Duties.poisoned &&
          s.backing == Backing.unknown && s.logicalAdvance == 0)

private theorem poisoned_permits_nothing_holds
    (t : Target) (p : Premises) (o : Observation) (out : Outcome)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes) :
    poisonedPermitsNothing t p o out = true := by
  have hi := invariants_at t p o out ho hout
  unfold allInvariants at hi
  unfold poisonedPermitsNothing
  cases hg : growPostState t p o out with
  | none => rfl
  | some s =>
      rw [hg] at hi
      simp only [Bool.and_eq_true] at hi
      exact hi.1.1.1.1.2

/-- **Acceptance item 3, in the model.** A poisoned post-state permits no
recovery, carries the whole conservative duty set, claims no backing and
authorises no content. -/
theorem a_poisoned_arm_permits_nothing
    (t : Target) (p : Premises) (o : Observation) (out : Outcome) (s : PostState)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes)
    (h : growPostState t p o out = some s) (hp : s.isPoisoned = true) :
    s.recovery = Recovery.none ∧ s.duties = Duties.poisoned ∧
      s.backing = Backing.unknown ∧ s.logicalAdvance = 0 := by
  have hall : poisonedPermitsNothing t p o out = true :=
    poisoned_permits_nothing_holds t p o out ho hout
  unfold poisonedPermitsNothing at hall
  rw [h] at hall
  simp [hp] at hall
  exact ⟨hall.1.1.1, hall.1.1.2, hall.1.2, hall.2⟩

private def capacityIsNeverContent
    (t : Target) (p : Premises) (o : Observation) (out : Outcome) : Bool :=
  match growPostState t p o out with
  | none => true
  | some s => o.call.mayTransferPartially || s.logicalAdvance == 0

private theorem capacity_is_never_content_holds
    (t : Target) (p : Premises) (o : Observation) (out : Outcome)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes) :
    capacityIsNeverContent t p o out = true := by
  have hi := invariants_at t p o out ho hout
  unfold allInvariants at hi
  unfold capacityIsNeverContent
  cases hg : growPostState t p o out with
  | none => rfl
  | some s =>
      rw [hg] at hi
      simp only [Bool.and_eq_true] at hi
      exact hi.1.1.1.2

/-- Capacity is not content: no resizing call advances the content length.
Only an observed transfer does. -/
theorem capacity_is_never_content
    (t : Target) (p : Premises) (o : Observation) (out : Outcome) (s : PostState)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes)
    (h : growPostState t p o out = some s)
    (hc : o.call.mayTransferPartially = false) : s.logicalAdvance = 0 := by
  have hall : capacityIsNeverContent t p o out = true :=
    capacity_is_never_content_holds t p o out ho hout
  unfold capacityIsNeverContent at hall
  rw [h] at hall
  simp [hc] at hall
  exact hall

private def unpinnedConcludesNothing
    (t : Target) (p : Premises) (o : Observation) (out : Outcome) : Bool :=
  match growPostState t p o out with
  | none => true
  | some s => t != Target.unpinned || s.isPoisoned

private theorem unpinned_concludes_nothing_holds
    (t : Target) (p : Premises) (o : Observation) (out : Outcome)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes) :
    unpinnedConcludesNothing t p o out = true := by
  have hi := invariants_at t p o out ho hout
  unfold allInvariants at hi
  unfold unpinnedConcludesNothing
  cases hg : growPostState t p o out with
  | none => rfl
  | some s =>
      rw [hg] at hi
      simp only [Bool.and_eq_true] at hi
      exact hi.1.1.2

/-- Without a pinned target nothing is concluded. -/
theorem an_unpinned_target_concludes_nothing
    (p : Premises) (o : Observation) (out : Outcome) (s : PostState)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes)
    (h : growPostState .unpinned p o out = some s) : s.isPoisoned = true := by
  have hall : unpinnedConcludesNothing .unpinned p o out = true :=
    unpinned_concludes_nothing_holds .unpinned p o out ho hout
  unfold unpinnedConcludesNothing at hall
  rw [h] at hall
  simpa using hall

private def concurrentWriterPoisons
    (t : Target) (p : Premises) (o : Observation) (out : Outcome) : Bool :=
  match growPostState t p o out with
  | none => true
  | some s => p.soleWriter || s.isPoisoned

private theorem concurrent_writer_poisons_holds
    (t : Target) (p : Premises) (o : Observation) (out : Outcome)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes) :
    concurrentWriterPoisons t p o out = true := by
  have hi := invariants_at t p o out ho hout
  unfold allInvariants at hi
  unfold concurrentWriterPoisons
  cases hg : growPostState t p o out with
  | none => rfl
  | some s =>
      rw [hg] at hi
      simp only [Bool.and_eq_true] at hi
      exact hi.1.2

/-- A writer the observation did not exclude poisons every arm. -/
theorem a_concurrent_writer_poisons_every_arm
    (t : Target) (p : Premises) (o : Observation) (out : Outcome) (s : PostState)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes)
    (h : growPostState t p o out = some s) (hw : p.soleWriter = false) :
    s.isPoisoned = true := by
  have hall : concurrentWriterPoisons t p o out = true :=
    concurrent_writer_poisons_holds t p o out ho hout
  unfold concurrentWriterPoisons at hall
  rw [h] at hall
  simp [hw] at hall
  exact hall

private def unobservedResultPoisons
    (t : Target) (p : Premises) (o : Observation) (out : Outcome) : Bool :=
  match growPostState t p o out with
  | none => true
  | some s => p.resultObserved || out == Outcome.notAttempted || s.isPoisoned

private theorem unobserved_result_poisons_holds
    (t : Target) (p : Premises) (o : Observation) (out : Outcome)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes) :
    unobservedResultPoisons t p o out = true := by
  have hi := invariants_at t p o out ho hout
  unfold allInvariants at hi
  unfold unobservedResultPoisons
  cases hg : growPostState t p o out with
  | none => rfl
  | some s =>
      rw [hg] at hi
      simp only [Bool.and_eq_true] at hi
      exact hi.2

/-- An unobserved result concludes nothing, whatever else was measured. The
not-attempted arm is excluded because it issues no call and so has no result to
observe. -/
theorem an_unobserved_result_poisons_every_attempt
    (t : Target) (p : Premises) (o : Observation) (out : Outcome) (s : PostState)
    (ho : o ∈ allObservations) (hout : out ∈ allOutcomes)
    (h : growPostState t p o out = some s) (hr : p.resultObserved = false)
    (hne : out ≠ Outcome.notAttempted) : s.isPoisoned = true := by
  have hall : unobservedResultPoisons t p o out = true :=
    unobserved_result_poisons_holds t p o out ho hout
  unfold unobservedResultPoisons at hall
  rw [h] at hall
  have hbeq : (out == Outcome.notAttempted) = false := by
    simpa using hne
  simp [hr, hbeq] at hall
  exact hall

/-! ## The named arms -/

private def seed : Observation := ⟨.ftruncate, 4, 2, 4, 2, some 2⟩

/-- The same call, refused: the length was re-read and had not moved. -/
private def refusedSeed : Observation := ⟨.ftruncate, 4, 2, 2, 2, some 2⟩

private def observed : Premises := ⟨true, true, true, true, true, true⟩

/-- A length-only grant moves the length, reserves nothing, keeps the store
duty and requires a remap before the added range is used. -/
theorem a_length_only_grant_reserves_nothing :
    growPostState .linux observed seed .granted =
      some ⟨.grownTo 4, .notReserved, .remapBeforeUse 4,
            ⟨true, true, true, false, true, false⟩, .none, 0⟩ := by
  decide

/-- An allocating grant reserves the range it covers, and only then is the
store duty discharged. -/
theorem an_allocating_grant_reserves_the_range :
    growPostState .linux ⟨true, false, true, true, true, true⟩
        ⟨.posixFallocate, 4, 2, 4, 4, some 2⟩ .granted =
      some ⟨.grownTo 4, .reserved 4, .remapBeforeUse 4,
            ⟨true, false, true, false, true, false⟩, .none, 0⟩ := by
  decide

/-- An allocating grant the backing does not cover is not a grant. -/
theorem an_allocating_grant_without_the_backing_is_poisoned :
    growPostState .linux ⟨true, false, true, true, true, true⟩
        ⟨.posixFallocate, 4, 2, 4, 2, some 2⟩ .granted = some poisonedState := by
  decide

/-- A capacity refusal leaves the length where it was, retains the mapping and
defers the retry until the capacity changes. -/
theorem a_capacity_refusal_keeps_the_length_and_defers_the_retry :
    ∀ e ∈ [Errno.enospc, Errno.edquot, Errno.efbig],
      growPostState .linux observed refusedSeed (.refused e) =
        some ⟨.unchanged 2, .notReserved, .retainExisting,
              ⟨false, true, true, false, true, false⟩, .retryAfterCapacityChange, 0⟩ := by
  decide

/-- `ftruncate(2)` documents no length change on failure, so a failure that
reports one contradicts the observation. -/
theorem a_length_only_refusal_may_not_have_grown_the_file :
    growPostState .linux observed ⟨.ftruncate, 4, 2, 3, 2, some 2⟩ (.refused .efbig) =
      some poisonedState := by
  decide

/-- The allocating calls leave a partial effect unspecified, so a length change
on failure is admitted with unknown backing rather than as a grant. -/
theorem an_allocating_refusal_may_have_changed_the_length :
    growPostState .linux observed ⟨.fallocate, 4, 2, 3, 2, some 2⟩ (.refused .enospc) =
      some ⟨.grownTo 3, .unknown, .retainExisting,
            ⟨false, true, true, true, true, false⟩, .retryAfterCapacityChange, 0⟩ := by
  decide

/-- `posix_fallocate(3)` does not set `errno`: reading it there is a premise
violation, and leaving it alone decides the arm. -/
theorem reading_errno_from_the_returning_call_is_a_premise_violation :
    growPostState .linux observed ⟨.posixFallocate, 4, 2, 2, 2, some 2⟩ (.refused .efbig) =
        some poisonedState ∧
    growPostState .linux ⟨true, false, true, true, true, true⟩
        ⟨.posixFallocate, 4, 2, 2, 2, some 2⟩ (.refused .efbig) =
      some ⟨.unchanged 2, .unknown, .retainExisting,
            ⟨false, true, true, true, true, false⟩, .retryAfterCapacityChange, 0⟩ := by
  decide

/-- A transfer advances the content by exactly what moved, and the next attempt
is re-derived from the observed length rather than re-issued whole. -/
theorem a_partial_transfer_advances_by_exactly_what_moved :
    growPostState .linux observed ⟨.writeCall, 4, 2, 3, 3, some 2⟩ (.partialTransfer 1) =
      some ⟨.grownTo 3, .reserved 3, .remapBeforeUse 3,
            ⟨true, true, false, true, true, false⟩, .retryFromObservedLength, 1⟩ := by
  decide

/-- Only a transferring call can transfer partially. -/
theorem a_partial_outcome_from_a_resizing_call_is_poisoned :
    growPostState .linux observed ⟨.ftruncate, 4, 2, 3, 2, some 2⟩ (.partialTransfer 1) =
      some poisonedState := by
  decide

/-- The boundary signal left at its default disposition terminates the process,
so no post-state is observable. -/
theorem an_unhandled_boundary_signal_poisons_the_transfer :
    growPostState .linux ⟨true, true, true, true, true, false⟩
        ⟨.writeCall, 4, 2, 3, 3, some 2⟩ (.partialTransfer 1) = some poisonedState := by
  decide

/-- A request that does not grow the file, and a mapping of length zero, are
outside the declared domain rather than answered. -/
theorem the_domain_is_declared :
    growPostState .linux observed ⟨.ftruncate, 2, 2, 2, 2, some 2⟩ .granted = none ∧
    growPostState .linux observed ⟨.ftruncate, 4, 2, 4, 2, some 0⟩ .granted = none := by
  decide

/-- A repeat of the identical request decides nothing new until the capacity
changes, and is refused outright after an undetermined outcome. -/
theorem the_retry_verdict_follows_the_recovery :
    (growPostState .linux observed refusedSeed (.refused .efbig)).map
        (fun s => (retryVerdict s false, retryVerdict s true)) =
      some (.sameClassExpected, .undetermined) ∧
    retryVerdict poisonedState true = .refused ∧
    (growPostState .linux observed ⟨.fallocate, 4, 2, 2, 2, some 2⟩ (.refused .eintr)).map
        (fun s => retryVerdict s false) = some .undetermined := by
  decide

/-! ## What a mapping witnesses -/

/-- A mapping witnesses nothing past the end of the file, and nothing inside a
hole, however long it is. -/
theorem a_mapping_witnesses_only_backed_bytes_inside_the_file :
    ∀ m ∈ [0, 2, 4, 8], ∀ f ∈ [0, 2, 4],
      (mappingWitness m f (.reserved 8)).usable ≤ f ∧
        (mappingWitness m f (.reserved 8)).usable ≤ m ∧
        (mappingWitness m f .notReserved).usable = 0 ∧
        (mappingWitness m f .unknown).usable = 0 := by
  decide

/-- A mapping twice the length of the file records the excess and witnesses
nothing there: this is the fact a forged "successful mapping" contradicts. -/
theorem a_mapping_beyond_the_end_witnesses_nothing_there :
    mappingWitness 4 2 (.reserved 2) = ⟨2, 2⟩ := by
  decide

/-! ## The recorded trace and its ledger

The reduced ledger of one recorded scenario: at most one mapping epoch and one
descriptor epoch are live, which is what the recorded scenarios do. -/

inductive Event where
  | openFd (gen len backed : Nat)
  | mapFile (gen len fileLen : Nat)
  | remapFile (src dst len fileLen : Nat)
  | grow (call : GrowCall) (request lenBefore lenAfter : Nat) (ok : Bool)
  | logical (len : Nat)
  | unmapFile (gen len : Nat)
  | closeFd (gen : Nat)
  | endTrace (liveMaps liveFds : Nat)
  deriving DecidableEq, Repr

structure Ledger where
  fileLen : Nat
  /-- Leading bytes of the file known to be backed. -/
  backed : Nat
  logical : Nat
  liveMap : Option (Nat × Nat)
  liveFd : Option Nat
  highestMap : Nat
  highestFd : Nat
  retiredMap : List Nat
  ended : Bool
  deriving DecidableEq, Repr

def Ledger.initial : Ledger := ⟨0, 0, 0, none, none, 0, 0, [], false⟩

/-- One step of the ledger. `none` is a refusal: a recorded trace is never
repaired and never partially accepted. -/
def step (l : Ledger) (e : Event) : Option Ledger :=
  if l.ended then none
  else
    match e with
    | .openFd gen len backed =>
        if l.liveFd.isSome || gen ≤ l.highestFd then none
        else some { l with fileLen := len, backed := min backed len,
                           liveFd := some gen, highestFd := gen }
    | .mapFile gen len fileLen =>
        if l.liveMap.isSome || gen ≤ l.highestMap || l.retiredMap.contains gen ||
            fileLen ≠ l.fileLen then none
        else some { l with liveMap := some (gen, len), highestMap := gen }
    | .remapFile src dst len fileLen =>
        match l.liveMap with
        | none => none
        | some (live, _) =>
            if live ≠ src || dst ≤ l.highestMap || l.retiredMap.contains dst ||
                fileLen ≠ l.fileLen then none
            else some { l with liveMap := some (dst, len), highestMap := dst,
                               retiredMap := src :: l.retiredMap }
    | .grow call request lenBefore lenAfter ok =>
        if lenBefore ≠ l.fileLen || request ≤ lenBefore then none
        else if ok then
          if lenAfter < request then none
          else some { l with fileLen := lenAfter,
                             backed := if call.reservesBacking then max l.backed lenAfter
                                       else l.backed }
        else
          if lenAfter ≠ lenBefore && call.failureMayChangeLength = false then none
          else some { l with fileLen := lenAfter }
    | .logical len =>
        if min l.fileLen l.backed < len then none else some { l with logical := len }
    | .unmapFile gen len =>
        match l.liveMap with
        | none => none
        | some (live, mapped) =>
            if live ≠ gen || mapped ≠ len then none
            else some { l with liveMap := none, retiredMap := gen :: l.retiredMap }
    | .closeFd gen =>
        if l.liveFd ≠ some gen then none else some { l with liveFd := none }
    | .endTrace liveMaps liveFds =>
        if liveMaps ≠ (if l.liveMap.isSome then 1 else 0) ||
            liveFds ≠ (if l.liveFd.isSome then 1 else 0) then none
        else some { l with ended := true }

/-- Run a whole recorded trace. -/
def run (events : List Event) : Option Ledger :=
  events.foldl (fun state e => state.bind (fun l => step l e)) (some Ledger.initial)

/-- The recorded length-only scenario, in the model's units: a 2-byte backed
file grown to 4 bytes by a call that reserves nothing, remapped, and torn down
completely. -/
def lengthOnlyTrace : List Event :=
  [ .openFd 1 2 2,
    .mapFile 1 2 2,
    .grow .ftruncate 4 2 4 true,
    .remapFile 1 2 4 4,
    .logical 2,
    .unmapFile 2 4,
    .closeFd 1,
    .endTrace 0 0 ]

/-- The recorded trace is accepted and leaves nothing live. Non-vacuity for
every refusal below. -/
theorem the_recorded_trace_is_accepted :
    run lengthOnlyTrace = some ⟨4, 2, 2, none, none, 2, 1, [2, 1], true⟩ := by
  decide

/-- A length-only grant never raises the backed prefix: the file is twice as
long and not one byte more is backed. -/
theorem a_length_only_grant_never_raises_the_backed_prefix :
    (run lengthOnlyTrace).map (fun l => (l.fileLen, l.backed)) = some (4, 2) := by
  decide

/-- Content is bounded by the backed prefix, not by the length: the same trace
with the content claimed into the hole is refused. -/
theorem content_claimed_into_the_hole_is_refused :
    run [.openFd 1 2 2, .mapFile 1 2 2, .grow .ftruncate 4 2 4 true,
         .remapFile 1 2 4 4, .logical 4] = none := by
  decide

/-! ## The mutation matrix

Each entry is the recorded trace with one thing changed. Every one is refused.
The three families the card names are the first three groups. -/

/-- Lost residuals: a release deleted, and a residual misdeclared. -/
def lostResidualMutations : List (List Event) :=
  [ [.openFd 1 2 2, .mapFile 1 2 2, .grow .ftruncate 4 2 4 true, .remapFile 1 2 4 4,
     .logical 2, .closeFd 1, .endTrace 0 0],
    [.openFd 1 2 2, .mapFile 1 2 2, .grow .ftruncate 4 2 4 true, .remapFile 1 2 4 4,
     .logical 2, .unmapFile 2 4, .endTrace 0 0],
    [.openFd 1 2 2, .mapFile 1 2 2, .grow .ftruncate 4 2 4 true, .remapFile 1 2 4 4,
     .logical 2, .unmapFile 2 4, .closeFd 1, .endTrace 1 0] ]

/-- Forged successful mappings: a mapping over a length the file never reached,
a remap recording a forged length, a release at another length, and content
claimed inside the hole. -/
def forgedMappingMutations : List (List Event) :=
  [ [.openFd 1 2 2, .mapFile 1 2 4, .endTrace 1 1],
    [.openFd 1 2 2, .mapFile 1 2 2, .grow .ftruncate 4 2 4 true, .remapFile 1 2 4 2,
     .endTrace 1 1],
    [.openFd 1 2 2, .mapFile 1 2 2, .grow .ftruncate 4 2 4 true, .remapFile 1 2 4 4,
     .unmapFile 2 2, .endTrace 0 1],
    [.openFd 1 2 2, .mapFile 1 2 2, .grow .ftruncate 4 2 4 true, .remapFile 1 2 4 4,
     .logical 4, .endTrace 1 1] ]

/-- Confused generations: a released generation acquired again, a generation
that goes backwards, a remap from an epoch that is not live, and a release of
one. -/
def confusedGenerationMutations : List (List Event) :=
  [ [.openFd 1 2 2, .mapFile 1 2 2, .unmapFile 1 2, .mapFile 1 2 2, .endTrace 1 1],
    [.openFd 1 2 2, .mapFile 2 2 2, .grow .ftruncate 4 2 4 true, .remapFile 2 1 4 4,
     .endTrace 1 1],
    [.openFd 1 2 2, .mapFile 1 2 2, .grow .ftruncate 4 2 4 true, .remapFile 7 2 4 4,
     .endTrace 1 1],
    [.openFd 1 2 2, .mapFile 1 2 2, .unmapFile 2 2, .endTrace 0 1] ]

/-- Forged grants: a refusal that grew the file anyway, a grant that did not
reach the request, and a call recorded from a length the file never had. -/
def forgedGrantMutations : List (List Event) :=
  [ [.openFd 1 2 2, .mapFile 1 2 2, .grow .ftruncate 4 2 4 false, .endTrace 1 1],
    [.openFd 1 2 2, .mapFile 1 2 2, .grow .ftruncate 4 2 3 true, .endTrace 1 1],
    [.openFd 1 2 2, .mapFile 1 2 2, .grow .ftruncate 4 3 4 true, .endTrace 1 1] ]

def allMutations : List (List Event) :=
  lostResidualMutations ++ forgedMappingMutations ++ confusedGenerationMutations ++
    forgedGrantMutations

/-- **Acceptance item 2, in the model.** Every mutation of the recorded trace
is refused: lost residuals, forged successful mappings, confused generations
and forged grants. -/
theorem every_mutation_is_refused : allMutations.all (fun t => (run t).isNone) = true := by
  decide

/-- The matrix is not vacuous: fourteen distinct mutated traces, none of them
the recorded one. -/
theorem the_mutation_matrix_is_not_vacuous :
    allMutations.length = 14 ∧ allMutations.all (fun t => t ≠ lengthOnlyTrace) = true := by
  decide

/-- A trace that stops before its teardown record has not ended, so its
residuals are undeclared. -/
theorem a_trace_without_a_teardown_record_is_incomplete :
    (run [Event.openFd 1 2 2, .mapFile 1 2 2]).map Ledger.ended = some false := by
  decide

/-- An event after the teardown record is refused. -/
theorem an_event_after_the_teardown_record_is_refused :
    run (lengthOnlyTrace ++ [Event.closeFd 1]) = none := by
  decide

end QCore.CapacityBoundary
