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

import QCore.CastRepresentation

/-! Audited companion to CG08-CASTS-AND-TYPE-QUERIES (gateway rule 11b). -/
#print axioms QCore.CastRepresentation.kinds_unique
#print axioms QCore.CastRepresentation.provenance_rows
#print axioms QCore.CastRepresentation.cast_has_no_custody_effect
#print axioms QCore.CastRepresentation.tracked_pointer_to_integer_refused
#print axioms QCore.CastRepresentation.untracked_pointer_to_integer_is_data
#print axioms QCore.CastRepresentation.integer_to_pointer_has_no_identity
#print axioms QCore.CastRepresentation.pointer_to_boolean_is_the_null_test
#print axioms QCore.CastRepresentation.pointer_value_keeps_identity
#print axioms QCore.CastRepresentation.unlisted_cast_refused
#print axioms QCore.CastRepresentation.refused_row_refused
#print axioms QCore.CastRepresentation.constant_type_unevaluated
#print axioms QCore.CastRepresentation.alignof_never_evaluated
#print axioms QCore.CastRepresentation.unevaluated_operand_has_no_events
#print axioms QCore.CastRepresentation.evaluated_operand_once
#print axioms QCore.CastRepresentation.evaluated_effect_refused
#print axioms QCore.CastRepresentation.consumed_value_is_clangs
#print axioms QCore.CastRepresentation.marker_never_consumed
#print axioms QCore.CastRepresentation.constant_vectors_consumed
#print axioms QCore.CastRepresentation.non_typedef_non_tag_never_folds
