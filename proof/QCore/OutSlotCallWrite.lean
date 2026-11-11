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
# A call that may write caller memory poisons a bound out slot (card OUT-SLOT-CALL-WRITE-ALIASING)

The decision implemented by `crates/qcore-c-gateway/src/legacy_out_slot_call_write.rs`
(`may_reach`, `Frame::writes`) and its application in `legacy_transfer.rs`'s
`transfer`, gateway rule 11r(8).

An out slot `*X` is caller memory. A call whose actual designates the slot's
object, or a byte of it, may leave any value there (`memcpy`'s copy is
character-typed, so no effective-type rule excludes it). The gateway reads
each actual's derivation into a class; an *excluded* class (a scalar, a null
pointer constant, a string literal, a pointer to `const`, the address of a
pointer-free object of the frame, a position the callee's own `Out` mode
states) designates no caller object (`Premises.excluded_designates_nothing`,
the reading the rule states). A `Borrowed` write transfers no tracked
obligation, so the modelled call leaves only data in what it reaches
(`effect`). The abstract slot is unwritten, bound to a delivered site, or
ambiguous:

* the transfer keeps the slot when no actual reaches (`kept_when_no_actual_reaches`)
  and poisons a bound slot otherwise (`a_reaching_call_poisons_a_bound_slot`);
* the transfer is sound: every concrete value the call can leave is in the
  post-state's concretization (`transfer_sound`), including an unwritten slot
  kept unwritten (`unwritten_stays_sound`);
* a bound slot survives only when every actual is excluded
  (`bound_kept_only_when_excluded`);
* the reading before the card, which kept the slot across every `Borrowed`
  call, is unsound: a reaching actual can leave data where the slot claims a
  site (`the_old_reading_is_unsound`).

## What this is not

A model of one call's effect on one slot, not of C's object model or of the
whole analysis. That Clang's types and the AST's actual derivations are what
the build compiles (trust boundary 28), that a pointer to `const` is not
written through by the callee, and that a callee's `Out` position writes only
the object it states are the rule's stated readings, carried here as the
`Premises` hypotheses, not proved.
-/

namespace QCore.OutSlotCallWrite

/-- The gateway's reading of one call actual (`may_reach`). -/
inductive Reach where
  | scalar
  | null
  | string
  | constPointee
  | frameObject
  | statedOut
  | formal
  | localValue
  | member
  | other
  deriving DecidableEq, Repr

/-- The classes that designate no caller object. -/
def excluded : Reach → Bool
  | .scalar | .null | .string | .constPointee | .frameObject | .statedOut => true
  | _ => false

/-- `Frame::writes`: some actual of the call is not excluded. -/
def writes (actuals : List Reach) : Bool :=
  actuals.any fun a => !excluded a

/-- What the slot may hold: a delivered site, or data (no custody). -/
inductive Val where
  | site (s : Nat)
  | data (d : Nat)
  deriving DecidableEq, Repr

/-- The analyser's slot binding: absent (unwritten), bound, or poisoned. -/
inductive Abs where
  | unwritten
  | bound (s : Nat)
  | ambiguous
  deriving DecidableEq, Repr

/-- The concrete values an abstract slot admits; `orig` is the caller's
value at entry, which an unwritten slot keeps or a data write replaces. -/
def admits (orig : Val) : Abs → Val → Prop
  | .unwritten, v => v = orig ∨ ∃ d, v = .data d
  | .bound s, v => v = .site s
  | .ambiguous, _ => True

/-- The transfer after the call's own effect: a reaching call poisons a
bound slot; an unwritten or poisoned slot is left as it is. -/
def transfer (actuals : List Reach) : Abs → Abs
  | .bound s => if writes actuals then .ambiguous else .bound s
  | a => a

/-- A concrete call over the slot: each actual's class and whether it
designates the slot's object; the value left is the old one unless a
designating actual wrote data `d`. -/
structure Call where
  actuals : List (Reach × Bool)
  d : Nat

