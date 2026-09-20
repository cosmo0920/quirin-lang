namespace QCore.NativePlatformMatrix

inductive Status where
  | observed
  | unknown
  deriving DecidableEq, Repr

inductive EvidenceClass where
  | currentJoined
  | carriedHistorical
  | absent
  deriving DecidableEq, Repr

structure Row where
  host : String
  backend : String
  sanitizers : String
  threadAdapter : String
  observer : String
  status : Status
  currentEvidence : EvidenceClass
  transparencyClaimed : Bool
  deriving DecidableEq, Repr

def linuxEpoll : Row := {
  host := "linux"
  backend := "epoll"
  sanitizers := "address,undefined,leak"
  threadAdapter := "libevent_pthreads_initialized"
  observer := "bounded_generic_libevent_probe"
  status := .observed
  currentEvidence := .currentJoined
  transparencyClaimed := false
}

def linuxTsan : Row := {
  host := "linux"
  backend := "epoll"
  sanitizers := "thread"
  threadAdapter := "libevent_pthreads_initialized"
  observer := "bounded_generic_libevent_probe"
  status := .observed
  currentEvidence := .currentJoined
  transparencyClaimed := false
}

def unknownRow (host backend sanitizers threadAdapter : String)
    (history : EvidenceClass) : Row := {
  host := host
  backend := backend
  sanitizers := sanitizers
  threadAdapter := threadAdapter
  observer := "bounded_generic_libevent_probe"
  status := .unknown
  currentEvidence := history
  transparencyClaimed := false
}

def selectedMatrix : List Row := [
  linuxEpoll,
  unknownRow "linux" "poll" "address,undefined,leak"
    "libevent_pthreads_initialized" .carriedHistorical,
  unknownRow "linux" "select" "address,undefined,leak"
    "libevent_pthreads_initialized" .carriedHistorical,
  unknownRow "macos" "kqueue" "address,undefined"
    "libevent_pthreads_initialized" .carriedHistorical,
  unknownRow "windows" "win32" "address"
    "libevent_windows_threads_initialized" .carriedHistorical,
  unknownRow "windows" "iocp_profile" "address"
    "libevent_windows_threads_initialized" .carriedHistorical,
  linuxTsan]

def admissible (row : Row) : Bool :=
  !row.transparencyClaimed &&
  match row.status with
  | .observed => row.currentEvidence == .currentJoined
  | .unknown => row.currentEvidence != .currentJoined

theorem selected_matrix_is_total_and_fail_closed :
    selectedMatrix.length = 7 ∧ selectedMatrix.all admissible = true := by decide

theorem carried_history_does_not_promote
    (host backend sanitizers threads : String) :
    (unknownRow host backend sanitizers threads .carriedHistorical).status = .unknown := rfl

theorem current_tsan_row_requires_join :
    linuxTsan.status = .observed ∧ linuxTsan.currentEvidence = .currentJoined := by decide

theorem transparency_claim_refuses (row : Row) :
    admissible { row with transparencyClaimed := true } = false := by
  simp [admissible]

end QCore.NativePlatformMatrix
