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

import QCore.StdEvent
import QCore.ObsReordering

/-! `source.event.v1` × the source line vocabulary (card `COMPOSE-EVENT-LINES`).

# What is proved here, and what is deliberately not

Card `STD-EVENT` proved two things about this route: the determinism boundary,
and the reactor projection. This card composes source lines into it, and the
whole question a reader should ask is whether either of those two survives. Both
do, and that is the content of this module:

1. **A line decides nothing.** `erase` drops every line and every payload from a
   composed program, and `erasing_the_lines_leaves_the_reactor_projection` and
   `erasing_the_lines_leaves_the_run_unchanged` say that the reactor projection
   and the ledger's own admission decision are unmoved by it. So
   `QCore.StdEvent`'s theorems about a program apply verbatim to the same
   program with lines, and this card proves no custody or determinism claim of
   its own — it *inherits* them.

2. **The payload is a queue, not a count.** `wake D BYTE;` delivers one declared
   byte and a ready turn drains the oldest undrained one, so the value a turn
   binds is the byte its own wake delivered. That is what the named premise
   `EVENT-V1-STREAM-BYTE-ORDER` is about, and it is why the emitted program
   compares the byte it read against the one this ledger predicts.

3. **The comparison is strict, and strict is the degenerate case rather than a
   separate rule.** Every record an event program writes is written by the
   entry's own thread — the profile is single-threaded — so every record is a
   barrier of `QCore.ObsReordering` and the declared relation admits exactly one
   stream. `the_event_relation_is_strict` and
   `a_native_event_stream_is_admitted_exactly_when_it_is_byte_identical` are that
   statement, and they are the reason this card declares **no** reordering.

# What is not proved here

That the Rust lowering reads these statements faithfully out of source tokens;
that the rendering of a line produces particular bytes (a line's text is opaque
here, exactly as it is in `QCore.ObsReordering`); anything about the platform,
which is two declared premises checked at run time by the emitted program. -/

namespace QCore.ComposeEventLines

open QCore.StdEvent

/-- The two streams a composed program writes. No order between them is modeled
here or anywhere else. -/
inductive Stream where
  | out
  | err
  deriving DecidableEq, BEq, Repr

/-- One statement of a composed entry.

It is `QCore.StdEvent.Statement` plus exactly three differences: a `wake` carries
the byte it delivers, a ready turn may bind the byte it drains, and there is a
line-producing statement. A line's bytes are a `Nat` for the same reason
`QCore.ObsReordering.Text` is one — nothing below reads them, so no theorem here
can depend on a rendering. -/
inductive Statement where
  | logLevel (level : DiagnosticLevel)
  | contextOpen
  | contextClose
  | endpoints (first second : Nat)
  | watch (registration : Registration)
  | unwatch (event : Nat)
  | wake (endpoint : Nat) (payload : Nat)
  | dispatchReady (event : Nat) (binding : Option Nat)
  | dispatchIdle
  /-- `print RENDERING OPERAND;` or `log LEVEL RENDERING OPERAND;`, already
  rendered. -/
  | line (stream : Stream) (text : Nat)
  | ret (status : Int)
  deriving DecidableEq, BEq, Repr

/-! ## Erasure: a composed program is an event program with lines in it -/

/-- The event statement a composed statement stands for, or `none` for a line.

This is the whole soundness argument of the composition in one function: what
the reactor and the determinism ledger see is exactly what they saw before this
card. -/
def erase : Statement → Option QCore.StdEvent.Statement
  | .logLevel level => some (.logLevel level)
  | .contextOpen => some .contextOpen
  | .contextClose => some .contextClose
  | .endpoints a b => some (.endpoints a b)
  | .watch r => some (.watch r)
  | .unwatch e => some (.unwatch e)
  | .wake d _ => some (.wake d)
  | .dispatchReady e _ => some (.dispatchReady e)
  | .dispatchIdle => some .dispatchIdle
  | .line _ _ => none
  | .ret status => some (.ret status)

def eraseProgram (p : List Statement) : List QCore.StdEvent.Statement :=
  p.filterMap erase

/-- The reactor projection of a composed program: the projection of its erasure,
and nothing beside it. -/
def project (p : List Statement) : List QCore.StdEvent.Operation :=
  QCore.StdEvent.projectProgram (eraseProgram p)

def isLine : Statement → Bool
  | .line _ _ => true
  | _ => false

/-- **A line projects to no reactor operation.** It is program output; the
reactor models none of it. -/
theorem a_line_erases_to_nothing (s : Stream) (text : Nat) :
    erase (.line s text) = none := rfl

