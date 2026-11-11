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

import QCore.StdHttp

/-! A **computed** `Content-Length` inside `source.http.v1`
(card `HTTP-V1-COMPUTED-LENGTH`).

`QCore.StdHttp` proved the `Content-Length` equality for a message whose every
component is a literal. This module proves that **widening a component to a
value the entry computes does not weaken that equality, and does not weaken the
`CR`/`LF` injection wall either** — which is the whole content of the card.

Three claims, in the order the card argues them.

**A composition's length is the sum of its pieces' lengths.** Every piece a
`concat` admits is check-time-known, so the compiler holds the composed bytes;
the length is then an arithmetic fact about lengths it already has, and is
itself a check-time constant. That is standing constraint 15h, stated here as an
equation so that a piece form whose length the compiler could not compute would
change a theorem rather than slip past one.

**A rendered number parses back to the number it rendered.** `dec(NAME)` writes
the decimal digits of a check-time constant, and `parseLength` — the model of the
`1*DIGIT` reader `admitFraming` is handed — recovers exactly that constant from
them. Composed with the first claim, that is the statement a computed
`Content-Length` is still *decided*: the gate is `QCore.StdHttp.admitFraming`
itself, unedited, applied to numbers the compiler computed rather than to numbers
the author wrote.

**The injection wall is exact over a composition, in both directions.** A
composition is framing-free **exactly when** every piece is — so composing
cannot launder a `CR` into a head, and cannot spoil a head built from clean
pieces either. The rendering case is settled outright: decimal digits are field
content, so a rendered component can never carry a framing byte at all.

What is **not** proved here: that the Rust lowering extracts these pieces
faithfully from source tokens (a tested Rust boundary, as everywhere in this
family); that `Rendering::render`'s `hex` spelling is modelled — it is not, and
it is deliberately not a `Content-Length`, being refused by the `1*DIGIT` rule
this module's `parseLength` states; or that a negative `dec` rendering is
modelled — the `-` it writes is field content but not a digit, which is the same
refusal by the same rule. Both are measured on the Rust side instead. -/

namespace QCore.HttpComputedLength

open QCore.StdHttp

/-! ### Decimal text, and the roundtrip that makes a length decidable -/

/-- The decimal digits of a natural number, most significant first. -/
def digits (n : Nat) : List Nat :=
  if n < 10 then [n] else digits (n / 10) ++ [n % 10]
termination_by n
decreasing_by exact Nat.div_lt_self (by omega) (by omega)

/-- The number a digit list denotes. -/
def fromDigits (ds : List Nat) : Nat := ds.foldl (fun acc d => acc * 10 + d) 0

theorem fromDigits_snoc (ds : List Nat) (d : Nat) :
    fromDigits (ds ++ [d]) = fromDigits ds * 10 + d := by
  simp [fromDigits]

/-- **The roundtrip.** Rendering a number as decimal text and reading that text
back yields the number — which is why a `Content-Length` the compiler *wrote*
from a length it *computed* is a number the framing gate can compare. -/
theorem fromDigits_digits (n : Nat) : fromDigits (digits n) = n := by
  rw [digits]
  split
  · simp [fromDigits]
  · rename_i big
    rw [fromDigits_snoc, fromDigits_digits (n / 10)]
    omega
termination_by n
decreasing_by exact Nat.div_lt_self (by omega) (by omega)

/-- Every digit of a rendering is a digit, which is what makes the rendered text
`1*DIGIT` and therefore a `Content-Length` at all. -/
theorem digits_lt_ten (n : Nat) : ∀ d ∈ digits n, d < 10 := by
  rw [digits]
  split
  · intro d hd
    simp at hd
    omega
  · rename_i big
    intro d hd
    rcases List.mem_append.mp hd with left | right
    · exact digits_lt_ten (n / 10) d left
    · simp at right
      omega
termination_by n
decreasing_by exact Nat.div_lt_self (by omega) (by omega)

/-- A rendering is never empty, so it is `1*DIGIT` and not `0*DIGIT`. -/
theorem digits_ne_nil (n : Nat) : digits n ≠ [] := by
  rw [digits]
  split
  · simp
  · simp

