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

import QCore.ScalarCallKillPolicy

/-! Card **LINK-RECORD-KILL-POLICY-SCALAR-CALLS**: rule 46's kill policy for
scalar-only calls (gateway rule 46, amended in place; shared by rule 91).

Three directions are audited.

**The refined policy is sound.** Under the frame premise (a contracted call
writes only what its actuals, its nested calls' actuals and its written
place may denote; a scalar denotes nothing, an undecided actual anything),
the refined reading over-approximates the call's writes, and the nested
calls' reading over-approximates theirs, so rule 46's facts and rule 91's
ready records it keeps still hold.

**It removes no kill it should not.** Every call the old policy decided is
decided with the same reach; a call with an undecided actual or a nested
call with a non-place actual is still read as writing everything; a kill
is removed only when no actual is undecided, every nested actual is a
place, and the call has a nested call or a scalar actual.

**The Rust vectors and the measured shape.** `decidedBefore` and `decided`
agree with `KILL_VECTORS`; `flb_errno()`'s shape is decided only now.

Faithful classification of the actuals (premises T6 and N10), a place's
root binding over-approximating what it denotes, the frame premise and the
Rust correspondence are trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/link_member_premises.rs`. -/

-- The refined policy is sound.
#print axioms QCore.ScalarCallKillPolicy.the_refined_policy_is_sound
#print axioms QCore.ScalarCallKillPolicy.the_nested_kill_is_sound
#print axioms QCore.ScalarCallKillPolicy.the_refined_policy_keeps_every_fact
#print axioms QCore.ScalarCallKillPolicy.the_refined_policy_keeps_every_ready_record

-- It removes no kill it should not.
#print axioms QCore.ScalarCallKillPolicy.every_old_reach_is_kept
#print axioms QCore.ScalarCallKillPolicy.a_pointer_carrying_call_keeps_its_kill
#print axioms QCore.ScalarCallKillPolicy.a_kill_is_removed_only_for_scalar_calls

-- The Rust vectors and the measured shape.
#print axioms QCore.ScalarCallKillPolicy.the_errno_report_is_decided
#print axioms QCore.ScalarCallKillPolicy.vectors_hold
