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

import QCore.DestroyerHelperComposition
#print axioms QCore.DestroyerHelperComposition.composed_only_over_closed_empty_sets
#print axioms QCore.DestroyerHelperComposition.an_open_set_refuses
#print axioms QCore.DestroyerHelperComposition.a_non_empty_set_refuses
#print axioms QCore.DestroyerHelperComposition.no_run_facts_refuses
#print axioms QCore.DestroyerHelperComposition.a_deeper_frame_refuses
#print axioms QCore.DestroyerHelperComposition.a_refused_invocation_applies_no_state
#print axioms QCore.DestroyerHelperComposition.the_composition_carries_the_footprint
#print axioms QCore.DestroyerHelperComposition.a_decided_invocation_is_kept_by_the_fold
#print axioms QCore.DestroyerHelperComposition.a_release_after_the_helpers_take_refuses
#print axioms QCore.DestroyerHelperComposition.a_take_needs_a_live_object
#print axioms QCore.DestroyerHelperComposition.kept_leaves_the_object
#print axioms QCore.DestroyerHelperComposition.evidence_only_through_the_named_helper
#print axioms QCore.DestroyerHelperComposition.the_schedulers_case_composes
