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

import QCore.CmetricsOptsConditions
#print axioms QCore.CmetricsOptsConditions.cleanup_preserves_raw_representation
#print axioms QCore.CmetricsOptsConditions.selected_contents_retire
#print axioms QCore.CmetricsOptsConditions.old_alias_cannot_revive_from_unchanged_pointer_bytes
#print axioms QCore.CmetricsOptsConditions.failed_append_has_no_field_commit
#print axioms QCore.CmetricsOptsConditions.append_preserves_index_representation
#print axioms QCore.CmetricsOptsConditions.nonempty_cleanup_needs_new_liveness
#print axioms QCore.CmetricsOptsConditions.exact_release_event
