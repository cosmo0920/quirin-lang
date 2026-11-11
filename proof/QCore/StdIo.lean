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

/-! `source.std-io.v1` — the effect discipline of `std.fmt`, `std.io` and
`std.logger` (card `STD-MODULES`).

The claim proved here is the one the Rust route relies on: **a rendering cannot
touch custody.** It is proved structurally, not by inspecting a checker — the
operand type's two bare constructors have no resource occurrence to consume,
release, retain or leak.

Card `STD-FMT-BORROW` adds the third, `borrow NAME`, and it changes none of
that. A borrowed render resolves against a *separate* environment of identities
a checked custody model published, and this route publishes none, so every
borrowed render written here refuses. The theorems below state both halves: the
refusal, and the wall — neither operand form can reach the other's environment,
in either direction. Whether a supplied identity is *live* is not this module's
question; `QCore.ComposedStdIo` proves that where the ledger is.

What is **not** proved here: that the Rust lowering extracts these events
faithfully from source tokens, that the C backend's `printf` conversions render
what the Rust `Rendering::render` renders (that is compared, per stream, by the
Rust tests), or anything at all about durability, buffering or stream
interleaving. `flush` is modeled as a barrier that produces no line; sink
handoff is a stated boundary, not a theorem. -/

namespace QCore.StdIo

/-- The `std.fmt` vocabulary. Both forms are total, so a rendering has no error
exit and no failure path to account for. -/
inductive Rendering where
  | dec
  | hex
  deriving DecidableEq, Repr

/-- The complete ordered `std.logger` severity vocabulary. `error` remains the
error-type declaration keyword globally and is recognized contextually after
`log`. A source `log` level labels a line and filters nothing; `log_level` is a
separate system-trace configuration event. -/
inductive Level where
  | trace
  | debug
  | info
  | warn
  | error
  | fatal
  deriving DecidableEq, Repr

inductive Stream where
  | out
  | err
  deriving DecidableEq, Repr

/-- Source-authored I/O as two independently ordered projections. There is no
cross-stream ordering field to accidentally claim. -/
structure Streams where
  out : List String
  err : List String
  deriving DecidableEq, Repr

def Streams.empty : Streams := ⟨[], []⟩

/-- Pointwise ordered concatenation, matching Rust `Streams::compose`. -/
def Streams.compose (left right : Streams) : Streams :=
  ⟨left.out ++ right.out, left.err ++ right.err⟩

theorem streams_left_identity (streams : Streams) :
    Streams.empty.compose streams = streams := by
  cases streams
  rfl

theorem streams_right_identity (streams : Streams) :
    streams.compose Streams.empty = streams := by
  cases streams
  simp [Streams.compose, Streams.empty]

theorem streams_compose_associative (a b c : Streams) :
    (a.compose b).compose c = a.compose (b.compose c) := by
  cases a
  cases b
  cases c
  simp [Streams.compose, List.append_assoc]

/-- **The effect discipline, as a type.** The two bare constructors cannot
denote a resource at all; the third denotes one only through an identity a
checked custody model published for it, and renders that identity and nothing
else. Every theorem below is downstream of this. -/
inductive Operand where
  | literal (value : Int)
  | scalar (name : String)
  /-- `borrow NAME` (card `STD-FMT-BORROW`). Resolved against the identities a
  custody model supplied, which on this route is the empty list. -/
  | borrowed (name : String)
  deriving DecidableEq, Repr

inductive Event where
  /-- Entry-level system-trace threshold. It carries `io` but contributes no
  source-authored line to either stream. -/
  | logLevel (l : Level)
  | print (r : Rendering) (o : Operand)
  | log (l : Level) (r : Rendering) (o : Operand)
  | flush
  | ret
  deriving DecidableEq, Repr

/-- The qualitative effects this model distinguishes, matching the Rust `Effect`
row on the three constructors this route could conceivably produce. -/
inductive Effect where
  | io
  | alloc
  | mut
  deriving DecidableEq, Repr

