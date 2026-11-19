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

import QCore.OpensslHandshakeSlots

/-! Card **OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS** (gateway rule 12n,
C04.19.3, C04.18): the handshake slots whose setters were undeclared.

A setter may register several slots (the custom extension setters), and is
then a writer of each; an entry registering a slot it is not listed for, or a
listed writer whose entry dropped the registration, refuses the slot; a slot
the library writes through no function pointer formal (the CT validation
slot) is never accepted and stays under the unstated-slots premise; and
stating more invocations never improves a verdict.

The Rust boundary is pinned by the `quirin-provider-openssl` checker tests
and `crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`. -/

-- A setter may write several slots.
#print axioms QCore.OpensslHandshakeSlots.a_further_registration_is_a_registration
#print axioms QCore.OpensslHandshakeSlots.the_shape_registration_is_a_registration
#print axioms QCore.OpensslHandshakeSlots.an_unlisted_registering_entry_refuses
#print axioms QCore.OpensslHandshakeSlots.a_writer_without_the_registration_refuses
#print axioms QCore.OpensslHandshakeSlots.every_registering_entry_is_a_writer
#print axioms QCore.OpensslHandshakeSlots.every_writer_registers_the_slot

-- A slot the library writes is never accepted.
#print axioms QCore.OpensslHandshakeSlots.a_library_written_slot_is_never_accepted
#print axioms QCore.OpensslHandshakeSlots.an_unstated_slot_needs_the_premise

-- Stating more slots never improves a verdict.
#print axioms QCore.OpensslHandshakeSlots.stating_more_invocations_never_improves_a_verdict
#print axioms QCore.OpensslHandshakeSlots.a_row_is_safe_only_if_every_new_invocation_decides
