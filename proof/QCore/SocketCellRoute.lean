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

import QCore.SocketFootprint

/-! The socket protocol cell routed from source (card `CT42-CELL-ROUTE`,
wave 39): the derived-alias projection and the `disjoint NAME NAME;` question.

Two statements this namespace carries, each the model of one edit the card
took in Rust.

**The T09 edit (`CT42-CELL-DERIVED-PREMISE`).** `DisjointFootprintsV1`'s
certificate names the premise `DeclaredProtocolCellIdentity` — "a declared
protocol cell really is the shared logical state, and no undeclared descriptor
also advances it" — for a **declared** descriptor alias. The route projects
`socket_footprint`'s derived cells as a **derived** alias instead, which the
checker handles identically except that it stamps no such premise: the cell was
read off the transport model's own decided connection structure, not declared.
`a_derived_projection_stamps_no_declared_identity` is that narrowing;
`a_declared_alias_stamps_the_premise` is its non-vacuity; and
`derived_and_declared_aliases_decide_alike` is the sameness of everything else.

**The route (`CT42-CELL-UNROUTED`).** `disjoint NAME NAME;` asks the
unedited algebra about two custodians' derived cells. Over an admitted
`accept`, the binding and its connector share one cell
(`QCore.SocketFootprint.an_accepted_pair_shares_one_cell`), so the question is
refused as `sharedProtocolCell` — `a_question_over_an_accepted_pair_is_refused`
— while two sockets of distinct cells decide disjoint. A name the derivation
enumerated no cell for answers nothing: the transport checker refuses it before
the question is asked, and the model says so with `none` rather than a
judgement.

Not modelled here, and stated: that the transport model's cross-name coupling
*is* the shared logical state (the Rust checker's measured behaviour; residual
`SOCKET-CELL-COUPLING-UNPROVED`), threads, real descriptor numbers, and the
dup-like alias no source form creates. -/
namespace QCore.SocketCellRoute

open QCore.DisjointFootprint
open QCore.SocketFootprint

/-- The two descriptor-alias events of the T09 checker. -/
inductive AliasEvent where
  | declare (handle descriptor cell : Nat)
  | derive (handle descriptor cell : Nat)
  deriving DecidableEq, Repr

/-- The footprint either event mints: the same `descriptor` footprint. -/
def footprintOf : AliasEvent → Footprint
  | .declare _ descriptor cell => .descriptor descriptor cell
  | .derive _ descriptor cell => .descriptor descriptor cell

/-- The handle either event mints. -/
def handleOf : AliasEvent → Nat
  | .declare handle _ _ => handle
  | .derive handle _ _ => handle

/-- Whether an event stamps `DeclaredProtocolCellIdentity` on the certificate:
a declared alias does, a derived one does not. This is the one line the two
events decide differently. -/
def stampsDeclaredIdentity : AliasEvent → Bool
  | .declare _ _ _ => true
  | .derive _ _ _ => false

/-- The certificate carries the premise exactly when some event stamped it. -/
def carriesDeclaredIdentity (events : List AliasEvent) : Bool :=
  events.any stampsDeclaredIdentity

/-- The route's projection of a derived cell assignment (newest first, as
`State.cells` is kept): one derived alias per socket, its handle and descriptor
the socket's position in acquisition order and its cell the derived one. -/
def projection (cells : List (String × Nat)) : List AliasEvent :=
  (cells.reverse.zipIdx).map (fun entry => .derive entry.2 entry.2 entry.1.2)

/-- The T09 narrowing: a projection made of derived aliases stamps no
`DeclaredProtocolCellIdentity`, whatever the cells are. -/
theorem a_derived_projection_stamps_no_declared_identity (cells : List (String × Nat)) :
    carriesDeclaredIdentity (projection cells) = false := by
  unfold carriesDeclaredIdentity projection
  simp [stampsDeclaredIdentity]