/-- Whether an event writes anything at all. -/
def writes : Event → Bool
  | .logLevel _ => true
  | .print _ _ => true
  | .log _ _ _ => true
  | .flush => true
  | .ret => false

/-- The derived effect row of a program, as a list without duplicates. -/
def derivedEffects (events : List Event) : List Effect :=
  if events.any writes then [Effect.io] else []

/-- How many lines an event contributes to a stream. `flush` contributes none:
it is a barrier, not a record. -/
def linesOn (s : Stream) : Event → Nat
  | .logLevel _ => 0
  | .print _ _ => if s = Stream.out then 1 else 0
  | .log _ _ _ => if s = Stream.err then 1 else 0
  | .flush => 0
  | .ret => 0

def lineCount (s : Stream) (events : List Event) : Nat :=
  (events.map (linesOn s)).sum

/-! ## The effect discipline -/

/-- **No program on this route can allocate.** -/
theorem no_program_allocates (events : List Event) :
    Effect.alloc ∉ derivedEffects events := by
  unfold derivedEffects
  split <;> simp

/-- **No program on this route can mutate.** -/
theorem no_program_mutates (events : List Event) :
    Effect.mut ∉ derivedEffects events := by
  unfold derivedEffects
  split <;> simp

/-- The derived row is `{Io}` or empty — never anything else. -/
theorem derived_row_is_io_or_empty (events : List Event) :
    derivedEffects events = [Effect.io] ∨ derivedEffects events = [] := by
  unfold derivedEffects
  split
  · exact Or.inl rfl
  · exact Or.inr rfl

/-- A program that writes derives exactly `{Io}`. -/
theorem writing_derives_io {events : List Event} (h : events.any writes = true) :
    derivedEffects events = [Effect.io] := by
  simp [derivedEffects, h]

/-- A program that writes nothing derives nothing: importing the modules is not
using them, so a file that renders nothing carries no IO effect. -/
theorem silent_program_derives_nothing {events : List Event}
    (h : events.any writes = false) : derivedEffects events = [] := by
  simp [derivedEffects, h]

/-- A bare `return` is silent. -/
theorem a_bare_return_is_silent : derivedEffects [Event.ret] = [] := rfl

/-- Selecting a system-trace adapter is an explicit IO effect even though the
configuration record itself contributes no source-authored line. -/
theorem log_level_carries_io_without_a_source_line (level : Level) :
    derivedEffects [Event.logLevel level, Event.ret] = [Effect.io]
      ∧ lineCount Stream.out [Event.logLevel level, Event.ret] = 0
      ∧ lineCount Stream.err [Event.logLevel level, Event.ret] = 0 := by
  cases level <;> decide

/-! ## Stream separation

The two streams are counted separately and nothing relates their orders. These
theorems record that separation as the model's actual content, so a later card
cannot quietly start claiming an interleaving. -/

/-- `print` never writes to the logger's stream. -/
theorem print_never_writes_stderr (r : Rendering) (o : Operand) :
    linesOn Stream.err (Event.print r o) = 0 := rfl

/-- `log` never writes to standard output, at any level. -/
theorem log_never_writes_stdout (l : Level) (r : Rendering) (o : Operand) :
    linesOn Stream.out (Event.log l r o) = 0 := rfl

/-- Every level writes to the same stream: a level is a label, not a selector. -/
theorem every_level_uses_one_stream (r : Rendering) (o : Operand) :
    linesOn Stream.err (Event.log Level.trace r o) = 1
      ∧ linesOn Stream.err (Event.log Level.debug r o) = 1
      ∧ linesOn Stream.err (Event.log Level.info r o) = 1
      ∧ linesOn Stream.err (Event.log Level.warn r o) = 1
      ∧ linesOn Stream.err (Event.log Level.error r o) = 1
      ∧ linesOn Stream.err (Event.log Level.fatal r o) = 1 :=
  ⟨rfl, rfl, rfl, rfl, rfl, rfl⟩

