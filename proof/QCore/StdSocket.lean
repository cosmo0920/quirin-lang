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

/-! `source.socket.v1` — the custody and determinism discipline of the
`std.socket` bundled module (card `QUIRIN-SOCKET-V1`).

Three claims are proved here, and they are the three the Rust route rests on.

**A socket is an owned descriptor.** Creation and `accept` acquire one, `close`
releases exactly one, and no transport statement moves the ledger at all. A
program whose ledger is empty at its return has closed everything it opened,
which is what the unedited ownership checker discharges on the Rust side; this
module states what that route's operation list gives it.

**A receive is satisfied by the program's own committed history.** The stream
model is a FIFO of bytes an earlier `send` of the same program committed. A
receive is admitted only when the stream already holds at least the declared
count, so an admitted receive *has* its bytes before it runs — that is
non-blocking as a theorem rather than as a timing assumption, and it is why the
bounded wait in the emitted C is a witness and not a check.

**What a receive takes is determined by that history alone.** Draining a stream
returns exactly the bytes committed to it, in order. This is the soundness of
the declared-payload verification: the certificate publishes those bytes and the
emitted program refuses to continue unless the bytes it actually read are them,
so the named premise `SOCKET-V1-STREAM-BYTE-ORDER` produces a failed run when it
is false instead of a wrong observation.

The datagram half is stated separately and differently, because it *is*
different: a queue of whole datagrams, each carrying the socket of this same
program that sent it, taken whole or refused.

What is **not** proved here: that the Rust lowering extracts these statements
faithfully from source tokens; that the platform's loopback transport is the
FIFO this model assumes (a premise, converted into a bounded failure rather than
proved away); or anything about a failed primitive — the emitted program exits
there, so a run either produces the modelled observation or produces none, and
that partial-correctness boundary is stated rather than proved away. Nor is the
ownership checker re-proved: `QCore.Cycle3` and its neighbours own the custody
rules. -/

namespace QCore.StdSocket

/-- Which transport a socket speaks. The two are never interchangeable. -/
inductive Kind where
  | tcp
  | udp
  deriving DecidableEq, Repr

/-- What a payload byte may be.
**This is half the effect discipline, as a type.** Neither constructor mentions
a socket, so a payload byte can never be one — not because a checker refuses it,
but because the syntax cannot say it. -/
inductive Operand where
  | literal (value : Int)
  | scalar (name : String)
  deriving DecidableEq, Repr

/-- The statements of a flat entry. A socket is named only at a custody or
transport position, which is the other half of the effect discipline. -/
inductive Statement where
  | «let» (binding : String) (value : Int)
  | create (name : String) (kind : Kind)
  | bind (name : String)
  | listen (name : String)
  | connect (client listener : String)
  | accept (binding listener : String)
  | send (name : String) (payload : List Operand)
  | recv (name : String) (count : Nat)
  | sendto (name target : String) (payload : List Operand)
  | recvfrom (name : String) (count : Nat)
  | close (name : String)
  /-- A `source.std-io.v1` line, reused unchanged. It is one constructor here
  because nothing this module proves depends on which line it is: a rendering
  touches no custody and commits no byte, and that is the point. -/
  | line
  | ret
  deriving DecidableEq, Repr

/-- The largest payload one send may carry. -/
def maxPayload : Nat := 1024

/-- The largest number of bytes one stream may hold committed and unreceived. -/
def maxInflight : Nat := 4096

/-- The effects this model distinguishes, matching the Rust `Effect` row. -/
inductive Effect where
  | io
  | alloc
  | mut
  deriving DecidableEq, Repr

/-! ## The descriptor ledger

A list of names, each holding exactly one descriptor: what the ownership
footprint carries on this route's own axis. -/

abbrev Ledger := List String

def liveDescriptors (ledger : Ledger) : Nat := ledger.length

def held (ledger : Ledger) (name : String) : Prop := name ∈ ledger

def acquire (ledger : Ledger) (name : String) : Ledger := name :: ledger

def release (ledger : Ledger) (name : String) : Ledger :=
  ledger.filter (fun entry => entry ≠ name)

