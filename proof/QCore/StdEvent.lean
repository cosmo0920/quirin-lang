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

/-! `source.event.v1` — the determinism boundary and the reactor projection of
`std.event` (card `STD-EVENT`).

# What is proved here, and what is deliberately not

This card's Rust side does **two** things: it projects a source statement
sequence onto the existing reactor model, and it decides — over its own
readiness ledger — whether each dispatch turn has one well-defined observation.
The custody half is `QCore.DescriptorCustody`'s and the reactor checker's, and
it is **not reconstructed here**: this module proves the two claims that are
this card's own.

1. **The determinism boundary.** A dispatch turn is admitted only where the
   ready set the ledger derives is exactly the one the statement declares, and a
   set with two members is admitted by *no* declaration at all. That is the
   by-name refusal the card rests on, and `two_ready_registrations_admit_no_turn`
   is it. The ground is that `poll(2)` reports a set and the order a reactor runs
   one ready set's callbacks in is the implementation's — the existing model says
   so itself (`explore_activation_orders`). Covering such a turn waits on
   comparison up to an admitted reordering (card `OBS-REORDERING`); this boundary
   does not depend on that card.

2. **Readiness is induced by the program alone.** `readiness_comes_only_from_a_wake`
   and `a_program_with_no_wake_admits_no_ready_turn` are the formal content of
   "v1 waits only on readiness this program caused". They are what makes the
   platform premise `EVENT-V1-LEVEL-TRIGGERED-READINESS` the *only* thing about
   the outside world a v1 verdict rests on.

The projection theorems state the surface table as code: a `wake` projects to
nothing (the reactor models no byte flow and this card invents none beside it),
a `watch` projects to the reactor's own registration pair, and every registration
carries priority zero — which is why a two-member ready set is *refused* rather
than *ordered*.

# What is not proved here

That the Rust lowering extracts these statements faithfully from source tokens;
that the emitted C's `poll` call reports what this ledger predicts (that is a
declared premise, **checked at run time** by the emitted program and compared by
the Rust tests, never proved); anything about durability, buffering, or a
descriptor's real number. Identities are `Nat` here because this module models
the program *after* name resolution — the names-to-identities step is the Rust
lowering's, and is covered by its own tests. -/

namespace QCore.StdEvent

/-! ## Explicit diagnostic logging

The event trace remains a structured observation. `log_level` is the separate,
source-selected adapter which may render that observation on stdout. Absence is
the null sink; a selected threshold admits its level and every more severe
level. -/

inductive DiagnosticLevel where
  | trace
  | debug
  | info
  | warn
  | error
  | fatal
  deriving DecidableEq, BEq, Repr

def DiagnosticLevel.rank : DiagnosticLevel → Nat
  | .trace => 0
  | .debug => 1
  | .info => 2
  | .warn => 3
  | .error => 4
  | .fatal => 5

def diagnosticEnabled : Option DiagnosticLevel → DiagnosticLevel → Bool
  | none, _ => false
  | some threshold, level => threshold.rank ≤ level.rank

theorem an_absent_log_level_is_quiet (level : DiagnosticLevel) :
    diagnosticEnabled none level = false := rfl

theorem trace_log_level_admits_every_level (level : DiagnosticLevel) :
    diagnosticEnabled (some .trace) level = true := by
  cases level <;> decide

/-- What a registration watches for. `read` is the executable arm: its readiness
is induced by the program itself. `write` is check-only — see
`a_write_registration_is_never_ready`. -/
inductive Interest where
  | read
  | write
  deriving DecidableEq, BEq, Repr

structure Registration where
  event : Nat
  endpoint : Nat
  interest : Interest
  deriving DecidableEq, BEq, Repr

/-- One statement of an entry, after name resolution. -/
inductive Statement where
  | logLevel (level : DiagnosticLevel)
  | contextOpen
  | contextClose
  | endpoints (first second : Nat)
  | watch (registration : Registration)
  | unwatch (event : Nat)
  /-- `wake D;` — one byte into `D`'s peer, so a read registration on `D`
  becomes ready. It projects to no reactor operation at all. -/
  | wake (endpoint : Nat)
  | dispatchReady (event : Nat)
  | dispatchIdle
  | ret (status : Int)
  deriving DecidableEq, BEq, Repr

