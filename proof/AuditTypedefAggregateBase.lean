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

import QCore.TypedefAggregateBase

/-! Audited companion to AG1-INDIRECT-MEMBER-ACCESS (gateway rule 61 (1)). -/

#print axioms QCore.TypedefAggregateBase.an_agreeing_alias_names_its_struct_everywhere
#print axioms QCore.TypedefAggregateBase.a_disagreeing_alias_names_nothing
#print axioms QCore.TypedefAggregateBase.an_other_declaration_names_nothing
#print axioms QCore.TypedefAggregateBase.a_missing_fact_never_admits
#print axioms QCore.TypedefAggregateBase.decision_vectors_hold
#print axioms QCore.TypedefAggregateBase.an_admitted_local_is_the_struct_pointer
#print axioms QCore.TypedefAggregateBase.the_store_decides_alike_over_either_spelling
#print axioms QCore.TypedefAggregateBase.a_refused_alias_keeps_the_refusal
#print axioms QCore.TypedefAggregateBase.a_struct_alias_is_never_variably_modified
