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

/-!
# Rendering an admitted protection growth (card `CT34-SANITIZED`)

Card CT34-GROWTH gave protection growth a checked existence in
`CheckedMutexInvariantV1` and recorded, as its boundary `CT34-GROWTH-NATIVE`,
that the C backend refused to render one: nothing in that backend reads a
generation's declared shape, so an emitted `mtx_t` would protect less than the
model says. This module models the decision that replaces that refusal.

## What a growth is at C level

A `CheckedMutexInvariantV1` footprint is a **declared shape**. The backend
protects by *discipline* — the checker decided which accesses may run under
which guard — and a C mutex has no extent to widen. So the honest rendering of a
growth is: **no `mtx_t` operation at all**, plus the one bookkeeping line that
keeps the emitted program's observation comparable with the reference
execution's. `operations_are_exactly_the_traces_own_mutex_calls` and
`growth_contributes_no_mutex_operation` are the statement of it.

Nothing here claims a C mutex began protecting more memory. What the sanitized
native run beside this module witnesses is narrower, and is stated in
`docs/reports/remaining-ct34-sanitized/README.md`: the accesses the checker
admitted *after* the growth are real, in-bounds accesses to real allocations
inside the growing section's own lock/unlock pair, and the two renderings agree
line for line.

## What is decided here

Rendering is total in the sense that matters: it answers `some` statements or
`none`, never a partial program. A growth is rendered **only** where the
supplied certificate admits it — a record naming this event index, task, guard,
generation, interval and new predicate identity, whose interval the
certificate's own effective protected shape also carries. Every other growth is
refused, and a refusal anywhere refuses the whole program
(`a_refused_growth_refuses_the_whole_program`). That is the fail-closed half
card CT34-GROWTH's boundary leaves standing: the backend renders checked
modules and nothing else.

The reference execution and the emitted program are modelled as two functions
written separately — `stepLines` produces observation lines, `stepStatements`
produces C statements, and `observe` says what a statement prints — and
`emitted_and_reference_renderings_agree` proves they agree on every trace. In
Rust these are `interpret_mutex_program` and `emit_mutex_c11`, neither derived
from the other's output. That a `printf` statement prints its own argument is a
premise of this model, not a theorem of it, exactly as source extraction is
elsewhere in this development.
-/

namespace QCore.ProtectionGrowthEmission

/-- A byte interval of one shared resource, as `AccessFootprint`. -/
structure Footprint where
  resource : Nat
  start : Nat
  stop : Nat
deriving DecidableEq, Repr

/-- The events this backend renders: the single-task subset, plus the growth
this card adds. -/
inductive Event where
  | createMutex (mutex : Nat)
  | lock (mutex guard : Nat)
  | unlock (mutex guard : Nat)
  | write (footprint : Footprint)
  | read (footprint : Footprint)
  | grow (task guard : Nat) (footprint : Footprint) (invariant : Nat)
  | destroyMutex (mutex : Nat)
deriving DecidableEq, Repr

/-- `AdmittedProtectionGrowth`: one growth the checker admitted, carrying every
field the event carries. -/
structure Growth where
  event : Nat
  task : Nat
  mutex : Nat
  guard : Nat
  footprint : Footprint
  toInvariant : Nat
deriving DecidableEq, Repr

/-- The part of `MutexInvariantCertificate` the backend reads: the admitted
growths, and the *effective* protected shape they produced. -/
structure Certificate where
  growths : List Growth
  protectedShape : List (Nat × Footprint)
deriving DecidableEq, Repr

/-- One line of the execution observation. -/
inductive Line where
  | created (mutex : Nat)
  | locked (mutex guard : Nat)
  | unlocked (mutex guard : Nat)
  | wrote (footprint : Footprint)
  | readBack (footprint : Footprint)
  | grew (mutex guard : Nat) (footprint : Footprint)
  | destroyed (mutex : Nat)
deriving DecidableEq, Repr

