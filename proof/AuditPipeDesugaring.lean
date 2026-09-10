import QCore.PipeDesugaring

-- Card `PIPE-OPERATOR`: the pipe `|>` and the claimed logical-operator tokens.

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
#print axioms QCore.PipeDesugaring.only_delimiters_are_synthesized

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

-- 5. The claimed logical operators refuse; the single `&` is not claimed.
#print axioms QCore.PipeDesugaring.the_or_operator_refuses
#print axioms QCore.PipeDesugaring.the_and_operator_refuses
#print axioms QCore.PipeDesugaring.the_double_bar_refuses
#print axioms QCore.PipeDesugaring.a_single_ampersand_is_not_claimed
#print axioms QCore.PipeDesugaring.the_operator_family_is_exactly_as_declared

-- 6. Tokenization: longest match at width two, with the quant delimiters
-- untouched and the future or-operator's constraint recorded.
#print axioms QCore.PipeDesugaring.the_quant_delimiters_are_untouched
#print axioms QCore.PipeDesugaring.the_pipe_wins_by_longest_match
#print axioms QCore.PipeDesugaring.the_and_wins_by_longest_match
#print axioms QCore.PipeDesugaring.the_two_byte_table_is_injective
#print axioms QCore.PipeDesugaring.three_ampersands_are_and_then_reserved
