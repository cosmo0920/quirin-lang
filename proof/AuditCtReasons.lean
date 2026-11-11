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

import QCore.CtReasons

-- Card `GEN-CTREASONS` (follow-on `D02-GEN-CTREASONS`): the CT ledger's
-- `newly_decidable_not_mapped_reasons` map, derived rather than carried. Model
-- of `scripts/ct_stage_summary.py`'s `newly_decidable_ids` /
-- `newly_decidable_reasons` / `check_matrix_ct_reasons` and the copy of the
-- first two that `docs/reports/remaining-d02/build_matrix.py` emits. That the
-- two implementations compute these functions is a transcription premise; the
-- generator writing the key and the checker refusing a hand edit to it in both
-- directions is where that premise is checked.

-- The published keys are the companion list, and every one of them is a case
-- the manifest currently leaves unmapped. Keying the map to the list is what
-- makes both recorded staleness classes impossible: a paragraph about a mapped
-- case (HYGIENE-16's CT39 row) and a paragraph belonging to no published claim
-- (this card's CT11 row).
#print axioms QCore.CtReasons.keys_are_the_published_list
#print axioms QCore.CtReasons.every_selected_case_is_unmapped
#print axioms QCore.CtReasons.select_drops_a_mapped_case

-- Total-or-refuse, and the resolution rule the text comes from: a lane row wins
-- for the case it names, the archive decides a case no lane measured, and a case
-- neither classifies is a hole rather than an empty paragraph.
#print axioms QCore.CtReasons.an_unclassified_case_refuses
#print axioms QCore.CtReasons.a_lane_reason_wins
#print axioms QCore.CtReasons.the_archive_reason_decides_an_unmeasured_case

-- The drift check, both directions, plus its non-vacuity: a committed row the
-- derivation cannot produce is reported, a derived row the file drops is
-- reported, and equal key lists report nothing.
#print axioms QCore.CtReasons.a_row_outside_the_derivation_is_reported
#print axioms QCore.CtReasons.a_missing_derived_row_is_reported
#print axioms QCore.CtReasons.equal_key_lists_report_nothing
#print axioms QCore.CtReasons.mem_iff

-- The transcribed instance, decided rather than argued: the derived list at this
-- card's base commit, the CT11 row refused by name, HYGIENE-16's CT39 row
-- refused by the same rule, the reconciled map reporting nothing, and mapping a
-- case retiring its row with nobody editing the map.
#print axioms QCore.CtReasons.the_derived_list_is_ct24_and_ct47
#print axioms QCore.CtReasons.the_ct11_row_is_refused
#print axioms QCore.CtReasons.the_ct39_row_would_be_refused
#print axioms QCore.CtReasons.the_committed_map_after_this_card_reports_nothing
#print axioms QCore.CtReasons.mapping_ct24_retires_its_row
