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

import QCore.Concurrency

namespace QCore.CondvarJoin

/-! Card **W47-CONDVAR-JOIN-PUBLICATION** (gateway rule 10q, contract
specification §C04.10, the narrowed amendment of constraint 27): condition
variable publication (C4) and join before release (C5).

Companion to `crates/qcore-core/src/condvar_join.rs`,
`crates/qcore-c-gateway/src/condvar_census.rs` and
`crates/quirin-source-analysis/src/condvar_join_census.rs`.

* **The condvar edge.** A wait releases and reacquires its mutex, so the
  waiter's critical section is split at every wait and each wakeup's recheck
  starts a new section. The mutex totally orders sections; a schedule is a
  list of sections. When the publisher's section writes every result and then
  the predicate, and no other section writes either, any prefix of the
  schedule whose memory shows the predicate set shows exactly the published
  snapshot (`observed_done_reads_the_published_snapshot`); before the
  publication the predicate is unset (`no_claim_before_publication`).
* **The waiter.** A wakeup without the predicate waits again, whatever woke
  it; a timeout that leaves the loop claims nothing; only an observed
  predicate claims (`a_claim_needs_the_predicate`,
  `a_wakeup_without_the_predicate_claims_nothing`, `a_run_claims_only_at_an_observed_predicate`).
* **Join before release.** A thread acts only while live; only a checked,
  successful join ends it. If every live thread that retains an object was so
  joined when the object is released, no thread started before the release
  that retains it accesses it afterwards
  (`join_before_release_leaves_no_access_after_it`). A timeout, a publication,
  an unchecked or a failed join leave the live set unchanged
  (`a_timeout_is_not_a_join`, `publication_is_not_a_join`,
  `an_unchecked_or_failed_join_grants_nothing`).
* **Decisions.** The kernel's verdicts: only a discharge admits; every kept
  refusal refuses; a discharged wait is in a predicate loop under its mutex;
  a discharged signal holds the paired mutex; a discharged release saw a
  checked join on every path; the per-unit route never discharges. -/

/-! ## Verdicts -/

inductive Verdict where
  | discharged
  | unresolved
  | notDischarged
  | refused
  deriving DecidableEq, Repr

def Verdict.rank : Verdict → Nat
  | .discharged => 0
  | .unresolved => 1
  | .notDischarged => 2
  | .refused => 3

/-- Composition: the more severe verdict. -/
def Verdict.join (a b : Verdict) : Verdict := if b.rank > a.rank then b else a

def Verdict.admits : Verdict → Bool
  | .discharged => true
  | _ => false

theorem only_a_discharge_admits {v : Verdict} (h : v.admits = true) : v = .discharged := by
  cases v <;> simp_all [Verdict.admits]

theorem a_join_is_no_less_severe (a b : Verdict) :
    a.rank ≤ (a.join b).rank ∧ b.rank ≤ (a.join b).rank := by
  unfold Verdict.join
  by_cases h : b.rank > a.rank <;> simp [h] <;> omega

theorem a_joined_discharge_needs_both (a b : Verdict) (h : (a.join b) = .discharged) :
    a = .discharged ∧ b = .discharged := by
  cases a <;> cases b <;> simp_all [Verdict.join, Verdict.rank]

/-! ## Kept refusals (constraint 27 as amended) -/

inductive Refusal where
  | broadcast
  | waitOutsidePredicateLoop
  | signalWithoutMutex
  | condvarWithTwoMutexes
  | staticInitializer
  | c11Condvar
  | rwlock
  | tryOrTimedLock
  | recursiveOrDynamicMutex
  deriving DecidableEq, Repr

/-- A shape the lift does not name is a kept refusal: its verdict. -/
def refusalVerdict (_ : Refusal) : Verdict := .refused

theorem every_kept_refusal_refuses (r : Refusal) : (refusalVerdict r).admits = false := by
  simp [refusalVerdict, Verdict.admits]

inductive Held where
  | yes
  | no
  | unknown
  deriving DecidableEq, Repr

inductive LoopBody where
  | waitOnly
  | waitThenBreakOnNonzero
  | other
  deriving DecidableEq, Repr

/-- One wait, as the census reads it. `inLoop` is false for a wait outside a
predicate loop; `predicate` says the loop condition reads a field. -/
structure WaitSite where
  twoMutexes : Bool
  inLoop : Bool
  predicate : Bool
  held : Held
  underMutex : Bool
  body : LoopBody
  timed : Bool
  deriving DecidableEq, Repr

/-- `wait_verdict`. -/
def waitVerdict (w : WaitSite) : Verdict :=
  if w.twoMutexes then .refused
  else if !w.inLoop || !w.predicate then .refused
  else match w.held with
    | .no => .notDischarged
    | .unknown => .unresolved
    | .yes =>
      if !w.underMutex then .notDischarged
      else match w.body with
        | .waitOnly => .discharged
        | .waitThenBreakOnNonzero => if w.timed then .discharged else .unresolved
        | .other => .unresolved

theorem a_wait_outside_a_predicate_loop_refuses (w : WaitSite) (h : w.inLoop = false) :
    waitVerdict w = .refused := by
  unfold waitVerdict; cases w.twoMutexes <;> simp [h]

theorem one_condvar_with_two_mutexes_refuses_a_wait (w : WaitSite) (h : w.twoMutexes = true) :
    waitVerdict w = .refused := by
  simp [waitVerdict, h]

theorem a_discharged_wait_rechecks_under_its_mutex (w : WaitSite)
    (h : waitVerdict w = .discharged) :
    w.twoMutexes = false ∧ w.inLoop = true ∧ w.predicate = true ∧ w.held = .yes ∧
      w.underMutex = true := by
  unfold waitVerdict at h
  cases h1 : w.twoMutexes <;> cases h2 : w.inLoop <;> cases h3 : w.predicate <;>
    cases h4 : w.held <;> cases h5 : w.underMutex <;> cases h6 : w.body <;>
    cases h7 : w.timed <;> simp_all

/-- One signal: whether it is a broadcast, the condvar's pairing, and whether
the paired mutex is held (or not decided). -/
structure SignalSite where
  broadcast : Bool
  twoMutexes : Bool
  paired : Bool
  heldPaired : Bool
  heldUnknown : Bool
  deriving DecidableEq, Repr

/-- `signal_verdict`. -/
def signalVerdict (s : SignalSite) : Verdict :=
  if s.broadcast then .refused
  else if s.twoMutexes then .refused
  else if !s.paired then .unresolved
  else if s.heldPaired then .discharged
  else if s.heldUnknown then .unresolved
  else .refused

theorem a_broadcast_refuses (s : SignalSite) (h : s.broadcast = true) :
    signalVerdict s = .refused := by
  simp [signalVerdict, h]

theorem a_signal_without_the_mutex_refuses (s : SignalSite) (hb : s.broadcast = false)
    (ht : s.twoMutexes = false) (hp : s.paired = true) (hh : s.heldPaired = false)
    (hu : s.heldUnknown = false) : signalVerdict s = .refused := by
  simp [signalVerdict, hb, ht, hp, hh, hu]

theorem a_discharged_signal_holds_the_paired_mutex (s : SignalSite)
    (h : signalVerdict s = .discharged) : s.broadcast = false ∧ s.heldPaired = true := by
  unfold signalVerdict at h
  cases h1 : s.broadcast <;> cases h2 : s.twoMutexes <;> cases h3 : s.paired <;>
    cases h4 : s.heldPaired <;> cases h5 : s.heldUnknown <;> simp_all

/-- The per-unit resource route derives no thread, predicate loop or join. -/
def perUnitRouteVerdict : Verdict := .unresolved

theorem the_per_unit_route_refuses_every_synchronization_call :
    perUnitRouteVerdict.admits = false := by
  simp [perUnitRouteVerdict, Verdict.admits]

