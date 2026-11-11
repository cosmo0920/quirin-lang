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

import QCore.SummaryValidationPremises

/-! Audited abstract companion to W47-SUMMARY-VALIDATION-PROTECTED-FORMALS (gateway rule 4b). -/

#print axioms QCore.SummaryValidationPremises.the_installed_contract_is_the_own_contract
#print axioms QCore.SummaryValidationPremises.the_validation_run_reads_the_reported_premises
#print axioms QCore.SummaryValidationPremises.a_root_is_never_bound_in_both_runs_or_in_neither
#print axioms QCore.SummaryValidationPremises.may_bind_agrees
#print axioms QCore.SummaryValidationPremises.the_old_events_are_contained
#print axioms QCore.SummaryValidationPremises.every_old_binding_is_kept
#print axioms QCore.SummaryValidationPremises.a_published_summary_passed_on_the_reported_premises
#print axioms QCore.SummaryValidationPremises.a_published_summary_passed_the_old_gate
#print axioms QCore.SummaryValidationPremises.an_assignment_of_the_run_is_the_body's
#print axioms QCore.SummaryValidationPremises.what_protection_adds
#print axioms QCore.SummaryValidationPremises.a_copy_of_a_protected_formal_is_bound
#print axioms QCore.SummaryValidationPremises.a_declaration_keeps_its_premises
#print axioms QCore.SummaryValidationPremises.an_all_borrowed_candidate_protects_nothing
#print axioms QCore.SummaryValidationPremises.an_all_borrowed_candidate_changes_no_premise
#print axioms QCore.SummaryValidationPremises.the_old_validation_set_is_empty_and_settled
#print axioms QCore.SummaryValidationPremises.the_copy_was_never_bound_in_the_old_validation
#print axioms QCore.SummaryValidationPremises.the_copy_is_bound_in_the_reported_run
#print axioms QCore.SummaryValidationPremises.the_old_validation_disagreed_with_the_reported_run
#print axioms QCore.SummaryValidationPremises.the_copy_is_bound_in_the_new_validation
#print axioms QCore.SummaryValidationPremises.a_read_only_body_binds_only_the_formal
