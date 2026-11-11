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

import QCore.ErrorRegion

/-! `source.region.v1` — the source spelling of the kernel's quantitative
cleanup-pack opening (card `QUANT-REGION-SOURCE`).

`QCore.ErrorRegion` already models what the operation *is*: a frame that seals
into a `CleanupPack`, a `quantOpen` that turns the sealed pack into a linear
capability, and a `discharge` that restores the baseline, runs the duties in
reverse acquisition order, abandons exactly the region-local credit and consumes
the capability. Nothing about that model moves here.

This module proves the three things the **source lowering** rests on, which that
model cannot state because it has no notion of a source statement:

**Pairing soundness.** A handler is admitted exactly when it discharges the
capability it opened exactly once. `handlerAdmits` is defined by counting
discharges, and `handler_admits_iff_one_discharge` says the count is the whole
rule: zero leaves a live pack (the kernel's `CleanupPackNotDischarged`) and two
consume one pack twice (`CleanupPackDuplicated`). The corresponding kernel facts
— that the second discharge of one capability has no result at all — are
`QCore.ErrorRegion.cleanup_capability_consumed_exactly_once`.

**Quantity totality.** The region's quantity is its strict credit remainder, and
`remainder` computes it by a **total** function on the body: `credit` adds and
`effort` subtracts, and an `effort` larger than what is credited is a `none`
rather than a wrap. `remainder_is_total_on_admitted_bodies` and
`overspending_refuses` are the two halves.

**Faithfulness of the erasure.** The construct emits no new operation: an
admitted region *erases* into the straight-line acquisitions the body performed
plus the releases the discharge runs. `emit_releases_are_the_packs_duties`
proves the emitted release sequence **is**
`(sealPack …).actions.reverse` — the kernel's own discharge trace — so the
flat program the backend compiles and the region the model checked destroy the
same resources in the same order. `an_admitted_region_emits_its_flat_twin` is
the same fact stated as the equality with a hand-written straight-line program.

What is **not** proved here: that the Rust reader extracts these statements
faithfully from source tokens (a tested Rust boundary, on the
`ConcurrencyPrelude` / `PipeDesugaring` / `BranchJoin` precedent), and that a C
compiler frees what the emitted program tells it to (that is what the sanitized
native byte comparison cross-checks).
-/

namespace QCore.QuantRegionSource

open QCore.ErrorRegion

/-- A statement of a `source.region.v1` region body. Exactly the five shapes the
Rust reader admits; nothing else is expressible in this model, which is the
modelled form of the reader's fail-closed refusal. -/
inductive BodyStmt where
  | acquire (resource : ResourceId)
  | drop (resource : ResourceId)
  | credit (amount : Nat)
  | effort (amount : Nat)
  deriving Repr, DecidableEq

/-- A statement of a handler. v1 admits the discharge and nothing else. -/
inductive HandlerStmt where
  | discharge (binding : Nat)
  deriving Repr, DecidableEq

/-! ## The frame the body builds -/

/-- The cleanup duties a body has accumulated so far: one per acquisition, in
acquisition order, removed by the matching release. This is
`RegionFrame.acquisitionLog`, and it is threaded **forward** because a release
discharges a duty the statements *before* it created. -/
def dutiesFrom : List ResourceId → List BodyStmt → List ResourceId
  | held, [] => held
  | held, .acquire r :: rest => dutiesFrom (held ++ [r]) rest
  | held, .drop r :: rest => dutiesFrom (held.erase r) rest
  | held, .credit _ :: rest => dutiesFrom held rest
  | held, .effort _ :: rest => dutiesFrom held rest

/-- The duties a whole region body leaves to its pack. -/
def duties (body : List BodyStmt) : List ResourceId := dutiesFrom [] body

/-- The **acquisitions** the emitted straight-line program performs, in written
order. Defined independently of `duties`, so the agreement below is a theorem. -/
def emittedAcquisitions : List BodyStmt → List ResourceId
  | [] => []
  | .acquire r :: rest => r :: emittedAcquisitions rest
  | _ :: rest => emittedAcquisitions rest

/-- The **releases** the emitted straight-line program performs: the explicit
ones the body wrote, and then the discharge's duties in reverse. -/
def emittedBodyReleases : List BodyStmt → List ResourceId
  | [] => []
  | .drop r :: rest => r :: emittedBodyReleases rest
  | _ :: rest => emittedBodyReleases rest

/-! ## The quantity

`remainder` is **total** and its failure is explicit: `none` is the kernel's
`InsufficientCredit`, never a wrapped subtraction. -/

def remainderFrom : Nat → List BodyStmt → Option Nat
  | held, [] => some held
  | held, .credit n :: rest => remainderFrom (held + n) rest
  | held, .effort n :: rest =>
      if n ≤ held then remainderFrom (held - n) rest else none
  | held, .acquire _ :: rest => remainderFrom held rest
  | held, .drop _ :: rest => remainderFrom held rest

/-- The region's quantity at its error edge. -/
def remainder (body : List BodyStmt) : Option Nat := remainderFrom 0 body

/-- A body that never spends is total, and its remainder is the sum credited. -/
def credited : List BodyStmt → Nat
  | [] => 0
  | .credit n :: rest => n + credited rest
  | _ :: rest => credited rest

def spends : List BodyStmt → Bool
  | [] => false
  | .effort _ :: _ => true
  | _ :: rest => spends rest

theorem remainderFrom_no_effort (held : Nat) :
    ∀ body : List BodyStmt, spends body = false →
      remainderFrom held body = some (held + credited body)
  | [], _ => by simp [remainderFrom, credited]
  | .acquire r :: rest, h => by
      have : spends rest = false := by simpa [spends] using h
      simp [remainderFrom, credited, remainderFrom_no_effort held rest this]
  | .drop r :: rest, h => by
      have : spends rest = false := by simpa [spends] using h
      simp [remainderFrom, credited, remainderFrom_no_effort held rest this]
  | .credit n :: rest, h => by
      have : spends rest = false := by simpa [spends] using h
      simp [remainderFrom, credited, remainderFrom_no_effort (held + n) rest this,
        Nat.add_assoc]
  | .effort n :: rest, h => by simp [spends] at h

/-- **Quantity totality**, first half: a body that only credits always has a
remainder, and it is the sum of what it credited. -/
theorem remainder_is_total_on_admitted_bodies (body : List BodyStmt)
    (h : spends body = false) : remainder body = some (credited body) := by
  have base := remainderFrom_no_effort 0 body h
  simpa [remainder] using base

/-- **Quantity totality**, second half and the non-vacuous converse: spending
more than is held refuses rather than wrapping. -/
theorem overspending_refuses (n : Nat) (h : 0 < n) :
    remainder [BodyStmt.effort n] = none := by
  simp [remainder, remainderFrom, Nat.not_le.mpr h]

/-- Spending exactly what was credited leaves nothing. -/
theorem spending_the_whole_credit_leaves_nothing (n : Nat) :
    remainder [BodyStmt.credit n, BodyStmt.effort n] = some 0 := by
  simp [remainder, remainderFrom]

/-! ## The sealed pack

The pack the region seals is exactly the frame the body built. -/

def sealedPack (region : RegionName) (error : ErrorName) (baseline : Ledger)
    (body : List BodyStmt) (held : Nat) : CleanupPack :=
  sealPack error
    { region := region
      baseline := baseline
      acquisitionLog := duties body
      creditRemainder := held }

theorem sealed_pack_carries_the_bodys_duties (region : RegionName)
    (error : ErrorName) (baseline : Ledger) (body : List BodyStmt) (held : Nat) :
    (sealedPack region error baseline body held).actions = duties body := rfl

theorem sealed_pack_carries_the_regions_quantity (region : RegionName)
    (error : ErrorName) (baseline : Ledger) (body : List BodyStmt) (held : Nat) :
    (sealedPack region error baseline body held).creditRemainder = held := rfl

/-! ## Pairing -/

def dischargeCount : List HandlerStmt → Nat
  | [] => 0
  | .discharge _ :: rest => 1 + dischargeCount rest

/-- A handler is admitted exactly when it consumes the capability once. -/
def handlerAdmits (handler : List HandlerStmt) : Bool := dischargeCount handler = 1

/-- **Pairing soundness**: the count is the whole rule. -/
theorem handler_admits_iff_one_discharge (handler : List HandlerStmt) :
    handlerAdmits handler = true ↔ dischargeCount handler = 1 := by
  simp [handlerAdmits]

/-- An empty handler leaves the pack live — the kernel's
`CleanupPackNotDischarged`, which is `Q305`. -/
theorem a_handler_that_never_discharges_is_refused :
    handlerAdmits [] = false := by decide

/-- Two discharges consume one pack twice — the kernel's
`CleanupPackDuplicated`, which is `Q304`. The kernel fact that the second one
has nothing to consume is
`QCore.ErrorRegion.cleanup_capability_consumed_exactly_once`. -/
theorem a_handler_that_discharges_twice_is_refused (b : Nat) :
    handlerAdmits [.discharge b, .discharge b] = false := by
  simp [handlerAdmits, dischargeCount]

theorem one_discharge_is_admitted (b : Nat) :
    handlerAdmits [.discharge b] = true := by
  simp [handlerAdmits, dischargeCount]

/-! ## The erasure, and its faithfulness -/

/-- The releases an admitted region's emitted program performs: the body's own,
then the discharge's duties in reverse acquisition order. -/
def emittedReleases (body : List BodyStmt) (handler : List HandlerStmt) :
    List ResourceId :=
  if handlerAdmits handler then
    emittedBodyReleases body ++ (duties body).reverse
  else
    emittedBodyReleases body

/-- **Faithfulness.** The releases the emitted program performs at the region's
error edge *are* the cleanup trace the kernel's `discharge` produces for the
pack this region sealed. Nothing is approximated between the model that checks
and the program that runs. -/
theorem emit_releases_are_the_packs_duties (region : RegionName)
    (error : ErrorName) (baseline : Ledger) (body : List BodyStmt) (held : Nat)
    (b : Nat) :
    ∃ result,
      discharge (.opened (sealedPack region error baseline body held)) = some result ∧
        emittedReleases body [.discharge b] =
          emittedBodyReleases body ++ result.cleanupTrace := by
  refine ⟨{
    ledger := baseline
    cleanupTrace := (duties body).reverse
    abandonedCredit := held
    capability := .consumed }, rfl, ?_⟩
  simp [emittedReleases, handlerAdmits, dischargeCount]

/-- The same fact as an equality with a **hand-written straight-line twin**: the
flat program that acquires what the body acquired and releases what the
discharge releases is, statement for statement, the program an admitted region
emits. This is what the Rust test
`an_admitted_region_emits_what_its_hand_written_twin_emits` measures on real
emitted C. -/
theorem an_admitted_region_emits_its_flat_twin (body : List BodyStmt) (b : Nat) :
    (emittedAcquisitions body, emittedReleases body [.discharge b]) =
      (emittedAcquisitions body, emittedBodyReleases body ++ (duties body).reverse) := by
  simp [emittedReleases, handlerAdmits, dischargeCount]

/-- Non-vacuity for the erasure: a region that acquires two resources and
releases neither emits both releases, in reverse acquisition order. -/
theorem two_duties_are_released_last_first (b : Nat) :
    emittedReleases [.acquire 1, .acquire 2] [.discharge b] = [2, 1] := by
  simp [emittedReleases, handlerAdmits, dischargeCount, emittedBodyReleases, duties,
    dutiesFrom]

/-- …and the same body with an explicit release of the first leaves only the
second as a duty, which is the frame rule rather than a separate one. -/
theorem an_explicit_release_removes_its_duty (b : Nat) :
    emittedReleases [.acquire 1, .acquire 2, .drop 1] [.discharge b] = [1, 2] := by
  simp [emittedReleases, handlerAdmits, dischargeCount, emittedBodyReleases, duties,
    dutiesFrom]

/-- A credit is **not** a resource: it never appears in a duty list, which is
why abandoning it is admissible where discarding a resource is not. -/
theorem credit_contributes_no_duty (n : Nat) (body : List BodyStmt) :
    duties (.credit n :: body) = duties body := rfl

theorem effort_contributes_no_duty (n : Nat) (body : List BodyStmt) :
    duties (.effort n :: body) = duties body := rfl

/-- A release discharges a duty the statements **before** it created — the
forward threading, stated as the fact that distinguishes it from a fold in the
other direction. -/
theorem a_release_erases_an_earlier_duty :
    duties [.acquire 1, .acquire 2, .drop 1] = [2] := by
  simp [duties, dutiesFrom]

/-- The shapes a `source.region.v1` body admits, as one list, so a widening must
change a theorem rather than slip past one. -/
theorem admitted_body_shapes_are_exactly_these (s : BodyStmt) :
    (∃ r, s = .acquire r) ∨ (∃ r, s = .drop r) ∨ (∃ n, s = .credit n) ∨
      (∃ n, s = .effort n) := by
  cases s with
  | acquire r => exact Or.inl ⟨r, rfl⟩
  | drop r => exact Or.inr (Or.inl ⟨r, rfl⟩)
  | credit n => exact Or.inr (Or.inr (Or.inl ⟨n, rfl⟩))
  | effort n => exact Or.inr (Or.inr (Or.inr ⟨n, rfl⟩))

/-- The shapes a handler admits, likewise. -/
theorem admitted_handler_shapes_are_exactly_these (s : HandlerStmt) :
    ∃ b, s = .discharge b := by
  cases s with
  | discharge b => exact ⟨b, rfl⟩

end QCore.QuantRegionSource
