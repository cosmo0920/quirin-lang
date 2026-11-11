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

import QCore.TaskTraceWrites

/-!
# TRACE-ALPHABET-LEASE-BOUNDARY — the other side of CT03's boundary

`QCore.TaskTraceWrites` is the transcription of the F02 relation over the
*observable* alphabet card `CT03-PROOF-ALPHABET` grew by the T05 Buffer write,
and its conservativity theorems are about that alphabet. This module is about
the **observation** alphabet on the other side of the same boundary — the
reference run's own `qcore_core::task_execution::TaskEvent` — and about the
function that crosses it, `qcore_core::task_trace_relation::reference_trace`.

Wave 31 integrated two cards that disagreed there. `WRITE-LEND-TASKS` added
three observation constructors (a Buffer write and two range-write-lease
events) and refused all three; `CT03-PROOF-ALPHABET` admitted the write and
deleted the error variant the refusal used. The integrator resolved it as a
union — write encoded, leases refused — and the residual
`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` recorded that the resolution was
*unproven and unexercised*: CT03's theorems say nothing about the lease side,
and no test drove a lease-carrying run into the function. This module is the
proof half of that discharge; the exercise half is
`compiler/tests/trace_alphabet_lease_boundary.rs`.

What is proved here, about `referenceTrace`:

* `succeeds_iff_no_event_is_refusable` — the crossing is decided by the events
  alone, and the decision is exact in both directions. A lease-carrying
  observation is **always** refused (no lease slips through on any shape of
  run), and a refusable-free one **always** encodes (the refusal is not
  incidentally blocking admitted traffic).
* `encoding_is_positionwise` with `encoding_preserves_length` — on success the
  output is, position for position, the per-event encoding of the input. This
  is the formal content of "refused rather than silently dropped": no event is
  elided, none is reordered, and none is invented.
* `refusal_points_at_a_refused_event` and `refusal_is_the_first` — the index the
  refusal carries is the offending event's own position, and it is the *first*
  such position. The Rust pin `index: 2` on the `WRITE-LEND-TASKS` flagship is
  an instance, not a coincidence.
* `no_lease_reaches_an_encoded_trace` — the negative statement on its own: an
  encoded trace's source contains no lease event, so no lease can reach
  `AdmittedCausalOrder` through this route.
* `the_flagship_run_is_refused_at_its_lease` and
  `the_lease_free_twin_is_an_admitted_causal_order` — non-vacuity in both
  directions on the card's real runs, transcribed from the pinned reference
  traces, in the manner of CT03's `a_stale_read_is_refused` /
  `a_rewritten_payload_is_admitted`.
* `the_dropped_encoding_would_have_been_admitted` — **why the refusal is the
  fail-closed answer, rather than a matter of taste.** Had the arm dropped the
  lease events and encoded the rest, the remainder is not merely a shorter
  trace: `TaskTraceWrites.declare` **admits** it, as an admitted causal order
  for a run in which two workers held outstanding write authority the order does
  not mention. Dropping is therefore a move in the *admitting* direction, which
  invariant 1 forbids. Refusing blocks a comparison; dropping would have
  licensed a false one.

What this module does **not** establish, stated so the boundary keeps its
shape: it says nothing about whether the observable alphabet *should* grow to
carry leases. That question belongs to the card that makes a worker exercise
its lease (`WRITE-LEND-TASKS-EXERCISE`); CT03's method — a conservativity
theorem plus a rule shown non-vacuous in both directions — is the only
precedent for taking it, and its second half is unavailable while no worker-side
range write exists to admit or refuse. Nor does it say anything about native
executions or about the Rust implementation's faithfulness beyond the
transcription.
-/

namespace QCore.TaskTraceLeaseBoundary

open TaskTraceRelation (Outcome)
open TaskTraceWrites (Event declare)

/-! ## The observation alphabet

