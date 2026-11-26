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

import QCore.OwnedOutFormalDelivery

/-!
# The keep-or-deliver summary of an in-out slot (card OUT-SLOT-CONDITIONAL-DELIVERY, gateway rule 85)

USER DECISION 2026-11-25: "lanich four of queued cards", choosing the
return-family and this-lane set, which includes this card: build the
keep-or-deliver mode with Lean parity.

`if (!*out) *out = make();` writes the slot only where it held null. W4's
`out` mode needs a store on every exit (`QCore.OwnedOutFormalDelivery`), so
it reads nothing here; the card states a mode of its own.

The module has four parts.

1. **The callee's validation** (`keepOrDeliver`, bound to the Rust
   `KEEP_OR_DELIVER_VECTORS` of `legacy_keep_or_deliver_summary.rs`, which run
   against its `decide`): the body is run from a null slot, whose exits must
   read W4's `out` mode (`outMode`), and from a slot holding a live object
   the caller owns, whose every exit must still hold exactly that object,
   live. A store over a non-null slot (a partial-path store, a slot first
   holding another object's member, a second store), a release or a hand-off
   of the caller's object fails the second run.
2. **The caller's reading**: the call is lowered to a branch on the slot's
   value at the call. A known-null slot takes the delivered arm, a slot
   holding the caller's live object takes the kept arm, and anything else
   (an untracked or borrowed value, an unbound name) reaches the kept arm's
   `Owned` position with no live identity and refuses: `Unknown`.
3. **The obligations**: the caller owes exactly one fresh object after the
   delivered arm, exactly its prior object after the kept arm, and so a
   caller that releases what it owes discharges every obligation, and one
   that ignores a delivery leaks. Neither reading mints a second custody of
   the prior object or hides a delivery.
4. **Why a table of its own**: no single parameter mode states both arms.
   An `out` reading of a kept object mints a second custody; a `borrowed`
   reading of a delivery hides it.

## What this is not

A model of the transfer function, of the Clang reading or of the CFG. That
the two runs' observations are those of the body is the gateway's (trust
boundary 28), as is that the branch the caller's lowering adds tests the
value the callee's slot holds at entry (`BranchOnNull` on `NAME` for the
actual `&NAME`).
-/

namespace QCore.OutSlotKeepOrDeliver

open QCore.CustodySummary (OutSlot)
open QCore.OwnedOutFormalDelivery (outMode outModeFrom live a_read_mode_delivers_on_every_exit)

/-! ## 1. The callee's validation -/

/-- One exit of the owned-entry run: whether the slot still holds the
caller's object, live and non-null (`true`), or anything else (`false`). -/
abbrev KeptExit := Bool

/-- The mode the two runs read: W4's `out` mode of the null-entry run, kept
only when the owned-entry run reached an exit and every exit kept the
caller's object. `some n` is the delivered contract's `Out { nullable := n }`;
`none` is no summary. -/
def keepOrDeliver (nullRun : List OutSlot) (ownedRun : List KeptExit) : Option Bool :=
  if ownedRun ≠ [] ∧ ownedRun.all id then outMode nullRun else none

/-- The Rust `KEEP_OR_DELIVER_VECTORS`: the null-entry run's exits, the
owned-entry run's exits, the mode. -/
def keepOrDeliverVectors : List (List OutSlot × List KeptExit × Option Bool) := [
  -- `if (!*out) *out = make();`
  ([live], [true], some false),
  -- `if (c) return -1; if (!*out) *out = make(); return 0;`
  ([.null, live], [true, true], some true),
  -- A partial-path store `if (c) *out = make();`: overwrites a kept object.
  ([live, .null], [false, true], none),
  -- A slot first holding another object's member, then a fresh one.
  ([.ambiguous], [false], none),
  -- `if (!*out) *out = make(); else free(*out);`: the kept object released.
  ([live], [false], none),
  -- An every-exit store: W4's `out`, never kept.
  ([live], [false], none),
  -- Only null in the null run: nothing delivered.
  ([.null], [true], none),
  -- A possibly-null result in the null run.
  ([.site false .live true], [true], none),
  -- No reachable exit.
  ([], [], none),
  ([live], [], none)
]

