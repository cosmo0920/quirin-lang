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

import QCore.AppRkCallbacks

/-!
# The member pre-pass on every route (card MEMBER-PREPASS-OTHER-ROUTES)

Gateway rule 12n, amended in place; user decisions R1 and F1 of 2026-10-14.
The pinned route's member-facts pre-pass now also runs on the current and
no-build routes, a checkpoint carries the run's facts, and the profile's
scope file is re-checked at run time. Abstract rule theorems, not an
extraction of the Rust analyzer (`crates/qcore-audit/src/member_prepass.rs`,
`crates/qcore-audit/src/compose.rs`,
`crates/qcore-audit/src/callback_member_fold.rs`).

* **The scope re-check.** An exclusion holds only while its evidence stands
  and no evidence file names a final token
  (`a_named_token_drops_the_exclusion`,
  `failed_evidence_drops_the_exclusion`). A dropped exclusion reads as its
  unit (`a_dropped_exclusion_reads_as_the_unit`), so **a stale exclusion
  never closes a member** (`a_stale_exclusion_never_closes_a_member`), and
  exclusions that hold change nothing under C's rule that a unit naming no
  token stores nothing (`held_exclusions_keep_the_run_set`).
* **The checkpoint carrier.** A composer whose checkpoints carry no run facts
  refuses a run whose contracts invoke a member
  (`a_checkpoint_without_facts_refuses`); with the run's closure carried, a
  composition that succeeds publishes exactly the run's fold
  (`the_composed_decision_is_the_runs`), and a run whose units decided over
  that closure always composes (`a_run_decided_over_its_facts_composes`).
-/

namespace QCore.MemberPrepassRoutes

open QCore.OpCallbacks (Closure UnitReading runClosure foldKeeps)

/-! ## The scope re-check -/

/-- The re-check of one exclusion: its evidence stands (`ok`: the link lines
say so, or every unresolved include is named as generated and every source
exists) and no evidence file names a final token. -/
def recheck (ok : Bool) (namesToken : List Bool) : Bool :=
  ok && namesToken.all (· == false)

/-- An evidence file that names a final token drops the exclusion. -/
theorem a_named_token_drops_the_exclusion {ok : Bool} {fs : List Bool} (h : true ∈ fs) :
    recheck ok fs = false := by
  unfold recheck
  cases ok with
  | false => rfl
  | true =>
    simp only [Bool.true_and, List.all_eq_false]
    exact ⟨true, h, by decide⟩

/-- Evidence that cannot be checked drops the exclusion, whatever the
tokens. -/
theorem failed_evidence_drops_the_exclusion (fs : List Bool) : recheck false fs = false := rfl

/-- One excluded unit at run time: whether its exclusion held against the
final tokens, and what the unit reads as when read like any other (`none`:
it cannot be read, a failed preprocessing or a unit that is not C). -/
structure Exclusion where
  holds : Bool
  reading : UnitReading

/-- What an excluded unit contributes to the run set: nothing while its
exclusion holds, otherwise its own reading. -/
def contribution (e : Exclusion) : UnitReading :=
  if e.holds then some (.closed []) else e.reading

/-- The pre-pass's run set over the read units and the scope's exclusions. -/
def prepassClosure (units : List UnitReading) (excluded : List Exclusion) : Closure :=
  runClosure (units ++ excluded.map contribution)

/-- A dropped exclusion reads as the unit itself. -/
theorem a_dropped_exclusion_reads_as_the_unit (e : Exclusion) (h : e.holds = false) :
    contribution e = e.reading := by
  simp [contribution, h]

/-- **A stale exclusion never closes a member**: a dropped exclusion whose
unit cannot be read keeps the run set open. -/
theorem a_stale_exclusion_never_closes_a_member {units : List UnitReading}
    {excluded : List Exclusion} {e : Exclusion} (he : e ∈ excluded)
    (hh : e.holds = false) (hr : e.reading = none) :
    prepassClosure units excluded = .opened := by
  apply QCore.OpCallbacks.an_unread_unit_keeps_the_member_open
  apply List.mem_append.mpr
  right
  exact List.mem_map.mpr ⟨e, he, by rw [a_dropped_exclusion_reads_as_the_unit e hh, hr]⟩

