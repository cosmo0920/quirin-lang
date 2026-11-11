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

import QCore.WriteLendTasks

/-!
# Exercising a range write lease (card `WRITE-LEND-TASKS-EXERCISE`)

Card `WRITE-LEND-TASKS` proved that no byte of a payload has two write
authorities at any point of a spawn/join interleaving
(`QCore.WriteLendTasks.no_byte_has_two_write_authorities`). It proved that
about authority **held**: in that slice no operation admitted a lease in a
worker's body at all, so the theorem was about a permission nobody used.

This module is about authority **exercised**. A holder now writes bytes
through its lease, and the two questions that opens are:

1. **The rebasing law.** A holder names a byte relative to its own lease; the
   byte it reaches is `low + i`. Which relative indices may it name? The Rust
   answer is split in two, because the two halves are known in two different
   places: the worker's body states the exclusive bound `n` it touches
   (`FunctionSummary.lease_footprint`, derived where the body is checked), and
   the creator's spawn site — the only place the lease's real `[low, high)`
   exists — decides `n ≤ high - low`. `the_footprint_fit_is_exactly_the_rebasing_law`
   below says that one arithmetic comparison is equivalent to the whole family
   of per-index containment facts, which is what licenses checking a summary
   once instead of every index at every site.

2. **Exclusivity survives the exercise.** The wave-31 theorem counted
   *authorities*. Here writes are counted: `no_byte_is_written_by_two_parties`
   says that in any reachable ledger state two admitted writes to one byte
   come from the same writer. The owner is included, by
   `the_owner_writes_no_byte_while_any_lease_is_out`.

What is **not** proved here, and is deliberately observation rather than
theorem: that the emitted C's `((unsigned char *)&buffer->payload)[i]` is a
distinct memory location in the sense of C11 3.14 (that is a reading of the
standard, carried in the helper's own comment and measured by the card's
sanitized native run), and anything about a native schedule. The reference
run's agreement with the emitted C is likewise measured, not proved.
-/

namespace QCore.WriteLendTasksExercise

open QCore.SlicePermissions
open QCore.WriteLendTasks

/-! ## 1. The rebasing law -/

/-- The owner-side byte a holder reaches by naming its own relative index `i`:
`crates/qcore-core/src/task_execution.rs`'s "`index` is relative to the
authority's own base", and the emitted C's `base + index`. -/
def rebase (lease : Lease) (i : Nat) : Nat := lease.range.low + i

/-- Whether one relative index is admitted: the rebased byte lies inside the
lease. -/
def admitsIndex (lease : Lease) (i : Nat) : Bool :=
  lease.range.contains (rebase lease i)

/-- The **spawn-site decision**: the worker's stated exclusive footprint bound
fits the lease it is handed. This is the one comparison
`check_function`'s caller performs — `bound > high - low` refuses. -/
def footprintFits (r : Range) (bound : Nat) : Bool :=
  decide (bound ≤ r.high - r.low)

/-- A rebased index is inside its lease exactly when it is below the lease's
extent. The lease's own `low` cancels, which is why a worker can be checked
without knowing where its lease sits. -/
theorem admits_index_iff_below_extent (lease : Lease) (i : Nat)
    (nonempty : lease.range.low ≤ lease.range.high) :
    admitsIndex lease i = true ↔ i < lease.range.high - lease.range.low := by
  obtain ⟨_, low, high⟩ := lease
  unfold admitsIndex rebase Range.contains
  simp only [Bool.and_eq_true, decide_eq_true_eq]
  simp only at nonempty
  omega

/-- **The footprint fit is exactly the rebasing law.** One arithmetic
comparison at the spawn site stands for every per-index containment fact the
worker's body could need — so checking the summary once is not an
approximation of checking each write, it is the same statement. This is the
theorem that licenses the summary at all. -/
theorem the_footprint_fit_is_exactly_the_rebasing_law (lease : Lease) (bound : Nat)
    (nonempty : lease.range.low ≤ lease.range.high) :
    footprintFits lease.range bound = true ↔ ∀ i, i < bound → admitsIndex lease i = true := by
  simp only [footprintFits, decide_eq_true_eq]
  constructor
  · intro fits i below
    exact (admits_index_iff_below_extent lease i nonempty).mpr (by omega)
  · intro every
    cases bound with
    | zero => omega
    | succ n =>
        have inside := (admits_index_iff_below_extent lease n nonempty).mp
          (every n (Nat.lt_succ_self n))
        omega

