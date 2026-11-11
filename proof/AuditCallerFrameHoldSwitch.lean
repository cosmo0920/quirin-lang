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

import QCore.CallerFrameHoldSwitch

/-! Card RULE32A-PILOT: every public theorem of rule 32a's hold switch is audited. -/

#print axioms QCore.CallerFrameHoldSwitch.the_hold_vectors_are_the_package
#print axioms QCore.CallerFrameHoldSwitch.every_hold_vector_is_the_package_decision
#print axioms QCore.CallerFrameHoldSwitch.holds_iff
#print axioms QCore.CallerFrameHoldSwitch.a_held_call_reads_unknown
#print axioms QCore.CallerFrameHoldSwitch.an_undeclared_callee_is_not_held
#print axioms QCore.CallerFrameHoldSwitch.a_list_naming_the_hold_is_refused
#print axioms QCore.CallerFrameHoldSwitch.a_hold_is_asked_under_every_list
#print axioms QCore.CallerFrameHoldSwitch.the_switch_never_removes_the_hold
#print axioms QCore.CallerFrameHoldSwitch.the_switch_never_widens
#print axioms QCore.CallerFrameHoldSwitch.a_refusing_off_would_widen
#print axioms QCore.CallerFrameHoldSwitch.holding_more_never_certifies_a_link
#print axioms QCore.CallerFrameHoldSwitch.holding_more_changes_no_member_row
#print axioms QCore.CallerFrameHoldSwitch.holding_more_moves_an_outside_row
