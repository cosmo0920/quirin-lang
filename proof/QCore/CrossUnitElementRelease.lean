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

import QCore.CrossUnitSummary
import QCore.ElementReleaseSummary

/-!
# A released-element summary across units (card LERI-CROSS-UNIT-ELEMENTS-RELEASED)

Gateway rule 43 attaches `ElementsReleased` to a derived summary; rule 4c
transports a derived summary from its home unit to a caller in another unit,
and rule 40 derives the link resolution that names the home. This module
states what the transport must keep and what every consumer must do with the
clause once it has crossed.

* **Transport keeps the clause.** The caller's certificate replays the home
  and compares the imported contract with the replayed one *whole*
  (`check_imports`: `validated == inferred_imports`), container effects
  included. So an import that passes carries exactly the home's clauses
  (`transport_keeps_the_clause`), an import that dropped or added one is
  refused (`a_changed_clause_is_refused`), and the home's gate still holds of
  the import (`the_gate_crosses_with_the_summary`).
* **Every consumer applies the clause or refuses.** A consumer either applies
  container effects (the gateway's call reading: `forget_published`) or
  cannot (a borrow-only reading, the extern-call bridge). A summary with a
  clause is read by the first as *forget* and refused by the second; neither
  reads it as a borrow (`no_consumer_borrows_a_released_summary`,
  `a_consumer_that_cannot_apply_refuses`). The reading of an admitted import
  is the reading of the home's own summary (`the_import_reads_as_the_home`),
  so rule 43's caller-side soundness (`forgetting_covers_every_release`)
  holds across units unchanged (`forgetting_holds_across_units`).
* **No import, no reading.** A refused home supplies nothing; the call is
  uncontracted and refused (`a_missing_import_is_refused`), an incomplete
  definition census imports nothing (`an_incomplete_census_imports_nothing`),
  and a declaration in force wins over an import (rule 4c's
  `declaration_wins`).
* **One crossing.** A caller that carries the clause on is never a home: a
  home's environment holds no import (`a_carried_clause_crosses_no_second_unit`).

## What this is not

The Rust/model boundary is the extraction: that `check_imports` compares the
whole contract, that each consumer routes through `has_container_effect`,
and that the route's homes are rule 40's. Those are pinned by
`crates/qcore-c-gateway/tests/cross_unit_elements_released.rs` and
`crates/qcore-audit/tests/link_standing.rs`, whose fixture rows are
`vectors` below. P-LERI-CALLER-PUBLISHED and rule 40's derived link facts are
the premises they were; this module adds none.
-/

namespace QCore.CrossUnitElementRelease

open QCore.ElementReleaseSummary

/-- A summary as the import channel carries it: its custody projection
(abstract) and the positions of its `ElementsReleased` clauses. -/
structure Summary where
  custody : Nat
  released : List Nat
  deriving DecidableEq, Repr

/-- `check_imports`: the imported contract equals the replayed home's. -/
def checked (imported replayed : Summary) : Prop := imported = replayed

theorem transport_keeps_the_clause (imported replayed : Summary)
    (h : checked imported replayed) : imported.released = replayed.released := by
  rw [h]

theorem a_changed_clause_is_refused (imported replayed : Summary)
    (h : imported.released ≠ replayed.released) : ¬ checked imported replayed := by
  intro hc
  exact h (transport_keeps_the_clause _ _ hc)

/-- The home's W4 gate (rule 43, `gate`) is a property of the clauses, so it
holds of every import that passed the check. -/
theorem the_gate_crosses_with_the_summary (roots : List Root) (imported replayed : Summary)
    (hg : gate roots replayed.released) (h : checked imported replayed) :
    gate roots imported.released := by
  rw [transport_keeps_the_clause _ _ h]
  exact hg

/-- What a consumer can do with container effects. -/
inductive Consumer where
  /-- Applies them (the gateway's call reading). -/
  | applies
  /-- Reads custody modes only (a borrow-only route, the extern-call bridge). -/
  | borrowOnly
  deriving DecidableEq, Repr

/-- How one call is read. -/
inductive Reading where
  /-- No clause: the custody modes as stated. -/
  | borrow
  /-- The clause applied: every published object is forgotten. -/
  | forget
  /-- Refused: `Unknown`. -/
  | refused
  deriving DecidableEq, Repr

/-- `has_container_effect` decides the reading. -/
def read (c : Consumer) (s : Summary) : Reading :=
  match s.released with
  | [] => .borrow
  | _ :: _ => match c with
    | .applies => .forget
    | .borrowOnly => .refused

theorem no_consumer_borrows_a_released_summary (c : Consumer) (s : Summary)
    (h : s.released ≠ []) : read c s ≠ .borrow := by
  unfold read
  cases hs : s.released with
  | nil => exact absurd hs h
  | cons p rest => cases c <;> simp

theorem a_consumer_that_cannot_apply_refuses (s : Summary) (h : s.released ≠ []) :
    read .borrowOnly s = .refused := by
  unfold read
  cases hs : s.released with
  | nil => exact absurd hs h
  | cons p rest => rfl

theorem an_applying_consumer_forgets (s : Summary) (h : s.released ≠ []) :
    read .applies s = .forget := by
  unfold read
  cases hs : s.released with
  | nil => exact absurd hs h
  | cons p rest => rfl

theorem the_import_reads_as_the_home (c : Consumer) (imported replayed : Summary)
    (h : checked imported replayed) : read c imported = read c replayed := by
  rw [h]

/-- The caller's state after one call read `r`. -/
def after (r : Reading) (s : Caller) : Option Caller :=
  match r with
  | .borrow => some s
  | .forget => some (apply s)
  | .refused => none

/-- Rule 43's soundness at the caller, across units: an admitted import whose
home released elements is read as *forget*, and then no object the callee
released among the caller's published ones is trusted. -/
theorem forgetting_holds_across_units (imported replayed : Summary) (caller : Caller)
    (releasedByCallee : List Nat) (h : checked imported replayed)
    (hr : replayed.released ≠ [])
    (premise : ∀ o, o ∈ releasedByCallee → o ∈ caller.published) :
    ∃ s', after (read .applies imported) caller = some s' ∧
      ∀ o, o ∈ releasedByCallee → ¬ trusted s' o := by
  have hi : imported.released ≠ [] := by
    rw [transport_keeps_the_clause _ _ h]; exact hr
  refine ⟨apply caller, ?_, ?_⟩
  · simp [an_applying_consumer_forgets _ hi, after]
  · intro o ho
    exact (forgetting_covers_every_release caller releasedByCallee premise o ho).1

/-- A dropped clause would be unsound: the borrow reading keeps a released
object trusted, which is why the check refuses it. -/
theorem a_dropped_clause_would_trust_a_released_object (o : Nat) (custody : Nat) :
    let caller : Caller := { published := [o], withdrawn := [] }
    after (read .applies { custody, released := [] }) caller = some caller ∧
      trusted caller o ∧
      after (read .applies { custody, released := [0] }) caller = some (apply caller) ∧
      ¬ trusted (apply caller) o := by
  simp [read, after, trusted, apply]

/-- The call's summary: a declaration, an import, or nothing. -/
def lookupSummary (declared : CrossUnitSummary.Name → Option Summary)
    (imports : List (CrossUnitSummary.Home Summary)) (name : CrossUnitSummary.Name) :
    Option Summary :=
  CrossUnitSummary.lookup declared imports name

/-- A call with no summary in force is refused (`call to uncontracted
function`), whatever the consumer. -/
def readCall (c : Consumer) : Option Summary → Reading
  | none => .refused
  | some s => read c s

theorem a_missing_import_is_refused (c : Consumer)
    (declared : CrossUnitSummary.Name → Option Summary) (name : CrossUnitSummary.Name)
    (hd : declared name = none) :
    readCall c (lookupSummary declared [] name) = .refused := by
  simp [readCall, lookupSummary, CrossUnitSummary.lookup, hd]

/-- The import phase: the selected homes when the run's definition census is
complete, none otherwise (rule 4c; `current_route.rs`). A unit whose census
is incomplete is not analysed for export at all: the census already refuses,
and the run continues on its first-phase rows. -/
def importPhase (censusComplete : Bool) (selected : List (CrossUnitSummary.Home Summary)) :
    List (CrossUnitSummary.Home Summary) :=
  if censusComplete then selected else []

theorem an_incomplete_census_imports_nothing (c : Consumer)
    (declared : CrossUnitSummary.Name → Option Summary)
    (selected : List (CrossUnitSummary.Home Summary)) (name : CrossUnitSummary.Name)
    (hd : declared name = none) :
    readCall c (lookupSummary declared (importPhase false selected) name) = .refused := by
  simp [importPhase, readCall, lookupSummary, CrossUnitSummary.lookup, hd]

/-- A home's environment holds no import (`export_homes` and `check_imports`
refuse one), so a caller whose summary carries an imported clause on is
never exported again. -/
def exportable (homeImports : List (CrossUnitSummary.Home Summary)) : Prop :=
  homeImports = []

theorem a_carried_clause_crosses_no_second_unit
    (home : CrossUnitSummary.Home Summary) (rest : List (CrossUnitSummary.Home Summary)) :
    ¬ exportable (home :: rest) := by
  simp [exportable]

-- The fixture rows of `crates/qcore-c-gateway/tests/cross_unit_elements_released.rs`
-- (`CROSS_UNIT_VECTORS`): the import's clauses, whether they equal the
-- replayed home's, the consumer, and the outcome (`none`: the import refused).

def outcome (c : Consumer) (imported replayed : Summary) : Option Reading :=
  if imported = replayed then some (read c imported) else none

def vectors : List (Summary × Summary × Consumer × Option Reading) :=
  [ (⟨0, [0]⟩, ⟨0, [0]⟩, .applies, some .forget),
    (⟨0, [0]⟩, ⟨0, [0]⟩, .borrowOnly, some .refused),
    (⟨0, []⟩, ⟨0, [0]⟩, .applies, none),
    (⟨0, [0, 1]⟩, ⟨0, [0]⟩, .applies, none),
    (⟨0, []⟩, ⟨0, []⟩, .applies, some .borrow),
    (⟨0, []⟩, ⟨0, []⟩, .borrowOnly, some .borrow) ]

theorem vectors_hold :
    vectors.all (fun (i, r, c, out) => outcome c i r = out) = true := by
  decide

end QCore.CrossUnitElementRelease
