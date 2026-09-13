import QCore.ScopedTasks
import QCore.ReadShare
import QCore.WriteLendTasks
import QCore.MutexShareCross

/-! Which kinds may cross a task-creation boundary (card `CT46-SENDSYNC`).

Model of `qcore_core::transfer`. Two things are established, and nothing else.

1. **The judgment is total and closed.** Every constructor of the boundary
   kind vocabulary, of the declared-type tag set and of the capture-mode set
   has exactly one answer, `admits` is decided on the whole 16 x 6 product and
   `admitsCapture` on the whole 16 x 6 x 16 product (13 x 4 x 13 originally,
   14 x 5 x 14 after card `WRITE-LEND-TASKS` grew both vocabularies by one,
   and 16 x 6 x 16 after card `MUTEX-SHARE-CROSS` grew them by two and one).
   There is no fallback arm and no residual case, so a kind nobody classified
   cannot slip through as an unlisted default.

2. **Each admitting arm is backed by a ledger rule that is already proved.**
   The obligations below are *statements about the existing ledgers*, not new
   assumptions: the move arm's obligation is discharged by
   `QCore.ScopedTasks.rollback_restores_original_custody` together with
   `reserved_resource_is_unavailable_to_parent`, and the lend arm's by
   `QCore.ReadShare.an_outstanding_share_suspends_exclusive_authority`, and
   (card `WRITE-LEND-TASKS`) the write-lease arm's by
   `QCore.WriteLendTasks.no_byte_has_two_write_authorities`, and (card
   `MUTEX-SHARE-CROSS`) the mutex-share arm's by
   `QCore.MutexShareCross.aSharedPayloadIsReachableOnlyThroughTheProtocol`.
   The judgment does not establish those rules; it records which rule answers
   for which kind, and the theorems here check that the recording is accurate.

Card `MUTEX-SHARE-CROSS` closes half of the gap CT46 named. The
mutex-mediated *capture* now exists, for `Kind.protectedResource` and that
kind alone, so this file's old `noModeRequestsTheMutexProtocol` is replaced by
`theMutexProtocolIsReachableForExactlyOneKind` and
`eachProtocolIsRequestedByExactlyOneMode`. The half that stays open is the
task route's `ownedBuffer`, which has no mutex to cross under and refuses with
its own discipline named (`theTaskBuffersMutexGapStaysOpen`).

What is deliberately **not** here: any claim of data-race freedom, any
source-to-native refinement, and any statement about C layout. Nor is the
word *verified* this file's — the classification is all that is decided here,
and whether a named generation exists and protects the payload is the source
route's gate, with the protocol's own soundness in `QCore.MutexShareCross`. -/

namespace QCore.Transferability

/-! ### The closed vocabularies -/

/-- Every kind of thing a name can denote at a task-creation boundary: the
executable scalar grammar, the lowering's binding grammar, the mutex route's
three binders (which have no value type at all), and one arm for the declared
type forms this profile does not elaborate. -/
inductive Kind where
  | unit
  | i32
  | ownedBuffer
  | readLoan
  | writeLease
  | threadScope
  | taskHandle
  | taskResult
  | errorValue
  | knownErrorTag
  /-- The checked-mutex route's `protected N` payload (card
  `MUTEX-SHARE-CROSS`): CT46's subject. Sendable, and shareable only under a
  verified mutex protocol. -/
  | protectedResource
  /-- What a mutex-mediated crossing delivers: the right to reach the payload
  under the declared generation, and nothing else. The product of a crossing,
  never a source of one. -/
  | mutexShare
  | mutexGeneration
  | guard
  | protectedFact
  | unmodeled
  deriving DecidableEq, Repr

/-- The whole kind vocabulary, in declaration order. Totality below is stated
against this list, so a constructor missing from it is caught by
`kindInventoryIsComplete`. -/
def allKinds : List Kind :=
  [.unit, .i32, .ownedBuffer, .readLoan, .writeLease, .threadScope, .taskHandle,
   .taskResult, .errorValue, .knownErrorTag, .protectedResource, .mutexShare,
   .mutexGeneration, .guard, .protectedFact, .unmodeled]

/-- The declared-type tag set of the source grammar: fifteen tags, of which
this profile elaborates spellings under three. -/
inductive DeclaredTag where
  | atomic | named | tuple | array | slice | pointer | owned | borrow
  | quant | accessQuant | result | function | never | task | mutex
  deriving DecidableEq, Repr

