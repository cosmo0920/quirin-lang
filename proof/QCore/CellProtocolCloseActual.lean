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

import QCore.DescriptorCellProtocol

/-!
# The close actual of a declared cell (card CELL-PROTOCOL-CLOSE-ACTUAL, gateway rule 38d)

USER DECISION 2026-11-02 (verbatim): "B as recommended"; its Q3: a close of a
declared cell member is that cell's `#stale`. Abstract rule theorems, not an
extraction of the Rust analyzer
(`crates/gateway-rules/rule-38-descriptor-cells/src/close_actual.rs`:
`admits`, `reading`, `ADMISSION_VECTORS`, `READING_VECTORS`; the host is
`legacy_call_argument_access.rs`'s `cell_close_actual` and
`legacy_descriptor_cells.rs`'s close step).

The access gate (W44 premise 1) refused an integer member actual at `close`'s
custody position before rule 38c could read `close(r->CELL)`. Rule 38d admits
exactly one more actual there, and its custody effect is rule 38c's close.

* **The admission set is exactly the stated one**
  (`the_admission_set_is_exactly_the_stated_one`): rule 38 in force, rule
  38c's close statement of a declared record's one-segment cell, a
  function-map callee, a release position, the descriptor kind, the integer
  member read of the step's own cell. Every single departure keeps the
  gate's refusal (`every_other_site_keeps_the_refusal`), with rule 38 off
  nothing is admitted (`rule_38_off_admits_nothing`).
* **An admitted close feeds rule 38c's `#stale` step**
  (`an_admitted_close_is_the_cells_stale`): on a path with a fact of the
  caller's record, a cell naming a live identity is released once through
  the cell table (`QCore.DescriptorCells.release`), and the record's pending
  names of that identity are staled (`QCore.DescriptorCellProtocol.staleIdentity`),
  the same step the protocol vectors' `release` take; afterwards no name of
  the identity is live (`the_closed_identity_has_no_live_name`).
* **Never a silent release.** Without a fact rule 38c reads nothing and the
  gate's refusal stands (`an_admitted_close_without_a_fact_refuses`); any
  result is a release of a live single identity
  (`an_admitted_close_never_releases_silently`); a second close, a close on
  the guard's dead edge and a close of the join's identity refuse
  (`a_second_admitted_close_refuses`, `a_close_on_the_dead_edge_refuses`,
  `a_close_of_the_joined_identity_refuses`).
* **The protocol is restored** by the two admitted closes and the sentinel's
  dead store, for both sentinel guards
  (`admitted_closes_and_the_sentinel_restore_the_protocol`).
* **Vectors.** `admission_vectors_hold`, `reading_vectors_hold` (Rust
  `close_actual::ADMISSION_VECTORS`, `close_actual::READING_VECTORS`).
-/

namespace QCore.CellProtocolCloseActual

open QCore.DescriptorCells
open QCore.DescriptorCellProtocol

/-! ## The admission -/

inductive Statement where
  | protocolClose
  | other
  deriving DecidableEq, Repr

inductive Callee where
  | functionMap
  | withheld
  | uncontracted
  deriving DecidableEq, Repr

inductive Position where
  | release
  | otherCustody
  | borrowed
  | beyond
  deriving DecidableEq, Repr

inductive Kind where
  | descriptor
  | other
  deriving DecidableEq, Repr

inductive Actual where
  | cellRead
  | other
  deriving DecidableEq, Repr

/-- One refused statement, as the host reads it. -/
structure Site where
  ruleOn : Bool
  statement : Statement
  callee : Callee
  position : Position
  kind : Kind
  actual : Actual
  deriving DecidableEq, Repr

/-- The one site the gate admits. -/
def admitted : Site :=
  { ruleOn := true, statement := .protocolClose, callee := .functionMap,
    position := .release, kind := .descriptor, actual := .cellRead }

/-- The gate's admission, condition by condition. -/
def admits (s : Site) : Bool :=
  s.ruleOn && s.statement == .protocolClose && s.callee == .functionMap
    && s.position == .release && s.kind == .descriptor && s.actual == .cellRead

/-- **The admission set is exactly the stated one.** -/
theorem the_admission_set_is_exactly_the_stated_one (s : Site) :
    admits s = true ↔ s = admitted := by
  obtain ⟨r, st, c, p, k, a⟩ := s
  constructor
  · intro h
    simp only [admits, Bool.and_eq_true, beq_iff_eq] at h
    obtain ⟨⟨⟨⟨⟨hr, hst⟩, hc⟩, hp⟩, hk⟩, ha⟩ := h
    subst hr hst hc hp hk ha
    rfl
  · intro h
    cases h
    rfl

/-- Every other site keeps the gate's refusal. -/
theorem every_other_site_keeps_the_refusal (s : Site) (h : s ≠ admitted) :
    admits s = false := by
  cases hs : admits s
  · rfl
  · exact absurd ((the_admission_set_is_exactly_the_stated_one s).mp hs) h

/-- With rule 38 off nothing is admitted. -/
theorem rule_38_off_admits_nothing (s : Site) (h : s.ruleOn = false) :
    admits s = false := by
  simp [admits, h]

/-! ## Rule 38c's reading of an admitted close -/

/-- What one path's table says of the closed cell, after adoption. -/
inductive Cell where
  | live
  | joinedLive
  | maybe
  | stale
  | absent
  deriving DecidableEq, Repr

inductive Reading where
  | stale
  | gate
  | refused (r : Refusal)
  deriving DecidableEq, Repr

/-- Rule 38c's reading; `fact`: the path holds a fact of the caller's record. -/
def reading (fact : Bool) (c : Cell) : Reading :=
  if !fact then .gate
  else match c with
    | .live => .stale
    | .stale => .refused .doubleRelease
    | .joinedLive => .refused .undecided
    | .maybe => .refused .undecided
    | .absent => .refused .undecided

/-- The view of a table entry; `joined` is the identity a join of two
disagreeing paths gives (Rust `cells::JOINED`). -/
def cellOf (joined : Nat) : Option (Nat × Status) → Cell
  | some (i, .live) => if i = joined then .joinedLive else .live
  | some (_, .maybe) => .maybe
  | some (_, .stale) => .stale
  | none => .absent

/-- The host's admitted close of cell `k` on one path: the pending name is
adopted, then rule 38c's reading decides. `none` is a refusal (the gate's,
or rule 38c's). -/
def admittedClose (joined : Nat) (current : Option Fact) (t : Table) (k : Key) :
    Option (Fact × Table) :=
  match current with
  | none => none
  | some f =>
    let a := adopt f t k
    match reading true (cellOf joined (a.2 k)), a.2 k with
    | .stale, some (i, _) =>
      match release a.2 k true with
      | .ok t' => some (staleIdentity a.1 i, t')
      | .error _ => none
    | _, _ => none

