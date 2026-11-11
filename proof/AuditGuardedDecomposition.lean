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

import QCore.GuardedContractApplications
import QCore.GuardedDecomposition
#print axioms QCore.GuardedDecomposition.satisfied_requires_positive_guard_and_conclusion
#print axioms QCore.GuardedDecomposition.guarded_satisfaction_sound
#print axioms QCore.GuardedDecomposition.forbidden_requires_positive_refutation
#print axioms QCore.GuardedDecomposition.absent_operation_does_not_discharge_forbidden
#print axioms QCore.GuardedDecomposition.open_guard_does_not_discharge
#print axioms QCore.GuardedDecomposition.terminal_requires_separate_postcondition
#print axioms QCore.GuardedDecomposition.every_terminal_is_checked
#print axioms QCore.GuardedDecomposition.open_terminal_remains_open
#print axioms QCore.GuardedDecomposition.changed_symbol_is_retained
#print axioms QCore.GuardedDecomposition.dependency_is_marked_for_recheck
#print axioms QCore.GuardedDecomposition.proposed_lowering_preserves_original
#print axioms QCore.GuardedContractApplications.Cjson.nonnull_prev_defeats_raw_rejection
#print axioms QCore.GuardedContractApplications.Cjson.admitted_membership_excludes_positive_invalidity
#print axioms QCore.GuardedContractApplications.Plugin.candidate_premise_decomposes_both_alternatives
#print axioms QCore.GuardedContractApplications.Plugin.lifetime_requires_its_own_premise

#print axioms QCore.GuardedDecomposition.enablement_requires_all_prerequisites
#print axioms QCore.GuardedDecomposition.missing_terminal_refuses
