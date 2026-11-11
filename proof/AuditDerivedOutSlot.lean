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

import QCore.DerivedOutSlot

/-! Audited companion to SLOT-MOVE-AND-INTERIOR-BORROW: derived out slots (gateway rule 12z). -/

#print axioms QCore.DerivedOutSlot.a_tracked_slot_is_never_overwritten
#print axioms QCore.DerivedOutSlot.a_bind_needs_an_untracked_slot
#print axioms QCore.DerivedOutSlot.run_append
#print axioms QCore.DerivedOutSlot.step_keeps_refused
#print axioms QCore.DerivedOutSlot.run_keeps_refused
#print axioms QCore.DerivedOutSlot.the_slot_value_is_never_released
#print axioms QCore.DerivedOutSlot.the_base_release_ends_the_slot_value
#print axioms QCore.DerivedOutSlot.a_safe_path_never_uses_a_value_its_base_outlived
#print axioms QCore.DerivedOutSlot.out_vectors_hold
