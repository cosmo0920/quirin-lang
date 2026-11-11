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

import QCore.WriteLend

/-! Write lends crossing the spawn boundary (card `WRITE-LEND-TASKS`).

Card `SLICE-PERMISSIONS-WRITE-LEND` proved the sequential half — two bindings
of one entry holding write authority over disjoint ranges — and named this
card's residual: *"both parties are one sequential entry's bindings — the F1
task-crossing remainder"*. This module is the crossing's model: the scoped-task
ledger's **range write leases**, mirroring `qcore_core::scoped_tasks`'s
`ResourceState::WriteLeased`, its `lease`/`reclaim_lease` transitions, and the
whole-payload rule under which the owner holds no authority — read included —
while any lease is outstanding.

The disjointness decision is `QCore.SlicePermissions.overlaps` — the wave-29
rule, **imported rather than restated**, exactly as the Rust ledger calls
`ownership::axes_overlap` rather than re-implementing it.

Four things are proved, and they are the four the crossing rests on.

**The admission is the disjointness question.** A lease is admitted exactly
when its range is nonempty, disjoint from every outstanding lease, and within
the holder bound — so "no byte is leased twice" is what the rule decides, not
an approximation of it.

**The invariant survives every interleaving.** Every ledger state reachable by
admitted opens and closes — every point of any spawn/join interleaving — has
pairwise-disjoint leases.

**No byte has two write authorities at any point of the interleaving.** For
any reachable state and any byte: at most one lease holds the byte, and the
owner counts as an authority exactly when no lease is outstanding at all — so
the total number of parties holding write authority over any byte is at most
one. This is the cross-thread exclusivity statement the card exists for.

**The authority returns at the join, exactly.** The owner's whole-payload
authority is reconstructed precisely when the last lease is closed, and
closing one lease never disturbs a sibling's.

What is deliberately **not** here, carried as the Rust side's own boundary:
that the frontend extracts the capture's bounds faithfully from tokens
(tested), that the reservation/commit/rollback plumbing routes a pending lease
correctly (tested against the ledger), any statement about the C memory model
or a native schedule, and the holder's *exercise* of its authority — no
operation writes through a lease in this slice, which is the named residual
`WRITE-LEND-TASKS-EXERCISE`. -/

namespace QCore.WriteLendTasks

open QCore.SlicePermissions

/-- One outstanding range write lease: the child task holding it and the
half-open byte interval `[low, high)` it holds — `scoped_tasks::LeaseRange`
under a `ShareHolder::Borrower`. -/
structure Lease where
  holder : Nat
  range : Range
  deriving DecidableEq, Repr

/-- The number of simultaneous lease holders the profile admits for one
resource — the write-side `MAX_READERS`, mirroring `scoped_tasks::MAX_WRITERS`. -/
def MAX_WRITERS : Nat := 2

/-- **The lease admission**: nonempty, within the holder bound, and disjoint
from every outstanding lease — where disjointness is the wave-29 `overlaps`
decision, imported rather than restated. -/
def admitsLease (leases : List Lease) (r : Range) : Bool :=
  decide (r.low < r.high) && decide (leases.length < MAX_WRITERS)
    && leases.all (fun lease => !overlaps lease.range r)

/-- Open one lease: the ledger's `lease` transition, total-or-nothing — a
refused lease changes no state. -/
def openLease (leases : List Lease) (holder : Nat) (r : Range) : Option (List Lease) :=
  if admitsLease leases r then some ({ holder := holder, range := r } :: leases) else none

/-- Close every lease one holder held: the ledger's `reclaim_lease` at the
creator's join, which is the only operation that returns a lease. -/
def closeLease (leases : List Lease) (holder : Nat) : List Lease :=
  leases.filter (fun lease => !(lease.holder == holder))

/-- The leases holding byte `b`. -/
def leasesHolding (leases : List Lease) (b : Nat) : List Lease :=
  leases.filter (fun lease => lease.range.contains b)

/-- The whole-payload rule: the owner holds write authority exactly when no
lease is outstanding at all — `use_resource`'s `LeaseOutstanding` refusal,
as a decision. -/
def ownerMayWrite (leases : List Lease) : Bool :=
  leases.isEmpty

