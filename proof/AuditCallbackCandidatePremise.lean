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

import QCore.CallbackCandidatePremise

/-! Card **CALLBACK-CANDIDATE-LITERAL-PREMISE** (gateway rule 57 amended;
contract specification §C04.5): a literal-argument premise at a callback
candidate. Audited: a premise-carrying candidate, and every candidate under
a poisoned premise set, has no contract in force; a premise-free one is
classified as before; the guard only withdraws; an applied invocation or a
releasing registry has no premise-carrying candidate; the guarded decision
applies or releases only where the old one did and equals it on a
premise-free set; the card's vectors (`flb_hash_table_add`'s identity shape,
`rd_kafka_topic_partition_destroy0`'s releasing shape, `d2i_PKCS12_bio`'s
composed shape). The Rust is `LiteralPremises::withdraws_candidate`,
`Parser::candidate_withdrawn` (first in `classified_in`) and
`Parser::candidate_contract` (a registration and a constructed member
registration), bound by `tests/callback_literal_premise.rs` in
`qcore-c-gateway` (trust boundary 28). -/

#print axioms QCore.CallbackCandidatePremise.a_premise_carrying_candidate_is_missing
#print axioms QCore.CallbackCandidatePremise.a_poisoned_set_leaves_no_candidate_contracted
#print axioms QCore.CallbackCandidatePremise.a_premise_free_candidate_is_classified_as_before
#print axioms QCore.CallbackCandidatePremise.the_guard_only_withdraws
#print axioms QCore.CallbackCandidatePremise.a_contracted_class_has_no_premise
#print axioms QCore.CallbackCandidatePremise.applied_only_over_premise_free_identities
#print axioms QCore.CallbackCandidatePremise.releasing_only_over_premise_free_candidates
#print axioms QCore.CallbackCandidatePremise.guarded_map_is_old_when_premise_free
#print axioms QCore.CallbackCandidatePremise.a_premise_free_set_is_decided_as_before
#print axioms QCore.CallbackCandidatePremise.the_guarded_decision_applies_only_where_the_old_did
#print axioms QCore.CallbackCandidatePremise.the_guarded_registry_releases_only_where_the_old_did
#print axioms QCore.CallbackCandidatePremise.cand_identity_shape_was_applied
#print axioms QCore.CallbackCandidatePremise.cand_identity_shape_refuses
#print axioms QCore.CallbackCandidatePremise.cand_releasing_shape_was_releasing
#print axioms QCore.CallbackCandidatePremise.cand_releasing_shape_refuses
#print axioms QCore.CallbackCandidatePremise.cand_composed_shape_refuses
#print axioms QCore.CallbackCandidatePremise.cand_premise_free_identity_applies
#print axioms QCore.CallbackCandidatePremise.cand_premise_free_releasing_releases
