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

import QCore.SymbolicOverlapAdmission
#print axioms QCore.SymbolicOverlapAdmission.valid_matching_metadata_preserves_original_guard
#print axioms QCore.SymbolicOverlapAdmission.failed_bounds_loader_refuses
#print axioms QCore.SymbolicOverlapAdmission.failed_resource_loader_refuses
#print axioms QCore.SymbolicOverlapAdmission.failed_original_graph_match_refuses
#print axioms QCore.SymbolicOverlapAdmission.failed_erasure_refuses
#print axioms QCore.SymbolicOverlapAdmission.changed_flow_entry_refuses
#print axioms QCore.SymbolicOverlapAdmission.changed_lifecycle_entry_refuses
#print axioms QCore.SymbolicOverlapAdmission.one_sided_flow_refuses
#print axioms QCore.SymbolicOverlapAdmission.metadata_does_not_prove_legacy_support
#print axioms QCore.SymbolicOverlapAdmission.pure_null_preserves_original_guard
#print axioms QCore.SymbolicOverlapAdmission.pure_null_graph_introduction_refuses
#print axioms QCore.SymbolicOverlapAdmission.pure_null_metadata_introduction_refuses
#print axioms QCore.SymbolicOverlapAdmission.both_cases_require_resource_validation
