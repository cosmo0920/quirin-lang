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
# Retained pointers (card CALLEE-RETENTION-CLASS1, stage (b), rule 12t)

The abstract rule implemented by `crates/qcore-c-gateway/src/retained_by.rs`
(`decide`, `replay`) and its route-G adapter `legacy_retained_by.rs`,
contract specification §C04.14.

USER DECISION 2026-10-05 ("split by shape", `W48-CALLEE-DECISION-TABLE`
class 1): a callee that keeps a pointer `P` the caller passed states
`resource_retained_by(P, holder)`. The caller keeps custody of `P`'s object,
but must establish that the object outlives the holder, or that the holder is
reset or destroyed before the object is released; otherwise the result is
never `Safe`.

## The model

* `Retained` is what the retained actual denotes: static storage, an object
  the caller of this function supplied, one identity the body tracks, or
  anything else. `Holder` is what holds it: an automatic object of this frame,
  one identity the body tracks, the program-lifetime (`global`) holder, or
  anything else.
* `decide` is the call-site table: outlives (nothing to record), a record, or
  undecided.
* A path is a list of `Event`s; `step` applies one to the live records, and a
  refusal is absorbing (`replay`). A release of an identity some live record
  holds refuses; a reset or release of a holder drops what it held; at the
  exit a frame holder dies and a tracked holder still holding refuses.

## The theorems

* `static_outlives_every_holder`, `undecided_is_refused`,
  `foreign_holder_is_undecided`, `global_holder_of_a_non_static_is_undecided`:
  the table.
* `a_safe_path_never_releases_a_held_identity`: on a path `replay` calls
  `Safe`, no release reaches an identity a live record holds — the
  use-after-free hazard is excluded on every path published `Safe`.
* `a_safe_path_leaves_no_tracked_holder_holding`: at the exit of a `Safe`
  path only frame holders remain, and those die with the frame.
* `release_before_reset_is_refused`, `reset_before_release_is_safe`,
  `a_holder_outliving_its_pointer_is_refused`, the global and unknown
  holders: the brief's controls.
* `decision_vectors_hold`, `trace_vectors_hold`: the tables shared with Rust
  (`DECISION_VECTORS`, `TRACE_VECTORS`).

## What this is not

A model of the decision, over abstract identities. That the gateway reads the
actuals as the spelling classes say (the frame holder's single automatic
declaration and its holder-only mentions, a string literal at a pointer
position), that the transfer's records join by union, and the correspondence
with the compiled Rust are review premises (trust boundary 28), pinned by the
shared vectors and `crates/qcore-c-gateway/tests/callee_retention_class1.rs`.
A refused release is never a defect claim: a dangling pointer left in a
holder is a hazard only if the holder reads it.
-/

namespace QCore.RetainedBy

/-- What the retained pointer's actual denotes. -/
inductive Retained where
  | static
  | callerObject (name : Nat)
  | tracked (site : Nat)
  | other
  deriving DecidableEq, Repr

/-- What the holder's actual denotes. -/
inductive Holder where
  | frame (name : Nat)
  | tracked (site : Nat)
  | global
  | foreign
  deriving DecidableEq, Repr

/-- What a record keys on. -/
inductive HeldObject where
  | site (site : Nat)
  | name (name : Nat)
  deriving DecidableEq, Repr

/-- What holds a record. -/
inductive HeldBy where
  | frame (name : Nat)
  | site (site : Nat)
  deriving DecidableEq, Repr

inductive Decision where
  | outlives
  | record (object : HeldObject) (by_ : HeldBy)
  | undecided
  deriving DecidableEq, Repr

/-- Rule 12t's table (`retained_by::decide`). -/
def decide : Retained → Holder → Decision
  | .static, _ => .outlives
  | .other, _ => .undecided
  | _, .foreign => .undecided
  | _, .global => .undecided
  | .callerObject _, .frame _ => .outlives
  | .callerObject n, .tracked h => .record (.name n) (.site h)
  | .tracked s, .frame l => .record (.site s) (.frame l)
  | .tracked s, .tracked h => .record (.site s) (.site h)

theorem static_outlives_every_holder (h : Holder) : decide .static h = .outlives := by
  cases h <;> rfl

theorem foreign_holder_is_undecided (r : Retained) (hr : r ≠ .static) :
    decide r .foreign = .undecided := by
  cases r <;> simp_all [decide]

theorem global_holder_of_a_non_static_is_undecided (r : Retained) (hr : r ≠ .static) :
    decide r .global = .undecided := by
  cases r <;> simp_all [decide]

