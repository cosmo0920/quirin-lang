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
# CT50 — the lifetime-identity discharge, proved on the case's own trace terms

CT50's scenario is `New lifetime reuses old numeric address/descriptor` (spec
§55, semantic profile `ScopedOwnedTasksV1`, expected *Old capability cannot
authorize new identity*). Its artifact is
`compiler/tests/fixtures/ct/CT50/case.qr` — one root task that allocates a
Buffer carrying `7`, releases it, allocates a second Buffer carrying `42`,
reads that one back and releases it — and the named workspace test
`compiler/tests/ct50_proof_leg.rs::ct50_artifact_is_the_lean_proved_lifetime_identity_shape`
compiles that fixture through the real pipeline, executes the checked task
module, derives the admitted causal order with the production observer and
checks the artifact against `referenceRun` and against the transcription of
`dischargesLifetimeIdentity` below, with `ct50_trace_digest` recomputed from
the artifact's own events by the F02 digest fold.

`QCore.Ct01OwnedTransfer` is imported **only** for the registered
`ReflBEq`/`LawfulBEq` instances on the F02 alphabet; no CT01 definition is
reused, and this module adds nothing to `QCore.TaskTraceRelation`.

What this file establishes:

* `dischargesLifetimeIdentity` — one **total decision procedure** for the
  scenario's property over the F02 observation alphabet, with meaning theorems
  that keep it honest: an accepted trace allocates and releases each of the two
  generations exactly once; the **old generation's whole existence closes
  before the new one begins** (`[staleAllocation, staleRelease,
  freshAllocation]` is a subtrace); **no fact of the trace uses the old
  generation** — no read of it, and no completion carrying it out
  (`t.countP isStaleUse = 0`); the one read names the **new** generation and
  carries the new allocation's own payload, not the old one's; and that read
  sits inside the new generation's own lifetime.
* `ct50_reference_run_discharges_the_lifetime_identity` — CT50's own reference
  run is accepted.
* `an_admitted_rendering_is_the_reference_run` — CT50's run has **one actor**,
  and for a one-actor reference the F02 relation admits nothing but the run
  itself. The closure
  `every_admitted_rendering_discharges_the_lifetime_identity` is therefore the
  degenerate case of CT01's and CT37's closures, and it is **proved** here
  rather than assumed.
* Eight named refusals, four of which are proved to fail **exactly one** named
  clause with every other clause still holding, so the clause that catches each
  mutation is measured rather than asserted.

What this file does **not** establish. CT50's scenario names a *numeric
address*: the native run places the second mapping at the first one's address
with `MAP_FIXED_NOREPLACE`, and that reuse is **invisible to the F02
alphabet**, which carries generations and not addresses. The property proved
here is the capability-level half — two disjoint generation lifetimes, and no
use of the old generation anywhere in the run — and the address-level half
stays with the existing `run_native_task_scenarios` observation, unproved. Nor
is anything claimed about the emitted C's native execution, about the Rust
pipeline's correctness, or about programs outside this profile.
-/

namespace QCore.Ct50LifetimeIdentity
open QCore.TaskTraceRelation

/-! ## CT50's six facts, named -/

/-- The first lifetime's generation: allocated, then released before the
second allocation runs. -/
def stale : Nat := 1

/-- The second lifetime's generation. The native run places it at the first
one's numeric address; the alphabet below sees only that it is a different
generation. -/
def fresh : Nat := 2

def staleAllocation : Event := .allocated 0 stale 7
def staleRelease : Event := .dropped 0 stale
def freshAllocation : Event := .allocated 0 fresh 42
def freshRead : Event := .read 0 fresh 42
def freshRelease : Event := .dropped 0 fresh
def rootResult : Event := .completed 0 (.i32 42)

/-- CT50's reference run, in the order the reference execution performs it. -/
def referenceRun : List Event :=
  [staleAllocation, staleRelease, freshAllocation, freshRead, freshRelease,
    rootResult]

theorem ct50_reference_run_is_an_admitted_causal_order :
    declare referenceRun = 0 := by decide

/-- Every fact of the run is the root task's: CT50 spawns nothing. -/
theorem root_projection_of_the_reference :
    projection referenceRun 0 = referenceRun := by decide

