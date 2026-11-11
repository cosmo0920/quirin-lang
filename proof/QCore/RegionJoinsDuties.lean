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

import QCore.RegionCreditJoins

/-! `REGION-JOINS-DUTIES` — acquisitions in the composed region body, on the
composed route `source.epoch.tasks.v1` (wave 29). CT25's duty half: *"cleanup
destroys owner after joins"*.

Wave 28's `REGION-CREDIT-JOINS` landed the composed region with a **duty-free**
pack: `ParentStmt` has exactly two constructors, so the sealed pack carried
only the credit remainder (standing constraint 15m). This card supersedes that
premise **deliberately**: the composed body now admits the task route's own
acquisition and release — `let NAME = qcore_buffer_new(INT)?;`, `drop NAME;` —
so `OwnerStmt` below has exactly four constructors, each embedding unchanged
into the sequential route's `QuantRegionSource.BodyStmt`. Nothing about
`QCore.ErrorRegion`, `QCore.QuantRegionSource` or `QCore.RegionCreditJoins`
moves here; the wave-28 module's theorems still hold of the duty-free
sublanguage, and `the_duty_free_sublanguage_is_unchanged` says the two
embeddings agree on it.

**The pack after the joins carries the body's own duties.** The sealed pack's
actions are exactly the acquisitions the body did not release, in acquisition
order (`the_pack_after_joins_carries_the_bodys_duties`), and nothing a child
held can be among them: a duty is either inherited from the frame the region
was entered with — empty, on this route — or one of the body's own
acquisitions (`duties_are_the_bodys_own`). The "after joins" premise itself —
no child live at the opening — is `QCore.RegionCreditJoins`'s own `live`
machinery, consumed unchanged.

**The CT25 property.** Discharging the opened pack restores the baseline,
destroys the owners in **reverse acquisition order**, abandons exactly the
remainder and consumes the capability
(`the_cleanup_destroys_the_owners_after_the_joins`) — the kernel's own
`discharge`, definitionally.

**The erasure is a program now, and it is the flat twin.** The Rust split
keeps each acquisition at its own byte offsets in the task half and writes one
`drop NAME;` per remaining duty after them; the releases that program performs
are the model's own cleanup trace (`the_splits_releases_are_the_models_trace`,
the sequential route's `emit_releases_are_the_packs_duties` reached from this
route), so the program the unedited scoped-task route compiles and the pack
the model checked destroy the same owners in the same order.

What is **not** proved here, stated: that the Rust reader extracts these
statements faithfully from source tokens and that the split's textual
placement lands where the plan says (both tested Rust boundaries, the
`QuantRegionSource` precedent), and that the simultaneous two-owner shape is
refused — that refusal is the **task route's** custody rule over the erased
program's real `?` failure edge (`REGION-JOINS-MULTI-OWNER`), a fact of the
task model rather than of this one. -/

namespace QCore.RegionJoinsDuties

open QCore.ErrorRegion
open QCore.QuantRegionSource

/-- A statement of the composed region body after this card: the parent's own
quantities, and the task route's own acquisition and release. Exactly the four
shapes the Rust reader admits — the wave-28 `ParentStmt` plus the two owner
shapes this card lands. -/
inductive OwnerStmt where
  | acquire (resource : ResourceId)
  | drop (resource : ResourceId)
  | credit (amount : Nat)
  | effort (amount : Nat)
  deriving Repr, DecidableEq

/-- Each composed shape is a sequential-route body shape unchanged, so every
theorem of `QuantRegionSource` applies to the embedded body. -/
def embed : OwnerStmt → BodyStmt
  | .acquire r => .acquire r
  | .drop r => .drop r
  | .credit n => .credit n
  | .effort n => .effort n

/-- The admitted shapes as one list, so a widening must change a theorem
rather than slip past one. Four constructors where wave 28 had two — the
supersession of constraint 15m's premise, stated as a shape. -/
theorem admitted_owner_shapes_are_exactly_these (s : OwnerStmt) :
    (∃ r, s = .acquire r) ∨ (∃ r, s = .drop r) ∨
      (∃ n, s = .credit n) ∨ (∃ n, s = .effort n) := by
  cases s with
  | acquire r => exact Or.inl ⟨r, rfl⟩
  | drop r => exact Or.inr (Or.inl ⟨r, rfl⟩)
  | credit n => exact Or.inr (Or.inr (Or.inl ⟨n, rfl⟩))
  | effort n => exact Or.inr (Or.inr (Or.inr ⟨n, rfl⟩))

/-- The wave-28 duty-free sublanguage, embedded into this card's body. -/
def liftParent : QCore.RegionCreditJoins.ParentStmt → OwnerStmt
  | .credit n => .credit n
  | .effort n => .effort n

