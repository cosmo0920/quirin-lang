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

import QCore.PremiseFormula

/-! Card **SP1-SYMBOLIC-OPENSSL-PREMISES** (C04.18.9, grammar G05): the
symbolic premise statement `requires resource_premise(NAME, FORMULA);`.

Well-formed formulas denote, the loader's acceptance agrees with the
judgment on the shared decision vectors, the four OpenSSL formulas are
well-formed, `#inert` is its definition, and the unstated-slots premise
entails the ex_data premise (not conversely).

The Rust boundary is pinned by `crates/qcore-frontend/tests/premise_formula.rs`
(`the_decision_vectors_are_the_lean_table`, `the_vocabulary_is_the_lean_table`,
`the_openssl_formulas_are_the_lean_definitions`). -/

-- Well-formed formulas denote.
#print axioms QCore.PremiseFormula.check_denotes
#print axioms QCore.PremiseFormula.wf_denotes
#print axioms QCore.PremiseFormula.quant_body_denotes

-- The defined term.
#print axioms QCore.PremiseFormula.inert_definition
#print axioms QCore.PremiseFormula.inert_definition_sort

-- The loader's acceptance agrees with the judgment.
#print axioms QCore.PremiseFormula.decision_vectors_agree

-- The four OpenSSL formulas.
#print axioms QCore.PremiseFormula.openssl_formulas_wf
#print axioms QCore.PremiseFormula.eval_exData
#print axioms QCore.PremiseFormula.unstated_ex_data_part
#print axioms QCore.PremiseFormula.unstated_slots_part

-- The unstated-slots premise entails the ex_data premise.
#print axioms QCore.PremiseFormula.unstated_entails_ex_data
#print axioms QCore.PremiseFormula.unstated_entails_ex_data_for_ssl
#print axioms QCore.PremiseFormula.ex_data_does_not_entail_unstated

-- Card SP5-GLOBAL-PROSE-PREMISE-BAN (C04.18): a string statement is not a
-- premise; every accepted premise is a well-formed formula and denotes. The
-- clause vectors are bound to the Rust `CLAUSE_DECISION_VECTORS` by
-- `crates/qcore-frontend/tests/premise_formula.rs`
-- (`the_clause_vectors_are_the_lean_table`).
#print axioms QCore.PremiseFormula.string_is_not_a_premise
#print axioms QCore.PremiseFormula.accept_formula
#print axioms QCore.PremiseFormula.formula_premise_iff_wf
#print axioms QCore.PremiseFormula.accepted_is_wf
#print axioms QCore.PremiseFormula.accepted_is_a_formula
#print axioms QCore.PremiseFormula.accepted_denotes
#print axioms QCore.PremiseFormula.clause_vectors_agree
