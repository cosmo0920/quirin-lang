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

import QCore.StringConcat

/-! Sub-extent lending: a borrow that names a **range** of an owned buffer, and
the range-wise exclusion rule it makes possible (card `CT07-SLICE-PERMISSIONS`).

Card `STD-BUFFER` recorded the gap as `STD-BUFFER-SLICE` — *"a borrowed
sub-range"* — and the wave-26 CT lane named the same gap as what keeps CT07 and
CT08 unmapped: no source form lends part of a buffer to anyone, so two parties
holding disjoint permission over two ranges is not expressible.

Four things are proved here, and they are the four the Rust implementation rests
on.

**The disjointness decision is exact.** `overlaps` is
`max low < min high`, and `overlaps_iff_a_shared_byte` proves it is *the*
set-theoretic question rather than an approximation of it. Every bound on this
route is a check-time constant, so "provably disjoint" and "disjoint" are one
predicate and there is no third answer to fall closed to.

**The exclusion is range-wise for a write and whole for a release.** A write
conflicts with a loan exactly when its byte is inside the loan
(`a_write_conflicts_exactly_when_its_byte_is_lent`), so a write outside every
live loan is admitted — which is the card. A release conflicts with *every* live
loan however narrow, because it ends the storage all of them address.

**The shipped whole-buffer rule is this rule's degenerate case.**
`the_whole_extent_decision_is_the_shipped_one` is an equation between this
module's `excludedStore` and `QCore.StringConcat.excluded` — the definition the
route has been running since card `STRING-CONCAT` — for a loan of the whole
extent. Nothing was replaced: the old rule is the new one at `[0, extent)`.

**The exclusion is soundness, byte by byte.** Over a window in which no
statement stores inside the lent range and none releases the buffer, every byte
the loan's view addresses is the byte that was there when the loan's UTF-8
decision was made. That is the range-wise generalisation of
`QCore.StringConcat.an_excluded_window_preserves_the_buffers_bytes`, and it is
strictly stronger in the useful direction: it survives stores the old statement's
hypothesis forbade.

What is **not** proved here, carried over from the family's boundary: that the
Rust lowering extracts these events faithfully from source tokens (a tested Rust
boundary), and anything about a *write* lend — this card lends read permission
over a range and keeps the write authority with the owner, which is why the
window's exactness argument (one flat statement sequence, no loop) is untouched
and a `for` beside a loan is still refused. -/

namespace QCore.SlicePermissions

open QCore.StdString
open QCore.StringConcat

/-! ## 1. A range, and the decision -/

/-- A half-open byte range `[low, high)` of one buffer. -/
structure Range where
  low : Nat
  high : Nat
  deriving DecidableEq, Repr

/-- Whether a range covers one byte. -/
def Range.contains (r : Range) (i : Nat) : Bool :=
  decide (r.low ≤ i) && decide (i < r.high)

/-- **The disjointness decision.** Two half-open ranges share a byte exactly when
`max low < min high`. -/
def overlaps (a b : Range) : Bool :=
  decide (max a.low b.low < min a.high b.high)

/-- **The decision is the question, not an approximation of it.** This is what
makes "anything not provably disjoint refuses" vacuous on this route rather than
a concession: a pair of ranges is disjoint or it is not, and the checker decides
which. -/
theorem overlaps_iff_a_shared_byte (a b : Range) :
    overlaps a b = true ↔ ∃ i, a.contains i = true ∧ b.contains i = true := by
  simp only [overlaps, Range.contains, Bool.and_eq_true, decide_eq_true_eq]
  constructor
  · intro shared
    exact ⟨max a.low b.low, ⟨by omega, by omega⟩, ⟨by omega, by omega⟩⟩
  · rintro ⟨i, ⟨_, _⟩, ⟨_, _⟩⟩
    omega

/-- The decision is symmetric, which is why one call site answers both
directions. -/
theorem overlaps_symm (a b : Range) : overlaps a b = overlaps b a := by
  simp only [overlaps, decide_eq_decide]
  omega

