namespace QCore.DescriptorCustody

/-! Generic formal boundary for socket-pair creation, descriptor close and
error queries. The executable checker supplies these branch and custody facts;
accepted evidence cannot turn a failed pair/close into a discharged resource. -/

abbrev DescriptorId := Nat

structure SocketPair where
  succeeded : Bool
  outputPresent : Bool
  read : DescriptorId
  write : DescriptorId
  liveAfter : List DescriptorId
  deriving DecidableEq, Repr

def SocketPair.Valid (pair : SocketPair) : Prop :=
  (pair.succeeded = true ∧ pair.outputPresent = true ∧
    pair.read ≠ pair.write ∧ pair.liveAfter = [pair.read, pair.write]) ∨
  (pair.succeeded = false ∧ pair.outputPresent = false ∧
    pair.liveAfter = [])

instance (pair : SocketPair) : Decidable pair.Valid := by
  unfold SocketPair.Valid
  infer_instance

structure SocketPairCertificate where
  checked : SocketPair
  deriving DecidableEq, Repr

def checkSocketPair (pair : SocketPair) : Option SocketPairCertificate :=
  if pair.Valid then some { checked := pair } else none

private theorem checkSocketPair_valid
    {pair : SocketPair} {certificate : SocketPairCertificate}
    (result : checkSocketPair pair = some certificate) : pair.Valid := by
  unfold checkSocketPair at result
  split at result
  · assumption
  · contradiction

theorem accepted_successful_socketpair_creates_two_distinct_owned_descriptors
    {pair : SocketPair} {certificate : SocketPairCertificate}
    (result : checkSocketPair pair = some certificate)
    (succeeded : pair.succeeded = true) :
    pair.outputPresent = true ∧ pair.read ≠ pair.write ∧
      pair.liveAfter = [pair.read, pair.write] := by
  have valid := checkSocketPair_valid result
  rcases valid with valid | valid
  · exact valid.2
  · rw [succeeded] at valid
    cases valid.1

theorem accepted_failed_socketpair_creates_no_custody
    {pair : SocketPair} {certificate : SocketPairCertificate}
    (result : checkSocketPair pair = some certificate)
    (failed : pair.succeeded = false) :
    pair.outputPresent = false ∧ pair.liveAfter = [] := by
  have valid := checkSocketPair_valid result
  rcases valid with valid | valid
  · rw [failed] at valid
    cases valid.1
  · exact valid.2

structure Close where
  liveBefore : Bool
  succeeded : Bool
  liveAfter : Bool
  closeRecorded : Bool
  deriving DecidableEq, Repr

def Close.Valid (close : Close) : Prop :=
  (close.liveBefore = true ∧ close.succeeded = true ∧
    close.liveAfter = false ∧ close.closeRecorded = true) ∨
  (close.liveBefore = true ∧ close.succeeded = false ∧
    close.liveAfter = true ∧ close.closeRecorded = false)

instance (close : Close) : Decidable close.Valid := by
  unfold Close.Valid
  infer_instance

structure CloseCertificate where
  checked : Close
  deriving DecidableEq, Repr

def checkClose (close : Close) : Option CloseCertificate :=
  if close.Valid then some { checked := close } else none

private theorem checkClose_valid
    {close : Close} {certificate : CloseCertificate}
    (result : checkClose close = some certificate) : close.Valid := by
  unfold checkClose at result
  split at result
  · assumption
  · contradiction

theorem accepted_successful_close_discharges_once
    {close : Close} {certificate : CloseCertificate}
    (result : checkClose close = some certificate)
    (succeeded : close.succeeded = true) :
    close.liveBefore = true ∧ close.liveAfter = false ∧
      close.closeRecorded = true := by
  have valid := checkClose_valid result
  rcases valid with valid | valid
  · exact ⟨valid.1, valid.2.2⟩
  · rw [succeeded] at valid
    cases valid.2.1

theorem accepted_failed_close_retains_custody
    {close : Close} {certificate : CloseCertificate}
    (result : checkClose close = some certificate)
    (failed : close.succeeded = false) :
    close.liveBefore = true ∧ close.liveAfter = true ∧
      close.closeRecorded = false := by
  have valid := checkClose_valid result
  rcases valid with valid | valid
  · rw [failed] at valid
    cases valid.2.1
  · exact ⟨valid.1, valid.2.2⟩

structure ErrorQuery where
  knownClosed : Bool
  custodyPreserved : Bool
  deriving DecidableEq, Repr

def ErrorQuery.Valid (query : ErrorQuery) : Prop :=
  query.knownClosed = false ∧ query.custodyPreserved = true

instance (query : ErrorQuery) : Decidable query.Valid := by
  unfold ErrorQuery.Valid
  infer_instance

structure ErrorQueryCertificate where
  checked : ErrorQuery
  deriving DecidableEq, Repr

def checkErrorQuery (query : ErrorQuery) : Option ErrorQueryCertificate :=
  if query.Valid then some { checked := query } else none

private theorem checkErrorQuery_valid
    {query : ErrorQuery} {certificate : ErrorQueryCertificate}
    (result : checkErrorQuery query = some certificate) : query.Valid := by
  unfold checkErrorQuery at result
  split at result
  · assumption
  · contradiction

theorem accepted_error_query_is_borrowed
    {query : ErrorQuery} {certificate : ErrorQueryCertificate}
    (result : checkErrorQuery query = some certificate) :
    query.knownClosed = false ∧ query.custodyPreserved = true :=
  checkErrorQuery_valid result

structure Teardown where
  liveOwned : List DescriptorId
  closed : List DescriptorId
  deriving DecidableEq, Repr

def Teardown.Valid (teardown : Teardown) : Prop :=
  teardown.liveOwned = [] ∧ teardown.closed.Nodup

instance (teardown : Teardown) : Decidable teardown.Valid := by
  unfold Teardown.Valid
  infer_instance

structure TeardownCertificate where
  checked : Teardown
  deriving DecidableEq, Repr

def checkTeardown (teardown : Teardown) : Option TeardownCertificate :=
  if teardown.Valid then some { checked := teardown } else none

private theorem checkTeardown_valid
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) : teardown.Valid := by
  unfold checkTeardown at result
  split at result
  · assumption
  · contradiction

theorem accepted_descriptor_teardown_is_complete_and_unique
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) :
    teardown.liveOwned = [] ∧ teardown.closed.Nodup :=
  checkTeardown_valid result

/-- Historical theorem name retained for the Monkey adapter. -/
theorem accepted_monkey_descriptor_teardown_is_complete_and_unique
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) :
    teardown.liveOwned = [] ∧ teardown.closed.Nodup :=
  accepted_descriptor_teardown_is_complete_and_unique result

end QCore.DescriptorCustody
