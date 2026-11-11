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

import QCore.ScalarPositions
#print axioms QCore.ScalarPositions.fieldValue_lit_iff
#print axioms QCore.ScalarPositions.fieldValue_lit_value
#print axioms QCore.ScalarPositions.fieldValue_name_same_type
#print axioms QCore.ScalarPositions.fieldValue_bool_iff
#print axioms QCore.ScalarPositions.bool_values_fit
#print axioms QCore.ScalarPositions.fieldValue_in_range
#print axioms QCore.ScalarPositions.construct_well_typed
#print axioms QCore.ScalarPositions.project_well_typed
#print axioms QCore.ScalarPositions.i32Operand_type
#print axioms QCore.ScalarPositions.i32Operand_refuses_other_types
#print axioms QCore.ScalarPositions.convertStmt_well_typed
#print axioms QCore.ScalarPositions.convertStmt_succeeds_iff
#print axioms QCore.ScalarPositions.no_target_no_field
#print axioms QCore.ScalarPositions.fieldAgreement_holds
#print axioms QCore.ScalarPositions.decideBinary_known_iff
#print axioms QCore.ScalarPositions.decideBinary_refusal_is_raise
#print axioms QCore.ScalarPositions.decideBinary_raises_needs_permission
#print axioms QCore.ScalarPositions.decideBinary_without_permission
#print axioms QCore.ScalarPositions.decideBinary_known_in_range
#print axioms QCore.ScalarPositions.argumentAdmitted_type
#print axioms QCore.ScalarPositions.decideCompare_bool_is_bool
#print axioms QCore.ScalarPositions.externAgreement_holds
