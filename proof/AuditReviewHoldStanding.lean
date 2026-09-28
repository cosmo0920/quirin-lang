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
reason; the observed shape (a held call beside a standing refusal) and an
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
`crates/qcore-c-gateway/tests/review_hold_standing.rs`. -/

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
#print axioms QCore.ReviewHoldStanding.Row.the_observed_shape_is_unknown
#print axioms QCore.ReviewHoldStanding.Row.an_alleged_violation_is_unknown
