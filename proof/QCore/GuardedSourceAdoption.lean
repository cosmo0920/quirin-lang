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

import QCore.GuardedDecomposition

/-! Explicit guarded source metadata admission and sealed whole-source recovery.
Byte/role extraction and Rust correspondence remain maintained assumptions;
this model adds no C truth, lifecycle execution or verdict consumer. -/
namespace QCore.GuardedSourceAdoption

structure Snapshot where
  source : List Nat
  graph : List Nat
  interpreted : List Nat
  abi : List Nat
  premises : List Nat
  deriving DecidableEq

structure Plan where
  version : Nat
  before : Snapshot
  after : Snapshot
  leftBytes : List Nat
  rightBytes : List Nat
  addition : List Nat
  roles : Nat → Option GuardedDecomposition.Role
  document : GuardedDecomposition.Lowered

/-- Whole-source deterministic insertion and all independently replayed
semantic outputs must agree; a familiar clause head is insufficient. -/
def admissible (p : Plan) : Bool :=
  decide (p.version = 2 ∧
    p.before.source = p.leftBytes ++ p.rightBytes ∧
    p.after.source = p.leftBytes ++ p.addition ++ p.rightBytes ∧
    p.before.graph = p.after.graph ∧
    p.before.interpreted = p.after.interpreted ∧
    p.before.abi = p.after.abi ∧ p.before.premises = p.after.premises) &&
  GuardedDecomposition.admissible p.roles p.document

/-- Models the private-constructor receipt: successful construction holds
both complete snapshots and does not accept a replacement presented source. -/
def recover (p : Plan) (presented : Snapshot) : Option Snapshot :=
  if admissible p && decide (presented = p.after) then some p.before else none

theorem recovered_only_for_exact_tip (p : Plan) (presented original : Snapshot)
    (accepted : recover p presented = some original) : presented = p.after := by
  unfold recover at accepted
  split at accepted
  next h => exact of_decide_eq_true (Bool.and_eq_true_iff.mp h).2
  next => contradiction

theorem changed_tip_refuses (p : Plan) (presented : Snapshot)
    (changed : presented ≠ p.after) : recover p presented = none := by
  simp [recover, changed]

theorem unsupported_version_refuses (p : Plan) (version : p.version ≠ 2) :
    admissible p = false := by
  simp [admissible, version]

theorem changed_interpreted_output_refuses (p : Plan)
    (changed : p.before.interpreted ≠ p.after.interpreted) :
    admissible p = false := by
  simp [admissible, changed]

theorem changed_original_graph_refuses (p : Plan)
    (changed : p.before.graph ≠ p.after.graph) : admissible p = false := by
  simp [admissible, changed]

theorem changed_legacy_premises_refuse (p : Plan)
    (changed : p.before.premises ≠ p.after.premises) : admissible p = false := by
  simp [admissible, changed]

theorem accepted_metadata_was_fully_validated (p : Plan)
    (accepted : admissible p = true) :
    GuardedDecomposition.admissible p.roles p.document = true :=
  (Bool.and_eq_true_iff.mp accepted).2

theorem recovery_retains_complete_original (p : Plan) (presented original : Snapshot)
    (accepted : recover p presented = some original) : original = p.before := by
  unfold recover at accepted
  split at accepted
  next => exact (Option.some.inj accepted).symm
  next => contradiction

theorem absent_terminal_cannot_be_adopted (p : Plan) (branch : Nat)
    (present : branch ∈ GuardedDecomposition.branchDomain p.document)
    (missing : branch ∉ p.document.terminals.map GuardedDecomposition.Terminal.branch) :
    admissible p = false := by
  have incomplete := GuardedDecomposition.missing_terminal_refuses p.document branch present missing
  simp [admissible, GuardedDecomposition.admissible, incomplete]

end QCore.GuardedSourceAdoption
