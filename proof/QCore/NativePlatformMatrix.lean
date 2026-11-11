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

namespace QCore.NativePlatformMatrix

inductive Status where
  | observed
  | unknown
  deriving DecidableEq, Repr

inductive Profile where
  | linuxEpollAsan
  | linuxPollAsan
  | linuxSelectAsan
  | macosKqueueAsan
  | windowsWin32Asan
  | windowsIocpAsan
  | linuxEpollTsan
  deriving DecidableEq, Repr

structure Dimensions where
  host : String
  backend : String
  sanitizers : String
  threadAdapter : String
  observer : String
  deriving DecidableEq, Repr

def expectedDimensions : Profile → Dimensions
  | .linuxEpollAsan => ⟨"linux", "epoll", "address,undefined,leak",
      "libevent_pthreads_initialized", "bounded_generic_libevent_probe"⟩
  | .linuxPollAsan => ⟨"linux", "poll", "address,undefined,leak",
      "libevent_pthreads_initialized", "bounded_generic_libevent_probe"⟩
  | .linuxSelectAsan => ⟨"linux", "select", "address,undefined,leak",
      "libevent_pthreads_initialized", "bounded_generic_libevent_probe"⟩
  | .macosKqueueAsan => ⟨"macos", "kqueue", "address,undefined",
      "libevent_pthreads_initialized", "bounded_generic_libevent_probe"⟩
  | .windowsWin32Asan => ⟨"windows", "win32", "address",
      "libevent_windows_threads_initialized", "bounded_generic_libevent_probe"⟩
  | .windowsIocpAsan => ⟨"windows", "iocp_profile", "address",
      "libevent_windows_threads_initialized", "bounded_generic_libevent_probe"⟩
  | .linuxEpollTsan => ⟨"linux", "epoll", "thread",
      "libevent_pthreads_initialized", "bounded_generic_libevent_probe"⟩

def expectedUnknownReason : Profile → String
  | .linuxEpollAsan => "missing_current_epoll_sanitizer_observer_artifact_join"
  | .linuxPollAsan => "missing_current_poll_sanitizer_observer_artifact_join"
  | .linuxSelectAsan => "missing_current_select_sanitizer_observer_artifact_join"
  | .macosKqueueAsan => "missing_current_kqueue_sanitizer_observer_artifact_join"
  | .windowsWin32Asan => "missing_current_win32_sanitizer_observer_artifact_join"
  | .windowsIocpAsan => "missing_current_iocp_sanitizer_observer_artifact_join"
  | .linuxEpollTsan => "thread_sanitizer_not_executed_for_this_probe"

def expectsHistory : Profile → Bool
  | .linuxEpollTsan => false
  | _ => true

structure ObservedFacts where
  artifactActual : String
  artifactExpected : String
  sourceRevisionActual : String
  sourceRevisionExpected : String
  harnessActual : String
  harnessSelected : String
  compilerPathClaimed : String
  compilerPathSelected : String
  compilerVersionClaimed : String
  compilerVersionSelected : String
  commandClaimed : String
  commandSelected : String
  coreArchiveClaimed : String
  coreArchiveSelected : String
  pthreadArchiveClaimed : String
  pthreadArchiveSelected : String
  executableClaimed : String
  executableSelected : String
  runtimeHostActual : String
  runtimeBackendActual : String
  sanitizersActual : String
  threadAdapterActual : String
  observerActual : String
  processExit : Int
  activationCount : Nat
  callbackCount : Nat
  callbackFlags : Nat
  dispatchThread : Nat
  activatingThread : Nat
  callbackThread : Nat
  property : String
  observerScope : String
  observerTransparency : String
  deriving DecidableEq, Repr

def observedAdmissible (dimensions : Dimensions) (facts : ObservedFacts) : Bool :=
  facts.artifactActual == facts.artifactExpected &&
  facts.artifactExpected == "libevent/generic-cross-thread-activation" &&
  facts.sourceRevisionActual == facts.sourceRevisionExpected &&
  facts.harnessActual == facts.harnessSelected &&
  facts.compilerPathClaimed == facts.compilerPathSelected &&
  facts.compilerVersionClaimed == facts.compilerVersionSelected &&
  facts.commandClaimed == facts.commandSelected &&
  facts.coreArchiveClaimed == facts.coreArchiveSelected &&
  facts.pthreadArchiveClaimed == facts.pthreadArchiveSelected &&
  facts.executableClaimed == facts.executableSelected &&
  facts.runtimeHostActual == dimensions.host &&
  facts.runtimeBackendActual == dimensions.backend &&
  facts.sanitizersActual == dimensions.sanitizers &&
  facts.threadAdapterActual == dimensions.threadAdapter &&
  facts.observerActual == dimensions.observer &&
  facts.processExit == 0 && facts.activationCount == 1 && facts.callbackCount == 1 &&
  facts.callbackFlags == 2 && facts.dispatchThread != facts.activatingThread &&
  facts.callbackThread == facts.dispatchThread &&
  facts.property == "generic_libevent_cross_thread_activation" &&
  facts.observerScope == "generic_libevent_only" &&
  facts.observerTransparency == "not_claimed"

structure Row where
  profile : Profile
  dimensions : Dimensions
  status : Status
  currentEvidence : Option ObservedFacts
  unknownReason : Option String
  carriedHistorical : Bool
  deriving DecidableEq, Repr

