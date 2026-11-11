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
# Storage duration and volatile (card CG15-STORAGE-DURATION-AND-VOLATILE)

The abstract rule implemented by `crates/qcore-c-gateway/src/storage_duration.rs`
(`FORM_TABLE`, `storage_of`, `static_address`, `volatile_trace`) and its
route-G adapter `legacy_storage_duration.rs` (`static_duration_declaration`,
`settle_volatile`, `decided_region`), gateway rule 11h.

Stated and proved here:

* **Lifetimes.** An object's storage duration gives its lifetime: a frame, its
  thread, or the program (C11 6.2.4). A static- or thread-duration object
  outlives every frame of the function that names it
  (`a_static_object_outlives_every_frame`,
  `a_thread_local_object_outlives_every_frame`), and exactly the automatic
  storage does not (`outlives_frame_iff`).
* **Static initialisation is no event.** A block-scope static-duration
  declaration runs nothing when control reaches it
  (`a_static_declaration_is_no_event`); lowering it as a store on every call,
  the pre-card reading, decides a later test by a value the object does not
  hold after the first call (`per_call_initialisation_misjudges_the_second_call`).
* **A pointer stored to a global escapes the frame.** After a tracked identity
  is stored into static storage the frame no longer holds it
  (`a_static_store_escapes_the_frame`), and with no registry contract the
  function is refused: never `Safe` and never a per-call leak
  (`a_static_store_is_refused_not_a_leak`).
* **A global read carries no custody.** A value read from static storage has
  no identity (`a_static_read_has_no_identity`), reading it changes no custody
  (`a_static_read_changes_no_custody`), and a custody-taking use of it refuses
  (`releasing_a_static_read_refuses`).
* **Volatile.** A `volatile` object's value at a read is the environment's,
  so a stored identity is not what a later read yields
  (`a_volatile_read_need_not_return_the_stored_identity`) and a value-derived
  fact misjudges a branch whose arms differ
  (`a_known_value_misjudges_a_volatile_branch`). A branch on a volatile read
  whose arms differ in custody is refused
  (`a_custody_decision_on_a_volatile_read_is_refused`); one whose arms agree is
  admitted and agrees with every environment
  (`a_neutral_volatile_branch_is_sound`).
* **The observable trace.** A `volatile` access is an event with a count and
  an order: every access is counted once (`trace_count`), a sequenced pair is
  ordered (`sequenced_accesses_are_ordered`), and an unsequenced pair is never
  ordered (`unsequenced_accesses_are_never_ordered`).
* **The table.** `formTable` is `FORM_TABLE` row for row; an unlisted form is
  refused (`unlisted_form_refused`); `volatile` is not `_Atomic`
  (`atomic_access_is_cg19s`).

Premises stated, not proved: faithful Clang extraction of storage classes,
qualifiers, node lines and the call and return nodes (trust boundary 28); the
C semantics above (C11 5.1.2.3p2, 6.2.4, 6.5p2, 6.7.9p4); and, in the Rust,
that a branch node's decided region (the nodes one successor reaches and
another does not) contains every node whose execution the branch decides.
`formTable` and `shapeVectors` bind the Rust table and the gateway's verdicts
on the pinned shapes mechanically (`tests/unit/storage_duration.rs`,
`tests/storage_duration_volatile.rs`).
-/

namespace QCore.StorageDuration

/-! ## Storage and lifetime -/

inductive Storage where
  | automatic
  | static
  | threadLocal
  deriving DecidableEq, Repr

/-- `Storage::outlives_frame`. -/
def Storage.outlivesFrame : Storage → Bool
  | .automatic => false
  | .static => true
  | .threadLocal => true

structure Interval where
  start : Nat
  stop : Nat
  deriving DecidableEq, Repr

/-- `a` lies within `b`. -/
def Interval.within (a b : Interval) : Prop :=
  b.start ≤ a.start ∧ a.stop ≤ b.stop

/-- One execution: a frame of the function inside its thread inside the
program. -/
structure Execution where
  program : Interval
  thread : Interval
  frame : Interval
  thread_in_program : thread.within program
  frame_in_thread : frame.within thread

/-- The lifetime of an object of each storage duration. -/
def lifetime (e : Execution) : Storage → Interval
  | .automatic => e.frame
  | .static => e.program
  | .threadLocal => e.thread

theorem a_static_object_outlives_every_frame (e : Execution) :
    e.frame.within (lifetime e .static) := by
  obtain ⟨h1, h2⟩ := e.frame_in_thread
  obtain ⟨h3, h4⟩ := e.thread_in_program
  exact ⟨Nat.le_trans h3 h1, Nat.le_trans h2 h4⟩

