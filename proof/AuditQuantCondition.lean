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

import QCore.QuantCondition
#print axioms QCore.QuantCondition.any_covers_normal
#print axioms QCore.QuantCondition.any_covers_error
#print axioms QCore.QuantCondition.error_selection_is_exact
#print axioms QCore.QuantCondition.normal_assertion_does_not_supply_entry
#print axioms QCore.QuantCondition.entry_assertion_does_not_supply_normal
#print axioms QCore.QuantCondition.exit_conditions_do_not_prove_termination
#print axioms QCore.QuantCondition.every_explicit_entry_is_required
#print axioms QCore.QuantCondition.regrouping_preserves_explicit_entry
