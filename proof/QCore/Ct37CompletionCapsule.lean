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

import QCore.Ct01OwnedTransfer

/-!
# CT37 — the completion-capsule discharge, proved on the case's own trace terms

CT37's scenario is `Child returns allocation and finishes early` with the
expected reading `Allocation remains live in completion capsule` (spec §55,
semantic profile `ScopedOwnedTasksV1`). This module gives that scenario a
proved statement over the F02 observation alphabet of
`QCore.TaskTraceRelation`, on the CT01 precedent
(`QCore.Ct01OwnedTransfer`): `referenceRun` below transcribes the reference
execution of `compiler/tests/fixtures/ct/CT37/case.qr` through the real
pipeline — the worker allocates the Buffer in its own body (the worker-local
generation `4294967297`), returns it inside its completion record, and
finishes before the parent joins; the parent joins, reads the allocation's own
payload back out of the capsule, releases exactly once and completes. The
named Rust runner
`compiler/tests/ct37_proof_leg.rs::ct37_artifact_is_the_lean_proved_completion_capsule_shape`
checks, event for event, that the artifact's reference run is exactly this
term, recomputes `ct37_trace_digest` from it with wrapping `u64` arithmetic,
and runs the same total decision procedure this file defines.

What this file establishes:

* `dischargesCompletionCapsule` — one **total decision procedure** for the
  scenario's property, and the meaning theorems that keep it honest: a trace
  it accepts has the child perform exactly its two scenario facts — allocate
  the Buffer, finish carrying that very generation — and nothing else
  (`discharged_child_facts_are_exactly_the_scenario`); has the child's only
  completion sit strictly **before** the parent's only join — the early finish
  (`discharged_return_precedes_the_join`); keeps the moved generation
  unreleased until the parent has joined and read the allocation's own payload
  back — the capsule keeps the allocation live
  (`discharged_read_back_sits_between_join_and_release`,
  `discharged_release_is_unique`); and returns the very generation the child's
  only allocation created
  (`discharged_return_preserves_the_generation`).
* `ct37_reference_run_discharges_the_completion_capsule` — CT37's own
  reference trace is accepted.
* `every_admitted_rendering_discharges_the_completion_capsule` — for
  **arbitrary** traces: any observation the F02 relation admits against CT37's
  reference run (`relate referenceRun observed = 0`) is accepted by the
  procedure. Unlike CT01, one of this scenario's facts — the child completing
  before the parent joins — crosses actors, so the closure additionally
  extracts the relation's own order constraint (`checkOrder`'s
  completion-before-join conjunct) rather than reading everything off the
  per-actor projections.
* The named refusals — a join before the return, a missing release, a child
  that releases instead of returning, a double release, a generation-changing
  return and a capsule that loses the payload are each refused, so the
  procedure is not vacuous.

What this file does **not** establish: that the emitted C's native execution
refines the reference execution (the native side is covered only by the
`native_task_scenarios` harness's observations, which force worker-body
completion before creation returns and byte-compare the streams), that the
Rust pipeline is correct, or anything about programs outside this profile.
-/

namespace QCore.Ct37CompletionCapsule
open QCore.TaskTraceRelation

/-! ## CT37's seven facts, named

The moved generation is the worker-local `4294967297` (`(1 <<< 32) + 1`): the
allocation happens in the child's own body, so it carries the child's
generation counter rather than a root one. -/

def movedGeneration : Nat := 4294967297

def creation : Event := .spawned 1 0
def allocation : Event := .allocated 1 movedGeneration 42
def earlyReturn : Event := .completed 1 (.buffer movedGeneration)
def collection : Event := .joined 1
def readBack : Event := .read 0 movedGeneration 42
def release : Event := .dropped 0 movedGeneration
def rootResult : Event := .completed 0 (.i32 42)

/-- CT37's reference run: the checked task module's reference execution,
rendered by the production observer. The named Rust runner asserts the
artifact reproduces exactly this term. -/
def referenceRun : List Event :=
  [creation, allocation, earlyReturn, collection, readBack, release, rootResult]

/-- The parent's program order: spawn, join, read the capsule's payload back,
drop, complete. The parent allocates nothing — the allocation is the child's. -/
def parentOrder : List Event :=
  [creation, collection, readBack, release, rootResult]