/-- The transcription pin the Rust runner recomputes from CT50's own artifact
with wrapping `u64` arithmetic (the F02 digest fold). -/
theorem ct50_trace_digest : mixTrace referenceRun = 1699395409980170611 := by
  decide

/-! ## The vocabulary the procedure needs -/

def isRead : Event → Bool
  | .read _ _ _ => true
  | _ => false

def isReadOf (buffer : Nat) : Event → Bool
  | .read _ candidate _ => candidate == buffer
  | _ => false

/-- A read of `buffer` that carries `payload`. -/
def readsBack (buffer payload : Nat) : Event → Bool
  | .read _ candidate value => candidate == buffer && value == payload
  | _ => false

/-- A **use** of the old capability: a fact that names the stale generation and
is neither that generation's own allocation nor its own release. A read of it,
a loan of it, a return of it, or a completion carrying it out of the task are
all uses; there is no other way for a fact to name a generation. -/
def isStaleUse (event : Event) : Bool :=
  mentions stale event && !isAllocationOf stale event
    && !isReleaseOf stale event

/-! ## The decision procedure -/

/-- Total decision procedure for CT50's scenario property over the F02
alphabet. A trace is accepted exactly when it reports six facts, all of them
the root task's; each generation is allocated once and released once; **no fact
uses the stale generation**; there is exactly one read and it names the fresh
generation carrying the fresh allocation's own payload; the stale generation's
allocation and release both precede the fresh allocation; and the read sits
between the fresh allocation and the fresh release. The Rust transcription in
`compiler/tests/ct50_proof_leg.rs` runs this same procedure on the artifact. -/
def dischargesLifetimeIdentity (t : List Event) : Bool :=
  decide (t.length = 6)
    && decide (projection t 0 = t)
    && decide (t.countP (isAllocationOf stale) = 1)
    && decide (t.countP (isReleaseOf stale) = 1)
    && decide (t.countP (isAllocationOf fresh) = 1)
    && decide (t.countP (isReleaseOf fresh) = 1)
    && decide (t.countP isStaleUse = 0)
    && decide (t.countP isRead = 1)
    && decide (t.countP (readsBack fresh 42) = 1)
    && decide (List.Sublist [staleAllocation, staleRelease, freshAllocation] t)
    && decide (List.Sublist [freshAllocation, freshRead, freshRelease] t)

private theorem discharged_clauses {t : List Event}
    (h : dischargesLifetimeIdentity t = true) :
    t.length = 6 ∧ projection t 0 = t
      ∧ t.countP (isAllocationOf stale) = 1
      ∧ t.countP (isReleaseOf stale) = 1
      ∧ t.countP (isAllocationOf fresh) = 1
      ∧ t.countP (isReleaseOf fresh) = 1
      ∧ t.countP isStaleUse = 0
      ∧ t.countP isRead = 1
      ∧ t.countP (readsBack fresh 42) = 1
      ∧ List.Sublist [staleAllocation, staleRelease, freshAllocation] t
      ∧ List.Sublist [freshAllocation, freshRead, freshRelease] t := by
  simp only [dischargesLifetimeIdentity, Bool.and_eq_true, decide_eq_true_eq]
    at h
  exact ⟨h.1.1.1.1.1.1.1.1.1.1, h.1.1.1.1.1.1.1.1.1.2, h.1.1.1.1.1.1.1.1.2,
    h.1.1.1.1.1.1.1.2, h.1.1.1.1.1.1.2, h.1.1.1.1.1.2, h.1.1.1.1.2,
    h.1.1.1.2, h.1.1.2, h.1.2, h.2⟩

/-! ## Meaning: what an accepted trace has been shown to satisfy

Each theorem is stated for an arbitrary trace, so the procedure cannot drift
away from the property it is named after. -/

/-- Each generation is allocated exactly once and released exactly once: no
leak, no double release, on either lifetime. -/
theorem discharged_each_generation_is_allocated_and_released_once
    {t : List Event} (h : dischargesLifetimeIdentity t = true) :
    t.countP (isAllocationOf stale) = 1 ∧ t.countP (isReleaseOf stale) = 1
      ∧ t.countP (isAllocationOf fresh) = 1
      ∧ t.countP (isReleaseOf fresh) = 1 := by
  obtain ⟨-, -, h1, h2, h3, h4, -⟩ := discharged_clauses h
  exact ⟨h1, h2, h3, h4⟩

