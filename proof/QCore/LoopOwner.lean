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

/-! ## Typed field dispatch (card W47-LOOP-OWNER-TYPED-DISPATCH)

A registration stores a callee in a field; the typed reading keys it by the
record type Clang resolved (`record`), or by none when no typed fact names the
site (`any`: the field-keyed reading). A dispatch through a field is typed by
the records Clang resolved for it, untyped, or unmatched (a cast, a union, an
anonymous record). Type-keyed dispatch is a sub-relation of field-keyed
dispatch (`type_keyed_is_a_sub_relation`), so it only removes edges; an untyped
dispatch is exactly the field-keyed one (`an_untyped_dispatch_is_field_keyed`);
a call through record `T` never reaches a registrant stored only in another
record (`a_call_through_a_record_reaches_only_its_registrants`); every removed
edge is a cross-type one or belongs to an unmatched dispatch, whose every
registrant is unresolved and so never discharged
(`a_removed_edge_is_cross_type_or_refused`,
`an_unmatched_dispatch_never_discharges`). -/

inductive Keyed where
  | any
  | record (name : String)
  deriving DecidableEq, Repr

inductive Typing where
  | untyped
  | records (names : List String)
  | unmatched
  deriving DecidableEq, Repr

structure Registration (α : Type) where
  field : String
  callee : α
  keys : List Keyed

def keyReached (names : List String) : Keyed → Bool
  | .any => true
  | .record r => names.contains r

/-- Whether a dispatch typed `typing` reaches a registration keyed `keys`. -/
def reaches (typing : Typing) (keys : List Keyed) : Bool :=
  match typing with
  | .untyped => true
  | .records names => keys.any (keyReached names)
  | .unmatched => false

/-- Field-keyed dispatch: every registrant of the field. -/
def fieldKeyed {α : Type} (regs : List (Registration α)) (field : String) : List α :=
  (regs.filter fun r => r.field == field).map (·.callee)

/-- Type-keyed dispatch. -/
def typeKeyed {α : Type} (regs : List (Registration α)) (field : String) (typing : Typing) :
    List α :=
  (regs.filter fun r => r.field == field && reaches typing r.keys).map (·.callee)

/-- The registrants an unmatched dispatch leaves unresolved: all of the field's. -/
def unmatchedUnresolved {α : Type} (regs : List (Registration α)) (field : String) :
    Typing → List α
  | .unmatched => fieldKeyed regs field
  | _ => []

theorem type_keyed_is_a_sub_relation {α : Type} {regs : List (Registration α)}
    {field : String} {typing : Typing} {c : α} (h : c ∈ typeKeyed regs field typing) :
    c ∈ fieldKeyed regs field := by
  simp only [typeKeyed, fieldKeyed, List.mem_map, List.mem_filter, Bool.and_eq_true] at h ⊢
  obtain ⟨r, ⟨hr, hf, _⟩, hc⟩ := h
  exact ⟨r, ⟨hr, hf⟩, hc⟩

theorem an_untyped_dispatch_is_field_keyed {α : Type} (regs : List (Registration α))
    (field : String) : typeKeyed regs field .untyped = fieldKeyed regs field := by
  simp [typeKeyed, fieldKeyed, reaches]

/-- A registration stored only in records other than the dispatch's is never
reached. -/
theorem a_call_through_a_record_reaches_only_its_registrants
    {names : List String} {keys : List Keyed}
    (hkeys : ∀ k ∈ keys, ∃ u, k = .record u ∧ u ∉ names) :
    reaches (.records names) keys = false := by
  simp only [reaches, List.any_eq_false]
  intro k hk
  obtain ⟨u, rfl, hu⟩ := hkeys k hk
  simp [keyReached, hu]

/-- An edge the typed reading removes is a cross-type one, or its dispatch is
unmatched and the callee is unresolved. -/
theorem a_removed_edge_is_cross_type_or_refused {α : Type} {regs : List (Registration α)}
    {field : String} {typing : Typing} {c : α}
    (hfield : c ∈ fieldKeyed regs field) (htyped : c ∉ typeKeyed regs field typing) :
    (typing = .unmatched ∧ c ∈ unmatchedUnresolved regs field typing) ∨
      ∃ names, typing = .records names ∧
        ∀ r ∈ regs, r.field = field → r.callee = c →
          ∀ k ∈ r.keys, ∃ u, k = .record u ∧ u ∉ names := by
  cases typing with
  | untyped => exact absurd (by rw [an_untyped_dispatch_is_field_keyed]; exact hfield) htyped
  | unmatched => exact Or.inl ⟨rfl, hfield⟩
  | records names =>
    refine Or.inr ⟨names, rfl, ?_⟩
    intro r hr hf hc k hk
    have hnot : reaches (.records names) r.keys = false := by
      cases hreach : reaches (.records names) r.keys with
      | false => rfl
      | true =>
        exfalso
        apply htyped
        simp only [typeKeyed, List.mem_map, List.mem_filter, Bool.and_eq_true]
        exact ⟨r, ⟨hr, by simp [hf], hreach⟩, hc⟩
    simp only [reaches, List.any_eq_false] at hnot
    have hk' := hnot k hk
    cases k with
    | any => simp [keyReached] at hk'
    | record u => exact ⟨u, rfl, by simpa [keyReached] using hk'⟩

/-- An unmatched dispatch leaves every registrant of its field unresolved, and
an operation in one is never discharged. -/
theorem an_unmatched_dispatch_never_discharges {α : Type} [DecidableEq α]
    {regs : List (Registration α)} {field : String} {c : α}
    (hfield : c ∈ fieldKeyed regs field) (owners : Owners) (contexts : List Context)
    (unresolved : Bool) :
    siteVerdict owners contexts
      (unresolved || (unmatchedUnresolved regs field .unmatched).contains c) ≠ .discharged := by
  have h : (unmatchedUnresolved regs field .unmatched).contains c = true := by
    simpa [unmatchedUnresolved] using hfield
  rw [h, Bool.or_true]
  exact unresolved_is_never_discharged owners contexts

/-! ### Step B: the instance flag of a runtime plugin copy

The route switch reads the instance's own coroutine flag, which its
constructor sets from the plugin it was created from; a runtime copy of the
plugin installed on the instance is joined with it conservatively. The route
is dropped only when both derive no coroutine
(`the_route_is_dropped_only_when_both_derive_none`); a copy written with the
coroutine flag, or either part underived, keeps it
(`a_coroutine_copy_keeps_the_route`, `an_underived_part_keeps_the_route`). -/

def joinFlag : Option Bool → Option Bool → Option Bool
  | some true, _ => some true
  | _, some true => some true
  | some false, some false => some false
  | _, _ => none

/-- The flag of an instance created from `creating` and running a copy whose
flags derive `copy`. -/
def instanceFlag (creating copy : Option Bool) : Option Bool := joinFlag creating copy

theorem no_coroutine_needs_both (creating copy : Option Bool) :
    instanceFlag creating copy = some false ↔ creating = some false ∧ copy = some false := by
  cases creating with
  | none => cases copy with
    | none => simp [instanceFlag, joinFlag]
    | some b => cases b <;> simp [instanceFlag, joinFlag]
  | some a => cases a <;> cases copy with
    | none => simp [instanceFlag, joinFlag]
    | some b => cases b <;> simp [instanceFlag, joinFlag]

theorem a_coroutine_copy_keeps_the_route {α : Type} {registrants : List α} {r : α}
    (creating : Option Bool) (hr : r ∈ registrants) :
    r ∈ coroutineDispatched (fun _ => instanceFlag creating (some true)) registrants := by
  have : instanceFlag creating (some true) = some true := by
    cases creating with
    | none => rfl
    | some a => cases a <;> rfl
  simp [coroutineDispatched, this, hr]

theorem an_underived_part_keeps_the_route {α : Type} {registrants : List α} {r : α}
    {creating copy : Option Bool} (hpart : creating = none ∨ copy = none) (hr : r ∈ registrants) :
    r ∈ coroutineDispatched (fun _ => instanceFlag creating copy) registrants := by
  have hne : instanceFlag creating copy ≠ some false := by
    intro h
    obtain ⟨h1, h2⟩ := (no_coroutine_needs_both creating copy).mp h
    rcases hpart with h | h <;> simp_all
  simp [coroutineDispatched, hr, hne]

theorem the_route_is_dropped_only_when_both_derive_none {α : Type} {registrants : List α}
    {r : α} {creating copy : Option Bool} (hr : r ∈ registrants)
    (hdropped : r ∉ coroutineDispatched (fun _ => instanceFlag creating copy) registrants) :
    creating = some false ∧ copy = some false := by
  apply (no_coroutine_needs_both creating copy).mp
  cases h : instanceFlag creating copy with
  | none => exact absurd (by simp [coroutineDispatched, hr, h]) hdropped
  | some b =>
    cases b with
    | false => rfl
    | true => exact absurd (by simp [coroutineDispatched, hr, h]) hdropped

/-! ### The list-head instance (card W47-LOOP-OWNER-INSTANCE-FLOW)

On a path from a fresh context (its input list empty, P-FRESH-CONTEXT) to the
read of the list's head, each call either creates one instance from a
descriptor whose coroutine flag is known (the creation appends it), leaves the
list alone, or is an unmodelled mutation. The census derives the head's flag
only when no path mutates the list unmodelled and every path creates exactly
once; the derived flag is then the flag of the only element the list holds
(`a_derived_head_is_the_one_creation`). A second creation or an unmodelled
mutation leaves it underived, which keeps the coroutine route
(`a_second_creator_keeps_the_route`, `an_unmodelled_mutation_keeps_the_route`).
The empty list's read is P-HEAD-INSTANCE's, outside the census. -/