/-- **An empty range shares no byte with anything**, itself included — a
consequence of the decision rather than a case beside it, and the reason
`borrow_utf8(b, 2, 2)` excludes no write. -/
theorem an_empty_range_overlaps_nothing (a : Range) (b : Range) (empty : b.high ≤ b.low) :
    overlaps a b = false := by
  simp only [overlaps, decide_eq_false_iff_not, Nat.not_lt]
  omega

/-- Adjacent ranges are disjoint: `[0, 2)` and `[2, 4)` share no byte. The
half-open convention is what makes "the two halves of a buffer" expressible
without an off-by-one. -/
theorem adjacent_ranges_are_disjoint (low mid high : Nat) :
    overlaps ⟨low, mid⟩ ⟨mid, high⟩ = false := by
  simp only [overlaps, decide_eq_false_iff_not, Nat.not_lt]
  omega

/-- Disjointness in the form the soundness argument uses: no byte is in both. -/
theorem disjoint_ranges_share_no_byte (a b : Range) (disjoint : overlaps a b = false)
    (i : Nat) (inside : a.contains i = true) : b.contains i = false := by
  cases held : b.contains i with
  | false => rfl
  | true =>
      have shared : overlaps a b = true := (overlaps_iff_a_shared_byte a b).mpr ⟨i, inside, held⟩
      rw [disjoint] at shared
      exact Bool.noConfusion shared

/-! ## 2. A write is a one-byte range

Stating a write as a range is what makes "this write is inside that loan" the
same question as "these two loans overlap", decided by one function. -/

/-- The footprint of `b[i] = v`. -/
def writeRange (i : Nat) : Range := ⟨i, i + 1⟩

/-- **A write conflicts with a loan exactly when its byte is lent.** -/
theorem a_write_conflicts_exactly_when_its_byte_is_lent (r : Range) (i : Nat) :
    overlaps r (writeRange i) = r.contains i := by
  -- The `show` is load-bearing rather than cosmetic: unfolding `writeRange`
  -- *inside* a `simp` set rewrites the proposition under `decide` without
  -- rewriting `decide`'s own instance argument, and the goal stops being
  -- type-correct. Restating it definitionally builds both together.
  show decide (max r.low i < min r.high (i + 1))
      = (decide (r.low ≤ i) && decide (i < r.high))
  rw [Bool.eq_iff_iff]
  simp only [Bool.and_eq_true, decide_eq_true_eq]
  omega

/-! ## 3. Loans, and the exclusion rule -/

/-- One loan whose window has not closed: which buffer, which view, the interval
of statement indices it covers, and **which bytes**. -/
structure Loan where
  buffer : String
  view : String
  from_ : Nat
  through : Nat
  range : Range
  deriving DecidableEq, Repr

/-- Whether a loan's window covers statement `i`. -/
def liveAt (loan : Loan) (i : Nat) : Bool :=
  decide (loan.from_ ≤ i) && decide (i ≤ loan.through)

/-- **The store arm: range-wise.** A write of byte `byte` at statement `i`
conflicts with a live loan of the same buffer whose range covers that byte. -/
def excludedStore (loans : List Loan) (buffer : String) (i byte : Nat) : Bool :=
  loans.any (fun loan =>
    loan.buffer == buffer && liveAt loan i && overlaps loan.range (writeRange byte))

/-- **The release arm: whole.** A release ends the storage every loan addresses,
so it conflicts with any live loan of the buffer however narrow. Making this arm
range-wise would let a program free the bytes a live view reads, which is the one
thing the window exists to prevent. -/
def excludedRelease (loans : List Loan) (buffer : String) (i : Nat) : Bool :=
  loans.any (fun loan => loan.buffer == buffer && liveAt loan i)

/-- A write inside a live loan is refused. -/
theorem a_write_inside_a_live_loan_is_refused
    (loan : Loan) (i byte : Nat)
    (after : loan.from_ ≤ i) (before : i ≤ loan.through)
    (lent : loan.range.contains byte = true) :
    excludedStore [loan] loan.buffer i byte = true := by
  simp [excludedStore, liveAt, after, before, a_write_conflicts_exactly_when_its_byte_is_lent,
    lent]

