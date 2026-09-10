import QCore.PipeDesugaring

-- Cards `PIPE-OPERATOR` (the pipe `|>` and the claimed logical-operator
-- tokens) and `PIPE-CHAIN` (multi-stage pipelines as statement sequences).

-- 1. The identity. A piped statement and the statement a reader would have
-- written by hand are the same token sequence, so no checker downstream can
-- tell a pipe was written. This is the whole basis of "zero new checker rules".
#print axioms QCore.PipeDesugaring.sink_pipeline_is_its_hand_written_statement
#print axioms QCore.PipeDesugaring.log_pipeline_is_its_hand_written_statement
#print axioms QCore.PipeDesugaring.call_pipeline_is_its_hand_written_call
#print axioms QCore.PipeDesugaring.the_release_pipeline_is_the_release_call
#print axioms QCore.PipeDesugaring.the_print_pipeline_is_the_print_statement
#print axioms QCore.PipeDesugaring.an_existing_argument_follows_the_piped_value
#print axioms QCore.PipeDesugaring.an_empty_argument_list_is_the_bare_call

-- 2. Neutrality: nothing already written can move.
#print axioms QCore.PipeDesugaring.pipe_free_is_unchanged
#print axioms QCore.PipeDesugaring.a_pipe_free_stream_stays_pipe_free
#print axioms QCore.PipeDesugaring.a_rewritten_sink_statement_contains_no_pipe
#print axioms QCore.PipeDesugaring.a_rewritten_call_statement_contains_no_pipe
#print axioms QCore.PipeDesugaring.the_argument_carrying_rewrite_contains_no_pipe

-- 3. Attribution: the bytes the reader wrote keep their own spans, and only
-- delimiters are invented.
#print axioms QCore.PipeDesugaring.rewrite_preserves_written_origins
#print axioms QCore.PipeDesugaring.a_sink_pipeline_synthesizes_nothing
#print axioms QCore.PipeDesugaring.synthesized_tokens_are_anchored_at_the_pipe
#print axioms QCore.PipeDesugaring.only_delimiters_are_synthesized_in_a_single_stage_pipeline

-- 4. Fail-closed: every shape outside the v1 grammar refuses, and a refusal
-- leaves the statement untouched rather than half-rewritten.
#print axioms QCore.PipeDesugaring.piping_into_an_operandless_stage_refuses
#print axioms QCore.PipeDesugaring.an_empty_operand_refuses
#print axioms QCore.PipeDesugaring.an_empty_stage_refuses
#print axioms QCore.PipeDesugaring.a_stage_that_is_not_a_name_refuses
#print axioms QCore.PipeDesugaring.a_sink_stage_of_the_wrong_arity_refuses
#print axioms QCore.PipeDesugaring.a_call_stage_without_a_closing_paren_refuses
#print axioms QCore.PipeDesugaring.refused_shapes_rewrite_to_nothing
#print axioms QCore.PipeDesugaring.a_refused_statement_is_unchanged
#print axioms QCore.PipeDesugaring.a_refused_statement_keeps_its_pipe

-- 5. Chains (card `PIPE-CHAIN`): a pipeline of n stages is the sequence of n
-- statements a reader would have written, each non-final stage binding a
-- hygienic temporary the next receives.
#print axioms QCore.PipeDesugaring.a_one_stage_chain_is_the_single_stage_rewrite
#print axioms QCore.PipeDesugaring.the_stage_rewrite_agrees_with_the_single_stage_rewrite
#print axioms QCore.PipeDesugaring.the_two_stage_chain_is_its_hand_written_sequence
#print axioms QCore.PipeDesugaring.the_first_stage_receives_the_readers_operand
#print axioms QCore.PipeDesugaring.a_later_stage_receives_the_previous_temporary
#print axioms QCore.PipeDesugaring.a_consuming_stage_moves_the_previous_temporary

-- 5.2 Fail-closed: only a call stage may feed a later stage, and every sink is
-- last-stage-only. Card `PIPE-SCALAR-RESULT` widened `producesResult` from the
-- acquisition keyword to the stage kind; the refusal theorems below are
-- unchanged in statement, and `refused_chains_rewrite_to_nothing` changed one
-- conjunct on purpose rather than being slipped past.
#print axioms QCore.PipeDesugaring.a_non_final_stage_that_produces_nothing_refuses
#print axioms QCore.PipeDesugaring.a_sink_is_last_stage_only
#print axioms QCore.PipeDesugaring.move_on_the_first_stage_refuses
#print axioms QCore.PipeDesugaring.an_empty_chain_refuses
#print axioms QCore.PipeDesugaring.refused_chains_rewrite_to_nothing

-- 5.2.1 Card `PIPE-SCALAR-RESULT`: the widening, in both directions.
#print axioms QCore.PipeDesugaring.a_call_stage_may_feed_a_later_stage
#print axioms QCore.PipeDesugaring.a_two_argument_scalar_composition_is_its_hand_written_sequence
#print axioms QCore.PipeDesugaring.an_adapter_stage_still_produces_a_result
#print axioms QCore.PipeDesugaring.a_sink_still_produces_nothing

-- 5.3 Why the sequence is admissible at all: consecutive statements' byte
-- extents are disjoint by construction, and the naive anchoring's are not.
-- This is what retires wave 18's recorded `PIPE-CHAIN` blocker.
#print axioms QCore.PipeDesugaring.consecutive_statements_are_disjoint
#print axioms QCore.PipeDesugaring.the_naive_anchoring_touches_at_the_pipe
#print axioms QCore.PipeDesugaring.the_touching_byte_is_covered_by_both_naive_windows
#print axioms QCore.PipeDesugaring.the_landed_anchoring_separates_the_same_byte

-- 5.4 Hygiene: temporaries are distinct, carry the reserved prefix, and an
-- ordinary binding does not.
#print axioms QCore.PipeDesugaring.temporaries_are_pairwise_distinct
#print axioms QCore.PipeDesugaring.a_temporary_carries_the_reserved_prefix
#print axioms QCore.PipeDesugaring.an_ordinary_binding_does_not
#print axioms QCore.PipeDesugaring.a_chain_synthesizes_only_delimiters_binders_and_temporaries
#print axioms QCore.PipeDesugaring.a_chain_preserves_every_written_origin
#print axioms QCore.PipeDesugaring.a_rewritten_chain_contains_no_pipe

-- 6. The claimed logical operators refuse; the single `&` is not claimed.
#print axioms QCore.PipeDesugaring.the_or_operator_refuses
#print axioms QCore.PipeDesugaring.the_and_operator_refuses
#print axioms QCore.PipeDesugaring.the_double_bar_refuses
#print axioms QCore.PipeDesugaring.a_single_ampersand_is_not_claimed
#print axioms QCore.PipeDesugaring.the_operator_family_is_exactly_as_declared

-- 7. Tokenization: longest match at width two, with the quant delimiters
-- untouched and the future or-operator's constraint recorded.
#print axioms QCore.PipeDesugaring.the_quant_delimiters_are_untouched
#print axioms QCore.PipeDesugaring.the_pipe_wins_by_longest_match
#print axioms QCore.PipeDesugaring.the_and_wins_by_longest_match
#print axioms QCore.PipeDesugaring.the_two_byte_table_is_injective
#print axioms QCore.PipeDesugaring.three_ampersands_are_and_then_reserved
