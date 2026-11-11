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

import QCore.HypothesisGraph
#print axioms QCore.HypothesisGraph.edges_decrease
#print axioms QCore.HypothesisGraph.paths_decrease
#print axioms QCore.HypothesisGraph.graph_acyclic
#print axioms QCore.HypothesisGraph.warrant_changes_no_support
#print axioms QCore.HypothesisGraph.incompatible_context_refuses
#print axioms QCore.HypothesisGraph.open_atom_never_promotes
#print axioms QCore.HypothesisGraph.refuted_atom_never_promotes
#print axioms QCore.HypothesisGraph.legacy_erasure
#print axioms QCore.HypothesisGraph.erasing_publication_preserves_verdict

#print axioms QCore.HypothesisGraph.composition_support_needs_every_atom

#print axioms QCore.HypothesisGraph.inserting_open_atom_never_promotes
#print axioms QCore.HypothesisGraph.inserting_refuted_atom_never_promotes

#print axioms QCore.HypothesisGraph.graph_composition_requires_valid_graph
#print axioms QCore.HypothesisGraph.invalid_graph_never_discharges
#print axioms QCore.HypothesisGraph.valid_graph_composition_is_acyclic
