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

import QCore.MonkeyLinkedSelection

-- Card CURRENT-MONKEY-LINKED-CONFIGURATION: the current route's Monkey units
-- under --monkey-linked. Model of `linked_monkey` and the not-linked task in
-- `crates/qcore-audit/src/current_route.rs`. Which files the Monkey unit rule
-- and the located-path selection pick is a Rust boundary pinned by
-- `a_linked_monkey_run_replaces_or_keeps_every_standalone_unit`.
-- Record: `docs/reports/current-monkey-linked-configuration/README.md`.

-- Nothing vanishes from the denominator.
#print axioms QCore.MonkeyLinkedSelection.every_linked_unit_is_analysed
#print axioms QCore.MonkeyLinkedSelection.every_standalone_unit_is_accounted
#print axioms QCore.MonkeyLinkedSelection.without_a_standalone_database_only_linked_units

-- Exactly one reading per file; the standalone configuration is never analysed.
#print axioms QCore.MonkeyLinkedSelection.not_linked_iff
#print axioms QCore.MonkeyLinkedSelection.rows_carry_their_configuration
#print axioms QCore.MonkeyLinkedSelection.no_file_is_read_twice
#print axioms QCore.MonkeyLinkedSelection.no_standalone_unit_is_analysed
#print axioms QCore.MonkeyLinkedSelection.analysed_rows_are_exactly_the_linked_units

-- Keys name one configuration (the link step's condition 11).
#print axioms QCore.MonkeyLinkedSelection.keys_name_one_configuration

-- A not-linked unit is Unknown and supplies nothing to the definition census.
#print axioms QCore.MonkeyLinkedSelection.a_not_linked_unit_is_unknown
#print axioms QCore.MonkeyLinkedSelection.a_not_linked_unit_is_never_safe
#print axioms QCore.MonkeyLinkedSelection.a_not_linked_unit_defines_nothing
#print axioms QCore.MonkeyLinkedSelection.a_not_linked_unit_never_makes_the_census_incomplete
#print axioms QCore.MonkeyLinkedSelection.an_unscanned_analysed_unit_makes_the_census_incomplete

-- Vectors and non-vacuity.
#print axioms QCore.MonkeyLinkedSelection.fixture_vector
#print axioms QCore.MonkeyLinkedSelection.census_shape_vector
#print axioms QCore.MonkeyLinkedSelection.not_linked_is_inhabited
