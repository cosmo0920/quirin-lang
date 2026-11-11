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

import QCore.MemberPathUnitHelpers

/-! Audited companion to MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS (gateway rule 30, amended; contract specification C04.19.6): rule 30's bounded path reading is sound for every frame depth (every concrete run's final reading is admitted), so a validated must path reads null, a validated set path is untouched or null and a validated stale path is untouched on every run; reading a followed call is reading its body (a call summary is inlining); a call that is not followed, at the depth bound or recursive, never validates; an exit is exempt from a state only when it returns the enumerator of the state's `!=` guard, where that guard is false. -/

#print axioms QCore.MemberPathUnitHelpers.untouched_end_is_entry
#print axioms QCore.MemberPathUnitHelpers.aexec_sound
#print axioms QCore.MemberPathUnitHelpers.a_validated_must_path_reads_null
#print axioms QCore.MemberPathUnitHelpers.a_validated_set_path_is_kept_or_null
#print axioms QCore.MemberPathUnitHelpers.a_validated_stale_path_is_untouched
#print axioms QCore.MemberPathUnitHelpers.a_call_summary_is_inlining
#print axioms QCore.MemberPathUnitHelpers.a_call_runs_its_body
#print axioms QCore.MemberPathUnitHelpers.a_path_not_followed_never_validates
#print axioms QCore.MemberPathUnitHelpers.the_depth_bound_never_validates
#print axioms QCore.MemberPathUnitHelpers.a_null_store_validates_must
#print axioms QCore.MemberPathUnitHelpers.an_exempt_exit_falsifies_its_guard
#print axioms QCore.MemberPathUnitHelpers.an_unnamed_return_is_never_exempt
#print axioms QCore.MemberPathUnitHelpers.an_eq_guard_is_never_exempt
