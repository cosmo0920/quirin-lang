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

import QCore.BitCountBuiltins

/-! Audited companion to K2-BUILTIN-BIT-COUNTS (gateway rule 94). -/

#print axioms QCore.BitCountBuiltins.undefined_at_zero
#print axioms QCore.BitCountBuiltins.defined_off_zero
#print axioms QCore.BitCountBuiltins.the_neutral_contract_is_the_identity
#print axioms QCore.BitCountBuiltins.conversion_keeps_zero
#print axioms QCore.BitCountBuiltins.widening_keeps_nonzero
#print axioms QCore.BitCountBuiltins.isRead_val
#print axioms QCore.BitCountBuiltins.isZero_val
#print axioms QCore.BitCountBuiltins.read_of_isRead
#print axioms QCore.BitCountBuiltins.zero_of_isZero
#print axioms QCore.BitCountBuiltins.cmp_shape
#print axioms QCore.BitCountBuiltins.proves_sound
#print axioms QCore.BitCountBuiltins.admitted_exactly_when
#print axioms QCore.BitCountBuiltins.an_admitted_site_has_a_nonzero_operand
#print axioms QCore.BitCountBuiltins.an_admitted_call_is_defined
#print axioms QCore.BitCountBuiltins.an_unguarded_site_is_refused
#print axioms QCore.BitCountBuiltins.an_unstable_object_is_refused
#print axioms QCore.BitCountBuiltins.a_narrowed_operand_is_refused
#print axioms QCore.BitCountBuiltins.a_statement_guard_needs_structured_flow
#print axioms QCore.BitCountBuiltins.proof_vectors_hold
#print axioms QCore.BitCountBuiltins.decision_vectors_hold
#print axioms QCore.BitCountBuiltins.the_tables_are_counted
