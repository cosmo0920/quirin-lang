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

namespace QCore.DescriptorNativeObservation

/-! Observation-level lifecycle model for native descriptor records (card
W43-DESCRIPTOR-NATIVE-CHECK).

One `Slot` is the replayed state of one identity: a descriptor number or a
pointer. An observed trace is a list of `(identity, event)` pairs; `run`
accepts it only when every step satisfies the lifecycle properties the Rust
replay (`qcore_contract::descriptor_observation`) checks, so an accepted
trace is exactly a violation-free run. The model extends the single-slot
`QCore.DescriptorObservation` (Monkey socketpair descriptors) with kinds,
counted dl references and multi-identity leak sets.

Boundaries. This proves properties of the model, not of the Rust replay
(their correspondence is maintained by review, trust boundary 28), not of
the C observer, and not that records are authentic or complete: identities
the observer never saw acquired are outside the model and are counted, not
judged, by the replay. A reused number or address is a new generation; the
model cannot tell which generation a caller meant when a number is reused
(the `CT50-PROOF-ADDRESS-REUSE-UNPROVED` shape). -/

inductive Kind where
  | fd | file | dir | dlHandle | addrinfo | mapping | processStream | ifaddrs
  deriving DecidableEq, Repr

/-- The loader reference-counts dl handles; every other kind has one owner. -/
def Kind.counted : Kind → Bool
  | .dlHandle => true
  | _ => false

structure Slot where
  kind : Kind := .fd
  generation : Nat := 0
  live : Bool := false
  refs : Nat := 0
  deriving DecidableEq, Repr

inductive Event where
  | acquire (kind : Kind) (generation : Nat)
  | use (generation : Nat)
  | release (kind : Kind) (generation : Nat)
  deriving DecidableEq, Repr

def Event.generation : Event → Nat
  | .acquire _ g => g
  | .use g => g
  | .release _ g => g

def Event.isAcquire : Event → Bool
  | .acquire _ _ => true
  | _ => false

/-- One accepted step, or `none` when a lifecycle property fails. -/
def step (slot : Slot) : Event → Option Slot
  | .acquire kind g =>
      if slot.live then
        -- A counted kind acquired while live adds a reference to the same
        -- generation; any other acquisition of a live identity is refused.
        if kind.counted && decide (slot.kind = kind) && decide (g = slot.generation) then
          some { slot with refs := slot.refs + 1 }
        else none
      else if g = slot.generation + 1 then
        some { kind, generation := g, live := true, refs := 1 }
      else none
  | .use g =>
      if slot.live && decide (g = slot.generation) then some slot else none
  | .release kind g =>
      if slot.live && decide (g = slot.generation) && decide (slot.kind = kind) then
        if kind.counted && decide (1 < slot.refs) then
          some { slot with refs := slot.refs - 1 }
        else
          some { slot with live := false, refs := 0 }
      else none

/-- Replay one identity's events. -/
def run (slot : Slot) : List Event → Option Slot
  | [] => some slot
  | e :: rest => (step slot e).bind (fun next => run next rest)

theorem run_append (slot : Slot) (pre post : List Event) :
    run slot (pre ++ post) = (run slot pre).bind (fun mid => run mid post) := by
  induction pre generalizing slot with
  | nil => simp [run]
  | cons e rest ih =>
    simp only [List.cons_append, run]
    cases step slot e with
    | none => simp
    | some next => simp [ih]

/-! ### Step characterizations -/

private theorem ite_of_true {c : Prop} [Decidable c] {α : Type} {a b : α} (h : c) :
    (if c then a else b) = a := by
  simp [h]

private theorem ite_of_false {c : Prop} [Decidable c] {α : Type} {a b : α} (h : ¬c) :
    (if c then a else b) = b := by
  simp [h]

