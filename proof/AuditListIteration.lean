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

import QCore.ListIteration

/-! Card **W45-FB-BACKLOG-CUSTODY**: intrusive circular-list iteration
(gateway constraint 5l, `crates/qcore-c-gateway/src/legacy_list_iteration.rs`).

Four directions are audited.

**Safe iteration under unlinking.** Unlinking the current node keeps the ring
well formed, leaves the saved successor linked, and a safe iteration that
unlinks every node visits each exactly once.

**Plain iteration refuses it.** A plain iteration that unlinks its current
node reads a cleared link; a safe one that unlinks its saved successor does.

**Rule 4 is exact on the model.** Every admitted body runs to the end of every
well-formed ring without reading a cleared link; every refused body has a
ring on which it is stuck.

**An element binding moves no custody.** Rule 3 unbinds one name and changes
no resource status.

Faithful recognition of the preprocessed macro spellings and the stated
premises (P-ILI-BODY, P-ILI-UNTRACKED, P-ILI-CALLEE-FRAME) are review premises
pinned by `crates/qcore-c-gateway/tests/intrusive_list_iteration.rs`. -/

-- Safe iteration under unlinking.
#print axioms QCore.ListIteration.unlinking_preserves_distinctness
#print axioms QCore.ListIteration.the_saved_successor_stays_linked
#print axioms QCore.ListIteration.safe_unlinking_visits_each_once

-- Plain iteration refuses it.
#print axioms QCore.ListIteration.plain_unlinking_reads_a_cleared_link
#print axioms QCore.ListIteration.unlinking_the_successor_reads_a_cleared_link

-- Rule 4 is exact on the model.
#print axioms QCore.ListIteration.plain_keep_visits
#print axioms QCore.ListIteration.safe_keep_visits
#print axioms QCore.ListIteration.plain_unlinking_the_next_node_runs
#print axioms QCore.ListIteration.an_admitted_body_never_reads_a_cleared_link
#print axioms QCore.ListIteration.a_refused_body_has_a_stuck_ring
#print axioms QCore.ListIteration.plain_refuses_unlinking_the_current_node
#print axioms QCore.ListIteration.safe_admits_unlinking_the_current_node

-- An element binding moves no custody.
#print axioms QCore.ListIteration.element_binding_moves_no_custody
