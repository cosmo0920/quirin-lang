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

import QCore.DisjointFootprint

/-! A model of the bounded source-to-concurrency route for record-field writes.

The source route admits one object, one lexical thread scope, two or more
spawns before any join, one direct field-write capture per worker, and worker
bodies exactly `write PARAM; return;`. This module models the two-worker kernel
of that grammar. It derives the same `Footprint` shape the Rust lowering hands
to `DisjointFootprintsV1`; it does not encode bit-fields as byte ranges.

Two premises remain explicit. The declared layout numbers are the target ABI's
real layout, and `RustCorrespondence` says the compiled Rust implementation
implements this Lean decision. Neither is proved here. -/
namespace QCore.BitfieldSourceConcurrency

open QCore.DisjointFootprint

/-- A source layout field, before projection to the footprint checker. -/
inductive SourceStorage where
  | field (byteOffset byteSize : Nat)
  | bits (unit bitOffset bitWidth byteOffset byteSize : Nat)
  deriving DecidableEq, Repr

def SourceStorage.bytes : SourceStorage → Interval
  | .field offset size => ⟨offset, offset + size⟩
  | .bits _ _ _ offset size => ⟨offset, offset + size⟩

def SourceStorage.unit : SourceStorage → Option Nat
  | .field _ _ => none
  | .bits unit _ _ _ _ => some unit

/-- The source derivation: bit offsets and widths are retained by the source
layout but only unit identity reaches the disjointness rule. -/
def derive (storage : SourceStorage) : Footprint :=
  .memory 1 1 storage.bytes storage.unit

/-- One real worker write and its live interval in statement order. -/
structure WorkerWrite where
  spawn : Nat
  join : Nat
  storage : SourceStorage
  deriving DecidableEq, Repr

def simultaneous (left right : WorkerWrite) : Bool :=
  decide (left.spawn < right.join ∧ right.spawn < left.join)

/-- The route asks the footprint checker only for writes whose spawn-to-join
windows overlap. A sequentialized pair is outside this bounded route. -/
def sourceDecision (left right : WorkerWrite) : Option Decision :=
  if simultaneous left right then
    some (decideDisjoint (derive left.storage) (derive right.storage))
  else none

/-- Adjacent bit-fields in one unit, written by two simultaneously live
workers, refuse as one shared memory location. Their bit and byte ranges are
universally quantified and cannot buy admission. -/
theorem simultaneous_writes_to_one_unit_are_refused
    (unit leftBit leftWidth rightBit rightWidth leftByte leftSize rightByte rightSize : Nat) :
    sourceDecision
        ⟨0, 3, .bits unit leftBit leftWidth leftByte leftSize⟩
        ⟨1, 2, .bits unit rightBit rightWidth rightByte rightSize⟩
      = some (.refused .sharedBitfieldUnit) := by
  simp [sourceDecision, simultaneous, derive, SourceStorage.bytes, SourceStorage.unit,
    decideDisjoint, decideMemory, sharedUnit]

/-- Ordinary addressable fields whose byte ranges are disjoint are admitted
under the same two-worker source shape. -/
theorem ordinary_disjoint_fields_are_admitted (leftSize gap rightSize : Nat) :
    sourceDecision
        ⟨0, 3, .field 0 leftSize⟩
        ⟨1, 2, .field (leftSize + gap) rightSize⟩
      = some .disjoint := by
  simp [sourceDecision, simultaneous, derive, SourceStorage.bytes, SourceStorage.unit,
    decideDisjoint, decideMemory, sharedUnit, Interval.disjoint]

/-- Distinct bit-field units are decided by bytes, providing the attribution
control for the shared-unit refusal. -/
theorem distinct_units_with_disjoint_bytes_are_admitted
    (leftUnit rightUnit leftSize gap rightSize : Nat)
    (different : leftUnit ≠ rightUnit) :
    sourceDecision
        ⟨0, 3, .bits leftUnit 0 1 0 leftSize⟩
        ⟨1, 2, .bits rightUnit 0 1 (leftSize + gap) rightSize⟩
      = some .disjoint := by
  simp [sourceDecision, simultaneous, derive, SourceStorage.bytes, SourceStorage.unit,
    decideDisjoint, decideMemory, sharedUnit, different, Interval.disjoint]

/-- A spawn after the first join is not concurrent evidence and the bounded
route answers nothing. This is the adversarial sequentialization control. -/
theorem sequentialized_writes_are_not_a_concurrent_question (left right : SourceStorage) :
    sourceDecision ⟨0, 1, left⟩ ⟨2, 3, right⟩ = none := by
  simp [sourceDecision, simultaneous]

/-- The explicit, unproved Rust-to-Lean correspondence premise. Review must
establish this equality for the retained Rust lowering and certificate; Lean
does not extract or execute that code. -/
structure RustCorrespondence (left right : WorkerWrite) (rustDecision : Option Decision) : Prop where
  sameDecision : rustDecision = sourceDecision left right

/-- The Rust route's successful-window premise. The source uses propagating
spawn/join operations, but this property model does not prove their failure
edges or the scoped-task lifetime ledger. -/
structure SuccessfulTaskWindow (spawnResults joinResults : List Bool) : Prop where
  allSpawnsSucceed : spawnResults.all id = true
  allJoinsSucceed : joinResults.all id = true

/-- What the model proves about Rust is conditional on the correspondence
premise, rather than silently treating a transcription as extraction. -/
theorem rust_refuses_adjacent_writes_under_correspondence
    (unit leftBit leftWidth rightBit rightWidth leftByte leftSize rightByte rightSize : Nat)
    (rustDecision : Option Decision)
    (spawnResults joinResults : List Bool)
    (_successful : SuccessfulTaskWindow spawnResults joinResults)
    (corresponds : RustCorrespondence
      ⟨0, 3, .bits unit leftBit leftWidth leftByte leftSize⟩
      ⟨1, 2, .bits unit rightBit rightWidth rightByte rightSize⟩
      rustDecision) :
    rustDecision = some (.refused .sharedBitfieldUnit) := by
  rw [corresponds.sameDecision]
  exact simultaneous_writes_to_one_unit_are_refused unit leftBit leftWidth rightBit rightWidth
    leftByte leftSize rightByte rightSize

end QCore.BitfieldSourceConcurrency
