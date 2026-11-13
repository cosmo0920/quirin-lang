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

import QCore.CrossUnitSummary

/-! Card **W44-CALLEE-CROSS-UNIT-SUMMARY**: conditional transport and the
missing, ambiguous, source-changed, and environment-changed refusals. -/

#print axioms QCore.CrossUnitSummary.declaration_wins
#print axioms QCore.CrossUnitSummary.admitted_import_is_valid
#print axioms QCore.CrossUnitSummary.missing_resolution_refuses
#print axioms QCore.CrossUnitSummary.ambiguous_home_refuses
#print axioms QCore.CrossUnitSummary.changed_source_refuses
#print axioms QCore.CrossUnitSummary.changed_environment_refuses

/-! Card **LINK-STANDING** (rule 40): a derived resolution, read from the
linked executable's link-resolution artifact, discharges the same admission
obligation as a stated map; every failed condition refuses, and a refused
home never supplies a summary. -/

#print axioms QCore.CrossUnitSummary.standing_none_iff
#print axioms QCore.CrossUnitSummary.failed_condition_refuses
#print axioms QCore.CrossUnitSummary.database_mismatch_refuses
#print axioms QCore.CrossUnitSummary.unverified_digest_refuses
#print axioms QCore.CrossUnitSummary.unlinked_caller_refuses
#print axioms QCore.CrossUnitSummary.shared_object_refuses
#print axioms QCore.CrossUnitSummary.weak_definition_refuses
#print axioms QCore.CrossUnitSummary.duplicate_definition_refuses
#print axioms QCore.CrossUnitSummary.interposable_refuses
#print axioms QCore.CrossUnitSummary.unlinked_home_configuration_refuses
#print axioms QCore.CrossUnitSummary.unanalysed_linked_configuration_refuses
#print axioms QCore.CrossUnitSummary.ambiguous_key_refuses
#print axioms QCore.CrossUnitSummary.unavailable_type_refuses
#print axioms QCore.CrossUnitSummary.unequal_type_refuses
#print axioms QCore.CrossUnitSummary.typeAgrees_iff
#print axioms QCore.CrossUnitSummary.missing_caller_type_disagrees
#print axioms QCore.CrossUnitSummary.derived_resolution_discharges
#print axioms QCore.CrossUnitSummary.derived_admission_is_valid
#print axioms QCore.CrossUnitSummary.derived_refusal_is_not_admitted
#print axioms QCore.CrossUnitSummary.refused_home_supplies_nothing
#print axioms QCore.CrossUnitSummary.standing_vectors_hold

/-! Card **LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS** (rule 40,
condition 13): the key is decided over the configurations the link can use.
A second configuration shown outside the link (recorded, `linked: false`, no
link input attributed to it) leaves the key and the census to the linked
one; any other second configuration keeps the refusal. -/

#print axioms QCore.CrossUnitSummary.shownOutside_iff
#print axioms QCore.CrossUnitSummary.outside_configuration_is_no_link_input
#print axioms QCore.CrossUnitSummary.an_unrecorded_configuration_is_not_outside
#print axioms QCore.CrossUnitSummary.an_attributed_configuration_is_not_outside
#print axioms QCore.CrossUnitSummary.an_unreadable_attribution_shows_nothing_outside
#print axioms QCore.CrossUnitSummary.a_single_configuration_key_is_unique
#print axioms QCore.CrossUnitSummary.an_outside_second_configuration_leaves_the_key
#print axioms QCore.CrossUnitSummary.a_usable_second_configuration_keeps_the_key
#print axioms QCore.CrossUnitSummary.a_usable_second_configuration_refuses
#print axioms QCore.CrossUnitSummary.the_census_leaves_the_linked_home_unique
#print axioms QCore.CrossUnitSummary.an_unexcluded_second_definition_is_ambiguous
#print axioms QCore.CrossUnitSummary.an_outside_second_configuration_admits_the_linked_home
#print axioms QCore.CrossUnitSummary.outside_vectors_hold
