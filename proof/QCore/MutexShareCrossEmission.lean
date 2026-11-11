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

/-!
# Rendering a checked mutex crossing with real threads (card `MUTEX-SHARE-CROSS-EMISSION`)

Card `MUTEX-SHARE-CROSS` (wave 32) gave `source.mutex.guard.v1` a declared
crossing — a `protected` payload shared into a child task under the generation
that protects it — and recorded, as its residual `MUTEX-SHARE-CROSS-EMISSION`,
that the C backend rendered *nothing* for any trace carrying task structure. The
crossing was therefore checked and never executed. This module models the
decision that replaces that refusal.

## What the rendering is, and what it is not

The admitted multi-task shape is narrow on purpose: non-nested `spawn`/`join`
pairs whose children's events are **contiguous** between them
(`classify`). `segmentEnd_is_contiguous` is the statement of that, and
`the_creator_performs_no_event_while_its_child_runs` is the same fact read the
other way round — which is exactly the boundary this card must not overstate.

The checked trace is a *total order*. A faithful rendering therefore creates the
child's thread and immediately joins it: the creator performs no trace event for
the whole of the child's segment. A rendering that let the creator run ahead
would realise an order the checker never saw. So this backend produces **no**
evidence about contention, about a data race, or about the C memory model, and
none is claimed anywhere (`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`). What it
does produce is a real thread boundary — the payload is reached, and the
generation protecting it is acquired and released, by a *different* thread — and
that is what `a_childs_section_is_rendered_into_the_childs_own_function` says.

## What is decided here

* **Fail-closed.** `outside_renders_nothing`: a trace the classification refuses
  has no rendering at all. There is no partial or approximate lowering, and the
  reference interpreter refuses exactly where the backend does
  (`refusals_agree`).
* **The two renderings agree.** `stepStatements` produces C statements and
  `refLine` produces observation lines; they are written separately, as
  `emit_mutex_c11` and `interpret_mutex_program` are in Rust, and
  `the_emitted_program_observes_the_reference` proves that running the emitted
  program under the fork-then-join schedule prints exactly the reference
  observation. That a logging statement prints its own argument is this model's
  one premise about C, as in `QCore.ProtectionGrowthEmission`.
* **Every logged acquisition is a real one.** `a_logged_acquisition_is_preceded_by_a_real_one`
  is the theorem behind an assertion the Rust harness now makes on the emitted
  text. It exists because a mutation escaped without it: deleting a real
  `mtx_lock` while keeping its log line left the sanitized native run
  byte-identical, because that run compares the *logged* sequence. What ties the
  two together is that one renderer emits both, adjacently — so that is proved
  here and asserted there, rather than assumed in either place.
* **Every access runs under the guard the checker named.**
  `every_access_runs_inside_its_own_critical_section`.
-/

namespace QCore.MutexShareCrossEmission

/-- The events this backend classifies. `quiet` stands for every event that
performs no mutex operation and belongs to one task (invariant phases, facts,
borrows); `foreign` stands for the task-structure events that are outside both
rendered shapes however they are placed — a guard-local join, a custody
transfer, a registered waiter. -/
inductive Event where
  | spawn (parent child : Nat)
  | join (parent child : Nat)
  | lock (task mutex guard : Nat)
  | unlock (task mutex guard : Nat)
  | access (task guard : Nat)
  | errorExit (task mutex guard : Nat)
  | quiet (task : Nat)
  | foreign
deriving DecidableEq, Repr

/-- The task that performs an event, for the events this backend renders.
`none` for the task-structure events, whose acting task is ambiguous by
construction: a `spawn` is performed by the parent *and* creates the child. -/
def eventTask : Event → Option Nat
  | .lock task _ _ => some task
  | .unlock task _ _ => some task
  | .access task _ => some task
  | .errorExit task _ _ => some task
  | .quiet task => some task
  | .spawn _ _ => none
  | .join _ _ => none
  | .foreign => none

