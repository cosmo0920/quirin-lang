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

import QCore.InPlaceFailureUse

/-! Card **CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE** (gateway rule 74n): a
use whose position carries `resource_kind_failure_use` is admitted exactly
where its object is bound to its latest initialiser's site and that site is
not released (before the status test and on the failure edge), never before
the initialiser or after the finaliser; a use without the clause is rule
74i's. Each step is rule 74i's or an admitted failure use, so only the clause
turns a rule 74i verdict `Safe`, and a `Safe` verdict used only admitted
objects. The Rust correspondence is pinned by
`crates/qcore-c-gateway/tests/in_place_regex_failure_use.rs`. -/

-- Exactly the stated positions.
#print axioms QCore.InPlaceFailureUse.admitted_iff
#print axioms QCore.InPlaceFailureUse.certified_is_admitted
#print axioms QCore.InPlaceFailureUse.admitted_before_the_status_test
#print axioms QCore.InPlaceFailureUse.admitted_on_the_failure_edge
#print axioms QCore.InPlaceFailureUse.certified_on_the_zero_edge
#print axioms QCore.InPlaceFailureUse.refused_before_the_initialiser
#print axioms QCore.InPlaceFailureUse.not_admitted_after_the_finaliser
#print axioms QCore.InPlaceFailureUse.a_use_without_the_clause_is_rule_74i
#print axioms QCore.InPlaceFailureUse.a_plain_use_on_the_failure_edge_is_refused
-- Only admitted failure uses differ from rule 74i.
#print axioms QCore.InPlaceFailureUse.each_step_is_74i_or_an_admitted_failure_use
#print axioms QCore.InPlaceFailureUse.a_program_without_the_clause_is_rule_74i
#print axioms QCore.InPlaceFailureUse.a_new_safe_verdict_rests_on_an_admitted_failure_use
#print axioms QCore.InPlaceFailureUse.transferF_diagnostics_grow
#print axioms QCore.InPlaceFailureUse.runF_keeps_diagnostics
#print axioms QCore.InPlaceFailureUse.a_run_without_refusal_used_only_admitted_objects
#print axioms QCore.InPlaceFailureUse.a_safe_verdict_used_only_admitted_objects
-- Witness programs.
#print axioms QCore.InPlaceFailureUse.the_regex_match_failure_path_was_refused_and_is_safe
#print axioms QCore.InPlaceFailureUse.the_regex_match_success_path_is_safe
#print axioms QCore.InPlaceFailureUse.a_regerror_before_the_test_is_safe
#print axioms QCore.InPlaceFailureUse.a_regexec_on_the_failure_edge_is_not_safe
#print axioms QCore.InPlaceFailureUse.a_regerror_before_the_initialiser_is_not_safe
#print axioms QCore.InPlaceFailureUse.a_regerror_after_regfree_is_a_candidate
#print axioms QCore.InPlaceFailureUse.a_regerror_after_a_failure_edge_regfree_is_not_safe