/-- **A write outside the loan's range is admitted.** This is the card: before
it, a buffer under a live view was frozen whole, and two disjoint permissions
over one buffer were not expressible. -/
theorem a_write_outside_the_loan_is_admitted
    (loan : Loan) (i byte : Nat) (outside : loan.range.contains byte = false) :
    excludedStore [loan] loan.buffer i byte = false := by
  simp [excludedStore, a_write_conflicts_exactly_when_its_byte_is_lent, outside]

/-- A write after the window is admitted whatever byte it names — the interval
rule, unchanged by the range. -/
theorem a_write_after_the_window_is_admitted
    (loan : Loan) (i byte : Nat) (past : loan.through < i) :
    excludedStore [loan] loan.buffer i byte = false := by
  simp [excludedStore, liveAt, Nat.not_le.mpr past]

/-- A loan of one buffer constrains no other. -/
theorem a_loan_constrains_only_its_own_buffer
    (loan : Loan) (other : String) (i byte : Nat) (different : loan.buffer ≠ other) :
    excludedStore [loan] other i byte = false := by
  simp [excludedStore, different]

/-- **A release is refused for a live loan however narrow**, including an empty
one. The asymmetry with the store arm is deliberate and is the whole of why the
two arms are separate definitions. -/
theorem a_release_is_refused_for_any_live_loan
    (loan : Loan) (i : Nat) (after : loan.from_ ≤ i) (before : i ≤ loan.through) :
    excludedRelease [loan] loan.buffer i = true := by
  simp [excludedRelease, liveAt, after, before]

/-! ## 4. Two disjoint loans

The shape CT07 asks for: two ranges of one buffer held at once, with the
permission each carries decided rather than assumed. -/

/-- **Disjoint loans partition their writes.** A byte one loan lends is a byte
the other does not, so the write each excludes is a write the other admits. -/
theorem disjoint_loans_partition_their_writes
    (a b : Loan) (i byte : Nat)
    (same : b.buffer = a.buffer)
    (disjoint : overlaps a.range b.range = false)
    (a_after : a.from_ ≤ i) (a_before : i ≤ a.through)
    (lent : a.range.contains byte = true) :
    excludedStore [a] a.buffer i byte = true ∧ excludedStore [b] a.buffer i byte = false := by
  refine ⟨a_write_inside_a_live_loan_is_refused a i byte a_after a_before lent, ?_⟩
  rw [← same]
  exact a_write_outside_the_loan_is_admitted b i byte
    (disjoint_ranges_share_no_byte a.range b.range disjoint byte lent)

/-- And a write disjoint from **both** live loans is admitted by both, which is
the third range of the CT07 fixture: two views alive, the owner writing between
them. -/
theorem a_write_outside_both_loans_is_admitted
    (a b : Loan) (buffer : String) (i byte : Nat)
    (outside_a : a.range.contains byte = false)
    (outside_b : b.range.contains byte = false) :
    excludedStore [a, b] buffer i byte = false := by
  simp [excludedStore, a_write_conflicts_exactly_when_its_byte_is_lent, outside_a, outside_b]

/-! ## 5. The whole-extent loan is this rule's degenerate case

Not a claim beside the shipped rule but an **equation with it**: the definition
compared against is `QCore.StringConcat.excluded`, which is what the route has
been running since card `STRING-CONCAT`. -/

/-- The range a whole-extent borrow lends. -/
def wholeRange (extent : Nat) : Range := ⟨0, extent⟩

/-- A whole-extent loan lends every byte of its buffer. -/
theorem a_whole_extent_loan_lends_every_byte (extent byte : Nat) (inside : byte < extent) :
    (wholeRange extent).contains byte = true := by
  simp [wholeRange, Range.contains, inside]

