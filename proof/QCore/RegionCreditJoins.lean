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

import QCore.QuantRegionSource

/-! `REGION-CREDIT-JOINS` — the composition of a flat-epoch credit plan with the
quantitative error region, on the composed route `source.epoch.tasks.v1`
(wave 28).

This is the seam two wave-27 cards named from opposite sides: `CT23-T07-SEAM`
(credits and error edges live on the epoch routes) and
`REGION-CREDIT-WITH-JOINS` (`quant open` lives on the flat sequential route).
The Rust composition places one `error region … catch (E) quant open … {
discharge …; }` **after every spawn and join** of a composed epoch entry, with a
body admitting only the parent's own quantities — `credit N;`, `effort N;` —
and the decided error edge. Nothing about `QCore.ErrorRegion` or
`QCore.QuantRegionSource` moves here; this module proves the four facts the
composition itself rests on.

**The pack is parent-local.** A composed region body has no acquisition shape
at all — `ParentStmt` has exactly two constructors — so the pack it seals
carries **no duty** (`a_parent_local_body_builds_no_duty`), and the one thing
its opening can discard is the credit remainder
(`the_opening_abandons_exactly_the_parents_remainder`). That is CT22's
scenario — *"quant opening discards parent-local unused credit"* — stated as a
shape rather than as a convention.

**"After joins" means no child holds anything at the opening.** The parent's
task facts before the region are modelled as a list of spawn/join events, and
`live` computes the children still outstanding. A spawn answered by its join
leaves nothing (`a_join_returns_its_spawn`), and the sequential
spawn-then-join discipline the checked scoped-task ledger enforces leaves no
child at the opening (`the_sequential_spawn_join_discipline_leaves_no_child`) —
so the credit the discharge abandons is held by the parent alone.

**The region erases.** The emitted program of a composed region performs no
acquisition and no release (`the_region_erases_from_the_task_program`), which
is the modelled form of the Rust measurement that the emitted C is
byte-identical with and without the region (constraint 15l reached from a
second route).

**The published quantity is re-derived.** The Rust driver publishes the
abandoned credit only after cross-checking the model's own cleanup trace
against the reader's independent fold; `the_readers_fold_is_the_models_remainder`
is that cross-check's soundness — the two computations agree on every body, so
a disagreement in Rust is a defect of the wiring, never of the program.

What is **not** proved here, stated: that the Rust reader extracts these
statements faithfully from source tokens (a tested Rust boundary, the same
`QuantRegionSource` precedent), and that the checked scoped-task ledger's
"every creation joined before its scope exits" holds — that is the task
model's own theorem territory, consumed here as the hypothesis shape of the
`live` lemmas. -/

namespace QCore.RegionCreditJoins

open QCore.ErrorRegion
open QCore.QuantRegionSource

/-- A statement of a composed region body: the parent's own quantities and
nothing else. There is deliberately **no** acquisition constructor — a
task-route resource has its own custody model, and sealing one into a pack is
the named follow-on `REGION-JOINS-DUTIES`, not an omission. -/
inductive ParentStmt where
  | credit (n : Nat)
  | effort (n : Nat)
  deriving Repr, DecidableEq

/-- The composed body is a restriction of the sequential route's admitted body:
each shape embeds into `QuantRegionSource.BodyStmt` unchanged, so every theorem
of that module applies to the embedded body. -/
def embed : ParentStmt → BodyStmt
  | .credit n => .credit n
  | .effort n => .effort n

/-- The admitted shapes as one list, so a widening must change a theorem rather
than slip past one. -/
theorem admitted_parent_shapes_are_exactly_these (s : ParentStmt) :
    (∃ n, s = .credit n) ∨ (∃ n, s = .effort n) := by
  cases s with
  | credit n => exact Or.inl ⟨n, rfl⟩
  | effort n => exact Or.inr ⟨n, rfl⟩

/-! ## The pack is parent-local -/

theorem dutiesFrom_parent (held : List ResourceId) :
    ∀ body : List ParentStmt, dutiesFrom held (body.map embed) = held
  | [] => rfl
  | .credit n :: rest => by
      simpa [embed, dutiesFrom] using dutiesFrom_parent held rest
  | .effort n :: rest => by
      simpa [embed, dutiesFrom] using dutiesFrom_parent held rest

