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

import QCore.ReadShare

#print axioms QCore.ReadShare.lending_adds_exactly_the_granted_share
#print axioms QCore.ReadShare.the_outstanding_total_never_exceeds_full
#print axioms QCore.ReadShare.excess_allocation_of_authority_is_refused
#print axioms QCore.ReadShare.a_zero_share_is_not_a_share
#print axioms QCore.ReadShare.an_over_full_share_is_refused
#print axioms QCore.ReadShare.only_the_owner_may_lend
#print axioms QCore.ReadShare.a_third_simultaneous_holder_is_refused
#print axioms QCore.ReadShare.an_outstanding_share_suspends_exclusive_authority
#print axioms QCore.ReadShare.reconstruction_waits_for_the_last_share
#print axioms QCore.ReadShare.a_returned_share_is_held_by_nobody
#print axioms QCore.ReadShare.returning_the_last_share_restores_exclusive_authority
#print axioms QCore.ReadShare.returning_requires_the_recorded_share
#print axioms QCore.ReadShare.a_retained_share_refuses_every_reclamation
#print axioms QCore.ReadShare.two_positive_shares_admit_exactly_two_borrowers
#print axioms QCore.ReadShare.the_owner_keeps_its_read_across_committed_lending
#print axioms QCore.ReadShare.a_pending_share_is_readable_by_nobody
#print axioms QCore.ReadShare.lending_changes_only_the_lent_resource
#print axioms QCore.ReadShare.returning_changes_only_the_lent_resource
#print axioms QCore.ReadShare.the_admitted_share_round_trip_is_authority_neutral
