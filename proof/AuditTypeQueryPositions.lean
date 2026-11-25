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

import QCore.TypeQueryPositions

/-! Audited companion to K2-NO-TYPED-ACCESS (gateway rule 79). -/

#print axioms QCore.TypeQueryPositions.admitted_exactly_when
#print axioms QCore.TypeQueryPositions.a_refusal_negates_its_condition
#print axioms QCore.TypeQueryPositions.run_of_inert
#print axioms QCore.TypeQueryPositions.sublist_of_inert
#print axioms QCore.TypeQueryPositions.an_admitted_condition_is_the_identity_on_every_path
#print axioms QCore.TypeQueryPositions.an_admitted_condition_owes_nothing
#print axioms QCore.TypeQueryPositions.the_lowering_is_the_condition
#print axioms QCore.TypeQueryPositions.a_call_or_store_is_refused
#print axioms QCore.TypeQueryPositions.an_aliased_read_is_refused
#print axioms QCore.TypeQueryPositions.an_evaluated_query_is_refused
#print axioms QCore.TypeQueryPositions.a_variably_modified_query_is_refused
#print axioms QCore.TypeQueryPositions.a_condition_without_a_query_is_refused
#print axioms QCore.TypeQueryPositions.sizeof_is_no_declarator
#print axioms QCore.TypeQueryPositions.a_product_run_is_no_declaration
#print axioms QCore.TypeQueryPositions.dropping_product_runs_loses_no_declaration
#print axioms QCore.TypeQueryPositions.decision_vectors_hold
#print axioms QCore.TypeQueryPositions.the_tables_are_counted