theorem eraseProgram_cons (head : Statement) (tail : List Statement) :
    eraseProgram (head :: tail)
      = (match erase head with | some e => [e] | none => []) ++ eraseProgram tail := by
  cases head <;> rfl

/-- A statement erases to nothing exactly when it is a line. -/
theorem erasing_to_nothing_is_being_a_line (statement : Statement) :
    erase statement = none ↔ isLine statement = true := by
  cases statement <;> simp [erase, isLine]

/-- Erasure ignores the lines: dropping them first changes nothing. -/
theorem erasing_the_lines_leaves_the_erasure (p : List Statement) :
    eraseProgram (p.filter fun s => !isLine s) = eraseProgram p := by
  induction p with
  | nil => rfl
  | cons head tail ih =>
    by_cases line : isLine head = true
    · have erased : erase head = none := (erasing_to_nothing_is_being_a_line head).mpr line
      rw [List.filter_cons_of_neg (by simp [line]), ih]
      simp [eraseProgram, erased]
    · have kept : ¬ erase head = none := by
        intro h
        exact line ((erasing_to_nothing_is_being_a_line head).mp h)
      rw [List.filter_cons_of_pos (by simp [line]), eraseProgram, eraseProgram,
        List.filterMap_cons, List.filterMap_cons]
      cases value : erase head with
      | none => exact absurd value kept
      | some e => simpa [eraseProgram] using ih

/-- **Erasing every line leaves the reactor projection byte for byte.** The
strong form, in the shape card `STD-EVENT` used for `wake`: whatever the reactor
concludes about a composed program it concludes about the same program with every
line removed, so composing lines decides no custody question. -/
theorem erasing_the_lines_leaves_the_reactor_projection (p : List Statement) :
    project (p.filter fun s => !isLine s) = project p := by
  unfold project
  rw [erasing_the_lines_leaves_the_erasure]

/-! ## The readiness ledger, with payloads

The ledger is `QCore.StdEvent.State` with the pending multiset refined into a
**queue of bytes per endpoint**. `forget` is the projection back, and
`ready_set_is_unchanged_by_the_payloads` says the refinement decides nothing the
coarse ledger did not: readiness is still "this endpoint has an undrained byte".
-/

structure State where
  watched : List Registration := []
  /-- `(endpoint, byte)` in the order the bytes were written. -/
  pending : List (Nat × Nat) := []
  /-- The value namespace: a binding and the byte it names. -/
  scalars : List (Nat × Nat) := []
  deriving DecidableEq, Repr

/-- The coarse ledger this one refines. -/
def forget (s : State) : QCore.StdEvent.State :=
  { watched := s.watched, pending := s.pending.map Prod.fst }

def readySet (s : State) : List Registration :=
  QCore.StdEvent.readySet (forget s)

/-- **The refinement decides nothing new.** A registration is ready here exactly
when it is ready in the coarse ledger, so every `QCore.StdEvent` theorem about
readiness — the by-name two-member refusal included — is a theorem about this
ledger. -/
theorem ready_set_is_the_coarse_one (s : State) :
    readySet s = QCore.StdEvent.readySet (forget s) := rfl

def admits (s : State) : Statement → Bool
  | .dispatchReady e _ =>
      match readySet s with
      | [r] => r.event == e
      | _ => false
  | .dispatchIdle => (readySet s).isEmpty
  | _ => true

/-- The oldest undrained byte of `endpoint`, if any. -/
def peek (s : State) (endpoint : Nat) : Option Nat :=
  (s.pending.find? fun entry => entry.1 == endpoint).map Prod.snd

/-- Remove that byte, and only it. -/
def dropFirst : List (Nat × Nat) → Nat → List (Nat × Nat)
  | [], _ => []
  | entry :: rest, endpoint =>
      if entry.1 == endpoint then rest else entry :: dropFirst rest endpoint

def step (s : State) : Statement → State
  | .watch r => { s with watched := r :: s.watched }
  | .unwatch e => { s with watched := s.watched.filter fun r => r.event != e }
  | .wake d payload => { s with pending := s.pending ++ [(d, payload)] }
  | .dispatchReady _ binding =>
      match readySet s with
      | [r] =>
          let byte := (peek s r.endpoint).getD 0
          { s with
              pending := dropFirst s.pending r.endpoint,
              scalars :=
                match binding with
                | some name => (name, byte) :: s.scalars
                | none => s.scalars }
      | _ => s
  | _ => s

def run (s : State) : List Statement → Option State
  | [] => some s
  | statement :: rest =>
      if admits s statement then run (step s statement) rest else none

/-! ## A line decides nothing -/