/-- **The readiness ledger.** It models what this program did to itself, never
what the kernel did: `pending` is the multiset of endpoints this program wrote a
byte into and has not read back. -/
structure State where
  watched : List Registration := []
  pending : List Nat := []
  deriving DecidableEq, Repr

/-- The registrations the ledger says are ready.

A **write** registration is never here: v1 induces no send-buffer pressure, so
it has nothing to say about one. -/
def isRead : Interest → Bool
  | .read => true
  | .write => false

def readySet (s : State) : List Registration :=
  s.watched.filter fun r => isRead r.interest && s.pending.contains r.endpoint

/-- Whether a statement's declared readiness is the one the ledger derives.

This is the determinism boundary, in one function. Note that
`dispatchReady` matches **only** a one-element ready set: a two-member set
admits no declaration, which is the by-name refusal. -/
def admits (s : State) : Statement → Bool
  | .dispatchReady e =>
      match readySet s with
      | [r] => r.event == e
      | _ => false
  | .dispatchIdle => (readySet s).isEmpty
  | _ => true

/-- The ledger's transition. A ready turn drains exactly one byte — the whole
body of a v1 callback. -/
def step (s : State) : Statement → State
  | .watch r => { s with watched := r :: s.watched }
  | .unwatch e => { s with watched := s.watched.filter fun r => r.event != e }
  | .wake d => { s with pending := d :: s.pending }
  | .dispatchReady _ =>
      match readySet s with
      | [r] => { s with pending := s.pending.erase r.endpoint }
      | _ => s
  | _ => s

/-- Run a program, refusing at the first statement whose declaration the ledger
does not support. `none` is a refusal. -/
def run (s : State) : List Statement → Option State
  | [] => some s
  | statement :: rest =>
      if admits s statement then run (step s statement) rest else none

/-! ## The determinism boundary -/

/-- **The by-name refusal.** A ready set with two or more members admits no
dispatch declaration whatsoever — not the one naming the first, not the one
naming the second, and not the idle one.

This is the formal content of the card's boundary. `poll(2)` reports a set; the
order a reactor runs one ready set's callbacks in is the implementation's, not
one the program induced. A certificate compares one observation, so a turn with
more than one ready registration is refused rather than ordered. -/
theorem two_ready_registrations_admit_no_turn
    (s : State) (a b : Registration) (rest : List Registration)
    (h : readySet s = a :: b :: rest) :
    ∀ e : Nat, admits s (.dispatchReady e) = false := by
  intro e
  simp [admits, h]

theorem two_ready_registrations_admit_no_idle_turn
    (s : State) (a b : Registration) (rest : List Registration)
    (h : readySet s = a :: b :: rest) :
    admits s .dispatchIdle = false := by
  simp [admits, h]

/-- The dual: an idle turn is admitted exactly when nothing is ready. -/
theorem an_idle_turn_is_admitted_exactly_when_nothing_is_ready (s : State) :
    admits s .dispatchIdle = true ↔ readySet s = [] := by
  simp [admits, List.isEmpty_iff]

/-- A declared ready turn names the one ready registration and no other. -/
theorem a_ready_turn_names_the_one_ready_registration
    (s : State) (e : Nat) (r : Registration) (hr : readySet s = [r])
    (h : admits s (.dispatchReady e) = true) : r.event = e := by
  rw [admits, hr] at h
  simpa using h

/-- And an empty ready set admits no ready declaration at all. -/
theorem an_empty_ready_set_admits_no_ready_turn
    (s : State) (hr : readySet s = []) : ∀ e : Nat, admits s (.dispatchReady e) = false := by
  intro e
  simp [admits, hr]

/-- Every ready registration is a **read** registration on an endpoint this
program has written a byte into and not read back. Nothing else can be ready,
which is the whole content of "readiness is induced by the program alone". -/
theorem a_ready_registration_is_a_read_on_a_pending_endpoint
    (s : State) (r : Registration) (h : r ∈ readySet s) :
    r ∈ s.watched ∧ r.interest = Interest.read ∧ s.pending.contains r.endpoint = true := by
  unfold readySet at h
  rw [List.mem_filter, Bool.and_eq_true] at h
  obtain ⟨member, condition⟩ := h
  refine ⟨member, ?_, condition.2⟩
  cases interest : r.interest with
  | read => rfl
  | write =>
    rw [interest] at condition
    exact absurd condition.1 (by simp [isRead])

