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

import QCore.OpensslBioReaders
import QCore.OpensslBioSlots

/-!
# A slot invocation beside a reference drop

Card **OPENSSL-FREE-SESSION-REMOVE-INVOCATION** (contract specification
C04.19 amended in place, gateway rule 12n; user decision 2026-11-19 to
launch the follow-up). Beside `QCore.OpensslBioReaders` (rule 12n's decision
of one opaque slot, `decideSlot`) and `QCore.OpensslBioSlots` (which premise
covers what).

`SSL_CTX_free` and `SSL_free` run the session-remove callback, and
`BIO_free` and `SSL_free` run a BIO's callback (`BIO_CB_FREE`) and its
method's destroy function, beside their reference drop. Modelled as the
Rust code decides it:

* **Admission** (`contract_loader.rs`, C04.19). Beside reference clauses, a
  callback clause stands only when every reference clause is a drop and
  every callback clause is a slot invocation (`admitted`). The admission
  only grows, and only by such declarations (`the_admission_only_grows`,
  `the_admission_grows_only_by_slot_invocations_beside_drops`); a keep, a
  plain member invocation, a synchronous invocation and a registration
  beside a reference clause still reject.
* **Order** (`legacy_transfer.rs`). The callback route is applied before
  the reference route (`newOrder`), so what a candidate reads of the dropped
  object is the object before the drop (`the_new_order_reads_the_object_before_the_drop`).
  The old order applied it after (`oldOrder`), and at a last drop that
  object is freed (`the_old_order_read_a_freed_object`).
* **A drop that may not be the last** (`legacy_retained_by.rs`). Only the
  last drop runs the slot, and the call does not decide which drop is the
  last. So the call's reading does not depend on it
  (`the_reading_does_not_depend_on_the_last_drop`): a candidate effect on
  the dropped object (a member-path state, a deferred member reading)
  refuses (`a_member_effect_beside_a_drop_refuses`); what stands is the
  static decision of each slot and a requirement that the caller hold the
  object, read before the drop.

Proved: applied implies every slot set is closed with every candidate the
identity, no member effect and the object held
(`applied_only_over_closed_identity_slots`); stating more slots never
improves the outcome (`stating_more_slots_never_improves`); the free premise
covers only the ex_data callbacks, so a BIO's free callbacks must be stated
(`QCore.OpensslBioSlots.the_free_premise_covers_only_ex_data`). `free_*`
are the frees' vectors.

## What this is not

Which callbacks the frees run (the pages), the run's set of each key, the
built-in methods' behaviour and what the unstated premise covers are
inputs, not modelled here (trust boundary 28; the crate's checker and
`crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`).
-/

namespace QCore.OpensslFreeInvocations

open QCore.CallbackLiteralPremise (Outcome Candidate)
open QCore.OpensslBioReaders (SlotFacts decideSlot combine combine_applied decideSlot_applied
  combine_refused)

/-! ## Admission -/

/-- A reference clause. -/
inductive Reference where
  | kept
  | dropped
  deriving DecidableEq, Repr

/-- A callback clause, by the kind the loader distinguishes here. -/
inductive Callback where
  | invoked
  | slotInvoked
  | member
  | registered
  | other
  deriving DecidableEq, Repr

/-- A declaration's reference and callback clauses, and whether it states a
slot move or a derived out. -/
structure Declared where
  references : List Reference
  callbacks : List Callback
  slotMoves : Bool
  deriving DecidableEq, Repr

/-- C04.19 before this card: a callback clause stands only beside
retentions and plain predicates. -/
def admittedOld (d : Declared) : Bool :=
  d.callbacks.isEmpty || (!d.slotMoves && d.references.isEmpty)

/-- Whether every reference clause is a drop and every callback clause a slot
invocation. -/
def slotInvocationsBesideDrops (d : Declared) : Bool :=
  !d.references.isEmpty && d.references.all (· == .dropped) &&
    d.callbacks.all (· == .slotInvoked)