theorem step_acquire_some {slot next : Slot} {k : Kind} {g : Nat} :
    step slot (.acquire k g) = some next ↔
      (slot.live = true ∧ k.counted = true ∧ slot.kind = k ∧ g = slot.generation ∧
        next = { slot with refs := slot.refs + 1 }) ∨
      (slot.live = false ∧ g = slot.generation + 1 ∧
        next = { kind := k, generation := g, live := true, refs := 1 }) := by
  constructor
  · intro h
    simp only [step] at h
    cases hl : slot.live
    · rw [hl, ite_of_false Bool.false_ne_true] at h
      by_cases hg : g = slot.generation + 1
      · rw [ite_of_true hg] at h
        exact Or.inr ⟨rfl, hg, (Option.some.inj h).symm⟩
      · rw [ite_of_false hg] at h
        cases h
    · rw [hl, ite_of_true rfl] at h
      by_cases hc : (k.counted && decide (slot.kind = k) && decide (g = slot.generation)) = true
      · rw [ite_of_true hc] at h
        simp only [Bool.and_eq_true, decide_eq_true_eq] at hc
        exact Or.inl ⟨rfl, hc.1.1, hc.1.2, hc.2, (Option.some.inj h).symm⟩
      · rw [ite_of_false hc] at h
        cases h
  · intro h
    rcases h with ⟨hl, hc, hk, hg, rfl⟩ | ⟨hl, hg, rfl⟩
    · subst hg
      simp [step, hl, hc, hk]
    · simp [step, hl, hg]

theorem step_use_some {slot next : Slot} {g : Nat} :
    step slot (.use g) = some next ↔ slot.live = true ∧ g = slot.generation ∧ next = slot := by
  constructor
  · intro h
    simp only [step] at h
    by_cases hc : (slot.live && decide (g = slot.generation)) = true
    · rw [ite_of_true hc] at h
      simp only [Bool.and_eq_true, decide_eq_true_eq] at hc
      exact ⟨hc.1, hc.2, (Option.some.inj h).symm⟩
    · rw [ite_of_false hc] at h
      cases h
  · rintro ⟨hl, rfl, rfl⟩
    simp [step, hl]

theorem step_release_some {slot next : Slot} {k : Kind} {g : Nat} :
    step slot (.release k g) = some next ↔
      slot.live = true ∧ g = slot.generation ∧ slot.kind = k ∧
        ((k.counted = true ∧ 1 < slot.refs ∧ next = { slot with refs := slot.refs - 1 }) ∨
         (¬(k.counted = true ∧ 1 < slot.refs) ∧ next = { slot with live := false, refs := 0 })) := by
  constructor
  · intro h
    simp only [step] at h
    by_cases hc : (slot.live && decide (g = slot.generation) && decide (slot.kind = k)) = true
    · rw [ite_of_true hc] at h
      simp only [Bool.and_eq_true, decide_eq_true_eq] at hc
      refine ⟨hc.1.1, hc.1.2, hc.2, ?_⟩
      by_cases hr : (k.counted && decide (1 < slot.refs)) = true
      · rw [ite_of_true hr] at h
        simp only [Bool.and_eq_true, decide_eq_true_eq] at hr
        exact Or.inl ⟨hr.1, hr.2, (Option.some.inj h).symm⟩
      · rw [ite_of_false hr] at h
        simp only [Bool.and_eq_true, decide_eq_true_eq] at hr
        exact Or.inr ⟨hr, (Option.some.inj h).symm⟩
    · rw [ite_of_false hc] at h
      cases h
  · rintro ⟨hl, rfl, hk, ⟨hc, hr, rfl⟩ | ⟨hr, rfl⟩⟩
    · simp [step, hl, hk, hc, hr]
    · have : (k.counted && decide (1 < slot.refs)) = false := by
        cases hkc : k.counted <;> simp_all
      simp [step, hl, hk, this]

/-! ### Generation monotonicity -/