/-- **The barrier moves no observation.** Inserting `flush` anywhere leaves both
streams' line counts unchanged. This is what makes `flush` comparable against a
native run that produces no extra bytes. -/
theorem flush_changes_no_line_count (s : Stream) (before after : List Event) :
    lineCount s (before ++ Event.flush :: after) = lineCount s (before ++ after) := by
  simp [lineCount, List.map_append, List.sum_append, linesOn]

/-- Repeating the barrier still moves nothing. -/
theorem repeated_flush_changes_no_line_count (s : Stream) (events : List Event) :
    lineCount s (Event.flush :: Event.flush :: events) = lineCount s events := by
  simp [lineCount, linesOn]

/-- A barrier alone writes nothing on either stream, which is exactly what the
logger-only fixture measures natively. -/
theorem a_bare_barrier_writes_nothing :
    lineCount Stream.out [Event.flush, Event.ret] = 0
      ∧ lineCount Stream.err [Event.flush, Event.ret] = 0 := ⟨rfl, rfl⟩

/-- `flush` does write — it is not a no-op in the effect row, only in the line
counts. A barrier still needs the `Io` effect, so it still needs its module. -/
theorem the_barrier_still_carries_io :
    derivedEffects [Event.flush, Event.ret] = [Effect.io] := rfl

/-! ## The borrowed render, and why this route has none (card `STD-FMT-BORROW`)

A rendering resolves its operand against **two** environments, and the point of
them being two is that neither lookup can reach the other's names: a literal
needs neither, a scalar reads only the integers this entry bound, and a borrow
reads only the identities a checked custody model supplied. `source.std-io.v1`
has no custody model, so its identity environment is empty — which is why every
borrowed render written on this route refuses, structurally rather than by a
rule. -/

/-- Look one name up in an association list. -/
def lookup (env : List (String × Int)) (name : String) : Option Int :=
  match env with
  | [] => none
  | (k, v) :: rest => if k = name then some v else lookup rest name

/-- What a rendering publishes, given the two environments, or `none` when it
has no admitted step. -/
def renderOperand (scalars identities : List (String × Int)) : Operand → Option Int
  | .literal value => some value
  | .scalar name => lookup scalars name
  | .borrowed name => lookup identities name

/-- **A route with no custody model renders no borrow.** This is the whole
`source.std-io.v1` half of the card: the refusal is the absence of a model, not
a rule this route states, so it cannot be weakened by editing a checker. -/
theorem a_route_without_a_custody_model_renders_no_borrow
    (scalars : List (String × Int)) (name : String) :
    renderOperand scalars [] (.borrowed name) = none := rfl

/-- **A borrowed render reads no scalar.** Whatever this entry bound with `let`,
the borrowed operand's result is the same — the two namespaces are two lookups
in two environments and nothing crosses. -/
theorem a_borrowed_render_reads_no_scalar
    (s₁ s₂ identities : List (String × Int)) (name : String) :
    renderOperand s₁ identities (.borrowed name)
      = renderOperand s₂ identities (.borrowed name) := rfl

/-- And the wall in the other direction: a scalar render reads no identity, so
supplying identities can never make a bare operand denote a resource. This is
the pin that keeps `Q028`'s resource-free rule for a bare operand unweakened. -/
theorem a_scalar_render_reads_no_identity
    (scalars i₁ i₂ : List (String × Int)) (name : String) :
    renderOperand scalars i₁ (.scalar name) = renderOperand scalars i₂ (.scalar name) := rfl

/-- A literal reads neither environment. -/
theorem a_literal_render_reads_neither_environment
    (s₁ s₂ i₁ i₂ : List (String × Int)) (value : Int) :
    renderOperand s₁ i₁ (.literal value) = renderOperand s₂ i₂ (.literal value) := rfl

