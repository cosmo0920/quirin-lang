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

* **Every refusal kind** (card SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE, the
  last section). A witness avoids every refusal of every kind
  (`a_witness_avoids_every_refusal_kind`). The amended decision reads the
  settled runs only, so two schedules of the same runs decide the same
  (`publication_is_order_independent`), where the old one did not
  (`the_old_decision_read_the_schedule`). A leak whose every live run passes
  a refusal stays withdrawn in every order
  (`a_refused_only_leak_stays_withdrawn_any_kind`), and a decided report is
  never `Safe` (`a_decided_report_is_never_safe`). The order independence is
  stated where rule 84 decides the report or the settled exit is unmarked.

* **A mark beside a `Pending` hold** (card
  SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF, the last section). The side
  condition is a blocking leaf, `Unknown` or `Pending` (`blocking`), so the
  order independence covers a function whose only blocking leaves are holds
  (`publication_is_order_independent_beside_a_hold`), where the old side
  condition read the schedule (`the_pending_case_read_the_schedule`). The
  settle step only appends, so every `Pending` reason is kept
  (`a_pending_reason_is_preserved`) and the fold is never `Safe`
  (`a_decision_beside_a_blocking_leaf_is_never_safe`); a withdrawal beside
  holds only is exactly `Pending`
  (`a_withdrawal_beside_holds_only_is_pending`). A mark beside no blocking
  leaf is left to rule 74c's held report
  (`a_mark_beside_no_blocking_leaf_is_left_to_rule_74c`).

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

/-! ## Every refusal kind (card SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE)

Rule 84 decided only the identities a refused release marks. A refused
hand-on, test, re-acquisition or escape marks its identity too, and a leak
`report_leaks` claimed on an iterate before such a marker joined the exit was
published, while the same leak was not when the marker arrived first. The
amended rule decides every report whose settled exit marks the identity by any
refusal beside an `Unknown` leaf from the settled runs alone: it is published
exactly when a run no mark lies on keeps the identity live.

A schedule is the order in which the runs reach the exit. The work list's
iterate after a prefix of it sees the union join of that prefix, and claims
the identity when it is live there and unmarked. `held` is whether some
iterate claimed it. -/

/-- The refusal kinds that mark an identity unresolved. -/
inductive Kind where
  | release
  | handOn
  | test
  | reacquisition
  | escape
  | other
deriving DecidableEq, Repr

/-- One step of a path: whether it blocks the replay, and which refusal, if
any, it is. -/
structure KStep where
  marks : Bool
  refusal : Option Kind
deriving DecidableEq, Repr

/-- One reaching path with refusal kinds. -/
structure KRun where
  steps : List KStep
  live : Bool
deriving DecidableEq, Repr

/-- No mark of the identity lies on the path. -/
def kAvoids (p : KRun) : Bool := p.steps.all (fun s => !s.marks)

/-- The settled exit marks the identity: some run marks it. -/
def kMarked (runs : List KRun) : Bool := runs.any (fun p => !kAvoids p)

/-- The restricted replay's witness: a run no mark lies on keeps it live. -/
def kWitnessed (runs : List KRun) : Bool := runs.any (fun p => kAvoids p && p.live)

/-- The premise, over every kind: every refusal of the identity marks it (or
sets `unmodeled`, which the replay cuts as a mark). -/
def EveryRefusalMarks (runs : List KRun) : Prop :=
  ∀ p ∈ runs, ∀ s ∈ p.steps, s.refusal.isSome = true → s.marks = true

theorem a_witness_avoids_every_refusal_kind (runs : List KRun)
    (premise : EveryRefusalMarks runs) (w : kWitnessed runs = true) :
    ∃ p ∈ runs, p.live = true ∧ ∀ s ∈ p.steps, ∀ k : Kind, s.refusal ≠ some k := by
  unfold kWitnessed at w
  obtain ⟨p, mem, hp⟩ := List.any_eq_true.mp w
  simp only [Bool.and_eq_true] at hp
  refine ⟨p, mem, hp.2, ?_⟩
  intro s smem k hk
  have hall := List.all_eq_true.mp hp.1 s smem
  have := premise p mem s smem (by simp [hk])
  simp [this] at hall

/-- An iterate after the prefix `pre` claims the identity: live in the
union join and unmarked. -/
def claimsOn (pre : List KRun) : Bool := pre.any (·.live) && !kMarked pre