/-- **The old lifetime closes before the new one opens.** The stale
generation's allocation and its release both precede the fresh generation's
allocation, as a subtrace of any accepted trace. -/
theorem discharged_the_old_lifetime_closes_before_the_new_one_opens
    {t : List Event} (h : dischargesLifetimeIdentity t = true) :
    List.Sublist [staleAllocation, staleRelease, freshAllocation] t :=
  (discharged_clauses h).2.2.2.2.2.2.2.2.2.1

/-- **No fact of the trace uses the old capability.** Every fact naming the
stale generation is that generation's own allocation or its own release; there
is no read of it, no loan of it, no return of it and no completion carrying it
out. This is the case's expected outcome — *old capability cannot authorize new
identity* — at the level the alphabet can see. -/
theorem discharged_no_fact_uses_the_old_capability {t : List Event}
    (h : dischargesLifetimeIdentity t = true) : t.countP isStaleUse = 0 :=
  (discharged_clauses h).2.2.2.2.2.2.1

/-- A consequence of the clause above, spelled out: the stale generation is
never read. -/
theorem discharged_the_old_generation_is_never_read {t : List Event}
    (h : dischargesLifetimeIdentity t = true) :
    t.countP (isReadOf stale) = 0 := by
  have hzero := discharged_no_fact_uses_the_old_capability h
  rw [List.countP_eq_zero] at hzero ⊢
  intro event mem
  have huse := hzero event mem
  intro hread
  refine huse ?_
  cases event with
  | read _ candidate _ =>
    simp only [isReadOf] at hread
    simp [isStaleUse, mentions, bufferOf, isAllocationOf, isReleaseOf, hread]
  | allocated _ _ _ => simp [isReadOf] at hread
  | allocationFailed _ => simp [isReadOf] at hread
  | spawned _ _ => simp [isReadOf] at hread
  | creationFailed _ => simp [isReadOf] at hread
  | dropped _ _ => simp [isReadOf] at hread
  | loaned _ _ => simp [isReadOf] at hread
  | returned _ _ => simp [isReadOf] at hread
  | completed _ _ => simp [isReadOf] at hread
  | joined _ => simp [isReadOf] at hread

/-- **The one read names the new identity, with the new allocation's own
payload.** Exactly one read happens, and the counted read is of the fresh
generation carrying `42` — the value the fresh allocation created, not the
stale one's `7`. -/
theorem discharged_the_one_read_names_the_new_generation {t : List Event}
    (h : dischargesLifetimeIdentity t = true) :
    t.countP isRead = 1 ∧ t.countP (readsBack fresh 42) = 1
      ∧ freshRead ∈ t := by
  obtain ⟨-, -, -, -, -, -, -, hreads, hfresh, -, hchain⟩ :=
    discharged_clauses h
  exact ⟨hreads, hfresh, hchain.subset (by decide)⟩

/-- The read happens inside the new generation's own lifetime: after its
allocation and before its release. -/
theorem discharged_the_read_sits_inside_the_new_lifetime {t : List Event}
    (h : dischargesLifetimeIdentity t = true) :
    List.Sublist [freshAllocation, freshRead, freshRelease] t :=
  (discharged_clauses h).2.2.2.2.2.2.2.2.2.2

/-- Every fact of an accepted trace is the root task's: this scenario has one
actor. -/
theorem discharged_every_fact_is_the_root_tasks {t : List Event}
    (h : dischargesLifetimeIdentity t = true) : projection t 0 = t :=
  (discharged_clauses h).2.1

/-! ### One clause is **not** independent, and that is stated rather than left
to be discovered

The card's mutation harness weakened the payload clause
`t.countP (readsBack fresh 42) = 1` to the bare read count and **no test
reddened** (`R5`). The reason is not that the mutants are thin: the clause is
*implied* by the two clauses beside it, so no trace can separate them. That is
published here as a theorem rather than recorded as a gap — and the clause is
kept, because it is the sentence the scenario is about and the meaning theorem
above reads it directly. -/

