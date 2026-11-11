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

import QCore.RelationalPost

/-! Audited companion to CG23-RELATIONAL-POSTCONDITIONS (gateway rule 11r). -/

#print axioms QCore.RelationalPost.meets_sound
#print axioms QCore.RelationalPost.meets_complete
#print axioms QCore.RelationalPost.holds_iff
#print axioms QCore.RelationalPost.neg_sat
#print axioms QCore.RelationalPost.every_value_reaches_one_edge
#print axioms QCore.RelationalPost.test_shape_is_exact
#print axioms QCore.RelationalPost.select_sound
#print axioms QCore.RelationalPost.the_edge_disposition_is_the_returned_class
#print axioms QCore.RelationalPost.join_sound
#print axioms QCore.RelationalPost.join_refuses_disagreement
#print axioms QCore.RelationalPost.a_validated_summary_selects_each_exit_disposition
#print axioms QCore.RelationalPost.check_sound
#print axioms QCore.RelationalPost.withdrawn_selects_nothing
#print axioms QCore.RelationalPost.meets_vectors_hold
#print axioms QCore.RelationalPost.select_vectors_hold
