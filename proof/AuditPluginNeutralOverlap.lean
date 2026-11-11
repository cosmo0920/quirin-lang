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

import QCore.PluginNeutralOverlap
#print axioms QCore.PluginNeutralOverlap.matched_atoms_retain_exact_statement
#print axioms QCore.PluginNeutralOverlap.matched_refinements_retain_children_and_target
#print axioms QCore.PluginNeutralOverlap.missing_or_changed_atom_refuses
#print axioms QCore.PluginNeutralOverlap.missing_or_changed_refinement_refuses
#print axioms QCore.PluginNeutralOverlap.failed_bounds_parse_refuses
#print axioms QCore.PluginNeutralOverlap.absent_metadata_uses_original_predicate
#print axioms QCore.PluginNeutralOverlap.independently_matched_attachment_preserves_original_predicate
