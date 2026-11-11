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

-- 5.5 The intrinsic stage (card `PIPE-INTRINSIC-STAGE`): a call-shaped model
-- intrinsic is a call stage by the existing definition, so the payoff chain is
-- the statement sequence the route already lowers, and a keyword-shaped one is
-- unreachable.
#print axioms QCore.PipeDesugaring.no_intrinsic_is_a_sink_or_operandless
#print axioms QCore.PipeDesugaring.every_call_shaped_intrinsic_produces_a_result
#print axioms QCore.PipeDesugaring.the_buffer_read_chain_is_its_hand_written_sequence
#print axioms QCore.PipeDesugaring.an_intrinsic_takes_the_piped_value_as_its_first_argument
#print axioms QCore.PipeDesugaring.an_allocating_intrinsic_keeps_its_adapter_tail
#print axioms QCore.PipeDesugaring.a_keyword_shaped_intrinsic_is_not_reachable

-- 5.6 The two statement readers: they agree on a pipe-free statement and do
-- not on a desugared one, which is why the sequential route now reads the
-- program's own stream rather than re-lexing the source bytes.
#print axioms QCore.PipeDesugaring.the_two_readers_agree_on_a_pipe_free_statement
#print axioms QCore.PipeDesugaring.the_byte_reader_loses_the_desugared_statement
#print axioms QCore.PipeDesugaring.the_byte_order_and_the_statement_order_differ

-- 5.7 The releasing stage (card `PIPE-DROP-STAGE`): `drop` joins the closed
-- statement table and nothing else does. A releasing entry emits by the same
-- rule as a rendering one and binds no result by the same predicate, so its
-- terminality is `a_non_final_stage_that_produces_nothing_refuses` applied
-- rather than a rule of its own — and the allocating chain
-- `PIPE-INTRINSIC-STAGE` §3.4 measured as unreleasable becomes spellable end to
-- end. `keywordShapedIntrinsics` lost `drop` in the same change, which is the
-- §5.5 census pin moved by name rather than slipped past.
#print axioms QCore.PipeDesugaring.the_statement_stage_table_is_exactly_three_heads
#print axioms QCore.PipeDesugaring.release_pipeline_is_its_hand_written_statement
#print axioms QCore.PipeDesugaring.the_drop_pipeline_is_the_drop_statement
#print axioms QCore.PipeDesugaring.a_release_pipeline_synthesizes_nothing
#print axioms QCore.PipeDesugaring.every_releasing_head_binds_no_result
#print axioms QCore.PipeDesugaring.a_releasing_stage_in_a_non_final_position_refuses
#print axioms QCore.PipeDesugaring.a_release_is_last_stage_only
#print axioms QCore.PipeDesugaring.the_allocating_chain_is_its_hand_written_sequence
#print axioms QCore.PipeDesugaring.a_release_and_a_render_emit_by_the_same_rule
#print axioms QCore.PipeDesugaring.the_release_is_the_only_keyword_shaped_head_in_the_table

-- 6. The operator gate, as retabled by card `OR-AND-LOWERING`: the three
-- single-byte characters are reserved and refuse everywhere, the two
-- connectives are admitted in an `if` condition and refused outside one, and
-- the two families are disjoint. What a connective *means* is
-- `QCore.Connectives`.
#print axioms QCore.PipeDesugaring.reserved_and_connective_are_disjoint
#print axioms QCore.PipeDesugaring.a_reserved_character_refuses_everywhere
#print axioms QCore.PipeDesugaring.the_single_bar_is_reserved
#print axioms QCore.PipeDesugaring.the_single_ampersand_is_reserved
#print axioms QCore.PipeDesugaring.the_caret_is_reserved
#print axioms QCore.PipeDesugaring.a_connective_outside_a_condition_refuses
#print axioms QCore.PipeDesugaring.a_connective_inside_a_condition_is_admitted
#print axioms QCore.PipeDesugaring.the_operator_family_is_exactly_as_declared

-- 7. Tokenization: longest match at width two, with the quant delimiters
-- untouched and the residual `|>` ambiguity recorded against the bitwise wave.
#print axioms QCore.PipeDesugaring.the_quant_delimiters_are_untouched
#print axioms QCore.PipeDesugaring.the_pipe_wins_by_longest_match
#print axioms QCore.PipeDesugaring.the_or_wins_by_longest_match
#print axioms QCore.PipeDesugaring.the_and_wins_by_longest_match
#print axioms QCore.PipeDesugaring.the_two_byte_table_is_injective
#print axioms QCore.PipeDesugaring.three_ampersands_are_and_then_reserved
#print axioms QCore.PipeDesugaring.three_bars_are_or_then_reserved
#print axioms QCore.PipeDesugaring.the_caret_never_absorbs_a_neighbour
