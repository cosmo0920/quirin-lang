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

import QCore.RegionJoinsDuties
import QCore.ReadLoan
import QCore.EpochSourceLowering

/-! `REGION-JOINS-LOAN` — read-loan captures and owner adoption on the
composed route `source.epoch.tasks.v1` (wave 30). CT25's loan half, completing
the scenario whose duty half wave 29 landed: *"cleanup destroys owner after
joins **and loan expiry**"*.

Two languages widen, each conservatively, and one custody model is consumed
unchanged:

**The capture language.** Wave 15's `EpochSourceLowering.Capture` has two
constructors and recorded "a read loan has no composed spelling"
(`T08-COMPOSE-LOAN`). This card supersedes that **deliberately**:
`LoanCapture` below has exactly three, the third being the counted read share
the task route already executes (`read PLACE share N`, declared in the plan as
the place-free `read;`). The refusal's stated reason dissolved rather than
being overridden: the lender on this route is always the **creator** — a
spawned worker is a leaf function and lends nothing — and creator-scope bytes
are outside the epoch's allocation (`REGION-JOINS-EPOCH-AXIS`), so the loan
has no epoch residency to represent. The inclusion of the old language is
injective and misses the new constructor
(`the_loan_free_capture_language_is_unchanged`, `a_read_capture_is_new`), so
every wave-15 agreement judgment is undisturbed.

**The region body.** Wave 29's `OwnerStmt` has four constructors; `LoanStmt`
below has five, the fifth being the **adoption** `move NAME;` — the region
takes a live entry owner's cleanup duty into the pack, which from the pack's
side is an acquisition (`an_adoption_is_an_acquisition_of_the_pack`, the
design decision as one definitional equation). Every wave-29 theorem applies
to the embedded body, and the adoption-free sublanguage is unchanged
(`the_adoption_free_sublanguage_is_unchanged`).

**The loan's expiry is `QCore.ReadLoan`'s own machinery, consumed unchanged.**
While the share is out, release is refused for everyone — the owner included
(`an_outstanding_loan_cannot_be_released`); the join is the only exit from a
loan, and it restores exclusive authority to the owner alone. The composed
CT25 statement (`the_cleanup_destroys_the_lent_owner_after_joins_and_expiry`)
is those facts side by side with the pack's discharge: while lent, the
destruction refuses; after the join, it is admitted; and the discharge of the
pack that adopted the owner destroys exactly it and abandons exactly the
remainder.

What is **not** proved here, stated: that the Rust reader extracts the
adoption faithfully from source tokens and the split's textual placement
lands where the plan says (both tested Rust boundaries, the `QuantRegionSource`
precedent); that the erased `drop NAME;` is the `release` modeled here
(correspondence by inspection, as everywhere in this family); and that the
after-joins ordering holds of a given program — that is the checked task IR's
span rule, refused at `Q600`, wave 28's own machinery. -/

namespace QCore.RegionJoinsLoan

open QCore.ErrorRegion
open QCore.QuantRegionSource
open QCore.RegionJoinsDuties

/-! ## The capture language -/

/-- A composed capture after this card: a scalar copy, an owned move, or a
counted read share of a creator-owned resource — exactly the three clauses
the Rust plan reader admits (`copy;`, `move;`, `read;`). -/
inductive LoanCapture where
  | copy
  | move
  | read
  deriving Repr, DecidableEq

/-- The wave-15 capture language, included into this card's. -/
def liftCapture : QCore.EpochSourceLowering.Capture → LoanCapture
  | .copy => .copy
  | .move => .move

/-- The admitted captures as one list, so a widening must change a theorem
rather than slip past one. Three constructors where wave 15 had two — the
supersession of `T08-COMPOSE-LOAN`, stated as a shape. -/
theorem admitted_captures_are_exactly_these (c : LoanCapture) :
    c = .copy ∨ c = .move ∨ c = .read := by
  cases c
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr rfl)

/-- **The supersession is conservative**: the inclusion of the loan-free
capture language is injective, so no two wave-15 captures are conflated… -/
theorem the_loan_free_capture_language_is_unchanged
    (a b : QCore.EpochSourceLowering.Capture)
    (h : liftCapture a = liftCapture b) : a = b := by
  cases a <;> cases b <;> first | rfl | simp [liftCapture] at h

/-- …and the read capture is genuinely new: nothing of the old language maps
to it, so every wave-15 agreement judgment is undisturbed. -/
theorem a_read_capture_is_new (c : QCore.EpochSourceLowering.Capture) :
    liftCapture c ≠ .read := by
  cases c <;> simp [liftCapture]

/-! ## The region body -/

/-- A statement of the composed region body after this card: wave 29's four
shapes plus the **adoption** — `move NAME;`, the region taking a live entry
owner's cleanup duty into the pack. Exactly the five shapes the Rust reader
admits. -/
inductive LoanStmt where
  | adopt (resource : ResourceId)
  | acquire (resource : ResourceId)
  | drop (resource : ResourceId)
  | credit (amount : Nat)
  | effort (amount : Nat)
  deriving Repr, DecidableEq

