import QCore.EventDriven

namespace QCore.LoopOwner

/-! Card **W47-LOOP-OWNER-CONTRACTS** (gateway rule 10p, contract
specification §C04.9): the loop-owner obligation over derived thread identity.

Companion to `crates/qcore-core/src/loop_owner.rs` and
`crates/qcore-c-gateway/src/thread_entry_census.rs`.

* A context is a derived thread and the mode (the threaded flag of the
  instance the path is about). An edge keeps the thread; a loop dispatch
  fixes the mode (hop L); a threaded-flag branch requires one (hop F).
* The owner of a loop is the single derived thread of its dispatcher. A
  callback dispatched by loop `L` runs on `owner L`
  (`a_dispatched_callback_runs_on_the_owner`).
* An operation is discharged only when every derived context runs on the
  owner of its mode's loop and no path is unresolved
  (`discharged_means_every_context_runs_on_its_owner`). A discharge then
  derives the reactor's single-dispatcher premise instead of assuming it
  (`the_owner_route_derives_the_dispatcher_premise`), and no two accesses of a
  loop's links come from different threads
  (`a_discharged_loop_has_no_concurrent_mutator`); a foreign access conflicts
  with the dispatcher's own walk, which is the reactor's L05
  (`a_foreign_access_conflicts_with_the_dispatcher`).
* Unresolved means refused, never discharged (`unresolved_is_never_discharged`,
  `no_derived_caller_is_never_discharged`,
  `the_per_unit_route_refuses_every_obligated_call`).
* C2: an enqueue never discharges "applied" (`an_enqueue_never_applies`).
* C3: the paired collectors' state is a boundary claim only when both calls
  succeeded on the owner, and a resume's running state only under active
  ingestion (`a_boundary_claim_needs_both_results_and_the_owner`). -/

abbrev Thread := Nat

inductive Mode where
  | nonThreaded
  | threaded
  | either
  deriving DecidableEq, Repr

structure Context where
  thread : Thread
  mode : Mode
  deriving DecidableEq, Repr

/-- An edge's effect on the mode (hops F and L). -/
inductive Effect where
  | keep
  | require (mode : Mode)
  | dispatch (mode : Mode)
  deriving DecidableEq, Repr

def applyEffect : Effect → Mode → Option Mode
  | .keep, mode => some mode
  | .dispatch fixed, _ => some fixed
  | .require required, .either => some required
  | .require required, mode => if mode = required then some mode else none

/-- One propagation step along a call or dispatch edge: the thread is kept. -/
def step (effect : Effect) (context : Context) : Option Context :=
  (applyEffect effect context.mode).map fun mode => { thread := context.thread, mode := mode }

theorem a_step_keeps_the_thread {effect : Effect} {context next : Context}
    (h : step effect context = some next) : next.thread = context.thread := by
  unfold step at h
  cases hm : applyEffect effect context.mode with
  | none => simp [hm] at h
  | some mode =>
    simp [hm] at h
    rw [← h]

/-- The derived owner of each loop kind. -/
structure Owners where
  engine : Option Thread
  worker : Option Thread
  deriving DecidableEq, Repr

/-- The owner of a loop is the single thread of its dispatcher, and only when
no path to the dispatcher is unresolved. -/
def ownerOfDispatcher (threads : List Thread) (unresolved : Bool) : Option Thread :=
  if unresolved then none
  else match threads.eraseDups with
    | [t] => some t
    | _ => none

def candidates (owners : Owners) : Mode → Option (List Thread)
  | .nonThreaded => owners.engine.map fun t => [t]
  | .threaded => owners.worker.map fun t => [t]
  | .either =>
    match owners.engine, owners.worker with
    | some e, some w => some [e, w]
    | _, _ => none

inductive Verdict where
  | discharged
  | notDischarged
  | unresolved
  deriving DecidableEq, Repr

def contextVerdict (owners : Owners) (context : Context) : Verdict :=
  match candidates owners context.mode with
  | none => .unresolved
  | some owners =>
    if context.thread ∈ owners then
      (if owners.length = 1 then .discharged else .unresolved)
    else .notDischarged