inductive PathEvent where
  /-- A creation from a descriptor whose coroutine flag is `flag`. -/
  | create (flag : Bool)
  /-- A call that neither creates an instance nor mutates an input list. -/
  | other
  /-- A list mutation the census does not model. -/
  | mutate
  deriving DecidableEq, Repr

/-- The input list after one event; `none` once it is not known. -/
def listStep : Option (List Bool) → PathEvent → Option (List Bool)
  | none, _ => none
  | some l, .create flag => some (l ++ [flag])
  | some l, .other => some l
  | some _, .mutate => none

/-- The list the path leaves, from the fresh context's empty list. -/
def listAfter (events : List PathEvent) : Option (List Bool) :=
  events.foldl listStep (some [])

def created : List PathEvent → List Bool
  | [] => []
  | .create flag :: rest => flag :: created rest
  | _ :: rest => created rest

def mutates (events : List PathEvent) : Bool :=
  events.any (· == .mutate)

/-- The census's derivation on one path. -/
def headDerivation (events : List PathEvent) : Option Bool :=
  if mutates events then none
  else match created events with
    | [flag] => some flag
    | _ => none

/-- Over every path: one flag all paths derive, or none. -/
def pathsDerivation : List (List PathEvent) → Option Bool
  | [] => none
  | first :: rest =>
    match headDerivation first with
    | none => none
    | some flag => if rest.all (fun p => headDerivation p == some flag) then some flag else none

theorem listStep_unmutated (l : List Bool) (events : List PathEvent) (h : mutates events = false) :
    events.foldl listStep (some l) = some (l ++ created events) := by
  induction events generalizing l with
  | nil => simp [created]
  | cons e rest ih =>
    have hrest : mutates rest = false := by
      simp [mutates, List.any_cons] at h ⊢; exact h.2
    cases e with
    | create flag =>
      simp only [List.foldl_cons, listStep, created]
      rw [ih _ hrest]; simp
    | other =>
      simp only [List.foldl_cons, listStep, created]
      exact ih _ hrest
    | mutate => simp [mutates] at h

/-- A derived head is the one creation: the list holds exactly that instance. -/
theorem a_derived_head_is_the_one_creation {events : List PathEvent} {flag : Bool}
    (h : headDerivation events = some flag) : listAfter events = some [flag] := by
  unfold headDerivation at h
  cases hm : mutates events with
  | true => simp [hm] at h
  | false =>
    unfold listAfter
    rw [listStep_unmutated [] events hm]
    simp only [hm, Bool.false_eq_true, ite_false] at h
    cases hc : created events with
    | nil => simp [hc] at h
    | cons x rest =>
      cases rest with
      | nil =>
        simp only [hc, Option.some.injEq] at h
        simp [h]
      | cons y rest' => simp [hc] at h

/-- Every path agrees: each leaves exactly the one created instance. -/
theorem every_path_reads_the_one_creation {paths : List (List PathEvent)} {flag : Bool}
    (h : pathsDerivation paths = some flag) :
    ∀ p ∈ paths, listAfter p = some [flag] := by
  cases paths with
  | nil => simp [pathsDerivation] at h
  | cons first rest =>
    unfold pathsDerivation at h
    cases hf : headDerivation first with
    | none => simp [hf] at h
    | some f =>
      simp only [hf] at h
      split at h
      · rename_i hall
        have hfl : f = flag := by simpa using h
        subst hfl
        intro p hp
        rcases List.mem_cons.mp hp with hp | hp
        · subst hp; exact a_derived_head_is_the_one_creation hf
        · have := List.all_eq_true.mp hall p hp
          exact a_derived_head_is_the_one_creation (by simpa using this)
      · simp at h

theorem a_second_creator_keeps_the_route {events : List PathEvent}
    (h : 2 ≤ (created events).length) : headDerivation events = none := by
  unfold headDerivation
  cases hm : mutates events with
  | true => simp
  | false =>
    simp only [Bool.false_eq_true, ite_false]
    cases hc : created events with
    | nil => rfl
    | cons x rest =>
      cases rest with
      | nil => rw [hc] at h; simp at h
      | cons y rest' => rfl

theorem an_unmodelled_mutation_keeps_the_route {events : List PathEvent}
    (h : PathEvent.mutate ∈ events) : headDerivation events = none := by
  have : mutates events = true := List.any_eq_true.mpr ⟨_, h, by simp⟩
  simp [headDerivation, this]

theorem a_path_without_a_creation_derives_nothing {events : List PathEvent}
    (h : created events = []) : headDerivation events = none := by
  unfold headDerivation
  cases mutates events with
  | true => simp
  | false => simp [h]

/-- An underived head keeps the coroutine route for the copy's registrant. -/
theorem an_underived_head_keeps_the_route {α : Type} {registrants : List α} {r : α}
    {paths : List (List PathEvent)} (copy : Option Bool) (hr : r ∈ registrants)
    (h : pathsDerivation paths = none) :
    r ∈ coroutineDispatched (fun _ => instanceFlag (pathsDerivation paths) copy) registrants := by
  rw [h]
  exact an_underived_part_keeps_the_route (Or.inl rfl) hr

/-! ### The holder escape