def allTags : List DeclaredTag :=
  [.atomic, .named, .tuple, .array, .slice, .pointer, .owned, .borrow,
   .quant, .accessQuant, .result, .function, .never, .task, .mutex]

/-- The five capture keywords the parser accepts. `write` parses and has never
had a lowering, so it appears as a mode the judgment refuses for every kind
rather than as a mode that silently does not exist; `lendWrite` (card
`WRITE-LEND-TASKS`, spelled `lend PLACE LOW..HIGH`) is deliberately distinct
from it, so the pinned `write` refusal is untouched. -/
inductive Mode where
  | copy
  | move
  | lend
  | write
  | lendWrite
  /-- `share PLACE under MUTEX` (card `MUTEX-SHARE-CROSS`): the crossing CT46
  asks for. The only mode that requests `checkedMutex`. -/
  | shareUnderMutex
  deriving DecidableEq, Repr

def allModes : List Mode :=
  [.copy, .move, .lend, .write, .lendWrite, .shareUnderMutex]

/-- A discipline under which a kind that is not freely shareable may still be
reached by more than one task. `countedReadShares` removes mutation for the
window; `checkedMutex` would order it instead, and no mode produces it;
`disjointWriteLeases` (card `WRITE-LEND-TASKS`) **partitions the bytes**, so
each party mutates only its own disjoint range. -/
inductive Protocol where
  | countedReadShares
  | checkedMutex
  | disjointWriteLeases
  deriving DecidableEq, Repr

/-- The sharing mode that requests a protocol, and only that protocol (card
`MUTEX-SHARE-CROSS`). Stating the correspondence as a function is what lets
the judgment be written once: a sharing mode is admitted exactly when the
kind's own sharing discipline is the protocol that mode requests. -/
def requestedBy : Protocol → Mode
  | .countedReadShares => .lend
  | .checkedMutex => .shareUnderMutex
  | .disjointWriteLeases => .lendWrite

def allProtocols : List Protocol :=
  [.countedReadShares, .checkedMutex, .disjointWriteLeases]

/-- Why a kind does not cross. -/
inductive NotTransferable where
  | notAValue
  | owedToCreatingFrame
  | unmodeledType
  | resultPositionOnly
  deriving DecidableEq, Repr

/-- The transfer class of a kind. `movedOnly` with `some p` in its first field
is CT46's "Send but not Sync, shareable under `p`" shape; the second field
(card `WRITE-LEND-TASKS`) names the discipline under which more than one party
may hold **write** authority at once, if any. -/
inductive Class where
  | duplicated
  | movedOnly (shareableUnder mutablyShareableUnder : Option Protocol)
  | never (reason : NotTransferable)
  deriving DecidableEq, Repr

/-- What an admitted capture delivers. -/
inductive Transfer where
  | duplicate
  | moveCustody
  | lendShare (protocol : Protocol)
  | lendWriteRange (protocol : Protocol)
  /-- Card `MUTEX-SHARE-CROSS`: the child may reach the payload under the named
  protocol, and only under it. Nothing is duplicated, no custody crosses, and
  no authority is parked with the child. -/
  | shareUnderMutex (protocol : Protocol)
  deriving DecidableEq, Repr

/-! ### The judgment -/

/-- **The judgment.** Total over `Kind` by exhaustive match. -/
def transferable : Kind → Class
  | .i32 => .duplicated
  | .unit => .never .resultPositionOnly
  | .ownedBuffer => .movedOnly (some .countedReadShares) (some .disjointWriteLeases)
  | .readLoan => .never .notAValue
  | .writeLease => .never .notAValue
  | .threadScope => .never .owedToCreatingFrame
  | .taskHandle => .never .owedToCreatingFrame
  | .taskResult => .never .owedToCreatingFrame
  | .errorValue => .never .owedToCreatingFrame
  | .knownErrorTag => .never .owedToCreatingFrame
  | .protectedResource => .movedOnly (some .checkedMutex) (some .checkedMutex)
  | .mutexShare => .never .notAValue
  | .mutexGeneration => .never .notAValue
  | .guard => .never .notAValue
  | .protectedFact => .never .notAValue
  | .unmodeled => .never .unmodeledType

/-- The "Send" direction: may a value of this kind cross carrying anything? -/
def sendable (c : Class) : Bool :=
  match c with
  | .duplicated => true
  | .movedOnly _ _ => true
  | .never _ => false

/-- The "Sync" direction: under which protocol may the parent keep it while a
child also reaches it? A duplicated kind answers `none` because duplication
leaves nothing shared. -/
def shareableUnder (c : Class) : Option Protocol :=
  match c with
  | .movedOnly p _ => p
  | .duplicated => none
  | .never _ => none

