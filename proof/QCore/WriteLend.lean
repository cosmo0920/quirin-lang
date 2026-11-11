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

import QCore.SlicePermissions

/-! The write lend: a loan that carries **write authority** over a range of an
owned buffer, so two parties mutate two disjoint ranges independently (card
`SLICE-PERMISSIONS-WRITE-LEND`).

Card `CT07-SLICE-PERMISSIONS` proved the read half — the range-wise exclusion
under which the owner *keeps* the write authority — and recorded the write half
as unreachable without "a custody shape in which the owner's authority over the
lent range is genuinely transferred", the footprint fraction `STD-BUFFER-SLICE`
predicted a borrow node would need. This module is that shape's model, mirroring
the extended cycle-3 ownership IR (`ownership.rs`'s `LendRange`, `WriteRange`,
`ReadRange` and the `MutableRange` borrow kind).

Five things are proved, and they are the five the Rust implementation rests on.

**The exclusivity rule is the disjointness question.** A second lend is admitted
exactly when its range is disjoint from every live loan's, and disjointness is
`QCore.SlicePermissions.overlaps` — the decision the read card already proved
equal to the set-theoretic question, imported rather than restated.

**The owner's authority is transferred range-wise.** An owner access is refused
exactly when its byte footprint shares a point with a live lend, so inside the
lent range the owner has neither read nor write and outside every lend it keeps
both. An empty footprint (an extent query) conflicts with nothing.

**Rebasing is sound.** A loan access at index `i` is the owner's byte
`low + i`: it stays inside the lend, it is a byte the buffer has, and two
disjoint lends' rebased accesses can never collide — the partition CT07's
"independently" means.

**Every byte has at most one writer.** With pairwise-disjoint lends, a byte is
either exactly one loan's or the owner's, never two parties'.

**The unlent bytes survive the window.** Over a window whose stores all land
inside the lend's range, every byte outside it is exactly what it was — the
mirror image of the read card's soundness statement, and the reason the owner's
reads after the window are reads of the bytes it kept.

What is **not** proved here, carried over from the family's boundary: that the
Rust lowering extracts these events faithfully from source tokens (a tested Rust
boundary), the lexical window computation (`through` is a tested last-use scan),
and the composed string route's total refusal of a lend beside a read loan —
that refusal is a one-arm Rust boundary (`SLICE-PERMISSIONS-COMPOSED-LEND`),
recorded in the inventory at the end of this file. -/

namespace QCore.WriteLend

open QCore.SlicePermissions
open QCore.StringConcat

/-! ## 1. Loans, and the exclusivity rule -/

/-- What a live loan of one resource holds — the ownership IR's three borrow
kinds. A whole-resource borrow of either kind holds every byte; a range lend
holds exactly `[low, high)`. -/
inductive LoanKind where
  | shared
  | mutable
  | mutableRange (range : Range)
  deriving DecidableEq, Repr

structure Loan where
  resource : Nat
  kind : LoanKind
  deriving DecidableEq, Repr

/-- Whether one live loan blocks a **candidate lend** of range `r` of the same
resource. A whole borrow blocks any non-empty lend; a range lend blocks exactly
an overlapping one. -/
def blocksLend (r : Range) : LoanKind → Bool
  | .shared => decide (r.low < r.high)
  | .mutable => decide (r.low < r.high)
  | .mutableRange held => overlaps held r

/-- **The lend admission**: a new lend of `r` is admitted exactly when no live
loan of the resource blocks it. -/
def admitsLend (loans : List Loan) (resource : Nat) (r : Range) : Bool :=
  loans.all (fun loan => !(loan.resource == resource && blocksLend r loan.kind))

/-- **The exclusivity rule is the disjointness question.** Against one live
range lend, a second lend is admitted exactly when the two ranges are disjoint —
and `overlaps` is the set-theoretic question itself
(`QCore.SlicePermissions.overlaps_iff_a_shared_byte`), so "no byte is lent
twice" is what this decides rather than an approximation of it. -/
theorem a_second_lend_is_admitted_iff_disjoint (resource : Nat) (held candidate : Range) :
    admitsLend [{ resource := resource, kind := .mutableRange held }] resource candidate
      = !overlaps held candidate := by
  simp [admitsLend, blocksLend]

