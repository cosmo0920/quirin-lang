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

import QCore.OuterContract

/-! Audited companion to OUTER-CONTRACT (gateway rule 31: the runtime projection of outer contracts). -/

#print axioms QCore.OuterContract.Verdict.join_comm
#print axioms QCore.OuterContract.Verdict.join_assoc
#print axioms QCore.OuterContract.static_projection_is_identity
#print axioms QCore.OuterContract.runtime_never_promotes
#print axioms QCore.OuterContract.a_row_without_closed_world_facts_is_unchanged
#print axioms QCore.OuterContract.a_covered_fact_is_pending_with_its_contract
#print axioms QCore.OuterContract.a_pending_contribution_names_its_contract
#print axioms QCore.OuterContract.every_outer_pending_names_a_contract
#print axioms QCore.OuterContract.an_uncovered_fact_is_never_safe_or_pending
#print axioms QCore.OuterContract.runtime_is_the_join
#print axioms QCore.OuterContract.reachability_over_approximates
#print axioms QCore.OuterContract.an_unclassified_pointer_reaches_every_record
#print axioms QCore.OuterContract.a_primitive_pointer_reaches_no_record
#print axioms QCore.OuterContract.projection_vectors_hold
#print axioms QCore.OuterContract.rk_callback_envelope_is_pending_with_both_loaded
