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

import QCore.BranchArms

-- Card `IF-ARMS`: arm exits and the continuation hoisting they require.
--
-- Card `IF-STATEMENT` refused a `return` inside a branch arm on the ground that
-- "a return is the last operation of a program". This module is the semantics
-- of removing that restriction, built on `QCore.BranchJoin`'s own operations,
-- validator and interpreter — so these theorems are about the model that card
-- proved rather than a paraphrase of it.

-- 1. Termination: which sequences end the function on every path. This is the
-- flag the join needs, since an arm that ends the function does not reach the
-- join at all. The mixed shape — one arm ending, one falling through — is
-- exactly what the hoist below removes.
#print axioms QCore.BranchArms.a_return_terminates
#print axioms QCore.BranchArms.a_branch_terminates_when_both_arms_do
#print axioms QCore.BranchArms.the_missing_arm_falls_through

-- 2. An arm exit carries the program exit's obligations, because it *is* the
-- program exit: one `ret` case of one step function, demanding an empty live
-- list wherever it stands. That is what lets the join leave a returning arm
-- out — its obligations were discharged, not deferred.
#print axioms QCore.BranchArms.the_arm_exit_is_the_program_exit
#print axioms QCore.BranchArms.an_exit_with_live_custody_refuses
#print axioms QCore.BranchArms.a_discharged_exit_succeeds

-- 3. The continuation hoisting: nothing moves for a branch with no arm exit
-- (the model counterpart of byte-identity for every program card
-- `IF-STATEMENT` already admitted), and a mixed branch with no continuation to
-- place is refused rather than joined one-sidedly.
#print axioms QCore.BranchArms.the_hoist_is_the_identity_without_an_arm_exit
#print axioms QCore.BranchArms.a_mixed_branch_with_no_continuation_refuses

-- 4. The rewrite is correct: the hoisted program observes exactly what the
-- branch followed by its continuation observes, whichever arm the decided
-- condition selects. The supporting lemmas are stated too, because the whole
-- argument rests on them: interpreting a concatenation, and the fact that a
-- terminating arm never falls out of its branch.
#print axioms QCore.BranchArms.interp_append
#print axioms QCore.BranchArms.a_terminating_operation_observes
#print axioms QCore.BranchArms.a_terminating_sequence_observes
#print axioms QCore.BranchArms.the_hoist_preserves_the_observation

-- 5. …and it is a *move*: the rewritten program has exactly as many operations
-- as the program it came from, so no acquisition is spent twice, no call takes
-- two positions in the flat call sequence and no usage demand is doubled. This
-- is why the shape that would need the continuation in two arms is refused
-- rather than served by duplication.
#print axioms QCore.BranchArms.appendSeq_size
#print axioms QCore.BranchArms.the_hoist_moves_statements_without_duplicating_them

-- 6. Concrete programs: the canonical early return in both spellings, observing
-- the same thing and exiting through the arm's own `return`; a statement after a
-- branch every arm of which returns; and a mixed branch whose *surviving* arm
-- leaks, which refuses at the program's own exit. Collected as one refused list
-- and one admitted list, so a further widening must change a theorem.
#print axioms QCore.BranchArms.the_early_return_is_admitted
#print axioms QCore.BranchArms.the_unhoisted_spelling_is_admitted_by_the_survivor_join
#print axioms QCore.BranchArms.the_hoisted_program_observes_what_its_source_does
#print axioms QCore.BranchArms.the_early_return_observes_its_own_exit
#print axioms QCore.BranchArms.an_operation_after_a_returning_branch_refuses
#print axioms QCore.BranchArms.a_leaking_survivor_refuses
#print axioms QCore.BranchArms.refused_arm_shapes_are_exactly_these
#print axioms QCore.BranchArms.admitted_arm_shapes_are_exactly_these
