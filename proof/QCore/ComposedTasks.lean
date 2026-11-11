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

import QCore.StdIo

/-! `source.std-io-tasks.v1` — the checked composition of `source.std-io.v1`
with the checked task route (card `COMPOSE-CONCURRENCY`).

This is a model of `crates/qcore-core/src/composed_tasks.rs`, and — like that
module — it models the **join**, not a third checker. A composed program is the
parent's own step sequence with `source.std-io.v1` lines standing in it; the
task model's custody rules are not reconstructed here, so nothing here can
weaken one.

What is proved is what the join owns, and one thing more that the Rust side can
only measure:

* **A line moves no custody.** The ledger a whole composed run leaves is
  *exactly* the one the same program leaves with every line erased — which is
  the task route's own program — and the ledger is a function of the task steps
  alone.
* **A line moves no synthesized trace byte.** Erasing every line leaves the
  trace emissions unchanged, and the trace emissions are a function of the task
  steps and the workers' buffer sizes alone. That is why the F02 emission
  digests, the CT fixtures and every pinned trace byte are untouched by this
  card.
* **The parent is the only writer, and its own emissions do not move.** A
  worker's buffered trace enters the observation at that worker's join and
  nowhere else, so the parent's own emissions — its trace lines *and* its source
  lines together, in order — are the same list whatever the workers wrote. That
  is the interleaving argument the v1 boundary rests on, as a theorem rather
  than as a reading of the emitted runtime.
* **A line inside a worker has no admitted placement.** The dual: nothing of the
  kind holds for a worker's own writes, and the placement predicate refuses one.
* **The line vocabulary is `source.std-io.v1`'s own.** Each line projects into a
  `QCore.StdIo.Event`, and the stream it writes, the lines it publishes and the
  effect row it derives are all read off *that* route's own functions. This
  route invents no rendering.

Three things are deliberately **not** claimed. The order between standard output
and standard error is not modeled, exactly as on every other route that writes:
the two are separate lists here and nothing relates them. The digits a rendering
publishes are not modeled — a line carries the *number* that crossed from the
task model into it, and the digits are what the Rust tests compare against a
native run. And nothing here says the Rust lowering extracts these steps
faithfully from source tokens, or that the emitted C performs them; those are
tested Rust boundaries, on the `ConcurrencyPrelude` precedent. -/

namespace QCore.ComposedTasks

/-- A resource identity, as the task ledger names it. -/
abbrev Ident := Nat

/-- A worker's runtime identity. The parent is not one of these: it is
`Emission.parentTrace`'s own constructor, so "written by the parent" is a
structural fact here rather than a side condition. -/
abbrev Worker := Nat

/-- A name in the task model's **value** namespace. -/
abbrev Name := Nat

/-- The digits are not modeled; the number is. See the module header. -/
abbrev Text := Int

/-- **The rendering discipline, as a type.** Two constructors, and neither can
denote a resource, a read loan or a task handle: an operand is an integer
literal written at the site, or a name the task model bound to an `i32`.
`source.std-io.v1`'s third form, `borrow NAME`, is deliberately absent — the
task ledger publishes no identity to a rendering in v1, and
`QCore.ComposedStdIo` is where a borrowed render is proved, on the route whose
model publishes one. -/
inductive Operand where
  | lit (number : Int)
  | scalar (name : Name)
  deriving DecidableEq, Repr

/-- One line-producing statement. The vocabulary is `QCore.StdIo`'s, reused
rather than restated: see `project`. -/
inductive Line where
  | print (r : QCore.StdIo.Rendering) (o : Operand)
  | log (l : QCore.StdIo.Level) (r : QCore.StdIo.Rendering) (o : Operand)
  | flush
  deriving DecidableEq, Repr

/-- One step of the **parent's** body. The task steps are the ones that move
custody or write a trace line; `line` is this composition's own. -/
inductive Step where
  | alloc (r : Ident)
  | drop (r : Ident)
  | spawn (w : Worker) (moved : List Ident)
  | join (w : Worker) (returned : List Ident)
  | bind (n : Name) (v : Int)
  | line (l : Line)
  | ret
  deriving DecidableEq, Repr

