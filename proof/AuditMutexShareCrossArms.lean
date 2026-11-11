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

import QCore.MutexShareCrossArms

-- Card `MUTEX-SHARE-CROSS-ARMS` (wave 35), discharging `STABILITY-REEVAL` #1's
-- residual `MUTEX-SHARE-CROSS-UNEXERCISED-ARMS`.
--
-- Wave 32's `MUTEX-SHARE-CROSS` report stated, under a heading naming the
-- `STABILITY-REEVAL` cadence, that "there are no unexercised arms in the card's
-- own rules". Wave 34 measured four. Two are driven by a named test
-- (`crates/qcore-frontend/tests/mutex_share_cross_arms.rs`); the two below are
-- unreachable from any program, and are kept as fail-closed backstops on the
-- strength of these theorems rather than on the strength of a green suite.
--
-- This is the `RETURN-POSITION-ESCAPE-UNREACHABLE` treatment: for each arm, one
-- theorem that it is dead today and one that says what it would catch if its
-- premise dropped. An arm retired by assertion would be the same defect over
-- again in the other direction.

-- **Arm C**, the `Ok(other)` protocol residue. The mutex mode's admitting arm
-- constructs its protocol as a literal, so an admitted capture is the exact
-- value the preceding arm matches, for every kind in the vocabulary. The
-- second theorem is the same fact in the shape the Rust `match` is written in.
#print axioms QCore.MutexShareCrossArms.theProtocolResidueIsEmpty
#print axioms QCore.MutexShareCrossArms.theOkOtherArmIsUnreachable

-- Non-vacuity, for both arms at once: both unreachability theorems are
-- implications, and a judgment that refused every kind would prove them while
-- meaning the admitting path was dead too. It is not.
#print axioms QCore.MutexShareCrossArms.theAdmittingCaseExists

-- Why arm C is **kept**. Under the pre-card pass-through judgment — quoted
-- from `transfer.rs`'s own comment — an `own<Buffer>` is admitted as a share
-- under `countedReadShares`, and `receivedKind` does not intercept it, because
-- it reads the transfer's constructor and not its protocol. Arm C is the only
-- refusal between that value and an unprotected crossing.
#print axioms QCore.MutexShareCrossArms.thePassThroughJudgmentWouldReachTheProtocolArm
#print axioms QCore.MutexShareCrossArms.theCardsLiteralIsWhatEmptiesTheResidue

-- **Arm D**, the payload lookup that cannot miss — the non-local half. No kind
-- but the protected payload reaches the lookup at all, so the name the
-- lowering looks up is one the same map's `contains_key` has already answered
-- for. The local half is three lines of Rust control flow and is argued in the
-- card's report, not here.
#print axioms QCore.MutexShareCrossArms.theLookupAlwaysHits

-- Why arm D is **kept**, and the honest statement of how thin its death is:
-- the judgment reads the kind's class and nothing else, so arm D is dead by
-- *uniqueness alone* and by nothing structural. A second mutex-bearing kind —
-- `FB-SYSTEM-EXTENSIONS`' lock-order work — admits exactly as the payload does
-- while naming no protected resource, and arm D is the refusal that catches it.
#print axioms QCore.MutexShareCrossArms.theClassFormIsTheJudgment
#print axioms QCore.MutexShareCrossArms.theJudgmentCannotTellTwoKindsOfOneClassApart
#print axioms QCore.MutexShareCrossArms.armDLeansOnUniqueness
