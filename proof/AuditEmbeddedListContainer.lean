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

import QCore.EmbeddedListContainer

/-! Audited companion to CG10-EMBEDDED-LIST-BORROWED-CONTAINER (gateway rule 69). -/

#print axioms QCore.EmbeddedListContainer.a_borrowed_only_helper_changes_no_custody
#print axioms QCore.EmbeddedListContainer.a_releasing_helper_is_not_a_noop
#print axioms QCore.EmbeddedListContainer.the_noop_reading_misses_a_use_after_release
#print axioms QCore.EmbeddedListContainer.an_unknown_registration_decides_no_destroy
#print axioms QCore.EmbeddedListContainer.an_unknown_registration_decides_no_add
#print axioms QCore.EmbeddedListContainer.an_element_added_to_an_undecided_container_has_no_one_outcome
#print axioms QCore.EmbeddedListContainer.container_vectors_hold
#print axioms QCore.EmbeddedListContainer.a_bare_frame_holder_is_not_embedded
#print axioms QCore.EmbeddedListContainer.a_non_formal_root_is_not_embedded
#print axioms QCore.EmbeddedListContainer.only_the_gate_refusal_is_read
#print axioms QCore.EmbeddedListContainer.the_result_is_the_gate_refusal_or_a_container_refusal
#print axioms QCore.EmbeddedListContainer.the_writes_are_kept
#print axioms QCore.EmbeddedListContainer.the_rule_never_admits
#print axioms QCore.EmbeddedListContainer.the_verdict_of_every_node_is_unchanged
#print axioms QCore.EmbeddedListContainer.the_status_of_a_function_is_unchanged
#print axioms QCore.EmbeddedListContainer.no_embedded_actual_keeps_the_gate_refusal
#print axioms QCore.EmbeddedListContainer.an_excluded_statement_keeps_the_gate_refusal
#print axioms QCore.EmbeddedListContainer.an_admitting_reading_keeps_the_gate_refusal
#print axioms QCore.EmbeddedListContainer.a_refusal_about_another_place_keeps_the_gate_refusal
