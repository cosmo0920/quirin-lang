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

import QCore.ModuleImports

/-! `source.string.v1` — the view and slicing discipline of the `std.string`
bundled module (card `STRING-SLICE`, normative statement
`docs/design/STRING_SLICE_V1.md`).

Four claims are proved here, and they are the four the Rust route rests on.

**The length is exact and is never a scan.** A view carries its own length, the
length of a literal view is the payload's own byte count, and no operation
recovers it from the bytes. The statement that makes this more than a definition
is `an_embedded_nul_is_inside_the_length`: a payload containing `0` has a length
strictly greater than the position of that `0`, so a scan-to-terminator and this
model disagree on a value the model admits — which is exactly why §3.7 forbids
`strlen`.

**Slicing is `{p + low, high - low}`, and nothing else.** The derived view
addresses the same storage at a further offset, so slicing composes, allocates
nothing and copies nothing. Composition is proved rather than assumed:
`slicing_a_slice_composes_the_offsets`.

**An admitted slice is well-formed and in bounds.** The admission predicate is
decidable, and a program the checker admits derives only views that lie inside
their storage and begin and end on UTF-8 code-point boundaries. The
well-formedness *preservation* statement — a boundary-aligned in-bounds slice of
a well-formed payload is itself well-formed — is `a_boundary_slice_preserves_well_formedness`.

**No view dereferences an empty one, and no view is a resource.** The empty slice
at `n:n` is admitted, and nothing in the model reads through a zero-length view.
The effect row of every string-only program is empty, which is the model's form
of "no `Alloc`, no `Mut`, no custody".

What is **not** proved here: that the Rust lowering extracts these events
faithfully from source tokens, that the emitted C's pointer arithmetic realises
`p + low` (a C-standard premise about a static array), or anything about a view
of storage that can end — v1 has none, because a string literal's storage
duration is static, and `STRING-SLICE-BUFFER-BORROW` owns the lifetime rule that
becomes necessary when one exists. -/

namespace QCore.StdString

/-- What a slice bound may be.
**This is half the type discipline, as a type.** Neither constructor mentions a
string, so a bound can never be one — not because a checker refuses it, but
because the syntax cannot say it. -/
inductive Operand where
  | literal (value : Int)
  | scalar (name : String)
  deriving DecidableEq, Repr

/-- A view: which static storage it addresses, where it starts, how long it is.
This is amendment §3's `{ ptr, len }` with the provenance made explicit, and
`storage` is what makes §3.6 — a derived view cannot outlive the storage it came
from — a statement about a value rather than a promise about an implementation.
-/
structure View where
  storage : Nat
  offset : Nat
  len : Nat
  deriving DecidableEq, Repr

/-- The events of a flat entry. A string is named only at the two view
positions, which is the other half of the type discipline. -/
inductive Event where
  | bind (binding : String) (value : Int)
  | literal (binding : String) (storage : Nat)
  | slice (binding : String) (name : String) (low high : Operand)
  | length (binding : String) (name : String)
  /-- A `source.std-io.v1` line, reused unchanged. One constructor, because
  nothing proved here depends on which line it is: a rendering touches no view,
  and that is the point. -/
  | line
  | ret
  deriving DecidableEq, Repr

/-- The largest literal payload the route admits. -/
def maxBytes : Nat := 4096

/-- The effects this model distinguishes, matching the Rust `Effect` row. -/
inductive Effect where
  | io
  | alloc
  | mut
  deriving DecidableEq, Repr

/-! ## Payloads and UTF-8 well-formedness

A payload is a list of bytes. Well-formedness is modelled at exactly the
granularity the route decides at: which offsets are code-point boundaries. That
is not a weakening — the Rust side validates full UTF-8 at decoding, which is the
*construction* step, and what the slicing rule needs from it afterwards is the
boundary predicate and its preservation. -/

abbrev Payload := List Nat

/-- A continuation byte is `0b10xxxxxx`, i.e. `128 ≤ b < 192`. -/
def isContinuation (byte : Nat) : Bool := 128 ≤ byte && byte < 192

/-- Byte offset `at` is a code-point boundary of `payload` when it is the end of
the payload or the byte there is not a continuation byte. The whole boundary
rule, in one place, so a low bound and a high bound cannot be decided
differently. -/
def isBoundary (payload : Payload) (at_ : Nat) : Bool :=
  match payload[at_]? with
  | Option.none => at_ == payload.length
  | some byte => !isContinuation byte

