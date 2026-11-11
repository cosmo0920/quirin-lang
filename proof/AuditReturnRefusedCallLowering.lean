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

import QCore.ReturnRefusedCallLowering

/-! Audited Lean companion to card W43-RETURN-EXPRESSION: a whole-call
`return` whose synthetic assignment the statement classifier refuses is lowered
to that refusal and a `NoOp` exit (`legacy_return_call::lowers_refused_whole_call`
and the `Statement::Return` lowering in `legacy_c.rs`). Abstract rule theorems;
extraction and Rust correspondence remain constraint-28 premises. -/

#print axioms QCore.ReturnRefusedCallLowering.an_earlier_admission_is_unchanged
#print axioms QCore.ReturnRefusedCallLowering.the_lowering_only_replaces_the_return_refusal
#print axioms QCore.ReturnRefusedCallLowering.an_admitted_classification_is_never_lowered
#print axioms QCore.ReturnRefusedCallLowering.another_shape_keeps_the_return_refusal
#print axioms QCore.ReturnRefusedCallLowering.a_type_operator_keeps_the_return_refusal
#print axioms QCore.ReturnRefusedCallLowering.a_spelled_reserved_place_keeps_the_return_refusal
#print axioms QCore.ReturnRefusedCallLowering.a_lowered_return_is_still_poisoned
#print axioms QCore.ReturnRefusedCallLowering.the_lowering_escapes_nothing
#print axioms QCore.ReturnRefusedCallLowering.the_lowering_reports_exactly_what_the_refusal_reported
#print axioms QCore.ReturnRefusedCallLowering.a_function_with_a_lowered_return_is_never_safe
#print axioms QCore.ReturnRefusedCallLowering.the_lowering_never_promotes
#print axioms QCore.ReturnRefusedCallLowering.the_site_counter_is_restored
#print axioms QCore.ReturnRefusedCallLowering.the_lowering_is_not_vacuous
#print axioms QCore.ReturnRefusedCallLowering.the_cast_witness_is_not_lowered