/-! ### The bytes of a rendering -/

/-- The ASCII byte of one decimal digit. -/
def digitByte (d : Nat) : UInt8 := UInt8.ofNat (48 + d)

theorem digitByte_toNat (d : Nat) (small : d < 10) : (digitByte d).toNat = 48 + d := by
  simp [digitByte, UInt8.toNat_ofNat] <;> omega

/-- `dec(NAME)`'s bytes: the decimal text of a check-time constant, which is
exactly what `Rendering::render` writes on the Rust side. -/
def decBytes (n : Nat) : List UInt8 := (digits n).map digitByte

/-- RFC 9112 §6.2's `1*DIGIT`, as a byte predicate. -/
def isDigitByte (b : UInt8) : Bool := 48 ≤ b.toNat && b.toNat ≤ 57

/-- The value of one digit byte. -/
def digitValue (b : UInt8) : Nat := b.toNat - 48

/-- The `Content-Length` reader `admitFraming` is handed: a non-empty run of
decimal digits, and nothing else. A value that is not `1*DIGIT` — a `hex`
rendering's `0x…`, a negative rendering's `-`, a token with a space in it — has
no length and is refused. -/
def parseLength (bs : List UInt8) : Option Nat :=
  if bs.isEmpty || !bs.all isDigitByte then none else some (fromDigits (bs.map digitValue))

theorem decBytes_ne_nil (n : Nat) : decBytes n ≠ [] := by
  simp [decBytes, digits_ne_nil n]

theorem decBytes_all_digits (n : Nat) : (decBytes n).all isDigitByte = true := by
  simp only [decBytes, List.all_eq_true, List.mem_map]
  rintro b ⟨d, hd, rfl⟩
  have small := digits_lt_ten n d hd
  simp only [isDigitByte, digitByte_toNat d small, decide_eq_true_eq, Bool.and_eq_true]
  omega

theorem decBytes_digitValues (n : Nat) : (decBytes n).map digitValue = digits n := by
  simp only [decBytes, List.map_map]
  rw [List.map_congr_left (g := id) ?_, List.map_id]
  intro d hd
  have small := digits_lt_ten n d hd
  simp [Function.comp, digitValue, digitByte_toNat d small]

/-- **The rendering roundtrip at the byte level.** The `Content-Length` reader
recovers from `dec(n)`'s bytes exactly the `n` the compiler rendered. -/
theorem a_rendering_parses_back_to_the_number_it_rendered (n : Nat) :
    parseLength (decBytes n) = some n := by
  have empty : (decBytes n).isEmpty = false := by
    cases h : decBytes n with
    | nil => exact absurd h (decBytes_ne_nil n)
    | cons _ _ => rfl
  simp only [parseLength, empty, decBytes_all_digits n, Bool.not_true, Bool.or_self,
    decBytes_digitValues n, fromDigits_digits n]
  simp

/-- **A rendered component can never carry a framing byte.** Decimal digits are
field content, so the `CR`/`LF` exclusion that makes an admitted head's framing
unforgeable is satisfied outright by every `dec(NAME)` — the wall is not merely
checked there, it is unreachable. -/
theorem a_rendering_carries_no_framing_byte (n : Nat) :
    isFieldContent (decBytes n) = true := by
  simp only [isFieldContent, List.all_eq_true, decBytes, List.mem_map]
  rintro b ⟨d, hd, rfl⟩
  have small := digits_lt_ten n d hd
  simp only [isFieldByte, digitByte_toNat d small, decide_eq_true_eq]
  omega

/-! ### Composition -/

/-- What one piece of a component value may be. Every form is check-time-known,
which is the premise everything below rests on. -/
inductive Piece where
  /-- A quoted token: this module's own syntax, constructing no `string` value. -/
  | literal (bytes : List UInt8)
  /-- A name this entry bound to text. -/
  | name (n : String)
  /-- `dec(NAME)` — the decimal text of a bound integer. -/
  | scalar (n : String)
  deriving Repr

/-- The check-time environment a component resolves against. Two maps that never
merge: a text name is never a scalar and a scalar name is never text. -/
structure Env where
  text : String → Option (List UInt8)
  scalars : String → Option Nat

