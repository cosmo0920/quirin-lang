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

import QCore.ScalarPrecedence
#print axioms QCore.ScalarPrecedence.no_candidate_keeps_the_old_plan
#print axioms QCore.ScalarPrecedence.refused_probe_keeps_the_old_plan
#print axioms QCore.ScalarPrecedence.every_old_admission_stands
#print axioms QCore.ScalarPrecedence.selection_preserves_the_effect
#print axioms QCore.ScalarPrecedence.selected_typed_plan_retains_old_aliases
#print axioms QCore.ScalarPrecedence.a_failed_original_alias_refuses_the_prepared_plan
#print axioms QCore.ScalarPrecedence.effects_execute_once
#print axioms QCore.ScalarPrecedence.a_changed_selection_is_fully_discharged
#print axioms QCore.ScalarPrecedence.independent_readers_compose