/-- **A write registration is never ready.** The model expresses it and the
reactor checks it; this route induces no send-buffer pressure, so the ledger has
nothing to say about it. That is exactly why a module containing one is accepted
and checked but not executed. -/
theorem a_write_registration_is_never_ready
    (s : State) (r : Registration) (h : r.interest = Interest.write) :
    r ∉ readySet s := by
  intro member
  have ready := a_ready_registration_is_a_read_on_a_pending_endpoint s r member
  rw [h] at ready
  exact absurd ready.2.1 (by decide)

/-- **A program with no induced readiness admits no ready turn.** With an empty
ledger nothing is ready, so every `dispatch ready E;` is refused — the refusal
is the absence of an induced wake, not a rule about that statement. -/
theorem a_program_with_no_wake_admits_no_ready_turn (s : State) (h : s.pending = []) :
    ∀ e : Nat, admits s (.dispatchReady e) = false := by
  have empty : readySet s = [] := by
    unfold readySet
    rw [List.filter_eq_nil_iff]
    intro r _
    simp [h]
  exact an_empty_ready_set_admits_no_ready_turn s empty

/-- **Only a wake grows the ledger.** Every other statement leaves `pending` no
longer than it was, so readiness cannot appear from anywhere else. -/
theorem readiness_comes_only_from_a_wake (s : State) (statement : Statement)
    (h : ∀ d, statement ≠ .wake d) :
    (step s statement).pending.length ≤ s.pending.length := by
  cases statement with
  | wake d => exact absurd rfl (h d)
  | dispatchReady e =>
    unfold step
    cases readySet s with
    | nil => simp
    | cons head tail =>
      cases tail with
      | nil => simpa using List.length_erase_le (l := s.pending) (a := head.endpoint)
      | cons _ _ => simp
  | _ => simp [step]

/-- **A ready turn drains exactly one byte.** This is why the canonical
program's *following* turn is idle: the callback consumed the one readiness the
program induced. -/
theorem a_ready_turn_drains_exactly_one_byte
    (s : State) (e : Nat) (r : Registration) (h : readySet s = [r]) :
    (step s (.dispatchReady e)).pending = s.pending.erase r.endpoint := by
  unfold step
  rw [h]

/-! ## The projection onto the reactor model

The operation vocabulary below is the subset of
`qcore_core::event_reactor::EventOperation` that `source.event.v1` produces. It
is a *projection target*, not a second reactor: what these operations mean is
decided by the reactor checker and by `QCore.DescriptorCustody`, and nothing
here restates either. -/

inductive Operation where
  | createBase
  | freeBase
  | createPair (first second : Nat)
  | closeSocket (endpoint : Nat)
  | assign (event endpoint priority : Nat) (interest : Interest)
  | add (event : Nat)
  | beginDispatch
  | activate (event : Nat)
  | callbackEnter (event : Nat)
  | callbackExit (event : Nat)
  | endDispatch
  | delete (event : Nat)
  | freeEvent (event : Nat)
  deriving DecidableEq, BEq, Repr

/-- The surface table of `docs/design/STD_EVENT_V1.md` §5, as code. -/
def project : Statement → List Operation
  | .logLevel _ => []
  | .contextOpen => [.createBase]
  | .contextClose => [.freeBase]
  | .endpoints a b => [.createPair a b]
  | .watch r => [.assign r.event r.endpoint 0 r.interest, .add r.event]
  | .unwatch e => [.delete e, .freeEvent e]
  | .wake _ => []
  | .dispatchReady e =>
      [.beginDispatch, .activate e, .callbackEnter e, .callbackExit e, .endDispatch]
  | .dispatchIdle => [.beginDispatch, .endDispatch]
  | .ret _ => []

def projectProgram (p : List Statement) : List Operation :=
  (p.map project).flatten

/-- **A wake projects to nothing.** The reactor models no byte flow, and this
card does not invent one beside it: the readiness induction lives entirely in
this module's own ledger. -/
theorem a_wake_projects_to_nothing (d : Nat) : project (.wake d) = [] := rfl

