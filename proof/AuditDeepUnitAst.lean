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

import QCore.DeepUnitAst

/-! Audited companion to RDHTTP-HEADER-SUMMARY-GAP: a whole-unit typed AST nesting beyond serde_json's default limit is read within an admitted bound (rule 13o); beyond it the unit keeps no index and its examined bodies stay unvalidated in the fold (rule 13p). -/

#print axioms QCore.DeepUnitAst.an_index_read_before_is_read_after
#print axioms QCore.DeepUnitAst.the_index_is_the_document
#print axioms QCore.DeepUnitAst.beyond_the_bound_there_is_no_index
#print axioms QCore.DeepUnitAst.only_the_deep_band_changes
#print axioms QCore.DeepUnitAst.a_unit_without_an_index_keeps_the_declaration_unvalidated
#print axioms QCore.DeepUnitAst.the_fold_never_skips_a_unit
#print axioms QCore.DeepUnitAst.depth_vectors_hold
#print axioms QCore.DeepUnitAst.band_edges
