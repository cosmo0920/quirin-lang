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

import QCore.ReadLoan

#print axioms QCore.ReadLoan.reserve_demands_exclusive_authority
#print axioms QCore.ReadLoan.a_borrower_cannot_lend_onward
#print axioms QCore.ReadLoan.an_outstanding_loan_cannot_be_lent_again
#print axioms QCore.ReadLoan.failed_creation_restores_the_owner_exactly
#print axioms QCore.ReadLoan.a_reserved_resource_grants_no_authority_at_all
#print axioms QCore.ReadLoan.an_outstanding_loan_suspends_exclusive_authority
#print axioms QCore.ReadLoan.an_outstanding_loan_cannot_be_released
#print axioms QCore.ReadLoan.a_loan_admits_exactly_its_owner_and_its_borrower
#print axioms QCore.ReadLoan.commit_lends_only_the_reserved_resource
#print axioms QCore.ReadLoan.commit_requires_its_own_reservation
#print axioms QCore.ReadLoan.a_rolled_back_creation_cannot_be_committed
#print axioms QCore.ReadLoan.closing_a_loan_requires_the_recorded_pair
#print axioms QCore.ReadLoan.a_retained_loan_refuses_every_reclamation
#print axioms QCore.ReadLoan.a_successful_join_restores_exclusive_authority_to_the_owner
#print axioms QCore.ReadLoan.a_successful_join_revokes_the_borrower_read
#print axioms QCore.ReadLoan.closing_a_loan_preserves_every_other_resource
#print axioms QCore.ReadLoan.the_admitted_lending_round_trip_is_authority_neutral
