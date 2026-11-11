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

import QCore.BuiltinExpectCondition

/-! Audited Lean companion to card BUILTIN-EXPECT-NULL-TEST.

The model states rule 5h's reading of `__builtin_expect(e, c)` (the
`likely`/`unlikely` macros) over the Boolean trees of
`QCore.ConditionCallDesugar`: the wrapper has its operand's value, an
admitted reading evaluates exactly as C evaluates the wrapped expression for
every leaf assignment and either argument order, its desugared plan is that
evaluation, and an effectful hint or a refused operand refuses. The admitted
hint spellings are exactly the integer literals. Faithful C token extraction,
the hint classification and the Rust-to-Lean correspondence are explicit
review premises, per standing constraint 28; the Rust fixtures bind the
admission vector and exercise the correspondence on the bounded shapes.
-/

#print axioms QCore.BuiltinExpectCondition.the_wrapper_has_the_operand_value
#print axioms QCore.BuiltinExpectCondition.the_reading_is_c_evaluation
#print axioms QCore.BuiltinExpectCondition.the_desugared_reading_is_c_evaluation
#print axioms QCore.BuiltinExpectCondition.a_leaf_decides_as_its_bare_test
#print axioms QCore.BuiltinExpectCondition.the_argument_order_is_unobservable
#print axioms QCore.BuiltinExpectCondition.an_effectful_hint_refuses
#print axioms QCore.BuiltinExpectCondition.a_refused_operand_refuses
#print axioms QCore.BuiltinExpectCondition.a_nested_effectful_hint_refuses
#print axioms QCore.BuiltinExpectCondition.an_effectful_hint_is_observable
#print axioms QCore.BuiltinExpectCondition.admitted_spellings_are_literals
#print axioms QCore.BuiltinExpectCondition.vectors_match