/-- The borrow record the shipped rule reads, from this card's loan. -/
def Loan.toBorrow (loan : Loan) : QCore.StringConcat.Borrow :=
  { buffer := loan.buffer, view := loan.view, from_ := loan.from_, through := loan.through }

/-- **The shipped decision, reproduced.** For a loan of the whole extent and a
byte inside it, the range-wise rule and `QCore.StringConcat.excluded` — the rule
this route ran before sub-extent lending existed — return the same answer at
every statement index. Nothing was replaced. -/
theorem the_whole_extent_decision_is_the_shipped_one
    (buffer view : String) (from_ through extent i byte : Nat) (inside : byte < extent) :
    excludedStore
        [{ buffer := buffer, view := view, from_ := from_, through := through,
           range := wholeRange extent }]
        buffer i byte
      = QCore.StringConcat.excluded
          [{ buffer := buffer, view := view, from_ := from_, through := through }] buffer i := by
  simp [excludedStore, QCore.StringConcat.excluded, QCore.StringConcat.liveAt, liveAt,
    a_write_conflicts_exactly_when_its_byte_is_lent,
    a_whole_extent_loan_lends_every_byte extent byte inside]

/-- The release arm reproduces it too, and without a side condition: a release
is refused for a live loan of any range, so the whole-extent case is the general
one seen at `[0, extent)`. -/
theorem the_release_decision_is_the_shipped_one (loan : Loan) (i : Nat) :
    excludedRelease [loan] loan.buffer i
      = QCore.StringConcat.excluded [loan.toBorrow] loan.buffer i := by
  simp [excludedRelease, QCore.StringConcat.excluded, QCore.StringConcat.liveAt, liveAt,
    Loan.toBorrow]

/-! ## 6. The exclusion as soundness, byte by byte

The range-wise generalisation of
`QCore.StringConcat.an_excluded_window_preserves_the_buffers_bytes`: it survives
stores the earlier statement's hypothesis forbade, and that is exactly the
programs this card admits. -/

/-- One byte of one buffer, as the model holds it: `none` if the buffer was
released or the index is past its end. -/
def byteAt (contents : Contents) (buffer : String) (j : Nat) : Option Nat :=
  (contents buffer).bind (fun payload => payload[j]?)

/-- Whether a step changes a byte **inside `r`** of `buffer`, or ends it. A store
outside the range is not a touch, which is the one line that separates this
module from the whole-buffer statement. -/
def touchesRange (buffer : String) (r : Range) : Step → Bool
  | .store name index _ => (name == buffer) && r.contains index
  | .release name => name == buffer
  | .other => false

/-- A step that touches no lent byte leaves every lent byte exactly as it was. -/
theorem an_untouching_step_preserves_the_lent_bytes
    (contents : Contents) (buffer : String) (r : Range) (s : Step) (j : Nat)
    (lent : r.contains j = true)
    (untouched : touchesRange buffer r s = false) :
    byteAt (step contents s) buffer j = byteAt contents buffer j := by
  cases s with
  | store target index byte =>
      by_cases same : target = buffer
      · subst same
        simp only [touchesRange, beq_self_eq_true, Bool.true_and] at untouched
        have different : index ≠ j := by
          intro equal
          rw [equal, lent] at untouched
          exact Bool.noConfusion untouched
        cases held : contents target with
        | none => simp [byteAt, step, held]
        | some payload => simp [byteAt, step, held, List.getElem?_set_ne different]
      · simp [byteAt, step, Ne.symm same]
  | release name =>
      simp only [touchesRange, beq_eq_false_iff_ne, ne_eq] at untouched
      simp [byteAt, step, Ne.symm untouched]
  | other => rfl

