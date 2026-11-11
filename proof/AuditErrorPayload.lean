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

import QCore.ErrorPayload

#print axioms QCore.ErrorPayload.an_outcome_carries_at_most_one_identity
#print axioms QCore.ErrorPayload.payload_free_outcomes_carry_nothing
#print axioms QCore.ErrorPayload.completion_holds_exactly_what_it_names
#print axioms QCore.ErrorPayload.an_unreleased_local_refuses_the_completion
#print axioms QCore.ErrorPayload.a_foreign_payload_refuses_the_completion
#print axioms QCore.ErrorPayload.a_completed_payload_belongs_to_nobody
#print axioms QCore.ErrorPayload.completion_parks_exactly_the_carried_identity
#print axioms QCore.ErrorPayload.the_join_delivers_a_tagged_payload_into_the_bound_error
#print axioms QCore.ErrorPayload.the_join_delivers_an_owned_result_directly
#print axioms QCore.ErrorPayload.the_join_transfers_only_the_carried_identity
#print axioms QCore.ErrorPayload.a_second_join_cannot_transfer_the_payload_again
#print axioms QCore.ErrorPayload.a_payload_inside_a_bound_error_cannot_be_released
#print axioms QCore.ErrorPayload.a_loaded_error_cannot_be_re_raised
#print axioms QCore.ErrorPayload.only_the_holder_can_take_the_payload
#print axioms QCore.ErrorPayload.taking_a_payload_yields_exclusive_authority
#print axioms QCore.ErrorPayload.a_payload_cannot_be_taken_twice
#print axioms QCore.ErrorPayload.a_taken_error_may_be_re_raised
#print axioms QCore.ErrorPayload.taking_preserves_every_other_identity
#print axioms QCore.ErrorPayload.the_admitted_payload_round_trip_ends_released