/-- Where a line was written. -/
inductive Placement where
  | parent
  | worker (w : Worker)
  deriving DecidableEq, Repr

/-- The v1 placement rule, and the whole of it. -/
def admits : Placement → Bool
  | .parent => true
  | .worker _ => false

/-- One line of the emitted program's standard output. A worker's trace line is
told apart from the parent's by its own constructor, because that distinction is
the entire content of the boundary this module proves. -/
inductive Emission where
  | parentTrace
  | workerTrace (w : Worker)
  | source (text : Text)
  deriving DecidableEq, Repr

def isTrace : Emission → Bool
  | .parentTrace => true
  | .workerTrace _ => true
  | .source _ => false

/-- Whether an emission was written by the parent **at its own step**: every
source line, and every trace line reporting the parent's own activity. A
worker's trace line is not one — the parent replays it, at the join. -/
def parentEmission : Emission → Bool
  | .parentTrace => true
  | .workerTrace _ => false
  | .source _ => true

structure State where
  ledger : List Ident
  env : Name → Option Int
  out : List Emission
  err : List Emission

/-- What a rendering may read. There is no third case, which is the effect
discipline restated as a function: nothing an operand can be resolves to a
resource. -/
def readOperand (env : Name → Option Int) : Operand → Option Int
  | .lit number => some number
  | .scalar name => env name

def emitLine (s : State) : Line → State
  | .print _ o =>
      match readOperand s.env o with
      | some v => { s with out := s.out ++ [.source v] }
      | none => s
  | .log _ _ o =>
      match readOperand s.env o with
      | some v => { s with err := s.err ++ [.source v] }
      | none => s
  | .flush => s

/-- One step of the composed run. `worker w` is the number of trace lines the
worker `w` buffered — the model of the private event array the emitted runtime
gives each worker. It enters the observation at `join w`, which is the one place
the parent replays it. -/
def step (worker : Worker → Nat) (s : State) : Step → State
  | .alloc r => { s with ledger := r :: s.ledger, out := s.out ++ [.parentTrace] }
  | .drop r => { s with ledger := s.ledger.erase r, out := s.out ++ [.parentTrace] }
  | .spawn _ moved =>
      { s with
        ledger := s.ledger.filter (fun x => !(moved.contains x)),
        out := s.out ++ [.parentTrace] }
  | .join w returned =>
      { s with
        ledger := returned ++ s.ledger,
        out := s.out ++ List.replicate (worker w) (Emission.workerTrace w) ++ [.parentTrace] }
  | .bind n v => { s with env := fun m => if m = n then some v else s.env m }
  | .line l => emitLine s l
  | .ret => { s with out := s.out ++ [.parentTrace] }

def exec (worker : Worker → Nat) : List Step → State → State
  | [], s => s
  | p :: rest, s => exec worker rest (step worker s p)

/-- Whether a step is this composition's own. -/
def isLine : Step → Bool
  | .line _ => true
  | .alloc _ => false
  | .drop _ => false
  | .spawn _ _ => false
  | .join _ _ => false
  | .bind _ _ => false
  | .ret => false

/-- The program with every source line erased: the task route's own program. -/
def eraseLines (steps : List Step) : List Step :=
  steps.filter (fun p => !isLine p)

private theorem filter_replicate (n : Nat) (e : Emission) (f : Emission → Bool) :
    (List.replicate n e).filter f = if f e then List.replicate n e else [] := by
  induction n with
  | zero => simp
  | succ k ih =>
    by_cases h : f e <;> simp [List.replicate_succ, h, ih]

/-! ## The ledger is the task projection's -/

/-- The ledger transition of one step, in the ledger alone. -/
def ledgerStep (l : List Ident) : Step → List Ident
  | .alloc r => r :: l
  | .drop r => l.erase r
  | .spawn _ moved => l.filter (fun x => !(moved.contains x))
  | .join _ returned => returned ++ l
  | .bind _ _ => l
  | .line _ => l
  | .ret => l

def ledgerRun : List Step → List Ident → List Ident
  | [], l => l
  | p :: rest, l => ledgerRun rest (ledgerStep l p)