/-- The exact bytes one piece denotes. Total wherever the names resolve — and
never deferred to run time, which is the property the gates below need. -/
def resolve (e : Env) : Piece → Option (List UInt8)
  | .literal bytes => some bytes
  | .name n => e.text n
  | .scalar n => (e.scalars n).map decBytes

/-- A composition: the pieces' bytes, in written order. -/
def compose (e : Env) : List Piece → Option (List UInt8)
  | [] => some []
  | p :: ps =>
    match resolve e p, compose e ps with
    | some a, some b => some (a ++ b)
    | _, _ => none

/-- The resolved pieces of a composition, when every piece resolves. -/
def resolved (e : Env) (ps : List Piece) : List (List UInt8) :=
  ps.map (fun p => (resolve e p).getD [])

/-- **A composition is the flattening of its resolved pieces.** Stated so the
two theorems below can be about `List.flatten` and hold for any way of getting
there. -/
theorem compose_eq_flatten (e : Env) (ps : List Piece)
    (all : ∀ p ∈ ps, (resolve e p).isSome = true) :
    compose e ps = some (resolved e ps).flatten := by
  induction ps with
  | nil => rfl
  | cons p ps ih =>
    have head : (resolve e p).isSome = true := all p (List.mem_cons_self ..)
    have rest : ∀ q ∈ ps, (resolve e q).isSome = true := fun q hq =>
      all q (List.mem_cons_of_mem _ hq)
    cases hp : resolve e p with
    | none => rw [hp] at head; exact absurd head (by simp)
    | some a =>
      simp only [compose, hp, ih rest, resolved, List.map_cons, List.flatten_cons,
        Option.getD_some]

/-- **The length arithmetic, as an equation.** A composition's byte count is the
sum of its pieces' byte counts — so a length derived from a composed payload is
a check-time constant exactly as a literal's own length is. -/
theorem the_composed_length_is_the_sum_of_the_piece_lengths (parts : List (List UInt8)) :
    parts.flatten.length = (parts.map List.length).sum := by
  induction parts with
  | nil => rfl
  | cons p ps ih => simp [ih]

/-- **The injection wall over a composition, exactly.** A composition carries no
framing byte **if and only if** none of its pieces does. The forward direction
says composing cannot spoil clean pieces; the backward direction says composing
cannot launder a dirty one. -/
theorem a_composition_is_framing_free_exactly_when_its_pieces_are
    (parts : List (List UInt8)) :
    isFieldContent parts.flatten = true ↔ ∀ p ∈ parts, isFieldContent p = true := by
  simp only [isFieldContent, List.all_eq_true, List.mem_flatten]
  constructor
  · intro h p hp b hb
    exact h b ⟨p, hp, hb⟩
  · rintro h b ⟨p, hp, hb⟩
    exact h p hp b hb

/-- The contrapositive, spelled out because it is the direction the card's
refusal test measures: a piece carrying a `CR` or an `LF` makes the whole
composition carry one, so it is refused at the field gate rather than hidden
inside a composed value. -/
theorem a_framing_byte_in_a_piece_survives_the_composition
    (parts : List (List UInt8)) (p : List UInt8) (mem : p ∈ parts)
    (bad : isFieldContent p = false) : isFieldContent parts.flatten = false := by
  cases clean : isFieldContent parts.flatten with
  | false => rfl
  | true =>
    have := (a_composition_is_framing_free_exactly_when_its_pieces_are parts).mp clean p mem
    rw [this] at bad
    exact absurd bad (by simp)

/-! ### The gate, unchanged -/

/-- **The card's theorem.** A `Content-Length` written as `dec(len(content))` is
admitted by `QCore.StdHttp.admitFraming` — the *same* gate, *unedited* — because
the rendering parses back to the content's own byte count. A computed length is
therefore still **decided**, not assumed. -/
theorem a_rendered_content_length_is_admitted
    (m : Message) (bytes : List UInt8) (hasContent : m.content = some bytes) :
    admitFraming m (parseLength (decBytes bytes.length)) = some (some bytes.length) := by
  rw [a_rendering_parses_back_to_the_number_it_rendered]
  simp [admitFraming, hasContent]

