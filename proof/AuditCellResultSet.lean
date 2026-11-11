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

import QCore.CellResultSet

/-! Audited companion to CELL-RESULT-SET-ALIGNMENT (gateway rule 38f as generalised; user decisions 2026-11-04, "Basically, Q1(a) and Q2(i) ...", "Write [1, MAX]", "(α) live_set suffix", "Accept the leak"): a per-end sentinel's cell carries a live set and an absent set; a dead edge reads "absent" only when it meets the live set nowhere and a live edge reads "live" only when it meets the absent set nowhere, each sound for every value; without declared sets the reading is rule 38f's; the declared [1, MAX] decides the `> 0` dead edge while POSIX's [0, MAX] leaves descriptor 0 on it; the Rust set vectors hold. -/

#print axioms QCore.CellResultSet.meets_of_mem
#print axioms QCore.CellResultSet.the_dead_edge_reading_is_sound
#print axioms QCore.CellResultSet.the_live_edge_reading_is_sound
#print axioms QCore.CellResultSet.without_sets_the_reading_is_rule_38f
#print axioms QCore.CellResultSet.the_declared_bound_decides_the_positive_dead_edge
#print axioms QCore.CellResultSet.posix_leaves_descriptor_zero_on_the_positive_dead_edge
#print axioms QCore.CellResultSet.set_vectors_hold
