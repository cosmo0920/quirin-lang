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

import QCore.OpensslExDataPremise

/-! Card **OPENSSL-EX-DATA-REGISTRY** (C04.18): every declaration of
`quirin-provider-openssl` that allocates, copies or finally frees an object
of an ex_data class names `openssl_ex_data_callbacks`.

The checker's audit is total over constructors and frees, the premise is
named exactly where the audit reads a run (over ex_data classes only), a
dropped premise or a second listing refuses, and the premise moves no
verdict.

The Rust boundary is pinned by the `quirin-provider-openssl` checker tests
`every_ex_data_class_object_names_the_premise` and
`a_drifted_ex_data_reading_is_refused`. -/

-- The audit is total over constructors and frees.
#print axioms QCore.OpensslExDataPremise.a_constructor_or_free_is_audited
#print axioms QCore.OpensslExDataPremise.an_unaudited_constructor_or_free_refuses

-- The premise is named exactly where the audit reads a run.
#print axioms QCore.OpensslExDataPremise.a_running_entry_names_the_premise
#print axioms QCore.OpensslExDataPremise.dropping_the_premise_refuses
#print axioms QCore.OpensslExDataPremise.the_premise_is_named_only_where_audited
#print axioms QCore.OpensslExDataPremise.a_run_names_only_ex_data_classes
#print axioms QCore.OpensslExDataPremise.a_second_listing_refuses

-- The premise moves no verdict.
#print axioms QCore.OpensslExDataPremise.the_ex_data_premise_moves_no_verdict

-- Vectors from the crate's audit.
#print axioms QCore.OpensslExDataPremise.vector_x509_new_premised
#print axioms QCore.OpensslExDataPremise.vector_x509_new_unpremised
#print axioms QCore.OpensslExDataPremise.vector_accept_state_unpremised
#print axioms QCore.OpensslExDataPremise.vector_up_reference_unpremised
#print axioms QCore.OpensslExDataPremise.vector_up_reference_premised
#print axioms QCore.OpensslExDataPremise.vector_unaudited_free_refuses
#print axioms QCore.OpensslExDataPremise.vector_unaudited_plain_call
#print axioms QCore.OpensslExDataPremise.vector_unknown_class_refuses
