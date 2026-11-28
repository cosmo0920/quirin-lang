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

import QCore.OutSlotKeepOrDeliver

/-!
# A member moved out of its owner, and a slot delivered or left unwritten (card OUT-SLOT-MEMBER-THEN-FRESH)

USER DECISION 2026-11-28: option (a1)+(b) for `rd_kafka_assignment` /
`rd_kafka_subscription`, which move an op's member into the caller's slot and
store a fresh list when that is null.

The module has three parts.

1. **(a1) The owned member of a result** (contract specification C04.19.4,
   `resource_member_owned(result, TAG.PATH) when FIELD == VALUE`): a result
   state is admitted on a nullable result only beside an owned member, and
   never with the liveness part (`stateAdmitted`); the owned state is put on
   the object only where the decided discriminant is exactly the guard
   (`ownedAt`).
2. **Gateway rule 96, the read-and-null move-out** (`reading`, bound to the
   Rust `MOVE_OUT_VECTORS` of `legacy_member_move_out.rs`, run against its
   `reading`): the read takes the member's object only from a link the body
   holds alone (an alias) or from an unspent owned-member state in a body
   that writes the member only to null it (a fresh, possibly null object);
   everything else is the read's own lowering. The object's custody is then
   held exactly once: by the reader, the cleared member giving the owner's
   destroy nothing to release (`one_holder_after_the_move`).
3. **Rule 74l's amendment (mode (b)), delivers or leaves unwritten**
   (`slotMode`, bound to `SLOT_MODE_VECTORS` of
   `legacy_keep_or_deliver_summary.rs`, run against its `mode`): when the
   null entry reads W4's `out` mode and the owned entry fails, the summary is
   read only by a caller whose slot is known null; any other caller refuses.

## What this is not

A model of the transfer function, the Clang reading or the CFG. That the
runs' observations are those of the body, that the next statement is the
null store, and that the declared owned member is the object the owner's
destroy would free are the gateway's and the declaration's (trust boundary
28; the declared assumption C04.19.4 states).
-/

namespace QCore.MemberMoveOut

open QCore.CustodySummary (OutSlot)
open QCore.OwnedOutFormalDelivery (outMode live)
open QCore.OutSlotKeepOrDeliver (KeptExit keepOrDeliver Entry Outcome realOwedAfter owedAfter)

/-! ## 1. (a1) The owned member of a result -/

/-- The result mode a declaration states. -/
inductive ResultMode where
  | ownedNonnull
  | ownedNullable
  | other
  deriving DecidableEq, Repr

/-- Whether a declaration's result states load: a non-null owned result as
before; a nullable owned one only beside an owned member and without the
liveness part. -/
def stateAdmitted (result : ResultMode) (statesOwned statesLive : Bool) : Bool :=
  match result with
  | .ownedNonnull => true
  | .ownedNullable => statesOwned && !statesLive
  | .other => false

theorem the_nonnull_reading_is_unchanged (owned live : Bool) :
    stateAdmitted .ownedNonnull owned live = true := rfl

theorem a_nullable_result_needs_an_owned_member (live : Bool) :
    stateAdmitted .ownedNullable false live = false := by
  cases live <;> rfl

theorem liveness_still_needs_a_nonnull_result (owned : Bool) :
    stateAdmitted .ownedNullable owned true = false := by
  cases owned <;> rfl

theorem no_other_result_carries_a_state (owned live : Bool) :
    stateAdmitted .other owned live = false := rfl

/-- The owned-member state at a constructor call: only where the decided
discriminant is exactly the guard. -/
def ownedAt (guard : Int) (decided : Option Int) : Bool :=
  decided == some guard

theorem an_undecided_discriminant_states_no_owned_member (guard : Int) :
    ownedAt guard none = false := rfl

theorem another_value_states_no_owned_member (guard value : Int) (other : value ≠ guard) :
    ownedAt guard (some value) = false := by
  simp [ownedAt, other]

/-! ## 2. Rule 96, the read-and-null move-out -/

/-- The member's link at the read, as the flow sees it. -/
inductive Link where
  | none
  | oneLive
  | onSomePaths
  | several
  deriving DecidableEq, Repr

/-- What the read becomes. -/
inductive Reading where
  /-- The reader aliases the linked child. -/
  | alias
  /-- The reader takes a fresh, possibly null object. -/
  | freshNullable
  /-- The read's own lowering. -/
  | fallback
  deriving DecidableEq, Repr

/-- The reading: `state` an unspent owned-member state, `sanctioned` the
destination may receive custody, `admitted` the body writes the member only
to null it after its move-outs. -/
def reading (link : Link) (state sanctioned admitted : Bool) : Reading :=
  if !sanctioned then .fallback
  else
    match link with
    | .oneLive => .alias
    | .none => if state && admitted then .freshNullable else .fallback
    | _ => .fallback

/-- The Rust `MOVE_OUT_VECTORS`. -/
def moveOutVectors : List (Link × Bool × Bool × Bool × Reading) := [
  (.oneLive, false, true, false, .alias),
  (.oneLive, true, true, true, .alias),
  (.onSomePaths, true, true, true, .fallback),
  (.several, false, true, false, .fallback),
  (.none, true, true, true, .freshNullable),
  (.none, true, true, false, .fallback),
  (.none, false, true, true, .fallback),
  (.none, true, false, true, .fallback),
  (.oneLive, false, false, false, .fallback)
]

theorem move_out_vectors_hold :
    moveOutVectors.all (fun (l, s, c, a, r) => reading l s c a == r) = true := by
  decide

