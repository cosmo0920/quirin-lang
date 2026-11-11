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

import QCore.ScalarTasks
#print axioms QCore.ScalarTasks.capture_admitted_iff
#print axioms QCore.ScalarTasks.capture_refuses_another_type
#print axioms QCore.ScalarTasks.capture_preserves_value
#print axioms QCore.ScalarTasks.fixedScalar_crosses_only_by_copy
#print axioms QCore.ScalarTasks.captured_operand_raises_under_permission
#print axioms QCore.ScalarTasks.captured_operand_refused_without_permission
#print axioms QCore.ScalarTasks.raised_variant_is_raisable
#print axioms QCore.ScalarTasks.conversion_raise_is_raisable
#print axioms QCore.ScalarTasks.join_returns_the_completion
#print axioms QCore.ScalarTasks.join_consumes_the_completion
#print axioms QCore.ScalarTasks.join_leaves_other_tasks
#print axioms QCore.ScalarTasks.worker_error_reaches_creator_unchanged
#print axioms QCore.ScalarTasks.worker_conversion_error_reaches_creator_unchanged
#print axioms QCore.ScalarTasks.worker_value_reaches_creator
#print axioms QCore.ScalarTasks.taskAgreement_holds
