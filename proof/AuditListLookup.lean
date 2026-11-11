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

import QCore.ListLookup

#print axioms QCore.ListLookup.empty_is_null
#print axioms QCore.ListLookup.head_is_the_same_identity
#print axioms QCore.ListLookup.nonnull_is_a_member
#print axioms QCore.ListLookup.out_of_range_is_null
#print axioms QCore.ListLookup.vectors_hold
#print axioms QCore.ListLookup.refined_nonnull_needs_valid_member
#print axioms QCore.ListLookup.unknown_head_never_creates_identity
#print axioms QCore.ListLookup.explicit_null_stays_null
#print axioms QCore.ListLookup.physical_vectors_match_indexed_lookup
#print axioms QCore.ListLookup.coherent_prefix_selects_the_same_list_identity
#print axioms QCore.ListLookup.a_view_never_hides_a_tracked_site
#print axioms QCore.ListLookup.a_real_site_keeps_its_original_obligation
#print axioms QCore.ListLookup.a_released_peer_cannot_be_erased
#print axioms QCore.ListLookup.read_view_vectors_hold
#print axioms QCore.ListLookup.a_known_matching_record_is_eligible
#print axioms QCore.ListLookup.an_unknown_record_refuses
#print axioms QCore.ListLookup.a_mismatched_record_refuses
#print axioms QCore.ListLookup.eligibility_requires_known_matching_tags
#print axioms QCore.ListLookup.record_vectors_hold
