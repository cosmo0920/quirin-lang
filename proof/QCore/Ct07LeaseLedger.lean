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

import QCore.WriteLendTasksExercise

/-!
# CT07 — the disjointness theorem bound to the case's own lease ledger

CT07's scenario is `Two disjoint array halves mutated independently` (spec §55,
semantic profile `ScopedWriteLeasesV1`, expected `Accept with
bounds/disjointness proof`). Since card `CT07-TWO-THREAD-REMAP` (wave 37) the
case's artifact is a two-thread program: one allocation whose payload is a
single `int32_t`, two children each holding a **range write lease** over one
of its two-byte halves — `lend buffer 0..2` and `lend buffer 2..4` — each
writing one byte through its own lease, the owner reading both bytes back only
after both joins.

The theorem about that mechanism has existed since wave 31:
`QCore.WriteLendTasks.no_byte_has_two_write_authorities` — for every ledger
state a spawn/join interleaving can **reach** and every byte of the payload,
at most one party holds write authority. What was missing (residual
`CT07-TWO-THREAD-PROOF-LEG`) was the *binding*: nothing said that CT07's own
checked artifact produces a ledger state the theorem's `Reachable` hypothesis
covers. The CT01/CT37/CT03 pattern cannot supply it, by decision: those legs
bind through the F02 trace relation, and `reference_trace` **refuses** a lease
event under constraint 15n as amended by `WRITE-LEND-TASKS-EXERCISE`
(`CT07-PROOF-LEG-PATTERN-UNAVAILABLE`). So this leg binds **through the
ledger** instead, and the alphabet does not grow.

The witness is a **replay**. A ledger step is an admitted open or a close;
`replay` folds the artifact's steps from the empty ledger through the wave-31
`openLease`/`closeLease` transitions, total-or-nothing. The one general
theorem this file adds is `replay_reaches`: whatever state a successful replay
produces is `Reachable`. That is the equivalence the Rust runner
(`compiler/tests/ct07_proof_leg.rs::ct07_artifact_is_the_lean_proved_disjoint_halves_shape`)
rests on: it transcribes `replay` clause for clause, feeds it the lease opens
and closes CT07's reference execution actually performed (in that order, with
the children's own identities as holders), requires `some`, and so has — by
this theorem — a `Reachable` witness for the very state the artifact reaches.
`ct07Steps` transcribes those steps; `ct07Ledger` is the state after both
opens, the point at which two threads hold authority at once; the pin
`ct07_ledger_digest` is recomputed from the artifact with wrapping `u64`
arithmetic exactly as the F02 legs recompute theirs.

What is bound, precisely:

* **`no_byte_has_two_write_authorities`** (wave 31), instantiated on
  `ct07Ledger` as `ct07_no_byte_has_two_write_authorities` — the lease set
  ↔ `Reachable` term binding the residual's discharge names.
* **`no_byte_is_written_by_two_parties`** (wave 36), instantiated on the same
  state as `ct07_no_byte_is_written_by_two_parties`, with the artifact's two
  byte writes shown admitted against it (`ct07_writes_are_admitted`). The
  writes are a consequence checked on top of the lease binding, not a second
  binding route.

The scenario shape is `dischargesDisjointHalves`: exactly two leases, held by
two different children, and every byte of the payload under **exactly one**
write authority — computed by the theorem's own `writeAuthorities`, so the
`≤ 1` the theorem proves is met with equality on every byte: the two threads
partition the payload. The three named mutations of the fixture are each
decided here for the reason the theorem gives, separately from where the
checker catches them: overlapping halves refuse at the disjointness clause
(`a_second_lease_is_admitted_iff_disjoint`), an empty half refuses at the
nonemptiness clause (`an_empty_range_is_not_a_lease`), and a single writer is
a reachable state that is **not the scenario** — the theorem holds of it, the
shape does not.

What this does **not** establish, on the wave-31 boundary: anything about the
C memory model or a native schedule (observation only, the heap-scenario
runner's), or a verified extraction between Lean and Rust — the agreement is
the checked-digest transcription pattern F02 established, here over the
ledger rather than the trace. -/

namespace QCore.Ct07LeaseLedger

open QCore.SlicePermissions
open QCore.WriteLendTasks
open QCore.WriteLendTasksExercise

/-! ## The replay: from the artifact's steps to a `Reachable` witness -/

/-- One ledger transition the reference execution performs: a lease opened at
a child's spawn (`TaskEvent::BufferLeased`; the ledger's `lease`), or every
lease of a child closed at its creator's join (`TaskEvent::BufferLeaseReturned`;
the ledger's `reclaim_lease`). -/
inductive Step where
  | lease (holder : Nat) (range : Range)
  | reclaim (holder : Nat)
  deriving DecidableEq, Repr

/-- One step applied to a ledger state: the wave-31 transitions, unchanged. -/
def step (s : List Lease) : Step → Option (List Lease)
  | .lease holder r => openLease s holder r
  | .reclaim holder => some (closeLease s holder)

