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

/-!
Contract-bundle admission model for `qcore-contract::bundle`.

The Rust loader independently compares the manifest hash and owner, checks the
exact asset inventory and validates every asset before exposing a snapshot.
This model represents bytes/digests as natural numbers and filesystem/JSON/path
checks as an explicit Boolean premise. It proves refusal and selection properties
of that decision, not SHA-256 collision resistance, filesystem race freedom,
JSON extraction correctness, predicate truth, or Rust–Lean correspondence.
-/
namespace QCore.ContractBundle

structure Asset where
  path : Nat
  declaredDigest : Nat
  actualDigest : Nat
  bytes : Nat
  deriving DecidableEq, Repr

structure Input where
  manifestDigest : Nat
  expectedDigest : Nat
  owner : Nat
  expectedOwner : Nat
  declaredPaths : List Nat
  actualPaths : List Nat
  assets : List Asset
  wellFormed : Bool
  deriving Repr

def admitted (input : Input) : Bool :=
  input.wellFormed && decide (input.manifestDigest = input.expectedDigest) &&
  decide (input.owner = input.expectedOwner) &&
  decide (input.declaredPaths = input.actualPaths) &&
  decide (input.assets.map Asset.path = input.declaredPaths) &&
  input.assets.all (fun asset => decide (asset.actualDigest = asset.declaredDigest))

def select (input : Input) (path : Nat) : Option Nat :=
  if admitted input then (input.assets.find? (fun asset => asset.path == path)).map Asset.bytes
  else none

theorem admission_binds_manifest (input : Input) (h : admitted input = true) :
    input.manifestDigest = input.expectedDigest := by
  simp only [admitted, Bool.and_eq_true, decide_eq_true_eq] at h
  exact h.1.1.1.1.2

theorem admission_binds_file_set (input : Input) (h : admitted input = true) :
    input.declaredPaths = input.actualPaths := by
  simp only [admitted, Bool.and_eq_true, decide_eq_true_eq] at h
  exact h.1.1.2

theorem manifest_mismatch_refuses (input : Input)
    (h : input.manifestDigest ≠ input.expectedDigest) (path : Nat) :
    select input path = none := by
  have refused : admitted input = false := by
    cases eq : admitted input with
    | false => rfl
    | true => exact False.elim (h (admission_binds_manifest input eq))
  simp [select, refused]

theorem invalid_structure_refuses (input : Input)
    (h : input.wellFormed = false) (path : Nat) : select input path = none := by
  simp [select, admitted, h]

theorem selected_bytes_are_snapshot_bytes (input : Input) (path bytes : Nat)
    (h : select input path = some bytes) :
    ∃ asset ∈ input.assets, asset.bytes = bytes := by
  unfold select at h
  split at h
  next _ =>
    cases found : input.assets.find? (fun asset => asset.path == path) with
    | none => simp [found] at h
    | some asset =>
      simp [found] at h
      exact ⟨asset, List.mem_of_find?_eq_some found, h⟩
  next _ => contradiction

-- A nonempty admitting control prevents refusal-only vacuity.
example : select ⟨7, 7, 3, 3, [1], [1], [⟨1, 8, 8, 42⟩], true⟩ 1 = some 42 := by
  decide

end QCore.ContractBundle