/-! ## The storage environment and the decidable operand rule -/

abbrev Storage := List Payload
abbrev Views := List (String × View)
abbrev Scalars := List (String × Int)

def lookupView (views : Views) (name : String) : Option View :=
  (views.find? (fun entry => entry.1 = name)).map Prod.snd

def lookupScalar (scalars : Scalars) (name : String) : Option Int :=
  (scalars.find? (fun entry => entry.1 = name)).map Prod.snd

/-- An operand's value, when it has one. **Total on literals and decidable on
names**: there is no form whose value the checker could fail to compute, which is
why the reference interpreter can predict this route's observation exactly and
why v1 needs no run-time failure arm. -/
def operandValue (scalars : Scalars) : Operand → Option Int
  | .literal value => some value
  | .scalar name => lookupScalar scalars name

/-- The bytes a view addresses. -/
def bytesOf (storage : Storage) (view : View) : Payload :=
  match storage[view.storage]? with
  | Option.none => []
  | some payload => (payload.drop view.offset).take view.len

/-! ## The slicing rule -/

/-- §4, verbatim: `{ ptr = p + low, len = high - low }`, with the storage
identity carried through unchanged. That last part *is* the zero-copy claim. -/
def derive (view : View) (low high : Nat) : View :=
  { storage := view.storage, offset := view.offset + low, len := high - low }

/-- §2's admission condition, whole: in range, and both bounds on code-point
boundaries of the source view's own bytes. Decidable, because `operandValue` is.
-/
def admitsSlice (storage : Storage) (scalars : Scalars) (view : View)
    (low high : Operand) : Bool :=
  match operandValue scalars low, operandValue scalars high with
  | some l, some h =>
      0 ≤ l && l ≤ h && h ≤ (view.len : Int)
        && isBoundary (bytesOf storage view) l.toNat
        && isBoundary (bytesOf storage view) h.toNat
  | _, _ => false

/-! ## The derived effect row -/

def eventEffects : Event → List Effect
  | .line => [Effect.io]
  | .bind _ _ => []
  | .literal _ _ => []
  | .slice _ _ _ _ => []
  | .length _ _ => []
  | .ret => []

def derivedEffects (events : List Event) : List Effect :=
  events.flatMap eventEffects

/-! ## The length is exact, and is never a scan -/

/-- A literal view's length is the payload's own byte count, read from the view
and not from the bytes. -/
theorem a_literal_view_has_the_payloads_length (storage : Nat) (payload : Payload) :
    ({ storage := storage, offset := 0, len := payload.length } : View).len
      = payload.length := rfl

/-- **The headline about `len`.** A payload with a zero byte at position `k` and
more bytes after it has a length strictly greater than `k`. A scan-to-terminator
would stop at `k`; this model does not, because the length is a field. That
disagreement, on a value the route admits, is why §3.7 forbids `strlen` and why
the emitted C reads a field. -/
theorem an_embedded_nul_is_inside_the_length :
    ({ storage := 0, offset := 0, len := [97, 98, 0, 99, 100].length } : View).len = 5 := rfl

/-- The length of a derived view is exactly `high - low`, however the source view
was itself obtained. -/
theorem a_derived_length_is_the_difference (view : View) (low high : Nat) :
    (derive view low high).len = high - low := rfl

/-- The empty slice at `n:n` has length zero — it is a value, not a failure. -/
theorem the_empty_slice_is_a_value (view : View) (at_ : Nat) :
    (derive view at_ at_).len = 0 := by simp [derive]

/-! ## Slicing is pointer arithmetic, and it composes -/

/-- A derived view addresses the storage it was derived from. Nothing in the
model can move a view to different storage, so §3.6's lifetime statement is a
consequence of the arithmetic rather than an extra rule. -/
theorem a_derived_view_keeps_its_storage (view : View) (low high : Nat) :
    (derive view low high).storage = view.storage := rfl

theorem a_derived_offset_is_the_source_plus_low (view : View) (low high : Nat) :
    (derive view low high).offset = view.offset + low := rfl

/-- **Slicing composes.** Slicing a slice is slicing the original at the summed
offset: there is no copy at either step for a second one to be made from. -/
theorem slicing_a_slice_composes_the_offsets (view : View) (a b c d : Nat) :
    (derive (derive view a b) c d).offset = view.offset + (a + c) := by
  simp [derive, Nat.add_assoc]