/-- A caller's object outlives this frame, so a frame holder needs no record. -/
theorem a_caller_object_outlives_a_frame_holder (n l : Nat) :
    decide (.callerObject n) (.frame l) = .outlives := rfl

/-- One event of a path. -/
inductive Event where
  | retain (d : Decision)
  | release (site : Nat)
  | resetFrame (name : Nat)
  | resetSite (site : Nat)
  deriving DecidableEq, Repr

abbrev Records := List (HeldObject × HeldBy)

/-- Whether a live record holds the identity `s`. -/
def holds (rs : Records) (s : Nat) : Bool := rs.any fun r => r.1 == .site s

/-- Drop the records a holder held. -/
def drop (rs : Records) (b : HeldBy) : Records := rs.filter fun r => r.2 != b

/-- One step: the new records, or `none` for a refusal. -/
def step (rs : Records) : Event → Option Records
  | .retain .outlives => some rs
  | .retain .undecided => none
  | .retain (.record o b) => some ((o, b) :: rs)
  | .release s => if holds rs s then none else some (drop rs (.site s))
  | .resetFrame l => some (drop rs (.frame l))
  | .resetSite h => some (drop rs (.site h))

/-- The records after a path, or `none` once a step refused. -/
def run (rs : Records) : List Event → Option Records
  | [] => some rs
  | e :: es => (step rs e).bind fun rs' => run rs' es

/-- Whether a record names a tracked holder. -/
def tracked? (r : HeldObject × HeldBy) : Bool :=
  match r.2 with
  | .site _ => true
  | .frame _ => false

/-- `retained_by::replay`: `Safe` when no step refused and no tracked holder
still holds at the exit (a frame holder dies with the frame). -/
def replay (es : List Event) : Bool :=
  match run [] es with
  | none => false
  | some rs => !(rs.any tracked?)

theorem run_append (rs : Records) (pre post : List Event) :
    run rs (pre ++ post) = (run rs pre).bind fun r => run r post := by
  induction pre generalizing rs with
  | nil => simp [run]
  | cons e es ih =>
      simp only [List.cons_append, run]
      cases step rs e with
      | none => rfl
      | some r => simp only [Option.bind_some]; exact ih r

theorem run_none (post : List Event) : (none : Option Records).bind (fun r => run r post) = none :=
  rfl

/-- **An undecided retention is refused**, wherever it stands on the path. -/
theorem undecided_is_refused (pre post : List Event) :
    replay (pre ++ .retain .undecided :: post) = false := by
  unfold replay
  rw [run_append]
  cases run [] pre with
  | none => rfl
  | some r => simp [run, step]

/-- A release of an identity a live record holds is the one refused step. -/
theorem release_refused_iff (rs : Records) (s : Nat) :
    step rs (.release s) = none ↔ holds rs s = true := by
  simp only [step]
  split <;> simp_all

/-- **On a `Safe` path no release reaches an identity a live record holds**:
whatever the prefix before a release leaves recorded, it does not hold the
released identity. -/
theorem a_safe_path_never_releases_a_held_identity (pre post : List Event) (s : Nat)
    (hsafe : replay (pre ++ .release s :: post) = true) (rs : Records)
    (hpre : run [] pre = some rs) : holds rs s = false := by
  unfold replay at hsafe
  rw [run_append, hpre] at hsafe
  simp only [Option.bind_some, run] at hsafe
  cases hh : holds rs s with
  | false => rfl
  | true =>
      have : step rs (.release s) = none := (release_refused_iff rs s).mpr hh
      rw [this] at hsafe
      simp at hsafe

