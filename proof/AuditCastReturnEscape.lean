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

import QCore.CastReturnEscape

/-! Card **W44-LEAK-PIN-AUDIT**, discharging `RETURN-CAST-PLACE-NOT-ESCAPED`:
`return (T *) p;` is decided exactly as `return p;` is, when the cast preserves
the carrier.

Four directions are audited. **Delegation preserves the carrier**: only a
pointer-to-pointer conversion of a place that is neither a declared scalar nor
a possible descriptor, or `(int)` over a possible descriptor, is decided as the
uncast return, and a pointer cast of an expression is not.
**Crossings refuse**: an integer-to-pointer or pointer-to-integer cast of a
possibly tracked value, and a cast of a descriptor to another integer type, is
refused and never escaped; untracked scalar data keeps its pre-card answer.
**The cast is the uncast return, and only withdraws**: a delegated answer
reports exactly what the uncast return reports, whatever the rule answers it
reports about the returned site at most what the pre-card answer reported, and
the corpus shape's false leak is gone. **Non-vacuity**: every answer is reached.

What this is not: a proof about the Rust parser or about the declaration
reader's extraction (trust boundary 28). A remaining leak is a provisional
candidate, never an upstream defect. -/

#print axioms QCore.CastReturnEscape.delegation_preserves_the_carrier
#print axioms QCore.CastReturnEscape.a_descriptor_through_int_is_the_uncast_return
#print axioms QCore.CastReturnEscape.a_descriptor_cast_to_a_pointer_refuses
#print axioms QCore.CastReturnEscape.a_descriptor_cast_to_another_integer_refuses
#print axioms QCore.CastReturnEscape.a_pointer_cast_to_an_integer_refuses
#print axioms QCore.CastReturnEscape.no_crossing_is_delegated
#print axioms QCore.CastReturnEscape.untracked_scalar_data_is_unchanged
#print axioms QCore.CastReturnEscape.a_pointer_cast_of_an_expression_is_not_delegated
#print axioms QCore.CastReturnEscape.the_cast_is_the_uncast_return
#print axioms QCore.CastReturnEscape.the_rule_only_withdraws
#print axioms QCore.CastReturnEscape.the_cast_return_leak_is_withdrawn
#print axioms QCore.CastReturnEscape.every_answer_is_reached
