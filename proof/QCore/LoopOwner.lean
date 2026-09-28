import QCore.EventDriven

namespace QCore.LoopOwner

/-! Card **W47-LOOP-OWNER-CONTRACTS** (gateway rule 10p, contract
specification §C04.9): the loop-owner obligation over derived thread identity.

Companion to `crates/qcore-core/src/loop_owner.rs` and
`crates/qcore-c-gateway/src/thread_entry_census.rs` with
`crates/quirin-source-analysis/src/loop_owner_census.rs`.

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

/-! ## Hop C: a coroutine runs on the thread that resumes it

Card **W47-LOOP-OWNER-COROUTINE-HOP**. The census derives a coroutine entry's
resumers from the source (a loop dispatcher that constructs the coroutine and
resumes the handle it returned); a handle it cannot tie to its constructor,
and a body's continuation after a yield, are undetermined resumers. The body's
contexts are exactly its derived resumers' (`a_coroutine_runs_on_its_resumers`),
so a coroutine resumed only on `owner L` runs on `owner L`
(`a_coroutine_resumed_only_on_the_owner_runs_on_the_owner`), one non-owner
resumer refuses (`a_non_owner_resumer_refuses`) and an undetermined resumer
never discharges (`an_undetermined_resumer_never_discharges`,
`a_body_that_may_yield_is_never_discharged`). -/

/-- A coroutine's resumer: a derived context, or one the census cannot derive. -/
inductive Resumer where
  | derived (context : Context)
  | undetermined
  deriving DecidableEq, Repr

/-- The contexts a coroutine body runs in: its derived resumers', thread kept. -/
def coroutineContexts : List Resumer → List Context
  | [] => []
  | .derived c :: rest => c :: coroutineContexts rest
  | .undetermined :: rest => coroutineContexts rest

def coroutineUnresolved (resumers : List Resumer) : Bool :=
  resumers.any (· == .undetermined)

/-- A body that may yield continues on a resumer that is not derived. -/
def resumersOf (initial : List Context) (mayYield : Bool) : List Resumer :=
  initial.map .derived ++ (if mayYield then [.undetermined] else [])

/-- The owner verdict of an operation inside a coroutine body. -/
def coroutineVerdict (owners : Owners) (resumers : List Resumer) : Verdict :=
  siteVerdict owners (coroutineContexts resumers) (coroutineUnresolved resumers)

theorem a_coroutine_runs_on_its_resumers (c : Context) (resumers : List Resumer) :
    c ∈ coroutineContexts resumers ↔ Resumer.derived c ∈ resumers := by
  induction resumers with
  | nil => simp [coroutineContexts]
  | cons r rest ih =>
    cases r with
    | derived d =>
      simp only [coroutineContexts, List.mem_cons, ih, Resumer.derived.injEq]
    | undetermined => simp [coroutineContexts, ih]

theorem an_undetermined_resumer_never_discharges {owners : Owners} {resumers : List Resumer}
    (h : Resumer.undetermined ∈ resumers) :
    coroutineVerdict owners resumers ≠ .discharged := by
  have hu : coroutineUnresolved resumers = true :=
    List.any_eq_true.mpr ⟨.undetermined, h, by simp⟩
  unfold coroutineVerdict
  rw [hu]
  exact unresolved_is_never_discharged owners _

theorem a_body_that_may_yield_is_never_discharged (owners : Owners) (initial : List Context) :
    coroutineVerdict owners (resumersOf initial true) ≠ .discharged := by
  apply an_undetermined_resumer_never_discharges
  simp [resumersOf]

theorem a_non_owner_resumer_refuses {owners : Owners} {resumers : List Resumer} {c : Context}
    {list : List Thread} (hc : Resumer.derived c ∈ resumers)
    (hl : candidates owners c.mode = some list) (hn : c.thread ∉ list) :
    coroutineVerdict owners resumers = .notDischarged :=
  a_non_owner_context_refuses ((a_coroutine_runs_on_its_resumers c resumers).mpr hc) hl hn