/-- The child's program order: allocate, finish carrying the allocation. The
child neither reads nor releases — the allocation departs inside the
completion record. -/
def childOrder : List Event := [allocation, earlyReturn]

theorem ct37_reference_run_is_an_admitted_causal_order :
    declare referenceRun = 0 := by decide

theorem parent_projection_of_the_reference :
    projection referenceRun 0 = parentOrder := by decide

theorem child_projection_of_the_reference :
    projection referenceRun 1 = childOrder := by decide

/-- The transcription pin the Rust runner recomputes from CT37's own artifact
with wrapping `u64` arithmetic (the F02 digest fold). -/
theorem ct37_trace_digest : mixTrace referenceRun = 17239871955803798867 := by
  decide

/-! ## The decision procedure -/

/-- The cross-actor fact of this scenario: the child's first completion sits
strictly before the parent's first join — the child *finished early*. Stated
positionally because no single actor's projection can carry it. -/
def returnPrecedesTheJoin (t : List Event) : Bool :=
  match (positions t (isCompletionOf 1)).head?, (positions t (isJoinOf 1)).head? with
  | some completion, some join => decide (completion < join)
  | _, _ => false

/-- Total decision procedure for CT37's scenario property over the F02
alphabet. A trace is accepted exactly when it reports seven facts; the
parent's own facts are precisely `parentOrder` in that order; the child's own
facts are precisely `childOrder` — allocate, then finish carrying the
allocation; the moved generation has exactly one allocation and exactly one
release, the child exactly one completion and exactly one join; and the
child's completion precedes the parent's join. The Rust transcription in
`compiler/tests/ct37_proof_leg.rs` runs this same procedure on the artifact. -/
def dischargesCompletionCapsule (t : List Event) : Bool :=
  decide (t.length = 7)
    && decide (projection t 0 = parentOrder)
    && decide (projection t 1 = childOrder)
    && decide (t.countP (isAllocationOf movedGeneration) = 1)
    && decide (t.countP (isReleaseOf movedGeneration) = 1)
    && decide (t.countP (isCompletionOf 1) = 1)
    && decide (t.countP (isJoinOf 1) = 1)
    && returnPrecedesTheJoin t

private theorem discharged_clauses {t : List Event}
    (h : dischargesCompletionCapsule t = true) :
    t.length = 7 ∧ projection t 0 = parentOrder ∧ projection t 1 = childOrder ∧
      t.countP (isAllocationOf movedGeneration) = 1 ∧
      t.countP (isReleaseOf movedGeneration) = 1 ∧
      t.countP (isCompletionOf 1) = 1 ∧ t.countP (isJoinOf 1) = 1 ∧
      returnPrecedesTheJoin t = true := by
  simp only [dischargesCompletionCapsule, Bool.and_eq_true, decide_eq_true_eq] at h
  exact ⟨h.1.1.1.1.1.1.1, h.1.1.1.1.1.1.2, h.1.1.1.1.1.2, h.1.1.1.1.2,
    h.1.1.1.2, h.1.1.2, h.1.2, h.2⟩

/-! ## Positional helpers

`positions` is the relation's own index list. The procedure's cross-actor
clause and its meaning theorems need three general facts about it: its length
is the predicate's count, a listed index really holds a matching event, and
two indexed events in order form a two-element subtrace. -/

private theorem positionsFrom_length {p : Event → Bool} :
    ∀ (t : List Event) (k : Nat), (positionsFrom k p t).length = t.countP p := by
  intro t
  induction t with
  | nil => intro k; rfl
  | cons a rest ih =>
    intro k
    by_cases ha : p a
    · simp [positionsFrom, ha, ih]
    · simp [positionsFrom, ha, ih]

private theorem positions_length {p : Event → Bool} {t : List Event} :
    (positions t p).length = t.countP p :=
  positionsFrom_length t 0