/-- The mutable-sharing direction (card `WRITE-LEND-TASKS`): under which
protocol may more than one party hold write authority at once? Distinct from
`shareableUnder` on purpose - read sharing removes mutation, mutable sharing
partitions the bytes, and a consumer must ask for exactly one of them. -/
def mutablyShareableUnder (c : Class) : Option Protocol :=
  match c with
  | .movedOnly _ p => p
  | .duplicated => none
  | .never _ => none

/-- The declared-type tag map, total over `DeclaredTag`. Deliberately
conservative: `atomic`, `owned` and `borrow` are tags whose elaborated
spellings are classified from the elaborated scalar, so the tag alone must not
claim them. -/
def ofDeclaredTag : DeclaredTag → Kind
  | .task => .taskHandle
  | .result => .taskResult
  | .mutex => .mutexGeneration
  | .atomic => .unmodeled
  | .named => .unmodeled
  | .tuple => .unmodeled
  | .array => .unmodeled
  | .slice => .unmodeled
  | .pointer => .unmodeled
  | .owned => .unmodeled
  | .borrow => .unmodeled
  | .quant => .unmodeled
  | .accessQuant => .unmodeled
  | .function => .unmodeled
  | .never => .unmodeled

/-- The operational judgment: kind against mode. A `write` capture is refused
for every kind before anything else is asked. -/
def admits (k : Kind) (m : Mode) : Option Transfer :=
  match m with
  | .write => none
  | _ =>
    match transferable k with
    | .never _ => none
    | .duplicated => match m with
      | .copy => some .duplicate
      | _ => none
    | .movedOnly p q => match m with
      | .move => some .moveCustody
      -- Card `MUTEX-SHARE-CROSS`: each sharing mode requests one named
      -- protocol and is admitted exactly when the kind's own discipline is
      -- that protocol. A kind sharing under a different one refuses.
      | .lend => match p with
        | some .countedReadShares => some (.lendShare .countedReadShares)
        | _ => none
      | .shareUnderMutex => match p with
        | some .checkedMutex => some (.shareUnderMutex .checkedMutex)
        | _ => none
      | .lendWrite => match q with
        | some .disjointWriteLeases => some (.lendWriteRange .disjointWriteLeases)
        | _ => none
      | _ => none

/-- What the child ends up holding. Lending delivers a loan, whatever it was
lent from; that asymmetry is the one place where the parent's kind and the
worker's declared parameter kind differ on purpose. -/
def receivedKind (source : Kind) : Transfer → Kind
  | .duplicate => source
  | .moveCustody => source
  | .lendShare _ => .readLoan
  | .lendWriteRange _ => .writeLease
  | .shareUnderMutex _ => .mutexShare

/-- The protocol an admitted transfer names, if it names one (card
`MUTEX-SHARE-CROSS`). A duplicate and a move name none: nothing is shared. -/
def protocolOf : Transfer → Option Protocol
  | .duplicate => none
  | .moveCustody => none
  | .lendShare p => some p
  | .lendWriteRange p => some p
  | .shareUnderMutex p => some p

/-- The whole boundary decision: the parent holds `place`, the capture asks for
`mode`, and the worker declares it receives `parameter`. All three must agree. -/
def admitsCapture (place : Kind) (mode : Mode) (parameter : Kind) : Option Transfer :=
  match admits place mode with
  | none => none
  | some t => if receivedKind place t = parameter then some t else none

/-! ### Totality of the vocabularies

The judgment is a wildcard-free match, so the compiler already forces an arm
per constructor. These theorems force the *inventories* to stay in step with
the constructors, which is what every quantified statement below ranges over. -/

theorem kindInventoryIsComplete : ∀ k : Kind, k ∈ allKinds := by
  intro k; cases k <;> simp [allKinds]

theorem kindInventoryHasNoDuplicates : allKinds.Nodup := by decide

theorem kindInventoryHasSixteenEntries : allKinds.length = 16 := by decide

theorem tagInventoryIsComplete : ∀ t : DeclaredTag, t ∈ allTags := by
  intro t; cases t <;> simp [allTags]

theorem tagInventoryHasNoDuplicates : allTags.Nodup := by decide

theorem tagInventoryHasFifteenEntries : allTags.length = 15 := by decide

theorem modeInventoryIsComplete : ∀ m : Mode, m ∈ allModes := by
  intro m; cases m <;> simp [allModes]

theorem modeInventoryHasSixEntries : allModes.length = 6 := by decide