/-- **The supersession is conservative.** On the duty-free sublanguage the two
embeddings are one function, so every wave-28 theorem — the parent-local pack,
the CT22 abandonment, the erasure to nothing — still holds of exactly the
bodies it held of. -/
theorem the_duty_free_sublanguage_is_unchanged
    (s : QCore.RegionCreditJoins.ParentStmt) :
    embed (liftParent s) = QCore.RegionCreditJoins.embed s := by
  cases s <;> rfl

/-! ## The pack after the joins -/

/-- The pack a composed region seals after the joins, its body now carrying
real acquisitions. -/
def ownerPack (region : RegionName) (error : ErrorName) (baseline : Ledger)
    (body : List OwnerStmt) (held : Nat) : CleanupPack :=
  sealedPack region error baseline (body.map embed) held

/-- The sealed pack's actions are exactly the body's unreleased acquisitions,
in acquisition order — the duty fold the Rust driver re-derives before
publication. -/
theorem the_pack_after_joins_carries_the_bodys_duties (region : RegionName)
    (error : ErrorName) (baseline : Ledger) (body : List OwnerStmt)
    (held : Nat) :
    (ownerPack region error baseline body held).actions =
      duties (body.map embed) := rfl

/-- …and still exactly the remainder: an owner is not a credit. -/
theorem the_pack_still_carries_the_quantity (region : RegionName)
    (error : ErrorName) (baseline : Ledger) (body : List OwnerStmt)
    (held : Nat) :
    (ownerPack region error baseline body held).creditRemainder = held := rfl

