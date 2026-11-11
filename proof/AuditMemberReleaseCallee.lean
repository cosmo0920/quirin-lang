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

import QCore.MemberReleaseCallee

/-! Card **W45-SR5-SOUNDNESS** (`MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE`,
`crates/qcore-c-gateway/src/legacy_member_release.rs`, section 4): under rule 5k
as repaired, a released member is not released again through any callee of the
body; a callee that releases it is reported and withholds the place, an
uncharacterised one withholds it, and the chain composes. Premise: P-B08-SHALLOW
for a callee with no body here. Not a proof about the Rust reader. -/

#print axioms QCore.MemberReleaseCallee.an_admitted_member_is_not_released_again
#print axioms QCore.MemberReleaseCallee.a_reported_place_is_never_admitted
#print axioms QCore.MemberReleaseCallee.an_uncharacterised_callee_withholds
#print axioms QCore.MemberReleaseCallee.a_callee_of_other_members_admits
#print axioms QCore.MemberReleaseCallee.a_declared_callee_admits
#print axioms QCore.MemberReleaseCallee.a_wrapper_carries_its_callees_releases
#print axioms QCore.MemberReleaseCallee.a_wrapper_over_an_unknown_callee_is_unknown
#print axioms QCore.MemberReleaseCallee.compose_well_formed
#print axioms QCore.MemberReleaseCallee.the_measured_hole_is_reported
#print axioms QCore.MemberReleaseCallee.the_measured_hole_is_not_admitted
#print axioms QCore.MemberReleaseCallee.the_wrapper_row_is_reported
#print axioms QCore.MemberReleaseCallee.the_other_member_row_stays_admitted
#print axioms QCore.MemberReleaseCallee.the_declared_destructor_stays_admitted