/-- A body of parent-local quantities accumulates **no** cleanup duty. -/
theorem a_parent_local_body_builds_no_duty (body : List ParentStmt) :
    duties (body.map embed) = [] :=
  dutiesFrom_parent [] body

/-- The pack a composed region seals after the joins. -/
def parentPack (region : RegionName) (error : ErrorName) (baseline : Ledger)
    (body : List ParentStmt) (held : Nat) : CleanupPack :=
  sealedPack region error baseline (body.map embed) held

/-- The sealed pack holds only the quantity: no action list, exactly the
remainder. -/
theorem the_pack_after_joins_holds_only_the_quantity (region : RegionName)
    (error : ErrorName) (baseline : Ledger) (body : List ParentStmt)
    (held : Nat) :
    (parentPack region error baseline body held).actions = [] ∧
      (parentPack region error baseline body held).creditRemainder = held := by
  refine ⟨?_, rfl⟩
  simpa [parentPack, sealed_pack_carries_the_bodys_duties] using
    a_parent_local_body_builds_no_duty body

/-- **The CT22 property.** Discharging the opened pack restores the baseline,
runs **no** cleanup action, abandons **exactly** the parent-local remainder and
consumes the capability. The abandonment is the kernel's own explicit event,
never an omission. -/
theorem the_opening_abandons_exactly_the_parents_remainder (region : RegionName)
    (error : ErrorName) (baseline : Ledger) (body : List ParentStmt)
    (held : Nat) :
    discharge (.opened (parentPack region error baseline body held)) =
      some
        { ledger := baseline
          cleanupTrace := []
          abandonedCredit := held
          capability := .consumed } := by
  have h := a_parent_local_body_builds_no_duty body
  simp [discharge, parentPack, sealedPack, sealPack, duties] at h ⊢
  simp [h]

/-- The boundary: a parent that spent everything abandons nothing, and the
consumption still happens — `spent_out.qcore`'s shape. -/
theorem discharging_an_empty_pack_still_consumes (region : RegionName)
    (error : ErrorName) (baseline : Ledger) (body : List ParentStmt) :
    discharge (.opened (parentPack region error baseline body 0)) =
      some
        { ledger := baseline
          cleanupTrace := []
          abandonedCredit := 0
          capability := .consumed } :=
  the_opening_abandons_exactly_the_parents_remainder region error baseline body 0

/-! ## The erasure -/

theorem a_parent_local_body_emits_no_acquisition (body : List ParentStmt) :
    emittedAcquisitions (body.map embed) = [] := by
  induction body with
  | nil => rfl
  | cons s rest ih => cases s <;> simpa [embed, emittedAcquisitions] using ih

theorem a_parent_local_body_emits_no_release (body : List ParentStmt) :
    emittedBodyReleases (body.map embed) = [] := by
  induction body with
  | nil => rfl
  | cons s rest ih => cases s <;> simpa [embed, emittedBodyReleases] using ih

/-- The composed region contributes **nothing** to the task program: no
acquisition and no release, with or without the discharge. This is the modelled
form of the byte-identical emitted C. -/
theorem the_region_erases_from_the_task_program (body : List ParentStmt)
    (b : Nat) :
    emittedAcquisitions (body.map embed) = [] ∧
      emittedReleases (body.map embed) [.discharge b] = [] := by
  refine ⟨a_parent_local_body_emits_no_acquisition body, ?_⟩
  simp [emittedReleases, handlerAdmits, dischargeCount,
    a_parent_local_body_builds_no_duty body,
    a_parent_local_body_emits_no_release body]

/-! ## The re-derived quantity -/

/-- The reader's own fold over the written statements — the figure the Rust
driver computes independently of the model. -/
def parentRemainderFrom : Nat → List ParentStmt → Option Nat
  | held, [] => some held
  | held, .credit n :: rest => parentRemainderFrom (held + n) rest
  | held, .effort n :: rest =>
      if n ≤ held then parentRemainderFrom (held - n) rest else none

