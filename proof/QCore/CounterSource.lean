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

namespace QCore.CounterSource

/-! Exact arithmetic for the bounded unsigned unit-update source profile.
The target word maximum and live/exclusive counter binding are premises. -/
def increment (value maximum : Nat) : Option Nat :=
  if value < maximum then some (value + 1) else none

def decrement (value maximum : Nat) : Option Nat :=
  if 0 < value ∧ value ≤ maximum then some (value - 1) else none

theorem increment_at_maximum_rejected (maximum : Nat) :
    increment maximum maximum = none := by simp [increment]

theorem decrement_at_zero_rejected (maximum : Nat) :
    decrement 0 maximum = none := by simp [decrement]

theorem accepted_increment_exact {value maximum after : Nat}
    (accepted : increment value maximum = some after) :
    after = value + 1 ∧ after ≤ maximum := by
  unfold increment at accepted
  split at accepted
  next bound =>
    cases accepted
    omega
  next => contradiction

theorem accepted_decrement_exact {value maximum after : Nat}
    (accepted : decrement value maximum = some after) :
    after + 1 = value ∧ after ≤ maximum := by
  unfold decrement at accepted
  split at accepted
  next bound =>
    cases accepted
    omega
  next => contradiction

theorem increment_then_decrement_conserves_counter {value maximum after : Nat}
    (accepted : increment value maximum = some after) :
    decrement after maximum = some value := by
  have exact := accepted_increment_exact accepted
  simp only [decrement]
  have valid : 0 < after ∧ after ≤ maximum := by omega
  rw [ite_eq_left valid]
  congr 1
  omega

theorem decrement_then_increment_conserves_counter {value maximum after : Nat}
    (accepted : decrement value maximum = some after) :
    increment after maximum = some value := by
  have exact := accepted_decrement_exact accepted
  unfold decrement at accepted
  split at accepted
  next bounds =>
    simp only [increment]
    have valid : after < maximum := by omega
    rw [ite_eq_left valid]
    congr 1
    omega
  next => contradiction

end QCore.CounterSource
