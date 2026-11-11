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

import QCore.RegionJoinsLoan
import QCore.CleanupTaskComposition

-- Card `REGION-JOINS-LOAN` (wave 30): read-loan captures and owner adoption
-- on `source.epoch.tasks.v1` — CT25's loan half, "cleanup destroys owner
-- after joins and loan expiry". Deliberately discharges `T08-COMPOSE-LOAN`;
-- the wave-15, wave-29 and T04 modules are consumed, not edited.

-- The capture language: three constructors where wave 15 had two, the
-- inclusion injective and the read capture genuinely new.
#print axioms QCore.RegionJoinsLoan.admitted_captures_are_exactly_these
#print axioms QCore.RegionJoinsLoan.the_loan_free_capture_language_is_unchanged
#print axioms QCore.RegionJoinsLoan.a_read_capture_is_new

-- The region body: five shapes where wave 29 had four, the adoption an
-- acquisition of the pack by definition, and the adoption-free sublanguage
-- unchanged.
#print axioms QCore.RegionJoinsLoan.admitted_region_shapes_are_exactly_these
#print axioms QCore.RegionJoinsLoan.an_adoption_is_an_acquisition_of_the_pack
#print axioms QCore.RegionJoinsLoan.the_adoption_free_sublanguage_is_unchanged

-- The pack after the joins carries the adopted duties beside the remainder,
-- every duty is the parent's own, and an adoption moves no credit.
#print axioms QCore.RegionJoinsLoan.the_pack_after_joins_carries_the_adopted_duties
#print axioms QCore.RegionJoinsLoan.duties_are_the_parents_own
#print axioms QCore.RegionJoinsLoan.an_adoption_moves_no_credit

-- The CT25 property, both halves: while the share is live the destruction
-- refuses; the join — and nothing else — readmits it; the discharge of the
-- adopting pack destroys exactly the owner; and the lending round trip
-- returns the destruction to the owner.
#print axioms QCore.RegionJoinsLoan.a_live_share_refuses_the_destruction
#print axioms QCore.RegionJoinsLoan.the_loan_expires_at_the_join
#print axioms QCore.RegionJoinsLoan.the_cleanup_destroys_the_lent_owner_after_joins_and_expiry
#print axioms QCore.RegionJoinsLoan.the_round_trip_returns_the_destruction_to_the_owner

-- The flagship's own figures are non-vacuous: one adopted duty and remainder
-- 3, a body release erases the adopted duty, and an adopted and an acquired
-- duty are destroyed last-first.
#print axioms QCore.RegionJoinsLoan.the_flagship_adopts_one_owner_and_abandons_three
#print axioms QCore.RegionJoinsLoan.an_adopted_owner_released_in_the_body_erases_its_duty
#print axioms QCore.RegionJoinsLoan.an_adopted_and_an_acquired_duty_are_destroyed_last_first

-- Card `CT24-CLEANUP-TASK-COMPOSITION`: the cleanup/loan product. The
-- existing audit owns this successor module so adding the parity theorem does
-- not create an uncoordinated global audit-count change.
#print axioms QCore.CleanupTaskComposition.cleanup_of_the_lent_owner_before_join_refuses
#print axioms QCore.CleanupTaskComposition.the_same_cleanup_after_join_is_admitted
#print axioms QCore.CleanupTaskComposition.independent_cleanup_before_join_preserves_the_live_task
