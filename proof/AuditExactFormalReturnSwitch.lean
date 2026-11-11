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

import QCore.ExactFormalReturnSwitch

/-! Card RULE36-PILOT: every public theorem of rule 36's switch is audited. -/

#print axioms QCore.ExactFormalReturnSwitch.the_switch_on_is_the_classifier
#print axioms QCore.ExactFormalReturnSwitch.the_switch_on_is_the_transfer
#print axioms QCore.ExactFormalReturnSwitch.the_switch_off_refuses
#print axioms QCore.ExactFormalReturnSwitch.the_switch_off_never_validates_an_exit
#print axioms QCore.ExactFormalReturnSwitch.the_switch_off_never_widens_an_exit
#print axioms QCore.ExactFormalReturnSwitch.the_switch_off_keeps_custody_and_adds_unknown
#print axioms QCore.ExactFormalReturnSwitch.every_switched_transfer_keeps_owed_resources
#print axioms QCore.ExactFormalReturnSwitch.the_switch_off_asks_no_route
#print axioms QCore.ExactFormalReturnSwitch.the_route_is_only_the_derived_cfgs
#print axioms QCore.ExactFormalReturnSwitch.the_switch_on_is_the_confirmed_route
#print axioms QCore.ExactFormalReturnSwitch.the_switch_off_retains_the_old_summary
#print axioms QCore.ExactFormalReturnSwitch.the_switch_on_is_the_refinement
#print axioms QCore.ExactFormalReturnSwitch.every_switched_refinement_keeps_parameter_custody
