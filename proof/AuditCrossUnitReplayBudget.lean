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

import QCore.CrossUnitReplayBudget

/-! Card **CURRENT-RUN-WORKER-SPIN** (rule 4c, amended): a caller holds only
the cross-unit homes it may reach and at most the per-unit replay budget.
The import work is linear in the units, holding fewer homes never supplies a
summary, and within the budget every name the caller's analysis can consult
reads what the full table supplies.

Card **LINK-STANDING-IMPORT-LIMITERS** (rules 4c and 40, amended): above the
budget a caller holds a subset of at most the budget, drawn from its
reachable homes; a held home reads what the full table supplies and a
withheld one reads `Unknown`; under a derived resolution a member-named home
with no token in the caller's unit is never held and only shrinks the
scope. -/

#print axioms QCore.CrossUnitReplayBudget.union_length_ge
#print axioms QCore.CrossUnitReplayBudget.addGroups_length
#print axioms QCore.CrossUnitReplayBudget.fill_length
#print axioms QCore.CrossUnitReplayBudget.select_within_budget
#print axioms QCore.CrossUnitReplayBudget.held_within_budget
#print axioms QCore.CrossUnitReplayBudget.replay_work_bound
#print axioms QCore.CrossUnitReplayBudget.addGroups_mem
#print axioms QCore.CrossUnitReplayBudget.fill_mem
#print axioms QCore.CrossUnitReplayBudget.select_sub_reachable
#print axioms QCore.CrossUnitReplayBudget.held_is_a_run_home
#print axioms QCore.CrossUnitReplayBudget.within_budget_holds_reachable
#print axioms QCore.CrossUnitReplayBudget.unprobed_is_never_held
#print axioms QCore.CrossUnitReplayBudget.probing_shrinks_reachable
#print axioms QCore.CrossUnitReplayBudget.restricted_lookup_is_full_or_absent
#print axioms QCore.CrossUnitReplayBudget.held_lookup_is_full
#print axioms QCore.CrossUnitReplayBudget.withheld_import_call_is_unknown
#print axioms QCore.CrossUnitReplayBudget.consulted_lookup_unchanged
#print axioms QCore.CrossUnitReplayBudget.scope_vectors_hold
