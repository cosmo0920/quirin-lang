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

import QCore.HypothesisGraph

/-! Detached canonical-byte census only. `encode`, `decode` and `hash` denote
independently held production operations; no cryptographic injectivity, C
extraction, Rust/Lean correspondence or proposition truth is asserted here. -/
namespace QCore.PluginReadingEnvironment

structure Snapshot where
  digest : String
  canonical : String
  deriving DecidableEq

def valid (encode : α → String) (decode : String → Option α)
    (hash : String → String) (expected : String) (snapshot : Snapshot) : Prop :=
  snapshot.digest = expected ∧ hash snapshot.canonical = expected ∧
  ∃ environment, decode snapshot.canonical = some environment ∧
    encode environment = snapshot.canonical

theorem valid_binds_exact_canonical_bytes
    (encode : α → String) (decode : String → Option α)
    (hash : String → String) (expected : String) (snapshot : Snapshot)
    (accepted : valid encode decode hash expected snapshot) :
    hash snapshot.canonical = expected ∧
    ∃ environment, decode snapshot.canonical = some environment ∧
      encode environment = snapshot.canonical := ⟨accepted.2.1, accepted.2.2⟩

theorem stale_digest_refuses
    (encode : α → String) (decode : String → Option α)
    (hash : String → String) (expected : String) (snapshot : Snapshot)
    (stale : hash snapshot.canonical ≠ expected) :
    ¬ valid encode decode hash expected snapshot := by
  intro accepted
  exact stale accepted.2.1

theorem noncanonical_bytes_refuse
    (encode : α → String) (decode : String → Option α)
    (hash : String → String) (expected : String) (snapshot : Snapshot)
    (noncanonical : ∀ environment, decode snapshot.canonical = some environment →
      encode environment ≠ snapshot.canonical) :
    ¬ valid encode decode hash expected snapshot := by
  rintro ⟨_, _, environment, decoded, canonical⟩
  exact noncanonical environment decoded canonical

/-- Referenced hashes belong only to retained observations or bound receipts. -/
def selected (referenced : List String) (snapshots : List Snapshot) : List Snapshot :=
  snapshots.filter fun snapshot => referenced.contains snapshot.digest

theorem publication_requires_retained_reference
    (referenced : List String) (snapshots : List Snapshot) (snapshot : Snapshot) :
    snapshot ∈ selected referenced snapshots ↔
      snapshot ∈ snapshots ∧ snapshot.digest ∈ referenced := by
  simp [selected]

theorem no_snapshot_survives_empty_rollback (snapshots : List Snapshot) :
    selected [] snapshots = [] := by
  simp [selected]

structure Published (α : Type) where
  original : α
  environments : List Snapshot

def attach (original : α) (referenced : List String) (snapshots : List Snapshot) : Published α :=
  ⟨original, selected referenced snapshots⟩

theorem original_payload_is_unchanged (original : α)
    (referenced : List String) (snapshots : List Snapshot) :
    (attach original referenced snapshots).original = original := rfl

theorem no_existing_semantic_consumer_changes (checker : α → β) (original : α)
    (referenced : List String) (snapshots : List Snapshot) :
    checker (attach original referenced snapshots).original = checker original := rfl

/-- Reversal must establish canonical identity explicitly. This states no
relationship between English prose, a source program and a model proposition. -/
theorem exact_reconstruction_has_held_digest
    (hash : String → String) (reverse : String → String)
    (heldOld heldNew : String) (exact : reverse heldNew = heldOld) :
    hash (reverse heldNew) = hash heldOld := congrArg hash exact

end QCore.PluginReadingEnvironment
