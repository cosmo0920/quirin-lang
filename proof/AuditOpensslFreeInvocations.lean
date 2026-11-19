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

import QCore.OpensslFreeInvocations

/-! Card **OPENSSL-FREE-SESSION-REMOVE-INVOCATION** (contract specification
C04.19 amended in place, gateway rule 12n): a slot invocation beside a
reference drop. Audited: the admission only grows, and only by slot
invocations beside drops; a keep or another callback clause beside a
reference clause still rejects; the callback route reads the object before
the drop (the old order read it after, freed at a last drop); the reading
does not depend on whether the drop is the last, a member effect beside a
drop refuses, and applied implies closed identity slot sets; stating more
slots never improves the outcome; the frees' vectors. The Rust is
`contract_loader.rs` (C04.19), `legacy_transfer.rs` (the order) and
`legacy_retained_by.rs` (the refusal beside a drop), bound by
`tests/openssl_undeclared_handshake_callbacks.rs` in `qcore-c-gateway`
(trust boundary 28). -/

#print axioms QCore.OpensslFreeInvocations.the_admission_only_grows
#print axioms QCore.OpensslFreeInvocations.the_admission_grows_only_by_slot_invocations_beside_drops
#print axioms QCore.OpensslFreeInvocations.a_keep_beside_an_invocation_rejects
#print axioms QCore.OpensslFreeInvocations.another_callback_beside_a_drop_rejects
#print axioms QCore.OpensslFreeInvocations.the_new_order_reads_the_object_before_the_drop
#print axioms QCore.OpensslFreeInvocations.the_old_order_read_a_freed_object
#print axioms QCore.OpensslFreeInvocations.the_reading_does_not_depend_on_the_last_drop
#print axioms QCore.OpensslFreeInvocations.a_member_effect_beside_a_drop_refuses
#print axioms QCore.OpensslFreeInvocations.a_requirement_on_an_unheld_object_refuses
#print axioms QCore.OpensslFreeInvocations.applied_only_over_closed_identity_slots
#print axioms QCore.OpensslFreeInvocations.a_refused_slot_refuses
#print axioms QCore.OpensslFreeInvocations.stating_more_slots_never_improves
#print axioms QCore.OpensslFreeInvocations.free_shape_is_admitted
#print axioms QCore.OpensslFreeInvocations.free_shape_was_refused_before
#print axioms QCore.OpensslFreeInvocations.free_empty_slots_apply
#print axioms QCore.OpensslFreeInvocations.free_identity_candidate_applies
#print axioms QCore.OpensslFreeInvocations.free_uncontracted_candidate_refuses
#print axioms QCore.OpensslFreeInvocations.free_effectful_destroy_refuses
#print axioms QCore.OpensslFreeInvocations.free_unregistered_key_refuses
#print axioms QCore.OpensslFreeInvocations.free_without_run_facts_refuses
#print axioms QCore.OpensslFreeInvocations.free_member_effect_refuses_at_any_drop