The twin of `qcore_core::task_execution::TaskEvent` in the shape card
`WRITE-LEND-TASKS` left it: the eight pre-existing observation constructors,
the T05 Buffer write CT03 admits, and the two range-write-lease events. A
result's outcome is modelled with its own refusing case, because
`reference_trace`'s two refusals share one index space and a model with only
one of them would overstate the encoder's totality. -/

/-- The twin of `qcore_core::task_execution::TaskOutcome`'s renderability:
`errorWithBuffer` is the one shape `outcome()` refuses
(`UnsupportedOutcome`). -/
inductive RunOutcome where
  | encodable (outcome : Outcome)
  | errorWithBuffer (tag generation : Nat)
  deriving DecidableEq, BEq, Repr, Inhabited

inductive Observed where
  | bufferAllocated (task buffer initial : Nat)
  | bufferAllocationFailed (task : Nat)
  | bufferRead (task buffer value : Nat)
  | bufferWritten (task buffer value : Nat)
  | bufferDropped (task buffer : Nat)
  | spawn (task worker : Nat)
  | bufferLoaned (borrower buffer : Nat)
  | bufferReturned (borrower buffer : Nat)
  | bufferLeased (borrower buffer low high : Nat)
  | bufferLeaseReturned (borrower buffer : Nat)
  | createFailed (worker : Nat)
  | result (task : Nat) (outcome : RunOutcome)
  | join (task : Nat)
  deriving DecidableEq, BEq, Repr, Inhabited

/-- The twin of `ReferenceTraceError`: two refusals, each carrying the index of
the observation event that caused it. -/
inductive Refusal where
  | unsupportedOutcome (index : Nat)
  | unsupportedObservedEvent (index : Nat)
  deriving DecidableEq, BEq, Repr, Inhabited

def Refusal.index : Refusal → Nat
  | .unsupportedOutcome index => index
  | .unsupportedObservedEvent index => index

/-- A range write lease, opened or closed: the two constructors this boundary
refuses. -/
def isLease : Observed → Bool
  | .bufferLeased .. => true
  | .bufferLeaseReturned .. => true
  | _ => false

/-- Every shape `reference_trace` refuses: a lease event, or a result carrying
the outcome the observable alphabet has no rendering for. -/
def isRefusable : Observed → Bool
  | .bufferLeased .. => true
  | .bufferLeaseReturned .. => true
  | .result _ (.errorWithBuffer ..) => true
  | _ => false

theorem lease_is_refusable {e : Observed} (h : isLease e = true) : isRefusable e = true := by
  cases e <;> simp [isLease, isRefusable] at h ⊢

/-! ## The crossing

`encodeEventAt` is the body of `reference_trace`'s `map` closure, index and
all; `traceFrom` is the `enumerate().map(...).collect::<Result<_, _>>()` around
it, which stops at the first `Err` and produces nothing. -/

def encodeEventAt (index : Nat) : Observed → Except Refusal Event
  | .bufferAllocated task buffer initial => .ok (.allocated task buffer initial)
  | .bufferAllocationFailed task => .ok (.allocationFailed task)
  | .bufferRead task buffer value => .ok (.read task buffer value)
  | .bufferWritten task buffer value => .ok (.written task buffer value)
  | .bufferDropped task buffer => .ok (.dropped task buffer)
  | .spawn task worker => .ok (.spawned task worker)
  | .bufferLoaned borrower buffer => .ok (.loaned borrower buffer)
  | .bufferReturned borrower buffer => .ok (.returned borrower buffer)
  | .createFailed worker => .ok (.creationFailed worker)
  | .join task => .ok (.joined task)
  | .result task (.encodable outcome) => .ok (.completed task outcome)
  -- The two refusals, each at the offending event's own index.
  | .result _ (.errorWithBuffer ..) => .error (.unsupportedOutcome index)
  | .bufferLeased .. => .error (.unsupportedObservedEvent index)
  | .bufferLeaseReturned .. => .error (.unsupportedObservedEvent index)