/-- Two lends that were admitted together share no byte. -/
theorem two_admitted_lends_share_no_byte (resource : Nat) (held candidate : Range)
    (admitted :
      admitsLend [{ resource := resource, kind := .mutableRange held }] resource candidate
        = true)
    (i : Nat) (inside : held.contains i = true) : candidate.contains i = false := by
  rw [a_second_lend_is_admitted_iff_disjoint] at admitted
  exact disjoint_ranges_share_no_byte held candidate (by
    cases overlapped : overlaps held candidate with
    | false => rfl
    | true => rw [overlapped] at admitted; exact Bool.noConfusion admitted) i inside

/-- An **empty** lend is admitted beside any loan — a consequence of the
interval decision, not a case beside it, and the reason `lend(b, 4, 4)`
conflicts with nothing. -/
theorem an_empty_lend_is_admitted_beside_any_loan (loans : List Loan) (resource : Nat)
    (r : Range) (empty : r.high ≤ r.low) : admitsLend loans resource r = true := by
  induction loans with
  | nil => rfl
  | cons loan rest ih =>
      simp only [admitsLend, List.all_cons, Bool.and_eq_true] at ih ⊢
      refine ⟨?_, ih⟩
      cases kind : loan.kind with
      | shared | mutable =>
          simp [blocksLend, Nat.not_lt.mpr empty]
      | mutableRange held =>
          simp [blocksLend, an_empty_range_overlaps_nothing held r empty]

/-- A whole-resource borrow of either kind blocks every non-empty lend: it
holds every byte, so any byte the lend would take is one it already holds. -/
theorem a_whole_borrow_blocks_every_nonempty_lend (resource : Nat) (r : Range)
    (nonempty : r.low < r.high) (kind : LoanKind) (whole : kind ≠ LoanKind.mutableRange r)
    (is_whole : kind = .shared ∨ kind = .mutable) :
    admitsLend [{ resource := resource, kind := kind }] resource r = false := by
  rcases is_whole with shared | mutable
  · subst shared; simp [admitsLend, blocksLend, nonempty]
  · subst mutable; simp [admitsLend, blocksLend, nonempty]


/-! ## 2. The owner's authority, transferred range-wise -/

/-- Whether one live loan blocks an **owner access** of footprint `a`. A range
lend blocks exactly an overlapping footprint; a whole mutable borrow blocks any
non-empty one; a shared borrow blocks a write but no read, which is why the two
directions below take a flag. -/
def blocksOwner (write : Bool) (a : Range) : LoanKind → Bool
  | .shared => write && decide (a.low < a.high)
  | .mutable => decide (a.low < a.high)
  | .mutableRange held => overlaps a held

/-- The owner-access decision: refused exactly when some live loan blocks it. -/
def ownerBlocked (loans : List Loan) (resource : Nat) (write : Bool) (a : Range) : Bool :=
  loans.any (fun loan => loan.resource == resource && blocksOwner write a loan.kind)

/-- **An owner write of one byte is refused exactly where a lend holds it.**
This is the transfer: inside `[low, high)` the authority is the loan's, outside
it the owner's — stated as an equation, so both directions are one theorem. -/
theorem the_owner_is_excluded_exactly_where_the_loan_holds
    (resource : Nat) (held : Range) (write : Bool) (i : Nat) :
    ownerBlocked [{ resource := resource, kind := .mutableRange held }] resource write
        (writeRange i)
      = held.contains i := by
  simp [ownerBlocked, blocksOwner, overlaps_symm,
    a_write_conflicts_exactly_when_its_byte_is_lent]

/-- With **two disjoint lends** live, an owner byte write is refused exactly
where either loan holds it — and a byte both hold does not exist, so the three
authorities partition the buffer: `left`'s bytes, `right`'s bytes, and the
owner's remainder. This is CT07's "independently", as an equation. -/
theorem every_byte_has_exactly_one_write_authority
    (resource : Nat) (left right : Range) (i : Nat)
    (disjoint : overlaps left right = false) :
    ownerBlocked
        [{ resource := resource, kind := .mutableRange left },
         { resource := resource, kind := .mutableRange right }]
        resource true (writeRange i)
      = (left.contains i || right.contains i)
    ∧ (left.contains i && right.contains i) = false := by
  constructor
  · simp [ownerBlocked, blocksOwner, overlaps_symm,
      a_write_conflicts_exactly_when_its_byte_is_lent]
  · cases inside : left.contains i with
    | false => rfl
    | true =>
        simp [disjoint_ranges_share_no_byte left right disjoint i inside]

