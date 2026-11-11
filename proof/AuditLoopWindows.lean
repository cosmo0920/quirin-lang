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

import QCore.LoopWindows

-- Card `LOOPS-ON-COMPOSED-ROUTES` (wave 39): a loan's window computed **over
-- iterations**, so a `for` beside a borrow or a lend is admitted where the
-- window covers it — lifting `SLICE-PERMISSIONS-LOOP-WINDOW` (buffer route)
-- and `STRING-CONCAT-LOOP-WINDOW` (composed string route), which refused the
-- loop because the window was computed over top-level statements only.

-- The vocabulary the card adds, printed so that a later change to any of it is
-- visible in this surface rather than only inside a proof.
#print axioms QCore.LoopWindows.copies
#print axioms QCore.LoopWindows.tagFrom
#print axioms QCore.LoopWindows.tagBody
#print axioms QCore.LoopWindows.taggedFrom
#print axioms QCore.LoopWindows.tagged
#print axioms QCore.LoopWindows.extend
#print axioms QCore.LoopWindows.lastUse
#print axioms QCore.LoopWindows.unrollStmt
#print axioms QCore.LoopWindows.unrollStmts
#print axioms QCore.LoopWindows.toItem
#print axioms QCore.LoopWindows.toItems
#print axioms QCore.LoopWindows.flatTagged
#print axioms QCore.LoopWindows.shallowTagged
#print axioms QCore.LoopWindows.lastUseShallow
#print axioms QCore.LoopWindows.bodyOnlyUse
#print axioms QCore.LoopWindows.admittedInBody

-- **Soundness relative to the unrolling.** Every use in the unrolled program
-- lies inside the window; unrolled positions are attributed to non-decreasing
-- top-level indices; and so everything between a loan's opening and any of its
-- uses in the unrolled program is attributed inside the iteration window.
#print axioms QCore.LoopWindows.foldl_extend_ge
#print axioms QCore.LoopWindows.foldl_extend_covers
#print axioms QCore.LoopWindows.every_use_lies_inside_the_window
#print axioms QCore.LoopWindows.tagFrom_origin
#print axioms QCore.LoopWindows.tagBody_origin
#print axioms QCore.LoopWindows.taggedFrom_origin_ge
#print axioms QCore.LoopWindows.pairwise_of_one_origin
#print axioms QCore.LoopWindows.origins_are_monotone
#print axioms QCore.LoopWindows.the_unrolled_window_lies_inside_the_iteration_window

-- **The unrolling is `LOOP-V1`'s.** Forgetting the tags gives the untagged
-- unrolling, and on the buffer route's events that is
-- `QCore.SourceLoop.unrollList` of the same program read as that module's items.
#print axioms QCore.LoopWindows.copies_map
#print axioms QCore.LoopWindows.tagFrom_untagged
#print axioms QCore.LoopWindows.tagBody_untagged
#print axioms QCore.LoopWindows.taggedFrom_untagged
#print axioms QCore.LoopWindows.tagged_forgets_to_the_unrolling
#print axioms QCore.LoopWindows.copies_is_repeatAppend
#print axioms QCore.LoopWindows.toItem_unrolls
#print axioms QCore.LoopWindows.toItems_unroll
#print axioms QCore.LoopWindows.the_unrolling_is_loop_v1s

-- **Equal where there is no loop**: a loop-free program's tagged unrolling is
-- the program with its own indices, so no admitted flat program's window moves.
#print axioms QCore.LoopWindows.the_window_is_exact_over_a_flat_program

-- **Non-vacuous**: the replaced top-level-only computation misses a use inside
-- a loop body — the admitting arm `STABILITY-REEVAL-3` §4 measured and the
-- card's Rust mutation re-exposes.
#print axioms QCore.LoopWindows.a_top_level_only_window_misses_a_body_use

-- **A longer live set refuses no less**: the IR's owner-access, lend-admission
-- and drop decisions are monotone in the live loans in the refusing direction,
-- which is what makes an over-approximation of liveness safe.
#print axioms QCore.LoopWindows.a_longer_live_set_refuses_no_less
#print axioms QCore.LoopWindows.a_longer_live_set_admits_no_more_lends
#print axioms QCore.LoopWindows.a_longer_live_set_blocks_the_drop_no_less

-- Card `COMPOSED-WINDOW-REACH` (wave 41): the alphabet the implementation
-- actually produces, and the Rust binding.
--
-- What stood here was `the_admitted_body_shapes_are_exactly_these`, a theorem
-- that each of a two-constructor type's constructors is a member of a list
-- spelling both — true of any such pair and silent about the route
-- (`LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY`). It is superseded by a decision on
-- `LendEvent`, the window-relevant shape of the Rust `BufferEvent`, which a
-- mutation can falsify.
#print axioms QCore.LoopWindows.LendEvent
#print axioms QCore.LoopWindows.usesLoan
#print axioms QCore.LoopWindows.unnameable
#print axioms QCore.LoopWindows.admittedInBody
#print axioms QCore.LoopWindows.every_admitted_body_event_is_windowed
#print axioms QCore.LoopWindows.the_unnameable_events_are_exactly_the_inadmissible_ones
#print axioms QCore.LoopWindows.an_unnameable_event_uses_no_loan

-- The unrolling at an alphabet that carries a loan. `§4`'s
-- `the_unrolling_is_loop_v1s` is stated at `QCore.StdBuffer.Event`, whose eight
-- constructors cannot spell a lend, so the tie was to a structure the
-- implementation never produces (`LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION`). These
-- put the same recursion on both sides of the boundary.
#print axioms QCore.LoopWindows.copies_eq_flatten_replicate
#print axioms QCore.LoopWindows.repeatAppend_eq_flatten_replicate
#print axioms QCore.LoopWindows.the_lend_alphabets_unrolling_is_loop_v1s_recursion
#print axioms QCore.LoopWindows.loop_v1s_unrolling_is_the_same_flattening
#print axioms QCore.LoopWindows.every_lend_use_lies_inside_the_window

-- The shared vectors `compiler/tests/composed_window_reach.rs` reads out of
-- this module's source and replays against the Rust `last_uses`. Each row's
-- spelling is decided here to be the program's own rendering, so neither side
-- can drift from the other's transcription of it.
#print axioms QCore.LoopWindows.renderEvent
#print axioms QCore.LoopWindows.render
#print axioms QCore.LoopWindows.renderList
#print axioms QCore.LoopWindows.agreement
#print axioms QCore.LoopWindows.tableHolds
#print axioms QCore.LoopWindows.the_agreement_table_holds
#print axioms QCore.LoopWindows.the_table_contains_a_body_only_use
