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

import QCore.ScalarReadIdentity
#print axioms QCore.ScalarReadIdentity.vectors_match
#print axioms QCore.ScalarReadIdentity.admission_needs_every_fact
#print axioms QCore.ScalarReadIdentity.pointer_destination_refuses
#print axioms QCore.ScalarReadIdentity.scalar_result_has_no_resource_identity
#print axioms QCore.ScalarReadIdentity.every_other_binding_survives
#print axioms QCore.ScalarReadIdentity.checked_read_errors_survive
#print axioms QCore.ScalarReadIdentity.all_other_state_survives
#print axioms QCore.ScalarReadIdentity.neither_scalar_comparison_outcome_is_removed
#print axioms QCore.ScalarReadIdentity.original_alias_is_retained
#print axioms QCore.ScalarReadIdentity.every_typed_alias_is_retained
