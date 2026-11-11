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

import QCore.TypedAccessPathStatements

/-! Audited abstract companion to W46-ACCESS-PATH-STATEMENTS. -/

#print axioms QCore.TypedAccessPathStatements.missing_type_refuses
#print axioms QCore.TypedAccessPathStatements.missing_field_owner_refuses
#print axioms QCore.TypedAccessPathStatements.unproven_value_type_refuses
#print axioms QCore.TypedAccessPathStatements.pointer_store_refuses
#print axioms QCore.TypedAccessPathStatements.union_refuses
#print axioms QCore.TypedAccessPathStatements.cast_refuses
#print axioms QCore.TypedAccessPathStatements.call_refuses
#print axioms QCore.TypedAccessPathStatements.any_unlisted_form_refuses
#print axioms QCore.TypedAccessPathStatements.mixed_path_with_tracked_root_refuses
#print axioms QCore.TypedAccessPathStatements.refused_shape_has_no_lowering
#print axioms QCore.TypedAccessPathStatements.every_hop_discharge_adds_no_diagnostic
#print axioms QCore.TypedAccessPathStatements.nonpointer_store_preserves_custody
#print axioms QCore.TypedAccessPathStatements.pointer_read_binds_no_custody
#print axioms QCore.TypedAccessPathStatements.refused_pointer_read_stays_unknown
#print axioms QCore.TypedAccessPathStatements.borrowed_read_with_frame_refuses
#print axioms QCore.TypedAccessPathStatements.untracked_alias_declarations_preserve_custody
#print axioms QCore.TypedAccessPathStatements.tracked_alias_declaration_refuses
