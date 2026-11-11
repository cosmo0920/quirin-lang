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

import QCore.SourceCoordinate

-- Card `GATEWAY-C-LOCATION`: the presented original coordinate of an
-- existing-C gateway diagnostic. Model of the presentation reader in
-- `crates/qcore-c-gateway/src/legacy_location.rs` (`walk_marked`,
-- `map_from_marked`). The audited *scope* rule is unchanged and lives in
-- `QCore.EnumerationAttribution`. Clang's own marker flags are a tool
-- boundary, pinned behaviourally by
-- `crates/qcore-c-gateway/tests/gateway_c_location.rs` and the reader-level
-- matrix in `crates/qcore-c-gateway/tests/unit/legacy_location.rs`.
-- Record: `docs/reports/gateway-c-location/README.md`.

-- The anti-forgery theorem and the two lemmas it rests on: a spelling becomes
-- presentable only where a marker ENTERED a file, and a content line presents
-- only a spelling already on the stack.
#print axioms QCore.SourceCoordinate.mem_stackNames_cons
#print axioms QCore.SourceCoordinate.step_adds_only_an_entering_spelling
#print axioms QCore.SourceCoordinate.content_keeps_the_stack_names
#print axioms QCore.SourceCoordinate.content_presents_a_stack_name
#print axioms QCore.SourceCoordinate.presented_files_are_entered_files
#print axioms QCore.SourceCoordinate.a_flagless_marker_names_no_presented_file

-- Include depth 0 is named by the caller, never by the opening marker.
#print axioms QCore.SourceCoordinate.the_primary_frame_presents_no_marker_spelling

-- The two measured `#line` directions withdraw the coordinate rather than
-- renaming it, and the audited scope of the same stream is unmoved.
#print axioms QCore.SourceCoordinate.a_relabelled_header_presents_nothing
#print axioms QCore.SourceCoordinate.a_relabelled_tail_stays_in_scope_and_presents_nothing

-- Non-vacuity: an agreeing flagless marker still resynchronises, which is a
-- `#line` renumbering the file it is written in (C11 6.10.4).
#print axioms QCore.SourceCoordinate.an_agreeing_resync_sets_the_line
#print axioms QCore.SourceCoordinate.the_name_comparison_is_load_bearing

-- Why a macro expansion presents its expansion site.
#print axioms QCore.SourceCoordinate.content_lines_advance_by_one

-- Fail-closed, with the well-formed control.
#print axioms QCore.SourceCoordinate.content_before_the_opening_marker_presents_nothing
#print axioms QCore.SourceCoordinate.an_underflowing_return_breaks_the_reader
#print axioms QCore.SourceCoordinate.a_well_formed_return_does_not_break_the_reader

-- L21.1 rule 1 over the four refusal grounds.
#print axioms QCore.SourceCoordinate.every_location_refusal_says_nothing_about_the_source
#print axioms QCore.SourceCoordinate.the_environment_subject_is_not_the_source_subject
