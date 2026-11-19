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

import QCore.OpensslBioReaders

/-! Card **OPENSSL-BIO-READER-INVOCATIONS** (gateway rule 57 amended;
contract specification §C04.5 "Beside a slot invocation"): the PEM and
PKCS#12 readers state the BIO's 14 slot invocations beside their
literal-argument premise. Audited: the reader shape is admitted and was
refused before; a plain member invocation is still refused; at an unmet call
(or under a poisoned set) no slot is decided; at a met call each slot is
decided as rule 12n decides it and the call applies only over closed slot
sets of identity candidates; a slot without run facts or registrations
refuses; the premise adds no candidate; stating more slots never improves
the outcome; the readers' vectors. The Rust is `contract_loader.rs` (the
retaining branch) and `legacy_literal_premise::restrict_to_declared_literals`,
bound by `tests/callback_literal_premise.rs` in `qcore-contract` and
`tests/openssl_undeclared_handshake_callbacks.rs` in `qcore-c-gateway`
(trust boundary 28). -/

#print axioms QCore.OpensslBioReaders.a_slot_invocation_stands_beside_a_premise
#print axioms QCore.OpensslBioReaders.an_unmet_premise_decides_no_slot
#print axioms QCore.OpensslBioReaders.a_poisoned_set_decides_no_slot
#print axioms QCore.OpensslBioReaders.a_met_premise_defers_to_the_slots
#print axioms QCore.OpensslBioReaders.applied_only_over_closed_identity_slots
#print axioms QCore.OpensslBioReaders.a_refused_slot_refuses
#print axioms QCore.OpensslBioReaders.a_slot_without_run_facts_refuses
#print axioms QCore.OpensslBioReaders.an_unregistered_slot_refuses
#print axioms QCore.OpensslBioReaders.an_empty_closed_slot_invokes_nothing
#print axioms QCore.OpensslBioReaders.the_premise_adds_no_candidate
#print axioms QCore.OpensslBioReaders.stating_more_slots_never_improves
#print axioms QCore.OpensslBioReaders.reader_shape_is_admitted
#print axioms QCore.OpensslBioReaders.reader_shape_was_refused_before
#print axioms QCore.OpensslBioReaders.pkcs12_shape_is_admitted
#print axioms QCore.OpensslBioReaders.a_plain_member_invocation_still_refused
#print axioms QCore.OpensslBioReaders.reader_null_callback_empty_slots_apply
#print axioms QCore.OpensslBioReaders.reader_reuse_arm_decides_no_slot
#print axioms QCore.OpensslBioReaders.reader_uncontracted_bio_candidate_refuses
#print axioms QCore.OpensslBioReaders.reader_effectful_bio_method_refuses
#print axioms QCore.OpensslBioReaders.reader_unregistered_key_refuses
#print axioms QCore.OpensslBioReaders.reader_without_run_facts_refuses
#print axioms QCore.CallbackLiteralPremise.the_admission_only_grows
#print axioms QCore.CallbackLiteralPremise.the_admission_grows_only_by_slot_invocations
#print axioms QCore.CallbackLiteralPremise.a_plain_member_invocation_beside_a_premise_rejects