theorem ledger_of_step (worker : Worker → Nat) (s : State) (p : Step) :
    (step worker s p).ledger = ledgerStep s.ledger p := by
  cases p with
  | line l =>
    cases l with
    | print r o => cases h : readOperand s.env o <;> simp [step, emitLine, ledgerStep, h]
    | log lv r o => cases h : readOperand s.env o <;> simp [step, emitLine, ledgerStep, h]
    | flush => rfl
  | alloc r => rfl
  | drop r => rfl
  | spawn w moved => rfl
  | join w returned => rfl
  | bind n v => rfl
  | ret => rfl

theorem a_line_leaves_the_ledger_unchanged
    (worker : Worker → Nat) (s : State) (l : Line) :
    (step worker s (.line l)).ledger = s.ledger := by
  rw [ledger_of_step]; rfl

/-- The ledger a composed run leaves is a function of its **task** steps and its
starting ledger alone: no line, no rendering and no stream reaches it. -/
theorem the_ledger_is_a_function_of_the_task_steps_alone
    (worker : Worker → Nat) (steps : List Step) (s : State) :
    (exec worker steps s).ledger = ledgerRun steps s.ledger := by
  induction steps generalizing s with
  | nil => rfl
  | cons p rest ih => rw [exec, ledgerRun, ih, ledger_of_step]

theorem ledgerRun_ignores_lines (steps : List Step) (l : List Ident) :
    ledgerRun steps l = ledgerRun (eraseLines steps) l := by
  induction steps generalizing l with
  | nil => rfl
  | cons p rest ih =>
    cases p <;>
      simp [ledgerRun, eraseLines, isLine, ledgerStep, ih]

/-- **Composing added no custody path.** The ledger a whole composed run leaves
is exactly the one the same program leaves with every source line erased. -/
theorem the_composed_ledger_is_the_task_projections_ledger
    (worker : Worker → Nat) (steps : List Step) (s : State) :
    (exec worker steps s).ledger = (exec worker (eraseLines steps) s).ledger := by
  rw [the_ledger_is_a_function_of_the_task_steps_alone,
      the_ledger_is_a_function_of_the_task_steps_alone, ledgerRun_ignores_lines]

/-- A whole program of nothing but lines leaves the ledger untouched. -/
theorem a_program_of_lines_leaves_the_ledger_unchanged
    (worker : Worker → Nat) (lines : List Line) (s : State) :
    (exec worker (lines.map Step.line) s).ledger = s.ledger := by
  induction lines generalizing s with
  | nil => rfl
  | cons l rest ih =>
    rw [List.map_cons, exec, ih, ledger_of_step]
    rfl

/-! ## The synthesized trace does not move -/

def traceStep (worker : Worker → Nat) (o : List Emission) : Step → List Emission
  | .alloc _ => o ++ [.parentTrace]
  | .drop _ => o ++ [.parentTrace]
  | .spawn _ _ => o ++ [.parentTrace]
  | .join w _ => o ++ List.replicate (worker w) (Emission.workerTrace w) ++ [.parentTrace]
  | .bind _ _ => o
  | .line _ => o
  | .ret => o ++ [.parentTrace]

def traceRun (worker : Worker → Nat) : List Step → List Emission → List Emission
  | [], o => o
  | p :: rest, o => traceRun worker rest (traceStep worker o p)

theorem trace_of_step (worker : Worker → Nat) (s : State) (p : Step) :
    (step worker s p).out.filter isTrace = traceStep worker (s.out.filter isTrace) p := by
  cases p with
  | line l =>
    cases l with
    | print r o =>
      cases h : readOperand s.env o <;>
        simp [step, emitLine, traceStep, h, List.filter_append, isTrace]
    | log lv r o =>
      cases h : readOperand s.env o <;> simp [step, emitLine, traceStep, h]
    | flush => rfl
  | join w returned =>
    simp [step, traceStep, List.filter_append, filter_replicate, isTrace]
  | alloc r => simp [step, traceStep, List.filter_append, isTrace]
  | drop r => simp [step, traceStep, List.filter_append, isTrace]
  | spawn w m => simp [step, traceStep, List.filter_append, isTrace]
  | bind n v => rfl
  | ret => simp [step, traceStep, List.filter_append, isTrace]

