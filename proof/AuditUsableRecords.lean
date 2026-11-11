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

import QCore.UsableRecords

-- Card `W48-QR-USABLE-RECORDS`: field writes, nested paths, records across
-- calls, and FFI records kept in FFI files.

-- A field write changes exactly that field.
#print axioms QCore.UsableRecords.lookup_map_other
#print axioms QCore.UsableRecords.lookup_map_same
#print axioms QCore.UsableRecords.write_changes_exactly_that_field
#print axioms QCore.UsableRecords.write_leaves_every_other_path

-- A nested path addresses exactly one sub-object, in the tree and in the
-- flattened layout.
#print axioms QCore.UsableRecords.a_nested_path_addresses_exactly_one_sub_object
#print axioms QCore.UsableRecords.a_path_through_a_leaf_addresses_nothing
#print axioms QCore.UsableRecords.the_sample_write_is_seen_only_at_its_path
#print axioms QCore.UsableRecords.start_succ
#print axioms QCore.UsableRecords.start_mono
#print axioms QCore.UsableRecords.sibling_slices_are_disjoint

-- The nested aggregate: composition conserves custody, the drop-through over a
-- tree releases exactly the held leaves, and no two owned leaves share a path.
#print axioms QCore.UsableRecords.under_held
#print axioms QCore.UsableRecords.nested_composition_conserves_custody
#print axioms QCore.UsableRecords.drop_through_tree_releases_exactly_the_live_leaves
#print axioms QCore.UsableRecords.under_paths
#print axioms QCore.UsableRecords.under_keeps_paths_distinct
#print axioms QCore.UsableRecords.under_path_head
#print axioms QCore.UsableRecords.compose_path_head
#print axioms QCore.UsableRecords.compose_keeps_paths_distinct

-- The refill is admitted exactly into a hole and moves custody.
#print axioms QCore.UsableRecords.a_refill_is_admitted_exactly_into_a_hole
#print axioms QCore.UsableRecords.a_refill_moves_custody

-- Custody is conserved across a record-passing call.
#print axioms QCore.UsableRecords.an_admitted_record_argument_owns_nothing
#print axioms QCore.UsableRecords.a_record_passing_call_conserves_custody
#print axioms QCore.UsableRecords.a_record_that_owns_is_never_passed

-- An FFI record never leaves an FFI file without the assumption.
#print axioms QCore.UsableRecords.an_admitted_ffi_bundle_marks_every_function_file
#print axioms QCore.UsableRecords.every_ffi_bundle_result_carries_the_assumption
#print axioms QCore.UsableRecords.an_unmarked_function_file_is_refused