def classes (c : Call) : List Reach := c.actuals.map Prod.fst

def designatesSlot (c : Call) : Bool := c.actuals.any Prod.snd

def effect (c : Call) (v : Val) : Val :=
  if designatesSlot c then .data c.d else v

/-- The readings the rule states (trust boundary 28 and the module's list). -/
structure Premises (c : Call) : Prop where
  excluded_designates_nothing : ∀ p ∈ c.actuals, excluded p.1 = true → p.2 = false

theorem no_reaching_actual_designates (c : Call) (h : Premises c)
    (hw : writes (classes c) = false) : designatesSlot c = false := by
  unfold writes classes at hw
  unfold designatesSlot
  rw [List.any_eq_false] at hw ⊢
  intro p hp
  have hx : excluded p.1 = true := by
    have := hw p.1 (List.mem_map.mpr ⟨p, hp, rfl⟩)
    simpa using this
  simp [h.excluded_designates_nothing p hp hx]

theorem kept_when_no_actual_reaches (actuals : List Reach) (a : Abs)
    (hw : writes actuals = false) : transfer actuals a = a := by
  cases a <;> simp [transfer, hw]

theorem a_reaching_call_poisons_a_bound_slot (actuals : List Reach) (s : Nat)
    (hw : writes actuals = true) : transfer actuals (.bound s) = .ambiguous := by
  simp [transfer, hw]

theorem bound_kept_only_when_excluded (actuals : List Reach) (s : Nat)
    (h : transfer actuals (.bound s) = .bound s) : ∀ a ∈ actuals, excluded a = true := by
  intro a ha
  cases hw : writes actuals
  · unfold writes at hw
    rw [List.any_eq_false] at hw
    simpa using hw a ha
  · simp [transfer, hw] at h

theorem unwritten_stays_sound (orig v : Val) (c : Call)
    (hv : admits orig .unwritten v) : admits orig .unwritten (effect c v) := by
  unfold effect
  split
  · exact Or.inr ⟨c.d, rfl⟩
  · exact hv

theorem transfer_sound (orig v : Val) (a : Abs) (c : Call) (h : Premises c)
    (hv : admits orig a v) : admits orig (transfer (classes c) a) (effect c v) := by
  cases hw : writes (classes c)
  · rw [kept_when_no_actual_reaches _ a hw]
    simp [effect, no_reaching_actual_designates c h hw, hv]
  · cases a with
    | unwritten => exact unwritten_stays_sound orig v c hv
    | bound s => simp [transfer, hw, admits]
    | ambiguous => simp [transfer, admits]

/-- Before the card a `Borrowed` call kept every slot: `transfer` was the
identity. A reaching actual that designates the slot then leaves data where
the slot claims a delivered site. -/
def oldTransfer (_actuals : List Reach) (a : Abs) : Abs := a

theorem the_old_reading_is_unsound :
    ∃ (c : Call) (orig v : Val) (s : Nat), Premises c ∧ admits orig (.bound s) v ∧
      ¬ admits orig (oldTransfer (classes c) (.bound s)) (effect c v) := by
  refine ⟨⟨[(.formal, true)], 0⟩, .data 1, .site 7, 7, ?_, rfl, ?_⟩
  · constructor
    intro p hp hx
    simp at hp
    subst hp
    simp [excluded] at hx
  · simp [oldTransfer, admits, effect, designatesSlot]

theorem the_memcpy_shape_poisons :
    transfer [.formal, .frameObject, .scalar] (.bound 3) = .ambiguous := by
  decide

theorem distinct_locals_keep :
    transfer [.frameObject, .frameObject, .scalar] (.bound 3) = .bound 3 := by
  decide

theorem a_const_source_keeps :
    transfer [.frameObject, .constPointee, .scalar] (.bound 3) = .bound 3 := by
  decide

end QCore.OutSlotCallWrite