/-- C04.19 amended: beside reference drops, slot invocations stand. -/
def admitted (d : Declared) : Bool :=
  d.callbacks.isEmpty || (!d.slotMoves && (d.references.isEmpty || slotInvocationsBesideDrops d))

theorem the_admission_only_grows {d : Declared} (h : admittedOld d = true) :
    admitted d = true := by
  unfold admittedOld at h
  unfold admitted
  simp only [Bool.or_eq_true, Bool.and_eq_true] at h ⊢
  rcases h with h | ⟨hs, hr⟩
  · exact Or.inl h
  · exact Or.inr ⟨hs, Or.inl hr⟩

theorem the_admission_grows_only_by_slot_invocations_beside_drops {d : Declared}
    (h : admitted d = true) (hold : admittedOld d = false) :
    d.references ≠ [] ∧ (∀ r ∈ d.references, r = .dropped) ∧
      ∀ c ∈ d.callbacks, c = .slotInvoked := by
  unfold admitted at h
  unfold admittedOld at hold
  cases hc : d.callbacks.isEmpty with
  | true => simp [hc] at hold
  | false =>
      cases hs : d.slotMoves with
      | true => simp [hc, hs] at h
      | false =>
          cases hr : d.references.isEmpty with
          | true => simp [hc, hs, hr] at hold
          | false =>
              simp only [hc, hs, hr, Bool.false_or, Bool.not_false, Bool.true_and] at h
              unfold slotInvocationsBesideDrops at h
              simp only [hr, Bool.not_false, Bool.true_and, Bool.and_eq_true,
                List.all_eq_true, beq_iff_eq] at h
              refine ⟨?_, h.1, h.2⟩
              intro he
              simp [he] at hr

theorem a_keep_beside_an_invocation_rejects {cs : List Callback} (hc : cs ≠ [])
    {rs : List Reference} (hk : Reference.kept ∈ rs) (s : Bool) :
    admitted { references := rs, callbacks := cs, slotMoves := s } = false := by
  have hce : cs.isEmpty = false := by
    cases cs with
    | nil => exact absurd rfl hc
    | cons _ _ => rfl
  have hre : rs.isEmpty = false := by
    cases rs with
    | nil => cases hk
    | cons _ _ => rfl
  have hall : rs.all (· == .dropped) = false := by
    apply Bool.eq_false_iff.mpr
    intro h
    have := List.all_eq_true.mp h _ hk
    simp at this
  simp [admitted, slotInvocationsBesideDrops, hce, hre, hall]

theorem another_callback_beside_a_drop_rejects {cs : List Callback} {c : Callback}
    (hc : c ∈ cs) (hk : c ≠ .slotInvoked) (s : Bool) :
    admitted { references := [.dropped], callbacks := cs, slotMoves := s } = false := by
  have hce : cs.isEmpty = false := by
    cases cs with
    | nil => cases hc
    | cons _ _ => rfl
  have hall : cs.all (· == .slotInvoked) = false := by
    apply Bool.eq_false_iff.mpr
    intro h
    have := List.all_eq_true.mp h _ hc
    simp at this
    exact hk this
  simp [admitted, slotInvocationsBesideDrops, hce, hall]

/-! ## Order -/

/-- The dropped object as a candidate sees it: whether the caller holds a
reference to it and whether it is freed. -/
structure Object where
  held : Bool
  freed : Bool
  deriving DecidableEq, Repr

/-- The reference route's drop: the caller's reference ends; a last drop
frees the object. -/
def drop (last : Bool) (o : Object) : Object :=
  { held := false, freed := o.freed || last }

/-- The object the callback route reads, in the order this card applies:
before the drop. -/
def newOrder (_last : Bool) (o : Object) : Object := o

/-- The object the callback route read in the old order: after the drop. -/
def oldOrder (last : Bool) (o : Object) : Object := drop last o