The object holding the plugin copy escapes into the installed instance's
`data`. The route reads the instance's coroutine flag and which instance the
callback is registered on; an escape is admitted only when what it can reach
writes neither (the flag has one writer, the instance's plugin field two, and
the holder's type is named in the test alone). An escape that can write
either leaves the derivation underived. -/

structure EscapeReach where
  writesFlag : Bool
  writesInstance : Bool
  deriving DecidableEq, Repr

def afterEscape (derived : Option Bool) (escape : EscapeReach) : Option Bool :=
  if escape.writesFlag || escape.writesInstance then none else derived

theorem an_escape_that_writes_neither_keeps_the_derivation (derived : Option Bool) :
    afterEscape derived { writesFlag := false, writesInstance := false } = derived := by
  simp [afterEscape]

theorem an_escape_that_writes_the_flag_or_the_instance_keeps_the_route {α : Type}
    {registrants : List α} {r : α} {derived copy : Option Bool} {escape : EscapeReach}
    (hw : escape.writesFlag = true ∨ escape.writesInstance = true) (hr : r ∈ registrants) :
    r ∈ coroutineDispatched (fun _ => instanceFlag (afterEscape derived escape) copy)
      registrants := by
  have : afterEscape derived escape = none := by
    rcases hw with h | h <;> simp [afterEscape, h]
  rw [this]
  exact an_underived_part_keeps_the_route (Or.inl rfl) hr

/-- The route is dropped only through a derived single creation whose
descriptor runs no coroutine, an escape that writes neither fact, and a copy
that runs none. -/
theorem the_route_is_dropped_only_through_one_non_coroutine_creation {α : Type}
    {registrants : List α} {r : α} {paths : List (List PathEvent)} {escape : EscapeReach}
    {copy : Option Bool} (hr : r ∈ registrants)
    (hdropped : r ∉ coroutineDispatched
      (fun _ => instanceFlag (afterEscape (pathsDerivation paths) escape) copy) registrants) :
    escape.writesFlag = false ∧ escape.writesInstance = false ∧ copy = some false ∧
      ∀ p ∈ paths, listAfter p = some [false] := by
  obtain ⟨hcreating, hcopy⟩ := the_route_is_dropped_only_when_both_derive_none hr hdropped
  unfold afterEscape at hcreating
  by_cases hw : (escape.writesFlag || escape.writesInstance) = true
  · simp [hw] at hcreating
  · simp only [Bool.or_eq_true, not_or, Bool.not_eq_true] at hw
    simp only [hw.1, hw.2, Bool.or_false, Bool.false_eq_true, ite_false] at hcreating
    exact ⟨hw.1, hw.2, hcopy, every_path_reads_the_one_creation hcreating⟩

/-! ### The old test's path

The previous test (`90978ac6~1`) calls the collector operations from the
test thread itself; that context is resolved and off both loops' owners, so
the site is refused whatever the instance derivation says. -/

theorem the_old_test_path_is_never_discharged {owners : Owners} {contexts : List Context}
    {unresolved : Bool} {t : Thread} {list : List Thread}
    (hc : { thread := t, mode := .either } ∈ contexts)
    (hl : candidates owners .either = some list) (hn : t ∉ list) :
    siteVerdict owners contexts unresolved ≠ .discharged := by
  rw [a_non_owner_context_refuses hc hl hn]
  simp

/-! ### The keys a property setter carries (card W47-SERVICE-PROPERTY-KEYS)

A setter call `S(ctx, K1, V1, …, Kn, Vn, NULL)` hands its keys, in order, to a
property function that runs, for every key-table entry the key matches, that
entry's arm (a handler, or a scalar store), besides a part common to every
key. The census reads the call as reaching the common part and the reach of
every entry some key matches — only when the argument list is a
NULL-terminated sequence of literal key/value pairs and every key matches an
entry; any other list, or a key no entry matches, keeps every branch
(`every`, the whole call). The run covered is any prefix of the keys: the
setter stops at the first failing key. Case variants match the same entries
under the derived comparison (ASCII case folding). A read reach that holds a
hazard is a mutation on the list-head path, so the route is kept. The old
test's path is refused by `the_old_test_path_is_never_discharged`, which
does not depend on the instance flag this reading feeds. -/

/-- One variadic argument as the typed reading records it. -/
inductive Arg where
  | str (text : String)
  | null
  | other
  deriving DecidableEq, Repr

/-- The literal keys of a NULL-terminated key/value tail, or none. -/
def literalKeys : List Arg → Option (List String)
  | [.null] => some []
  | .str k :: v :: rest => if v = .null then none else (literalKeys rest).map (k :: ·)
  | _ => none

/-- One key-table entry and what its arm reaches. -/
structure KeyEntry (α : Type) where
  key : String
  reach : List α

/-- The entries a key matches under the derived comparison. -/
def matchingEntries {α : Type} (eq : String → String → Bool) (table : List (KeyEntry α))
    (k : String) : List (KeyEntry α) :=
  table.filter (fun e => eq e.key k)

/-- What the census reads a setter call as reaching. -/
def keyedReach {α : Type} (eq : String → String → Bool) (table : List (KeyEntry α))
    (common every : List α) (args : List Arg) : List α :=
  match literalKeys args with
  | none => every
  | some keys =>
    if keys.all (fun k => !(matchingEntries eq table k).isEmpty) then
      common ++ keys.flatMap (fun k => (matchingEntries eq table k).flatMap KeyEntry.reach)
    else every

/-- What a run over the processed keys `done` reaches. -/
def keyRun {α : Type} (eq : String → String → Bool) (table : List (KeyEntry α))
    (common : List α) (done : List String) : List α :=
  common ++ done.flatMap (fun k => (matchingEntries eq table k).flatMap KeyEntry.reach)

theorem literalKeys_cons_str {k : String} {v : Arg} {rest : List Arg} {keys : List String}
    (h : literalKeys (.str k :: v :: rest) = some keys) :
    v ≠ .null ∧ ∃ tail, literalKeys rest = some tail ∧ keys = k :: tail := by
  unfold literalKeys at h
  by_cases hv : v = .null
  · simp [hv] at h
  · simp only [hv, ite_false] at h
    cases hr : literalKeys rest with
    | none => simp [hr] at h
    | some tail =>
      simp only [hr, Option.map_some, Option.some.injEq] at h
      exact ⟨hv, tail, rfl, h.symm⟩

/-- A read key list is NULL-terminated, with one value after each key and no
other null argument: exactly `2n + 1` arguments for `n` keys, the last null. -/
theorem a_read_key_list_is_null_terminated_and_paired :
    ∀ (args : List Arg) (keys : List String), literalKeys args = some keys →
      args.length = 2 * keys.length + 1 ∧ args.getLast? = some .null
  | [], _, h => by simp [literalKeys] at h
  | [.null], keys, h => by
    simp only [literalKeys, Option.some.injEq] at h
    subst h; simp
  | [.str _], _, h => by simp [literalKeys] at h
  | [.other], _, h => by simp [literalKeys] at h
  | .null :: _ :: _, _, h => by simp [literalKeys] at h
  | .other :: _ :: _, _, h => by simp [literalKeys] at h
  | .str k :: v :: rest, keys, h => by
    obtain ⟨_, tail, htail, hkeys⟩ := literalKeys_cons_str h
    obtain ⟨hlen, hlast⟩ := a_read_key_list_is_null_terminated_and_paired rest tail htail
    subst hkeys
    refine ⟨by simp [hlen]; omega, ?_⟩
    cases rest with
    | nil => simp at hlen
    | cons r rs =>
      simp only [List.getLast?_cons_cons] at hlast ⊢
      exact hlast

/-- An argument list the census cannot read keeps every branch. -/
theorem an_unreadable_argument_keeps_every_branch {α : Type} {eq : String → String → Bool}
    {table : List (KeyEntry α)} {common every : List α} {args : List Arg}
    (h : literalKeys args = none) : keyedReach eq table common every args = every := by
  simp [keyedReach, h]

theorem a_non_literal_key_is_unreadable (rest : List Arg) :
    literalKeys (.other :: rest) = none := by
  cases rest <;> simp [literalKeys]

theorem an_odd_list_is_unreadable (k : String) : literalKeys [.str k, .null] = none := by
  simp [literalKeys]

theorem an_unterminated_list_is_unreadable (k v : String) :
    literalKeys [.str k, .str v] = none := by
  simp [literalKeys]

/-- A key no entry of the derived table matches keeps every branch. -/
theorem a_key_absent_from_the_table_keeps_every_branch {α : Type}
    {eq : String → String → Bool} {table : List (KeyEntry α)} {common every : List α}
    {args : List Arg} {keys : List String} {k : String}
    (hk : literalKeys args = some keys) (hin : k ∈ keys)
    (hnone : matchingEntries eq table k = []) :
    keyedReach eq table common every args = every := by
  have hnot : ¬ (keys.all (fun k => !(matchingEntries eq table k).isEmpty) = true) := by
    intro hall
    have := List.all_eq_true.mp hall k hin
    simp [hnone] at this
  simp only [keyedReach, hk]
  simp [hnot]

/-- A literal key sequence whose keys all match reaches only the common part
and its keys' matched entries' reach. -/
theorem a_literal_key_sequence_reaches_only_its_keys_effects {α : Type}
    {eq : String → String → Bool} {table : List (KeyEntry α)} {common every : List α}
    {args : List Arg} {keys : List String} {x : α}
    (hk : literalKeys args = some keys)
    (hall : keys.all (fun k => !(matchingEntries eq table k).isEmpty) = true)
    (hx : x ∈ keyedReach eq table common every args) :
    x ∈ common ∨ ∃ k ∈ keys, ∃ e ∈ table, eq e.key k = true ∧ x ∈ e.reach := by
  simp only [keyedReach, hk, hall, ite_true, List.mem_append, List.mem_flatMap] at hx
  rcases hx with hx | ⟨k, hk', e, he, hxe⟩
  · exact Or.inl hx
  · simp only [matchingEntries, List.mem_filter] at he
    exact Or.inr ⟨k, hk', e, he.1, he.2, hxe⟩

/-- The reading covers every run: whatever prefix of the keys the setter
processes before it stops, what that run reaches is read, provided the whole
call's reach covers the common part and every entry's reach. -/
theorem the_key_reading_covers_the_run {α : Type} {eq : String → String → Bool}
    {table : List (KeyEntry α)} {common every : List α} {args : List Arg}
    {keys done : List String}
    (hevery : ∀ x, (x ∈ common ∨ ∃ e ∈ table, x ∈ e.reach) → x ∈ every)
    (hk : literalKeys args = some keys) (hdone : done <+: keys) :
    ∀ x ∈ keyRun eq table common done, x ∈ keyedReach eq table common every args := by
  intro x hx
  simp only [keyRun, List.mem_append, List.mem_flatMap] at hx
  simp only [keyedReach, hk]
  split
  · simp only [List.mem_append, List.mem_flatMap]
    rcases hx with hx | ⟨k, hkd, e, he, hxe⟩
    · exact Or.inl hx
    · exact Or.inr ⟨k, hdone.subset hkd, e, he, hxe⟩
  · apply hevery
    rcases hx with hx | ⟨_, _, e, he, hxe⟩
    · exact Or.inl hx
    · simp only [matchingEntries, List.mem_filter] at he
      exact Or.inr ⟨e, he.1, hxe⟩

/-- A matched key's hazard (the `Plugins_File` registration) stays in the
read reach. -/
theorem a_matched_hazard_stays_read {α : Type} {eq : String → String → Bool}
    {table : List (KeyEntry α)} {common every : List α} {args : List Arg}
    {keys : List String} {k : String} {e : KeyEntry α} {h : α}
    (hevery : ∀ x, (x ∈ common ∨ ∃ e ∈ table, x ∈ e.reach) → x ∈ every)
    (hk : literalKeys args = some keys) (hin : k ∈ keys) (he : e ∈ table)
    (hmatch : eq e.key k = true) (hh : h ∈ e.reach) :
    h ∈ keyedReach eq table common every args := by
  apply the_key_reading_covers_the_run hevery hk (List.prefix_refl keys)
  simp only [keyRun, List.mem_append, List.mem_flatMap]
  exact Or.inr ⟨k, hin, e, by simp [matchingEntries, List.mem_filter, he, hmatch], hh⟩

/-- The derived comparison: equality after ASCII case folding. -/
def foldKey (s : String) : List Char := s.toList.map Char.toLower

def foldEq (a b : String) : Bool := foldKey a == foldKey b

/-- Case variants match the same entries. -/
theorem a_case_variant_matches_the_same_entries {α : Type} (table : List (KeyEntry α))
    {a b : String} (h : foldKey a = foldKey b) :
    matchingEntries foldEq table a = matchingEntries foldEq table b := by
  unfold matchingEntries foldEq
  simp [h]

/-- The setter call as a list-head path event: a read reach holding a hazard
is a mutation the single-creation derivation does not admit. -/
def setterEvent {α : Type} (hazard : α → Bool) (reach : List α) : PathEvent :=
  if reach.any hazard then .mutate else .other

theorem a_key_reach_holding_a_hazard_keeps_the_route {α : Type} {hazard : α → Bool}
    {reach : List α} {x : α} {before after : List PathEvent}
    (hx : x ∈ reach) (hh : hazard x = true) :
    headDerivation (before ++ setterEvent hazard reach :: after) = none := by
  have hany : reach.any hazard = true := List.any_eq_true.mpr ⟨x, hx, hh⟩
  apply an_unmodelled_mutation_keeps_the_route
  simp [setterEvent, hany]

theorem a_key_reach_without_a_hazard_leaves_the_one_creation {α : Type}
    {hazard : α → Bool} {reach : List α} {flag : Bool}
    (h : reach.any hazard = false) :
    headDerivation [setterEvent hazard reach, .create flag] = some flag := by
  simp [setterEvent, h, headDerivation, mutates, created]

/-! ### Predefined-macro arms (card W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS)

A reserved macro is decided from the recorded compiler, per command. A
preprocessed unit starts from the macros its compiler defines under the
command's flags (the probe: built-ins, then the command line's `-D`/`-U` in
order) and then processes its files' `#define`/`#undef` directives in order;
a condition is evaluated at some point of that sequence. The reading decides
a name *defined* when every unit of the file starts with it and no directive
of any of them `#undef`s it, *undefined* when none starts with it and no
directive `#define`s it, and nothing otherwise, or when the derivation is
absent (a compiler that is unavailable or not the recorded one, a failed
run). Such a decision agrees with the name's definedness at every point of
every unit (`a_predefined_macro_behaves_as_an_owned_macro`), so the arm the
preprocessor takes is kept (`the_taken_arm_is_read`). A condition's reading
is three-valued and agrees with the preprocessor's value whenever its names'
decisions do (`the_condition_reading_agrees_with_the_preprocessor`); an
arithmetic comparison is never evaluated and keeps its arm
(`an_undecidable_comparison_keeps_the_arm`); a missing derivation decides
nothing and keeps every arm (`a_compiler_mismatch_decides_nothing`,
`a_compiler_mismatch_keeps_every_arm`); a `-U` after a `-D` undefines
(`a_minus_u_undefines`); a file's own directive keeps the opposite decision
from being made (`a_file_directive_wins`). None of it bears on the old
test's path, which is refused on its test-thread context whatever the arms
(`the_old_test_path_is_never_discharged_whatever_the_arms`). -/