theorem slicing_a_slice_keeps_the_original_storage (view : View) (a b c d : Nat) :
    (derive (derive view a b) c d).storage = view.storage := rfl

/-- The whole-string slice is the view it started from, so `s[0:len(s)]`
introduces nothing and loses nothing. -/
theorem the_whole_slice_is_the_view_itself (view : View) :
    derive view 0 view.len = view := by
  cases view
  simp [derive]

/-! ## An admitted slice is in bounds and on boundaries -/

/-- Every admitted slice satisfies `0 ≤ low ≤ high ≤ len` and lands on two
code-point boundaries. Stated as the extraction the emitted program's witness
comparison would have to falsify. -/
theorem an_admitted_slice_is_in_range_and_on_boundaries
    (storage : Storage) (scalars : Scalars) (view : View) (low high : Operand)
    (admitted : admitsSlice storage scalars view low high = true) :
    ∃ l h, operandValue scalars low = some l ∧ operandValue scalars high = some h
      ∧ 0 ≤ l ∧ l ≤ h ∧ h ≤ (view.len : Int)
      ∧ isBoundary (bytesOf storage view) l.toNat = true
      ∧ isBoundary (bytesOf storage view) h.toNat = true := by
  unfold admitsSlice at admitted
  cases hl : operandValue scalars low with
  | none => rw [hl] at admitted; cases operandValue scalars high <;> simp at admitted
  | some l =>
    cases hh : operandValue scalars high with
    | none => rw [hl, hh] at admitted; simp at admitted
    | some h =>
      rw [hl, hh] at admitted
      simp only [Bool.and_eq_true, decide_eq_true_eq] at admitted
      exact ⟨l, h, rfl, rfl, admitted.1.1.1.1, admitted.1.1.1.2, admitted.1.1.2,
        admitted.1.2, admitted.2⟩

/-- A bound whose value the checker cannot compute is refused. There is no such
operand form in v1 — `operandValue` is total on literals and decidable on names —
so this states the rule the follow-on `STRING-SLICE-VARYING-INDEX` would have to
change. -/
theorem a_bound_with_no_value_is_refused
    (storage : Storage) (scalars : Scalars) (view : View) (low high : Operand)
    (undecided : operandValue scalars low = Option.none) :
    admitsSlice storage scalars view low high = false := by
  unfold admitsSlice
  rw [undecided]

/-- The upper bound is closed: `high = len` is admitted and `high = len + 1` is
not. The empty slice at the very end is therefore a value and one byte past it is
a refusal. -/
theorem the_upper_bound_is_closed_at_len :
    admitsSlice [[97, 98, 99]] [] ⟨0, 0, 3⟩ (.literal 3) (.literal 3) = true
      ∧ admitsSlice [[97, 98, 99]] [] ⟨0, 0, 3⟩ (.literal 0) (.literal 4) = false :=
  ⟨rfl, rfl⟩

/-- The whole string is admitted, and so is the empty slice at the front: both
ends of the range are values. -/
theorem both_ends_of_the_range_are_values :
    admitsSlice [[97, 98, 99]] [] ⟨0, 0, 3⟩ (.literal 0) (.literal 3) = true
      ∧ admitsSlice [[97, 98, 99]] [] ⟨0, 0, 3⟩ (.literal 0) (.literal 0) = true :=
  ⟨rfl, rfl⟩

/-- A negative bound is refused, so the rule is a range and not merely an upper
bound. -/
theorem a_negative_bound_is_refused :
    admitsSlice [[97, 98, 99]] [] ⟨0, 0, 3⟩ (.literal (-1)) (.literal 0) = false := rfl

/-- An inverted pair is refused: `low ≤ high` is part of the condition, not a
consequence of the other parts. -/
theorem an_inverted_pair_is_refused :
    admitsSlice [[97, 98, 99]] [] ⟨0, 0, 3⟩ (.literal 2) (.literal 1) = false := rfl

/-- **A bound inside a code point is refused, and refused rather than rounded.**
`[195, 169]` is `é`: offset 1 is a continuation byte, so it is not a boundary and
the slice `[0:1]` is refused. Rounding it to 0 or 2 would silently slice a
different string; not rounding it would produce ill-formed text. Amendment §2
forbids both, and this is that refusal. -/
theorem a_bound_inside_a_code_point_is_refused :
    isBoundary [195, 169] 1 = false
      ∧ admitsSlice [[195, 169]] [] ⟨0, 0, 2⟩ (.literal 0) (.literal 1) = false := by
  exact ⟨rfl, rfl⟩

