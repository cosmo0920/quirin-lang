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

import QCore.BucketTally

-- Card `HYGIENE-16` (findings `D02-BUCKETS-STALE`, `D02-BUCKETS-LABELS`,
-- `CT-LABELS-T00-REVERT`): the CT ledger's label-resolution and bucket-tally
-- relation. Model of `scripts/ct_stage_summary.py`'s `unmapped_labels` /
-- `unmapped_bucket_counts` and `compiler/tests/ct_corpus.rs`'s copies of the
-- same two functions. That the two implementations compute these functions is a
-- transcription premise; the two derivations refuse each other's drift against
-- the committed acceptance record, which is where that premise is checked.

-- The tally accounts for every label it is given, and therefore for every case
-- the manifest currently leaves unmapped. This is the property whose silent
-- violation was the finding: a hand-kept mirror summing 22 against 18.
#print axioms QCore.BucketTally.total_bump
#print axioms QCore.BucketTally.total_tally
#print axioms QCore.BucketTally.resolve_length
#print axioms QCore.BucketTally.tally_total_is_the_case_count

-- Why making the counted set the manifest's own unmapped set is sufficient:
-- mapping a case moves the published total without anyone editing the tally.
#print axioms QCore.BucketTally.dropping_a_case_drops_the_total

-- Total-or-refuse: an unclassified case is a hole, never a zero.
#print axioms QCore.BucketTally.an_unclassified_case_refuses

-- The resolution rule, both halves: a lane row wins for the case it names, and
-- the archive decides a case no lane measured.
#print axioms QCore.BucketTally.a_lane_row_wins
#print axioms QCore.BucketTally.the_archive_decides_an_unmeasured_case

-- Card `CT-LABELS-T00-REVERT`'s soundness argument: restoring card T00's frozen
-- archive to its pre-CTWIN bytes is invisible to the published rows, and so to
-- the published counts, because every case the two archives disagree on is a
-- case some CT lane measured.
#print axioms QCore.BucketTally.archive_edits_under_lane_coverage_are_invisible
#print axioms QCore.BucketTally.archive_edits_under_lane_coverage_move_no_count

-- The same three cases, decided on the transcribed instance rather than argued,
-- with the non-vacuity companion: an unmeasured case still reads its label out
-- of the archive, so the invisibility above is the resolution rule at work.
#print axioms QCore.BucketTally.the_ctwin_restoration_publishes_the_same_rows
#print axioms QCore.BucketTally.the_ctwin_restoration_publishes_the_same_tally
#print axioms QCore.BucketTally.the_archive_still_decides_ct44
#print axioms QCore.BucketTally.the_transcribed_instance_accounts_for_every_case
