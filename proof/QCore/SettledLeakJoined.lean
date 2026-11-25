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

import QCore.VerdictSemigroup

/-!
# A withdrawn leak with a path avoiding every mark is published (card SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_joined_leak_witness.rs`, gateway rule 84,
amending rule 74c (A).

Rule 74c (A) withdraws a held leak report when the settled exit marks the
identity by a refused release beside an `Unknown` leaf. The union join
cannot tell a path through the refused release from another path to the same
exit. In `int fd = k_open(path); if (flag && k_close(fd) == 0) {}` the
close of a possibly-invalid descriptor is refused, and the `flag`-false path
leaves a valid descriptor open, so that leak was withdrawn with the other.

The Rust replays the custody work list for the identity and drops every edge
whose state marks it. The restricted exit is therefore the join over the
paths no mark lies on. This module models a run's reaching paths as a list,
each path with its steps (whether a step marks the identity, whether it is a
refused release of it) and whether the identity is live when the path ends.

## What is proved

* **The cut join is the avoiding join.** Joining only the runs no mark lies
  on is joining the avoiding paths (`the_cut_join_is_the_avoiding_join`).
* **A witness avoids every refused release.** Every refused release marks its
  identity (`RefusalsMark`, the premise `mark_unresolved_effect` gives), so a
  witnessed run is a live path with no refused release on it
  (`a_witness_avoids_every_refused_release`). Every refusal sets the path's
  `unmodeled` flag, which the replay also drops (`RefusalsBlock`), so the
  witness passes no refusal at all, an undecided test's edges included
  (`a_witness_passes_no_refusal`). If every live path passes a
  refused release, nothing is witnessed
  (`no_witness_through_a_refused_release`), and the report stays withdrawn
  (`a_refused_only_leak_stays_withdrawn`).
* **The decision.** `publishes` re-publishes a withdrawn report exactly when
  it is witnessed; a report rule 74c (A) does not withdraw is untouched. Its
  table is `publishVectors` (`publish_vectors_hold`).
* **Only `Unknown -> Unsafe`.** A re-published report adds a leak link to a
  function rule 74c (A) left `Unknown`, so the fold becomes provisional
  `Unsafe`, never `Safe` or `Pending` (`a_publication_is_never_safe`).
* **The pinned instance.** The run through the refused close and the
  `flag`-false run join at the exit: the exit is marked (rule 74c (A)
  withdraws) and the `flag`-false run is witnessed (rule 84 publishes)
  (`the_pinned_instance`). With the close on every path, nothing is witnessed
  (`the_both_arms_control`).

## What this is not

A proof that the Rust replays the work list faithfully, or that its path
search computes `witnessed`. That the replay is the work list's (its uncut
form is asserted equal to the settled states under `debug_assertions`) and
that every refusing release branch calls `mark_unresolved_effect` on its
targets are review premises (trust boundary 28). They are pinned by
`crates/qcore-c-gateway/tests/unit/legacy_joined_leak_witness.rs` (the
shared table) and the fixtures in
`crates/qcore-c-gateway/tests/settled_leak_joined.rs`. The path-feasibility
reading (every edge an edge refinement does not prove infeasible is taken,
and a live identity at the exit is the leak) is the one every published leak
rests on. This is not a statement about any upstream library.
-/

namespace QCore.SettledLeakJoined

open QCore.VerdictSemigroup (V fold)

/-- One step of a path: whether it blocks the replay (it marks the identity
unresolved, or carries any refusal: the Rust's `unresolved_effect_sites` and
`unmodeled`), whether it is a refused release of the identity, and whether it
is refused at all. -/
structure Step where
  marks : Bool
  refusedRelease : Bool
  refused : Bool
deriving DecidableEq, Repr

/-- One reaching path: its steps and whether the identity is live when it
ends at the exit. -/
structure PathRun where
  steps : List Step
  live : Bool
deriving DecidableEq, Repr

/-- No mark of the identity lies on the path. -/
def avoids (p : PathRun) : Bool := p.steps.all (fun s => !s.marks)

/-- The premise: every refused release marks its identity. -/
def RefusalsMark (runs : List PathRun) : Prop :=
  ∀ p ∈ runs, ∀ s ∈ p.steps, s.refusedRelease = true → s.marks = true

/-- The premise: every refused step sets the path's `unmodeled` flag, which
blocks the replay as a mark does. -/
def RefusalsBlock (runs : List PathRun) : Prop :=
  ∀ p ∈ runs, ∀ s ∈ p.steps, s.refused = true → s.marks = true

/-- The settled (union-joined) exit marks the identity: some path marks it. -/
def markedAtExit (runs : List PathRun) : Bool := runs.any (fun p => !avoids p)

/-- The restricted exit claims the identity: some path no mark lies on keeps
it live. -/
def witnessed (runs : List PathRun) : Bool := runs.any (fun p => avoids p && p.live)

