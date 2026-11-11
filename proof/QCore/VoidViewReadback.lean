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
# `void *` views and readbacks (card CG12-VOID-VIEW-READBACK, gateway rule 11j)

The abstract rule implemented by `crates/qcore-c-gateway/src/void_view.rs`
(`view_of`, `view_position`, `decide_readback`, `flow_binds`) and its hooks in
`typed_access_path.rs` (`Reader::path`, `Reader::statement`), `legacy_c.rs`
(`typed_path_statement`, the `IndirectScalarAccess` arm) and
`legacy_void_view.rs` (the static premises and `bound_read_resolves`).

Gap entry M05: a tracked pointer may be stored into a `void *` member (card
ALIAS-AGGREGATE), but reading it back was refused, because `void *` names no
pointee. Two rules supply the missing facts.

* **The view.** An explicit cast whose Clang kind is `BitCast`, from `void *`
  to an object pointer, keeps the pointer value (C11 6.3.2.3p1/p7) and gives
  the place its pointee type. It stands only where a pointer is read through,
  over the root or over a pointer loaded from a never-bound root.
* **The readback.** A pointer destination reading a `void *` member of an
  admitted store base is bound to the member's identity when four static
  premises hold (store base, the types agree, the owner and the member stay in
  the body) and the flow finds the field link an admitted store recorded,
  whole, with its child live. Otherwise it has no identity; where a known
  stored type differs from a view, the statement is refused (type punning).

The event model below is the flow's reading of the member between the store
and the readback: a store replaces the identity, a refused statement or
uncontracted call (`poison`) and a join where the store did not happen on
every path (`join`) leave no identity, and a released child is not read back.

## What this is not

A model of the decisions and of the flow arms they rely on. Faithful Clang
extraction of cast kinds, types and declaration ids is trust boundary 28;
C11 6.3.2.3p7 (the round trip through `void *`) and P-CG12-VIEW-EFFECTIVE-TYPE
(a view of a root or of a never-bound load reads the object at its effective
type) are stated premises. The Rust–Lean correspondence is bound by the shared
vectors (`tests/unit/void_view.rs`, `tests/cg12_void_view_readback.rs`).
-/

namespace QCore.VoidViewReadback

/-! ## 1. The view -/

inductive CastClass where
  | bitCast
  | noOp
  | other
  deriving DecidableEq, Repr

inductive OperandClass where
  | voidPointer
  | objectPointer
  | other
  deriving DecidableEq, Repr

inductive TargetClass where
  | object
  | voidPointer
  | function
  | other
  deriving DecidableEq, Repr

inductive ViewDecision where
  | view
  | kind
  | operand
  | target
  deriving DecidableEq, Repr

/-- `view_of` in `void_view.rs`. -/
def viewOf : CastClass → OperandClass → TargetClass → ViewDecision
  | .bitCast, .voidPointer, .object => .view
  | .bitCast, .voidPointer, _ => .target
  | .bitCast, _, _ => .operand
  | _, _, _ => .kind