/-- The number of parties holding write authority over byte `b`: the leases
holding it, plus the owner exactly when no lease is outstanding. -/
def writeAuthorities (leases : List Lease) (b : Nat) : Nat :=
  (leasesHolding leases b).length + (if leases.isEmpty then 1 else 0)

/-- Every ledger state a spawn/join interleaving can produce: the empty
ledger, an admitted open, or a close. "Any point of the interleaving" below
quantifies over exactly this relation. -/
inductive Reachable : List Lease → Prop
  | start : Reachable []
  | opened {s t : List Lease} {holder : Nat} {r : Range} :
      Reachable s → openLease s holder r = some t → Reachable t
  | closed {s : List Lease} (holder : Nat) :
      Reachable s → Reachable (closeLease s holder)

/-- The invariant: outstanding leases are pairwise disjoint. -/
def PairwiseDisjoint (leases : List Lease) : Prop :=
  leases.Pairwise (fun a b => overlaps a.range b.range = false)

/-! ## 1. The admission is the disjointness question -/

/-- Against one outstanding lease, a second is admitted exactly when it is
nonempty and disjoint — the read card's decision, imported, deciding the
crossing. -/
theorem a_second_lease_is_admitted_iff_disjoint (held : Lease) (r : Range) :
    admitsLease [held] r = (decide (r.low < r.high) && !overlaps held.range r) := by
  simp [admitsLease, MAX_WRITERS]

