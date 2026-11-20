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

import QCore.OpensslRemainingSetters

/-! Card **OPENSSL-REMAINING-HANDSHAKE-SETTERS** (gateway rule 12n,
C04.19.3, C04.18): the remaining OpenSSL handshake setters and the
certificate store's callbacks.

A slot whose writer no manual page documents is never accepted and stays
under the unstated-slots premise; a call through a macro alias is its
writer's call, so listing the alias as a not-writer loses no candidate; a
premise statement drops a kind only when that kind's slots are stated, and
stating slots keeps it honest; and stating more invocations never improves a
verdict.

The Rust boundary is pinned by the `quirin-provider-openssl` checker tests
and `crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`. -/

-- A setter no page documents leaves its slot unstated.
#print axioms QCore.OpensslRemainingSetters.an_undocumented_slot_is_never_accepted
#print axioms QCore.OpensslRemainingSetters.a_page_omitting_a_writer_refuses
#print axioms QCore.OpensslRemainingSetters.an_undocumented_slot_needs_the_premise

-- A macro alias adds no candidate.
#print axioms QCore.OpensslRemainingSetters.an_alias_call_is_its_writers_call
#print axioms QCore.OpensslRemainingSetters.an_alias_call_and_a_writer_call_agree

-- A premise's statement drops a kind only when its slots are stated.
#print axioms QCore.OpensslRemainingSetters.dropping_a_kind_needs_its_slots_stated
#print axioms QCore.OpensslRemainingSetters.stating_slots_keeps_a_statement_honest

-- Stating more slots never improves a verdict.
#print axioms QCore.OpensslRemainingSetters.stating_the_remaining_slots_never_improves_a_verdict
#print axioms QCore.OpensslRemainingSetters.a_row_is_safe_only_if_every_remaining_invocation_decides
