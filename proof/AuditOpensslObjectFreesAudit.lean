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

import QCore.OpensslObjectFreesAudit

/-! Card **OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS** (C04.18, gateway
rule 12n): the free audit.

Every release is audited; a free the audit reads as running names exactly
its object kind's statement of the unstated-slots premise (dropping it,
naming another kind's, or adding the handshake's refuses); a free statement
stands only where the audit reads a run; a free whose run another premise
names carries it; the ex_data premise alone leaves a key's method
uncovered; and the premise moves no verdict.

The Rust boundary is pinned by the `quirin-provider-openssl` checker tests
`every_free_names_what_it_runs_on_the_objects_it_frees` and
`a_drifted_free_reading_is_refused`. -/

-- The audit is total over releases.
#print axioms QCore.OpensslObjectFreesAudit.a_release_is_audited
#print axioms QCore.OpensslObjectFreesAudit.an_unaudited_release_refuses

-- A running free names exactly its audited statement.
#print axioms QCore.OpensslObjectFreesAudit.a_running_free_names_its_statement
#print axioms QCore.OpensslObjectFreesAudit.dropping_the_statement_refuses
#print axioms QCore.OpensslObjectFreesAudit.another_kinds_statement_refuses
#print axioms QCore.OpensslObjectFreesAudit.the_handshake_statement_beside_it_refuses

-- A free statement stands only where the audit reads a run.
#print axioms QCore.OpensslObjectFreesAudit.a_statement_is_named_only_where_audited
#print axioms QCore.OpensslObjectFreesAudit.a_named_by_free_carries_the_premise

-- The ex_data premise alone leaves a key's method uncovered.
#print axioms QCore.OpensslObjectFreesAudit.key_free_vector_needs_the_premise

-- The premise moves no verdict.
#print axioms QCore.OpensslObjectFreesAudit.the_free_premise_moves_no_verdict

-- Vectors from the crate's audit.
#print axioms QCore.OpensslObjectFreesAudit.vector_key_free_as_landed
#print axioms QCore.OpensslObjectFreesAudit.vector_key_free_as_before_refuses
#print axioms QCore.OpensslObjectFreesAudit.vector_certificate_free_with_the_key_statement_refuses
#print axioms QCore.OpensslObjectFreesAudit.vector_engine_free_with_the_ssl_statement_refuses
#print axioms QCore.OpensslObjectFreesAudit.vector_bio_free_as_landed
#print axioms QCore.OpensslObjectFreesAudit.vector_stack_free_with_a_statement_refuses
#print axioms QCore.OpensslObjectFreesAudit.vector_unaudited_free_refuses
#print axioms QCore.OpensslObjectFreesAudit.vector_unaudited_plain_call
