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

/-! `source.http.v1` — the message grammar, the framing invariants and the
`Content-Length` equality of the `std.http` bundled module
(card `HTTP-QUIRIN-V1`).

Three claims are proved here, and they are the three the Rust route rests on.

**A message's wire bytes are its head followed by its content**, and its head
ends in an empty line. That is RFC 9112 §2.1's own shape, and it is stated as an
equation rather than as a comment so that a change to the builder changes a
theorem.

**The framing is constructed, not spelled.** No admitted component may carry a
`CR` or an `LF` — that is what `admitField` and `admitStart` decide — so every
`CRLF` in an admitted message's head is one `headBytes` wrote. The count is
exact: `1 + fields + 1`, the start line, one line per field, and the empty line
that ends the head. Header injection is therefore not expressible on this route,
which is a property of the grammar rather than a promise about inputs.

**A declared `Content-Length` is the content's byte length.** This is the card's
signature claim and the one that is only decidable because every component is a
literal: `admitFraming` returns the declared value exactly when it equals the
content's length, and refuses otherwise. The two directions are both proved —
an admitted message's declaration *is* its length, and a message whose
declaration is not its length is refused.

What is **not** proved here: that the Rust lowering extracts these messages
faithfully from source tokens; that the transport delivers them (that is
`QCore.StdSocket`'s subject, and this route inherits it unedited); or that the
subset modelled is HTTP/1.1 — it is a **bounded subset**, and everything outside
it is named in `docs/reports/http-quirin-v1/README.md` rather than proved away.
In particular there is no chunked coding, no persistent connection, no parsing
of unknown input, and no computed text anywhere. -/

namespace QCore.StdHttp

/-- The one version this route admits, spelled as it appears on the wire. -/
def version : String := "HTTP/1.1"

/-- The two bytes that end every line of a message head. -/
def crlf : List UInt8 := [13, 10]

/-- A message's start line. RFC 9112 §3 request-line or §4 status-line. -/
inductive Start where
  | request (method target ver : String)
  | response (ver : String) (status : Nat) (reason : String)
  deriving Repr

/-- One header field, exactly as it will appear on the wire. -/
structure Field where
  name : String
  value : String
  deriving Repr

/-- One declared message.

`content = none` and `content = some []` are **different**: the first is a
message with no content and the second a message whose content is zero bytes,
which must declare `Content-Length: 0`. -/
structure Message where
  start : Start
  fields : List Field
  content : Option (List UInt8)
  deriving Repr

/-- RFC 9110 §5.6.2 `tchar`. -/
def isTchar (b : UInt8) : Bool :=
  (48 ≤ b.toNat ∧ b.toNat ≤ 57) ∨ (65 ≤ b.toNat ∧ b.toNat ≤ 90)
    ∨ (97 ≤ b.toNat ∧ b.toNat ≤ 122)
    ∨ b.toNat ∈ [33, 35, 36, 37, 38, 39, 42, 43, 45, 46, 94, 95, 96, 124, 126]

/-- RFC 9110 §5.6.2 `token`: `1*tchar`. -/
def isToken (bytes : List UInt8) : Bool :=
  !bytes.isEmpty && bytes.all isTchar

/-- A byte admitted inside a reason phrase or a field value: `HTAB`, `SP`, or a
visible ASCII character.

`obs-text` (`%x80-FF`) is deliberately excluded, which is stricter than
RFC 9110 §5.5 and stated as such: it is opaque data with no defined character
encoding, and a route whose whole evidence is a byte comparison declines to put
bytes on a wire whose meaning it cannot state. -/
def isFieldByte (b : UInt8) : Bool :=
  b.toNat = 9 ∨ (32 ≤ b.toNat ∧ b.toNat ≤ 126)

/-- **The load-bearing exclusion.** Neither framing byte is a field byte, so an
admitted component cannot carry one. -/
theorem no_framing_byte_is_a_field_byte :
    isFieldByte 13 = false ∧ isFieldByte 10 = false := by
  constructor <;> rfl

/-- Nor is either a `tchar`, so a method or a field name cannot carry one
either. -/
theorem no_framing_byte_is_a_tchar :
    isTchar 13 = false ∧ isTchar 10 = false := by
  constructor <;> rfl

/-- A whole component is admitted only when **every** byte of it is. -/
def isFieldContent (bytes : List UInt8) : Bool := bytes.all isFieldByte

/-- **The injection wall, as a theorem.** An admitted component contains no
framing byte, so the CRLFs a head carries are exactly the ones the builder
wrote. This is the statement header injection is impossible *by construction*
rather than by input sanitation. -/
theorem an_admitted_component_carries_no_framing_byte
    (bytes : List UInt8) (admitted : isFieldContent bytes = true) :
    13 ∉ bytes ∧ 10 ∉ bytes := by
  simp only [isFieldContent, List.all_eq_true] at admitted
  constructor <;> intro mem <;>
    · have := admitted _ mem
      simp [isFieldByte] at this

/-- The same for a token component. -/
theorem an_admitted_token_carries_no_framing_byte
    (bytes : List UInt8) (admitted : isToken bytes = true) :
    13 ∉ bytes ∧ 10 ∉ bytes := by
  simp only [isToken, Bool.and_eq_true, List.all_eq_true] at admitted
  constructor <;> intro mem <;>
    · have := admitted.2 _ mem
      simp [isTchar] at this

/-- One field line: `field-name ":" SP field-value CRLF`. -/
def fieldLine (f : Field) : List UInt8 :=
  f.name.toUTF8.toList ++ [58, 32] ++ f.value.toUTF8.toList ++ crlf

/-- The start line's bytes, without its terminating CRLF. -/
def startText : Start → String
  | .request m t v => m ++ " " ++ t ++ " " ++ v
  | .response v s r => v ++ " " ++ toString s ++ " " ++ r

/-- The message head: start line, each field line, and the empty line that ends
it (RFC 9112 §2.1).

**The framing is written here and nowhere else.** -/
def headBytes (m : Message) : List UInt8 :=
  (startText m.start).toUTF8.toList ++ crlf ++ (m.fields.flatMap fieldLine) ++ crlf

/-- The whole message: head then content. -/
def wireBytes (m : Message) : List UInt8 :=
  headBytes m ++ (m.content.getD [])

/-- **RFC 9112 §2.1 as an equation.** -/
theorem the_wire_is_the_head_then_the_content (m : Message) :
    wireBytes m = headBytes m ++ m.content.getD [] := rfl

/-- A message with no content is exactly its head. -/
theorem a_message_with_no_content_is_its_head (m : Message) (empty : m.content = none) :
    wireBytes m = headBytes m := by
  simp [wireBytes, empty]

/-- **The framing, decomposed.** A head is a start line terminated by a CRLF,
then the field lines, then the CRLF that ends it. -/
theorem the_head_is_the_start_line_then_the_fields_then_the_empty_line (m : Message) :
    headBytes m =
      ((startText m.start).toUTF8.toList ++ crlf) ++ (m.fields.flatMap fieldLine) ++ crlf := by
  simp [headBytes, List.append_assoc]

/-- Every field line ends in a CRLF: the builder writes one, and the components
cannot contain one. -/
theorem every_field_line_ends_in_crlf (f : Field) :
    ∃ prefixBytes, fieldLine f = prefixBytes ++ crlf :=
  ⟨f.name.toUTF8.toList ++ [58, 32] ++ f.value.toUTF8.toList, by simp [fieldLine]⟩

/-- The field lines of a head are either absent or CRLF-terminated as a whole,
which is the step the empty-line theorem below needs. -/
theorem the_field_lines_are_crlf_terminated (fields : List Field) :
    fields = [] ∨ ∃ prefixBytes, fields.flatMap fieldLine = prefixBytes ++ crlf := by
  induction fields with
  | nil => exact Or.inl rfl
  | cons f rest ih =>
    refine Or.inr ?_
    rcases ih with empty | ⟨p, hp⟩
    · exact ⟨f.name.toUTF8.toList ++ [58, 32] ++ f.value.toUTF8.toList,
        by simp [empty, fieldLine]⟩
    · exact ⟨fieldLine f ++ p, by simp [hp, List.append_assoc]⟩

/-- **Every head ends in an empty line**, whatever it carries: the builder's last
act is the CRLF that terminates the head, and it follows the CRLF that ended the
previous line — the start line's when there are no fields, the last field's
otherwise. -/
theorem the_head_ends_in_an_empty_line (m : Message) :
    ∃ prefixBytes, headBytes m = prefixBytes ++ crlf ++ crlf := by
  rcases the_field_lines_are_crlf_terminated m.fields with empty | ⟨p, hp⟩
  · exact ⟨(startText m.start).toUTF8.toList, by simp [headBytes, empty]⟩
  · exact ⟨(startText m.start).toUTF8.toList ++ crlf ++ p,
      by simp [headBytes, hp, List.append_assoc]⟩

/-- The framing rule of RFC 9112 §6, decided over a message whose components are
all literals.

`some n` is a message that declares `Content-Length: n` and carries exactly `n`
bytes of content; `none` is a message that declares no length and carries no
content; anything else is refused. -/
def admitFraming (m : Message) (declared : Option Nat) : Option (Option Nat) :=
  match declared, m.content with
  | some n, some bytes => if n = bytes.length then some (some n) else none
  | some n, none => if n = 0 then some (some n) else none
  | none, none => some none
  | none, some _ => none

/-- **The card's signature theorem.** An admitted declaration *is* the content's
byte length. A literal's byte count is known at check time, so this equality is
decided by the compiler rather than trusted. -/
theorem a_declared_length_is_the_content_length
    (m : Message) (n : Nat) (bytes : List UInt8)
    (hasContent : m.content = some bytes)
    (admitted : admitFraming m (some n) = some (some n)) :
    n = bytes.length := by
  simp only [admitFraming, hasContent] at admitted
  split at admitted
  · assumption
  · exact absurd admitted (by simp)

/-- And the converse, which is what makes the check a *gate* rather than a
description: a declaration that is not the content's length is refused. -/
theorem a_length_that_is_not_the_content_length_is_refused
    (m : Message) (n : Nat) (bytes : List UInt8)
    (hasContent : m.content = some bytes) (differs : n ≠ bytes.length) :
    admitFraming m (some n) = none := by
  simp only [admitFraming, hasContent]
  simp [differs]

/-- Content with no declared length is refused: RFC 9112 §6.3 would then frame it
by closing the connection, which this route does not model. -/
theorem content_without_a_declared_length_is_refused
    (m : Message) (bytes : List UInt8) (hasContent : m.content = some bytes) :
    admitFraming m none = none := by
  simp [admitFraming, hasContent]

/-- A message with no content and no declared length is admitted, framed as
having none (RFC 9112 §6.3 rule 6). -/
theorem no_content_and_no_length_is_admitted
    (m : Message) (empty : m.content = none) :
    admitFraming m none = some none := by
  simp [admitFraming, empty]

/-- A declared length on a message with no content must be zero. -/
theorem a_length_on_an_empty_message_must_be_zero
    (m : Message) (n : Nat) (empty : m.content = none) (nonzero : n ≠ 0) :
    admitFraming m (some n) = none := by
  simp only [admitFraming, empty]
  simp [nonzero]

/-- The start line's own admission: the version is exactly `HTTP/1.1`, a method
is a token, and a status code is three digits in the registered range. -/
def admitStart : Start → Bool
  | .request m t v =>
    v == version && isToken m.toUTF8.toList && isFieldContent t.toUTF8.toList
      && !t.isEmpty
  | .response v s r => v == version && 100 ≤ s && s ≤ 599 && isFieldContent r.toUTF8.toList

/-- **The version is checked, never supplied.** A start line naming any other
version is refused, which is what makes an admitted program's protocol visible in
its own source. -/
theorem the_version_is_exactly_http_1_1 (v : String) (other : v ≠ version) :
    admitStart (.response v 200 "OK") = false := by
  simp [admitStart, other]

theorem a_status_outside_the_registered_range_is_refused (s : Nat) (low : s < 100) :
    admitStart (.response version s "OK") = false := by
  simp [admitStart]
  omega

/-- One header field's admission. -/
def admitField (f : Field) : Bool :=
  isToken f.name.toUTF8.toList && isFieldContent f.value.toUTF8.toList

/-- **The whole injection argument, assembled.** A message whose field is
admitted carries no framing byte in either half of that field, so the two CRLFs
that bracket it in the head are the builder's own. That, together with the
decomposition above, is the statement "every CRLF in an admitted head is one this
module wrote" — which is why header injection is not expressible on this route
rather than merely not intended. -/
theorem an_admitted_field_carries_no_framing_byte (f : Field)
    (admitted : admitField f = true) :
    (13 ∉ f.name.toUTF8.toList ∧ 10 ∉ f.name.toUTF8.toList)
      ∧ (13 ∉ f.value.toUTF8.toList ∧ 10 ∉ f.value.toUTF8.toList) := by
  simp only [admitField, Bool.and_eq_true] at admitted
  exact ⟨an_admitted_token_carries_no_framing_byte _ admitted.1,
    an_admitted_component_carries_no_framing_byte _ admitted.2⟩

/-- A head with no fields is exactly the start line and the empty line that ends
it: two CRLFs and nothing between them. -/
theorem a_fieldless_head_is_the_start_line_and_an_empty_line
    (s : Start) (c : Option (List UInt8)) :
    headBytes ⟨s, [], c⟩ = (startText s).toUTF8.toList ++ crlf ++ crlf := by
  simp [headBytes]

/-- And one more field adds exactly one more line. Stated as an equation so that
a change to what a head carries changes this theorem rather than slipping past
it. -/
theorem one_more_field_adds_exactly_one_line
    (s : Start) (f : Field) (fs : List Field) (c : Option (List UInt8)) :
    headBytes ⟨s, f :: fs, c⟩ =
      (startText s).toUTF8.toList ++ crlf ++ fieldLine f
        ++ (fs.flatMap fieldLine) ++ crlf := by
  simp [headBytes, List.append_assoc]

/-- The module gate, proved against the same registry and by the same generic
argument every other bundled module is. -/
theorem the_http_capability_needs_its_module
    {names : List String} {imports : QCore.ModuleImports.Imports}
    (resolved : QCore.ModuleImports.resolve names = some imports)
    (enabled : imports.http = true) : "std.http" ∈ names :=
  QCore.ModuleImports.admitted_http_requires_its_module resolved enabled

/-- **The composition rule this card had to argue, restated at the route.** The
HTTP module opens no socket and constructs no string: a program that speaks HTTP
names its transport too, and a program that binds a payload names the text module
too. -/
theorem the_http_module_grants_neither_transport_nor_text :
    QCore.ModuleImports.admitsSocket
        ⟨false, false, false, false, false, false, false, false, false, true⟩ true = false
      ∧ QCore.ModuleImports.admitsString
        ⟨false, false, false, false, false, false, false, false, false, true⟩ true = false :=
  ⟨rfl, rfl⟩

/-- **The effect discipline.** The HTTP vocabulary derives nothing: declaring a
message, adding a field and setting content compute a byte sequence at check
time — they allocate nothing, mutate nothing and write nothing. Every effect an
admitted program declares is the transport's or the output module's.

Stated as a function over the whole vocabulary so a widening changes a theorem
rather than slipping past one. -/
inductive Statement where
  | declare (binding : String) (start : Start)
  | header (message : String) (field : Field)
  | content (message : String) (bytes : List UInt8)
  deriving Repr

/-- The effects one HTTP statement derives: none, on every constructor. -/
def derivedEffects : Statement → List String
  | .declare _ _ => []
  | .header _ _ => []
  | .content _ _ => []

theorem no_http_statement_derives_an_effect (s : Statement) : derivedEffects s = [] := by
  cases s <;> rfl

/-- The vocabulary is exactly three statements, stated as one list so that a
fourth changes this theorem. -/
theorem the_statement_forms_are_exactly_three
    (s : Statement) :
    (∃ b st, s = .declare b st) ∨ (∃ m f, s = .header m f) ∨ (∃ m b, s = .content m b) := by
  cases s with
  | declare b st => exact Or.inl ⟨b, st, rfl⟩
  | header m f => exact Or.inr (Or.inl ⟨m, f, rfl⟩)
  | content m b => exact Or.inr (Or.inr ⟨m, b, rfl⟩)

end QCore.StdHttp
