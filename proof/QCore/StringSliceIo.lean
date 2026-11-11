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

/-! Rendering text: the composition of `std.string` with the output modules
(card `STRING-SLICE-IO`, amendment §7 of `docs/design/STRING_SLICE_V1.md`).

Amendment §7 stated the shape of this card before it existed: "Output modules
may render a string only after their own capability and observation rules are
extended". Three claims are proved here, and they are the three the Rust
composition rests on.

**The composition is neutral on the route it composes.** A text line *erases* to
the base route's own line, and the erasure changes nothing: the derived effect
row of a composed program is the row of its erasure, so every theorem
`QCore.StdString` proves about a program is a theorem about the same program
with rendered text in it. This is the shape `QCore.ComposeEventLines` used for
the same purpose and is reused for the same reason.

**The rendering is the identity on bytes.** What reaches the stream is the
view's own payload, framed by the sink's line discipline and by nothing else: no
re-encoding, no escaping, no terminator and no newline beyond the framing's. The
statement that makes this more than a definition is
`a_terminator_scan_would_write_fewer_bytes`: on a payload the route admits, a
scan-to-terminator write and this model **disagree**, which is exactly why the
emitted C writes with a length and why `%s` and `%.*s` appear in no emitted
line.

**One framing, two line kinds.** The stream, the `[LEVEL] ` prefix and the
single trailing newline are one function of the level, applied to a body that is
either a rendered number or a view's bytes — `the_two_line_kinds_share_one_framing`
is `rfl`, because there is one framing and not two that agree.

What is **not** proved here: that the Rust lowering extracts these events
faithfully from source tokens, that `fwrite` writes what C says it writes, or
anything about a view of storage that can end — v1 has none. -/

namespace QCore.StringSliceIo

open QCore.StdString

/-! ## The sink vocabulary

`std.logger`'s ordered levels and the two streams, modelled only as far as the
framing reads them. -/

inductive Level where
  | trace | debug | info | warn | error | fatal
  deriving DecidableEq, Repr

inductive Stream where
  | out | err
  deriving DecidableEq, Repr

/-- The ASCII bytes of a level's bracketed tag, followed by one space: the
prefix a `log` line carries and a `print` line does not. Written as bytes
because the whole subject of this card is which bytes reach a stream. -/
def Level.tagBytes : Level → Payload
  | .trace => [91, 84, 82, 65, 67, 69, 93, 32]
  | .debug => [91, 68, 69, 66, 85, 71, 93, 32]
  | .info => [91, 73, 78, 70, 79, 93, 32]
  | .warn => [91, 87, 65, 82, 78, 93, 32]
  | .error => [91, 69, 82, 82, 79, 82, 93, 32]
  | .fatal => [91, 70, 65, 84, 65, 76, 93, 32]

/-- The one newline byte a line's framing appends. -/
def newline : Nat := 10

/-- The stream a line goes to. `none` is `print`, `some level` is `log LEVEL` —
the same discriminant the Rust `line_framing` reads. -/
def streamOf : Option Level → Stream
  | Option.none => Stream.out
  | some _ => Stream.err

/-- The bytes a line's framing writes before its body. -/
def prefixBytes : Option Level → Payload
  | Option.none => []
  | some level => level.tagBytes

/-! ## What a line's body is

Two constructors, one framing. A scalar body is the digits `std.fmt` produced; a
text body is a view's own bytes. Nothing else can be a body — in particular a
custody identity cannot, because a view publishes none. -/

inductive Body where
  /-- The bytes of a rendered `i32`, as `dec` or `hex` produced them. -/
  | scalar (digits : Payload)
  /-- A view's own bytes, unchanged. -/
  | text (bytes : Payload)
  deriving DecidableEq, Repr

def bodyBytes : Body → Payload
  | .scalar digits => digits
  | .text bytes => bytes

/-- **The framing, once.** The stream, the prefix and the single trailing
newline, applied to whatever the body is. -/
def written (level : Option Level) (body : Body) : Stream × Payload :=
  (streamOf level, prefixBytes level ++ bodyBytes body ++ [newline])

/-! ## The composed program

A text operand, the composed event type, and the erasure back to the base
route's program. -/

/-- What a rendered text operand may be. Three forms, each denoting a view of
this program's own static storage; none of them can denote a scalar, and none
can denote a custody identity. -/
inductive TextOperand where
  | literal (storage : Nat)
  | name (binding : String)
  | slice (binding : String) (low high : Operand)
  deriving DecidableEq, Repr