theorem step_generation_monotone {slot next : Slot} {e : Event}
    (accepted : step slot e = some next) :
    slot.generation ≤ next.generation ∧ slot.generation ≤ e.generation ∧
      (slot.live = false → slot.generation < e.generation ∧ next.generation = e.generation ∧
        next.live = true) := by
  cases e with
  | acquire k g =>
    rcases step_acquire_some.mp accepted with ⟨hl, _, _, hg, rfl⟩ | ⟨hl, hg, rfl⟩
    · simp [Event.generation, hg, hl]
    · simp [Event.generation, hg]
  | use g =>
    obtain ⟨hl, hg, rfl⟩ := step_use_some.mp accepted
    simp [Event.generation, hg, hl]
  | release k g =>
    obtain ⟨hl, hg, _, ⟨_, _, rfl⟩ | ⟨_, rfl⟩⟩ := step_release_some.mp accepted
    · simp [Event.generation, hg, hl]
    · simp [Event.generation, hg, hl]

/-- Every event of an accepted run names a generation no older than the
starting slot's; if the slot starts dead, strictly newer. -/
theorem run_events_not_older {slot final : Slot} {events : List Event}
    (accepted : run slot events = some final) :
    ∀ e ∈ events, slot.generation ≤ e.generation ∧
      (slot.live = false → slot.generation < e.generation) := by
  induction events generalizing slot with
  | nil => simp
  | cons head rest ih =>
    simp only [run] at accepted
    cases hstep : step slot head with
    | none => simp [hstep] at accepted
    | some next =>
      simp only [hstep, Option.bind_some] at accepted
      have mono := step_generation_monotone hstep
      intro e member
      rcases List.mem_cons.mp member with rfl | tail
      · exact ⟨mono.2.1, fun dead => (mono.2.2 dead).1⟩
      · have later := (ih accepted e tail).1
        refine ⟨Nat.le_trans mono.1 later, fun dead => ?_⟩
        have fresh := mono.2.2 dead
        omega

/-! ### The four required observation properties -/

/-- **Release matches kind.** An accepted release names the live generation
and the kind that generation was acquired with. -/
theorem release_matches_live_kind {slot next : Slot} {kind : Kind} {g : Nat}
    (accepted : step slot (.release kind g) = some next) :
    slot.live = true ∧ slot.generation = g ∧ slot.kind = kind := by
  obtain ⟨hl, hg, hk, _⟩ := step_release_some.mp accepted
  exact ⟨hl, hg.symm, hk⟩

/-- A release keeps the generation it names. -/
theorem release_keeps_generation {slot next : Slot} {kind : Kind} {g : Nat}
    (accepted : step slot (.release kind g) = some next) : next.generation = g := by
  obtain ⟨_, hg, _, ⟨_, _, rfl⟩ | ⟨_, rfl⟩⟩ := step_release_some.mp accepted <;> simp [hg]

/-- A non-counted release always ends its generation. -/
theorem single_owner_release_is_terminal {slot next : Slot} {kind : Kind} {g : Nat}
    (single : kind.counted = false) (accepted : step slot (.release kind g) = some next) :
    next.live = false ∧ next.generation = g := by
  obtain ⟨_, hg, _, ⟨hc, _, _⟩ | ⟨_, rfl⟩⟩ := step_release_some.mp accepted
  · rw [single] at hc
    cases hc
  · exact ⟨rfl, hg.symm⟩

/-- Along an accepted run, the slot keeps the acquired kind for as long as it
is at the acquired generation. -/
theorem kind_stable_at_generation {kind : Kind} {g : Nat} :
    ∀ (s : Slot) (events : List Event) (f : Slot),
      g ≤ s.generation → (s.generation = g → s.kind = kind) →
      run s events = some f → g ≤ f.generation ∧ (f.generation = g → f.kind = kind) := by
  intro s events
  induction events generalizing s with
  | nil =>
    intro f low same accepted
    simp only [run, Option.some.injEq] at accepted
    subst accepted
    exact ⟨low, same⟩
  | cons head rest ih =>
    intro f low same accepted
    simp only [run] at accepted
    cases hstep : step s head with
    | none => simp [hstep] at accepted
    | some next =>
      simp only [hstep, Option.bind_some] at accepted
      apply ih next f _ _ accepted
      · exact Nat.le_trans low (step_generation_monotone hstep).1
      · intro atG
        cases head with
        | acquire k' g' =>
          rcases step_acquire_some.mp hstep with ⟨_, _, _, _, rfl⟩ | ⟨_, hg, rfl⟩
          · exact same atG
          · simp only at atG
            omega
        | use g' =>
          obtain ⟨_, _, rfl⟩ := step_use_some.mp hstep
          exact same atG
        | release k' g' =>
          obtain ⟨_, _, _, ⟨_, _, rfl⟩ | ⟨_, rfl⟩⟩ := step_release_some.mp hstep
          · exact same atG
          · exact same atG