theorem a_thread_local_object_outlives_every_frame (e : Execution) :
    e.frame.within (lifetime e .threadLocal) :=
  e.frame_in_thread

theorem outlives_frame_iff (s : Storage) : s.outlivesFrame = true ↔ s ≠ .automatic := by
  cases s <;> decide

/-! ## Custody of one frame -/

inductive Verdict where
  | safe
  | leak
  | refuse
  deriving DecidableEq, Repr

/-- The frame's custody: the identities it owes, and whether a refusal was
reached. -/
structure Frame where
  live : List Nat
  refused : Bool
  deriving DecidableEq, Repr

/-- A value: a tracked identity, or a pointer or scalar with none. -/
inductive Val where
  | ident (id : Nat)
  | none
  deriving DecidableEq, Repr

inductive Event where
  | acquire (id : Nat)
  | release (v : Val)
  /-- A block-scope static-duration declaration. -/
  | staticDeclaration
  /-- A tracked identity stored into a static- or thread-duration object,
  with no registry contract. -/
  | storeStatic (id : Nat)
  /-- A read of a static- or thread-duration object. -/
  | readStatic
  deriving DecidableEq, Repr

/-- What a read of static storage yields: no identity. -/
def staticRead : Val := .none

def step (f : Frame) : Event → Frame
  | .acquire id => { f with live := id :: f.live }
  | .release (.ident id) =>
      if id ∈ f.live then { f with live := f.live.filter (· ≠ id) }
      else { f with refused := true }
  | .release .none => { f with refused := true }
  | .staticDeclaration => f
  | .storeStatic id =>
      if id ∈ f.live then { live := f.live.filter (· ≠ id), refused := true }
      else f
  | .readStatic => f

def run (f : Frame) : List Event → Frame
  | [] => f
  | e :: rest => run (step f e) rest

def verdict (f : Frame) : Verdict :=
  if f.refused then .refuse else if f.live = [] then .safe else .leak

theorem a_static_declaration_is_no_event (f : Frame) : step f .staticDeclaration = f := rfl

theorem refused_is_sticky (f : Frame) (e : Event) (h : f.refused = true) :
    (step f e).refused = true := by
  cases e with
  | acquire => simpa [step] using h
  | release v =>
      cases v with
      | ident id => by_cases hm : id ∈ f.live <;> simp [step, hm, h]
      | none => simp [step]
  | staticDeclaration => simpa [step] using h
  | storeStatic id => by_cases hm : id ∈ f.live <;> simp [step, hm, h]
  | readStatic => simpa [step] using h

theorem run_refused_is_sticky (f : Frame) (events : List Event) (h : f.refused = true) :
    (run f events).refused = true := by
  induction events generalizing f with
  | nil => simpa [run] using h
  | cons e rest ih => exact ih (step f e) (refused_is_sticky f e h)

theorem a_static_store_escapes_the_frame (f : Frame) (id : Nat) :
    id ∉ (step f (.storeStatic id)).live := by
  by_cases hm : id ∈ f.live <;> simp [step, hm]

theorem a_static_store_is_refused_not_a_leak (f : Frame) (id : Nat) (rest : List Event)
    (h : id ∈ f.live) :
    verdict (run (step f (.storeStatic id)) rest) = .refuse := by
  have hr : (step f (.storeStatic id)).refused = true := by simp [step, h]
  simp [verdict, run_refused_is_sticky _ rest hr]

theorem a_static_read_has_no_identity : staticRead = .none := rfl

theorem a_static_read_changes_no_custody (f : Frame) : step f .readStatic = f := rfl

theorem releasing_a_static_read_refuses (f : Frame) :
    (step f (.release staticRead)).refused = true := rfl

/-! ## The pre-card reading of a static initializer

`static void *cache = 0; if (cache) release(cache); cache = b;` over two calls.
The object keeps its value across calls; the pre-card lowering re-applied the
initializer on every call. -/

/-- Whether the call releases, given the value `cache` holds on entry. -/
def releasesOnEntry (cache : Val) : Bool :=
  match cache with
  | .ident _ => true
  | .none => false

/-- The value `cache` holds when the second call reaches the test: the first
call stored `b`. -/
def secondCallValue (b : Nat) : Val := .ident b

/-- The pre-card reading: the initializer (null) on every call. -/
def perCallValue : Val := .none

theorem per_call_initialisation_misjudges_the_second_call (b : Nat) :
    releasesOnEntry (secondCallValue b) ≠ releasesOnEntry perCallValue := by
  simp [releasesOnEntry, secondCallValue, perCallValue]

/-! ## Volatile