/-- The site verdict: one resolved non-owner context refuses; otherwise any
unresolved path, unresolved context or absent caller leaves it unresolved. -/
def siteVerdict (owners : Owners) (contexts : List Context) (unresolved : Bool) : Verdict :=
  if contexts.any (fun c => contextVerdict owners c == .notDischarged) then .notDischarged
  else if unresolved || contexts.isEmpty ||
      contexts.any (fun c => contextVerdict owners c == .unresolved) then .unresolved
  else .discharged

theorem unresolved_is_never_discharged (owners : Owners) (contexts : List Context) :
    siteVerdict owners contexts true ≠ .discharged := by
  unfold siteVerdict
  split <;> simp

theorem no_derived_caller_is_never_discharged (owners : Owners) (unresolved : Bool) :
    siteVerdict owners [] unresolved ≠ .discharged := by
  unfold siteVerdict
  simp

theorem context_discharged_on_owner {owners : Owners} {context : Context}
    (h : contextVerdict owners context = .discharged) :
    ∃ t, candidates owners context.mode = some [t] ∧ context.thread = t := by
  unfold contextVerdict at h
  cases hc : candidates owners context.mode with
  | none => simp [hc] at h
  | some list =>
    simp only [hc] at h
    by_cases hm : context.thread ∈ list
    · by_cases hl : list.length = 1
      · match list, hl with
        | [t], _ => exact ⟨t, rfl, by simpa using hm⟩
      · simp [hm, hl] at h
    · simp [hm] at h

/-- A discharge means every derived context runs on the owner of its mode's
loop: the operation's derived thread set is exactly `{owner L}` per mode. -/
theorem discharged_means_every_context_runs_on_its_owner
    {owners : Owners} {contexts : List Context} {unresolved : Bool}
    (h : siteVerdict owners contexts unresolved = .discharged) :
    unresolved = false ∧ contexts ≠ [] ∧
      ∀ c ∈ contexts, ∃ t, candidates owners c.mode = some [t] ∧ c.thread = t := by
  unfold siteVerdict at h
  by_cases h1 : contexts.any (fun c => contextVerdict owners c == .notDischarged) = true
  · simp [h1] at h
  · by_cases h2 : (unresolved || contexts.isEmpty ||
        contexts.any (fun c => contextVerdict owners c == .unresolved)) = true
    · simp [h1, h2] at h
    · simp only [Bool.or_eq_true, not_or] at h2
      obtain ⟨⟨hu, he⟩, hr⟩ := h2
      refine ⟨by simpa using hu, by simpa using he, ?_⟩
      intro c hc
      apply context_discharged_on_owner
      have hn : ¬ (contextVerdict owners c == .notDischarged) = true := by
        intro hx; exact h1 (List.any_eq_true.mpr ⟨c, hc, hx⟩)
      have hr' : ¬ (contextVerdict owners c == .unresolved) = true := by
        intro hx; exact hr (List.any_eq_true.mpr ⟨c, hc, hx⟩)
      cases hv : contextVerdict owners c <;> simp_all

/-- One resolved context off its loop's owner refuses the site, whatever else
is unresolved. -/
theorem a_non_owner_context_refuses
    {owners : Owners} {contexts : List Context} {unresolved : Bool} {c : Context}
    {list : List Thread} (hc : c ∈ contexts) (hl : candidates owners c.mode = some list)
    (hn : c.thread ∉ list) :
    siteVerdict owners contexts unresolved = .notDischarged := by
  have hv : contextVerdict owners c = .notDischarged := by
    unfold contextVerdict; simp [hl, hn]
  unfold siteVerdict
  have : contexts.any (fun c => contextVerdict owners c == .notDischarged) = true :=
    List.any_eq_true.mpr ⟨c, hc, by simp [hv]⟩
  simp [this]

/-- A callback dispatched by the engine loop runs on the loop's owner, in the
non-threaded mode: the dispatch fixes the mode and keeps the dispatcher's
thread, and the owner is that thread. -/
theorem a_dispatched_callback_runs_on_the_owner
    {owners : Owners} {dispatcher : Context} {t : Thread}
    (howner : owners.engine = some t) (hthread : dispatcher.thread = t) :
    step (.dispatch .nonThreaded) dispatcher = some { thread := t, mode := .nonThreaded } ∧
      contextVerdict owners { thread := t, mode := .nonThreaded } = .discharged := by
  constructor
  · simp [step, applyEffect, hthread]
  · simp [contextVerdict, candidates, howner]

