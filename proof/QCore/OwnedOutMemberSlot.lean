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

import QCore.PointerStore

/-!
# An owned value delivered into a member slot of a caller-supplied object (card CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT, gateway rule 72)

USER DECISION 2026-11-21: launch the card. `return
cmt_mpack_consume_string_tag(reader, &opts->ns);` hands the callee the
address of a member of an object reached from a formal, and the callee stores
a fresh allocation through it. Rule 72 reads the call as the store `*slot =
fresh` made by the callee, decided by rule 10v's owner premise.

The module has five parts.

1. **The flow decision** (`decideDelivery`, bound to the Rust
   `DECISION_VECTORS` of `owned_out_member.rs`): admitted only into an
   untracked owner.
2. **The slot reader** (`readSlot`, bound to `ROOT_VECTORS`): `& R -> F (-> G)*
   (. H)*`, and nothing else.
3. **The caller-supplied roots** (`Roots`): an unrebound formal, or a name
   every write of which is a root, a cast of one or a member read through
   one; a name with any other write is never a root.
4. **The delivery over a concrete frame** (`Frame`, `deliver`): an admitted
   delivery adds nothing the frame owes and drops nothing it owed, and the
   delivered identity is held by the caller's object; a tracked owner is
   refused, so a frame-owned record's member is never overwritten here.
5. **The summary gate**: a body with a delivery publishes no summary, so no
   caller reads the delivery as `Borrowed`.

## What this is not

A model of the Clang reading or of the CFG. That the actual is spelled as the
slot reader reads it, that the call is the statement's only call, and that
the callee's declaration is true are the gateway's and the declaration's
(trust boundary 28 and contract specification C06). That a pointer read out of
a caller's object designates an object a caller holds is premise
P-72-CALLER-OBJECT, and the old value of the slot is P-CG11-CALLER-SLOT:
stated, not proved.
-/

namespace QCore.OwnedOutMemberSlot

open QCore.PointerStore (OwnerClass Decision)

/-! ## 1. The flow decision -/

/-- `owned_out_member::decide`: the destination's owner alone decides. -/
def decideDelivery : OwnerClass → Decision
  | .untracked => .admit
  | .tracked => .ownedField
  | .unresolved => .unresolvedOwner

/-- The Rust `DECISION_VECTORS`. -/
def decisionVectors : List (OwnerClass × Decision) := [
  (.untracked, .admit),
  (.tracked, .ownedField),
  (.unresolved, .unresolvedOwner)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun (o, d) => decideDelivery o == d) = true := by
  decide

/-- Admitted exactly into an untracked owner: a tracked owner may own the
member's old value (only O04-B08's field link rewrites one). -/
theorem admitted_iff_untracked (o : OwnerClass) :
    decideDelivery o = .admit ↔ o = .untracked := by
  cases o <;> simp [decideDelivery]

/-! ## 2. The slot reader -/

/-- A fused token of a slot actual. -/
inductive Tok where
  | amp
  | arrow
  | dot
  | name (n : String)
  | other
  deriving DecidableEq, Repr

/-- The selections after the root: each `->` or `.` followed by a member. -/
def steps : List Tok → Option (List (Bool × String))
  | [] => some []
  | .arrow :: .name m :: rest => (steps rest).map ((true, m) :: ·)
  | .dot :: .name m :: rest => (steps rest).map ((false, m) :: ·)
  | _ => none

/-- Split selections at the last arrow: the links before it (all arrows),
and the final member path from it on (that arrow, then dots only). -/
def splitLast : List (Bool × String) → Option (List String × List String)
  | [] => none
  | (arrow, m) :: rest =>
    match splitLast rest with
    | some (links, path) =>
      if arrow then some (m :: links, path) else none
    | none =>
      if arrow && rest.all (fun s => !s.1) then some ([], m :: rest.map (·.2)) else none

/-- `read_slot`: the root, the links and the final member path. -/
def readSlot : List Tok → Option (String × List String × List String)
  | .amp :: .name r :: rest =>
    match steps rest with
    | some ss@((true, _) :: _) => (splitLast ss).map fun (links, path) => (r, links, path)
    | _ => none
  | _ => none

