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

import QCore.BranchJoin

-- Card `IF-STATEMENT`: the source branching construct `source.branch.v1` and
-- its custody join. Wave 18 recorded that a short-circuit had nothing to lower
-- onto; this is the semantics of the construct that removes that blocker.
--
-- Note on the operator family: the logical operators that will lower onto this
-- construct are `||` (or) and `&&` (and); the single characters `|` and `&` are
-- reserved for future binary/bitwise operations. Neither operator is modelled
-- here — the condition tier below deliberately has no connectives, because
-- supplying them is the successor card's whole content.

-- 1. The condition tier is total and decidable. This is what lets one
-- reference observation, one CFG observation and one observation parsed from
-- the compiled program agree: a run-time-varying condition would leave "the"
-- observation ill-defined.
#print axioms QCore.BranchJoin.every_condition_has_a_value
#print axioms QCore.BranchJoin.the_comparisons_are_the_relations

-- 2. The join is agreement between the arms that *survive* to it, never
-- widening. The joined state is both survivors' state, so nothing either
-- established is lost and nothing neither established is invented.
--
-- Card `IF-ARMS` changed this section's shape, not its strength: an arm that
-- returned discharged its obligations at its own exit and does not reach the
-- join, so it constrains it not at all. The last theorem is the complete
-- characterisation — the only shape with no joined state is two surviving arms
-- that disagree — so nothing that refused at the join before is admitted at it
-- now.
#print axioms QCore.BranchJoin.the_join_of_two_survivors_is_their_common_state
#print axioms QCore.BranchJoin.the_join_widens_neither_survivor
#print axioms QCore.BranchJoin.disagreeing_survivors_do_not_join
#print axioms QCore.BranchJoin.a_returning_arm_does_not_constrain_the_join
#print axioms QCore.BranchJoin.a_returning_else_arm_does_not_constrain_the_join
#print axioms QCore.BranchJoin.a_branch_whose_arms_both_return_returns
#print axioms QCore.BranchJoin.the_join_refuses_only_two_disagreeing_survivors
#print axioms QCore.BranchJoin.the_join_rule_is_exactly_agreement_between_survivors

-- 3. Fail-closed, on concrete programs: the conditional leak, the shape where
-- one arm moves what the other releases (which comparing live resources alone
-- would admit), a leak in the arm the decided condition does *not* take, an
-- early return that leaves a resource live, and two arms spending one
-- acquisition identity. Collected as one list so a future widening must change
-- a theorem — as card `IF-ARMS` did, replacing the `return`-inside-an-arm row
-- with the obligation that widening rests on.
#print axioms QCore.BranchJoin.a_conditional_leak_refuses
#print axioms QCore.BranchJoin.both_arms_releasing_is_admitted
#print axioms QCore.BranchJoin.moving_in_one_arm_and_releasing_in_the_other_refuses
#print axioms QCore.BranchJoin.a_leak_in_the_untaken_arm_refuses
#print axioms QCore.BranchJoin.an_arm_return_joins_over_the_survivor
#print axioms QCore.BranchJoin.an_arm_return_must_discharge_its_custody
#print axioms QCore.BranchJoin.two_arms_cannot_spend_one_acquisition_identity
#print axioms QCore.BranchJoin.refused_shapes_are_exactly_these

-- 4. Nothing branch-free moves. Every operation that is not a branch steps
-- exactly as it did before this card, and a branch-free sequence runs exactly
-- as the pre-card driver ran it — with the branch shown to be the one
-- operation the two disagree on, so neither statement is vacuous.
#print axioms QCore.BranchJoin.a_branch_free_operation_steps_exactly_as_before
#print axioms QCore.BranchJoin.a_branch_is_the_one_operation_they_disagree_on
#print axioms QCore.BranchJoin.stepOp_is_linear_on_branch_free
#print axioms QCore.BranchJoin.runSeq_is_linear_on_branch_free

-- 5. Block allocation: the numbering the emitter has always produced. A
-- branch-free operation takes exactly its own id and advances the counter by
-- one, so a branch-free program is numbered 0, 1, 2, … — the model counterpart
-- of the byte-identical-C pin. A branch allocates its arms inside its own
-- numbering, so the claim is about branch-free programs rather than vacuous.
#print axioms QCore.BranchJoin.allocOp_monotone
#print axioms QCore.BranchJoin.allocSeq_monotone
#print axioms QCore.BranchJoin.a_branch_free_operation_takes_one_consecutive_block
#print axioms QCore.BranchJoin.the_branch_free_program_is_numbered_from_zero
#print axioms QCore.BranchJoin.the_branch_free_program_is_branch_free
#print axioms QCore.BranchJoin.a_branch_allocates_its_arms_after_itself
#print axioms QCore.BranchJoin.a_branch_precedes_its_arms
#print axioms QCore.BranchJoin.a_sequence_advances_past_its_head

-- 6. What the observation is: the decided condition selects the arm, and the
-- branch's whole effect is that arm's — there is no merge step, because there
-- is nothing to merge. The branched program observes what its hand-written
-- unbranched twin does, while being a genuinely different program.
#print axioms QCore.BranchJoin.the_observed_arm_is_the_one_the_condition_selects
#print axioms QCore.BranchJoin.a_true_condition_observes_the_then_arm
#print axioms QCore.BranchJoin.a_false_condition_observes_the_else_arm
#print axioms QCore.BranchJoin.a_comparison_condition_selects_by_its_relation
#print axioms QCore.BranchJoin.an_empty_arm_changes_nothing
#print axioms QCore.BranchJoin.the_admitted_branch_observes_its_taken_arm
#print axioms QCore.BranchJoin.the_branch_observes_what_its_hand_written_twin_does
#print axioms QCore.BranchJoin.the_twin_is_branch_free_and_the_branched_program_is_not
