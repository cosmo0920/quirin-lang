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

import QCore.PremiseFormula.FluentBit

/-! Card **SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES** (C04.18.9): the three
`quirin-provider-fluent-bit` premises as formulas over the Fluent Bit block
of the closed vocabulary.

The formulas are well-formed; the Fluent Bit decision vectors agree with the
judgment (`decision_vectors_agree`, re-proved over the extended table); each
"so" or "means" of the former prose is entailed by the formulas.

The Rust boundary is pinned by `crates/qcore-frontend/tests/premise_formula.rs`
(`the_vocabulary_is_the_lean_table`, `the_decision_vectors_are_the_lean_table`,
`the_fluent_bit_formulas_are_the_lean_definitions`). -/

-- The extended vocabulary still denotes, and the extended vectors agree.
#print axioms QCore.PremiseFormula.check_denotes
#print axioms QCore.PremiseFormula.decision_vectors_agree

-- The three formulas are well-formed.
#print axioms QCore.PremiseFormula.FluentBit.fluent_bit_formulas_wf

-- What each states.
#print axioms QCore.PremiseFormula.FluentBit.eval_untracked
#print axioms QCore.PremiseFormula.FluentBit.eval_ingestion
#print axioms QCore.PremiseFormula.FluentBit.eval_ingestion_core
#print axioms QCore.PremiseFormula.FluentBit.eval_synchronous

-- The entailments the prose states.
#print axioms QCore.PremiseFormula.FluentBit.untracked_admitted_faithful
#print axioms QCore.PremiseFormula.FluentBit.ingestion_core_entails_premise
#print axioms QCore.PremiseFormula.FluentBit.synchronous_output_not_in_tree
