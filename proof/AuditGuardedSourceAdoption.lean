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

import QCore.GuardedSourceAdoption
#print axioms QCore.GuardedSourceAdoption.recovered_only_for_exact_tip
#print axioms QCore.GuardedSourceAdoption.changed_tip_refuses
#print axioms QCore.GuardedSourceAdoption.unsupported_version_refuses
#print axioms QCore.GuardedSourceAdoption.changed_interpreted_output_refuses
#print axioms QCore.GuardedSourceAdoption.changed_original_graph_refuses
#print axioms QCore.GuardedSourceAdoption.changed_legacy_premises_refuse
#print axioms QCore.GuardedSourceAdoption.accepted_metadata_was_fully_validated
#print axioms QCore.GuardedSourceAdoption.recovery_retains_complete_original
#print axioms QCore.GuardedSourceAdoption.absent_terminal_cannot_be_adopted