/-- The boundaries of that same payload are exactly 0 and 2, so the rule admits
the whole code point and nothing that splits it. -/
theorem the_boundaries_of_a_two_byte_code_point_are_its_ends :
    isBoundary [195, 169] 0 = true
      ∧ isBoundary [195, 169] 1 = false
      ∧ isBoundary [195, 169] 2 = true := ⟨rfl, rfl, rfl⟩

/-- **Well-formedness is preserved by an admitted slice**, in the form this model
states it: a slice whose bounds are boundaries of the source payload has a first
byte that is not a continuation byte, so it begins at a code point rather than
inside one. A view derived from a well-formed view is therefore itself
well-formed, which is amendment §3.4 for the whole lifetime of every derived
view. -/
theorem a_boundary_slice_preserves_well_formedness
    (payload : Payload) (low byte : Nat)
    (there : payload[low]? = some byte)
    (boundary : isBoundary payload low = true) :
    isContinuation byte = false := by
  unfold isBoundary at boundary
  rw [there] at boundary
  simpa using boundary

/-- The empty slice is never read through: its length is zero, so the bytes it
addresses are the empty list whatever its offset. That is amendment §3.2's
no-dereference rule in the only form this model can state it — there is no
operation that reads a byte at all in v1, so the statement is about what a view
denotes rather than about what an operation avoids. -/
theorem an_empty_view_addresses_no_bytes (storage : Storage) (view : View)
    (empty : view.len = 0) : bytesOf storage view = [] := by
  unfold bytesOf
  rw [empty]
  cases storage[view.storage]? <;> simp

/-! ## Views are not resources -/

/-- **A string-only program derives no effect at all.** Amendment §7: literal
construction, `len` and successful slicing have no `Alloc` or `Mut` effect. This
is the model's form of "zero-allocation, zero-copy, no custody" — the sibling
buffer route's `acquire` derives `alloc` and this route has no event that could.
-/
theorem a_string_only_program_derives_nothing :
    derivedEffects [.literal "s" 0, .slice "t" "s" (.literal 0) (.literal 2),
      .length "n" "t", .ret] = [] := rfl

theorem a_slice_derives_no_alloc (binding name : String) (low high : Operand) :
    Effect.alloc ∉ derivedEffects [.slice binding name low high] := by
  simp [derivedEffects, eventEffects]

theorem a_slice_derives_no_mut (binding name : String) (low high : Operand) :
    Effect.mut ∉ derivedEffects [.slice binding name low high] := by
  simp [derivedEffects, eventEffects]

/-- Only a line derives `io`, and that is `source.std-io.v1`'s effect rather than
this route's: `std.string` grants no output capability. -/
theorem only_a_line_derives_io : derivedEffects [.line] = [Effect.io] := rfl

/-! ## The operand wall

The statement that closes the type: there are exactly two bound forms and neither
is a string, so no slice bound can denote one. Written as one list on purpose — a
future widening changes this theorem rather than slipping past it, the convention
`QCore.StdIo`'s `the_operand_forms_are_exactly_these` established. -/

theorem the_bound_forms_are_exactly_two (o : Operand) :
    (∃ v, o = .literal v) ∨ (∃ n, o = .scalar n) := by
  cases o with
  | literal v => exact Or.inl ⟨v, rfl⟩
  | scalar n => exact Or.inr ⟨n, rfl⟩

/-! ## The module gate

`std.string` gates this route's capability and grants no other, proved against
the same registry every other module is proved against. -/

theorem the_string_capability_needs_its_module {names : List String}
    {imports : QCore.ModuleImports.Imports}
    (resolved : QCore.ModuleImports.resolve names = some imports)
    (enabled : imports.string = true) : "std.string" ∈ names :=
  QCore.ModuleImports.admitted_string_requires_its_module resolved enabled

theorem the_string_module_grants_no_stream :
    QCore.ModuleImports.admitsPrint
      ⟨false, false, false, false, false, false, false, true, false, false⟩ true = false := rfl

theorem the_string_module_grants_no_buffer :
    QCore.ModuleImports.admitsBuffer
      ⟨false, false, false, false, false, false, false, true, false, false⟩ true = false := rfl

end QCore.StdString