/-- The C-level mutex operations. A rendering that quietly acquired, released,
initialised or destroyed something shows up here. -/
inductive MutexOp where
  | init (mutex : Nat)
  | acquire (mutex : Nat)
  | release (mutex : Nat)
  | destroy (mutex : Nat)
deriving DecidableEq, Repr

/-- One emitted C statement: a mutex call paired with the line it prints, or a
bare `printf`. -/
inductive Statement where
  | call (op : MutexOp) (line : Line)
  | print (line : Line)
deriving DecidableEq, Repr

/-- What a statement prints. -/
def observe : Statement → Line
  | .call _ line => line
  | .print line => line

/-- The mutex operations a statement performs. -/
def operationOf : Statement → List MutexOp
  | .call op _ => [op]
  | .print _ => []

/-- The mutex operations an *event* calls for, independently of any rendering. -/
def mutexOpsOfEvent : Event → List MutexOp
  | .createMutex m => [.init m]
  | .lock m _ => [.acquire m]
  | .unlock m _ => [.release m]
  | .destroyMutex m => [.destroy m]
  | .write _ => []
  | .read _ => []
  -- The claim of this card, at the level of one event.
  | .grow .. => []

/-- The generation a guard names, found by walking back to the `lock` that minted
it — the model of `holding_mutex`. The walk is over the events already rendered,
most recent first. A guard the trace never minted is `none`: a refusal, not a
guess. -/
def holdingMutex : List Event → Nat → Option Nat
  | [], _ => none
  | .lock m g :: rest, guard => if g = guard then some m else holdingMutex rest guard
  | _ :: rest, guard => holdingMutex rest guard

/-- Does the certificate admit the growth at `index`?

Every field of the event is compared against the certificate's own record; the
generation comes from the guard the trace really minted, never from the record;
and the certificate's effective shape must carry the grown interval, so a
certificate that admits a growth its own published shape denies is refused
rather than rendered. -/
def admits (cert : Certificate) (index task guard : Nat) (footprint : Footprint)
    (invariant mutex : Nat) : Bool :=
  cert.growths.any (fun g =>
      g.event == index && g.task == task && g.guard == guard && g.mutex == mutex &&
        g.footprint == footprint && g.toInvariant == invariant) &&
    cert.protectedShape.contains (mutex, footprint)

/-- The statements one event renders as, or `none` if the backend refuses it. -/
def stepStatements (cert : Certificate) (rendered : List Event) (index : Nat) :
    Event → Option (List Statement)
  | .createMutex m => some [.call (.init m) (.created m)]
  | .lock m g => some [.call (.acquire m) (.locked m g)]
  | .unlock m g => some [.call (.release m) (.unlocked m g)]
  | .write f => some [.print (.wrote f)]
  | .read f => some [.print (.readBack f)]
  | .destroyMutex m => some [.call (.destroy m) (.destroyed m)]
  | .grow task guard f invariant =>
    match holdingMutex rendered guard with
    | none => none
    | some mutex =>
      if admits cert index task guard f invariant mutex then
        -- The whole of a growth's rendering: one bookkeeping line, no call.
        some [.print (.grew mutex guard f)]
      else
        none

/-- The observation lines one event contributes, written against the event list
rather than against the emitted statements. -/
def stepLines (cert : Certificate) (rendered : List Event) (index : Nat) :
    Event → Option (List Line)
  | .createMutex m => some [.created m]
  | .lock m g => some [.locked m g]
  | .unlock m g => some [.unlocked m g]
  | .write f => some [.wrote f]
  | .read f => some [.readBack f]
  | .destroyMutex m => some [.destroyed m]
  | .grow task guard f invariant =>
    match holdingMutex rendered guard with
    | none => none
    | some mutex =>
      if admits cert index task guard f invariant mutex then
        some [.grew mutex guard f]
      else
        none

