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
# Interior borrows (card SLOT-MOVE-AND-INTERIOR-BORROW, class 8, rule 12y)

The abstract rule implemented by `crates/qcore-c-gateway/src/slot_move_borrow.rs`
(`borrow_step`, `borrow_replay`, `BorrowState::in_family`) and its route-G
adapter `legacy_slot_move_borrow.rs` (`before`, `after`), contract
specification §C04.3.4.

USER DECISION 2026-10-05 (`W48-CALLEE-DECISION-TABLE` class 8): the result
is borrowed from the parent: no release obligation for the caller; valid only
while the parent lives and is not mutated through another path (a parent
release or an invalidating mutation ends it, and a later use is refused); a
write through it is a write to the parent; a lazily created child is created
inside and owned by the parent.

## The model

* A borrow is a name with a record: the take that produced it (an alias keeps
  it) and its parent. Its **family** is every name whose record chain reaches
  that take: its aliases and the borrows taken from them.
* A mutation of a name (a call or store that may write what it reaches; a
  write through a borrow is one) ends every borrow outside that name's
  family; a parent's release is the same; a lazily created child's take also
  mutates its parent. An ended name's use, alias, mutation or release is
  refused; a borrow's release is refused (it owns nothing). A refusal is
  sticky.

## The theorems

* `a_borrow_release_is_refused`: no release obligation, and no release.
* `a_parent_release_ends_its_borrows`, `an_invalidating_mutation_ends_a_borrow`:
  the borrow ends, and `a_use_after_the_end_is_refused`.
* `a_write_through_a_borrow_keeps_it`: a write through it reaches the parent,
  not the borrow; `a_write_through_a_borrow_ends_its_sibling` (vector).
* `a_lazily_created_child_is_owned_by_the_parent` (vectors): its release is
  refused, the parent's release last is no refusal.
* `a_safe_path_never_uses_an_ended_borrow`.
* `borrow_trace_vectors_hold`: the table shared with Rust.

## What this is not

A model of the decision over abstract names. Which instructions the gateway
reads as mutations (every call or store, with only the borrow's own family and
declared scalars exempt; an unread actual mutates everything), the frame
premise that a call reaches only what its actuals reach, and the
correspondence with the compiled Rust are review premises (trust boundary
28), pinned by the shared vectors and
`crates/qcore-c-gateway/tests/slot_move_interior_borrow.rs`.
-/

namespace QCore.InteriorBorrow

structure Record where
  key : Nat
  parent : Nat
  deriving DecidableEq, Repr

structure State where
  borrows : List (Nat × Record)
  ended : List Nat
  refused : Bool
  deriving DecidableEq, Repr

def State.empty : State := ⟨[], [], false⟩

def lookup (bs : List (Nat × Record)) (n : Nat) : Option Record :=
  (bs.find? fun p => p.1 == n).map (·.2)

def unbind (bs : List (Nat × Record)) (n : Nat) : List (Nat × Record) :=
  bs.filter fun p => p.1 != n

/-- Whether `name`'s record chain reaches the take `key`, within `fuel` steps. -/
def reaches (bs : List (Nat × Record)) (key : Nat) : Nat → Nat → Bool
  | 0, _ => false
  | fuel + 1, name =>
      match lookup bs name with
      | none => false
      | some r => r.key == key || reaches bs key fuel r.parent

/-- `BorrowState::in_family`: `name` is in the family of the borrow `b`. -/
def inFamily (bs : List (Nat × Record)) (b name : Nat) : Bool :=
  match lookup bs b with
  | none => false
  | some r => reaches bs r.key (bs.length + 1) name

/-- End every borrow outside the family of the mutated `name`. -/
def endUnlessFamily (st : State) (name : Nat) : State :=
  let gone := (st.borrows.map (·.1)).filter fun b => !inFamily st.borrows b name
  { st with borrows := st.borrows.filter (fun p => !gone.contains p.1),
            ended := gone ++ st.ended }

inductive Event where
  | take (borrow parent : Nat) (created : Bool)
  | alias (copy of_ : Nat)
  | mutate (name : Nat)
  | use (name : Nat)
  | release (name : Nat)
  deriving DecidableEq, Repr

