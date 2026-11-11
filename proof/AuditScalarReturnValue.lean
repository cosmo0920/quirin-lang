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

import QCore.ScalarReturnValue

/-! Card **W48-BOTTOM-UP-CALLEE-BODIES**: a scalar `return` value read as the
value of a statement (gateway rule 12f).

Five directions are audited.

**The walker.** An automatic member carries no base; `rd_clock`'s return is
admitted with none; a call, an address and a pointer value are refused.

**Defined where the obligations hold.** An admitted value with every base
live evaluates; a released base fails its obligation.

**Return and statement agree.** The two lowerings coincide, the return
escapes nothing, and an owed object stays owed.

**The decision.** The hook admits exactly a refused return of a scalar-result
function with no descriptor and a walker-admitted value; the Rust table
`DECISION_VECTORS` is `decisionVectors` (`decision_vectors_hold`).

**Never `Safe` by declining.** A declined return's `Unknown` leaf keeps its
function off `Safe`.

The walker's lexical reading and the correspondence with the compiled Rust
are trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/unit/legacy_scalar_return_value.rs` and
`crates/qcore-c-gateway/tests/w48_bottom_up_callee_bodies.rs`. -/

-- The walker.
#print axioms QCore.ScalarReturnValue.an_automatic_member_carries_no_base
#print axioms QCore.ScalarReturnValue.rd_clock_is_admitted_with_no_base
#print axioms QCore.ScalarReturnValue.a_call_is_refused
#print axioms QCore.ScalarReturnValue.an_address_is_refused
#print axioms QCore.ScalarReturnValue.a_pointer_is_refused

-- Defined where the obligations hold.
#print axioms QCore.ScalarReturnValue.admitted_value_is_defined
#print axioms QCore.ScalarReturnValue.a_released_base_fails_its_obligation

-- Return and statement agree.
#print axioms QCore.ScalarReturnValue.return_matches_statement
#print axioms QCore.ScalarReturnValue.a_scalar_return_escapes_nothing
#print axioms QCore.ScalarReturnValue.an_owned_object_stays_owed

-- The decision.
#print axioms QCore.ScalarReturnValue.decide_iff
#print axioms QCore.ScalarReturnValue.decision_vectors_hold

-- Never Safe.
#print axioms QCore.ScalarReturnValue.a_declined_return_is_never_safe