/-- `emit_mutex_c11`, walking the trace with the events already rendered and the
current event index. -/
def emitFrom (cert : Certificate) (rendered : List Event) (index : Nat) :
    List Event → Option (List Statement)
  | [] => some []
  | event :: rest =>
    match stepStatements cert rendered index event with
    | none => none
    | some head =>
      match emitFrom cert (event :: rendered) (index + 1) rest with
      | none => none
      | some tail => some (head ++ tail)

/-- `interpret_mutex_program`, the same walk over the independently written
line function. -/
def interpretFrom (cert : Certificate) (rendered : List Event) (index : Nat) :
    List Event → Option (List Line)
  | [] => some []
  | event :: rest =>
    match stepLines cert rendered index event with
    | none => none
    | some head =>
      match interpretFrom cert (event :: rendered) (index + 1) rest with
      | none => none
      | some tail => some (head ++ tail)

/-- The emitted program. -/
def emit (cert : Certificate) (events : List Event) : Option (List Statement) :=
  emitFrom cert [] 0 events

/-- The reference execution. -/
def interpret (cert : Certificate) (events : List Event) : Option (List Line) :=
  interpretFrom cert [] 0 events

/-- The mutex operations a rendering performs, in order. -/
def operations (statements : List Statement) : List MutexOp :=
  statements.flatMap operationOf

/-- The trace with every growth deleted. -/
def withoutGrowths : List Event → List Event
  | [] => []
  | .grow .. :: rest => withoutGrowths rest
  | event :: rest => event :: withoutGrowths rest

/-- Whether an event is a growth. -/
def isGrowth : Event → Bool
  | .grow .. => true
  | _ => false

/-! ## The two renderings agree -/

/-- Per event: the statements' observation is exactly the lines, and the two
functions refuse on exactly the same events. -/
theorem step_renderings_agree (cert : Certificate) (rendered : List Event) (index : Nat)
    (event : Event) :
    (stepStatements cert rendered index event).map (List.map observe) =
      stepLines cert rendered index event := by
  cases event with
  | grow task guard f invariant =>
    simp only [stepStatements, stepLines]
    cases holdingMutex rendered guard with
    | none => rfl
    | some mutex =>
      by_cases h : admits cert index task guard f invariant mutex <;> simp [h, observe]
  | _ => rfl

/-- The emitted program's observation is the reference execution, on every trace:
both refuse exactly the same traces, and on the rest they produce exactly the
same lines. This is the model of the line-for-line comparison the CT harness
performs on a real compiled and sanitizer-run program. -/
theorem emitted_program_observes_the_reference
    (cert : Certificate) (rendered : List Event) (index : Nat) (events : List Event) :
    (emitFrom cert rendered index events).map (List.map observe) =
      interpretFrom cert rendered index events := by
  induction events generalizing rendered index with
  | nil => rfl
  | cons event rest ih =>
    have step := step_renderings_agree cert rendered index event
    have tail := ih (event :: rendered) (index + 1)
    simp only [emitFrom, interpretFrom]
    cases hs : stepStatements cert rendered index event with
    | none =>
      rw [hs] at step
      simp only [Option.map_none] at step
      rw [← step]
      simp
    | some produced =>
      rw [hs] at step
      simp only [Option.map_some] at step
      rw [← step]
      cases ht : emitFrom cert (event :: rendered) (index + 1) rest with
      | none =>
        rw [ht] at tail
        simp only [Option.map_none] at tail
        rw [← tail]
        simp
      | some rendered' =>
        rw [ht] at tail
        simp only [Option.map_some] at tail
        rw [← tail]
        simp

/-- The whole-program corollary. -/
theorem emitted_and_reference_renderings_agree (cert : Certificate) (events : List Event) :
    (emit cert events).map (List.map observe) = interpret cert events :=
  emitted_program_observes_the_reference cert [] 0 events

/-! ## A growth performs no mutex operation -/

/-- A rendered growth is one bare `printf`. -/
theorem a_growth_statement_performs_no_mutex_operation (mutex guard : Nat) (f : Footprint) :
    operationOf (.print (.grew mutex guard f)) = [] := rfl

