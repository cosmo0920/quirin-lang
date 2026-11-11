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

import QCore.ResultConstructorSwitch

/-! Card RULE37-PILOT: every public theorem of rule 37's switch is audited. -/

#print axioms QCore.ResultConstructorSwitch.the_switch_on_reads_rule_37
#print axioms QCore.ResultConstructorSwitch.the_switch_off_reads_the_old_scan
#print axioms QCore.ResultConstructorSwitch.an_escape_with_the_switch_on_escapes_with_it_off
#print axioms QCore.ResultConstructorSwitch.the_switch_off_never_widens_the_scan
#print axioms QCore.ResultConstructorSwitch.the_switch_on_is_the_constructor_step
#print axioms QCore.ResultConstructorSwitch.the_switch_off_refuses_the_checked_sizeof_constructor
#print axioms QCore.ResultConstructorSwitch.the_switch_on_links_the_checked_sizeof_constructor
#print axioms QCore.ResultConstructorSwitch.the_switch_off_withdraws_a_premised_specification
#print axioms QCore.ResultConstructorSwitch.the_switch_off_never_widens_the_specifications
#print axioms QCore.ResultConstructorSwitch.an_unpremised_specification_ignores_the_switch
#print axioms QCore.ResultConstructorSwitch.no_switch_establishes_a_specification
#print axioms QCore.ResultConstructorSwitch.an_admitted_premised_specification_carries_the_premise
#print axioms QCore.ResultConstructorSwitch.a_withdrawn_specification_carries_nothing