/-- The ledger after one statement. Only the three custody statements move it;
every transport statement, every line and the return leave it exactly as it was.

That is the whole reason a socket's custody can be decided by the *unedited*
cycle-3 checker: the transport is orthogonal to the ledger. -/
def step (ledger : Ledger) : Statement → Ledger
  | .create name _ => acquire ledger name
  | .accept binding _ => acquire ledger binding
  | .close name => release ledger name
  | .«let» _ _ => ledger
  | .bind _ => ledger
  | .listen _ => ledger
  | .connect _ _ => ledger
  | .send _ _ => ledger
  | .recv _ _ => ledger
  | .sendto _ _ _ => ledger
  | .recvfrom _ _ => ledger
  | .line => ledger
  | .ret => ledger

def run (ledger : Ledger) : List Statement → Ledger
  | [] => ledger
  | statement :: rest => run (step ledger statement) rest

theorem an_empty_ledger_holds_no_descriptors : liveDescriptors [] = 0 := rfl

theorem an_acquisition_adds_exactly_one (ledger : Ledger) (name : String) :
    liveDescriptors (acquire ledger name) = liveDescriptors ledger + 1 := by
  simp [liveDescriptors, acquire]

theorem a_release_removes_exactly_its_own (ledger : Ledger) (name : String) :
    ¬ held (release ledger name) name := by
  simp [held, release, List.mem_filter]

theorem a_release_keeps_every_other_entry (ledger : Ledger) (name other : String)
    (different : other ≠ name) :
    held (release ledger name) other ↔ held ledger other := by
  simp [held, release, List.mem_filter, different]

/-- A closed socket is not held, so every later statement naming it is a
use-after-close the inherited checker refuses. This is the statement form of
"`close` is the release". -/
theorem a_closed_socket_is_not_held (ledger : Ledger) (name : String) :
    ¬ held (step ledger (.close name)) name :=
  a_release_removes_exactly_its_own ledger name

/-- Every non-custody statement leaves the ledger byte for byte, stated as one
list so that widening the grammar changes a theorem rather than slipping past
it. -/
theorem transport_statements_leave_the_ledger_unchanged (ledger : Ledger)
    (name target : String) (count : Nat) (payload : List Operand) (value : Int) :
    step ledger (.bind name) = ledger
      ∧ step ledger (.listen name) = ledger
      ∧ step ledger (.connect name target) = ledger
      ∧ step ledger (.send name payload) = ledger
      ∧ step ledger (.recv name count) = ledger
      ∧ step ledger (.sendto name target payload) = ledger
      ∧ step ledger (.recvfrom name count) = ledger
      ∧ step ledger (.«let» name value) = ledger
      ∧ step ledger .line = ledger
      ∧ step ledger .ret = ledger :=
  ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

/-- An `accept` acquires a **second** socket beside the listener it reads: the
listener stays held, so a program that accepts owes two closes and not one. -/
theorem an_accept_acquires_beside_its_listener (ledger : Ledger)
    (binding listener : String) (listening : held ledger listener) :
    held (step ledger (.accept binding listener)) binding
      ∧ held (step ledger (.accept binding listener)) listener := by
  refine ⟨?_, ?_⟩
  · simp [held, step, acquire]
  · simp only [held, step, acquire, List.mem_cons]
    exact Or.inr listening

/-! ## The stream: a receive is satisfied by this program's own history

The model of one direction of one connected pair. `commit` is what a `send`
does; `take`/`drop` are what a `recv` does. Everything this route claims about
blocking and about payload verification is a statement about these three. -/

abbrev Stream := List Nat

/-- What a `send` does: append the declared bytes to the peer's stream. -/
def commit (stream : Stream) (bytes : List Nat) : Stream := stream ++ bytes

/-- Whether a receive of `count` bytes is admitted.
**This is the determinism rule**, and it is decidable because the stream is a
list this model already holds: a receive is admitted exactly when the bytes it
asks for are already there. -/
def admitsReceive (stream : Stream) (count : Nat) : Bool :=
  0 < count && count ≤ stream.length && count ≤ maxPayload

