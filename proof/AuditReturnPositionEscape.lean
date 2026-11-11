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

import QCore.ReturnPositionEscape

/-! Card C-GATEWAY-RETURN-POSITION: a pointer-valued bounded typed access chain
in a `return` expression, and what its exit may escape.

Card REACH-GATE refused a pointer-valued chain leaf in a `return` on the ground
that `EscapeIfOwned` models only a canonical place. This card supplies that
argument's answer instead of loosening it: the chain rule already refuses a
bound base, and every place the projection tracks has one, so an admitted chain
names no tracked place and the exit has nothing to escape.

The audit covers both directions. Admitting: the strict rule's answer is
unchanged wherever it answered, the widening replaces refusals only, and an
expression neither rule admits keeps its refusal. Discharging — the direction
invariant 1 constrains: the widening never manufactures an escape, and every
obligation the narrow rule reported is still reported, the moved exits
reporting strictly more.

`the_two_answers_agree_on_every_admitted_chain` is the measured boundary
`RETURN-POSITION-ESCAPE-UNREACHABLE`: it is why the Rust arm forcing
`returned_place := none` is unobservable by any test today, and
`without_the_base_premise_the_answers_part` is why that arm is kept as a
backstop rather than simplified away.
-/

#print axioms QCore.ReturnPositionEscape.an_admitted_chain_names_no_tracked_place
#print axioms QCore.ReturnPositionEscape.escaping_an_admitted_chain_changes_nothing
#print axioms QCore.ReturnPositionEscape.the_strict_answer_is_unchanged
#print axioms QCore.ReturnPositionEscape.the_widening_only_replaces_refusals
#print axioms QCore.ReturnPositionEscape.the_widening_never_creates_an_escape
#print axioms QCore.ReturnPositionEscape.an_unadmitted_expression_keeps_its_refusal
#print axioms QCore.ReturnPositionEscape.the_widening_reports_at_least_as_much
#print axioms QCore.ReturnPositionEscape.the_widening_is_accusing
#print axioms QCore.ReturnPositionEscape.the_accusing_direction_is_not_vacuous
#print axioms QCore.ReturnPositionEscape.the_two_answers_agree_on_every_admitted_chain
#print axioms QCore.ReturnPositionEscape.without_the_base_premise_the_answers_part
#print axioms QCore.ReturnPositionEscape.the_parting_witness_is_never_admitted
