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

import QCore.OpaqueCallbackSlots

/-! Card **OPENSSL-SSL-CTX-CALLBACK-SLOTS**: callback slots of a record the
library keeps opaque (gateway rule 12n, C04.19.3).

A slot invocation decides only over the run's facts, only while no unit
defines the record, only when a declaration registers the slot, and then as a
member over the run's set: an open set, an uncontracted or an effectful
candidate refuses, and the identity needs every candidate to be the identity.
The member form still never decides a record no unit defines. Every declared
writer's value is a candidate; a refused slot never publishes `Safe`. The
checker's slot shapes state the registration (and the retention) and release
nothing, and every stated invocation and the unstated-slots premise are in
every accepted declaration.

The Rust boundary is pinned by
`crates/qcore-c-gateway/tests/openssl_callback_slots.rs` and the
`quirin-provider-openssl` checker tests. -/

-- When a slot decides.
#print axioms QCore.OpaqueCallbackSlots.a_slot_without_the_run_facts_never_decides
#print axioms QCore.OpaqueCallbackSlots.a_slot_whose_record_a_unit_defines_never_decides
#print axioms QCore.OpaqueCallbackSlots.a_slot_nothing_registers_never_decides
#print axioms QCore.OpaqueCallbackSlots.an_unregistered_empty_slot_never_decides
#print axioms QCore.OpaqueCallbackSlots.a_decided_slot_is_the_run_decision
#print axioms QCore.OpaqueCallbackSlots.an_open_slot_never_decides
#print axioms QCore.OpaqueCallbackSlots.an_uncontracted_candidate_refuses
#print axioms QCore.OpaqueCallbackSlots.an_effectful_candidate_refuses
#print axioms QCore.OpaqueCallbackSlots.identity_needs_every_candidate_identity
#print axioms QCore.OpaqueCallbackSlots.the_member_form_never_decides_an_undefined_record

-- Every writer is a candidate.
#print axioms QCore.OpaqueCallbackSlots.every_declared_writers_value_is_a_candidate

-- Never `Safe` by an undecided slot.
#print axioms QCore.OpaqueCallbackSlots.a_refused_slot_never_publishes_safe
#print axioms QCore.OpaqueCallbackSlots.a_row_resting_on_a_slot_is_safe_only_if_it_decides

-- The checker's slot shapes.
#print axioms QCore.OpaqueCallbackSlots.a_registration_states_its_slot_and_releases_nothing
#print axioms QCore.OpaqueCallbackSlots.a_stated_invocation_is_in_every_accepted_declaration
#print axioms QCore.OpaqueCallbackSlots.the_unstated_premise_is_in_every_accepted_declaration
