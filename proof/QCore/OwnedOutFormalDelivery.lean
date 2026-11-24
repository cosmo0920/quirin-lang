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

import QCore.CustodySummary

/-!
# An owned result stored through a formal's slot (card OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY, gateway rule 74l)

USER DECISION 2026-11-23: picked as one of the "soundness first" cards.
`*partitions = rd_kafka_topic_partition_list_new(0);` stores an owned result
through a formal's slot. The delivery reading is W4's `out` parameter mode
(`QCore.CustodySummary`); this module states when it is read and what the
gateway says when it is not.

The module has four parts.

1. **The mode read off the exits** (`outMode`, bound to the Rust
   `OUT_MODE_VECTORS` of `owned_out_formal.rs`, which run against
   `legacy_summary::out_mode`): `out` only when every reachable exit holds a
   fresh, live, non-null site or the null constant, and at least one holds a
   site. A partial-path store, a released, escaped or contained site, a
   possibly-null site and an ambiguous slot each read no mode.
2. **The validation relation** (`QCore.CustodySummary.validate`): a validated
   `out` summary has no exit that leaves the slot unwritten or holds a
   released site, so no caller is told of a delivery a path does not make.
3. **The caller**: the delivered site becomes the caller's obligation, so a
   caller that ignores it leaks, and one that releases it discharges it.
4. **The refusal**: a store through a slot no contract sanctions is refused
   by name; the renaming changes no verdict, and it is never read as a
   discard.

## What this is not

A model of the transfer function, of the Clang reading or of the CFG. That
the observations are those of the body is the gateway's (trust boundary 28);
the heap-kind condition (`kind.is_heap()`) is checked by the Rust unit test
`a_non_heap_delivery_reads_no_mode`, not here.
-/

namespace QCore.OwnedOutFormalDelivery

open QCore.CustodySummary (OutSlot Disposition Summary ExitObservation ParameterMode
  validate outSlotMatches validated_summary_out_slots_match)

/-! ## 1. The mode read off the exits -/

/-- `legacy_summary::out_mode` over one slot's observations, with the
accumulated `nullable` and `delivered` flags. `some n` is `Out { nullable := n }`;
`none` is no summary. -/
def outModeFrom : List OutSlot → Bool → Bool → Option Bool
  | [], nullable, delivered => if delivered then some nullable else none
  | .site _ disposition siteNullable :: rest, nullable, _ =>
    if siteNullable || disposition != .live then none
    else outModeFrom rest nullable true
  | .null :: rest, _, delivered => outModeFrom rest true delivered
  | .unwritten :: _, _, _ => none
  | .ambiguous :: _, _, _ => none

def outMode (exits : List OutSlot) : Option Bool := outModeFrom exits false false