/-- Non-vacuity: the same cell declared carries the premise. -/
theorem a_declared_alias_stamps_the_premise (handle descriptor cell : Nat) :
    carriesDeclaredIdentity [.declare handle descriptor cell] = true := by
  simp [carriesDeclaredIdentity, stampsDeclaredIdentity]

/-- Everything but the premise is the same: a derived alias and a declared
alias of one descriptor and cell mint the same footprint, so every question
over them decides alike. -/
theorem derived_and_declared_aliases_decide_alike (hL dL cL hR dR cR : Nat) :
    decideDisjoint (footprintOf (.derive hL dL cL)) (footprintOf (.derive hR dR cR))
      = decideDisjoint (footprintOf (.declare hL dL cL)) (footprintOf (.declare hR dR cR)) := by
  rfl

/-- The route's question: the unedited algebra over the two names' derived
cells, or `none` when a name has no cell — the transport checker's `Q081`
refuses that name before the question is asked, so the model answers nothing
rather than inventing a judgement. Descriptor numbers do not enter the
decision (`decideDisjoint` reads cells only), so the question is stated over
cells. -/
def question (s : State) (left right : String) : Option Decision :=
  match cellOf s left, cellOf s right with
  | some cellL, some cellR => some (decideDisjoint (.descriptor 0 cellL) (.descriptor 1 cellR))
  | _, _ => none

/-- CT42, in the model: after an admitted accept, the question over the
binding and its connector is refused as a shared protocol cell. -/
theorem a_question_over_an_accepted_pair_is_refused (s s' : State)
    (binding listener connector : String)
    (admitted : step s (.accept binding listener) = some s')
    (queued : oldest s.pending listener = some connector)
    (fresh : connector ≠ binding) :
    question s' binding connector = some (.refused .sharedProtocolCell) := by
  obtain ⟨connector', cell, queued', assigned, hBinding⟩ :=
    an_admitted_accept_takes_an_existing_cell s s' binding listener admitted
  have hSame : connector' = connector := by
    rw [queued] at queued'
    exact (Option.some.injEq _ _).mp queued' |>.symm
  subst hSame
  have hConnector : cellOf s' connector' = some cell := by
    rw [an_accept_keeps_every_other_assignment s s' binding listener connector' fresh admitted]
    exact assigned
  simp [question, hBinding, hConnector, decideDisjoint]

/-- The positive control: two sockets of distinct derived cells decide
disjoint. -/
theorem a_question_over_distinct_cells_is_admitted (s : State) (left right : String)
    (cellL cellR : Nat) (hL : cellOf s left = some cellL) (hR : cellOf s right = some cellR)
    (distinct : cellL ≠ cellR) :
    question s left right = some .disjoint := by
  simp [question, hL, hR, decideDisjoint, distinct]

/-- A name with no derived cell answers nothing: no judgement is invented for
a custodian the derivation never enumerated. -/
theorem a_question_naming_no_socket_answers_nothing (s : State) (left right : String)
    (missing : cellOf s left = none) :
    question s left right = none := by
  simp [question, missing]

/-- The question is symmetric: the algebra reads cells only, and equality of
cells is. -/
theorem the_question_is_symmetric (s : State) (left right : String) :
    question s left right = question s right left := by
  unfold question
  cases cellOf s left <;> cases cellOf s right <;> simp [decideDisjoint]
  rename_i cellL cellR
  by_cases h : cellL = cellR
  · subst h; simp
  · have h' : cellR ≠ cellL := fun hEq => h hEq.symm
    simp [h, h']

/-- The canonical stream program, end to end through the route: the pair is
refused, the listener admitted, and the projection carries no declared premise
— all three computed. -/
example :
    (run State.initial canonical).map (fun s => question s "client" "server")
      = some (some (.refused .sharedProtocolCell)) ∧
    (run State.initial canonical).map (fun s => question s "client" "listener")
      = some (some .disjoint) ∧
    (run State.initial canonical).map (fun s => carriesDeclaredIdentity (projection s.cells))
      = some false := by
  refine ⟨?_, ?_, ?_⟩ <;> decide

end QCore.SocketCellRoute
