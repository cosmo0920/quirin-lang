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

import QCore.DisjointFootprint

/-! Protocol-cell identity derived from a socket program's connection
structure (card `CT42-PROTOCOL-CELL`; the two unpaired arms made refusals by
card `CT42-CELL-ROUTE`, wave 39).

`QCore.DisjointFootprint` decides descriptor disjointness by protocol-cell
identity, but its cell is *declared* (`DeclareDescriptorAlias`). This namespace
is the model of the derivation that reads the cell off `source.socket.v1`'s
own decided connection structure instead: a socket is created in a fresh cell
of its own, and an `accept` pairs the fresh accepted binding with the oldest
queued connector of its listener — the completed connection — by giving it the
connector's cell. Nothing else ever merges two cells, because an admitted
connector connects exactly once and an accepted binding is fresh by
construction.

**The walk is partial.** Card `STABILITY-REEVAL-3` (wave 38) measured that the
wave-31 model's two "unreachable for an admitted program" arms — an accept with
nothing queued, and a queued connector with no cell — gave the binding a fresh
singleton cell, which the unedited `decideDisjoint` then judges *disjoint* from
the binding's real connector: the admitting direction
(`CT42-CELL-FALLBACK-ARM-DIRECTION`). Card `CT42-CELL-ROUTE` made both arms,
and a connect the address resolution gives no listener, **refusals**: `step`
returns `none` there, and the theorem `an_admitted_accept_takes_an_existing_cell`
states the direction — an accept the walk admits gives its binding a cell some
already-created socket holds, never a fresh one.

The theorems are about the derivation and its composition with the footprint
algebra: a create's cell is fresh (so unpaired sockets keep distinct cells), an
accept gives the pair one cell and moves nobody else's, the three unpaired
arms refuse, and a derived pair is refused by the **unedited** `decideDisjoint`
as `sharedProtocolCell` while a listener's own cell decides disjoint. They are
not about the transport state machine (the Rust checker's admission decides
which programs the derivation runs on), dup-like descriptor aliases (no source
form creates one), threads or real descriptor numbers. -/
namespace QCore.SocketFootprint

open QCore.DisjointFootprint

/-- The statements the derivation reads. Every other statement of the route
leaves the cell assignment unchanged. A `connect`'s listener is the socket the
shared address resolution names for it, `none` when it names none — the
external-peer fact the transport checker refuses at `Q085`. -/
inductive Statement where
  | create (name : String)
  | connect (client : String) (listener : Option String)
  | accept (binding listener : String)
  deriving DecidableEq, Repr

/-- The derivation state: the cell assignment (newest first), the queued
connectors per listener in arrival order, and the fresh-cell counter. -/
structure State where
  cells : List (String × Nat)
  pending : List (String × String)
  next : Nat
  deriving DecidableEq, Repr

def State.initial : State := ⟨[], [], 0⟩

/-- The derived cell of one socket name. -/
def cellOf (s : State) (name : String) : Option Nat :=
  (s.cells.find? (fun entry => entry.1 = name)).map (·.2)

/-- The oldest queued connector of one listener. -/
def oldest (pending : List (String × String)) (listener : String) : Option String :=
  (pending.find? (fun entry => entry.1 = listener)).map (·.2)

/-- Remove the oldest queued connector of one listener. -/
def taken (pending : List (String × String)) (listener : String) : List (String × String) :=
  pending.eraseP (fun entry => entry.1 = listener)

/-- One derivation step, **partial**.

A `create` assigns the fresh cell and bumps the counter. A `connect` with a
resolved listener queues the connector; one the resolution gave no listener
**refuses**. An `accept` gives the fresh binding the oldest queued connector's
cell — the completed connection — and **refuses** when no connector is queued
or the queued name has no cell. All three refusals are unreachable for an
admitted program, where every `connect` resolves and every `accept` pops a
connector the program created; they are refusals rather than fresh cells
because a fresh cell here is judged *disjoint* from the binding's real
connector by the unedited algebra (`CT42-CELL-FALLBACK-ARM-DIRECTION`). -/
def step (s : State) : Statement → Option State
  | .create name => some ⟨(name, s.next) :: s.cells, s.pending, s.next + 1⟩
  | .connect _ none => none
  | .connect client (some listener) =>
      some ⟨s.cells, s.pending ++ [(listener, client)], s.next⟩
  | .accept binding listener =>
      match oldest s.pending listener with
      | none => none
      | some connector =>
          match cellOf s connector with
          | none => none
          | some cell => some ⟨(binding, cell) :: s.cells, taken s.pending listener, s.next⟩

def run (s : State) : List Statement → Option State
  | [] => some s
  | statement :: rest =>
      match step s statement with
      | none => none
      | some s' => run s' rest