/-- Some iterate of the schedule `order` claimed the identity: the join of
some prefix of it did. -/
def held (order : List KRun) : Bool :=
  (List.range (order.length + 1)).any (fun n => claimsOn (order.take n))

/-- Whether rule 84 decides the report (`decides` in the Rust). -/
def decides (marked unknownLeaf : Bool) : Bool := marked && unknownLeaf

/-- The decision before this card for an identity marked only by refusals
other than a release: rule 74c (A) did not withdraw a held report, and rule
84 did not take the identity, so the report was published exactly when an
iterate held it. -/
def oldDecision (order : List KRun) : Bool := held order

/-- The amended decision: a report rule 84 decides is published when
witnessed; any other report as before. -/
def decision (unknownLeaf : Bool) (order : List KRun) : Bool :=
  if decides (kMarked order) unknownLeaf then kWitnessed order else held order

theorem any_perm {α : Type} (f : α → Bool) {xs ys : List α} (h : xs.Perm ys) :
    xs.any f = ys.any f := by
  apply Bool.eq_iff_iff.mpr
  simp only [List.any_eq_true]
  exact ⟨fun ⟨x, m, fx⟩ => ⟨x, h.mem_iff.mp m, fx⟩,
    fun ⟨x, m, fx⟩ => ⟨x, h.mem_iff.mpr m, fx⟩⟩