/-- Each shape as wave 29's body language sees it. **The adoption is an
acquisition of the pack**: the duty it seals is the same `Destroy`, the only
difference living in the erasure (the adopted owner's acquisition already
stands in the entry's own text, so nothing is kept for it). -/
def embedOwner : LoanStmt → OwnerStmt
  | .adopt r => .acquire r
  | .acquire r => .acquire r
  | .drop r => .drop r
  | .credit n => .credit n
  | .effort n => .effort n

/-- The composed shape in the sequential route's body language, through
wave 29's own embedding — so every theorem of `QuantRegionSource` and
`RegionJoinsDuties` applies to the embedded body. -/
def embed (s : LoanStmt) : BodyStmt :=
  QCore.RegionJoinsDuties.embed (embedOwner s)

theorem map_embed (body : List LoanStmt) :
    body.map embed =
      (body.map embedOwner).map QCore.RegionJoinsDuties.embed := by
  induction body with
  | nil => rfl
  | cons s rest ih => simp [embed, ih]

/-- The admitted shapes as one list — five constructors where wave 29 had
four. -/
theorem admitted_region_shapes_are_exactly_these (s : LoanStmt) :
    (∃ r, s = .adopt r) ∨ (∃ r, s = .acquire r) ∨ (∃ r, s = .drop r) ∨
      (∃ n, s = .credit n) ∨ (∃ n, s = .effort n) := by
  cases s with
  | adopt r => exact Or.inl ⟨r, rfl⟩
  | acquire r => exact Or.inr (Or.inl ⟨r, rfl⟩)
  | drop r => exact Or.inr (Or.inr (Or.inl ⟨r, rfl⟩))
  | credit n => exact Or.inr (Or.inr (Or.inr (Or.inl ⟨n, rfl⟩)))
  | effort n => exact Or.inr (Or.inr (Or.inr (Or.inr ⟨n, rfl⟩)))

/-- The design decision as one definitional equation. -/
theorem an_adoption_is_an_acquisition_of_the_pack (r : ResourceId) :
    embedOwner (.adopt r) = .acquire r := rfl

/-- The wave-29 adoption-free sublanguage, included into this card's body. -/
def liftOwner : OwnerStmt → LoanStmt
  | .acquire r => .acquire r
  | .drop r => .drop r
  | .credit n => .credit n
  | .effort n => .effort n

/-- **The supersession is conservative** on the adoption-free sublanguage:
the inclusion composed with this card's embedding is wave 29's own, so every
wave-29 theorem — the duty fold, the destruction order, the CT25 duty half —
still holds of exactly the bodies it held of. -/
theorem the_adoption_free_sublanguage_is_unchanged (s : OwnerStmt) :
    embedOwner (liftOwner s) = s := by
  cases s <;> rfl

/-! ## The pack, with an adopted duty -/

/-- The pack a composed region seals after the joins, its body now admitting
adoptions beside acquisitions. -/
def loanPack (region : RegionName) (error : ErrorName) (baseline : Ledger)
    (body : List LoanStmt) (held : Nat) : CleanupPack :=
  ownerPack region error baseline (body.map embedOwner) held

/-- The sealed pack's actions are exactly the body's unreleased owners —
adopted and acquired alike — in order of appearance: the duty fold the Rust
driver re-derives before publication. -/
theorem the_pack_after_joins_carries_the_adopted_duties (region : RegionName)
    (error : ErrorName) (baseline : Ledger) (body : List LoanStmt)
    (held : Nat) :
    (loanPack region error baseline body held).actions =
      duties (body.map embed) := by
  rw [map_embed]
  rfl

/-- Every duty of the pack is one of the body's own owners — adopted or
acquired — and nothing a child held: wave 29's fold, through the embedding. -/
theorem duties_are_the_parents_own (body : List LoanStmt) (r : ResourceId)
    (h : r ∈ duties (body.map embed)) :
    r ∈ emittedAcquisitions (body.map embed) := by
  rw [map_embed] at h ⊢
  exact QCore.RegionJoinsDuties.duties_are_the_bodys_own (body.map embedOwner) r h

/-- An adoption moves no credit: the reader's quantity fold is untouched, so
the wave-28 cross-check reads the same figure through it. -/
theorem an_adoption_moves_no_credit (held : Nat) (r : ResourceId)
    (rest : List LoanStmt) :
    remainderFrom held ((LoanStmt.adopt r :: rest).map embed) =
      remainderFrom held (rest.map embed) := rfl

/-! ## The CT25 property, both halves -/

/-- **While the share is live, the destruction refuses** — for everyone, the
lender included: `QCore.ReadLoan`'s own rule, consumed unchanged. This is the
`live_share_drop.qcore` control (`LoanOutstanding` at the task route's own
`Q600`). -/
theorem a_live_share_refuses_the_destruction (custody : QCore.ReadLoan.Custody)
    {resource owner borrower : Nat}
    (lent : custody resource = some (.loaned owner borrower)) (task : Nat) :
    QCore.ReadLoan.release custody resource task = none :=
  QCore.ReadLoan.an_outstanding_loan_cannot_be_released custody lent task

/-- **The loan expires at the join, and only then is the destruction
admitted**: the join restores exclusive authority to the owner alone, and
release demands exactly that authority. -/
theorem the_loan_expires_at_the_join {custody closed : QCore.ReadLoan.Custody}
    {resource owner borrower : Nat}
    (joined : QCore.ReadLoan.closeLoan custody resource owner borrower = some closed) :
    (QCore.ReadLoan.release closed resource owner).isSome := by
  have exclusive :=
    (QCore.ReadLoan.a_successful_join_restores_exclusive_authority_to_the_owner
        joined owner).mpr rfl
  simp [QCore.ReadLoan.release, exclusive]

/-- **The CT25 scenario, whole**: while the worker's share is out the owner
cannot be destroyed; the join — and nothing else — closes the loan and
readmits the destruction; and the discharge of the pack that adopted the
owner destroys exactly it, restores the baseline, abandons exactly the
parent-local remainder and consumes the capability. The after-joins ordering
premise (`Q600`, wave 28's span rule) is what places the discharge on the
right-hand side of the expiry. -/
theorem the_cleanup_destroys_the_lent_owner_after_joins_and_expiry
    (region : RegionName) (error : ErrorName) (baseline : Ledger)
    (held : Nat) (r : ResourceId)
    {custody closed : QCore.ReadLoan.Custody} {resource owner borrower : Nat}
    (lent : custody resource = some (.loaned owner borrower))
    (joined : QCore.ReadLoan.closeLoan custody resource owner borrower = some closed) :
    QCore.ReadLoan.release custody resource owner = none ∧
      (QCore.ReadLoan.release closed resource owner).isSome ∧
      discharge (.opened (loanPack region error baseline [.adopt r] held)) =
        some
          { ledger := baseline
            cleanupTrace := [r]
            abandonedCredit := held
            capability := .consumed } :=
  ⟨a_live_share_refuses_the_destruction custody lent owner,
    the_loan_expires_at_the_join joined, rfl⟩

/-- The whole admitted lending lifetime beside the destruction: a reserve,
commit, close round trip is authority-neutral, so the owner ends it with
exactly the exclusive authority it began with — the destruction the discharge
performs after the join is the one the owner could have performed before the
lending began. -/
theorem the_round_trip_returns_the_destruction_to_the_owner
    {custody reserved lent closed : QCore.ReadLoan.Custody}
    {resource creator borrower reservation : Nat}
    (parked : QCore.ReadLoan.reserve custody resource creator reservation = some reserved)
    (created : QCore.ReadLoan.commit reserved resource creator borrower reservation = some lent)
    (joined : QCore.ReadLoan.closeLoan lent resource creator borrower = some closed) :
    (QCore.ReadLoan.release closed resource creator).isSome := by
  rw [QCore.ReadLoan.the_admitted_lending_round_trip_is_authority_neutral
    parked created joined]
  have owned := QCore.ReadLoan.reserve_demands_exclusive_authority parked
  simp [QCore.ReadLoan.release, QCore.ReadLoan.canUseExclusively, owned]

/-! ## The flagship's own figures -/

/-- Non-vacuity on `CT25_LOAN/case.qcore`'s shape: one adopted owner, credit
5, effort 2 — one duty, remainder 3, and the discharge's trace destroys
exactly the owner. -/
theorem the_flagship_adopts_one_owner_and_abandons_three :
    duties ([LoanStmt.adopt 1, .credit 5, .effort 2].map embed) = [1] ∧
      remainder ([LoanStmt.adopt 1, .credit 5, .effort 2].map embed) =
        some 3 := by
  constructor <;> rfl

/-- `adopted_then_dropped.qcore`'s shape: an adopted owner released in the
body erases its duty, so the pack is duty-free again. -/
theorem an_adopted_owner_released_in_the_body_erases_its_duty :
    duties ([LoanStmt.adopt 1, .drop 1, .credit 5, .effort 2].map embed) =
      [] := rfl

/-- Two surviving duties — one adopted, one acquired — are destroyed
last-first, exactly as wave 29's two acquisitions are (`adopt_and_acquire`'s
shape, whose erased program the task route refuses for its own custody
reason; the pack's order is stated here for the shape itself). -/
theorem an_adopted_and_an_acquired_duty_are_destroyed_last_first :
    (duties ([LoanStmt.adopt 1, .acquire 2].map embed)).reverse = [2, 1] := rfl

end QCore.RegionJoinsLoan
