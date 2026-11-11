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

/-! The **socket × event** composition — a checked socket as a reactor-registered
event source (card `SOCKET-EVENT-COMPOSE`).

Four claims are proved here, and they are the four the Rust composition rests on.

**A registration is a loan, and it moves no custody.** A `watch` acquires no
descriptor, an `unwatch` releases none, and neither does a dispatch turn or
either context statement. The only statements that move the descriptor ledger
are the two that moved it before this card — which is exactly why the socket
route's custody half can still be decided by the *unedited* cycle-3 ownership
checker, with the loan expressed as that IR's own `BorrowShared` rather than as a
second ownership model.

**The loan decides three things, and all three are the borrow rules.** A `close`
and a state-changing transport call are refused while the socket is borrowed
(they are the ownership IR's *drop* and *write*); a transfer is admitted (it is a
*read*, and a shared loan admits reads). "Deregister before close" is therefore a
consequence of the loan rather than a rule this composition wrote.

**A dispatch turn observes and drains nothing.** Level-triggered readiness: a
turn leaves the ledger, the loans and the readiness of every socket exactly as it
found them, so the work a turn announces is done by the entry's own transport
statements, which the socket model already checks.

**Erasing the reactor statements leaves the socket program.** The composition's
five new statements map a composed program to the bare `source.socket.v1` program
under `erase`, and the ledger's answer is unchanged by that map. So every theorem
`QCore.StdSocket` proves about a program is a theorem about the same program with
registrations in it — the `erase` shape `QCore.ComposeEventLines` used for a
source line, reused for the same reason.