/-- What a receive takes, and what it leaves. -/
def taken (stream : Stream) (count : Nat) : List Nat := stream.take count
def remaining (stream : Stream) (count : Nat) : Stream := stream.drop count

/-- **A receive on an empty stream is refused**, never blocked on. There is
nothing for it to take, and the program itself would have to run further to put
anything there — which it cannot, being the only thread. -/
theorem a_receive_on_an_empty_stream_is_refused (count : Nat) :
    admitsReceive [] count = false := by
  cases count with
  | zero => simp [admitsReceive]
  | succ n => simp [admitsReceive]

/-- **An admitted receive has its bytes before it runs.** This is the
non-blocking property as a theorem: the count it takes is exactly the count it
asked for, with no wait in the statement. -/
theorem an_admitted_receive_takes_its_full_count (stream : Stream) (count : Nat)
    (admitted : admitsReceive stream count = true) :
    (taken stream count).length = count := by
  simp [admitsReceive, Bool.and_eq_true, decide_eq_true_eq] at admitted
  simp [taken, admitted.1.2]

/-- A receive never invents a byte: what it takes is a prefix of what was
committed. -/
theorem a_receive_takes_a_prefix (stream : Stream) (count : Nat) :
    taken stream count ++ remaining stream count = stream := by
  simp [taken, remaining]

/-- **Draining a stream returns exactly the bytes committed to it.** The
soundness of the published receipts: the certificate's bytes are this value, and
the emitted program refuses to continue unless the bytes it read are them. -/
theorem draining_returns_the_committed_bytes (bytes : List Nat) :
    taken (commit [] bytes) bytes.length = bytes := by
  simp [taken, commit]

/-- **The stream is FIFO.** Two commits are returned by two receives in the
order they were made, which is what makes a request-then-response exchange
predictable at check time. -/
theorem two_commits_are_returned_in_order (first second : List Nat) :
    taken (commit (commit [] first) second) first.length = first
      ∧ taken (remaining (commit (commit [] first) second) first.length) second.length
          = second := by
  refine ⟨?_, ?_⟩
  · simp [taken, commit]
  · simp [taken, remaining, commit]

/-- A commit can only lengthen a stream, so a send never consumes what an
earlier one put there. -/
theorem a_commit_never_shortens (stream : Stream) (bytes : List Nat) :
    stream.length ≤ (commit stream bytes).length := by
  simp [commit]

/-- A receive strictly shortens the stream by its own count, so the history a
later receive is checked against is the one this receive left. -/
theorem a_receive_leaves_exactly_the_rest (stream : Stream) (count : Nat) :
    (remaining stream count).length = stream.length - count := by
  simp [remaining]

/-! ## The datagram queue: taken whole, and from a known sender -/

/-- One queued datagram: the socket of this same program that sent it, and its
bytes. Both halves are checked by the emitted program — the bytes against the
certificate's receipt, the sender against that socket's own bound address. -/
abbrev Datagram := String × List Nat
abbrev Queue := List Datagram

/-- Whether a datagram receive of `count` bytes is admitted. **A datagram is
taken whole**: the declared count must be the head datagram's own length, so
there is no truncation and no reassembly. -/
def admitsDatagram (queue : Queue) (count : Nat) : Bool :=
  match queue with
  | [] => false
  | datagram :: _ => datagram.2.length = count && 0 < count && count ≤ maxPayload

theorem a_datagram_receive_on_an_empty_queue_is_refused (count : Nat) :
    admitsDatagram [] count = false := rfl

/-- A count that is not the head datagram's length is refused rather than
truncating it — the rule that makes the datagram half different from the stream
half rather than a special case of it. -/
theorem a_partial_datagram_is_refused (sender : String) (bytes : List Nat)
    (rest : Queue) (count : Nat) (mismatched : bytes.length ≠ count) :
    admitsDatagram ((sender, bytes) :: rest) count = false := by
  simp [admitsDatagram, mismatched]

/-- What a datagram receive takes: the head, whole, with its sender. -/
def takenDatagram (queue : Queue) : Option Datagram := queue.head?

