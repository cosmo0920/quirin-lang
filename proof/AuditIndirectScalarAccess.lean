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

import QCore.IndirectScalarAccess

/-! Audited Lean companion to card W43-INDIRECT-ACCESS-GENERAL. -/

#print axioms QCore.IndirectScalarAccess.the_statuses_are_unchanged
#print axioms QCore.IndirectScalarAccess.the_markers_are_unchanged
#print axioms QCore.IndirectScalarAccess.custody_and_field_links_are_unchanged
#print axioms QCore.IndirectScalarAccess.no_binding_gains_a_target
#print axioms QCore.IndirectScalarAccess.a_spared_binding_is_untouched
#print axioms QCore.IndirectScalarAccess.without_a_frame_only_the_destination_changes
#print axioms QCore.IndirectScalarAccess.an_unbound_base_resolves
#print axioms QCore.IndirectScalarAccess.a_nullable_base_refuses
#print axioms QCore.IndirectScalarAccess.a_mixed_base_refuses
#print axioms QCore.IndirectScalarAccess.an_ambiguous_base_refuses
#print axioms QCore.IndirectScalarAccess.an_unknown_base_with_a_target_refuses
#print axioms QCore.IndirectScalarAccess.a_released_base_refuses
#print axioms QCore.IndirectScalarAccess.an_unresolved_base_refuses
#print axioms QCore.IndirectScalarAccess.a_resolved_tracked_base_is_one_live_identity
#print axioms QCore.IndirectScalarAccess.a_resolved_base_is_never_accused
#print axioms QCore.IndirectScalarAccess.an_accusation_names_a_released_settled_identity
#print axioms QCore.IndirectScalarAccess.an_admitted_statement_adds_no_diagnostic
#print axioms QCore.IndirectScalarAccess.a_refusal_is_never_safe
#print axioms QCore.IndirectScalarAccess.an_admitted_statement_keeps_its_lowering
#print axioms QCore.IndirectScalarAccess.a_call_keeps_the_refusal
#print axioms QCore.IndirectScalarAccess.an_address_keeps_the_refusal
#print axioms QCore.IndirectScalarAccess.an_untyped_access_keeps_the_refusal
#print axioms QCore.IndirectScalarAccess.a_pointer_value_keeps_the_refusal
#print axioms QCore.IndirectScalarAccess.a_pointer_destination_keeps_the_refusal
#print axioms QCore.IndirectScalarAccess.incrementing_the_pointer_keeps_the_refusal
#print axioms QCore.IndirectScalarAccess.a_live_identity_is_admitted
#print axioms QCore.IndirectScalarAccess.an_unbound_formal_is_admitted
#print axioms QCore.IndirectScalarAccess.a_guarded_untracked_derived_pointer_is_admitted
#print axioms QCore.IndirectScalarAccess.a_released_identity_is_refused_and_accused
#print axioms QCore.IndirectScalarAccess.the_live_identity_is_not_accused
#print axioms QCore.IndirectScalarAccess.a_full_syntax_candidate_is_admitted
