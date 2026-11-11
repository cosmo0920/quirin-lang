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

import QCore.OsEndpointPostState

namespace QCore.DescriptorCustody

/-! Generic formal boundary for socket-pair creation, descriptor close and
error queries. The executable checker supplies these branch and custody facts;
accepted evidence cannot turn a failed pair/close into a discharged resource.

**Card O03-U4 — the close clause is target-qualified.** The historical
`Close.Valid` carried a second disjunct, `succeeded = false → liveAfter = true`,
i.e. contract spec §C10's "returns on failure" policy asserted unconditionally
and without a target. `docs/design/OS_FAILURE_POSTSTATE_PROFILE.md` §13.1 records
why that is unfaithful on the selected target: Linux releases the descriptor on
every `close` failure except `EBADF`, so the old clause both **rejected a
truthful record** of a real failure and modelled the epoch as usable, which is
exactly what licenses the retry the platform documents as a hazard.

The repair is in two parts, and both directions are more conservative:

* the legacy four-field `Close` record keeps its shape and its **success** arm,
  and its failure arm is **withdrawn**: one `liveAfter : Bool` cannot express
  "the descriptor is gone **and** nothing was discharged", so no failed close is
  admitted through it at all;
* failed closes are represented by `TargetedClose` below, whose validity is
  decided by `QCore.OsEndpointPostState.closePostState` — the same relation
  `crates/qcore-core/src/os_endpoint.rs` decides — so the declared model can no
  longer disagree with it. -/

abbrev DescriptorId := Nat

structure SocketPair where
  succeeded : Bool
  outputPresent : Bool
  read : DescriptorId
  write : DescriptorId
  liveAfter : List DescriptorId
  deriving DecidableEq, Repr

def SocketPair.Valid (pair : SocketPair) : Prop :=
  (pair.succeeded = true ∧ pair.outputPresent = true ∧
    pair.read ≠ pair.write ∧ pair.liveAfter = [pair.read, pair.write]) ∨
  (pair.succeeded = false ∧ pair.outputPresent = false ∧
    pair.liveAfter = [])

instance (pair : SocketPair) : Decidable pair.Valid := by
  unfold SocketPair.Valid
  infer_instance

structure SocketPairCertificate where
  checked : SocketPair
  deriving DecidableEq, Repr

def checkSocketPair (pair : SocketPair) : Option SocketPairCertificate :=
  if pair.Valid then some { checked := pair } else none

private theorem checkSocketPair_valid
    {pair : SocketPair} {certificate : SocketPairCertificate}
    (result : checkSocketPair pair = some certificate) : pair.Valid := by
  unfold checkSocketPair at result
  split at result
  · assumption
  · contradiction

theorem accepted_successful_socketpair_creates_two_distinct_owned_descriptors
    {pair : SocketPair} {certificate : SocketPairCertificate}
    (result : checkSocketPair pair = some certificate)
    (succeeded : pair.succeeded = true) :
    pair.outputPresent = true ∧ pair.read ≠ pair.write ∧
      pair.liveAfter = [pair.read, pair.write] := by
  have valid := checkSocketPair_valid result
  rcases valid with valid | valid
  · exact valid.2
  · rw [succeeded] at valid
    cases valid.1

theorem accepted_failed_socketpair_creates_no_custody
    {pair : SocketPair} {certificate : SocketPairCertificate}
    (result : checkSocketPair pair = some certificate)
    (failed : pair.succeeded = false) :
    pair.outputPresent = false ∧ pair.liveAfter = [] := by
  have valid := checkSocketPair_valid result
  rcases valid with valid | valid
  · rw [failed] at valid
    cases valid.1
  · exact valid.2

/-- The historical close record: one success bit and one liveness bit. Its
success arm is faithful; its failure arm is not representable, because a single
`liveAfter : Bool` cannot say "released, and nothing discharged". -/
structure Close where
  liveBefore : Bool
  succeeded : Bool
  liveAfter : Bool
  closeRecorded : Bool
  deriving DecidableEq, Repr

/-- Only the success arm survives. It is exactly the relation's `zero` arm under
its sole-custody premise, which `the_legacy_success_arm_is_the_relation_under_
sole_custody` states explicitly rather than leaving implicit. -/
def Close.Valid (close : Close) : Prop :=
  close.liveBefore = true ∧ close.succeeded = true ∧
    close.liveAfter = false ∧ close.closeRecorded = true

instance (close : Close) : Decidable close.Valid := by
  unfold Close.Valid
  infer_instance

structure CloseCertificate where
  checked : Close
  deriving DecidableEq, Repr

def checkClose (close : Close) : Option CloseCertificate :=
  if close.Valid then some { checked := close } else none

