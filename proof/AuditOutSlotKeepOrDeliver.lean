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

import QCore.OutSlotKeepOrDeliver

/-! Audited companion to OUT-SLOT-CONDITIONAL-DELIVERY (gateway rule 85). -/

#print axioms QCore.OutSlotKeepOrDeliver.keep_or_deliver_vectors_hold
#print axioms QCore.OutSlotKeepOrDeliver.a_read_mode_keeps_on_every_owned_exit
#print axioms QCore.OutSlotKeepOrDeliver.a_read_mode_delivers_on_every_null_exit
#print axioms QCore.OutSlotKeepOrDeliver.an_overwrite_of_the_kept_object_reads_no_mode
#print axioms QCore.OutSlotKeepOrDeliver.a_null_run_refusal_is_kept
#print axioms QCore.OutSlotKeepOrDeliver.a_fresh_object_only_for_a_known_null_slot
#print axioms QCore.OutSlotKeepOrDeliver.an_unplaced_slot_is_unknown
#print axioms QCore.OutSlotKeepOrDeliver.the_reading_is_the_effect
#print axioms QCore.OutSlotKeepOrDeliver.the_kept_arm_mints_no_second_custody
#print axioms QCore.OutSlotKeepOrDeliver.the_delivered_arm_adds_one_obligation
#print axioms QCore.OutSlotKeepOrDeliver.a_released_delivery_is_discharged
#print axioms QCore.OutSlotKeepOrDeliver.an_ignored_delivery_leaks_in_the_caller
#print axioms QCore.OutSlotKeepOrDeliver.a_kept_object_stays_the_callers
#print axioms QCore.OutSlotKeepOrDeliver.an_out_reading_mints_a_second_custody
#print axioms QCore.OutSlotKeepOrDeliver.a_borrowed_reading_hides_a_delivery