inductive Directive where
  | define (name : String)
  | undef (name : String)
  deriving DecidableEq, Repr

/-- One directive's effect on the defined set. -/
def applyDirective (defined : String → Bool) : Directive → String → Bool
  | .define m, n => if n = m then true else defined n
  | .undef m, n => if n = m then false else defined n

/-- The defined set after a sequence of directives, in order. -/
def definedAfter (defined : String → Bool) : List Directive → String → Bool
  | [] => defined
  | d :: rest => definedAfter (applyDirective defined d) rest

/-- One preprocessed unit: the macros defined at its start (the probe), and
its files' directives in the order the preprocessor processes them. -/
structure PUnit where
  start : String → Bool
  events : List Directive

/-- The decision for a name over a file's units; `none` is a derivation that
is absent or failed. -/
def decidePredefined : Option (List PUnit) → String → Truth
  | none, _ => .unknown
  | some [], _ => .unknown
  | some units, n =>
    if units.all (fun u => u.start n && !u.events.contains (.undef n)) then .yes
    else if units.all (fun u => !u.start n && !u.events.contains (.define n)) then .no
    else .unknown

theorem definedAfter_without_undef (n : String) :
    ∀ (events : List Directive) (d : String → Bool),
      (.undef n) ∉ events → d n = true → definedAfter d events n = true := by
  intro events
  induction events with
  | nil => intro d _ h; simpa [definedAfter] using h
  | cons e rest ih =>
    intro d hnot hd
    simp only [List.mem_cons, not_or] at hnot
    apply ih _ hnot.2
    cases e with
    | define m => by_cases hm : n = m <;> simp [applyDirective, hm, hd]
    | undef m =>
      have : n ≠ m := fun h => hnot.1 (by rw [h])
      simp [applyDirective, this, hd]

theorem definedAfter_without_define (n : String) :
    ∀ (events : List Directive) (d : String → Bool),
      (.define n) ∉ events → d n = false → definedAfter d events n = false := by
  intro events
  induction events with
  | nil => intro d _ h; simpa [definedAfter] using h
  | cons e rest ih =>
    intro d hnot hd
    simp only [List.mem_cons, not_or] at hnot
    apply ih _ hnot.2
    cases e with
    | undef m => by_cases hm : n = m <;> simp [applyDirective, hm, hd]
    | define m =>
      have : n ≠ m := fun h => hnot.1 (by rw [h])
      simp [applyDirective, this, hd]

/-- A decided reserved macro agrees with its definedness at every point
(`k` directives processed) of every unit of the file. -/
theorem a_predefined_macro_behaves_as_an_owned_macro {units : List PUnit} {n : String}
    {u : PUnit} (hu : u ∈ units) (k : Nat) :
    agrees (decidePredefined (some units) n) (definedAfter u.start (u.events.take k) n) = true := by
  cases units with
  | nil => simp at hu
  | cons first rest =>
    simp only [decidePredefined]
    split
    · rename_i hall
      rw [List.all_eq_true] at hall
      have h := hall u hu
      simp only [Bool.and_eq_true, Bool.not_eq_true', List.contains_eq_mem,
        decide_eq_false_iff_not] at h
      have hk : (.undef n) ∉ u.events.take k := fun hm => h.2 (List.mem_of_mem_take hm)
      simp [agrees, definedAfter_without_undef n _ _ hk h.1]
    · split
      · rename_i _ hall
        rw [List.all_eq_true] at hall
        have h := hall u hu
        simp only [Bool.and_eq_true, Bool.not_eq_true', List.contains_eq_mem,
          decide_eq_false_iff_not] at h
        have hk : (.define n) ∉ u.events.take k := fun hm => h.2 (List.mem_of_mem_take hm)
        simp [agrees, definedAfter_without_define n _ _ hk h.1]
      · simp [agrees]

/-- An absent or failed derivation decides nothing. -/
theorem a_compiler_mismatch_decides_nothing (n : String) :
    decidePredefined none n = .unknown := rfl

/-- A `-U` after a `-D` of the same name on the command line: the unit starts
without it, and with no `#define` of it in its files the name is decided
undefined. -/
theorem a_minus_u_undefines (builtin : String → Bool) (commandLine : List Directive)
    (n : String) (events : List Directive) (h : (.define n) ∉ events) :
    decidePredefined
        (some [{ start := definedAfter builtin (commandLine ++ [.define n, .undef n]),
                 events := events }]) n = .no := by
  have hstart : definedAfter builtin (commandLine ++ [.define n, .undef n]) n = false := by
    have : ∀ (d : String → Bool) (pre : List Directive),
        definedAfter d (pre ++ [.define n, .undef n]) n = false := by
      intro d pre
      induction pre generalizing d with
      | nil => simp [definedAfter, applyDirective]
      | cons e rest ih => simpa [definedAfter] using ih (applyDirective d e)
    exact this builtin commandLine
  simp [decidePredefined, hstart, h]

/-- A file's own directive wins: a `#define` anywhere in a unit keeps the
name from being decided undefined, an `#undef` from being decided defined. -/
theorem a_file_directive_wins {units : List PUnit} {n : String} {u : PUnit}
    (hu : u ∈ units) :
    ((.define n) ∈ u.events → decidePredefined (some units) n ≠ .no) ∧
    ((.undef n) ∈ u.events → decidePredefined (some units) n ≠ .yes) := by
  constructor
  · intro hdef hno
    have hagree := a_predefined_macro_behaves_as_an_owned_macro (n := n) hu u.events.length
    rw [hno] at hagree
    cases units with
    | nil => simp at hu
    | cons first rest =>
      simp only [decidePredefined] at hno
      split at hno
      · simp at hno
      · split at hno
        · rename_i _ hall
          rw [List.all_eq_true] at hall
          have h := hall u hu
          simp only [Bool.and_eq_true, Bool.not_eq_true', List.contains_eq_mem,
            decide_eq_false_iff_not] at h
          exact h.2 hdef
        · simp at hno
  · intro hundef hyes
    cases units with
    | nil => simp at hu
    | cons first rest =>
      simp only [decidePredefined] at hyes
      split at hyes
      · rename_i hall
        rw [List.all_eq_true] at hall
        have h := hall u hu
        simp only [Bool.and_eq_true, Bool.not_eq_true', List.contains_eq_mem,
          decide_eq_false_iff_not] at h
        exact h.2 hundef
      · split at hyes <;> simp at hyes

