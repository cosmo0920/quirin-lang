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

import QCore.OwnedOutFormalDelivery

/-! Audited companion to OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY (gateway rule 74l). -/

#print axioms QCore.OwnedOutFormalDelivery.out_mode_vectors_hold
#print axioms QCore.OwnedOutFormalDelivery.refused_anywhere
#print axioms QCore.OwnedOutFormalDelivery.a_partial_path_store_reads_no_mode
#print axioms QCore.OwnedOutFormalDelivery.a_dead_delivery_reads_no_mode
#print axioms QCore.OwnedOutFormalDelivery.a_possibly_null_delivery_reads_no_mode
#print axioms QCore.OwnedOutFormalDelivery.an_ambiguous_slot_reads_no_mode
#print axioms QCore.OwnedOutFormalDelivery.a_read_mode_delivers_on_every_exit
#print axioms QCore.OwnedOutFormalDelivery.only_null_reads_no_mode
#print axioms QCore.OwnedOutFormalDelivery.a_validated_out_summary_has_no_partial_path
#print axioms QCore.OwnedOutFormalDelivery.a_validated_out_summary_has_no_released_delivery
#print axioms QCore.OwnedOutFormalDelivery.an_ignored_delivery_leaks_in_the_caller
#print axioms QCore.OwnedOutFormalDelivery.a_released_delivery_is_discharged
#print axioms QCore.OwnedOutFormalDelivery.the_renaming_keeps_every_leaf
#print axioms QCore.OwnedOutFormalDelivery.an_unsanctioned_slot_is_refused
#print axioms QCore.OwnedOutFormalDelivery.a_slot_store_is_never_read_as_a_discard
#print axioms QCore.OwnedOutFormalDelivery.only_the_unsanctioned_slot_changed
