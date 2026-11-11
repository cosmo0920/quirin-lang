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

import QCore.BoundsTransport

/-! Audited companion to CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (gateway rules 14h-14j). -/

#print axioms QCore.BoundsTransport.the_empty_table_changes_nothing
#print axioms QCore.BoundsTransport.its_own_body_keeps_it
#print axioms QCore.BoundsTransport.a_refuted_declaration_is_withdrawn
#print axioms QCore.BoundsTransport.a_withdrawal_never_accuses
#print axioms QCore.BoundsTransport.a_caller_of_a_refuted_declaration_is_never_safe
#print axioms QCore.BoundsTransport.mem_bounds_table_iff
#print axioms QCore.BoundsTransport.the_table_is_stable_across_phases
#print axioms QCore.BoundsTransport.a_tip_is_in_force_iff
#print axioms QCore.BoundsTransport.an_out_of_force_tip_is_held
#print axioms QCore.BoundsTransport.a_tip_assumption_is_listed_iff_in_force
#print axioms QCore.BoundsTransport.a_composition_would_omit_the_requested_rows
#print axioms QCore.BoundsTransport.a_bounds_run_is_never_composed_from_checkpoints
#print axioms QCore.BoundsTransport.tip_vectors_hold
#print axioms QCore.BoundsTransport.tip_vectors_count
