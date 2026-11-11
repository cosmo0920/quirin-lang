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

import QCore.ScalarCalls
#print axioms QCore.ScalarCalls.narrow_sound
#print axioms QCore.ScalarCalls.arithmetic_in_range
#print axioms QCore.ScalarCalls.add_exact
#print axioms QCore.ScalarCalls.mul_exact
#print axioms QCore.ScalarCalls.division_zero_refuses
#print axioms QCore.ScalarCalls.remainder_overflow_refuses
#print axioms QCore.ScalarCalls.arguments_preserved
#print axioms QCore.ScalarCalls.wrong_arity_refuses
#print axioms QCore.ScalarCalls.self_call_refuses
#print axioms QCore.ScalarCalls.mutual_call_refuses
#print axioms QCore.ScalarCalls.immutable_prefix
#print axioms QCore.ScalarCalls.agreement_holds

#print axioms QCore.ScalarCalls.increment_41_returns_42
#print axioms QCore.ScalarCalls.unit_call_not_integer

#print axioms QCore.ScalarCalls.show_42_returns_and_prints
#print axioms QCore.ScalarCalls.direct_print_requires_io
#print axioms QCore.ScalarCalls.callee_io_requires_io
#print axioms QCore.ScalarCalls.explicit_empty_refuses_io
#print axioms QCore.ScalarCalls.inferred_or_explicit_io_admitted
