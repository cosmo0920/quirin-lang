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

import QCore.PremiseFlow
#check QCore.PremiseFlow.proposition
#print axioms QCore.PremiseFlow.conjunction_sound
#print axioms QCore.PremiseFlow.independent_alternatives_sound
#print axioms QCore.PremiseFlow.positive_refutation_sound
#print axioms QCore.PremiseFlow.open_is_not_negation_evidence
#print axioms QCore.PremiseFlow.supported_forbidden_refutes
#print axioms QCore.PremiseFlow.expression_sound
#print axioms QCore.PremiseFlow.terminal_success_implies_condition
#print axioms QCore.PremiseFlow.terminal_cannot_continue
#print axioms QCore.PremiseFlow.alternatives_retain_both_dependencies
#print axioms QCore.PremiseFlow.active_reference_refuses
#print axioms QCore.PremiseFlow.missing_reference_refuses
#print axioms QCore.PremiseFlow.nested_terminal_refuses
#print axioms QCore.PremiseFlow.terminal_reference_in_continuation_refuses
#print axioms QCore.PremiseFlow.foreign_context_refuses
#print axioms QCore.PremiseFlow.wrong_identity_refuses
#print axioms QCore.PremiseFlow.proposed_solution_does_not_establish_legacy
#print axioms QCore.PremiseFlow.positive_refutation_permits
#print axioms QCore.PremiseFlow.terminal_requires_both_conditions
#print axioms QCore.PremiseFlow.exhausted_resolution_refuses
