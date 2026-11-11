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

import QCore.VariadicConditions

/-! Card **VA-LOOP-AND-POINTER-ARG** (gateway rule 12c): a `va_arg` in a
controlling expression, and the never-owned pointer value.

Audited in two directions. **The condition lowering**: splitting the protocol
graph's head node into the event and the test removes no path and adds none
(`lower_sound`, `lower_complete`), so a certified protocol graph has, lowered,
no violating node and only accepted exit states; the `while` shape is
certified and its master-branch variant (a `return` without `va_end` in the
body) is certified by no assignment. **The never-owned binding**: a `va_arg`
pointer is never released, never changes custody, is never accused, is read
at a borrowed position, and a null test narrows only the binding; an owned
identity at a release is applied (non-vacuity).

What this is not: the AST reading and the Rust graph-lowering's
correspondence to `lower` (trust boundary 28 and review, pinned by tests). -/

-- The condition lowering.
#print axioms QCore.VariadicConditions.length_lower
#print axioms QCore.VariadicConditions.lower_sound
#print axioms QCore.VariadicConditions.lower_complete
#print axioms QCore.VariadicConditions.lowered_no_violation
#print axioms QCore.VariadicConditions.lowered_exit
#print axioms QCore.VariadicConditions.the_while_protocol_is_certified
#print axioms QCore.VariadicConditions.the_while_loop_is_well_formed
#print axioms QCore.VariadicConditions.the_lowered_while_loop_has_no_violation
#print axioms QCore.VariadicConditions.the_leaking_loop_reaches_the_exit_active
#print axioms QCore.VariadicConditions.the_leaking_loop_is_never_certified

-- The never-owned binding.
#print axioms QCore.VariadicConditions.never_owned_is_never_released
#print axioms QCore.VariadicConditions.never_owned_changes_no_custody
#print axioms QCore.VariadicConditions.never_owned_is_never_accused
#print axioms QCore.VariadicConditions.never_owned_is_read_at_a_borrowed_position
#print axioms QCore.VariadicConditions.a_null_test_refines_only_the_binding
#print axioms QCore.VariadicConditions.an_owned_identity_is_released