/-- And the converse, which is what keeps the check a **gate** rather than a
description: a rendered length of a number that is not the content's byte count
is refused, by that same unedited function. -/
theorem a_rendered_length_that_is_not_the_content_length_is_refused
    (m : Message) (bytes : List UInt8) (n : Nat)
    (hasContent : m.content = some bytes) (differs : n ≠ bytes.length) :
    admitFraming m (parseLength (decBytes n)) = none := by
  rw [a_rendering_parses_back_to_the_number_it_rendered]
  exact a_length_that_is_not_the_content_length_is_refused m n bytes hasContent differs

/-- **The two halves composed: the card in one statement.** A message whose
content is a composition, declaring the sum of its pieces' lengths rendered as
decimal text, is admitted — and the number it declares *is* the number of bytes
it carries. Neither side was written by the author; both are the compiler's. -/
theorem a_composed_content_is_framed_by_the_sum_of_its_pieces
    (m : Message) (parts : List (List UInt8))
    (hasContent : m.content = some parts.flatten) :
    admitFraming m (parseLength (decBytes ((parts.map List.length).sum)))
      = some (some ((parts.map List.length).sum)) := by
  rw [← the_composed_length_is_the_sum_of_the_piece_lengths parts]
  exact a_rendered_content_length_is_admitted m parts.flatten hasContent

/-- A message whose content is a composition but which declares a *different*
number is refused, by the same function. The pair is what makes the equality a
gate over computed components as it already was over written ones. -/
theorem a_composed_content_with_a_wrong_declared_length_is_refused
    (m : Message) (parts : List (List UInt8)) (n : Nat)
    (hasContent : m.content = some parts.flatten)
    (differs : n ≠ (parts.map List.length).sum) :
    admitFraming m (parseLength (decBytes n)) = none := by
  rw [← the_composed_length_is_the_sum_of_the_piece_lengths parts] at differs
  exact a_rendered_length_that_is_not_the_content_length_is_refused m parts.flatten n
    hasContent differs

/-! ### The vocabulary this card adds -/

/-- The two statements this card adds to the route. Written as one list so that a
third changes this theorem rather than slipping past it — the discipline
`QCore.StdHttp.the_statement_forms_are_exactly_three` applies to the *message*
vocabulary, which this card does not widen. -/
inductive Added where
  /-- `let NAME = concat(PIECE, …);` -/
  | compose (binding : String) (pieces : List Piece)
  /-- `let NAME = len(NAME);` -/
  | length (binding name : String)
  deriving Repr

/-- The effects one added statement derives: none, on both constructors. A
composition happens at check time — it allocates nothing, mutates nothing and
writes nothing — so amendment §7's "no `Alloc` or `Mut` for a string operation"
survives this card, and so does `QCore.StdHttp.no_http_statement_derives_an_effect`
for the vocabulary it is about. -/
def addedEffects : Added → List String
  | .compose _ _ => []
  | .length _ _ => []

theorem no_computed_statement_derives_an_effect (s : Added) : addedEffects s = [] := by
  cases s <;> rfl

theorem the_added_statement_forms_are_exactly_two (s : Added) :
    (∃ b ps, s = .compose b ps) ∨ (∃ b n, s = .length b n) := by
  cases s with
  | compose b ps => exact Or.inl ⟨b, ps, rfl⟩
  | length b n => exact Or.inr ⟨b, n, rfl⟩

/-- **A composition of one is refused.** A composition of one *is* that piece,
and admitting it would give the route two spellings of one value with nothing to
tell them apart by — `source.string.v1`'s own rule, inherited. -/
def admitComposition (ps : List Piece) : Bool := 2 ≤ ps.length

theorem a_composition_of_one_is_refused (p : Piece) : admitComposition [p] = false := by
  simp [admitComposition]

theorem a_composition_of_none_is_refused : admitComposition [] = false := by
  simp [admitComposition]

theorem a_composition_of_two_is_admitted (p q : Piece) : admitComposition [p, q] = true := by
  simp [admitComposition]

end QCore.HttpComputedLength
