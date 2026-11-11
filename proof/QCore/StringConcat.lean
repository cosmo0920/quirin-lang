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

import QCore.StdString

/-! Computed text: `concat`, `borrow_utf8`, and the two obligations amendment §5
names (card `STRING-CONCAT`, `docs/design/STRING_SLICE_V1.md` §5 and follow-on
`STRING-SLICE-CONCAT`).

Card `STRING-SLICE-IO` recorded the obstacle this card removes: "every payload
v1 can render is a literal or a slice of one, so a program renders text it
**wrote** and never text it computed." Four claims are proved here, and they are
the four the Rust implementation rests on.

**The length arithmetic.** A composition's length is the sum of its pieces'
lengths, and its bytes are their concatenation. That is what makes a
`Content-Length` a program *derives* as decidable as one it writes, and it is
the operational content of the whole card.

**The composition is static storage of its own.** A composed view addresses a
payload the compiler computed, not a window onto any piece — so it needs no
allocation, no custody and no lifetime rule, and it is what lets a composition of
a *borrowed* view outlive the buffer the piece came from. The theorem that makes
that more than a convention is `a_composition_addresses_none_of_its_pieces`.

**The borrow exclusion.** A borrow's window is an interval of statement indices,
and a store or release inside it is refused. The statement that makes this
soundness rather than bookkeeping is
`an_excluded_window_preserves_the_buffers_bytes`: over a window in which nothing
touches the buffer, the bytes a view addresses at the end are the bytes that were
validated at the start.

**The static UTF-8 decision.** v1's buffers hold only constant bytes written at
constant indices, so their contents are a check-time value and the validity
question is decided by running a decision function on it. What is proved here is
that the decision is *made on those bytes* and that it separates the cases the
card's tests measure — including the one that says **yes**: a zeroed buffer is
well-formed, because U+0000 is an ordinary code point.

What is **not** proved here, carried over from the family's convention and from
`QCore.StdString`'s own boundary: that the Rust lowering extracts these events
faithfully from source tokens, and that this model's `wellFormed` is full UTF-8
validation — it decodes the lead/continuation structure and does not reject
overlong forms or surrogates, which the Rust side rejects by delegating to the
language's own decoder at the one point bytes enter the model. `freeze_utf8` is
**not** modelled, because it is not implemented: v1's `View` declaration has
no custody field for a release to discharge. `views_equal_iff_components_equal`
characterizes equality of this representation; it is not an ownership theorem. -/

namespace QCore.StringConcat

open QCore.StdString

/-! ## 1. Composition

A composition is the concatenation of its pieces' bytes, into storage of its
own. -/

/-- The bytes a composition produces. Written as `flatten` because that *is* the
operation; there is nothing else for a definition to say. -/
def compose (pieces : List Payload) : Payload := pieces.flatten

/-- The lengths a composition's record publishes, in written order. -/
def pieceLens (pieces : List Payload) : List Nat := pieces.map List.length

/-- **The length arithmetic.** A composition's length is the sum of its pieces'
lengths — the equation the certificate publishes, and the reason a computed
`Content-Length` is a check-time constant. -/
theorem the_composed_length_is_the_sum_of_the_piece_lengths (pieces : List Payload) :
    (compose pieces).length = (pieceLens pieces).sum := by
  unfold compose pieceLens
  induction pieces with
  | nil => simp
  | cons head tail ih => simp [List.sum_cons, ih]

/-- Two pieces compose by appending, which is the base case a reader checks. -/
theorem composing_two_pieces_appends_their_bytes (a b : Payload) :
    compose [a, b] = a ++ b := by
  simp [compose]

/-- And their lengths add. -/
theorem composing_two_pieces_adds_their_lengths (a b : Payload) :
    (compose [a, b]).length = a.length + b.length := by
  simp [compose]

/-- Composition is associative in the sense the surface needs: a composition of
three is the composition of a composition. Stated because the route admits any
arity and a reader should not have to take the generalisation on trust. -/
theorem a_composition_of_three_is_a_composition_of_a_composition (a b c : Payload) :
    compose [a, b, c] = compose [compose [a, b], c] := by
  simp [compose]

/-- An empty piece contributes nothing — not an error, and not a byte. -/
theorem an_empty_piece_contributes_no_bytes (a b : Payload) :
    compose [a, [], b] = compose [a, b] := by
  simp [compose]

/-! ## 2. A composition is storage of its own

The storage a view addresses is `Nat`-indexed in `QCore.StdString`. A
composition's result is a **fresh** index — the one past the end of the
environment it was computed in — so it is not a window onto any piece. -/

/-- The view a composition binds: a fresh storage slot, offset zero, the
composed length. -/
def composedView (slot : Nat) (pieces : List Payload) : View :=
  { storage := slot, offset := 0, len := (compose pieces).length }