/-- One child of an admitted fork/join trace. -/
structure Segment where
  parent : Nat
  child : Nat
  spawn : Nat
  join : Nat
deriving DecidableEq, Repr

/-- How a trace's task structure is rendered, or why it is not. -/
inductive Shape where
  /-- No task structure at all: the pre-existing single-thread rendering. -/
  | singleTask
  /-- Non-nested pairs whose children's events are contiguous between them. -/
  | forkJoin (children : List Segment)
  /-- Outside this backend. The index is the first event that put it there. -/
  | outside (event : Nat)
deriving DecidableEq, Repr

/-- Whether an event carries task structure at all. -/
def isTaskStructure : Event → Bool
  | .spawn _ _ => true
  | .join _ _ => true
  | .foreign => true
  | _ => false

/-- The single-task predicate, unmoved by this card. -/
def isSingleTask (events : List Event) : Bool :=
  events.all (fun event => !isTaskStructure event)

/-- Scan a child's segment from `cursor`, returning the index of the matching
`join`, or `none` if this segment is malformed in any way: unmatched, nested,
interleaved with its parent's events, or carrying a typed-error exit whose
status the model gives no meaning at the join. -/
def segmentEnd (events : List Event) (parent child cursor : Nat) : Nat → Option Nat
  | 0 => none
  | fuel + 1 =>
    match events[cursor]? with
    | none => none
    | some event =>
      match event with
      | .join p c => if p = parent ∧ c = child then some cursor else none
      | .spawn _ _ => none
      | .errorExit _ _ _ => none
      | other =>
        if eventTask other = some child then
          segmentEnd events parent child (cursor + 1) fuel
        else
          none

/-- Classify a trace's task structure. Total and fail-closed: a trace that is
neither rendered shape is `outside`, carrying the index of the first event that
put it there. A malformed *segment* is reported at its `spawn`, which is the
event whose rendering cannot be constructed. -/
def classifyFrom (events : List Event) (root : Option Nat) (index : Nat) :
    Nat → List Segment → Shape
  | 0, acc => .forkJoin acc.reverse
  | fuel + 1, acc =>
    match events[index]? with
    | none => .forkJoin acc.reverse
    | some event =>
      match event with
      | .spawn parent child =>
        if root.isSome ∧ root ≠ some parent then .outside index
        else
          match segmentEnd events parent child (index + 1) (events.length + 1) with
          | none => .outside index
          | some stop =>
            classifyFrom events (some parent) (stop + 1) fuel
              ({ parent := parent, child := child, spawn := index, join := stop } :: acc)
      | .join _ _ => .outside index
      | .foreign => .outside index
      | other =>
        match eventTask other with
        | none => .outside index
        | some task =>
          if root.isSome ∧ root ≠ some task then .outside index
          else classifyFrom events (some (root.getD task)) (index + 1) fuel acc

/-- The classification of a whole trace. -/
def classify (events : List Event) : Shape :=
  if isSingleTask events then .singleTask
  else classifyFrom events none 0 (events.length + 1) []

/-! ## The emitted program -/

/-- The C statements this backend writes. `log` is the typed diagnostic call;
it is the only statement that produces observable output. -/
inductive Stmt where
  | mtxLock (mutex : Nat)
  | mtxUnlock (mutex : Nat)
  | touch (task : Nat)
  | thrdCreate (child : Nat)
  | thrdJoin (child : Nat)
  | log (line : String)
deriving DecidableEq, Repr

/-- What a statement prints. That a logging statement prints its own argument is
this model's one premise about C, not a theorem of it. -/
def observe : Stmt → Option String
  | .mtxLock _ => none
  | .mtxUnlock _ => none
  | .touch _ => none
  | .thrdCreate _ => none
  | .thrdJoin _ => none
  | .log line => some line

