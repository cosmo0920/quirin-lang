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
# Slot moves (card SLOT-MOVE-AND-INTERIOR-BORROW, class 2, rule 12x)

The abstract rule implemented by `crates/qcore-c-gateway/src/slot_move_borrow.rs`
(`decide_slot`, `slot_step`, `slot_replay`) and its route-G adapter
`legacy_slot_move_borrow.rs`, contract specification §C04.15.

USER DECISION 2026-10-05 (`W48-CALLEE-DECISION-TABLE` class 2): after the
call the slot `T **s` still owns an object, possibly a new one; every alias of
the old pointer is invalid (a use is refused, never `Safe`); on failure the
original stays owned by the slot.

## The model

* `decideSlot` is the call-site table over what the slot actual's binding
  denotes: a definitely-NULL slot (nothing moves under `nullable`), exactly one
  live identity this body alone holds (the move), the caller's slot passed
  through (nothing moves here), anything else (refused).
* A path is a list of `Event`s over names and sites. A move keeps the slot's
  binding and the set of live obligations as they were and invalidates every
  other name bound to the slot's identity; the failure arm changes nothing; a
  use or release of an invalid name refuses, and a refusal is sticky.

## The theorems

* `the_slot_keeps_its_identity_and_its_obligation`: a move leaves the slot's
  binding and the live obligations unchanged — the slot owns exactly one
  object before and after.
* `every_old_alias_is_invalid_after_a_move` and `a_use_of_an_old_alias_is_refused`.
* `the_failure_arm_keeps_the_original`: a failed move changes nothing.
* `a_safe_path_never_uses_an_invalid_name`: on a path whose replay is not
  refused, no use reaches a name invalid at that point.
* `slot_vectors_hold`, `slot_trace_vectors_hold`: the tables shared with Rust.

## What this is not

A model of the decision over abstract names and sites. That the gateway reads
`&L` and a passed-through formal slot as the classes say, that its alias set
is the names whose bindings target the slot's identity, and the correspondence
with the compiled Rust are review premises (trust boundary 28), pinned by the
shared vectors and `crates/qcore-c-gateway/tests/slot_move_interior_borrow.rs`.
-/

namespace QCore.SlotMove

inductive SlotBinding where
  | definitelyNull
  | oneTracked (heldElsewhere : Bool)
  | callerSlot
  | other
  deriving DecidableEq, Repr

inductive SlotOutcome where
  | nothing
  | move
  | refuse
  deriving DecidableEq, Repr

/-- Rule 12x's table (`slot_move_borrow::decide_slot`). -/
def decideSlot : SlotBinding → Bool → SlotOutcome
  | .definitelyNull, true => .nothing
  | .definitelyNull, false => .refuse
  | .callerSlot, _ => .nothing
  | .oneTracked false, _ => .move
  | .oneTracked true, _ => .refuse
  | .other, _ => .refuse

/-- Only exactly one identity this body alone holds is moved. -/
theorem a_move_needs_one_identity_held_nowhere_else (b : SlotBinding) (n : Bool)
    (h : decideSlot b n = .move) : b = .oneTracked false := by
  cases b with
  | definitelyNull => cases n <;> simp [decideSlot] at h
  | oneTracked held => cases held <;> simp_all [decideSlot]
  | callerSlot => simp [decideSlot] at h
  | other => simp [decideSlot] at h

theorem an_identity_held_elsewhere_is_refused (n : Bool) :
    decideSlot (.oneTracked true) n = .refuse := rfl

structure State where
  bound : List (Nat × Nat)
  invalid : List Nat
  live : List Nat
  refused : Bool
  deriving DecidableEq, Repr

def State.empty : State := ⟨[], [], [], false⟩

def lookup (b : List (Nat × Nat)) (n : Nat) : Option Nat :=
  (b.find? fun p => p.1 == n).map (·.2)

def unbind (b : List (Nat × Nat)) (n : Nat) : List (Nat × Nat) :=
  b.filter fun p => p.1 != n

inductive Event where
  | bind (name site : Nat)
  | move (slot : Nat) (failed : Bool)
  | use (name : Nat)
  | release (name : Nat)
  deriving DecidableEq, Repr

/-- Whether a binding is an alias of `site` other than the slot. -/
def isAlias (slot site : Nat) (p : Nat × Nat) : Bool := p.1 != slot && p.2 == site

/-- `slot_move_borrow::slot_step`. -/
def step (st : State) : Event → State
  | .bind n s =>
      { st with bound := (n, s) :: unbind st.bound n,
                invalid := st.invalid.filter (· != n),
                live := if st.live.contains s then st.live else s :: st.live }
  | .move slot failed =>
      match lookup st.bound slot with
      | none => { st with refused := true }
      | some s =>
          if failed then st
          else { st with bound := st.bound.filter (fun p => !isAlias slot s p),
                         invalid := (st.bound.filter (isAlias slot s)).map (·.1) ++ st.invalid }
  | .use n => if st.invalid.contains n then { st with refused := true } else st
  | .release n =>
      if st.invalid.contains n then { st with refused := true }
      else match lookup st.bound n with
        | some s => { st with live := st.live.filter (· != s) }
        | none => st

def run (st : State) (es : List Event) : State := es.foldl step st

theorem run_append (st : State) (pre post : List Event) :
    run st (pre ++ post) = run (run st pre) post := by
  simp [run, List.foldl_append]

theorem step_keeps_refused (st : State) (e : Event) (h : st.refused = true) :
    (step st e).refused = true := by
  cases e with
  | bind n s => simpa [step] using h
  | move slot failed =>
      simp only [step]
      split
      · rfl
      · split <;> simpa using h
  | use n => simp only [step]; split <;> simp_all
  | release n =>
      simp only [step]
      split
      · rfl
      · split <;> simpa using h

