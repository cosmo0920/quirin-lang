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

import QCore.JoinedProvenanceCycles
#print axioms QCore.JoinedProvenanceCycles.fixed_point_is_sound
#print axioms QCore.JoinedProvenanceCycles.weakening_stays_sound
#print axioms QCore.JoinedProvenanceCycles.claiming_nothing_is_sound
#print axioms QCore.JoinedProvenanceCycles.meet_satisfies_the_inequality
#print axioms QCore.JoinedProvenanceCycles.acyclic_solution_is_unique
#print axioms QCore.JoinedProvenanceCycles.steps_increase_rank
#print axioms QCore.JoinedProvenanceCycles.an_off_cycle_node_does_not_return