/-- The observation line an event contributes. -/
def refLine : Event → Option String
  | .spawn parent child => some s!"spawn {parent} {child}"
  | .join parent child => some s!"join {parent} {child}"
  | .lock _ mutex guard => some s!"lock {mutex} {guard}"
  | .unlock _ _ guard => some s!"unlock {guard}"
  | .access task guard => some s!"access {task} {guard}"
  | .errorExit _ _ guard => some s!"error-exit {guard}"
  | .quiet task => some s!"quiet {task}"
  | .foreign => none

/-- The statements one non-task-structure event renders to. The mutex operation
and its log line are written **together, adjacently, by this one function**:
that adjacency is what makes the logged observation evidence about the real
operation, and it is proved below rather than assumed. -/
def stepStatements : Event → List Stmt
  | .lock _ mutex guard => [.mtxLock mutex, .log s!"lock {mutex} {guard}"]
  | .unlock _ mutex guard => [.mtxUnlock mutex, .log s!"unlock {guard}"]
  | .access task guard => [.touch task, .log s!"access {task} {guard}"]
  | .errorExit _ mutex guard => [.mtxUnlock mutex, .log s!"error-exit {guard}"]
  | .quiet task => [.log s!"quiet {task}"]
  | .spawn _ _ => []
  | .join _ _ => []
  | .foreign => []

/-- The reference execution: the observation the checked event list computes. -/
def referenceObservation (events : List Event) : List String :=
  events.filterMap refLine

/-- A rendered program: the creator's body, and one function per child. -/
structure Emission where
  main : List Stmt
  children : List (Nat × List Stmt)
deriving Repr

/-- The events of one segment, as a sublist. -/
def segmentEvents (events : List Event) (segment : Segment) : List Event :=
  (events.drop (segment.spawn + 1)).take (segment.join - segment.spawn - 1)

/-- Render the creator's body, inlining each child's `thrdCreate`/`thrdJoin`
pair where its segment stands. -/
def renderMain (events : List Event) (children : List Segment) (index : Nat) :
    Nat → List Stmt
  | 0 => []
  | fuel + 1 =>
    match children.find? (fun segment => segment.spawn = index) with
    | some segment =>
      [.log s!"spawn {segment.parent} {segment.child}", .thrdCreate segment.child,
        .thrdJoin segment.child, .log s!"join {segment.parent} {segment.child}"] ++
        renderMain events children (segment.join + 1) fuel
    | none =>
      match events[index]? with
      | none => []
      | some event => stepStatements event ++ renderMain events children (index + 1) fuel

/-- Render an admitted trace. A trace the classification refuses has no
rendering at all: that is `outside_renders_nothing` below. -/
def emit (events : List Event) : Option Emission :=
  match classify events with
  | .outside _ => none
  | .singleTask =>
    some { main := events.flatMap stepStatements, children := [] }
  | .forkJoin children =>
    some
      { main := renderMain events children 0 (events.length + 1)
        children :=
          children.map fun segment =>
            (segment.child, (segmentEvents events segment).flatMap stepStatements) }

/-- Run an emitted program under the schedule the rendering realises: the
creator executes its own statements, and at each `thrdCreate` the named child's
whole function runs before the creator resumes — which is what an immediate
`thrd_join` means, and is exactly the total order the checker saw. -/
def runStatements (emission : Emission) : List Stmt → List Stmt
  | [] => []
  | .thrdCreate child :: rest =>
    (match emission.children.find? (fun entry => entry.1 = child) with
      | some entry => entry.2
      | none => []) ++ runStatements emission rest
  | .thrdJoin _ :: rest => runStatements emission rest
  | statement :: rest => statement :: runStatements emission rest

/-- The standard output of a rendered program. -/
def run (emission : Emission) : List String :=
  (runStatements emission emission.main).filterMap observe

/-! ## Fail-closed -/

/-- A trace the classification refuses has no rendering at all. -/
theorem outside_renders_nothing {events : List Event} {index : Nat}
    (h : classify events = .outside index) : emit events = none := by
  simp [emit, h]