/-- The synthesized trace a composed run produces is a function of its **task**
steps, its workers' buffer sizes and its starting trace alone: no source line
reaches it. -/
theorem the_trace_is_a_function_of_the_task_steps_alone
    (worker : Worker → Nat) (steps : List Step) (s : State) :
    (exec worker steps s).out.filter isTrace
      = traceRun worker steps (s.out.filter isTrace) := by
  induction steps generalizing s with
  | nil => rfl
  | cons p rest ih => rw [exec, traceRun, ih, trace_of_step]

theorem traceRun_ignores_lines (worker : Worker → Nat) (steps : List Step)
    (o : List Emission) :
    traceRun worker steps o = traceRun worker (eraseLines steps) o := by
  induction steps generalizing o with
  | nil => rfl
  | cons p rest ih =>
    cases p <;>
      simp [traceRun, eraseLines, isLine, traceStep, ih]

/-- **Erasing the source lines leaves exactly the task rendering.** The Rust test
`removing_the_source_lines_leaves_exactly_the_task_rendering` checks this over
the four published examples; this is the general statement. -/
theorem erasing_the_source_lines_leaves_the_task_trace
    (worker : Worker → Nat) (steps : List Step) (s : State) :
    (exec worker steps s).out.filter isTrace
      = (exec worker (eraseLines steps) s).out.filter isTrace := by
  rw [the_trace_is_a_function_of_the_task_steps_alone,
      the_trace_is_a_function_of_the_task_steps_alone, traceRun_ignores_lines]

/-! ## The parent is the only writer -/

/-- Two states a run cannot tell apart when only the workers' buffer sizes
differ: the same custody, the same value environment, the same logger stream,
and the same **parent** emissions on standard output. -/
def Agree (a b : State) : Prop :=
  a.ledger = b.ledger ∧ a.env = b.env ∧ a.err = b.err ∧
    a.out.filter parentEmission = b.out.filter parentEmission

