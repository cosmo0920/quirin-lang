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

import QCore.BacklogLedger
#print axioms QCore.BacklogLedger.guarded_sub_does_not_wrap
#print axioms QCore.BacklogLedger.guarded_add_does_not_wrap
#print axioms QCore.BacklogLedger.bare_sub_wraps
#print axioms QCore.BacklogLedger.bare_add_wraps
#print axioms QCore.BacklogLedger.satSub_exact
#print axioms QCore.BacklogLedger.satSub_le
#print axioms QCore.BacklogLedger.satAdd_le_max
#print axioms QCore.BacklogLedger.satAdd_exact
#print axioms QCore.BacklogLedger.decide_vectors
#print axioms QCore.BacklogLedger.unguarded_refuses
#print axioms QCore.BacklogLedger.dirty_update_refuses
#print axioms QCore.BacklogLedger.discharge_is_exact
#print axioms QCore.BacklogLedger.append_preserves
#print axioms QCore.BacklogLedger.append_overflow_clamp_unreachable
#print axioms QCore.BacklogLedger.removeFirst_total
#print axioms QCore.BacklogLedger.remove_underflow_clamp_unreachable
#print axioms QCore.BacklogLedger.remove_preserves
#print axioms QCore.BacklogLedger.removeFirst_none_of_absent
#print axioms QCore.BacklogLedger.removeFirst_removes
#print axioms QCore.BacklogLedger.repeated_removal_is_a_no_op
#print axioms QCore.BacklogLedger.total_map_residency
#print axioms QCore.BacklogLedger.residency_preserves
#print axioms QCore.BacklogLedger.resident_le_total
#print axioms QCore.BacklogLedger.rollback_restores
#print axioms QCore.BacklogLedger.doubled_discharge_breaks_or_clamps
#print axioms QCore.BacklogLedger.doubled_discharge_breaks
#print axioms QCore.BacklogLedger.missing_discharge_breaks
#print axioms QCore.BacklogLedger.partial_fanout_preserves
#print axioms QCore.BacklogLedger.wouldExceed_iff
#print axioms QCore.BacklogLedger.batch_overshoot_bound
#print axioms QCore.BacklogLedger.batch_overshoot_reachable
#print axioms QCore.BacklogLedger.negative_magnitude
#print axioms QCore.BacklogLedger.negative_magnitude_in_range