private theorem checkClose_valid
    {close : Close} {certificate : CloseCertificate}
    (result : checkClose close = some certificate) : close.Valid := by
  unfold checkClose at result
  split at result
  · assumption
  · contradiction

theorem accepted_successful_close_discharges_once
    {close : Close} {certificate : CloseCertificate}
    (result : checkClose close = some certificate)
    (_succeeded : close.succeeded = true) :
    close.liveBefore = true ∧ close.liveAfter = false ∧
      close.closeRecorded = true := by
  have valid := checkClose_valid result
  exact ⟨valid.1, valid.2.2⟩

/-- **Historical name, corrected content (card O03-U4).**

*Old meaning:* every accepted record whose close failed had `liveAfter = true`
and `closeRecorded = false` — the descriptor still live, usable and retriable.
That is contract spec §C10's "returns on failure" policy, asserted for every
target at once, and it is **false on the selected target**: Linux releases the
descriptor on every failure except `EBADF`.

*New meaning:* the legacy record cannot represent a failed close at all, so no
accepted record is one. What a failed close leaves behind is decided by
`TargetedClose` against the target-qualified relation, where retention is
admitted only where the documentation actually states it
(`a_failed_close_retains_the_descriptor_only_on_posix2024_eintr`).

This is a refusal, not a vacuity: the hypotheses are contradictory because
`checkClose` rejects the record, and `the_returns_on_failure_record_is_refused`
exhibits the concrete record the old clause accepted. -/
theorem accepted_failed_close_retains_custody
    {close : Close} {certificate : CloseCertificate}
    (result : checkClose close = some certificate)
    (failed : close.succeeded = false) :
    False := by
  have succeeded := (checkClose_valid result).2.1
  rw [failed] at succeeded
  exact Bool.noConfusion succeeded

/-- The same fact under a name that says what it establishes. -/
theorem a_failed_legacy_close_record_is_refused
    {close : Close} {certificate : CloseCertificate}
    (result : checkClose close = some certificate)
    (failed : close.succeeded = false) :
    False :=
  accepted_failed_close_retains_custody result failed

/-- The concrete record the withdrawn clause accepted: a failed close claiming
the descriptor is still live and nothing was discharged. -/
theorem the_returns_on_failure_record_is_refused :
    checkClose ⟨true, false, true, false⟩ = none := by
  decide

structure ErrorQuery where
  knownClosed : Bool
  custodyPreserved : Bool
  deriving DecidableEq, Repr

def ErrorQuery.Valid (query : ErrorQuery) : Prop :=
  query.knownClosed = false ∧ query.custodyPreserved = true

instance (query : ErrorQuery) : Decidable query.Valid := by
  unfold ErrorQuery.Valid
  infer_instance

structure ErrorQueryCertificate where
  checked : ErrorQuery
  deriving DecidableEq, Repr

def checkErrorQuery (query : ErrorQuery) : Option ErrorQueryCertificate :=
  if query.Valid then some { checked := query } else none

private theorem checkErrorQuery_valid
    {query : ErrorQuery} {certificate : ErrorQueryCertificate}
    (result : checkErrorQuery query = some certificate) : query.Valid := by
  unfold checkErrorQuery at result
  split at result
  · assumption
  · contradiction

theorem accepted_error_query_is_borrowed
    {query : ErrorQuery} {certificate : ErrorQueryCertificate}
    (result : checkErrorQuery query = some certificate) :
    query.knownClosed = false ∧ query.custodyPreserved = true :=
  checkErrorQuery_valid result

structure Teardown where
  liveOwned : List DescriptorId
  closed : List DescriptorId
  deriving DecidableEq, Repr

def Teardown.Valid (teardown : Teardown) : Prop :=
  teardown.liveOwned = [] ∧ teardown.closed.Nodup

instance (teardown : Teardown) : Decidable teardown.Valid := by
  unfold Teardown.Valid
  infer_instance

structure TeardownCertificate where
  checked : Teardown
  deriving DecidableEq, Repr

def checkTeardown (teardown : Teardown) : Option TeardownCertificate :=
  if teardown.Valid then some { checked := teardown } else none

private theorem checkTeardown_valid
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) : teardown.Valid := by
  unfold checkTeardown at result
  split at result
  · assumption
  · contradiction

theorem accepted_descriptor_teardown_is_complete_and_unique
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) :
    teardown.liveOwned = [] ∧ teardown.closed.Nodup :=
  checkTeardown_valid result

/-- Historical theorem name retained for the Monkey adapter. -/
theorem accepted_monkey_descriptor_teardown_is_complete_and_unique
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) :
    teardown.liveOwned = [] ∧ teardown.closed.Nodup :=
  accepted_descriptor_teardown_is_complete_and_unique result

/-! ## The target-qualified close record (card O03-U4)

