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

import QCore.Bitwise

-- Card `BITWISE-CONDITIONS`: the integer tier, unary negation and the
-- check-time constant bindings of `source.branch.v1`. The card unifies the
-- three follow-ons `BITWISE-OPERATORS`, `IF-CONNECTIVE-NEGATION` and
-- `IF-CONDITION-BINDINGS`, and — like `AuditConnectives` — everything below is
-- stated against `QCore.BranchJoin`'s own operations, validator and
-- interpreter, so these are theorems about the model card `IF-STATEMENT`
-- proved rather than about a paraphrase of it.

-- 1. The operators. `|`, `^` and `&` characterized bit by bit against Lean's
-- own `BitVec` operations rather than against a private definition, with the
-- three shown distinct so the table is not vacuous, and with the totality that
-- is the reason this family could be admitted where a shift could not: C leaves
-- all three defined for every operand pair.
#print axioms QCore.Bitwise.the_operators_are_their_truth_tables
#print axioms QCore.Bitwise.the_three_operators_are_distinct
#print axioms QCore.Bitwise.the_operators_are_total
#print axioms QCore.Bitwise.each_operator_is_commutative_and_associative
#print axioms QCore.Bitwise.the_operators_do_not_commute_with_each_other

-- 2. The closed integer tier and its evaluator have no state input. This is
-- a vocabulary restriction; the fold/value correspondence below is audited.

-- 3. Neutrality. Folding a condition's operands is value-preserving, which is
-- what lets the model work over `BranchJoin.Condition` while the Rust carries
-- the operand tree into `qcore-core` so the emitted C computes the bit
-- operation itself. Where a widened condition has a shipped spelling the two
-- evaluate identically — this card moves no verdict for a program that writes
-- no bit operation — and the claim is not vacuous, because a bitwise operand
-- has no shipped spelling at all.
#print axioms QCore.Bitwise.the_fold_preserves_the_value
#print axioms QCore.Bitwise.the_shipped_tier_evaluates_unchanged
#print axioms QCore.Bitwise.a_bitwise_operand_has_no_shipped_spelling
#print axioms QCore.Bitwise.the_masked_comparison_is_admitted_and_true

-- 4. The negation, which is the **arm swap** and nothing else: no new
-- operation, no new condition constructor, no new rule. `!C` is the branch a
-- reader writes with the arms exchanged — the model counterpart of the card's
-- byte-equality pin — `!!A` is literally `A`'s own lowering, and the arm swap
-- is the same rewrite De Morgan would have performed, so the card did not have
-- to choose between them.
#print axioms QCore.Bitwise.the_negation_is_the_arm_swap
#print axioms QCore.Bitwise.a_negated_leaf_is_its_hand_swapped_twin
#print axioms QCore.Bitwise.a_double_negation_is_the_identity_lowering
#print axioms QCore.Bitwise.the_negation_obeys_de_morgan

-- 5. Neutrality against the connective card: on a negation-free tree this
-- lowering **is** `QCore.Connectives.lower`, operation for operation, and a
-- tree has a previous spelling exactly when it is negation-free — so the card
-- adds a constructor and moves nothing.
#print axioms QCore.Bitwise.a_negation_free_tree_lowers_exactly_as_before
#print axioms QCore.Bitwise.a_tree_has_a_previous_spelling_exactly_when_it_is_negation_free

-- 6. Correctness, against `BranchJoin`'s own interpreter: however the tree
-- nests — through a negation now included — the lowered branch observes what
-- the arm the ordinary truth table selects observes.
#print axioms QCore.Bitwise.the_lowering_is_the_truth_table
#print axioms QCore.Bitwise.a_negation_observes_the_other_arm

-- 7. Fail-closed through a negation. An arm swap is the same pair of arms in
-- the other order and the join is symmetric, so nothing can be laundered by
-- negating a condition: the conditional leak, the move-versus-release shape and
-- the `return` inside an arm all still refuse, collected as one list so a
-- widening must change a theorem, with the agreeing-arms positive control
-- beside them.
#print axioms QCore.Bitwise.a_conditional_leak_refuses_through_a_negation
#print axioms QCore.Bitwise.moving_in_one_arm_and_releasing_in_the_other_refuses_through_a_negation
#print axioms QCore.Bitwise.agreeing_arms_are_admitted_through_a_negation
#print axioms QCore.Bitwise.a_return_in_an_arm_is_admitted_through_a_negation
#print axioms QCore.Bitwise.refused_negation_shapes_are_exactly_these

-- 8. The declared rankings, and the rule for declaring one: **a precedence is
-- declared only where it agrees with C.** `&` tighter than `^` tighter than
-- `|`, `!` tighter than `&&` tighter than `||` — all C's own order, each pinned
-- against the intended grouping (equal) and the other grouping (different) so
-- none can pass vacuously. The homogeneous-chain finding card `OR-AND-LOWERING`
-- recorded for the connectives is carried over to the integers: for one
-- operator the grouping is unobservable, so the declaration is load-bearing
-- only where the operators mix.
#print axioms QCore.Bitwise.the_bitwise_ranking_is_cs_own
#print axioms QCore.Bitwise.the_xor_level_sits_between_the_other_two
#print axioms QCore.Bitwise.a_homogeneous_bitwise_chain_carries_the_same_value_under_either_grouping
#print axioms QCore.Bitwise.the_two_groupings_are_distinct_trees
#print axioms QCore.Bitwise.the_bitwise_operators_are_left_associative
#print axioms QCore.Bitwise.the_negation_binds_tighter_than_the_connectives
#print axioms QCore.Bitwise.and_still_binds_tighter_than_or

-- 9. …and the two refusals that are the other half of that rule. C ranks `!`
-- tighter and `|`, `^`, `&` looser than the comparisons; a tier in which a
-- comparison yields a condition and a condition is never an integer operand
-- must rank them the other way. Rather than silently redefine
-- `flags | MASK == 0`, the ungrouped spelling is refused — and the grouped one
-- is admitted, so each refusal is about the grouping and not about the
-- operator. The sort discipline is refused in both directions, a comparison is
-- non-associative, and the whole surface is one list.
#print axioms QCore.Bitwise.an_ungrouped_bitwise_comparison_operand_is_refused
#print axioms QCore.Bitwise.an_ungrouped_negation_operand_is_refused
#print axioms QCore.Bitwise.the_two_sorts_do_not_coerce
#print axioms QCore.Bitwise.a_comparison_chain_is_refused
#print axioms QCore.Bitwise.a_double_negation_parses
#print axioms QCore.Bitwise.refused_condition_shapes_are_exactly_these

-- 10. The check-time constant bindings, the decidable subset of
-- `IF-CONDITION-BINDINGS`. A bound name parses as its value, so the condition
-- that reads a binding **is** the condition that writes the literal out and the
-- binding contributes no operation. The custody rule is stated at the position
-- it protects: a resource identity is not an integer, and the refusal is about
-- custody rather than about the environment being empty.
#print axioms QCore.Bitwise.a_bound_name_is_its_value
#print axioms QCore.Bitwise.a_bound_name_is_an_integer_operand
#print axioms QCore.Bitwise.a_resource_name_is_not_an_integer
#print axioms QCore.Bitwise.the_custody_refusal_is_about_custody
#print axioms QCore.Bitwise.a_binding_contributes_no_operation
