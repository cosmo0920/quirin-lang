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

import QCore.TaskTraceRelation

/-!
# CT01 — the owned-transfer discharge, proved on the case's own trace terms

CT01's scenario is `Move an owned buffer to child; child returns it; parent
joins and drops` (spec §55, semantic profile `ScopedOwnedTasksV1`, expected
`Accept`). This module gives that scenario a proved statement over the F02
observation alphabet of `QCore.TaskTraceRelation`, whose `roundTrip` term is
CT01's own reference run: the workspace test
`compiler/tests/f02_trace_relation.rs::the_declared_trace_fixtures_are_the_reference_runs`
checks, event for event, that compiling `compiler/tests/fixtures/ct/CT01/case.qr`
through the real pipeline and executing the checked task module reproduces
exactly the recorded trace that `roundTrip` transcribes, and
`ct01_trace_digest` below pins the transcription itself with the F02 digest
fold, which the named Rust runner
`compiler/tests/ct01_proof_leg.rs::ct01_artifact_is_the_lean_proved_owned_transfer_shape`
recomputes from the artifact with wrapping `u64` arithmetic.

What this file establishes:

* `dischargesOwnedTransfer` — one **total decision procedure** for the
  scenario's property, and the meaning theorems that keep it honest: a trace it
  accepts has the parent perform exactly allocate, spawn, join, read, drop,
  complete in that order (`discharged_parent_facts_are_exactly_the_scenario`),
  releases the moved buffer **exactly once, by the parent, after the join**
  (`discharged_release_is_unique`, `discharged_join_precedes_the_release`),
  and its child's **only** completion returns the very generation the parent's
  only allocation created (`discharged_child_return_preserves_the_generation`).
* `ct01_reference_run_discharges_the_owned_transfer` — CT01's own reference
  trace is accepted by the procedure.
* `every_admitted_rendering_discharges_the_owned_transfer` — for **arbitrary**
  traces: any observation the F02 relation admits against CT01's reference run
  (`relate roundTrip observed = 0`) is accepted by the procedure, so the
  property is a fact about every admitted rendering of this run, not about one
  string of bytes.
* The named refusals — a release before the join, a missing release, a child
  release, a second release and a generation-changing return are all refused,
  so the procedure is not vacuous.

What this file does **not** establish: that the emitted C's native execution
refines the reference execution (the native side is covered only by the
byte-comparison observation of `run_native_owned_transfer` and by F02's
relation checks, exactly as before), that the Rust pipeline is correct, or
anything about programs outside this profile.
-/

namespace QCore.Ct01OwnedTransfer
open QCore.TaskTraceRelation

/- The derived `BEq` on the F02 alphabet is structural equality; making that a
registered `LawfulBEq` fact is what lets the relation's own `==` conjuncts be
read back as propositional equalities below. -/
deriving instance ReflBEq, LawfulBEq for Outcome
deriving instance ReflBEq, LawfulBEq for Event

/-! ## CT01's seven facts, named -/

def allocation : Event := .allocated 0 1 42
def creation : Event := .spawned 1 0
def childReturn : Event := .completed 1 (.buffer 1)
def collection : Event := .joined 1
def readBack : Event := .read 0 1 42
def release : Event := .dropped 0 1
def rootResult : Event := .completed 0 (.i32 42)

/-- The parent's program order: allocate, spawn, join, read, drop, complete. -/
def parentOrder : List Event :=
  [allocation, creation, collection, readBack, release, rootResult]

theorem parent_projection_of_the_reference :
    projection roundTrip 0 = parentOrder := by decide

theorem child_projection_of_the_reference :
    projection roundTrip 1 = [childReturn] := by decide

/-- The transcription pin the Rust runner recomputes from CT01's own artifact
with wrapping `u64` arithmetic (the F02 digest fold). -/
theorem ct01_trace_digest : mixTrace roundTrip = 13023433800416048247 := by
  decide

/-! ## The decision procedure -/

/-- Total decision procedure for CT01's scenario property over the F02
alphabet. A trace is accepted exactly when it reports seven facts; the parent's
own facts are precisely `parentOrder` in that order; the child's own fact is
precisely the generation-preserving return; and the moved buffer has exactly
one allocation, the moved-in generation has exactly one release, and the child
has exactly one completion — so a second release, a child-side release, a leak
or a generation-changing return all refuse. The Rust transcription in
`compiler/tests/ct01_proof_leg.rs` runs this same procedure on the artifact. -/
def dischargesOwnedTransfer (t : List Event) : Bool :=
  decide (t.length = 7)
    && decide (projection t 0 = parentOrder)
    && decide (projection t 1 = [childReturn])
    && decide (t.countP (isAllocationOf 1) = 1)
    && decide (t.countP (isReleaseOf 1) = 1)
    && decide (t.countP (isCompletionOf 1) = 1)

private theorem discharged_clauses {t : List Event}
    (h : dischargesOwnedTransfer t = true) :
    t.length = 7 ∧ projection t 0 = parentOrder ∧
      projection t 1 = [childReturn] ∧ t.countP (isAllocationOf 1) = 1 ∧
      t.countP (isReleaseOf 1) = 1 ∧ t.countP (isCompletionOf 1) = 1 := by
  simp only [dischargesOwnedTransfer, Bool.and_eq_true, decide_eq_true_eq] at h
  exact ⟨h.1.1.1.1.1, h.1.1.1.1.2, h.1.1.1.2, h.1.1.2, h.1.2, h.2⟩

/-! ## Meaning: what an accepted trace has been shown to satisfy

Each theorem is stated for an arbitrary trace, so the procedure cannot drift
away from the property it is named after. -/

/-- The parent performs exactly the scenario's six facts, in the scenario's
order: allocate, spawn, join, read the moved payload back, drop, complete. -/
theorem discharged_parent_facts_are_exactly_the_scenario {t : List Event}
    (h : dischargesOwnedTransfer t = true) :
    projection t 0 = [allocation, creation, collection, readBack, release,
      rootResult] :=
  (discharged_clauses h).2.1

/-- The moved generation is released exactly once, and that one release is the
parent's own fact (`release = .dropped 0 1`, whose actor is task 0). -/
theorem discharged_release_is_unique {t : List Event}
    (h : dischargesOwnedTransfer t = true) :
    t.countP (isReleaseOf 1) = 1 ∧ release ∈ t := by
  refine ⟨(discharged_clauses h).2.2.2.2.1, ?_⟩
  have hp := discharged_parent_facts_are_exactly_the_scenario h
  have : release ∈ projection t 0 := by rw [hp]; decide
  exact (List.mem_filter.mp this).1

/-- The join happens, the release happens, and the join precedes the release:
the two-fact chain is a subtrace of any accepted trace. Together with
`discharged_release_is_unique` this is `the parent drops after the join`. -/
theorem discharged_join_precedes_the_release {t : List Event}
    (h : dischargesOwnedTransfer t = true) :
    [collection, release].Sublist t := by
  have hp := discharged_parent_facts_are_exactly_the_scenario h
  have chain : [collection, release].Sublist parentOrder := by decide
  have hfilter : (projection t 0).Sublist t := List.filter_sublist
  rw [hp] at hfilter
  exact chain.trans hfilter

/-- The payload is read back between the join and the release. -/
theorem discharged_read_back_sits_between_join_and_release {t : List Event}
    (h : dischargesOwnedTransfer t = true) :
    [collection, readBack, release].Sublist t := by
  have hp := discharged_parent_facts_are_exactly_the_scenario h
  have chain : [collection, readBack, release].Sublist parentOrder := by decide
  have hfilter : (projection t 0).Sublist t := List.filter_sublist
  rw [hp] at hfilter
  exact chain.trans hfilter

/-- The child's only completion returns `.buffer 1` — the same generation the
parent's only allocation created — so the return is generation-preserving. -/
theorem discharged_child_return_preserves_the_generation {t : List Event}
    (h : dischargesOwnedTransfer t = true) :
    t.countP (isCompletionOf 1) = 1 ∧ childReturn ∈ t ∧
      t.countP (isAllocationOf 1) = 1 ∧ allocation ∈ t ∧
      bufferOf childReturn = bufferOf allocation := by
  obtain ⟨-, hp0, hp1, halloc, -, hcomp⟩ := discharged_clauses h
  refine ⟨hcomp, ?_, halloc, ?_, by decide⟩
  · have : childReturn ∈ projection t 1 := by rw [hp1]; decide
    exact (List.mem_filter.mp this).1
  · have : allocation ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1

/-! ## CT01's reference run is accepted -/

theorem ct01_reference_run_discharges_the_owned_transfer :
    dischargesOwnedTransfer roundTrip = true := by decide

/-! ## Every admitted rendering is accepted

`relate` is the F02 relation between CT01's reference run and an independently
observed rendering of it. The theorem below closes the property over every
trace that relation admits — for arbitrary traces, not an enumerated domain —
so the discharge is a fact about the admitted causal order itself. -/

private theorem countP_congr_on {p q : Event → Bool} :
    ∀ {t : List Event}, (∀ e ∈ t, p e = q e) → t.countP p = t.countP q := by
  intro t
  induction t with
  | nil => intro _; rfl
  | cons a rest ih =>
    intro h
    have ha := h a (List.mem_cons_self ..)
    have hr := ih fun e he => h e (List.mem_cons_of_mem a he)
    simp [List.countP_cons, ha, hr]

private theorem countP_beq_le_one_of_no_duplicates :
    ∀ {t : List Event}, hasDuplicate t = false →
      ∀ x : Event, t.countP (· == x) ≤ 1 := by
  intro t
  induction t with
  | nil => intro _ x; simp
  | cons a rest ih =>
    intro h x
    have hparts : a ∉ rest ∧ hasDuplicate rest = false := by
      simpa [hasDuplicate, Bool.or_eq_false_iff] using h
    by_cases hax : a = x
    · subst hax
      have hz : rest.countP (· == a) = 0 := by
        rw [List.countP_eq_zero]
        intro e he heq
        exact hparts.1 (eq_of_beq heq ▸ he)
      simp [hz]
    · have hrest := ih hparts.2 x
      have hne : (a == x) = false := by
        simpa using hax
      simp [hne]
      omega

/-- The one lemma the closure needs per counted fact: a predicate that, on the
reference's events, holds exactly of one named event, counts to exactly one on
any admitted rendering containing that event. -/
private theorem countP_eq_one_of_unique_witness {p : Event → Bool}
    {x : Event} {t : List Event}
    (hax : ∀ e ∈ roundTrip, p e = (e == x))
    (hsub : ∀ e ∈ t, e ∈ roundTrip)
    (hdup : hasDuplicate t = false)
    (hmem : x ∈ t) : t.countP p = 1 := by
  have hagree : t.countP p = t.countP (· == x) :=
    countP_congr_on fun e he => hax e (hsub e he)
  have hle := countP_beq_le_one_of_no_duplicates hdup x
  have hge : t.countP (· == x) ≠ 0 := fun hzero =>
    (List.countP_eq_zero.mp hzero x hmem) (by simp)
  omega

private theorem allocation_is_the_unique_allocation_witness :
    ∀ e ∈ roundTrip, isAllocationOf 1 e = (e == allocation) := by
  intro e he
  simp only [roundTrip, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

private theorem release_is_the_unique_release_witness :
    ∀ e ∈ roundTrip, isReleaseOf 1 e = (e == release) := by
  intro e he
  simp only [roundTrip, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

private theorem childReturn_is_the_unique_completion_witness :
    ∀ e ∈ roundTrip, isCompletionOf 1 e = (e == childReturn) := by
  intro e he
  simp only [roundTrip, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

theorem every_admitted_rendering_discharges_the_owned_transfer
    (t : List Event) (h : relate roundTrip t = 0) :
    dischargesOwnedTransfer t = true := by
  have hattr := admitted_traces_are_attributable roundTrip t h
  have hdup := admitted_traces_have_distinct_occurrences roundTrip t h
  have hlen : t.length = 7 := by
    have := admitted_traces_report_the_same_facts roundTrip t h
    simpa [roundTrip] using this
  simp only [relate, hattr, hdup, Bool.false_eq_true, ite_false] at h
  by_cases hmultiset : (t.length != roundTrip.length
      || t.any fun event => !roundTrip.contains event) = true
  · rw [ite_eq_left hmultiset] at h
    exact absurd h (by decide)
  rw [ite_eq_right hmultiset] at h
  have hsub : ∀ e ∈ t, e ∈ roundTrip := by
    intro e he
    have hparts := Bool.or_eq_false_iff.mp (Bool.not_eq_true _ ▸ hmultiset :
      (t.length != roundTrip.length
        || t.any fun event => !roundTrip.contains event) = false)
    have := List.any_eq_false.mp hparts.2 e he
    exact List.contains_iff_mem.mp (by simpa using this)
  by_cases hactors : ((sortNats (roundTrip.filterMap actorOf)).any
      fun actor => projection t actor != projection roundTrip actor) = true
  · rw [ite_eq_left hactors] at h
    exact absurd h (by decide)
  rw [ite_eq_right hactors] at h
  have hprojections : projection t 0 = projection roundTrip 0 ∧
      projection t 1 = projection roundTrip 1 := by
    have hs : sortNats (roundTrip.filterMap actorOf) = [0, 1] := by decide
    have hfalse := Bool.not_eq_true _ ▸ hactors
    rw [hs] at hfalse
    simp [List.any, Bool.or_eq_false_iff] at hfalse
    exact hfalse
  have hp0 : projection t 0 = parentOrder :=
    hprojections.1.trans parent_projection_of_the_reference
  have hp1 : projection t 1 = [childReturn] :=
    hprojections.2.trans child_projection_of_the_reference
  have hallocation : allocation ∈ t := by
    have : allocation ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1
  have hrelease : release ∈ t := by
    have : release ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1
  have hchild : childReturn ∈ t := by
    have : childReturn ∈ projection t 1 := by rw [hp1]; decide
    exact (List.mem_filter.mp this).1
  have hcAlloc := countP_eq_one_of_unique_witness
    allocation_is_the_unique_allocation_witness hsub hdup hallocation
  have hcRelease := countP_eq_one_of_unique_witness
    release_is_the_unique_release_witness hsub hdup hrelease
  have hcChild := countP_eq_one_of_unique_witness
    childReturn_is_the_unique_completion_witness hsub hdup hchild
  simp only [dischargesOwnedTransfer, hlen, hp0, hp1, hcAlloc, hcRelease,
    hcChild]
  decide

/-! ## Named refusals: the procedure is not vacuous

Each mutant is CT01's reference run with exactly one scenario fact broken. -/

/-- The release moved before the join: refused. -/
theorem a_release_before_the_join_is_refused :
    dischargesOwnedTransfer
      [allocation, creation, childReturn, release, collection, readBack,
        rootResult] = false := by decide

/-- The release removed — the leak shape: refused. -/
theorem a_missing_release_is_refused :
    dischargesOwnedTransfer
      [allocation, creation, childReturn, collection, readBack, rootResult]
      = false := by decide

/-- The release performed by the child instead of the parent: refused. -/
theorem a_release_by_the_child_is_refused :
    dischargesOwnedTransfer
      [allocation, creation, childReturn, collection, readBack,
        .dropped 1 1, rootResult] = false := by decide

/-- A second release of the moved generation: refused. -/
theorem a_double_release_is_refused :
    dischargesOwnedTransfer
      [allocation, creation, childReturn, collection, readBack, release,
        .dropped 1 1, rootResult] = false := by decide

/-- A child return carrying a different generation: refused. -/
theorem a_generation_changing_return_is_refused :
    dischargesOwnedTransfer
      [allocation, creation, .completed 1 (.buffer 2), collection, readBack,
        release, rootResult] = false := by decide

end QCore.Ct01OwnedTransfer
