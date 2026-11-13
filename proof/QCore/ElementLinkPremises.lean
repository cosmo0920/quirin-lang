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
# An element destroyer's link premises, checked (card LERI-HELD-AND-LINK-MEMBER-PREMISES)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_link_member.rs` (gateway rule 46), with
its hooks in `legacy_container_publication.rs` (rule 34's foreign arm and
rule 43's held arm) and `legacy_transfer.rs` (the link records).

An element destroyer runs `mk_list_del(&p->M)` and frees `p`.
`QCore.ElementReleaseSummary` described a held object's links as "not
cleared" and so assumed that they were links an unlink may remove
(P-LERI-HELD-LINKS); it read the destroyer's unlink as removing the element
from the list the caller iterates (P-LERI-LINK-MEMBER). Both are decided here.

**P-LERI-HELD-LINKS.** A link record (`struct mk_list`) has two pointer
fields; `mk_list_del` reads both and writes through them, so it is defined
only when both are valid (`unlinkDefined`). A fresh allocation's fields are
indeterminate and a zeroed one's are null: no defined unlink
(`a_fresh_object_has_no_defined_unlink`,
`a_zeroed_object_has_no_defined_unlink`), so rule 43's unconditional held arm
admitted an undefined unlink
(`the_unchecked_held_arm_admits_an_undefined_unlink`). A checked
self-initialisation whose path is one member and whose writes cover both
fields makes that member unlinkable (`an_initialised_member_is_unlinkable`);
one field is not enough (`a_half_initialisation_is_not_enough`). The body
records `(object, member)` facts; a statement that may write an object's
memory drops its facts, an inert one keeps them, and the join keeps the
facts of both sides (`retaining_preserves_every_fact`,
`the_join_preserves_every_fact`). The held arm admits only a recorded
member (`the_held_arm_is_sound`) and only replaces an admission of rule 43
with a refusal (`the_check_only_refuses`).

**P-LERI-LINK-MEMBER.** An element is linked into the iterated list through
the member its `container_of` binding names. Unlinking member `M` removes it
from that list when every binding names `M` (`an_admitted_unlink_removes_the_element`);
unlinking another member leaves it linked while it is freed
(`a_mismatched_member_leaves_the_element_linked`), so the foreign arm admits
only an exact member (`foreignAdmits`).

## What this is not

A model of the decision. That the lexical reader records the `container_of`
member and the rule 36b write set, that the Rust kill policy's
"may write" over-approximates the C statement's writes (the root binding of
every actual of a contract call with no nested call; everything for every
other statement that is not inert), and the correspondence with the compiled
Rust are review premises (trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/link_member_premises.rs`. P-CRR-MEMBER (the
element is linked into the iterated list through its binding member) is
rule 10f's, still stated.
-/

namespace QCore.ElementLinkPremises

-- P-LERI-HELD-LINKS: the link record.

/-- One link field's value. -/
inductive Ptr where
  | indeterminate
  | null
  | valid
  deriving DecidableEq, Repr

/-- A `struct mk_list`: `prev` and `next`. -/
structure Link where
  prev : Ptr
  next : Ptr
  deriving DecidableEq, Repr

/-- `mk_list_del` reads `entry->prev` and `entry->next` and writes through
them: defined only when both are valid pointers. -/
def unlinkDefined (l : Link) : Bool :=
  l.prev == .valid && l.next == .valid

/-- A fresh allocation's links. -/
def fresh : Link := ⟨.indeterminate, .indeterminate⟩

/-- A zeroed allocation's links, and the links `mk_list_del` leaves. -/
def zeroed : Link := ⟨.null, .null⟩

/-- A checked self-initialisation storing each written field's own address. -/
def selfInit (writes : List String) (l : Link) : Link :=
  { prev := if "prev" ∈ writes then .valid else l.prev,
    next := if "next" ∈ writes then .valid else l.next }

/-- The member a self-initialisation makes unlinkable (`initialised_member`):
a one-member path whose writes cover both link fields. -/
def initialisedMember : List String → List String → Option String
  | [m], writes => if "next" ∈ writes ∧ "prev" ∈ writes then some m else none
  | _, _ => none

theorem a_fresh_object_has_no_defined_unlink : unlinkDefined fresh = false := by decide

theorem a_zeroed_object_has_no_defined_unlink : unlinkDefined zeroed = false := by decide

theorem a_half_initialisation_is_not_enough :
    unlinkDefined (selfInit ["next"] fresh) = false := by decide

theorem an_initialised_member_is_unlinkable (path writes : List String) (m : String)
    (l : Link) (h : initialisedMember path writes = some m) :
    unlinkDefined (selfInit writes l) = true := by
  match path, h with
  | [x], h =>
    simp only [initialisedMember] at h
    split at h
    · rename_i hw
      simp [unlinkDefined, selfInit, hw.1, hw.2]
    · simp at h

theorem an_initialised_member_is_the_path (path writes : List String) (m : String)
    (h : initialisedMember path writes = some m) : path = [m] := by
  match path, h with
  | [x], h =>
    simp only [initialisedMember] at h
    split at h
    · simp at h; simp [h]
    · simp at h

-- The facts a body keeps, and the statements that drop them.

/-- A concrete heap's link records, by object and member. -/
abbrev Heap := Nat → String → Link

/-- Every recorded `(object, member)` link is unlinkable in `h`. -/
def holds (facts : List (Nat × String)) (h : Heap) : Prop :=
  ∀ f ∈ facts, unlinkDefined (h f.1 f.2) = true

/-- A statement as the kill policy reads it: inert (writes no memory), a
contract call that may write only the objects its actuals may denote, or
anything else. -/
inductive Stmt where
  | inert
  | call (reached : List Nat)
  | other
  deriving DecidableEq, Repr

/-- The writes a statement may make (the review premise: an over-approximation
of the C statement). -/
def writesOnly : Stmt → Heap → Heap → Prop
  | .inert, h, h' => h' = h
  | .call reached, h, h' => ∀ x, x ∉ reached → ∀ m, h' x m = h x m
  | .other, _, _ => True

/-- `retain`: the facts a statement keeps. -/
def retain : Stmt → List (Nat × String) → List (Nat × String)
  | .inert, facts => facts
  | .call reached, facts => facts.filter (fun f => !reached.contains f.1)
  | .other, _ => []

theorem retaining_preserves_every_fact (s : Stmt) (facts : List (Nat × String))
    (h h' : Heap) (hold : holds facts h) (w : writesOnly s h h') :
    holds (retain s facts) h' := by
  cases s with
  | inert => simp only [writesOnly] at w; subst w; exact hold
  | call reached =>
    intro f hf
    simp only [retain, List.mem_filter] at hf
    obtain ⟨hmem, hnot⟩ := hf
    have hx : f.1 ∉ reached := by simpa using hnot
    rw [w f.1 hx f.2]
    exact hold f hmem
  | other => intro f hf; simp [retain] at hf

/-- The join keeps a fact only when both arriving paths recorded it. -/
def join (a b : List (Nat × String)) : List (Nat × String) :=
  a.filter (fun f => b.contains f)

theorem the_join_preserves_every_fact (a b : List (Nat × String)) (h : Heap)
    (ha : holds a h) (hb : holds b h) : holds (join a b) h ∧ holds (join b a) h := by
  constructor
  · intro f hf; simp only [join, List.mem_filter] at hf; exact ha f hf.1
  · intro f hf; simp only [join, List.mem_filter] at hf; exact hb f hf.1

/-- The initialisation adds its member's fact, and it holds. -/
theorem an_initialisation_establishes_its_fact (facts : List (Nat × String)) (h : Heap)
    (x : Nat) (path writes : List String) (m : String)
    (hold : holds facts h) (hm : initialisedMember path writes = some m)
    (h' : Heap) (hx : h' x m = selfInit writes (h x m))
    (frame : ∀ y n, (y, n) ≠ (x, m) → h' y n = h y n) :
    holds ((x, m) :: facts.filter (fun f => f ≠ (x, m))) h' := by
  rintro ⟨y, n⟩ hf
  simp only [List.mem_cons, List.mem_filter] at hf
  rcases hf with hf | ⟨hmem, hne⟩
  · rw [hf]; show unlinkDefined (h' x m) = true
    rw [hx]; exact an_initialised_member_is_unlinkable path writes m _ hm
  · have hne' : (y, n) ≠ (x, m) := of_decide_eq_true hne
    show unlinkDefined (h' y n) = true
    rw [frame y n hne']; exact hold (y, n) hmem

-- The held arm.

/-- Rule 46's held decision (`held_decides`): the destroyer names a member
this path initialised. -/
def heldDecides (link : Option String) (ready : List String) : Bool :=
  match link with
  | some m => ready.contains m
  | none => false

/-- Rule 43's held arm before this card: no link condition. -/
def heldBefore (_link : Option String) (_ready : List String) : Bool := true

theorem the_held_arm_is_sound (h : Heap) (x : Nat) (ready : List String)
    (link : Option String) (hold : holds (ready.map (fun m => (x, m))) h)
    (hd : heldDecides link ready = true) :
    ∃ m, link = some m ∧ unlinkDefined (h x m) = true := by
  cases link with
  | none => simp [heldDecides] at hd
  | some m =>
    refine ⟨m, rfl, ?_⟩
    simp only [heldDecides, List.contains_iff_mem] at hd
    exact hold (x, m) (List.mem_map.mpr ⟨m, hd, rfl⟩)

/-- Rule 43's held arm admitted a fresh object, whose unlink is undefined. -/
theorem the_unchecked_held_arm_admits_an_undefined_unlink :
    heldBefore (some "_head") [] = true ∧ heldDecides (some "_head") [] = false ∧
      unlinkDefined fresh = false := by
  decide

/-- A destroyer that names no member is never admitted on a held object. -/
theorem a_memberless_destroyer_refuses (ready : List String) :
    heldDecides none ready = false := rfl

/-- Rule 46 only replaces admissions with refusals. -/
theorem the_check_only_refuses (link : Option String) (ready : List String)
    (_h : heldDecides link ready = true) : heldBefore link ready = true := rfl

-- P-LERI-LINK-MEMBER: the foreign arm.

/-- An element's memberships: the list (by id) each link member links it
into, if any. -/
abbrev Memberships := String → Option Nat

/-- Unlinking member `m`. -/
def unlinkMember (mem : Memberships) (m : String) : Memberships :=
  fun n => if n = m then none else mem n

/-- The element is still linked into list `L`. -/
def linkedIn (mem : Memberships) (L : Nat) : Prop := ∃ n, mem n = some L

/-- Rule 46's foreign decision (`foreign_admits`): the destroyer names its
member, and every `container_of` binding names exactly that member. -/
def foreignAdmits (link : Option String) (members : List String) : Bool :=
  match link with
  | some m => !members.isEmpty && members.all (fun n => n == m)
  | none => false

/-- P-CRR-MEMBER, as this model states it: the iterated list `L` holds the
element through the binding member `f` only. -/
def throughOnly (mem : Memberships) (L : Nat) (f : String) : Prop :=
  mem f = some L ∧ ∀ n, mem n = some L → n = f

theorem an_admitted_unlink_removes_the_element (mem : Memberships) (L : Nat)
    (f : String) (members : List String) (link : Option String)
    (hthrough : throughOnly mem L f) (hf : f ∈ members)
    (hadm : foreignAdmits link members = true) :
    ∃ m, link = some m ∧ ¬ linkedIn (unlinkMember mem m) L := by
  cases link with
  | none => simp [foreignAdmits] at hadm
  | some m =>
    refine ⟨m, rfl, ?_⟩
    simp only [foreignAdmits, Bool.and_eq_true, Bool.not_eq_true', List.all_eq_true,
      beq_iff_eq] at hadm
    have hfm : f = m := hadm.2 f hf
    rintro ⟨n, hn⟩
    simp only [unlinkMember] at hn
    split at hn
    · simp at hn
    · rename_i hne
      have := hthrough.2 n hn
      exact hne (this.trans hfm)

theorem a_mismatched_member_leaves_the_element_linked (mem : Memberships) (L : Nat)
    (f m : String) (hthrough : throughOnly mem L f) (hne : f ≠ m) :
    linkedIn (unlinkMember mem m) L ∧ foreignAdmits (some m) [f] = false := by
  constructor
  · refine ⟨f, ?_⟩
    simp [unlinkMember, hne, hthrough.1]
  · simp [foreignAdmits, hne]

theorem a_memberless_destroyer_takes_no_element (members : List String) :
    foreignAdmits none members = false := rfl

-- The Rust vectors (`FOREIGN_VECTORS`, `INIT_VECTORS`, `HELD_VECTORS`,
-- checked by the unit tests of
-- `crates/qcore-c-gateway/src/legacy_link_member.rs`).

def foreignVectors : List (Option String × List String × Bool) :=
  [ (some "_head", ["_head"], true),
    (some "_head", ["_rotate_head"], false),
    (some "_head", ["_head", "_rotate_head"], false),
    (none, ["_head"], false),
    (some "_head", [], false) ]

def initVectors : List (List String × List String × Option String) :=
  [ (["_head"], ["next", "prev"], some "_head"),
    (["_head"], ["next"], none),
    (["_head"], ["prev"], none),
    ([], ["next", "prev"], none),
    (["inner", "_head"], ["next", "prev"], none) ]

def heldVectors : List (Option String × List String × Bool) :=
  [ (some "_head", ["_head"], true),
    (some "_head", [], false),
    (some "_head", ["_rotate_head"], false),
    (none, ["_head"], false) ]

theorem vectors_hold :
    foreignVectors.all (fun (l, ms, out) => foreignAdmits l ms == out) = true ∧
    initVectors.all (fun (p, w, out) => initialisedMember p w == out) = true ∧
    heldVectors.all (fun (l, r, out) => heldDecides l r == out) = true := by
  decide

end QCore.ElementLinkPremises