theorem keep_or_deliver_vectors_hold :
    keepOrDeliverVectors.all (fun (n, o, mode) => keepOrDeliver n o == mode) = true := by
  decide

/-- A read mode keeps the caller's object on every exit of the owned run. -/
theorem a_read_mode_keeps_on_every_owned_exit (nullRun : List OutSlot)
    (ownedRun : List KeptExit) (nullable : Bool)
    (read : keepOrDeliver nullRun ownedRun = some nullable) :
    ∀ e ∈ ownedRun, e = true := by
  intro e member
  unfold keepOrDeliver at read
  split at read
  · rename_i h
    have := List.all_eq_true.mp h.2 e member
    simpa using this
  · cases read

/-- A read mode delivers on every exit of the null run: a live non-null site
or the null constant. -/
theorem a_read_mode_delivers_on_every_null_exit (nullRun : List OutSlot)
    (ownedRun : List KeptExit) (nullable : Bool)
    (read : keepOrDeliver nullRun ownedRun = some nullable) :
    ∀ slot ∈ nullRun, slot = .null ∨ ∃ fromParameter, slot = .site fromParameter .live false := by
  unfold keepOrDeliver at read
  split at read
  · exact a_read_mode_delivers_on_every_exit nullRun false false nullable read
  · cases read

/-- Any owned-run exit that does not keep the caller's object (a store over a
non-null slot, its release, its hand-off) refuses the summary. -/
theorem an_overwrite_of_the_kept_object_reads_no_mode (nullRun : List OutSlot)
    (ownedRun : List KeptExit) (overwritten : false ∈ ownedRun) :
    keepOrDeliver nullRun ownedRun = none := by
  unfold keepOrDeliver
  split
  · rename_i h
    have := List.all_eq_true.mp h.2 false overwritten
    simp at this
  · rfl

/-- The null run alone is W4's `out` reader: anything it refuses, this
refuses. -/
theorem a_null_run_refusal_is_kept (nullRun : List OutSlot) (ownedRun : List KeptExit)
    (refused : outMode nullRun = none) : keepOrDeliver nullRun ownedRun = none := by
  unfold keepOrDeliver
  split <;> simp [refused]

/-! ## 2. The caller's reading -/

/-- What the caller's flow knows of the slot's value at the call. -/
inductive Entry where
  /-- Known null (`void *p = NULL;`). -/
  | null
  /-- A live, non-null object the caller owns, by its site. -/
  | owned (site : Nat)
  /-- Anything else: an untracked or borrowed value, an unbound name, a
  possibly-null value whose arms do not agree. -/
  | other
  deriving DecidableEq, Repr

/-- The arm the lowered branch takes, or the refusal. -/
inductive Outcome where
  /-- The delivered arm: the slot receives a fresh object (`nullable` if the
  mode is). -/
  | delivered (nullable : Bool)
  /-- The kept arm: the call required the prior object live and left it so. -/
  | kept (site : Nat)
  /-- The kept arm's `Owned` position met no live identity: `Unknown`. -/
  | refused
  deriving DecidableEq, Repr

/-- The caller's reading of a call under a validated mode. -/
def callerReading (nullable : Bool) : Entry → Outcome
  | .null => .delivered nullable
  | .owned site => .kept site
  | .other => .refused

/-- The caller owns a fresh object exactly when it knew the slot null. -/
theorem a_fresh_object_only_for_a_known_null_slot (nullable : Bool) (entry : Entry) :
    (∃ n, callerReading nullable entry = .delivered n) ↔ entry = .null := by
  cases entry <;> simp [callerReading]

/-- A slot the caller cannot place is `Unknown`, never a delivery or a keep. -/
theorem an_unplaced_slot_is_unknown (nullable : Bool) :
    callerReading nullable .other = .refused := rfl

/-! ## 3. The obligations -/

