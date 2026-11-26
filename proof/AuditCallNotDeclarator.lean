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

import QCore.CallNotDeclarator
#print axioms QCore.CallNotDeclarator.vectors_match
#print axioms QCore.CallNotDeclarator.never_adds_a_declaration
#print axioms QCore.CallNotDeclarator.an_admission_is_a_call
#print axioms QCore.CallNotDeclarator.an_admission_is_only_the_paren_star_disjunct
#print axioms QCore.CallNotDeclarator.a_declaration_keeps_the_old_test
#print axioms QCore.CallNotDeclarator.a_declaration_keeps_the_refusal
#print axioms QCore.CallNotDeclarator.a_non_call_keeps_the_old_test
#print axioms QCore.CallNotDeclarator.no_ast_answer_keeps_the_old_test
#print axioms QCore.CallNotDeclarator.a_keyword_declarator_stays