/-- The cut replay keeps exactly the avoiding paths; its exit is live when one
of them ends live. -/
def cutJoinLive (runs : List PathRun) : Bool := (runs.filter avoids).any (·.live)

theorem the_cut_join_is_the_avoiding_join (runs : List PathRun) :
    cutJoinLive runs = witnessed runs := by
  unfold cutJoinLive witnessed
  induction runs with
  | nil => rfl
  | cons p ps ih => by_cases h : avoids p = true <;> simp_all

theorem a_witness_avoids_every_refused_release (runs : List PathRun)
    (premise : RefusalsMark runs) (w : witnessed runs = true) :
    ∃ p ∈ runs, p.live = true ∧ ∀ s ∈ p.steps, s.refusedRelease = false := by
  unfold witnessed at w
  obtain ⟨p, mem, hp⟩ := List.any_eq_true.mp w
  simp only [Bool.and_eq_true] at hp
  refine ⟨p, mem, hp.2, ?_⟩
  intro s smem
  have hall := List.all_eq_true.mp hp.1 s smem
  cases hr : s.refusedRelease
  · rfl
  · have := premise p mem s smem hr
    simp [this] at hall

theorem a_witness_passes_no_refusal (runs : List PathRun)
    (premise : RefusalsBlock runs) (w : witnessed runs = true) :
    ∃ p ∈ runs, p.live = true ∧ ∀ s ∈ p.steps, s.refused = false := by
  unfold witnessed at w
  obtain ⟨p, mem, hp⟩ := List.any_eq_true.mp w
  simp only [Bool.and_eq_true] at hp
  refine ⟨p, mem, hp.2, ?_⟩
  intro s smem
  have hall := List.all_eq_true.mp hp.1 s smem
  cases hr : s.refused
  · rfl
  · have := premise p mem s smem hr
    simp [this] at hall

theorem no_witness_through_a_refused_release (runs : List PathRun)
    (premise : RefusalsMark runs)
    (every : ∀ p ∈ runs, p.live = true → ∃ s ∈ p.steps, s.refusedRelease = true) :
    witnessed runs = false := by
  cases w : witnessed runs
  · rfl
  · obtain ⟨p, mem, live, none⟩ := a_witness_avoids_every_refused_release runs premise w
    obtain ⟨s, smem, hs⟩ := every p mem live
    simp [none s smem] at hs

/-- The decision (`publishes` in the Rust): a held report is published unless
rule 74c (A) withdraws it and no avoiding path witnesses it. -/
def publishes (withdrawn witnessed : Bool) : Bool := !withdrawn || witnessed

/-- `(withdrawn, witnessed, published)`; the Rust `PUBLISH_VECTORS` is read
from this table by `tests/unit/legacy_joined_leak_witness.rs`. -/
def publishVectors : List (Bool × Bool × Bool) := [
  (false, false, true),
  (false, true, true),
  (true, false, false),
  (true, true, true)
]

theorem publish_vectors_hold :
    publishVectors.all (fun r => publishes r.1 r.2.1 == r.2.2) = true := by
  decide

theorem an_unwithdrawn_report_is_untouched (witnessed : Bool) :
    publishes false witnessed = true := by
  cases witnessed <;> rfl

theorem a_refused_only_leak_stays_withdrawn (runs : List PathRun)
    (premise : RefusalsMark runs)
    (every : ∀ p ∈ runs, p.live = true → ∃ s ∈ p.steps, s.refusedRelease = true) :
    publishes true (witnessed runs) = false := by
  rw [no_witness_through_a_refused_release runs premise every]
  rfl

/-- A re-published report adds a leak link beside the `Unknown` leaf rule
74c (A) required: the function folds to provisional `Unsafe`. -/
theorem a_publication_is_never_safe (xs : List V) :
    fold (xs ++ [.unknown, .provisionalUnsafe]) = .provisionalUnsafe := by
  rw [QCore.VerdictSemigroup.fold_append]
  cases fold xs <;> decide

/-- The pinned instance: the refused close's path and the `flag`-false path. -/
def pinned : List PathRun :=
  [⟨[⟨true, true, true⟩], true⟩, ⟨[], true⟩]

theorem the_pinned_instance :
    markedAtExit pinned = true ∧ witnessed pinned = true ∧
      publishes (markedAtExit pinned) (witnessed pinned) = true := by
  decide

/-- The control: the close on both arms. -/
def bothArms : List PathRun :=
  [⟨[⟨true, true, true⟩], true⟩, ⟨[⟨true, true, true⟩], true⟩]

theorem the_both_arms_control :
    markedAtExit bothArms = true ∧ witnessed bothArms = false ∧
      publishes (markedAtExit bothArms) (witnessed bothArms) = false := by
  decide

end QCore.SettledLeakJoined