private theorem positionsFrom_spec {p : Event → Bool} :
    ∀ {t : List Event} {k i : Nat}, i ∈ positionsFrom k p t →
      k ≤ i ∧ ∃ e, t[i - k]? = some e ∧ p e = true := by
  intro t
  induction t with
  | nil => intro k i h; simp [positionsFrom] at h
  | cons a rest ih =>
    intro k i h
    by_cases ha : p a
    · simp only [positionsFrom, ha, ite_true, List.mem_cons] at h
      rcases h with rfl | h
      · exact ⟨Nat.le_refl i, a, by simp, ha⟩
      · obtain ⟨hk, e, he, hpe⟩ := ih h
        refine ⟨by omega, e, ?_, hpe⟩
        have : i - k = (i - (k + 1)) + 1 := by omega
        rw [this, List.getElem?_cons_succ]
        exact he
    · simp only [positionsFrom, ha] at h
      obtain ⟨hk, e, he, hpe⟩ := ih h
      refine ⟨by omega, e, ?_, hpe⟩
      have : i - k = (i - (k + 1)) + 1 := by omega
      rw [this, List.getElem?_cons_succ]
      exact he

private theorem positions_spec {p : Event → Bool} {t : List Event} {i : Nat}
    (h : i ∈ positions t p) : ∃ e, t[i]? = some e ∧ p e = true := by
  have := positionsFrom_spec (p := p) (t := t) (k := 0) h
  simpa using this.2

/-- A predicate counted exactly once names one event: any two matching members
are equal. -/
private theorem eq_of_countP_eq_one {p : Event → Bool} {t : List Event}
    (hcount : t.countP p = 1) {a b : Event}
    (ha : a ∈ t) (hpa : p a = true) (hb : b ∈ t) (hpb : p b = true) : a = b := by
  have hlen : (t.filter p).length = 1 := by
    rw [← List.countP_eq_length_filter]; exact hcount
  obtain ⟨x, hx⟩ := List.length_eq_one_iff.mp hlen
  have hfa : a ∈ t.filter p := List.mem_filter.mpr ⟨ha, hpa⟩
  have hfb : b ∈ t.filter p := List.mem_filter.mpr ⟨hb, hpb⟩
  rw [hx, List.mem_singleton] at hfa hfb
  rw [hfa, hfb]

private theorem sublist_pair_of_getElem? {a b : Event} :
    ∀ {t : List Event} {i j : Nat}, i < j → t[i]? = some a → t[j]? = some b →
      [a, b].Sublist t := by
  intro t
  induction t with
  | nil => intro i j _ ha _; simp at ha
  | cons x rest ih =>
    intro i j hij ha hb
    cases i with
    | zero =>
      have hax : a = x := by simpa using ha.symm
      subst hax
      cases j with
      | zero => omega
      | succ j' =>
        have hbrest : rest[j']? = some b := by simpa using hb
        have hbmem : b ∈ rest := List.mem_of_getElem? hbrest
        exact List.Sublist.cons_cons a (List.singleton_sublist.mpr hbmem)
    | succ i' =>
      cases j with
      | zero => omega
      | succ j' =>
        have ha' : rest[i']? = some a := by simpa using ha
        have hb' : rest[j']? = some b := by simpa using hb
        exact List.Sublist.cons x (ih (by omega) ha' hb')

/-- The two first-position facts the accepted trace pins: the unique completion
and the unique join each have a singleton position list, and the completion's
index is strictly below the join's. -/
private theorem discharged_positions {t : List Event}
    (h : dischargesCompletionCapsule t = true) :
    ∃ c j, positions t (isCompletionOf 1) = [c] ∧
      positions t (isJoinOf 1) = [j] ∧ c < j := by
  obtain ⟨-, -, -, -, -, hcomp, hjoin, hcross⟩ := discharged_clauses h
  have hclen : (positions t (isCompletionOf 1)).length = 1 := by
    rw [positions_length, hcomp]
  have hjlen : (positions t (isJoinOf 1)).length = 1 := by
    rw [positions_length, hjoin]
  obtain ⟨c, hc⟩ := List.length_eq_one_iff.mp hclen
  obtain ⟨j, hj⟩ := List.length_eq_one_iff.mp hjlen
  refine ⟨c, j, hc, hj, ?_⟩
  unfold returnPrecedesTheJoin at hcross
  rw [hc, hj] at hcross
  simpa using hcross

/-! ## Meaning: what an accepted trace has been shown to satisfy

Each theorem is stated for an arbitrary trace, so the procedure cannot drift
away from the property it is named after. -/