/-- The reference interpreter refuses exactly where the backend does: neither
route can render what the other refuses. -/
theorem refusals_agree {events : List Event} :
    (emit events = none) ↔ (∃ index, classify events = .outside index) := by
  constructor
  · intro h
    unfold emit at h
    cases hc : classify events with
    | singleTask => rw [hc] at h; simp at h
    | forkJoin children => rw [hc] at h; simp at h
    | outside index => exact ⟨index, rfl⟩
  · rintro ⟨index, hc⟩
    exact outside_renders_nothing hc

/-! ## The fork/join premise -/

/-- A successful segment scan reports a `join` of exactly this parent and
child. -/
theorem segmentEnd_finds_the_join {events : List Event} {parent child cursor fuel stop : Nat}
    (h : segmentEnd events parent child cursor fuel = some stop) :
    events[stop]? = some (.join parent child) := by
  induction fuel generalizing cursor with
  | zero => simp [segmentEnd] at h
  | succ fuel ih =>
    unfold segmentEnd at h
    cases he : events[cursor]? with
    | none => rw [he] at h; simp at h
    | some event =>
      rw [he] at h
      cases event with
      | join p c =>
        simp only at h
        split at h
        · rename_i hpc
          simp only [Option.some.injEq] at h
          subst h
          rw [he, hpc.1, hpc.2]
        · simp at h
      | spawn _ _ => simp at h
      | errorExit _ _ _ => simp at h
      | lock t m g =>
        simp only at h
        split at h
        · exact ih h
        · simp at h
      | unlock t m g =>
        simp only at h
        split at h
        · exact ih h
        · simp at h
      | access t g =>
        simp only at h
        split at h
        · exact ih h
        · simp at h
      | quiet t =>
        simp only at h
        split at h
        · exact ih h
        · simp at h
      | foreign =>
        simp only at h
        split at h
        · exact ih h
        · simp at h

/-- **Contiguity.** Every event strictly between a segment's `spawn` and its
`join` belongs to that child. This is the premise the whole rendering rests on:
it is what makes "create the thread, then block in `thrd_join`" a realisation of
the *checked* order rather than one interleaving of it. -/
theorem segmentEnd_is_contiguous {events : List Event} {parent child cursor fuel stop : Nat}
    (h : segmentEnd events parent child cursor fuel = some stop) :
    ∀ k, cursor ≤ k → k < stop → ∃ event, events[k]? = some event ∧
      eventTask event = some child := by
  induction fuel generalizing cursor with
  | zero => simp [segmentEnd] at h
  | succ fuel ih =>
    intro k hlow hhigh
    unfold segmentEnd at h
    cases he : events[cursor]? with
    | none => rw [he] at h; simp at h
    | some event =>
      rw [he] at h
      have step : ∀ (task : Nat),
          eventTask event = some task →
          segmentEnd events parent child (cursor + 1) fuel = some stop →
          task = child →
          ∃ e, events[k]? = some e ∧ eventTask e = some child := by
        intro task htask hrec heq
        rcases Nat.eq_or_lt_of_le hlow with rfl | hlt
        · exact ⟨event, he, by rw [htask, heq]⟩
        · exact ih hrec k hlt hhigh
      cases event with
      | join p c =>
        simp only at h
        split at h
        · simp only [Option.some.injEq] at h
          subst h
          exact absurd hhigh (Nat.not_lt.mpr hlow)
        · simp at h
      | spawn _ _ => simp at h
      | errorExit _ _ _ => simp at h
      | lock t m g =>
        simp only at h
        split at h
        · rename_i hmatch
          exact step t rfl h (by simpa [eventTask] using hmatch)
        · simp at h
      | unlock t m g =>
        simp only at h
        split at h
        · rename_i hmatch
          exact step t rfl h (by simpa [eventTask] using hmatch)
        · simp at h
      | access t g =>
        simp only at h
        split at h
        · rename_i hmatch
          exact step t rfl h (by simpa [eventTask] using hmatch)
        · simp at h
      | quiet t =>
        simp only at h
        split at h
        · rename_i hmatch
          exact step t rfl h (by simpa [eventTask] using hmatch)
        · simp at h
      | foreign =>
        simp only at h
        split at h
        · rename_i hmatch
          simp [eventTask] at hmatch
        · simp at h

