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

namespace QCore.CleanupOutcome

/-! Conditional bookkeeping transitions. The OS result is supplied, and the
record's fields are distinct from actual OS custody. In particular a failure
leaves custody unresolved; preserving an fd field is not proof the fd is open. -/
inductive Entry where
  | null | absent | present
  deriving DecidableEq
inductive Outcome where
  | failure | success
  deriving DecidableEq
structure Result where
  returned : Int
  calls : Nat
  writes : Nat
  fieldsPreserved : Bool
  fieldsReset : Bool
  deriving DecidableEq

def replay (entry : Entry) (outcome : Outcome) (resetFields : Nat) : Result :=
  match entry, outcome with
  | .null, _ => ⟨-1, 0, 0, true, false⟩
  | .absent, _ => ⟨0, 0, 0, true, false⟩
  | .present, .failure => ⟨-1, 1, 0, true, false⟩
  | .present, .success => ⟨0, 1, resetFields, false, true⟩

theorem null_has_no_access (outcome : Outcome) (fields : Nat) :
    (replay .null outcome fields).calls = 0 ∧
    (replay .null outcome fields).writes = 0 := by cases outcome <;> simp [replay]

theorem absent_is_success_without_cleanup (outcome : Outcome) (fields : Nat) :
    (replay .absent outcome fields).returned = 0 ∧
    (replay .absent outcome fields).calls = 0 := by cases outcome <;> simp [replay]

theorem failure_preserves_bookkeeping (fields : Nat) :
    (replay .present .failure fields).returned = -1 ∧
    (replay .present .failure fields).writes = 0 ∧
    (replay .present .failure fields).fieldsPreserved = true := by simp [replay]

theorem success_clears_bookkeeping (fields : Nat) :
    (replay .present .success fields).calls = 1 ∧
    (replay .present .success fields).writes = fields ∧
    (replay .present .success fields).fieldsReset = true := by simp [replay]

inductive Custody where
  | absent | unresolved
  deriving DecidableEq

def custodyAfter (outcome : Outcome) : Custody :=
  match outcome with
  | .success => .absent
  | .failure => .unresolved

theorem failure_does_not_establish_discharge : custodyAfter .failure ≠ .absent := by decide

theorem discharge_requires_success {outcome : Outcome}
    (h : custodyAfter outcome = .absent) : outcome = .success := by
  cases outcome <;> simp_all [custodyAfter]

end QCore.CleanupOutcome