/-- The parent performs exactly the scenario's five facts, in the scenario's
order: spawn, join, read the capsule's payload back, drop, complete. In
particular the parent allocates nothing — the allocation the parent later
reads and releases is not its own. -/
theorem discharged_parent_facts_are_exactly_the_scenario {t : List Event}
    (h : dischargesCompletionCapsule t = true) :
    projection t 0 = [creation, collection, readBack, release, rootResult] :=
  (discharged_clauses h).2.1

/-- The child performs exactly the scenario's two facts, in the scenario's
order: allocate the Buffer, finish carrying it. The child neither reads nor
releases — its allocation departs inside the completion record, which is what
"the allocation remains live in the completion capsule" quantifies over. -/
theorem discharged_child_facts_are_exactly_the_scenario {t : List Event}
    (h : dischargesCompletionCapsule t = true) :
    projection t 1 = [allocation, earlyReturn] :=
  (discharged_clauses h).2.2.1

/-- The moved generation is released exactly once, and that one release is the
parent's own fact (`release = .dropped 0 movedGeneration`, whose actor is
task 0) — the child, which allocated it, never releases it. -/
theorem discharged_release_is_unique {t : List Event}
    (h : dischargesCompletionCapsule t = true) :
    t.countP (isReleaseOf movedGeneration) = 1 ∧ release ∈ t := by
  refine ⟨(discharged_clauses h).2.2.2.2.1, ?_⟩
  have hp := discharged_parent_facts_are_exactly_the_scenario h
  have : release ∈ projection t 0 := by rw [hp]; decide
  exact (List.mem_filter.mp this).1

/-- The join happens, the release happens, and the join precedes the release:
the capsule's content is not reclaimed before the parent has collected it. -/
theorem discharged_join_precedes_the_release {t : List Event}
    (h : dischargesCompletionCapsule t = true) :
    [collection, release].Sublist t := by
  have hp := discharged_parent_facts_are_exactly_the_scenario h
  have chain : [collection, release].Sublist parentOrder := by decide
  have hfilter : (projection t 0).Sublist t := List.filter_sublist
  rw [hp] at hfilter
  exact chain.trans hfilter

/-- The allocation's payload is read back between the join and the release:
the capsule delivered the allocation live, with its content intact
(`readBack` carries the very payload `allocation` created). -/
theorem discharged_read_back_sits_between_join_and_release {t : List Event}
    (h : dischargesCompletionCapsule t = true) :
    [collection, readBack, release].Sublist t := by
  have hp := discharged_parent_facts_are_exactly_the_scenario h
  have chain : [collection, readBack, release].Sublist parentOrder := by decide
  have hfilter : (projection t 0).Sublist t := List.filter_sublist
  rw [hp] at hfilter
  exact chain.trans hfilter

/-- The child's only completion returns `.buffer movedGeneration` — the same
generation the child's only allocation created — so the return is
generation-preserving, and the allocation entering the capsule is the one the
parent later receives. -/
theorem discharged_return_preserves_the_generation {t : List Event}
    (h : dischargesCompletionCapsule t = true) :
    t.countP (isCompletionOf 1) = 1 ∧ earlyReturn ∈ t ∧
      t.countP (isAllocationOf movedGeneration) = 1 ∧ allocation ∈ t ∧
      bufferOf earlyReturn = bufferOf allocation := by
  obtain ⟨-, -, hp1, halloc, -, hcomp, -, -⟩ := discharged_clauses h
  refine ⟨hcomp, ?_, halloc, ?_, by decide⟩
  · have : earlyReturn ∈ projection t 1 := by rw [hp1]; decide
    exact (List.mem_filter.mp this).1
  · have : allocation ∈ projection t 1 := by rw [hp1]; decide
    exact (List.mem_filter.mp this).1