/-- **The operand forms are exactly these three**, exhibited as one list so that
a future widening changes a theorem rather than slipping past it. -/
theorem the_operand_forms_are_exactly_these (o : Operand) :
    (∃ value, o = .literal value) ∨ (∃ name, o = .scalar name)
      ∨ (∃ name, o = .borrowed name) := by
  cases o with
  | literal value => exact Or.inl ⟨value, rfl⟩
  | scalar name => exact Or.inr (Or.inl ⟨name, rfl⟩)
  | borrowed name => exact Or.inr (Or.inr ⟨name, rfl⟩)

/-- A borrowed render is still a write, so it still needs `Io` and therefore
still needs its module: the widening buys no capability for free. -/
theorem a_borrowed_render_still_carries_io (r : Rendering) (name : String) :
    derivedEffects [Event.print r (.borrowed name), Event.ret] = [Effect.io] := rfl

/-- And it still allocates and mutates nothing: the derived row of a program of
borrowed renders is the row of any other writing program. -/
theorem a_borrowed_render_allocates_and_mutates_nothing (r : Rendering) (name : String) :
    Effect.alloc ∉ derivedEffects [Event.print r (.borrowed name), Event.ret]
      ∧ Effect.mut ∉ derivedEffects [Event.print r (.borrowed name), Event.ret] :=
  ⟨no_program_allocates _, no_program_mutates _⟩

/-- A borrowed render writes one line on the stream its statement names, exactly
as any other rendering does: the marker changes what is read, never where it is
written. -/
theorem a_borrowed_render_writes_one_line_on_its_own_stream (r : Rendering) (l : Level)
    (name : String) :
    linesOn Stream.out (Event.print r (.borrowed name)) = 1
      ∧ linesOn Stream.err (Event.print r (.borrowed name)) = 0
      ∧ linesOn Stream.err (Event.log l r (.borrowed name)) = 1
      ∧ linesOn Stream.out (Event.log l r (.borrowed name)) = 0 :=
  ⟨rfl, rfl, rfl, rfl⟩

/-! ## Capability gating, composed with the resolver -/

open QCore.ModuleImports

/-- A file that prints must have named `std.io`, whatever else it imported. -/
theorem printing_requires_the_io_module {names : List String} {imports : Imports}
    (resolved : resolve names = some imports)
    (admitted : admitsPrint imports true = true) : "std.io" ∈ names := by
  refine admitted_output_requires_the_io_import resolved ?_
  simpa [admitsPrint] using admitted

/-- A file that logs or flushes must have named `std.logger`. -/
theorem logging_requires_the_logger_module {names : List String} {imports : Imports}
    (resolved : resolve names = some imports)
    (admitted : admitsLog imports true = true) : "std.logger" ∈ names := by
  refine admitted_logging_requires_the_logger_import resolved ?_
  simpa [admitsLog] using admitted

/-- A file that names a rendering must have named `std.fmt`. -/
theorem rendering_requires_the_fmt_module {names : List String} {imports : Imports}
    (resolved : resolve names = some imports)
    (admitted : admitsRendering imports true = true) : "std.fmt" ∈ names := by
  refine admitted_rendering_requires_the_fmt_import resolved ?_
  simpa [admitsRendering] using admitted

/-- The three capabilities cannot be obtained from each other, or from the
older modules — card `UNSAFE-LINKAGE`'s waiver included: each row holds every
module except the one it needs.

Card `STD-BUFFER` grew the registry to seven and card `STRING-SLICE` to eight,
and each row grew with them: the new `std.buffer` and `std.string` columns are
`true` here, so the theorem still says what it says — *every* other module held,
and still refused. Its name and meaning are unchanged and the claim is strictly
stronger than before each widening. -/
theorem no_module_substitutes_for_another :
    admitsPrint ⟨true, true, true, true, false, true, true, true, false, false⟩ true = false
      ∧ admitsLog ⟨true, true, true, true, true, false, true, true, false, false⟩ true = false
      ∧ admitsRendering ⟨true, true, true, false, true, true, true, true, false, false⟩ true = false :=
  ⟨rfl, rfl, rfl⟩

end QCore.StdIo