private theorem countP_le_of_implies {p q : Event → Bool} :
    ∀ {t : List Event}, (∀ e ∈ t, p e = true → q e = true) →
      t.countP p ≤ t.countP q := by
  intro t
  induction t with
  | nil => intro _; simp
  | cons a rest ih =>
    intro h
    have hrest := ih fun e he => h e (List.mem_cons_of_mem a he)
    have ha := h a (List.mem_cons_self ..)
    by_cases hp : p a = true
    · have hq : q a = true := ha hp
      simp [hp, hq]
      omega
    · simp only [Bool.not_eq_true] at hp
      simp [List.countP_cons, hp]
      omega

theorem the_payload_clause_is_implied_by_the_read_window_and_the_read_count
    {t : List Event} (hwindow : List.Sublist [freshAllocation, freshRead,
      freshRelease] t) (hreads : t.countP isRead = 1) :
    t.countP (readsBack fresh 42) = 1 := by
  have hmem : freshRead ∈ t := hwindow.subset (by decide)
  have hle : t.countP (readsBack fresh 42) ≤ t.countP isRead := by
    refine countP_le_of_implies ?_
    intro e _ he
    cases e with
    | read _ _ _ => rfl
    | allocated _ _ _ => simp [readsBack] at he
    | allocationFailed _ => simp [readsBack] at he
    | spawned _ _ => simp [readsBack] at he
    | creationFailed _ => simp [readsBack] at he
    | dropped _ _ => simp [readsBack] at he
    | loaned _ _ => simp [readsBack] at he
    | returned _ _ => simp [readsBack] at he
    | completed _ _ => simp [readsBack] at he
    | joined _ => simp [readsBack] at he
  have hpos : t.countP (readsBack fresh 42) ≠ 0 := fun hzero =>
    (List.countP_eq_zero.mp hzero freshRead hmem) (by decide)
  omega

/-! ## CT50's reference run is accepted -/

theorem ct50_reference_run_discharges_the_lifetime_identity :
    dischargesLifetimeIdentity referenceRun = true := by decide

/-! ## Every admitted rendering is accepted — the degenerate case, proved

CT50's reference run has a single actor. The F02 relation demands that an
observed rendering report the same multiset of facts and agree with the
reference on **every actor's** projection; with one actor that forces the
rendering to be the reference run itself. So the closure here is not an
appeal to a symbolic argument over per-actor clauses, as CT01's and CT37's
are, but the statement that the admitted set is a singleton — which the Rust
runner also checks on the artifact by enumerating the relation's admitted
linearizations. -/

theorem an_admitted_rendering_is_the_reference_run (t : List Event)
    (h : relate referenceRun t = 0) : t = referenceRun := by
  have hattr := admitted_traces_are_attributable referenceRun t h
  have hdup := admitted_traces_have_distinct_occurrences referenceRun t h
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
  have hprojection : projection t 0 = projection referenceRun 0 := by
    have hs : sortNats (referenceRun.filterMap actorOf) = [0] := by decide
    have hfalse := Bool.not_eq_true _ ▸ hactors
    rw [hs] at hfalse
    simp [List.any] at hfalse
    exact hfalse
  have hself : projection t 0 = t := by
    refine List.filter_eq_self.mpr ?_
    intro e he
    have hmem := hsub e he
    simp only [referenceRun, List.mem_cons, List.not_mem_nil, or_false] at hmem
    rcases hmem with rfl | rfl | rfl | rfl | rfl | rfl <;> decide
  rw [← hself, hprojection, root_projection_of_the_reference]

/-- The closure, in the shape CT01 and CT37 state it: every rendering the F02
relation admits against CT50's reference run discharges the scenario. Here it
follows from the singleton fact above. -/
theorem every_admitted_rendering_discharges_the_lifetime_identity
    (t : List Event) (h : relate referenceRun t = 0) :
    dischargesLifetimeIdentity t = true := by
  rw [an_admitted_rendering_is_the_reference_run t h]
  exact ct50_reference_run_discharges_the_lifetime_identity

/-! ## Named refusals: the procedure is not vacuous