/-! A condition as the directive reader reads it: `defined`, `!`, `&&`,
`||`, and anything else (an arithmetic comparison, a macro's value) as an
unread operand whose actual value is arbitrary. -/

inductive Cond where
  | defined (name : String)
  | not (c : Cond)
  | and (a b : Cond)
  | or (a b : Cond)
  | unread
  deriving Repr

def Truth.neg : Truth → Truth
  | .yes => .no
  | .no => .yes
  | .unknown => .unknown

def Truth.conj : Truth → Truth → Truth
  | .no, _ => .no
  | _, .no => .no
  | .yes, .yes => .yes
  | _, _ => .unknown

def Truth.disj (a b : Truth) : Truth := (a.neg.conj b.neg).neg

/-- The reading: three-valued, from the names' decisions. -/
def readCond (decide : String → Truth) : Cond → Truth
  | .defined n => decide n
  | .not c => (readCond decide c).neg
  | .and a b => (readCond decide a).conj (readCond decide b)
  | .or a b => (readCond decide a).disj (readCond decide b)
  | .unread => .unknown

/-- The preprocessor's value, `unread` operands taking any values. -/
def evalCond (defined : String → Bool) (unread : Bool) : Cond → Bool
  | .defined n => defined n
  | .not c => !evalCond defined unread c
  | .and a b => evalCond defined unread a && evalCond defined unread b
  | .or a b => evalCond defined unread a || evalCond defined unread b
  | .unread => unread

theorem agrees_neg {t : Truth} {b : Bool} (h : agrees t b = true) :
    agrees t.neg (!b) = true := by
  cases t <;> cases b <;> simp_all [agrees, Truth.neg]

theorem agrees_conj {s t : Truth} {a b : Bool} (ha : agrees s a = true)
    (hb : agrees t b = true) : agrees (s.conj t) (a && b) = true := by
  cases s <;> cases t <;> cases a <;> cases b <;> simp_all [agrees, Truth.conj]

theorem the_condition_reading_agrees_with_the_preprocessor
    {decide : String → Truth} {defined : String → Bool} (unread : Bool)
    (h : ∀ n, agrees (decide n) (defined n) = true) :
    ∀ c, agrees (readCond decide c) (evalCond defined unread c) = true := by
  intro c
  induction c with
  | defined n => exact h n
  | not c ih => exact agrees_neg ih
  | and a b iha ihb => exact agrees_conj iha ihb
  | or a b iha ihb =>
    have hn := agrees_neg (agrees_conj (agrees_neg iha) (agrees_neg ihb))
    simp only [readCond, evalCond, Truth.disj]
    simpa [Bool.not_and, Bool.not_not] using hn
  | unread => simp [readCond, agrees]

/-- An arithmetic comparison is never evaluated: its arm is kept whatever
the arms after it. -/
theorem keptArms_unknown_head (rest : List Truth) :
    keptArms false (.unknown :: rest) = true :: keptArms false rest := rfl

theorem an_undecidable_comparison_keeps_the_arm (decide : String → Truth)
    (rest : List Truth) :
    keptArms false (readCond decide .unread :: rest) =
      true :: keptArms false rest :=
  keptArms_unknown_head rest

theorem readCond_without_decisions : ∀ c, readCond (fun _ => .unknown) c = .unknown := by
  intro c
  induction c with
  | defined n => rfl
  | not c ih => simp [readCond, ih, Truth.neg]
  | and a b iha ihb => simp [readCond, iha, ihb, Truth.conj]
  | or a b iha ihb => simp [readCond, iha, ihb, Truth.disj, Truth.neg, Truth.conj]
  | unread => rfl

theorem keptArms_all_unknown : ∀ (conds : List Cond),
    keptArms false (conds.map (readCond (fun _ => .unknown))) = conds.map (fun _ => true) := by
  intro conds
  induction conds with
  | nil => rfl
  | cons c rest ih =>
    rw [List.map_cons, readCond_without_decisions, keptArms_unknown_head, ih]
    rfl

/-- Without a derivation every arm of every group is kept, as before this
card. -/
theorem a_compiler_mismatch_keeps_every_arm (conds : List Cond) :
    keptArms false (conds.map (readCond (decidePredefined none))) =
      conds.map (fun _ => true) := by
  have : decidePredefined none = fun _ => .unknown := rfl
  rw [this]
  exact keptArms_all_unknown conds

/-- The old test's collector operations run on the test thread whatever arms
a reading keeps: every reading's contexts contain that context, so none
discharges the site. -/
theorem the_old_test_path_is_never_discharged_whatever_the_arms {owners : Owners}
    {readings : List (List Context)} {unresolved : Bool} {t : Thread} {list : List Thread}
    (hc : ∀ cs ∈ readings, { thread := t, mode := .either } ∈ cs)
    (hl : candidates owners .either = some list) (hn : t ∉ list) :
    ∀ cs ∈ readings, siteVerdict owners cs unresolved ≠ .discharged := by
  intro cs hcs
  exact the_old_test_path_is_never_discharged (hc cs hcs) hl hn

/-! ### The list-head instance's premises (card W47-PREMISES-INSTANCE)

The premises Step B's list-head derivation and the owner obligation stated
are derived from the program, or narrowed.

**P-FRESH-CONTEXT.** The config initializer's own steps from its fresh
allocation: the allocation's contents are no list (`none`), the list
initialiser makes the empty list, a step the census reads as neither keeps
it, and a mutation leaves it unknown. A derived initializer (its list
initialisation at its top level, nothing after it but other steps) ends with
the empty list, which is where the list-head derivation starts; one without
its list initialisation derives none. -/

inductive InitStep where
  | alloc
  | initList
  | other
  | mutate
  deriving DecidableEq, Repr

def initStep : Option (List Bool) → InitStep → Option (List Bool)
  | _, .alloc => none
  | _, .initList => some []
  | l, .other => l
  | _, .mutate => none

/-- The list the initializer leaves, from its allocation's unknown contents. -/
def freshAfter (steps : List InitStep) : Option (List Bool) :=
  steps.foldl initStep none

theorem initStep_initList (l : Option (List Bool)) : initStep l .initList = some [] := by
  cases l <;> rfl

theorem initStep_other (l : Option (List Bool)) : initStep l .other = l := by
  cases l <;> rfl

theorem initStep_others (l : Option (List Bool)) (post : List InitStep)
    (h : ∀ e ∈ post, e = .other) : post.foldl initStep l = l := by
  induction post generalizing l with
  | nil => rfl
  | cons e rest ih =>
    have he : e = .other := h e (List.mem_cons_self ..)
    subst he
    rw [List.foldl_cons, initStep_other]
    exact ih l (fun x hx => h x (List.mem_cons_of_mem _ hx))

/-- A derived initializer leaves the empty list. -/
theorem a_derived_fresh_context_is_empty (pre post : List InitStep)
    (h : ∀ e ∈ post, e = .other) :
    freshAfter (pre ++ .initList :: post) = some [] := by
  unfold freshAfter
  rw [List.foldl_append, List.foldl_cons, initStep_initList]
  exact initStep_others _ post h

theorem initStep_none_without_init (e : InitStep) (h : e ≠ .initList) :
    initStep none e = none := by
  cases e <;> simp_all [initStep]

/-- Without its list initialisation the initializer derives no list. -/
theorem an_initializer_without_its_list_init_is_not_fresh (steps : List InitStep)
    (h : ∀ e ∈ steps, e ≠ .initList) : freshAfter steps = none := by
  unfold freshAfter
  induction steps with
  | nil => rfl
  | cons e rest ih =>
    rw [List.foldl_cons, initStep_none_without_init e (h e (List.mem_cons_self ..))]
    exact ih (fun x hx => h x (List.mem_cons_of_mem _ hx))

/-- A derived fresh context is exactly the list-head derivation's start. -/
theorem a_fresh_context_starts_the_head_derivation {init : List InitStep}
    (h : freshAfter init = some []) (events : List PathEvent) :
    events.foldl listStep (freshAfter init) = listAfter events := by
  rw [h]
  rfl

/-! **P-REGISTERED-DESCRIPTORS.** The registered list is the generated
registration's descriptors (`none` when it is not read). One creation from a
literal makes one instance per matching registered descriptor, flagged as
that descriptor. The census derives a flag only from a read registration
with at least one match; a derived "no coroutine" holds for every instance
the creation makes, and an unread registration keeps the route. -/

structure Descriptor where
  name : String
  coroutine : Bool
  deriving DecidableEq, Repr

def createdFlags (nameMatches : String → String → Bool) (registered : List Descriptor)
    (literal : String) : List Bool :=
  (registered.filter (fun d => nameMatches d.name literal)).map (·.coroutine)

def registeredFlag (nameMatches : String → String → Bool) :
    Option (List Descriptor) → String → Option Bool
  | none, _ => none
  | some registered, literal =>
    match createdFlags nameMatches registered literal with
    | [] => none
    | flags => some (flags.any id)

theorem an_unread_registration_derives_nothing (nameMatches : String → String → Bool)
    (literal : String) : registeredFlag nameMatches none literal = none := rfl

theorem a_derived_registration_has_a_match {nameMatches : String → String → Bool}
    {registered : List Descriptor} {literal : String} {b : Bool}
    (h : registeredFlag nameMatches (some registered) literal = some b) :
    createdFlags nameMatches registered literal ≠ [] := by
  intro hnil
  simp [registeredFlag, hnil] at h

theorem every_created_instance_runs_no_coroutine {nameMatches : String → String → Bool}
    {registered : List Descriptor} {literal : String}
    (h : registeredFlag nameMatches (some registered) literal = some false) :
    ∀ f ∈ createdFlags nameMatches registered literal, f = false := by
  intro f hf
  cases hc : createdFlags nameMatches registered literal with
  | nil => rw [hc] at hf; cases hf
  | cons x rest =>
    have hany : (x :: rest).any id = false := by
      simp only [registeredFlag, hc] at h
      exact Option.some.inj h
    rw [hc] at hf
    cases f with
    | false => rfl
    | true =>
      have : (x :: rest).any id = true := List.any_eq_true.mpr ⟨true, hf, rfl⟩
      rw [this] at hany
      exact absurd hany (by decide)

theorem an_unread_registration_keeps_the_route {α : Type} {registrants : List α} {r : α}
    (nameMatches : String → String → Bool) (literal : String) (copy : Option Bool)
    (hr : r ∈ registrants) :
    r ∈ coroutineDispatched
      (fun _ => instanceFlag (registeredFlag nameMatches none literal) copy) registrants :=
  an_underived_part_keeps_the_route (Or.inl rfl) hr

/-! **P-HEAD-INSTANCE, narrowed.** The constructor's block from its allocation
to its append is straight-line: every step is a plain one or a null return.
Executing it stops at the first null return, so every execution either
returns a null pointer or reaches the append: a non-null creation is on the
list. What stays stated is that the creation succeeds (P-CREATIONS-SUCCEED),
since a failed one leaves the fresh list empty and its head no instance. -/

inductive CtorStep where
  | plain
  | returnNull
  | append
  deriving DecidableEq, Repr

/-- One execution of a straight-line block: up to its first null return. -/
def execute : List CtorStep → List CtorStep
  | [] => []
  | .returnNull :: _ => [.returnNull]
  | .plain :: rest => .plain :: execute rest
  | .append :: rest => .append :: execute rest

theorem a_straight_block_appends_or_returns_null (body : List CtorStep)
    (hb : ∀ s ∈ body, s = .plain ∨ s = .returnNull) :
    .returnNull ∈ execute (body ++ [.append]) ∨ .append ∈ execute (body ++ [.append]) := by
  induction body with
  | nil => right; simp [execute]
  | cons s rest ih =>
    rcases hb s (List.mem_cons_self ..) with h | h
    · subst h
      have ih' := ih (fun x hx => hb x (List.mem_cons_of_mem _ hx))
      rw [List.cons_append, execute]
      rcases ih' with h' | h'
      · left; exact List.mem_cons_of_mem _ h'
      · right; exact List.mem_cons_of_mem _ h'
    · subst h
      left
      rw [List.cons_append, execute]
      exact List.mem_cons_self ..

/-- A creation that returns non-null has appended its instance. -/
theorem a_non_null_creation_is_on_the_list (body : List CtorStep)
    (hb : ∀ s ∈ body, s = .plain ∨ s = .returnNull)
    (hnonnull : .returnNull ∉ execute (body ++ [.append])) :
    .append ∈ execute (body ++ [.append]) := by
  rcases a_straight_block_appends_or_returns_null body hb with h | h
  · exact absurd h hnonnull
  · exact h

theorem listStep_others (l : List Bool) (events : List PathEvent)
    (h : ∀ e ∈ events, e = .other) : events.foldl listStep (some l) = some l := by
  induction events with
  | nil => rfl
  | cons e rest ih =>
    have he : e = .other := h e (List.mem_cons_self ..)
    subst he
    rw [List.foldl_cons]
    exact ih (fun x hx => h x (List.mem_cons_of_mem _ hx))

/-- Without the success premise the fresh list stays empty: its head is not
an instance, and the census says nothing of that read. -/
theorem a_failed_creation_leaves_the_fresh_list_empty (events : List PathEvent)
    (h : ∀ e ∈ events, e = .other) : listAfter events = some [] :=
  listStep_others [] events h

/-! **P-INSTANCE.** A collector's instance is the last value written to it; a
collector written only by its constructor names its registration's instance,
and a loop dispatch passes that instance as the callback's first argument, so
an operation on the callback's unassigned first parameter names the instance
whose loop dispatched it. A refuted derivation withholds the discharge; the
premised verdict never adds one. -/

def collectorInstance (writes : List Nat) : Option Nat := writes.getLast?

theorem a_constructor_only_collector_names_its_registration (r : Nat) :
    collectorInstance [r] = some r := rfl

theorem a_repointed_collector_may_name_another (r o : Nat) :
    collectorInstance [r, o] = some o := rfl

inductive InstancePremise where
  | derived
  | stated
  | refuted
  deriving DecidableEq, Repr

def premisedVerdict : InstancePremise → Verdict → Verdict
  | .refuted, .discharged => .unresolved
  | _, v => v

theorem a_refuted_instance_is_never_discharged (v : Verdict) :
    premisedVerdict .refuted v ≠ .discharged := by
  cases v <;> simp [premisedVerdict]

theorem the_premises_never_add_a_discharge (p : InstancePremise) (v : Verdict)
    (h : premisedVerdict p v = .discharged) : v = .discharged := by
  cases p <;> cases v <;> simp_all [premisedVerdict]

theorem a_derived_premise_keeps_the_verdict (v : Verdict) :
    premisedVerdict .derived v = v := by
  cases v <;> rfl

theorem the_old_test_path_is_never_discharged_under_the_premises {owners : Owners}
    {contexts : List Context} {unresolved : Bool} {t : Thread} {list : List Thread}
    (p : InstancePremise)
    (hc : { thread := t, mode := .either } ∈ contexts)
    (hl : candidates owners .either = some list) (hn : t ∉ list) :
    premisedVerdict p (siteVerdict owners contexts unresolved) ≠ .discharged := by
  intro h
  exact the_old_test_path_is_never_discharged hc hl hn
    (the_premises_never_add_a_discharge p _ h)

/-! **P-INSTANCE-FIELDS-BY-MEMBER.** Every recorded write of an instance's
plugin or flag, or a collector's instance, is a member write (by an allowed
writer or not), a byte write, or a whole-object write. The flag is derived
only when every one is an allowed member write; a byte or whole-object write
keeps the route. -/

