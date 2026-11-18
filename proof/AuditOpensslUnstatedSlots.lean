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

import QCore.OpensslUnstatedSlots

/-! Card **OPENSSL-UNSTATED-CALLBACK-SLOTS** (gateway rule 12n, C04.19.3,
C04.18): the slots the narrowed OpenSSL premise covered, stated.

A slot page's NAME list is exactly the writers and not-writers of the slots
citing it, so a writer it names is never left out and an unnamed writer
refuses; the ex_data premise covers no slot, so a declaration that drops the
slot premise for it states every slot it may run; and a premise moves no
verdict: a row is `Safe` only where every stated invocation decides the
identity, and stating one more never improves it.

The Rust boundary is pinned by the `quirin-provider-openssl` checker tests
and `crates/qcore-c-gateway/tests/openssl_unstated_callback_slots.rs`. -/

-- Every writer a page names is classified.
#print axioms QCore.OpensslUnstatedSlots.a_named_function_is_a_writer_or_a_not_writer
#print axioms QCore.OpensslUnstatedSlots.an_unclassified_name_refuses
#print axioms QCore.OpensslUnstatedSlots.an_unnamed_writer_refuses

-- What a premise covers.
#print axioms QCore.OpensslUnstatedSlots.the_ex_data_premise_never_covers_a_slot
#print axioms QCore.OpensslUnstatedSlots.dropping_the_slot_premise_needs_every_slot_stated
#print axioms QCore.OpensslUnstatedSlots.no_premise_needs_every_callback_stated

-- A premise moves no verdict.
#print axioms QCore.OpensslUnstatedSlots.the_verdict_ignores_the_premise
#print axioms QCore.OpensslUnstatedSlots.a_row_is_safe_only_if_every_invocation_decides
#print axioms QCore.OpensslUnstatedSlots.stating_an_invocation_never_improves_a_verdict
