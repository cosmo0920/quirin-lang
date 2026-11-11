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

import QCore.TrackedSelfInit

#print axioms QCore.TrackedSelfInit.missing_source_fact_refuses
#print axioms QCore.TrackedSelfInit.every_leaf_write_is_retained
#print axioms QCore.TrackedSelfInit.resource_ledger_is_preserved
#print axioms QCore.TrackedSelfInit.nullable_facts_are_preserved
#print axioms QCore.TrackedSelfInit.admitted_root_pointer_is_preserved
#print axioms QCore.TrackedSelfInit.admitted_effect_removes_stale_member_facts
#print axioms QCore.TrackedSelfInit.unaffected_facts_survive
#print axioms QCore.TrackedSelfInit.no_child_or_membership_frame_is_erased
#print axioms QCore.TrackedSelfInit.a_borrowed_name_only_site_refuses
#print axioms QCore.TrackedSelfInit.refusal_is_unknown
#print axioms QCore.TrackedSelfInit.finite_composition_keeps_every_obligation
#print axioms QCore.TrackedSelfInit.concrete_self_writes_preserve_lifetimes
#print axioms QCore.TrackedSelfInit.embedded_self_edges_end_with_the_parent
#print axioms QCore.TrackedSelfInit.no_neutral_summary_transport
#print axioms QCore.TrackedSelfInit.no_indirect_callback_transport
#print axioms QCore.TrackedSelfInit.no_cross_unit_effect_privilege
#print axioms QCore.TrackedSelfInit.shared_flow_vectors
#print axioms QCore.TrackedSelfInit.shared_source_target_vectors