/-- Per event: whatever the backend renders an event as, the mutex operations it
performs are the ones the event itself calls for — and a growth calls for none. -/
theorem step_operations_are_the_events_own (cert : Certificate) (rendered : List Event)
    (index : Nat) (event : Event) (head : List Statement)
    (produced : stepStatements cert rendered index event = some head) :
    operations head = mutexOpsOfEvent event := by
  cases event with
  | grow task guard f invariant =>
    simp only [stepStatements] at produced
    cases hm : holdingMutex rendered guard with
    | none => rw [hm] at produced; exact absurd produced (by simp)
    | some mutex =>
      rw [hm] at produced
      by_cases h : admits cert index task guard f invariant mutex
      · simp only [h, ite_true] at produced
        cases produced
        rfl
      · simp only [h] at produced
        exact absurd produced (by simp)
  | _ => cases produced; rfl

/-- The mutex operations of a rendering are exactly the trace's own, computed
from the events without looking at the emitted C at all. -/
theorem operations_are_exactly_the_traces_own_mutex_calls
    (cert : Certificate) (rendered : List Event) (index : Nat) (events : List Event)
    (statements : List Statement)
    (produced : emitFrom cert rendered index events = some statements) :
    operations statements = events.flatMap mutexOpsOfEvent := by
  induction events generalizing rendered index statements with
  | nil => cases produced; rfl
  | cons event rest ih =>
    simp only [emitFrom] at produced
    cases hs : stepStatements cert rendered index event with
    | none => rw [hs] at produced; exact absurd produced (by simp)
    | some head =>
      rw [hs] at produced
      cases ht : emitFrom cert (event :: rendered) (index + 1) rest with
      | none => rw [ht] at produced; exact absurd produced (by simp)
      | some tail =>
        rw [ht] at produced
        cases produced
        have hhead := step_operations_are_the_events_own cert rendered index event head hs
        have htail := ih (rendered := event :: rendered) (index := index + 1) tail ht
        simp only [operations, List.flatMap_append, List.flatMap_cons] at *
        rw [hhead, htail]

/-- Deleting every growth from a trace leaves its mutex operations unchanged.
Together with the theorem above: growth is invisible to the `mtx_t`. -/
theorem growth_contributes_no_mutex_operation (events : List Event) :
    events.flatMap mutexOpsOfEvent = (withoutGrowths events).flatMap mutexOpsOfEvent := by
  induction events with
  | nil => rfl
  | cons event rest ih =>
    cases event <;> simp [withoutGrowths, mutexOpsOfEvent, ih]

/-! ## Fail-closed: only an admitted growth renders -/

/-- A growth whose guard the trace never minted is refused. -/
theorem growth_requires_a_minted_guard (cert : Certificate) (rendered : List Event)
    (index task guard : Nat) (f : Footprint) (invariant : Nat)
    (unminted : holdingMutex rendered guard = none) :
    stepStatements cert rendered index (.grow task guard f invariant) = none := by
  simp [stepStatements, unminted]

/-- A growth the certificate does not admit is refused. -/
theorem unadmitted_growth_is_refused (cert : Certificate) (rendered : List Event)
    (index task guard : Nat) (f : Footprint) (invariant mutex : Nat)
    (minted : holdingMutex rendered guard = some mutex)
    (unadmitted : admits cert index task guard f invariant mutex = false) :
    stepStatements cert rendered index (.grow task guard f invariant) = none := by
  simp [stepStatements, minted, unadmitted]

/-- A certificate carrying no record at this event index admits nothing there. -/
theorem growth_requires_a_record_at_this_index (cert : Certificate)
    (index task guard : Nat) (f : Footprint) (invariant mutex : Nat)
    (absent : ∀ g ∈ cert.growths, g.event ≠ index) :
    admits cert index task guard f invariant mutex = false := by
  simp only [admits, Bool.and_eq_false_iff, List.any_eq_false]
  left
  intro g hg
  have mismatch : (g.event == index) = false := beq_eq_false_iff_ne.mpr (absent g hg)
  simp [mismatch]