/-- The storage a composition is written into: appended, never overwriting a
payload a piece might still address. -/
def extend (storage : Storage) (pieces : List Payload) : Storage :=
  storage ++ [compose pieces]

/-- **The composed view addresses the composed payload**, whole, from its
start. -/
theorem the_composed_view_addresses_the_composed_payload
    (storage : Storage) (pieces : List Payload) :
    bytesOf (extend storage pieces) (composedView storage.length pieces)
      = compose pieces := by
  unfold bytesOf extend composedView
  simp

/-- **A composition addresses none of its pieces.** Its storage index is one the
environment did not hold, so no piece's storage is the composition's — which is
why a composition of a *borrowed* view is not itself a borrow, why it carries no
window, and why the buffer it was read from may be released immediately
afterwards. -/
theorem a_composition_addresses_none_of_its_pieces
    (storage : Storage) (pieces : List Payload) (piece : View)
    (held : piece.storage < storage.length) :
    piece.storage ≠ (composedView storage.length pieces).storage := by
  unfold composedView
  simp only []
  exact Nat.ne_of_lt held

/-- Extending the storage leaves every view the environment already held
addressing exactly the bytes it addressed. A composition is additive on the
environment and destructive of nothing. -/
theorem extending_the_storage_moves_no_existing_view
    (storage : Storage) (pieces : List Payload) (view : View)
    (held : view.storage < storage.length) :
    bytesOf (extend storage pieces) view = bytesOf storage view := by
  unfold bytesOf extend
  rw [List.getElem?_append_left held]

/-! ## 3. The admission rule

A composition of one is refused, and so is one longer than the route models.
Both are decided on the pieces alone. -/

/-- §5's admission condition for a composition, whole: two or more pieces, and a
result the route can model exactly. -/
def admitsConcat (pieces : List Payload) : Bool :=
  2 ≤ pieces.length && (compose pieces).length ≤ maxBytes

/-- A composition of one is refused. A composition of one *is* that piece, and
admitting it would give the route two spellings of one view with nothing to tell
them apart by. -/
theorem a_composition_of_one_is_refused (piece : Payload) :
    admitsConcat [piece] = false := by
  simp [admitsConcat]

/-- A composition of none is refused for the same reason. -/
theorem a_composition_of_none_is_refused : admitsConcat [] = false := by
  simp [admitsConcat]

/-- An over-long composition is refused rather than truncated: a payload the
route cannot model exactly is not modelled at all, the discipline
`QCore.StdString` applies to a literal. -/
theorem an_over_long_composition_is_refused (pieces : List Payload)
    (long : maxBytes < (compose pieces).length) :
    admitsConcat pieces = false := by
  unfold admitsConcat
  simp [Nat.not_le.mpr long]

/-- An admitted composition is inside the bound, in the other direction. -/
theorem an_admitted_composition_is_inside_the_bound (pieces : List Payload)
    (admitted : admitsConcat pieces = true) :
    (compose pieces).length ≤ maxBytes := by
  unfold admitsConcat at admitted
  simp only [Bool.and_eq_true, decide_eq_true_eq] at admitted
  exact admitted.2

/-! ## 4. The static UTF-8 decision

v1's buffers hold only constant bytes written at constant indices, so a buffer's
contents are a check-time value. The decision is therefore made by running a
function on that value, and this section is what that function is and what it
separates. -/

/-- How many bytes the sequence a lead byte opens occupies, when the byte opens
one at all. A continuation byte opens nothing, which is the case a lone `0x80`
falls into. -/
def leadLength (byte : Nat) : Option Nat :=
  if byte < 128 then some 1
  else if byte < 192 then Option.none
  else if byte < 224 then some 2
  else if byte < 240 then some 3
  else if byte < 248 then some 4
  else Option.none

/-- The decision, with the payload's own length as fuel so the definition needs
no termination argument: each step consumes at least one byte. -/
def scan : Nat → Payload → Bool
  | _, [] => true
  | 0, _ :: _ => false
  | fuel + 1, byte :: rest =>
      match leadLength byte with
      | Option.none => false
      | some width =>
          let tail := rest.take (width - 1)
          (tail.length == width - 1) && tail.all isContinuation
            && scan fuel (rest.drop (width - 1))

/-- Whether a payload is a sequence of well-formed UTF-8 code points, as far as
the lead/continuation structure decides it. -/
def wellFormed (payload : Payload) : Bool := scan payload.length payload

/-- The empty payload is well-formed: there is nothing in it to be ill-formed. -/
theorem the_empty_payload_is_well_formed : wellFormed [] = true := by
  decide

/-- **A zeroed buffer is well-formed, and that is a `yes` worth stating.**
`bytes(N)` zero-initialises, `U+0000` is an ordinary code point, and a buffer
nothing has written is therefore borrowable. A model that refused it would
disagree with the route's own `calloc`. -/
theorem a_zeroed_buffer_is_well_formed : wellFormed [0, 0, 0] = true := by
  decide

