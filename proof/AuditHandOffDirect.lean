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

import QCore.HandOffDirect

/-! Audited companion to HANDOFF-DIRECT-CALL-READING (gateway rule 12m amended, contract specification C04.19.5): a direct call of a hand-off declaration forks the caller's state into the guard's two exit classes, each value of a class owned as the guard says; whichever edge of the caller's own test the returned value takes, an accepted edge's owner is the guard's for that value; a calling body that names the object again on a value the guard gives the callback, or never compares the guard's constant, never decides; a decided call never touches a taken object and splits every value as the guard does; the Rust edge vectors are the Lean ones. -/

#print axioms QCore.HandOffDirect.a_class_owner_is_the_guard
#print axioms QCore.HandOffDirect.every_value_has_a_class
#print axioms QCore.HandOffDirect.the_split_state_is_sound_per_class
#print axioms QCore.HandOffDirect.a_caller_that_disagrees_never_decides
#print axioms QCore.HandOffDirect.an_unbranched_direct_call_never_decides
#print axioms QCore.HandOffDirect.a_converted_direct_call_never_decides
#print axioms QCore.HandOffDirect.a_decided_call_never_touches_a_taken_object
#print axioms QCore.HandOffDirect.a_decided_split_is_the_guard_on_every_value
#print axioms QCore.HandOffDirect.direct_vectors_hold
