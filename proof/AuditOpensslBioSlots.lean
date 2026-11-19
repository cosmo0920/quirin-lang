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

import QCore.OpensslBioSlots

/-! Card **OPENSSL-BIO-METHODS-AND-CALLBACKS** (gateway rule 12n, C04.19.3,
C04.18, C04.19): a BIO's method table and callback as opaque slots.

A method slot's run set is every function its setters are handed, so a
function a table holds is a candidate or a built-in; the narrowed
unstated-slots premise covers no BIO slot, the built-in methods are covered
only by `openssl_builtin_bio_methods`, and the free premise covers only the
ex_data free callbacks, so a BIO's callback and destroy slots are stated
beside its drop (card OPENSSL-FREE-SESSION-REMOVE-INVOCATION,
`QCore.OpensslFreeInvocations`); and stating the BIO slots never improves a
verdict.

The Rust boundary is pinned by the `quirin-provider-openssl` checker tests
and `crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`. -/

-- A method slot's set is what the setters are handed, beside the built-ins.
#print axioms QCore.OpensslBioSlots.a_setter_written_function_is_a_candidate
#print axioms QCore.OpensslBioSlots.a_held_function_outside_the_set_is_builtin

-- What each premise covers.
#print axioms QCore.OpensslBioSlots.no_premise_covers_a_bio_slot
#print axioms QCore.OpensslBioSlots.a_bio_slot_must_be_stated
#print axioms QCore.OpensslBioSlots.a_builtin_method_needs_its_premise
#print axioms QCore.OpensslBioSlots.the_free_premise_covers_only_ex_data
#print axioms QCore.OpensslBioSlots.the_narrowed_premise_never_covers_a_bio_slot

-- A BIO's drop states its slots.
#print axioms QCore.OpensslBioSlots.a_bio_drop_states_its_slots

-- Stating the BIO slots never improves a verdict.
#print axioms QCore.OpensslBioSlots.stating_the_bio_slots_never_improves_a_verdict
#print axioms QCore.OpensslBioSlots.a_row_is_safe_only_if_every_bio_invocation_decides
