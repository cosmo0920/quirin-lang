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

import QCore.BoundsPinHold

/-! Audited companion to CHUNKIO-META-LEN-BOUNDS-CONTRACTS (gateway rule 12r). -/

#print axioms QCore.BoundsPinHold.in_force_iff
#print axioms QCore.BoundsPinHold.changed_clause_never_in_force
#print axioms QCore.BoundsPinHold.changed_body_never_in_force
#print axioms QCore.BoundsPinHold.no_lock_never_in_force
#print axioms QCore.BoundsPinHold.another_identity_keeps
#print axioms QCore.BoundsPinHold.repin_in_force
#print axioms QCore.BoundsPinHold.standing_resolved_iff
#print axioms QCore.BoundsPinHold.a_mismatch_is_held
#print axioms QCore.BoundsPinHold.a_declaration_is_never_a_source_fact
#print axioms QCore.BoundsPinHold.judge_never_accuses
#print axioms QCore.BoundsPinHold.pin_mismatch_never_safe
#print axioms QCore.BoundsPinHold.clear_not_held
#print axioms QCore.BoundsPinHold.clear_not_violation
#print axioms QCore.BoundsPinHold.repinned_pending_is_safe
#print axioms QCore.BoundsPinHold.decision_vectors_hold
#print axioms QCore.BoundsPinHold.decision_vectors_count
#print axioms QCore.BoundsPinHold.standing_vectors_hold
#print axioms QCore.BoundsPinHold.standing_vectors_count
