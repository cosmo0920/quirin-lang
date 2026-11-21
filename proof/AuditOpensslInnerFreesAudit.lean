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

import QCore.OpensslInnerFreesAudit

/-! Card **OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS** (C04.18, C04.18.9,
gateway rule 12n): the free audit over calls that free keys and certificates
inside.

The handshake's statement entails the free statement and is well formed; a
run names exactly one of the two statements; an entry naming either is
listed; without the handshake's statement the reading is r45's; a call whose
ex_data reading names a key or certificate is listed, and one left out
refuses; the premise moves no verdict.

The Rust boundary is pinned by the `quirin-provider-openssl` checker tests
`every_inner_free_names_what_it_runs` and
`a_drifted_inner_free_reading_is_refused`. -/

-- The handshake's statement conjoins the free statement.
#print axioms QCore.OpensslInnerFreesAudit.conj_holds_right
#print axioms QCore.OpensslInnerFreesAudit.the_handshake_statement_entails_the_free_statement
#print axioms QCore.OpensslInnerFreesAudit.the_handshake_statement_is_well_formed

-- The revised reading.
#print axioms QCore.OpensslInnerFreesAudit.agrees_with_the_release_audit
#print axioms QCore.OpensslInnerFreesAudit.a_run_names_exactly_one_statement
#print axioms QCore.OpensslInnerFreesAudit.a_named_entry_is_listed

-- The extension.
#print axioms QCore.OpensslInnerFreesAudit.a_keyed_entry_is_listed
#print axioms QCore.OpensslInnerFreesAudit.an_unlisted_keyed_entry_refuses

-- The premise moves no verdict.
#print axioms QCore.OpensslInnerFreesAudit.the_inner_free_premise_moves_no_verdict

-- Vectors from the crate's audit.
#print axioms QCore.OpensslInnerFreesAudit.vector_loader_as_landed
#print axioms QCore.OpensslInnerFreesAudit.vector_loader_as_before_refuses
#print axioms QCore.OpensslInnerFreesAudit.vector_loader_unlisted_refuses
#print axioms QCore.OpensslInnerFreesAudit.vector_handshake_as_landed
#print axioms QCore.OpensslInnerFreesAudit.vector_handshake_with_both_statements_refuses
#print axioms QCore.OpensslInnerFreesAudit.vector_handshake_unlisted_refuses
#print axioms QCore.OpensslInnerFreesAudit.vector_constructor_frees_nothing
#print axioms QCore.OpensslInnerFreesAudit.vector_constructor_with_the_statement_refuses
#print axioms QCore.OpensslInnerFreesAudit.vector_unkeyed_plain_call
#print axioms QCore.OpensslInnerFreesAudit.vector_key_free_unchanged