Everything below is appended, so the declaration order — and with it every
auto-generated instance name the compatibility namespace `QCore.MonkeyEvent`
re-exports — is unchanged above this point.

`TargetedClose` carries what the legacy record omitted: the target, the observed
outcome, the premises the relation consumes, the five-valued descriptor state,
the permitted recovery and the discharge claim, each **separately**. Its
validity is not an independent policy: it is decided against
`QCore.OsEndpointPostState.closePostState`, the relation
`crates/qcore-core/src/os_endpoint.rs` decides, so the declared model cannot
drift from it. -/

open QCore.OsEndpointPostState

/-- A submitted close record. Every claimed field is checked, never believed. -/
structure TargetedClose where
  target : Target
  premises : Premises
  outcome : Outcome
  /-- The producer asserts the epoch was live before the call (premise A2). -/
  liveBefore : Bool
  descriptorAfter : DescriptorState
  /-- The producer asserts a `close` on this number is permitted afterwards. -/
  mayRetryClose : Bool
  /-- The producer asserts the descriptor custody obligation ended. -/
  custodyDischarged : Bool
  /-- The producer asserts the data reached the disk. No arm of the relation
  ever derives this, so a record claiming it is always refused. -/
  dataDutyDischarged : Bool
  deriving DecidableEq, Repr

def TargetedClose.derived (close : TargetedClose) : Option PostState :=
  closePostState close.target close.premises close.outcome

/-- Custody is discharged **exactly** when the epoch is released. A poisoned or
retained epoch discharges nothing, and durability is never discharged. -/
def TargetedClose.valid (close : TargetedClose) : Bool :=
  close.liveBefore && close.premises.epochLive && !close.dataDutyDischarged &&
    (match close.derived with
     | none => false
     | some state =>
         (close.descriptorAfter == state.descriptor) &&
           (close.mayRetryClose == state.mayClose) &&
           (close.custodyDischarged == (state.descriptor == DescriptorState.released)))

def TargetedClose.Valid (close : TargetedClose) : Prop := close.valid = true

instance (close : TargetedClose) : Decidable close.Valid :=
  inferInstanceAs (Decidable (close.valid = true))

structure TargetedCloseCertificate where
  checked : TargetedClose
  deriving DecidableEq, Repr

def checkTargetedClose (close : TargetedClose) : Option TargetedCloseCertificate :=
  if close.Valid then some { checked := close } else none

private theorem checkTargetedClose_valid
    {close : TargetedClose} {certificate : TargetedCloseCertificate}
    (result : checkTargetedClose close = some certificate) : close.Valid := by
  unfold checkTargetedClose at result
  split at result
  · assumption
  · contradiction

