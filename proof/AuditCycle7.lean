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

import QCore.Lowering

-- This audit prints the axiom dependencies of every Cycle 7 safety theorem.
-- Run it with: lake env lean AuditCycle7.lean
#print axioms QCore.Lowering.source_cfg_simulation
#print axioms QCore.Lowering.error_edge_has_cleanup_label
#print axioms QCore.Lowering.partial_initialization_gets_exact_cleanup
#print axioms QCore.Lowering.cleanup_release_ids
#print axioms QCore.Lowering.cleanup_releases_exactly_live_resources
#print axioms QCore.Lowering.cleanup_releases_in_lifo_order
#print axioms QCore.Lowering.cleanup_releases_each_identity_at_most_once
#print axioms QCore.Lowering.cfg_c_simulation
#print axioms QCore.Lowering.c_lowering_simulation
#print axioms QCore.Lowering.normal_value_and_transfer_are_preserved
#print axioms QCore.Lowering.error_tag_and_cleanup_trace_are_preserved
