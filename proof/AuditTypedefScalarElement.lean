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

import QCore.TypedefScalarElement

/-! Audited companion to CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4 (gateway rule 66). -/

#print axioms QCore.TypedefScalarElement.an_agreeing_scalar_typedef_is_its_spelling
#print axioms QCore.TypedefScalarElement.a_disagreeing_typedef_reads_nothing
#print axioms QCore.TypedefScalarElement.a_non_scalar_resolution_reads_nothing
#print axioms QCore.TypedefScalarElement.a_non_member_is_never_read
#print axioms QCore.TypedefScalarElement.decision_vectors_hold
#print axioms QCore.TypedefScalarElement.the_decision_is_the_models