def traceFrom (index : Nat) : List Observed → Except Refusal (List Event)
  | [] => .ok []
  | event :: rest =>
      match encodeEventAt index event with
      | .error refusal => .error refusal
      | .ok encoded =>
          match traceFrom (index + 1) rest with
          | .error refusal => .error refusal
          | .ok tail => .ok (encoded :: tail)

/-- `qcore_core::task_trace_relation::reference_trace`. -/
def referenceTrace (events : List Observed) : Except Refusal (List Event) :=
  traceFrom 0 events

/-! ## The event-level boundary

Two `rfl`-level facts, stated rather than left implicit: an event is refused
exactly when it is refusable, and a lease is always refused. -/

theorem encodeEventAt_ok_iff {index : Nat} {e : Observed} :
    (∃ encoded, encodeEventAt index e = .ok encoded) ↔ isRefusable e = false := by
  cases e with
  | result _ outcome => cases outcome <;> simp [encodeEventAt, isRefusable]
  | _ => simp [encodeEventAt, isRefusable]

theorem encodeEventAt_refuses_a_lease (index : Nat) {e : Observed} (h : isLease e = true) :
    encodeEventAt index e = .error (.unsupportedObservedEvent index) := by
  cases e <;> simp [isLease] at h ⊢ <;> rfl

/-! ## The crossing is decided by the events alone -/

/-- The exact characterisation, in both directions: a run crosses iff none of
its events is refusable. The forward direction says no lease ever slips
through; the backward direction says the refusal blocks nothing else. -/
theorem traceFrom_ok_iff (index : Nat) (events : List Observed) :
    (∃ encoded, traceFrom index events = .ok encoded) ↔
      events.all (fun e => !isRefusable e) = true := by
  induction events generalizing index with
  | nil => simp [traceFrom]
  | cons head rest ih =>
    constructor
    · rintro ⟨encoded, hok⟩
      cases hhead : encodeEventAt index head with
      | error refusal => simp [traceFrom, hhead] at hok
      | ok headEncoded =>
        cases htail : traceFrom (index + 1) rest with
        | error refusal => simp [traceFrom, hhead, htail] at hok
        | ok tail =>
          have hh : isRefusable head = false := encodeEventAt_ok_iff.mp ⟨headEncoded, hhead⟩
          have ht := (ih (index + 1)).mp ⟨tail, htail⟩
          simp [hh, ht]
    · intro hall
      simp only [List.all_cons, Bool.and_eq_true] at hall
      obtain ⟨hhead, htail⟩ := hall
      have hhead : isRefusable head = false := by
        simpa using hhead
      obtain ⟨headEncoded, hhe⟩ := (encodeEventAt_ok_iff (index := index)).mpr hhead
      obtain ⟨tail, htf⟩ := (ih (index + 1)).mpr htail
      exact ⟨headEncoded :: tail, by simp [traceFrom, hhe, htf]⟩

theorem succeeds_iff_no_event_is_refusable (events : List Observed) :
    (∃ encoded, referenceTrace events = .ok encoded) ↔
      events.all (fun e => !isRefusable e) = true :=
  traceFrom_ok_iff 0 events

