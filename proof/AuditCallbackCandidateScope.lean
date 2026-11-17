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

import QCore.CallbackCandidateScope

/-! Card **CG22-CALLBACK-CANDIDATE-CONTRACTS**: callback candidates in the W4
derivation scope (gateway rule 4a amended; `legacy_callee_scope.rs`,
`invocation_designators` in `legacy_callback_route.rs`).

Four directions are audited. **The widened scope admits only what linkage
pins**: every member is selected or an internal-linkage definition of the
unit, so an unselected external definition and an undefined name are never
admitted, whatever a body designates. **It only widens**: rule 4a's
direct-call scope is contained in it. **An admitted candidate is the body the
invocation runs.** **No candidate is read benign**: a candidate with no
declaration and no validated summary keeps the invocation refused, and only a
validated identity summary puts it in force as the identity. A witness
completes it.

What this is not: a proof about the Rust token readers (trust boundary 28's
correspondence premise), or about validation itself (`QCore.CustodySummary`). -/

-- The widened scope admits only what linkage pins.
#print axioms QCore.CallbackCandidateScope.every_member_is_selected_or_internal
#print axioms QCore.CallbackCandidateScope.an_unselected_external_definition_is_never_admitted
#print axioms QCore.CallbackCandidateScope.an_undefined_name_is_never_admitted

-- It only widens.
#print axioms QCore.CallbackCandidateScope.the_direct_call_scope_is_contained

-- An admitted candidate is the body the invocation runs.
#print axioms QCore.CallbackCandidateScope.an_admitted_candidate_runs_only_its_analysed_body

-- No candidate is read benign.
#print axioms QCore.CallbackCandidateScope.an_unvalidated_candidate_keeps_the_invocation_refused
#print axioms QCore.CallbackCandidateScope.a_validated_identity_candidate_is_in_force

-- Witness.
#print axioms QCore.CallbackCandidateScope.the_witness_scope