/-- ASCII text is well-formed. -/
theorem an_ascii_payload_is_well_formed : wellFormed [104, 105, 33] = true := by
  decide

/-- A two-byte code point is well-formed: `[195, 169]` is `é`. -/
theorem a_two_byte_code_point_is_well_formed : wellFormed [195, 169] = true := by
  decide

/-- A lone continuation byte is refused: it opens no sequence. -/
theorem a_lone_continuation_byte_is_refused : wellFormed [128] = false := by
  decide

/-- A lead byte whose continuation is missing is refused: `[195, 33]` is the
truncated sequence the card's own test measures. -/
theorem a_truncated_sequence_is_refused : wellFormed [195, 33] = false := by
  decide

/-- A byte that begins no sequence at all is refused. -/
theorem a_byte_outside_the_encoding_is_refused : wellFormed [255] = false := by
  decide

/-- A lead byte at the very end of the payload is refused rather than accepted as
a one-byte code point. -/
theorem a_dangling_lead_byte_is_refused : wellFormed [195] = false := by
  decide

/-! ## 5. The borrow, and its window

A borrow yields a view of a buffer's storage, admitted only when that storage's
bytes are well-formed. The window is an interval of statement indices, which is
exact rather than conservative because the route is one flat sequence: there is
no branch to make a later index unreachable and no loop to make an earlier one
run again. -/

/-- A borrow record, as the certificate publishes it. -/
structure Borrow where
  buffer : String
  view : String
  from_ : Nat
  through : Nat
  deriving DecidableEq, Repr

/-- Whether a borrow's window covers statement `i`. -/
def liveAt (borrow : Borrow) (i : Nat) : Bool :=
  borrow.from_ ≤ i && i ≤ borrow.through

/-- Whether any live borrow of `buffer` covers statement `i`. -/
def excluded (borrows : List Borrow) (buffer : String) (i : Nat) : Bool :=
  borrows.any (fun borrow => borrow.buffer == buffer && liveAt borrow i)

/-- A statement inside a borrow's window that would change or end the buffer is
refused. -/
theorem a_statement_inside_the_window_is_refused
    (borrow : Borrow) (i : Nat)
    (after : borrow.from_ ≤ i) (before : i ≤ borrow.through) :
    excluded [borrow] borrow.buffer i = true := by
  simp [excluded, liveAt, after, before]

/-- A statement after the window is admitted. The window is an **interval**, not
"the rest of the entry" — which is what makes a program that borrows able to
release its buffer at all. -/
theorem a_statement_after_the_window_is_admitted
    (borrow : Borrow) (i : Nat) (past : borrow.through < i) :
    excluded [borrow] borrow.buffer i = false := by
  simp [excluded, liveAt, Nat.not_le.mpr past]

/-- A borrow of one buffer constrains no other. -/
theorem a_borrow_constrains_only_its_own_buffer
    (borrow : Borrow) (other : String) (i : Nat) (different : borrow.buffer ≠ other) :
    excluded [borrow] other i = false := by
  simp [excluded, different]

/-- A borrow nothing reads has `through = from_`, so it constrains only its own
statement — recorded because it is what makes an unused borrow free. -/
theorem an_unread_borrow_constrains_only_its_own_statement
    (buffer view : String) (at_ : Nat) (i : Nat) (past : at_ < i) :
    excluded [{ buffer := buffer, view := view, from_ := at_, through := at_ }] buffer i
      = false := by
  simp [excluded, liveAt, Nat.not_le.mpr past]

/-- **A view derived from a borrow extends the window.** A slice of a borrowed
view addresses the same buffer, so it is registered as a borrow with its own last
use, and the buffer is excluded at that later index too. Without this a program
could release the storage a live derived view addresses. -/
theorem a_derived_view_extends_the_window
    (buffer view derived : String) (at_ later : Nat) (reached : at_ ≤ later) :
    excluded
      [{ buffer := buffer, view := view, from_ := at_, through := at_ },
       { buffer := buffer, view := derived, from_ := at_, through := later }]
      buffer later = true := by
  simp [excluded, liveAt, reached]

/-! ### The soundness statement

What the exclusion buys, as an equation over the bytes. -/

/-- A statement, as far as a buffer's bytes are concerned. -/
inductive Step where
  | store (buffer : String) (index : Nat) (byte : Nat)
  | release (buffer : String)
  | other
  deriving DecidableEq, Repr

/-- The modelled contents of every buffer: a released one holds nothing. -/
abbrev Contents := String → Option Payload

/-- Whether a step changes or ends the bytes of `buffer`. -/
def touches (buffer : String) : Step → Bool
  | .store name _ _ => name == buffer
  | .release name => name == buffer
  | .other => false