/-- A fresh object only from an unspent state, no link, an admitted body and
a sanctioned destination. -/
theorem a_fresh_object_only_from_a_stated_member (link : Link) (state sanctioned admitted : Bool)
    (fresh : reading link state sanctioned admitted = .freshNullable) :
    link = .none ∧ state = true ∧ sanctioned = true ∧ admitted = true := by
  cases sanctioned <;> cases link <;> cases state <;> cases admitted <;>
    simp_all [reading]

/-- An alias only of a link the body holds alone. -/
theorem an_alias_only_of_a_link_held_alone (link : Link) (state sanctioned admitted : Bool)
    (aliased : reading link state sanctioned admitted = .alias) : link = .oneLive := by
  cases sanctioned <;> cases link <;> cases state <;> cases admitted <;> simp_all [reading]

/-- A link that may not hold on every path never decides. -/
theorem a_partial_link_reads_the_fallback (state sanctioned admitted : Bool) :
    reading .onSomePaths state sanctioned admitted = .fallback := by
  cases sanctioned <;> rfl

/-- A spent state (after the first move-out on the path) gives a second
move-out of an unlinked member nothing. -/
theorem a_second_move_out_takes_nothing (sanctioned admitted : Bool) :
    reading .none false sanctioned admitted = .fallback := by
  cases sanctioned <;> rfl

/-- Who holds the member's object after the pair and the owner's destroy:
`(reader, destroy)`. The destroy's arm frees what the member still holds,
and a moved member was cleared. -/
def holders (moved : Bool) : Bool × Bool :=
  if moved then (true, false) else (false, true)

/-- Exactly one holder, either way: the move-out never mints a second
custody, and without it the destroy still frees the member. -/
theorem one_holder_after_the_move (moved : Bool) :
    ((holders moved).1 && !(holders moved).2) || (!(holders moved).1 && (holders moved).2) := by
  cases moved <;> rfl

/-- Without the null store the destroy would free what the reader holds: the
pair needs it, which is why the lowering requires the next statement. -/
theorem an_uncleared_member_is_held_twice :
    (true, true) ≠ holders true ∧ (true, true) ≠ holders false := by
  decide

/-! ## 3. Rule 74l's amendment: delivers or leaves unwritten -/

/-- The summary a body reads. -/
inductive SlotMode where
  /-- Rule 85: both entries validate. -/
  | keepOrDeliver (nullable : Bool)
  /-- Mode (b): only the null entry; read only into a known-null slot. -/
  | nullOnly (nullable : Bool)
  | none
  deriving DecidableEq, Repr

/-- The mode the runs read: rule 85's where both entries validate, the null
entry's W4 `out` mode alone where only it does, nothing otherwise. -/
def slotMode (nullRun : List OutSlot) (ownedRun : List KeptExit) : SlotMode :=
  match keepOrDeliver nullRun ownedRun with
  | some n => .keepOrDeliver n
  | none =>
    match outMode nullRun with
    | some n => .nullOnly n
    | none => .none

/-- The Rust `SLOT_MODE_VECTORS`. -/
def slotModeVectors : List (List OutSlot × List KeptExit × SlotMode) := [
  -- `if (!*out) *out = make();`: rule 85.
  ([live], [true], .keepOrDeliver false),
  -- Early unwritten exits, then a delivery over the slot: mode (b).
  ([.null, .null, live, live], [true, true, false, false], .nullOnly true),
  -- An every-exit store over the slot: W4's `out` in the null run, mode (b)
  -- for the summary table (the ordinary W4 route publishes it first).
  ([live], [false], .nullOnly false),
  -- A possibly-null delivery: nothing.
  ([.site false .live true], [false], .none),
  -- A slot holding another object's value: nothing.
  ([.ambiguous], [false], .none),
  -- Only null: nothing delivered.
  ([.null], [true], .none),
  ([], [], .none)
]

theorem slot_mode_vectors_hold :
    slotModeVectors.all (fun (n, o, m) => slotMode n o == m) = true := by
  decide

/-- Mode (b) never weakens the null run: what W4's `out` reader refuses
publishes nothing. -/
theorem a_null_run_refusal_publishes_nothing (nullRun : List OutSlot) (ownedRun : List KeptExit)
    (refused : outMode nullRun = none) : slotMode nullRun ownedRun = .none := by
  unfold slotMode
  rw [QCore.OutSlotKeepOrDeliver.a_null_run_refusal_is_kept nullRun ownedRun refused]
  simp [refused]

/-- The caller's reading under mode (b): only a known-null slot is read. -/
def nullOnlyReading (nullable : Bool) : Entry → Outcome
  | .null => .delivered nullable
  | _ => .refused

theorem a_null_only_mode_delivers_only_into_a_known_null_slot (nullable : Bool) (entry : Entry) :
    (∃ n, nullOnlyReading nullable entry = .delivered n) ↔ entry = .null := by
  cases entry <;> simp [nullOnlyReading]

/-- A held object is never kept under mode (b): its owned run failed. -/
theorem a_held_object_is_refused (nullable : Bool) (site : Nat) :
    nullOnlyReading nullable (.owned site) = .refused := rfl

/-- On the entry it reads, the reading is the callee's effect. -/
theorem the_null_only_reading_is_the_effect (nullable : Bool) (fresh : Nat) (prior : List Nat) :
    owedAfter fresh prior (nullOnlyReading nullable .null) = realOwedAfter fresh prior .null :=
  rfl

/-- A partial class cover stays refused: the null run reads no mode when an
exit leaves the slot as the caller had it (`unwritten`), so a slot written on
some exits only reads nothing unless the caller seeded it null. -/
theorem an_unseeded_partial_store_reads_nothing (ownedRun : List KeptExit) :
    slotMode [live, .unwritten] ownedRun = .none := by
  have : outMode [live, .unwritten] = none := by decide
  exact a_null_run_refusal_publishes_nothing _ _ this

end QCore.MemberMoveOut
