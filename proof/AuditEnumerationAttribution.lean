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

import QCore.EnumerationAttribution

-- Unit `E01-TAIL`, residual `E01-F1-STDIN`: which text of a line-marked
-- preprocessing belongs to the audited main source. Model of the attribution
-- rule in `crates/qcore-c-gateway/src/legacy_enumeration.rs`
-- (`tokenize_marked`, `inventory_from_marked`). Clang's own marker flags are a
-- tool boundary, pinned behaviourally by
-- `crates/qcore-c-gateway/tests/e01_stdin_attribution.rs` and the reader-level
-- matrix in `crates/qcore-c-gateway/tests/unit/legacy_enumeration.rs`.
-- Record: `docs/reports/remaining-e01-tail/README.md`.

-- `#line` is inert for attribution: a flagless marker renames and nothing more.
#print axioms QCore.EnumerationAttribution.relabel_preserves_depth
#print axioms QCore.EnumerationAttribution.relabel_preserves_breakage
#print axioms QCore.EnumerationAttribution.enter_then_leave_restores_depth

-- The two measured directions that separate depth attribution from E01-F1's
-- name attribution, and the non-vacuity of the separation.
#print axioms QCore.EnumerationAttribution.a_relabelled_header_is_not_main_source
#print axioms QCore.EnumerationAttribution.name_attribution_counted_the_relabelled_header
#print axioms QCore.EnumerationAttribution.a_relabelled_tail_is_main_source
#print axioms QCore.EnumerationAttribution.name_attribution_dropped_the_relabelled_tail
#print axioms QCore.EnumerationAttribution.the_two_rules_are_different

-- The E01-F1-STDIN sentence: the audited scope does not depend on how the
-- transport spells the primary input, and name attribution had no such property.
#print axioms QCore.EnumerationAttribution.rename_preserves_depths
#print axioms QCore.EnumerationAttribution.depth_attribution_is_transport_independent
#print axioms QCore.EnumerationAttribution.name_attribution_is_not_transport_independent

-- Clang's measured preamble, and a forced `-include`, stay out of scope.
#print axioms QCore.EnumerationAttribution.the_preamble_contributes_no_main_source_token

-- Fail-closed on an unreadable include stack, with the well-formed control.
#print axioms QCore.EnumerationAttribution.an_underflowing_stream_is_unbalanced
#print axioms QCore.EnumerationAttribution.an_unclosed_stream_is_unbalanced
#print axioms QCore.EnumerationAttribution.a_well_formed_stream_is_balanced
#print axioms QCore.EnumerationAttribution.breakage_is_absorbing

-- §L21.1 rule 1 over the new refusal ground.
#print axioms QCore.EnumerationAttribution.the_unbalanced_ground_says_nothing_about_the_source
#print axioms QCore.EnumerationAttribution.the_two_subjects_are_distinct
