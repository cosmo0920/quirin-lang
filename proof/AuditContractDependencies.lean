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

import QCore.ContractDependencies

/-! Audited companion to VERDICT-CONTRACT-DEPENDENCIES: the per-row contract dependency set and body standing (gateway rules 13f-13h). -/

#print axioms QCore.ContractDependencies.reaches_trans
#print axioms QCore.ContractDependencies.an_applied_call_is_a_dependency
#print axioms QCore.ContractDependencies.a_consulted_declaration_is_a_dependency
#print axioms QCore.ContractDependencies.the_entry_contract_is_a_dependency
#print axioms QCore.ContractDependencies.a_summary_dependency_propagates
#print axioms QCore.ContractDependencies.a_summarys_applied_call_is_a_dependency
#print axioms QCore.ContractDependencies.publishing_the_set_changes_no_verdict
#print axioms QCore.ContractDependencies.two_sets_publish_the_same_verdict_row
#print axioms QCore.ContractDependencies.a_failure_wins_in_its_unit
#print axioms QCore.ContractDependencies.no_examined_body_is_a_declared_assumption
#print axioms QCore.ContractDependencies.a_held_declaration_is_never_validated
#print axioms QCore.ContractDependencies.validated_needs_evidence_in_every_unit
#print axioms QCore.ContractDependencies.an_unvalidated_body_is_never_validated
#print axioms QCore.ContractDependencies.a_failed_body_is_never_validated
#print axioms QCore.ContractDependencies.a_declared_assumption_is_never_validated
#print axioms QCore.ContractDependencies.outcome_vectors_hold
#print axioms QCore.ContractDependencies.standing_vectors_hold
#print axioms QCore.ContractDependencies.closure_vectors_hold