def admissible (row : Row) : Bool :=
  row.dimensions == expectedDimensions row.profile &&
  row.carriedHistorical == expectsHistory row.profile &&
  match row.status with
  | .observed => match row.currentEvidence with
      | some facts => row.unknownReason.isNone && observedAdmissible row.dimensions facts
      | none => false
  | .unknown => row.currentEvidence.isNone &&
      row.unknownReason == some (expectedUnknownReason row.profile)

def goodEvidence (profile : Profile) : ObservedFacts :=
  let dimensions := expectedDimensions profile
  {
  artifactActual := "libevent/generic-cross-thread-activation"
  artifactExpected := "libevent/generic-cross-thread-activation"
  sourceRevisionActual := "pinned-revision"
  sourceRevisionExpected := "pinned-revision"
  harnessActual := "harness-digest"
  harnessSelected := "harness-digest"
  compilerPathClaimed := "/usr/bin/clang"
  compilerPathSelected := "/usr/bin/clang"
  compilerVersionClaimed := "full clang version"
  compilerVersionSelected := "full clang version"
  commandClaimed := "command-digest"
  commandSelected := "command-digest"
  coreArchiveClaimed := "core-digest"
  coreArchiveSelected := "core-digest"
  pthreadArchiveClaimed := "pthread-digest"
  pthreadArchiveSelected := "pthread-digest"
  executableClaimed := "executable-digest"
  executableSelected := "executable-digest"
  runtimeHostActual := dimensions.host
  runtimeBackendActual := dimensions.backend
  sanitizersActual := dimensions.sanitizers
  threadAdapterActual := dimensions.threadAdapter
  observerActual := dimensions.observer
  processExit := 0
  activationCount := 1
  callbackCount := 1
  callbackFlags := 2
  dispatchThread := 10
  activatingThread := 11
  callbackThread := 10
  property := "generic_libevent_cross_thread_activation"
  observerScope := "generic_libevent_only"
  observerTransparency := "not_claimed"
  }

def observedRow (profile : Profile) (facts : ObservedFacts) : Row := {
  profile := profile
  dimensions := expectedDimensions profile
  status := .observed
  currentEvidence := some facts
  unknownReason := none
  carriedHistorical := expectsHistory profile
}

def unknownRow (profile : Profile) : Row := {
  profile := profile
  dimensions := expectedDimensions profile
  status := .unknown
  currentEvidence := none
  unknownReason := some (expectedUnknownReason profile)
  carriedHistorical := expectsHistory profile
}

def requiredProfiles : List Profile := [.linuxEpollAsan, .linuxPollAsan,
  .linuxSelectAsan, .macosKqueueAsan, .windowsWin32Asan, .windowsIocpAsan,
  .linuxEpollTsan]

def checkMatrix (rows : List Row) : Bool :=
  rows.map (·.profile) == requiredProfiles && rows.all admissible

def ValidMatrix (rows : List Row) : Bool := checkMatrix rows

def selectedMatrix : List Row := [
  observedRow .linuxEpollAsan (goodEvidence .linuxEpollAsan),
  observedRow .linuxPollAsan (goodEvidence .linuxPollAsan),
  observedRow .linuxSelectAsan (goodEvidence .linuxSelectAsan),
  unknownRow .macosKqueueAsan,
  unknownRow .windowsWin32Asan,
  unknownRow .windowsIocpAsan,
  observedRow .linuxEpollTsan (goodEvidence .linuxEpollTsan)]

def allUnknownMatrix : List Row := requiredProfiles.map unknownRow

theorem required_profiles_are_seven_and_unique :
    requiredProfiles.length = 7 ∧ requiredProfiles.Nodup := by decide

theorem valid_matrix_has_exact_profiles {rows : List Row} (h : ValidMatrix rows = true) :
    rows.map (·.profile) = requiredProfiles := by
  simp [ValidMatrix, checkMatrix] at h
  exact h.1

theorem selected_matrix_is_valid : ValidMatrix selectedMatrix = true := by decide

theorem all_unknown_matrix_is_valid : ValidMatrix allUnknownMatrix = true := by decide

theorem profile_relabel_refuses :
    admissible (observedRow .linuxPollAsan (goodEvidence .linuxEpollAsan)) = false := by decide

theorem same_thread_refuses :
    observedAdmissible (expectedDimensions .linuxEpollTsan)
      { goodEvidence .linuxEpollTsan with activatingThread := 10 } = false := by decide

theorem compiler_substitution_refuses :
    observedAdmissible (expectedDimensions .linuxEpollTsan)
      { goodEvidence .linuxEpollTsan with compilerPathClaimed := "other" } = false := by decide

theorem artifact_substitution_refuses :
    observedAdmissible (expectedDimensions .linuxEpollTsan)
      { goodEvidence .linuxEpollTsan with artifactActual := "monkey/callback" } = false := by decide

theorem transparency_claim_refuses :
    observedAdmissible (expectedDimensions .linuxEpollTsan)
      { goodEvidence .linuxEpollTsan with observerTransparency := "transparent" } = false := by decide

theorem carried_history_does_not_promote :
    (unknownRow .macosKqueueAsan).status = .unknown := rfl

end QCore.NativePlatformMatrix
