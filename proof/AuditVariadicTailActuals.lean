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

import QCore.VariadicTailActuals

/-! Card **CG26-TAIL-RESIDUAL-ACTUALS** (gateway rule 12g): a trailing actual
of a declared variadic tail read from Clang's node where rule 11w's token
premises refuse it.

Audited, in four directions. **Data**: over the arithmetic fragment (no cast,
no other node, no pointer-valued operator) an actual is data exactly when no
name in it carries a tracked identity, and every actual read as data carries
none, under either tail. **The borrowed place**: a `borrowed` reading is
exactly one carrying, answered pointer name under a declared borrowed tail;
under the data tail no actual is borrowed (nothing widens the data tail).
**Refusals**: an actual containing an explicit cast (the card
CAST-FORMALIZATION's) and pointer arithmetic are never read; an identity arm,
an identity condition, an unanswered root and an integer-carried identity are
refused by name. **Custody**: the readings, as the modeled tail of a call
whose tail positions are `borrowed`, change no outcome; the shared vectors
are the Rust `READ_VECTORS`.

What this is not: what a name carries (rule 10l's never-bound premise, the
integer-carried identities) and the Clang node and its type are premises
(trust boundary 28); format agreement is not claimed. -/

-- Data.
#print axioms QCore.VariadicTailActuals.data_carries_nothing
#print axioms QCore.VariadicTailActuals.nothing_carried_data
#print axioms QCore.VariadicTailActuals.data_iff_nothing_carried
#print axioms QCore.VariadicTailActuals.every_data_reading_carries_nothing

-- The borrowed place.
#print axioms QCore.VariadicTailActuals.classify_identity_is_a_name
#print axioms QCore.VariadicTailActuals.borrowed_is_the_whole_root
#print axioms QCore.VariadicTailActuals.the_data_tail_is_never_widened

-- Refusals.
#print axioms QCore.VariadicTailActuals.a_cast_is_never_read
#print axioms QCore.VariadicTailActuals.a_cast_is_refused_by_name
#print axioms QCore.VariadicTailActuals.pointer_arithmetic_is_never_read
#print axioms QCore.VariadicTailActuals.an_identity_arm_is_refused
#print axioms QCore.VariadicTailActuals.an_identity_condition_is_refused
#print axioms QCore.VariadicTailActuals.an_unanswered_root_is_refused
#print axioms QCore.VariadicTailActuals.an_integer_identity_is_refused

-- Custody and the shared vectors.
#print axioms QCore.VariadicTailActuals.the_reading_is_custody_neutral
#print axioms QCore.VariadicTailActuals.read_vectors_hold