theorem run_keeps_refused (st : State) (es : List Event) (h : st.refused = true) :
    (run st es).refused = true := by
  induction es generalizing st with
  | nil => simpa [run] using h
  | cons e es ih =>
      show (run (step st e) es).refused = true
      exact ih _ (step_keeps_refused st e h)

/-- Filtering out the slot's aliases keeps the slot's own binding. -/
theorem lookup_filter_keeps_slot (b : List (Nat × Nat)) (slot s : Nat) :
    lookup (b.filter fun p => !isAlias slot s p) slot = lookup b slot := by
  unfold lookup
  congr 1
  induction b with
  | nil => rfl
  | cons p ps ih =>
      simp only [List.filter_cons]
      by_cases hp : p.1 = slot
      · have hk : (!isAlias slot s p) = true := by simp [isAlias, hp]
        simp [hk, hp]
      · cases hk : (!isAlias slot s p) <;> simp [hp, ih]

/-- A move with the slot bound, as a record update. -/
theorem step_move (st : State) (slot s : Nat) (h : lookup st.bound slot = some s) :
    step st (.move slot false) =
      { st with bound := st.bound.filter (fun p => !isAlias slot s p),
                invalid := (st.bound.filter (isAlias slot s)).map (·.1) ++ st.invalid } := by
  simp [step, h]

/-- **The slot owns exactly one object before and after**: a move keeps the
slot's binding and the live obligations. -/
theorem the_slot_keeps_its_identity_and_its_obligation (st : State) (slot s : Nat)
    (h : lookup st.bound slot = some s) (failed : Bool) :
    lookup (step st (.move slot failed)).bound slot = some s ∧
      (step st (.move slot failed)).live = st.live := by
  cases failed with
  | true => simp [step, h]
  | false =>
      rw [step_move st slot s h]
      exact ⟨(lookup_filter_keeps_slot st.bound slot s).trans h, rfl⟩

/-- **Every alias of the old pointer is invalid after a move.** -/
theorem every_old_alias_is_invalid_after_a_move (st : State) (slot s a : Nat)
    (h : lookup st.bound slot = some s) (ha : (a, s) ∈ st.bound) (hne : a ≠ slot) :
    a ∈ (step st (.move slot false)).invalid := by
  rw [step_move st slot s h]
  apply List.mem_append_left
  exact List.mem_map.mpr ⟨(a, s), List.mem_filter.mpr ⟨ha, by simp [isAlias, hne]⟩, rfl⟩

/-- **A use of an old alias after a move is refused.** -/
theorem a_use_of_an_old_alias_is_refused (st : State) (slot s a : Nat)
    (h : lookup st.bound slot = some s) (ha : (a, s) ∈ st.bound) (hne : a ≠ slot) :
    (step (step st (.move slot false)) (.use a)).refused = true := by
  have hi := every_old_alias_is_invalid_after_a_move st slot s a h ha hne
  simp [step.eq_3, hi]

/-- **On failure the original stays owned by the slot**: nothing changes. -/
theorem the_failure_arm_keeps_the_original (st : State) (slot s : Nat)
    (h : lookup st.bound slot = some s) : step st (.move slot true) = st := by
  simp [step, h]

/-- **A `Safe` path never uses an invalid name**: if the whole path's replay
is not refused, the name a use reads was not invalid at that point. -/
theorem a_safe_path_never_uses_an_invalid_name (pre post : List Event) (a : Nat)
    (hsafe : (run State.empty (pre ++ .use a :: post)).refused = false) :
    (run State.empty pre).invalid.contains a = false := by
  rw [run_append] at hsafe
  cases hc : (run State.empty pre).invalid.contains a with
  | false => rfl
  | true =>
      have hm : a ∈ (run State.empty pre).invalid := by simpa using hc
      have hr : (step (run State.empty pre) (.use a)).refused = true := by
        simp [step, hm]
      have := run_keeps_refused _ post hr
      simp only [run, List.foldl_cons] at hsafe
      simp only [run] at this
      rw [this] at hsafe
      exact absurd hsafe (by simp)

/-! ## Shared vectors -/

/-- `SLOT_VECTORS` of `slot_move_borrow.rs`, row by row. -/
def slotVectors : List (SlotBinding × Bool × SlotOutcome) := [
  (.definitelyNull, true, .nothing),
  (.definitelyNull, false, .refuse),
  (.callerSlot, false, .nothing),
  (.oneTracked false, false, .move),
  (.oneTracked false, true, .move),
  (.oneTracked true, false, .refuse),
  (.other, true, .refuse),
  (.other, false, .refuse)
]

theorem slot_vectors_hold :
    slotVectors.all (fun (b, n, o) => decideSlot b n == o) = true := by
  decide

/-- `SLOT_TRACE_VECTORS`: each path, whether its replay refuses, and the
number of live obligations at its end. -/
def slotTraceVectors : List (List Event × Bool × Nat) := [
  ([.bind 0 1, .bind 1 1, .move 0 false, .use 1], true, 1),
  ([.bind 0 1, .move 0 false, .use 0, .release 0], false, 0),
  ([.bind 0 1, .bind 1 1, .move 0 true, .use 1], false, 1),
  ([.bind 0 1, .bind 1 1, .move 0 false, .release 1], true, 1),
  ([.move 0 false], true, 0)
]

theorem slot_trace_vectors_hold :
    slotTraceVectors.all (fun (es, refused, live) =>
      (run State.empty es).refused == refused && (run State.empty es).live.length == live) = true := by
  decide

end QCore.SlotMove
