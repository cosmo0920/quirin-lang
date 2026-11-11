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

import QCore.UncontractedAccessAttribution

/-! Audited companion to CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (gateway rule 12k). -/

#print axioms QCore.UncontractedAccessAttribution.only_the_gate_refusal_is_read
#print axioms QCore.UncontractedAccessAttribution.the_writes_are_kept
#print axioms QCore.UncontractedAccessAttribution.the_result_is_the_gate_refusal_or_the_callees
#print axioms QCore.UncontractedAccessAttribution.a_refusal_stays_a_refusal
#print axioms QCore.UncontractedAccessAttribution.the_verdict_of_every_node_is_unchanged
#print axioms QCore.UncontractedAccessAttribution.the_status_of_a_function_is_unchanged
#print axioms QCore.UncontractedAccessAttribution.a_contracted_callee_keeps_the_gate_refusal
#print axioms QCore.UncontractedAccessAttribution.a_held_callee_keeps_the_gate_refusal
#print axioms QCore.UncontractedAccessAttribution.a_descriptor_read_keeps_the_gate_refusal
#print axioms QCore.UncontractedAccessAttribution.an_unrecognised_actual_keeps_the_gate_refusal
#print axioms QCore.UncontractedAccessAttribution.an_admitting_reading_keeps_the_gate_refusal
#print axioms QCore.UncontractedAccessAttribution.another_refusal_keeps_the_gate_refusal
#print axioms QCore.UncontractedAccessAttribution.the_attributed_callee_is_the_statements
#print axioms QCore.UncontractedAccessAttribution.an_attributed_leaf_is_never_pending
#print axioms QCore.UncontractedAccessAttribution.the_attribution_is_idempotent
