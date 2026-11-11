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

import QCore.ScalarEpochTasks

-- Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-TYPES-EPOCH-TASKS`): fixed-width tasks
-- on the composed epoch route.

-- The derived shape reads no scalar type, so the plan–ledger agreement is the
-- `i32` program's.
#print axioms QCore.ScalarEpochTasks.clause_ignores_the_type
#print axioms QCore.ScalarEpochTasks.retains_ignores_the_type
#print axioms QCore.ScalarEpochTasks.the_derived_shape_is_type_blind
#print axioms QCore.ScalarEpochTasks.agreement_is_type_blind

-- The value statement's two error paths: a worker's raise is a join-failure
-- edge; an admitted creator exit abandons no child credit.
#print axioms QCore.ScalarEpochTasks.a_raising_worker_is_a_join_failure_edge
#print axioms QCore.ScalarEpochTasks.an_admitted_creator_exit_abandons_no_child_credit
#print axioms QCore.ScalarEpochTasks.an_exit_with_a_child_outstanding_is_refused