The first four mutants are CT50's reference run with exactly one scenario fact
broken and **nothing else changed** — each keeps the trace's length, its actor
and every count the other clauses read — so for each one the clause that
catches it is measured rather than asserted (the `…_fails_only_…` theorems
below list every other clause as still holding). The last four change the
trace's length as well, and are named as such. -/

/-- The two lifetimes overlap: the fresh generation is allocated while the
stale one is still live. -/
def overlappingLifetimes : List Event :=
  [staleAllocation, freshAllocation, staleRelease, freshRead, freshRelease,
    rootResult]

theorem overlapping_lifetimes_are_refused :
    dischargesLifetimeIdentity overlappingLifetimes = false := by decide

/-- …and the clause that refuses it is the old lifetime's closure: every other
clause of the procedure still holds of this mutant. -/
theorem overlapping_lifetimes_fail_only_the_old_lifetime_clause :
    overlappingLifetimes.length = 6
      ∧ projection overlappingLifetimes 0 = overlappingLifetimes
      ∧ overlappingLifetimes.countP (isAllocationOf stale) = 1
      ∧ overlappingLifetimes.countP (isReleaseOf stale) = 1
      ∧ overlappingLifetimes.countP (isAllocationOf fresh) = 1
      ∧ overlappingLifetimes.countP (isReleaseOf fresh) = 1
      ∧ overlappingLifetimes.countP isStaleUse = 0
      ∧ overlappingLifetimes.countP isRead = 1
      ∧ overlappingLifetimes.countP (readsBack fresh 42) = 1
      ∧ List.Sublist [freshAllocation, freshRead, freshRelease]
          overlappingLifetimes
      ∧ ¬ List.Sublist [staleAllocation, staleRelease, freshAllocation]
          overlappingLifetimes := by decide

/-- The old capability escapes: the task's result carries the stale generation
out after that generation has been released. -/
def escapingStaleCapability : List Event :=
  [staleAllocation, staleRelease, freshAllocation, freshRead, freshRelease,
    .completed 0 (.buffer stale)]

theorem an_escaping_stale_capability_is_refused :
    dischargesLifetimeIdentity escapingStaleCapability = false := by decide

/-- …and the clause that refuses it is the stale-use clause; every other
clause still holds. -/
theorem an_escaping_stale_capability_fails_only_the_stale_use_clause :
    escapingStaleCapability.length = 6
      ∧ projection escapingStaleCapability 0 = escapingStaleCapability
      ∧ escapingStaleCapability.countP (isAllocationOf stale) = 1
      ∧ escapingStaleCapability.countP (isReleaseOf stale) = 1
      ∧ escapingStaleCapability.countP (isAllocationOf fresh) = 1
      ∧ escapingStaleCapability.countP (isReleaseOf fresh) = 1
      ∧ escapingStaleCapability.countP isRead = 1
      ∧ escapingStaleCapability.countP (readsBack fresh 42) = 1
      ∧ List.Sublist [staleAllocation, staleRelease, freshAllocation]
          escapingStaleCapability
      ∧ List.Sublist [freshAllocation, freshRead, freshRelease]
          escapingStaleCapability
      ∧ escapingStaleCapability.countP isStaleUse = 1 := by decide

/-- The read happens after the fresh generation's release — a use after the
new identity has itself been given up. -/
def readAfterTheFreshRelease : List Event :=
  [staleAllocation, staleRelease, freshAllocation, freshRelease, freshRead,
    rootResult]

theorem a_read_after_the_fresh_release_is_refused :
    dischargesLifetimeIdentity readAfterTheFreshRelease = false := by decide

/-- …and the clause that refuses it is the read's position inside the new
lifetime; every other clause still holds. -/
theorem a_read_after_the_fresh_release_fails_only_the_read_window_clause :
    readAfterTheFreshRelease.length = 6
      ∧ projection readAfterTheFreshRelease 0 = readAfterTheFreshRelease
      ∧ readAfterTheFreshRelease.countP (isAllocationOf stale) = 1
      ∧ readAfterTheFreshRelease.countP (isReleaseOf stale) = 1
      ∧ readAfterTheFreshRelease.countP (isAllocationOf fresh) = 1
      ∧ readAfterTheFreshRelease.countP (isReleaseOf fresh) = 1
      ∧ readAfterTheFreshRelease.countP isStaleUse = 0
      ∧ readAfterTheFreshRelease.countP isRead = 1
      ∧ readAfterTheFreshRelease.countP (readsBack fresh 42) = 1
      ∧ List.Sublist [staleAllocation, staleRelease, freshAllocation]
          readAfterTheFreshRelease
      ∧ ¬ List.Sublist [freshAllocation, freshRead, freshRelease]
          readAfterTheFreshRelease := by decide