def isWake : Statement → Bool
  | .wake _ => true
  | _ => false

theorem projectProgram_cons (head : Statement) (tail : List Statement) :
    projectProgram (head :: tail) = project head ++ projectProgram tail := by
  simp [projectProgram]

/-- The general form of `a_wake_projects_to_nothing`: a wake, and nothing else,
is a statement with no reactor content. -/
theorem only_a_wake_projects_to_nothing (statement : Statement)
    (h : isWake statement = true) : project statement = [] := by
  cases statement <;> simp [isWake, project] at h ⊢

/-- The strong form: **erasing every wake from a program leaves its reactor
projection byte for byte.** So whatever the reactor concludes about a program's
custody, it concludes about the same program without its readiness inductions —
which is what licenses saying that `wake` is outside the custody model rather
than a hidden part of it. -/
theorem erasing_the_wakes_leaves_the_projection_unchanged (p : List Statement) :
    projectProgram (p.filter fun s => !isWake s) = projectProgram p := by
  induction p with
  | nil => rfl
  | cons head tail ih =>
    by_cases wake : isWake head = true
    · rw [List.filter_cons_of_neg (by simp [wake]), ih, projectProgram_cons,
        only_a_wake_projects_to_nothing head wake, List.nil_append]
    · rw [List.filter_cons_of_pos (by simp [wake]), projectProgram_cons,
        projectProgram_cons, ih]

/-- **A registration is the reactor's own registration pair**, naming the
descriptor. This is the whole content of the claim that `watch E D read;` *is*
the existing model's custody edge rather than a new one beside it. -/
theorem a_watch_projects_to_the_reactor_registration (r : Registration) :
    project (.watch r) = [.assign r.event r.endpoint 0 r.interest, .add r.event] := rfl

/-- **Every registration carries priority zero.** This is not an incidental
choice: a priority order would be a source-declared ordering premise, and v1
declares none — which is precisely why a two-member ready set is refused rather
than ordered. -/
theorem every_registration_has_priority_zero
    (p : List Statement) (event endpoint priority : Nat) (interest : Interest)
    (h : Operation.assign event endpoint priority interest ∈ projectProgram p) :
    priority = 0 := by
  unfold projectProgram at h
  simp only [List.mem_flatten, List.mem_map] at h
  obtain ⟨_, ⟨statement, _, rfl⟩, member⟩ := h
  cases statement <;> simp [project] at member
  exact member.2.2.1

/-- A dispatch turn brackets its callback, and an idle turn brackets nothing.
The reactor's own `EndDispatch` refuses a non-quiescent close, so this bracket
is what makes the turn checkable at all. -/
theorem a_ready_turn_brackets_exactly_one_callback (e : Nat) :
    project (.dispatchReady e) =
      [.beginDispatch, .activate e, .callbackEnter e, .callbackExit e, .endDispatch] := rfl

theorem an_idle_turn_runs_no_callback (e : Nat) :
    Operation.callbackEnter e ∉ project .dispatchIdle := by
  simp [project]

/-! ## Executability -/

def isWriteInterest : Statement → Bool
  | .watch r => !isRead r.interest
  | _ => false

/-- Whether v1 can *execute* a program: it can when no registration watches for
writability. The check is unaffected either way — see the module header. -/
def executable (p : List Statement) : Bool := !p.any isWriteInterest

