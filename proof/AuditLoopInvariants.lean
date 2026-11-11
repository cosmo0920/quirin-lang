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

import QCore.LoopInvariants

/-! Audited companion to CG27-LOOPS-RECURSION-BUDGETS (gateway rule 11x): loop
invariant induction, the counted-user loop summary, alias widening, the signed
threshold rank and budget exhaustion. -/

-- Invariant induction: holds on entry, preserved by the body, holds after every
-- iteration; a transition that leaves the invariant fails the check.
#print axioms QCore.LoopInvariants.Induction.invariant_holds_after_every_iteration
#print axioms QCore.LoopInvariants.Induction.invariant_holds_along_every_path_sequence
#print axioms QCore.LoopInvariants.Induction.a_broken_transition_is_refused
#print axioms QCore.LoopInvariants.Induction.a_passing_check_preserves
#print axioms QCore.LoopInvariants.Induction.postfixpoint_bounds_every_iterate

-- The counted-user loop summary: an admitted loop keeps the count and the pending
-- check for any number of iterations; a counting or opaque body is refused.
#print axioms QCore.LoopInvariants.CountedLoop.neutral_ops_have_no_effect_on_delta
#print axioms QCore.LoopInvariants.CountedLoop.neutral_ops_have_no_effect_on_pending
#print axioms QCore.LoopInvariants.CountedLoop.admitted_iterations_are_neutral
#print axioms QCore.LoopInvariants.CountedLoop.an_admitted_loop_preserves_delta
#print axioms QCore.LoopInvariants.CountedLoop.an_admitted_loop_preserves_pending
#print axioms QCore.LoopInvariants.CountedLoop.a_counting_body_is_refused
#print axioms QCore.LoopInvariants.CountedLoop.an_opaque_body_is_refused
#print axioms QCore.LoopInvariants.CountedLoop.one_counting_iteration_changes_the_count

-- Widening: every strict growth lowers a potential; the widened value is absorbing
-- and never Safe.
#print axioms QCore.LoopInvariants.Widening.a_strict_growth_lowers_the_potential
#print axioms QCore.LoopInvariants.Widening.unknown_is_absorbing
#print axioms QCore.LoopInvariants.Widening.growth_stays_within
#print axioms QCore.LoopInvariants.Widening.a_widened_alias_is_never_safe

-- The signed threshold rank: admitted actuals decrease, the relation is well
-- founded, publication is strong induction over the rank; the guard respelling is
-- decided by its context; shared vectors.
#print axioms QCore.LoopInvariants.SignedThreshold.the_base_context_has_rank_zero
#print axioms QCore.LoopInvariants.SignedThreshold.the_step_context_has_a_positive_rank
#print axioms QCore.LoopInvariants.SignedThreshold.an_admitted_actual_decreases
#print axioms QCore.LoopInvariants.SignedThreshold.every_multi_rank_actual_decreases
#print axioms QCore.LoopInvariants.SignedThreshold.threshold_calls_are_well_founded
#print axioms QCore.LoopInvariants.SignedThreshold.the_base_context_makes_no_admitted_call
#print axioms QCore.LoopInvariants.SignedThreshold.publish_by_threshold_induction
#print axioms QCore.LoopInvariants.SignedThreshold.a_read_guard_is_decided_by_its_context
#print axioms QCore.LoopInvariants.SignedThreshold.guard_vectors_hold
#print axioms QCore.LoopInvariants.SignedThreshold.step_vectors_hold
#print axioms QCore.LoopInvariants.SignedThreshold.the_actual_is_representable
#print axioms QCore.LoopInvariants.SignedThreshold.an_unsigned_double_step_can_increase
#print axioms QCore.LoopInvariants.SignedThreshold.a_signed_nonzero_decrement_moves_away_from_zero

-- Budgets: an exhausted leaf is Unknown, so the function is never Safe.
#print axioms QCore.LoopInvariants.Budget.an_exhausted_budget_is_never_safe
#print axioms QCore.LoopInvariants.Budget.the_same_function_without_exhaustion_is_safe