/-- Totality of the declared-tag map: every tag lands on a classified kind. -/
theorem everyTagMapsIntoTheVocabulary (t : DeclaredTag) : ofDeclaredTag t ∈ allKinds :=
  kindInventoryIsComplete _

/-- No declared tag is sendable on its own: elaboration of the spelling
decides, never the one-token tag. -/
theorem noDeclaredTagIsSendableOnItsOwn (t : DeclaredTag) :
    sendable (transferable (ofDeclaredTag t)) = false := by
  cases t <;> rfl

/-! ### The classification, and the two directions -/

/-- The per-kind classification, decided over the whole vocabulary. -/
theorem theClassificationIsPinned :
    allKinds.map (fun k => (k, transferable k)) =
      [(.unit, .never .resultPositionOnly),
       (.i32, .duplicated),
       (.ownedBuffer,
         .movedOnly (some .countedReadShares) (some .disjointWriteLeases)),
       (.readLoan, .never .notAValue),
       (.writeLease, .never .notAValue),
       (.threadScope, .never .owedToCreatingFrame),
       (.taskHandle, .never .owedToCreatingFrame),
       (.taskResult, .never .owedToCreatingFrame),
       (.errorValue, .never .owedToCreatingFrame),
       (.knownErrorTag, .never .owedToCreatingFrame),
       (.protectedResource, .movedOnly (some .checkedMutex) (some .checkedMutex)),
       (.mutexShare, .never .notAValue),
       (.mutexGeneration, .never .notAValue),
       (.guard, .never .notAValue),
       (.protectedFact, .never .notAValue),
       (.unmodeled, .never .unmodeledType)] := by decide

/-- CT46's own vocabulary. Exactly two kinds cross carrying anything, and
exactly one of them is shareable under a protocol: that one is the Buffer, the
"Send but not Sync" kind the case is about. -/
theorem exactlyOneKindIsSendableButNotFreelyShareable :
    allKinds.filter (fun k => sendable (transferable k))
        = [.i32, .ownedBuffer, .protectedResource] ∧
      allKinds.filter (fun k => (shareableUnder (transferable k)).isSome)
        = [.ownedBuffer, .protectedResource] ∧
      shareableUnder (transferable .ownedBuffer) = some .countedReadShares ∧
      shareableUnder (transferable .protectedResource) = some .checkedMutex ∧
      shareableUnder (transferable .ownedBuffer)
        ≠ shareableUnder (transferable .protectedResource) := by
  refine ⟨by decide, by decide, by decide, by decide, by decide⟩

/-- Card `WRITE-LEND-TASKS`: the mutable-sharing direction separates exactly
the same kind, under exactly the byte-partitioning protocol. -/
theorem exactlyOneKindIsMutablyShareable :
    allKinds.filter (fun k => (mutablyShareableUnder (transferable k)).isSome)
        = [.ownedBuffer, .protectedResource] ∧
      mutablyShareableUnder (transferable .ownedBuffer) = some .disjointWriteLeases ∧
      mutablyShareableUnder (transferable .protectedResource) = some .checkedMutex := by
  refine ⟨by decide, by decide, by decide⟩

/-- Guards, facts and mutex generations never cross, in any mode, for one
reason. This is the arm CT46's "shared wrapper" vocabulary lands on. -/
theorem guardsFactsAndMutexesNeverCross :
    ∀ k ∈ [Kind.guard, Kind.protectedFact, Kind.mutexGeneration],
      transferable k = .never .notAValue ∧ ∀ m ∈ allModes, admits k m = none := by decide

/-! ### The admission matrices, decided whole -/

/-- The 14 x 5 kind/mode matrix: exactly four cells admit. -/
theorem theAdmissionMatrixIsPinned :
    (allKinds.flatMap (fun k => allModes.map (fun m => (k, m)))).filter
        (fun p => (admits p.1 p.2).isSome) =
      [(.i32, .copy), (.ownedBuffer, .move), (.ownedBuffer, .lend),
       (.ownedBuffer, .lendWrite), (.protectedResource, .move),
       (.protectedResource, .shareUnderMutex)] := by decide

/-- A `write` capture is refused for every kind, the admitted ones included. -/
theorem noKindAdmitsAWriteCapture : ∀ k ∈ allKinds, admits k .write = none := by decide