/-- An index the checker cannot resolve to a constant is refused rather than
admitted, and that refusal is not a concession: for a *nonempty* lease there
are always indices this lease does not admit, so "not provably inside" and
"inside" are genuinely different questions. -/
theorem some_index_is_always_outside (lease : Lease)
    (nonempty : lease.range.low < lease.range.high) :
    admitsIndex lease (lease.range.high - lease.range.low) = false := by
  obtain ⟨_, low, high⟩ := lease
  simp only at nonempty
  have outside : ¬(low + (high - low) < high) := by omega
  simp [admitsIndex, rebase, Range.contains, outside]

/-! ## 2. Exclusivity survives the exercise -/

/-- One byte actually written, by one party. `writer` is a lease holder's
identity. -/
structure Write where
  writer : Nat
  byte : Nat
  deriving DecidableEq, Repr

/-- Whether a write is admitted against a ledger state: some outstanding lease
is the writer's and contains the byte. This is the conjunction the Rust side
splits across the worker (`ByteAuthority::Lease` plus the recorded footprint)
and the creator (the fit decision); the model states it whole. -/
def admitsWrite (leases : List Lease) (w : Write) : Bool :=
  leases.any (fun lease => lease.holder == w.writer && lease.range.contains w.byte)

/-- An admitted write lies inside a lease its own writer holds. The Rust rule
"the index, rebased into the lease, lies within `[low, high)`" as a property
of the write rather than of the index. -/
theorem an_admitted_write_lies_inside_its_writers_lease {leases : List Lease} {w : Write}
    (admitted : admitsWrite leases w = true) :
    ∃ lease ∈ leases, lease.holder = w.writer ∧ lease.range.contains w.byte = true := by
  simp only [admitsWrite, List.any_eq_true, Bool.and_eq_true, beq_iff_eq] at admitted
  obtain ⟨lease, mem, holder, contains⟩ := admitted
  exact ⟨lease, mem, holder, contains⟩

/-- A list with at most one element cannot tell two of its members apart. -/
private theorem members_of_a_short_list_agree {α : Type} {l : List α}
    (short : l.length ≤ 1) {x y : α} (hx : x ∈ l) (hy : y ∈ l) : x = y := by
  match l with
  | [] => cases hx
  | [_] =>
      rw [List.mem_singleton] at hx hy
      rw [hx, hy]
  | _ :: _ :: _ => simp at short

/-- **The exclusivity theorem, now about writes.** In any ledger state a
spawn/join interleaving can reach, two admitted writes to one byte are the
same writer's. Two threads therefore never write one byte — which is the
property the card's native run exhibits and this theorem makes general. The
argument is wave 31's, used rather than repeated: an admitted write lies in a
lease its writer holds, and at most one outstanding lease holds any byte. -/
theorem no_byte_is_written_by_two_parties {s : List Lease} (reachable : Reachable s)
    {a b : Write} (same_byte : a.byte = b.byte)
    (first : admitsWrite s a = true) (second : admitsWrite s b = true) :
    a.writer = b.writer := by
  obtain ⟨la, mem_a, holder_a, in_a⟩ := an_admitted_write_lies_inside_its_writers_lease first
  obtain ⟨lb, mem_b, holder_b, in_b⟩ := an_admitted_write_lies_inside_its_writers_lease second
  have short := at_most_one_lease_holds_a_byte
    (reachable_states_are_pairwise_disjoint reachable) a.byte
  have holds_a : la ∈ leasesHolding s a.byte := List.mem_filter.mpr ⟨mem_a, in_a⟩
  have holds_b : lb ∈ leasesHolding s a.byte :=
    List.mem_filter.mpr ⟨mem_b, same_byte ▸ in_b⟩
  have same_lease : la = lb := members_of_a_short_list_agree short holds_a holds_b
  rw [← holder_a, ← holder_b, same_lease]

/-- The owner writes no byte while any lease is out: card `WRITE-LEND-TASKS`'
whole-payload suspension, restated as a fact about writes so the theorem above
covers every party rather than only the holders. -/
theorem the_owner_writes_no_byte_while_any_lease_is_out (s : List Lease)
    (outstanding : s ≠ []) : ownerMayWrite s = false := by
  cases s with
  | nil => exact absurd rfl outstanding
  | cons _ _ => simp [ownerMayWrite]

/-! ## 3. The added surface, as one list

Two instructions, one summary field, one spawn-site decision. A widening
changes a theorem rather than slipping past. -/

inductive AddedSurface where
  | writeBufferByte
  | readBufferByte
  deriving DecidableEq, Repr

def addedSurface : List AddedSurface := [.writeBufferByte, .readBufferByte]

theorem the_added_operations_are_exactly_two : addedSurface.length = 2 := rfl

end QCore.WriteLendTasksExercise
