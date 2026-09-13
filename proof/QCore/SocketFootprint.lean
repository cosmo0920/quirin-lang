import QCore.DisjointFootprint

/-! Protocol-cell identity derived from a socket program's connection
structure (card `CT42-PROTOCOL-CELL`).

`QCore.DisjointFootprint` decides descriptor disjointness by protocol-cell
identity, but its cell is *declared* (`DeclareDescriptorAlias`). This namespace
is the model of the derivation that reads the cell off `source.socket.v1`'s
own decided connection structure instead: a socket is created in a fresh cell
of its own, and an `accept` pairs the fresh accepted binding with the oldest
queued connector of its listener — the completed connection — by giving it the
connector's cell. Nothing else ever merges two cells, because an admitted
connector connects exactly once and an accepted binding is fresh by
construction.

The theorems are about the derivation and its composition with the footprint
algebra: a create's cell is fresh (so unpaired sockets keep distinct cells), an
accept gives the pair one cell and moves nobody else's, and a derived pair is
refused by the **unedited** `decideDisjoint` as `sharedProtocolCell` while a
listener's own cell decides disjoint. They are not about the transport state
machine (the Rust checker's admission decides which programs the derivation
runs on), dup-like descriptor aliases (no source form creates one), threads or
real descriptor numbers. -/
namespace QCore.SocketFootprint

open QCore.DisjointFootprint

/-- The statements the derivation reads. Every other statement of the route
leaves the cell assignment unchanged. -/
inductive Statement where
  | create (name : String)
  | connect (client listener : String)
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

/-- One derivation step.

A `create` assigns the fresh cell and bumps the counter. A `connect` queues the
connector. An `accept` gives the fresh binding the oldest queued connector's
cell — the completed connection — and, fail-closed, a fresh cell of its own
when no connector is queued or the queued name has no cell (both unreachable
for an admitted program, where every `accept` pops a connector the program
created). -/
def step (s : State) : Statement → State
  | .create name => ⟨(name, s.next) :: s.cells, s.pending, s.next + 1⟩
  | .connect client listener => ⟨s.cells, s.pending ++ [(listener, client)], s.next⟩
  | .accept binding listener =>
      match oldest s.pending listener with
      | none => ⟨(binding, s.next) :: s.cells, s.pending, s.next + 1⟩
      | some connector =>
          match cellOf s connector with
          | none => ⟨(binding, s.next) :: s.cells, taken s.pending listener, s.next + 1⟩
          | some cell => ⟨(binding, cell) :: s.cells, taken s.pending listener, s.next⟩

def run (s : State) : List Statement → State
  | [] => s
  | statement :: rest => run (step s statement) rest

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

/-- Every step preserves the freshness invariant. -/
theorem step_preserves_bounded (s : State) (statement : Statement)
    (bounded : Bounded s) : Bounded (step s statement) := by
  cases statement with
  | create name =>
      intro entry h
      simp only [step, List.mem_cons] at h
      rcases h with h | h
      · subst h; exact Nat.lt_succ_self _
      · exact Nat.lt_trans (bounded entry h) (Nat.lt_succ_self _)
  | connect client listener =>
      intro entry h
      exact bounded entry h
  | accept binding listener =>
      cases hOldest : oldest s.pending listener with
      | none =>
          intro entry h
          simp only [step, hOldest, List.mem_cons] at h ⊢
          rcases h with h | h
          · subst h; exact Nat.lt_succ_self _
          · exact Nat.lt_trans (bounded entry h) (Nat.lt_succ_self _)
      | some connector =>
          cases hCell : cellOf s connector with
          | none =>
              intro entry h
              simp only [step, hOldest, hCell, List.mem_cons] at h ⊢
              rcases h with h | h
              · subst h; exact Nat.lt_succ_self _
              · exact Nat.lt_trans (bounded entry h) (Nat.lt_succ_self _)
          | some cell =>
              intro entry h
              simp only [step, hOldest, hCell, List.mem_cons] at h ⊢
              rcases h with h | h
              · subst h
                exact assigned_cell_is_bounded s connector cell bounded hCell
              · exact bounded entry h

theorem run_preserves_bounded (statements : List Statement) (s : State)
    (bounded : Bounded s) : Bounded (run s statements) := by
  induction statements generalizing s with
  | nil => exact bounded
  | cons statement rest ih => exact ih _ (step_preserves_bounded s statement bounded)

/-- A create assigns exactly the fresh cell. -/
theorem create_assigns_the_fresh_cell (s : State) (name : String) :
    cellOf (step s (.create name)) name = some s.next := by
  simp [step, cellOf]

/-- A create's cell is no existing socket's cell: the fresh counter is above
every assigned cell, so an unpaired socket can never collide with one that was
already there. -/
theorem a_fresh_cell_is_no_existing_cell (s : State) (name other : String) (cell : Nat)
    (bounded : Bounded s) (assigned : cellOf s other = some cell) :
    cellOf (step s (.create name)) name ≠ some cell := by
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
    cellOf (step s (.accept binding listener)) binding = some cell := by
  simp only [step, queued, assigned]
  simp [cellOf]

/-- An accept moves nobody else's cell: the connector keeps its assignment, and
so does every other socket. (The binding is fresh in an admitted program, so
the shadowing arm of the new entry never fires for a name that had a cell.) -/
theorem an_accept_keeps_every_other_assignment (s : State)
    (binding listener other : String) (h : other ≠ binding) :
    cellOf (step s (.accept binding listener)) other = cellOf s other := by
  have hHead : binding ≠ other := fun hEq => h hEq.symm
  cases hOldest : oldest s.pending listener with
  | none => simp [step, hOldest, cellOf, hHead]
  | some connector =>
      cases hCell : cellOf s connector with
      | none =>
          simp only [step, hOldest, hCell]
          simp [cellOf, hHead]
      | some cell =>
          simp only [step, hOldest, hCell]
          simp [cellOf, hHead]

/-- A create moves nobody else's cell either. -/
theorem a_create_keeps_every_other_assignment (s : State) (name other : String)
    (h : other ≠ name) :
    cellOf (step s (.create name)) other = cellOf s other := by
  have hHead : name ≠ other := fun hEq => h hEq.symm
  simp [step, cellOf, hHead]

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
connector, connect, accept. The pair shares one cell, the listener keeps its
own, and all three facts are computed rather than declared. -/
example :
    cellOf (run State.initial
      [.create "listener", .create "client",
       .connect "client" "listener", .accept "server" "listener"]) "client" = some 1 ∧
    cellOf (run State.initial
      [.create "listener", .create "client",
       .connect "client" "listener", .accept "server" "listener"]) "server" = some 1 ∧
    cellOf (run State.initial
      [.create "listener", .create "client",
       .connect "client" "listener", .accept "server" "listener"]) "listener" = some 0 := by
  refine ⟨?_, ?_, ?_⟩ <;> decide

/-- The refusal, on the canonical program's derived cells. -/
example :
    decideDisjoint (.descriptor 1 1) (.descriptor 2 1)
      = .refused .sharedProtocolCell := by
  decide

end QCore.SocketFootprint