/-- A growth the certificate's own effective shape does not carry is refused,
even where a record admits it: a certificate that disagrees with itself renders
nothing. -/
theorem growth_requires_the_published_shape (cert : Certificate)
    (index task guard : Nat) (f : Footprint) (invariant mutex : Nat)
    (hidden : cert.protectedShape.contains (mutex, f) = false) :
    admits cert index task guard f invariant mutex = false := by
  simp only [admits, Bool.and_eq_false_iff]
  exact Or.inr hidden

/-- A refusal at the head refuses the whole program: there is no partial
rendering. -/
theorem head_refusal_refuses_the_program (cert : Certificate) (rendered : List Event)
    (index : Nat) (event : Event) (rest : List Event)
    (refused : stepStatements cert rendered index event = none) :
    emitFrom cert rendered index (event :: rest) = none := by
  simp [emitFrom, refused]

/-- And a refusal in the tail refuses it too. -/
theorem tail_refusal_refuses_the_program (cert : Certificate) (rendered : List Event)
    (index : Nat) (event : Event) (rest : List Event)
    (refused : emitFrom cert (event :: rendered) (index + 1) rest = none) :
    emitFrom cert rendered index (event :: rest) = none := by
  simp only [emitFrom]
  cases stepStatements cert rendered index event with
  | none => rfl
  | some head => simp [refused]

/-- One refused growth refuses the whole program, wherever it sits. -/
theorem a_refused_growth_refuses_the_whole_program (cert : Certificate)
    (before : List Event) (rendered : List Event) (index : Nat)
    (task guard : Nat) (f : Footprint) (invariant : Nat) (after : List Event)
    (refused : ∀ seen position,
      stepStatements cert seen position (.grow task guard f invariant) = none) :
    emitFrom cert rendered index (before ++ .grow task guard f invariant :: after) = none := by
  induction before generalizing rendered index with
  | nil => exact head_refusal_refuses_the_program cert rendered index _ after (refused _ _)
  | cons event rest ih =>
    exact tail_refusal_refuses_the_program cert rendered index event _
      (ih (rendered := event :: rendered) (index := index + 1))

/-- The converse direction: a growth that *did* render was admitted, and rendered
as exactly one bookkeeping line naming the generation the trace really locked. -/
theorem a_rendered_growth_was_admitted (cert : Certificate) (rendered : List Event)
    (index task guard : Nat) (f : Footprint) (invariant : Nat) (head : List Statement)
    (produced : stepStatements cert rendered index (.grow task guard f invariant) = some head) :
    ∃ mutex, holdingMutex rendered guard = some mutex ∧
      admits cert index task guard f invariant mutex = true ∧
      head = [.print (.grew mutex guard f)] := by
  simp only [stepStatements] at produced
  cases hm : holdingMutex rendered guard with
  | none => rw [hm] at produced; exact absurd produced (by simp)
  | some mutex =>
    rw [hm] at produced
    by_cases h : admits cert index task guard f invariant mutex
    · simp only [h, ite_true, Option.some.injEq] at produced
      exact ⟨mutex, rfl, h, produced.symm⟩
    · simp only [h] at produced
      exact absurd produced (by simp)

/-- A trace that never grows renders the same under every certificate: this
card's admission cannot move a module that does not use it. -/
theorem a_growthless_trace_renders_independently_of_the_certificate
    (first second : Certificate) (rendered : List Event) (index : Nat) (events : List Event)
    (growthless : ∀ e ∈ events, isGrowth e = false) :
    emitFrom first rendered index events = emitFrom second rendered index events := by
  induction events generalizing rendered index with
  | nil => rfl
  | cons event rest ih =>
    have hg : isGrowth event = false := growthless event (by simp)
    have head : stepStatements second rendered index event
        = stepStatements first rendered index event := by
      cases event <;> simp [isGrowth] at hg <;> rfl
    have tail := ih (rendered := event :: rendered) (index := index + 1)
      (fun e he => growthless e (List.mem_cons_of_mem event he))
    simp only [emitFrom, head, tail]

