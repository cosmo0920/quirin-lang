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

namespace QCore.CallResultUse

/-! Source producer-use sets say nothing about an external call's outcome.
The source importer supplies complete paths and alias-substituted producers. -/
structure Use where
  producers : List Nat
  guard : Bool
  deriving DecidableEq

def ignored (uses : List Use) (producer : Nat) : Bool :=
  !(uses.any (fun use => use.producers.contains producer))

def guarded (uses : List Use) (producer : Nat) : Bool :=
  uses.any (fun use => use.guard && use.producers.contains producer)

theorem ignored_result_has_no_recorded_use {uses : List Use} {producer : Nat}
    (accepted : ignored uses producer = true) {use : Use} (member : use ∈ uses) :
    producer ∉ use.producers := by
  simp [ignored] at accepted
  exact accepted use member

theorem guarded_result_is_used {uses : List Use} {producer : Nat}
    (accepted : guarded uses producer = true) : ignored uses producer = false := by
  simp only [guarded, List.any_eq_true, Bool.and_eq_true] at accepted
  obtain ⟨use, member, _, present⟩ := accepted
  have used : uses.any (fun use => use.producers.contains producer) = true :=
    List.any_eq_true.mpr ⟨use, member, present⟩
  unfold ignored
  rw [used]
  rfl

def observeResult (uses : List Use) (producer : Nat) (outcome : Bool) : Option Bool :=
  if ignored uses producer then none else some outcome

theorem ignored_outcomes_are_indistinguishable {uses : List Use} {producer : Nat}
    (accepted : ignored uses producer = true) (first second : Bool) :
    observeResult uses producer first = observeResult uses producer second := by
  simp [observeResult, accepted]

end QCore.CallResultUse