private theorem valid_fields {close : TargetedClose} {state : PostState}
    (valid : close.Valid) (derived : close.derived = some state) :
    close.liveBefore = true ∧ close.premises.epochLive = true ∧
      close.dataDutyDischarged = false ∧ close.descriptorAfter = state.descriptor ∧
      close.mayRetryClose = state.mayClose ∧
      close.custodyDischarged = (state.descriptor == DescriptorState.released) := by
  have h : close.valid = true := valid
  unfold TargetedClose.valid at h
  rw [derived] at h
  simp only [Bool.and_eq_true, Bool.not_eq_true', beq_iff_eq] at h
  exact ⟨h.1.1.1, h.1.1.2, h.1.2, h.2.1.1, h.2.1.2, h.2.2⟩

/-- The legacy record's surviving arm is exactly the relation's `zero` arm under
its sole-custody premise, on every target. The premise was implicit before; it
is named here. -/
theorem the_legacy_success_arm_is_the_relation_under_sole_custody :
    ∀ t ∈ allTargets,
      (closePostState t ⟨true, true, true, true, true, true⟩ Outcome.zero).map
          PostState.descriptor = some DescriptorState.released ∧
        checkClose ⟨true, true, false, true⟩ = some { checked := ⟨true, true, false, true⟩ } := by
  decide

/-- An accepted record says exactly what the relation derives — nothing wider,
and nothing the relation did not conclude. -/
theorem accepted_close_agrees_with_the_endpoint_relation
    {close : TargetedClose} {certificate : TargetedCloseCertificate}
    {state : PostState}
    (result : checkTargetedClose close = some certificate)
    (derived : closePostState close.target close.premises close.outcome = some state) :
    close.descriptorAfter = state.descriptor ∧
      close.mayRetryClose = state.mayClose ∧
      close.custodyDischarged = (state.descriptor == DescriptorState.released) ∧
      close.dataDutyDischarged = false := by
  have fields := valid_fields (checkTargetedClose_valid result) derived
  exact ⟨fields.2.2.2.1, fields.2.2.2.2.1, fields.2.2.2.2.2, fields.2.2.1⟩

/-- No accepted record claims durability. The relation itself never derives it
either, over the whole domain, by
`QCore.OsEndpointPostState.no_outcome_ever_discharges_the_data_duty`. -/
theorem no_accepted_close_discharges_the_data_duty
    {close : TargetedClose} {certificate : TargetedCloseCertificate}
    {state : PostState}
    (result : checkTargetedClose close = some certificate)
    (derived : closePostState close.target close.premises close.outcome = some state) :
    close.dataDutyDischarged = false :=
  (valid_fields (checkTargetedClose_valid result) derived).2.2.1

/-- Discharge is exactly release. A retained or poisoned epoch still owes its
obligation, so an accepted record can never book one as closed. -/
theorem accepted_custody_is_discharged_exactly_when_the_epoch_is_released
    {close : TargetedClose} {certificate : TargetedCloseCertificate}
    {state : PostState}
    (result : checkTargetedClose close = some certificate)
    (derived : closePostState close.target close.premises close.outcome = some state)
    (discharged : close.custodyDischarged = true) :
    state.descriptor = DescriptorState.released := by
  have fields := valid_fields (checkTargetedClose_valid result) derived
  have := fields.2.2.2.2.2
  rw [discharged] at this
  exact beq_iff_eq.mp this.symm

/-! ### The corrected failure arms, over the whole domain -/

private def retainsOnlyOnPosix2024Eintr (t : Target) (p : Premises) (o : Outcome) : Bool :=
  match o, closePostState t p o with
  | Outcome.minusOne e, some state =>
      state.descriptor != DescriptorState.retained ||
        (t == Target.posix2024 && e == Errno.eintr)
  | _, _ => true

/-- **The corrected content of `accepted_failed_close_retains_custody`.** A
failed close retains the descriptor only where the documentation says it does —
POSIX.1-2024 `EINTR` — and nowhere else, on the whole `3 × 64 × 10` domain. -/
theorem a_failed_close_retains_the_descriptor_only_on_posix2024_eintr
    (t : Target) (p : Premises) (o : Outcome) :
    retainsOnlyOnPosix2024Eintr t p o = true :=
  of_domainAll (by decide) t p o

/-- On Linux, under the premises a release requires, every non-`EBADF` failure
releases the descriptor and permits no retry; whenever a premise is denied the
answer is poisoned instead, which also permits no retry. Both halves are
decided pointwise. -/
theorem a_failed_linux_close_releases_the_descriptor_and_permits_no_retry :
    ([Errno.eintr, Errno.eio, Errno.enospc, Errno.edquot, Errno.other].all fun e =>
      match closePostState Target.linux ⟨true, true, true, true, true, true⟩
          (Outcome.minusOne e) with
      | some state =>
          state.descriptor == DescriptorState.released && state.mayClose == false
      | none => false) = true := by
  decide

private def unpinnedFailureIsPoisoned (t : Target) (p : Premises) (o : Outcome) : Bool :=
  match t, o, closePostState t p o with
  | Target.unpinned, Outcome.minusOne _, some state =>
      state.descriptor.isPoisoned && state.mayClose == false &&
        state.duties.noRetryClose && state.duties.noUse &&
        state.duties.noTreatNumberAsFree && state.duties.keepInDenominator
  | _, _, _ => true

/-- With no target pinned — the case a trace vocabulary carrying neither target
nor `errno` is in — **every** failed close is poisoned, permits no recovery and
carries the whole conservative residual. This is the disposition
`qcore_core::event_reactor` now takes for a failed `CloseSocket`. -/
theorem an_unpinned_failed_close_is_poisoned_and_permits_no_recovery
    (t : Target) (p : Premises) (o : Outcome) :
    unpinnedFailureIsPoisoned t p o = true :=
  of_domainAll (by decide) t p o

/-! ### Refusals, and one admission so they are not vacuous -/

/-- The record the withdrawn legacy clause accepted, restated with a target: a
Linux `EIO` failure claiming a live, retriable descriptor. -/
theorem the_targeted_returns_on_failure_record_is_refused :
    checkTargetedClose
        ⟨Target.linux, granted, Outcome.minusOne Errno.eio, true,
          DescriptorState.retained, true, false, false⟩ = none := by
  decide

/-- The other direction of the same defect: the truthful record of that call,
which the legacy clause rejected outright, is admitted. -/
theorem the_truthful_linux_deferred_io_record_is_accepted :
    (checkTargetedClose
        ⟨Target.linux, granted, Outcome.minusOne Errno.eio, true,
          DescriptorState.released, false, true, false⟩).isSome = true := by
  decide

/-- A claimed retry after a failed Linux close is refused — the documented
hazard the old clause licensed. -/
theorem a_retry_claim_after_a_failed_linux_close_is_refused :
    ([Errno.eintr, Errno.eio, Errno.enospc, Errno.edquot, Errno.other, Errno.ebadf,
      Errno.unobserved].all fun e =>
        (checkTargetedClose
            ⟨Target.linux, granted, Outcome.minusOne e, true,
              (closePostState Target.linux granted (Outcome.minusOne e)).elim
                DescriptorState.attempted PostState.descriptor,
              true, false, false⟩).isNone) = true := by
  decide

/-- A record claiming durability is refused whatever else it says. -/
theorem a_durability_claim_is_refused :
    checkTargetedClose
        ⟨Target.linux, granted, Outcome.zero, true, DescriptorState.released,
          false, true, true⟩ = none := by
  decide

/-- A successful close identifies the epoch it closed only under sole custody;
without it the discharge claim is refused. -/
theorem a_successful_close_without_sole_custody_is_not_a_discharge :
    checkTargetedClose
        ⟨Target.linux, ⟨true, false, true, true, true, true⟩, Outcome.zero, true,
          DescriptorState.released, false, true, false⟩ = none := by
  decide

/-! ### Domain and transcription agreement with the Rust relation

`crates/qcore-core/tests/os_endpoint_correspondence.rs` enumerates the same
domain in this list order, encodes every derived post-state with the same
arithmetic, and asserts the same two numbers. A divergence in any field of any
answer, on either side, moves the digest. A digest is a comparison device, not a
proof device: the encoding is transcribed by hand on the Rust side, and neither
side is thereby shown faithful to the kernel. -/

def modulus : Nat := 18446744073709551616
def mixMul : Nat := 1000003
def stateSeed : Nat := 1469598103934665603
def domainSeed : Nat := 14695981039346656037
def domainMul : Nat := 1099511628211

def mix (accumulator value : Nat) : Nat :=
  (accumulator * mixMul + value + 1) % modulus

def encodeTarget : Target → Nat
  | .linux => 0
  | .posix2024 => 1
  | .unpinned => 2

def encodeErrno : Errno → Nat
  | .ebadf => 0
  | .eintr => 1
  | .eio => 2
  | .enospc => 3
  | .edquot => 4
  | .other => 5
  | .unobserved => 6

def encodeOutcome : Outcome → Nat
  | .notAttempted => 0
  | .resultUnobserved => 1
  | .zero => 2
  | .minusOne e => 3 + encodeErrno e

def bit (b : Bool) : Nat := if b then 1 else 0

def encodePremises (p : Premises) : Nat :=
  bit p.epochLive + 2 * bit p.soleCustodian + 4 * bit p.resultObserved +
    8 * bit p.errnoObserved + 16 * bit p.lastReference + 32 * bit p.noRecordLocks

def encodeInput (t : Target) (p : Premises) (o : Outcome) : Nat :=
  encodeTarget t + 3 * (encodePremises p + 64 * encodeOutcome o)

def encodeDescriptor : DescriptorState → Nat
  | .notAttempted => 0
  | .attempted => 1
  | .retained => 2
  | .released => 3
  | .indeterminate => 4

def encodeDescription : DescriptionState → Nat
  | .retained => 0
  | .released => 1
  | .unknown => 2

def encodeWriteBack : WriteBack → Nat
  | .failed => 0
  | .unknown => 1
  | .discharged => 2

def encodeDuties (d : Duties) : Nat :=
  bit d.releaseEpoch + 2 * bit d.noUse + 4 * bit d.noRetryClose +
    8 * bit d.noTreatNumberAsFree + 16 * bit d.keepInDenominator +
    32 * bit d.reportUnresolved + 64 * bit d.reportPossibleDataLoss

/-- `0` is the out-of-domain answer. -/
def encodePostState : Option PostState → Nat
  | none => 0
  | some s =>
      1 + encodeTarget s.target +
        8 * (encodeDescriptor s.descriptor +
          8 * (encodeDescription s.description +
            4 * (encodeWriteBack s.writeback + 4 * (encodeDuties s.duties + 128 * bit s.mayClose))))

/-- The relation's whole declared domain, in the model's own list order. -/
def closeDomain : List (Target × Premises × Outcome) :=
  allTargets.flatMap fun t =>
    allPremises.flatMap fun p =>
      allOutcomes.map fun o => (t, p, o)

def closeRelationDigest : Nat :=
  closeDomain.foldl (fun accumulator entry =>
    (accumulator * domainMul +
      mix (mix stateSeed (encodeInput entry.1 entry.2.1 entry.2.2))
        (encodePostState (closePostState entry.1 entry.2.1 entry.2.2)) + 1) % modulus)
    domainSeed

set_option maxRecDepth 100000 in
theorem close_relation_domain_size : closeDomain.length = 1920 := by decide

set_option maxRecDepth 100000 in
set_option maxHeartbeats 4000000 in
theorem close_relation_digest : closeRelationDigest = 11372109764820645253 := by decide

/-! ## The reactor-side arm (card O03-REACTOR)

`qcore_core::event_reactor`'s `CloseSocket` failure branch may now carry a
**declared** `(target, errno, record-locks)` premise. Nothing there observes a
platform: a declaring adapter supplies the three spellings as declared text,
they are resolved total-or-reject through closed vocabularies, and the reactor
then reads its disposition out of `closePostState` for the pinned target.

Modelled here so the declared model and the checker say the same thing:

* the three spellings are the closed alphabets plus one `unrecognized`
  constructor standing for **every** spelling outside them, which is faithful
  because the Rust resolver maps them all to the same refusal (pinned by
  `an_unrecognized_declared_spelling_refuses_the_event`);
* `reactorPremises` is the premise vector the trace vocabulary can justify:
  the epoch is live and solely custodied because this arm is reached only under
  exclusive socket-pair custody, the `-1` result is recorded by the trace, the
  errno class is named by the declaration, `lastReference` is **false** because
  the reactor models no open file description and claims no release of one, and
  A7 comes from the declaration's own spelling;
* `reactorDisposition` is not an independent policy: every arm reads
  `PostState.descriptor` and `PostState.mayClose`, so a change to the relation
  moves the reactor with it. -/

inductive DeclaredTarget where
  | linux
  | posix2024
  | unpinned
  /-- Every spelling outside the closed vocabulary. -/
  | unrecognized
  deriving DecidableEq, Repr

inductive DeclaredErrno where
  | ebadf
  | eintr
  | eio
  | enospc
  | edquot
  | other
  | unobserved
  | unrecognized
  deriving DecidableEq, Repr

inductive DeclaredLocks where
  | noneHeld
  /-- A7 declared unknown. Not a refusal: the relation has an answer for a
  denied A7, and it is the poisoned one. -/
  | unknownLocks
  | unrecognized
  deriving DecidableEq, Repr

def resolveDeclaredTarget : DeclaredTarget → Option Target
  | .linux => some .linux
  | .posix2024 => some .posix2024
  | .unpinned => some .unpinned
  | .unrecognized => none

def resolveDeclaredErrno : DeclaredErrno → Option Errno
  | .ebadf => some .ebadf
  | .eintr => some .eintr
  | .eio => some .eio
  | .enospc => some .enospc
  | .edquot => some .edquot
  | .other => some .other
  | .unobserved => some .unobserved
  | .unrecognized => none

def resolveDeclaredLocks : DeclaredLocks → Option Bool
  | .noneHeld => some true
  | .unknownLocks => some false
  | .unrecognized => none

/-- The premises the reactor supplies, with the declared A7 filled in. -/
def reactorPremises (noRecordLocks : Bool) : Premises :=
  ⟨true, true, true, true, false, noRecordLocks⟩

/-- What the relation's classification means for the reactor's descriptor
model. -/
inductive Disposition where
  /-- The declaration did not resolve; the event is refused. -/
  | refused
  /-- The epoch ended: custody is discharged and the number is retired. -/
  | releasedNumberRetired
  /-- The epoch is live and still owed; the documented retry is admitted. -/
  | retainedRetryPermitted
  /-- Neither usable, retriable nor free. -/
  | poisoned
  deriving DecidableEq, Repr

def dispositionOf (s : PostState) : Disposition :=
  match s.descriptor with
  | .released => .releasedNumberRetired
  | .retained => if s.mayClose then .retainedRetryPermitted else .poisoned
  | _ => .poisoned

def reactorDisposition (t : DeclaredTarget) (e : DeclaredErrno) (l : DeclaredLocks) :
    Disposition :=
  match resolveDeclaredTarget t, resolveDeclaredErrno e, resolveDeclaredLocks l with
  | some tt, some ee, some ll =>
      match closePostState tt (reactorPremises ll) (Outcome.minusOne ee) with
      | some s => dispositionOf s
      | none => .poisoned
  | _, _, _ => .refused

def allDeclaredTargets : List DeclaredTarget :=
  [.linux, .posix2024, .unpinned, .unrecognized]

def allDeclaredErrnos : List DeclaredErrno :=
  [.ebadf, .eintr, .eio, .enospc, .edquot, .other, .unobserved, .unrecognized]

def allDeclaredLocks : List DeclaredLocks :=
  [.noneHeld, .unknownLocks, .unrecognized]

/-- The whole declared `4 × 8 × 3` domain, in the order the Rust correspondence
test enumerates it. -/
def reactorDomainRows : List (DeclaredTarget × DeclaredErrno × DeclaredLocks) :=
  allDeclaredTargets.flatMap fun t =>
    allDeclaredErrnos.flatMap fun e =>
      allDeclaredLocks.map fun l => (t, e, l)

theorem mem_allDeclaredTargets (t : DeclaredTarget) : t ∈ allDeclaredTargets := by
  cases t <;> decide

theorem mem_allDeclaredErrnos (e : DeclaredErrno) : e ∈ allDeclaredErrnos := by
  cases e <;> decide

theorem mem_allDeclaredLocks (l : DeclaredLocks) : l ∈ allDeclaredLocks := by
  cases l <;> decide

def reactorAll (P : DeclaredTarget → DeclaredErrno → DeclaredLocks → Bool) : Bool :=
  allDeclaredTargets.all fun t =>
    allDeclaredErrnos.all fun e => allDeclaredLocks.all fun l => P t e l

/-- Bridge from a decided exhaustive check to a universally quantified fact. -/
theorem of_reactorAll {P : DeclaredTarget → DeclaredErrno → DeclaredLocks → Bool}
    (h : reactorAll P = true) (t : DeclaredTarget) (e : DeclaredErrno) (l : DeclaredLocks) :
    P t e l = true := by
  have ht := (List.all_eq_true.mp h) t (mem_allDeclaredTargets t)
  have he := (List.all_eq_true.mp ht) e (mem_allDeclaredErrnos e)
  exact (List.all_eq_true.mp he) l (mem_allDeclaredLocks l)

private def readsTheRelation (t : DeclaredTarget) (e : DeclaredErrno) (l : DeclaredLocks) :
    Bool :=
  match resolveDeclaredTarget t, resolveDeclaredErrno e, resolveDeclaredLocks l with
  | some tt, some ee, some ll =>
      match closePostState tt (reactorPremises ll) (Outcome.minusOne ee) with
      | some s => reactorDisposition t e l == dispositionOf s
      | none => reactorDisposition t e l == Disposition.poisoned
  | _, _, _ => reactorDisposition t e l == Disposition.refused

/-- The reactor decides nothing of its own: on every declared row its
disposition is the relation's own answer, read through `dispositionOf`. -/
theorem the_reactor_reads_its_disposition_out_of_the_relation
    (t : DeclaredTarget) (e : DeclaredErrno) (l : DeclaredLocks) :
    readsTheRelation t e l = true :=
  of_reactorAll (by decide) t e l

/-- Retention — and with it the only admitted retry — occurs on exactly one
declared row of the whole `4 × 8 × 3` domain. -/
theorem only_a_declared_posix2024_eintr_retains_the_epoch :
    (reactorDomainRows.filter fun row =>
        reactorDisposition row.1 row.2.1 row.2.2 == Disposition.retainedRetryPermitted) =
      [(DeclaredTarget.posix2024, DeclaredErrno.eintr, DeclaredLocks.noneHeld)] := by
  decide

/-- A declared Linux failure releases the epoch and retires the number on every
errno the kernel documents as releasing; `EBADF` refutes the epoch and an
unobserved errno excludes nothing, so both stay poisoned. -/
theorem a_declared_linux_failure_releases_except_on_ebadf_or_an_unobserved_errno :
    (allDeclaredErrnos.all fun e =>
      reactorDisposition .linux e .noneHeld ==
        (match e with
         | .ebadf | .unobserved => Disposition.poisoned
         | .unrecognized => Disposition.refused
         | _ => Disposition.releasedNumberRetired)) = true := by
  decide

private def unpinnedIsPoisoned (t : DeclaredTarget) (e : DeclaredErrno) (l : DeclaredLocks) :
    Bool :=
  match t, e, l with
  | .unpinned, .unrecognized, _ => true
  | .unpinned, _, .unrecognized => true
  | .unpinned, _, _ => reactorDisposition t e l == Disposition.poisoned
  | _, _, _ => true

/-- With `unpinned` declared — the answer an undeclared failure already gets —
every resolvable row is poisoned. Declaring nothing and declaring `unpinned`
are the same answer. -/
theorem a_declared_unpinned_failure_is_poisoned
    (t : DeclaredTarget) (e : DeclaredErrno) (l : DeclaredLocks) :
    unpinnedIsPoisoned t e l = true :=
  of_reactorAll (by decide) t e l

private def deniedLocksArePoisoned (t : DeclaredTarget) (e : DeclaredErrno)
    (l : DeclaredLocks) : Bool :=
  match t, e, l with
  | .unrecognized, _, _ => true
  | _, .unrecognized, _ => true
  | _, _, .unknownLocks => reactorDisposition t e l == Disposition.poisoned
  | _, _, _ => true

/-- A7 declared unknown poisons on every target and every errno: the effect
escapes all three modelled identities, so nothing is concluded. -/
theorem a_denied_record_lock_premise_is_poisoned_everywhere
    (t : DeclaredTarget) (e : DeclaredErrno) (l : DeclaredLocks) :
    deniedLocksArePoisoned t e l = true :=
  of_reactorAll (by decide) t e l

private def unrecognizedIsRefused (t : DeclaredTarget) (e : DeclaredErrno)
    (l : DeclaredLocks) : Bool :=
  if t == DeclaredTarget.unrecognized || e == DeclaredErrno.unrecognized ||
      l == DeclaredLocks.unrecognized then
    reactorDisposition t e l == Disposition.refused
  else
    reactorDisposition t e l != Disposition.refused

/-- An unrecognized spelling in any component refuses the event, and nothing
else does: the refusal never falls back to a target, an errno or a lock premise
the declaration did not name, and a recognized declaration is never refused. -/
theorem an_unrecognized_declaration_is_refused_and_nothing_else_is
    (t : DeclaredTarget) (e : DeclaredErrno) (l : DeclaredLocks) :
    unrecognizedIsRefused t e l = true :=
  of_reactorAll (by decide) t e l

private def releasedPermitsNoRetry (t : DeclaredTarget) (e : DeclaredErrno)
    (l : DeclaredLocks) : Bool :=
  match resolveDeclaredTarget t, resolveDeclaredErrno e, resolveDeclaredLocks l with
  | some tt, some ee, some ll =>
      match closePostState tt (reactorPremises ll) (Outcome.minusOne ee) with
      | some s =>
          (reactorDisposition t e l != Disposition.releasedNumberRetired || s.mayClose == false) &&
            (s.duties.reportPossibleDataLoss == false || s.writeback == WriteBack.failed)
      | none => true
  | _, _, _ => true

/-- A released declaration never permits a retry — the documented hazard — and
a release is never a durability claim: wherever the relation asks for the data
loss to be reported, the write-back is `failed`, and no arm ever discharges it
(`QCore.OsEndpointPostState.no_outcome_ever_discharges_the_data_duty`). -/
theorem a_released_declaration_never_permits_a_retry
    (t : DeclaredTarget) (e : DeclaredErrno) (l : DeclaredLocks) :
    releasedPermitsNoRetry t e l = true :=
  of_reactorAll (by decide) t e l

/-! ### The extended domain digest

`crates/qcore-core/tests/os_endpoint_correspondence.rs` enumerates these rows in
this list order, encodes each disposition with the same arithmetic, and
continues the relation's own fold from `closeRelationDigest`. The relation's
1920-row pin above is deliberately **kept**: the relation did not change, so its
value must not move, and the extended digest covers the reactor arm on top of
it. A digest is a comparison device, not a proof device. -/

/-- Disjoint from every relation input, so the two halves of the fold cannot
be confused. -/
def reactorDomainOffset : Nat := 4096

def encodeDeclaredTarget : DeclaredTarget → Nat
  | .linux => 0
  | .posix2024 => 1
  | .unpinned => 2
  | .unrecognized => 3

def encodeDeclaredErrno : DeclaredErrno → Nat
  | .ebadf => 0
  | .eintr => 1
  | .eio => 2
  | .enospc => 3
  | .edquot => 4
  | .other => 5
  | .unobserved => 6
  | .unrecognized => 7

def encodeDeclaredLocks : DeclaredLocks → Nat
  | .noneHeld => 0
  | .unknownLocks => 1
  | .unrecognized => 2

def encodeReactorInput (t : DeclaredTarget) (e : DeclaredErrno) (l : DeclaredLocks) : Nat :=
  reactorDomainOffset + encodeDeclaredTarget t +
    4 * (encodeDeclaredErrno e + 8 * encodeDeclaredLocks l)

/-- `0` stays reserved for the out-of-domain answer, as in `encodePostState`. -/
def encodeDisposition : Disposition → Nat
  | .refused => 1
  | .releasedNumberRetired => 2
  | .retainedRetryPermitted => 3
  | .poisoned => 4

def closeAdmissionDigest : Nat :=
  reactorDomainRows.foldl (fun accumulator entry =>
    (accumulator * domainMul +
      mix (mix stateSeed (encodeReactorInput entry.1 entry.2.1 entry.2.2))
        (encodeDisposition (reactorDisposition entry.1 entry.2.1 entry.2.2)) + 1) % modulus)
    closeRelationDigest

set_option maxRecDepth 100000 in
theorem close_admission_domain_size :
    closeDomain.length + reactorDomainRows.length = 2016 := by decide

set_option maxRecDepth 100000 in
theorem close_admission_digest : closeAdmissionDigest = 2640189996995932678 := by
  unfold closeAdmissionDigest
  rw [close_relation_digest]
  decide

end QCore.DescriptorCustody