/-- Every assigned cell is below the fresh counter. This is the freshness
invariant everything else leans on. -/
def Bounded (s : State) : Prop :=
  ∀ entry ∈ s.cells, entry.2 < s.next

theorem initial_is_bounded : Bounded State.initial := by
  intro entry h
  cases h

/-- An assignment read back out of the state is one of its entries. -/
theorem assigned_cell_is_bounded (s : State) (name : String) (cell : Nat)
    (bounded : Bounded s) (assigned : cellOf s name = some cell) : cell < s.next := by
  unfold cellOf at assigned
  cases hFind : s.cells.find? (fun entry => entry.1 = name) with
  | none => rw [hFind] at assigned; cases assigned
  | some found =>
      rw [hFind] at assigned
      have hCell : found.2 = cell := by
        simpa using assigned
      have hMem := List.mem_of_find?_eq_some hFind
      exact hCell ▸ bounded found hMem

/-- Every admitted step preserves the freshness invariant. -/
theorem step_preserves_bounded (s s' : State) (statement : Statement)
    (bounded : Bounded s) (admitted : step s statement = some s') : Bounded s' := by
  cases statement with
  | create name =>
      simp only [step, Option.some.injEq] at admitted
      subst admitted
      intro entry h
      simp only [List.mem_cons] at h
      rcases h with h | h
      · subst h; exact Nat.lt_succ_self _
      · exact Nat.lt_trans (bounded entry h) (Nat.lt_succ_self _)
  | connect client listener =>
      cases listener with
      | none => simp [step] at admitted
      | some listener =>
          simp only [step, Option.some.injEq] at admitted
          subst admitted
          intro entry h
          exact bounded entry h
  | accept binding listener =>
      cases hOldest : oldest s.pending listener with
      | none => simp [step, hOldest] at admitted
      | some connector =>
          cases hCell : cellOf s connector with
          | none => simp [step, hOldest, hCell] at admitted
          | some cell =>
              simp only [step, hOldest, hCell, Option.some.injEq] at admitted
              subst admitted
              intro entry h
              simp only [List.mem_cons] at h
              rcases h with h | h
              · subst h
                exact assigned_cell_is_bounded s connector cell bounded hCell
              · exact bounded entry h

theorem run_preserves_bounded (statements : List Statement) (s s' : State)
    (bounded : Bounded s) (admitted : run s statements = some s') : Bounded s' := by
  induction statements generalizing s with
  | nil =>
      simp only [run, Option.some.injEq] at admitted
      subst admitted
      exact bounded
  | cons statement rest ih =>
      simp only [run] at admitted
      cases hStep : step s statement with
      | none => rw [hStep] at admitted; cases admitted
      | some next =>
          rw [hStep] at admitted
          exact ih next (step_preserves_bounded s next statement bounded hStep) admitted

/-- A create assigns exactly the fresh cell. -/
theorem create_assigns_the_fresh_cell (s : State) (name : String) :
    (step s (.create name)).bind (fun s' => cellOf s' name) = some s.next := by
  simp [step, cellOf]

/-- A create's cell is no existing socket's cell: the fresh counter is above
every assigned cell, so an unpaired socket can never collide with one that was
already there. -/
theorem a_fresh_cell_is_no_existing_cell (s : State) (name other : String) (cell : Nat)
    (bounded : Bounded s) (assigned : cellOf s other = some cell) :
    (step s (.create name)).bind (fun s' => cellOf s' name) ≠ some cell := by
  rw [create_assigns_the_fresh_cell]
  intro h
  have hEq : s.next = cell := by injection h
  have := assigned_cell_is_bounded s other cell bounded assigned
  omega

/-- An accept that pops a connector with a cell gives the accepted binding
exactly that cell: the completed connection is one cell with two custodians. -/
theorem an_accepted_pair_shares_one_cell (s : State) (binding listener connector : String)
    (cell : Nat) (queued : oldest s.pending listener = some connector)
    (assigned : cellOf s connector = some cell) :
    (step s (.accept binding listener)).bind (fun s' => cellOf s' binding) = some cell := by
  simp only [step, queued, assigned]
  simp [cellOf]

/-- An admitted accept moves nobody else's cell: the connector keeps its
assignment, and so does every other socket. (The binding is fresh in an
admitted program, so the shadowing arm of the new entry never fires for a name
that had a cell.) -/
theorem an_accept_keeps_every_other_assignment (s s' : State)
    (binding listener other : String) (h : other ≠ binding)
    (admitted : step s (.accept binding listener) = some s') :
    cellOf s' other = cellOf s other := by
  have hHead : binding ≠ other := fun hEq => h hEq.symm
  cases hOldest : oldest s.pending listener with
  | none => simp [step, hOldest] at admitted
  | some connector =>
      cases hCell : cellOf s connector with
      | none => simp [step, hOldest, hCell] at admitted
      | some cell =>
          simp only [step, hOldest, hCell, Option.some.injEq] at admitted
          subst admitted
          simp [cellOf, hHead]

/-- A create moves nobody else's cell either. -/
theorem a_create_keeps_every_other_assignment (s : State) (name other : String)
    (h : other ≠ name) :
    (step s (.create name)).bind (fun s' => cellOf s' other) = cellOf s other := by
  have hHead : name ≠ other := fun hEq => h hEq.symm
  simp [step, cellOf, hHead]

-- ---------------------------------------------------------------------------
-- Card `CT42-CELL-ROUTE`: the unpaired arms refuse, and an admitted accept
-- never invents a cell.
-- ---------------------------------------------------------------------------

/-- A connect the address resolution gives no listener refuses. -/
theorem an_unresolved_connect_is_refused (s : State) (client : String) :
    step s (.connect client none) = none := by
  simp [step]

/-- An accept with nothing queued on its listener refuses. -/
theorem an_unqueued_accept_is_refused (s : State) (binding listener : String)
    (empty : oldest s.pending listener = none) :
    step s (.accept binding listener) = none := by
  simp [step, empty]

/-- An accept whose queued connector holds no cell refuses. -/
theorem an_orphaned_connector_is_refused (s : State) (binding listener connector : String)
    (queued : oldest s.pending listener = some connector)
    (unassigned : cellOf s connector = none) :
    step s (.accept binding listener) = none := by
  simp [step, queued, unassigned]

/-- The direction, stated: an accept the walk admits gives its binding a cell
that some connector already held — never a fresh singleton cell. This is what
makes the unedited algebra's `sharedProtocolCell` reachable for every admitted
pair and `disjoint` unreachable for a binding and its real connector. -/
theorem an_admitted_accept_takes_an_existing_cell (s s' : State) (binding listener : String)
    (admitted : step s (.accept binding listener) = some s') :
    ∃ connector cell, oldest s.pending listener = some connector ∧
      cellOf s connector = some cell ∧ cellOf s' binding = some cell := by
  cases hOldest : oldest s.pending listener with
  | none => simp [step, hOldest] at admitted
  | some connector =>
      cases hCell : cellOf s connector with
      | none => simp [step, hOldest, hCell] at admitted
      | some cell =>
          refine ⟨connector, cell, rfl, hCell, ?_⟩
          simp only [step, hOldest, hCell, Option.some.injEq] at admitted
          subst admitted
          simp [cellOf]

/-- The composition, CT42's refusal: two descriptors whose derived cells agree
are refused by the **unedited** footprint algebra as a shared protocol cell,
whatever their descriptor numbers. -/
theorem a_derived_pair_is_refused_as_a_shared_cell (dL dR cell : Nat) :
    decideDisjoint (.descriptor dL cell) (.descriptor dR cell)
      = .refused .sharedProtocolCell :=
  descriptors_sharing_a_cell_are_never_disjoint dL dR cell

/-- The composition's positive control: the listener's own fresh cell decides
disjoint from either end of the connection. -/
theorem a_distinct_derived_cell_decides_disjoint {dL dR cellL cellR : Nat}
    (h : cellL ≠ cellR) :
    decideDisjoint (.descriptor dL cellL) (.descriptor dR cellR) = .disjoint :=
  descriptors_over_distinct_cells_are_disjoint h

/-- The canonical stream program, end to end: create the listener, create the
connector, connect, accept. The walk admits it, the pair shares one cell, the
listener keeps its own, and all four facts are computed rather than declared. -/
def canonical : List Statement :=
  [.create "listener", .create "client",
   .connect "client" (some "listener"), .accept "server" "listener"]

example :
    (run State.initial canonical).bind (fun s => cellOf s "client") = some 1 ∧
    (run State.initial canonical).bind (fun s => cellOf s "server") = some 1 ∧
    (run State.initial canonical).bind (fun s => cellOf s "listener") = some 0 := by
  refine ⟨?_, ?_, ?_⟩ <;> decide

/-- The same program without its connect: the accept refuses, so the whole run
refuses — no cell is invented for `server`. -/
example :
    run State.initial
      [.create "listener", .create "client", .accept "server" "listener"] = none := by
  decide

/-- The refusal, on the canonical program's derived cells. -/
example :
    decideDisjoint (.descriptor 1 1) (.descriptor 2 1)
      = .refused .sharedProtocolCell := by
  decide

end QCore.SocketFootprint