/-- A candidate reading the object the caller holds reads it held and not
freed, whether or not the drop is the last; the old order read it unheld. -/
theorem the_new_order_reads_the_object_before_the_drop {o : Object} (hh : o.held = true)
    (hf : o.freed = false) (last : Bool) :
    (newOrder last o).held = true ∧ (newOrder last o).freed = false ∧
      (oldOrder last o).held = false :=
  ⟨hh, hf, rfl⟩

theorem the_old_order_read_a_freed_object (o : Object) :
    (oldOrder true o).freed = true ∧ (oldOrder true o).held = false := by
  simp [oldOrder, drop]

/-! ## The call's reading -/

/-- What a stated slot's candidates do to the dropped object, beside the
static decision: nothing, require it held (`resource_owned`, read on the
call's own position), or a member effect (a member-path state, a member
invocation deferred to its constructed state). -/
inductive Effect where
  | none
  | requiresHeld
  | member
  deriving DecidableEq, Repr

/-- The call's reading at a dropping call stating slot invocations: `pre` is
the object before the call, `last` whether the drop is the last (which the
call does not decide), `slots` the facts of each stated slot, `effects`
the candidates' effects on the dropped object. The object read is
`newOrder last pre`, the object before the drop. -/
def freeOutcome (pre : Object) (last : Bool) (slots : List SlotFacts) (effects : List Effect) :
    Outcome :=
  if effects.any (· == .member) then .refused
  else if effects.any (· == .requiresHeld) && !(newOrder last pre).held then .refused
  else combine (slots.map decideSlot)

theorem the_reading_does_not_depend_on_the_last_drop (pre : Object) (slots : List SlotFacts)
    (effects : List Effect) : freeOutcome pre true slots effects = freeOutcome pre false slots effects :=
  rfl

theorem a_member_effect_beside_a_drop_refuses {pre : Object} {last : Bool}
    {slots : List SlotFacts} {effects : List Effect} (h : Effect.member ∈ effects) :
    freeOutcome pre last slots effects = .refused := by
  have : effects.any (· == .member) = true := List.any_eq_true.mpr ⟨_, h, by simp⟩
  simp [freeOutcome, this]

theorem a_requirement_on_an_unheld_object_refuses {pre : Object} {last : Bool}
    {slots : List SlotFacts} {effects : List Effect} (hh : pre.held = false)
    (h : Effect.requiresHeld ∈ effects) : freeOutcome pre last slots effects = .refused := by
  have hr : effects.any (· == .requiresHeld) = true := List.any_eq_true.mpr ⟨_, h, by simp⟩
  cases hm : effects.any (· == .member) <;> simp [freeOutcome, newOrder, hm, hr, hh]