/-- **Exclusions that hold change nothing**: when every exclusion that holds
is true of its unit (the unit's reading is the empty closed set: under C's
rule that a member is reached only through its record's declaration and a
function only through its name, a unit naming no token stores nothing), the
pre-pass's set is the set over every unit read. -/
theorem held_exclusions_keep_the_run_set {units : List UnitReading}
    {excluded : List Exclusion}
    (h : ∀ e ∈ excluded, e.holds = true → e.reading = some (.closed [])) :
    prepassClosure units excluded = runClosure (units ++ excluded.map (·.reading)) := by
  unfold prepassClosure
  congr 2
  apply List.map_congr_left
  intro e he
  unfold contribution
  cases hh : e.holds with
  | false => rfl
  | true => simp [h e he hh]

/-! ## The checkpoint carrier -/

/-- A closed closure keeps itself in the fold. -/
theorem a_closed_closure_keeps_itself (ns : List String) :
    foldKeeps (.closed ns) (.closed ns) = true :=
  (QCore.AppRkCallbacks.the_fold_keeps_a_run_decision_exactly_when_closed (.closed ns)).mpr
    ⟨ns, rfl⟩

/-- The run's fold over the rows resting on a member invocation (each `true`
when the row is `Safe` or `Pending`): with the run's facts installed every
unit decided over the run's closure `c`, so a row stays exactly when `c`
keeps itself. -/
def runFold (c : Closure) (kept : List Bool) : List Bool :=
  kept.map (fun k => k && foldKeeps c c)

/-- The composer over what the checkpoints carry: refused (`none`) when the
contracts invoke a member and no run closure is carried; with the carried
closure, refused when its fold would demote a row the checkpoints hold
(the composer does not rewrite rows), otherwise the rows as held. -/
def compose (invokes : Bool) (carried : Option Closure) (kept : List Bool) :
    Option (List Bool) :=
  match carried with
  | none => if invokes then none else some kept
  | some c => if kept.any (fun k => k && !foldKeeps c c) then none else some kept

/-- **A checkpoint without the run's facts refuses** a run whose contracts
invoke a member, as before. -/
theorem a_checkpoint_without_facts_refuses (kept : List Bool) :
    compose true none kept = none := rfl

/-- **The checkpoint-with-facts decision equals the run's decision**: a
composition over the carried closure that succeeds publishes exactly the
run's fold of the rows. -/
theorem the_composed_decision_is_the_runs {invokes : Bool} {c : Closure}
    {kept out : List Bool} (h : compose invokes (some c) kept = some out) :
    out = runFold c kept := by
  simp only [compose] at h
  by_cases hany : kept.any (fun k => k && !foldKeeps c c) = true
  · simp [hany] at h
  · simp [hany] at h
    have hall := List.any_eq_false.mp (Bool.eq_false_iff.mpr hany)
    have hmap : kept.map (fun k => k && foldKeeps c c) = kept.map id := by
      apply List.map_congr_left
      intro k hk
      have := hall k hk
      cases k <;> cases hc : foldKeeps c c <;> simp_all
    rw [← h]
    unfold runFold
    rw [hmap, List.map_id]

/-- A run whose units decided over the carried closure (a kept row needs it
closed: an open run member never decides) always composes, with its rows
unchanged. -/
theorem a_run_decided_over_its_facts_composes {invokes : Bool} {c : Closure}
    {kept : List Bool} (h : ∀ k ∈ kept, k = true → ∃ ns, c = .closed ns) :
    compose invokes (some c) kept = some kept := by
  simp only [compose]
  have : kept.any (fun k => k && !foldKeeps c c) = false := by
    rw [List.any_eq_false]
    intro k hk
    cases k with
    | false => simp
    | true =>
      obtain ⟨ns, rfl⟩ := h true hk rfl
      simp [a_closed_closure_keeps_itself]
  simp [this]

end QCore.MemberPrepassRoutes
