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

import QCore.NullableReferenceDrop

/-! Audited companion to REFERENCE-DROP-NULL-ACCEPTING: the NULL-accepting reference drop (gateway rule 13b). -/

#print axioms QCore.NullableReferenceDrop.a_plain_actual_decides_as_today
#print axioms QCore.NullableReferenceDrop.a_non_nullable_drop_refuses_a_possibly_null_actual
#print axioms QCore.NullableReferenceDrop.a_nullable_drop_of_an_unheld_reference_is_refused
#print axioms QCore.NullableReferenceDrop.a_nullable_drop_of_null_is_a_no_op
#print axioms QCore.NullableReferenceDrop.only_the_null_edge_is_a_no_op
#print axioms QCore.NullableReferenceDrop.the_one_or_null_edge_is_the_family_drop
#print axioms QCore.NullableReferenceDrop.a_final_nullable_drop_is_the_last_reference
#print axioms QCore.NullableReferenceDrop.the_null_edge_changes_nothing
#print axioms QCore.NullableReferenceDrop.the_null_edge_discharges_and_creates_nothing
#print axioms QCore.NullableReferenceDrop.the_non_null_edge_is_the_drop
#print axioms QCore.NullableReferenceDrop.nullable_drop_traces
#print axioms QCore.NullableReferenceDrop.nullable_drop_vectors_hold
