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

import QCore.OpensslReleasedSlots

/-!
# A candidate's requirement folded at every position and clause form

Card **CALLBACK-MEMBER-RELEASED-REQUIREMENT-FOLD** (gateway rule 74j,
contract specification C04.19). Beside `QCore.OpensslReleasedSlots`, whose
`folded` strengthens a released slot object's nullability.

At a call, a candidate's `resource_owned(n)` requirement on a parameter
actual of the call is folded into the call's custody mode at that position
(`legacy_retained_by.rs`, `callback_contracts::fold_requirement`). Before
this card the fold was made at a borrowed position (rule 13u) and at a
released slot object (r60) only; at any other position the requirement was
dropped, so a nullable owned, consumed or released (plain member form)
position let a possibly null object reach a candidate requiring it live.

Modelled as the Rust code decides it:

* `Mode` is `ResourceParameterMode` with its nullability; `Form` the clause
  forms that put a requirement on the call (synchronous, table, plain member
  and slot invocation; the registry invocation and a deferred member reading
  refuse any requirement already and put none).
* `foldOld` is the base's fold, `fold` this card's: the same for every form,
  the requirement kept by `&&` at every held position, an out or removed
  position refused (`none`).
* `admits` is the custody precondition at the position: a null object meets
  it only where the mode is nullable.

Theorems: `the_old_fold_dropped_a_non_null_requirement` (the registered
defect, a plain member invocation through a nullable release);
`the_fold_never_drops_a_non_null_requirement`;
`the_old_fold_differed_by_form`; `the_fold_agrees_with_the_slot_fold`
(r60's released slot object, and `QCore.OpensslReleasedSlots.folded` for one
requirement); `the_fold_agrees_where_the_old_one_folded`;
`the_fold_only_strengthens` (no position is admitted that the declared mode
refused: no `Unknown` becomes `Safe`); `a_non_null_position_is_a_fixed_point`
(the shipped controls); `out_and_removed_refuse`; `fold_vectors_hold` (the
Rust unit test's rows).
-/

namespace QCore.CallbackRequirementFold

/-- `ResourceParameterMode`, with the nullability of each held mode. -/
inductive Mode where
  | borrowed
  | owned (nullable : Bool)
  | consumed (nullable : Bool)
  | released (nullable : Bool)
  | stored (nullable : Bool)
  | out (nullable : Bool)
  | removed (nullable : Bool)
  deriving DecidableEq, Repr

/-- The clause forms whose candidates put a requirement on the call. -/
inductive Form where
  | invoked
  | table
  | memberInvoked
  | slotInvoked
  deriving DecidableEq, Repr

/-- The base's fold: `slotObject` whether the position is a released slot
object (r60). Anything but a borrowed position or a released slot object
kept its declared mode: the requirement was dropped. -/
def foldOld (form : Form) (slotObject : Bool) (m : Mode) (n : Bool) : Mode :=
  match m with
  | .borrowed => .owned n
  | .released r => if form == .slotInvoked && slotObject then .released (r && n) else m
  | _ => m

/-- This card's fold, for every form: `none` refuses the call
(`callback-requirement-unfolded`). -/
def fold (m : Mode) (n : Bool) : Option Mode :=
  match m with
  | .borrowed => some (.owned n)
  | .owned r => some (.owned (r && n))
  | .consumed r => some (.consumed (r && n))
  | .released r => some (.released (r && n))
  | .stored r => some (.stored (r && n))
  | .out _ | .removed _ => none

/-- Whether a null object meets the mode's precondition (a borrowed
position requires nothing). -/
def nullAdmitted : Mode → Bool
  | .borrowed => true
  | .owned r | .consumed r | .released r | .stored r | .out r | .removed r => r

/-- The custody precondition on an actual that may be null (`maybeNull`)
and is otherwise held. -/
def admits (m : Mode) (maybeNull : Bool) : Bool := nullAdmitted m || !maybeNull

/-- The registered defect: a plain member invocation through
`resource_released(OBJ, nullable)`, whose candidate requires the object
non-null, kept the nullable release, so a possibly null object was
admitted. -/
theorem the_old_fold_dropped_a_non_null_requirement :
    admits (foldOld .memberInvoked false (.released true) false) true = true := by decide

