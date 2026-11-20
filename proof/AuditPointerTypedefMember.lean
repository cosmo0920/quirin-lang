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

import QCore.PointerTypedefMember

/-! Audited companion to CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3 (gateway rule 68). -/

#print axioms QCore.PointerTypedefMember.an_agreeing_alias_member_is_its_pointer_spelling
#print axioms QCore.PointerTypedefMember.a_disagreeing_alias_reads_nothing
#print axioms QCore.PointerTypedefMember.a_pointer_to_the_alias_is_never_single
#print axioms QCore.PointerTypedefMember.an_unresolved_pointee_is_never_single
#print axioms QCore.PointerTypedefMember.decision_vectors_hold
#print axioms QCore.PointerTypedefMember.the_decision_is_the_models
