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
provider read (`consumed`). The three facts a pipe guarantees are hypotheses
of the structure, not axioms: a reader takes only what was written, the host
writes at most the frame, and a write breaks only while part of the frame is
still unsent.

What is **not** modelled, and stays a Rust and OS boundary: that `FIONREAD` on
the probe reports `delivered - consumed` (measured in
`crates/qcore-verifier-host/src/request_consumption.rs`'s unit test and the
card's report), and that the Rust function transcribes `decide` (pinned by the
same file's `classify` tests). -/

namespace QCore.RequestConsumption

/-- One schedule of one execution, as far as the request pipe is concerned. -/
structure Run where
  length : Nat
  delivered : Nat
  consumed : Nat
  readerLost : Bool
  consumed_le_delivered : consumed ≤ delivered
  delivered_le_length : delivered ≤ length
  lost_only_short : readerLost = true → delivered < length

/-- What the host observes at the decision point. `unread` is the probe's
count, or `none` when no probe could be taken or read. -/
structure Observation where
  length : Nat
  delivered : Nat
  readerLost : Bool
  unread : Option Nat
  deriving DecidableEq, Repr

/-- The observation of a run whose probe works: the bytes left in the pipe are
the delivered bytes no reader took. -/
def observe (r : Run) : Observation :=
  ⟨r.length, r.delivered, r.readerLost, some (r.delivered - r.consumed)⟩

/-- The outcomes of the decision branch. `decoded` is the only one that goes
on to decode the response (and may still fail there as a protocol error); the
other three refuse. -/
inductive Outcome where
  | terminated
  | requestUnconsumed
  | unobservable
  | decoded
  deriving DecidableEq, Repr

/-- The decision, for a direct child that exited (`exitSuccess` says how) with
both output pipes at end of file. An unsuccessful exit takes precedence, as it
did before the card. -/
def decide (exitSuccess : Bool) (o : Observation) : Outcome :=
  if exitSuccess = false then .terminated
  else if o.readerLost = true then .requestUnconsumed
  else if o.delivered < o.length then .requestUnconsumed
  else
    match o.unread with
    | none => .unobservable
    | some 0 => .decoded
    | some (_ + 1) => .requestUnconsumed

/-- The decision before the card, on the same branch: a successful exit was
refused only after a broken pipe, and otherwise decoded whatever the provider
read (the branch was reached only once the frame was wholly written or the
pipe broke). -/
def legacyDecide (exitSuccess : Bool) (r : Run) : Outcome :=
  if exitSuccess = false then .terminated
  else if r.readerLost = true then .requestUnconsumed
  else .decoded

/-- **The outcome of a successful exit is a function of consumption alone.**
Whatever the schedule, the provider is decoded exactly when it consumed the
whole frame, and refused as an unconsumed request otherwise. -/
theorem decide_is_a_function_of_consumption (r : Run) :
    decide true (observe r) =
      if r.consumed = r.length then .decoded else .requestUnconsumed := by
  have hc := r.consumed_le_delivered
  have hd := r.delivered_le_length
  have hl := r.lost_only_short
  unfold decide observe
  by_cases hcl : r.consumed = r.length
  · have hdl : r.delivered = r.length := by omega
    have hnl : r.readerLost = false := by
      cases h : r.readerLost
      · rfl
      · have := hl h; omega
    have hz : r.delivered - r.consumed = 0 := by omega
    simp [hcl, hnl, hdl]
  · simp only [hcl, ite_false]
    cases hlost : r.readerLost
    · simp only [Bool.false_eq_true, ite_false]
      by_cases hshort : r.delivered < r.length
      · simp [hshort]
      · have hpos : r.delivered - r.consumed ≠ 0 := by omega
        simp only [hshort, ite_false, Bool.true_eq_false]
        obtain ⟨k, hk⟩ : ∃ k, r.delivered - r.consumed = k + 1 :=
          ⟨r.delivered - r.consumed - 1, by omega⟩
        simp [hk]
    · simp

/-- **Two schedules of the same consumption get the same outcome**: the order
of the host's write and the provider's exit, and how much of the frame was
delivered, no longer matter. -/
theorem the_outcome_does_not_depend_on_the_schedule (r₁ r₂ : Run)
    (hlength : r₁.length = r₂.length) (hconsumed : r₁.consumed = r₂.consumed) :
    decide true (observe r₁) = decide true (observe r₂) := by
  rw [decide_is_a_function_of_consumption, decide_is_a_function_of_consumption,
    hlength, hconsumed]

/-- **A provider that did not consume its whole request is never accepted**,
whatever the host observed: with a working probe, with none, after a broken
pipe or not, and whatever its exit. -/
theorem a_non_consuming_provider_is_never_decoded (r : Run)
    (hshort : r.consumed < r.length) (exitSuccess : Bool)
    (unread : Option Nat) (hunread : unread = none ∨ unread = some (r.delivered - r.consumed)) :
    decide exitSuccess ⟨r.length, r.delivered, r.readerLost, unread⟩ ≠ .decoded := by
  have hc := r.consumed_le_delivered
  unfold decide
  cases exitSuccess
  · simp
  · simp only [Bool.true_eq_false, ite_false]
    cases r.readerLost
    · simp only [Bool.false_eq_true, ite_false]
      by_cases hd : r.delivered < r.length
      · simp [hd]
      · simp only [hd, ite_false]
        rcases hunread with h | h
        · simp [h]
        · obtain ⟨k, hk⟩ : ∃ k, r.delivered - r.consumed = k + 1 :=
            ⟨r.delivered - r.consumed - 1, by omega⟩
          simp [h, hk]
    · simp

/-- **A provider that consumed its whole request and exited successfully is
accepted**, including one that read exactly its frame and exited before the
host closed its end: consumption, not end of file, is what counts. -/
theorem a_consuming_provider_is_decoded (r : Run) (hfull : r.consumed = r.length) :
    decide true (observe r) = .decoded := by
  rw [decide_is_a_function_of_consumption]
  simp [hfull]

/-- **A broken pipe is the same refusal as unread bytes**: both branches of the
old race now agree. -/
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
    decide true ⟨length, delivered, lost, none⟩ ≠ .decoded := by
  unfold decide
  cases lost <;> simp <;> split <;> simp

/-- The provider that never reads, delivered whole before its exit. -/
def silentWrittenFirst : Run :=
  ⟨8, 8, 0, false, by decide, by decide, by intro h; cases h⟩

/-- The same provider, exiting before the host's first write. -/
def silentExitedFirst : Run :=
  ⟨8, 0, 0, true, by decide, by decide, by intro _; decide⟩

/-- **What the card repaired.** Two schedules of the same non-reading provider
got different legacy outcomes — accepted when the write came first, refused
when the exit did — and get the same refusal now. -/
theorem the_legacy_decision_depended_on_the_schedule :
    legacyDecide true silentWrittenFirst = .decoded
      ∧ legacyDecide true silentExitedFirst = .requestUnconsumed
      ∧ decide true (observe silentWrittenFirst) = .requestUnconsumed
      ∧ decide true (observe silentExitedFirst) = .requestUnconsumed := by
  refine ⟨rfl, rfl, ?_, ?_⟩ <;> decide

end QCore.RequestConsumption