/-- Two leases that were admitted together share no byte. -/
theorem two_admitted_leases_share_no_byte (held : Lease) (r : Range)
    (admitted : admitsLease [held] r = true) (b : Nat)
    (inside : held.range.contains b = true) : r.contains b = false := by
  rw [a_second_lease_is_admitted_iff_disjoint] at admitted
  simp only [Bool.and_eq_true, Bool.not_eq_true'] at admitted
  exact disjoint_ranges_share_no_byte held.range r admitted.2 b inside

/-- An empty range is not a lease: it would carry no authority across the
boundary, so the crossing refuses it rather than admitting it vacuously —
a deliberate divergence from the sequential lend's empty-lend admission, in
the refusing direction. -/
theorem an_empty_range_is_not_a_lease (leases : List Lease) (r : Range)
    (empty : r.high ≤ r.low) : admitsLease leases r = false := by
  simp [admitsLease, Nat.not_lt.mpr empty]

/-- A third simultaneous holder is a represented refusal, by count, before any
range is compared. -/
theorem a_third_simultaneous_lease_is_refused (a b : Lease) (r : Range) :
    admitsLease [a, b] r = false := by
  simp [admitsLease, MAX_WRITERS]

/-! ## 2. The invariant survives every interleaving -/

/-- Every reachable ledger state has pairwise-disjoint leases: an admitted
open extends the state only with a range disjoint from everything
outstanding, and a close only removes. -/
theorem reachable_states_are_pairwise_disjoint {s : List Lease}
    (reachable : Reachable s) : PairwiseDisjoint s := by
  induction reachable with
  | start => exact List.Pairwise.nil
  | opened reach step ih =>
      rename_i previous t holder r
      unfold openLease at step
      by_cases admitted : admitsLease previous r = true
      · rw [ite_eq_left admitted] at step
        cases step
        simp only [admitsLease, Bool.and_eq_true, List.all_eq_true,
          Bool.not_eq_true'] at admitted
        refine List.Pairwise.cons ?_ ih
        intro lease outstanding
        show overlaps r lease.range = false
        rw [overlaps_symm]
        exact admitted.2 lease outstanding
      · rw [ite_eq_right admitted] at step
        simp at step
  | closed holder _ ih =>
      exact List.Pairwise.sublist List.filter_sublist ih

/-- With pairwise-disjoint leases, at most one lease holds any given byte:
a second lease holding the same byte would overlap the first, by the wave-29
equivalence between the decision and the set-theoretic question. -/
theorem at_most_one_lease_holds_a_byte {s : List Lease}
    (disjoint : PairwiseDisjoint s) (b : Nat) :
    (leasesHolding s b).length ≤ 1 := by
  induction s with
  | nil => simp [leasesHolding]
  | cons head rest ih =>
      have rest_disjoint : PairwiseDisjoint rest := disjoint.of_cons
      by_cases holds : head.range.contains b = true
      · -- The head holds the byte, so no outstanding sibling may: a sibling
        -- holding it too would share the byte with the head.
        have rest_empty : leasesHolding rest b = [] := by
          unfold leasesHolding
          rw [List.filter_eq_nil_iff]
          intro lease outstanding
          have head_lease := List.rel_of_pairwise_cons disjoint outstanding
          simp only [Bool.not_eq_true]
          exact disjoint_ranges_share_no_byte head.range lease.range head_lease b holds
        have step : leasesHolding (head :: rest) b = head :: leasesHolding rest b := by
          simp [leasesHolding, holds]
        rw [step, rest_empty]
        simp
      · have skipped : head.range.contains b = false := by
          revert holds; cases head.range.contains b <;> simp
        have step : leasesHolding (head :: rest) b = leasesHolding rest b := by
          simp [leasesHolding, skipped]
        rw [step]
        exact ih rest_disjoint

/-! ## 3. The exclusivity theorem -/

/-- **No byte has two write authorities at any point of the interleaving.**
For every ledger state a spawn/join interleaving can reach and every byte of
the payload, the number of parties holding write authority over that byte —
the lease holders whose ranges contain it, plus the owner exactly when no
lease is outstanding — is at most one. The two halves of the argument are the
two rules the Rust ledger enforces: admitted leases are pairwise disjoint
(so two leases never hold one byte), and the owner's whole-payload authority
is suspended while anything is out (so the owner and a holder are never both
authorities). -/
theorem no_byte_has_two_write_authorities {s : List Lease}
    (reachable : Reachable s) (b : Nat) : writeAuthorities s b ≤ 1 := by
  unfold writeAuthorities
  cases s with
  | nil => simp [leasesHolding]
  | cons head rest =>
      have bound := at_most_one_lease_holds_a_byte
        (reachable_states_are_pairwise_disjoint reachable) b
      simpa using bound

/-- The owner is an authority exactly when nothing is outstanding — the
model-level form of `use_resource`'s `LeaseOutstanding` refusal, in both
directions. -/
theorem the_owner_is_suspended_exactly_while_a_lease_is_out (s : List Lease) :
    ownerMayWrite s = true ↔ s = [] := by
  simp [ownerMayWrite]

/-! ## 4. The authority returns at the join, exactly -/

/-- Closing the last outstanding lease reconstructs the owner's whole-payload
authority. -/
theorem closing_the_last_lease_restores_the_owner (holder : Nat) (r : Range) :
    ownerMayWrite (closeLease [{ holder := holder, range := r }] holder) = true := by
  simp [closeLease, ownerMayWrite]

/-- A join returns only its own child's lease: a sibling's survives the close
untouched, so authority is reconstructed only when the last lease is back. -/
theorem a_join_never_disturbs_a_siblings_lease (sibling : Lease) (holder : Nat)
    (other : sibling.holder ≠ holder) (rest : List Lease) :
    sibling ∈ closeLease (sibling :: rest) holder := by
  have kept : (sibling.holder == holder) = false := by simpa using other
  simp [closeLease, kept]

/-- After a close, no lease of the closed holder remains: the join reclaims
the child's lease whole, never partially. -/
theorem a_closed_holder_retains_nothing (s : List Lease) (holder : Nat) :
    ∀ lease ∈ closeLease s holder, lease.holder ≠ holder := by
  intro lease kept
  have := (List.mem_filter.mp kept).2
  simpa using this

/-! ## 5. The added surface, as one list

One capture mode, one delivered kind, one protocol — so a widening changes a
theorem rather than slipping past. The composed epoch route refuses the
capture by name (`WRITE-LEND-TASKS-EPOCH`), a Rust one-arm boundary recorded
here as inventory. -/

inductive AddedSurface where
  | lendWriteCapture
  deriving DecidableEq, Repr

def addedSurface : List AddedSurface := [.lendWriteCapture]

theorem the_added_capture_is_exactly_one : addedSurface.length = 1 := rfl

end QCore.WriteLendTasks
