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
# The released list element and its summary (card LIST-ENTRY-RELEASE-IDENTITY)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_element_release.rs` (gateway rule 43),
with its hooks in `legacy_container_publication.rs` and the W4 gate.

**Part 1, the held object at an element destroyer.** `Obj` is the abstract
state of one object the caller's path tracks; `Concrete` its real state: the
list links (`cleared` once `mk_list_del` cleared them, `false` again after
a link) and whether it was freed. An element destroyer unlinks and then frees
(`concreteDestroy`): the unlink is defined only on links that are not
cleared, the free only on an object not freed. `destroyStep` is the rule:
a held object (live, unpublished, never unlinked) and a published one are
destroyed; an object a path already unlinked is refused; a released one is a
violation. `the_destroy_rule_is_sound` shows that every admitted destroy is
defined in every concrete state the abstract state describes and lands in a
state the result describes.

**Part 2, the released-element summary.** A body whose element custody is
rooted at formals states `ElementsReleased` at exactly those formals
(`required`), and is withheld when a root is anything else. At a call the
caller forgets every object it published (`apply`): the callee may have
released any object its container held, and the only tracked objects a
container the actual reaches may hold are the caller's published ones
(P-LERI-CALLER-PUBLISHED). `forgetting_covers_every_release` is the
soundness statement: whatever subset of the published objects the callee
released, none of them is still trusted afterwards, so a later unlink,
release or use of one refuses (`a_forgotten_object_is_refused`). The gate
accepts exactly the required clauses (`the_gate_is_exact`).

## What this is not

A model of the decision, as the gateway's other Lean modules are. That the
lexical reader recognises element bindings and container roots, that the
Rust gate and attachment compute `required` over the CFG's operands, and the
correspondence with the compiled Rust are review premises (trust boundary
28), pinned by `crates/qcore-c-gateway/tests/element_release_summary.rs`.
P-CRR-MEMBER, P-CRR-LINKED and P-LERI-CALLER-PUBLISHED are stated, not
proved.
-/

namespace QCore.ElementReleaseSummary

-- Part 1: the held object at an element destroyer.

