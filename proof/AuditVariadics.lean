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

import QCore.Variadics

/-! Card **CG26-VARIADICS** (gateway rule 11w): the `va_list` protocol and the
trailing actuals.

Audited, in four directions. **The table**: the abstract step is the
pointwise lifting of the concrete protocol, and its vectors are the Rust
`STEP_VECTORS`. **Soundness of the certificate**: a graph `certify` accepts
has every reachable concrete state in its assignment, no reachable violating
step, and every concrete state reaching the exit accepted (no missing
`va_end`, including a `va_copy` without its own). **What is refused**: a
`va_arg` after `va_end`, a second pass, an active or passed local at the exit,
and a parameter list started or ended here. **Non-vacuity**: a straight
start/end graph is certified; the same graph with a `return` between them is
certified by no assignment. **Trailing actuals**: reading every tail position
as `borrowed` is the identity on the abstract state, and the address of a
scalar demands nothing.

What this is not: the graph's construction from Clang's AST, and the AST
itself, are trust boundary 28 premises; format agreement (K10) is not claimed. -/

-- The table.
#print axioms QCore.Variadics.step_sound
#print axioms QCore.Variadics.step_refuses
#print axioms QCore.Variadics.exit_sound
#print axioms QCore.Variadics.step_vectors_hold

-- Soundness of the certificate.
#print axioms QCore.Variadics.run_sound
#print axioms QCore.Variadics.certified_invariant
#print axioms QCore.Variadics.certified_no_violation
#print axioms QCore.Variadics.certified_exit

-- What is refused.
#print axioms QCore.Variadics.an_arg_after_end_is_refused
#print axioms QCore.Variadics.a_second_pass_is_refused
#print axioms QCore.Variadics.an_active_local_at_exit_is_refused
#print axioms QCore.Variadics.a_passed_local_at_exit_is_refused
#print axioms QCore.Variadics.a_parameter_is_never_started
#print axioms QCore.Variadics.a_parameter_is_never_ended

-- Non-vacuity.
#print axioms QCore.Variadics.the_straight_protocol_is_certified
#print axioms QCore.Variadics.the_skipping_path_reaches_the_exit_active
#print axioms QCore.Variadics.the_skipping_protocol_is_never_certified

-- Trailing actuals.
#print axioms QCore.Variadics.borrowed_tail_is_the_identity
#print axioms QCore.Variadics.a_scalar_address_demands_nothing