/-- `slot_move_borrow::borrow_step`. -/
def step (st : State) : Event → State
  | .take b p created =>
      if st.ended.contains p then { st with refused := true }
      else
        let st := if created then endUnlessFamily st p else st
        { st with borrows := (b, ⟨b, p⟩) :: unbind st.borrows b,
                  ended := st.ended.filter (· != b) }
  | .alias c o =>
      if st.ended.contains o then { st with refused := true }
      else match lookup st.borrows o with
        | some r => { st with borrows := (c, r) :: unbind st.borrows c,
                              ended := st.ended.filter (· != c) }
        | none => st
  | .mutate n => if st.ended.contains n then { st with refused := true } else endUnlessFamily st n
  | .use n => if st.ended.contains n then { st with refused := true } else st
  | .release n =>
      if st.ended.contains n || (lookup st.borrows n).isSome then { st with refused := true }
      else endUnlessFamily st n

def run (st : State) (es : List Event) : State := es.foldl step st

theorem run_append (st : State) (pre post : List Event) :
    run st (pre ++ post) = run (run st pre) post := by
  simp [run, List.foldl_append]

theorem endUnlessFamily_refused (st : State) (n : Nat) :
    (endUnlessFamily st n).refused = st.refused := rfl

theorem step_keeps_refused (st : State) (e : Event) (h : st.refused = true) :
    (step st e).refused = true := by
  cases e with
  | take b p created =>
      simp only [step]
      split
      · rfl
      · cases created <;> simpa [endUnlessFamily] using h
  | alias c o =>
      simp only [step]
      split
      · rfl
      · split <;> simpa using h
  | mutate n =>
      simp only [step]
      split
      · rfl
      · simpa [endUnlessFamily] using h
  | use n => simp only [step]; split <;> simp_all
  | release n =>
      simp only [step]
      split
      · rfl
      · simpa [endUnlessFamily] using h

theorem run_keeps_refused (st : State) (es : List Event) (h : st.refused = true) :
    (run st es).refused = true := by
  induction es generalizing st with
  | nil => simpa [run] using h
  | cons e es ih =>
      show (run (step st e) es).refused = true
      exact ih _ (step_keeps_refused st e h)

/-- **A borrow owns nothing**: its release is refused, never a release. -/
theorem a_borrow_release_is_refused (st : State) (b : Nat) (r : Record)
    (h : lookup st.borrows b = some r) : (step st (.release b)).refused = true := by
  simp [step, h]

/-- A name that is no borrow is in no borrow's family. -/
theorem no_borrow_no_family (bs : List (Nat × Record)) (b n : Nat)
    (h : lookup bs n = none) : inFamily bs b n = false := by
  unfold inFamily
  cases lookup bs b with
  | none => rfl
  | some r =>
      show reaches bs r.key (bs.length + 1) n = false
      simp [reaches, h]

theorem ended_after_end (st : State) (b n : Nat) (hb : (lookup st.borrows b).isSome = true)
    (hf : inFamily st.borrows b n = false) : (endUnlessFamily st n).ended.contains b = true := by
  simp only [endUnlessFamily, List.contains_append, Bool.or_eq_true]
  left
  apply List.elem_eq_true_of_mem
  apply List.mem_filter.mpr
  refine ⟨?_, by simp [hf]⟩
  cases hl : lookup st.borrows b with
  | none => simp [hl] at hb
  | some r =>
      unfold lookup at hl
      cases hfind : st.borrows.find? (fun p => p.1 == b) with
      | none => simp [hfind] at hl
      | some p =>
          have hp := List.mem_of_find?_eq_some hfind
          have hk := List.find?_some hfind
          simp at hk
          exact List.mem_map.mpr ⟨p, hp, hk⟩

/-- **A parent's release ends every borrow taken from it** (the parent is no
borrow, so it is in no borrow's family). -/
theorem a_parent_release_ends_its_borrows (st : State) (b p : Nat)
    (hb : (lookup st.borrows b).isSome = true) (hp : lookup st.borrows p = none)
    (hnot : st.ended.contains p = false) :
    (step st (.release p)).ended.contains b = true := by
  simp only [step, hnot, hp, Option.isSome_none, Bool.or_false, Bool.false_eq_true, ite_false]
  exact ended_after_end st b p hb (no_borrow_no_family _ b p hp)