/-- Replay a step list from a state, total-or-nothing: one refused open
refuses the whole replay. -/
def replayFrom (s : List Lease) : List Step → Option (List Lease)
  | [] => some s
  | x :: rest => (step s x).bind (fun t => replayFrom t rest)

/-- Replay from the empty ledger — the state every task module starts in. -/
def replay (steps : List Step) : Option (List Lease) :=
  replayFrom [] steps

/-- One admitted step preserves reachability. -/
theorem step_reaches {s t : List Lease} {x : Step}
    (reach : Reachable s) (run : step s x = some t) : Reachable t := by
  cases x with
  | lease holder r => exact Reachable.opened reach run
  | reclaim holder =>
      simp only [step, Option.some.injEq] at run
      exact run ▸ Reachable.closed holder reach

/-- A replay from a reachable state reaches only reachable states. -/
theorem replayFrom_reaches {steps : List Step} :
    ∀ {s t : List Lease}, Reachable s → replayFrom s steps = some t → Reachable t := by
  induction steps with
  | nil =>
      intro s t reach run
      simp only [replayFrom, Option.some.injEq] at run
      exact run ▸ reach
  | cons x rest ih =>
      intro s t reach run
      simp only [replayFrom, Option.bind_eq_some_iff] at run
      obtain ⟨u, stepped, tail⟩ := run
      exact ih (step_reaches reach stepped) tail

/-- **The equivalence the runner rests on.** Whatever state a successful
replay of the artifact's steps produces is a state the wave-31 `Reachable`
relation covers — so a Rust transcription of `replay` that returns `some` has
produced a witness for `no_byte_has_two_write_authorities`' hypothesis. -/
theorem replay_reaches {steps : List Step} {t : List Lease}
    (run : replay steps = some t) : Reachable t :=
  replayFrom_reaches Reachable.start run

/-! ## CT07's own terms -/

/-- `BUFFER_PAYLOAD_BYTES`: the payload is one `int32_t`, so its halves are
two bytes each (residual `WRITE-LEND-TASKS-EXTENT`, unchanged here). -/
def PAYLOAD_BYTES : Nat := 4

/-- `lend buffer 0..2`. -/
def leftHalf : Range := { low := 0, high := 2 }

/-- `lend buffer 2..4`. -/
def rightHalf : Range := { low := 2, high := 4 }

/-- The ledger steps of CT07's reference execution, in the order it performs
them, each holder the child's own task identity: the first child's lease
opens at its spawn, the second's at its spawn, and each closes at its
creator's join. -/
def ct07Steps : List Step :=
  [ .lease 1 leftHalf, .lease 2 rightHalf, .reclaim 1, .reclaim 2 ]

/-- The two-thread point: both leases out at once. `openLease` conses, so the
second child's lease is at the head. -/
def ct07Ledger : List Lease :=
  [ { holder := 2, range := rightHalf }, { holder := 1, range := leftHalf } ]

/-- The artifact's two byte writes, each named by the owner-side byte the
rebasing law produces from the worker's relative index `0`. -/
def ct07Writes : List Write :=
  [ { writer := 1, byte := 0 }, { writer := 2, byte := 2 } ]

/-- Both opens are admitted, and the state they reach is `ct07Ledger`. -/
theorem ct07_both_leases_are_admitted :
    replay (ct07Steps.take 2) = some ct07Ledger := by decide

/-- The whole run replays: after both joins every lease has returned, so the
owner's whole-payload authority is reconstructed exactly there. -/
theorem ct07_the_run_returns_every_lease : replay ct07Steps = some [] := by decide

/-- **The artifact-side witness**: CT07's two-thread ledger state is
`Reachable`, by the replay of the steps the reference execution performs. -/
theorem ct07_ledger_is_reachable : Reachable ct07Ledger :=
  replay_reaches ct07_both_leases_are_admitted

/-- **The bound theorem.** `QCore.WriteLendTasks.no_byte_has_two_write_authorities`
on CT07's own ledger state: for every byte, at most one write authority. -/
theorem ct07_no_byte_has_two_write_authorities (b : Nat) :
    writeAuthorities ct07Ledger b ≤ 1 :=
  no_byte_has_two_write_authorities ct07_ledger_is_reachable b

/-- The artifact's two writes are admitted against the two-thread state: each
lies inside a lease its own writer holds. -/
theorem ct07_writes_are_admitted : ct07Writes.all (admitsWrite ct07Ledger) = true := by
  decide

/-- **The second theorem, instantiated.**
`QCore.WriteLendTasksExercise.no_byte_is_written_by_two_parties` on CT07's
ledger: two admitted writes to one byte are the same writer's. -/
theorem ct07_no_byte_is_written_by_two_parties {a b : Write} (same_byte : a.byte = b.byte)
    (first : admitsWrite ct07Ledger a = true) (second : admitsWrite ct07Ledger b = true) :
    a.writer = b.writer :=
  no_byte_is_written_by_two_parties ct07_ledger_is_reachable same_byte first second

