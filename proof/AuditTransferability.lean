import QCore.Transferability

-- Card `CT46-SENDSYNC`: which kinds may cross a task-creation boundary, derived
-- from the kind rather than enumerated per profile. Model of
-- `qcore_core::transfer`, which the frontend consults beside its existing
-- capture enumeration under a fail-closed agreement check.
--
-- Outside every statement below and recorded in
-- `docs/reports/remaining-ct46-sendsync/README.md`: data-race freedom,
-- source-to-native refinement, C layout, and any admission of mutex-mediated
-- sharing. The ledger rules the admitting arms name are proved in
-- `QCore.ScopedTasks` and `QCore.ReadShare`, not here.

-- The three vocabularies are closed and the inventories the quantified
-- statements range over really are complete.
#print axioms QCore.Transferability.kindInventoryIsComplete
#print axioms QCore.Transferability.kindInventoryHasNoDuplicates
#print axioms QCore.Transferability.kindInventoryHasThirteenEntries
#print axioms QCore.Transferability.tagInventoryIsComplete
#print axioms QCore.Transferability.tagInventoryHasNoDuplicates
#print axioms QCore.Transferability.tagInventoryHasFifteenEntries
#print axioms QCore.Transferability.modeInventoryIsComplete
#print axioms QCore.Transferability.modeInventoryHasFourEntries

-- Totality over the declared-type grammar, and its deliberate conservatism: a
-- one-token tag never claims sendability, because only the elaborated spelling
-- can decide it.
#print axioms QCore.Transferability.everyTagMapsIntoTheVocabulary
#print axioms QCore.Transferability.noDeclaredTagIsSendableOnItsOwn

-- The classification itself, and CT46's two directions. Exactly one kind is
-- sendable but not freely shareable, and it is the owned Buffer; the "shared
-- wrapper" vocabulary — guards, facts, mutex generations — never crosses.
#print axioms QCore.Transferability.theClassificationIsPinned
#print axioms QCore.Transferability.exactlyOneKindIsSendableButNotFreelyShareable
#print axioms QCore.Transferability.guardsFactsAndMutexesNeverCross

-- Both admission matrices decided whole: 13 x 4 and 13 x 4 x 13, three
-- admitting cells each, with the write keyword refused for every kind.
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

-- The recorded gap. `checkedMutex` is named by the judgment and produced by
-- nothing, over the whole matrix — and the kind it would apply to really is
-- the sendable, otherwise-shareable one, so the gap is CT46's question rather
-- than an uninteresting refusal.
#print axioms QCore.Transferability.noModeRequestsTheMutexProtocol
#print axioms QCore.Transferability.theGapKindIsSendableAndOtherwiseShareable

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
