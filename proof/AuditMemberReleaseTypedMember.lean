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

import QCore.MemberReleaseTypedMember

/-! Audited companion to MEMBER-EFFECT-COMPONENT-SCOPE (gateway rule 38j amended). -/

#print axioms QCore.MemberReleaseTypedMember.an_old_admission_is_kept
#print axioms QCore.MemberReleaseTypedMember.a_new_admission_is_an_untyped_member
#print axioms QCore.MemberReleaseTypedMember.an_admission_needs_the_place_and_a_nullable_release
#print axioms QCore.MemberReleaseTypedMember.an_untyped_base_is_never_admitted
#print axioms QCore.MemberReleaseTypedMember.a_decayed_array_is_refused
#print axioms QCore.MemberReleaseTypedMember.a_function_pointer_is_refused
#print axioms QCore.MemberReleaseTypedMember.a_qualified_pointer_is_refused
#print axioms QCore.MemberReleaseTypedMember.a_union_base_is_refused
#print axioms QCore.MemberReleaseTypedMember.typedPointer_type
#print axioms QCore.MemberReleaseTypedMember.an_admitted_actual_is_an_object_pointer
#print axioms QCore.MemberReleaseTypedMember.the_body_stays_sound
#print axioms QCore.MemberReleaseTypedMember.the_unit_plays_no_part
