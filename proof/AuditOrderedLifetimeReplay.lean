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

import QCore.OrderedLifetimeReplay
#print axioms QCore.OrderedLifetimeReplay.summary_composition_preserves_the_complete_order
#print axioms QCore.OrderedLifetimeReplay.refusal_survives_every_later_step
#print axioms QCore.OrderedLifetimeReplay.a_later_ending_never_repairs_an_earlier_refusal
#print axioms QCore.OrderedLifetimeReplay.exact_removal_before_release_decides
#print axioms QCore.OrderedLifetimeReplay.poison_cannot_be_cleared_by_exact_removal
#print axioms QCore.OrderedLifetimeReplay.unmatched_then_matching_removal_still_refuses
#print axioms QCore.OrderedLifetimeReplay.an_admitted_import_preserves_every_replayed_effect
#print axioms QCore.OrderedLifetimeReplay.a_missing_home_cannot_authorize_lifetime_import
#print axioms QCore.OrderedLifetimeReplay.an_empty_projection_cannot_authorize_lifetime_import
#print axioms QCore.OrderedLifetimeReplay.an_admitted_join_retains_the_same_trace_on_both_exits
#print axioms QCore.OrderedLifetimeReplay.a_bounded_trace_is_complete_and_within_its_limit
#print axioms QCore.OrderedLifetimeReplay.admitted_imports_keep_the_independently_expected_identity
#print axioms QCore.OrderedLifetimeReplay.a_removal_cannot_be_published_as_an_ordinary_borrow
#print axioms QCore.OrderedLifetimeReplay.mixed_effect_vectors_do_not_establish_an_order
#print axioms QCore.OrderedLifetimeReplay.an_unresolved_external_call_cannot_supply_a_removal
#print axioms QCore.OrderedLifetimeReplay.preseeded_local_inference_is_not_source_authority
