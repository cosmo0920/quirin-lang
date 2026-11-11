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

import QCore.ExecutableModel

-- Print the foundational dependencies of every Cycle 8 safety theorem.
-- Run with: lake env lean AuditCycle8.lean
#print axioms QCore.ExecutableModel.progress_up_to_typed_error
#print axioms QCore.ExecutableModel.preservation
#print axioms QCore.ExecutableModel.steps_preservation
#print axioms QCore.ExecutableModel.no_duplication_of_resource_identities
#print axioms QCore.ExecutableModel.no_double_release
#print axioms QCore.ExecutableModel.region_cleanup_theorem
