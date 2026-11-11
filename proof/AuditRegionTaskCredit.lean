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

import QCore.RegionTaskCredit

#print axioms QCore.RegionTaskCredit.step_preserves_accounted
#print axioms QCore.RegionTaskCredit.run_preserves_accounted
#print axioms QCore.RegionTaskCredit.discharge_only_moves_the_packs_remainder
#print axioms QCore.RegionTaskCredit.spending_retires_no_duty
#print axioms QCore.RegionTaskCredit.endowment_moves_credit_and_no_duty
#print axioms QCore.RegionTaskCredit.the_parent_sinks_cannot_reach_child_credit
#print axioms QCore.RegionTaskCredit.discharge_requires_every_join
#print axioms QCore.RegionTaskCredit.discharge_requires_every_loan_to_expire
#print axioms QCore.RegionTaskCredit.a_parent_cannot_abandon_credit_held_by_a_child
#print axioms QCore.RegionTaskCredit.a_normal_exit_is_gated_like_a_discharge
#print axioms QCore.RegionTaskCredit.a_normal_exit_refuses_a_remainder
#print axioms QCore.RegionTaskCredit.a_failed_join_closes_nothing
#print axioms QCore.RegionTaskCredit.a_failed_join_leaves_the_pack_undischargeable
#print axioms QCore.RegionTaskCredit.a_failed_creation_restores_the_parent_exactly
#print axioms QCore.RegionTaskCredit.a_creation_records_every_duty_it_opens
#print axioms QCore.RegionTaskCredit.a_join_returns_exactly_the_unspent_child_credit
#print axioms QCore.RegionTaskCredit.sealing_carries_every_duty
#print axioms QCore.RegionTaskCredit.sealing_requires_no_open_pack
#print axioms QCore.RegionTaskCredit.the_admitted_run_is_accepted_and_conserves_its_credit
#print axioms QCore.RegionTaskCredit.discharging_before_the_join_is_refused
#print axioms QCore.RegionTaskCredit.a_failed_creation_leaves_nothing_to_wait_for
#print axioms QCore.RegionTaskCredit.a_normal_exit_with_a_live_join_is_refused
#print axioms QCore.RegionTaskCredit.an_outstanding_loan_refuses_the_discharge
