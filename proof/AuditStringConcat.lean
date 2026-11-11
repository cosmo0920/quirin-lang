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

import QCore.StringConcat

-- Card `STRING-CONCAT`: text a program **computes** rather than writes, and the
-- `std.buffer` → `std.string` bridge amendment §5 names
-- (`docs/design/STRING_SLICE_V1.md` §5 and follow-on `STRING-SLICE-CONCAT`).

-- The vocabulary the card adds, printed so that a later change to any of it is
-- visible in this surface rather than only inside a proof.
#print axioms QCore.StringConcat.compose
#print axioms QCore.StringConcat.pieceLens
#print axioms QCore.StringConcat.composedView
#print axioms QCore.StringConcat.extend
#print axioms QCore.StringConcat.admitsConcat
#print axioms QCore.StringConcat.leadLength
#print axioms QCore.StringConcat.wellFormed
#print axioms QCore.StringConcat.liveAt
#print axioms QCore.StringConcat.excluded
#print axioms QCore.StringConcat.touches
#print axioms QCore.StringConcat.step
#print axioms QCore.StringConcat.run
#print axioms QCore.StringConcat.addedEffects

-- **The length arithmetic.** This is what makes a `Content-Length` a program
-- derives as decidable as one it writes.
#print axioms QCore.StringConcat.the_composed_length_is_the_sum_of_the_piece_lengths
#print axioms QCore.StringConcat.composing_two_pieces_appends_their_bytes
#print axioms QCore.StringConcat.composing_two_pieces_adds_their_lengths
#print axioms QCore.StringConcat.a_composition_of_three_is_a_composition_of_a_composition
#print axioms QCore.StringConcat.an_empty_piece_contributes_no_bytes

-- **A composition is static storage of its own** — not a window onto a piece,
-- which is why it needs no allocation, no custody and no lifetime rule, and why
-- composing a borrowed view lets the buffer be released immediately after.
#print axioms QCore.StringConcat.the_composed_view_addresses_the_composed_payload
#print axioms QCore.StringConcat.a_composition_addresses_none_of_its_pieces
#print axioms QCore.StringConcat.extending_the_storage_moves_no_existing_view

-- The admission rule, in both directions.
#print axioms QCore.StringConcat.a_composition_of_one_is_refused
#print axioms QCore.StringConcat.a_composition_of_none_is_refused
#print axioms QCore.StringConcat.an_over_long_composition_is_refused
#print axioms QCore.StringConcat.an_admitted_composition_is_inside_the_bound

-- **The static UTF-8 decision**, and the cases it separates — including the one
-- it says *yes* to: a zeroed buffer is well-formed, because `bytes(N)`
-- zero-initialises and `U+0000` is an ordinary code point.
#print axioms QCore.StringConcat.the_empty_payload_is_well_formed
#print axioms QCore.StringConcat.a_zeroed_buffer_is_well_formed
#print axioms QCore.StringConcat.an_ascii_payload_is_well_formed
#print axioms QCore.StringConcat.a_two_byte_code_point_is_well_formed
#print axioms QCore.StringConcat.a_lone_continuation_byte_is_refused
#print axioms QCore.StringConcat.a_truncated_sequence_is_refused
#print axioms QCore.StringConcat.a_byte_outside_the_encoding_is_refused
#print axioms QCore.StringConcat.a_dangling_lead_byte_is_refused

-- **The borrow window**, as an interval: refused inside, admitted after, and
-- extended by every view derived from the borrow.
#print axioms QCore.StringConcat.a_statement_inside_the_window_is_refused
#print axioms QCore.StringConcat.a_statement_after_the_window_is_admitted
#print axioms QCore.StringConcat.a_borrow_constrains_only_its_own_buffer
#print axioms QCore.StringConcat.an_unread_borrow_constrains_only_its_own_statement
#print axioms QCore.StringConcat.a_derived_view_extends_the_window

-- **The exclusion as soundness.** Over a window in which nothing touches the
-- buffer, the bytes a borrowed view addresses at the end are the bytes that
-- were validated at the start — so the view the program renders is the view the
-- checker decided was well-formed, and not a later one.
#print axioms QCore.StringConcat.an_untouching_step_preserves_the_bytes
#print axioms QCore.StringConcat.an_excluded_window_preserves_the_buffers_bytes
#print axioms QCore.StringConcat.a_changed_buffer_had_a_touching_statement

-- Amendment §7 survives the card: neither added operation derives an effect, so
-- the `alloc` and `mut` a composed module carries are the **buffer's**.
#print axioms QCore.StringConcat.a_composition_derives_no_effect
#print axioms QCore.StringConcat.a_borrow_derives_no_effect
#print axioms QCore.StringConcat.neither_added_operation_derives_alloc
#print axioms QCore.StringConcat.neither_added_operation_derives_mut

-- The view's equality is determined by its three components. This is a
-- representation check, not an ownership theorem. The added vocabulary is
-- exactly two operations.
#print axioms QCore.StringConcat.views_equal_iff_components_equal
#print axioms QCore.StringConcat.the_added_operations_are_exactly_two