inductive FieldWrite where
  | member (allowed : Bool)
  | bytes
  | whole
  deriving DecidableEq, Repr

def fieldsByMember (writes : List FieldWrite) : Bool :=
  writes.all (fun w => w == .member true)

def flagByMember (writes : List FieldWrite) (flag : Option Bool) : Option Bool :=
  if fieldsByMember writes then flag else none

theorem allowed_member_writes_keep_the_derivation (flag : Option Bool) (n : Nat) :
    flagByMember (List.replicate n (.member true)) flag = flag := by
  have : fieldsByMember (List.replicate n (.member true)) = true := by
    simp [fieldsByMember]
  simp [flagByMember, this]

theorem a_non_member_write_keeps_the_route {α : Type} {registrants : List α} {r : α}
    {writes : List FieldWrite} {flag copy : Option Bool}
    (hw : FieldWrite.bytes ∈ writes ∨ FieldWrite.whole ∈ writes) (hr : r ∈ registrants) :
    r ∈ coroutineDispatched (fun _ => instanceFlag (flagByMember writes flag) copy)
      registrants := by
  have hf : fieldsByMember writes = false := by
    unfold fieldsByMember
    rcases hw with h | h
    · exact List.all_eq_false.mpr ⟨_, h, by decide⟩
    · exact List.all_eq_false.mpr ⟨_, h, by decide⟩
  have : flagByMember writes flag = none := by simp [flagByMember, hf]
  rw [this]
  exact an_underived_part_keeps_the_route (Or.inl rfl) hr

/-! ### The list-head region's macro invocations and the holder's opacity
(card W47-PREMISES-HOLDER-REGION)

**Region macros.** The lexical region sees a macro invocation only as a name
(`lexicalPath`); the program runs its expansion (`expandedPath`), under the
unit's own preprocessor table (`Expander`, from the unit's token dump). The
census reads every invocation from that table (`regionReading`): an
invocation the table cannot expand refuses, and so does one whose expansion
does anything but leave the list alone; otherwise the invocation adds
nothing to the lexical events. A read region then derives exactly what the
expansion runs (`the_region_reading_derives_what_the_expansion_runs`), so a
derived head is the one creation of the program's own path. The table is the
unit's: an invocation reads the definition the unit includes, whatever
another read file defines under the same name
(`the_included_definition_is_the_one_read`).