/-- **An empty owner footprint conflicts with nothing** — an extent query reads
no byte, which is why `extent(b)` is admitted beside any loan while the
binding's liveness is still checked. -/
theorem an_extent_query_is_admitted_beside_any_loan
    (loans : List Loan) (resource : Nat) (write : Bool) (a : Range) (empty : a.high ≤ a.low) :
    ownerBlocked loans resource write a = false := by
  induction loans with
  | nil => rfl
  | cons loan rest ih =>
      simp only [ownerBlocked, List.any_cons, Bool.or_eq_false_iff] at ih ⊢
      refine ⟨?_, ih⟩
      cases kind : loan.kind with
      | shared | mutable =>
          simp [blocksOwner, Nat.not_lt.mpr empty]
      | mutableRange held =>
          have disjoint : overlaps a held = false := by
            rw [overlaps_symm]
            exact an_empty_range_overlaps_nothing held a empty
          simp [blocksOwner, disjoint]

/-- A shared loan blocks no read: the read direction's conflict set is the
loans carrying write authority, exactly as the IR's `has_loan_overlapping`
restricts it. -/
theorem a_shared_loan_blocks_no_read (resource : Nat) (a : Range) :
    ownerBlocked [{ resource := resource, kind := .shared }] resource false a = false := by
  simp [ownerBlocked, blocksOwner]

/-! ## 3. Rebasing into the owner's coordinates -/

/-- A loan access at the loan's own index `i` is the owner's byte `low + i` —
the one arithmetic the certificate's `LendRecord` publishes and the emitted
`owner + low` displacement performs. -/
def rebase (r : Range) (i : Nat) : Nat := r.low + i

/-- A rebased in-bounds access stays inside its lend — the containment the IR's
`WriteRange`-through-a-loan rule checks rather than trusts. -/
theorem a_rebased_access_stays_inside_its_lend (r : Range) (i : Nat)
    (inside : i < r.high - r.low) : r.contains (rebase r i) = true := by
  have low_le : r.low ≤ r.low + i := Nat.le_add_right _ _
  have lt_high : r.low + i < r.high := by omega
  simp [Range.contains, rebase, low_le, lt_high]

/-- A rebased access through an admitted lend is a byte the buffer has: the
lend's bound and the loan's own bound compose. -/
theorem a_rebased_access_is_a_byte_the_buffer_has (extent : Nat) (r : Range) (i : Nat)
    (admitted : admitsRange extent r = true) (inside : i < r.high - r.low) :
    rebase r i < extent :=
  an_admitted_range_lends_only_bytes_that_exist extent r admitted (rebase r i)
    (a_rebased_access_stays_inside_its_lend r i inside)

/-- **Disjoint lends rebase to distinct bytes.** Two parties writing through
their own loans can never write one byte, whatever in-bounds indices each
names — the set-theoretic content of "mutated independently". -/
theorem disjoint_lends_rebase_to_distinct_bytes (a b : Range) (i j : Nat)
    (disjoint : overlaps a b = false)
    (i_inside : i < a.high - a.low) (j_inside : j < b.high - b.low) :
    rebase a i ≠ rebase b j := by
  intro same
  have a_holds := a_rebased_access_stays_inside_its_lend a i i_inside
  have b_holds := a_rebased_access_stays_inside_its_lend b j j_inside
  rw [same] at a_holds
  have := disjoint_ranges_share_no_byte a b disjoint (rebase b j) a_holds
  rw [this] at b_holds
  exact Bool.noConfusion b_holds

/-! ## 4. The loan blocks the storage's end, and moves no quantity -/

/-- Whether any live loan holds the resource — the IR's `has_any_borrow`, under
which a drop or a move of the owner refuses. A range lend is a borrow: however
narrow its range, the storage it addresses is the whole allocation, so the
release arm is **not** range-wise — the read card's asymmetry, inherited. -/
def hasLoan (loans : List Loan) (resource : Nat) : Bool :=
  loans.any (fun loan => loan.resource == resource)

/-- Opening one write lend: a loan is appended, and **nothing else is an
argument** — in particular no ledger, which is the model-level statement that a
lend moves no quantity. The transfer is of authority, never of footprint, so
the live vector (a function of the ledger alone) and the published peak-live
measurement cannot move at a lend. -/
def openLend (loans : List Loan) (resource : Nat) (r : Range) : List Loan :=
  { resource := resource, kind := .mutableRange r } :: loans

/-- A lend blocks the drop of its owner however narrow — an empty range
included, because what ends at a release is the allocation and not a byte.
This is `QCore.Ownership.Loans.mutable_blocks_drop`'s shape for the ranged
kind, and the read card's release asymmetry inherited by the write one. -/
theorem a_lend_blocks_the_owners_drop
    (loans : List Loan) (resource : Nat) (r : Range) :
    hasLoan (openLend loans resource r) resource = true := by
  simp [hasLoan, openLend]

