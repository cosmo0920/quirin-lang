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

end QCore.CondvarJoin
