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

import QCore.MonkeyWaitSourcePolicy

#print axioms QCore.MonkeyWaitSourcePolicy.exact_fixture_is_accepted
#print axioms QCore.MonkeyWaitSourcePolicy.accepted_has_pinned_revision
#print axioms QCore.MonkeyWaitSourcePolicy.mixed_artifact_vector_is_rejected
#print axioms QCore.MonkeyWaitSourcePolicy.changed_selected_path_is_rejected
#print axioms QCore.MonkeyWaitSourcePolicy.missing_summary_is_rejected
#print axioms QCore.MonkeyWaitSourcePolicy.changed_call_sequence_is_rejected
#print axioms QCore.MonkeyWaitSourcePolicy.changed_contract_clause_is_rejected
