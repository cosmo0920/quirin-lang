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

import QCore.OpensslFreeIndirectCallbacks

/-! Card **OPENSSL-FREE-INDIRECT-CALLBACKS** (gateway rule 12n, C04.19.3,
C04.18): the callbacks the OpenSSL frees run on objects they free in turn.

The frees keep the unstated-slots premise while any callback they may run is
unstated, and the ex_data premise alone would not cover it; a statement
narrows only by kinds the call never runs; a slot the call does not run (a
store's own slots at its free) needs no statement; stating the lookup slots
never improves the outcome; and the frees' vectors.

The Rust boundary is pinned by the `quirin-provider-openssl` checker tests
and `crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`. -/

-- The frees keep the premise while a remainder is unstated.
#print axioms QCore.OpensslFreeIndirectCallbacks.a_free_with_a_remainder_keeps_the_premise
#print axioms QCore.OpensslFreeIndirectCallbacks.the_ex_data_premise_alone_leaves_the_remainder_uncovered
#print axioms QCore.OpensslFreeIndirectCallbacks.store_free_vector_needs_the_premise

-- A statement narrows only by kinds the call never runs.
#print axioms QCore.OpensslFreeIndirectCallbacks.dropping_a_kind_the_call_never_runs_keeps_it_honest

-- A slot the call does not run needs no statement.
#print axioms QCore.OpensslFreeIndirectCallbacks.a_slot_the_call_does_not_run_needs_no_statement

-- Stating the lookup slots never improves the outcome.
#print axioms QCore.OpensslFreeIndirectCallbacks.stating_the_lookup_slots_never_improves

-- The frees' vectors.
#print axioms QCore.OpensslFreeIndirectCallbacks.ssl_free_empty_slots_apply
#print axioms QCore.OpensslFreeIndirectCallbacks.an_identity_lookup_candidate_applies
#print axioms QCore.OpensslFreeIndirectCallbacks.an_uncontracted_lookup_candidate_refuses
#print axioms QCore.OpensslFreeIndirectCallbacks.an_effectful_lookup_candidate_refuses
#print axioms QCore.OpensslFreeIndirectCallbacks.an_unregistered_lookup_key_refuses