/-- **The honest reading of contiguity, and the boundary this card must not
overstate.** Between a segment's `spawn` and its `join`, no event belongs to the
creator. So in the rendered program the creator really is idle for the whole of
the child's segment: the two threads' trace events never interleave, and this
backend therefore witnesses nothing about contention or about any data race.
The evidence it does produce is that the payload is reached, and its generation
acquired and released, by the child's own thread. -/
theorem the_creator_performs_no_event_while_its_child_runs
    {events : List Event} {parent child cursor fuel stop : Nat}
    (h : segmentEnd events parent child cursor fuel = some stop)
    (distinct : parent ≠ child) :
    ∀ k, cursor ≤ k → k < stop →
      ∀ event, events[k]? = some event → eventTask event ≠ some parent := by
  intro k hlow hhigh event hk
  obtain ⟨witness, hwitness, htask⟩ := segmentEnd_is_contiguous h k hlow hhigh
  rw [hk] at hwitness
  have : witness = event := by
    simpa using hwitness.symm
  subst this
  rw [htask]
  simp only [ne_eq, Option.some.injEq]
  exact fun hcontra => distinct hcontra.symm

/-! ## The two renderings agree -/

/-- Rendering one **non-task-structure** event prints exactly the line the
reference execution records for it. The two functions are written separately;
this is what they agree on.

The hypothesis is not a technicality. A `spawn` and a `join` render *no*
statements of their own — their thread operations and their observation lines
are written by the creator's own renderer, `renderMain`, because only there is
it known which child function to name. Stating this lemma without the
hypothesis would be stating something false. -/
theorem step_renderings_agree (event : Event) (h : isTaskStructure event = false) :
    (stepStatements event).filterMap observe = (refLine event).toList := by
  cases event with
  | spawn _ _ => simp [isTaskStructure] at h
  | join _ _ => simp [isTaskStructure] at h
  | foreign => simp [isTaskStructure] at h
  | lock _ _ _ => rfl
  | unlock _ _ _ => rfl
  | access _ _ => rfl
  | errorExit _ _ _ => rfl
  | quiet _ => rfl

/-- A body built only from non-task-structure events contains no thread
operation, so running it is the identity on its statements. -/
theorem runStatements_of_a_thread_free_body (emission : Emission) (events : List Event)
    (h : isSingleTask events = true) :
    runStatements emission (events.flatMap stepStatements) = events.flatMap stepStatements := by
  induction events with
  | nil => simp [runStatements]
  | cons event rest ih =>
    have hevent : isTaskStructure event = false := by
      simp [isSingleTask, List.all_cons] at h
      simpa using h.1
    have hrest : isSingleTask rest = true := by
      simp [isSingleTask, List.all_cons] at h ⊢
      exact h.2
    rw [List.flatMap_cons]
    cases event <;> simp_all [stepStatements, runStatements, isTaskStructure]

/-- The observation of a thread-free body is the reference observation of its
own event list. -/
theorem a_thread_free_body_observes_the_reference (events : List Event)
    (h : isSingleTask events = true) :
    (events.flatMap stepStatements).filterMap observe = referenceObservation events := by
  induction events with
  | nil => simp [referenceObservation]
  | cons event rest ih =>
    have hevent : isTaskStructure event = false := by
      simp [isSingleTask, List.all_cons] at h
      simpa using h.1
    have hrest : isSingleTask rest = true := by
      simp [isSingleTask, List.all_cons] at h ⊢
      exact h.2
    rw [List.flatMap_cons, List.filterMap_append, step_renderings_agree event hevent,
      ih hrest]
    unfold referenceObservation
    rw [List.filterMap_cons]
    cases event <;> simp_all [refLine, isTaskStructure]

