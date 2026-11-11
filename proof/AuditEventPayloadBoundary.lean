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

import QCore.EventPayloadBoundary

#print axioms QCore.EventPayloadBoundary.accepted_projection_has_four_paths
#print axioms QCore.EventPayloadBoundary.accepted_projection_has_exact_writes
#print axioms QCore.EventPayloadBoundary.all_four_guard_alternatives_are_present
#print axioms QCore.EventPayloadBoundary.accepted_allocation_binds_queue_extent
#print axioms QCore.EventPayloadBoundary.accepted_access_is_in_bounds
#print axioms QCore.EventPayloadBoundary.accepted_access_has_source_sized_extent
#print axioms QCore.EventPayloadBoundary.negative_index_is_rejected
#print axioms QCore.EventPayloadBoundary.signed_increment_overflow_is_rejected
#print axioms QCore.EventPayloadBoundary.accepted_authority_grants_both_writes
#print axioms QCore.EventPayloadBoundary.accepted_context_is_exact
#print axioms QCore.EventPayloadBoundary.source_substitution_is_rejected
#print axioms QCore.EventPayloadBoundary.authority_substitution_is_rejected
#print axioms QCore.EventPayloadBoundary.allocation_evidence_substitution_is_rejected
