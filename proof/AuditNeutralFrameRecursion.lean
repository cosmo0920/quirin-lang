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

import QCore.NeutralFrameRecursion

/-! Card C03-NFR: neutral-frame recursion. Axiom surface for the two-point
assumption lattice, its monotone descent to the greatest post-fixed point, the
cascade that makes publication all-or-nothing over a strongly connected
component, the divergence asymmetry that fixes the seeded candidate at the
bottom element, the rank an effectful family would have to supply instead, and
what the published neutral summary asserts through the unchanged validation
relation. No placeholder, no project axiom. -/

-- Lattice, order and descent.
#print axioms QCore.NeutralFrameRecursion.step_descends
#print axioms QCore.NeutralFrameRecursion.step_preserves_bounds
#print axioms QCore.NeutralFrameRecursion.top_is_bounded
#print axioms QCore.NeutralFrameRecursion.post_fixed_iff_stable
#print axioms QCore.NeutralFrameRecursion.step_is_monotone
#print axioms QCore.NeutralFrameRecursion.descent_reaches_a_post_fixed_point
#print axioms QCore.NeutralFrameRecursion.descent_stays_bounded
#print axioms QCore.NeutralFrameRecursion.descent_is_the_greatest_post_fixed_point

-- What a fixed point publishes, and what it refuses.
#print axioms QCore.NeutralFrameRecursion.every_published_member_validated_under_the_published_map
#print axioms QCore.NeutralFrameRecursion.a_refused_member_is_never_published
#print axioms QCore.NeutralFrameRecursion.an_always_refusing_component_publishes_nothing
#print axioms QCore.NeutralFrameRecursion.an_always_validating_component_publishes_every_member
#print axioms QCore.NeutralFrameRecursion.cascade_demotion_propagates_along_calls
#print axioms QCore.NeutralFrameRecursion.a_strongly_connected_component_publishes_all_or_nothing

-- Why the optimistic start is admissible only at the bottom element.
#print axioms QCore.NeutralFrameRecursion.neutral_claim_is_prefix_closed
#print axioms QCore.NeutralFrameRecursion.neutral_claim_is_independent_of_returning
#print axioms QCore.NeutralFrameRecursion.an_exit_claim_depends_on_returning
#print axioms QCore.NeutralFrameRecursion.a_divergent_run_satisfies_every_exit_claim
#print axioms QCore.NeutralFrameRecursion.an_exit_claim_cannot_refuse_a_divergent_effectful_candidate
#print axioms QCore.NeutralFrameRecursion.the_neutral_claim_refuses_an_effectful_run

-- The rank any effectful recursive family has to carry instead.
#print axioms QCore.NeutralFrameRecursion.ranked_calls_are_well_founded
#print axioms QCore.NeutralFrameRecursion.a_ranked_base_member_has_no_admitted_call

-- What the published neutral summary asserts.
#print axioms QCore.NeutralFrameRecursion.the_neutral_summary_binds_no_formal
#print axioms QCore.NeutralFrameRecursion.the_neutral_summary_names_no_owner
#print axioms QCore.NeutralFrameRecursion.the_neutral_summary_governs_every_formal
#print axioms QCore.NeutralFrameRecursion.a_neutral_formal_carries_no_disposition
#print axioms QCore.NeutralFrameRecursion.a_neutral_formal_writes_through_no_slot
#print axioms QCore.NeutralFrameRecursion.a_neutral_formal_is_never_contained
#print axioms QCore.NeutralFrameRecursion.a_neutral_exit_returns_nothing
#print axioms QCore.NeutralFrameRecursion.result_of_exit_match
#print axioms QCore.NeutralFrameRecursion.published_neutral_member_is_analyzed_safely
#print axioms QCore.NeutralFrameRecursion.published_neutral_member_has_a_reachable_exit
#print axioms QCore.NeutralFrameRecursion.published_neutral_member_matches_every_exit
#print axioms QCore.NeutralFrameRecursion.a_published_neutral_member_hands_back_no_custody