/-- After the fold, a non-null requirement admits no possibly null actual,
whatever the position and form (or the call refuses). -/
theorem the_fold_never_drops_a_non_null_requirement {m m' : Mode}
    (h : fold m false = some m') : admits m' true = false := by
  cases m <;> simp [fold] at h <;> subst h <;> simp [admits, nullAdmitted]

/-- The base's fold read the form: at the same nullable released position
and requirement, the plain member form and the slot form disagreed. `fold`
takes no form, so every clause form folds alike. -/
theorem the_old_fold_differed_by_form :
    foldOld .memberInvoked false (.released true) false ≠
      foldOld .slotInvoked true (.released true) false := by decide

/-- At a released slot object the fold is r60's. -/
theorem the_fold_agrees_with_the_slot_fold (r n : Bool) :
    fold (.released r) n = some (foldOld .slotInvoked true (.released r) n) := by
  cases r <;> cases n <;> rfl

/-- And r60's `folded` over one requirement is this fold's nullability. -/
theorem the_slot_fold_is_one_requirement (r n : Bool) :
    fold (.released r) n =
      some (.released (QCore.OpensslReleasedSlots.folded r
        [QCore.OpensslReleasedSlots.Effect.requiresHeld n])) := by
  cases r <;> cases n <;> rfl

/-- Where the base folded (a borrowed position, a released slot object), the
fold is the base's. -/
theorem the_fold_agrees_where_the_old_one_folded (form : Form) (slotObject : Bool)
    (m : Mode) (n : Bool)
    (h : m = .borrowed ∨ (form = .slotInvoked ∧ slotObject = true ∧ ∃ r, m = .released r)) :
    fold m n = some (foldOld form slotObject m n) := by
  rcases h with h | ⟨hf, hs, r, hr⟩
  · subst h; rfl
  · subst hf hs hr; cases r <;> cases n <;> rfl

/-- The fold only strengthens a held position: whatever it admits, the
declared mode admitted. A borrowed position becomes the requirement itself
(rule 13u's reading, unchanged). -/
theorem the_fold_only_strengthens {m m' : Mode} {n maybeNull : Bool}
    (hm : m ≠ .borrowed) (h : fold m n = some m') (ha : admits m' maybeNull = true) :
    admits m maybeNull = true := by
  cases m <;> simp [fold] at h <;> (try exact absurd rfl hm) <;> subst h <;>
    cases maybeNull <;> simp_all [admits, nullAdmitted]

/-- A non-null held position is a fixed point: the shipped controls
(`rd_kafka_op_destroy` and `mk_thread_destroy` release non-null,
`rd_kafka_background_queue_serve` consumes non-null, `rd_kafka_poll_cb` owns
non-null) read as before. -/
theorem a_non_null_position_is_a_fixed_point (n : Bool) :
    fold (.released false) n = some (.released false) ∧
      fold (.consumed false) n = some (.consumed false) ∧
      fold (.owned false) n = some (.owned false) ∧
      fold (.stored false) n = some (.stored false) := by
  cases n <;> decide

/-- A nullable requirement changes no held position. -/
theorem a_nullable_requirement_changes_no_held_position (m : Mode) (hm : m ≠ .borrowed)
    (hh : fold m true ≠ none) : fold m true = some m := by
  cases m <;> simp_all [fold]

/-- An out or removed position states no held object: the call refuses. -/
theorem out_and_removed_refuse (r n : Bool) :
    fold (.out r) n = none ∧ fold (.removed r) n = none := ⟨rfl, rfl⟩

/-- The rows of the Rust unit test `the_requirement_fold_covers_every_mode`:
(mode, requirement nullable, result). -/
def foldVectors : List (Mode × Bool × Option Mode) :=
  [ (.borrowed, false, some (.owned false)), (.borrowed, true, some (.owned true)),
    (.owned true, false, some (.owned false)), (.owned true, true, some (.owned true)),
    (.consumed true, false, some (.consumed false)),
    (.released true, false, some (.released false)),
    (.released true, true, some (.released true)),
    (.stored true, false, some (.stored false)),
    (.released false, true, some (.released false)),
    (.out false, true, none), (.removed true, false, none) ]

theorem fold_vectors_hold : foldVectors.all (fun (m, n, r) => fold m n == r) = true := by
  decide

end QCore.CallbackRequirementFold
