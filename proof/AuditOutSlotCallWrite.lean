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

import QCore.OutSlotCallWrite

/-! Audited companion to OUT-SLOT-CALL-WRITE-ALIASING (gateway rule 11r(8)). -/

#print axioms QCore.OutSlotCallWrite.no_reaching_actual_designates
#print axioms QCore.OutSlotCallWrite.kept_when_no_actual_reaches
#print axioms QCore.OutSlotCallWrite.a_reaching_call_poisons_a_bound_slot
#print axioms QCore.OutSlotCallWrite.bound_kept_only_when_excluded
#print axioms QCore.OutSlotCallWrite.unwritten_stays_sound
#print axioms QCore.OutSlotCallWrite.transfer_sound
#print axioms QCore.OutSlotCallWrite.the_old_reading_is_unsound
#print axioms QCore.OutSlotCallWrite.the_memcpy_shape_poisons
#print axioms QCore.OutSlotCallWrite.distinct_locals_keep
#print axioms QCore.OutSlotCallWrite.a_const_source_keeps
