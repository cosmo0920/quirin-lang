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

import QCore.RetentionPath

/-! An exact declared removal wrapper forwards the caller's object and holder.
Source review and source pins establish the wrapper reading; these theorems
are about the existing retention model, not extraction from C. -/
namespace QCore.EventRemovalWrapper
open QCore.RetentionPath

def disable (object : HeldObject) (holder : HeldBy) : Event :=
  .remove (some object, some holder)

theorem disable_ends_a_fresh_exact_registration (object : HeldObject) (holder : HeldBy) :
    step [⟨object, holder, false⟩] (disable object holder) = some [] := by
  simp [disable, step, exact]

theorem disable_keeps_an_already_poisoned_registration (object : HeldObject) (holder : HeldBy) :
    step [⟨object, holder, true⟩] (disable object holder) = some [⟨object, holder, true⟩] := by
  cases object <;> simp [disable, step, exact, poisonIf, mayName, HeldObject.identity]

theorem disable_does_not_destroy_its_holder (object : HeldObject) (holder : HeldBy) :
    ends (disable object holder) holder = false := by
  simp [disable, ends]

end QCore.EventRemovalWrapper