theorem a_write_interest_is_not_executable (p : List Statement) (r : Registration)
    (interest : r.interest = Interest.write) (h : Statement.watch r ∈ p) :
    executable p = false := by
  unfold executable
  simp only [Bool.not_eq_false', List.any_eq_true]
  exact ⟨.watch r, h, by simp [isWriteInterest, interest, isRead]⟩

/-! ## The derived effect row -/

inductive Effect where
  | io
  | alloc
  | mut
  deriving DecidableEq, Repr

def touchesTheOs : Statement → Bool
  | .logLevel _ => true
  | .ret _ => false
  | _ => true

def derivedEffects (p : List Statement) : List Effect :=
  if p.any touchesTheOs then [Effect.io] else []

/-- **No program on this route carries `alloc` or `mut`.** Not because the
checker refuses them: the grammar has no value vocabulary at all, so there is no
statement that could allocate or mutate. -/
theorem no_program_carries_alloc_or_mut (p : List Statement) :
    Effect.alloc ∉ derivedEffects p ∧ Effect.mut ∉ derivedEffects p := by
  unfold derivedEffects
  split <;> simp

theorem a_program_of_returns_carries_nothing (status : Int) :
    derivedEffects [.ret status] = [] := by
  simp [derivedEffects, touchesTheOs]

theorem opening_a_context_carries_io :
    derivedEffects [.contextOpen, .contextClose, .ret 0] = [Effect.io] := by
  simp [derivedEffects, touchesTheOs]

/-- Logger selection is an explicit IO effect but remains outside the reactor
projection, so it cannot create or discharge custody. -/
theorem log_level_carries_io_and_projects_no_reactor_operation
    (level : DiagnosticLevel) :
    derivedEffects [.logLevel level, .ret 0] = [Effect.io]
      ∧ project (.logLevel level) = [] := by
  cases level <;> decide

/-! ## The canonical program

The fixture the Rust route compiles, runs and compares. Stated here as an
executable check rather than as prose, so that the design document's example and
this module cannot drift apart silently. -/

def canonicalRegistration : Registration := { event := 0, endpoint := 0, interest := .read }

def canonical : List Statement :=
  [ .contextOpen,
    .endpoints 0 1,
    .watch canonicalRegistration,
    .wake 0,
    .dispatchReady 0,
    .dispatchIdle,
    .unwatch 0,
    .ret 0 ]

theorem the_canonical_program_is_admitted : (run {} canonical).isSome = true := by decide

/-- Remove the one `wake`, and the ready turn is refused. The admission is the
induced readiness and nothing else. -/
theorem the_canonical_program_without_its_wake_is_refused :
    run {} (canonical.filter fun s => !isWake s) = none := by decide

/-- Two induced readinesses on two watched endpoints, and **no** declaration is
admitted — the by-name refusal, on a concrete program. -/
theorem two_induced_readinesses_refuse_every_declaration :
    run {}
      [ .contextOpen,
        .endpoints 0 1,
        .endpoints 2 3,
        .watch { event := 0, endpoint := 0, interest := .read },
        .watch { event := 1, endpoint := 2, interest := .read },
        .wake 0,
        .wake 2,
        .dispatchReady 0 ] = none := by decide

theorem two_induced_readinesses_refuse_the_other_declaration :
    run {}
      [ .contextOpen,
        .endpoints 0 1,
        .endpoints 2 3,
        .watch { event := 0, endpoint := 0, interest := .read },
        .watch { event := 1, endpoint := 2, interest := .read },
        .wake 0,
        .wake 2,
        .dispatchReady 1 ] = none := by decide

theorem two_induced_readinesses_refuse_the_idle_declaration :
    run {}
      [ .contextOpen,
        .endpoints 0 1,
        .endpoints 2 3,
        .watch { event := 0, endpoint := 0, interest := .read },
        .watch { event := 1, endpoint := 2, interest := .read },
        .wake 0,
        .wake 2,
        .dispatchIdle ] = none := by decide

/-- Draining the first readiness makes the second turn well defined again. The
boundary is on the *state*, not on the spelling. -/
theorem draining_one_readiness_admits_the_next_turn :
    (run {}
      [ .contextOpen,
        .endpoints 0 1,
        .watch { event := 0, endpoint := 0, interest := .read },
        .wake 0,
        .dispatchReady 0,
        .dispatchIdle ]).isSome = true := by decide

/-- A write registration is checked exactly as a read one is — the run admits
it — and is separately reported as not executable. Both halves, together, are
the `accept_check_only` arm. -/
theorem a_write_registration_is_checked_but_not_executable :
    (run {}
      [ .contextOpen,
        .endpoints 0 1,
        .watch { event := 0, endpoint := 0, interest := .write },
        .dispatchIdle,
        .unwatch 0,
        .ret 0 ]).isSome = true ∧
    executable
      [ .contextOpen,
        .endpoints 0 1,
        .watch { event := 0, endpoint := 0, interest := .write },
        .dispatchIdle,
        .unwatch 0,
        .ret 0 ] = false := by decide

end QCore.StdEvent
