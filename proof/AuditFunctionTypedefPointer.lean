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

import QCore.FunctionTypedefPointer

/-! Audited companion to CG10-READER-GAPS (gateway rule 71b). -/

#print axioms QCore.FunctionTypedefPointer.decision_vectors_hold
#print axioms QCore.FunctionTypedefPointer.a_disagreeing_declaration_keeps_the_object_reading
#print axioms QCore.FunctionTypedefPointer.a_pointer_array_typedef_is_an_object
#print axioms QCore.FunctionTypedefPointer.a_pointer_typedef_is_not_a_function_typedef
#print axioms QCore.FunctionTypedefPointer.an_alias_is_not_followed
#print axioms QCore.FunctionTypedefPointer.read_only_for_a_function_typedef
#print axioms QCore.FunctionTypedefPointer.a_function_pointer_adds_no_obligation