/-- **Without a fact rule 38c reads nothing: the gate's refusal stands.** -/
theorem an_admitted_close_without_a_fact_refuses (j : Nat) (t : Table) (k : Key) :
    admittedClose j none t k = none := rfl

/-- **An admitted close is the cell's `#stale`**: a live single identity is
released once and the record's pending names of it are staled. -/
theorem an_admitted_close_is_the_cells_stale (j : Nat) (f : Fact) (t : Table) (k : Key)
    (i : Nat) (h : (adopt f t k).2 k = some (i, .live)) (hj : i ≠ j) :
    admittedClose j (some f) t k =
      some (staleIdentity (adopt f t k).1 i, staleAll (adopt f t k).2 i) := by
  simp [admittedClose, h, reading, cellOf, hj, release]

/-- After it no name of the closed identity is live. -/
theorem the_closed_identity_has_no_live_name (j : Nat) (f f' : Fact) (t t' : Table)
    (k k' : Key) (i : Nat) (h : (adopt f t k).2 k = some (i, .live)) (hj : i ≠ j)
    (hr : admittedClose j (some f) t k = some (f', t')) :
    t' k' ≠ some (i, .live) := by
  rw [an_admitted_close_is_the_cells_stale j f t k i h hj] at hr
  simp only [Option.some.injEq, Prod.mk.injEq] at hr
  obtain ⟨_, ht⟩ := hr
  subst ht
  simp only [staleAll]
  split
  · rename_i j' s _
    by_cases hji : j' = i
    · simp [hji]
    · simp [hji]
  · simp

/-- **Never a silent release**: an admitted close yields a result only from
a fact and a live single identity of the cell, and the result stales it. -/
theorem an_admitted_close_never_releases_silently (j : Nat) (c : Option Fact) (t : Table)
    (k : Key) (r : Fact × Table) (h : admittedClose j c t k = some r) :
    ∃ f i, c = some f ∧ (adopt f t k).2 k = some (i, .live) ∧ i ≠ j ∧
      r.2 k = some (i, .stale) := by
  cases c with
  | none => cases h
  | some f =>
    refine ⟨f, ?_⟩
    cases ha : (adopt f t k).2 k with
    | none => simp [admittedClose, ha, reading, cellOf] at h
    | some v =>
      obtain ⟨i, s⟩ := v
      cases s with
      | maybe => simp [admittedClose, ha, reading, cellOf] at h
      | stale => simp [admittedClose, ha, reading, cellOf] at h
      | live =>
        by_cases hj : i = j
        · simp [admittedClose, ha, reading, cellOf, hj] at h
        · rw [an_admitted_close_is_the_cells_stale j f t k i ha hj] at h
          simp only [Option.some.injEq] at h
          subst h
          exact ⟨i, rfl, rfl, hj, by simp [staleAll, ha]⟩

/-! ## Traces over the protocol model (`r` = key 0, `w` = 1, `m` = 2) -/

/-- Rust `cells::JOINED`. -/
def joinedIdentity : Nat := 18446744073709551615

/-- A live test (identities 100 and 101), the two ends closed, the sentinel
stored dead: whether the record keeps protocol `g`. -/
def closeTrace (g : Guard) : Bool :=
  match admittedClose joinedIdentity (some (introduced true 100)) empty 0 with
  | none => false
  | some (f, t) =>
    match admittedClose joinedIdentity (some f) t 1 with
    | none => false
    | some (f, t) =>
      match store g f t 0 (some (-1)) with
      | .ok f => holds g f t
      | .error _ => false

/-- **The admitted closes and the sentinel's dead store restore the
protocol**, for both sentinel guards. -/
theorem admitted_closes_and_the_sentinel_restore_the_protocol :
    closeTrace .positive = true ∧ closeTrace .notMinusOne = true := by
  decide

/-- A second close of one cell refuses (never two closes). -/
theorem a_second_admitted_close_refuses :
    (match admittedClose joinedIdentity (some (introduced true 100)) empty 0 with
      | none => false
      | some (f, t) => (admittedClose joinedIdentity (some f) t 0).isNone) = true := by
  decide

/-- A close of the `#same` name after its target's close refuses too. -/
theorem a_close_of_the_same_name_after_its_target_refuses :
    (match admittedClose joinedIdentity (some (introduced true 100)) empty 0 with
      | none => false
      | some (f, t) => (admittedClose joinedIdentity (some f) t 2).isNone) = true := by
  decide

/-- On the guard's dead edge no cell is live: the close refuses. -/
theorem a_close_on_the_dead_edge_refuses (k : Key) :
    (admittedClose joinedIdentity (some (introduced false 100)) empty k).isNone = true := by
  simp [admittedClose, adopt, introduced, empty, reading, cellOf]

/-- A cell live under the join's identity is never released. -/
theorem a_close_of_the_joined_identity_refuses (f : Fact) (t : Table) (k : Key)
    (h : (adopt f t k).2 k = some (joinedIdentity, .live)) :
    admittedClose joinedIdentity (some f) t k = none := by
  simp [admittedClose, h, reading, cellOf]

/-! ## The shared vectors -/

/-- Rust `close_actual::ADMISSION_VECTORS`, entry for entry. -/
def admissionVectors : List (Site × Bool) := [
  (admitted, true),
  ({ admitted with ruleOn := false }, false),
  ({ admitted with statement := .other }, false),
  ({ admitted with callee := .withheld }, false),
  ({ admitted with callee := .uncontracted }, false),
  ({ admitted with position := .otherCustody }, false),
  ({ admitted with position := .borrowed }, false),
  ({ admitted with position := .beyond }, false),
  ({ admitted with kind := .other }, false),
  ({ admitted with actual := .other }, false)
]

theorem admission_vectors_hold :
    admissionVectors.all (fun (s, b) => admits s == b) = true := by
  decide

/-- Rust `close_actual::READING_VECTORS`, entry for entry. -/
def readingVectors : List (Bool × Cell × Reading) := [
  (true, .live, .stale),
  (true, .stale, .refused .doubleRelease),
  (true, .joinedLive, .refused .undecided),
  (true, .maybe, .refused .undecided),
  (true, .absent, .refused .undecided),
  (false, .live, .gate),
  (false, .stale, .gate),
  (false, .joinedLive, .gate),
  (false, .maybe, .gate),
  (false, .absent, .gate)
]

theorem reading_vectors_hold :
    readingVectors.all (fun (f, c, r) => reading f c == r) = true := by
  decide

end QCore.CellProtocolCloseActual
