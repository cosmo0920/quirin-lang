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

import QCore.SingleEmissionRoute

-- Card `TRACE-STDIO`: the single emission route's decision procedure.
--
-- The migration itself — four synthesized C backends and `source.std-io.v1`
-- moved onto one C-level printing route with byte-identical output — is
-- established by measurement, not by this model. What is proved here is that
-- the procedure reporting "no second route" is sound, complete and names the
-- right spelling.

-- The substring decision, correct against the standard relations. Everything
-- below rests on these two.
#print axioms QCore.SingleEmissionRoute.prefixOf_iff
#print axioms QCore.SingleEmissionRoute.containsInfix_iff

-- The spelling table is ordered by non-increasing length, and every entry is
-- non-empty; both decided, so reordering the table breaks the build rather
-- than silently weakening maximality.
#print axioms QCore.SingleEmissionRoute.spellings_sorted_by_decreasing_length
#print axioms QCore.SingleEmissionRoute.every_spelling_is_nonempty

-- Soundness and completeness of the per-line scan.
#print axioms QCore.SingleEmissionRoute.scanLine_none_iff
#print axioms QCore.SingleEmissionRoute.scanLine_some_contains
#print axioms QCore.SingleEmissionRoute.scanLine_some_mem
#print axioms QCore.SingleEmissionRoute.a_direct_call_is_always_reported

-- Maximality: the reported spelling is a longest one the line contains. The
-- containing pairs that make this necessary are decided, not asserted.
#print axioms QCore.SingleEmissionRoute.find?_sorted_is_maximal
#print axioms QCore.SingleEmissionRoute.the_reported_spelling_is_maximal
#print axioms QCore.SingleEmissionRoute.the_containing_pairs_are_really_present
#print axioms QCore.SingleEmissionRoute.an_fprintf_line_is_reported_as_fprintf
#print axioms QCore.SingleEmissionRoute.an_fputs_line_is_reported_as_fputs

-- The whole-program procedure: an exact characterization of "no violation",
-- in both directions, and its composition over concatenated fragments.
#print axioms QCore.SingleEmissionRoute.empty_means_no_direct_call
#print axioms QCore.SingleEmissionRoute.no_direct_call_means_empty
#print axioms QCore.SingleEmissionRoute.violation_free_iff
#print axioms QCore.SingleEmissionRoute.concatenation_is_violation_free_iff_both_are

-- Exemption is whole-line identity: the prelude's own lines are exempt, a
-- look-alike macro is not, a routed write is not a violation, and reported
-- positions are the one-based positions a reader will find in the file.
#print axioms QCore.SingleEmissionRoute.the_prelude_definitions_are_exempt
#print axioms QCore.SingleEmissionRoute.a_look_alike_line_is_not_exempt
#print axioms QCore.SingleEmissionRoute.a_routed_write_is_not_a_violation
#print axioms QCore.SingleEmissionRoute.positions_are_one_based_and_count_every_line

-- The two-sided tie the Rust test reproduces independently.
#print axioms QCore.SingleEmissionRoute.domain_has_eleven_lines
#print axioms QCore.SingleEmissionRoute.the_domain_outcomes_are_pinned
