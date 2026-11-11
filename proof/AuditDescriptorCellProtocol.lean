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

import QCore.DescriptorCellProtocol

/-! Audited companion to MK-EVENT-CELL-CROSS-FUNCTION (gateway rule 38c, contract specification C04.6.3, grammar G05i; user decision 2026-11-02, "B as recommended"): a record type's declared cell protocol; a forwarded cell equals the in-body reading (adopting the names a live guard test introduced gives the in-body creation's table, so the member-equality guard decides equal), an unresolved forward never decides (an untested record decides no guard, a test never overrides a fact, a conflicting guard never decides dead), a forwarded cell ends only by its own close or its holder's release (a dead protocol names no live cell, a store never overwrites a live name, a release stales the identity's pending names, a live name holds the record), a creation establishes only the `!= -1` sentinel and a live-looking sentinel store breaks the protocol; the Rust protocol vectors hold. -/

#print axioms QCore.DescriptorCellProtocol.a_forwarded_cell_equals_the_in_body_reading
#print axioms QCore.DescriptorCellProtocol.the_adopted_names_leave_the_entry
#print axioms QCore.DescriptorCellProtocol.the_forwarded_guard_decides_equal
#print axioms QCore.DescriptorCellProtocol.an_untested_record_decides_no_guard
#print axioms QCore.DescriptorCellProtocol.a_test_never_overrides_a_fact
#print axioms QCore.DescriptorCellProtocol.a_conflicting_guard_never_decides_dead
#print axioms QCore.DescriptorCellProtocol.a_dead_protocol_names_no_live_cell
#print axioms QCore.DescriptorCellProtocol.a_store_never_overwrites_a_live_cell
#print axioms QCore.DescriptorCellProtocol.a_release_stales_the_pending_names
#print axioms QCore.DescriptorCellProtocol.a_live_cell_holds_the_record
#print axioms QCore.DescriptorCellProtocol.a_creation_establishes_only_the_sentinel
#print axioms QCore.DescriptorCellProtocol.a_live_looking_sentinel_breaks
#print axioms QCore.DescriptorCellProtocol.protocol_vectors_hold