/-- A heap-kinded site the callee resolved (the vectors' `site-…`). -/
def live : OutSlot := .site false .live false

/-- The Rust `OUT_MODE_VECTORS`. -/
def outModeVectors : List (List OutSlot × Option Bool) := [
  ([live], some false),
  ([live, live], some false),
  ([live, .null], some true),
  ([live, .unwritten], none),
  ([.unwritten], none),
  ([.null], none),
  ([.site false .released false], none),
  ([live, .site false .released false], none),
  ([.site false .escaped false], none),
  ([.site false .contained false], none),
  ([.site false .live true], none),
  ([.ambiguous], none),
  ([live, .ambiguous], none),
  ([], none)
]

theorem out_mode_vectors_hold :
    outModeVectors.all (fun (exits, mode) => outMode exits == mode) = true := by
  decide

/-- Any exit that refuses on its own refuses the whole list, whatever the
flags. -/
theorem refused_anywhere (bad : OutSlot)
    (refuses : ∀ rest nullable delivered, outModeFrom (bad :: rest) nullable delivered = none) :
    ∀ (exits : List OutSlot) (nullable delivered : Bool), bad ∈ exits →
      outModeFrom exits nullable delivered = none := by
  intro exits
  induction exits with
  | nil => intro _ _ member; cases member
  | cons head rest ih =>
    intro nullable delivered member
    cases List.mem_cons.mp member with
    | inl same => subst same; exact refuses rest nullable delivered
    | inr later =>
      cases head with
      | unwritten => rfl
      | ambiguous => rfl
      | null => exact ih true delivered later
      | site fromParameter disposition siteNullable =>
        simp only [outModeFrom]
        split
        · rfl
        · exact ih nullable true later

/-- A partial-path store: an exit that leaves the slot as the caller had it
reads no mode. -/
theorem a_partial_path_store_reads_no_mode (exits : List OutSlot)
    (partial_path : OutSlot.unwritten ∈ exits) : outMode exits = none :=
  refused_anywhere .unwritten (fun _ _ _ => rfl) exits false false partial_path

/-- A store followed by a release (or a delivery moved on or contained):
the slot holds a site that is not live. -/
theorem a_dead_delivery_reads_no_mode (exits : List OutSlot) (fromParameter nullable : Bool)
    (disposition : Disposition) (dead : disposition ≠ .live)
    (member : OutSlot.site fromParameter disposition nullable ∈ exits) :
    outMode exits = none := by
  refine refused_anywhere (.site fromParameter disposition nullable) ?_ exits false false member
  intro rest n d
  simp only [outModeFrom]
  have : (disposition != Disposition.live) = true := by simpa using dead
  simp [this]

/-- A possibly-null result (`*out = maybe_make();`). -/
theorem a_possibly_null_delivery_reads_no_mode (exits : List OutSlot) (fromParameter : Bool)
    (disposition : Disposition) (member : OutSlot.site fromParameter disposition true ∈ exits) :
    outMode exits = none :=
  refused_anywhere (.site fromParameter disposition true) (fun _ _ _ => by simp [outModeFrom])
    exits false false member

/-- An ambiguous slot (another object's value, or several sites). -/
theorem an_ambiguous_slot_reads_no_mode (exits : List OutSlot)
    (member : OutSlot.ambiguous ∈ exits) : outMode exits = none :=
  refused_anywhere .ambiguous (fun _ _ _ => rfl) exits false false member

/-- What a read mode guarantees: every exit holds a live non-null site or the
null constant. -/
theorem a_read_mode_delivers_on_every_exit :
    ∀ (exits : List OutSlot) (nullable delivered : Bool) (mode : Bool),
      outModeFrom exits nullable delivered = some mode →
      ∀ slot ∈ exits, slot = .null ∨ ∃ fromParameter, slot = .site fromParameter .live false := by
  intro exits
  induction exits with
  | nil => intro _ _ _ _ slot member; cases member
  | cons head rest ih =>
    intro nullable delivered mode read slot member
    cases head with
    | unwritten => simp [outModeFrom] at read
    | ambiguous => simp [outModeFrom] at read
    | null =>
      cases List.mem_cons.mp member with
      | inl same => exact Or.inl same
      | inr later => exact ih true delivered mode read slot later
    | site fromParameter disposition siteNullable =>
      simp only [outModeFrom] at read
      split at read
      · cases read
      · rename_i ok
        cases List.mem_cons.mp member with
        | inl same =>
          subst same
          have h1 : siteNullable = false := by
            cases siteNullable <;> simp_all
          have h2 : disposition = .live := by
            cases disposition <;> simp_all
          exact Or.inr ⟨fromParameter, by rw [h1, h2]⟩
        | inr later => exact ih nullable true mode read slot later

/-- A mode is read only when some exit delivers a site: null alone is no
delivery. -/
theorem only_null_reads_no_mode (count : Nat) :
    outMode (List.replicate count OutSlot.null) = none := by
  suffices ∀ n nullable, outModeFrom (List.replicate n OutSlot.null) nullable false = none from
    this count false
  intro n
  induction n with
  | zero => intro _; rfl
  | succ k ih => intro _; exact ih true

/-! ## 2. The validation relation -/

/-- A validated `out` summary has no exit that leaves the slot unwritten. -/
theorem a_validated_out_summary_has_no_partial_path (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits) (nullable : Bool)
    (paired : (ParameterMode.out nullable, some OutSlot.unwritten)
      ∈ summary.parameters.zip observation.outSlots) : False := by
  have matched := validated_summary_out_slots_match summary safe exits valid observation member
    (ParameterMode.out nullable) (some OutSlot.unwritten) paired
  simp [outSlotMatches] at matched

/-- A validated `out` summary has no exit whose slot holds a released site. -/
theorem a_validated_out_summary_has_no_released_delivery (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits) (nullable fromParameter
      siteNullable : Bool)
    (paired : (ParameterMode.out nullable,
        some (OutSlot.site fromParameter Disposition.released siteNullable))
      ∈ summary.parameters.zip observation.outSlots) : False := by
  have matched := validated_summary_out_slots_match summary safe exits valid observation member
    _ _ paired
  simp [outSlotMatches] at matched

/-! ## 3. The caller -/

/-- The caller's custody of the delivered site at its own exit. -/
inductive CallerUse where
  /-- `q_free(p)` on the caller's path. -/
  | released
  /-- `fill(&p);` and nothing more. -/
  | ignored
  /-- Handed on through a contract that consumes it. -/
  | consumed
  deriving DecidableEq, Repr

/-- Whether the caller's exit still owes the delivered site. -/
def owes : CallerUse → Bool
  | .released => false
  | .consumed => false
  | .ignored => true

/-- The call instantiates the `out` mode by binding the slot's variable to a
fresh obligation of the caller (`call_instantiation`), so the obligation is
the caller's whatever the callee did. -/
def callerOwesAfterDelivery (mode : Option Bool) (use : CallerUse) : Bool :=
  match mode with
  | some _ => owes use
  | none => false

theorem an_ignored_delivery_leaks_in_the_caller (nullable : Bool) :
    callerOwesAfterDelivery (some nullable) .ignored = true := rfl

theorem a_released_delivery_is_discharged (nullable : Bool) :
    callerOwesAfterDelivery (some nullable) .released = false := rfl

/-! ## 4. The refusal -/

/-- Where an owned call result goes. -/
inductive Destination where
  /-- A `*NAME` slot the contract or summary in force declares `out`. -/
  | sanctionedSlot
  /-- A `*NAME` slot nothing sanctions. -/
  | unsanctionedSlot
  /-- A named variable. -/
  | variable
  /-- No destination the importer names: `make();`, `a[i] = make();`. -/
  | unnamed
  deriving DecidableEq, Repr

inductive Reading where
  /-- Modeled by the flow; the verdict is decided there. -/
  | modeled
  /-- `owned result of resource API … stored through … without an
      out-parameter contract`. -/
  | refusedSlot
  /-- `discarded owned result of resource API …`. -/
  | refusedDiscard
  deriving DecidableEq, Repr

/-- The reading before this card. -/
def readingBefore : Destination → Reading
  | .sanctionedSlot => .modeled
  | .unsanctionedSlot => .refusedDiscard
  | .variable => .modeled
  | .unnamed => .refusedDiscard

/-- The reading after this card. -/
def reading : Destination → Reading
  | .sanctionedSlot => .modeled
  | .unsanctionedSlot => .refusedSlot
  | .variable => .modeled
  | .unnamed => .refusedDiscard

/-- What a reading contributes: a refusal is an `unsupported` leaf, which
makes the row `Unknown`; a modeled store contributes nothing here. -/
inductive Leaf where
  | none
  | unsupported
  deriving DecidableEq, Repr

def leafOf : Reading → Leaf
  | .modeled => .none
  | .refusedSlot => .unsupported
  | .refusedDiscard => .unsupported

theorem the_renaming_keeps_every_leaf (d : Destination) :
    leafOf (reading d) = leafOf (readingBefore d) := by
  cases d <;> rfl

theorem an_unsanctioned_slot_is_refused (d : Destination) :
    reading d = .refusedSlot ↔ d = .unsanctionedSlot := by
  cases d <;> simp [reading]

theorem a_slot_store_is_never_read_as_a_discard :
    reading .sanctionedSlot ≠ .refusedDiscard ∧ reading .unsanctionedSlot ≠ .refusedDiscard := by
  decide

theorem only_the_unsanctioned_slot_changed (d : Destination) :
    reading d ≠ readingBefore d ↔ d = .unsanctionedSlot := by
  cases d <;> simp [reading, readingBefore]

end QCore.OwnedOutFormalDelivery
