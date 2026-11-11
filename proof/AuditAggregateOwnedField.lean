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

import QCore.AggregateOwnedField

-- Card `AGGREGATE-OWNED-FIELD` (wave 39): the owned record field, and the
-- four rules constraint 15k recorded as missing, landed in the path-extended
-- cycle-3 ownership model.

-- The ledger keyed by place: a release removes exactly its own resource and
-- keeps every other, and a move re-owns without changing quantity.
#print axioms QCore.AggregateOwnedField.release_removes_exactly_its_own
#print axioms QCore.AggregateOwnedField.release_keeps_every_other
#print axioms QCore.AggregateOwnedField.move_preserves_length

-- Rule 1, construction moves in: each source resource is owned by its field
-- afterwards, the live quantity is unchanged, and the aggregate holds exactly
-- the sources' resources in declaration order.
#print axioms QCore.AggregateOwnedField.compose_moves_each_source_in
#print axioms QCore.AggregateOwnedField.compose_two_moves_both_in
#print axioms QCore.AggregateOwnedField.compose_moves_no_quantity
#print axioms QCore.AggregateOwnedField.compose_holds_the_sources

-- Rule 2, the partial move: a move out leaves a hole, a hole refuses a second
-- move, and every other field is untouched.
#print axioms QCore.AggregateOwnedField.move_out_leaves_a_hole
#print axioms QCore.AggregateOwnedField.a_hole_refuses_a_second_move
#print axioms QCore.AggregateOwnedField.move_out_keeps_the_other_fields

-- Rule 3, drop-through-aggregate — the headline: a whole-record drop releases
-- exactly the live owned fields, in declaration order, nothing twice, a hole
-- tolerated, and the released resources leave the ledger.
#print axioms QCore.AggregateOwnedField.dropThrough_released
#print axioms QCore.AggregateOwnedField.drop_through_releases_exactly_the_live_fields
#print axioms QCore.AggregateOwnedField.drop_through_releases_nothing_twice
#print axioms QCore.AggregateOwnedField.drop_through_tolerates_a_hole
#print axioms QCore.AggregateOwnedField.drop_through_is_in_declaration_order
#print axioms QCore.AggregateOwnedField.drop_through_releases_from_the_ledger

-- Rule 4, the leak rule that sees into the aggregate: an unreleased field is a
-- leak named by its path, and the whole-record drop discharges it.
#print axioms QCore.AggregateOwnedField.an_unreleased_field_is_a_leak_named_by_its_path
#print axioms QCore.AggregateOwnedField.drop_through_discharges_the_leak

-- The copy rule: never, by type, hollow or not.
#print axioms QCore.AggregateOwnedField.an_aggregate_is_never_copied
#print axioms QCore.AggregateOwnedField.a_hollow_aggregate_is_still_not_copied

-- Type agreement at construction, as one equivalence with both refusing
-- directions.
#print axioms QCore.AggregateOwnedField.agreement_is_exactly_kind_for_kind
#print axioms QCore.AggregateOwnedField.a_scalar_at_a_bytes_field_is_refused
#print axioms QCore.AggregateOwnedField.a_buffer_at_an_i32_field_is_refused

-- Neutrality: a record of scalars composes nothing, so every program written
-- before the card is unchanged by it.
#print axioms QCore.AggregateOwnedField.a_record_of_scalars_composes_nothing