/-- The events of a composed entry: the base route's own, plus the rendered text
line this card adds. -/
inductive Event where
  | string (event : QCore.StdString.Event)
  | text (level : Option Level) (operand : TextOperand)
  deriving DecidableEq, Repr

/-- **The erasure.** A rendered text line erases to the base route's line, which
is the constructor `QCore.StdString` already reasons about; every other event is
its own. -/
def erase : Event → QCore.StdString.Event
  | .string event => event
  | .text _ _ => .line

def eraseProgram (events : List Event) : List QCore.StdString.Event :=
  events.map erase

/-- The composed effect row, derived directly rather than through the erasure,
so the neutrality statement below is an equality between two computations rather
than a definition. -/
def composedEffects : List Event → List Effect
  | [] => []
  | .string event :: rest => eventEffects event ++ composedEffects rest
  | .text _ _ :: rest => Effect.io :: composedEffects rest

/-! ## Neutrality: the composition moves nothing on the route it composes -/

/-- **The headline.** The effect row of a composed program is the row of its
erasure. A rendered text line derives `io` and nothing else — no `alloc`, no
`mut` — which is amendment §7 for the composed route, and it means every effect
theorem of `QCore.StdString` is a theorem about a program with text in it. -/
theorem erasing_the_text_lines_leaves_the_effect_row (events : List Event) :
    composedEffects events = derivedEffects (eraseProgram events) := by
  induction events with
  | nil => rfl
  | cons event rest ih =>
    cases event with
    | string e => simp [composedEffects, eraseProgram, derivedEffects, erase, ih,
        List.flatMap_cons]
    | text level operand =>
      simp [composedEffects, eraseProgram, derivedEffects, erase, ih, eventEffects,
        List.flatMap_cons]

theorem a_text_line_derives_io (level : Option Level) (operand : TextOperand) :
    composedEffects [.text level operand] = [Effect.io] := rfl

theorem a_text_line_derives_no_alloc (level : Option Level) (operand : TextOperand) :
    Effect.alloc ∉ composedEffects [.text level operand] := by
  simp [composedEffects]

theorem a_text_line_derives_no_mut (level : Option Level) (operand : TextOperand) :
    Effect.mut ∉ composedEffects [.text level operand] := by
  simp [composedEffects]

/-- A composed program with no text in it **is** its erasure, so a module that
renders no text travels the base route's own reasoning unchanged. That is the
model's form of the measured statement that such a program's emitted C is
byte-identical to what it was before this card. -/
theorem a_program_with_no_text_is_its_own_erasure (events : List QCore.StdString.Event) :
    eraseProgram (events.map Event.string) = events := by
  induction events with
  | nil => rfl
  | cons event rest ih => simp [eraseProgram, erase] at ih ⊢; exact ih

/-! ## The rendering rule: the bytes of the view, and nothing else -/

/-- The body of a text line is the view's own bytes. Stated as a definition and
then as a theorem, because "the identity function on bytes" is exactly the
rendering rule this card decided. -/
def textBody (storage : Storage) (view : View) : Body :=
  .text (bytesOf storage view)

theorem the_rendered_body_is_the_views_own_bytes (storage : Storage) (view : View) :
    bodyBytes (textBody storage view) = bytesOf storage view := rfl

/-- **One framing, two line kinds.** A rendered scalar line and a rendered text
line with the same body bytes are the same written line: same stream, same
prefix, same single newline. It is `rfl` because there is one framing rather
than two implementations that agree — the property the Rust side pins as
`a_rendered_scalar_line_is_a_text_line_of_its_digits`. -/
theorem the_two_line_kinds_share_one_framing (level : Option Level) (bytes : Payload) :
    written level (.scalar bytes) = written level (.text bytes) := rfl

/-- A `print` line goes to standard output with no prefix; a `log LEVEL` line
goes to the logger's stream with its bracketed tag. -/
theorem a_print_line_is_unprefixed_on_standard_output (bytes : Payload) :
    written Option.none (.text bytes) = (Stream.out, bytes ++ [newline]) := by
  simp [written, streamOf, prefixBytes, bodyBytes]

