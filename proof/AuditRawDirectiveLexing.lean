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

import QCore.RawDirectiveLexing

-- Unit `E01-F1-RAW`: the raw token scan's logical-line directive rule. Model of
-- `crates/qcore-c-gateway/src/legacy_c.rs`'s `tokenize`. The Rust byte lexer's
-- identifier, literal and comment scanners, and `function_names`' reading of
-- the token stream, are Rust boundaries recorded in
-- `docs/reports/remaining-e01-f1-raw/README.md`.

-- The classification is a total function into disjoint classes, and `#`
-- introduces a directive exactly when it is line-initial.
#print axioms QCore.RawDirectiveLexing.classify_is_total
#print axioms QCore.RawDirectiveLexing.classify_is_single_valued
#print axioms QCore.RawDirectiveLexing.hash_at_line_start_is_a_directive
#print axioms QCore.RawDirectiveLexing.hash_after_a_token_is_a_punctuator
#print axioms QCore.RawDirectiveLexing.directive_iff_line_initial
#print axioms QCore.RawDirectiveLexing.only_hash_introduces_a_directive
#print axioms QCore.RawDirectiveLexing.splice_iff_newline_follows

-- A directive contributes no token, so a macro replacement list perturbs no
-- file-scope brace depth — single-line and continued alike.
#print axioms QCore.RawDirectiveLexing.skipLogicalLine_plain
#print axioms QCore.RawDirectiveLexing.a_directive_contributes_no_token
#print axioms QCore.RawDirectiveLexing.skipLogicalLine_across_a_splice
#print axioms QCore.RawDirectiveLexing.a_continued_directive_contributes_no_token
#print axioms QCore.RawDirectiveLexing.a_directive_body_moves_no_brace_depth
#print axioms QCore.RawDirectiveLexing.a_continued_directive_body_moves_no_brace_depth

-- Line-initiality: what restores it, what preserves it and what clears it.
#print axioms QCore.RawDirectiveLexing.a_newline_starts_a_logical_line
#print axioms QCore.RawDirectiveLexing.a_splice_preserves_line_initiality
#print axioms QCore.RawDirectiveLexing.a_hash_across_a_splice_after_code_is_a_punctuator
#print axioms QCore.RawDirectiveLexing.white_space_preserves_line_initiality
#print axioms QCore.RawDirectiveLexing.a_block_comment_that_spans_lines_starts_a_logical_line
#print axioms QCore.RawDirectiveLexing.a_block_comment_within_a_line_preserves_the_state
#print axioms QCore.RawDirectiveLexing.a_line_comment_swallows_its_continuation
#print axioms QCore.RawDirectiveLexing.a_token_clears_line_initiality

-- Stringification and pasting are punctuators, not directives.
#print axioms QCore.RawDirectiveLexing.stringification_is_emitted
#print axioms QCore.RawDirectiveLexing.pasting_is_two_punctuators

-- Non-vacuity: the reduced `librdkafka/src/rdkafka.c` shape, decided under both
-- rules.
#print axioms QCore.RawDirectiveLexing.the_repaired_lexer_balances_the_measured_shape
#print axioms QCore.RawDirectiveLexing.the_pre_repair_lexer_desynchronises_the_measured_shape
#print axioms QCore.RawDirectiveLexing.the_repair_changes_the_measured_shape
#print axioms QCore.RawDirectiveLexing.the_repaired_lexer_sees_the_later_definitions
#print axioms QCore.RawDirectiveLexing.the_pre_repair_lexer_counts_an_unclosed_macro_brace

-- The repair is confined to directive text and phase-2 splices.
#print axioms QCore.RawDirectiveLexing.the_rules_agree_on_directive_free_input

-- Exhaustive agreement with the Rust lexer over the length-3 domain.
#print axioms QCore.RawDirectiveLexing.the_digest_domain_is_exhaustive
#print axioms QCore.RawDirectiveLexing.the_domain_checksum_is_pinned
