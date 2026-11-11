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

import QCore.DescriptorCells

/-!
# Declared cell protocols (card MK-EVENT-CELL-CROSS-FUNCTION, gateway rule 38c)

The model of contract specification C04.6.3 and gateway rule 38c. USER
DECISION 2026-11-02 (verbatim): "B as recommended". Abstract rule theorems,
not an extraction of the Rust analyzer
(`crates/gateway-rules/rule-38-descriptor-cells/src/protocol.rs`: `Fact`,
`PROTOCOL_VECTORS`).

A record type's declared protocol (`quant cell`) says: whenever the guard
holds, each `#fresh` cell names its own live descriptor and each `#same`
cell its target's; otherwise no cell names a live one. A body keeps, per
record, a fact: what the guard says, whether the body touched the record,
and the caller's identities a guard test's live edge introduced (pending
until an operation reads the cell). The model fixes the vectors' record: a
`!= -1` sentinel cell `r` (key 0, the guard), a second end `w` (key 1) and a
member `m` (key 2) that is `#same(r)`; the guard kind is a parameter of the
general theorems.

* **A forwarded cell equals the in-body reading**
  (`a_forwarded_cell_equals_the_in_body_reading`): adopting the three names
  a live test introduced gives exactly the table an in-body creation of the
  channel gives (two identities, `m` a second name of `r`'s), so every later
  release and guard decision is the one rule 38 makes in one body.
* **An unresolved forward never decides**: an untested record introduces no
  name, so its guard is undecided (`an_untested_record_decides_no_guard`); a
  test never overrides a fact the path holds (`a_test_never_overrides_a_fact`);
  a touched record whose join disagrees on the guard keeps its protocol only
  when every cell is live under its identity, never by the guard
  (`a_conflicting_guard_never_decides_dead`).
* **A forwarded cell ends only by its own close or its holder's release**: a
  record that keeps its protocol with the guard dead has no live name
  (`a_dead_protocol_names_no_live_cell`), a store never overwrites a live
  name (`a_store_never_overwrites_a_live_cell`), a release of one name stales
  its identity's pending names too (`a_release_stales_the_pending_names`), and
  a live name keeps the holder's release refused (`a_live_cell_holds_the_record`).
* **Guards.** A creation establishes only a `!= -1` guard
  (`a_creation_establishes_only_the_sentinel`); a sentinel stored with a
  live-looking value breaks the protocol (`a_live_looking_sentinel_breaks`).
* **Vectors.** `protocol_vectors_hold` (Rust `protocol::PROTOCOL_VECTORS`).
-/

namespace QCore.DescriptorCellProtocol

open QCore.DescriptorCells

/-- The guard of a protocol. -/
inductive Guard where
  | notMinusOne
  | positive
  | equalOne
  deriving DecidableEq, Repr

/-- Whether a stored integer makes the guard say live. -/
def saysLive : Guard → Int → Bool
  | .notMinusOne, v => v != -1
  | .positive, v => decide (v > 0)
  | .equalOne, v => v == 1

/-- What the guard says on a path. -/
inductive GuardFact where
  | live
  | dead
  | conflict
  deriving DecidableEq, Repr

/-- One record's fact on one path; `entry` holds the pending names. -/
structure Fact where
  guard : Option GuardFact
  touched : Bool
  entry : Table

/-- The record's cells (`r`, `w`, `m`) and the `#fresh` cell each names. -/
def cells : List Key := [0, 1, 2]

def groupOf : Key → Key
  | 2 => 0
  | k => k

/-- A cell's status: the table's, else the pending entry's. -/
def status (f : Fact) (t : Table) (k : Key) : Option (Nat × Status) :=
  match t k with
  | some v => some v
  | none => f.entry k

def liveId (f : Fact) (t : Table) (k : Key) : Option Nat :=
  match status f t k with
  | some (i, .live) => some i
  | _ => none

/-- Every cell live under its group's identity, the two groups distinct. -/
def allLive (f : Fact) (t : Table) : Bool :=
  match liveId f t 0, liveId f t 1, liveId f t 2 with
  | some a, some b, some c => a != b && c == a
  | _, _, _ => false

def notLive : Option (Nat × Status) → Bool
  | none => true
  | some (_, .stale) => true
  | _ => false

/-- No cell may be live. -/
def noneLive (f : Fact) (t : Table) : Bool :=
  cells.all (fun k => notLive (status f t k))

/-- Whether the record keeps its protocol on this path. -/
def holds (g : Guard) (f : Fact) (t : Table) : Bool :=
  !f.touched
    || (allLive f t && (g == .notMinusOne || f.guard == some .live))
    || (noneLive f t && f.guard == some .dead)

def empty : Table := fun _ => none

/-- A guard test's edge: the live edge introduces identities `i` (for `r`
and `m`) and `i + 1` (for `w`). -/
def introduced (live : Bool) (i : Nat) : Fact :=
  { guard := some (if live then .live else .dead)
    touched := false
    entry := if live then
      (fun k => if k = 0 then some (i, .live) else if k = 1 then some (i + 1, .live)
        else if k = 2 then some (i, .live) else none)
      else empty }