/-- The early finish, as a subtrace: the child's completion sits strictly
before the parent's join. This is the scenario's cross-actor fact — neither
actor's own projection can carry it, which is why the procedure checks it
positionally. -/
theorem discharged_return_precedes_the_join {t : List Event}
    (h : dischargesCompletionCapsule t = true) :
    [earlyReturn, collection].Sublist t := by
  obtain ⟨c, j, hc, hj, hlt⟩ := discharged_positions h
  obtain ⟨-, -, hp1, -, -, hcomp, hjoin, -⟩ := discharged_clauses h
  obtain ⟨e₁, he₁, hpe₁⟩ := positions_spec (hc ▸ List.mem_singleton_self c)
  obtain ⟨e₂, he₂, hpe₂⟩ := positions_spec (hj ▸ List.mem_singleton_self j)
  have hret : earlyReturn ∈ t := by
    have : earlyReturn ∈ projection t 1 := by rw [hp1]; decide
    exact (List.mem_filter.mp this).1
  have hcol : collection ∈ t := by
    have hp0 := discharged_parent_facts_are_exactly_the_scenario h
    have : collection ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1
  have he₁ret : e₁ = earlyReturn :=
    eq_of_countP_eq_one hcomp (List.mem_of_getElem? he₁) hpe₁ hret (by decide)
  have he₂col : e₂ = collection :=
    eq_of_countP_eq_one hjoin (List.mem_of_getElem? he₂) hpe₂ hcol (by decide)
  exact sublist_pair_of_getElem? hlt (he₁ret ▸ he₁) (he₂col ▸ he₂)

/-! ## CT37's reference run is accepted -/

theorem ct37_reference_run_discharges_the_completion_capsule :
    dischargesCompletionCapsule referenceRun = true := by decide

/-! ## Every admitted rendering is accepted

`relate` is the F02 relation between CT37's reference run and an independently
observed rendering of it. The theorem below closes the property over every
trace that relation admits — for arbitrary traces, not an enumerated domain.
The per-actor clauses and the counts follow CT01's closure verbatim; the
cross-actor clause is new here, and is extracted from the relation's own order
admission: `checkOrder`'s completion-before-join conjunct is the fact that no
admitted rendering places the join before the child's completion. -/

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
    (hax : ∀ e ∈ referenceRun, p e = (e == x))
    (hsub : ∀ e ∈ t, e ∈ referenceRun)
    (hdup : hasDuplicate t = false)
    (hmem : x ∈ t) : t.countP p = 1 := by
  have hagree : t.countP p = t.countP (· == x) :=
    countP_congr_on fun e he => hax e (hsub e he)
  have hle := countP_beq_le_one_of_no_duplicates hdup x
  have hge : t.countP (· == x) ≠ 0 := fun hzero =>
    (List.countP_eq_zero.mp hzero x hmem) (by simp)
  omega