**The holder's opacity.** Each carrier of the holder's pointer (a read of the
instance's user pointer, a local, a parameter it is passed to) has uses; the
derivation admits comparison, reading through, passing on and, in the
holder's own file, writing a member other than the copy. An opaque holder
writes neither the flag nor the instance's plugin and none of the copy's
bytes, so the escape keeps the derivation
(`an_opaque_holder_keeps_the_derivation`); a carrier that writes the flag,
the copy or a holder byte elsewhere, or hands the pointer to code the reading
does not read, leaves it underived and keeps the route
(`a_carrier_that_writes_keeps_the_route`). The route is dropped only through
a read region whose expansion creates once from a non-coroutine descriptor
and an opaque holder
(`the_route_is_dropped_only_through_read_macros_and_an_opaque_holder`). -/

/-- One item of the region as the lexical reader sees it. -/
inductive RegionItem where
  /-- A call the lexical reading counts. -/
  | call (event : PathEvent)
  /-- A macro invocation: a name to the lexical reading. -/
  | invoke (name : String)
  deriving DecidableEq, Repr

/-- A unit's preprocessor table: each name's expansion as the events it
runs; `none` for a name the unit cannot expand (no facts, a failed unit, a
definition the census did not read). -/
abbrev Expander := String → Option (List PathEvent)

/-- The events the program runs: every invocation replaced by its expansion. -/
def expandedPath (ex : Expander) : List RegionItem → Option (List PathEvent)
  | [] => some []
  | .call e :: rest => (expandedPath ex rest).map (e :: ·)
  | .invoke n :: rest =>
    match ex n with
    | none => none
    | some es => (expandedPath ex rest).map (es ++ ·)

/-- The lexical reading, which drops invocations. -/
def lexicalPath : List RegionItem → List PathEvent
  | [] => []
  | .call e :: rest => e :: lexicalPath rest
  | .invoke _ :: rest => lexicalPath rest

/-- An expansion that leaves the list alone. -/
def quietExpansion (es : List PathEvent) : Bool := es.all (· == .other)

/-- The census's reading: an unexpandable or non-quiet invocation refuses. -/
def regionReading (ex : Expander) : List RegionItem → Option (List PathEvent)
  | [] => some []
  | .call e :: rest => (regionReading ex rest).map (e :: ·)
  | .invoke n :: rest =>
    match ex n with
    | none => none
    | some es => if quietExpansion es then regionReading ex rest else none

theorem an_unexpandable_invocation_refuses {ex : Expander} {n : String}
    (h : ex n = none) : ∀ items : List RegionItem, RegionItem.invoke n ∈ items →
      regionReading ex items = none := by
  intro items
  induction items with
  | nil => simp
  | cons item rest ih =>
    intro hmem
    rcases List.mem_cons.mp hmem with heq | hrest
    · subst heq; simp [regionReading, h]
    · cases item with
      | call e => simp [regionReading, ih hrest]
      | invoke m =>
        simp only [regionReading]
        split
        · rfl
        · split
          · exact ih hrest
          · rfl

theorem a_loud_expansion_refuses {ex : Expander} {n : String} {es : List PathEvent}
    (h : ex n = some es) (hloud : quietExpansion es = false) :
    ∀ items : List RegionItem, RegionItem.invoke n ∈ items →
      regionReading ex items = none := by
  intro items
  induction items with
  | nil => simp
  | cons item rest ih =>
    intro hmem
    rcases List.mem_cons.mp hmem with heq | hrest
    · subst heq; simp [regionReading, h, hloud]
    · cases item with
      | call e => simp [regionReading, ih hrest]
      | invoke m =>
        simp only [regionReading]
        split
        · rfl
        · split
          · exact ih hrest
          · rfl

theorem created_quiet {es : List PathEvent} (h : quietExpansion es = true) :
    created es = [] := by
  induction es with
  | nil => rfl
  | cons e rest ih =>
    simp only [quietExpansion, List.all_cons, Bool.and_eq_true, beq_iff_eq] at h
    obtain ⟨he, hr⟩ := h
    subst he
    exact ih hr

theorem mutates_quiet {es : List PathEvent} (h : quietExpansion es = true) :
    mutates es = false := by
  induction es with
  | nil => rfl
  | cons e rest ih =>
    simp only [quietExpansion, List.all_cons, Bool.and_eq_true, beq_iff_eq] at h
    obtain ⟨he, hr⟩ := h
    subst he
    simpa [mutates] using ih hr

theorem created_append (a b : List PathEvent) : created (a ++ b) = created a ++ created b := by
  induction a with
  | nil => rfl
  | cons e rest ih => cases e <;> simp [created, ih]

theorem mutates_append (a b : List PathEvent) :
    mutates (a ++ b) = (mutates a || mutates b) := by
  simp [mutates, List.any_append]

/-- A read region runs the events it read, plus quiet expansions only: the
same creations and the same mutations. -/
theorem the_region_reading_derives_what_the_expansion_runs {ex : Expander} :
    ∀ {items : List RegionItem} {evs : List PathEvent}, regionReading ex items = some evs →
      ∃ run, expandedPath ex items = some run ∧ created run = created evs ∧
        mutates run = mutates evs := by
  intro items
  induction items with
  | nil =>
    intro evs h
    simp [regionReading] at h
    subst h
    exact ⟨[], rfl, rfl, rfl⟩
  | cons item rest ih =>
    intro evs h
    cases item with
    | call e =>
      simp only [regionReading, Option.map_eq_some_iff] at h
      obtain ⟨evs', hr, rfl⟩ := h
      obtain ⟨run, hrun, hc, hm⟩ := ih hr
      refine ⟨e :: run, by simp [expandedPath, hrun], ?_, ?_⟩
      · cases e <;> simp [created, hc]
      · unfold mutates at hm ⊢
        simp only [List.any_cons, hm]
    | invoke n =>
      simp only [regionReading] at h
      cases hn : ex n with
      | none => simp [hn] at h
      | some es =>
        simp only [hn] at h
        split at h
        · rename_i hq
          obtain ⟨run, hrun, hc, hm⟩ := ih h
          refine ⟨es ++ run, by simp [expandedPath, hn, hrun], ?_, ?_⟩
          · rw [created_append, created_quiet hq, hc]; rfl
          · rw [mutates_append, mutates_quiet hq, hm]; rfl
        · simp at h

/-- The derivation the census makes on a read region. -/
def regionDerivation (ex : Expander) (items : List RegionItem) : Option Bool :=
  (regionReading ex items).bind headDerivation

/-- A derived region head is the one creation of the program's own path. -/
theorem a_derived_region_head_is_the_programs_one_creation {ex : Expander}
    {items : List RegionItem} {flag : Bool} (h : regionDerivation ex items = some flag) :
    ∃ run, expandedPath ex items = some run ∧ listAfter run = some [flag] := by
  unfold regionDerivation at h
  cases hr : regionReading ex items with
  | none => simp [hr] at h
  | some evs =>
    simp only [hr, Option.bind] at h
    obtain ⟨run, hrun, hc, hm⟩ := the_region_reading_derives_what_the_expansion_runs hr
    refine ⟨run, hrun, a_derived_head_is_the_one_creation ?_⟩
    unfold headDerivation at h ⊢
    rw [hc, hm]
    exact h

/-- Without invocations the reading is the lexical one. -/
theorem a_region_without_invocations_reads_lexically {ex : Expander} :
    ∀ {items : List RegionItem}, (∀ n, RegionItem.invoke n ∉ items) →
      regionReading ex items = some (lexicalPath items) := by
  intro items
  induction items with
  | nil => intro _; rfl
  | cons item rest ih =>
    intro h
    cases item with
    | call e =>
      have hrest : ∀ n, RegionItem.invoke n ∉ rest := fun n hn => h n (List.mem_cons_of_mem _ hn)
      simp [regionReading, lexicalPath, ih hrest]
    | invoke n => exact absurd (by simp : RegionItem.invoke n ∈ RegionItem.invoke n :: rest) (h n)

/-- A unit's table from the headers it includes, in order: the last
definition of a name wins (a redefinition follows an `#undef`). -/
def tableOf (included : List (List (String × List PathEvent))) : Expander := fun n =>
  ((included.flatten).reverse.find? (·.1 == n)).map (·.2)

/-- The definition the unit includes is the one read: an invocation of a
name the unit defines quietly is admitted, while a table that also held
another file's creating definition of the same name (as a name-keyed reading
across the read files would) refuses it. -/
theorem the_included_definition_is_the_one_read {n : String} {es : List PathEvent}
    (hq : quietExpansion es = true) :
    regionReading (tableOf [[(n, es)]]) [.invoke n] = some [] ∧
      regionReading (tableOf [[(n, es)], [(n, [PathEvent.create true])]]) [.invoke n] =
        none := by
  constructor
  · simp [regionReading, tableOf, hq]
  · simp [regionReading, tableOf, quietExpansion]

/-- A carrier's use of the holder's pointer. -/
inductive HolderUse where
  | compare
  | readThrough
  | passOn
  /-- A member written or its address taken; `copy` when it is the copy. -/
  | writeMember (copy : Bool)
  | writeFlag
  | writeInstance
  /-- Handed to code the reading does not read, returned or stored. -/
  | unread
  deriving DecidableEq, Repr

structure Carrier where
  /-- The carrier is in the holder's own file (the only one naming its type). -/
  inHolderFile : Bool
  uses : List HolderUse
  deriving DecidableEq, Repr

def useAdmitted (c : Carrier) : HolderUse → Bool
  | .compare | .readThrough | .passOn => true
  | .writeMember copy => c.inHolderFile && !copy
  | _ => false

def holderOpaque (cs : List Carrier) : Bool := cs.all fun c => c.uses.all (useAdmitted c)

/-- What the carriers can do to the route's facts. -/
def escapeOf (cs : List Carrier) : EscapeReach :=
  { writesFlag := cs.any (fun c => c.uses.any (· == .writeFlag)),
    writesInstance := cs.any (fun c => c.uses.any (· == .writeInstance)) }

/-- The derivation after the holder: underived unless the holder is opaque. -/
def holderDerivation (cs : List Carrier) (derived : Option Bool) : Option Bool :=
  if holderOpaque cs then afterEscape derived (escapeOf cs) else none

