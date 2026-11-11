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

import QCore.MemberEffectSummary

/-! Audited companion to MEMBER-EFFECT-SUMMARY step 2 (gateway rule 38j). -/

#print axioms QCore.MemberEffectSummary.body_sound
#print axioms QCore.MemberEffectSummary.cascade_sound
#print axioms QCore.MemberEffectSummary.safe_of_inv
#print axioms QCore.MemberEffectSummary.inv_sublist
#print axioms QCore.MemberEffectSummary.inv_of_checkBefore
#print axioms QCore.MemberEffectSummary.flatten_safe_of_inv
#print axioms QCore.MemberEffectSummary.quiet_checkAfter
#print axioms QCore.MemberEffectSummary.safe_append
#print axioms QCore.MemberEffectSummary.two_releases_are_refused
#print axioms QCore.MemberEffectSummary.a_use_after_a_call_is_refused
#print axioms QCore.MemberEffectSummary.a_callee_touching_a_released_place_is_refused
#print axioms QCore.MemberEffectSummary.a_member_effect_never_enters_the_function_table
#print axioms QCore.MemberEffectSummary.an_ordinary_reader_sees_no_contract