set_option maxRecDepth 16000 in
/-- The 14 x 5 x 14 boundary matrix: exactly four cells admit out of 980. -/
theorem theCaptureMatrixIsPinned :
    (allKinds.flatMap (fun place =>
      allModes.flatMap (fun mode =>
        allKinds.map (fun parameter => (place, mode, parameter))))).filter
        (fun t => (admitsCapture t.1 t.2.1 t.2.2).isSome) =
      [(.i32, .copy, .i32),
       (.ownedBuffer, .move, .ownedBuffer),
       (.ownedBuffer, .lend, .readLoan),
       (.ownedBuffer, .lendWrite, .writeLease),
       (.protectedResource, .move, .protectedResource),
       (.protectedResource, .shareUnderMutex, .mutexShare)] := by decide

set_option maxRecDepth 16000 in
/-- The boundary decision never admits more than the kind judgment, and what it
admits is exactly what the kind judgment admitted. -/
theorem captureAdmissionRefinesKindAdmission :
    ∀ place ∈ allKinds, ∀ mode ∈ allModes, ∀ parameter ∈ allKinds,
      ∀ t, admitsCapture place mode parameter = some t →
        admits place mode = some t ∧ receivedKind place t = parameter := by decide

/-- An admission implies the class is sendable — never the other way round. -/
theorem everyAdmissionIsSendable :
    ∀ k ∈ allKinds, ∀ m ∈ allModes,
      (admits k m).isSome → sendable (transferable k) = true := by decide

/-- Refusal and non-admission coincide exactly: a kind admits nothing in any
mode precisely when its class is a refusal. Stated as an equality of decisions
so neither direction can be lost. -/
theorem refusalIsExactlyNonAdmission :
    ∀ k ∈ allKinds,
      decide (sendable (transferable k) = false) =
        allModes.all (fun m => (admits k m).isNone) := by decide

/-- An admitted lend names exactly the protocol its class names. -/
theorem anAdmittedLendNamesItsClassProtocol :
    ∀ k ∈ allKinds, ∀ m ∈ allModes,
      ∀ p ∈ [Protocol.countedReadShares, .checkedMutex, .disjointWriteLeases],
        admits k m = some (.lendShare p) →
          shareableUnder (transferable k) = some p := by decide

/-! ### The gap CT46 closes at, and the half card `MUTEX-SHARE-CROSS` closes

`checkedMutex` is the discipline CT46 asks for. Card `CT46-SENDSYNC` named it
and produced it from nothing, and this section's theorem was
`noModeRequestsTheMutexProtocol`. Card `MUTEX-SHARE-CROSS` supplies the capture
mode, for `protectedResource` and for that kind alone, so the theorem is
**replaced** by the two facts that took its place: the protocol is requested by
exactly one mode, and reachable for exactly one kind. The task route's
`ownedBuffer` still has no mutex to cross under, which is the half that stays
open (`MUTEX-SHARE-CROSS-TASK-BUFFER`). -/

/-- Every protocol is requested by exactly one mode, and no other mode anywhere
in the matrix reaches it. This is the property that makes "refuses when it
crosses any other way" quantified rather than exemplified: the disciplines are
not interchangeable, so a payload cannot cross under a protocol that does not
govern it. -/
theorem eachProtocolIsRequestedByExactlyOneMode :
    ∀ p ∈ [Protocol.countedReadShares, .checkedMutex, .disjointWriteLeases],
      ∀ k ∈ allKinds, ∀ m ∈ allModes,
        ((admits k m).bind protocolOf = some p) → m = requestedBy p := by decide

/-- The mutex protocol is reachable for exactly one kind, by exactly one mode,
and it delivers a mutex share rather than a loan or a lease. No kind is *lent*
under a mutex — card `CT46-SENDSYNC`'s shape survives, which is what keeps the
read-share ledger from being asked to carry a mutex. -/
theorem theMutexProtocolIsReachableForExactlyOneKind :
    allKinds.filter (fun k => shareableUnder (transferable k) = some .checkedMutex)
        = [.protectedResource] ∧
      (allKinds.flatMap (fun k => allModes.map (fun m => (k, m)))).filter
          (fun q => admits q.1 q.2 = some (.shareUnderMutex .checkedMutex))
        = [(.protectedResource, .shareUnderMutex)] ∧
      (∀ k ∈ allKinds, ∀ m ∈ allModes,
        admits k m ≠ some (.lendShare .checkedMutex) ∧
          admits k m ≠ some (.lendWriteRange .checkedMutex)) := by
  refine ⟨by decide, by decide, by decide⟩