/-- The worker loop likewise. -/
theorem a_worker_callback_runs_on_the_owner
    {owners : Owners} {dispatcher : Context} {t : Thread}
    (howner : owners.worker = some t) (hthread : dispatcher.thread = t) :
    step (.dispatch .threaded) dispatcher = some { thread := t, mode := .threaded } ∧
      contextVerdict owners { thread := t, mode := .threaded } = .discharged := by
  constructor
  · simp [step, applyEffect, hthread]
  · simp [contextVerdict, candidates, howner]

/-- A dispatcher with an unresolved path has no owner. -/
theorem an_unresolved_dispatcher_has_no_owner (threads : List Thread) :
    ownerOfDispatcher threads true = none := by
  simp [ownerOfDispatcher]

/-- A threaded-flag branch the context's mode excludes is not taken. -/
theorem an_infeasible_branch_is_pruned (context : Context)
    (h : context.mode = .nonThreaded) :
    step (.require .threaded) context = none := by
  simp [step, applyEffect, h]

/-! ## Ownership excludes concurrent mutation of the loop's links -/

/-- One access to a loop's links. -/
structure Access where
  context : Context
  mutates : Bool
  deriving DecidableEq, Repr

/-- The reactor's L05 shape: two accesses from different threads, one of them
a mutation, with no ordering edge. -/
def conflicting (a b : Access) : Bool :=
  a.context.thread != b.context.thread && (a.mutates || b.mutates)

/-- The single-dispatcher premise as a derived fact: every access runs on
the owner. -/
def singleDispatcher (owner : Thread) (accesses : List Access) : Bool :=
  accesses.all fun a => a.context.thread == owner

/-- Every access of one engine loop discharged means every access runs on the
engine owner. -/
theorem discharged_accesses_run_on_the_owner
    {owners : Owners} {t : Thread} {accesses : List Access}
    (howner : owners.engine = some t)
    (hmode : ∀ a ∈ accesses, a.context.mode = .nonThreaded)
    (h : siteVerdict owners (accesses.map (·.context)) false = .discharged) :
    singleDispatcher t accesses = true := by
  obtain ⟨_, _, hall⟩ := discharged_means_every_context_runs_on_its_owner h
  unfold singleDispatcher
  rw [List.all_eq_true]
  intro a ha
  obtain ⟨u, hu, hth⟩ := hall a.context (List.mem_map.mpr ⟨a, ha, rfl⟩)
  rw [hmode a ha] at hu
  simp [candidates, howner] at hu
  simp [hth, hu]

/-- Replaces the Cycle 18 route's `sameBaseSingleDispatcher : Bool` premise for
this route: an execution whose flag is the derived single-dispatcher fact has
it `true` because every loop access was discharged on the owner, not because
it was supplied. -/
theorem the_owner_route_derives_the_dispatcher_premise
    {owners : Owners} {t : Thread} {accesses : List Access}
    (execution : QCore.EventDriven.Execution)
    (hflag : execution.sameBaseSingleDispatcher = singleDispatcher t accesses)
    (howner : owners.engine = some t)
    (hmode : ∀ a ∈ accesses, a.context.mode = .nonThreaded)
    (h : siteVerdict owners (accesses.map (·.context)) false = .discharged) :
    execution.sameBaseSingleDispatcher = true := by
  rw [hflag]
  exact discharged_accesses_run_on_the_owner howner hmode h

/-- A loop whose accesses all run on the owner has no conflicting pair: no
other thread mutates its links concurrently. -/
theorem a_discharged_loop_has_no_concurrent_mutator
    {t : Thread} {accesses : List Access}
    (h : singleDispatcher t accesses = true) :
    ∀ a ∈ accesses, ∀ b ∈ accesses, conflicting a b = false := by
  intro a ha b hb
  unfold singleDispatcher at h
  rw [List.all_eq_true] at h
  have hta := h a ha
  have htb := h b hb
  simp at hta htb
  simp [conflicting, hta, htb]