private theorem allocation_is_the_unique_allocation_witness :
    ∀ e ∈ referenceRun, isAllocationOf movedGeneration e = (e == allocation) := by
  intro e he
  simp only [referenceRun, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

private theorem release_is_the_unique_release_witness :
    ∀ e ∈ referenceRun, isReleaseOf movedGeneration e = (e == release) := by
  intro e he
  simp only [referenceRun, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

private theorem earlyReturn_is_the_unique_completion_witness :
    ∀ e ∈ referenceRun, isCompletionOf 1 e = (e == earlyReturn) := by
  intro e he
  simp only [referenceRun, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

private theorem collection_is_the_unique_join_witness :
    ∀ e ∈ referenceRun, isJoinOf 1 e = (e == collection) := by
  intro e he
  simp only [referenceRun, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide

/-! ### Extracting the completion-before-join conjunct from `checkOrder`

`checkOrder` is a `firstFailure` over sixteen indexed flags. The transcription
below restates that flag list definitionally (`rfl` — the kernel's own
zeta/beta reduction is the witness that nothing was changed), so that the
seventeenth flag can be addressed by name without editing
`QCore.TaskTraceRelation`. -/

private def completionJoinFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap subjectOf)).any fun child =>
    match (positions events (isJoinOf child)).head? with
    | some join =>
        (positions events (isCompletionOf child)).length != 1
          || ((positions events (isCompletionOf child)).headD 0) > join
    | none => false

private def orderChecks (events : List Event) : List (Nat × Bool) :=
  let buffers := sortNats (events.filterMap bufferOf)
  let children := sortNats (events.filterMap subjectOf)
  let allocations := fun buffer => positions events (isAllocationOf buffer)
  let releases := fun buffer => positions events (isReleaseOf buffer)
  let uses := fun buffer => positions events (mentions buffer)
  let creations := fun child => positions events (isCreationOf child)
  let joins := fun child => positions events (isJoinOf child)
  let roots := positions events (fun event =>
    match event with | .completed task _ => task == 0 | _ => false)
  [ (5, buffers.any fun buffer => (allocations buffer).length > 1)
  , (6, buffers.any fun buffer => (allocations buffer).isEmpty)
  , (7, buffers.any fun buffer =>
          match (allocations buffer).head? with
          | some allocation => (uses buffer).any (fun index => index < allocation)
          | none => false)
  , (8, buffers.any fun buffer =>
          match payloadOf events buffer with
          | some payload => events.any fun event =>
              match event with
              | .read _ candidate value => candidate == buffer && value != payload
              | _ => false
          | none => false)
  , (9, buffers.any fun buffer => (releases buffer).length > 1)
  , (10, buffers.any fun buffer =>
          match (releases buffer).head? with
          | some release => (uses buffer).any (fun index => index > release)
          | none => false)
  , (11, buffers.any fun buffer =>
          !(allocations buffer).isEmpty && (releases buffer).isEmpty)
  , (12, children.any fun child => (creations child).length > 1)
  , (13, children.any fun child => (creations child).isEmpty)
  , (14, children.any fun child =>
          match (creations child).head? with
          | some creation => (positions events (isAbout child)).any (fun i => i < creation)
          | none => false)
  , (15, children.any fun child => (joins child).length > 1)
  , (16, children.any fun child =>
          !(creations child).isEmpty && (joins child).isEmpty)
  , (17, completionJoinFlag events)
  , (18, children.any fun child =>
          match (joins child).head? with
          | some join =>
              (positions events (fun e => isAbout child e && !isReturn e)).any
                (fun index => index > join)
          | none => false)
  , (19, events.any fun event =>
          match event with
          | .loaned borrower buffer =>
              let opened := (positions events (fun e => e == .loaned borrower buffer)).headD 0
              let reads := positions events (fun e =>
                match e with
                | .read task candidate _ => task == borrower && candidate == buffer
                | _ => false)
              let returns := positions events (fun e => e == .returned borrower buffer)
              reads.any (fun index => index < opened)
                || (match (joins borrower).head? with
                    | some join => returns.length != 1 || (returns.headD 0) < join
                    | none => false)
          | _ => false)
  , (20, roots.length != 1 || (roots.headD 0) + 1 != events.length)
  ]

private theorem checkOrder_as_checks (events : List Event) :
    checkOrder events = firstFailure (orderChecks events) := rfl

private theorem firstFailure_ne_zero {checks : List (Nat × Bool)}
    {c : Nat × Bool} (hmem : c ∈ checks) (hflag : c.2 = true)
    (hnz : ∀ d ∈ checks, d.1 ≠ 0) : firstFailure checks ≠ 0 := by
  intro h0
  unfold firstFailure at h0
  cases hfind : checks.find? (fun check => check.2) with
  | none =>
    have := List.find?_eq_none.mp hfind c hmem
    simp [hflag] at this
  | some d =>
    rw [hfind] at h0
    exact hnz d (List.mem_of_find?_eq_some hfind) h0

private theorem orderChecks_indices_nonzero (events : List Event) :
    ∀ d ∈ orderChecks events, d.1 ≠ 0 := by
  intro d hd
  simp only [orderChecks, List.mem_cons, List.not_mem_nil, or_false] at hd
  rcases hd with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
    | rfl | rfl | rfl | rfl | rfl | rfl <;> simp

private theorem completionJoinFlag_false_of_checkOrder_zero {t : List Event}
    (h : checkOrder t = 0) : completionJoinFlag t = false := by
  cases hb : completionJoinFlag t with
  | false => rfl
  | true =>
    refine absurd h ?_
    rw [checkOrder_as_checks]
    exact firstFailure_ne_zero (c := (17, completionJoinFlag t))
      (by simp [orderChecks]) hb (orderChecks_indices_nonzero t)

private theorem mem_insertNat_of {a b : Nat} :
    ∀ {l : List Nat}, b = a ∨ b ∈ l → b ∈ insertNat a l := by
  intro l
  induction l with
  | nil =>
    intro h
    rcases h with rfl | h
    · simp [insertNat]
    · simp at h
  | cons x rest ih =>
    intro h
    simp only [insertNat]
    split
    · next hax =>
      have : a = x := by simpa using hax
      subst this
      rcases h with rfl | h
      · exact List.mem_cons_self ..
      · exact h
    · split
      · next =>
        rcases h with rfl | h
        · exact List.mem_cons_self ..
        · exact List.mem_cons_of_mem a h
      · next =>
        rcases h with rfl | h
        · exact List.mem_cons_of_mem x (ih (Or.inl rfl))
        · rcases List.mem_cons.mp h with rfl | h
          · exact List.mem_cons_self ..
          · exact List.mem_cons_of_mem x (ih (Or.inr h))

private theorem mem_foldl_insertNat {a : Nat} :
    ∀ {l : List Nat} {acc : List Nat}, a ∈ acc ∨ a ∈ l →
      a ∈ l.foldl (fun sorted value => insertNat value sorted) acc := by
  intro l
  induction l with
  | nil =>
    intro acc h
    rcases h with h | h
    · exact h
    · simp at h
  | cons v rest ih =>
    intro acc h
    apply ih
    rcases h with h | h
    · exact Or.inl (mem_insertNat_of (Or.inr h))
    · rcases List.mem_cons.mp h with rfl | h
      · exact Or.inl (mem_insertNat_of (Or.inl rfl))
      · exact Or.inr h

private theorem mem_sortNats {l : List Nat} {a : Nat} (h : a ∈ l) :
    a ∈ sortNats l :=
  mem_foldl_insertNat (Or.inr h)

/-- No event is both a completion and a join: the two pinned positions of an
accepted trace are distinct indices. -/
private theorem no_event_is_completion_and_join {e : Event}
    (hc : isCompletionOf 1 e = true) (hj : isJoinOf 1 e = true) : False := by
  cases e <;> simp [isCompletionOf, isJoinOf] at hc hj

/-- The extraction: an admitted rendering's unique child completion sits at or
before its unique join — `checkOrder`'s own conjunct, read back positionally. -/
private theorem admitted_completion_not_after_join {t : List Event}
    (horder : checkOrder t = 0)
    (hmem : (1 : Nat) ∈ sortNats (t.filterMap subjectOf))
    {c j : Nat}
    (hcomp : positions t (isCompletionOf 1) = [c])
    (hjoin : positions t (isJoinOf 1) = [j]) : c ≤ j := by
  by_cases hle : c ≤ j
  · exact hle
  · exfalso
    have hflag : completionJoinFlag t = true := by
      unfold completionJoinFlag
      refine List.any_eq_true.mpr ⟨1, hmem, ?_⟩
      simp only [hjoin, hcomp, List.head?_cons, List.length_cons,
        List.length_nil, List.headD_cons]
      simp
      omega
    rw [completionJoinFlag_false_of_checkOrder_zero horder] at hflag
    exact absurd hflag (by decide)

theorem every_admitted_rendering_discharges_the_completion_capsule
    (t : List Event) (h : relate referenceRun t = 0) :
    dischargesCompletionCapsule t = true := by
  have hattr := admitted_traces_are_attributable referenceRun t h
  have hdup := admitted_traces_have_distinct_occurrences referenceRun t h
  have horder := admitted_traces_satisfy_the_order_constraints referenceRun t h
  have hlen : t.length = 7 := by
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
      projection t 1 = projection referenceRun 1 := by
    have hs : sortNats (referenceRun.filterMap actorOf) = [0, 1] := by decide
    have hfalse := Bool.not_eq_true _ ▸ hactors
    rw [hs] at hfalse
    simp [List.any, Bool.or_eq_false_iff] at hfalse
    exact hfalse
  have hp0 : projection t 0 = parentOrder :=
    hprojections.1.trans parent_projection_of_the_reference
  have hp1 : projection t 1 = childOrder :=
    hprojections.2.trans child_projection_of_the_reference
  have hallocation : allocation ∈ t := by
    have : allocation ∈ projection t 1 := by rw [hp1]; decide
    exact (List.mem_filter.mp this).1
  have hrelease : release ∈ t := by
    have : release ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1
  have hreturn : earlyReturn ∈ t := by
    have : earlyReturn ∈ projection t 1 := by rw [hp1]; decide
    exact (List.mem_filter.mp this).1
  have hcollection : collection ∈ t := by
    have : collection ∈ projection t 0 := by rw [hp0]; decide
    exact (List.mem_filter.mp this).1
  have hcAlloc := countP_eq_one_of_unique_witness
    allocation_is_the_unique_allocation_witness hsub hdup hallocation
  have hcRelease := countP_eq_one_of_unique_witness
    release_is_the_unique_release_witness hsub hdup hrelease
  have hcReturn := countP_eq_one_of_unique_witness
    earlyReturn_is_the_unique_completion_witness hsub hdup hreturn
  have hcJoin := countP_eq_one_of_unique_witness
    collection_is_the_unique_join_witness hsub hdup hcollection
  -- The cross-actor clause, from the relation's own order admission.
  obtain ⟨c, hcpos⟩ : ∃ c, positions t (isCompletionOf 1) = [c] := by
    have hl : (positions t (isCompletionOf 1)).length = 1 := by
      rw [positions_length, hcReturn]
    exact List.length_eq_one_iff.mp hl
  obtain ⟨j, hjpos⟩ : ∃ j, positions t (isJoinOf 1) = [j] := by
    have hl : (positions t (isJoinOf 1)).length = 1 := by
      rw [positions_length, hcJoin]
    exact List.length_eq_one_iff.mp hl
  have hmem1 : (1 : Nat) ∈ sortNats (t.filterMap subjectOf) :=
    mem_sortNats (List.mem_filterMap.mpr ⟨collection, hcollection, by decide⟩)
  have hle : c ≤ j := admitted_completion_not_after_join horder hmem1 hcpos hjpos
  have hne : c ≠ j := by
    intro hcj
    subst hcj
    obtain ⟨e₁, he₁, hpe₁⟩ := positions_spec (hcpos ▸ List.mem_singleton_self c)
    obtain ⟨e₂, he₂, hpe₂⟩ := positions_spec (hjpos ▸ List.mem_singleton_self c)
    have : e₁ = e₂ := by
      have := he₁.symm.trans he₂
      simpa using this
    exact no_event_is_completion_and_join hpe₁ (this ▸ hpe₂)
  have hrptj : returnPrecedesTheJoin t = true := by
    unfold returnPrecedesTheJoin
    rw [hcpos, hjpos]
    simp only [List.head?_cons]
    simpa using Nat.lt_of_le_of_ne hle hne
  simp only [dischargesCompletionCapsule, hlen, hp0, hp1, hcAlloc, hcRelease,
    hcReturn, hcJoin, hrptj]
  decide

/-! ## Named refusals: the procedure is not vacuous

Each mutant is CT37's reference run with exactly one scenario fact broken. -/

/-- The join moved before the child's return — the anti-"finishes early" shape,
with every per-actor projection and every count intact: refused, and only the
positional clause refuses it. -/
theorem a_join_before_the_return_is_refused :
    dischargesCompletionCapsule
      [creation, allocation, collection, earlyReturn, readBack, release,
        rootResult] = false := by decide

/-- The release removed — the leak shape: refused. -/
theorem a_missing_release_is_refused :
    dischargesCompletionCapsule
      [creation, allocation, earlyReturn, collection, readBack, rootResult]
      = false := by decide

/-- The child releases its allocation instead of returning it live — the
emptied capsule: refused. -/
theorem a_release_by_the_child_is_refused :
    dischargesCompletionCapsule
      [creation, allocation, .dropped 1 movedGeneration, earlyReturn,
        collection, readBack, rootResult] = false := by decide

/-- A second release of the moved generation: refused. -/
theorem a_double_release_is_refused :
    dischargesCompletionCapsule
      [creation, allocation, earlyReturn, collection, readBack, release,
        .dropped 1 movedGeneration, rootResult] = false := by decide

/-- A child return carrying a different generation: refused. -/
theorem a_generation_changing_return_is_refused :
    dischargesCompletionCapsule
      [creation, allocation, .completed 1 (.buffer 4294967298), collection,
        readBack, release, rootResult] = false := by decide

/-- A capsule that loses the allocation's payload — the parent reads back a
value the allocation never carried: refused. -/
theorem a_payload_losing_capsule_is_refused :
    dischargesCompletionCapsule
      [creation, allocation, earlyReturn, collection,
        .read 0 movedGeneration 7, release, rootResult] = false := by decide

end QCore.Ct37CompletionCapsule
