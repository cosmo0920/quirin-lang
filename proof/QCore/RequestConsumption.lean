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

/-! Card `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`: the verifier host
decides a process provider's successful exit from whether the provider consumed
its whole request, never from whether the host's request write or the
provider's exit came first.

This models `request_consumption::classify` and the decision branch of
`ProcessProvider::run_process` in `crates/qcore-verifier-host/src/` (the
branch taken once the direct child has exited and both output pipes are at end
of file). A *run* is one schedule of one execution: how many request bytes the
host managed to deliver into the pipe before the decision (`delivered`),
whether a write met a broken pipe (`readerLost`), and how many bytes the
provider read (`consumed`). The facts a pipe guarantees are hypotheses of the
structure, not axioms: a reader takes only what was written, the host writes
at most the frame, a write breaks only while part of the frame is still
unsent, and a broken pipe has no reader left.

Card `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT` adds the request pipe's other
holders. A descendant that inherited the pipe, with both outputs closed, can
still read after the direct child's exit, so the host decides only once it is
the pipe's last holder, waiting until the deadline at most. A run now also says
whether another process held the pipe when the host decided (`held`); the
observation carries the holders (`Holders`); and the last section models the
wait over time: once the last other holder is gone nobody can read, so the
outcome is the final consumption's, whenever the host looks. All of that is the
**Linux** host. Off Linux the holders cannot be observed and the host decides
as before the card (`Holders.countOnly`); the last section states what that
decision is and the timing it keeps for a descendant that reads late.

