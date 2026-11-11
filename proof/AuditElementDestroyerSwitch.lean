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

import QCore.ElementDestroyerSwitch

/-! Card RULE34-PILOT: every public theorem of rule 34's switch is audited. -/

#print axioms QCore.ElementDestroyerSwitch.the_phase_vectors_are_the_arm
#print axioms QCore.ElementDestroyerSwitch.every_phase_vector_is_the_arm_phase
#print axioms QCore.ElementDestroyerSwitch.an_admission_has_one_phase_per_effect
#print axioms QCore.ElementDestroyerSwitch.only_a_destroyer_moves_before_the_call
#print axioms QCore.ElementDestroyerSwitch.the_switch_on_orders_every_destroyer_first
#print axioms QCore.ElementDestroyerSwitch.the_switch_off_refuses_a_destroyer
#print axioms QCore.ElementDestroyerSwitch.a_call_without_a_destroyer_ignores_the_switch
#print axioms QCore.ElementDestroyerSwitch.with_no_destroyer_the_arm_order_is_the_host_order
#print axioms QCore.ElementDestroyerSwitch.the_switch_on_is_rule_34
#print axioms QCore.ElementDestroyerSwitch.the_switch_on_destroys_an_admitted_element_safely
#print axioms QCore.ElementDestroyerSwitch.the_switch_off_refuses_the_destroyer
#print axioms QCore.ElementDestroyerSwitch.the_switch_off_reads_unknown
#print axioms QCore.ElementDestroyerSwitch.the_switch_off_never_widens
#print axioms QCore.ElementDestroyerSwitch.the_switch_off_never_admits_a_refused_destroy