/-- The abstract state of one tracked object on the caller's path. -/
inductive Obj where
  /-- Live, unpublished, unlinked on no arriving path. -/
  | held
  /-- Live after this body unlinked it (a restored publication, a foreign
  element, a destroyer's held object). -/
  | heldUnlinked
  /-- Linked by this body into a container that outlives the call. -/
  | published
  | released
  deriving DecidableEq, Repr

/-- The object's real state: its links cleared by an unlink, and freed. -/
structure Concrete where
  cleared : Bool
  freed : Bool
  deriving DecidableEq, Repr

/-- The concrete states an abstract state describes. -/
def describes : Obj → Concrete → Prop
  | .held, c => c.cleared = false ∧ c.freed = false
  | .published, c => c.cleared = false ∧ c.freed = false
  | .heldUnlinked, c => c.cleared = true ∧ c.freed = false
  | .released, c => c.freed = true

/-- `mk_list_del` and then the free: the unlink is defined only on links an
earlier unlink did not clear, the free only on an object not freed. -/
def concreteDestroy (c : Concrete) : Option Concrete :=
  if c.cleared = false ∧ c.freed = false then some { cleared := true, freed := true } else none

/-- A refusal (`Unknown`) or a violation candidate, or the next state. -/
inductive Outcome where
  | ok (obj : Obj)
  | refused
  | violation
  deriving DecidableEq, Repr

/-- Rule 34 with rule 43's held arm: an element destroyer's step. -/
def destroyStep : Obj → Outcome
  | .held => .ok .released
  | .published => .ok .released
  | .heldUnlinked => .refused
  | .released => .violation

/-- Rule 34 before rule 43: the held arm refused. -/
def destroyStepBefore : Obj → Outcome
  | .held => .refused
  | o => destroyStep o

theorem the_destroy_rule_is_sound (o o' : Obj) (c : Concrete) (hd : describes o c)
    (hs : destroyStep o = .ok o') :
    ∃ c', concreteDestroy c = some c' ∧ describes o' c' := by
  cases o <;> simp [destroyStep] at hs <;> subst hs <;>
    simp [describes] at hd <;> simp [concreteDestroy, hd, describes]

theorem a_held_object_is_destroyed : destroyStep .held = .ok .released := rfl

theorem an_unlinked_object_is_refused : destroyStep .heldUnlinked = .refused := rfl

/-- The refusal is necessary: an object whose links an unlink cleared has no
defined destroy. -/
theorem an_unlinked_object_has_no_defined_destroy (c : Concrete)
    (hd : describes .heldUnlinked c) : concreteDestroy c = none := by
  simp [describes] at hd
  simp [concreteDestroy, hd]

theorem a_destroyed_object_cannot_be_released_again (o : Obj) (h : destroyStep o = .ok .released) :
    destroyStep .released = .violation := by
  cases o <;> simp [destroyStep] at h <;> rfl

/-- Rule 43 only adds the held arm: every other decision is rule 34's. -/
theorem the_held_arm_is_the_only_change (o : Obj) (h : o ≠ .held) :
    destroyStep o = destroyStepBefore o := by
  cases o <;> simp_all [destroyStepBefore]

/-- Rule 43 never turns a decision of rule 34 into another decision: what was
admitted stays admitted to the same state, and only a refusal is replaced. -/
theorem the_held_arm_only_replaces_a_refusal (o : Obj) :
    destroyStepBefore o = destroyStep o ∨ destroyStepBefore o = .refused := by
  cases o <;> simp [destroyStepBefore]

-- Part 2: the released-element summary.

/-- Where one element custody of a body is taken from. -/
inductive Root where
  /-- A container rooted at the formal in this position. -/
  | formal (position : Nat)
  /-- A static object, which no clause names. -/
  | staticObject
  /-- A local, a callee's result, an element: anything else. -/
  | other
  deriving DecidableEq, Repr

/-- The `ElementsReleased` positions a body's element custody requires, or
`none` when the summary is withheld. -/
def required : List Root → Option (List Nat)
  | [] => some []
  | .formal p :: rest => (required rest).map (p :: ·)
  | _ :: _ => none

/-- The W4 gate's element condition: a body that takes element custody
states exactly the required positions (as a set); a body that takes none
states none. -/
def gate (roots : List Root) (stated : List Nat) : Prop :=
  match roots with
  | [] => stated = []
  | _ :: _ => ∃ req, required roots = some req ∧ ∀ p, p ∈ req ↔ p ∈ stated

theorem a_formal_root_is_required (roots : List Root) (req : List Nat) (p : Nat)
    (h : required roots = some req) (hp : Root.formal p ∈ roots) : p ∈ req := by
  induction roots generalizing req with
  | nil => simp at hp
  | cons r rest ih =>
    cases r with
    | formal q =>
      simp [required] at h
      obtain ⟨tail, ht, rfl⟩ := h
      simp at hp
      rcases hp with hp | hp
      · simp [hp]
      · exact List.mem_cons_of_mem _ (ih tail ht hp)
    | staticObject => simp [required] at h
    | other => simp [required] at h

theorem only_formal_roots_are_required (roots : List Root) (req : List Nat) (p : Nat)
    (h : required roots = some req) (hp : p ∈ req) : Root.formal p ∈ roots := by
  induction roots generalizing req with
  | nil => simp [required] at h; subst h; simp at hp
  | cons r rest ih =>
    cases r with
    | formal q =>
      simp [required] at h
      obtain ⟨tail, ht, rfl⟩ := h
      simp at hp
      rcases hp with hp | hp
      · simp [hp]
      · exact List.mem_cons_of_mem _ (ih tail ht hp)
    | staticObject => simp [required] at h
    | other => simp [required] at h

theorem a_static_root_withholds_the_summary (roots : List Root) (h : Root.staticObject ∈ roots) :
    required roots = none := by
  induction roots with
  | nil => simp at h
  | cons r rest ih =>
    cases r with
    | formal q =>
      simp at h
      simp [required, ih h]
    | staticObject => rfl
    | other => rfl

theorem an_other_root_withholds_the_summary (roots : List Root) (h : Root.other ∈ roots) :
    required roots = none := by
  induction roots with
  | nil => simp at h
  | cons r rest ih =>
    cases r with
    | formal q =>
      simp at h
      simp [required, ih h]
    | staticObject => rfl
    | other => rfl

/-- The gate is exact: it accepts a body's statement precisely when the
statement names every root formal and nothing else. -/
theorem the_gate_is_exact (r : Root) (rest : List Root) (stated : List Nat) :
    gate (r :: rest) stated ↔
      ∃ req, required (r :: rest) = some req ∧ ∀ p, p ∈ stated ↔ Root.formal p ∈ r :: rest := by
  constructor
  · rintro ⟨req, h, hs⟩
    refine ⟨req, h, fun p => ?_⟩
    constructor
    · intro hp
      exact only_formal_roots_are_required _ req p h ((hs p).mpr hp)
    · intro hp
      exact (hs p).mp (a_formal_root_is_required _ req p h hp)
  · rintro ⟨req, h, hs⟩
    refine ⟨req, h, fun p => ?_⟩
    constructor
    · intro hp
      exact (hs p).mpr (only_formal_roots_are_required _ req p h hp)
    · intro hp
      exact a_formal_root_is_required _ req p h ((hs p).mp hp)

theorem a_body_without_element_custody_states_nothing (stated : List Nat) :
    gate [] stated ↔ stated = [] := Iff.rfl

theorem a_withheld_body_never_passes_the_gate (r : Root) (rest : List Root) (stated : List Nat)
    (h : required (r :: rest) = none) : ¬ gate (r :: rest) stated := by
  rintro ⟨req, hr, _⟩
  rw [h] at hr
  cases hr

/-- A caller's tracked objects: those it published, and those it withdrew. -/
structure Caller where
  published : List Nat
  withdrawn : List Nat
  deriving DecidableEq, Repr

/-- The clause applied at a call: every published object is withdrawn. -/
def apply (s : Caller) : Caller :=
  { published := [], withdrawn := s.withdrawn ++ s.published }

/-- Whether a later unlink, release or use of `o` is admitted: only an
object still published and never withdrawn. -/
def trusted (s : Caller) (o : Nat) : Prop := o ∈ s.published ∧ o ∉ s.withdrawn

/-- Soundness of the summary at the caller: under P-LERI-CALLER-PUBLISHED the
objects the callee released among those the caller tracks lie in its
published list, and none of them is trusted after the call. -/
theorem forgetting_covers_every_release (s : Caller) (releasedByCallee : List Nat)
    (premise : ∀ o, o ∈ releasedByCallee → o ∈ s.published) :
    ∀ o, o ∈ releasedByCallee → ¬ trusted (apply s) o ∧ o ∈ (apply s).withdrawn := by
  intro o ho
  refine ⟨?_, ?_⟩
  · intro ht
    simp [trusted, apply] at ht
  · simp [apply, premise o ho]

theorem a_forgotten_object_is_refused (s : Caller) (o : Nat) : ¬ trusted (apply s) o := by
  simp [trusted, apply]

/-- The clause changes nothing for a caller that published nothing. -/
theorem a_caller_that_published_nothing_is_unchanged (w : List Nat) :
    apply { published := [], withdrawn := w } = { published := [], withdrawn := w } := by
  simp [apply]

/-- Why the clause must be applied: read as `Borrowed`, the call would leave a
released object trusted. -/
theorem the_borrowed_reading_would_trust_a_released_object (o : Nat) :
    let s : Caller := { published := [o], withdrawn := [] }
    trusted s o ∧ ¬ trusted (apply s) o := by
  simp [trusted, apply]

/-- A caller passes the clause on for its own formal and is withheld for any
other actual: the caller's summary is the same `required` reading. -/
theorem the_clause_is_carried_on (p : Nat) :
    required [.formal p] = some [p] ∧ required [.other] = none := by
  simp [required]

-- The Rust vectors (`ELEMENT_RELEASE_VECTORS`, checked by the unit tests of
-- `crates/qcore-c-gateway/src/legacy_element_release.rs`): each row is a
-- list of roots and the required positions, `none` when withheld.

def vectors : List (List Root × Option (List Nat)) :=
  [ ([], some []),
    ([.formal 0], some [0]),
    ([.formal 0, .formal 0], some [0, 0]),
    ([.formal 1, .formal 0], some [1, 0]),
    ([.formal 0, .staticObject], none),
    ([.other], none),
    ([.staticObject, .formal 0], none) ]

theorem vectors_hold : vectors.all (fun (roots, out) => required roots = out) = true := by
  decide

end QCore.ElementReleaseSummary
