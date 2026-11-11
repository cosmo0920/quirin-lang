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

import QCore.ReferenceResult

/-! Audited companion to REFCOUNT-REFERENCE-RETURNING-DECLARATIONS: returned references and result holders (gateway rules 13d, 13e). -/

#print axioms QCore.ReferenceResult.admitted_of_validated
#print axioms QCore.ReferenceResult.a_validated_body_never_returns_a_formal
#print axioms QCore.ReferenceResult.a_validated_body_returns_only_obtained_references
#print axioms QCore.ReferenceResult.a_validated_non_null_body_never_returns_null
#print axioms QCore.ReferenceResult.a_withdrawn_declaration_is_never_safe
#print axioms QCore.ReferenceResult.body_vectors_hold
#print axioms QCore.ReferenceResult.a_jump_withdraws
#print axioms QCore.ReferenceResult.a_result_holder_released_first_is_safe
#print axioms QCore.ReferenceResult.a_result_holder_dropped_first_is_safe
#print axioms QCore.ReferenceResult.a_result_holder_handed_on_is_never_safe
#print axioms QCore.ReferenceResult.a_borrowed_familys_last_drop_keeps_its_records
#print axioms QCore.ReferenceResult.a_result_holder_without_a_tracked_result_is_undecided
#print axioms QCore.ReferenceResult.an_object_released_while_its_handle_holds_it_is_refused
#print axioms QCore.ReferenceResult.a_predefined_identifier_outlives_every_holder
#print axioms QCore.ReferenceResult.a_transferred_reference_leaves_the_callers_family
#print axioms QCore.ReferenceResult.a_drop_after_a_transfer_is_an_over_drop
#print axioms QCore.ReferenceResult.a_use_after_a_transfer_is_refused