/-! ## The waiter (NC8) -/

inductive Wakeup where
  | signalled
  | spurious
  | timedOut
  deriving DecidableEq, Repr

inductive Step where
  | rewait
  | claim
  | exitWithoutClaim
  deriving DecidableEq, Repr

/-- `waiter_step`: the predicate is rechecked under the reacquired mutex after
every wakeup. -/
def waiterStep (predicate : Bool) (w : Wakeup) (breakOnTimeout : Bool) : Step :=
  if predicate then .claim
  else if w = .timedOut ∧ breakOnTimeout = true then .exitWithoutClaim
  else .rewait

theorem a_claim_needs_the_predicate {p : Bool} {w : Wakeup} {b : Bool}
    (h : waiterStep p w b = .claim) : p = true := by
  unfold waiterStep at h
  cases p <;> simp_all
  split at h <;> simp_all

theorem a_wakeup_without_the_predicate_claims_nothing (w : Wakeup) (b : Bool) :
    waiterStep false w b ≠ .claim := by
  intro h
  exact absurd (a_claim_needs_the_predicate h) (by simp)

theorem a_spurious_wakeup_waits_again (b : Bool) :
    waiterStep false .spurious b = .rewait := by
  simp [waiterStep]

theorem a_timeout_that_leaves_claims_nothing :
    waiterStep false .timedOut true = .exitWithoutClaim := by
  simp [waiterStep]

/-- `run_waiter`: `some true` claims, `some false` left without a claim. -/
def runWaiter : List (Bool × Wakeup) → Bool → Option Bool
  | [], _ => none
  | (p, w) :: rest, b =>
    match waiterStep p w b with
    | .claim => some true
    | .exitWithoutClaim => some false
    | .rewait => runWaiter rest b

theorem a_run_claims_only_at_an_observed_predicate :
    ∀ (xs : List (Bool × Wakeup)) (b : Bool), runWaiter xs b = some true →
      ∃ w, (true, w) ∈ xs
  | [], _, h => by simp [runWaiter] at h
  | (p, w) :: rest, b, h => by
    unfold runWaiter at h
    cases hs : waiterStep p w b with
    | claim =>
      have := a_claim_needs_the_predicate hs
      subst this
      exact ⟨w, by simp⟩
    | exitWithoutClaim => simp [hs] at h
    | rewait =>
      simp [hs] at h
      obtain ⟨w', hw'⟩ := a_run_claims_only_at_an_observed_predicate rest b h
      exact ⟨w', by simp [hw']⟩

/-! ## Publication through mutex sections (the condvar edge) -/

abbrev Field := Nat
abbrev Memory := Field → Nat

inductive Op where
  | write (f : Field) (v : Nat)
  | read (f : Field)
  deriving DecidableEq, Repr

def Op.writes : Op → Field → Bool
  | .write g _, f => g == f
  | .read _, _ => false

def apply (m : Memory) : Op → Memory
  | .write f v => fun g => if g = f then v else m g
  | .read _ => m

/-- One critical section, run atomically under the mutex. -/
def runSection (m : Memory) (s : List Op) : Memory := s.foldl apply m

/-- A schedule: sections in the order the mutex serialises them. -/
def run (m : Memory) (sched : List (List Op)) : Memory := sched.foldl runSection m

theorem apply_other {m : Memory} {op : Op} {f : Field} (h : op.writes f = false) :
    apply m op f = m f := by
  cases op with
  | write g v =>
    simp [Op.writes] at h
    simp [apply]
    intro hfg
    exact absurd hfg.symm h
  | read g => simp [apply]

theorem runSection_other :
    ∀ (s : List Op) (m : Memory) (f : Field), (∀ op ∈ s, op.writes f = false) →
      runSection m s f = m f
  | [], _, _, _ => rfl
  | op :: rest, m, f, h => by
    simp only [runSection, List.foldl_cons]
    have hop : op.writes f = false := h op (by simp)
    have := runSection_other rest (apply m op) f (fun o ho => h o (by simp [ho]))
    simp only [runSection] at this
    rw [this, apply_other hop]

theorem run_other :
    ∀ (sched : List (List Op)) (m : Memory) (f : Field),
      (∀ s ∈ sched, ∀ op ∈ s, op.writes f = false) → run m sched f = m f
  | [], _, _, _ => rfl
  | s :: rest, m, f, h => by
    simp only [run, List.foldl_cons]
    have := run_other rest (runSection m s) f (fun t ht => h t (by simp [ht]))
    simp only [run] at this
    rw [this, runSection_other s m f (h s (by simp))]

theorem run_append (m : Memory) (a b : List (List Op)) : run m (a ++ b) = run (run m a) b := by
  simp [run, List.foldl_append]

/-- The publisher's section: every result field gets its snapshot value, then
the predicate is set. -/
def publication (results : List Field) (snap : Field → Nat) (done : Field) : List Op :=
  results.map (fun f => Op.write f (snap f)) ++ [Op.write done 1]