/-- The kind recorded for a generation at its acquisition is the kind every
later accepted release of that generation names. -/
theorem release_kind_is_acquisition_kind {slot acquired final : Slot} {kind released : Kind}
    {g : Nat} {pre post : List Event}
    (acquire : step slot (.acquire kind g) = some acquired)
    (fresh : slot.live = false)
    (accepted : run acquired (pre ++ Event.release released g :: post) = some final) :
    released = kind := by
  have acquiredKind : acquired.kind = kind ∧ acquired.generation = g := by
    rcases step_acquire_some.mp acquire with ⟨hl, _⟩ | ⟨_, _, rfl⟩
    · rw [fresh] at hl
      cases hl
    · exact ⟨rfl, rfl⟩
  rw [run_append] at accepted
  cases hpre : run acquired pre with
  | none => simp [hpre] at accepted
  | some mid =>
    simp only [hpre, Option.bind_some, run] at accepted
    have atMid := kind_stable_at_generation acquired pre mid (Nat.le_of_eq acquiredKind.2.symm)
      (fun _ => acquiredKind.1) hpre
    cases hrel : step mid (.release released g) with
    | none => simp [hrel] at accepted
    | some _ =>
      have facts := release_matches_live_kind hrel
      rw [← facts.2.2]
      exact atMid.2 facts.2.1

/-- **No use after release** and **release at most once.** After a release
that ends generation `g`, no accepted continuation uses or releases `g`. -/
theorem nothing_names_a_released_generation {slot next final : Slot} {kind : Kind} {g : Nat}
    {post : List Event}
    (release : step slot (.release kind g) = some next) (ended : next.live = false)
    (accepted : run next post = some final) :
    Event.use g ∉ post ∧ (∀ k : Kind, Event.release k g ∉ post) := by
  have sameGeneration := release_keeps_generation release
  have newer := run_events_not_older accepted
  constructor
  · intro member
    have := (newer _ member).2 ended
    simp [Event.generation, sameGeneration] at this
  · intro k member
    have := (newer _ member).2 ended
    simp [Event.generation, sameGeneration] at this

