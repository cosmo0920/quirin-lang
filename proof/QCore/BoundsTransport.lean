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

import QCore.IndexExtent
import QCore.BoundsPinHold
import QCore.RefutedContractWithdrawal

/-!
# Transport of bounds declarations on the audit routes (card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS, rules 14h–14j)

The abstract rules implemented by `crates/qcore-audit/src/refuted_contracts.rs`
(`run_two_phases_with_bounds`), `crates/qcore-audit/src/bounds_selection.rs`
(`apply_refutation_table`, the tip records of `apply_bounds_pins`) and
`crates/qcore-audit/src/checkpoint_run.rs` (the refusal of a checkpointed bounds
run), contract specification C09.1 with C06.2 and C06.4.

* **Refutation (rule 14h).** The bounds table is the union of every unit's
  refuting bodies (`QCore.RefutedContractWithdrawal.table`, so independent of
  unit order and grouping). In the second phase a call to a declaration in the
  table, outside its own body, is CG16's unstated call: its obligation is
  unresolved, so the caller is never `Safe`, and the withdrawal accuses
  nothing. An empty table changes nothing. The table is read from evidence
  computed under the declarations as loaded, so the second phase finds the same
  table as the first.
* **Tips (rule 14i).** A bounds declaration tracking a tip is in force exactly
  when its text is the pinned one, the tip resolves and the audited sites equal
  the tip's; otherwise it is held (`QCore.BoundsPinHold.standing`), and it is a
  listed assumption exactly when in force.
* **Checkpoints (rule 14j).** A checkpoint carries no bounds row, so a run that
  selects bounds is never composed from checkpoints: what a composition would
  publish omits every requested row.

## What this is not

A model of the routes' scheduling or of Git: that each route re-analyses every
unit when either table is not empty, and that tip resolution reads the blobs it
says it reads, are review premises bound by the Rust tests
`crates/qcore-audit/tests/bounds_transport.rs`.
-/

namespace QCore.BoundsTransport

open QCore.IndexExtent

/-! ## Refutation -/

/-- A call obligation of a bounds declaration in phase two: withdrawn (the
unstated call, unresolved) when the declaration is in the table and the call
is not in its own body; otherwise the judgement as decided. -/
def callJudgement (table : List Nat) (callee : Nat) (own : Bool)
    (decided : Outcome × Outcome) : Outcome × Outcome :=
  if !own && table.contains callee then (.unresolved 0, .unresolved 0) else decided

theorem the_empty_table_changes_nothing (c : Nat) (o : Bool) (d : Outcome × Outcome) :
    callJudgement [] c o d = d := by
  simp [callJudgement]

theorem its_own_body_keeps_it (t : List Nat) (c : Nat) (d : Outcome × Outcome) :
    callJudgement t c true d = d := by
  simp [callJudgement]

theorem a_refuted_declaration_is_withdrawn (t : List Nat) (c : Nat) (d : Outcome × Outcome)
    (h : c ∈ t) : callJudgement t c false d = (.unresolved 0, .unresolved 0) := by
  simp [callJudgement, h]

/-- The withdrawal accuses nothing. -/
theorem a_withdrawal_never_accuses (t : List Nat) (c : Nat) (d : Outcome × Outcome)
    (h : c ∈ t) : (callJudgement t c false d).1 ≠ .violation true := by
  rw [a_refuted_declaration_is_withdrawn t c d h]
  simp

/-- A caller of a refuted declaration, outside its own body, is never `Safe`. -/
theorem a_caller_of_a_refuted_declaration_is_never_safe (js : List (Outcome × Outcome))
    (r : Nat) (t : List Nat) (c : Nat) (d : Outcome × Outcome) (h : c ∈ t)
    (hj : callJudgement t c false d ∈ js) : boundsVerdict js r ≠ .safe := by
  intro hs
  have := (safe_only_when_every_obligation_discharges js r hs).2 _ hj
  rw [a_refuted_declaration_is_withdrawn t c d h] at this
  simp [Outcome.clear] at this