/-- **The cross-check's soundness**: the reader's fold and the model's
remainder are one function on every body, so the driver's re-derivation can
disagree only through a defect of the wiring, never of the program. -/
theorem the_readers_fold_is_the_models_remainder :
    ∀ (held : Nat) (body : List ParentStmt),
      parentRemainderFrom held body = remainderFrom held (body.map embed)
  | _, [] => rfl
  | held, .credit n :: rest => by
      simpa [embed, parentRemainderFrom, remainderFrom] using
        the_readers_fold_is_the_models_remainder (held + n) rest
  | held, .effort n :: rest => by
      by_cases h : n ≤ held
      · simpa [embed, parentRemainderFrom, remainderFrom, h] using
          the_readers_fold_is_the_models_remainder (held - n) rest
      · simp [embed, parentRemainderFrom, remainderFrom, h]

/-- Overspending still refuses on this route: the quantity never wraps. -/
theorem overspending_refuses_here (n : Nat) (h : 0 < n) :
    parentRemainderFrom 0 [.effort n] = none := by
  simp [parentRemainderFrom, Nat.not_le.mpr h]

/-- Non-vacuity, on the flagship's own figures: `credit 5; effort 2;` leaves 3
for the opening to abandon, and `credit 5; effort 5;` leaves nothing. -/
theorem the_flagship_abandons_three :
    parentRemainderFrom 0 [.credit 5, .effort 2] = some 3 := by decide

theorem the_boundary_abandons_nothing :
    parentRemainderFrom 0 [.credit 5, .effort 5] = some 0 := by decide

/-! ## After the joins, no child holds anything -/

/-- The parent's task facts before the opening, in written order — which is
execution order on this route, since the entry is one flat statement
sequence. -/
inductive ParentEvent where
  | spawn (t : Nat)
  | join (t : Nat)
  deriving Repr, DecidableEq

/-- The children still outstanding after a prefix of the parent's statements:
a spawn adds its child, a join returns it. Threaded forward, exactly as the
duty frame is. -/
def liveFrom : List Nat → List ParentEvent → List Nat
  | held, [] => held
  | held, .spawn t :: rest => liveFrom (held ++ [t]) rest
  | held, .join t :: rest => liveFrom (held.erase t) rest

def live (events : List ParentEvent) : List Nat := liveFrom [] events

theorem erase_append (t : Nat) :
    ∀ held : List Nat, t ∉ held → (held ++ [t]).erase t = held
  | [], _ => by simp
  | x :: rest, h => by
      have hx : (x == t) = false := by
        have : x ≠ t := fun e => h (by simp [e])
        simpa using this
      have hrest : t ∉ rest := fun m => h (List.mem_cons_of_mem _ m)
      simp [List.cons_append, hx, erase_append t rest hrest]

/-- A spawn answered by its join leaves the outstanding set unchanged — the
pairwise form of "after joins". The freshness hypothesis is the scoped-task
ledger's own fact that one handle names one live child. -/
theorem a_join_returns_its_spawn (t : Nat) (held : List Nat)
    (h : t ∉ held) (rest : List ParentEvent) :
    liveFrom held (.spawn t :: .join t :: rest) = liveFrom held rest := by
  simp [liveFrom, erase_append t held h]

/-- The discipline the checked scoped-task ledger enforces — every creation
joined, in order — leaves **no child at the opening**, which is what makes the
sealed remainder parent-local. -/
theorem the_sequential_spawn_join_discipline_leaves_no_child :
    ∀ tasks : List Nat,
      live (tasks.flatMap fun t => [.spawn t, .join t]) = []
  | [] => rfl
  | t :: rest => by
      have step :
          live ((t :: rest).flatMap fun t => [.spawn t, .join t]) =
            live (rest.flatMap fun t => [.spawn t, .join t]) := by
        simp [live, List.flatMap_cons,
          a_join_returns_its_spawn t [] (by simp)
            (rest.flatMap fun t => [.spawn t, .join t])]
      exact step.trans (the_sequential_spawn_join_discipline_leaves_no_child rest)

end QCore.RegionCreditJoins