/-- Membership through a `List.erase`, hand-rolled as `RegionCreditJoins`'s
`erase_append` is. -/
theorem mem_of_mem_erase (b : ResourceId) :
    ∀ (l : List ResourceId) (a : ResourceId), a ∈ l.erase b → a ∈ l
  | [], _, h => by simp at h
  | x :: rest, a, h => by
      by_cases hx : (x == b) = true
      · exact List.mem_cons_of_mem _ (by simpa [List.erase_cons, hx] using h)
      · have hx' : (x == b) = false := by
          simpa using hx
        rcases (by simpa [List.erase_cons, hx'] using h : a = x ∨ a ∈ rest.erase b)
          with h1 | h2
        · simp [h1]
        · exact List.mem_cons_of_mem _ (mem_of_mem_erase b rest a h2)

theorem dutiesFrom_are_held_or_acquired (held : List ResourceId) :
    ∀ (body : List OwnerStmt) (r : ResourceId),
      r ∈ dutiesFrom held (body.map embed) →
        r ∈ held ∨ r ∈ emittedAcquisitions (body.map embed)
  | [], r, h => Or.inl (by simpa [dutiesFrom] using h)
  | .acquire a :: rest, r, h => by
      rcases dutiesFrom_are_held_or_acquired (held ++ [a]) rest r
          (by simpa [embed, dutiesFrom] using h) with h1 | h2
      · rcases List.mem_append.mp h1 with h3 | h4
        · exact Or.inl h3
        · exact Or.inr (by simp [embed, emittedAcquisitions, List.mem_singleton.mp h4])
      · exact Or.inr (by simpa [embed, emittedAcquisitions] using List.mem_cons_of_mem a h2)
  | .drop d :: rest, r, h => by
      rcases dutiesFrom_are_held_or_acquired (held.erase d) rest r
          (by simpa [embed, dutiesFrom] using h) with h1 | h2
      · exact Or.inl (mem_of_mem_erase d held r h1)
      · exact Or.inr (by simpa [embed, emittedAcquisitions] using h2)
  | .credit n :: rest, r, h => by
      rcases dutiesFrom_are_held_or_acquired held rest r
          (by simpa [embed, dutiesFrom] using h) with h1 | h2
      · exact Or.inl h1
      · exact Or.inr (by simpa [embed, emittedAcquisitions] using h2)
  | .effort n :: rest, r, h => by
      rcases dutiesFrom_are_held_or_acquired held rest r
          (by simpa [embed, dutiesFrom] using h) with h1 | h2
      · exact Or.inl h1
      · exact Or.inr (by simpa [embed, emittedAcquisitions] using h2)

/-- **Nothing a child held is in the pack.** Every duty of a composed region's
pack is one of the body's own acquisitions: the fold starts from the empty
frame — which is what "after every join" buys, no inherited duty and no live
child — so the cleanup destroys owners the parent's own epilogue acquired and
nothing else. -/
theorem duties_are_the_bodys_own (body : List OwnerStmt) (r : ResourceId)
    (h : r ∈ duties (body.map embed)) :
    r ∈ emittedAcquisitions (body.map embed) := by
  rcases dutiesFrom_are_held_or_acquired [] body r h with h1 | h2
  · simp at h1
  · exact h2

/-! ## The CT25 property -/

/-- **The cleanup destroys the owners after the joins.** Discharging the
opened pack restores the baseline, destroys the remaining owners in reverse
acquisition order, abandons exactly the parent-local remainder and consumes
the capability — the kernel's own `discharge`, definitionally. -/
theorem the_cleanup_destroys_the_owners_after_the_joins (region : RegionName)
    (error : ErrorName) (baseline : Ledger) (body : List OwnerStmt)
    (held : Nat) :
    discharge (.opened (ownerPack region error baseline body held)) =
      some
        { ledger := baseline
          cleanupTrace := (duties (body.map embed)).reverse
          abandonedCredit := held
          capability := .consumed } := rfl

/-- A handler that walks away from a held owner is refused — the kernel's
`CleanupPackNotDischarged` (`Q305`), CT21's shape now reachable with a real
duty on this route. -/
theorem a_handler_that_walks_away_from_a_duty_is_refused :
    handlerAdmits ([] : List HandlerStmt) = false :=
  a_handler_that_never_discharges_is_refused

/-- A duty alone makes the pack nonempty: one live owner, zero credit, and the
catch still requires its opening — the kernel's
`CatchWithoutRequiredQuantOpen` (`Q306`), previously reachable on this route
only through the remainder. -/
theorem a_duty_alone_makes_the_pack_nonempty (region : RegionName)
    (error : ErrorName) (baseline : Ledger) (r : ResourceId) :
    (ownerPack region error baseline [.acquire r] 0).actions ≠ [] := by
  simp [ownerPack, sealedPack, sealPack, duties, dutiesFrom, embed]

/-! ## The erasure, now a program -/

/-- The releases the erased task half performs are the model's own cleanup
trace for the pack this region sealed — the sequential route's
`emit_releases_are_the_packs_duties`, reached from this route. This is the
soundness of the driver's second cross-check: the split's written drops and
the model's destroys are one computation. -/
theorem the_splits_releases_are_the_models_trace (region : RegionName)
    (error : ErrorName) (baseline : Ledger) (body : List OwnerStmt)
    (held : Nat) (b : Nat) :
    ∃ result,
      discharge (.opened (ownerPack region error baseline body held)) =
          some result ∧
        emittedReleases (body.map embed) [.discharge b] =
          emittedBodyReleases (body.map embed) ++ result.cleanupTrace :=
  emit_releases_are_the_packs_duties region error baseline (body.map embed) held b

/-- An owner statement moves no credit: the reader's quantity fold is
untouched by acquisitions and releases, so the wave-28 cross-check reads the
same figure through them. -/
theorem an_owner_moves_no_credit (held : Nat) (r : ResourceId)
    (rest : List OwnerStmt) :
    remainderFrom held ((OwnerStmt.acquire r :: rest).map embed) =
        remainderFrom held (rest.map embed) ∧
      remainderFrom held ((OwnerStmt.drop r :: rest).map embed) =
        remainderFrom held (rest.map embed) := by
  exact ⟨rfl, rfl⟩

/-! ## The flagship's own figures -/

/-- Non-vacuity on `CT25/case.qcore`'s shape: one owner, credit 5, effort 2 —
one duty, remainder 3, and the discharge's trace destroys exactly the owner. -/
theorem the_flagship_seals_one_owner_and_abandons_three :
    duties ([OwnerStmt.acquire 1, .credit 5, .effort 2].map embed) = [1] ∧
      remainder ([OwnerStmt.acquire 1, .credit 5, .effort 2].map embed) =
        some 3 := by
  constructor <;> rfl

/-- `dropped_owner.qcore`'s shape: a body release erases its duty, so the pack
is duty-free again and only the remainder is abandoned. -/
theorem a_body_release_erases_its_duty :
    duties ([OwnerStmt.acquire 1, .drop 1, .credit 5, .effort 2].map embed) =
      [] := by rfl

/-- `sequenced_owners.qcore`'s shape: the first owner released before the
second is acquired leaves exactly the second as a duty. -/
theorem a_sequenced_body_leaves_the_second_owner :
    duties ([OwnerStmt.acquire 1, .drop 1, .acquire 2].map embed) = [2] := by
  rfl

/-- Two surviving duties are destroyed last-first — the reverse the split
writes and the model's trace both name. -/
theorem two_owners_are_destroyed_last_first :
    (duties ([OwnerStmt.acquire 1, .acquire 2].map embed)).reverse = [2, 1] := by
  rfl

end QCore.RegionJoinsDuties