/-- The task's own result is attributed to a child: this run has no child, so
the fact belongs to nobody. -/
def resultClaimedByASecondActor : List Event :=
  [staleAllocation, staleRelease, freshAllocation, freshRead, freshRelease,
    .completed 1 (.i32 42)]

theorem a_result_claimed_by_a_second_actor_is_refused :
    dischargesLifetimeIdentity resultClaimedByASecondActor = false := by decide

/-- …and the clause that refuses it is the single-actor clause; **every other
clause of the procedure** still holds. -/
theorem a_result_claimed_by_a_second_actor_fails_only_the_single_actor_clause :
    resultClaimedByASecondActor.length = 6
      ∧ resultClaimedByASecondActor.countP (isAllocationOf stale) = 1
      ∧ resultClaimedByASecondActor.countP (isReleaseOf stale) = 1
      ∧ resultClaimedByASecondActor.countP (isAllocationOf fresh) = 1
      ∧ resultClaimedByASecondActor.countP (isReleaseOf fresh) = 1
      ∧ resultClaimedByASecondActor.countP isStaleUse = 0
      ∧ resultClaimedByASecondActor.countP isRead = 1
      ∧ resultClaimedByASecondActor.countP (readsBack fresh 42) = 1
      ∧ List.Sublist [staleAllocation, staleRelease, freshAllocation]
          resultClaimedByASecondActor
      ∧ List.Sublist [freshAllocation, freshRead, freshRelease]
          resultClaimedByASecondActor
      ∧ projection resultClaimedByASecondActor 0 ≠ resultClaimedByASecondActor
      := by decide

/-- A *read* attributed to a second actor is refused too, but it is **not** an
isolating mutant and is not presented as one: moving the read off the root task
also moves it out of the read-window chain, so two clauses catch it. Measured
by the card's harness (`R4`, which reddened nothing when the single-actor
clause alone was removed) and recorded here rather than papered over. -/
def readByASecondActor : List Event :=
  [staleAllocation, staleRelease, freshAllocation, .read 1 fresh 42,
    freshRelease, rootResult]

theorem a_read_by_a_second_actor_is_refused :
    dischargesLifetimeIdentity readByASecondActor = false := by decide

theorem a_read_by_a_second_actor_fails_two_clauses :
    projection readByASecondActor 0 ≠ readByASecondActor
      ∧ ¬ List.Sublist [freshAllocation, freshRead, freshRelease]
          readByASecondActor := by decide

/-- The read names the **old** generation after the new one exists: the
scenario's own crux, stated at the level the alphabet sees. -/
theorem a_read_of_the_stale_generation_is_refused :
    dischargesLifetimeIdentity
      [staleAllocation, staleRelease, freshAllocation, .read 0 stale 7,
        freshRelease, rootResult] = false := by decide

/-- The read names the new generation but carries the **old** payload. -/
theorem a_read_carrying_the_old_payload_is_refused :
    dischargesLifetimeIdentity
      [staleAllocation, staleRelease, freshAllocation, .read 0 fresh 7,
        freshRelease, rootResult] = false := by decide

/-- The stale generation is never released — the leak shape. Shorter than the
reference run, so the length clause refuses it beside the release count. -/
theorem a_leaked_stale_generation_is_refused :
    dischargesLifetimeIdentity
      [staleAllocation, freshAllocation, freshRead, freshRelease, rootResult]
      = false := by decide

/-- The fresh generation is released twice. Longer than the reference run, so
the length clause refuses it beside the release count. -/
theorem a_double_release_of_the_fresh_generation_is_refused :
    dischargesLifetimeIdentity
      [staleAllocation, staleRelease, freshAllocation, freshRead, freshRelease,
        .dropped 0 fresh, rootResult] = false := by decide

end QCore.Ct50LifetimeIdentity
