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

import QCore.PremiseFormula.Libevent

/-! Card **SP4-SYMBOLIC-LIBEVENT-PREMISES** (C04.18.9, SP4 block): the seven
premises of `mk_http_thread_destroy` (`qcore-provider-libevent`,
`contracts/monkey/intrusive_list.qv`) as formulas.

The SP4 terms are registered and interpreted in `QCore.PremiseFormula` (its
kind lemmas now range over every vocabulary row, whatever the count); the
seven formulas are well-formed, each states exactly its `eval_*` reading, the
callee and membership premises compose, and the seven are jointly satisfiable
yet each assumes something.

The Rust boundary is pinned by `crates/qcore-frontend/tests/premise_formula.rs`
(`the_vocabulary_is_the_lean_table`, `the_decision_vectors_are_the_lean_table`,
`the_libevent_formulas_are_the_lean_definitions`). -/

-- The vocabulary's kind lemmas, over the SP4 rows too.
#print axioms QCore.PremiseFormula.prim0_kind
#print axioms QCore.PremiseFormula.prim1_kind
#print axioms QCore.PremiseFormula.prim2_kind
#print axioms QCore.PremiseFormula.check_denotes
#print axioms QCore.PremiseFormula.decision_vectors_agree

-- The seven formulas.
#print axioms QCore.PremiseFormula.Libevent.libevent_formulas_wf
#print axioms QCore.PremiseFormula.Libevent.eval_parent_embedding
#print axioms QCore.PremiseFormula.Libevent.eval_tls_params
#print axioms QCore.PremiseFormula.Libevent.eval_callee_live
#print axioms QCore.PremiseFormula.Libevent.eval_borrowed_chain
#print axioms QCore.PremiseFormula.Libevent.eval_valid_membership
#print axioms QCore.PremiseFormula.Libevent.eval_no_later_alias
#print axioms QCore.PremiseFormula.Libevent.eval_valgrind_neutral

-- They compose, hold together, and each assumes something.
#print axioms QCore.PremiseFormula.Libevent.concurrent_runs_leave_the_callee
#print axioms QCore.PremiseFormula.Libevent.libevent_formulas_consistent
#print axioms QCore.PremiseFormula.Libevent.libevent_formulas_assume_something