A `volatile` object's value at a read is chosen by the environment
(C11 5.1.2.3p2): a function from the read's index to a value. -/

abbrev Env := Nat → Val

/-- A read of a `volatile` object: the environment's value, whatever was
stored. -/
def volatileRead (env : Env) (index : Nat) (_stored : Val) : Val := env index

theorem a_volatile_read_need_not_return_the_stored_identity (id : Nat) :
    ∃ env : Env, volatileRead env 0 (.ident id) ≠ .ident id :=
  ⟨fun _ => .none, by simp [volatileRead]⟩

/-- A branch on a volatile read: the first arm's verdict when the value is
non-null, the second's otherwise. -/
def concreteBranch (arms : Verdict × Verdict) (v : Val) : Verdict :=
  match v with
  | .ident _ => arms.1
  | .none => arms.2

/-- A value-derived fact: the analysis assumes the value it last saw. -/
def refinedBranch (arms : Verdict × Verdict) (known : Val) : Verdict :=
  concreteBranch arms known

theorem a_known_value_misjudges_a_volatile_branch (arms : Verdict × Verdict)
    (h : arms.1 ≠ arms.2) (known : Val) :
    ∃ env : Env, concreteBranch arms (volatileRead env 0 known) ≠ refinedBranch arms known := by
  cases known with
  | ident id => exact ⟨fun _ => .none, by simpa [concreteBranch, refinedBranch, volatileRead] using h.symm⟩
  | none => exact ⟨fun _ => .ident 0, by simpa [concreteBranch, refinedBranch, volatileRead] using h⟩

/-- Rule 11h's branch: refused when the arms differ in custody, the arms'
common verdict otherwise. -/
def volatileBranch (arms : Verdict × Verdict) : Verdict :=
  if arms.1 = arms.2 then arms.1 else .refuse

theorem a_custody_decision_on_a_volatile_read_is_refused (arms : Verdict × Verdict)
    (h : arms.1 ≠ arms.2) : volatileBranch arms = .refuse := by
  simp [volatileBranch, h]

theorem a_neutral_volatile_branch_is_sound (arms : Verdict × Verdict) (h : arms.1 = arms.2)
    (env : Env) (index : Nat) (stored : Val) :
    concreteBranch arms (volatileRead env index stored) = volatileBranch arms := by
  cases hv : volatileRead env index stored <;> simp [concreteBranch, volatileBranch, h]

/-! ## The observable trace