/-- The half that stays open, stated so it cannot be quietly assumed closed:
the task route's Buffer really is sendable and really is shareable, under a
protocol that is **not** the mutex — so a `share ... under ...` of it does not
admit. The refusal's *reason* is pinned in the refusal section below. -/
theorem theTaskBuffersMutexGapStaysOpen :
    sendable (transferable .ownedBuffer) = true ∧
      (shareableUnder (transferable .ownedBuffer)).isSome ∧
      shareableUnder (transferable .ownedBuffer) ≠ some .checkedMutex ∧
      admits .ownedBuffer .shareUnderMutex = none := by
  refine ⟨by decide, by decide, by decide, by decide⟩

/-- The mirror image: the mutex payload does not admit either lend family, so
neither sharing route can be reached by spelling the other. -/
theorem theMutexPayloadRefusesBothLendFamilies :
    admits .protectedResource .lend = none ∧
      admits .protectedResource .lendWrite = none := by
  refine ⟨by decide, by decide⟩

/-! ### The refusal reasons, decided beside the admissions
    (card `CT47-COPY-CAPTURE`)

Model of `qcore_core::transfer::TransferRefusal` and of the decision order the
frontend's boundary refusal now surfaces: since card `CT47-COPY-CAPTURE`, a
capture the judgment refuses is attributed **by the judgment** at the crossing
construct (`Q600`, a condition widening stated in spec §L21) with a message
built from exactly this reason vocabulary — no longer by falling through to
the backend's executable-subset catch-all. The theorems establish that the
reasons partition the boundary against the admissions: every cell of both
matrices is decided exactly once, so moving the attribution cannot have moved
the refused set. -/

/-- Why a capture is refused, mirroring `TransferRefusal`. -/
inductive Refusal where
  | kindNotTransferable (reason : NotTransferable)
  | modeNotAdmittedForKind
  | protocolUnavailable (protocol : Protocol)
  | writeCaptureUnimplemented
  | receivedKindMismatch
  deriving DecidableEq, Repr

/-- The refusing half of the operational judgment, in the same decision order
as `admits`: `write` is refused by name before anything else is asked, a
non-transferable kind refuses with its class's own reason, and a transferable
kind in a mode it does not carry refuses for the mode. -/
def refuse (k : Kind) (m : Mode) : Option Refusal :=
  match m with
  | .write => some .writeCaptureUnimplemented
  | _ =>
    match transferable k with
    | .never reason => some (.kindNotTransferable reason)
    | .duplicated => match m with
      | .copy => none
      | _ => some .modeNotAdmittedForKind
    | .movedOnly p q => match m with
      | .move => none
      | .lend => match p with
        | some .countedReadShares => none
        | some other => some (.protocolUnavailable other)
        | none => some (.protocolUnavailable .checkedMutex)
      | .shareUnderMutex => match p with
        | some .checkedMutex => none
        | some other => some (.protocolUnavailable other)
        | none => some (.protocolUnavailable .checkedMutex)
      | .lendWrite => match q with
        | some .disjointWriteLeases => none
        | some other => some (.protocolUnavailable other)
        | none => some .modeNotAdmittedForKind
      | _ => some .modeNotAdmittedForKind

/-- The refusing half of the whole boundary decision: an admitted transfer
whose delivered kind is not the declared parameter is a mismatch, and an
unadmitted one carries `refuse`'s own reason. -/
def refuseCapture (place : Kind) (mode : Mode) (parameter : Kind) : Option Refusal :=
  match admits place mode with
  | none => refuse place mode
  | some t => if receivedKind place t = parameter then none else some .receivedKindMismatch

/-- Refusal and admission partition the 13 x 4 matrix: every cell is decided,
and decided exactly once. This is what makes moving the refusal's attribution
(card `CT47-COPY-CAPTURE`) incapable of moving the refused set. -/
theorem refusalComplementsAdmission :
    ∀ k ∈ allKinds, ∀ m ∈ allModes, (refuse k m).isSome = (admits k m).isNone := by decide

set_option maxRecDepth 8000 in
/-- The same partition over the whole 13 x 4 x 13 boundary matrix. -/
theorem theBoundaryIsDecidedExactlyOnce :
    ∀ place ∈ allKinds, ∀ mode ∈ allModes, ∀ parameter ∈ allKinds,
      (refuseCapture place mode parameter).isSome =
        (admitsCapture place mode parameter).isNone := by decide

/-- A `write` capture is refused by its own name for every kind — the reason
is the mode's missing lowering, never the kind. -/
theorem aWriteCaptureIsRefusedByName :
    ∀ k ∈ allKinds, refuse k .write = some .writeCaptureUnimplemented := by decide