theorem a_logged_line_carries_its_level_tag (level : Level) (bytes : Payload) :
    written (some level) (.text bytes)
      = (Stream.err, level.tagBytes ++ bytes ++ [newline]) := rfl

/-- The framing appends **exactly one** newline, whatever the body contains. A
body that itself contains a newline byte therefore writes more than one line's
worth of bytes, which is recorded rather than forbidden: the view's bytes are
the view's bytes. -/
theorem the_framing_appends_exactly_one_newline (level : Option Level) (bytes : Payload)
    (framing : (prefixBytes level).count newline = 0) :
    (written level (.text bytes)).2.count newline = bytes.count newline + 1 := by
  simp [written, bodyBytes, List.count_append, framing]

theorem no_level_tag_contains_a_newline (level : Level) :
    (prefixBytes (some level)).count newline = 0 := by
  cases level <;> rfl

theorem the_print_framing_contains_no_newline :
    (prefixBytes Option.none).count newline = 0 := rfl

/-- The empty view writes the framing and nothing else: one newline on standard
output, rather than nothing at all. An empty view is a value (§4) and is never
dereferenced, and the emitted write is a zero-length one. -/
theorem the_empty_view_writes_only_the_framing :
    written Option.none (.text []) = (Stream.out, [newline]) := rfl

/-! ## The NUL decision, stated as a disagreement

This is the section that makes the rendering rule load-bearing rather than
obvious. -/

/-- Every byte of the view is written, `0` included: the body's length is the
view's length, never the distance to a terminator. -/
theorem every_byte_of_the_view_is_written (bytes : Payload) :
    (bodyBytes (.text bytes)).length = bytes.length := rfl

/-- **The headline about NUL.** On `"ab\0cd"` — a payload this route admits, and
the one `QCore.StdString.an_embedded_nul_is_inside_the_length` is stated about —
a scan-to-terminator write and this model disagree: the scan writes two bytes
and the model writes five. That disagreement is why the emitted C writes with a
length rather than through any `printf` conversion, every one of which stops at
the first `0`. -/
theorem a_terminator_scan_would_write_fewer_bytes :
    ([97, 98, 0, 99, 100] : Payload).takeWhile (fun byte => byte != 0) = [97, 98]
      ∧ (bodyBytes (.text [97, 98, 0, 99, 100])).length = 5 := by
  exact ⟨rfl, rfl⟩

/-- And the whole written line carries the NUL as an ordinary byte, inside a
line the framing closes with one newline. -/
theorem an_embedded_nul_reaches_the_stream :
    written (some Level.warn) (.text [97, 98, 0, 99, 100])
      = (Stream.err, Level.warn.tagBytes ++ [97, 98, 0, 99, 100] ++ [newline]) := rfl

/-! ## The operand rule: one decision for a slice, wherever it is written -/

/-- Whether a text operand denotes a view this entry can render. The slice arm
calls `QCore.StdString.admitsSlice` — the **same** predicate a bound slice is
decided by — which is what makes the theorem below an equality rather than a
comparison of two rules. -/
def admitsText (storage : Storage) (scalars : Scalars) (views : Views) :
    TextOperand → Bool
  | .literal at_ => (storage[at_]?).isSome
  | .name binding => (lookupView views binding).isSome
  | .slice binding low high =>
      match lookupView views binding with
      | Option.none => false
      | some view => admitsSlice storage scalars view low high

/-- **An inline slice is admitted exactly when the bound one is.** The rendering
position adds no admission and removes none: a slice written at an operand and
the same slice written as a `let` are one decision. -/
theorem an_inline_slice_is_admitted_exactly_when_the_bound_one_is
    (storage : Storage) (scalars : Scalars) (views : Views) (binding : String)
    (view : View) (low high : Operand) (found : lookupView views binding = some view) :
    admitsText storage scalars views (.slice binding low high)
      = admitsSlice storage scalars view low high := by
  simp [admitsText, found]

/-- **Fail-closed on a name this entry bound no string for.** A bare rendering
operand renders a string and only a string: a name with no view refuses, and so
does a slice of one. Nothing falls back to rendering it as a number. -/
theorem a_name_with_no_view_refuses
    (storage : Storage) (scalars : Scalars) (views : Views) (binding : String)
    (unbound : lookupView views binding = Option.none) (low high : Operand) :
    admitsText storage scalars views (.name binding) = false
      ∧ admitsText storage scalars views (.slice binding low high) = false := by
  constructor <;> simp [admitsText, unbound]

