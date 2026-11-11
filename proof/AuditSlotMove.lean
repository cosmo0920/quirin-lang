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

import QCore.SlotMove

/-! Audited companion to SLOT-MOVE-AND-INTERIOR-BORROW: class 2 slot moves (gateway rule 12x). -/

#print axioms QCore.SlotMove.a_move_needs_one_identity_held_nowhere_else
#print axioms QCore.SlotMove.an_identity_held_elsewhere_is_refused
#print axioms QCore.SlotMove.run_append
#print axioms QCore.SlotMove.step_keeps_refused
#print axioms QCore.SlotMove.run_keeps_refused
#print axioms QCore.SlotMove.lookup_filter_keeps_slot
#print axioms QCore.SlotMove.step_move
#print axioms QCore.SlotMove.the_slot_keeps_its_identity_and_its_obligation
#print axioms QCore.SlotMove.every_old_alias_is_invalid_after_a_move
#print axioms QCore.SlotMove.a_use_of_an_old_alias_is_refused
#print axioms QCore.SlotMove.the_failure_arm_keeps_the_original
#print axioms QCore.SlotMove.a_safe_path_never_uses_an_invalid_name
#print axioms QCore.SlotMove.slot_vectors_hold
#print axioms QCore.SlotMove.slot_trace_vectors_hold