/-- Running one step over the contents. -/
def step (contents : Contents) : Step → Contents
  | .store buffer index byte =>
      fun name =>
        if name = buffer then (contents name).map (fun payload => payload.set index byte)
        else contents name
  | .release buffer => fun name => if name = buffer then Option.none else contents name
  | .other => contents

def run (contents : Contents) : List Step → Contents
  | [] => contents
  | s :: rest => run (step contents s) rest

/-- A step that does not touch a buffer leaves its bytes exactly as they were. -/
theorem an_untouching_step_preserves_the_bytes
    (contents : Contents) (buffer : String) (s : Step)
    (untouched : touches buffer s = false) :
    step contents s buffer = contents buffer := by
  cases s with
  | store name index byte =>
      simp only [touches, beq_eq_false_iff_ne, ne_eq] at untouched
      simp [step, Ne.symm untouched]
  | release name =>
      simp only [touches, beq_eq_false_iff_ne, ne_eq] at untouched
      simp [step, Ne.symm untouched]
  | other => rfl

/-- **The borrow exclusion, as soundness.** Over a window in which no statement
touches the buffer, the bytes a borrowed view addresses at the end of the window
are the bytes that were validated at its start — so the view the program renders
is the view the checker decided was well-formed UTF-8, and not a later one. -/
theorem an_excluded_window_preserves_the_buffers_bytes
    (contents : Contents) (buffer : String) :
    ∀ window : List Step, (∀ s ∈ window, touches buffer s = false) →
      run contents window buffer = contents buffer := by
  intro window
  induction window generalizing contents with
  | nil => intro _; rfl
  | cons head tail ih =>
      intro untouched
      have head_untouched : touches buffer head = false := untouched head (by simp)
      have tail_untouched : ∀ s ∈ tail, touches buffer s = false := by
        intro s held
        exact untouched s (by simp [held])
      show run (step contents head) tail buffer = contents buffer
      rw [ih (step contents head) tail_untouched]
      exact an_untouching_step_preserves_the_bytes contents buffer head head_untouched

/-- The contrapositive the checker actually runs: a window over which the bytes
did change is one in which some statement touched the buffer — which is the
statement the refusal names. -/
theorem a_changed_buffer_had_a_touching_statement
    (contents : Contents) (buffer : String) (window : List Step)
    (changed : run contents window buffer ≠ contents buffer) :
    ∃ s ∈ window, touches buffer s = true := by
  apply Classical.byContradiction
  intro missing
  apply changed
  apply an_excluded_window_preserves_the_buffers_bytes
  intro s held
  by_cases touched : touches buffer s = true
  · exact absurd ⟨s, held, touched⟩ missing
  · simpa using touched

/-! ## 6. Effects, and the custody that is not here -/

/-- The events this card adds, beside `QCore.StdString`'s own. -/
inductive Added where
  | concat (binding : String)
  | borrow (binding buffer : String)
  deriving DecidableEq, Repr

/-- **Neither operation derives an effect.** Composing text is a compile-time
computation whose result becomes static storage, and borrowing is a read — so
amendment §7's "no `Alloc` or `Mut` for a string operation" survives this card
exactly. The `alloc` and `mut` a composed module can carry are the **buffer's**.
-/
def addedEffects : Added → List Effect
  | .concat _ => []
  | .borrow _ _ => []

theorem a_composition_derives_no_effect (binding : String) :
    addedEffects (.concat binding) = [] := rfl

theorem a_borrow_derives_no_effect (binding buffer : String) :
    addedEffects (.borrow binding buffer) = [] := rfl

theorem neither_added_operation_derives_alloc (added : Added) :
    Effect.alloc ∉ addedEffects added := by
  cases added <;> simp [addedEffects]

theorem neither_added_operation_derives_mut (added : Added) :
    Effect.mut ∉ addedEffects added := by
  cases added <;> simp [addedEffects]

/-- These three components completely determine a view. This is a property
of the representation, not a proof that arbitrary strings carry no custody.
An additional independent field would invalidate the reverse implication. -/
theorem views_equal_iff_components_equal (left right : View) :
    left = right ↔ left.storage = right.storage ∧ left.offset = right.offset ∧
      left.len = right.len := by
  cases left
  cases right
  simp only [View.mk.injEq]

/-- The added vocabulary is exactly two operations, written as one list so a
widening changes a theorem rather than slipping past it — the
`QCore.StdIo.the_operand_forms_are_exactly_these` convention. -/
theorem the_added_operations_are_exactly_two (added : Added) :
    (∃ binding, added = .concat binding) ∨ (∃ binding buffer, added = .borrow binding buffer) := by
  cases added with
  | concat binding => exact Or.inl ⟨binding, rfl⟩
  | borrow binding buffer => exact Or.inr ⟨binding, buffer, rfl⟩

end QCore.StringConcat