/-- A line is always admitted: it asks the ledger no question. -/
theorem a_line_is_always_admitted (s : State) (stream : Stream) (text : Nat) :
    admits s (.line stream text) = true := rfl

/-- And it changes no ledger state. -/
theorem a_line_moves_no_ledger_state (s : State) (stream : Stream) (text : Nat) :
    step s (.line stream text) = s := rfl

/-- **Erasing every line leaves the run unchanged.** Together with
`erasing_the_lines_leaves_the_reactor_projection` this is the composition's whole
soundness claim: neither half of card `STD-EVENT`'s answer moves. -/
theorem erasing_the_lines_leaves_the_run_unchanged (s : State) (p : List Statement) :
    run s (p.filter fun t => !isLine t) = run s p := by
  induction p generalizing s with
  | nil => rfl
  | cons head tail ih =>
    by_cases line : isLine head = true
    · have shape : ∃ stream text, head = .line stream text := by
        cases head <;> simp [isLine] at line ⊢
      obtain ⟨stream, text, rfl⟩ := shape
      rw [List.filter_cons_of_neg (by simp [line]), ih]
      simp [run, a_line_is_always_admitted, a_line_moves_no_ledger_state]
    · rw [List.filter_cons_of_pos (by simp [line]), run, run, ih]

/-! ## The determinism boundary, inherited -/

/-- **The by-name refusal, unmoved.** Two ready registrations admit no dispatch
declaration, with lines in the program or without them — the boundary is about
the ledger's state and nothing else. -/
theorem two_ready_registrations_admit_no_turn
    (s : State) (a b : Registration) (rest : List Registration)
    (h : readySet s = a :: b :: rest) :
    ∀ (e : Nat) (binding : Option Nat), admits s (.dispatchReady e binding) = false := by
  intro e binding
  simp [admits, h]

theorem two_ready_registrations_admit_no_idle_turn
    (s : State) (a b : Registration) (rest : List Registration)
    (h : readySet s = a :: b :: rest) :
    admits s .dispatchIdle = false := by
  simp [admits, h]

/-- A binding buys no turn: `let v = dispatch ready E;` is admitted exactly where
`dispatch ready E;` is. -/
theorem a_binding_admits_exactly_what_the_bare_turn_admits
    (s : State) (e : Nat) (binding : Option Nat) :
    admits s (.dispatchReady e binding) = admits s (.dispatchReady e none) := by
  simp [admits]

/-! ## The payload is the byte its own wake delivered -/

/-- A wake appends its byte to the endpoint's queue. -/
theorem a_wake_queues_its_byte (s : State) (d payload : Nat) :
    (step s (.wake d payload)).pending = s.pending ++ [(d, payload)] := rfl

/-- **First in, first out.** With two bytes queued on one endpoint, the first
ready turn binds the first byte and the second binds the second. Stated on a
concrete program, so the model and `examples/events/wake_payload.qr` cannot
drift apart silently. -/
theorem the_wake_payload_example_binds_seventy_two_then_one_hundred_and_five :
    (run {}
      [ .contextOpen,
        .endpoints 0 1,
        .watch { event := 0, endpoint := 0, interest := .read },
        .wake 0 72,
        .wake 0 105,
        .dispatchReady 0 (some 0),
        .line .err 0,
        .dispatchReady 0 (some 1),
        .line .err 1,
        .dispatchIdle,
        .unwatch 0,
        .ret 0 ]).map State.scalars = some [(1, 105), (0, 72)] := by decide

/-- The same program with the two wakes exchanged binds the two bytes the other
way round. The binding is the queue's own order and not the statement's. -/
theorem exchanging_the_two_wakes_exchanges_the_two_bound_bytes :
    (run {}
      [ .contextOpen,
        .endpoints 0 1,
        .watch { event := 0, endpoint := 0, interest := .read },
        .wake 0 105,
        .wake 0 72,
        .dispatchReady 0 (some 0),
        .dispatchReady 0 (some 1),
        .ret 0 ]).map State.scalars = some [(1, 72), (0, 105)] := by decide

/-- A turn that binds nothing still drains, so the program that follows it is the
same program either way. -/
theorem an_unbound_turn_drains_the_same_byte :
    (run {}
      [ .contextOpen,
        .endpoints 0 1,
        .watch { event := 0, endpoint := 0, interest := .read },
        .wake 0 72,
        .dispatchReady 0 none,
        .dispatchIdle,
        .ret 0 ]).isSome = true := by decide

/-- And with no wake there is no turn to bind at all: the refusal is the absence
of an induced readiness, exactly as it is without a binding. -/
theorem a_binding_without_a_wake_is_refused :
    run {}
      [ .contextOpen,
        .endpoints 0 1,
        .watch { event := 0, endpoint := 0, interest := .read },
        .dispatchReady 0 (some 0) ] = none := by decide