/-- The Rust `ROOT_VECTORS`, spelled as tokens (the final path's members
joined with `.` there). -/
def slotVectors : List (List Tok × Option (String × List String × List String)) := [
  ([.amp, .name "opts", .arrow, .name "ns"], some ("opts", [], ["ns"])),
  ([.amp, .name "context", .arrow, .name "scope_span", .arrow, .name "instrumentation_scope",
      .arrow, .name "name"],
    some ("context", ["scope_span", "instrumentation_scope"], ["name"])),
  ([.amp, .name "context", .arrow, .name "span", .arrow, .name "status", .dot, .name "message"],
    some ("context", ["span"], ["status", "message"])),
  ([.amp, .name "ciov", .arrow, .name "iov_base"], some ("ciov", [], ["iov_base"])),
  ([.amp, .name "s"], none),
  ([.amp, .name "s", .dot, .name "p"], none),
  ([.amp, .name "s", .arrow, .name "a", .dot, .name "b", .arrow, .name "c"], none),
  ([.amp, .name "s", .arrow, .name "p", .other, .other, .other], none),
  ([.amp, .other, .name "s", .arrow, .name "p", .other], none),
  ([.name "s", .arrow, .name "p"], none)
]

theorem slot_vectors_hold :
    slotVectors.all (fun (ts, r) => readSlot ts == r) = true := by
  decide

/-- **A member of this frame's own record object is never a slot**: the first
selection must dereference the root. -/
theorem a_dot_first_is_never_a_slot (r m : String) (rest : List Tok) :
    readSlot (.amp :: .name r :: .dot :: .name m :: rest) = none := by
  simp only [readSlot, steps]
  cases steps rest <;> rfl

/-- Without the address-of, nothing is a slot. -/
theorem an_unaddressed_actual_is_never_a_slot (t : Tok) (rest : List Tok) (h : t ≠ .amp) :
    readSlot (t :: rest) = none := by
  cases t <;> simp_all [readSlot]

/-! ## 3. The caller-supplied roots -/

/-- One write of a name: from a name, a cast of a name or a member read
through a name (`N`, `(T *) N`, `N->F…`), or anything else. -/
inductive Write where
  | fromName (n : String)
  | other
  deriving DecidableEq, Repr

/-- The roots after one step: the unrebound formals, and each other name with
at least one write, every write from a current root. -/
def step (formals : List String) (writes : String → List Write) (names : List String)
    (roots : List String) : List String :=
  formals.filter (fun f => (writes f).isEmpty) ++
    names.filter (fun n =>
      !(writes n).isEmpty &&
        (writes n).all (fun w => match w with
          | .fromName m => roots.contains m
          | .other => false))

/-- `caller_roots`: the step iterated (the Rust loop runs to its fixpoint,
reached within `names.length + 1` steps). -/
def roots (formals : List String) (writes : String → List Write) (names : List String) :
    Nat → List String
  | 0 => formals.filter (fun f => (writes f).isEmpty)
  | k + 1 => step formals writes names (roots formals writes names k)

/-- **A name with any other write is never a root**: an allocation, a local's
address, an arithmetic write each make the name refuse, at every iterate. -/
theorem another_write_is_never_a_root (formals : List String) (writes : String → List Write)
    (names : List String) (n : String) (hf : n ∉ formals) (ho : Write.other ∈ writes n)
    (k : Nat) : n ∉ roots formals writes names k := by
  have hne : (writes n).isEmpty = false := by
    cases h : writes n with
    | nil => rw [h] at ho; cases ho
    | cons _ _ => rfl
  cases k with
  | zero =>
    simp only [roots, List.mem_filter]
    intro ⟨hm, _⟩
    exact hf hm
  | succ k =>
    simp only [roots, step, List.mem_append, List.mem_filter]
    intro h
    rcases h with ⟨hm, _⟩ | ⟨_, hall⟩
    · exact hf hm
    · simp only [Bool.and_eq_true, List.all_eq_true] at hall
      have := hall.2 _ ho
      simp at this

