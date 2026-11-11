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

import QCore.Connectives

-- Card `OR-AND-LOWERING`: the logical connectives `||` and `&&` as a
-- desugaring onto card `IF-STATEMENT`'s branch construct. Everything below is
-- stated against `QCore.BranchJoin`'s own operations, validator and
-- interpreter, so these are theorems about the model that card proved rather
-- than about a paraphrase of it.
--
-- The token retable itself — `||` and `&&` as the two logical operators, and
-- `|`, `&`, `^` reserved for future bitwise operations — is `QCore.PipeDesugaring`
-- §6 and §7, checked by `AuditPipeDesugaring.lean`.

-- 1. The identity. The lowering produces exactly the nested branch a reader
-- would have written by hand, with `handOr`/`handAnd` defined *independently*
-- of `lower`. This is the model counterpart of the card's load-bearing
-- measurement, that `a || b` emits byte-identical C to that twin.
#print axioms QCore.Connectives.or_lowers_to_its_hand_written_chain
#print axioms QCore.Connectives.and_lowers_to_its_hand_written_nesting
#print axioms QCore.Connectives.the_two_connectives_nest_oppositely

-- 2. Short-circuit, structurally: the right operand is the condition of a
-- branch nested inside an arm of the left operand's, so it is reached on
-- exactly one path. Or nests into the else arm, and into the then arm.
#print axioms QCore.Connectives.the_right_operand_of_an_or_lives_in_the_else_arm
#print axioms QCore.Connectives.the_right_operand_of_an_and_lives_in_the_then_arm

-- 3. Neutrality. A connective-free condition lowers to the branch card
-- `IF-STATEMENT` already produced — the model counterpart of the
-- byte-identical-C pin for programs that use no connective — and the claim is
-- not vacuous, because a connective-free tree is exactly a leaf.
#print axioms QCore.Connectives.a_leaf_lowers_to_the_branch_it_already_was
#print axioms QCore.Connectives.a_connective_free_tree_is_a_leaf

-- 4. Correctness. However the connectives nest, the lowered branch observes
-- what the arm the ordinary two-valued truth table selects observes: the
-- short-circuit nesting and a strict reading agree, which is exactly the claim
-- a desugaring has to make. Proved against `BranchJoin`'s interpreter.
#print axioms QCore.Connectives.only_observes_its_operation
#print axioms QCore.Connectives.the_lowering_is_the_truth_table
#print axioms QCore.Connectives.an_or_is_disjunction
#print axioms QCore.Connectives.an_and_is_conjunction

-- 5. Precedence, declared rather than inherited from a parser accident: `&&`
-- binds tighter than `||`, both are left-associative, and parentheses group.
-- The two groupings are distinct trees, so the pin is not vacuous.
#print axioms QCore.Connectives.and_binds_tighter_than_or
#print axioms QCore.Connectives.the_two_groupings_are_different_trees
#print axioms QCore.Connectives.parentheses_group
#print axioms QCore.Connectives.the_connectives_are_left_associative

-- 6. The associativity finding, measured in the Rust and explained here: for a
-- homogeneous chain the two groupings lower to the *same* nested branch, so the
-- associativity declaration is load-bearing only where the connectives mix.
-- The trees are nonetheless distinct, so this is a statement about the lowering
-- rather than about the parser.
#print axioms QCore.Connectives.or_lowers_associatively
#print axioms QCore.Connectives.and_lowers_associatively
#print axioms QCore.Connectives.the_groupings_are_distinct_trees_that_lower_alike

-- 7. Fail-closed. Every level of the nesting is an ordinary branch, so every
-- level runs card `IF-STATEMENT`'s custody join: the conditional leak through
-- either connective, and the shape where one arm moves what the other releases,
-- all still refuse. Collected as one list so a widening must change a theorem,
-- with the agreeing-arms positive control beside them — and with the whole
-- point stated once, that the connective program and its hand-written twin are
-- the *same term*.
#print axioms QCore.Connectives.an_or_conditional_leak_refuses
#print axioms QCore.Connectives.an_and_conditional_leak_refuses
#print axioms QCore.Connectives.moving_in_one_arm_and_releasing_in_the_other_refuses_through_a_connective
#print axioms QCore.Connectives.agreeing_arms_are_admitted_through_a_connective
#print axioms QCore.Connectives.refused_connective_shapes_are_exactly_these
#print axioms QCore.Connectives.the_connective_program_is_its_hand_written_twin