/-- **The range exclusion, as soundness.** Over a window in which no statement
stores inside the lent range and none releases the buffer, every byte the loan's
view addresses at the end of the window is the byte that was there at its start —
so the text the program renders is the text whose UTF-8 validity the checker
decided, and not a later one. Writes outside the range happen freely and change
nothing the view can see. -/
theorem an_excluded_range_preserves_its_bytes
    (contents : Contents) (buffer : String) (r : Range) (j : Nat) (lent : r.contains j = true) :
    ∀ window : List Step, (∀ s ∈ window, touchesRange buffer r s = false) →
      byteAt (run contents window) buffer j = byteAt contents buffer j := by
  intro window
  induction window generalizing contents with
  | nil => intro _; rfl
  | cons head tail ih =>
      intro untouched
      have head_untouched : touchesRange buffer r head = false := untouched head (by simp)
      have tail_untouched : ∀ s ∈ tail, touchesRange buffer r s = false := by
        intro s held
        exact untouched s (by simp [held])
      show byteAt (run (step contents head) tail) buffer j = byteAt contents buffer j
      rw [ih (step contents head) tail_untouched]
      exact an_untouching_step_preserves_the_lent_bytes contents buffer r head j lent
        head_untouched

/-- The contrapositive the checker actually runs: a lent byte that changed had a
statement that touched it — which is the statement the refusal names. -/
theorem a_changed_lent_byte_had_a_touching_statement
    (contents : Contents) (buffer : String) (r : Range) (j : Nat) (lent : r.contains j = true)
    (window : List Step)
    (changed : byteAt (run contents window) buffer j ≠ byteAt contents buffer j) :
    ∃ s ∈ window, touchesRange buffer r s = true := by
  apply Classical.byContradiction
  intro missing
  apply changed
  apply an_excluded_range_preserves_its_bytes contents buffer r j lent
  intro s held
  by_cases touched : touchesRange buffer r s = true
  · exact absurd ⟨s, held, touched⟩ missing
  · simpa using touched

/-! ## 7. The bound, and what an admitted range can see -/

/-- `0 <= low <= high <= extent`, decided at check time. -/
def admitsRange (extent : Nat) (r : Range) : Bool :=
  decide (r.low ≤ r.high) && decide (r.high ≤ extent)

/-- An admitted range lends only bytes the buffer has. -/
theorem an_admitted_range_lends_only_bytes_that_exist
    (extent : Nat) (r : Range) (admitted : admitsRange extent r = true)
    (i : Nat) (lent : r.contains i = true) : i < extent := by
  simp only [admitsRange, Range.contains, Bool.and_eq_true, decide_eq_true_eq] at admitted lent
  omega

/-- An inverted range is refused rather than silently emptied: a model that
admitted `[3, 1)` as the empty range would let a spelling with no reading pass. -/
theorem an_inverted_range_is_refused (extent low high : Nat) (inverted : high < low) :
    admitsRange extent ⟨low, high⟩ = false := by
  simp only [admitsRange, Bool.and_eq_false_iff, decide_eq_false_iff_not, Nat.not_le]
  exact Or.inl inverted

/-- A range past the extent is refused. -/
theorem a_range_past_the_extent_is_refused (extent low high : Nat) (past : extent < high) :
    admitsRange extent ⟨low, high⟩ = false := by
  simp only [admitsRange, Bool.and_eq_false_iff, decide_eq_false_iff_not, Nat.not_le]
  exact Or.inr past

/-- The whole extent is admitted, which is the statement that the pre-card
spelling is still a program. -/
theorem the_whole_extent_is_an_admitted_range (extent : Nat) :
    admitsRange extent (wholeRange extent) = true := by
  simp [admitsRange, wholeRange]

/-! ## 8. What this card added

One operation, written as one list so that a widening changes a theorem rather
than slipping past it: the existing borrow gained a range, and nothing else was
added. In particular no **write** lend exists — the owner keeps the write
authority over every byte it did not lend, which is why the window's exactness
argument (one flat statement sequence) is untouched and a `for` beside a loan is
still refused. -/

inductive AddedOperation where
  | rangeBorrow
  deriving DecidableEq, Repr

def addedOperations : List AddedOperation := [.rangeBorrow]

theorem the_added_operation_is_exactly_one : addedOperations.length = 1 := rfl

end QCore.SlicePermissions
