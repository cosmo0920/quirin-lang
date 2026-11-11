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

import QCore.ReturnCorrectness

/-! Audited companion to GAPO-RETURN-CORRECTNESS (gateway rules 15x, 15y, 15z). -/
#print axioms QCore.ReturnCorrectness.declined_restores_probe_state
#print axioms QCore.ReturnCorrectness.declined_restores_consultations
#print axioms QCore.ReturnCorrectness.declined_keeps_facts
#print axioms QCore.ReturnCorrectness.declined_without_new_facts_is_identity
#print axioms QCore.ReturnCorrectness.declined_answer
#print axioms QCore.ReturnCorrectness.declined_probe_moves_no_site
#print axioms QCore.ReturnCorrectness.replacement_from_before_has_no_gap
#print axioms QCore.ReturnCorrectness.replacement_after_first_gaps
#print axioms QCore.ReturnCorrectness.no_return_never_continues
#print axioms QCore.ReturnCorrectness.settle_identity
#print axioms QCore.ReturnCorrectness.settle_call_cases
#print axioms QCore.ReturnCorrectness.settle_cases
#print axioms QCore.ReturnCorrectness.settle_keeps_refusal
#print axioms QCore.ReturnCorrectness.settled_ends_or_refuses
#print axioms QCore.ReturnCorrectness.settled_path_end_only_when_established
#print axioms QCore.ReturnCorrectness.uncast_at_least_as_strict
#print axioms QCore.ReturnCorrectness.cast_actual_hid_a_released_place
