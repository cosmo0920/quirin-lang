import QCore.DescriptorCellProtocol

/-!
# Per-end sentinels (card CELL-PROTOCOL-PER-END-GUARDS, gateway rule 38f)

USER DECISION 2026-11-03 (verbatim): "Per-end sentinels". Each channel end
carries its own sentinel: a record states at most one record guard (rule 38c,
`QCore.DescriptorCellProtocol`) and any number of per-end sentinels, each a
sentinel guard over exactly one `#fresh` cell, the guard itself, with its
`#same` names. Abstract rule theorems, not an extraction of the Rust analyzer
(`crates/gateway-rules/rule-38-descriptor-cells/src/protocol.rs`: `Fact`,
`Guard::dead_edge_decides`, `Fact::join`, `Fact::release_holder`,
`Fact::normalize`, `PER_END_VECTORS`; `cells.rs`: `join_keeping_stale`).

USER DIRECTION 2026-11-03 (standing, verbatim): "The Fluent Bit's caller
tests sometimes check the incorrect assertions so we need to pay attention
for the assertions and actual returning values of Fluent Bit functions." A
sentinel's meaning comes from the creator's result set and the absent value,
never from the test the destroyer writes. The `> 0` end tests are wrong:
descriptor 0 is a success value of every creator, and calloc's 0 the absent
value.

The model is one per-end group: the cell `c` (key 0), its own guard, and a
`#same` name `m` (key 1).

