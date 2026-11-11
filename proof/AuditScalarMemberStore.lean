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

import QCore.ScalarMemberStore

/-! Audited Lean companion to card W42-INDIRECT-ACCESS. -/

#print axioms QCore.ScalarMemberStore.an_admitted_store_has_exact_shape
#print axioms QCore.ScalarMemberStore.an_admitted_store_has_a_scalar_field
#print axioms QCore.ScalarMemberStore.an_admitted_store_has_no_rhs_mutation
#print axioms QCore.ScalarMemberStore.a_pointer_field_refuses
#print axioms QCore.ScalarMemberStore.an_rhs_mutation_refuses
#print axioms QCore.ScalarMemberStore.an_ambiguous_base_refuses
#print axioms QCore.ScalarMemberStore.a_nullable_base_refuses
#print axioms QCore.ScalarMemberStore.a_released_base_refuses
#print axioms QCore.ScalarMemberStore.an_unresolved_base_refuses
#print axioms QCore.ScalarMemberStore.an_alias_obligation_refuses
#print axioms QCore.ScalarMemberStore.a_successful_store_preserves_the_projection
#print axioms QCore.ScalarMemberStore.a_flow_refused_store_preserves_the_projection
#print axioms QCore.ScalarMemberStore.a_flow_refused_store_records_unknown_alias
#print axioms QCore.ScalarMemberStore.the_bounded_rule_is_not_vacuous
