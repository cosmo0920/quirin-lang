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

import QCore.MemberReleaseUntypedBase

/-! Audited companion to MEMBER-RELEASE-UNTYPED-BASE (gateway rule 38j amended again). -/

#print axioms QCore.MemberReleaseUntypedBase.a_previous_admission_is_kept
#print axioms QCore.MemberReleaseUntypedBase.a_new_admission_is_an_untyped_base
#print axioms QCore.MemberReleaseUntypedBase.an_admission_still_needs_the_place_and_a_nullable_release
#print axioms QCore.MemberReleaseUntypedBase.a_failed_place_check_is_refused
#print axioms QCore.MemberReleaseUntypedBase.a_union_base_is_refused
#print axioms QCore.MemberReleaseUntypedBase.an_incomplete_record_is_refused
#print axioms QCore.MemberReleaseUntypedBase.a_qualified_base_is_refused
#print axioms QCore.MemberReleaseUntypedBase.a_local_base_is_refused
#print axioms QCore.MemberReleaseUntypedBase.a_decayed_array_is_refused
#print axioms QCore.MemberReleaseUntypedBase.a_function_pointer_is_refused
#print axioms QCore.MemberReleaseUntypedBase.an_admitted_actual_is_a_struct_pointer_member
#print axioms QCore.MemberReleaseUntypedBase.the_body_stays_sound
#print axioms QCore.MemberReleaseUntypedBase.the_unit_plays_no_part