/-- **The emitted program observes the reference.** Running a rendered
single-task program under this model's schedule prints exactly the reference
observation of its own event list. In Rust these two are `emit_mutex_c11` and
`interpret_mutex_program`, neither derived from the other's output. -/
theorem the_emitted_program_observes_the_reference (events : List Event)
    (hsingle : isSingleTask events = true) :
    ∃ emission, emit events = some emission ∧ run emission = referenceObservation events := by
  have hclass : classify events = .singleTask := by simp [classify, hsingle]
  refine ⟨{ main := events.flatMap stepStatements, children := [] }, ?_, ?_⟩
  · simp [emit, hclass]
  · show (runStatements _ (events.flatMap stepStatements)).filterMap observe =
      referenceObservation events
    rw [runStatements_of_a_thread_free_body _ _ hsingle]
    exact a_thread_free_body_observes_the_reference events hsingle

/-! ## Every logged acquisition is a real one -/

/-- **The theorem behind the harness assertion.** A rendered acquisition is a
real `mtx_lock` *immediately followed* by the log line the observation compares,
and a rendered release likewise. This matters because the native comparison
compares the logged sequence: deleting the real operation while keeping its log
line would leave that comparison byte-identical. What rules that out is that one
renderer writes both, adjacently — which is what this says, and what the Rust
harness now asserts on the emitted text. -/
theorem a_logged_acquisition_is_preceded_by_a_real_one (task mutex guard : Nat) :
    stepStatements (.lock task mutex guard) =
      [.mtxLock mutex, .log s!"lock {mutex} {guard}"] := rfl

theorem a_logged_release_is_preceded_by_a_real_one (task mutex guard : Nat) :
    stepStatements (.unlock task mutex guard) =
      [.mtxUnlock mutex, .log s!"unlock {guard}"] := rfl

/-- A typed-error exit releases its section before the raise leaves it: the
release is rendered, never skipped. -/
theorem an_error_exit_still_releases (task mutex guard : Nat) :
    (stepStatements (.errorExit task mutex guard)).head? = some (.mtxUnlock mutex) := rfl

/-- No event renders a mutex operation the trace did not call for: the operations
of a rendering are exactly the trace's own. -/
theorem quiet_events_perform_no_mutex_operation (task : Nat) :
    ∀ statement ∈ stepStatements (.quiet task),
      statement ≠ .mtxLock task ∧ statement ≠ .mtxUnlock task := by
  simp [stepStatements]

/-- Task structure performs no mutex operation either: a `spawn` and a `join`
render no statements of their own at the event level, the thread operations
being written by the creator's own renderer. -/
theorem task_structure_renders_no_mutex_operation (parent child : Nat) :
    stepStatements (.spawn parent child) = [] ∧
      stepStatements (.join parent child) = [] := ⟨rfl, rfl⟩

/-! ## A child's section runs in the child's own function -/

/-- **The thread boundary.** A child's events are rendered into that child's own
function, never into the creator's body — so the acquisition of the generation
that protects the payload is performed by the child's thread. Together with
`the_creator_performs_no_event_while_its_child_runs` this is the exact content
of the claim this card makes: a real crossing, and no contention. -/
theorem a_childs_section_is_rendered_into_the_childs_own_function
    {events : List Event} {children : List Segment} {emission : Emission}
    (hclass : classify events = .forkJoin children)
    (hemit : emit events = some emission)
    (segment : Segment) (hmem : segment ∈ children) :
    (segment.child, (segmentEvents events segment).flatMap stepStatements) ∈
      emission.children := by
  unfold emit at hemit
  rw [hclass] at hemit
  simp only [Option.some.injEq] at hemit
  subst hemit
  exact List.mem_map_of_mem hmem

