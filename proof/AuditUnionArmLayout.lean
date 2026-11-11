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

import QCore.UnionArmLayout

/-! Audited companion to RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION (gateway rule 12n, amended): a union's places are separated by Clang's record layout. A write over bytes disjoint from a read's never changes the read; two places the rule keeps apart are known and disjoint, so a store into one never changes a read of the other; the whole union, a place whose bytes are not given and a union two units lay out differently alias every place; the same arm path always aliases and aliasing is symmetric; the layout only narrows Q3's sharing, never adding a store a member takes. UNION-OPAQUE-RECORD-WRITES-BY-TYPE: an opaque write of a record is keyed by its target; the target reading only narrows the type reading, an untargeted write is read by type, a separate object and a path below a record the union does not hold never key into the union, a write into the union is decided by its own place, a write into a separate object or a non-overlapping one never changes the read, and an unknown range aliases. -/

#print axioms QCore.UnionArmLayout.a_disjoint_write_never_changes_the_read
#print axioms QCore.UnionArmLayout.separated_places_are_disjoint
#print axioms QCore.UnionArmLayout.a_separated_store_never_changes_the_read
#print axioms QCore.UnionArmLayout.an_unplaced_place_aliases
#print axioms QCore.UnionArmLayout.an_ungiven_range_aliases
#print axioms QCore.UnionArmLayout.the_same_arm_aliases
#print axioms QCore.UnionArmLayout.places_alias_is_symmetric
#print axioms QCore.UnionArmLayout.disagreeing_units_unplace
#print axioms QCore.UnionArmLayout.a_disagreeing_union_aliases_everything
#print axioms QCore.UnionArmLayout.the_layout_takes_only_shared_stores
#print axioms QCore.UnionArmLayout.layout_vectors_hold
#print axioms QCore.UnionArmLayout.the_target_reading_only_narrows_the_type_reading
#print axioms QCore.UnionArmLayout.an_untargeted_write_is_read_by_type
#print axioms QCore.UnionArmLayout.a_separate_object_never_keys_into_the_union
#print axioms QCore.UnionArmLayout.a_write_below_an_unheld_record_never_keys_into_the_union
#print axioms QCore.UnionArmLayout.a_union_write_is_decided_by_its_place
#print axioms QCore.UnionArmLayout.a_separate_write_never_changes_the_read
#print axioms QCore.UnionArmLayout.a_non_overlapping_opaque_write_never_changes_the_read
#print axioms QCore.UnionArmLayout.an_unknown_range_opaque_write_aliases
#print axioms QCore.UnionArmLayout.opaque_vectors_hold
