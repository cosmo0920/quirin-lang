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

import QCore.SizeofOperand

/-! Audited Lean companion to card W44-COLLECTION-ALLOCATION. -/

#print axioms QCore.SizeofOperand.vla_is_vm
#print axioms QCore.SizeofOperand.pointers_vm
#print axioms QCore.SizeofOperand.derefN_pointers
#print axioms QCore.SizeofOperand.admits_spec
#print axioms QCore.SizeofOperand.admitted_operand_type
#print axioms QCore.SizeofOperand.an_admitted_operand_is_not_evaluated
#print axioms QCore.SizeofOperand.an_admitted_operand_has_no_effect
#print axioms QCore.SizeofOperand.a_variably_modified_operand_is_evaluated
#print axioms QCore.SizeofOperand.an_unreadable_declaration_is_refused
#print axioms QCore.SizeofOperand.an_undeclared_name_is_refused
#print axioms QCore.SizeofOperand.an_array_declarator_is_refused
#print axioms QCore.SizeofOperand.a_possibly_variably_modified_specifier_is_refused
#print axioms QCore.SizeofOperand.a_deeper_dereference_is_refused
#print axioms QCore.SizeofOperand.no_stars_is_refused
#print axioms QCore.SizeofOperand.the_marker_is_not_an_access_or_a_literal
#print axioms QCore.SizeofOperand.no_operand_leaves_the_gate_unchanged
#print axioms QCore.SizeofOperand.the_view_refuses_no_more
#print axioms QCore.SizeofOperand.an_admitted_statement_keeps_its_lowering
#print axioms QCore.SizeofOperand.a_refusal_is_withdrawn_only_by_the_view
#print axioms QCore.SizeofOperand.the_mention_filter_only_forgets
#print axioms QCore.SizeofOperand.no_operand_forgets_no_name
#print axioms QCore.SizeofOperand.the_collection_allocation_is_admitted
#print axioms QCore.SizeofOperand.the_allocation_gate_passes_the_view
