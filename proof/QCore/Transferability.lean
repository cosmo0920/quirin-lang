import QCore.ScopedTasks
import QCore.ReadShare

/-! Which kinds may cross a task-creation boundary (card `CT46-SENDSYNC`).

Model of `qcore_core::transfer`. Two things are established, and nothing else.

1. **The judgment is total and closed.** Every constructor of the boundary
   kind vocabulary, of the declared-type tag set and of the capture-mode set
   has exactly one answer, `admits` is decided on the whole 13 x 4 product and
   `admitsCapture` on the whole 13 x 4 x 13 product. There is no fallback arm
   and no residual case, so a kind nobody classified cannot slip through as an
   unlisted default.

2. **Each admitting arm is backed by a ledger rule that is already proved.**
   The obligations below are *statements about the existing ledgers*, not new
   assumptions: the move arm's obligation is discharged by
   `QCore.ScopedTasks.rollback_restores_original_custody` together with
   `reserved_resource_is_unavailable_to_parent`, and the lend arm's by
   `QCore.ReadShare.an_outstanding_share_suspends_exclusive_authority`. The
   judgment does not establish those rules; it records which rule answers for
   which kind, and the theorems here check that the recording is accurate.

What is deliberately **not** here: any claim of data-race freedom, any
source-to-native refinement, any statement about C layout, and any admission of
mutex-mediated sharing. `ShareProtocol.checkedMutex` is named so that the gap
CT46 asks about is a decidable proposition (`noModeRequestsTheMutexProtocol`)
rather than an omission. -/

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
  | threadScope
  | taskHandle
  | taskResult
  | errorValue
  | knownErrorTag
  | mutexGeneration
  | guard
  | protectedFact
  | unmodeled
  deriving DecidableEq, Repr

/-- The whole kind vocabulary, in declaration order. Totality below is stated
against this list, so a constructor missing from it is caught by
`kindInventoryIsComplete`. -/
def allKinds : List Kind :=
  [.unit, .i32, .ownedBuffer, .readLoan, .threadScope, .taskHandle, .taskResult,
   .errorValue, .knownErrorTag, .mutexGeneration, .guard, .protectedFact, .unmodeled]

/-- The declared-type tag set of the source grammar: fifteen tags, of which
this profile elaborates spellings under three. -/
inductive DeclaredTag where
  | atomic | named | tuple | array | slice | pointer | owned | borrow
  | quant | accessQuant | result | function | never | task | mutex
  deriving DecidableEq, Repr

def allTags : List DeclaredTag :=
  [.atomic, .named, .tuple, .array, .slice, .pointer, .owned, .borrow,
   .quant, .accessQuant, .result, .function, .never, .task, .mutex]

/-- The four capture keywords the parser accepts. `write` parses and has never
had a lowering, so it appears as a mode the judgment refuses for every kind
rather than as a mode that silently does not exist. -/
inductive Mode where
  | copy
  | move
  | lend
  | write
  deriving DecidableEq, Repr

def allModes : List Mode := [.copy, .move, .lend, .write]

/-- A discipline under which a kind that is not freely shareable may still be
reached by more than one task. `countedReadShares` removes mutation for the
window; `checkedMutex` would order it instead, and no mode produces it. -/
inductive Protocol where
  | countedReadShares
  | checkedMutex
  deriving DecidableEq, Repr

/-- Why a kind does not cross. -/
inductive NotTransferable where
  | notAValue
  | owedToCreatingFrame
  | unmodeledType
  | resultPositionOnly
  deriving DecidableEq, Repr

/-- The transfer class of a kind. `movedOnly` with `some p` is CT46's
"Send but not Sync, shareable under `p`" shape. -/
inductive Class where
  | duplicated
  | movedOnly (shareableUnder : Option Protocol)
  | never (reason : NotTransferable)
  deriving DecidableEq, Repr

/-- What an admitted capture delivers. -/
inductive Transfer where
  | duplicate
  | moveCustody
  | lendShare (protocol : Protocol)
  deriving DecidableEq, Repr

/-! ### The judgment -/

/-- **The judgment.** Total over `Kind` by exhaustive match. -/
def transferable : Kind → Class
  | .i32 => .duplicated
  | .unit => .never .resultPositionOnly
  | .ownedBuffer => .movedOnly (some .countedReadShares)
  | .readLoan => .never .notAValue
  | .threadScope => .never .owedToCreatingFrame
  | .taskHandle => .never .owedToCreatingFrame
  | .taskResult => .never .owedToCreatingFrame
  | .errorValue => .never .owedToCreatingFrame
  | .knownErrorTag => .never .owedToCreatingFrame
  | .mutexGeneration => .never .notAValue
  | .guard => .never .notAValue
  | .protectedFact => .never .notAValue
  | .unmodeled => .never .unmodeledType

/-- The "Send" direction: may a value of this kind cross carrying anything? -/
def sendable (c : Class) : Bool :=
  match c with
  | .duplicated => true
  | .movedOnly _ => true
  | .never _ => false

/-- The "Sync" direction: under which protocol may the parent keep it while a
child also reaches it? A duplicated kind answers `none` because duplication
leaves nothing shared. -/
def shareableUnder (c : Class) : Option Protocol :=
  match c with
  | .movedOnly p => p
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
    | .movedOnly p => match m with
      | .move => some .moveCustody
      | .lend => match p with
        | some protocol => some (.lendShare protocol)
        | none => none
      | _ => none