/-- **An invalidating mutation ends the borrow**: a mutation of any name
outside the borrow's family. -/
theorem an_invalidating_mutation_ends_a_borrow (st : State) (b n : Nat)
    (hb : (lookup st.borrows b).isSome = true) (hf : inFamily st.borrows b n = false)
    (hnot : st.ended.contains n = false) :
    (step st (.mutate n)).ended.contains b = true := by
  simp only [step, hnot, Bool.false_eq_true, ite_false]
  exact ended_after_end st b n hb hf

/-- **A use after the end is refused.** -/
theorem a_use_after_the_end_is_refused (st : State) (b : Nat) (h : st.ended.contains b = true) :
    (step st (.use b)).refused = true := by
  have hm : b ∈ st.ended := by simpa using h
  simp [step, hm]

/-- A borrow is in its own family. -/
theorem in_own_family (bs : List (Nat × Record)) (b : Nat) (r : Record)
    (h : lookup bs b = some r) : inFamily bs b b = true := by
  simp [inFamily, h, reaches]

/-- **A write through a borrow keeps it**: the write reaches its parent, so
it ends the borrows outside the borrow's family, never the borrow itself. -/
theorem a_write_through_a_borrow_keeps_it (st : State) (b : Nat) (r : Record)
    (h : lookup st.borrows b = some r) :
    b ∉ (st.borrows.map (·.1)).filter (fun k => !inFamily st.borrows k b) := by
  have hown := in_own_family st.borrows b r h
  simp [hown]

theorem a_write_through_a_borrow_does_not_end_it (st : State) (b : Nat) (r : Record)
    (h : lookup st.borrows b = some r) (hn : b ∉ st.ended) :
    b ∉ (endUnlessFamily st b).ended := by
  simp only [endUnlessFamily, List.mem_append, not_or]
  exact ⟨a_write_through_a_borrow_keeps_it st b r h, hn⟩

/-- **A `Safe` path never uses an ended borrow.** -/
theorem a_safe_path_never_uses_an_ended_borrow (pre post : List Event) (b : Nat)
    (hsafe : (run State.empty (pre ++ .use b :: post)).refused = false) :
    (run State.empty pre).ended.contains b = false := by
  rw [run_append] at hsafe
  cases hc : (run State.empty pre).ended.contains b with
  | false => rfl
  | true =>
      have hr := a_use_after_the_end_is_refused _ b hc
      have := run_keeps_refused _ post hr
      simp only [run, List.foldl_cons] at hsafe
      simp only [run] at this
      rw [this] at hsafe
      exact absurd hsafe (by simp)

/-! ## The brief's controls and the shared vectors -/

/-- `BORROW_TRACE_VECTORS` of `slot_move_borrow.rs` (name 0 the parent), with
whether the replay refuses. -/
def borrowTraceVectors : List (List Event × Bool) := [
  ([.take 1 0 false, .release 0, .use 1], true),
  ([.take 1 0 false, .mutate 0, .use 1], true),
  ([.take 1 0 false, .release 1], true),
  ([.take 1 0 false, .mutate 1, .use 1], false),
  ([.take 1 0 false, .take 2 0 false, .mutate 1, .use 2], true),
  ([.take 1 0 true, .use 1, .release 0], false),
  ([.take 1 0 false, .take 2 0 true, .use 1], true),
  ([.take 1 0 false, .alias 2 1, .mutate 1, .use 2], false)
]

theorem borrow_trace_vectors_hold :
    borrowTraceVectors.all (fun (es, refused) => (run State.empty es).refused == refused) = true := by
  decide

/-- **A lazily created child is owned by the parent**: the caller owes it
nothing and may not release it, and the parent's release (last) is no
refusal. -/
theorem a_lazily_created_child_is_owned_by_the_parent :
    (run State.empty [.take 1 0 true, .release 1]).refused = true ∧
      (run State.empty [.take 1 0 true, .use 1, .release 0]).refused = false := by
  decide

/-- **A write through one borrow ends its sibling**: it is a write to the
parent. -/
theorem a_write_through_a_borrow_ends_its_sibling :
    (run State.empty [.take 1 0 false, .take 2 0 false, .mutate 1]).ended.contains 2 = true ∧
      (run State.empty [.take 1 0 false, .take 2 0 false, .mutate 1]).ended.contains 1 = false := by
  decide

end QCore.InteriorBorrow
