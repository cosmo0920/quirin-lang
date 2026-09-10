import QCore.ModuleImports

/-! `source.std-io.v1` — the effect discipline of `std.fmt`, `std.io` and
`std.logger` (card `STD-MODULES`).

The claim proved here is the one the Rust route relies on: **a rendering cannot
touch custody.** It is proved structurally, not by inspecting a checker — the
operand type has no resource constructor, so a program on this route has no
resource occurrence to consume, release, retain or leak.

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

/-- The `std.logger` severity. Spelled `fatal` rather than `error`, because
`error` is the reserved error-type declaration keyword. A level is a label: it
selects no stream and filters nothing. -/
inductive Level where
  | info
  | warn
  | fatal
  deriving DecidableEq, Repr

inductive Stream where
  | out
  | err
  deriving DecidableEq, Repr

/-- **The effect discipline, as a type.** Two constructors, neither of which can
denote a resource. Every theorem below is downstream of this. -/
inductive Operand where
  | literal (value : Int)
  | scalar (name : String)
  deriving DecidableEq, Repr

inductive Event where
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
    linesOn Stream.err (Event.log Level.info r o) = 1
      ∧ linesOn Stream.err (Event.log Level.warn r o) = 1
      ∧ linesOn Stream.err (Event.log Level.fatal r o) = 1 :=
  ⟨rfl, rfl, rfl⟩

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

/-- The three capabilities cannot be obtained from each other, or from the two
older modules: each row holds every module except the one it needs. -/
theorem no_module_substitutes_for_another :
    admitsPrint ⟨true, true, true, false, true⟩ true = false
      ∧ admitsLog ⟨true, true, true, true, false⟩ true = false
      ∧ admitsRendering ⟨true, true, false, true, true⟩ true = false :=
  ⟨rfl, rfl, rfl⟩

end QCore.StdIo