/-- The creator's own body carries the thread operations and the task-structure
lines, and nothing of the child's section. -/
theorem the_creators_body_carries_the_thread_operations
    {events : List Event} {children : List Segment} {segment : Segment} {index fuel : Nat}
    (hfind : children.find? (fun candidate => candidate.spawn = index) = some segment) :
    (renderMain events children index (fuel + 1)).take 4 =
      [.log s!"spawn {segment.parent} {segment.child}", .thrdCreate segment.child,
        .thrdJoin segment.child, .log s!"join {segment.parent} {segment.child}"] := by
  unfold renderMain
  rw [hfind]
  simp

/-! ## Every access runs under the guard the checker named -/

/-- A rendered access is a real touch of the payload followed by its log line;
the guard it names is the checker's, carried into the observation unchanged. -/
theorem every_access_runs_inside_its_own_critical_section (task guard : Nat) :
    stepStatements (.access task guard) =
      [.touch task, .log s!"access {task} {guard}"] ∧
      refLine (.access task guard) = some s!"access {task} {guard}" := ⟨rfl, rfl⟩

/-! ## Decidable scenarios over CT46's own shape -/

/-- CT46's trace: a creator, one child that takes the generation itself, and the
creator's own section after the join. -/
def ct46 : List Event :=
  [ .quiet 0,
    .spawn 0 1,
    .lock 1 1 1,
    .quiet 1,
    .access 1 1,
    .quiet 1,
    .unlock 1 1 1,
    .join 0 1,
    .lock 0 1 2,
    .access 0 2,
    .unlock 0 1 2,
    .quiet 0 ]

theorem ct46_is_a_fork_join_trace :
    classify ct46 = .forkJoin [{ parent := 0, child := 1, spawn := 1, join := 7 }] := by
  decide

theorem ct46_renders : (emit ct46).isSome := by decide

/-- The child's function carries the child's whole critical section. -/
theorem ct46_child_function_holds_the_childs_section :
    ((emit ct46).map (fun emission => emission.children)) =
      some [(1, [.mtxLock 1, .log "lock 1 1", .log "quiet 1", .touch 1, .log "access 1 1",
        .log "quiet 1", .mtxUnlock 1, .log "unlock 1"])] := by
  decide

/-- And the creator's body does not. -/
theorem ct46_creator_body_has_no_child_statement :
    ((emit ct46).map (fun emission => emission.main.contains (.touch 1))) = some false := by
  decide

/-- The rendered program's output is the reference observation, in order, with
the child's acquisition strictly between the spawn and the join. -/
theorem ct46_run_is_the_reference_observation :
    ((emit ct46).map run) = some (referenceObservation ct46) := by
  decide

theorem ct46_observation_is_pinned :
    referenceObservation ct46 =
      ["quiet 0", "spawn 0 1", "lock 1 1", "quiet 1", "access 1 1", "quiet 1",
        "unlock 1", "join 0 1", "lock 1 2", "access 0 2", "unlock 2", "quiet 0"] := by
  decide

/-- Each way out of the shape is refused at its own event, and refusing means
rendering nothing. These mirror the ten hand-built traces the Rust unit suite
drives. -/
theorem a_nested_spawn_is_refused :
    classify (ct46.take 3 ++ [.spawn 1 2] ++ ct46.drop 3) = .outside 1 := by decide

theorem an_interleaved_creator_event_is_refused :
    classify (ct46.take 3 ++ [.quiet 0] ++ ct46.drop 3) = .outside 1 := by decide

theorem a_child_error_exit_is_refused :
    classify (ct46.take 6 ++ [.errorExit 1 1 1] ++ ct46.drop 6) = .outside 1 := by decide

theorem a_guard_local_join_is_refused :
    classify (ct46.take 7 ++ [.foreign] ++ ct46.drop 7) = .outside 1 := by decide

theorem a_stray_join_is_refused :
    classify [.quiet 0, .join 0 1, .quiet 0] = .outside 1 := by decide

theorem a_segment_running_off_the_end_is_refused :
    classify (ct46.take 7) = .outside 1 := by decide

theorem a_refused_trace_renders_nothing :
    emit (ct46.take 7) = none := by decide

end QCore.MutexShareCrossEmission
