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

import QCore.FunctionPointerDeclarators

/-! Audited companion to FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS (ledger RECORD-BODY-TOTAL-OR-REJECT; the lexical declaration reader of `typed_field_access.rs`): a record member that is exactly a function-pointer declarator is skipped and joins no class, every other member reads as in the same body without it, every other unparsed declaration still refuses the whole record, rule 5g's flow-refusing store route keeps its domain of records read with every member, and a function-pointer formal is read as a name of no committed type while every other unparsed formal still refuses the function. -/

#print axioms QCore.FunctionPointerDeclarators.an_unreadable_member_refuses_the_record
#print axioms QCore.FunctionPointerDeclarators.a_body_without_unreadable_members_is_read
#print axioms QCore.FunctionPointerDeclarators.the_reading_extends_the_old_reading
#print axioms QCore.FunctionPointerDeclarators.skipping_preserves_the_other_members
#print axioms QCore.FunctionPointerDeclarators.a_read_member_is_a_parsed_declaration
#print axioms QCore.FunctionPointerDeclarators.a_function_pointer_member_is_never_classified
#print axioms QCore.FunctionPointerDeclarators.a_record_read_past_a_function_pointer_is_outside_the_store_route
#print axioms QCore.FunctionPointerDeclarators.on_a_fully_read_record_the_store_route_reads_as_before
#print axioms QCore.FunctionPointerDeclarators.a_function_pointer_formal_is_opaque
#print axioms QCore.FunctionPointerDeclarators.an_opaque_formal_is_never_committed
#print axioms QCore.FunctionPointerDeclarators.an_unreadable_formal_refuses_the_function
#print axioms QCore.FunctionPointerDeclarators.a_function_read_past_a_function_pointer_formal_is_outside_the_store_route
#print axioms QCore.FunctionPointerDeclarators.vectors_hold