/-- The bounds table is the refutation table of C06.4's two phases: a
declaration is in it exactly when some unit's body refuted it. -/
theorem mem_bounds_table_iff {U : Type} (refutes : U → List Nat) (units : List U) (f : Nat) :
    f ∈ QCore.RefutedContractWithdrawal.table refutes units ↔ ∃ u ∈ units, f ∈ refutes u :=
  QCore.RefutedContractWithdrawal.mem_table_iff refutes units f

/-- Phase two finds the table phase one found: the evidence is computed under
the declarations as loaded, whatever table is applied. -/
def phaseTable {U : Type} (refutesAsLoaded : U → List Nat) (units : List U)
    (_applied : List Nat) : List Nat :=
  QCore.RefutedContractWithdrawal.table refutesAsLoaded units

theorem the_table_is_stable_across_phases {U : Type} (r : U → List Nat)
    (units : List U) (t : List Nat) : phaseTable r units t = phaseTable r units [] := by
  simp [phaseTable]

/-! ## Tips -/

/-- Why a bounds tip entry is not in force. -/
inductive TipReason where
  | contractChanged | tipUnresolved | sourceChanged
  deriving DecidableEq, Repr

/-- The C06.2 tip decision (`QCore.ContractTipMode.decision`'s order: the
text, then resolution, then the sites). -/
def tipDecision (textMatches resolved sitesEqual : Bool) : Option TipReason :=
  if !textMatches then some .contractChanged
  else if !resolved then some .tipUnresolved
  else if !sitesEqual then some .sourceChanged
  else none

/-- The declaration's pin as the bounds standing reads it. -/
def tipPin (textMatches resolved sitesEqual : Bool) : QCore.BoundsPinHold.Pin :=
  match tipDecision textMatches resolved sitesEqual with
  | none => .inForce
  | some .contractChanged => .mismatch .contractChanged
  | some _ => .mismatch .sourceChanged

theorem a_tip_is_in_force_iff (t r s : Bool) :
    tipPin t r s = .inForce ↔ t = true ∧ r = true ∧ s = true := by
  cases t <;> cases r <;> cases s <;> simp [tipPin, tipDecision]

/-- An unresolved tip, a changed text or other sites hold the declaration. -/
theorem an_out_of_force_tip_is_held (rv t r s : Bool) (h : ¬ (t = true ∧ r = true ∧ s = true)) :
    QCore.BoundsPinHold.standing rv (tipPin t r s) = .held := by
  cases rv <;> cases t <;> cases r <;> cases s <;> simp_all [tipPin, tipDecision,
    QCore.BoundsPinHold.standing]

/-- A tip in force is listed as an assumption of the run, and only then. -/
def listedAssumption (t r s : Bool) : Bool := tipPin t r s == .inForce

theorem a_tip_assumption_is_listed_iff_in_force (t r s : Bool) :
    listedAssumption t r s = true ↔ t = true ∧ r = true ∧ s = true := by
  rw [← a_tip_is_in_force_iff]
  simp [listedAssumption]

/-! ## Checkpoints -/

/-- What a composition from checkpoints publishes of the requested bounds
rows: what the checkpoints carry, which is nothing. -/
def composedRows (_requested : List Nat) : List Nat := []

/-- A checkpointed run is admitted only without the bounds selection. -/
def admitsCheckpoints (boundsSelected : Bool) : Bool := !boundsSelected

theorem a_composition_would_omit_the_requested_rows (requested : List Nat)
    (h : requested ≠ []) : composedRows requested ≠ requested := by
  intro e
  exact h (by simpa [composedRows] using e.symm)

theorem a_bounds_run_is_never_composed_from_checkpoints :
    admitsCheckpoints true = false := by
  decide

/-! ## Vectors shared with Rust (`crates/qcore-audit/tests/bounds_transport.rs`) -/

/-- (text matches, resolved, sites equal, reason). -/
def tipVectors : List (Bool × Bool × Bool × Option TipReason) := [
  (true, true, true, none),
  (true, true, false, some .sourceChanged),
  (true, false, true, some .tipUnresolved),
  (false, true, true, some .contractChanged),
  (false, false, false, some .contractChanged)
]

theorem tip_vectors_hold :
    tipVectors.all (fun (t, r, s, x) => tipDecision t r s == x) = true := by decide

theorem tip_vectors_count : tipVectors.length = 5 := by decide

end QCore.BoundsTransport
