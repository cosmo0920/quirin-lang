import QCore.EventObservation
import QCore.EventDriven
import QCore.MonkeyWaitProjection

namespace QCore.ObservedWaitComposition

inductive ObserverFlag where
  | functionSections | dataSections | addressUndefinedSanitizers
  deriving DecidableEq, Repr

def spellObserverFlag : ObserverFlag → String
  | .functionSections => "-ffunction-sections"
  | .dataSections => "-fdata-sections"
  | .addressUndefinedSanitizers => "-fsanitize=address,undefined"

def parseObserverFlag (value : String) : Option ObserverFlag :=
  if value == "-ffunction-sections" then some .functionSections
  else if value == "-fdata-sections" then some .dataSections
  else if value == "-fsanitize=address,undefined" then some .addressUndefinedSanitizers
  else none

theorem observer_flags_round_trip (flag : ObserverFlag) :
    parseObserverFlag (spellObserverFlag flag) = some flag := by cases flag <;> decide

theorem unsupported_thread_sanitizer_refuses :
    parseObserverFlag "-fsanitize=thread" = none := by decide

def dependencyOutputsAdmitted (modes outputs : Nat) : Bool :=
  (modes == 0 && outputs == 0) || (modes == 1 && outputs == 1)

theorem unpaired_dependency_output_refuses :
    dependencyOutputsAdmitted 0 1 = false ∧
      dependencyOutputsAdmitted 1 0 = false := by decide

/-! Semantic model of the bounded Rust join. Native identities are retained
before fresh model names are assigned. Hash computation, C import, contract
parsing and native observation truth remain external boundaries; this module
does not prove a refinement from Rust or from C. -/

structure ModelEvent where
  id : Nat
  generation : Nat
  deriving DecidableEq, Repr

def renameEvent (native : EventObservation.Identity) : ModelEvent :=
  ⟨1, native.generation⟩

def lifecycle : List (EventDriven.Lifecycle × EventDriven.TransitionKind ×
    EventDriven.Lifecycle) := [
  (.idle, .assign, .idle),
  (.idle, .add, .pending),
  (.pending, .activate, .active),
  (.active, .callbackEnter, .running),
  (.running, .callbackExitOneShot, .idle),
  (.idle, .free, .freed)]

structure JoinAssumptions where
  threads : String
  observationTrust : String
  linkageTrust : String
  nativeRunManifestSha256Valid : Bool
  deriving DecidableEq, Repr

def requiredJoinAssumptions : JoinAssumptions := {
  threads := "pthreads"
  observationTrust := "producer_supplied_runtime_evidence"
  linkageTrust := "producer_supplied_selected_artifact_linkage"
  nativeRunManifestSha256Valid := true
}

def requiredKind : MonkeyWaitProjection.RequiredObservation → EventObservation.Kind
  | .baseCreateSuccess => .baseCreate
  | .eventCreateTimeoutNoDescriptorWithPriority => .eventNew
  | .eventAddSuccess => .eventAdd
  | .dispatchBegin => .dispatchBegin
  | .timeoutActivation => .activateTimeout
  | .callbackEnter => .callbackEnterTimeout
  | .callbackExit => .callbackExit
  | .dispatchEnd => .dispatchEnd
  | .eventFree => .eventFree
  | .argumentScopeEnd => .argumentEnd
  | .baseFree => .baseFree

theorem source_obligations_match_observation_sequence (context : EventObservation.Context) :
    MonkeyWaitProjection.requiredObservations.map requiredKind =
      (EventObservation.expected context).map EventObservation.Record.kind := rfl

theorem projected_lifecycle_is_legal :
    lifecycle.all (fun (before, kind, after) =>
      EventDriven.legalLifecycle before kind after) = true := by decide

theorem model_generation_preserves_native_generation
    (native : EventObservation.Identity) :
    (renameEvent native).generation = native.generation := rfl

-- Mapping a singleton identity domain to the fresh name is injective on that
-- domain. This does not authorize identifying two native pointers.
theorem fresh_name_is_injective_on_selected_domain
    (selected left right : EventObservation.Identity)
    (leftSelected : left ∈ [selected]) (rightSelected : right ∈ [selected]) :
    left = right := by
  simp only [List.mem_singleton] at leftSelected rightSelected
  exact leftSelected.trans rightSelected.symm

def project (trace : EventObservation.Trace) (context : EventObservation.Context)
    (assumptions : JoinAssumptions) :
    Option (ModelEvent × List (EventDriven.Lifecycle × EventDriven.TransitionKind ×
      EventDriven.Lifecycle)) :=
  if assumptions = requiredJoinAssumptions then
    match EventObservation.check trace context with
    | none => none
    | some _ => some (renameEvent context.event, lifecycle)
  else none

theorem accepted_replay_requires_exact_join_assumptions
    {trace : EventObservation.Trace} {context : EventObservation.Context}
    {assumptions : JoinAssumptions}
    {result : ModelEvent × List (EventDriven.Lifecycle × EventDriven.TransitionKind ×
      EventDriven.Lifecycle)}
    (accepted : project trace context assumptions = some result) :
    assumptions = requiredJoinAssumptions := by
  unfold project at accepted
  split at accepted
  · assumption
  · contradiction

theorem accepted_projection_retains_callback_identity
    {trace : EventObservation.Trace} {context : EventObservation.Context}
    {result : ModelEvent × List (EventDriven.Lifecycle × EventDriven.TransitionKind ×
      EventDriven.Lifecycle)}
    (accepted : project trace context requiredJoinAssumptions = some result) :
    ((trace.records.drop 5).head?).bind EventObservation.Record.event =
      some context.event := by
  unfold project at accepted
  cases checked : EventObservation.check trace context with
  | none => simp [checked] at accepted
  | some certificate =>
    exact EventObservation.accepted_callback_binds_expected_event_generation checked

def wrongThreads : JoinAssumptions :=
  { requiredJoinAssumptions with threads := "single" }

def invalidManifest : JoinAssumptions :=
  { requiredJoinAssumptions with nativeRunManifestSha256Valid := false }

theorem substituted_thread_assumption_refuses
    (trace : EventObservation.Trace) (context : EventObservation.Context) :
    project trace context wrongThreads = none := by
  simp [project, wrongThreads, requiredJoinAssumptions]

theorem invalid_manifest_identity_refuses
    (trace : EventObservation.Trace) (context : EventObservation.Context) :
    project trace context invalidManifest = none := by
  simp [project, invalidManifest, requiredJoinAssumptions]

end QCore.ObservedWaitComposition