/-- The caller's obligations after the call, as the sites it owes: the
fresh one is `fresh`; `prior` is what it owed before. -/
def owedAfter (fresh : Nat) (prior : List Nat) : Outcome → Option (List Nat)
  | .delivered _ => some (fresh :: prior)
  | .kept _ => some prior
  | .refused => none

/-- The callee's real effect, from the two runs: on a null entry the slot
receives the fresh object; on an owned entry it keeps the prior one. -/
def realOwedAfter (fresh : Nat) (prior : List Nat) : Entry → Option (List Nat)
  | .null => some (fresh :: prior)
  | .owned _ => some prior
  | .other => none

/-- The reading states the callee's effect exactly, wherever it states one:
the obligations discharge exactly on the delivering exits. -/
theorem the_reading_is_the_effect (nullable : Bool) (fresh : Nat) (prior : List Nat)
    (entry : Entry) :
    owedAfter fresh prior (callerReading nullable entry) = realOwedAfter fresh prior entry := by
  cases entry <;> rfl

/-- The kept arm mints nothing: an owned prior object is owed once, before
and after. -/
theorem the_kept_arm_mints_no_second_custody (nullable : Bool) (fresh site : Nat)
    (prior : List Nat) (once : prior.count site = 1) :
    (owedAfter fresh prior (callerReading nullable (.owned site))).map (·.count site) = some 1 := by
  simp [callerReading, owedAfter, once]

/-- The delivered arm adds exactly the fresh object. -/
theorem the_delivered_arm_adds_one_obligation (nullable : Bool) (fresh : Nat)
    (prior : List Nat) (new : fresh ∉ prior) :
    (owedAfter fresh prior (callerReading nullable .null)).map (·.count fresh) = some 1 := by
  simp [callerReading, owedAfter, List.count_eq_zero_of_not_mem new]

/-- What the caller does with the slot's object at its exit. -/
inductive Use where
  | released
  | ignored
  deriving DecidableEq, Repr

/-- The sites the caller still owes at its exit, starting from none: a
released slot object is discharged, an ignored one is still owed. -/
def owedAtExit (fresh : Nat) (outcome : Outcome) (use : Use) : Option (List Nat) :=
  match outcome, use with
  | .delivered _, .released => some []
  | .delivered _, .ignored => some [fresh]
  | .kept _, .released => some []
  | .kept site, .ignored => some [site]
  | .refused, _ => none

theorem a_released_delivery_is_discharged (nullable : Bool) (fresh : Nat) :
    owedAtExit fresh (callerReading nullable .null) .released = some [] := rfl

theorem an_ignored_delivery_leaks_in_the_caller (nullable : Bool) (fresh : Nat) :
    owedAtExit fresh (callerReading nullable .null) .ignored = some [fresh] := rfl

theorem a_kept_object_stays_the_callers (nullable : Bool) (fresh site : Nat) :
    owedAtExit fresh (callerReading nullable (.owned site)) .ignored = some [site] := rfl

/-! ## 4. Why a table of its own -/

/-- An `out` reading at every call: the slot's prior value is overwritten by
a fresh object, so an owned prior object would still be owed and a second one
minted. -/
def outReadingOwed (fresh : Nat) (prior : List Nat) : Entry → List Nat
  | _ => fresh :: prior

/-- A `borrowed` reading at every call: nothing is delivered. -/
def borrowedReadingOwed (prior : List Nat) : Entry → List Nat
  | _ => prior

theorem an_out_reading_mints_a_second_custody (fresh site : Nat) (prior : List Nat) :
    some (outReadingOwed fresh prior (.owned site)) ≠ realOwedAfter fresh prior (.owned site) := by
  simp [outReadingOwed, realOwedAfter]

theorem a_borrowed_reading_hides_a_delivery (fresh : Nat) (prior : List Nat) :
    some (borrowedReadingOwed prior .null) ≠ realOwedAfter fresh prior .null := by
  simp [borrowedReadingOwed, realOwedAfter]

end QCore.OutSlotKeepOrDeliver
