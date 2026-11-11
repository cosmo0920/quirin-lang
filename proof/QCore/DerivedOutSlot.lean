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
# Derived out slots (card SLOT-MOVE-AND-INTERIOR-BORROW, rule 12z)

The abstract rule implemented by `crates/qcore-c-gateway/src/slot_move_borrow.rs`
(`decide_out`) and its route-G adapter `legacy_slot_move_borrow.rs`, contract
specification §C04.16, absorbing `INTERIOR-OUT-SLOT-MODE`.

USER DECISION 2026-10-05: the slot receives a pointer derived from parameter
`k`, owning nothing, under §C04.3.1's rules.

## The model

* `decideOut` is the call-site table over what the slot actual held before
  the call: no tracked identity (the slot is bound to a pointer derived from
  the base, or keeps its old untracked value), a tracked identity (refused: the
  write could lose it), a NULL slot (nothing, under `nullable`), the caller's
  slot passed through (nothing here), anything else (refused).
* A path over one base and one slot: a write binds the slot to the base; the
  slot's value owns nothing, so its release is refused; the base's release
  invalidates it, and a later use is refused (refused rather than accused,
  because the slot may still hold its old value).

## The theorems

* `a_tracked_slot_is_never_overwritten`, `a_bind_needs_an_untracked_slot`.
* `the_slot_value_is_never_released`, `the_base_release_ends_the_slot_value`.
* `a_safe_path_never_uses_a_value_its_base_outlived`.
* `out_vectors_hold`: the table shared with Rust.
-/

namespace QCore.DerivedOutSlot

inductive Before where
  | untracked
  | tracked
  | nullSlot
  | callerSlot
  | other
  deriving DecidableEq, Repr

inductive Outcome where
  | bind
  | nothing
  | refuse
  deriving DecidableEq, Repr

/-- Rule 12z's table (`slot_move_borrow::decide_out`). -/
def decideOut : Before → Bool → Outcome
  | .untracked, _ => .bind
  | .nullSlot, true => .nothing
  | .nullSlot, false => .refuse
  | .callerSlot, _ => .nothing
  | .tracked, _ => .refuse
  | .other, _ => .refuse

/-- **A slot holding a tracked identity is never overwritten**: the call is
refused, so no custody is lost through the write. -/
theorem a_tracked_slot_is_never_overwritten (n : Bool) : decideOut .tracked n = .refuse := rfl

theorem a_bind_needs_an_untracked_slot (b : Before) (n : Bool) (h : decideOut b n = .bind) :
    b = .untracked := by
  cases b <;> cases n <;> simp_all [decideOut]

/-- The slot's value over one base: whether it is bound, and whether the base
was released since. -/
structure State where
  bound : Bool
  baseReleased : Bool
  refused : Bool
  deriving DecidableEq, Repr

def State.empty : State := ⟨false, false, false⟩

inductive Event where
  | write
  | useValue
  | releaseValue
  | releaseBase
  deriving DecidableEq, Repr

def step (st : State) : Event → State
  | .write => { st with bound := true, baseReleased := false }
  | .useValue => if st.bound && st.baseReleased then { st with refused := true } else st
  | .releaseValue => if st.bound then { st with refused := true } else st
  | .releaseBase => { st with baseReleased := true }

def run (st : State) (es : List Event) : State := es.foldl step st

theorem run_append (st : State) (pre post : List Event) :
    run st (pre ++ post) = run (run st pre) post := by
  simp [run, List.foldl_append]

theorem step_keeps_refused (st : State) (e : Event) (h : st.refused = true) :
    (step st e).refused = true := by
  cases e <;> simp only [step] <;> (try split) <;> simp_all

theorem run_keeps_refused (st : State) (es : List Event) (h : st.refused = true) :
    (run st es).refused = true := by
  induction es generalizing st with
  | nil => simpa [run] using h
  | cons e es ih =>
      show (run (step st e) es).refused = true
      exact ih _ (step_keeps_refused st e h)

/-- **The slot's value owns nothing**: releasing it is refused. -/
theorem the_slot_value_is_never_released (st : State) (h : st.bound = true) :
    (step st .releaseValue).refused = true := by
  simp [step, h]

/-- **Releasing the base ends the slot's value**: a later use is refused. -/
theorem the_base_release_ends_the_slot_value (st : State) (h : st.bound = true) :
    (step (step st .releaseBase) .useValue).refused = true := by
  simp [step, h]

/-- **A `Safe` path never uses a value whose base it released.** -/
theorem a_safe_path_never_uses_a_value_its_base_outlived (pre post : List Event)
    (hsafe : (run State.empty (pre ++ .useValue :: post)).refused = false) :
    ((run State.empty pre).bound && (run State.empty pre).baseReleased) = false := by
  rw [run_append] at hsafe
  cases hc : ((run State.empty pre).bound && (run State.empty pre).baseReleased) with
  | false => rfl
  | true =>
      have hr : (step (run State.empty pre) .useValue).refused = true := by
        simp [step, hc]
      have := run_keeps_refused _ post hr
      simp only [run, List.foldl_cons] at hsafe
      simp only [run] at this
      rw [this] at hsafe
      exact absurd hsafe (by simp)

/-- `OUT_VECTORS` of `slot_move_borrow.rs`, row by row. -/
def outVectors : List (Before × Bool × Outcome) := [
  (.untracked, false, .bind),
  (.untracked, true, .bind),
  (.tracked, true, .refuse),
  (.nullSlot, true, .nothing),
  (.nullSlot, false, .refuse),
  (.callerSlot, false, .nothing),
  (.other, true, .refuse)
]

theorem out_vectors_hold :
    outVectors.all (fun (b, n, o) => decideOut b n == o) = true := by
  decide

end QCore.DerivedOutSlot
