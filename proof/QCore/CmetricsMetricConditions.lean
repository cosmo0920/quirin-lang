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

/-! Store-trace projection for the low-level CMetrics setter conditions.
Fields, conversions, CAS success and complete call-trace correspondence are
explicit inputs. This model proves no C backend, progress or joint snapshot. -/
namespace QCore.CmetricsMetricConditions
structure Store where
  field : Nat
  index : Int
  value : Int
  cas : Bool := false
  deriving DecidableEq, Repr

def casPair (field timeField : Nat) (index value timestamp : Int) : List Store :=
  [⟨field, index, value, true⟩, ⟨timeField, 0, timestamp, false⟩]

theorem returned_pair_orders_success_before_timestamp
    (field timeField : Nat) (index value timestamp : Int) :
    ∃ first second, casPair field timeField index value timestamp = [first, second] ∧
      first.field = field ∧ first.index = index ∧ first.value = value ∧
      first.cas = true ∧ second.field = timeField ∧ second.value = timestamp := by
  exact ⟨_, _, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

/-- Defined scalar conversion is supplied as an integer in the shared signed/
unsigned range; no floating conversion correctness is inferred here. -/
def scalarStores (valueField signedField unsignedField timeField tagField : Nat)
    (bits converted timestamp tag : Int) : List Store :=
  [⟨valueField, 0, bits, false⟩, ⟨signedField, 0, converted, false⟩,
   ⟨unsignedField, 0, converted, false⟩, ⟨timeField, 0, timestamp, false⟩,
   ⟨tagField, 0, tag, false⟩]

theorem scalar_mirrors_use_the_same_defined_integer
    (valueField signedField unsignedField timeField tagField : Nat)
    (bits converted timestamp tag : Int) :
    ∃ valueStore signedStore unsignedStore timeStore tagStore,
      scalarStores valueField signedField unsignedField timeField tagField
        bits converted timestamp tag = [valueStore, signedStore, unsignedStore, timeStore, tagStore] ∧
      valueStore.value = bits ∧ signedStore.value = converted ∧ unsignedStore.value = converted ∧
      timeStore.value = timestamp ∧ tagStore.value = tag := by
  exact ⟨_, _, _, _, _, rfl, rfl, rfl, rfl, rfl, rfl⟩

def normalizedSum (flagField sumField : Nat) (enabled bits : Int) : List Store :=
  if enabled = 0 then [⟨flagField, 0, 0, false⟩, ⟨sumField, 0, 0, false⟩]
  else [⟨flagField, 0, 1, false⟩, ⟨sumField, 0, bits, false⟩]

theorem disabled_sum_has_zero_flag_and_bits (flagField sumField : Nat) (bits : Int) :
    normalizedSum flagField sumField 0 bits =
      [⟨flagField, 0, 0, false⟩, ⟨sumField, 0, 0, false⟩] := by
  simp [normalizedSum]

theorem enabled_sum_normalizes_flag (flagField sumField : Nat) (enabled bits : Int)
    (nonzero : enabled ≠ 0) :
    normalizedSum flagField sumField enabled bits =
      [⟨flagField, 0, 1, false⟩, ⟨sumField, 0, bits, false⟩] := by
  simp [normalizedSum, nonzero]

def quantileActive (metricPresent arrayPresent : Bool) (index count : Int) : Bool :=
  metricPresent && arrayPresent && decide (0 ≤ index ∧ index < count)

def quantileStores (metricPresent arrayPresent : Bool) (index count : Int)
    (field timeField : Nat) (bits timestamp : Int) : List Store :=
  if quantileActive metricPresent arrayPresent index count then
    casPair field timeField index bits timestamp else []

theorem null_metric_has_no_stores (arrayPresent : Bool) (index count : Int)
    (field timeField : Nat) (bits timestamp : Int) :
    quantileStores false arrayPresent index count field timeField bits timestamp = [] := by
  simp [quantileStores, quantileActive]

theorem null_array_has_no_stores (metricPresent : Bool) (index count : Int)
    (field timeField : Nat) (bits timestamp : Int) :
    quantileStores metricPresent false index count field timeField bits timestamp = [] := by
  simp [quantileStores, quantileActive]

theorem invalid_index_has_no_stores (metricPresent arrayPresent : Bool) (index count : Int)
    (field timeField : Nat) (bits timestamp : Int) (invalid : index < 0 ∨ count ≤ index) :
    quantileStores metricPresent arrayPresent index count field timeField bits timestamp = [] := by
  have outside : ¬ (0 ≤ index ∧ index < count) := by
    intro inside
    rcases invalid with negative | large
    · exact (Int.not_le.mpr negative) inside.1
    · exact (Int.not_lt.mpr large) inside.2
  simp [quantileStores, quantileActive, outside]

theorem admitted_quantile_has_ordered_pair (index count : Int)
    (field timeField : Nat) (bits timestamp : Int) (valid : 0 ≤ index ∧ index < count) :
    quantileStores true true index count field timeField bits timestamp =
      casPair field timeField index bits timestamp := by
  simp [quantileStores, quantileActive, valid]

structure State where
  cells : Nat → Int → Int
  ownership : Nat → Nat → Prop

def applyStore (state : State) (store : Store) : State :=
  { state with cells := fun field index =>
      if field = store.field ∧ index = store.index then store.value else state.cells field index }

def execute (state : State) : List Store → State
  | [] => state
  | store :: rest => execute (applyStore state store) rest

theorem stores_preserve_ownership (state : State) (stores : List Store) :
    (execute state stores).ownership = state.ownership := by
  induction stores generalizing state with
  | nil => rfl
  | cons store rest inductionStep =>
    exact inductionStep (applyStore state store)

end QCore.CmetricsMetricConditions