/-! ## The observation, and why the comparison is strict

A record is written by the thread that wrote its bytes. Every record an event
program writes is written by the entry's own thread: the one accepted profile is
single-threaded, so the thread running a callback *is* that thread and there is
no second emitter to order against. The consequence is that every record is a
barrier in `QCore.ObsReordering`'s sense, the declared relation admits exactly
one stream, and comparing a native run against it **is** byte equality. -/

/-- The records one statement contributes to one stream, given the selected
diagnostic threshold.

A line contributes its own bytes; every other statement contributes its retained
trace record, and only where `log_level` installed the adapter. `traced` is the
trace record's bytes. -/
def records (level : Option DiagnosticLevel) (traced : Nat) (stream : Stream) :
    Statement → List Nat
  | .line s text => if s == stream then [text] else []
  | .logLevel _ => []
  | _ => match level, stream with
      | some _, .out => [traced]
      | _, _ => []

/-- **Without `log_level` the retained trace is not stream bytes.** The events
themselves are unchanged — they are in the certificate and in the emitted
program's typed logger calls — but the null sink writes none of them. -/
theorem a_quiet_program_writes_no_trace_record (traced : Nat) (stream : Stream)
    (statement : Statement) (h : isLine statement = false) :
    records none traced stream statement = [] := by
  cases statement <;> simp [records, isLine] at h ⊢

/-- A source line's bytes are program output on its own stream, whatever the
threshold is. -/
theorem a_line_writes_its_own_stream (level : Option DiagnosticLevel) (traced text : Nat)
    (stream : Stream) :
    records level traced stream (.line stream text) = [text] := by
  cases stream <;> rfl

theorem a_line_writes_no_other_stream (level : Option DiagnosticLevel) (traced text : Nat)
    (first second : Stream) (h : first ≠ second) :
    records level traced second (.line first text) = [] := by
  cases first <;> cases second <;> first | rfl | exact absurd rfl h

/-- The observation of one stream: every record alone in its own window, because
no other thread of control is ever live. -/
def observation (level : Option DiagnosticLevel) (traced : Nat) (stream : Stream)
    (p : List Statement) : QCore.ObsReordering.Stream :=
  (p.flatMap (records level traced stream)).map fun text => [(QCore.ObsReordering.Emitter.parent, text)]

/-- **Every window of an event program's observation is a barrier.** -/
theorem every_window_is_a_barrier (level : Option DiagnosticLevel) (traced : Nat)
    (stream : Stream) (p : List Statement) :
    ∀ a ∈ observation level traced stream p, QCore.ObsReordering.IsBarrier a := by
  intro a ha
  unfold observation at ha
  rw [List.mem_map] at ha
  obtain ⟨_, _, rfl⟩ := ha
  rfl

/-- **The declared relation is strict.** No reordering is admitted, because none
is needed: this route has one emitter. -/
theorem the_event_relation_is_strict (level : Option DiagnosticLevel) (traced : Nat)
    (stream : Stream) (p : List Statement) :
    QCore.ObsReordering.Strict (observation level traced stream p) :=
  QCore.ObsReordering.barriers_are_strict (every_window_is_a_barrier level traced stream p)

/-- **And therefore a native run is admitted exactly when it is byte-identical.**
The strict comparison is not a second rule beside `qcore.obs-reordering.v1`; it
is that relation at the declaration which admits nothing, which is what card
`OBS-REORDERING` proved in general and what this route is a case of. -/
theorem a_native_event_stream_is_admitted_exactly_when_it_is_byte_identical
    (level : Option DiagnosticLevel) (traced : Nat) (stream : Stream) (p : List Statement)
    (observed : QCore.ObsReordering.Stream) :
    QCore.ObsReordering.Equiv (observation level traced stream p) observed
      ↔ observation level traced stream p = observed :=
  QCore.ObsReordering.strict_iff_byte_equality (the_event_relation_is_strict level traced stream p)

/-- The one emitter, named: every record is the entry thread's. -/
theorem every_record_is_the_entry_threads (level : Option DiagnosticLevel) (traced : Nat)
    (stream : Stream) (p : List Statement) :
    ∀ a ∈ observation level traced stream p, ∀ r ∈ a, r.1 = QCore.ObsReordering.Emitter.parent := by
  intro a ha r hr
  unfold observation at ha
  rw [List.mem_map] at ha
  obtain ⟨_, _, rfl⟩ := ha
  rw [List.mem_singleton] at hr
  subst hr
  rfl

end QCore.ComposeEventLines
