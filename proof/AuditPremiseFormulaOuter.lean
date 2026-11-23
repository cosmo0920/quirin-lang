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

import QCore.PremiseFormula.Outer

/-! Card **SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS** (C04.20, C04.18.9's SP8
block): the two outer contract statements as formulas.

The SP8 block extends the vocabulary and the constants (candidate inventory
constants of sort `obj`); the extended judgment still denotes and the
extended decision vectors agree. Both statements are well-formed; the Fluent
Bit statement entails the librdkafka statement; a string statement is
refused and the header vectors agree with the Rust reader.

The Rust boundary is pinned by `crates/qcore-frontend/tests/premise_formula.rs`. -/

-- The extended vocabulary still denotes, and the extended vectors agree.
#print axioms QCore.PremiseFormula.check_denotes
#print axioms QCore.PremiseFormula.decision_vectors_agree

-- The two statements are well-formed.
#print axioms QCore.PremiseFormula.Outer.outer_formulas_wf

-- The relation the Fluent Bit sentence stated.
#print axioms QCore.PremiseFormula.Outer.fb_entails_rk
#print axioms QCore.PremiseFormula.Outer.fb_states_own

-- The string statement is refused; the header vectors agree.
#print axioms QCore.PremiseFormula.Outer.string_is_not_an_outer_statement
#print axioms QCore.PremiseFormula.Outer.outer_vectors_agree