What is **not** proved here: that the Rust lowering extracts these statements
faithfully from source tokens; the reactor model's own lifecycle rules
(`QCore.DescriptorCustody` owns them, and the Rust route hands them the
projection unedited); the transport half (`QCore.StdSocket`'s subject); or
anything about a failed primitive — the emitted program exits there. Partial
correctness only, stated rather than proved away. -/

namespace QCore.SocketEventCompose

/-- What a registration watches for. `read` is the executable arm; a `write`
registration is checked and never executed, because this route induces no
send-buffer pressure and so has nothing to say about its readiness. -/
inductive Interest where
  | read
  | write
  deriving DecidableEq, Repr

/-- The statements of a composed entry.

The transport statements are grouped by **what the ownership IR makes of them**,
because that is the only thing this module's claims depend on: `configure` is a
`bind`/`listen`/`connect` (an ownership *write*), `transfer` is a
`send`/`recv`/`sendto`/`recvfrom`/`accept` (an ownership *read*). Grouping them
here is not a simplification of the Rust route — it is the statement that the
loan discipline reads nothing else about them. -/
inductive Statement where
  /-- `let NAME = tcp;` / `let NAME = udp;` — acquires. -/
  | create (name : String)
  /-- `bind`, `listen`, `connect` — an ownership **write** of the socket. -/
  | configure (name : String)
  /-- `send`, `recv`, `sendto`, `recvfrom`, `accept` — an ownership **read**. -/
  | transfer (name : String)
  /-- `close NAME;` — releases. -/
  | close (name : String)
  /-- `event open;` -/
  | reactorOpen
  /-- `event close;` -/
  | reactorClose
  /-- `watch E S INTEREST;` — opens the loan. -/
  | watch (registration socket : String) (interest : Interest)
  /-- `unwatch E;` — ends it. -/
  | unwatch (registration : String)
  /-- `dispatch ready E;` / `dispatch idle;` -/
  | dispatch (ready : Option String)
  /-- A `source.std-io.v1` line, reused unchanged. -/
  | line
  /-- `return;` -/
  | ret
  deriving DecidableEq, Repr

/-! ## The descriptor ledger

A list of names, each holding exactly one descriptor: what the ownership
footprint carries on this route's own axis, exactly as `QCore.StdSocket` models
it. -/

abbrev Ledger := List String

def acquire (ledger : Ledger) (name : String) : Ledger := name :: ledger

def release (ledger : Ledger) (name : String) : Ledger :=
  ledger.filter (fun entry => entry ≠ name)

/-- The ledger after one statement.

Every arm is written out rather than caught by a wildcard, so a widening of the
statement grammar changes a theorem rather than slipping past one. Only the two
custody statements move it — and in particular **none of the five statements this
card adds does**. -/
def ledgerStep (ledger : Ledger) : Statement → Ledger
  | .create name => acquire ledger name
  | .close name => release ledger name
  | .configure _ => ledger
  | .transfer _ => ledger
  | .reactorOpen => ledger
  | .reactorClose => ledger
  | .watch _ _ _ => ledger
  | .unwatch _ => ledger
  | .dispatch _ => ledger
  | .line => ledger
  | .ret => ledger

def runLedger (ledger : Ledger) : List Statement → Ledger
  | [] => ledger
  | statement :: rest => runLedger (ledgerStep ledger statement) rest

/-! ## The loans

One entry per live registration: which registration, which socket it borrows, and
what it watches for. This is the `OwnershipOp::BorrowShared` the Rust route
builds, seen as a set of outstanding loans. -/

structure Loan where
  registration : String
  socket : String
  interest : Interest
  deriving DecidableEq, Repr

abbrev Loans := List Loan

/-- Whether the ledger still holds this socket. -/
def holds (ledger : Ledger) (name : String) : Bool :=
  ledger.any (fun entry => entry == name)

/-- Whether any live registration borrows this socket. -/
def borrowed (loans : Loans) (socket : String) : Bool :=
  loans.any (fun loan => loan.socket == socket)

/-- The loans after one statement. Again every arm is written out: only the two
registration statements move them. -/
def loanStep (loans : Loans) : Statement → Loans
  | .watch registration socket interest =>
      { registration := registration, socket := socket, interest := interest } :: loans
  | .unwatch registration => loans.filter (fun loan => loan.registration ≠ registration)
  | .create _ => loans
  | .configure _ => loans
  | .transfer _ => loans
  | .close _ => loans
  | .reactorOpen => loans
  | .reactorClose => loans
  | .dispatch _ => loans
  | .line => loans
  | .ret => loans

/-! ## The loan discipline

Three rules, and none of them is this composition's. They are what the *unedited*
cycle-3 ownership checker answers for an outstanding `BorrowShared`:
`DropWhileBorrowed`, `OwnerMutationWhileBorrowed`, and the admission of a read
through a shared loan. -/

def admits (ledger : Ledger) (loans : Loans) : Statement → Bool
  | .create _ => true
  | .close name => holds ledger name && !borrowed loans name
  | .configure name => holds ledger name && !borrowed loans name
  | .transfer name => holds ledger name
  | .watch _ socket _ => holds ledger socket
  | .unwatch _ => true
  | .reactorOpen => true
  | .reactorClose => true
  | .dispatch _ => true
  | .line => true
  | .ret => true

/-! ## A registration moves no custody -/

theorem a_registration_acquires_no_descriptor
    (ledger : Ledger) (registration socket : String) (interest : Interest) :
    ledgerStep ledger (.watch registration socket interest) = ledger := rfl

theorem retiring_a_registration_releases_no_descriptor
    (ledger : Ledger) (registration : String) :
    ledgerStep ledger (.unwatch registration) = ledger := rfl

theorem the_context_statements_move_no_custody (ledger : Ledger) :
    ledgerStep ledger .reactorOpen = ledger ∧ ledgerStep ledger .reactorClose = ledger :=
  ⟨rfl, rfl⟩

/-- The socket a registration watches is still held: a loan is not a transfer. -/
theorem a_watched_socket_is_still_held
    (ledger : Ledger) (registration socket : String) (interest : Interest)
    (held : socket ∈ ledger) :
    socket ∈ ledgerStep ledger (.watch registration socket interest) := held

/-! ## Dispatch neutrality -/

theorem a_dispatch_turn_changes_no_custody (ledger : Ledger) (declared : Option String) :
    ledgerStep ledger (.dispatch declared) = ledger := rfl

theorem a_dispatch_turn_changes_no_loan (loans : Loans) (declared : Option String) :
    loanStep loans (.dispatch declared) = loans := rfl

/-! ## The three loan rules, which are the borrow rules -/

theorem watching_borrows_the_socket
    (loans : Loans) (registration socket : String) (interest : Interest) :
    borrowed (loanStep loans (.watch registration socket interest)) socket = true := by
  simp [borrowed, loanStep]

theorem a_close_while_registered_is_refused
    (ledger : Ledger) (loans : Loans) (name : String)
    (loaned : borrowed loans name = true) :
    admits ledger loans (.close name) = false := by
  simp [admits, loaned]

theorem reconfiguring_a_registered_socket_is_refused
    (ledger : Ledger) (loans : Loans) (name : String)
    (loaned : borrowed loans name = true) :
    admits ledger loans (.configure name) = false := by
  simp [admits, loaned]

/-- The other side of the same rule, and the reason the composition is usable at
all: a transfer is a **read**, and a shared loan admits reads. -/
theorem a_transfer_on_a_registered_socket_is_admitted
    (ledger : Ledger) (loans : Loans) (name : String)
    (held : holds ledger name = true) :
    admits ledger loans (.transfer name) = true := by
  simp [admits, held]

/-! ## Deregistration ordering

Retiring a registration removes exactly its own loan, and leaves every other.
Together with the rule above, that is "deregister before close" as a
**consequence**: the close is admitted only once no loan on the socket is
outstanding. -/

theorem retiring_removes_exactly_its_own
    (loans : Loans) (registration : String) :
    ∀ loan ∈ loanStep loans (.unwatch registration), loan.registration ≠ registration := by
  intro loan mem
  simp [loanStep, List.mem_filter] at mem
  exact mem.2

theorem retiring_keeps_every_other_loan
    (loans : Loans) (registration : String) (loan : Loan)
    (other : loan.registration ≠ registration) (mem : loan ∈ loans) :
    loan ∈ loanStep loans (.unwatch registration) := by
  simp [loanStep, List.mem_filter]
  exact ⟨mem, other⟩

/-- The shape the Rust route's `Q083` rests on: once the one loan on a socket is
retired, the socket is no longer borrowed and its close is admitted. -/
theorem retiring_the_only_loan_admits_the_close
    (ledger : Ledger) (registration socket : String) (interest : Interest)
    (held : holds ledger socket = true) :
    admits ledger
      (loanStep [{ registration := registration, socket := socket, interest := interest }]
        (.unwatch registration))
      (.close socket) = true := by
  simp [admits, loanStep, borrowed, held]

/-! ## Lexical spans

A loan is an `OwnershipOp::BorrowShared`, whose body is a lexical span — the
ownership IR's borrow nodes "delimit lexical loan lifetimes" in their own words.
So registrations retire in the reverse of the order they are taken, and a crossed
pair is refused by name rather than approximated. -/

def isReactor : Statement → Bool
  | .reactorOpen => true
  | .reactorClose => true
  | .watch _ _ _ => true
  | .unwatch _ => true
  | .dispatch _ => true
  | .create _ => false
  | .configure _ => false
  | .transfer _ => false
  | .close _ => false
  | .line => false
  | .ret => false

def nested (stack : List String) : List Statement → Bool
  | [] => stack.isEmpty
  | .watch registration _ _ :: rest => nested (registration :: stack) rest
  | .unwatch registration :: rest =>
      match stack with
      | [] => false
      | top :: below => registration == top && nested below rest
  | _ :: rest => nested stack rest

theorem properly_nested_spans_are_admitted :
    nested [] [.watch "first" "a" .read, .watch "second" "b" .read,
               .unwatch "second", .unwatch "first"] = true := by
  decide

theorem crossed_spans_are_refused :
    nested [] [.watch "first" "a" .read, .watch "second" "b" .read,
               .unwatch "first", .unwatch "second"] = false := by
  decide

theorem a_registration_never_retired_is_refused :
    nested [] [.watch "first" "a" .read] = false := by
  decide

/-! ## Erasure: the composed program is the socket program

`erase` drops every statement this card adds. The ledger's answer is unchanged
under it, so every custody theorem about a bare `source.socket.v1` program is a
theorem about the same program with registrations in it. -/

def erase : List Statement → List Statement
  | [] => []
  | statement :: rest =>
      if isReactor statement then erase rest else statement :: erase rest

theorem erasing_the_reactor_statements_leaves_the_ledger :
    ∀ (statements : List Statement) (ledger : Ledger),
      runLedger ledger (erase statements) = runLedger ledger statements := by
  intro statements
  induction statements with
  | nil => intro _; rfl
  | cons statement rest ih =>
    intro ledger
    cases statement <;> simp [erase, isReactor, runLedger, ledgerStep, ih]

theorem erasing_a_composed_program_leaves_its_transport :
    erase [.reactorOpen, .create "a", .configure "a", .watch "r" "a" .read,
           .dispatch (some "r"), .transfer "a", .unwatch "r", .reactorClose,
           .close "a", .ret]
      = [.create "a", .configure "a", .transfer "a", .close "a", .ret] := by
  decide

/-! ## The determinism boundary

The readiness of a socket is a fact about **this program's own committed
history** — a connection a `connect` queued, bytes a `send` committed, a datagram
a `sendto` queued — and nothing else. A turn is admitted only when the declared
readiness is the whole of it. -/

/-- What the program's own history says about each socket. -/
abbrev Ready := String → Bool

def readyRegistrations (ready : Ready) (loans : Loans) : List String :=
  (loans.filter (fun loan => loan.interest == Interest.read && ready loan.socket)).map
    Loan.registration

def admitsTurn (ready : Ready) (loans : Loans) (declared : Option String) : Bool :=
  match readyRegistrations ready loans, declared with
  | [], none => true
  | [one], some named => one == named
  | _, _ => false

theorem an_idle_turn_needs_an_empty_ready_set
    (ready : Ready) (loans : Loans) (admitted : admitsTurn ready loans none = true) :
    readyRegistrations ready loans = [] := by
  unfold admitsTurn at admitted
  split at admitted <;> simp_all

theorem a_declared_turn_names_the_one_ready_registration
    (ready : Ready) (loans : Loans) (named : String)
    (admitted : admitsTurn ready loans (some named) = true) :
    readyRegistrations ready loans = [named] := by
  unfold admitsTurn at admitted
  split at admitted <;> simp_all

theorem two_ready_registrations_admit_no_turn
    (ready : Ready) (loans : Loans) (declared : Option String)
    {first second : String} {rest : List String}
    (two : readyRegistrations ready loans = first :: second :: rest) :
    admitsTurn ready loans declared = false := by
  -- The ready set alone decides it: with two or more members the match falls to
  -- its last arm whatever the turn declared, which is why the refusal is on the
  -- **state** rather than on the spelling.
  unfold admitsTurn
  rw [two]

/-- A `write` registration is never ready here: this route induces no
send-buffer pressure, so it has nothing to say about one. -/
theorem a_write_registration_is_never_ready
    (ready : Ready) (registration socket : String) :
    readyRegistrations ready
      [{ registration := registration, socket := socket, interest := .write }] = [] := by
  simp [readyRegistrations]

/-- A registration on a socket this program committed nothing to is not ready:
readiness comes only from the program's own history. -/
theorem readiness_comes_only_from_the_programs_own_history
    (ready : Ready) (loans : Loans) (quiet : ∀ socket, ready socket = false) :
    readyRegistrations ready loans = [] := by
  simp [readyRegistrations, quiet]

/-- Level-triggered readiness, as an equation: a turn changes nothing a later
turn reads, so a second turn on the same readiness reports the same thing. The
work the turn announces is done by the entry's own transport statements. -/
theorem a_turn_drains_nothing
    (ready : Ready) (ledger : Ledger) (loans : Loans) (declared : Option String) :
    ledgerStep ledger (.dispatch declared) = ledger
      ∧ loanStep loans (.dispatch declared) = loans
      ∧ readyRegistrations ready (loanStep loans (.dispatch declared))
          = readyRegistrations ready loans :=
  ⟨rfl, rfl, rfl⟩

/-! ## Effects

The four bookkeeping statements derive **nothing**, and that is a measurement
rather than an omission: the emitted program performs no call for any of them,
because the reactor it runs *is* the declared registration set materialised as
the poll set at each turn. A turn performs the one wait the composition adds. -/

inductive Effect where
  | io
  | alloc
  | mut
  deriving DecidableEq, Repr

def derived : Statement → List Effect
  | .create _ => [.alloc]
  | .close _ => [.alloc]
  | .configure _ => [.io]
  | .transfer _ => [.io]
  | .dispatch _ => [.io]
  | .line => [.io]
  | .reactorOpen => []
  | .reactorClose => []
  | .watch _ _ _ => []
  | .unwatch _ => []
  | .ret => []

theorem a_dispatch_turn_derives_io (declared : Option String) :
    derived (.dispatch declared) = [Effect.io] := rfl

theorem the_registration_statements_derive_nothing
    (registration socket : String) (interest : Interest) :
    derived .reactorOpen = [] ∧ derived .reactorClose = []
      ∧ derived (.watch registration socket interest) = []
      ∧ derived (.unwatch registration) = [] :=
  ⟨rfl, rfl, rfl, rfl⟩

/-- Stated over the whole grammar, so a widening changes a theorem rather than
slipping past it. -/
theorem no_statement_derives_mut (statement : Statement) :
    Effect.mut ∉ derived statement := by
  cases statement <;> simp [derived]

/-! ## The module gate

The composition is a **conjunction**: registering a socket with the reactor needs
both `std.socket` and `std.event`, and neither alone grants it. The transport
half is the registry's own gate, restated here rather than re-proved, so the
composition cannot drift from it. -/

def admitsRegistration (socket event registers : Bool) : Bool :=
  !registers || (socket && event)

theorem a_registration_needs_both_modules :
    ∀ socket event, admitsRegistration socket event true = true → socket = true ∧ event = true := by
  decide

theorem the_socket_module_alone_grants_no_registration :
    admitsRegistration true false true = false := rfl

theorem the_event_module_alone_grants_no_registration :
    admitsRegistration false true true = false := rfl

theorem code_that_registers_nothing_needs_neither :
    ∀ socket event, admitsRegistration socket event false = true := by decide

theorem the_transport_half_still_needs_its_module {names : List String}
    {imports : QCore.ModuleImports.Imports}
    (resolved : QCore.ModuleImports.resolve names = some imports)
    (enabled : imports.socket = true) : "std.socket" ∈ names :=
  QCore.ModuleImports.admitted_socket_requires_its_module resolved enabled

end QCore.SocketEventCompose
