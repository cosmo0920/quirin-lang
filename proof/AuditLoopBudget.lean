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

import QCore.LoopBudget

-- Card `LOOP-UNROLL-BUDGET`: the three-layer unrolling budget of
-- `source.loop.v1` — an absolute ceiling, a default budget, and a declared
-- licence checked against the bound the checker computes.

-- The decision is total: five answers, no partiality, no default case, and a
-- sixth answer would have to edit a theorem.
#print axioms QCore.LoopBudget.the_verdicts_are_exactly_these
#print axioms QCore.LoopBudget.the_decision_is_total

-- Layer one. The load-bearing statement: whatever a program declares, an
-- admitted loop's bound is inside the ceiling — proved over the declaration
-- rather than assumed about it. With the refusing reading of the same fact, the
-- ordering that makes a ceiling a ceiling, and the exact boundary from both
-- sides.
#print axioms QCore.LoopBudget.the_default_is_inside_the_ceiling
#print axioms QCore.LoopBudget.every_admitted_bound_is_inside_the_ceiling
#print axioms QCore.LoopBudget.a_declaration_cannot_raise_the_ceiling
#print axioms QCore.LoopBudget.the_ceiling_is_judged_before_the_bound
#print axioms QCore.LoopBudget.the_largest_declaration_licenses_the_whole_ceiling
#print axioms QCore.LoopBudget.nothing_above_the_largest_declaration_is_read

-- Layer two: the default budget, both sides of its boundary.
#print axioms QCore.LoopBudget.the_default_needs_no_declaration
#print axioms QCore.LoopBudget.a_bound_above_the_default_needs_one

-- Layer three: the declaration is checked, never trusted, and its condition is
-- strict — so the boundary is exactly where the spelling says it is.
#print axioms QCore.LoopBudget.the_declared_condition_is_checked
#print axioms QCore.LoopBudget.the_declared_condition_is_strict
#print axioms QCore.LoopBudget.a_bound_below_the_declared_limit_is_licensed

-- Invariant 1's shape for this change: the card only refuses. Every bound the
-- three-layer decision admits was already admitted by the single ceiling it
-- replaces; the second theorem makes that non-vacuous in the other direction,
-- and the third shows the declaration is what gives the refused program back.
#print axioms QCore.LoopBudget.the_budget_admits_nothing_loop_v1_refused
#print axioms QCore.LoopBudget.the_budget_refuses_something_loop_v1_admitted
#print axioms QCore.LoopBudget.a_declaration_recovers_it

-- Erasure: an attributed program and an unattributed one present the checker
-- the same events, cost the same and run to the same ledger. The emission half
-- of the same claim is the Rust byte comparison.
#print axioms QCore.LoopBudget.the_licence_erases_from_the_unrolling
#print axioms QCore.LoopBudget.the_licence_moves_no_event
#print axioms QCore.LoopBudget.the_licence_changes_no_cost
#print axioms QCore.LoopBudget.an_attributed_program_unrolls_to_its_unattributed_one

-- The one-pass peak: the repair the cost measurement forced, stated abstractly
-- over any stepper and any measure. One pass gives the maximum over every
-- prefix, so the rescan and the pass are the same computation with different
-- costs.
#print axioms QCore.LoopBudget.the_one_pass_maximum_is_the_maximum_over_every_prefix
#print axioms QCore.LoopBudget.the_maximum_is_a_value_the_run_passed_through