/-- The transcription pin the Rust runner recomputes from CT07's own artifact
with wrapping `u64` arithmetic: the F02 fold's seed and multiplier over the
leases in ledger order, each encoded from its holder and bounds. -/
def encodeLease (lease : Lease) : Nat :=
  1 + 16 * (lease.holder + 8 * (lease.range.low + 64 * lease.range.high))

def ledgerModulus : Nat := 18446744073709551616
def ledgerSeed : Nat := 1469598103934665603
def ledgerMul : Nat := 1000003

def mixLedger (leases : List Lease) : Nat :=
  leases.foldl (fun accumulator lease =>
    (accumulator * ledgerMul + encodeLease lease + 1) % ledgerModulus) ledgerSeed

theorem ct07_ledger_digest : mixLedger ct07Ledger = 8876998389998880787 := by decide

/-! ## The scenario shape -/

/-- No two leases share a holder. -/
def distinctHolders : List Lease → Bool
  | [] => true
  | lease :: rest => rest.all (fun other => !(other.holder == lease.holder)) && distinctHolders rest

/-- **CT07's scenario, as a decision on a ledger state**: exactly two leases,
held by two different children, and every byte of the payload under exactly
one write authority — `writeAuthorities`, the theorem's own quantity, met with
equality where the theorem proves `≤ 1`. "Two disjoint halves" is the
partition; "independently" is the two distinct holders. -/
def dischargesDisjointHalves (s : List Lease) : Bool :=
  s.length == 2 && distinctHolders s
    && (List.range PAYLOAD_BYTES).all (fun b => writeAuthorities s b == 1)

theorem ct07_discharges_disjoint_halves : dischargesDisjointHalves ct07Ledger = true := by
  decide

/-- A discharged state has every payload byte held by exactly one lease. -/
theorem a_discharged_ledger_holds_every_byte_exactly_once {s : List Lease}
    (discharged : dischargesDisjointHalves s = true) (b : Nat) (inside : b < PAYLOAD_BYTES) :
    (leasesHolding s b).length = 1 := by
  simp only [dischargesDisjointHalves, Bool.and_eq_true, List.all_eq_true, List.mem_range,
    beq_iff_eq] at discharged
  obtain ⟨⟨two, _⟩, every⟩ := discharged
  have byte := every b inside
  cases s with
  | nil => simp at two
  | cons head rest =>
      unfold writeAuthorities at byte
      simpa using byte

/-- A discharged state's two leases are held by two different parties: the
scenario's "independently" is two holders, never one child leased twice. -/
theorem a_discharged_ledger_has_two_distinct_holders {s : List Lease}
    (discharged : dischargesDisjointHalves s = true) :
    ∃ a b, s = [a, b] ∧ a.holder ≠ b.holder := by
  simp only [dischargesDisjointHalves, Bool.and_eq_true, beq_iff_eq] at discharged
  obtain ⟨⟨two, distinct⟩, _⟩ := discharged
  match s, two with
  | [a, b], _ =>
      refine ⟨a, b, rfl, ?_⟩
      simp only [distinctHolders, List.all_cons, List.all_nil, Bool.and_true,
        Bool.not_eq_true', beq_eq_false_iff_ne] at distinct
      intro same
      exact distinct same.symm

/-! ## The three mutations, decided for the theorem's reason -/

/-- `lend buffer 0..3` beside `lend buffer 2..4`: the second open is refused —
`a_second_lease_is_admitted_iff_disjoint`'s disjointness clause, byte 2 being
shared. -/
theorem overlapping_halves_are_refused :
    replay [ .lease 1 { low := 0, high := 3 }, .lease 2 rightHalf ] = none := by decide

theorem the_overlap_is_the_disjointness_clause :
    overlaps { low := 0, high := 3 } rightHalf = true := by decide

/-- `lend buffer 2..2`: the second open is refused — `an_empty_range_is_not_a_lease`. -/
theorem an_empty_half_is_refused :
    replay [ .lease 1 leftHalf, .lease 2 { low := 2, high := 2 } ] = none := by decide

theorem the_empty_half_is_the_nonemptiness_clause :
    admitsLease [ { holder := 1, range := leftHalf } ] { low := 2, high := 2 } = false :=
  an_empty_range_is_not_a_lease _ _ (Nat.le_refl 2)

/-- One writer: a reachable state — the theorem holds of it — that is **not
the scenario**: one lease, and bytes 2 and 3 under no authority at all. -/
theorem a_single_writer_is_reachable_but_not_the_scenario :
    replay [ .lease 1 leftHalf ] = some [ { holder := 1, range := leftHalf } ] ∧
      dischargesDisjointHalves [ { holder := 1, range := leftHalf } ] = false := by decide

theorem a_single_writer_leaves_the_other_half_unheld :
    writeAuthorities [ { holder := 1, range := leftHalf } ] 2 = 0 := by decide

end QCore.Ct07LeaseLedger