/-- The Rust `VIEW_VECTORS`, as classes: operand type, cast kind and target
type (the Rust rows spell Clang's printing), and the decision. -/
def viewVectors : List (String × CastClass × OperandClass × TargetClass × ViewDecision) := [
  ("void * -> struct T *", .bitCast, .voidPointer, .object, .view),
  ("const void * -> const char **", .bitCast, .voidPointer, .object, .view),
  ("const void * -> const struct T *", .bitCast, .voidPointer, .object, .view),
  ("void * -> int *", .bitCast, .voidPointer, .object, .view),
  ("int * -> char *", .bitCast, .objectPointer, .object, .operand),
  ("struct T * -> struct A *", .bitCast, .objectPointer, .object, .operand),
  ("void * -> void (*)(void)", .bitCast, .voidPointer, .function, .target),
  ("void * -> const void * (NoOp)", .noOp, .voidPointer, .voidPointer, .kind),
  ("volatile void * -> int *", .bitCast, .other, .object, .operand),
  ("void * -> volatile int *", .bitCast, .voidPointer, .other, .target),
  ("long -> int * (IntegralToPointer)", .other, .other, .object, .kind)
]

theorem view_vectors_hold :
    viewVectors.all (fun (_, c, o, t, d) => viewOf c o t == d) = true := by
  decide

theorem view_vectors_count : viewVectors.length = 11 := by decide

/-- Only a bit-cast from `void *` to an object pointer is a view; every other
(unlisted) cast is refused. -/
theorem a_view_is_exactly_a_bit_cast_from_void_to_an_object_pointer
    (c : CastClass) (o : OperandClass) (t : TargetClass) :
    viewOf c o t = .view ↔ (c = .bitCast ∧ o = .voidPointer ∧ t = .object) := by
  cases c <;> cases o <;> cases t <;> simp [viewOf]

/-- A cast between two object pointers is a pun (M22), never a view. -/
theorem a_cast_between_object_pointers_is_never_a_view (c : CastClass) (t : TargetClass) :
    viewOf c .objectPointer t ≠ .view := by
  cases c <;> cases t <;> simp [viewOf]

/-- A cast to a function or `void` pointer names no object: never a view. -/
theorem a_non_object_target_is_never_a_view (c : CastClass) (o : OperandClass) :
    viewOf c o .function ≠ .view ∧ viewOf c o .voidPointer ≠ .view ∧ viewOf c o .other ≠ .view := by
  cases c <;> cases o <;> simp [viewOf]

inductive ViewPosition where
  | root
  | load
  deriving DecidableEq, Repr

/-- `view_position` in `void_view.rs`: whether the root is never bound, and
whether it carries a tracked identity at the access (the flow's half, through
the chain's untracked-only reading). -/
def positionAdmits : ViewPosition → Bool → Bool → Bool
  | _, _, true => false
  | .root, _, false => true
  | .load, neverBound, false => neverBound

/-- The Rust `POSITION_VECTORS`. -/
def positionVectors : List (String × ViewPosition × Bool × Bool × Bool) := [
  ("*(int *)a", .root, true, false, true),
  ("((struct T *)a)->x", .root, true, false, true),
  ("((char *)a)[1]", .root, true, false, true),
  ("((struct T *)o)->x", .root, false, false, true),
  ("((struct T *)o)->x", .root, false, true, false),
  ("*(int *)h", .root, false, true, false),
  ("*(int *)f->p", .load, true, false, true),
  ("((struct T *)s->v)->x", .load, false, true, false)
]

theorem position_vectors_hold :
    positionVectors.all (fun (_, p, n, t, a) => positionAdmits p n t == a) = true := by
  decide

/-- A view is never read over a tracked identity: its object has the type its
contract gives it, which the view does not check. -/
theorem a_view_over_a_tracked_identity_is_refused (p : ViewPosition) (n : Bool) :
    positionAdmits p n true = false := by
  cases p <;> rfl

/-- A view over a pointer loaded from a root the closure does not prove never
bound (a field link) is refused. -/
theorem a_view_over_a_link_is_refused (t : Bool) : positionAdmits .load false t = false := by
  cases t <;> rfl

/-- A view over an untracked root keeps the root's own obligation: it adds no
premise. -/
theorem a_view_over_an_untracked_root_adds_no_premise (n : Bool) :
    positionAdmits .root n false = true := rfl

/-! ## 2. The readback: static premises -/

inductive ReadbackDecision where
  | identity
  | noStoreBase
  | pun
  | ownerEscapes
  | memberEscapes
  deriving DecidableEq, Repr

/-- `decide_readback` in `void_view.rs`. -/
def decideReadback (storeBase typesAgree ownerEscapes memberEscapes : Bool) : ReadbackDecision :=
  if !storeBase then .noStoreBase
  else if !typesAgree then .pun
  else if ownerEscapes then .ownerEscapes
  else if memberEscapes then .memberEscapes
  else .identity

/-- The Rust `DECISION_VECTORS` (their bodies are run through the gateway). -/
def decisionVectors : List (String × Bool × Bool × Bool × Bool × ReadbackDecision) := [
  ("struct T *c = s->v", true, true, false, false, .identity),
  ("struct T *c = (struct T *) s->v", true, true, false, false, .identity),
  ("s->v = q_alloc(); char *c = s->v", true, true, false, false, .identity),
  ("struct A *c = (struct A *) s->v (stored struct T)", true, false, false, false, .pun),
  ("char *c = s->v (stored struct T)", true, false, false, false, .pun),
  ("q_observe(s) before the readback", true, true, true, false, .ownerEscapes),
  ("void **w = &s->v", true, true, false, true, .memberEscapes)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun (_, b, t, o, m, d) => decideReadback b t o m == d) = true := by
  decide

theorem decision_vectors_count : decisionVectors.length = 7 := by decide

/-- The readback gives an identity only when every static premise holds. -/
theorem identity_exactly_when (b t o m : Bool) :
    decideReadback b t o m = .identity ↔ (b = true ∧ t = true ∧ o = false ∧ m = false) := by
  cases b <;> cases t <;> cases o <;> cases m <;> simp [decideReadback]

/-- A type-mismatched view of a stored identity is refused (type punning). -/
theorem a_type_mismatched_view_is_refused (o m : Bool) :
    decideReadback true false o m = .pun := by
  cases o <;> cases m <;> rfl

/-- An owner some callee, alias or cast may reach never gives an identity. -/
theorem an_escaping_owner_gives_no_identity (b t m : Bool) :
    decideReadback b t true m ≠ .identity := by
  cases b <;> cases t <;> cases m <;> simp [decideReadback]

/-- A member that is addressed, incremented or compound-assigned never gives
an identity. -/
theorem an_escaping_member_gives_no_identity (b t o : Bool) :
    decideReadback b t o true ≠ .identity := by
  cases b <;> cases t <;> cases o <;> simp [decideReadback]

/-- A member no admitted store names (not a store base) gives no identity
and is not refused: the read stays the borrowed alias it was. -/
theorem an_unlisted_owner_keeps_the_old_reading (t o m : Bool) :
    decideReadback false t o m = .noStoreBase := rfl

/-- `flow_binds` in `void_view.rs`: the destination is bound to the member's
identity only on an admitted readback whose flow premise resolves. -/
def flowBinds (d : ReadbackDecision) (resolves : Bool) : Bool :=
  d == .identity && resolves

theorem the_flow_binds_only_an_admitted_resolved_readback (d : ReadbackDecision) (r : Bool) :
    flowBinds d r = true ↔ (d = .identity ∧ r = true) := by
  cases d <;> cases r <;> simp [flowBinds]

/-! ## 3. The readback: the flow's event model -/

/-- What happens to the member (and its child) between the owner's
acquisition and the readback. -/
inductive Event where
  /-- An admitted field-link store of identity `n`. -/
  | store (n : Nat)
  /-- A refused statement or an uncontracted call: the binding is unknown. -/
  | poison
  /-- A join with a path on which the last store did not happen (rule 11c's
  partial link). -/
  | join
  /-- A release of identity `n`. -/
  | release (n : Nat)
  deriving DecidableEq, Repr

structure State where
  /-- The member's binding: the identity of the last store, if known. -/
  binding : Option Nat
  /-- The link may not hold on every path. -/
  partialLink : Bool
  /-- The identities released so far. -/
  released : List Nat
  deriving DecidableEq, Repr

def init : State := ⟨none, false, []⟩

def step (s : State) : Event → State
  | .store n => { s with binding := some n, partialLink := false }
  | .poison => { s with binding := none }
  | .join => { s with partialLink := true }
  | .release n => { s with released := n :: s.released }

def run (s : State) (es : List Event) : State := es.foldl step s

/-- The readback (`bound_read_resolves`): the stored identity when the
binding is known, the link holds on every path and the child is live. -/
def readback (s : State) : Option Nat :=
  match s.binding with
  | some n => if !s.partialLink && !s.released.contains n then some n else none
  | none => none

/-- The Rust `EVENT_VECTORS`, as events (identities `t` = 1, `u` = 2), with
whether the readback yields the stored identity. -/
def eventVectors : List (List Event × Bool) := [
  ([.store 1, .release 2], true),
  ([.store 1, .store 2, .release 1], true),
  ([.store 1, .release 2, .poison], false),
  ([.store 1, .release 1, .release 2], false),
  ([.store 1, .join, .release 2], false),
  ([.release 1, .release 2], false)
]

theorem event_vectors_hold :
    eventVectors.all (fun (es, b) => (readback (run init es)).isSome == b) = true := by
  decide

theorem event_vector_outcomes :
    eventVectors.map (·.2) = [true, true, false, false, false, false] := by decide

theorem run_append (s : State) (es fs : List Event) :
    run s (es ++ fs) = run (run s es) fs := by
  simp [run, List.foldl_append]

/-- A readback right after a store of a live child returns that identity. -/
theorem a_readback_returns_the_stored_identity (s : State) (n : Nat)
    (live : n ∉ s.released) :
    readback (step s (.store n)) = some n := by
  simp [step, readback, live]

/-- An intervening poison leaves no identity until the next store. -/
theorem an_intervening_unknown_store_yields_no_identity (s : State) (fs : List Event)
    (noStore : ∀ n, Event.store n ∉ fs) :
    readback (run (step s .poison) fs) = none := by
  induction fs generalizing s with
  | nil => simp [run, step, readback]
  | cons e fs ih =>
    have tail : ∀ n, Event.store n ∉ fs := fun n h => noStore n (List.mem_cons_of_mem _ h)
    cases e with
    | store n => exact absurd (List.mem_cons_self) (noStore n)
    | poison =>
      have := ih s tail
      simpa [run, step] using this
    | join =>
      have := ih { s with partialLink := true } tail
      simpa [run, step] using this
    | release m =>
      have := ih { s with released := m :: s.released } tail
      simpa [run, step] using this

/-- A join where the store did not happen on every path leaves no identity
until the next store. -/
theorem a_partial_link_yields_no_identity (s : State) (fs : List Event)
    (noStore : ∀ n, Event.store n ∉ fs) :
    readback (run (step s .join) fs) = none := by
  induction fs generalizing s with
  | nil =>
    simp only [run, List.foldl, step, readback]
    cases s.binding <;> simp
  | cons e fs ih =>
    have tail : ∀ n, Event.store n ∉ fs := fun n h => noStore n (List.mem_cons_of_mem _ h)
    cases e with
    | store n => exact absurd (List.mem_cons_self) (noStore n)
    | poison =>
      have := ih { s with binding := none } tail
      simpa [run, step] using this
    | join =>
      have := ih s tail
      simpa [run, step] using this
    | release m =>
      have := ih { s with released := m :: s.released } tail
      simpa [run, step] using this

/-- A released child is never read back. -/
theorem a_released_child_is_never_read_back (s : State) (n : Nat)
    (released : n ∈ s.released) :
    readback s ≠ some n := by
  unfold readback
  cases s.binding with
  | none => simp
  | some m =>
    by_cases e : m = n
    · subst e; split <;> simp_all
    · split <;> simp_all

/-- The identity of the last store, with poison and joins clearing it. -/
def lastStore : List Event → Option Nat → Option Nat
  | [], acc => acc
  | .store n :: es, _ => lastStore es (some n)
  | .poison :: es, _ => lastStore es none
  | .join :: es, _ => lastStore es none
  | .release _ :: es, acc => lastStore es acc

/-- Soundness of the readback: whatever it returns is the identity of the last
store, with no poison or join after it. -/
theorem the_readback_is_the_last_store (s : State) (es : List Event) (n : Nat)
    (h : readback (run s es) = some n) :
    lastStore es (if s.partialLink then none else s.binding) = some n := by
  induction es generalizing s with
  | nil =>
    simp only [run, List.foldl] at h
    unfold readback at h
    simp only [lastStore]
    cases hb : s.binding with
    | none => simp [hb] at h
    | some m =>
      rw [hb] at h
      cases hp : s.partialLink <;> simp_all
  | cons e es ih =>
    cases e with
    | store m =>
      have := ih { s with binding := some m, partialLink := false } (by simpa [run, step] using h)
      simpa [lastStore] using this
    | poison =>
      have := ih { s with binding := none } (by simpa [run, step] using h)
      simpa [lastStore] using this
    | join =>
      have := ih { s with partialLink := true } (by simpa [run, step] using h)
      simpa [lastStore] using this
    | release m =>
      have := ih { s with released := m :: s.released } (by simpa [run, step] using h)
      simpa [lastStore] using this

/-- From the acquisition, a readback returns only an identity some store in the
trace wrote. -/
theorem lastStore_is_stored (es : List Event) (acc : Option Nat) (n : Nat)
    (h : lastStore es acc = some n) : acc = some n ∨ Event.store n ∈ es := by
  induction es generalizing acc with
  | nil => simp [lastStore] at h; exact Or.inl h
  | cons e es ih =>
    cases e with
    | store m =>
      rcases ih (some m) (by simpa [lastStore] using h) with h' | h'
      · right; simp at h'; subst h'; exact List.mem_cons_self
      · right; exact List.mem_cons_of_mem _ h'
    | poison =>
      rcases ih none (by simpa [lastStore] using h) with h' | h'
      · simp at h'
      · right; exact List.mem_cons_of_mem _ h'
    | join =>
      rcases ih none (by simpa [lastStore] using h) with h' | h'
      · simp at h'
      · right; exact List.mem_cons_of_mem _ h'
    | release m =>
      rcases ih acc (by simpa [lastStore] using h) with h' | h'
      · left; exact h'
      · right; exact List.mem_cons_of_mem _ h'

theorem a_readback_returns_only_a_stored_identity (es : List Event) (n : Nat)
    (h : readback (run init es) = some n) : Event.store n ∈ es := by
  have := the_readback_is_the_last_store init es n h
  simp [init] at this
  rcases lastStore_is_stored es none n this with h' | h'
  · simp at h'
  · exact h'

/-- Non-vacuity: the flagship readback yields its identity, and one poison
flips it. -/
theorem the_flagship_readback_yields_its_identity :
    readback (run init [.store 1]) = some 1 := by decide

theorem one_poison_flips_the_flagship :
    readback (run init [.store 1, .poison]) = none := by decide

end QCore.VoidViewReadback