theorem writes_set :
    ∀ (fs : List Field) (m : Memory) (snap : Field → Nat) (f : Field), f ∈ fs →
      runSection m (fs.map (fun g => Op.write g (snap g))) f = snap f
  | [], _, _, _, h => by simp at h
  | g :: rest, m, snap, f, h => by
    simp only [List.map_cons, runSection, List.foldl_cons]
    by_cases hin : f ∈ rest
    · have := writes_set rest (apply m (Op.write g (snap g))) snap f hin
      simpa [runSection] using this
    · have hfg : f = g := by simpa [hin] using h
      subst hfg
      have hother : ∀ op ∈ rest.map (fun g => Op.write g (snap g)), op.writes f = false := by
        intro op hop
        simp only [List.mem_map] at hop
        obtain ⟨g', hg', rfl⟩ := hop
        simp [Op.writes]
        intro heq
        exact hin (heq ▸ hg')
      have := runSection_other _ (apply m (Op.write f (snap f))) f hother
      simp only [runSection] at this
      rw [this]
      simp [apply]

theorem publication_sets (results : List Field) (snap : Field → Nat) (done : Field)
    (hd : done ∉ results) (m : Memory) :
    runSection m (publication results snap done) done = 1 ∧
      ∀ f ∈ results, runSection m (publication results snap done) f = snap f := by
  unfold publication runSection
  rw [List.foldl_append]
  constructor
  · simp [apply]
  · intro f hf
    have hne : f ≠ done := fun h => hd (h ▸ hf)
    simp only [List.foldl_cons, List.foldl_nil, apply, hne, ↓reduceIte]
    exact writes_set results m snap f hf

/-- No section other than the publication writes the predicate or a result. -/
def Quiet (results : List Field) (done : Field) (sched : List (List Op)) : Prop :=
  ∀ s ∈ sched, ∀ op ∈ s, op.writes done = false ∧ ∀ f ∈ results, op.writes f = false

/-- Before the publication the predicate is unset: a waiter rechecking it
under the mutex makes no result claim. -/
theorem no_claim_before_publication (results : List Field) (done : Field) (m0 : Memory)
    (h0 : m0 done = 0) (pre : List (List Op)) (hq : Quiet results done pre) (k : Nat) :
    run m0 (pre.take k) done = 0 := by
  rw [run_other]
  · exact h0
  · intro s hs op hop
    exact (hq s (List.mem_of_mem_take hs) op hop).1

/-- After the publication every recheck that sees the predicate reads the
published snapshot. -/
theorem after_publication_the_snapshot_holds (results : List Field) (snap : Field → Nat)
    (done : Field) (hd : done ∉ results) (m0 : Memory) (pre post : List (List Op))
    (hq : Quiet results done post) (k : Nat) :
    run m0 (pre ++ publication results snap done :: post.take k) done = 1 ∧
      ∀ f ∈ results, run m0 (pre ++ publication results snap done :: post.take k) f = snap f := by
  have split : pre ++ publication results snap done :: post.take k =
      (pre ++ [publication results snap done]) ++ post.take k := by simp
  rw [split, run_append, run_append]
  have hpub := publication_sets results snap done hd (run m0 pre)
  simp only [run, List.foldl_cons, List.foldl_nil] at hpub ⊢
  constructor
  · have := run_other (post.take k) (runSection (run m0 pre) (publication results snap done)) done
      (fun s hs op hop => (hq s (List.mem_of_mem_take hs) op hop).1)
    simp only [run] at this
    rw [this]
    exact hpub.1
  · intro f hf
    have := run_other (post.take k) (runSection (run m0 pre) (publication results snap done)) f
      (fun s hs op hop => (hq s (List.mem_of_mem_take hs) op hop).2 f hf)
    simp only [run] at this
    rw [this]
    exact hpub.2 f hf

/-- **C4.** In a schedule `pre ++ W :: post` whose only writer of the predicate
and the results is the publication `W`, every prefix in which the predicate is
observed set reads exactly the published snapshot. -/
theorem observed_done_reads_the_published_snapshot (results : List Field)
    (snap : Field → Nat) (done : Field) (hd : done ∉ results) (m0 : Memory)
    (h0 : m0 done = 0) (pre post : List (List Op)) (hpre : Quiet results done pre)
    (hpost : Quiet results done post) (n : Nat)
    (seen : run m0 ((pre ++ publication results snap done :: post).take n) done = 1) :
    ∀ f ∈ results, run m0 ((pre ++ publication results snap done :: post).take n) f = snap f := by
  rw [List.take_append] at seen ⊢
  by_cases hn : n ≤ pre.length
  · have hz : n - pre.length = 0 := by omega
    rw [hz] at seen
    simp only [List.take_zero, List.append_nil] at seen
    rw [no_claim_before_publication results done m0 h0 pre hpre n] at seen
    simp at seen
  · have hpre_all : pre.take n = pre := List.take_of_length_le (by omega)
    obtain ⟨k, hk⟩ : ∃ k, n - pre.length = k + 1 := ⟨n - pre.length - 1, by omega⟩
    rw [hpre_all, hk, List.take_succ_cons]
    exact (after_publication_the_snapshot_holds results snap done hd m0 pre post hpost k).2

/-! ## Join before release (C5) -/

abbrev Thread := Nat
abbrev Obj := Nat

structure ThreadState where
  started : List Thread
  live : List Thread
  released : List Obj
  deriving DecidableEq, Repr

inductive Event where
  | spawn (t : Thread)
  | access (t : Thread) (o : Obj)
  /-- A join whose zero result the path checked. -/
  | joinChecked (t : Thread)
  /-- A join whose result is unchecked, or known to have failed. -/
  | joinUnchecked (t : Thread)
  | joinFailed (t : Thread)
  | timeout
  | publish (t : Thread)
  | release (o : Obj)
  deriving DecidableEq, Repr

/-- The release rule: every live thread that retains the object has ended. -/
def ReleaseOk (retains : Thread → Obj → Bool) (s : ThreadState) (o : Obj) : Prop :=
  ∀ t ∈ s.live, retains t o = false

/-- Execution: a thread acts only while live; a spawned thread is fresh; only
a checked join ends a thread; a release must satisfy the rule. -/
def step (retains : Thread → Obj → Bool) (s : ThreadState) : Event → Option ThreadState
  | .spawn t => if t ∈ s.started then none
      else some { s with started := t :: s.started, live := t :: s.live }
  | .access t o => if t ∈ s.live ∧ o ∉ s.released then some s else none
  | .joinChecked t => some { s with live := s.live.filter (· ≠ t) }
  | .joinUnchecked _ | .joinFailed _ | .timeout | .publish _ => some s
  | .release o => if (s.live.all fun t => !retains t o) then
      some { s with released := o :: s.released } else none

def exec (retains : Thread → Obj → Bool) : ThreadState → List Event → Option ThreadState
  | s, [] => some s
  | s, e :: rest => (step retains s e).bind (fun s' => exec retains s' rest)

theorem a_timeout_is_not_a_join (retains : Thread → Obj → Bool) (s : ThreadState) :
    step retains s .timeout = some s := rfl

theorem publication_is_not_a_join (retains : Thread → Obj → Bool) (s : ThreadState)
    (t : Thread) : step retains s (.publish t) = some s := rfl

theorem an_unchecked_or_failed_join_grants_nothing (retains : Thread → Obj → Bool)
    (s : ThreadState) (t : Thread) :
    step retains s (.joinUnchecked t) = some s ∧ step retains s (.joinFailed t) = some s :=
  ⟨rfl, rfl⟩

/-- A release is accepted exactly under the rule. -/
theorem an_accepted_release_joined_every_retainer (retains : Thread → Obj → Bool)
    (s s' : ThreadState) (o : Obj) (h : step retains s (.release o) = some s') :
    ReleaseOk retains s o := by
  simp only [step] at h
  split at h
  · rename_i hall
    intro t ht
    have := List.all_eq_true.mp hall t ht
    simpa using this
  · simp at h

/-- While a started thread is not live, no step makes it live again. -/
theorem ended_stays_ended (retains : Thread → Obj → Bool) (t : Thread) :
    ∀ (evs : List Event) (s s' : ThreadState), t ∈ s.started → t ∉ s.live →
      exec retains s evs = some s' → t ∈ s'.started ∧ t ∉ s'.live
  | [], s, s', hs, hl, h => by
    simp [exec] at h
    subst h
    exact ⟨hs, hl⟩
  | e :: rest, s, s', hs, hl, h => by
    simp only [exec] at h
    cases hstep : step retains s e with
    | none => simp [hstep] at h
    | some s1 =>
      simp [hstep] at h
      have hs1 : t ∈ s1.started ∧ t ∉ s1.live := by
        cases e with
        | spawn u =>
          simp only [step] at hstep
          split at hstep
          · simp at hstep
          · rename_i hu
            simp only [Option.some.injEq] at hstep
            subst hstep
            refine ⟨by simp [hs], ?_⟩
            simp only [List.mem_cons, not_or]
            exact ⟨fun htu => hu (htu ▸ hs), hl⟩
        | access u o =>
          simp only [step] at hstep
          split at hstep
          · simp only [Option.some.injEq] at hstep; subst hstep; exact ⟨hs, hl⟩
          · simp at hstep
        | joinChecked u =>
          simp only [step, Option.some.injEq] at hstep
          subst hstep
          refine ⟨hs, ?_⟩
          simp [List.mem_filter, hl]
        | joinUnchecked u =>
          simp only [step, Option.some.injEq] at hstep; subst hstep; exact ⟨hs, hl⟩
        | joinFailed u =>
          simp only [step, Option.some.injEq] at hstep; subst hstep; exact ⟨hs, hl⟩
        | timeout =>
          simp only [step, Option.some.injEq] at hstep; subst hstep; exact ⟨hs, hl⟩
        | publish u =>
          simp only [step, Option.some.injEq] at hstep; subst hstep; exact ⟨hs, hl⟩
        | release o =>
          simp only [step] at hstep
          split at hstep
          · simp only [Option.some.injEq] at hstep; subst hstep; exact ⟨hs, hl⟩
          · simp at hstep
      exact ended_stays_ended retains t rest s1 s' hs1.1 hs1.2 h

/-- An ended thread (started, not live) never acts again. -/
theorem an_ended_thread_never_acts (retains : Thread → Obj → Bool) (t : Thread) (o : Obj) :
    ∀ (post : List Event) (s s' : ThreadState), t ∈ s.started → t ∉ s.live →
      exec retains s post = some s' → Event.access t o ∉ post
  | [], _, _, _, _, _ => by simp
  | e :: rest, s, s', hs, hl, h => by
    simp only [exec] at h
    cases hstep : step retains s e with
    | none => simp [hstep] at h
    | some s1 =>
      simp [hstep] at h
      intro hmem
      rcases List.mem_cons.mp hmem with heq | hrest
      · subst heq
        simp only [step] at hstep
        split at hstep
        · rename_i hcond
          exact hl hcond.1
        · simp at hstep
      · have hs1 := ended_stays_ended retains t [e] s s1 hs hl (by simp [exec, hstep])
        exact an_ended_thread_never_acts retains t o rest s1 s' hs1.1 hs1.2 h hrest

/-- **C5.** If every live thread that retains `o` has ended (a checked join)
when `o` is released, no thread started before the release that retains `o`
accesses it afterwards. -/
theorem join_before_release_leaves_no_access_after_it (retains : Thread → Obj → Bool)
    (s : ThreadState) (o : Obj) (hok : ReleaseOk retains s o) (t : Thread)
    (hstarted : t ∈ s.started) (hret : retains t o = true) (post : List Event)
    (s' : ThreadState) (h : exec retains s post = some s') : Event.access t o ∉ post := by
  have hnot : t ∉ s.live := fun hl => by
    have := hok t hl
    simp [hret] at this
  exact an_ended_thread_never_acts retains t o post s s' hstarted hnot h

/-! ## The release decision -/

inductive Evidence where
  | checkedSuccess
  | unchecked
  | failed
  | absent
  deriving DecidableEq, Repr

/-- `release_verdict`: every path of every live thread needs a checked join;
otherwise refused, and unresolved when a premise is open. -/
def releaseVerdict (paths : List Evidence) (openPremise : Bool) : Verdict :=
  if paths.all (· == .checkedSuccess) then
    (if openPremise then .unresolved else .discharged)
  else .notDischarged

theorem a_discharged_release_saw_a_checked_join_on_every_path (paths : List Evidence)
    (o : Bool) (h : releaseVerdict paths o = .discharged) :
    (∀ e ∈ paths, e = .checkedSuccess) ∧ o = false := by
  unfold releaseVerdict at h
  split at h
  · rename_i hall
    cases o
    · refine ⟨fun e he => ?_, rfl⟩
      have := List.all_eq_true.mp hall e he
      simpa using this
    · simp at h
  · simp at h

theorem publication_timeout_or_an_unchecked_join_refuses (paths : List Evidence) (o : Bool)
    (e : Evidence) (he : e ∈ paths) (hne : e ≠ .checkedSuccess) :
    releaseVerdict paths o = .notDischarged := by
  unfold releaseVerdict
  have : paths.all (· == .checkedSuccess) = false := by
    rw [List.all_eq_false]
    exact ⟨e, he, by simpa using hne⟩
  simp [this]

/-! ## The condvar edge on the Cycle 14 lifetime model (`QCore.Concurrency`)

A wait is the waiter's unlock and relock of its mutex. Lowered this way, the
publish/await/join shape is an accepted `QCore.Concurrency.Lifetime` trace, and
the release before the join is refused there too: the Rust checker's vector
clocks supply `priorAccessesOrdered = false` for it (the publisher's accesses
are not ordered before the waiter's release), which the projection refuses. -/

section Lifetime
open QCore.Concurrency.Lifetime

/-- A wait on task `t`'s mutex: the unlock that starts it and the relock at
its wakeup. Another task's critical section runs between the two. -/
def waitEdge (t : Nat) (between : List QCore.Concurrency.Lifetime.Event) :
    List QCore.Concurrency.Lifetime.Event :=
  [QCore.Concurrency.Lifetime.Event.unlock t] ++ between ++
    [QCore.Concurrency.Lifetime.Event.lock t]

def lifetimeExec (s : QCore.Concurrency.Lifetime.State) :
    List QCore.Concurrency.Lifetime.Event → Option QCore.Concurrency.Lifetime.State
  | [] => some s
  | e :: rest => (QCore.Concurrency.Lifetime.step s e).bind (fun s' => lifetimeExec s' rest)

/-- The waiter (task 0) owns the storage (resource 7) and starts the publisher
(task 1). -/
def lifetimeStart : QCore.Concurrency.Lifetime.State := { custody := [(7, 0)] }

open QCore.Concurrency.Lifetime.Event in
/-- The publisher's critical section. -/
def publisherSection : List QCore.Concurrency.Lifetime.Event :=
  [lock 1, access 1 7, unlock 1]

open QCore.Concurrency.Lifetime.Event in
/-- Publish under the mutex, await through the condvar edge, join, release. -/
def publishAwaitJoin : List QCore.Concurrency.Lifetime.Event :=
  [spawn 0 1, lock 0] ++ waitEdge 0 publisherSection ++
    [access 0 7, unlock 0, join 0 1, destroy 0 7 true]

open QCore.Concurrency.Lifetime.Event in
/-- The same shape releasing before the join: the publisher's accesses are not
ordered before the release, so the vector clocks supply `false`. -/
def releaseBeforeJoin : List QCore.Concurrency.Lifetime.Event :=
  [spawn 0 1, lock 0] ++ waitEdge 0 publisherSection ++
    [access 0 7, unlock 0, destroy 0 7 false, join 0 1]

theorem the_condvar_edge_is_an_accepted_lifetime_trace :
    (lifetimeExec lifetimeStart publishAwaitJoin).isSome = true := by
  decide

theorem a_release_before_the_join_is_refused_by_the_lifetime_model :
    lifetimeExec lifetimeStart releaseBeforeJoin = none := by
  decide

end Lifetime

/-! ## Stop paths (card W47-CONDVAR-JOIN-STOP-PATHS)

Companion to `null_branch_excluded`, `join_succeeds`, `JoinEvidence::{UncheckedFirst,
Repeated}`, `DescendantJoin` and `release_verdict` in
`crates/qcore-core/src/condvar_join.rs`.

* **An excluded null arm.** After a dereference of `X->F`, an analysed execution
  (named premise P-DEREF: a null dereference ends it) has `X->F` non-null. A
  pre-publication store writes an object nobody else can reach yet, so it never
  writes a reachable one; if every store in the window is one, the field is still
  non-null at the test (`the_null_arm_is_infeasible_after_a_dereference`). A store
  to a reachable object can null it (`a_published_store_can_null_the_field`), and
  the decision drops the arm only with every part of its derivation
  (`a_dropped_null_arm_had_its_derivation`).
* **An unchecked first join.** Under the named premise P-POSIX-JOIN (the model
  `posixSucceeds`), a join returns 0 unless the target is not joinable, already
  joined, awaited by another joiner, the caller, or in a join cycle with it. Each
  derived exclusion certifies the absence of its case; with all four and a first
  join on the path the join succeeds
  (`an_unchecked_first_join_with_every_case_excluded_succeeds`). A detached
  target, a repeated join and a join from the thread itself never count.
* **Descendants.** A live thread that retains the object blocks its release,
  whoever started it (`a_live_descendant_blocks_the_release`); the decision needs
  every descendant derived joined (`a_discharged_release_joined_every_descendant`).
* **The two tests' shapes.** The old test's release paths and the fixed test's
  (a checked join, a first join whose busy case is not excluded, a repeated join)
  are both not discharged. -/

section StopPaths

/-! ### The null arm -/

structure Heap where
  field : Nat → Option Nat
  reachable : List Nat

inductive Store where
  | store (o : Nat) (v : Option Nat)

def applyStore (h : Heap) : Store → Heap
  | .store o v => { h with field := fun p => if p = o then v else h.field p }

def runStores (h : Heap) (stores : List Store) : Heap := stores.foldl applyStore h

/-- A pre-publication store writes an object no one else can reach yet. -/
def PrePublication (h : Heap) : Store → Prop
  | .store o _ => o ∉ h.reachable

theorem applyStore_reachable (h : Heap) (s : Store) : (applyStore h s).reachable = h.reachable := by
  cases s; rfl

theorem a_pre_publication_store_leaves_a_reachable_field (h : Heap) (s : Store) (x : Nat)
    (hx : x ∈ h.reachable) (hs : PrePublication h s) : (applyStore h s).field x = h.field x := by
  cases s with
  | store o v =>
    simp only [PrePublication] at hs
    have hne : x ≠ o := fun heq => hs (heq ▸ hx)
    simp [applyStore, hne]

theorem pre_publication_stores_leave_a_reachable_field :
    ∀ (stores : List Store) (h : Heap) (x : Nat), x ∈ h.reachable →
      (∀ s ∈ stores, ∀ h' : Heap, h'.reachable = h.reachable → PrePublication h' s) →
      (runStores h stores).field x = h.field x ∧ (runStores h stores).reachable = h.reachable
  | [], h, _, _, _ => by simp [runStores]
  | s :: rest, h, x, hx, hall => by
    have hs : PrePublication h s := hall s (by simp) h rfl
    have h1 := a_pre_publication_store_leaves_a_reachable_field h s x hx hs
    have hr := applyStore_reachable h s
    have ih := pre_publication_stores_leave_a_reachable_field rest (applyStore h s) x
      (by rw [hr]; exact hx)
      (fun s' hs' h' hh' => hall s' (by simp [hs']) h' (by rw [hh', hr]))
    simp only [runStores, List.foldl_cons] at ih ⊢
    exact ⟨by rw [ih.1, h1], by rw [ih.2, hr]⟩

/-- **P-DEREF.** A dereference of `x`'s field in an analysed execution saw it
non-null; if every store until the test is a pre-publication store, the test
sees it non-null: the null arm is infeasible. -/
theorem the_null_arm_is_infeasible_after_a_dereference (h : Heap) (x : Nat)
    (hx : x ∈ h.reachable) (hderef : h.field x ≠ none) (stores : List Store)
    (hall : ∀ s ∈ stores, ∀ h' : Heap, h'.reachable = h.reachable → PrePublication h' s) :
    (runStores h stores).field x ≠ none := by
  rw [(pre_publication_stores_leave_a_reachable_field stores h x hx hall).1]
  exact hderef

/-- A store to a reachable object is not a pre-publication store and can null
the field: without the derivation the arm stays. -/
theorem a_published_store_can_null_the_field (h : Heap) (x : Nat) (hx : x ∈ h.reachable) :
    ¬ PrePublication h (.store x none) ∧ (applyStore h (.store x none)).field x = none := by
  refine ⟨fun hs => hs hx, ?_⟩
  simp [applyStore]

/-- The kernel's decision (`null_branch_excluded`): every part of the
derivation, the dominating dereference first. -/
structure NullBranchEvidence where
  dominatingDereference : Bool
  localDefeat : Bool
  baseTypeRead : Bool
  unresolvedWrites : Bool
  everyStorePrePublication : Bool
  deriving DecidableEq, Repr

def nullBranchExcluded (e : NullBranchEvidence) : Bool :=
  e.dominatingDereference && !e.localDefeat && e.baseTypeRead && !e.unresolvedWrites &&
    e.everyStorePrePublication

inductive Arm where
  | null
  | nonNull
  deriving DecidableEq, Repr

/-- The arms the path reader follows at a null test. -/
def arms (e : NullBranchEvidence) : List Arm :=
  if nullBranchExcluded e then [.nonNull] else [.null, .nonNull]

theorem a_dropped_null_arm_had_its_derivation (e : NullBranchEvidence)
    (h : Arm.null ∉ arms e) :
    e.dominatingDereference = true ∧ e.localDefeat = false ∧ e.baseTypeRead = true ∧
      e.unresolvedWrites = false ∧ e.everyStorePrePublication = true := by
  unfold arms at h
  split at h
  · rename_i hx
    simp only [nullBranchExcluded, Bool.and_eq_true, Bool.not_eq_true'] at hx
    exact ⟨hx.1.1.1.1, hx.1.1.1.2, hx.1.1.2, hx.1.2, hx.2⟩
  · simp at h

theorem without_a_dominating_dereference_the_null_arm_stays (e : NullBranchEvidence)
    (h : e.dominatingDereference = false) : Arm.null ∈ arms e := by
  simp [arms, nullBranchExcluded, h]

theorem a_store_after_publication_keeps_the_null_arm (e : NullBranchEvidence)
    (h : e.everyStorePrePublication = false) : Arm.null ∈ arms e := by
  simp [arms, nullBranchExcluded, h]

/-! ### Joins under P-POSIX-JOIN -/

/-- The facts of one join that decide its POSIX result. -/
structure JoinFacts where
  joinable : Bool
  joinedBefore : Bool
  otherJoiner : Bool
  isSelf : Bool
  cycle : Bool
  deriving DecidableEq, Repr

/-- **P-POSIX-JOIN** (the named premise, as a model of the standard): the join
returns 0 exactly when none of its error cases holds. -/
def posixSucceeds (f : JoinFacts) : Bool :=
  f.joinable && !f.joinedBefore && !f.otherJoiner && !f.isSelf && !f.cycle

/-- The four derived exclusions of an unchecked first join (`JoinExclusions`). -/
structure JoinExclusions where
  joinable : Bool
  notBusy : Bool
  notSelf : Bool
  noCycle : Bool
  deriving DecidableEq, Repr

def joinSucceeds (e : JoinExclusions) : Bool :=
  e.joinable && e.notBusy && e.notSelf && e.noCycle

/-- What a derived exclusion certifies about the join it excludes. -/
def Certifies (e : JoinExclusions) (f : JoinFacts) : Prop :=
  (e.joinable = true → f.joinable = true) ∧ (e.notBusy = true → f.otherJoiner = false) ∧
    (e.notSelf = true → f.isSelf = false) ∧ (e.noCycle = true → f.cycle = false)

theorem an_unchecked_first_join_with_every_case_excluded_succeeds (e : JoinExclusions)
    (f : JoinFacts) (hc : Certifies e f) (hfirst : f.joinedBefore = false)
    (h : joinSucceeds e = true) : posixSucceeds f = true := by
  simp only [joinSucceeds, Bool.and_eq_true] at h
  obtain ⟨⟨⟨hj, hb⟩, hs⟩, hy⟩ := h
  obtain ⟨cj, cb, cs, cy⟩ := hc
  simp [posixSucceeds, cj hj, cb hb, cs hs, cy hy, hfirst]

theorem a_detached_thread_join_never_succeeds (f : JoinFacts) (h : f.joinable = false) :
    posixSucceeds f = false := by
  simp [posixSucceeds, h]

theorem a_repeated_join_never_succeeds (f : JoinFacts) (h : f.joinedBefore = true) :
    posixSucceeds f = false := by
  simp [posixSucceeds, h]

theorem a_join_from_the_thread_itself_never_succeeds (f : JoinFacts) (h : f.isSelf = true) :
    posixSucceeds f = false := by
  simp [posixSucceeds, h]

theorem an_underived_case_does_not_count (e : JoinExclusions)
    (h : e.joinable = false ∨ e.notBusy = false ∨ e.notSelf = false ∨ e.noCycle = false) :
    joinSucceeds e = false := by
  rcases h with h | h | h | h <;> simp [joinSucceeds, h]

/-! ### The release decision with stop paths -/

inductive StopEvidence where
  | checkedSuccess
  | unchecked
  | uncheckedFirst (e : JoinExclusions)
  | repeated
  | failed
  | absent
  deriving DecidableEq, Repr

def StopEvidence.admitted : StopEvidence → Bool
  | .checkedSuccess => true
  | .uncheckedFirst e => joinSucceeds e
  | _ => false

inductive Descendant where
  | joined
  | neverJoined
  | unresolved
  deriving DecidableEq, Repr

/-- `release_verdict` with stop paths: every path admitted, then no descendant
never joined, then nothing open. -/
def stopReleaseVerdict (paths : List StopEvidence) (descendants : List Descendant)
    (openPremise : Bool) : Verdict :=
  if paths.all StopEvidence.admitted then
    if descendants.contains .neverJoined then .notDischarged
    else if openPremise || descendants.contains .unresolved then .unresolved
    else .discharged
  else .notDischarged

theorem a_discharged_release_joined_every_descendant (paths : List StopEvidence)
    (ds : List Descendant) (o : Bool) (h : stopReleaseVerdict paths ds o = .discharged) :
    (∀ e ∈ paths, e.admitted = true) ∧ (∀ d ∈ ds, d = .joined) ∧ o = false := by
  unfold stopReleaseVerdict at h
  split at h
  · rename_i hall
    split at h
    · simp at h
    · rename_i hnever
      split at h
      · simp at h
      · rename_i hopen
        simp only [Bool.or_eq_true, not_or, Bool.not_eq_true] at hopen
        refine ⟨fun e he => List.all_eq_true.mp hall e he, fun d hd => ?_, hopen.1⟩
        cases d with
        | joined => rfl
        | neverJoined => exact absurd (List.elem_eq_true_of_mem hd) (by simpa using hnever)
        | unresolved =>
          exact absurd (List.elem_eq_true_of_mem hd) (by simpa using hopen.2)
  · simp at h

theorem a_repeated_join_refuses (paths : List StopEvidence) (ds : List Descendant) (o : Bool)
    (h : StopEvidence.repeated ∈ paths) : stopReleaseVerdict paths ds o = .notDischarged := by
  have : paths.all StopEvidence.admitted = false := by
    rw [List.all_eq_false]
    exact ⟨_, h, by simp [StopEvidence.admitted]⟩
  simp [stopReleaseVerdict, this]

theorem an_unchecked_first_join_counts_only_with_every_case_excluded (e : JoinExclusions) :
    (StopEvidence.uncheckedFirst e).admitted = true ↔ joinSucceeds e = true := by
  simp [StopEvidence.admitted]

theorem an_unresolved_descendant_blocks_the_discharge (paths : List StopEvidence)
    (ds : List Descendant) (o : Bool) (h : Descendant.unresolved ∈ ds) :
    stopReleaseVerdict paths ds o ≠ .discharged := by
  intro hd
  have := (a_discharged_release_joined_every_descendant paths ds o hd).2.1 _ h
  cases this

theorem a_never_joined_descendant_refuses (paths : List StopEvidence) (ds : List Descendant)
    (o : Bool) (hall : paths.all StopEvidence.admitted = true) (h : Descendant.neverJoined ∈ ds) :
    stopReleaseVerdict paths ds o = .notDischarged := by
  simp [stopReleaseVerdict, hall, h]

/-- Operationally: a live thread that retains the object blocks its release,
whoever started it; only its join (removing it from `live`) lifts the block. -/
theorem a_live_descendant_blocks_the_release (retains : Thread → Obj → Bool)
    (s : ThreadState) (t : Thread) (o : Obj) (hl : t ∈ s.live) (hr : retains t o = true) :
    step retains s (.release o) = none := by
  have : (s.live.all fun u => !retains u o) = false := by
    rw [List.all_eq_false]
    exact ⟨t, hl, by simp [hr]⟩
  simp [step, this]

/-! ### The two tests' shapes (negative control and fixed test) -/

/-- The busy case of the fixed test's status-branch join is not excluded (the
same entry-named identifier is joined on six other threads). -/
def statusBranchJoin : JoinExclusions :=
  { joinable := true, notBusy := false, notSelf := true, noCycle := true }

/-- The fixed test's release: a checked join (the main path), the status
branch's first join, and the repeated join after `flb_start`'s failure. -/
def fixedTestPaths : List StopEvidence :=
  [.checkedSuccess, .uncheckedFirst statusBranchJoin, .repeated]

/-- The old test's `unlink` before `flb_stop`, and its `flb_destroy` after an
unchecked `flb_stop` whose join may have failed. -/
def oldTestUnlinkPaths : List StopEvidence := [.absent]
def oldTestDestroyPaths : List StopEvidence :=
  [.checkedSuccess, .failed, .uncheckedFirst statusBranchJoin, .repeated]

theorem the_fixed_test_release_is_not_discharged :
    stopReleaseVerdict fixedTestPaths [.unresolved] false = .notDischarged := by decide

theorem the_old_test_stays_not_discharged :
    stopReleaseVerdict oldTestUnlinkPaths [.unresolved] false = .notDischarged ∧
      stopReleaseVerdict oldTestDestroyPaths [.unresolved] false = .notDischarged := by decide

/-- Even with the busy case excluded, the repeated join alone keeps the fixed
test's release not discharged. -/
theorem the_repeated_join_alone_keeps_the_fixed_test_not_discharged (ds : List Descendant)
    (o : Bool) :
    stopReleaseVerdict [.checkedSuccess,
      .uncheckedFirst { joinable := true, notBusy := true, notSelf := true, noCycle := true },
      .repeated] ds o = .notDischarged :=
  a_repeated_join_refuses _ ds o (by simp)

end StopPaths

/-! ## The strictly paired start/stop lifecycle (W47-FLB-LIB-LIFECYCLE, W47-FLB-LIB-STRICT-PAIRING)

Companion to `crates/qcore-core/src/paired_lifecycle.rs`,
`crates/qcore-c-gateway/src/condvar_census/lifecycle.rs` and
`crates/quirin-source-analysis/src/lifecycle_census.rs` (gateway rule 10q as
amended, contract specification §C04.11).

* **The state machine.** `lstep` is `paired_lifecycle::step` under the user's
  decision of 2026-09-29: a start only from created (a restart is not
  derived), a stop only from started-ok (a start that returned 0), a destroy
  from created, start-failed or stopped, nothing after a destroy. A stop after
  a failed start, a second stop, a stop of a never-started context, a double
  start and a destroy of a running context are refused (`none`); so is a stop
  or a destroy after a start whose value is undecided (not established).
* **At most one stop.** A history the machine accepts stops at most once, so
  with L2 in force a conforming path joins the engine thread exactly once,
  and that join is its first.
* **Body clauses.** L2 is in force only where every followed path of the stop
  joins the started thread exactly once; L7 only where no followed nonzero
  return of the start leaves a thread it began unjoined.
* **The gate.** A path whose history the machine refuses never discharges its
  release; without a refusal the release decision is unchanged. The fixed
  test's failed-start path is refused; its conforming paths and the old
  test's shape stay not discharged. -/

section Lifecycle

inductive LOp where
  | start
  | stop
  | destroy
  deriving DecidableEq, Repr

inductive LOutcome where
  | succeeded
  | failed
  | undecided
  deriving DecidableEq, Repr

inductive LState where
  | created
  | started (o : LOutcome)
  | stopped
  | destroyed
  deriving DecidableEq, Repr

/-- `paired_lifecycle::step` (the strict pair); `none` is a refused
precondition (a violation, or one not established). -/
def lstep : LState → LOp → LOutcome → Option LState
  | .destroyed, _, _ => none
  | .created, .start, o => some (.started o)
  | .stopped, .start, _ => none
  | .started _, .start, _ => none
  | .started .succeeded, .stop, _ => some .stopped
  | .started .failed, .stop, _ => none
  | .started .undecided, .stop, _ => none
  | .created, .stop, _ => none
  | .stopped, .stop, _ => none
  | .created, .destroy, _ => some .destroyed
  | .started .failed, .destroy, _ => some .destroyed
  | .stopped, .destroy, _ => some .destroyed
  | .started .succeeded, .destroy, _ => none
  | .started .undecided, .destroy, _ => none

/-- A history run from a state. -/
def lrun : LState → List (LOp × LOutcome) → Option LState
  | s, [] => some s
  | s, (op, o) :: rest => (lstep s op o).bind (fun s' => lrun s' rest)

theorem lrun_append (xs ys : List (LOp × LOutcome)) (t : LState) :
    lrun t (xs ++ ys) = (lrun t xs).bind (fun u => lrun u ys) := by
  induction xs generalizing t with
  | nil => simp [lrun]
  | cons x xs ih =>
    obtain ⟨op, oc⟩ := x
    simp only [List.cons_append, lrun]
    cases lstep t op oc with
    | none => rfl
    | some u => exact ih u

/-- A refused step anywhere rejects the whole history. -/
theorem a_refused_step_rejects_the_history (pre rest : List (LOp × LOutcome)) (s : LState)
    (op : LOp) (o : LOutcome) (hpre : lrun .created pre = some s) (hs : lstep s op o = none) :
    lrun .created (pre ++ (op, o) :: rest) = none := by
  rw [lrun_append, hpre]
  simp [lrun, hs]

theorem starting_twice_is_rejected (o o' : LOutcome) : lstep (.started o) .start o' = none := rfl

theorem a_restart_after_a_stop_is_rejected (o : LOutcome) : lstep .stopped .start o = none := rfl

theorem nothing_follows_a_destroy (op : LOp) (o : LOutcome) : lstep .destroyed op o = none := by
  cases op <;> rfl

/-- A stop is admitted only from started-ok, and leads to stopped. -/
theorem a_stop_is_admitted_only_from_started_ok (s s' : LState) (o : LOutcome)
    (h : lstep s .stop o = some s') : s = .started .succeeded ∧ s' = .stopped := by
  cases s with
  | started oc => cases oc <;> simp_all [lstep]
  | _ => simp_all [lstep]

theorem a_stop_after_a_failed_start_is_a_violation (o : LOutcome) :
    lstep (.started .failed) .stop o = none := rfl

theorem a_second_stop_is_a_violation (o o' : LOutcome) :
    lrun .created [(.start, .succeeded), (.stop, o), (.stop, o')] = none := rfl

theorem a_stop_of_a_never_started_context_is_a_violation (o : LOutcome) :
    lstep .created .stop o = none := rfl

/-- A start is accepted only from created. -/
theorem an_accepted_start_began_created (s s' : LState) (o : LOutcome)
    (h : lstep s .start o = some s') : s = .created := by
  cases s <;> simp_all [lstep]

/-- A destroy is accepted only from created, start-failed or stopped. -/
theorem an_accepted_destroy_was_not_running (s s' : LState) (o : LOutcome)
    (h : lstep s .destroy o = some s') :
    s = .created ∨ s = .started .failed ∨ s = .stopped := by
  cases s with
  | started oc => cases oc <;> simp_all [lstep]
  | _ => simp_all [lstep]

theorem destroying_a_running_context_is_rejected (o : LOutcome) :
    lstep (.started .succeeded) .destroy o = none := rfl

theorem a_destroy_after_a_failed_start_is_admitted (o : LOutcome) :
    lstep (.started .failed) .destroy o = some .destroyed := rfl

/-- The conforming sequences: start, stop, destroy; a failed start, then
destroy; a destroy of a never-started context. -/
theorem the_strict_sequences_are_accepted (o o' : LOutcome) :
    lrun .created [(.start, .succeeded), (.stop, o), (.destroy, o')] = some .destroyed ∧
      lrun .created [(.start, .failed), (.destroy, o)] = some .destroyed ∧
      lrun .created [(.destroy, o)] = some .destroyed := ⟨rfl, rfl, rfl⟩

/-- A double start anywhere in a history rejects the whole history. -/
theorem a_history_with_a_double_start_is_rejected (o o' : LOutcome)
    (pre rest : List (LOp × LOutcome)) (hpre : lrun .created pre = some .created) :
    lrun .created (pre ++ (.start, o) :: (.start, o') :: rest) = none := by
  rw [lrun_append, hpre]
  simp [lrun, lstep]

/-- How many stops a history holds. -/
def stopsIn : List (LOp × LOutcome) → Nat
  | [] => 0
  | (op, _) :: rest => (if op = .stop then 1 else 0) + stopsIn rest

/-- How many more stops a state admits. -/
def stopBudget : LState → Nat
  | .created => 1
  | .started .succeeded => 1
  | _ => 0

/-- One accepted step spends a stop's budget, and never adds any. -/
theorem an_accepted_step_spends_its_stop (s u : LState) (op : LOp) (o : LOutcome)
    (h : lstep s op o = some u) : (if op = .stop then 1 else 0) + stopBudget u ≤ stopBudget s := by
  cases s with
  | destroyed => simp [lstep] at h
  | created =>
    cases op with
    | start => simp only [lstep, Option.some.injEq] at h; subst h; cases o <;> simp [stopBudget]
    | stop => simp [lstep] at h
    | destroy => simp only [lstep, Option.some.injEq] at h; subst h; simp [stopBudget]
  | started so =>
    cases so <;> cases op <;> simp [lstep] at h <;> subst h <;> simp [stopBudget]
  | stopped =>
    cases op <;> simp [lstep] at h <;> subst h <;> simp [stopBudget]

theorem an_accepted_history_stays_within_its_stop_budget (h : List (LOp × LOutcome)) :
    ∀ (s t : LState), lrun s h = some t → stopsIn h ≤ stopBudget s := by
  induction h with
  | nil => intro s t _; simp [stopsIn]
  | cons x rest ih =>
    obtain ⟨op, oc⟩ := x
    intro s t hrun
    simp only [lrun] at hrun
    cases hstep : lstep s op oc with
    | none => simp [hstep] at hrun
    | some u =>
      simp only [hstep, Option.bind_some] at hrun
      have hrest := ih u t hrun
      have hb := an_accepted_step_spends_its_stop s u op oc hstep
      simp only [stopsIn]
      by_cases hop : op = .stop <;> simp only [hop, ite_true, ite_false] at hb ⊢ <;> omega

/-- A history the strict pair accepts stops at most once. -/
theorem a_conforming_history_stops_at_most_once (h : List (LOp × LOutcome)) (t : LState)
    (hrun : lrun .created h = some t) : stopsIn h ≤ 1 :=
  an_accepted_history_stays_within_its_stop_budget h .created t hrun

/-! ### Body clauses -/

inductive BJoin where
  | firstOfStarted
  | repeated
  | notStarted
  | unresolved
  deriving DecidableEq, Repr

/-- One followed path of a declared stop's body: its joins and whether the
reader left anything open on it. -/
structure BPath where
  joins : List BJoin
  isOpen : Bool
  deriving DecidableEq, Repr

inductive CStatus where
  | inForce
  | refuted
  | held
  deriving DecidableEq, Repr

/-- The first stop ends the started thread on a path: one join, its first. -/
def endsStarted (p : BPath) : Bool := p.joins == [.firstOfStarted]

/-- `ends_started_status` (L2). -/
def endsStartedStatus (ps : List BPath) : CStatus :=
  if ps.isEmpty then .held
  else if ps.all (fun p => !endsStarted p && !p.joins.contains .unresolved) then .refuted
  else if ps.any (fun p => !endsStarted p) then .held
  else if ps.any (fun p => p.isOpen) then .held
  else .inForce

theorem the_first_stop_clause_is_in_force_only_if_every_path_joins_the_started_thread_once
    (ps : List BPath) (h : endsStartedStatus ps = .inForce) :
    ∀ p ∈ ps, p.joins = [.firstOfStarted] ∧ p.isOpen = false := by
  unfold endsStartedStatus at h
  split at h
  · cases h
  · split at h
    · cases h
    · split at h
      · cases h
      · split at h
        · cases h
        · rename_i _ _ hends hopen
          intro p hp
          have he : ¬ (!endsStarted p) = true := by
            intro hx; exact hends (List.any_eq_true.mpr ⟨p, hp, hx⟩)
          have ho : ¬ p.isOpen = true := by
            intro hx; exact hopen (List.any_eq_true.mpr ⟨p, hp, hx⟩)
          refine ⟨?_, by simpa using ho⟩
          simpa [endsStarted] using he

/-- One followed nonzero return of a declared start: whether it leaves a
thread it began unjoined, and whether the reader left anything open. -/
structure SPath where
  leavesThread : Bool
  isOpen : Bool
  deriving DecidableEq, Repr

/-- `failed_start_status` (L7). -/
def failedStartStatus (ps : List SPath) : CStatus :=
  if ps.isEmpty then .held
  else if ps.all (fun p => p.leavesThread) then .refuted
  else if ps.any (fun p => p.leavesThread) then .held
  else if ps.any (fun p => p.isOpen) then .held
  else .inForce

theorem the_failed_start_clause_is_in_force_only_if_no_return_leaves_a_thread
    (ps : List SPath) (h : failedStartStatus ps = .inForce) :
    ∀ p ∈ ps, p.leavesThread = false ∧ p.isOpen = false := by
  unfold failedStartStatus at h
  split at h
  · cases h
  · split at h
    · cases h
    · split at h
      · cases h
      · split at h
        · cases h
        · rename_i _ _ hleave hopen
          intro p hp
          have hl : ¬ p.leavesThread = true := by
            intro hx; exact hleave (List.any_eq_true.mpr ⟨p, hp, hx⟩)
          have ho : ¬ p.isOpen = true := by
            intro hx; exact hopen (List.any_eq_true.mpr ⟨p, hp, hx⟩)
          exact ⟨by simpa using hl, by simpa using ho⟩

/-- A body with no followed path decides nothing. -/
theorem no_followed_path_holds_the_clause :
    endsStartedStatus [] = .held ∧ failedStartStatus [] = .held := ⟨rfl, rfl⟩

/-- L2 in force: a conforming path that stops joins the engine thread exactly
once, whichever followed body path its one stop took, and that join is the
thread's first. -/
theorem a_conforming_path_has_exactly_one_engine_join (h : List (LOp × LOutcome)) (t : LState)
    (hrun : lrun .created h = some t) (hstop : 1 ≤ stopsIn h) (ps : List BPath)
    (hl2 : endsStartedStatus ps = .inForce) (p : BPath) (hp : p ∈ ps) :
    stopsIn h = 1 ∧ p.joins = [.firstOfStarted] :=
  ⟨Nat.le_antisymm (a_conforming_history_stops_at_most_once h t hrun) hstop,
    (the_first_stop_clause_is_in_force_only_if_every_path_joins_the_started_thread_once
      ps hl2 p hp).1⟩

/-! ### The gate on a release -/

/-- `lifecycle_gate` joined before the release decision. -/
def gatedVerdict (violated : Bool) (v : Verdict) : Verdict :=
  (if violated then Verdict.notDischarged else Verdict.discharged).join v

theorem a_lifecycle_violation_blocks_the_release (v : Verdict) :
    gatedVerdict true v ≠ .discharged := by
  cases v <;> simp [gatedVerdict, Verdict.join, Verdict.rank]

theorem without_a_violation_the_release_is_unchanged (v : Verdict) :
    gatedVerdict false v = v := by
  cases v <;> simp [gatedVerdict, Verdict.join, Verdict.rank]

/-- The gate of one path: refused where the strict pair refuses its history. -/
def historyGate (h : List (LOp × LOutcome)) (v : Verdict) : Verdict :=
  gatedVerdict (lrun .created h).isNone v

theorem a_refused_history_never_discharges (h : List (LOp × LOutcome)) (v : Verdict)
    (hv : lrun .created h = none) : historyGate h v ≠ .discharged := by
  simp only [historyGate, hv, Option.isNone_none]
  exact a_lifecycle_violation_blocks_the_release v

/-- A stop after a failed start, and a second stop, never discharge whatever
the release's own reading. -/
theorem a_stop_after_a_failed_start_or_a_second_stop_never_discharges (o o' : LOutcome)
    (rest : List (LOp × LOutcome)) (v : Verdict) :
    historyGate ((.start, .failed) :: (.stop, o) :: rest) v ≠ .discharged ∧
      historyGate ((.start, .succeeded) :: (.stop, o) :: (.stop, o') :: rest) v ≠
        .discharged :=
  ⟨a_refused_history_never_discharges _ v rfl, a_refused_history_never_discharges _ v rfl⟩

/-! ### The two tests' shapes under the strict pair -/

/-- The status branch's first join on the fixed test's conforming paths: its
busy and cycle cases are not excluded (the census's `not busy` and `no cycle`
exclusions fail at the three trees). -/
def conformingStatusBranchJoin : JoinExclusions :=
  { joinable := true, notBusy := false, notSelf := true, noCycle := false }

/-- The fixed test's conforming paths (after a start that returned 0): the
checked join at :1077, or the status branch's unchecked first join at :1058.
No repeated join. -/
def fixedTestConformingPaths : List StopEvidence :=
  [.checkedSuccess, .uncheckedFirst conformingStatusBranchJoin]

/-- The fixed test under the strict pair: its failed-start path (start
returned nonzero, stop, destroy) is refused and never discharges; its
conforming path (start returned 0, stop, destroy) is accepted, joins the
engine thread once per path, and stays not discharged, since the status
branch's join is not counted; even with every POSIX case excluded, the
unresolved descendants keep it from discharging. -/
theorem the_fixed_test_under_the_strict_pair (v : Verdict) :
    historyGate [(.start, .failed), (.stop, .undecided), (.destroy, .undecided)] v ≠
        .discharged ∧
      lrun .created [(.start, .succeeded), (.stop, .undecided), (.destroy, .undecided)] =
        some .destroyed ∧
      StopEvidence.repeated ∉ fixedTestConformingPaths ∧
      historyGate [(.start, .succeeded), (.stop, .undecided), (.destroy, .undecided)]
          (stopReleaseVerdict fixedTestConformingPaths [.unresolved] false) = .notDischarged ∧
      stopReleaseVerdict [.checkedSuccess,
          .uncheckedFirst { joinable := true, notBusy := true, notSelf := true, noCycle := true }]
          [.unresolved] false ≠ .discharged := by
  refine ⟨a_refused_history_never_discharges _ v rfl, rfl, by decide, by decide, ?_⟩
  exact an_unresolved_descendant_blocks_the_discharge _ _ false (by simp)

/-- The old test (negative control): its `unlink` before the stop conforms
and stays not discharged (no join on its paths); its `flb_destroy` after the
unguarded stop is refused on the failed-start path and not discharged on the
conforming ones (a failed join at :1077 on a path). -/
theorem the_old_and_fixed_test_shapes_stay_not_discharged (o : LOutcome) :
    historyGate [(.start, o)] (stopReleaseVerdict oldTestUnlinkPaths [.unresolved] false) =
        .notDischarged ∧
      historyGate [(.start, .failed), (.stop, .undecided), (.destroy, .undecided)]
          (stopReleaseVerdict oldTestDestroyPaths [.unresolved] false) = .notDischarged ∧
      historyGate [(.start, .succeeded), (.stop, .undecided), (.destroy, .undecided)]
          (stopReleaseVerdict [.checkedSuccess, .failed,
            .uncheckedFirst conformingStatusBranchJoin] [.unresolved] false) =
        .notDischarged ∧
      historyGate [(.start, .succeeded), (.stop, .undecided), (.destroy, .undecided)]
          (stopReleaseVerdict fixedTestConformingPaths [.unresolved] false) = .notDischarged := by
  cases o <;> decide

end Lifecycle

end QCore.CondvarJoin
