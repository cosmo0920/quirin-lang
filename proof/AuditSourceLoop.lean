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

import QCore.SourceLoop

-- Card `LOOP-V1`: `source.loop.v1`, the checked bounded counted loop.

-- Unrolling is sound, and it is what the checker does. The flat event list the
-- inherited cycle-3 checker is handed has exactly the effect of running the
-- body `count` times, so nothing is widened between the program a reader wrote
-- and the program the checker decides.
#print axioms QCore.SourceLoop.run_append
#print axioms QCore.SourceLoop.running_the_unrolling_is_iterating_the_body
#print axioms QCore.SourceLoop.a_loop_unrolls_to_its_body_repeated
#print axioms QCore.SourceLoop.a_nested_loop_unrolls_by_the_same_rule

-- The zero bound, and why it is refused rather than admitted: its body is never
-- examined, and admitting it would be the one way a statement could enter a
-- program without passing the checker.
#print axioms QCore.SourceLoop.a_zero_bound_unrolls_to_nothing
#print axioms QCore.SourceLoop.a_zero_bound_runs_no_event
#print axioms QCore.SourceLoop.a_bound_of_one_runs_the_body_once

-- Per-iteration custody identity: one pass decides every bound, with both a
-- positive control and a non-vacuous converse in the accusing direction.
#print axioms QCore.SourceLoop.an_identity_body_iterates
#print axioms QCore.SourceLoop.a_body_of_stores_and_lines_is_the_identity
#print axioms QCore.SourceLoop.an_acquire_then_release_body_is_the_identity
#print axioms QCore.SourceLoop.a_body_that_only_acquires_grows_the_ledger
#print axioms QCore.SourceLoop.a_body_that_only_acquires_is_not_the_identity

-- The index takes every value below the bound and never reaches it.
#print axioms QCore.SourceLoop.the_index_takes_every_value_below_the_bound
#print axioms QCore.SourceLoop.the_index_never_reaches_the_bound

-- The cost of the checking route, stated as the product it is — which is why
-- the budget is a product over a nest rather than a depth limit.
#print axioms QCore.SourceLoop.repeatAppend_length
#print axioms QCore.SourceLoop.a_loops_cost_is_its_bound_times_its_body

-- Neutrality: a program that writes no loop is unchanged by this construct,
-- against a `loopFree` predicate written independently of `unroll`, with a
-- theorem making the claim non-vacuous.
#print axioms QCore.SourceLoop.a_loop_free_item_unrolls_to_itself
#print axioms QCore.SourceLoop.a_loop_free_sequence_unrolls_to_itself
#print axioms QCore.SourceLoop.a_loop_is_the_one_item_that_is_not_loop_free

-- The refused shapes as one list, so a widening must change a theorem rather
-- than slip past one.
#print axioms QCore.SourceLoop.refused_shapes_are_exactly_these
