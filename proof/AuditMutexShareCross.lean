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

import QCore.MutexShareCross
import QCore.Transferability

-- Card `MUTEX-SHARE-CROSS` (wave 32): CT46's composition — *"Known shared
-- wrapper with non-Sync but Send payload"*, expected **"Accept only through
-- verified mutex protocol"**. A `protected N` payload crosses into a child
-- task because a verified mutex protocol protects it, and refuses when it
-- crosses any other way.
--
-- The soundness half is proved against `QCore.MutexInvariant`'s **existing**
-- executable model, unedited by this card. That is the card's discipline
-- rather than an economy: a proof that had to add a rule to the mutex model
-- would be evidence that the composition had not happened.

-- The obligation the judgment's fifth row carries, and the rule that answers
-- for it. Printed as a surface so a later weakening is visible here rather
-- than only inside a proof.
#print axioms QCore.MutexShareCross.MutexShareObligation
#print axioms QCore.MutexShareCross.mutexShareObligationHolds

-- **The card's theorem.** A payload crossing under this route is, at every
-- point of every interleaving, reachable only through the protocol: any
-- admitted access to a footprint a live generation protects names a guard,
-- and that guard is one the accessing task holds over the very generation
-- that protects the footprint. The three conjuncts are the three ways the
-- property could fail — reaching without naming a guard, naming a guard
-- someone else holds, and naming a guard over a different mutex.
#print axioms QCore.MutexShareCross.aSharedPayloadIsReachableOnlyThroughTheProtocol

-- The same statement lifted along an arbitrary admitted schedule. `pre` may
-- interleave the creator's statements and the child's in any admitted order,
-- so "at every point of every interleaving" is the theorem's own quantifier
-- rather than a reading of it.
#print axioms QCore.MutexShareCross.everyReachAlongAnAdmittedTraceHoldsTheProtocol

-- Why the protocol is worth crossing under: the generation admits one holder
-- at a time, and only that holder releases it. Together with the theorem
-- above this is CT46's expected answer in full — the payload is reachable
-- only through the generation, and the generation orders the reachers.
#print axioms QCore.MutexShareCross.aSecondAcquisitionOfALiveGenerationIsRefused
#print axioms QCore.MutexShareCross.onlyTheHolderReleasesTheGeneration

-- The crossing itself. The spawn hypothesis is deliberately unused: the rule
-- does not depend on the crossing having happened, because a mutex share
-- parks no authority with the child and grants it only the right to acquire.
#print axioms QCore.MutexShareCross.aSpawnedChildReachesThePayloadOnlyThroughTheProtocol
#print axioms QCore.MutexShareCross.theCreatorAndTheChildAreHeldToOneRule

-- The judgment's fifth row, mirroring `qcore_core::transfer`. The protocol is
-- reachable for exactly one kind and requested by exactly one mode, which is
-- what makes "refuses when it crosses any other way" quantified rather than
-- exemplified.
#print axioms QCore.Transferability.theMutexProtocolIsReachableForExactlyOneKind
#print axioms QCore.Transferability.eachProtocolIsRequestedByExactlyOneMode
#print axioms QCore.Transferability.theMutexPayloadCrossesOnlyUnderTheMutexMode
#print axioms QCore.Transferability.aCrossProtocolRequestNamesTheKindsOwnProtocol
#print axioms QCore.Transferability.theMutexPayloadRefusesBothLendFamilies

-- The half of CT46's gap that stays open, stated so it cannot be quietly
-- assumed closed: the task route's Buffer has no mutex to cross under, and
-- refuses with its own sharing discipline named. Residual
-- `MUTEX-SHARE-CROSS-TASK-BUFFER`.
#print axioms QCore.Transferability.theTaskBuffersMutexGapStaysOpen

-- The grown matrices, decided whole, and the partition theorem card
-- `CT47-COPY-CAPTURE` established — re-decided over 16 x 6 x 16 rather than
-- weakened, so adding an admitting row did not move the refused set of any
-- cell this card did not touch.
#print axioms QCore.Transferability.theAdmissionMatrixIsPinned
#print axioms QCore.Transferability.theCaptureMatrixIsPinned
#print axioms QCore.Transferability.theBoundaryIsDecidedExactlyOnce
#print axioms QCore.Transferability.refusalComplementsAdmission
#print axioms QCore.Transferability.aWriteCaptureIsRefusedByName
#print axioms QCore.Transferability.everyAdmittedTransferIsBacked
