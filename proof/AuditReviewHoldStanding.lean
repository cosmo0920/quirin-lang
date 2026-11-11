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

import QCore.ReviewHoldStanding

/-! Card **W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL**: a call held for a
human act (awaiting review, or a pin mismatch) never masks a reason that would
stand without the hold (gateway rule 10i as amended, spec §L21.2).

Four directions are audited.

**The published verdict.** A `Pending` row is joined with the capped verdict
of its function's analysis with every held declaration in force; the result is
never below the held verdict, moves only a `Pending` row, never reaches `Safe`
from a held row and never `Unsafe` through the second analysis.

**Pending only from the hold.** A published `Pending` row has every reason of
both analyses `Safe` or `Pending`, and a held link for a human-resolvable
reason; a held call beside a standing refusal and an
alleged violation of the accepted declaration are `Unknown`.

**A held call never removes another reason.** On a row the hold touched,
every reason of the second analysis, capped, is below the published verdict,
and so is that analysis' whole verdict; the fold `settle` computes over the
held reasons with the standing reasons appended is exactly the published
verdict.

**The other holds.** A hold whose reason is not human-resolvable is `Unknown`
and absorbs every refusal it preempts.

The Rust boundary — that `review_standing::settle` records these reasons and
that the second analysis is the held one with the holds lifted — is pinned by
`crates/qcore-c-gateway/tests/review_hold_standing.rs`.

**The concrete reason** (card W47-REVIEW-HOLD-CONCRETE-REASON). A row holding
a call names every marker of every held declaration, reason verbatim, with a
category read from the marker's structure; the annotation is fail-closed on a
hold the environment cannot name, never changes a verdict, keeps a published
`Pending` row `Pending`, and survives composition. The Rust boundary is pinned
by `crates/qcore-audit/src/review_hold_reasons.rs`'s tests,
`crates/qcore-audit/tests/pending_review_report.rs` and
`crates/qcore-audit/tests/checkpoint_cli.rs`. -/

-- The cap and the published verdict.
#print axioms QCore.ReviewHoldStanding.cap_rank_le
#print axioms QCore.ReviewHoldStanding.cap_of_rank_le
#print axioms QCore.ReviewHoldStanding.cap_join
#print axioms QCore.ReviewHoldStanding.cap_ne_unsafe
#print axioms QCore.ReviewHoldStanding.publish_never_lowers
#print axioms QCore.ReviewHoldStanding.only_a_pending_row_moves
#print axioms QCore.ReviewHoldStanding.a_held_row_is_never_published_safe
#print axioms QCore.ReviewHoldStanding.the_second_analysis_never_publishes_unsafe
#print axioms QCore.ReviewHoldStanding.pending_only_if_the_resolution_is_safe
#print axioms QCore.ReviewHoldStanding.never_below_the_resolution

-- The other holds.
#print axioms QCore.ReviewHoldStanding.an_unknown_hold_absorbs_every_refusal

-- The standing reasons and their fold.
#print axioms QCore.ReviewHoldStanding.fold_standing_rank_le
#print axioms QCore.ReviewHoldStanding.join_unknown_of_rank_le
#print axioms QCore.ReviewHoldStanding.pending_join_standing

-- The row.
#print axioms QCore.ReviewHoldStanding.Row.the_settled_fold_is_published
#print axioms QCore.ReviewHoldStanding.Row.value_rank_le_of_fold
#print axioms QCore.ReviewHoldStanding.Row.pending_only_if_every_reason_is_a_hold
#print axioms QCore.ReviewHoldStanding.Row.a_held_call_never_removes_another_reason
#print axioms QCore.ReviewHoldStanding.Row.never_below_the_verdict_without_the_rewrite
#print axioms QCore.ReviewHoldStanding.Row.never_lowers_the_held_verdict
#print axioms QCore.ReviewHoldStanding.Row.a_held_call_beside_a_standing_refusal_is_unknown
#print axioms QCore.ReviewHoldStanding.Row.an_alleged_violation_is_unknown

-- Card W47-REVIEW-HOLD-CONCRETE-REASON: the concrete reason of every review
-- hold on the row it holds (the loader keeps a reason verbatim and refuses a
-- blank one; every held declaration's every marker is published and nothing
-- else; an unnamed hold publishes nothing); the category and the reasons never
-- change a verdict, a published Pending row stays Pending naming every hold,
-- and composition keeps every row's reasons.
#print axioms QCore.ReviewHoldStanding.ConcreteReason.Marker.category_eq_rewrite_iff
#print axioms QCore.ReviewHoldStanding.ConcreteReason.a_blank_reason_is_refused
#print axioms QCore.ReviewHoldStanding.ConcreteReason.a_loaded_marker_keeps_its_reason_verbatim
#print axioms QCore.ReviewHoldStanding.ConcreteReason.holdsOf_names_every_marker
#print axioms QCore.ReviewHoldStanding.ConcreteReason.holdsOf_invents_nothing
#print axioms QCore.ReviewHoldStanding.ConcreteReason.holdsOf_refuses_an_unnamed_hold
#print axioms QCore.ReviewHoldStanding.ConcreteReason.holdsOf_nonempty
#print axioms QCore.ReviewHoldStanding.ConcreteReason.annotate_holds
#print axioms QCore.ReviewHoldStanding.ConcreteReason.annotation_keeps_the_verdict
#print axioms QCore.ReviewHoldStanding.ConcreteReason.adding_reasons_never_changes_the_verdict
#print axioms QCore.ReviewHoldStanding.ConcreteReason.the_category_never_changes_the_verdict
#print axioms QCore.ReviewHoldStanding.ConcreteReason.every_hold_names_its_concrete_reason
#print axioms QCore.ReviewHoldStanding.ConcreteReason.two_holds_on_one_row_list_both
#print axioms QCore.ReviewHoldStanding.ConcreteReason.an_unnamed_hold_is_not_published
#print axioms QCore.ReviewHoldStanding.ConcreteReason.every_published_reason_is_a_loaded_one
#print axioms QCore.ReviewHoldStanding.ConcreteReason.a_pending_row_stays_pending_and_names_every_hold
#print axioms QCore.ReviewHoldStanding.ConcreteReason.compose_value
#print axioms QCore.ReviewHoldStanding.ConcreteReason.compose_append
#print axioms QCore.ReviewHoldStanding.ConcreteReason.composition_never_loses_a_reason
#print axioms QCore.ReviewHoldStanding.ConcreteReason.a_merge_keeps_both_reports_reasons