/-- At an unmarked settled exit, some iterate claimed the identity exactly when
the settled exit does (every prefix's join is below the settled join). -/
theorem held_at_an_unmarked_exit (order : List KRun) (h : kMarked order = false) :
    held order = claimsOn order := by
  apply Bool.eq_iff_iff.mpr
  constructor
  · intro hh
    unfold held at hh
    obtain ⟨n, _, hc⟩ := List.any_eq_true.mp hh
    unfold claimsOn at hc ⊢
    simp only [Bool.and_eq_true] at hc
    obtain ⟨p, pm, pl⟩ := List.any_eq_true.mp hc.1
    simp only [h, Bool.not_false, Bool.and_true]
    exact List.any_eq_true.mpr ⟨p, (List.take_sublist n order).subset pm, pl⟩
  · intro hc
    unfold held
    refine List.any_eq_true.mpr ⟨order.length, ?_, ?_⟩
    · simp
    · simpa using hc

/-- **Publication is order-independent.** Two schedules of the same runs
decide the same, whenever rule 84 decides the report or the settled exit does
not mark the identity. -/
theorem publication_is_order_independent (unknownLeaf : Bool) (o₁ o₂ : List KRun)
    (perm : o₁.Perm o₂) (scope : unknownLeaf = true ∨ kMarked o₁ = false) :
    decision unknownLeaf o₁ = decision unknownLeaf o₂ := by
  have hm : kMarked o₁ = kMarked o₂ := any_perm _ perm
  have hw : kWitnessed o₁ = kWitnessed o₂ := any_perm _ perm
  have hl : o₁.any (·.live) = o₂.any (·.live) := any_perm _ perm
  have unmarked : kMarked o₁ = false → held o₁ = held o₂ := by
    intro hk
    have hk₂ : kMarked o₂ = false := hm ▸ hk
    rw [held_at_an_unmarked_exit o₁ hk, held_at_an_unmarked_exit o₂ hk₂]
    unfold claimsOn
    rw [hl, hk, hk₂]
  unfold decision
  cases hk : kMarked o₁
  · have hk₂ : kMarked o₂ = false := hm ▸ hk
    simp [decides, hk₂, unmarked hk]
  · have hk₂ : kMarked o₂ = true := hm ▸ hk
    rcases scope with hu | hu
    · simp [decides, hk₂, hu, hw]
    · simp [hk] at hu

/-- A refused-only leak stays withdrawn, whatever refusal kinds mark it and
whatever the schedule. -/
theorem a_refused_only_leak_stays_withdrawn_any_kind (order : List KRun)
    (premise : EveryRefusalMarks order)
    (every : ∀ p ∈ order, p.live = true → ∃ s ∈ p.steps, s.refusal.isSome = true)
    (marked : kMarked order = true) :
    decision true order = false := by
  have hw : kWitnessed order = false := by
    cases w : kWitnessed order
    · rfl
    · obtain ⟨p, mem, live, none⟩ := a_witness_avoids_every_refusal_kind order premise w
      obtain ⟨s, smem, hs⟩ := every p mem live
      cases hr : s.refusal with
      | none => simp [hr] at hs
      | some k => exact absurd hr (none s smem k)
  simp [decision, decides, marked, hw]

/-- A hand-on refused on one arm, and the other arm that avoids it: in either
order, the run that arrives first decides the old outcome. -/
def handOnArm : KRun := ⟨[⟨true, some .handOn⟩], true⟩
def avoidingArm : KRun := ⟨[], true⟩

/-- **The hazard, reproduced.** Before this card the outcome read the
schedule: the avoiding arm first was claimed and published, the hand-on arm
first was not. -/
theorem the_old_decision_read_the_schedule :
    oldDecision [avoidingArm, handOnArm] = true ∧
      oldDecision [handOnArm, avoidingArm] = false := by
  decide

/-- **The amended decision does not.** Both orders publish the avoiding arm's
leak. -/
theorem both_orders_publish_the_avoiding_arm :
    decision true [avoidingArm, handOnArm] = true ∧
      decision true [handOnArm, avoidingArm] = true := by
  decide

/-- Each kind on both arms: nothing is witnessed in either order. -/
theorem every_kind_on_both_arms_is_withdrawn (k : Kind) :
    decision true [⟨[⟨true, some k⟩], true⟩, ⟨[⟨true, some k⟩], true⟩] = false := by
  cases k <;> decide

/-- A publication under the amended rule sits beside the `Unknown` leaf that
`decides` requires, so the function folds to provisional `Unsafe`; a
withdrawal leaves that `Unknown` leaf. Neither is ever `Safe`. -/
theorem a_decided_report_is_never_safe (xs : List V) (published : Bool) :
    fold (xs ++ [.unknown] ++ (if published then [.provisionalUnsafe] else [])) ≠ .safe := by
  cases published <;> simp only [QCore.VerdictSemigroup.fold_append] <;>
    cases fold xs <;> decide

/-! ## A mark beside a `Pending` hold (card SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF)

Rule 84 decided a marked report only beside an `Unknown` leaf. A call held
for a human act refuses with a `Pending` reason and marks what it hands on,
so in a function whose only blocking leaves are such holds the report was
still decided by `held`, the schedule. The amended side condition is a
blocking leaf: an `Unknown` or a `Pending` one. A withdrawal removes only the
leak, so the hold's leaf and its reason stay, and the row is never `Safe`. A
mark beside no blocking leaf is left to rule 74c: withdrawing there could
leave no non-`Safe` leaf. -/

/-- A leaf that keeps a withdrawal from reaching `Safe` (`blocking` in the
Rust). -/
def blocking (unknownLeaf pendingLeaf : Bool) : Bool := unknownLeaf || pendingLeaf

/-- The decision with the widened side condition. -/
def decisionAt (unknownLeaf pendingLeaf : Bool) (order : List KRun) : Bool :=
  decision (blocking unknownLeaf pendingLeaf) order

/-- **Order independence, the `Pending` case included.** Two schedules of the
same runs decide the same whenever the function carries an `Unknown` or a
`Pending` leaf, or the settled exit is unmarked. -/
theorem publication_is_order_independent_beside_a_hold (unknownLeaf pendingLeaf : Bool)
    (o₁ o₂ : List KRun) (perm : o₁.Perm o₂)
    (scope : unknownLeaf = true ∨ pendingLeaf = true ∨ kMarked o₁ = false) :
    decisionAt unknownLeaf pendingLeaf o₁ = decisionAt unknownLeaf pendingLeaf o₂ := by
  unfold decisionAt
  apply publication_is_order_independent _ _ _ perm
  rcases scope with h | h | h
  · left; simp [blocking, h]
  · left; simp [blocking, h]
  · right; exact h

/-- A held call on one arm (a mark with a refusal), and the arm that avoids
it. -/
def heldArm : KRun := ⟨[⟨true, some .other⟩], true⟩

/-- **The hazard, reproduced.** Beside a `Pending` leaf only, the old side
condition left the report to the schedule: the avoiding arm first was
published, the held arm first was not. -/
theorem the_pending_case_read_the_schedule :
    decision false [avoidingArm, heldArm] = true ∧
      decision false [heldArm, avoidingArm] = false := by
  decide

/-- **The amended decision does not.** Beside a `Pending` leaf both orders
publish the avoiding arm's leak. -/
theorem both_orders_publish_beside_a_hold :
    decisionAt false true [avoidingArm, heldArm] = true ∧
      decisionAt false true [heldArm, avoidingArm] = true := by
  decide

/-- The hold on both arms: withdrawn in either order. -/
theorem a_hold_on_both_arms_is_withdrawn :
    decisionAt false true [heldArm, heldArm] = false := by
  decide

/-- A leak whose every live run passes a refusal (the hold included) stays
withdrawn beside a `Pending` leaf, whatever the schedule. -/
theorem a_refused_only_leak_stays_withdrawn_beside_a_hold (order : List KRun)
    (premise : EveryRefusalMarks order)
    (every : ∀ p ∈ order, p.live = true → ∃ s ∈ p.steps, s.refusal.isSome = true)
    (marked : kMarked order = true) :
    decisionAt false true order = false :=
  a_refused_only_leak_stays_withdrawn_any_kind order premise every marked

/-- Without a blocking leaf the decision is rule 74c's held report, as
before: this card does not decide that case. -/
theorem a_mark_beside_no_blocking_leaf_is_left_to_rule_74c (order : List KRun)
    (marked : kMarked order = true) :
    decisionAt false false order = held order := by
  simp [decisionAt, decision, decides, blocking, marked]

/-- A leaf of a function: its verdict and its reason. -/
structure Leaf where
  verdict : V
  reason : String
deriving DecidableEq, Repr

/-- The settle step: the function's leaves are kept and the leak is appended
when published. Nothing is removed. -/
def settleLeaves (leaves : List Leaf) (published : Bool) : List Leaf :=
  leaves ++ (if published then [⟨.provisionalUnsafe, "leak"⟩] else [])

/-- **Every `Pending` reason is preserved**, whether the leak is published
or withdrawn. -/
theorem a_pending_reason_is_preserved (leaves : List Leaf) (published : Bool)
    (l : Leaf) (member : l ∈ leaves) : l ∈ settleLeaves leaves published := by
  unfold settleLeaves
  exact List.mem_append_left _ member

/-- **Never `Safe`.** Beside a blocking leaf, published or withdrawn, the
function's fold is not `Safe`. -/
theorem a_decision_beside_a_blocking_leaf_is_never_safe (leaves : List Leaf)
    (published : Bool) (l : Leaf) (member : l ∈ leaves)
    (isBlocking : l.verdict = .pending ∨ l.verdict = .unknown) :
    fold ((settleLeaves leaves published).map (·.verdict)) ≠ .safe := by
  apply QCore.VerdictSemigroup.a_non_safe_link_keeps_the_chain_non_safe
    (x := l.verdict)
  · exact List.mem_map.mpr ⟨l, a_pending_reason_is_preserved leaves published l member, rfl⟩
  · rcases isBlocking with h | h <;> rw [h] <;> decide

theorem fold_of_pendings (xs : List V) (ne : xs ≠ []) (h : ∀ x ∈ xs, x = .pending) :
    fold xs = .pending := by
  induction xs with
  | nil => exact absurd rfl ne
  | cons x xs ih =>
    have hx := h x (List.mem_cons_self ..)
    cases xs with
    | nil => rw [hx]; rfl
    | cons y ys =>
      have rest := ih (List.cons_ne_nil _ _) (fun z hz => h z (List.mem_cons_of_mem _ hz))
      show QCore.VerdictSemigroup.V.join x (fold (y :: ys)) = V.pending
      rw [rest, hx]; rfl

/-- A withdrawal beside holds only folds to exactly `Pending`: no reason is
invented, and the row does not rise above the holds' own verdict. -/
theorem a_withdrawal_beside_holds_only_is_pending (leaves : List Leaf)
    (nonempty : leaves ≠ []) (holds : ∀ l ∈ leaves, l.verdict = .pending) :
    fold ((settleLeaves leaves false).map (·.verdict)) = .pending := by
  apply fold_of_pendings
  · simpa [settleLeaves] using nonempty
  · intro x hx
    simp only [settleLeaves, Bool.false_eq_true, ite_false, List.append_nil, List.mem_map] at hx
    obtain ⟨l, lm, rfl⟩ := hx
    exact holds l lm

end QCore.SettledLeakJoined
