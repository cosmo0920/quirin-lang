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

import QCore.CleanupOutcome

namespace QCore.UnmapWrapper

/-! This state is after any sync operation. Sync's full frame is a separate
premise. Native unmap's outcome controls wrapper bookkeeping and its counter. -/
structure State where
  upCounter : Nat
  dataSize : Nat
  allocSize : Nat
  deriving DecidableEq

def finishNativeUnmap (state : State) (outcome : CleanupOutcome.Outcome) : Option State :=
  match outcome with
  | .failure => some state
  | .success => match state.upCounter with
    | 0 => none
    | n + 1 => some { upCounter := n, dataSize := 0, allocSize := 0 }

theorem failed_native_preserves_post_sync_state (state : State) :
    finishNativeUnmap state .failure = some state := rfl

theorem success_at_zero_rejected (data alloc : Nat) :
    finishNativeUnmap ⟨0, data, alloc⟩ .success = none := rfl

theorem success_decrements_once (n data alloc : Nat) :
    finishNativeUnmap ⟨n + 1, data, alloc⟩ .success =
      some ⟨n, 0, 0⟩ := rfl

theorem accepted_success_has_positive_counter {before after : State}
    (h : finishNativeUnmap before .success = some after) : before.upCounter > 0 := by
  cases before with
  | mk count data alloc =>
    cases count with
    | zero => simp [finishNativeUnmap] at h
    | succ n => exact Nat.zero_lt_succ n

theorem accepted_success_balances_and_clears {before after : State}
    (h : finishNativeUnmap before .success = some after) :
    before.upCounter = after.upCounter + 1 ∧ after.dataSize = 0 ∧ after.allocSize = 0 := by
  cases before with
  | mk count data alloc =>
    cases count with
    | zero => simp [finishNativeUnmap] at h
    | succ n =>
      simp only [finishNativeUnmap, Option.some.injEq] at h
      subst after
      simp

end QCore.UnmapWrapper