/-- A coroutine whose every resumer is derived on the engine loop's owner in
the non-threaded mode runs there: the operation is discharged. -/
theorem a_coroutine_resumed_only_on_the_owner_runs_on_the_owner
    {owners : Owners} {t : Thread} {resumers : List Resumer}
    (howner : owners.engine = some t)
    (hall : ∀ r ∈ resumers, r = .derived { thread := t, mode := .nonThreaded })
    (hne : resumers ≠ []) :
    coroutineVerdict owners resumers = .discharged := by
  have hctx : ∀ c ∈ coroutineContexts resumers, c = { thread := t, mode := .nonThreaded } := by
    intro c hc
    have := hall _ ((a_coroutine_runs_on_its_resumers c resumers).mp hc)
    exact (Resumer.derived.inj this)
  have hu : coroutineUnresolved resumers = false := by
    unfold coroutineUnresolved
    rw [Bool.eq_false_iff]
    intro h
    obtain ⟨r, hr, hq⟩ := List.any_eq_true.mp h
    rw [hall r hr] at hq
    simp at hq
  have hnonempty : coroutineContexts resumers ≠ [] := by
    obtain ⟨r, rest, rfl⟩ := List.exists_cons_of_ne_nil hne
    rw [hall r (List.mem_cons_self ..)]
    simp [coroutineContexts]
  have hv : ∀ c ∈ coroutineContexts resumers, contextVerdict owners c = .discharged := by
    intro c hc
    rw [hctx c hc]
    simp [contextVerdict, candidates, howner]
  unfold coroutineVerdict siteVerdict
  rw [hu]
  have h1 : (coroutineContexts resumers).any
      (fun c => contextVerdict owners c == .notDischarged) = false := by
    rw [Bool.eq_false_iff]
    intro h
    obtain ⟨c, hc, hq⟩ := List.any_eq_true.mp h
    rw [hv c hc] at hq
    simp at hq
  have h2 : (coroutineContexts resumers).any
      (fun c => contextVerdict owners c == .unresolved) = false := by
    rw [Bool.eq_false_iff]
    intro h
    obtain ⟨c, hc, hq⟩ := List.any_eq_true.mp h
    rw [hv c hc] at hq
    simp at hq
  have h3 : (coroutineContexts resumers).isEmpty = false := by
    cases hcs : coroutineContexts resumers with
    | nil => exact absurd hcs hnonempty
    | cons _ _ => rfl
  simp [h1, h2, h3]

/-- The loop dispatcher's resume keeps its thread and fixes the mode, like the
direct dispatch (hop L): a dispatcher on thread `t` resumes on `t`. -/
theorem the_dispatcher_resumes_on_its_own_thread
    {dispatcher : Context} {t : Thread} (hthread : dispatcher.thread = t) :
    (step (.dispatch .nonThreaded) dispatcher).map Resumer.derived =
      some (.derived { thread := t, mode := .nonThreaded }) := by
  simp [step, applyEffect, hthread]

/-! ### Which inputs the hop dispatches

An input whose descriptor flags are derived to run no coroutine is never
dispatched from the coroutine entry; an underived flag keeps the dispatch. -/

def coroutineDispatched {α : Type} (flag : α → Option Bool) (registrants : List α) : List α :=
  registrants.filter fun r => flag r != some false

theorem an_input_without_a_coroutine_is_not_dispatched_by_the_hop {α : Type}
    {flag : α → Option Bool} {registrants : List α} {r : α} (h : flag r = some false) :
    r ∉ coroutineDispatched flag registrants := by
  simp [coroutineDispatched, h]

theorem an_underived_flag_keeps_the_hop {α : Type}
    {flag : α → Option Bool} {registrants : List α} {r : α}
    (h : flag r = none) (hr : r ∈ registrants) :
    r ∈ coroutineDispatched flag registrants := by
  simp [coroutineDispatched, h, hr]