/-- A kind that never crosses refuses with its class's own reason in every
non-write mode, so the boundary message can name the kind's rule rather than a
generic subset. -/
theorem aNeverKindRefusesWithItsClassReason :
    ∀ k ∈ allKinds, ∀ m ∈ [Mode.copy, .move, .lend, .lendWrite, .shareUnderMutex],
      ∀ reason ∈ [NotTransferable.notAValue, .owedToCreatingFrame,
                  .unmodeledType, .resultPositionOnly],
        transferable k = .never reason → refuse k m = some (.kindNotTransferable reason) := by
  decide

/-- **CT47's cell.** A `copy` of the owned Buffer — the nearest expressible
shape to the case's raw-pointer bit-copy — is refused *for its mode*: the kind
is sendable, so the refusal is exactly "no transfer/share contract admits
duplication of affine custody", not a vacuous kind refusal and not a backend
statement. -/
theorem copyOfTheOwnedBufferIsRefusedForItsMode :
    refuse .ownedBuffer .copy = some .modeNotAdmittedForKind ∧
      sendable (transferable .ownedBuffer) = true := ⟨rfl, rfl⟩

/-- Card `MUTEX-SHARE-CROSS`: **a cross-protocol request refuses with the
protocol the kind actually carries, named.** That is what makes the refusal
usable — a reader is told which discipline governs the payload, not merely that
this one does not. Both directions are pinned, so neither sharing route can be
reached by spelling the other, and the mutex payload's own cross-requests
refuse too. -/
theorem aCrossProtocolRequestNamesTheKindsOwnProtocol :
    refuse .ownedBuffer .shareUnderMutex
        = some (.protocolUnavailable .countedReadShares) ∧
      refuse .protectedResource .lend = some (.protocolUnavailable .checkedMutex) ∧
      refuse .protectedResource .lendWrite = some (.protocolUnavailable .checkedMutex) := by
  refine ⟨by decide, by decide, by decide⟩

/-- **CT46's cell**, the admitting one, stated beside CT47's refusing cell
above. The mutex payload crosses under the mutex mode, delivering a mutex share
— and the same payload refuses under every other mode of the matrix, which is
"accept only through verified mutex protocol" as a decision rather than a
convention. The word *verified* is not this theorem's: the classification is
all that is decided here, and the protocol's verification is
`QCore.MutexShareCross`. -/
theorem theMutexPayloadCrossesOnlyUnderTheMutexMode :
    admitsCapture .protectedResource .shareUnderMutex .mutexShare
        = some (.shareUnderMutex .checkedMutex) ∧
      allModes.filter (fun m => (admits .protectedResource m).isSome)
        = [.move, .shareUnderMutex] ∧
      (∀ parameter ∈ allKinds, parameter ≠ Kind.mutexShare →
        admitsCapture .protectedResource .shareUnderMutex parameter = none) := by
  refine ⟨by decide, by decide, by decide⟩

/-- The two delivered-kind mismatches the boundary can spell: a lend delivered
as custody and a move delivered as a loan. Both transfers are admitted; the
refusal is the declaration's. -/
theorem aDeliveredKindMismatchIsItsOwnReason :
    refuseCapture .ownedBuffer .lend .ownedBuffer = some .receivedKindMismatch ∧
      refuseCapture .ownedBuffer .move .readLoan = some .receivedKindMismatch := ⟨rfl, rfl⟩

/-! ### Per-kind soundness: each admitting arm names a proved ledger rule

The obligations below are statements about the *existing* ledgers. Nothing
here proves them — `QCore.ScopedTasks` and `QCore.ReadShare` do — and nothing
here weakens them. What is established is that the judgment's two admitting
arms are backed by rules that hold, and that its refusing arms carry no
obligation because they admit nothing. -/

/-- The obligation a `moveCustody` admission carries: a failed creation
restores the parent's binding exactly, and while the reservation stands the
parent cannot use the resource. Both halves come from the scoped-task ledger. -/
def MoveObligation : Prop :=
  (∀ {before reserved : ScopedTasks.Custody} {resources : List Nat} {parent reservation : Nat},
      ScopedTasks.transfer before resources (.task parent) (.reservation reservation)
          = some reserved →
        ScopedTasks.transfer reserved resources (.reservation reservation) (.task parent)
          = some before) ∧
  (∀ {before reserved : ScopedTasks.Custody} {resources : List Nat} {parent reservation : Nat},
      ScopedTasks.transfer before resources (.task parent) (.reservation reservation)
          = some reserved →
        ∀ (phases : Nat → ScopedTasks.Phase) (resource : Nat), resource ∈ resources →
          ScopedTasks.canUse phases reserved parent resource = false)

