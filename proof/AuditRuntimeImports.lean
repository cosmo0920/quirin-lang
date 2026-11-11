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

import QCore.RuntimeImports
#print axioms QCore.RuntimeImports.snapshot_overrides_disk
#print axioms QCore.RuntimeImports.absent_snapshot_and_disk_refuses
#print axioms QCore.RuntimeImports.staging_preserves_retained_source
#print axioms QCore.RuntimeImports.nonempty_runtime_imports_refuse_standalone
#print axioms QCore.RuntimeImports.contract_dependency_refuses
#print axioms QCore.RuntimeImports.explicit_executable_dependency_refuses
#print axioms QCore.RuntimeImports.admitted_edge_has_decreasing_rank
#print axioms QCore.RuntimeImports.admitted_graph_checks_every_edge
#print axioms QCore.RuntimeImports.admitted_path_decreases_rank
#print axioms QCore.RuntimeImports.admitted_graph_has_no_cycle
#print axioms QCore.RuntimeImports.admitted_io_requires_own_import
#print axioms QCore.RuntimeImports.neighbour_cannot_grant_io
#print axioms QCore.RuntimeImports.admitted_capability_requires_own_import
#print axioms QCore.RuntimeImports.absent_target_refuses