/-- A lease-carrying observation is refused, whatever else the run contains.
This is the statement the unexercised arm was asserting without proof. -/
theorem a_lease_carrying_observation_is_refused {events : List Observed}
    (h : events.any isLease = true) : ∃ refusal, referenceTrace events = .error refusal := by
  have hnot : ¬ events.all (fun e => !isRefusable e) = true := by
    intro hall
    obtain ⟨e, hmem, hlease⟩ := List.any_eq_true.mp h
    have : isRefusable e = false := by
      simpa using (List.all_eq_true.mp hall) e hmem
    rw [lease_is_refusable hlease] at this
    exact Bool.noConfusion this
  cases h' : referenceTrace events with
  | error refusal => exact ⟨refusal, rfl⟩
  | ok encoded => exact absurd ((succeeds_iff_no_event_is_refusable events).mp ⟨encoded, h'⟩) hnot

/-- The negative half on its own: an encoded trace's source carries no lease,
so no lease event can reach an `AdmittedCausalOrder` by this route. -/
theorem no_lease_reaches_an_encoded_trace {events : List Observed} {encoded : List Event}
    (h : referenceTrace events = .ok encoded) : events.any isLease = false := by
  have hall := (succeeds_iff_no_event_is_refusable events).mp ⟨encoded, h⟩
  cases hany : events.any isLease with
  | false => rfl
  | true =>
    obtain ⟨refusal, hrefusal⟩ := a_lease_carrying_observation_is_refused hany
    rw [hrefusal] at h
    exact absurd h (by simp)

/-! ## Refused, not dropped

Length preservation and positionwise agreement together are the formal content
of the in-code claim "refused rather than encoded — never silently dropped":
nothing is elided, reordered or invented. -/

theorem traceFrom_preserves_length {index : Nat} :
    ∀ {events : List Observed} {encoded : List Event},
      traceFrom index events = .ok encoded → encoded.length = events.length := by
  intro events
  induction events generalizing index with
  | nil => intro encoded h; simp [traceFrom] at h; simp [← h]
  | cons head rest ih =>
    intro encoded h
    cases hhead : encodeEventAt index head with
    | error refusal => simp [traceFrom, hhead] at h
    | ok headEncoded =>
      cases htail : traceFrom (index + 1) rest with
      | error refusal => simp [traceFrom, hhead, htail] at h
      | ok tail =>
        simp only [traceFrom, hhead, htail] at h
        cases h
        simp [ih htail]

theorem encoding_preserves_length {events : List Observed} {encoded : List Event}
    (h : referenceTrace events = .ok encoded) : encoded.length = events.length :=
  traceFrom_preserves_length h

/-- Position for position, the output is the per-event encoding of the input at
the same position. -/
theorem traceFrom_positionwise {index : Nat} :
    ∀ {events : List Observed} {encoded : List Event},
      traceFrom index events = .ok encoded →
        ∀ i e f, events[i]? = some e → encoded[i]? = some f →
          encodeEventAt (index + i) e = .ok f := by
  intro events
  induction events generalizing index with
  | nil => intro encoded h i e f he _; simp at he
  | cons head rest ih =>
    intro encoded h i e f he hf
    cases hhead : encodeEventAt index head with
    | error refusal => simp [traceFrom, hhead] at h
    | ok headEncoded =>
      cases htail : traceFrom (index + 1) rest with
      | error refusal => simp [traceFrom, hhead, htail] at h
      | ok tail =>
        simp only [traceFrom, hhead, htail] at h
        cases h
        cases i with
        | zero =>
          simp at he hf
          subst he; subst hf
          simpa using hhead
        | succ j =>
          simp only [List.getElem?_cons_succ] at he hf
          have := ih htail j e f he hf
          have harith : index + 1 + j = index + (j + 1) := by omega
          rwa [harith] at this

theorem encoding_is_positionwise {events : List Observed} {encoded : List Event}
    (h : referenceTrace events = .ok encoded) :
    ∀ i e f, events[i]? = some e → encoded[i]? = some f → encodeEventAt i e = .ok f := by
  intro i e f he hf
  simpa using traceFrom_positionwise h i e f he hf

/-! ## The index is the first offender's own -/

/-- Whichever of the two refusals fires, the index it carries is the index the
encoder was called at — the offending event's own position. -/
theorem encodeEventAt_error_index {index : Nat} {e : Observed} {refusal : Refusal}
    (h : encodeEventAt index e = .error refusal) : refusal.index = index := by
  cases e with
  | result _ outcome =>
    cases outcome with
    | encodable _ => simp [encodeEventAt] at h
    | errorWithBuffer _ _ =>
      simp only [encodeEventAt, Except.error.injEq] at h
      subst h
      rfl
  | bufferLeased _ _ _ _ =>
    simp only [encodeEventAt, Except.error.injEq] at h
    subst h
    rfl
  | bufferLeaseReturned _ _ =>
    simp only [encodeEventAt, Except.error.injEq] at h
    subst h
    rfl
  | bufferAllocated _ _ _ => simp [encodeEventAt] at h
  | bufferAllocationFailed _ => simp [encodeEventAt] at h
  | bufferRead _ _ _ => simp [encodeEventAt] at h
  | bufferWritten _ _ _ => simp [encodeEventAt] at h
  | bufferDropped _ _ => simp [encodeEventAt] at h
  | spawn _ _ => simp [encodeEventAt] at h
  | bufferLoaned _ _ => simp [encodeEventAt] at h
  | bufferReturned _ _ => simp [encodeEventAt] at h
  | createFailed _ => simp [encodeEventAt] at h
  | join _ => simp [encodeEventAt] at h

theorem traceFrom_refusal_points_at_a_refused_event {index : Nat} :
    ∀ {events : List Observed} {refusal : Refusal},
      traceFrom index events = .error refusal →
        ∃ i e, events[i]? = some e ∧ refusal.index = index + i
          ∧ encodeEventAt (index + i) e = .error refusal := by
  intro events
  induction events generalizing index with
  | nil => intro refusal h; simp [traceFrom] at h
  | cons head rest ih =>
    intro refusal h
    cases hhead : encodeEventAt index head with
    | error headRefusal =>
      simp only [traceFrom, hhead] at h
      cases h
      exact ⟨0, head, by simp, by simpa using encodeEventAt_error_index hhead,
        by simpa using hhead⟩
    | ok headEncoded =>
      cases htail : traceFrom (index + 1) rest with
      | error tailRefusal =>
        simp only [traceFrom, hhead, htail] at h
        cases h
        obtain ⟨i, e, he, hidx, henc⟩ := ih htail
        refine ⟨i + 1, e, by simpa using he, by omega, ?_⟩
        have harith : index + (i + 1) = index + 1 + i := by omega
        rwa [harith]
      | ok tail => simp [traceFrom, hhead, htail] at h

theorem refusal_points_at_a_refused_event {events : List Observed} {refusal : Refusal}
    (h : referenceTrace events = .error refusal) :
    ∃ e, events[refusal.index]? = some e ∧ encodeEventAt refusal.index e = .error refusal := by
  obtain ⟨i, e, he, hidx, henc⟩ := traceFrom_refusal_points_at_a_refused_event h
  simp only [Nat.zero_add] at hidx henc
  exact ⟨e, by rw [hidx]; exact he, by rw [hidx]; exact henc⟩

/-- And it is the *first* such position: every earlier event encoded. -/
theorem traceFrom_refusal_is_the_first {index : Nat} :
    ∀ {events : List Observed} {refusal : Refusal},
      traceFrom index events = .error refusal →
        ∀ j e, events[j]? = some e → index + j < refusal.index → isRefusable e = false := by
  intro events
  induction events generalizing index with
  | nil => intro refusal h; simp [traceFrom] at h
  | cons head rest ih =>
    intro refusal h j e he hlt
    cases hhead : encodeEventAt index head with
    | error headRefusal =>
      simp only [traceFrom, hhead] at h
      cases h
      -- The refusal is this event's, so its index is `index`; no `j` is earlier.
      have hidx : refusal.index = index := encodeEventAt_error_index hhead
      exact absurd hlt (by omega)
    | ok headEncoded =>
      cases htail : traceFrom (index + 1) rest with
      | error tailRefusal =>
        simp only [traceFrom, hhead, htail] at h
        cases h
        cases j with
        | zero =>
          simp at he
          subst he
          exact encodeEventAt_ok_iff.mp ⟨headEncoded, hhead⟩
        | succ k =>
          simp only [List.getElem?_cons_succ] at he
          exact ih htail k e he (by omega)
      | ok tail => simp [traceFrom, hhead, htail] at h

theorem refusal_is_the_first {events : List Observed} {refusal : Refusal}
    (h : referenceTrace events = .error refusal) :
    ∀ j e, events[j]? = some e → j < refusal.index → isRefusable e = false := by
  intro j e he hlt
  exact traceFrom_refusal_is_the_first h j e he (by simpa using hlt)

/-! ## The card's real runs

Card `WRITE-LEND-TASKS`' admitted flagship, transcribed from the reference
trace `compiler/tests/write_lend_tasks.rs` pins: one allocation, two children
with disjoint write leases over the payload's two-byte halves, both live at
once, the owner's authority back after both joins. -/

def flagshipRun : List Observed :=
  [ .bufferAllocated 0 1 7
  , .spawn 1 0
  , .bufferLeased 1 1 0 2
  , .spawn 2 0
  , .bufferLeased 2 1 2 4
  , .result 1 (.encodable (.i32 1))
  , .join 1
  , .bufferLeaseReturned 1 1
  , .result 2 (.encodable (.i32 1))
  , .join 2
  , .bufferLeaseReturned 2 1
  , .bufferRead 0 1 7
  , .bufferDropped 0 1
  , .result 0 (.encodable (.i32 7)) ]

/-- The Rust pin `UnsupportedObservedEvent { index: 2 }`, proved. -/
theorem the_flagship_run_is_refused_at_its_lease :
    referenceTrace flagshipRun = .error (.unsupportedObservedEvent 2) := by
  rfl

/-- The read-share twin of the same program: the control that makes the refusal
attributable to the lease rather than to the program's spawn/join shape. -/
def shareRun : List Observed :=
  [ .bufferAllocated 0 1 7
  , .spawn 1 0
  , .bufferLoaned 1 1
  , .spawn 2 0
  , .bufferLoaned 2 1
  , .bufferRead 1 1 7
  , .result 1 (.encodable (.i32 7))
  , .join 1
  , .bufferReturned 1 1
  , .bufferRead 2 1 7
  , .result 2 (.encodable (.i32 7))
  , .join 2
  , .bufferReturned 2 1
  , .bufferRead 0 1 7
  , .bufferDropped 0 1
  , .result 0 (.encodable (.i32 7)) ]

/-- The control crosses, and what it crosses into is an admitted causal order of
the extended (CT03) relation — `declare = 0`. The boundary blocks the lease,
not the shape. -/
theorem the_lease_free_twin_is_an_admitted_causal_order :
    ∃ encoded, referenceTrace shareRun = .ok encoded ∧ declare encoded = 0 := by
  refine ⟨_, rfl, ?_⟩
  decide

/-! ## Why refusing, and not dropping

The measured reason the arm returns an error rather than filtering the lease
events out. `flagshipDropped` is `flagshipRun` with its two leases and two
lease returns removed — what a `filter` in place of the `return Err` would
produce. -/

def flagshipDropped : List Observed :=
  flagshipRun.filter (fun e => !isLease e)

/-- **The dropping variant is an admitting mistake, not a lossy one.** The
remainder is not merely a shorter trace: the relation *admits* it, as an
admitted causal order for a run in which two workers held outstanding write
authority over a payload the order never mentions. Refusing blocks a
comparison; dropping would have licensed a false one — invariant 1's
direction, measured rather than asserted. The Rust twin is
`compiler/tests/trace_alphabet_lease_boundary.rs::dropping_the_lease_events_would_have_produced_an_admitted_order`. -/
theorem the_dropped_encoding_would_have_been_admitted :
    ∃ encoded, referenceTrace flagshipDropped = .ok encoded
      ∧ encoded.length = 10 ∧ declare encoded = 0 := by
  refine ⟨_, rfl, rfl, ?_⟩
  decide

/-- And the two runs differ in exactly the four lease events, so the theorem
above is about *this* refusal and not about some other trace. -/
theorem the_dropped_run_loses_exactly_the_four_lease_events :
    flagshipRun.length = 14 ∧ flagshipDropped.length = 10
      ∧ (flagshipRun.filter isLease).length = 4 := by
  refine ⟨rfl, rfl, rfl⟩

end QCore.TaskTraceLeaseBoundary
