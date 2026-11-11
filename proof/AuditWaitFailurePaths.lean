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

import QCore.WaitFailurePaths

#print axioms QCore.WaitFailurePaths.exact_inventory_and_findings_are_accepted
#print axioms QCore.WaitFailurePaths.missing_creation_path_is_rejected
#print axioms QCore.WaitFailurePaths.registration_failure_cannot_be_promoted_to_unsafe
#print axioms QCore.WaitFailurePaths.success_observation_cannot_discharge_the_inventory
#print axioms QCore.WaitFailurePaths.every_unsafe_finding_has_harness_evidence
#print axioms QCore.WaitFailurePaths.registration_failure_remains_unknown
#print axioms QCore.WaitFailurePaths.successful_observation_remains_unknown
