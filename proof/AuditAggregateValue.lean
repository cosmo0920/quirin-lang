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

import QCore.AggregateValue

/-! Audited companion to CG14-AGGREGATE-VALUES-AND-INITIALIZERS (gateway rule 11f). -/

-- The decision tables, bound to the Rust tables row by row.
#print axioms QCore.AggregateValue.copy_vectors_hold
#print axioms QCore.AggregateValue.leaf_vectors_hold
#print axioms QCore.AggregateValue.literal_vectors_hold
#print axioms QCore.AggregateValue.copy_admitted_exactly_when
#print axioms QCore.AggregateValue.literal_admitted_exactly_when
#print axioms QCore.AggregateValue.leaf_admitted_exactly_when

-- Field-wise copy preserves custody: aliases, never duplicate owners.
#print axioms QCore.AggregateValue.copy_adds_no_owner
#print axioms QCore.AggregateValue.a_copy_only_aliases
#print axioms QCore.AggregateValue.custody_free_has_no_identity
#print axioms QCore.AggregateValue.a_custody_free_copy_moves_no_identity
#print axioms QCore.AggregateValue.releasing_both_copies_is_a_double_release
#print axioms QCore.AggregateValue.the_owned_field_model_agrees

-- Zero-fill yields no identity; an admitted initializer holds none.
#print axioms QCore.AggregateValue.zero_fill_has_no_identity
#print axioms QCore.AggregateValue.admitted_leaf_denotes_no_identity
#print axioms QCore.AggregateValue.an_unadmitted_leaf_may_carry_an_identity
#print axioms QCore.AggregateValue.an_admitted_initializer_holds_no_identity

-- A compound literal's lifetime bounds its borrows.
#print axioms QCore.AggregateValue.a_literal_borrow_ends_in_its_block
#print axioms QCore.AggregateValue.a_literal_never_escapes

-- Unlisted forms are refused; string extents are facts, not bounds.
#print axioms QCore.AggregateValue.an_unlisted_form_refuses
#print axioms QCore.AggregateValue.the_terminator_fact