/-- A foreign access conflicts with the dispatcher's own walk of the links (a
mutation on the owner): the reactor's L05. -/
theorem a_foreign_access_conflicts_with_the_dispatcher
    {walk foreign : Access} (hwalk : walk.mutates = true)
    (hthread : foreign.context.thread ≠ walk.context.thread) :
    conflicting walk foreign = true := by
  simp [conflicting, hwalk]
  exact fun h => hthread h.symm

/-! ## Refusal on the per-unit route -/

inductive Admission where
  | admitted
  | refused
  deriving DecidableEq, Repr

/-- A declared obligation admits its contract at a call only when discharged. -/
def admission : Verdict → Admission
  | .discharged => .admitted
  | _ => .refused

/-- A per-unit resource route derives no thread: its verdict is unresolved. -/
def perUnitRoute : Verdict := .unresolved

theorem the_per_unit_route_refuses_every_obligated_call :
    admission perUnitRoute = .refused := rfl

theorem only_a_discharge_admits (v : Verdict) :
    admission v = .admitted ↔ v = .discharged := by
  cases v <;> simp [admission]

/-! ## C2: two completion meanings -/

inductive Completion where
  | callbackFinished
  | requestEnqueued
  deriving DecidableEq, Repr

inductive Claim where
  | requestAvailable
  | callbackFinished
  | applied
  deriving DecidableEq, Repr

def completion : Mode → Option Completion
  | .nonThreaded => some .callbackFinished
  | .threaded => some .requestEnqueued
  | .either => none

def claimDischarged : Claim → Option Completion → Bool
  | .requestAvailable, some .requestEnqueued => true
  | .callbackFinished, some .callbackFinished => true
  | _, _ => false

theorem an_enqueue_never_applies (c : Option Completion) :
    claimDischarged .applied c = false := by
  cases c with
  | none => rfl
  | some c => cases c <;> rfl

theorem a_threaded_control_only_enqueues :
    completion .threaded = some .requestEnqueued ∧
      claimDischarged .callbackFinished (completion .threaded) = false ∧
      claimDischarged .requestAvailable (completion .threaded) = true := by
  decide

theorem an_unfixed_mode_claims_nothing (claim : Claim) :
    claimDischarged claim (completion .either) = false := by
  cases claim <;> rfl

/-! ## C3: the paired backend collectors, at the boundary only -/

inductive Result where
  | succeeded
  | failed
  | ignored
  deriving DecidableEq, Repr

inductive Direction where
  | pause
  | resume
  deriving DecidableEq, Repr

inductive Ingestion where
  | active
  | inactive
  | unknown
  deriving DecidableEq, Repr

inductive Pairing where
  | bothAtBoundary
  | noConclusion
  | refused
  deriving DecidableEq, Repr

def pairing (direction : Direction) (first second : Result) (ingestion : Ingestion)
    (owner : Verdict) : Pairing :=
  match owner with
  | .notDischarged => .refused
  | .unresolved => .noConclusion
  | .discharged =>
    if direction = .resume ∧ ingestion = .inactive then .refused
    else if direction = .resume ∧ ingestion = .unknown then .noConclusion
    else if first = .succeeded ∧ second = .succeeded then .bothAtBoundary
    else .noConclusion

theorem a_boundary_claim_needs_both_results_and_the_owner
    {direction : Direction} {first second : Result} {ingestion : Ingestion} {owner : Verdict}
    (h : pairing direction first second ingestion owner = .bothAtBoundary) :
    owner = .discharged ∧ first = .succeeded ∧ second = .succeeded ∧
      (direction = .resume → ingestion = .active) := by
  unfold pairing at h
  cases owner <;> simp at h
  refine ⟨rfl, ?_⟩
  cases direction <;> cases ingestion <;> cases first <;> cases second <;> simp_all

theorem a_discarded_result_gives_no_conclusion (direction : Direction) (second : Result)
    (ingestion : Ingestion) (owner : Verdict) :
    pairing direction .ignored second ingestion owner ≠ .bothAtBoundary := by
  intro h
  have := (a_boundary_claim_needs_both_results_and_the_owner h).2.1
  cases this

theorem a_resume_without_ingestion_is_refused (first second : Result) :
    pairing .resume first second .inactive .discharged = .refused := by
  simp [pairing]

end QCore.LoopOwner
