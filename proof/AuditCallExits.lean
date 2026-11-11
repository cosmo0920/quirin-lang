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

import QCore.CallExits

/-! Audited companion to CG21-CALL-EXITS-AND-BOUNDARIES (gateway rule 11o). -/
#print axioms QCore.CallExits.decision_vectors_hold
#print axioms QCore.CallExits.path_end_iff
#print axioms QCore.CallExits.attribute_alone_not_path_end
#print axioms QCore.CallExits.declaration_alone_not_path_end
#print axioms QCore.CallExits.path_end_no_exit
#print axioms QCore.CallExits.path_end_rest_unchecked
#print axioms QCore.CallExits.path_end_checks_pre
#print axioms QCore.CallExits.refused_no_claim
#print axioms QCore.CallExits.model_sound
#print axioms QCore.CallExits.nonlocal_path_end_hides_leak
#print axioms QCore.CallExits.boundaries_refused
#print axioms QCore.CallExits.boundary_table_refused
