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

import QCore.CallbackSuppliedNull

/-! Card **CALLBACK-SUPPLIED-NULL-ACTUAL** (gateway rule 13u amended in place;
contract specification §C04.19 item 2): the `supplied_nullable` actual.
Audited: a `nonnull` requirement at a nullable actual is never admitted and
refuses the call; an admitted candidate accepts NULL at every nullable
actual; marking `_` as nullable keeps a class or refuses, never admits more;
a list with no nullable actual reads exactly as
`QCore.CallbackContracts.classifyModes`; the shared vectors. The Rust is
`callback_contracts::classify` and `decide`, bound by its unit test
`nullable_vectors_hold_and_are_the_lean_vectors` and by
`tests/callback_supplied_null.rs` (trust boundary 28). -/

#print axioms QCore.CallbackSuppliedNull.a_nonnull_requirement_at_a_nullable_actual_is_not_admitted
#print axioms QCore.CallbackSuppliedNull.decide_refuses_a_nonnull_requirement
#print axioms QCore.CallbackSuppliedNull.an_admitted_candidate_accepts_null
#print axioms QCore.CallbackSuppliedNull.marking_never_admits_more
#print axioms QCore.CallbackSuppliedNull.a_marked_identity_was_an_identity
#print axioms QCore.CallbackSuppliedNull.without_a_nullable_actual_the_reading_is_unchanged
#print axioms QCore.CallbackSuppliedNull.classify_vectors_hold
#print axioms QCore.CallbackSuppliedNull.decision_vectors_hold
