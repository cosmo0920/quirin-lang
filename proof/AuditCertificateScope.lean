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

import QCore.CertificateScope

-- Card E01-TAIL-CERT-SCOPE. Structural replay reconstructs the complete
-- selected denominator from retained source evidence. Authentication of the
-- intended audit additionally uses a caller-held expected context.

#print axioms QCore.CertificateScope.structural_acceptance_has_exact_coverage
#print axioms QCore.CertificateScope.structural_acceptance_has_no_duplicate_targets
#print axioms QCore.CertificateScope.structural_acceptance_binds_inventory_digest
#print axioms QCore.CertificateScope.contextual_acceptance_binds_expected_targets
#print axioms QCore.CertificateScope.contextual_acceptance_binds_expected_input
#print axioms QCore.CertificateScope.contextual_acceptance_binds_expected_compile_task
#print axioms QCore.CertificateScope.contextual_acceptance_binds_expected_trust_context
#print axioms QCore.CertificateScope.unequal_expected_denominator_rejects
#print axioms QCore.CertificateScope.unequal_expected_input_rejects
#print axioms QCore.CertificateScope.unequal_expected_compile_task_rejects
#print axioms QCore.CertificateScope.unequal_expected_trust_context_rejects

#print axioms QCore.CertificateScope.scope_reduction_is_rejected
#print axioms QCore.CertificateScope.duplicate_targets_are_rejected
#print axioms QCore.CertificateScope.unresolved_coverage_is_rejected
#print axioms QCore.CertificateScope.a_coherent_smaller_artifact_is_structurally_distinct
#print axioms QCore.CertificateScope.expected_context_rejects_coherent_scope_replacement
#print axioms QCore.CertificateScope.changed_effective_input_is_rejected
#print axioms QCore.CertificateScope.complete_matching_scope_is_accepted
#print axioms QCore.CertificateScope.changed_compile_task_is_rejected
#print axioms QCore.CertificateScope.changed_trust_context_is_rejected