An expression's `volatile` accesses, in one serialisation. `seq a b`: C
sequences every access of `a` before every access of `b` (a comma, `&&`,
`||`, `?:`, an assignment's store after its operands, statement order);
`unseq a b`: C leaves them unsequenced (operator operands, call actuals). -/

inductive Expr where
  | pure
  | access (object : Nat) (write : Bool)
  | seq (a b : Expr)
  | unseq (a b : Expr)
  deriving DecidableEq, Repr

def count : Expr → Nat
  | .pure => 0
  | .access _ _ => 1
  | .seq a b => count a + count b
  | .unseq a b => count a + count b

/-- The accesses in the serialisation, as objects. -/
def accesses : Expr → List Nat
  | .pure => []
  | .access object _ => [object]
  | .seq a b => accesses a ++ accesses b
  | .unseq a b => accesses a ++ accesses b

def shift (k : Nat) (pairs : List (Nat × Nat)) : List (Nat × Nat) :=
  pairs.map fun p => (p.1 + k, p.2 + k)

def cross (m n : Nat) : List (Nat × Nat) :=
  (List.range m).flatMap fun i => (List.range n).map fun j => (i, m + j)

/-- The ordered pairs of access indices C gives. -/
def before : Expr → List (Nat × Nat)
  | .pure => []
  | .access _ _ => []
  | .seq a b => before a ++ shift (count a) (before b) ++ cross (count a) (count b)
  | .unseq a b => before a ++ shift (count a) (before b)

theorem trace_count (e : Expr) : (accesses e).length = count e := by
  induction e with
  | pure => rfl
  | access => rfl
  | seq a b iha ihb => simp [accesses, count, iha, ihb]
  | unseq a b iha ihb => simp [accesses, count, iha, ihb]

theorem before_bounded (e : Expr) : ∀ p ∈ before e, p.1 < p.2 ∧ p.2 < count e := by
  induction e with
  | pure => simp [before]
  | access => simp [before]
  | seq a b iha ihb =>
      intro p hp
      simp only [before, List.mem_append] at hp
      rcases hp with (hp | hp) | hp
      · have := iha p hp; simp [count]; omega
      · simp only [shift, List.mem_map] at hp
        obtain ⟨q, hq, rfl⟩ := hp
        have := ihb q hq; simp [count]; omega
      · simp only [cross, List.mem_flatMap, List.mem_range, List.mem_map] at hp
        obtain ⟨i, hi, j, hj, rfl⟩ := hp
        simp [count]; omega
  | unseq a b iha ihb =>
      intro p hp
      simp only [before, List.mem_append] at hp
      rcases hp with hp | hp
      · have := iha p hp; simp [count]; omega
      · simp only [shift, List.mem_map] at hp
        obtain ⟨q, hq, rfl⟩ := hp
        have := ihb q hq; simp [count]; omega

theorem sequenced_accesses_are_ordered (a b : Expr) (i j : Nat) (hi : i < count a)
    (hj : j < count b) : (i, count a + j) ∈ before (.seq a b) := by
  simp only [before, List.mem_append]
  right
  simp only [cross, List.mem_flatMap, List.mem_range, List.mem_map]
  exact ⟨i, hi, j, hj, rfl⟩

theorem unsequenced_accesses_are_never_ordered (a b : Expr) (i j : Nat) (hi : i < count a)
    (hj : count a ≤ j) : (i, j) ∉ before (.unseq a b) ∧ (j, i) ∉ before (.unseq a b) := by
  constructor
  · intro hp
    simp only [before, List.mem_append] at hp
    rcases hp with hp | hp
    · have := before_bounded a _ hp; simp at this; omega
    · simp only [shift, List.mem_map] at hp
      obtain ⟨q, _, hq⟩ := hp
      simp only [Prod.mk.injEq] at hq
      omega
  · intro hp
    have := before_bounded (.unseq a b) _ hp
    simp at this
    omega

/-! ## The decision table -/

inductive Disposition where
  | noEvent
  | noIdentity
  | neutral
  | forgotten
  | unrefined
  | refused
  | owned (card : String)
  deriving DecidableEq, Repr

/-- `FORM_TABLE`. -/
def formTable : List (String × Disposition) := [
  ("static declaration", .noEvent),
  ("static read", .noIdentity),
  ("static address", .noIdentity),
  ("static scalar store", .neutral),
  ("static identity store", .refused),
  ("static read released", .refused),
  ("volatile neutral", .neutral),
  ("volatile binding", .refused),
  ("volatile value fact", .forgotten),
  ("volatile refinement", .unrefined),
  ("volatile decision", .refused),
  ("atomic access", .owned "CG19")
]

theorem forms_unique : (formTable.map (·.1)).Nodup := by decide

def dispositionOf (form : String) : Disposition :=
  match formTable.find? (·.1 == form) with
  | some (_, disposition) => disposition
  | none => .refused

theorem unlisted_form_refused (form : String) (h : form ∉ formTable.map (·.1)) :
    dispositionOf form = .refused := by
  unfold dispositionOf
  have : formTable.find? (·.1 == form) = none := by
    rw [List.find?_eq_none]
    intro x hx heq
    apply h
    simp only [beq_iff_eq] at heq
    exact List.mem_map.mpr ⟨x, hx, heq⟩
  rw [this]

theorem atomic_access_is_cg19s : dispositionOf "atomic access" = .owned "CG19" := by decide

theorem every_refused_row_is_a_custody_form :
    ∀ row ∈ formTable, row.2 = .refused →
      row.1 ∈ ["static identity store", "static read released", "volatile binding",
        "volatile decision"] := by
  decide

/-- The custody verdict class a shape whose only construct is `form` gets. -/
def expectedOf : Disposition → Verdict
  | .refused => .refuse
  | .owned _ => .refuse
  | _ => .safe

/-- The pinned shapes the gateway decides (`SHAPE_VECTORS`): the shape's name,
its form, and the gateway's verdict class. -/
def shapeVectors : List (String × String × Verdict) := [
  ("table-reader", "static declaration", .safe),
  ("thread-local-formatter", "static address", .safe),
  ("static-scalar-counter", "static scalar store", .safe),
  ("static-cache-allocation", "static identity store", .refuse),
  ("static-initializer-per-call", "static read released", .refuse),
  ("global-release", "static read released", .refuse),
  ("volatile-sum", "volatile neutral", .safe),
  ("volatile-spin-wait", "volatile neutral", .safe),
  ("volatile-pointer-binding", "volatile binding", .refuse),
  ("volatile-flag-known-zero", "volatile decision", .refuse),
  ("volatile-branch-release", "volatile decision", .refuse)
]

theorem shape_vectors_follow_the_table :
    ∀ v ∈ shapeVectors, v.2.2 = expectedOf (dispositionOf v.2.1) := by
  decide

end QCore.StorageDuration