theorem an_opaque_holder_writes_nothing_the_route_reads {cs : List Carrier}
    (h : holderOpaque cs = true) : ∀ c ∈ cs, ∀ u ∈ c.uses,
      u ≠ .writeFlag ∧ u ≠ .writeInstance ∧ u ≠ .writeMember true ∧ u ≠ .unread ∧
        (∀ b, u = .writeMember b → c.inHolderFile = true) := by
  intro c hc u hu
  have hadm : useAdmitted c u = true :=
    List.all_eq_true.mp (List.all_eq_true.mp h c hc) u hu
  cases u with
  | compare => simp
  | readThrough => simp
  | passOn => simp
  | writeMember b =>
    simp only [useAdmitted, Bool.and_eq_true, Bool.not_eq_true'] at hadm
    refine ⟨by simp, by simp, ?_, by simp, ?_⟩
    · intro heq; cases heq; simp at hadm
    · intro b' _; exact hadm.1
  | writeFlag => simp [useAdmitted] at hadm
  | writeInstance => simp [useAdmitted] at hadm
  | unread => simp [useAdmitted] at hadm

theorem an_opaque_holder_keeps_the_derivation {cs : List Carrier} (derived : Option Bool)
    (h : holderOpaque cs = true) : holderDerivation cs derived = derived := by
  have hw := an_opaque_holder_writes_nothing_the_route_reads h
  have hf : (cs.any fun c => c.uses.any (· == .writeFlag)) = false := by
    rw [Bool.eq_false_iff]
    intro hany
    obtain ⟨c, hc, hu⟩ := List.any_eq_true.mp hany
    obtain ⟨u, hu, he⟩ := List.any_eq_true.mp hu
    exact (hw c hc u hu).1 (by simpa using he)
  have hi : (cs.any fun c => c.uses.any (· == .writeInstance)) = false := by
    rw [Bool.eq_false_iff]
    intro hany
    obtain ⟨c, hc, hu⟩ := List.any_eq_true.mp hany
    obtain ⟨u, hu, he⟩ := List.any_eq_true.mp hu
    exact (hw c hc u hu).2.1 (by simpa using he)
  simp [holderDerivation, h, escapeOf, afterEscape, hf, hi]

theorem a_carrier_that_writes_keeps_the_route {α : Type} {registrants : List α} {r : α}
    {cs : List Carrier} {c : Carrier} {u : HolderUse} {derived copy : Option Bool}
    (hc : c ∈ cs) (hu : u ∈ c.uses) (hbad : useAdmitted c u = false) (hr : r ∈ registrants) :
    r ∈ coroutineDispatched (fun _ => instanceFlag (holderDerivation cs derived) copy)
      registrants := by
  have hnot : holderOpaque cs = false := by
    rw [Bool.eq_false_iff]
    intro hall
    have := List.all_eq_true.mp (List.all_eq_true.mp hall c hc) u hu
    simp [hbad] at this
  have : holderDerivation cs derived = none := by simp [holderDerivation, hnot]
  rw [this]
  exact an_underived_part_keeps_the_route (Or.inl rfl) hr

/-- A `->data` reader that writes the flag keeps the route. -/
theorem a_data_reader_that_writes_the_flag_keeps_the_route {α : Type}
    {registrants : List α} {r : α} {cs : List Carrier} {c : Carrier}
    {derived copy : Option Bool} (hc : c ∈ cs) (hu : HolderUse.writeFlag ∈ c.uses)
    (hr : r ∈ registrants) :
    r ∈ coroutineDispatched (fun _ => instanceFlag (holderDerivation cs derived) copy)
      registrants :=
  a_carrier_that_writes_keeps_the_route hc hu rfl hr

/-- The route is dropped only through a read region whose program path
creates once from a non-coroutine descriptor, an opaque holder, and a copy
that runs none. -/
theorem the_route_is_dropped_only_through_read_macros_and_an_opaque_holder {α : Type}
    {registrants : List α} {r : α} {ex : Expander} {items : List RegionItem}
    {cs : List Carrier} {copy : Option Bool} (hr : r ∈ registrants)
    (hdropped : r ∉ coroutineDispatched
      (fun _ => instanceFlag (holderDerivation cs (regionDerivation ex items)) copy)
      registrants) :
    holderOpaque cs = true ∧ copy = some false ∧
      ∃ run, expandedPath ex items = some run ∧ listAfter run = some [false] := by
  obtain ⟨hcreating, hcopy⟩ := the_route_is_dropped_only_when_both_derive_none hr hdropped
  by_cases hop : holderOpaque cs = true
  · rw [an_opaque_holder_keeps_the_derivation _ hop] at hcreating
    exact ⟨hop, hcopy, a_derived_region_head_is_the_programs_one_creation hcreating⟩
  · simp [holderDerivation, hop] at hcreating

/-! #### The census graph's macro-made calls

The region's roots reach over the census graph; the program also runs the
calls macros make in the functions they reach. The census adds those calls
as edges and refuses a root that reaches a hazard only through one
(`closureCheck`); a root that passes reaches a hazard over the program's
graph only when the census graph already says so, so it was counted
(`a_passing_root_reaches_a_hazard_only_as_counted`). -/

/-- A hazard reached within `n` steps over `next`. -/
def reachesWithin {α : Type} (next : α → List α) (hazard : α → Bool) : Nat → α → Bool
  | 0, a => hazard a
  | n + 1, a => hazard a || (next a).any (reachesWithin next hazard n)

/-- The census's check: no root reaches a hazard over the census graph and
the macro-made calls unless it does over the census graph. -/
def closureCheck {α : Type} (census made : α → List α) (hazard : α → Bool) (n : Nat)
    (roots : List α) : Bool :=
  roots.all fun r =>
    reachesWithin census hazard n r || !reachesWithin (fun a => census a ++ made a) hazard n r

theorem a_passing_root_reaches_a_hazard_only_as_counted {α : Type}
    {census made : α → List α} {hazard : α → Bool} {n : Nat} {roots : List α} {r : α}
    (hcheck : closureCheck census made hazard n roots = true) (hr : r ∈ roots)
    (hprog : reachesWithin (fun a => census a ++ made a) hazard n r = true) :
    reachesWithin census hazard n r = true := by
  have h := List.all_eq_true.mp hcheck r hr
  simp only [Bool.or_eq_true, Bool.not_eq_true'] at h
  rcases h with h | h
  · exact h
  · rw [hprog] at h; exact absurd h (by simp)

theorem reachesWithin_more_edges {α : Type} {next more : α → List α} {hazard : α → Bool} :
    ∀ (n : Nat) (a : α), reachesWithin next hazard n a = true →
      reachesWithin (fun b => next b ++ more b) hazard n a = true := by
  intro n
  induction n with
  | zero => intro a h; simpa [reachesWithin] using h
  | succ n ih =>
    intro a h
    simp only [reachesWithin, Bool.or_eq_true, List.any_eq_true] at h ⊢
    rcases h with h | ⟨b, hb, hreach⟩
    · exact Or.inl h
    · exact Or.inr ⟨b, List.mem_append_left _ hb, ih b hreach⟩

/-- A root reaching a hazard only through a macro-made call fails the check. -/
theorem a_root_reaching_only_through_a_macro_call_refuses {α : Type}
    {census made : α → List α} {hazard : α → Bool} {n : Nat} {roots : List α} {r : α}
    (hr : r ∈ roots) (hcensus : reachesWithin census hazard n r = false)
    (hprog : reachesWithin (fun a => census a ++ made a) hazard n r = true) :
    closureCheck census made hazard n roots = false := by
  rw [Bool.eq_false_iff]
  intro hcheck
  have := a_passing_root_reaches_a_hazard_only_as_counted hcheck hr hprog
  rw [hcensus] at this
  exact absurd this (by simp)

/-! #### The holder's instance runs its copy's callback

An instance's plugin and user pointer are written by the constructor (from
its arguments, never the holder, which escapes only through the
installation) and by the installation, which writes both. So whenever the
user pointer is the holder, the plugin is the copy
(`the_holder_instance_runs_its_copy`): a dispatch `X->p->F(…, X->data)`
hands the holder only to the copy's `F`. -/

/-- A write of one instance's plugin and user pointer. -/
inductive InstanceWrite where
  /-- The constructor: a plugin and a user pointer from its arguments. -/
  | construct (p data : Nat)
  /-- The installation of the copy and the holder. -/
  | install
  deriving DecidableEq, Repr

structure InstanceState where
  p : Nat
  data : Nat
  deriving DecidableEq, Repr

def applyWrite (copy holder : Nat) (_s : InstanceState) : InstanceWrite → InstanceState
  | .construct p data => { p := p, data := data }
  | .install => { p := copy, data := holder }

/-- The pairing the dispatch rests on. -/
def holderPaired (copy holder : Nat) (s : InstanceState) : Prop :=
  s.data = holder → s.p = copy

theorem the_holder_instance_runs_its_copy {copy holder : Nat} :
    ∀ (writes : List InstanceWrite) (s : InstanceState),
      (∀ p d, InstanceWrite.construct p d ∈ writes → d ≠ holder) →
      holderPaired copy holder s →
      holderPaired copy holder (writes.foldl (applyWrite copy holder) s) := by
  intro writes
  induction writes with
  | nil => intro s _ h; exact h
  | cons w rest ih =>
    intro s hctor h
    simp only [List.foldl_cons]
    apply ih
    · intro p d hm; exact hctor p d (List.mem_cons_of_mem _ hm)
    · cases w with
      | construct p d =>
        intro hd
        exact absurd hd (hctor p d List.mem_cons_self)
      | install => intro _; rfl

/-- The old test's collector operations run on the test thread whatever the
region's macros or the holder derive. -/
theorem the_old_test_path_is_never_discharged_whatever_the_region {owners : Owners}
    {contexts : List Context} {unresolved : Bool} {t : Thread} {list : List Thread}
    (hc : { thread := t, mode := .either } ∈ contexts)
    (hl : candidates owners .either = some list) (hn : t ∉ list) :
    siteVerdict owners contexts unresolved ≠ .discharged :=
  the_old_test_path_is_never_discharged hc hl hn

end QCore.LoopOwner