/-- **The operand wall.** There are exactly three text-operand forms and every
one of them denotes a view. Written as one list on purpose — the convention
`QCore.StdString.the_bound_forms_are_exactly_two` established — so a widening
changes this theorem rather than slipping past it. In particular there is no
constructor carrying a custody identity: a view owns nothing and publishes none,
which is why `borrow NAME` has no meaning at this position. -/
theorem the_text_operand_forms_are_exactly_three (operand : TextOperand) :
    (∃ at_, operand = .literal at_)
      ∨ (∃ binding, operand = .name binding)
      ∨ (∃ binding low high, operand = .slice binding low high) := by
  cases operand with
  | literal at_ => exact Or.inl ⟨at_, rfl⟩
  | name binding => exact Or.inr (Or.inl ⟨binding, rfl⟩)
  | slice binding low high => exact Or.inr (Or.inr ⟨binding, low, high, rfl⟩)

/-! ## The selector decision, and the module gate it decides -/

/-- Whether a line names a `std.fmt` rendering. A scalar body is the output of
one; a text body is the view's own bytes and names none, because `dec` and `hex`
are total functions from an `i32` to digits and a `string` is already the bytes.
-/
def namesARendering : Body → Bool
  | .scalar _ => true
  | .text _ => false

theorem a_text_line_names_no_rendering (bytes : Payload) :
    namesARendering (.text bytes) = false := rfl

/-- **A module that renders only text needs no formatting module**, whatever it
imports — the same measurement card `STD-MODULES` made for `flush`, applied to
the operand rather than to the statement head. -/
theorem a_text_line_needs_no_formatting_module
    (imports : QCore.ModuleImports.Imports) (bytes : Payload) :
    QCore.ModuleImports.admitsRendering imports (namesARendering (.text bytes)) = true := rfl

/-- The other direction, so the narrowing is visibly not a weakening: a rendered
scalar still needs `std.fmt`, and a module holding `std.io` and `std.string`
without it is refused. -/
theorem a_scalar_line_still_needs_the_formatting_module (digits : Payload) :
    QCore.ModuleImports.admitsRendering
      ⟨false, false, false, false, true, false, false, true, false, false⟩
      (namesARendering (.scalar digits)) = false := rfl

/-- **A rendered string needs both modules, and neither grants the other.**
Writing text to standard output is a `std.io` capability and constructing the
string is a `std.string` one; a file that does both names both. -/
theorem a_rendered_string_needs_both_modules {names : List String}
    {imports : QCore.ModuleImports.Imports}
    (resolved : QCore.ModuleImports.resolve names = some imports)
    (strings : imports.string = true) (output : imports.io = true) :
    "std.string" ∈ names ∧ "std.io" ∈ names :=
  ⟨QCore.ModuleImports.admitted_string_requires_its_module resolved strings,
    QCore.ModuleImports.admitted_output_requires_the_io_import resolved output⟩

/-- A logged string needs `std.logger` beside `std.string`, on the same terms.
-/
theorem a_logged_string_needs_both_modules {names : List String}
    {imports : QCore.ModuleImports.Imports}
    (resolved : QCore.ModuleImports.resolve names = some imports)
    (strings : imports.string = true) (logging : imports.logger = true) :
    "std.string" ∈ names ∧ "std.logger" ∈ names :=
  ⟨QCore.ModuleImports.admitted_string_requires_its_module resolved strings,
    QCore.ModuleImports.admitted_logging_requires_the_logger_import resolved logging⟩

/-- And the independence survives the composition: holding both output modules
still grants no string capability, so this card did not make `std.io` accept
text — it made a module that names **both** able to. That is amendment §7's
sentence, as a registry fact. -/
theorem the_output_imports_alone_still_grant_no_string :
    QCore.ModuleImports.admitsString
      ⟨false, false, false, true, true, true, false, false, false, false⟩ true = false := rfl

theorem the_string_import_alone_still_grants_no_stream :
    QCore.ModuleImports.admitsPrint
      ⟨false, false, false, false, false, false, false, true, false, false⟩ true = false
      ∧ QCore.ModuleImports.admitsLog
        ⟨false, false, false, false, false, false, false, true, false, false⟩ true = false :=
  ⟨rfl, rfl⟩

end QCore.StringSliceIo