What is **not** modelled, and stays a Rust and OS boundary: that `FIONREAD`
reports `delivered - consumed`, and that `POLLERR` on a write end opened
through `/proc/self/fd` reports exactly that no other reader is left (both
measured in `crates/qcore-verifier-host/src/request_consumption.rs`'s unit
tests and the cards' reports), and that the Rust function transcribes `decide`
(pinned by the same file's `classify` tests). -/

namespace QCore.RequestConsumption

/-- One schedule of one execution, as far as the request pipe is concerned,
at the moment the host decides. `held` says whether a process other than the
host still held the pipe's read end then; a broken pipe has no reader, and
none can appear after it. -/
structure Run where
  length : Nat
  delivered : Nat
  consumed : Nat
  readerLost : Bool
  held : Bool
  consumed_le_delivered : consumed ≤ delivered
  delivered_le_length : delivered ≤ length
  lost_only_short : readerLost = true → delivered < length
  lost_unheld : readerLost = true → held = false

/-- Who else holds the request pipe at the decision: nobody, with the bytes no
reader took (`released`); another process (`held`); or it could not be
established (`unobservable`). -/
inductive Holders where
  | released (unread : Nat)
  | held
  | unobservable
  /-- Off Linux only: the other holders cannot be observed, so the count at
  the decision is decided on as it stands, as before card
  `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT` (the user's decision, "keep
  previous macOS"). -/
  | countOnly (unread : Nat)
  deriving DecidableEq, Repr

/-- What the host observes at the decision point. -/
structure Observation where
  length : Nat
  delivered : Nat
  readerLost : Bool
  holders : Holders
  deriving DecidableEq, Repr

/-- The observation of a run whose probe works: with no other holder, the
bytes left in the pipe are the delivered bytes no reader took. -/
def observe (r : Run) : Observation :=
  ⟨r.length, r.delivered, r.readerLost,
    if r.held then .held else .released (r.delivered - r.consumed)⟩

/-- The outcomes of the decision branch. `decoded` is the only one that goes
on to decode the response (and may still fail there as a protocol error); the
others refuse. -/
inductive Outcome where
  | terminated
  | requestUnconsumed
  | heldByDescendant
  | unobservable
  | decoded
  deriving DecidableEq, Repr

/-- The decision, for a direct child that exited (`exitSuccess` says how) with
both output pipes at end of file. An unsuccessful exit takes precedence, as it
did before either card; a broken pipe is final; a pipe another process holds
is never decided on its current count. -/
def decide (exitSuccess : Bool) (o : Observation) : Outcome :=
  if exitSuccess = false then .terminated
  else if o.readerLost = true then .requestUnconsumed
  else
    match o.holders with
    | .held => .heldByDescendant
    | .unobservable => .unobservable
    | .released unread | .countOnly unread =>
      if o.delivered < o.length then .requestUnconsumed
      else if unread = 0 then .decoded
      else .requestUnconsumed

/-- The decision before card `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`,
on the same branch: a successful exit was refused only after a broken pipe, and
otherwise decoded whatever the provider read (the branch was reached only once
the frame was wholly written or the pipe broke). -/
def legacyDecide (exitSuccess : Bool) (r : Run) : Outcome :=
  if exitSuccess = false then .terminated
  else if r.readerLost = true then .requestUnconsumed
  else .decoded

/-- **The outcome of a successful exit is a function of the holders at the
decision and the consumption alone.** Whatever the schedule, a pipe another
process holds is refused as held; otherwise the provider is decoded exactly
when it consumed the whole frame. -/
theorem decide_is_a_function_of_consumption_and_holders (r : Run) :
    decide true (observe r) =
      if r.held = true then .heldByDescendant
      else if r.consumed = r.length then .decoded else .requestUnconsumed := by
  have hc := r.consumed_le_delivered
  have hd := r.delivered_le_length
  have hl := r.lost_only_short
  have hu := r.lost_unheld
  unfold decide observe
  cases hlost : r.readerLost
  · cases hheld : r.held
    · by_cases hcl : r.consumed = r.length
      · have hdl : r.delivered = r.length := by omega
        simp [hcl, hdl]
      · by_cases hshort : r.delivered < r.length
        · simp [hcl, hshort]
        · obtain ⟨k, hk⟩ : ∃ k, r.delivered - r.consumed = k + 1 :=
            ⟨r.delivered - r.consumed - 1, by omega⟩
          simp [hcl, hshort, hk]
    · simp
  · have hf := hu hlost
    have hs := hl hlost
    have hcl : r.consumed ≠ r.length := by omega
    simp [hf, hcl]

/-- **With no other holder, the outcome of a successful exit is a function of
consumption alone**: decoded exactly when the provider consumed the whole
frame, and refused as an unconsumed request otherwise. -/
theorem decide_is_a_function_of_consumption (r : Run) (hfree : r.held = false) :
    decide true (observe r) =
      if r.consumed = r.length then .decoded else .requestUnconsumed := by
  rw [decide_is_a_function_of_consumption_and_holders]
  simp [hfree]

/-- **Two schedules with the same consumption and the same holders at the
decision get the same outcome**: the order of the host's write and the
provider's exit, and how much of the frame was delivered, do not matter. -/
theorem the_outcome_does_not_depend_on_the_schedule (r₁ r₂ : Run)
    (hlength : r₁.length = r₂.length) (hconsumed : r₁.consumed = r₂.consumed)
    (hheld : r₁.held = r₂.held) :
    decide true (observe r₁) = decide true (observe r₂) := by
  rw [decide_is_a_function_of_consumption_and_holders,
    decide_is_a_function_of_consumption_and_holders, hlength, hconsumed, hheld]

/-- **A request pipe another process still holds is never accepted**, whatever
the exit, the delivered count and the broken-pipe flag. -/
theorem a_held_request_is_never_decoded (exitSuccess : Bool) (o : Observation)
    (hheld : o.holders = .held) :
    decide exitSuccess o ≠ .decoded := by
  unfold decide
  cases exitSuccess <;> cases o.readerLost <;> simp [hheld]

/-- **A provider that did not consume its whole request is never accepted**,
whatever the host observed: with a working probe, with none, with the pipe
still held, after a broken pipe or not, and whatever its exit. -/
theorem a_non_consuming_provider_is_never_decoded (r : Run)
    (hshort : r.consumed < r.length) (exitSuccess : Bool) (holders : Holders)
    (hholders : holders = .unobservable ∨ holders = .held ∨
      holders = .released (r.delivered - r.consumed) ∨
      holders = .countOnly (r.delivered - r.consumed)) :
    decide exitSuccess ⟨r.length, r.delivered, r.readerLost, holders⟩ ≠ .decoded := by
  have hc := r.consumed_le_delivered
  unfold decide
  cases exitSuccess
  · simp
  · simp only [Bool.true_eq_false, ite_false]
    cases r.readerLost
    · simp only [Bool.false_eq_true, ite_false]
      rcases hholders with h | h | h | h
      · simp [h]
      · simp [h]
      · subst h
        by_cases hd : r.delivered < r.length
        · simp [hd]
        · obtain ⟨k, hk⟩ : ∃ k, r.delivered - r.consumed = k + 1 :=
            ⟨r.delivered - r.consumed - 1, by omega⟩
          simp [hd, hk]
      · subst h
        by_cases hd : r.delivered < r.length
        · simp [hd]
        · obtain ⟨k, hk⟩ : ∃ k, r.delivered - r.consumed = k + 1 :=
            ⟨r.delivered - r.consumed - 1, by omega⟩
          simp [hd, hk]
    · simp

/-- **A provider that consumed its whole request and exited successfully is
accepted once no other process holds the pipe**, including one that read
exactly its frame and exited before the host closed its end: consumption, not
end of file, is what counts. -/
theorem a_consuming_provider_is_decoded (r : Run) (hfull : r.consumed = r.length)
    (hfree : r.held = false) :
    decide true (observe r) = .decoded := by
  rw [decide_is_a_function_of_consumption r hfree]
  simp [hfull]

/-- **A broken pipe is the same refusal as unread bytes**: both branches of the
old race agree. -/
theorem a_broken_pipe_is_an_unconsumed_request (o : Observation)
    (hlost : o.readerLost = true) :
    decide true o = .requestUnconsumed := by
  simp [decide, hlost]

/-- An unsuccessful exit keeps its precedence over every request state. -/
theorem an_unsuccessful_exit_is_terminated (o : Observation) :
    decide false o = .terminated := by
  simp [decide]

/-- A consumption that cannot be observed is never read as consumed. -/
theorem an_unobservable_consumption_is_refused (length delivered : Nat) (lost : Bool) :
    decide true ⟨length, delivered, lost, .unobservable⟩ ≠ .decoded := by
  unfold decide
  cases lost <;> simp

/-- The provider that never reads, delivered whole before its exit. -/
def silentWrittenFirst : Run :=
  ⟨8, 8, 0, false, false, by decide, by decide, (by intro h; cases h), (by intro h; cases h)⟩

/-- The same provider, exiting before the host's first write. -/
def silentExitedFirst : Run :=
  ⟨8, 0, 0, true, false, by decide, by decide, (by intro _; decide), (by intro _; rfl)⟩

/-- **What card `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING` repaired.** Two
schedules of the same non-reading provider got different legacy outcomes —
accepted when the write came first, refused when the exit did — and get the
same refusal now. -/
theorem the_legacy_decision_depended_on_the_schedule :
    legacyDecide true silentWrittenFirst = .decoded
      ∧ legacyDecide true silentExitedFirst = .requestUnconsumed
      ∧ decide true (observe silentWrittenFirst) = .requestUnconsumed
      ∧ decide true (observe silentExitedFirst) = .requestUnconsumed := by
  refine ⟨rfl, rfl, ?_, ?_⟩ <;> decide

/-! ### The wait for the last holder

Instant `0` is the moment the direct child has exited successfully and both
outputs are at end of file, for a frame the host has wholly written. Another
process may still hold the request pipe; `release` is the first instant at
which none does. Reads happen only while someone holds the pipe, so the
consumption stops changing at `release`. -/

/-- One execution after the direct child's exit: the frame `length`, the
instant `release` its last other holder is gone, and the bytes read by each
instant. -/
structure Execution where
  length : Nat
  release : Nat
  consumedAt : Nat → Nat
  consumed_le : ∀ t, consumedAt t ≤ length
  settled : ∀ t, release ≤ t → consumedAt t = consumedAt release

/-- What a holder check at instant `t` observes. -/
def Execution.observeAt (x : Execution) (t : Nat) : Observation :=
  ⟨x.length, x.length, false,
    if t < x.release then .held else .released (x.length - x.consumedAt t)⟩

/-- The host's decision with the wait. It checks the holders after each
backoff; `check` is its first check at or after the release, wherever the
backoff put it. A check within the deadline decides on what it observes; if
that check would come only after the deadline, the execution is refused as
held. -/
def waitDecide (x : Execution) (deadline check : Nat) : Outcome :=
  if check ≤ deadline then decide true (x.observeAt check) else .heldByDescendant

/-- **The wait decides from the final consumption.** A check at or after the
release, within the deadline, decodes exactly when the whole frame was read by
the time the last other holder was gone. -/
theorem the_wait_decides_from_the_final_consumption (x : Execution)
    (deadline check : Nat) (hrelease : x.release ≤ check) (hdeadline : check ≤ deadline) :
    waitDecide x deadline check =
      if x.consumedAt x.release = x.length then .decoded else .requestUnconsumed := by
  have hs := x.settled check hrelease
  have hle := x.consumed_le check
  have hnot : ¬ check < x.release := by omega
  unfold waitDecide decide Execution.observeAt
  by_cases hfull : x.consumedAt x.release = x.length
  · have hz : x.length - x.consumedAt check = 0 := by omega
    simp [hdeadline, hnot, hfull, hz]
  · obtain ⟨k, hk⟩ : ∃ k, x.length - x.consumedAt check = k + 1 :=
      ⟨x.length - x.consumedAt check - 1, by omega⟩
    simp [hdeadline, hnot, hfull, hk]

/-- **When the host looks does not matter**: any two first checks after the
release, within the deadline, give the same outcome. -/
theorem the_wait_does_not_depend_on_when_the_host_checks (x : Execution)
    (deadline c₁ c₂ : Nat) (h₁ : x.release ≤ c₁) (h₁' : c₁ ≤ deadline)
    (h₂ : x.release ≤ c₂) (h₂' : c₂ ≤ deadline) :
    waitDecide x deadline c₁ = waitDecide x deadline c₂ := by
  rw [the_wait_decides_from_the_final_consumption x deadline c₁ h₁ h₁',
    the_wait_decides_from_the_final_consumption x deadline c₂ h₂ h₂']

/-- **A holder that outlives the deadline is refused as held**, never decoded:
the first check after its release comes after the deadline. -/
theorem a_holder_past_the_deadline_is_refused (x : Execution) (deadline check : Nat)
    (hlate : deadline < x.release) (hrelease : x.release ≤ check) :
    waitDecide x deadline check = .heldByDescendant := by
  have hpast : ¬ check ≤ deadline := by omega
  simp [waitDecide, hpast]

/-- **The wait decodes nothing while the pipe is held**: a decoded outcome
comes from a check at or after the release. -/
theorem the_wait_decodes_only_after_the_release (x : Execution) (deadline check : Nat)
    (hdecoded : waitDecide x deadline check = .decoded) : x.release ≤ check := by
  apply Nat.le_of_not_lt
  intro hlt
  unfold waitDecide at hdecoded
  split at hdecoded
  · simp [decide, Execution.observeAt, hlt] at hdecoded
  · cases hdecoded

/-- The decision without the wait, at instant `t`: on the holders seen then. -/
def immediateDecide (x : Execution) (t : Nat) : Outcome :=
  decide true (x.observeAt t)

/-- The decision of card `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING` at
instant `t`: the count then, whoever still held the pipe. -/
def countDecide (x : Execution) (t : Nat) : Outcome :=
  if x.consumedAt t = x.length then .decoded else .requestUnconsumed

/-- A descendant that reads the whole 8-byte frame and exits at instant 2. -/
def lateReader : Execution where
  length := 8
  release := 2
  consumedAt t := if t < 2 then 0 else 8
  consumed_le t := by split <;> omega
  settled t h := by simp [show ¬ t < 2 by omega]

/-- **Why the host waits.** For a descendant that reads the whole request and
exits, deciding at one instant depended on where that instant fell: the count
alone refused it before the read and accepted it after, and refusing whenever
the pipe is held still accepts or refuses it by the same timing. The wait
decodes it at every check after the release. -/
theorem an_immediate_decision_would_still_depend_on_the_schedule :
    countDecide lateReader 0 = .requestUnconsumed
      ∧ countDecide lateReader 2 = .decoded
      ∧ immediateDecide lateReader 0 = .heldByDescendant
      ∧ immediateDecide lateReader 2 = .decoded
      ∧ waitDecide lateReader 10 2 = .decoded
      ∧ waitDecide lateReader 10 7 = .decoded := by
  refine ⟨rfl, rfl, ?_, ?_, ?_, ?_⟩ <;> decide

/-! ### Off Linux

Everything above describes the Linux host: its observation of the holders is
`held` or `released`, and the wait theorems are about it. Off Linux the host
cannot tell its own read end from another process's, and by the user's decision
("keep previous macOS") it decides as before this card, on the count at the
direct child's exit (`countOnly`). That keeps the decision a function of the
count at that instant, which for a descendant that reads on its own schedule
is still timing. -/

/-- What the off-Linux host observes, for a wholly written frame, when it
decides at instant `t`. -/
def Execution.observeOffLinuxAt (x : Execution) (t : Nat) : Observation :=
  ⟨x.length, x.length, false, .countOnly (x.length - x.consumedAt t)⟩

/-- **Off Linux the decision is the count at the decision**: exactly the
pre-card decision `countDecide`, whoever still holds the pipe. -/
theorem off_linux_decides_on_the_count_at_the_decision (x : Execution) (t : Nat) :
    decide true (x.observeOffLinuxAt t) = countDecide x t := by
  have hle := x.consumed_le t
  unfold decide Execution.observeOffLinuxAt countDecide
  by_cases hfull : x.consumedAt t = x.length
  · simp [hfull]
  · obtain ⟨k, hk⟩ : ∃ k, x.length - x.consumedAt t = k + 1 :=
      ⟨x.length - x.consumedAt t - 1, by omega⟩
    simp [hfull, hk]

/-- **The residual off Linux.** For a descendant that reads its whole request
and exits at instant 2, the off-Linux host refuses it when it decides before
the read and accepts it after: the one shape that stays timing-dependent there
(`HOST-REQUEST-HOLDERS-OFF-LINUX-UNOBSERVABLE`). -/
theorem off_linux_a_late_reader_still_depends_on_the_schedule :
    decide true (lateReader.observeOffLinuxAt 0) = .requestUnconsumed
      ∧ decide true (lateReader.observeOffLinuxAt 2) = .decoded := by
  constructor <;> decide

end QCore.RequestConsumption