/-- What the child ends up holding. Lending delivers a loan, whatever it was
lent from; that asymmetry is the one place where the parent's kind and the
worker's declared parameter kind differ on purpose. -/
def receivedKind (source : Kind) : Transfer → Kind
  | .duplicate => source
  | .moveCustody => source
  | .lendShare _ => .readLoan

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

theorem kindInventoryHasThirteenEntries : allKinds.length = 13 := by decide

theorem tagInventoryIsComplete : ∀ t : DeclaredTag, t ∈ allTags := by
  intro t; cases t <;> simp [allTags]

theorem tagInventoryHasNoDuplicates : allTags.Nodup := by decide

theorem tagInventoryHasFifteenEntries : allTags.length = 15 := by decide

theorem modeInventoryIsComplete : ∀ m : Mode, m ∈ allModes := by
  intro m; cases m <;> simp [allModes]

theorem modeInventoryHasFourEntries : allModes.length = 4 := by decide

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
       (.ownedBuffer, .movedOnly (some .countedReadShares)),
       (.readLoan, .never .notAValue),
       (.threadScope, .never .owedToCreatingFrame),
       (.taskHandle, .never .owedToCreatingFrame),
       (.taskResult, .never .owedToCreatingFrame),
       (.errorValue, .never .owedToCreatingFrame),
       (.knownErrorTag, .never .owedToCreatingFrame),
       (.mutexGeneration, .never .notAValue),
       (.guard, .never .notAValue),
       (.protectedFact, .never .notAValue),
       (.unmodeled, .never .unmodeledType)] := by decide

/-- CT46's own vocabulary. Exactly two kinds cross carrying anything, and
exactly one of them is shareable under a protocol: that one is the Buffer, the
"Send but not Sync" kind the case is about. -/
theorem exactlyOneKindIsSendableButNotFreelyShareable :
    allKinds.filter (fun k => sendable (transferable k)) = [.i32, .ownedBuffer] ∧
      allKinds.filter (fun k => (shareableUnder (transferable k)).isSome) = [.ownedBuffer] ∧
      shareableUnder (transferable .ownedBuffer) = some .countedReadShares := by
  refine ⟨by decide, by decide, by decide⟩

/-- Guards, facts and mutex generations never cross, in any mode, for one
reason. This is the arm CT46's "shared wrapper" vocabulary lands on. -/
theorem guardsFactsAndMutexesNeverCross :
    ∀ k ∈ [Kind.guard, Kind.protectedFact, Kind.mutexGeneration],
      transferable k = .never .notAValue ∧ ∀ m ∈ allModes, admits k m = none := by decide

/-! ### The admission matrices, decided whole -/

/-- The 13 x 4 kind/mode matrix: exactly three cells admit. -/
theorem theAdmissionMatrixIsPinned :
    (allKinds.flatMap (fun k => allModes.map (fun m => (k, m)))).filter
        (fun p => (admits p.1 p.2).isSome) =
      [(.i32, .copy), (.ownedBuffer, .move), (.ownedBuffer, .lend)] := by decide

/-- A `write` capture is refused for every kind, the admitted ones included. -/
theorem noKindAdmitsAWriteCapture : ∀ k ∈ allKinds, admits k .write = none := by decide

set_option maxRecDepth 8000 in
/-- The 13 x 4 x 13 boundary matrix: exactly three cells admit out of 676. -/
theorem theCaptureMatrixIsPinned :
    (allKinds.flatMap (fun place =>
      allModes.flatMap (fun mode =>
        allKinds.map (fun parameter => (place, mode, parameter))))).filter
        (fun t => (admitsCapture t.1 t.2.1 t.2.2).isSome) =
      [(.i32, .copy, .i32),
       (.ownedBuffer, .move, .ownedBuffer),
       (.ownedBuffer, .lend, .readLoan)] := by decide

set_option maxRecDepth 8000 in
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
      ∀ p ∈ [Protocol.countedReadShares, Protocol.checkedMutex],
        admits k m = some (.lendShare p) →
          shareableUnder (transferable k) = some p := by decide

/-! ### The recorded gap CT46 closes at

`checkedMutex` is the discipline CT46 asks for. It is named by the judgment
and produced by nothing, which is exactly the residue: the classification half
of CT46 exists, the capture form does not. -/

/-- No kind is shareable under the mutex protocol, and no (kind, mode) pair
admits a mutex-mediated lend. Decided over the whole matrix, so this cannot
drift as a comment could. -/
theorem noModeRequestsTheMutexProtocol :
    (∀ k ∈ allKinds, shareableUnder (transferable k) ≠ some .checkedMutex) ∧
      (∀ k ∈ allKinds, ∀ m ∈ allModes,
        admits k m ≠ some (.lendShare .checkedMutex)) := by
  refine ⟨by decide, by decide⟩

/-- The gap is about a kind that really is sendable and really is shareable
under some *other* protocol — otherwise it would be an uninteresting refusal
rather than CT46's question. -/
theorem theGapKindIsSendableAndOtherwiseShareable :
    sendable (transferable .ownedBuffer) = true ∧
      shareableUnder (transferable .ownedBuffer) ≠ some .checkedMutex ∧
      (shareableUnder (transferable .ownedBuffer)).isSome := by
  refine ⟨by decide, by decide, by decide⟩

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