* **The guard's edges** (`dead_edge_decides_iff_no_descriptor_on_it`,
  `descriptor_zero_is_on_the_positive_dead_edge`,
  `the_live_edge_is_disjoint_from_the_dead_set`): a sentinel's dead edge
  decides "no cell is live" exactly when no possible descriptor value (rule
  38's premise: a live descriptor is never `-1`) is on it; so `!= -1`'s does
  and `> 0`'s does not, descriptor 0 being on it.
* **The dead edge of `> 0` is never read as absent**
  (`the_positive_dead_edge_never_reads_absent`,
  `a_close_on_the_positive_dead_edge_refuses`), while `!= -1`'s is
  (`the_minus_one_dead_edge_reads_absent`).
* **A join never drops a name that may be live**
  (`a_join_never_drops_a_possibly_live_name`,
  `the_table_join_keeps_a_possibly_live_name`), and two stale names stay
  stale (`two_stale_names_join_stale`).
* **The holder's release ends the protocol only without a live cell**
  (`a_holder_release_ends_the_protocol_only_without_a_live_cell`), naming the
  zero reading where a `> 0` dead edge left a cell undecided
  (`a_zero_reading_names_the_release`).
* **Each end is its own guard** (`a_record_keeps_its_protocols_iff_every_end_does`,
  `a_record_release_refuses_iff_some_end_may_hold_live`).
* **Vectors.** `per_end_vectors_hold` (Rust `protocol::PER_END_VECTORS`).
-/

namespace QCore.CellProtocolPerEnd

open QCore.DescriptorCells
open QCore.DescriptorCellProtocol (Guard GuardFact saysLive)

/-! ## The guard's edges -/

/-- Rule 38's premise: a live descriptor is never `-1`. -/
def possibleDescriptor (v : Int) : Prop := v ≠ -1

/-- The values on a guard's dead edge. -/
def deadValue : Guard → Int → Bool
  | .notMinusOne, v => v == -1
  | .positive, v => decide (v ≤ 0)
  | .equalOne, v => v != 1

/-- Rust `Guard::dead_edge_decides`. -/
def deadEdgeDecides : Guard → Bool
  | .positive => false
  | _ => true

/-- **A sentinel's dead edge decides exactly when it holds no possible
descriptor.** -/
theorem dead_edge_decides_iff_no_descriptor_on_it (g : Guard) (hs : g ≠ .equalOne) :
    deadEdgeDecides g = true ↔ ∀ v, deadValue g v = true → ¬ possibleDescriptor v := by
  cases g with
  | notMinusOne =>
    simp only [deadEdgeDecides, deadValue, possibleDescriptor, true_iff]
    intro v hv
    simp at hv
    simp [hv]
  | positive =>
    simp only [deadEdgeDecides, deadValue, possibleDescriptor]
    constructor
    · intro h
      cases h
    · intro h
      have h0 := h 0 (by decide)
      exact absurd (by decide : (0 : Int) ≠ -1) h0
  | equalOne => exact absurd rfl hs

/-- **Descriptor 0 is on the `> 0` dead edge**: a success value of every
creator and calloc's absent value at once, so that edge never decides. -/
theorem descriptor_zero_is_on_the_positive_dead_edge :
    deadValue .positive 0 = true ∧ possibleDescriptor 0 ∧ deadEdgeDecides .positive = false := by
  refine ⟨by decide, ?_, rfl⟩
  simp [possibleDescriptor]

/-- **A sentinel's live edge excludes its dead set**: on it the cell holds
no absent value, so the protocol names it live. -/
theorem the_live_edge_is_disjoint_from_the_dead_set (g : Guard) (hs : g ≠ .equalOne) (v : Int)
    (h : saysLive g v = true) : deadValue g v = false := by
  cases g with
  | notMinusOne => simpa [saysLive, deadValue] using h
  | positive =>
    simp [saysLive] at h
    simp [deadValue]
    omega
  | equalOne => exact absurd rfl hs

/-! ## One per-end group: `c` (key 0) and its `#same` name `m` (key 1) -/

/-- The identity a join gives names the paths disagree on (Rust `JOINED`). -/
def joined : Nat := 0

/-- One group's fact on one path; `zero`: a `> 0` dead edge was read. -/
structure Fact where
  guard : Option GuardFact
  touched : Bool
  entry : Table
  zero : Bool

def cells : List Key := [0, 1]

def empty : Table := fun _ => none

def status (f : Fact) (t : Table) (k : Key) : Option (Nat × Status) :=
  match t k with
  | some v => some v
  | none => f.entry k

def liveId (f : Fact) (t : Table) (k : Key) : Option Nat :=
  match status f t k with
  | some (i, .live) => some i
  | _ => none

def allLive (f : Fact) (t : Table) : Bool :=
  match liveId f t 0, liveId f t 1 with
  | some a, some b => a == b
  | _, _ => false

def notLive : Option (Nat × Status) → Bool
  | none => true
  | some (_, .stale) => true
  | _ => false

def noneLive (f : Fact) (t : Table) : Bool :=
  cells.all (fun k => notLive (status f t k))

def holds (g : Guard) (f : Fact) (t : Table) : Bool :=
  !f.touched
    || (allLive f t && (g == .notMinusOne || f.guard == some .live))
    || (noneLive f t && f.guard == some .dead)

/-- A guard test's edge: the live edge introduces the names live, a dead edge
that does not decide introduces them undecided, a deciding one none. -/
def introduced (g : Guard) (live : Bool) (i : Nat) : Fact :=
  let undecided := !live && !deadEdgeDecides g
  { guard := some (if live then .live else .dead)
    touched := false
    entry := fun k =>
      if k = 0 ∨ k = 1 then
        (if live then some (i, .live) else if undecided then some (i, .maybe) else none)
      else none
    zero := undecided }

def reached : Fact := { guard := none, touched := true, entry := empty, zero := false }

/-- Adopt `k`'s pending name into the table; the record is touched. -/
def adopt (f : Fact) (t : Table) (k : Key) : Fact × Table :=
  let f' := { f with touched := true, entry := fun k' => if k' = k then none else f.entry k' }
  match t k, f.entry k with
  | none, some v => (f', set t k v)
  | _, _ => (f', t)

def staleIdentity (f : Fact) (i : Nat) : Fact :=
  { f with entry := staleAll f.entry i }

/-- A store of `v` to `k` (the guard is `c`, key 0). -/
def store (g : Guard) (f : Fact) (t : Table) (k : Key) (v : Int) : Except Bool Fact :=
  -- `.error true`: overwritten; `.error false`: broken.
  let f := { f with touched := true }
  if !notLive (status f t k) then .error true
  else if k = 0 then
    if saysLive g v then .error false else .ok { f with guard := some .dead }
  else .ok f

def mayLive (v : Option (Nat × Status)) : Bool := !notLive v

/-- The identity two joined names keep: theirs when they agree. -/
def joinedId : Option (Nat × Status) → Option (Nat × Status) → Nat
  | some (i, _), some (j, _) => if i = j then i else joined
  | _, _ => joined

/-- One pending name of a join (Rust `Fact::join`'s entry). -/
def joinPending (x y : Option (Nat × Status)) : Option (Nat × Status) :=
  if x = y then x
  else if mayLive x || mayLive y then some (joinedId x y, .maybe)
  else none

/-- Rust `Fact::join`. -/
def join (a b : Fact) : Fact :=
  { guard := if a.guard = b.guard then a.guard else some .conflict
    touched := a.touched || b.touched
    entry := fun k => joinPending (a.entry k) (b.entry k)
    zero := a.zero || b.zero }

/-- Rust `Fact::normalize`: pending names whose cell either table records go
into this path's table. -/
def normalize (f : Fact) (t other : Table) : Fact × Table :=
  let known := fun k => (t k).isSome || (other k).isSome
  ({ f with entry := fun k => if known k ∧ (t k).isNone then none else f.entry k },
   fun k => match t k with
     | some v => some v
     | none => if known k then f.entry k else none)

/-- One name of a table join (Rust `Cells::join_keeping_stale` with every key
a protocol cell). -/
def joinEntry : Option (Nat × Status) → Option (Nat × Status) → Option (Nat × Status)
  | some x, some y =>
    if x = y then some x
    else if x.2 = .stale ∧ y.2 = .stale then some (joined, .stale)
    else some (if x.1 = y.1 then x.1 else joined, .maybe)
  | some (_, .stale), none => some (joined, .stale)
  | none, some (_, .stale) => some (joined, .stale)
  | none, none => none
  | _, _ => some (joined, .maybe)

def tableJoin (a b : Table) : Table := fun k => joinEntry (a k) (b k)

/-- The holder's release: `.ok` ends the protocol; `.error true` is the zero
reading, `.error false` the owner-released refusal. -/
def releaseHolder (f : Fact) (t : Table) : Except Bool Unit :=
  if noneLive f t then .ok () else .error f.zero

/-! ## The theorems -/

/-- **The `> 0` dead edge is never read as absent**: its names may be live. -/
theorem the_positive_dead_edge_never_reads_absent (i : Nat) :
    noneLive (introduced .positive false i) empty = false := by
  simp [noneLive, introduced, cells, status, empty, notLive, deadEdgeDecides]

/-- **The `!= -1` dead edge reads absent**: no name is introduced. -/
theorem the_minus_one_dead_edge_reads_absent (i : Nat) :
    noneLive (introduced .notMinusOne false i) empty = true := by
  simp [noneLive, introduced, cells, status, empty, notLive, deadEdgeDecides]

/-- **A close on the `> 0` dead edge refuses** (`descriptor-cell-undecided`). -/
theorem a_close_on_the_positive_dead_edge_refuses (i : Nat) :
    release (adopt (introduced .positive false i) empty 0).2 0 true = .error .undecided := by
  simp [release, adopt, introduced, empty, deadEdgeDecides, QCore.DescriptorCells.set]

theorem maybe_may_live (i : Nat) : mayLive (some (i, .maybe)) = true := rfl

/-- **A join never drops a name that may be live.** -/
theorem a_join_never_drops_a_possibly_live_name (a b : Fact) (k : Key)
    (h : mayLive (a.entry k) = true) : mayLive ((join a b).entry k) = true := by
  show mayLive (joinPending (a.entry k) (b.entry k)) = true
  unfold joinPending
  split
  · exact h
  · split
    · exact maybe_may_live _
    · rename_i h2
      simp [h] at h2

/-- **The table join keeps a name that may be live.** -/
theorem the_table_join_keeps_a_possibly_live_name (a b : Table) (k : Key)
    (h : mayLive (a k) = true) : mayLive (tableJoin a b k) = true := by
  show mayLive (joinEntry (a k) (b k)) = true
  rcases ha : a k with _ | ⟨i, s⟩
  · rw [ha] at h
    simp [mayLive, notLive] at h
  · rw [ha] at h
    have hs : s ≠ .stale := by
      intro e
      subst e
      simp [mayLive, notLive] at h
    rcases hb : b k with _ | ⟨j, t⟩
    · cases s
      · exact maybe_may_live _
      · exact maybe_may_live _
      · exact absurd rfl hs
    · rw [joinEntry.eq_1]
      split
      · exact h
      · split
        · rename_i hst
          exact absurd hst.1 hs
        · exact maybe_may_live _

/-- **Two stale names join stale**, whatever identities they name. -/
theorem two_stale_names_join_stale (a b : Table) (k : Key) (i j : Nat)
    (ha : a k = some (i, .stale)) (hb : b k = some (j, .stale)) :
    notLive (tableJoin a b k) = true := by
  show notLive (joinEntry (a k) (b k)) = true
  rw [ha, hb, joinEntry.eq_1]
  split
  · rfl
  · split
    · rfl
    · rename_i h2
      exact absurd ⟨rfl, rfl⟩ h2

/-- **The holder's release ends the protocol only without a live cell.** -/
theorem a_holder_release_ends_the_protocol_only_without_a_live_cell (f : Fact) (t : Table) :
    releaseHolder f t = .ok () ↔ noneLive f t = true := by
  unfold releaseHolder
  split <;> simp_all

/-- **The zero reading names the release** where a `> 0` dead edge left a
cell undecided. -/
theorem a_zero_reading_names_the_release (i : Nat) :
    releaseHolder (introduced .positive false i) empty = .error true := by
  have h := the_positive_dead_edge_never_reads_absent i
  unfold releaseHolder
  split
  · rename_i h'
    rw [h] at h'
    exact absurd h' (by decide)
  · rfl

/-! ## Each end is its own guard -/

/-- A record's ends: each its guard, its fact and its table view. -/
abbrev Ends := List (Guard × Fact × Table)

/-- **A record keeps its protocols exactly when every end does.** -/
theorem a_record_keeps_its_protocols_iff_every_end_does (ends : Ends) :
    ends.all (fun e => holds e.1 e.2.1 e.2.2) = true ↔
      ∀ e ∈ ends, holds e.1 e.2.1 e.2.2 = true := by
  simp [List.all_eq_true]

/-- **A record's release refuses exactly when some end may hold a live
cell.** -/
theorem a_record_release_refuses_iff_some_end_may_hold_live (ends : Ends) :
    ends.any (fun e => !noneLive e.2.1 e.2.2) = true ↔
      ∃ e ∈ ends, releaseHolder e.2.1 e.2.2 ≠ .ok () := by
  simp only [List.any_eq_true, Bool.not_eq_true']
  constructor
  · rintro ⟨e, he, hn⟩
    refine ⟨e, he, ?_⟩
    rw [Ne, a_holder_release_ends_the_protocol_only_without_a_live_cell]
    simp [hn]
  · rintro ⟨e, he, hr⟩
    refine ⟨e, he, ?_⟩
    rw [Ne, a_holder_release_ends_the_protocol_only_without_a_live_cell] at hr
    simpa using hr

/-! ## The shared vectors -/

inductive Step where
  | testLive
  | testDead
  | close (k : Key)
  | store (k : Key) (v : Int)
  | join
  | call
  | free
  deriving DecidableEq, Repr

inductive Outcome where
  | holds
  | broken
  | overwritten
  | doubleRelease
  | undecided
  | ownerReleased
  | zeroUndecided
  deriving DecidableEq, Repr

def refusalOutcome : Refusal → Outcome
  | .undecided => .undecided
  | .overwritten => .overwritten
  | .doubleRelease => .doubleRelease

/-- Run a trace as Rust `run_per_end_vector` does (identity 100); `live`: the
edge the last test took. -/
def run (g : Guard) : List Step → Option Fact → Table → Bool → Outcome
  | [], none, _, _ => .holds
  | [], some f, t, _ => if holds g f t then .holds else .broken
  | .testLive :: rest, _, t, _ => run g rest (some (introduced g true 100)) t true
  | .testDead :: rest, _, t, _ => run g rest (some (introduced g false 100)) t false
  | .close k :: rest, f, t, live =>
    let (f, t) := adopt (f.getD reached) t k
    match t k with
    | none => .undecided
    | some (i, _) =>
      match release t k true with
      | .ok t => run g rest (some (staleIdentity f i)) t live
      | .error r => refusalOutcome r
  | .store k v :: rest, f, t, live =>
    match store g (f.getD reached) t k v with
    | .ok f => run g rest (some f) t live
    | .error true => .overwritten
    | .error false => .broken
  | .join :: rest, f, t, live =>
    match f with
    | none => .holds
    | some f =>
      let other := introduced g (!live) 100
      let (f, t') := normalize f t empty
      let (o, ot) := normalize other empty t'
      run g rest (some (join f o)) (tableJoin t' ot) live
  | .call :: rest, f, t, live =>
    match f with
    | some f => if holds g f t then run g rest none empty live else .broken
    | none => run g rest none empty live
  | .free :: rest, f, t, live =>
    match f with
    | some f =>
      match releaseHolder f t with
      | .ok () => run g rest none t live
      | .error true => .zeroUndecided
      | .error false => .ownerReleased
    | none => run g rest none t live

/-- Rust `PER_END_VECTORS`, entry for entry (`pos:` is `> 0`, `nm1:` is
`!= -1`; `c` is key 0, `m` key 1). -/
def perEndVectors : List (Guard × List Step × Outcome) := [
  (.positive, [.testLive, .close 0, .store 0 0], .holds),
  (.positive, [.testLive, .close 0], .broken),
  (.positive, [.testLive, .close 0, .free], .holds),
  (.positive, [.testDead], .holds),
  (.positive, [.testDead, .free], .zeroUndecided),
  (.positive, [.testDead, .close 0], .undecided),
  (.positive, [.testLive, .close 0, .join], .broken),
  (.positive, [.testLive, .close 0, .join, .free], .zeroUndecided),
  (.notMinusOne, [.testLive, .close 0, .join, .free], .holds),
  (.notMinusOne, [.testDead, .free], .holds),
  (.positive, [.testLive, .close 0, .call], .broken),
  (.positive, [.testLive, .close 0, .close 1], .doubleRelease),
  (.positive, [.testLive, .store 0 0], .overwritten),
  (.positive, [.testDead, .store 0 0], .overwritten),
  (.notMinusOne, [.testDead, .store 0 (-1)], .holds),
  (.positive, [.testLive, .free], .ownerReleased),
  (.positive, [.testLive, .call], .holds)
]

theorem per_end_vectors_hold :
    perEndVectors.all (fun (g, trace, o) => run g trace none empty false == o) = true := by
  decide

end QCore.CellProtocolPerEnd