/-- **Each generation is released at most once** (single-owner kinds): an
accepted trace never contains two releases of one generation. -/
theorem single_owner_generation_released_at_most_once
    {slot final : Slot} {kind kind' : Kind} {g : Nat} {pre mid post : List Event}
    (single : kind.counted = false) :
    run slot (pre ++ Event.release kind g :: (mid ++ Event.release kind' g :: post)) ≠ some final := by
  intro accepted
  rw [run_append] at accepted
  cases hpre : run slot pre with
  | none => simp [hpre] at accepted
  | some before =>
    simp only [hpre, Option.bind_some, run] at accepted
    cases hrel : step before (.release kind g) with
    | none => simp [hrel] at accepted
    | some after =>
      simp only [hrel, Option.bind_some] at accepted
      have ended := (single_owner_release_is_terminal single hrel).1
      have absent := (nothing_names_a_released_generation hrel ended accepted).2 kind'
      exact absent (List.mem_append_right _ List.mem_cons_self)

/-- **No use after release** (single-owner kinds): an accepted trace never
uses a generation after releasing it. -/
theorem single_owner_generation_not_used_after_release
    {slot final : Slot} {kind : Kind} {g : Nat} {pre mid post : List Event}
    (single : kind.counted = false) :
    run slot (pre ++ Event.release kind g :: (mid ++ Event.use g :: post)) ≠ some final := by
  intro accepted
  rw [run_append] at accepted
  cases hpre : run slot pre with
  | none => simp [hpre] at accepted
  | some before =>
    simp only [hpre, Option.bind_some, run] at accepted
    cases hrel : step before (.release kind g) with
    | none => simp [hrel] at accepted
    | some after =>
      simp only [hrel, Option.bind_some] at accepted
      have ended := (single_owner_release_is_terminal single hrel).1
      have absent := (nothing_names_a_released_generation hrel ended accepted).1
      exact absent (List.mem_append_right _ List.mem_cons_self)

/-- A counted (dl) generation's references are released one at a time; the
generation stays live until its last reference is released. -/
theorem counted_release_keeps_generation_until_last {slot next : Slot} {g : Nat}
    (accepted : step slot (.release .dlHandle g) = some next) (more : 1 < slot.refs) :
    next.live = true ∧ next.generation = g ∧ next.refs = slot.refs - 1 := by
  obtain ⟨hl, hg, _, ⟨_, _, rfl⟩ | ⟨hr, _⟩⟩ := step_release_some.mp accepted
  · exact ⟨hl, hg.symm, rfl⟩
  · exact absurd ⟨rfl, more⟩ hr

/-! ### Multi-identity traces and the leak set at exit -/

abbrev State := Nat → Slot

def empty : State := fun _ => {}

def update (state : State) (identity : Nat) (slot : Slot) : State :=
  fun other => if other = identity then slot else state other

def stepAll (state : State) : Nat × Event → Option State
  | (identity, e) => (step (state identity) e).map (update state identity)

def runAll (state : State) : List (Nat × Event) → Option State
  | [] => some state
  | head :: rest => (stepAll state head).bind (fun next => runAll next rest)

/-- The events a trace applies to one identity, in order. -/
def project (identity : Nat) (trace : List (Nat × Event)) : List Event :=
  (trace.filter (fun entry => entry.1 = identity)).map Prod.snd

/-- A multi-identity run replays each identity independently. -/
theorem runAll_projects {state final : State} {trace : List (Nat × Event)}
    (accepted : runAll state trace = some final) (identity : Nat) :
    run (state identity) (project identity trace) = some (final identity) := by
  induction trace generalizing state with
  | nil =>
    simp only [runAll, Option.some.injEq] at accepted
    subst accepted
    simp [project, run]
  | cons head rest ih =>
    obtain ⟨owner, e⟩ := head
    simp only [runAll, stepAll] at accepted
    cases hstep : step (state owner) e with
    | none => simp [hstep] at accepted
    | some next =>
      simp only [hstep, Option.map_some, Option.bind_some] at accepted
      have tail := ih accepted
      by_cases same : owner = identity
      · subst same
        have updated : update state owner next owner = next := by simp [update]
        rw [updated] at tail
        simp [project, run, hstep]
        simpa [project] using tail
      · have unchanged : update state owner next identity = state identity := by
          simp [update, Ne.symm same]
        rw [unchanged] at tail
        simpa [project, List.filter_cons, same] using tail

/-- A generation is leaked when it is live at exit. -/
def leaked (final : State) (identity generation : Nat) : Prop :=
  (final identity).live = true ∧ (final identity).generation = generation

/-- The leak set is a function of the exit state: one generation per identity. -/
theorem leak_generation_unique {final : State} {identity g g' : Nat}
    (first : leaked final identity g) (second : leaked final identity g') : g = g' :=
  first.2.symm.trans second.2

/-- A live slot at the end of an accepted run was either live at its start
at the same generation, or acquired at that generation in the events. -/
theorem live_generation_was_acquired {slot final : Slot} {events : List Event}
    (accepted : run slot events = some final) (live : final.live = true) :
    (slot.live = true ∧ final.generation = slot.generation) ∨
      ∃ kind, Event.acquire kind final.generation ∈ events := by
  induction events generalizing slot with
  | nil =>
    simp only [run, Option.some.injEq] at accepted
    subst accepted
    exact Or.inl ⟨live, rfl⟩
  | cons head rest ih =>
    simp only [run] at accepted
    cases hstep : step slot head with
    | none => simp [hstep] at accepted
    | some next =>
      simp only [hstep, Option.bind_some] at accepted
      rcases ih accepted with ⟨nextLive, same⟩ | ⟨kind, member⟩
      · cases head with
        | acquire k g =>
          rcases step_acquire_some.mp hstep with ⟨hl, _, _, _, rfl⟩ | ⟨_, _, rfl⟩
          · exact Or.inl ⟨hl, same⟩
          · exact Or.inr ⟨k, by simp [same]⟩
        | use g =>
          obtain ⟨hl, _, rfl⟩ := step_use_some.mp hstep
          exact Or.inl ⟨hl, same⟩
        | release k g =>
          obtain ⟨hl, _, _, ⟨_, _, rfl⟩ | ⟨_, rfl⟩⟩ := step_release_some.mp hstep
          · exact Or.inl ⟨hl, same⟩
          · simp at nextLive
      · exact Or.inr ⟨kind, List.mem_cons_of_mem _ member⟩

/-- **The leak set at exit is well defined.** From the empty start state
(the start record acquires inherited descriptors), each identity has at most
one leaked generation (`leak_generation_unique`) and every leaked generation
was acquired by an event of the trace for that identity. -/
theorem leaked_generation_has_origin {final : State} {trace : List (Nat × Event)}
    {identity g : Nat} (accepted : runAll empty trace = some final)
    (leak : leaked final identity g) :
    ∃ kind, (identity, Event.acquire kind g) ∈ trace := by
  have projected := runAll_projects accepted identity
  rcases live_generation_was_acquired projected leak.1 with ⟨dead, _⟩ | ⟨kind, member⟩
  · simp [empty] at dead
  · refine ⟨kind, ?_⟩
    rw [leak.2] at member
    simp only [project, List.mem_map, List.mem_filter, decide_eq_true_eq] at member
    obtain ⟨⟨owner, e⟩, ⟨inTrace, same⟩, eq⟩ := member
    simp only at same eq
    subst same eq
    exact inTrace

/-! ### Executable anchors (matched by Rust unit tests of the same shapes) -/

theorem clean_fd_lifecycle_accepted :
    run {} [.acquire .fd 1, .use 1, .release .fd 1, .acquire .fd 2, .release .fd 2] =
      some { kind := .fd, generation := 2, live := false, refs := 0 } := by
  decide

theorem double_close_rejected :
    run {} [.acquire .fd 1, .release .fd 1, .release .fd 1] = none := by
  decide

theorem use_after_close_rejected :
    run {} [.acquire .fd 1, .release .fd 1, .use 1] = none := by
  decide

theorem stream_released_as_directory_rejected :
    run {} [.acquire .file 1, .release .dir 1] = none := by
  decide

theorem counted_dl_references_accepted :
    run {} [.acquire .dlHandle 1, .acquire .dlHandle 1, .release .dlHandle 1, .release .dlHandle 1] =
      some { kind := .dlHandle, generation := 1, live := false, refs := 0 } := by
  decide

theorem live_identity_reacquisition_rejected :
    run {} [.acquire .fd 1, .acquire .fd 2] = none := by
  decide

/-! ### Phase B: binding recorded executions to a declared contract

`qcore_contract::descriptor_contract_binding` checks each recorded execution
of a declared callee against the callee's loaded contract. The owned-result
rule is `OwnedResult.consistent`: a failing execution returns the kind's
failure value (`-1` for the integer descriptor kind, NULL for a pointer
kind), is admitted only if the contract may fail (`sentinel`/`nullable`), and
acquires nothing; any other execution acquires exactly the result, of the
declared kind. A release's caller precondition (a live identity of the
declared kind) is not a contract fact: breaking it is exactly a rejected
lifecycle step. Correspondence to the Rust binding is by review (rule 28). -/

structure OwnedResult where
  kind : Kind
  mayFail : Bool
  failure : Int
  deriving DecidableEq, Repr

structure Execution where
  result : Int
  acquired : List (Kind × Int)
  deriving DecidableEq, Repr

def OwnedResult.consistent (c : OwnedResult) (e : Execution) : Bool :=
  if e.result = c.failure then c.mayFail && e.acquired.isEmpty
  else decide (e.acquired = [(c.kind, e.result)])

theorem consistent_failure_acquires_nothing {c : OwnedResult} {e : Execution}
    (bound : c.consistent e = true) (failed : e.result = c.failure) :
    c.mayFail = true ∧ e.acquired = [] := by
  simp only [OwnedResult.consistent, failed, ↓reduceIte, Bool.and_eq_true,
    List.isEmpty_iff] at bound
  exact bound

theorem consistent_success_acquires_the_declared_result {c : OwnedResult} {e : Execution}
    (bound : c.consistent e = true) (succeeded : e.result ≠ c.failure) :
    e.acquired = [(c.kind, e.result)] := by
  simp only [OwnedResult.consistent, succeeded, ↓reduceIte, decide_eq_true_eq] at bound
  exact bound

/-- A contract that may not fail is contradicted by any failing execution. -/
theorem never_failing_contract_is_contradicted_by_a_failure {c : OwnedResult} {e : Execution}
    (never : c.mayFail = false) (failed : e.result = c.failure) :
    c.consistent e = false := by
  simp [OwnedResult.consistent, failed, never]

/-- The acquisition a bound success records enters the lifecycle model with
the declared kind. -/
theorem bound_acquisition_has_the_declared_kind {c : OwnedResult} {slot next : Slot} {g : Nat}
    (fresh : slot.live = false) (accepted : step slot (.acquire c.kind g) = some next) :
    next.kind = c.kind ∧ next.live = true := by
  rcases step_acquire_some.mp accepted with ⟨hl, _⟩ | ⟨_, _, rfl⟩
  · rw [fresh] at hl
    cases hl
  · exact ⟨rfl, rfl⟩

/-- A caller that releases an identity that is not live breaks the release
precondition; the lifecycle model rejects that step. It is a caller fact
(for example a second `close`), not a contradiction of the contract. -/
theorem release_of_a_dead_identity_is_rejected {slot : Slot} {k : Kind} {g : Nat}
    (dead : slot.live = false) : step slot (.release k g) = none := by
  simp [step, dead]

/-- A release of another kind than the live generation's is rejected. -/
theorem release_of_another_kind_is_rejected {slot : Slot} {k : Kind} {g : Nat}
    (other : slot.kind ≠ k) : step slot (.release k g) = none := by
  simp [step, other]

/-- The `ctr_random_get` shape: under a descriptor contract (failure `-1`), a
bound success that returns `0` acquires descriptor `0`, which a positive
guard (`fd > 0`) does not select. -/
theorem zero_descriptor_is_acquired_and_missed_by_a_positive_guard {c : OwnedResult}
    {acquired : List (Kind × Int)} (descriptor : c.failure = -1)
    (bound : c.consistent ⟨0, acquired⟩ = true) :
    acquired = [(c.kind, 0)] ∧ ¬ ((0 : Int) > 0) := by
  refine ⟨?_, by decide⟩
  have : (0 : Int) ≠ c.failure := by rw [descriptor]; decide
  exact consistent_success_acquires_the_declared_result bound this

theorem open_failure_binding_anchor :
    ({ kind := .fd, mayFail := true, failure := -1 } : OwnedResult).consistent ⟨-1, []⟩ = true := by
  decide

theorem open_success_binding_anchor :
    ({ kind := .fd, mayFail := true, failure := -1 } : OwnedResult).consistent
      ⟨3, [(.fd, 3)]⟩ = true := by
  decide

theorem wrong_kind_success_binding_anchor :
    ({ kind := .file, mayFail := true, failure := 0 } : OwnedResult).consistent
      ⟨4096, [(.dir, 4096)]⟩ = false := by
  decide

end QCore.DescriptorNativeObservation

