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

import QCore.RecordObjectPlace

/-! Audited companion to CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (gateway rule 12j). -/

#print axioms QCore.RecordObjectPlace.decision_vectors_hold
#print axioms QCore.RecordObjectPlace.admitted_exactly_on_a_dot_only_scalar_place_of_automatic_storage
#print axioms QCore.RecordObjectPlace.a_hop_is_never_admitted
#print axioms QCore.RecordObjectPlace.a_root_outside_automatic_storage_is_refused
#print axioms QCore.RecordObjectPlace.a_non_scalar_access_is_refused
#print axioms QCore.RecordObjectPlace.the_never_bound_answer_is_unchanged
#print axioms QCore.RecordObjectPlace.the_rule_only_adds
#print axioms QCore.RecordObjectPlace.an_admitted_place_has_no_obligation
#print axioms QCore.RecordObjectPlace.a_scalar_read_changes_no_binding
#print axioms QCore.RecordObjectPlace.a_scalar_store_changes_no_binding
#print axioms QCore.RecordObjectPlace.a_union_store_keeps_the_frame
#print axioms QCore.RecordObjectPlace.the_frame_adds_no_target