/-! ## 5. The unlent bytes survive the window

The mirror image of
`QCore.SlicePermissions.an_excluded_range_preserves_its_bytes`: there, writes
outside a read loan's range were shown to preserve the **lent** bytes; here,
writes **inside** a write lend's range are shown to preserve every byte outside
it. Together they are the two halves of one partition — each party's bytes are
untouchable by the other's admitted statements. -/

/-- Whether a step stays within the lend's authority: a store into `buffer`
must land inside `r`, and a release ends what it does not own. Every other
statement is free. -/
def confinedTo (buffer : String) (r : Range) : QCore.StringConcat.Step → Bool
  | .store name index _ => !(name == buffer) || r.contains index
  | .release name => !(name == buffer)
  | .other => true

/-- One confined step preserves every byte outside the lend. -/
theorem a_confined_step_preserves_every_unlent_byte
    (contents : Contents) (buffer : String) (r : Range) (s : QCore.StringConcat.Step)
    (j : Nat) (unlent : r.contains j = false)
    (confined : confinedTo buffer r s = true) :
    byteAt (QCore.StringConcat.step contents s) buffer j = byteAt contents buffer j := by
  cases s with
  | store target index byte =>
      by_cases same : target = buffer
      · subst same
        simp only [confinedTo, beq_self_eq_true, Bool.not_true, Bool.false_or] at confined
        have different : index ≠ j := by
          intro equal
          rw [equal, unlent] at confined
          exact Bool.noConfusion confined
        cases held : contents target with
        | none => simp [byteAt, QCore.StringConcat.step, held]
        | some payload =>
            simp [byteAt, QCore.StringConcat.step, held, List.getElem?_set_ne different]
      · simp [byteAt, QCore.StringConcat.step, Ne.symm same]
  | release name =>
      simp only [confinedTo, Bool.not_eq_true', beq_eq_false_iff_ne, ne_eq] at confined
      simp [byteAt, QCore.StringConcat.step, Ne.symm confined]
  | other => rfl

/-- **The window, byte by byte.** Over a window in which every store into the
buffer lands inside the lent range and nothing releases it, every byte outside
the range is exactly what it was — so what the owner reads back after the
window, in the bytes it kept, is what it wrote before the lend, however the
loan's party mutated its own range in between. -/
theorem a_confined_window_preserves_every_unlent_byte
    (contents : Contents) (buffer : String) (r : Range) (j : Nat)
    (unlent : r.contains j = false) :
    ∀ window : List QCore.StringConcat.Step,
      (∀ s ∈ window, confinedTo buffer r s = true) →
        byteAt (run contents window) buffer j = byteAt contents buffer j := by
  intro window
  induction window generalizing contents with
  | nil => intro _; rfl
  | cons head tail ih =>
      intro confined
      have head_confined : confinedTo buffer r head = true := confined head (by simp)
      have tail_confined : ∀ s ∈ tail, confinedTo buffer r s = true := by
        intro s held
        exact confined s (by simp [held])
      show byteAt (run (QCore.StringConcat.step contents head) tail) buffer j
          = byteAt contents buffer j
      rw [ih (QCore.StringConcat.step contents head) tail_confined]
      exact a_confined_step_preserves_every_unlent_byte contents buffer r head j unlent
        head_confined

/-- A rebased loan store is confined by construction: `part[i] = v` with `i`
inside the loan is a store the window admits, which ties §3's arithmetic to
§5's window without a second definition of either. -/
theorem a_rebased_loan_store_is_confined (buffer : String) (r : Range) (i byte : Nat)
    (inside : i < r.high - r.low) :
    confinedTo buffer r (.store buffer (rebase r i) byte) = true := by
  simp [confinedTo, a_rebased_access_stays_inside_its_lend r i inside]

/-! ## 6. What this card added

One operation, written as one list so that a widening changes a theorem rather
than slipping past it. The read module's own inventory
(`QCore.SlicePermissions.addedOperations`) still says that card added exactly
the range borrow; this card adds exactly the write lend, and the composed
string route's total refusal of a lend beside a read loan is a one-arm Rust
boundary rather than a theorem (`SLICE-PERMISSIONS-COMPOSED-LEND`). -/

inductive AddedOperation where
  | writeLend
  deriving DecidableEq, Repr

def addedOperations : List AddedOperation := [.writeLend]

theorem the_added_operation_is_exactly_one : addedOperations.length = 1 := rfl

end QCore.WriteLend
