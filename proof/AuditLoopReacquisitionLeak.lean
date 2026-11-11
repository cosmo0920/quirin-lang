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

import QCore.LoopReacquisitionLeak

/-! Audited companion to LOOP-REACQUISITION-LEAK-ACCUSATION (a leak joined across a refused re-acquisition, gateway rule 42). -/

#print axioms QCore.LoopReacquisitionLeak.a_current_report_is_published
#print axioms QCore.LoopReacquisitionLeak.an_unrefused_site_is_published
#print axioms QCore.LoopReacquisitionLeak.an_unreported_refusal_withdraws_nothing
#print axioms QCore.LoopReacquisitionLeak.the_rule_only_withdraws
#print axioms QCore.LoopReacquisitionLeak.withdrawn_iff_every_reporter_is_withdrawn
#print axioms QCore.LoopReacquisitionLeak.more_reporters_never_withdraw
#print axioms QCore.LoopReacquisitionLeak.a_withdrawal_names_a_reported_refusal
#print axioms QCore.LoopReacquisitionLeak.an_earlier_generation_of_a_refused_site_is_withdrawn
#print axioms QCore.LoopReacquisitionLeak.a_withdrawal_is_never_safe
#print axioms QCore.LoopReacquisitionLeak.a_withdrawn_key_has_its_refusal
#print axioms QCore.LoopReacquisitionLeak.a_fully_withdrawn_function_is_unknown
#print axioms QCore.LoopReacquisitionLeak.the_rule_moves_no_verdict_to_safe
#print axioms QCore.LoopReacquisitionLeak.withdraw_vectors_hold
#print axioms QCore.LoopReacquisitionLeak.the_vectors_name_site_seven
#print axioms QCore.LoopReacquisitionLeak.shape_vectors_hold