/-- Read a test only where the path holds no fact. -/
def test (current : Option Fact) (live : Bool) (i : Nat) : Option Fact :=
  match current with
  | some f => some f
  | none => some (introduced live i)

/-- Adopt `k`'s pending name into the table; the record is touched. -/
def adopt (f : Fact) (t : Table) (k : Key) : Fact × Table :=
  let f' := { f with touched := true, entry := fun k' => if k' = k then none else f.entry k' }
  match t k, f.entry k with
  | none, some v => (f', set t k v)
  | _, _ => (f', t)

/-- A release of identity `i` stales its pending names. -/
def staleIdentity (f : Fact) (i : Nat) : Fact :=
  { f with entry := staleAll f.entry i }

/-- A store of `v` to cell `k` (the guard is `r`, key 0). -/
def store (g : Guard) (f : Fact) (t : Table) (k : Key) (v : Option Int) :
    Except Refusal Fact :=
  let f := { f with touched := true }
  match v with
  | none => .error .undecided
  | some v =>
    if !notLive (status f t k) then .error .overwritten
    else if k = 0 then
      if saysLive g v then .error .undecided else .ok { f with guard := some .dead }
    else .ok f

/-- Join two facts of one record. -/
def join (a b : Fact) : Fact :=
  { guard := if a.guard = b.guard then a.guard else some .conflict
    touched := a.touched || b.touched
    entry := fun k => if a.entry k = b.entry k then a.entry k else none }

/-- The in-body creation of the channel: `r` and `w` fresh (identities `i`,
`i + 1`), `m` a second name of `r`'s, the witness holding. -/
def inBody (i : Nat) : Table :=
  set (set (set empty 0 (i, .live)) 1 (i + 1, .live)) 2 (i, .live)

/-- Adopt the three names in order. -/
def adoptAll (f : Fact) : Fact × Table :=
  let (f, t) := adopt f empty 0
  let (f, t) := adopt f t 1
  adopt f t 2

/-- **A forwarded cell equals the in-body reading.** -/
theorem a_forwarded_cell_equals_the_in_body_reading (i : Nat) (k : Key) :
    (adoptAll (introduced true i)).2 k = inBody i k := by
  rcases k with _ | _ | _ | k <;> simp [adoptAll, adopt, introduced, inBody, QCore.DescriptorCells.set, empty]

/-- The adopted fact keeps no pending name and is touched. -/
theorem the_adopted_names_leave_the_entry (i : Nat) (k : Key) :
    (adoptAll (introduced true i)).1.entry k = none ∧
      (adoptAll (introduced true i)).1.touched = true := by
  rcases k with _ | _ | _ | k <;> simp [adoptAll, adopt, introduced, QCore.DescriptorCells.set, empty]

/-- So the guard over `m` and `r` decides equal, as in one body. -/
theorem the_forwarded_guard_decides_equal (i : Nat) :
    equality ((adoptAll (introduced true i)).2 2) ((adoptAll (introduced true i)).2 0)
      = .equal := by
  simp [a_forwarded_cell_equals_the_in_body_reading, inBody, QCore.DescriptorCells.set, equality]

/-- **An untested record decides no guard**: nothing is introduced. -/
theorem an_untested_record_decides_no_guard (k k' : Key) :
    equality (empty k) (empty k') = .undecided := by
  simp [empty, equality]

/-- **A test never overrides a fact the path holds.** -/
theorem a_test_never_overrides_a_fact (f : Fact) (live : Bool) (i : Nat) :
    test (some f) live i = some f := rfl

/-- **A conflicting guard never decides dead.** -/
theorem a_conflicting_guard_never_decides_dead (g : Guard) (f : Fact) (t : Table)
    (hc : f.guard = some .conflict) (ht : f.touched = true) (hl : allLive f t = false) :
    holds g f t = false := by
  simp [holds, hc, ht, hl]

/-- **A record that keeps its protocol with the guard dead names no live
cell**: each was released, or never live. -/
theorem a_dead_protocol_names_no_live_cell (g : Guard) (f : Fact) (t : Table)
    (hd : f.guard = some .dead) (ht : f.touched = true) (hl : allLive f t = false)
    (h : holds g f t = true) : noneLive f t = true := by
  simp [holds, hd, ht, hl] at h
  exact h

/-- **A store never overwrites a live name.** -/
theorem a_store_never_overwrites_a_live_cell (g : Guard) (f : Fact) (t : Table)
    (k : Key) (v : Int) (i : Nat) (h : status f t k = some (i, .live)) :
    store g f t k (some v) = .error .overwritten := by
  simp [store, status] at h ⊢
  split at h <;> simp_all [notLive]

/-- **A release of one name stales its identity's pending names.** -/
theorem a_release_stales_the_pending_names (f : Fact) (i : Nat) (k : Key) (s : Status)
    (h : f.entry k = some (i, s)) :
    (staleIdentity f i).entry k = some (i, .stale) := by
  simp [staleIdentity, staleAll, h]

/-- **A live name holds the record**: the holder's release refuses while one
may be live. -/
theorem a_live_cell_holds_the_record (f : Fact) (t : Table) (k : Key) (i : Nat)
    (hk : k ∈ cells) (h : status f t k = some (i, .live)) : noneLive f t = false := by
  simp only [noneLive, List.all_eq_false]
  exact ⟨k, hk, by simp [h, notLive]⟩

/-- **A creation establishes only the `!= -1` guard**: over live cells with
the guard said dead, only a sentinel `!= -1` protocol holds. -/
theorem a_creation_establishes_only_the_sentinel (f : Fact) (t : Table)
    (hd : f.guard = some .dead) (ht : f.touched = true) (hl : allLive f t = true) :
    holds .notMinusOne f t = true ∧ holds .positive f t = false ∧
      holds .equalOne f t = false := by
  have hn : noneLive f t = false := by
    simp only [allLive] at hl
    split at hl
    · rename_i a b c ha _ _
      simp only [liveId] at ha
      split at ha
      · rename_i i hs
        exact a_live_cell_holds_the_record f t 0 i (by simp [cells]) hs
      · cases ha
    · cases hl
  simp [holds, hd, ht, hl, hn]

/-- **A sentinel stored with a live-looking value breaks the protocol.** -/
theorem a_live_looking_sentinel_breaks (f : Fact) (t : Table) (v : Int)
    (hn : notLive (status f t 0) = true) (hv : v ≠ -1) :
    store .notMinusOne f t 0 (some v) = .error .undecided := by
  simp [store, status] at hn ⊢
  simp [hn, saysLive, hv]

/-! ## The shared vectors -/

/-- A vector step (keys: `r` = 0, `w` = 1, `m` = 2). -/
inductive Step where
  | testLive
  | testDead
  | release (k : Key)
  | store (k : Key) (v : Int)
  | create
  | fail
  | joinDead
  deriving DecidableEq, Repr

inductive Outcome where
  | holds
  | broken
  | overwritten
  | doubleRelease
  | undecided
  deriving DecidableEq, Repr

def outcomeOf : Refusal → Outcome
  | .undecided => .undecided
  | .overwritten => .overwritten
  | .doubleRelease => .doubleRelease

/-- A store refusal of a value the protocol cannot read is `broken` in the
vectors, as Rust's `ProtocolBroken`. -/
def storeOutcome : Refusal → Outcome
  | .overwritten => .overwritten
  | _ => .broken

def reached : Fact := { guard := none, touched := true, entry := empty }

/-- Run a trace with an optional fact and a table, as Rust
`run_protocol_vector` does (identity 100 for a test, 1 and 2 for a creation). -/
def run : List Step → Option Fact → Table → Outcome
  | [], none, _ => .holds
  | [], some f, t => if holds .notMinusOne f t then .holds else .broken
  | .testLive :: rest, _, t => run rest (some (introduced true 100)) t
  | .testDead :: rest, _, t => run rest (some (introduced false 100)) t
  | .release k :: rest, f, t =>
    let (f, t) := adopt (f.getD reached) t k
    match t k with
    | none => .undecided
    | some (i, _) =>
      match QCore.DescriptorCells.release t k true with
      | .ok t => run rest (some (staleIdentity f i)) t
      | .error r => outcomeOf r
  | .store k v :: rest, f, t =>
    match store .notMinusOne (f.getD reached) t k (some v) with
    | .ok f => run rest (some f) t
    | .error r => storeOutcome r
  | .create :: rest, f, t =>
    let f := { f.getD reached with touched := true }
    match QCore.DescriptorCells.create t 0 1 true with
    | .error r => outcomeOf r
    | .ok t => match QCore.DescriptorCells.create t 1 2 true with
      | .error r => outcomeOf r
      | .ok t => match QCore.DescriptorCells.alias t 2 0 with
        | .error r => outcomeOf r
        | .ok t => run rest (some f) (witness (witness t 1 true) 2 true)
  | .fail :: rest, f, t =>
    let f := { f.getD reached with touched := true }
    match QCore.DescriptorCells.create t 0 1 true with
    | .error r => outcomeOf r
    | .ok t => match QCore.DescriptorCells.create t 1 2 true with
      | .error r => outcomeOf r
      | .ok t => match QCore.DescriptorCells.alias t 2 0 with
        | .error r => outcomeOf r
        | .ok t => run rest (some f) (witness (witness t 1 false) 2 false)
  | .joinDead :: rest, f, t =>
    run rest (f.map (fun f => join f (introduced false 100))) (joinEmpty t)

/-- Rust `PROTOCOL_VECTORS`, step for step. -/
def protocolVectors : List (List Step × Outcome) := [
  ([.testLive], .holds),
  ([.testDead], .holds),
  ([.testLive, .release 0, .release 1, .store 0 (-1)], .holds),
  ([.testLive, .release 0, .release 1], .broken),
  ([.testLive, .release 0, .release 1, .store 0 (-1), .store 1 (-1)], .holds),
  ([.testLive, .release 0, .release 2], .doubleRelease),
  ([.testLive, .store 0 (-1)], .overwritten),
  ([.testDead, .release 0], .undecided),
  ([.store 0 (-1), .create], .holds),
  ([.store 0 (-1), .fail], .holds),
  ([.store 0 5], .broken),
  ([.testLive, .release 0, .release 1, .store 0 (-1), .joinDead], .holds),
  ([.testLive, .release 1, .joinDead], .broken)
]

theorem protocol_vectors_hold :
    protocolVectors.all (fun (trace, o) => run trace none empty == o) = true := by
  decide

end QCore.DescriptorCellProtocol
