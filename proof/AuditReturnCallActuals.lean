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

import QCore.ReturnCallActuals

/-! Audited companion to CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (gateway rule 12i). -/

#print axioms QCore.ReturnCallActuals.the_lowering_is_the_statement_lowering_then_the_exit
#print axioms QCore.ReturnCallActuals.the_reads_precede_the_call_which_precedes_the_exit
#print axioms QCore.ReturnCallActuals.a_statement_w44_refuses_keeps_the_refusal
#print axioms QCore.ReturnCallActuals.an_uncontracted_callee_keeps_the_refusal
#print axioms QCore.ReturnCallActuals.a_descriptor_read_keeps_the_refusal
#print axioms QCore.ReturnCallActuals.a_declined_probe_keeps_the_refusal
#print axioms QCore.ReturnCallActuals.a_custody_position_is_never_used
#print axioms QCore.ReturnCallActuals.a_reallocated_result_keeps_the_refusal
#print axioms QCore.ReturnCallActuals.the_scalar_form_returns_no_custody
#print axioms QCore.ReturnCallActuals.the_exit_discharges_nothing_prior
#print axioms QCore.ReturnCallActuals.the_comparator_is_admitted
#print axioms QCore.ReturnCallActuals.the_comparator_is_read_before_it_is_returned
#print axioms QCore.ReturnCallActuals.a_releasing_return_keeps_the_refusal