/-- Discharged entirely by the scoped-task ledger's own theorems. -/
theorem moveObligationHolds : MoveObligation :=
  ⟨fun checked => ScopedTasks.rollback_restores_original_custody checked,
   fun checked phases resource member =>
     ScopedTasks.reserved_resource_is_unavailable_to_parent checked phases resource member⟩

/-- The obligation a `lendWriteRange disjointWriteLeases` admission carries
(card `WRITE-LEND-TASKS`): at any point of any spawn/join interleaving — any
lease-ledger state reachable by admitted opens and closes — no byte has two
write authorities: at most one outstanding lease holds it, and the owner is an
authority exactly when nothing is outstanding. That is what makes concurrent
mutation safe without a mutex: the protocol partitions the bytes. -/
def LeaseObligation : Prop :=
  ∀ {state : List WriteLendTasks.Lease}, WriteLendTasks.Reachable state →
    ∀ byte : Nat, WriteLendTasks.writeAuthorities state byte ≤ 1

/-- Discharged entirely by the write-lease ledger model's own theorem. -/
theorem leaseObligationHolds : LeaseObligation :=
  fun reachable byte => WriteLendTasks.no_byte_has_two_write_authorities reachable byte

/-- The obligation a `lendShare countedReadShares` admission carries: while any
share is outstanding, exclusive authority — and therefore the Buffer write and
the release — is refused for everyone. That is what makes read sharing safe
without a mutex: the protocol removes mutation for the window. -/
def LendObligation : Prop :=
  ∀ (custody : ReadShare.Custody) {resource owner : Nat}
    {shares : List (ReadShare.Holder × Nat)},
    custody resource = some (.loaned owner shares) → ∀ task : Nat,
      ReadShare.canUseExclusively custody task resource = false ∧
        ReadShare.canWrite custody task resource = false ∧
        ReadShare.release custody resource task = none

/-- Discharged entirely by the counted-share ledger's own theorem. -/
theorem lendObligationHolds : LendObligation := by
  intro custody resource owner shares lent task
  exact ReadShare.an_outstanding_share_suspends_exclusive_authority custody lent task

/-- The rule that answers for one admitted transfer. `checkedMutex` maps to
`False` because nothing admits it — the arm exists so that admitting one later
would demand a rule, not a comment. -/
def Backed : Transfer → Prop
  | .duplicate => True
  | .moveCustody => MoveObligation
  | .lendShare .countedReadShares => LendObligation
  | .lendShare .checkedMutex => False
  | .lendShare .disjointWriteLeases => False
  | .lendWriteRange .disjointWriteLeases => LeaseObligation
  | .lendWriteRange .countedReadShares => False
  | .lendWriteRange .checkedMutex => False
  -- Card `MUTEX-SHARE-CROSS`: the mutex-mediated crossing's rule, in the same
  -- record-which-rule-answers discipline as the three arms above. The other
  -- two protocols map to `False` because no kind shares under them this way.
  | .shareUnderMutex .checkedMutex => MutexShareCross.MutexShareObligation
  | .shareUnderMutex .countedReadShares => False
  | .shareUnderMutex .disjointWriteLeases => False

/-- Every admitting arm of the judgment is backed by an obligation that holds.
Stated over the whole matrix: whichever cell admits, the corresponding rule is
available. -/
theorem everyAdmittedTransferIsBacked :
    ∀ k ∈ allKinds, ∀ m ∈ allModes, ∀ t, admits k m = some t → Backed t := by
  intro k _ m _ t admitted
  cases k <;> cases m <;>
    simp [admits, transferable] at admitted <;>
    subst admitted <;>
    first
      | trivial
      | exact moveObligationHolds
      | exact lendObligationHolds
      | exact leaseObligationHolds
      | exact MutexShareCross.mutexShareObligationHolds

/-- The refusing arms carry no obligation, because they admit nothing at all —
so the judgment never leans on a rule it did not name. -/
theorem aRefusingKindNeedsNoRule :
    ∀ k ∈ allKinds, sendable (transferable k) = false →
      ∀ m ∈ allModes, admits k m = none := by decide

/-- The duplicated arm needs no custody rule because a copy splits no custody:
it is the only admitted transfer whose obligation is trivially `True`, and it
is admitted for exactly one kind. -/
theorem duplicationCarriesNoCustody :
    allKinds.filter (fun k => decide (transferable k = .duplicated)) = [.i32] := by decide

end QCore.Transferability
