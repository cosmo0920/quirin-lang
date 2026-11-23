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

import QCore.InPlaceCertifiedUse

/-! Card **CG25-IN-PLACE-REGEX** (gateway rule 74i): every in-place use is
rule 11v (7)'s certified use, whatever the kind, and an in-place object is
tracked only when its name has one declaration that every reference
designates. Each step agrees with CG25's reading or adds a refusal, so no
verdict that was not `Safe` becomes `Safe`, and a `Safe` verdict used only
certified objects. Which declaration a reference designates is read off Clang
(trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/in_place_regex_certified.rs`. -/

#print axioms QCore.InPlaceCertifiedUse.the_key_use_is_rule_11v_7
#print axioms QCore.InPlaceCertifiedUse.every_other_step_is_cg25
-- The certified use.
#print axioms QCore.InPlaceCertifiedUse.a_certified_use_changes_nothing
#print axioms QCore.InPlaceCertifiedUse.an_uncertified_use_is_refused
#print axioms QCore.InPlaceCertifiedUse.an_uncertified_use_is_never_safe
#print axioms QCore.InPlaceCertifiedUse.a_use_after_finalise_is_still_a_candidate
-- Only refusals are added.
#print axioms QCore.InPlaceCertifiedUse.each_step_agrees_or_refuses
#print axioms QCore.InPlaceCertifiedUse.transferC_diagnostics_grow
#print axioms QCore.InPlaceCertifiedUse.run_keeps_diagnostics
#print axioms QCore.InPlaceCertifiedUse.a_run_without_refusal_is_cg25
#print axioms QCore.InPlaceCertifiedUse.no_verdict_becomes_safe
#print axioms QCore.InPlaceCertifiedUse.a_run_without_refusal_used_only_certified_objects
#print axioms QCore.InPlaceCertifiedUse.a_safe_verdict_used_only_certified_objects
-- One declaration.
#print axioms QCore.InPlaceCertifiedUse.one_declaration_iff
#print axioms QCore.InPlaceCertifiedUse.one_declaration_is_cg25
#print axioms QCore.InPlaceCertifiedUse.a_shadowed_name_is_refused
-- Witness programs.
#print axioms QCore.InPlaceCertifiedUse.the_control_success_path_is_safe
#print axioms QCore.InPlaceCertifiedUse.the_control_failure_path_is_safe
#print axioms QCore.InPlaceCertifiedUse.probe_r4_was_safe_and_is_refused
#print axioms QCore.InPlaceCertifiedUse.probe_r3_was_safe_and_is_refused
#print axioms QCore.InPlaceCertifiedUse.a_use_before_the_initialiser_was_safe_and_is_refused
#print axioms QCore.InPlaceCertifiedUse.a_use_after_regfree_is_a_candidate
#print axioms QCore.InPlaceCertifiedUse.probe_r2_was_safe_and_is_refused
