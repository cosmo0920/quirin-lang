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

import QCore.CGatewayRefinements
#print axioms QCore.CGatewayRefinements.scalar_write_preserves_custody
#print axioms QCore.CGatewayRefinements.scalar_write_keeps_both_branches
#print axioms QCore.CGatewayRefinements.incomplete_expression_spares_nothing
#print axioms QCore.CGatewayRefinements.expression_frame_has_all_three_premises
#print axioms QCore.CGatewayRefinements.join_guarantees
#print axioms QCore.CGatewayRefinements.absent_predecessor_is_neutral
#print axioms QCore.CGatewayRefinements.null_guard_excludes_a_conditionally_named_site
#print axioms QCore.CGatewayRefinements.assignment_preserves_other_conditional_names
#print axioms QCore.CGatewayRefinements.copying_a_conditional_name_establishes_the_destination
#print axioms QCore.CGatewayRefinements.acquisition_establishes_its_own_conditional_name
#print axioms QCore.CGatewayRefinements.forgetting_relations_is_sound
#print axioms QCore.CGatewayRefinements.conditional_release_discharges_every_present_path
#print axioms QCore.CGatewayRefinements.conditional_identity_does_not_name_an_absent_lookup
#print axioms QCore.CGatewayRefinements.nonnull_does_not_resolve_identity
#print axioms QCore.CGatewayRefinements.null_alternative_has_no_target
#print axioms QCore.CGatewayRefinements.a_conditional_discharge_does_not_certify_the_lookup