theorem agree_step (worker worker' : Worker → Nat) {a b : State} (h : Agree a b)
    (p : Step) : Agree (step worker a p) (step worker' b p) := by
  obtain ⟨hl, he, hr, ho⟩ := h
  cases p with
  | alloc r =>
    refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [step, hl, he, hr, List.filter_append, ho]
  | drop r =>
    refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [step, hl, he, hr, List.filter_append, ho]
  | spawn w m =>
    refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [step, hl, he, hr, List.filter_append, ho]
  | join w r =>
    refine ⟨?_, ?_, ?_, ?_⟩ <;>
      simp [step, hl, he, hr, List.filter_append, parentEmission, ho]
  | bind n v =>
    refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [step, hl, he, hr, ho]
  | ret =>
    refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [step, hl, he, hr, List.filter_append, ho]
  | line l =>
    cases l with
    | print r o =>
      cases hv : readOperand a.env o with
      | none =>
        have hb : readOperand b.env o = none := by rw [← he]; exact hv
        refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [step, emitLine, hb, hl, he, hr, ho]
      | some v =>
        have hb : readOperand b.env o = some v := by rw [← he]; exact hv
        refine ⟨?_, ?_, ?_, ?_⟩ <;>
          simp [step, emitLine, hb, hl, he, hr, List.filter_append, ho]
    | log lv r o =>
      cases hv : readOperand a.env o with
      | none =>
        have hb : readOperand b.env o = none := by rw [← he]; exact hv
        refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [step, emitLine, hb, hl, he, hr, ho]
      | some v =>
        have hb : readOperand b.env o = some v := by rw [← he]; exact hv
        refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [step, emitLine, hb, hl, he, hr, ho]
    | flush =>
      refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [step, emitLine, hl, he, hr, ho]

theorem agree_exec (worker worker' : Worker → Nat) (steps : List Step) {a b : State}
    (h : Agree a b) : Agree (exec worker steps a) (exec worker' steps b) := by
  induction steps generalizing a b with
  | nil => exact h
  | cons p rest ih => exact ih (agree_step worker worker' h p)

/-- **The interleaving argument.** A worker's buffered trace enters the
observation only at that worker's join, so the parent's own emissions — its
trace lines and its source lines together, in order — are the same list whatever
the workers wrote.

This is exactly what makes a source line's position determined at *every* point
of the parent's body, which is why the v1 boundary admits a line anywhere there,
and it is exactly what fails for a line inside a worker, which is why `admits`
refuses one. -/
theorem the_parents_own_emissions_do_not_depend_on_the_workers
    (worker worker' : Worker → Nat) (steps : List Step) (s : State) :
    (exec worker steps s).out.filter parentEmission
      = (exec worker' steps s).out.filter parentEmission :=
  (agree_exec worker worker' steps (⟨rfl, rfl, rfl, rfl⟩ : Agree s s)).2.2.2

/-- The logger stream is untouched by the workers for the same reason, and this
states it separately because it is a separate observation: no synthesized trace
line is ever written there. -/
theorem the_logger_stream_does_not_depend_on_the_workers
    (worker worker' : Worker → Nat) (steps : List Step) (s : State) :
    (exec worker steps s).err = (exec worker' steps s).err :=
  (agree_exec worker worker' steps (⟨rfl, rfl, rfl, rfl⟩ : Agree s s)).2.2.1

/-- And so is the ledger. -/
theorem the_ledger_does_not_depend_on_the_workers
    (worker worker' : Worker → Nat) (steps : List Step) (s : State) :
    (exec worker steps s).ledger = (exec worker' steps s).ledger :=
  (agree_exec worker worker' steps (⟨rfl, rfl, rfl, rfl⟩ : Agree s s)).1

/-- A worker's trace enters the observation **only** at a join: no other step
contributes one. -/
theorem only_a_join_replays_a_workers_trace
    (worker : Worker → Nat) (s : State) (p : Step)
    (h : ∀ w r, p ≠ Step.join w r) :
    (step worker s p).out.filter (fun e => !parentEmission e)
      = s.out.filter (fun e => !parentEmission e) := by
  cases p with
  | join w r => exact absurd rfl (h w r)
  | alloc r => simp [step, List.filter_append, parentEmission]
  | drop r => simp [step, List.filter_append, parentEmission]
  | spawn w m => simp [step, List.filter_append, parentEmission]
  | bind n v => rfl
  | ret => simp [step, List.filter_append, parentEmission]
  | line l =>
    cases l with
    | print r o =>
      cases hv : readOperand s.env o <;>
        simp [step, emitLine, hv, List.filter_append, parentEmission]
    | log lv r o => cases hv : readOperand s.env o <;> simp [step, emitLine, hv]
    | flush => rfl

/-! ## Placement -/

theorem a_line_in_a_worker_has_no_admitted_placement (w : Worker) :
    admits (.worker w) = false := rfl

theorem a_line_in_the_parent_is_admitted : admits .parent = true := rfl

/-- The placement surface as one list, so a future widening changes a theorem
rather than slipping past it. -/
theorem the_placements_are_exactly_these (p : Placement) :
    p = .parent ∨ ∃ w, p = .worker w := by
  cases p with
  | parent => exact Or.inl rfl
  | worker w => exact Or.inr ⟨w, rfl⟩

/-! ## The line vocabulary is `source.std-io.v1`'s own -/

def projectOperand : Operand → QCore.StdIo.Operand
  | .lit number => .literal number
  | .scalar name => .scalar (toString name)

/-- Every line of this route is a `source.std-io.v1` event. The projection is
total and injective on the operand forms, and it never produces
`QCore.StdIo.Operand.borrowed`: this route has no borrowed render. -/
def project : Line → QCore.StdIo.Event
  | .print r o => .print r (projectOperand o)
  | .log l r o => .log l r (projectOperand o)
  | .flush => .flush

/-- How many lines a statement publishes on a stream, decided here. -/
def publishes (s : QCore.StdIo.Stream) : Line → Nat
  | .print _ _ => if s = QCore.StdIo.Stream.out then 1 else 0
  | .log _ _ _ => if s = QCore.StdIo.Stream.err then 1 else 0
  | .flush => 0

/-- …and decided the same way by `source.std-io.v1`. The two are not two
implementations that agree: this theorem is what makes the Rust side's reuse of
`std_io::rendered_line` a statement about one decision. -/
theorem the_streams_agree_with_the_std_io_route (s : QCore.StdIo.Stream) (l : Line) :
    publishes s l = QCore.StdIo.linesOn s (project l) := by
  cases l with
  | print r o => cases s <;> rfl
  | log lv r o => cases s <;> rfl
  | flush => cases s <;> rfl

theorem print_never_writes_stderr (r : QCore.StdIo.Rendering) (o : Operand) :
    publishes QCore.StdIo.Stream.err (.print r o) = 0 := rfl

theorem log_never_writes_stdout (lv : QCore.StdIo.Level) (r : QCore.StdIo.Rendering)
    (o : Operand) : publishes QCore.StdIo.Stream.out (.log lv r o) = 0 := rfl

theorem a_barrier_publishes_no_line (s : QCore.StdIo.Stream) :
    publishes s .flush = 0 := by cases s <;> rfl

/-- A resolved print publishes exactly one line on standard output, and touches
the logger stream not at all. -/
theorem a_resolved_print_publishes_exactly_one_line
    (worker : Worker → Nat) (s : State) (r : QCore.StdIo.Rendering) (o : Operand)
    (v : Int) (h : readOperand s.env o = some v) :
    (step worker s (.line (.print r o))).out = s.out ++ [.source v] ∧
      (step worker s (.line (.print r o))).err = s.err := by
  constructor <;> simp [step, emitLine, h]

/-- An operand the environment does not resolve publishes nothing at all: a
composed program never renders a number no model computed. On the Rust side the
checker refuses such a program before it runs; this is the model's own
fail-closed statement of the same thing. -/
theorem an_unresolved_operand_publishes_nothing
    (worker : Worker → Nat) (s : State) (l : Line) (o : Operand)
    (hl : l = .print (.dec) o ∨ ∃ lv r, l = .log lv r o)
    (h : readOperand s.env o = none) :
    (step worker s (.line l)).out = s.out ∧ (step worker s (.line l)).err = s.err := by
  rcases hl with rfl | ⟨lv, r, rfl⟩
  · constructor <;> simp [step, emitLine, h]
  · constructor <;> simp [step, emitLine, h]

/-- The operand surface as one list, so a future widening — a borrowed render,
say — changes a theorem rather than slipping past it. -/
theorem an_operand_is_a_literal_or_a_scalar_name (o : Operand) :
    (∃ n, o = .lit n) ∨ ∃ n, o = .scalar n := by
  cases o with
  | lit n => exact Or.inl ⟨n, rfl⟩
  | scalar n => exact Or.inr ⟨n, rfl⟩

/-- A rendering reads only a name the task model bound: `readOperand` consults
the environment and nothing else, and there is no operand form it could consult
a ledger for. -/
theorem a_rendering_reads_only_a_bound_name (env : Name → Option Int) (n : Name) :
    readOperand env (.scalar n) = env n := rfl

/-! ## The effect row -/

/-- The derived row, from `source.std-io.v1`'s own function over the projected
events. -/
def derivedRow (lines : List Line) : List QCore.StdIo.Effect :=
  QCore.StdIo.derivedEffects (lines.map project)

theorem a_program_of_no_lines_carries_nothing : derivedRow [] = [] := rfl

/-- Any line at all — a barrier included — carries `Io`, so it needs its module.
`QCore.StdIo.the_barrier_still_carries_io` is the same statement on the route
this one reuses. -/
theorem writing_requires_io (l : Line) (rest : List Line) :
    derivedRow (l :: rest) = [QCore.StdIo.Effect.io] := by
  cases l <;> rfl

theorem a_barrier_still_carries_io :
    derivedRow [Line.flush] = [QCore.StdIo.Effect.io] := rfl

/-- And never anything else: a rendering allocates nothing and mutates nothing,
which is the structural discipline `STD-MODULES` established, over this route's
own value namespace. -/
theorem no_line_carries_alloc_or_mut (lines : List Line) :
    QCore.StdIo.Effect.alloc ∉ derivedRow lines ∧
      QCore.StdIo.Effect.mut ∉ derivedRow lines := by
  cases lines with
  | nil => rw [a_program_of_no_lines_carries_nothing]; exact ⟨by simp, by simp⟩
  | cons l rest => rw [writing_requires_io]; exact ⟨by simp, by simp⟩

end QCore.ComposedTasks