/-! ### The configured reading of a conditional group

Each arm's condition is decided true, decided false or undecided under the
build configuration. An arm is kept unless it is decided false or an earlier
arm is decided true. Whatever arm the preprocessor takes under a
configuration that agrees with every decided condition is kept
(`the_taken_arm_is_read`); a decided-false arm is dropped
(`a_decided_false_arm_is_not_read`); after a decided-true arm nothing is read
(`nothing_after_a_decided_arm_is_read`). -/

inductive Truth where
  | yes
  | no
  | unknown
  deriving DecidableEq, Repr

/-- The actual value of a condition agrees with what the configuration decided. -/
def agrees : Truth → Bool → Bool
  | .yes, b => b
  | .no, b => !b
  | .unknown, _ => true

def agreesAll : List Truth → List Bool → Bool
  | c :: cs, b :: bs => agrees c b && agreesAll cs bs
  | [], [] => true
  | _, _ => false

/-- The arms the configured reading keeps; `decided` means an earlier arm was
decided true. -/
def keptArms : Bool → List Truth → List Bool
  | _, [] => []
  | decided, c :: rest => (!decided && c != .no) :: keptArms (decided || c == .yes) rest

/-- The arm the preprocessor takes: the first whose condition holds. -/
def takenArms : Bool → List Bool → List Bool
  | _, [] => []
  | done, b :: rest => (!done && b) :: takenArms (done || b) rest

/-- Pointwise: every arm marked in the first list is marked in the second. -/
def within : List Bool → List Bool → Bool
  | a :: as, b :: bs => (!a || b) && within as bs
  | _, _ => true

theorem taken_within_kept (known : List Truth) :
    ∀ (actual : List Bool) (decided done : Bool),
      agreesAll known actual = true → (decided = true → done = true) →
      within (takenArms done actual) (keptArms decided known) = true := by
  induction known with
  | nil => intro actual decided done _ _; cases actual <;> simp [takenArms, keptArms, within]
  | cons c rest ih =>
    intro actual decided done hagree hdec
    cases actual with
    | nil => simp [agreesAll] at hagree
    | cons b bs =>
      simp only [agreesAll, Bool.and_eq_true] at hagree
      obtain ⟨hcb, hrest⟩ := hagree
      simp only [takenArms, keptArms, within, Bool.and_eq_true]
      refine ⟨?_, ih bs _ _ hrest ?_⟩
      · cases done <;> cases b <;> cases decided <;> cases c <;> simp_all [agrees]
      · intro h
        cases decided <;> cases c <;> cases done <;> cases b <;> simp_all [agrees]

theorem the_taken_arm_is_read (known : List Truth) (actual : List Bool)
    (hagree : agreesAll known actual = true) :
    within (takenArms false actual) (keptArms false known) = true :=
  taken_within_kept known actual false false hagree (fun h => h)

/-- Pointwise: no decided-false arm is kept. -/
def noDecidedFalseKept : List Truth → List Bool → Bool
  | c :: cs, k :: ks => (c != .no || !k) && noDecidedFalseKept cs ks
  | _, _ => true

theorem a_decided_false_arm_is_not_read (known : List Truth) :
    ∀ decided, noDecidedFalseKept known (keptArms decided known) = true := by
  induction known with
  | nil => intro decided; simp [noDecidedFalseKept]
  | cons c rest ih =>
    intro decided
    simp only [keptArms, noDecidedFalseKept, Bool.and_eq_true]
    exact ⟨by cases c <;> cases decided <;> simp, ih _⟩

theorem nothing_after_a_decided_arm_is_read (known : List Truth) :
    keptArms true known = known.map (fun _ => false) := by
  induction known with
  | nil => rfl
  | cons c rest ih => simp [keptArms, ih]

end QCore.LoopOwner
