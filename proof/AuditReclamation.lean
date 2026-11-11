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

import QCore.Reclamation

/-! Audited companion to CG29-EXECUTION-COVERAGE-AND-RECLAMATION (gateway rule
11z): hazard-pointer and epoch-based reclamation protocols (gap entry W10). -/
#print axioms QCore.Reclamation.hp_init_inv
#print axioms QCore.Reclamation.hazardFree_spec
#print axioms QCore.Reclamation.hp_step_preserves
#print axioms QCore.Reclamation.hp_access_safe
#print axioms QCore.Reclamation.hp_accepted_safe
#print axioms QCore.Reclamation.hp_trace_safe
#print axioms QCore.Reclamation.ebr_init_inv
#print axioms QCore.Reclamation.quiescent_spec
#print axioms QCore.Reclamation.ebr_step_preserves
#print axioms QCore.Reclamation.ebr_access_safe
#print axioms QCore.Reclamation.ebr_accepted_safe
#print axioms QCore.Reclamation.ebr_trace_safe
#print axioms QCore.Reclamation.hp_vectors_hold
#print axioms QCore.Reclamation.ebr_vectors_hold
#print axioms QCore.Reclamation.hp_use_after_retire_refused