/-- **At the exit of a `Safe` path no tracked holder still holds**: what is
left is held by frame holders, which end with the frame. -/
theorem a_safe_path_leaves_no_tracked_holder_holding (es : List Event)
    (hsafe : replay es = true) :
    ∃ rs, run [] es = some rs ∧ ∀ r ∈ rs, tracked? r = false := by
  unfold replay at hsafe
  cases h : run [] es with
  | none => simp [h] at hsafe
  | some rs =>
      refine ⟨rs, rfl, fun r hr => ?_⟩
      rw [h] at hsafe
      simp only [Bool.not_eq_true', List.any_eq_false] at hsafe
      cases ht : tracked? r with
      | false => rfl
      | true => exact absurd ht (hsafe r hr)

/-! ## The brief's controls -/

/-- A tracked `P` held by a frame holder that is reset before `P` is
released: `Safe`. -/
theorem reset_before_release_is_safe :
    replay [.retain (decide (.tracked 1) (.frame 0)), .resetFrame 0, .release 1] = true := by
  decide

/-- Released before the holder is reset: never `Safe`. -/
theorem release_before_reset_is_refused :
    replay [.retain (decide (.tracked 1) (.frame 0)), .release 1, .resetFrame 0] = false := by
  decide

/-- A tracked heap holder released before `P`: `Safe`. -/
theorem a_holder_released_first_is_safe :
    replay [.retain (decide (.tracked 1) (.tracked 2)), .release 2, .release 1] = true := by
  decide

/-- **A holder outliving `P`**: `P` released while the holder still holds it,
never `Safe`. -/
theorem a_holder_outliving_its_pointer_is_refused :
    replay [.retain (decide (.tracked 1) (.tracked 2)), .release 1, .release 2] = false := by
  decide

/-- A tracked holder still holding at the exit outlives the frame. -/
theorem a_holder_holding_at_the_exit_is_refused :
    replay [.retain (decide (.callerObject 7) (.tracked 2))] = false := by
  decide

/-- A frame holder dies at the exit with what it holds. -/
theorem a_frame_holder_dies_at_the_exit :
    replay [.retain (decide (.tracked 1) (.frame 0))] = true := by
  decide

/-- **A global holder**: a string literal outlives it; a tracked pointer does
not, and neither does a caller's object. -/
theorem global_holder_controls :
    replay [.retain (decide .static .global)] = true ∧
      replay [.retain (decide (.tracked 1) .global)] = false ∧
      replay [.retain (decide (.callerObject 7) .global)] = false := by
  decide

/-- **An unknown holder lifetime**: a holder the caller supplied is never
`Safe`. -/
theorem an_unknown_holder_is_refused (r : Retained) (hr : r ≠ .static) :
    replay [.retain (decide r .foreign)] = false := by
  rw [foreign_holder_is_undecided r hr]
  decide

/-! ## Shared vectors -/

inductive RClass where
  | static
  | caller
  | tracked
  | other
  deriving DecidableEq, Repr

inductive HClass where
  | frame
  | tracked
  | global
  | foreign
  deriving DecidableEq, Repr

inductive Label where
  | outlives
  | record
  | undecided
  deriving DecidableEq, Repr

def RClass.sample : RClass → Retained
  | .static => .static
  | .caller => .callerObject 0
  | .tracked => .tracked 1
  | .other => .other

def HClass.sample : HClass → Holder
  | .frame => .frame 0
  | .tracked => .tracked 2
  | .global => .global
  | .foreign => .foreign

def label : Decision → Label
  | .outlives => .outlives
  | .record .. => .record
  | .undecided => .undecided

/-- `DECISION_VECTORS` of `retained_by.rs`, row by row. -/
def decisionVectors : List (RClass × HClass × Label) := [
  (.static, .frame, .outlives),
  (.static, .tracked, .outlives),
  (.static, .global, .outlives),
  (.static, .foreign, .outlives),
  (.caller, .frame, .outlives),
  (.caller, .tracked, .record),
  (.caller, .global, .undecided),
  (.caller, .foreign, .undecided),
  (.tracked, .frame, .record),
  (.tracked, .tracked, .record),
  (.tracked, .global, .undecided),
  (.tracked, .foreign, .undecided),
  (.other, .frame, .undecided),
  (.other, .tracked, .undecided),
  (.other, .global, .undecided),
  (.other, .foreign, .undecided)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun (r, h, l) => label (decide r.sample h.sample) == l) = true := by
  decide

/-- `TRACE_VECTORS` of `retained_by.rs`, each path as `retained_by::trace`
builds it, with whether `replay` calls it `Safe`. -/
def traceVectors : List (List Event × Bool) := [
  ([.retain (decide (.tracked 1) (.frame 0)), .resetFrame 0, .release 1], true),
  ([.retain (decide (.tracked 1) (.frame 0)), .release 1, .resetFrame 0], false),
  ([.retain (decide (.tracked 1) (.tracked 2)), .release 2, .release 1], true),
  ([.retain (decide (.tracked 1) (.tracked 2)), .release 1, .release 2], false),
  ([.retain (decide (.callerObject 7) (.tracked 2))], false),
  ([.retain (decide (.tracked 1) (.frame 0))], true),
  ([.retain (decide .static .global)], true),
  ([.retain (decide (.tracked 1) .global)], false),
  ([.retain (decide (.callerObject 7) .foreign)], false)
]

theorem trace_vectors_hold :
    traceVectors.all (fun (es, safe) => replay es == safe) = true := by
  decide

end QCore.RetainedBy
