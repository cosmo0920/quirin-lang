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

import QCore.CountedUsers

/-! Card **W45-FB-RETRY-LIFETIME**: counted users (`resource_user_*`,
`resource_user_exit`, contract specification §C04.7, gateway constraint 10e).

Four directions are audited.

**Counted-reference preservation.** A path's net count is the sum of its
operations' summaries over any split of the path, and without an opaque call
it is additions minus drops; an opaque call anywhere leaves it unknown.

**A failure arm with a positive residual is not `held`.** A clause stating
`restored` on a returned value is refuted by any path returning that value
with a positive count.

**`held` is earned.** A held clause means every path returning its value has a
known count equal to the clause's; an opaque call on such a path keeps the
clause from holding.

**The release check and the card's shapes.** A drop below the entry count with
no re-addition or check is pending at the exit (C3); a rollback of the path's
own addition owes no check; C2 and N2 are refuted and a balanced body holds.

Path enumeration, attribution, the frame premise UC-FRAME and the Rust
correspondence are trust boundary 28's premises, pinned by
`crates/qcore-c-gateway/tests/retry_lifetime.rs`. -/

-- Counted-reference preservation.
#print axioms QCore.CountedUsers.delta_append
#print axioms QCore.CountedUsers.delta_counts
#print axioms QCore.CountedUsers.an_opaque_call_leaves_the_count_unknown

-- A failure arm with a positive residual is not `held`.
#print axioms QCore.CountedUsers.a_positive_residual_refutes_a_restored_clause
#print axioms QCore.CountedUsers.a_positive_residual_is_not_held

-- `held` is earned.
#print axioms QCore.CountedUsers.combine_held
#print axioms QCore.CountedUsers.held_means_every_returning_path_agrees
#print axioms QCore.CountedUsers.an_opaque_call_is_never_held

-- The release check and the card's shapes.
#print axioms QCore.CountedUsers.an_unchecked_drop_below_entry_is_pending
#print axioms QCore.CountedUsers.a_rollback_owes_no_check
#print axioms QCore.CountedUsers.a_readdition_discharges_the_check
#print axioms QCore.CountedUsers.a_check_discharges_the_check
#print axioms QCore.CountedUsers.the_c2_shape_is_refuted
#print axioms QCore.CountedUsers.the_c2_success_arm_holds
#print axioms QCore.CountedUsers.the_n2_shape_is_refuted
#print axioms QCore.CountedUsers.the_n2_rollback_owes_no_check
#print axioms QCore.CountedUsers.a_balanced_body_holds

-- W46 bounded Fluent Bit helper classification: neutral calls preserve count
-- and pending release checks; the unmodeled flush lifecycle stays unknown.
#print axioms QCore.CountedUsers.a_neutral_helper_preserves_delta
#print axioms QCore.CountedUsers.a_neutral_helper_preserves_pending
#print axioms QCore.CountedUsers.an_opaque_flush_after_neutral_is_unknown
#print axioms QCore.CountedUsers.five_routing_queries_preserve_delta
#print axioms QCore.CountedUsers.opaque_routing_after_queries_remains_unknown