theorem applied_only_over_closed_identity_slots {pre : Object} {last : Bool}
    {slots : List SlotFacts} {effects : List Effect}
    (h : freeOutcome pre last slots effects = .applied) :
    (∀ e ∈ effects, e ≠ .member) ∧ (Effect.requiresHeld ∈ effects → pre.held = true) ∧
      ∀ s ∈ slots, ∃ cs, s = .closed cs ∧ ∀ k ∈ cs, k = .identity := by
  unfold freeOutcome at h
  cases hm : effects.any (· == .member) with
  | true => simp [hm] at h
  | false =>
      refine ⟨?_, ?_, ?_⟩
      · intro e he heq
        subst heq
        have : effects.any (· == .member) = true := List.any_eq_true.mpr ⟨_, he, by simp⟩
        simp [this] at hm
      · intro hr
        have hr' : effects.any (· == .requiresHeld) = true :=
          List.any_eq_true.mpr ⟨_, hr, by simp⟩
        cases hh : pre.held with
        | true => rfl
        | false => simp [hm, hr', hh, newOrder] at h
      · cases hq : (effects.any (· == .requiresHeld) && !(newOrder last pre).held) with
        | true => simp [hm, hq] at h
        | false =>
            simp only [hm, hq, Bool.false_eq_true, ↓reduceIte] at h
            have hall := combine_applied h
            intro s hs
            exact decideSlot_applied (hall (decideSlot s) (List.mem_map.mpr ⟨s, hs, rfl⟩))

theorem a_refused_slot_refuses {pre : Object} {last : Bool} {slots : List SlotFacts}
    {effects : List Effect} {s : SlotFacts} (hs : s ∈ slots) (hk : decideSlot s = .refused) :
    freeOutcome pre last slots effects = .refused := by
  unfold freeOutcome
  split
  · rfl
  · split
    · rfl
    · exact combine_refused (o := decideSlot s) (List.mem_map.mpr ⟨s, hs, rfl⟩)
        (by rw [hk]; decide)

theorem stating_more_slots_never_improves {pre : Object} {last : Bool}
    {slots more : List SlotFacts} {effects : List Effect}
    (h : freeOutcome pre last (slots ++ more) effects = .applied) :
    freeOutcome pre last slots effects = .applied := by
  have ⟨hm, hr, hs⟩ := applied_only_over_closed_identity_slots h
  have hm' : effects.any (· == .member) = false := by
    apply Bool.eq_false_iff.mpr
    intro ha
    obtain ⟨e, he, heq⟩ := List.any_eq_true.mp ha
    simp at heq
    exact hm e he heq
  have hq : (effects.any (· == .requiresHeld) && !(newOrder last pre).held) = false := by
    cases ha : effects.any (· == .requiresHeld) with
    | false => rfl
    | true =>
        obtain ⟨e, he, heq⟩ := List.any_eq_true.mp ha
        simp at heq
        subst heq
        simp [newOrder, hr he]
  unfold freeOutcome
  simp only [hm', hq, Bool.false_eq_true, ↓reduceIte]
  unfold combine
  have : (slots.map decideSlot).all (fun o => o == .applied) = true := by
    apply List.all_eq_true.mpr
    intro o ho
    obtain ⟨s, hsm, rfl⟩ := List.mem_map.mp ho
    obtain ⟨cs, rfl, hid⟩ := hs s (List.mem_append_left _ hsm)
    have : cs.all (fun c => c == .identity) = true :=
      List.all_eq_true.mpr (fun k hk => by simp [hid k hk])
    simp [decideSlot, this]
  simp [this]

/-! ## The frees' vectors -/

/-- `SSL_free`: the session-remove slot and the three BIO slots, each closed
and empty (the measured run), every actual `_` (no effect on the dropped
object). -/
def sslFreeSlots : List SlotFacts := List.replicate 4 (.closed [])

/-- The caller holds the reference it drops. -/
def held : Object := { held := true, freed := false }

/-- `SSL_free`'s clauses: one drop and four slot invocations. -/
def freeShape : Declared := ⟨[.dropped], List.replicate 4 .slotInvoked, false⟩

theorem free_shape_is_admitted : admitted freeShape = true := by decide

theorem free_shape_was_refused_before : admittedOld freeShape = false := by decide

theorem free_empty_slots_apply (last : Bool) : freeOutcome held last sslFreeSlots [] = .applied := by
  cases last <;> decide

theorem free_identity_candidate_applies :
    freeOutcome held true (.closed [.identity] :: sslFreeSlots) [] = .applied := by decide

theorem free_uncontracted_candidate_refuses :
    freeOutcome held true (.closed [.uncontracted] :: sslFreeSlots) [] = .refused := by decide

theorem free_effectful_destroy_refuses :
    freeOutcome held false (.closed [.effectful] :: sslFreeSlots) [] = .refused := by decide

theorem free_unregistered_key_refuses :
    freeOutcome held true (.unregistered :: sslFreeSlots) [] = .refused := by decide

theorem free_without_run_facts_refuses :
    freeOutcome held true (List.replicate 4 .noRun) [] = .refused := by decide

theorem free_member_effect_refuses_at_any_drop (last : Bool) :
    freeOutcome held last sslFreeSlots [.member] = .refused := by cases last <;> decide

end QCore.OpensslFreeInvocations
