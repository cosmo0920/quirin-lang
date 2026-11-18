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

import QCore.CallbackLiteralPremise

/-! Card **OPENSSL-PEM-CALLBACK-ARM** (gateway rule 57; contract
specification §C04.5 "Beside a synchronous invocation"): a literal-argument
premise beside a synchronous invocation. Audited: the loader admits a
premise beside invocations only; an unmet premise or a poisoned set
withdraws a callback declaration whatever its callback; a met premise defers
to rule 13u's decision unchanged; the rewrite grows only on withheld
declarations; the PEM readers' vectors, including the reuse arm the old
rewrite missed; and the null default (user decision 2026-11-18): a null
pointer constant invokes nothing under it and is refused without it, a
non-null callback is decided as before, the default never crosses the
premise, and it is never forwarded. The Rust is `contract_loader.rs` (the retaining branch) and
`legacy_literal_premise::restrict_to_declared_literals`, bound by
`tests/callback_literal_premise.rs` in `qcore-contract` and
`qcore-c-gateway` (trust boundary 28). -/

#print axioms QCore.CallbackLiteralPremise.a_premise_stands_only_beside_invocations
#print axioms QCore.CallbackLiteralPremise.every_other_clause_beside_a_premise_rejects
#print axioms QCore.CallbackLiteralPremise.invocations_with_a_premise_are_admitted
#print axioms QCore.CallbackLiteralPremise.no_premise_is_always_admitted
#print axioms QCore.CallbackLiteralPremise.an_unmet_premise_withdraws_a_callback_declaration
#print axioms QCore.CallbackLiteralPremise.a_mismatched_actual_withdraws
#print axioms QCore.CallbackLiteralPremise.a_missing_actual_never_satisfies
#print axioms QCore.CallbackLiteralPremise.a_poisoned_set_withdraws_every_declared_callee
#print axioms QCore.CallbackLiteralPremise.a_met_premise_defers_to_the_invocation
#print axioms QCore.CallbackLiteralPremise.applied_only_over_identities
#print axioms QCore.CallbackLiteralPremise.an_open_set_refuses
#print axioms QCore.CallbackLiteralPremise.an_uncontracted_candidate_refuses
#print axioms QCore.CallbackLiteralPremise.an_effectful_candidate_refuses
#print axioms QCore.CallbackLiteralPremise.the_new_rewrite_refuses_at_least_the_old
#print axioms QCore.CallbackLiteralPremise.the_rewrite_is_unchanged_on_the_function_map
#print axioms QCore.CallbackLiteralPremise.no_premise_is_transparent
#print axioms QCore.CallbackLiteralPremise.pem_reuse_arm_is_withdrawn
#print axioms QCore.CallbackLiteralPremise.pem_old_rewrite_missed_the_reuse_arm
#print axioms QCore.CallbackLiteralPremise.pem_identity_callback_applies
#print axioms QCore.CallbackLiteralPremise.pem_uncontracted_callback_refuses
#print axioms QCore.CallbackLiteralPremise.pem_null_callback_refuses
#print axioms QCore.CallbackLiteralPremise.a_null_callback_under_the_default_invokes_nothing
#print axioms QCore.CallbackLiteralPremise.without_the_default_a_null_callback_refuses
#print axioms QCore.CallbackLiteralPremise.the_plain_invocation_is_the_flag_off
#print axioms QCore.CallbackLiteralPremise.a_non_null_callback_is_decided_as_before
#print axioms QCore.CallbackLiteralPremise.the_default_never_crosses_the_premise
#print axioms QCore.CallbackLiteralPremise.applied_only_by_the_default_or_over_identities
#print axioms QCore.CallbackLiteralPremise.a_null_default_is_never_forwarded
#print axioms QCore.CallbackLiteralPremise.pem_null_callback_with_the_default_applies
#print axioms QCore.CallbackLiteralPremise.pem_null_callback_reuse_arm_is_withdrawn
#print axioms QCore.CallbackLiteralPremise.pem_formal_callback_with_the_default_refuses
