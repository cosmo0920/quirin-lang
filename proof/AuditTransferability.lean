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

import QCore.Transferability

-- Card `CT46-SENDSYNC`: which kinds may cross a task-creation boundary, derived
-- from the kind rather than enumerated per profile. Model of
-- `qcore_core::transfer`, which the frontend consults beside its existing
-- capture enumeration under a fail-closed agreement check.
--
-- Outside every statement below and recorded in
-- `docs/reports/remaining-ct46-sendsync/README.md`: data-race freedom,
-- source-to-native refinement and C layout. Mutex-mediated sharing was on
-- that list too until card `MUTEX-SHARE-CROSS` (wave 32) admitted it for
-- `Kind.protectedResource` alone; its own surface is
-- `AuditMutexShareCross.lean`. The rules the admitting arms name are proved
-- in `QCore.ScopedTasks`, `QCore.ReadShare`, `QCore.WriteLendTasks` and
-- `QCore.MutexShareCross`, not here.

-- The three vocabularies are closed and the inventories the quantified
-- statements range over really are complete.
#print axioms QCore.Transferability.kindInventoryIsComplete
#print axioms QCore.Transferability.kindInventoryHasNoDuplicates
#print axioms QCore.Transferability.kindInventoryHasSeventeenEntries
#print axioms QCore.Transferability.tagInventoryIsComplete
#print axioms QCore.Transferability.tagInventoryHasNoDuplicates
#print axioms QCore.Transferability.tagInventoryHasFifteenEntries
#print axioms QCore.Transferability.modeInventoryIsComplete
#print axioms QCore.Transferability.modeInventoryHasSixEntries

-- Totality over the declared-type grammar, and its deliberate conservatism: a
-- one-token tag never claims sendability, because only the elaborated spelling
-- can decide it.
#print axioms QCore.Transferability.everyTagMapsIntoTheVocabulary
#print axioms QCore.Transferability.noDeclaredTagIsSendableOnItsOwn

-- The classification itself, and CT46's two directions. Two kinds are
-- sendable but not freely shareable — the owned Buffer and (card
-- `MUTEX-SHARE-CROSS`) the checked-mutex route's protected payload — and they
-- share under *different* protocols, which is what keeps the disciplines from
-- being interchangeable. The "shared wrapper" vocabulary — guards, facts,
-- mutex generations — still never crosses.
#print axioms QCore.Transferability.theClassificationIsPinned
#print axioms QCore.Transferability.exactlyOneKindIsSendableButNotFreelyShareable
#print axioms QCore.Transferability.guardsFactsAndMutexesNeverCross

-- Both admission matrices decided whole: 16 x 6 and 16 x 6 x 16 since card
-- `MUTEX-SHARE-CROSS` (14 x 5 x 14 after `WRITE-LEND-TASKS`, 13 x 4 x 13
-- before it), six admitting cells each, with the write keyword still refused
-- for every kind.
#print axioms QCore.Transferability.theAdmissionMatrixIsPinned
#print axioms QCore.Transferability.noKindAdmitsAWriteCapture
#print axioms QCore.Transferability.theCaptureMatrixIsPinned
#print axioms QCore.Transferability.captureAdmissionRefinesKindAdmission

-- Internal coherence: an admission never contradicts the class it came from,
-- refusal and non-admission coincide exactly, and an admitted lend names the
-- protocol its own class names.
#print axioms QCore.Transferability.everyAdmissionIsSendable
#print axioms QCore.Transferability.refusalIsExactlyNonAdmission
#print axioms QCore.Transferability.anAdmittedLendNamesItsClassProtocol

-- The recorded gap, **half closed by card `MUTEX-SHARE-CROSS`** (wave 32).
-- `noModeRequestsTheMutexProtocol` and `theGapKindIsSendableAndOtherwiseShareable`
-- were this section's two entries; a mutex-mediated capture mode now exists,
-- for `Kind.protectedResource` and that kind alone, so they are replaced by
-- their two successors rather than deleted. The task route's `ownedBuffer`
-- still has no mutex to cross under, which is the half that stays open
-- (`MUTEX-SHARE-CROSS-TASK-BUFFER`). The card's own surface is in
-- `AuditMutexShareCross.lean`.
#print axioms QCore.Transferability.theMutexProtocolIsReachableForExactlyOneKind
#print axioms QCore.Transferability.theTaskBuffersMutexGapStaysOpen

-- Card `CT47-COPY-CAPTURE`: the refusal reasons, decided beside the
-- admissions. Refusal and admission partition both matrices — every cell
-- decided exactly once — so moving the boundary refusal's attribution from
-- the backend catch-all to the judgment cannot have moved the refused set;
-- a write capture is refused by its own name, a never-kind by its class's
-- reason, CT47's copy-of-a-Buffer cell for its mode with the kind still
-- sendable, and a delivered-kind mismatch as its own reason.
#print axioms QCore.Transferability.refusalComplementsAdmission
#print axioms QCore.Transferability.theBoundaryIsDecidedExactlyOnce
#print axioms QCore.Transferability.aWriteCaptureIsRefusedByName
#print axioms QCore.Transferability.aNeverKindRefusesWithItsClassReason
#print axioms QCore.Transferability.copyOfTheOwnedBufferIsRefusedForItsMode
#print axioms QCore.Transferability.aDeliveredKindMismatchIsItsOwnReason

-- Per-kind soundness: each admitting arm names a ledger rule that is already
-- proved, and each refusing arm carries no obligation because it admits
-- nothing. The move arm rests on reservation rollback and parent
-- unavailability; the lend arm on exclusive authority being suspended while
-- any share is outstanding.
#print axioms QCore.Transferability.moveObligationHolds
#print axioms QCore.Transferability.lendObligationHolds
#print axioms QCore.Transferability.everyAdmittedTransferIsBacked
#print axioms QCore.Transferability.aRefusingKindNeedsNoRule
#print axioms QCore.Transferability.duplicationCarriesNoCustody

-- Card `WRITE-LEND-TASKS`: the judgment's fourth admissible row. The grown
-- vocabularies stay decided whole (both matrices re-pinned above, with
-- `write` still refused by name for every kind); the mutable-sharing
-- direction separates the Buffer under the byte-partitioning protocol and
-- (card `MUTEX-SHARE-CROSS`) the protected payload under the mutex, since a
-- guard holder may write bytes another task will later reach; and the lease
-- arm is backed by the write-lease
-- ledger model's own exclusivity theorem — no byte has two write authorities
-- at any point of the interleaving.
#print axioms QCore.Transferability.mutablyShareableUnder
#print axioms QCore.Transferability.exactlyOneKindIsMutablyShareable
#print axioms QCore.Transferability.leaseObligationHolds

-- Card `W48-QR-TASK-RECORD-CAPTURES`: the aggregate kind, decided from its
-- leaves. Both judgment theorems re-decided over records rather than weakened
-- (the flat ones above are unchanged); the card's own surface is
-- `AuditTaskRecordCaptures.lean`.
#print axioms QCore.Transferability.theAggregateJudgmentIsTheConjunctionOfItsLeaves
#print axioms QCore.Transferability.theAggregateBoundaryIsDecidedExactlyOnce
#print axioms QCore.Transferability.theBoundaryIsDecidedExactlyOnceOverRecords
#print axioms QCore.Transferability.aWriteCaptureOfARecordIsRefusedByName
#print axioms QCore.Transferability.aWriteCaptureIsRefusedByNameOverRecords
