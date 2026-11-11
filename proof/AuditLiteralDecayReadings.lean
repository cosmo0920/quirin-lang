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

import QCore.LiteralDecayReadings

/-! Audited companion to W48-BU-SINGLE-LEAF-BODIES (gateway rule 35). -/

#print axioms QCore.LiteralDecayReadings.literal_vectors_hold
#print axioms QCore.LiteralDecayReadings.a_star_is_exempt_exactly_on_a_read_literal
#print axioms QCore.LiteralDecayReadings.a_store_into_a_literal_is_never_exempt
#print axioms QCore.LiteralDecayReadings.a_token_refuses_only_where_it_refused_before
#print axioms QCore.LiteralDecayReadings.the_gate_only_admits
#print axioms QCore.LiteralDecayReadings.every_other_token_is_unchanged
#print axioms QCore.LiteralDecayReadings.a_run_refused_only_at_literal_stars_is_admitted
#print axioms QCore.LiteralDecayReadings.a_literal_read_changes_no_binding_and_owes_nothing
#print axioms QCore.LiteralDecayReadings.decay_vectors_hold
#print axioms QCore.LiteralDecayReadings.a_decay_is_admitted_exactly_for_a_scalar_member
#print axioms QCore.LiteralDecayReadings.a_pointer_or_record_element_is_never_admitted
#print axioms QCore.LiteralDecayReadings.the_decay_is_the_first_element_address
#print axioms QCore.LiteralDecayReadings.an_admitted_decay_is_a_14c_element_address
#print axioms QCore.LiteralDecayReadings.a_14c_refused_element_is_never_a_decay
