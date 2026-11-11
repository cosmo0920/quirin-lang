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

import QCore.RegionJoinsDuties

-- Card `REGION-JOINS-DUTIES` (wave 29): acquisitions in the composed region
-- body — CT25's duty half, "cleanup destroys owner after joins", on
-- `source.epoch.tasks.v1`. Deliberately supersedes constraint 15m's duty-free
-- premise; the wave-28 module is consumed, not edited.

-- The admitted shapes as one list — four constructors where wave 28 had two —
-- and the supersession's conservativity over the duty-free sublanguage.
#print axioms QCore.RegionJoinsDuties.admitted_owner_shapes_are_exactly_these
#print axioms QCore.RegionJoinsDuties.the_duty_free_sublanguage_is_unchanged

-- The pack after the joins carries the body's own duties beside the
-- remainder, and nothing a child held can be among them.
#print axioms QCore.RegionJoinsDuties.the_pack_after_joins_carries_the_bodys_duties
#print axioms QCore.RegionJoinsDuties.the_pack_still_carries_the_quantity
#print axioms QCore.RegionJoinsDuties.duties_are_the_bodys_own

-- The CT25 property: the discharge restores the baseline, destroys the owners
-- in reverse acquisition order, abandons exactly the remainder and consumes
-- the capability; a walked-away duty refuses, and a duty alone makes the pack
-- nonempty.
#print axioms QCore.RegionJoinsDuties.the_cleanup_destroys_the_owners_after_the_joins
#print axioms QCore.RegionJoinsDuties.a_handler_that_walks_away_from_a_duty_is_refused
#print axioms QCore.RegionJoinsDuties.a_duty_alone_makes_the_pack_nonempty

-- The erasure is a program now, and it is the flat twin: the split's written
-- releases are the model's own cleanup trace, and an owner statement moves no
-- credit through the wave-28 cross-check.
#print axioms QCore.RegionJoinsDuties.the_splits_releases_are_the_models_trace
#print axioms QCore.RegionJoinsDuties.an_owner_moves_no_credit

-- The flagship's own figures are non-vacuous: one duty and remainder 3, a
-- body release erases its duty, a sequenced body leaves the second owner, and
-- two surviving duties are destroyed last-first.
#print axioms QCore.RegionJoinsDuties.the_flagship_seals_one_owner_and_abandons_three
#print axioms QCore.RegionJoinsDuties.a_body_release_erases_its_duty
#print axioms QCore.RegionJoinsDuties.a_sequenced_body_leaves_the_second_owner
#print axioms QCore.RegionJoinsDuties.two_owners_are_destroyed_last_first
