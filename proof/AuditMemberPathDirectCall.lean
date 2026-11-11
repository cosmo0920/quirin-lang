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

import QCore.MemberPathDirectCall

/-! Audited companion to FB-TEARDOWN-DESTROY-DECLARATIONS (gateway rule 30 at a direct call, contract specification C04.19.6 amended; decision 2 of 2026-10-16, the parameter guard): at a direct call a `gone` path is taken exactly once and cleared, a `stays` path is never taken, a `taken` (stale) path is taken once and still designated, so a later release is the second; a set and an unstated path never decide; a guard on a constant actual selects its arm, an unknown actual gives the cautious join of both arms, which never releases and decides only what every value decides. -/

#print axioms QCore.MemberPathDirectCall.the_direct_call_state_is_sound
#print axioms QCore.MemberPathDirectCall.a_release_after_taken_is_a_double_release
#print axioms QCore.MemberPathDirectCall.a_set_never_decides_at_a_direct_call
#print axioms QCore.MemberPathDirectCall.an_unstated_path_never_decides
#print axioms QCore.MemberPathDirectCall.a_constant_selects_its_arm
#print axioms QCore.MemberPathDirectCall.an_unknown_value_gives_the_join
#print axioms QCore.MemberPathDirectCall.an_unknown_value_never_releases
#print axioms QCore.MemberPathDirectCall.the_join_is_cautious
#print axioms QCore.MemberPathDirectCall.direct_vectors_hold
#print axioms QCore.MemberPathDirectCall.guard_vectors_hold