/-! ## Decidable scenarios

The trace of `compiler/tests/fixtures/ct/CT34/growth_then_guarded_access.qr`:
one generation over `x`, a section that writes `x`, grows onto `y` and writes
`y`, then a second section that reads both. -/

/-- `x[0,4)`. -/
def x : Footprint := ⟨1, 0, 4⟩
/-- `y[0,4)`. -/
def y : Footprint := ⟨2, 0, 4⟩

/-- The fixture's event list. -/
def scenario : List Event :=
  [.createMutex 1, .lock 1 1, .write x, .grow 0 1 y 2, .write y, .unlock 1 1,
    .lock 1 2, .read x, .read y, .unlock 1 2, .destroyMutex 1]

/-- The certificate the checker issues for it: one admitted growth at event 3,
and an effective shape carrying both intervals. -/
def scenarioCertificate : Certificate :=
  { growths := [⟨3, 0, 1, 1, y, 2⟩], protectedShape := [(1, x), (1, y)] }

/-- The fixture renders, and its observation is the reference execution's. -/
theorem the_fixture_renders_and_agrees :
    (emit scenarioCertificate scenario).map (List.map observe) =
      interpret scenarioCertificate scenario := by
  decide

/-- Its observation, in full. The `grew` line sits between the two writes, which
is what makes the second write an access to memory the growth had just brought
under the guard. -/
theorem the_fixture_observation_is_pinned :
    interpret scenarioCertificate scenario = some
      [.created 1, .locked 1 1, .wrote x, .grew 1 1 y, .wrote y, .unlocked 1 1,
        .locked 1 2, .readBack x, .readBack y, .unlocked 1 2, .destroyed 1] := by
  decide

/-- Six mutex operations: one `mtx_init`, two `mtx_lock`, two `mtx_unlock` and
one `mtx_destroy` — exactly what the same two sections without a growth perform.
This is the decidable instance of `operations_are_exactly_the_traces_own_mutex_calls`
and `growth_contributes_no_mutex_operation` together. -/
theorem the_fixture_performs_six_mutex_operations :
    (emit scenarioCertificate scenario).map operations = some
      [.init 1, .acquire 1, .release 1, .acquire 1, .release 1, .destroy 1] := by
  decide

/-- And the growth-free trace performs the same six. -/
theorem deleting_the_growth_moves_no_mutex_operation :
    scenario.flatMap mutexOpsOfEvent = (withoutGrowths scenario).flatMap mutexOpsOfEvent := by
  decide

/-- An emptied record refuses the whole program. -/
theorem the_fixture_with_no_admitted_growth_is_refused :
    emit { scenarioCertificate with growths := [] } scenario = none := by
  decide

/-- So does a record moved to another event index. -/
theorem the_fixture_with_a_relocated_record_is_refused :
    emit { scenarioCertificate with growths := [⟨4, 0, 1, 1, y, 2⟩] } scenario = none := by
  decide

/-- So does a record naming another interval, another predicate identity, or
another task. -/
theorem the_fixture_with_an_edited_record_is_refused :
    emit { scenarioCertificate with growths := [⟨3, 0, 1, 1, x, 2⟩] } scenario = none ∧
      emit { scenarioCertificate with growths := [⟨3, 0, 1, 1, y, 9⟩] } scenario = none ∧
        emit { scenarioCertificate with growths := [⟨3, 7, 1, 1, y, 2⟩] } scenario = none := by
  decide

/-- So does a shape that hides the growth the record admits. -/
theorem the_fixture_with_a_hidden_shape_is_refused :
    emit { scenarioCertificate with protectedShape := [(1, x)] } scenario = none := by
  decide

/-- And a growth naming a guard no section minted is refused before the
certificate is consulted at all. -/
theorem a_growth_under_an_unminted_guard_is_refused :
    emit scenarioCertificate [.createMutex 1, .grow 0 1 y 2] = none := by
  decide

end QCore.ProtectionGrowthEmission