/-- **A rebound formal is never a root**: a formal with a write is admitted
only as a name, and then only when every write is from a root. -/
theorem a_written_formal_is_only_a_derived_root (formals : List String)
    (writes : String → List Write) (names : List String) (f : String)
    (hn : f ∉ names) (hw : (writes f).isEmpty = false) (k : Nat) :
    f ∉ roots formals writes names k := by
  cases k with
  | zero =>
    simp [roots, List.mem_filter, hw]
  | succ k =>
    simp only [roots, step, List.mem_append, List.mem_filter]
    intro h
    rcases h with ⟨_, he⟩ | ⟨hm, _⟩
    · rw [hw] at he; cases he
    · exact hn hm

/-- The cmetrics shape: `map = (struct cmt_map *) context; opts = map->opts;`
with `context` an unrebound formal makes `opts` a root in two steps. -/
theorem the_cmetrics_chain_is_rooted :
    let writes : String → List Write := fun n =>
      if n = "map" then [.fromName "context"]
      else if n = "opts" then [.fromName "map"] else []
    "opts" ∈ roots ["reader", "index", "context"] writes ["map", "opts"] 2 := by
  decide

/-! ## 4. The delivery over a concrete frame -/

/-- A frame: the identities it owes (each released or returned before it
ends), and the identities held by caller-supplied objects. -/
structure Frame where
  owed : List Nat
  held : List Nat
  deriving DecidableEq, Repr

/-- The call delivering fresh identity `fresh` into a slot whose owner the
flow classed `o`: admitted, the caller's object holds it and the frame owes
nothing more; otherwise the node is the gate's refusal (`none`). -/
def deliver (f : Frame) (o : OwnerClass) (fresh : Nat) : Option Frame :=
  match decideDelivery o with
  | .admit => some { f with held := fresh :: f.held }
  | _ => none

/-- **Nothing is dropped and nothing is added to what the frame owes**: every
obligation before the call is an obligation after it, and the delivered
identity is not one. -/
theorem an_admitted_delivery_keeps_the_frames_obligations (f f' : Frame) (o : OwnerClass)
    (fresh : Nat) (h : deliver f o fresh = some f') : f'.owed = f.owed := by
  cases o <;> simp [deliver, decideDelivery] at h
  subst h
  rfl

/-- **The delivered identity is held by the caller's object**, never lost. -/
theorem the_delivered_identity_is_held (f f' : Frame) (o : OwnerClass) (fresh : Nat)
    (h : deliver f o fresh = some f') : fresh ∈ f'.held := by
  cases o <;> simp [deliver, decideDelivery] at h
  subst h
  simp

/-- **A frame-owned or unresolved owner is refused**: the member of a record
this body allocated may own its old value, and only O04-B08's field link
rewrites one. -/
theorem a_tracked_owner_is_refused (f : Frame) (fresh : Nat) :
    deliver f .tracked fresh = none ∧ deliver f .unresolved fresh = none :=
  ⟨rfl, rfl⟩

/-- **A frame leak is not laundered**: an identity the frame owes before the
call is owed after it. -/
theorem a_frame_obligation_survives (f f' : Frame) (o : OwnerClass) (fresh n : Nat)
    (h : deliver f o fresh = some f') (hn : n ∈ f.owed) : n ∈ f'.owed := by
  rw [an_admitted_delivery_keeps_the_frames_obligations f f' o fresh h]
  exact hn

/-! ## 5. The summary gate -/

/-- What a body's CFG holds, as the W4 gate reads it: an admitted pointer
store of any class (rule 10v's, rule 70's, or this rule's delivery). -/
structure Body where
  pointerStores : Nat
  deliveries : Nat
  deriving DecidableEq, Repr

/-- A delivery is a pointer store. -/
def Body.wellFormed (b : Body) : Prop := b.deliveries ≤ b.pointerStores

/-- `has_pointer_store`'s gate (rule 70's relaxation never states a
delivery): a body publishes an ordinary summary only with no pointer store. -/
def publishes (b : Body) : Bool := b.pointerStores == 0

/-- **A body with a delivery publishes no summary**: a caller sees an
uncontracted callee, never a `Borrowed` formal whose member now owns custody. -/
theorem a_delivery_withholds_the_summary (b : Body) (hw : b.wellFormed)
    (hd : 0 < b.deliveries) : publishes b = false := by
  unfold publishes Body.wellFormed at *
  have : b.pointerStores ≠ 0 := by omega
  simp [this]

end QCore.OwnedOutMemberSlot
