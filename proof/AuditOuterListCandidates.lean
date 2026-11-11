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

import QCore.OuterListCandidates

#print axioms QCore.OuterListCandidates.different_lists_have_different_identities
#print axioms QCore.OuterListCandidates.an_unbound_inventory_is_never_in_force
#print axioms QCore.OuterListCandidates.changed_source_is_never_in_force
#print axioms QCore.OuterListCandidates.a_reviewed_registration_keeps_its_context_live
#print axioms QCore.OuterListCandidates.a_reviewed_registration_has_success
#print axioms QCore.OuterListCandidates.a_client_registration_precedes_initialization
#print axioms QCore.OuterListCandidates.an_unreviewed_callback_is_outside_the_candidate
#print axioms QCore.OuterListCandidates.a_candidate_never_contributes_safe
#print axioms QCore.OuterListCandidates.an_in_force_candidate_awaits_review
#print axioms QCore.OuterListCandidates.an_unsupported_candidate_is_unknown
#print axioms QCore.OuterListCandidates.candidates_preserve_the_static_verdict