theorem an_admitted_datagram_receive_takes_the_head (sender : String)
    (bytes : List Nat) (rest : Queue) :
    takenDatagram ((sender, bytes) :: rest) = some (sender, bytes) := rfl

/-- The same stream theorem for datagrams: what arrives is exactly what was
sent, whole, from the socket that sent it. -/
theorem a_datagram_arrives_whole (sender : String) (bytes : List Nat) :
    takenDatagram [(sender, bytes)] = some (sender, bytes)
      ∧ admitsDatagram [(sender, bytes)] bytes.length
          = (0 < bytes.length && bytes.length ≤ maxPayload) := by
  refine ⟨rfl, ?_⟩
  simp [admitsDatagram]

/-! ## The effect discipline -/

/-- The effects one statement derives. Custody opens and closes a kernel
resource (`alloc`); every transport call is `io`; nothing is `mut`, because this
route mutates no object of the program — the bytes a send commits leave it
entirely and the bytes a receive takes arrive as new values. -/
def effects : Statement → List Effect
  | .create _ _ => [Effect.alloc]
  | .close _ => [Effect.alloc]
  | .accept _ _ => [Effect.alloc, Effect.io]
  | .bind _ => [Effect.io]
  | .listen _ => [Effect.io]
  | .connect _ _ => [Effect.io]
  | .send _ _ => [Effect.io]
  | .recv _ _ => [Effect.io]
  | .sendto _ _ _ => [Effect.io]
  | .recvfrom _ _ => [Effect.io]
  | .line => [Effect.io]
  | .«let» _ _ => []
  | .ret => []

theorem a_creation_derives_alloc (name : String) (kind : Kind) :
    effects (.create name kind) = [Effect.alloc] := rfl

theorem a_close_derives_alloc (name : String) :
    effects (.close name) = [Effect.alloc] := rfl

theorem a_transfer_derives_io (name target : String) (payload : List Operand)
    (count : Nat) :
    effects (.send name payload) = [Effect.io]
      ∧ effects (.recv name count) = [Effect.io]
      ∧ effects (.sendto name target payload) = [Effect.io]
      ∧ effects (.recvfrom name count) = [Effect.io] :=
  ⟨rfl, rfl, rfl, rfl⟩

theorem a_binding_derives_nothing (name : String) (value : Int) :
    effects (.«let» name value) = [] := rfl

/-- **`mut` is never derived, on any statement.** Stated over the whole grammar
rather than statement by statement, so a widening changes this theorem. -/
theorem no_statement_derives_mut (statement : Statement) :
    Effect.mut ∉ effects statement := by
  cases statement <;> simp [effects]

/-! ## The operand wall -/

/-- The two operand forms, stated as one list so a widening changes a theorem
rather than slipping past it. -/
theorem the_operand_forms_are_exactly_two (operand : Operand) :
    (∃ value, operand = .literal value) ∨ (∃ name, operand = .scalar name) := by
  cases operand with
  | literal value => exact Or.inl ⟨value, rfl⟩
  | scalar name => exact Or.inr ⟨name, rfl⟩

/-- **An operand is never a custody act.** A payload byte cannot be a socket,
because no constructor of `Operand` names one — which is why every byte this
route puts on the wire is decided before emission. -/
theorem an_operand_is_never_a_custody_act (operand : Operand) (ledger : Ledger) :
    (match operand with
      | .literal _ => ledger
      | .scalar _ => ledger) = ledger := by
  cases operand <;> rfl

/-! ## The module gate -/

open QCore.ModuleImports

/-- The socket capability needs its own module, proved against the same registry
and by the same generic argument every other bundled module is. -/
theorem the_socket_capability_needs_its_module {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.socket = true) : "std.socket" ∈ names :=
  admitted_socket_requires_its_module resolved enabled

/-- And it grants no stream in return: a module that receives a byte and prints
it names every module it uses. -/
theorem the_socket_module_grants_no_stream :
    admitsPrint ⟨false, false, false, false, false, false, false, false, true, false⟩ true = false
      ∧ admitsLog ⟨false, false, false, false, false, false, false, false, true, false⟩ true = false :=
  ⟨rfl, rfl⟩

end QCore.StdSocket
