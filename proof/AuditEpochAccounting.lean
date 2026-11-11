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

import QCore.EpochAccounting

#print axioms QCore.EpochAccounting.union_counts_a_repeated_cell_once
#print axioms QCore.EpochAccounting.union_ignores_a_repeated_identity
#print axioms QCore.EpochAccounting.union_is_at_most_the_double_count
#print axioms QCore.EpochAccounting.a_double_counted_cell_is_strictly_larger
#print axioms QCore.EpochAccounting.sum_dominates_each_task
#print axioms QCore.EpochAccounting.maximum_is_at_most_the_sum
#print axioms QCore.EpochAccounting.the_maximum_undercounts_two_concurrent_tasks
#print axioms QCore.EpochAccounting.live_is_at_most_the_private_peak
#print axioms QCore.EpochAccounting.live_tasks_le_private_sum
#print axioms QCore.EpochAccounting.outstanding_le_task_count
#print axioms QCore.EpochAccounting.residency_is_bounded_by_the_epoch_peak
#print axioms QCore.EpochAccounting.the_all_running_configuration_attains_the_bound
#print axioms QCore.EpochAccounting.the_ready_configuration_holds_only_shared_storage_and_epoch_overhead
#print axioms QCore.EpochAccounting.completion_reclaims_nothing
#print axioms QCore.EpochAccounting.dropping_a_completed_result_undercounts
#print axioms QCore.EpochAccounting.a_transferred_result_survives_the_join
#print axioms QCore.EpochAccounting.demands_within_credits_fit_the_allocation
#print axioms QCore.EpochAccounting.credits_beyond_the_allocation_are_not_covered
#print axioms QCore.EpochAccounting.work_total_does_not_determine_finish_time
#print axioms QCore.EpochAccounting.no_function_of_the_work_total_is_the_finish_time
