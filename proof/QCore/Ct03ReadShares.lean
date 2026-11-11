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
# CT03 — the shared-reads discharge, proved on the case's own trace terms

CT03's scenario is `Parent and child share reads with complementary positive
shares` (spec §55, semantic profile `ScopedReadSharesV1`, expected `Accept`).
This module gives that scenario a proved statement over the **extended** F02
observation alphabet of `QCore.TaskTraceWrites` — the third proof leg, and the
one that required growing the alphabet: CT03's reference run carries the T05
Buffer write (`qcore_buffer_write` at two sites), which the pinned alphabet
refused to encode. `referenceRun` below transcribes the reference execution of
`compiler/tests/fixtures/ct/CT03/case.qr` through the real pipeline: the
parent allocates one Buffer (initial payload `1`), writes `42` into it while
it holds exclusive authority, lends read shares to two children, all three
read `42` while the shares are out, joins both children and takes both shares
back, writes `7`, reads `7` back, releases exactly once and completes. The
named Rust runner
`compiler/tests/ct03_proof_leg.rs::ct03_artifact_is_the_lean_proved_shared_reads_shape`
checks, event for event, that the artifact's reference run is exactly this
term, recomputes `ct03_trace_digest` from it with wrapping `u64` arithmetic,
and runs the same total decision procedure this file defines.

What this file establishes:

* `dischargesSharedReads` — one **total decision procedure** for the
  scenario's property, and the meaning theorems that keep it honest: a trace
  it accepts has each borrower perform exactly its two scenario facts — read
  the shared generation, complete carrying the value it saw
  (`discharged_borrower_facts_are_exactly_the_scenario`); has the parent's two
  writes sit on either side of the lending window
  (`discharged_writes_sit_on_either_side_of_the_window`); has **each
  borrower's read sit strictly inside its own lending window** — after the
  loan, before the share returns — the scenario's cross-actor fact
  (`discharged_each_borrower_reads_inside_its_window`); has every read report
  the value the preceding write established, not the allocation's initial
  payload (`discharged_reads_report_the_written_values`); and releases the one
  shared generation exactly once, by the parent, after the restored payload is
  read back (`discharged_release_is_unique`,
  `discharged_release_follows_the_restored_read`).
* `ct03_reference_run_discharges_the_shared_reads` — CT03's own reference
  trace is accepted (`decide`, kernel-checked), and
  `ct03_reference_run_is_an_admitted_causal_order` — it is a declared admitted
  causal order of the extended relation.
* `every_admitted_rendering_discharges_the_shared_reads` — for **arbitrary**
  traces: any observation the extended F02 relation admits against CT03's
  reference run (`relate referenceRun observed = 0`) is accepted by the
  procedure. The per-actor clauses and the counts follow the CT01/CT37
  closures; the two window clauses are extracted from the relation's own loan
  and activity-after-join conjuncts (`loanFlag`, `activityAfterJoinFlag`).
* The named refusals — a read before its loan, a read after its share
  returned, a write inside the lending window (CT04's shape), a read that
  misses the restored payload, a missing release, a double release, and a
  borrower that writes are each refused, so the procedure is not vacuous.

What this file does **not** establish: that the emitted C's native execution
refines the reference execution (the native side is covered only by the
`native_heap_scenarios` harness's three fixed-schedule sanitized runs), that
the Rust pipeline is correct, or anything about programs outside this profile.
-/

namespace QCore.Ct03ReadShares
open QCore.TaskTraceWrites
open QCore.TaskTraceRelation (sortNats firstFailure)

/-! ## CT03's nineteen facts, named -/

def allocation : Event := .allocated 0 1 1
def prepared : Event := .written 0 1 42
def creationOne : Event := .spawned 1 0
def loanOne : Event := .loaned 1 1
def creationTwo : Event := .spawned 2 0
def loanTwo : Event := .loaned 2 1
def parentRead : Event := .read 0 1 42
def childOneRead : Event := .read 1 1 42
def childOneDone : Event := .completed 1 (.i32 42)
def collectionOne : Event := .joined 1
def returnOne : Event := .returned 1 1
def childTwoRead : Event := .read 2 1 42
def childTwoDone : Event := .completed 2 (.i32 42)
def collectionTwo : Event := .joined 2
def returnTwo : Event := .returned 2 1
def restored : Event := .written 0 1 7
def afterRead : Event := .read 0 1 7
def release : Event := .dropped 0 1
def rootResult : Event := .completed 0 (.i32 7)

/-- CT03's reference run: the checked task module's reference execution,
rendered by the production observer. The named Rust runner asserts the
artifact reproduces exactly this term. -/
def referenceRun : List Event :=
  [ allocation, prepared, creationOne, loanOne, creationTwo, loanTwo,
    parentRead, childOneRead, childOneDone, collectionOne, returnOne,
    childTwoRead, childTwoDone, collectionTwo, returnTwo, restored,
    afterRead, release, rootResult ]

/-- The parent's program order: allocate, write `42` under exclusive
authority, lend to each child as it is spawned, read while the shares are out,
join and take each share back, write `7` under reconstructed exclusive
authority, read it back, drop, complete. -/
def parentOrder : List Event :=
  [ allocation, prepared, creationOne, loanOne, creationTwo, loanTwo,
    parentRead, collectionOne, returnOne, collectionTwo, returnTwo, restored,
    afterRead, release, rootResult ]

/-- Each borrower's program order: read the shared generation, complete
carrying the value it saw. A borrower neither writes nor releases. -/
def childOneOrder : List Event := [childOneRead, childOneDone]
def childTwoOrder : List Event := [childTwoRead, childTwoDone]

theorem ct03_reference_run_is_an_admitted_causal_order :
    declare referenceRun = 0 := by decide

theorem parent_projection_of_the_reference :
    projection referenceRun 0 = parentOrder := by decide

theorem childOne_projection_of_the_reference :
    projection referenceRun 1 = childOneOrder := by decide

theorem childTwo_projection_of_the_reference :
    projection referenceRun 2 = childTwoOrder := by decide

/-- The transcription pin the Rust runner recomputes from CT03's own artifact
with wrapping `u64` arithmetic (the extended F02 digest fold, in which the
write carries the fresh code `11`). -/
theorem ct03_trace_digest : mixTrace referenceRun = 9638775376944872586 := by
  decide

/-! ## The decision procedure -/

/-- The cross-actor fact of this scenario, once per borrower: the borrower's
first shared read sits strictly inside its own lending window — after the
loan opens, before the share returns. Stated positionally over the loan
conjunct's own position lists, because no single actor's projection can carry
it. -/
def readSitsInsideTheWindow (child : Nat) (t : List Event) : Bool :=
  match (positions t (isLoanEventOf child 1)).head?,
        (positions t (isReadBy child 1)).head?,
        (positions t (isReturnEventOf child 1)).head? with
  | some loan, some readIndex, some returnIndex =>
      decide (loan < readIndex) && decide (readIndex < returnIndex)
  | _, _, _ => false

/-- Total decision procedure for CT03's scenario property over the extended
F02 alphabet. A trace is accepted exactly when it reports nineteen facts; the
parent's own facts are precisely `parentOrder` in that order — in particular
both writes are the parent's, one on each side of the lending window; each
borrower's own facts are precisely its read-then-complete; the shared
generation has exactly one allocation and exactly one release, each child
exactly one completion and one join; and each borrower's read sits strictly
inside its own lending window. The Rust transcription in
`compiler/tests/ct03_proof_leg.rs` runs this same procedure on the
artifact. -/
def dischargesSharedReads (t : List Event) : Bool :=
  decide (t.length = 19)
    && decide (projection t 0 = parentOrder)
    && decide (projection t 1 = childOneOrder)
    && decide (projection t 2 = childTwoOrder)
    && decide (t.countP (isAllocationOf 1) = 1)
    && decide (t.countP (isReleaseOf 1) = 1)
    && decide (t.countP (isCompletionOf 1) = 1)
    && decide (t.countP (isCompletionOf 2) = 1)
    && decide (t.countP (isJoinOf 1) = 1)
    && decide (t.countP (isJoinOf 2) = 1)
    && readSitsInsideTheWindow 1 t
    && readSitsInsideTheWindow 2 t

private theorem discharged_clauses {t : List Event}
    (h : dischargesSharedReads t = true) :
    t.length = 19 ∧ projection t 0 = parentOrder ∧
      projection t 1 = childOneOrder ∧ projection t 2 = childTwoOrder ∧
      t.countP (isAllocationOf 1) = 1 ∧ t.countP (isReleaseOf 1) = 1 ∧
      t.countP (isCompletionOf 1) = 1 ∧ t.countP (isCompletionOf 2) = 1 ∧
      t.countP (isJoinOf 1) = 1 ∧ t.countP (isJoinOf 2) = 1 ∧
      readSitsInsideTheWindow 1 t = true ∧
      readSitsInsideTheWindow 2 t = true := by
  simp only [dischargesSharedReads, Bool.and_eq_true, decide_eq_true_eq] at h
  exact ⟨h.1.1.1.1.1.1.1.1.1.1.1, h.1.1.1.1.1.1.1.1.1.1.2,
    h.1.1.1.1.1.1.1.1.1.2, h.1.1.1.1.1.1.1.1.2, h.1.1.1.1.1.1.1.2,
    h.1.1.1.1.1.1.2, h.1.1.1.1.1.2, h.1.1.1.1.2, h.1.1.1.2, h.1.1.2,
    h.1.2, h.2⟩

/-! ## Shape helpers -/

private theorem isReadBy_shape {c g : Nat} {e : Event}
    (h : isReadBy c g e = true) : ∃ v, e = Event.read c g v := by
  cases e <;> simp [isReadBy] at h
  next task buffer value =>
    exact ⟨value, by rw [h.1, h.2]⟩

private theorem isJoinOf_shape {c : Nat} {e : Event}
    (h : isJoinOf c e = true) : e = Event.joined c := by
  cases e <;> simp [isJoinOf] at h
  next child => rw [h]

/-- The pinned facts of one lending window in an accepted trace: the loan, the
borrower's read and the returned share each hold their first position, in
strictly increasing order. -/
private theorem window_spec {t : List Event} {c : Nat}
    (hwin : readSitsInsideTheWindow c t = true) :
    ∃ (l r ret : Nat) (er : Event),
      l < r ∧ r < ret ∧
      t[l]? = some (Event.loaned c 1) ∧
      t[r]? = some er ∧ isReadBy c 1 er = true ∧
      t[ret]? = some (Event.returned c 1) := by
  unfold readSitsInsideTheWindow at hwin
  cases hl : (positions t (isLoanEventOf c 1)).head? with
  | none => rw [hl] at hwin; simp at hwin
  | some l =>
    cases hr : (positions t (isReadBy c 1)).head? with
    | none => rw [hl, hr] at hwin; simp at hwin
    | some r =>
      cases hret : (positions t (isReturnEventOf c 1)).head? with
      | none => rw [hl, hr, hret] at hwin; simp at hwin
      | some ret =>
        rw [hl, hr, hret] at hwin
        simp only [Bool.and_eq_true, decide_eq_true_eq] at hwin
        obtain ⟨el, hel, hpel⟩ := positions_spec (mem_of_head? hl)
        obtain ⟨er, her, hper⟩ := positions_spec (mem_of_head? hr)
        obtain ⟨eret, heret, hperet⟩ := positions_spec (mem_of_head? hret)
        have hel' : el = Event.loaned c 1 := eq_of_beq hpel
        have heret' : eret = Event.returned c 1 := eq_of_beq hperet
        exact ⟨l, r, ret, er, hwin.1, hwin.2, hel' ▸ hel, her, hper,
          heret' ▸ heret⟩

/-! ## Meaning: what an accepted trace has been shown to satisfy

Each theorem is stated for an arbitrary trace, so the procedure cannot drift
away from the property it is named after. -/

/-- The parent performs exactly the scenario's fifteen facts, in the
scenario's order. In particular the parent — and only the parent, by the two
borrower projections — writes, and it writes only outside the lending
window. -/
theorem discharged_parent_facts_are_exactly_the_scenario {t : List Event}
    (h : dischargesSharedReads t = true) :
    projection t 0 = parentOrder :=
  (discharged_clauses h).2.1

/-- Each borrower performs exactly the scenario's two facts, in the scenario's
order: read the shared generation, complete carrying the value it saw. A
borrower neither writes nor releases — the share conveys read authority
only. -/
theorem discharged_borrower_facts_are_exactly_the_scenario {t : List Event}
    (h : dischargesSharedReads t = true) :
    projection t 1 = [childOneRead, childOneDone] ∧
      projection t 2 = [childTwoRead, childTwoDone] :=
  ⟨(discharged_clauses h).2.2.1, (discharged_clauses h).2.2.2.1⟩

/-- The shared generation is released exactly once, and that one release is
the parent's own fact (`release = .dropped 0 1`, whose actor is task 0) —
neither borrower ever releases the object it reads. -/
theorem discharged_release_is_unique {t : List Event}
    (h : dischargesSharedReads t = true) :
    t.countP (isReleaseOf 1) = 1 ∧ release ∈ t := by
  refine ⟨(discharged_clauses h).2.2.2.2.2.1, ?_⟩
  have hp := discharged_parent_facts_are_exactly_the_scenario h
  have : release ∈ projection t 0 := by rw [hp]; decide
  exact (List.mem_filter.mp this).1

/-- The parent's two writes sit on either side of the lending window: the
prepared write precedes both loans, and both returned shares precede the
restoring write — a write demands reconstructed exclusive authority, so no
write happens while a share is out. -/
theorem discharged_writes_sit_on_either_side_of_the_window {t : List Event}
    (h : dischargesSharedReads t = true) :
    [prepared, loanOne, loanTwo, returnOne, returnTwo, restored].Sublist t := by
  have hp := discharged_parent_facts_are_exactly_the_scenario h
  have chain : [prepared, loanOne, loanTwo, returnOne, returnTwo,
      restored].Sublist parentOrder := by decide
  have hfilter : (projection t 0).Sublist t := List.filter_sublist
  rw [hp] at hfilter
  exact chain.trans hfilter

/-- The shared generation is unreclaimed until the parent has restored its
payload and read it back: write `7`, read `7`, drop, as a subtrace. -/
theorem discharged_release_follows_the_restored_read {t : List Event}
    (h : dischargesSharedReads t = true) :
    [restored, afterRead, release].Sublist t := by
  have hp := discharged_parent_facts_are_exactly_the_scenario h
  have chain : [restored, afterRead, release].Sublist parentOrder := by decide
  have hfilter : (projection t 0).Sublist t := List.filter_sublist
  rw [hp] at hfilter
  exact chain.trans hfilter

/-- The value a read reports, the value a write establishes, the payload an
allocation creates. -/
def readValue : Event → Option Nat
  | .read _ _ value => some value
  | _ => none

def writtenValue : Event → Option Nat
  | .written _ _ value => some value
  | _ => none

def allocatedPayload : Event → Option Nat
  | .allocated _ _ payload => some payload
  | _ => none

/-- Every read reports the value the nearest preceding write established, not
the allocation's initial payload: the three in-window reads carry the prepared
write's `42`, the after-read carries the restoring write's `7`, and `42` is
not the allocated `1` — the write is observable through the shares. -/
theorem discharged_reads_report_the_written_values {t : List Event}
    (h : dischargesSharedReads t = true) :
    parentRead ∈ t ∧ childOneRead ∈ t ∧ childTwoRead ∈ t ∧ afterRead ∈ t ∧
      readValue parentRead = writtenValue prepared ∧
      readValue childOneRead = writtenValue prepared ∧
      readValue childTwoRead = writtenValue prepared ∧
      readValue afterRead = writtenValue restored ∧
      writtenValue prepared ≠ allocatedPayload allocation := by
  obtain ⟨-, hp0, hp1, hp2, -⟩ := discharged_clauses h
  refine ⟨?_, ?_, ?_, ?_, by decide, by decide, by decide, by decide, by decide⟩
  · have : parentRead ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1
  · have : childOneRead ∈ projection t 1 := by rw [hp1]; decide
    exact (List.mem_filter.mp this).1
  · have : childTwoRead ∈ projection t 2 := by rw [hp2]; decide
    exact (List.mem_filter.mp this).1
  · have : afterRead ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1

/-- The one allocation is the object everyone shares: both loans, both
borrower reads, both returns and the release all carry the very generation the
parent's only allocation created. -/
theorem discharged_shared_generation_is_the_allocations {t : List Event}
    (h : dischargesSharedReads t = true) :
    t.countP (isAllocationOf 1) = 1 ∧ allocation ∈ t ∧
      bufferOf loanOne = bufferOf allocation ∧
      bufferOf loanTwo = bufferOf allocation ∧
      bufferOf childOneRead = bufferOf allocation ∧
      bufferOf childTwoRead = bufferOf allocation ∧
      bufferOf release = bufferOf allocation := by
  obtain ⟨-, hp0, -⟩ := discharged_clauses h
  refine ⟨(discharged_clauses h).2.2.2.2.1, ?_, by decide, by decide,
    by decide, by decide, by decide⟩
  have : allocation ∈ projection t 0 := by rw [hp0]; decide
  exact (List.mem_filter.mp this).1

/-- Each borrower's read sits strictly inside its own lending window: loan,
read, returned share, as a subtrace, per borrower. This is the scenario's
cross-actor fact — neither actor's own projection can carry it, which is why
the procedure checks it positionally. -/
theorem discharged_each_borrower_reads_inside_its_window {t : List Event}
    (h : dischargesSharedReads t = true) :
    [loanOne, childOneRead, returnOne].Sublist t ∧
      [loanTwo, childTwoRead, returnTwo].Sublist t := by
  obtain ⟨-, -, hp1, hp2, -, -, -, -, -, -, hwin1, hwin2⟩ :=
    discharged_clauses h
  constructor
  · obtain ⟨l, r, ret, er, hlr, hrret, hel, her, hper, heret⟩ :=
      window_spec hwin1
    obtain ⟨v, hv⟩ := isReadBy_shape hper
    have hermem : er ∈ t := List.mem_of_getElem? her
    have herproj : er ∈ projection t 1 := by
      refine List.mem_filter.mpr ⟨hermem, ?_⟩
      rw [hv]
      rfl
    rw [hp1] at herproj
    have her' : er = childOneRead := by
      rcases List.mem_cons.mp herproj with heq | hrest
      · exact heq
      · rcases List.mem_cons.mp hrest with heq | hnil
        · rw [hv] at heq; exact absurd heq (by simp [childOneDone])
        · exact absurd hnil (by simp)
    exact sublist_triple_of_getElem? hlr hrret hel (her' ▸ her) heret
  · obtain ⟨l, r, ret, er, hlr, hrret, hel, her, hper, heret⟩ :=
      window_spec hwin2
    obtain ⟨v, hv⟩ := isReadBy_shape hper
    have hermem : er ∈ t := List.mem_of_getElem? her
    have herproj : er ∈ projection t 2 := by
      refine List.mem_filter.mpr ⟨hermem, ?_⟩
      rw [hv]
      rfl
    rw [hp2] at herproj
    have her' : er = childTwoRead := by
      rcases List.mem_cons.mp herproj with heq | hrest
      · exact heq
      · rcases List.mem_cons.mp hrest with heq | hnil
        · rw [hv] at heq; exact absurd heq (by simp [childTwoDone])
        · exact absurd hnil (by simp)
    exact sublist_triple_of_getElem? hlr hrret hel (her' ▸ her) heret

/-! ## CT03's reference run is accepted -/

theorem ct03_reference_run_discharges_the_shared_reads :
    dischargesSharedReads referenceRun = true := by decide

/-! ## Every admitted rendering is accepted

`relate` is the extended F02 relation between CT03's reference run and an
independently observed rendering of it. The theorem below closes the property
over every trace that relation admits — for arbitrary traces, not an
enumerated domain. The per-actor clauses and the counts follow the CT01/CT37
closures; the two window clauses are the new work, extracted from the
relation's own order admission — `loanFlag` (a share is read only while it is
out, and returns exactly once, at or after the join) and
`activityAfterJoinFlag` (a borrower's own activity precedes its join). -/

private theorem allocation_is_the_unique_allocation_witness :
    ∀ e ∈ referenceRun, isAllocationOf 1 e = (e == allocation) := by
  intro e he
  simp only [referenceRun, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
    | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

private theorem release_is_the_unique_release_witness :
    ∀ e ∈ referenceRun, isReleaseOf 1 e = (e == release) := by
  intro e he
  simp only [referenceRun, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
    | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

private theorem childOneDone_is_the_unique_completion_witness :
    ∀ e ∈ referenceRun, isCompletionOf 1 e = (e == childOneDone) := by
  intro e he
  simp only [referenceRun, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
    | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

private theorem childTwoDone_is_the_unique_completion_witness :
    ∀ e ∈ referenceRun, isCompletionOf 2 e = (e == childTwoDone) := by
  intro e he
  simp only [referenceRun, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
    | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

private theorem collectionOne_is_the_unique_join_witness :
    ∀ e ∈ referenceRun, isJoinOf 1 e = (e == collectionOne) := by
  intro e he
  simp only [referenceRun, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
    | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

private theorem collectionTwo_is_the_unique_join_witness :
    ∀ e ∈ referenceRun, isJoinOf 2 e = (e == collectionTwo) := by
  intro e he
  simp only [referenceRun, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
    | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

/-- The window extraction: in a trace whose order constraints the extended
relation admits, a borrower whose loan, join and shared read are all present
(with the join unique) has its first shared read strictly inside its own
lending window. The three inequalities are `loanFlag`'s and
`activityAfterJoinFlag`'s own conjuncts, read back positionally. -/
private theorem window_of_admitted {t : List Event} {c : Nat} {readEvent : Event}
    (hcnz : (c == 0) = false)
    (horder : checkOrder t = 0)
    (hloan : Event.loaned c 1 ∈ t)
    (hjoinmem : Event.joined c ∈ t)
    (hjoincount : t.countP (isJoinOf c) = 1)
    (hreadmem : readEvent ∈ t)
    (hreadpred : isReadBy c 1 readEvent = true) :
    readSitsInsideTheWindow c t = true := by
  -- The three first positions exist.
  obtain ⟨il, hil⟩ := exists_getElem?_of_mem hloan
  have hilmem : il ∈ positions t (isLoanEventOf c 1) :=
    mem_positions hil (by simp [isLoanEventOf])
  obtain ⟨l, hlhead⟩ := head?_some_of_mem hilmem
  obtain ⟨el, hel, hpel⟩ := positions_spec (mem_of_head? hlhead)
  have hel' : el = Event.loaned c 1 := eq_of_beq hpel
  obtain ⟨ir, hir⟩ := exists_getElem?_of_mem hreadmem
  have hirmem : ir ∈ positions t (isReadBy c 1) := mem_positions hir hreadpred
  obtain ⟨r, hrhead⟩ := head?_some_of_mem hirmem
  have hrmem : r ∈ positions t (isReadBy c 1) := mem_of_head? hrhead
  obtain ⟨er, her, hper⟩ := positions_spec hrmem
  obtain ⟨v, hv⟩ := isReadBy_shape hper
  have hjlen : (positions t (isJoinOf c)).length = 1 := by
    rw [positions_length, hjoincount]
  obtain ⟨j, hjpos⟩ := List.length_eq_one_iff.mp hjlen
  obtain ⟨ej, hej, hpej⟩ := positions_spec (hjpos ▸ List.mem_singleton_self j)
  have hej' : ej = Event.joined c := isJoinOf_shape hpej
  -- Extract the loan conjunct at this loan.
  have hflag19 := loanFlag_false_of_checkOrder_zero horder
  unfold loanFlag at hflag19
  have hbody := Bool.not_eq_true _ ▸
    List.any_eq_false.mp hflag19 (Event.loaned c 1) hloan
  dsimp only at hbody
  rw [Bool.or_eq_false_iff] at hbody
  obtain ⟨hreadsflag, hjoinflag⟩ := hbody
  -- l ≤ r, and l ≠ r because a loan is not a read.
  have hnotlt : ¬(r < (positions t (isLoanEventOf c 1)).headD 0) := by
    have := List.any_eq_false.mp hreadsflag r hrmem
    simpa using this
  rw [headD_eq_of_head? hlhead] at hnotlt
  have hlr : l < r := by
    have hne : l ≠ r := by
      intro heq
      rw [heq] at hel
      rw [her] at hel
      have hee : er = el := by simpa using hel
      rw [hel', hv] at hee
      exact absurd hee (by simp)
    omega
  -- The returned share exists uniquely, at or after the join.
  rw [hjpos] at hjoinflag
  simp only [List.head?_cons] at hjoinflag
  rw [Bool.or_eq_false_iff] at hjoinflag
  obtain ⟨hretlen', hretpos'⟩ := hjoinflag
  have hretlen : (positions t (isReturnEventOf c 1)).length = 1 := by
    simpa using hretlen'
  obtain ⟨ret, hretpos⟩ := List.length_eq_one_iff.mp hretlen
  have hjret : j ≤ ret := by
    have : ¬(((positions t (isReturnEventOf c 1)).headD 0) < j) := by
      simpa using hretpos'
    rw [hretpos] at this
    simpa using Nat.le_of_not_lt this
  -- r ≤ j from the activity-after-join conjunct, and r ≠ j.
  have hflag18 := activityAfterJoinFlag_false_of_checkOrder_zero horder
  unfold activityAfterJoinFlag at hflag18
  have hcmem : c ∈ sortNats (t.filterMap subjectOf) :=
    mem_sortNats (List.mem_filterMap.mpr ⟨Event.joined c, hjoinmem, rfl⟩)
  have hbody18' := Bool.not_eq_true _ ▸ List.any_eq_false.mp hflag18 c hcmem
  rw [hjpos] at hbody18'
  simp only [List.head?_cons] at hbody18'
  have hperAJ : (isAbout c er && !isReturn er) = true := by
    rw [hv]
    simp [isAbout, subjectOf, isReturn, hcnz]
  have hrmemAJ : r ∈ positions t (fun e => isAbout c e && !isReturn e) :=
    mem_positions her hperAJ
  have hrj : r < j := by
    have hnot : ¬(r > j) := by
      have := List.any_eq_false.mp hbody18' r hrmemAJ
      simpa using this
    have hne : r ≠ j := by
      intro heq
      rw [heq] at her
      rw [hej] at her
      have hee : ej = er := by simpa using her
      rw [hej', hv] at hee
      exact absurd hee (by simp)
    omega
  have hrret : r < ret := by omega
  -- Assemble the window.
  unfold readSitsInsideTheWindow
  rw [hlhead, hrhead, hretpos]
  simp [hlr, hrret]

theorem every_admitted_rendering_discharges_the_shared_reads
    (t : List Event) (h : relate referenceRun t = 0) :
    dischargesSharedReads t = true := by
  have hattr := admitted_traces_are_attributable referenceRun t h
  have hdup := admitted_traces_have_distinct_occurrences referenceRun t h
  have horder := admitted_traces_satisfy_the_order_constraints referenceRun t h
  have hlen : t.length = 19 := by
    have := admitted_traces_report_the_same_facts referenceRun t h
    simpa [referenceRun] using this
  simp only [relate, hattr, hdup, Bool.false_eq_true, ite_false] at h
  by_cases hmultiset : (t.length != referenceRun.length
      || t.any fun event => !referenceRun.contains event) = true
  · rw [ite_eq_left hmultiset] at h
    exact absurd h (by decide)
  rw [ite_eq_right hmultiset] at h
  have hsub : ∀ e ∈ t, e ∈ referenceRun := by
    intro e he
    have hparts := Bool.or_eq_false_iff.mp (Bool.not_eq_true _ ▸ hmultiset :
      (t.length != referenceRun.length
        || t.any fun event => !referenceRun.contains event) = false)
    have := List.any_eq_false.mp hparts.2 e he
    exact List.contains_iff_mem.mp (by simpa using this)
  by_cases hactors : ((sortNats (referenceRun.filterMap actorOf)).any
      fun actor => projection t actor != projection referenceRun actor) = true
  · rw [ite_eq_left hactors] at h
    exact absurd h (by decide)
  rw [ite_eq_right hactors] at h
  have hprojections : projection t 0 = projection referenceRun 0 ∧
      projection t 1 = projection referenceRun 1 ∧
      projection t 2 = projection referenceRun 2 := by
    have hs : sortNats (referenceRun.filterMap actorOf) = [0, 1, 2] := by decide
    have hfalse := Bool.not_eq_true _ ▸ hactors
    rw [hs] at hfalse
    simp [List.any, Bool.or_eq_false_iff] at hfalse
    exact hfalse
  have hp0 : projection t 0 = parentOrder :=
    hprojections.1.trans parent_projection_of_the_reference
  have hp1 : projection t 1 = childOneOrder :=
    hprojections.2.1.trans childOne_projection_of_the_reference
  have hp2 : projection t 2 = childTwoOrder :=
    hprojections.2.2.trans childTwo_projection_of_the_reference
  -- Memberships, each read off its own projection.
  have hallocation : allocation ∈ t := by
    have : allocation ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1
  have hrelease : release ∈ t := by
    have : release ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1
  have hloanOne : Event.loaned 1 1 ∈ t := by
    have : loanOne ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1
  have hloanTwo : Event.loaned 2 1 ∈ t := by
    have : loanTwo ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1
  have hjoinOne : Event.joined 1 ∈ t := by
    have : collectionOne ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1
  have hjoinTwo : Event.joined 2 ∈ t := by
    have : collectionTwo ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1
  have hreadOne : childOneRead ∈ t := by
    have : childOneRead ∈ projection t 1 := by rw [hp1]; decide
    exact (List.mem_filter.mp this).1
  have hreadTwo : childTwoRead ∈ t := by
    have : childTwoRead ∈ projection t 2 := by rw [hp2]; decide
    exact (List.mem_filter.mp this).1
  have hdoneOne : childOneDone ∈ t := by
    have : childOneDone ∈ projection t 1 := by rw [hp1]; decide
    exact (List.mem_filter.mp this).1
  have hdoneTwo : childTwoDone ∈ t := by
    have : childTwoDone ∈ projection t 2 := by rw [hp2]; decide
    exact (List.mem_filter.mp this).1
  -- The six counted facts, by unique witness.
  have hcAlloc := countP_eq_one_of_unique_witness
    allocation_is_the_unique_allocation_witness hsub hdup hallocation
  have hcRelease := countP_eq_one_of_unique_witness
    release_is_the_unique_release_witness hsub hdup hrelease
  have hcDone1 := countP_eq_one_of_unique_witness
    childOneDone_is_the_unique_completion_witness hsub hdup hdoneOne
  have hcDone2 := countP_eq_one_of_unique_witness
    childTwoDone_is_the_unique_completion_witness hsub hdup hdoneTwo
  have hcJoin1 := countP_eq_one_of_unique_witness
    collectionOne_is_the_unique_join_witness hsub hdup hjoinOne
  have hcJoin2 := countP_eq_one_of_unique_witness
    collectionTwo_is_the_unique_join_witness hsub hdup hjoinTwo
  -- The two window clauses, from the relation's own order admission.
  have hwin1 : readSitsInsideTheWindow 1 t = true :=
    window_of_admitted (by decide) horder hloanOne hjoinOne hcJoin1
      hreadOne (by decide)
  have hwin2 : readSitsInsideTheWindow 2 t = true :=
    window_of_admitted (by decide) horder hloanTwo hjoinTwo hcJoin2
      hreadTwo (by decide)
  simp only [dischargesSharedReads, hlen, hp0, hp1, hp2, hcAlloc, hcRelease,
    hcDone1, hcDone2, hcJoin1, hcJoin2, hwin1, hwin2]
  decide

/-! ## Named refusals: the procedure is not vacuous

Each mutant is CT03's reference run with exactly one scenario fact broken. -/

/-- The first borrower's read moved before its loan opened — every per-actor
projection and every count intact: refused, and only the window clause refuses
it. -/
theorem a_read_before_the_loan_is_refused :
    dischargesSharedReads
      [ allocation, prepared, creationOne, childOneRead, loanOne, creationTwo,
        loanTwo, parentRead, childOneDone, collectionOne, returnOne,
        childTwoRead, childTwoDone, collectionTwo, returnTwo, restored,
        afterRead, release, rootResult ] = false := by decide

/-- The first borrower's read moved after its share returned — projections and
counts again intact: refused, and only the window clause refuses it. -/
theorem a_read_after_the_return_is_refused :
    dischargesSharedReads
      [ allocation, prepared, creationOne, loanOne, creationTwo, loanTwo,
        parentRead, childOneDone, collectionOne, returnOne, childOneRead,
        childTwoRead, childTwoDone, collectionTwo, returnTwo, restored,
        afterRead, release, rootResult ] = false := by decide

/-- A parent write while the shares are out — CT04's shape, the scenario this
acceptance is the complement of: refused. -/
theorem a_write_inside_the_window_is_refused :
    dischargesSharedReads
      [ allocation, prepared, creationOne, loanOne, creationTwo, loanTwo,
        .written 0 1 9, childOneRead, childOneDone, collectionOne, returnOne,
        childTwoRead, childTwoDone, collectionTwo, returnTwo, restored,
        afterRead, release, rootResult ] = false := by decide

/-- The parent's after-read carrying the stale prepared value instead of the
restored one — the write that is not observable: refused. -/
theorem a_stale_after_read_is_refused :
    dischargesSharedReads
      [ allocation, prepared, creationOne, loanOne, creationTwo, loanTwo,
        parentRead, childOneRead, childOneDone, collectionOne, returnOne,
        childTwoRead, childTwoDone, collectionTwo, returnTwo, restored,
        .read 0 1 42, release, rootResult ] = false := by decide

/-- The release removed — the leak shape: refused. -/
theorem a_missing_release_is_refused :
    dischargesSharedReads
      [ allocation, prepared, creationOne, loanOne, creationTwo, loanTwo,
        parentRead, childOneRead, childOneDone, collectionOne, returnOne,
        childTwoRead, childTwoDone, collectionTwo, returnTwo, restored,
        afterRead, rootResult ] = false := by decide

/-- A second release of the shared generation: refused. -/
theorem a_double_release_is_refused :
    dischargesSharedReads
      [ allocation, prepared, creationOne, loanOne, creationTwo, loanTwo,
        parentRead, childOneRead, childOneDone, collectionOne, returnOne,
        childTwoRead, childTwoDone, collectionTwo, returnTwo, restored,
        afterRead, release, .dropped 1 1, rootResult ] = false := by decide

/-- A borrower that writes instead of reading — the share conveys read
authority only: refused. -/
theorem a_borrower_that_writes_is_refused :
    dischargesSharedReads
      [ allocation, prepared, creationOne, loanOne, creationTwo, loanTwo,
        parentRead, .written 1 1 5, childOneDone, collectionOne, returnOne,
        childTwoRead, childTwoDone, collectionTwo, returnTwo, restored,
        afterRead, release, rootResult ] = false := by decide

end QCore.Ct03ReadShares
