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

import QCore.CallbackLiteralPremise

/-!
# The BIO readers: slot invocations beside a literal-argument premise

Card **OPENSSL-BIO-READER-INVOCATIONS** (gateway rule 57 amended in place;
contract specification §C04.5 "Beside a slot invocation"; user decision
2026-11-19 to launch the follow-up). Beside `QCore.OpensslBioSlots`, which
states the BIO's callback and method table as opaque slots, and
`QCore.CallbackLiteralPremise`, whose admission now takes an opaque slot
invocation beside a premise.

`PEM_read_bio_X509`, `PEM_read_bio_PrivateKey` and `d2i_PKCS12_bio` read
through the caller's BIO, so they state the 14 BIO slot invocations beside
their `x` / `p12` premise (and, for the PEM readers, the password callback).
Modelled as the Rust code decides it:

* **Admission** (`contract_loader.rs`): the reader shape, a synchronous
  invocation and slot invocations beside a premise, is admitted; the old
  admission refused it (`reader_shape_is_admitted`,
  `reader_shape_was_refused_before`).
* **Route G** (`legacy_literal_premise::restrict_to_declared_literals`)
  renames every unmet call of a retaining declaration before the callback
  route, so at an unmet call no slot is decided and the callee is
  undeclared (`an_unmet_premise_decides_no_slot`).
* **At a met call** the callback route decides each invocation as before
  (`a_met_premise_defers_to_the_slots`): the password callback as rule 13u
  (with its null default), each slot as rule 12n decides an opaque slot
  (`decideSlot`): refused without the run's facts, refused when no loaded
  declaration registers the key, refused on an open set, and otherwise
  applied only when every candidate is the identity (an empty closed set
  invokes nothing). The call applies only when every invocation applies.

Proved: applied implies every slot set is closed with every candidate the
identity and the own invocation applied
(`applied_only_over_closed_identity_slots`); the premise adds no candidate
(`the_premise_adds_no_candidate`); stating more slots never improves the
outcome (`stating_more_slots_never_improves`). `reader_*` are the readers'
vectors.

## What this is not

Which BIO operations a read performs, that the 14 keys are every slot a BIO
read can run, the run's set of each key and the built-in methods' behaviour
(`openssl_builtin_bio_methods`) are inputs, not modelled here (trust
boundary 28; the crate's checker and
`crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`).
-/

namespace QCore.OpensslBioReaders

open QCore.CallbackLiteralPremise

/-- Rule 12n's facts for one opaque slot at a call. -/
inductive SlotFacts where
  /-- The route reads no run facts (no pre-pass). -/
  | noRun
  /-- No loaded declaration registers the key. -/
  | unregistered
  /-- The run set is open. -/
  | opened
  /-- The run's closed set of candidates. -/
  | closed (candidates : List Candidate)
  deriving DecidableEq, Repr

/-- Rule 12n's decision of one opaque slot invocation. -/
def decideSlot : SlotFacts → Outcome
  | .noRun => .refused
  | .unregistered => .refused
  | .opened => .refused
  | .closed candidates =>
      if candidates.all (fun c => c == .identity) then .applied else .refused

/-- A call applies only when every invocation it states applies. -/
def combine (outcomes : List Outcome) : Outcome :=
  if outcomes.all (fun o => o == .applied) then .applied else .refused

/-- The outcome of a reader call on route G: `own` is the decision of the
declaration's own synchronous invocation (`.applied` when it states none),
`slots` the facts of each stated slot invocation. -/
def readerOutcome (e : Environment) (c : Call) (own : Outcome) (slots : List SlotFacts) :
    Outcome :=
  if renamed e c then .undeclared
  else if e.retaining c.callee then combine (own :: slots.map decideSlot)
  else if e.functions c.callee then .applied
  else .undeclared

/-! ### Admission -/

theorem a_slot_invocation_stands_beside_a_premise {ps : List Premise} {n : Nat} :
    admits ps (List.replicate n .slotInvoked) = true ∧
      admits ps (.invoked :: List.replicate n .slotInvoked) = true := by
  unfold admits besidePremise
  simp

/-! ### Outside the premise no slot is decided -/

theorem an_unmet_premise_decides_no_slot {e : Environment} {c : Call} {own : Outcome}
    {slots : List SlotFacts} (hr : e.retaining c.callee = true) (hu : unmet e c = true) :
    readerOutcome e c own slots = .undeclared := by
  unfold readerOutcome renamed
  simp [hr, hu]

theorem a_poisoned_set_decides_no_slot {e : Environment} {c : Call} {own : Outcome}
    {slots : List SlotFacts} (hr : e.retaining c.callee = true) (hp : e.poisoned = true) :
    readerOutcome e c own slots = .undeclared :=
  an_unmet_premise_decides_no_slot hr (by unfold unmet; simp [hp])

/-! ### Within the premise each invocation decides, unchanged -/

theorem a_met_premise_defers_to_the_slots {e : Environment} {c : Call} {own : Outcome}
    {slots : List SlotFacts} (hr : e.retaining c.callee = true) (hu : unmet e c = false) :
    readerOutcome e c own slots = combine (own :: slots.map decideSlot) := by
  unfold readerOutcome renamed
  simp [hr, hu]

theorem combine_applied {os : List Outcome} (h : combine os = .applied) :
    ∀ o ∈ os, o = .applied := by
  unfold combine at h
  cases hall : os.all (fun o => o == .applied) with
  | false => simp [hall] at h
  | true =>
      intro o ho
      have := List.all_eq_true.mp hall o ho
      simpa using this

theorem decideSlot_applied {s : SlotFacts} (h : decideSlot s = .applied) :
    ∃ cs, s = .closed cs ∧ ∀ k ∈ cs, k = .identity := by
  cases s with
  | noRun => simp [decideSlot] at h
  | unregistered => simp [decideSlot] at h
  | opened => simp [decideSlot] at h
  | closed cs =>
      refine ⟨cs, rfl, ?_⟩
      cases hall : cs.all (fun c => c == .identity) with
      | false => simp [decideSlot, hall] at h
      | true =>
          intro k hk
          have := List.all_eq_true.mp hall k hk
          simpa using this

theorem applied_only_over_closed_identity_slots {e : Environment} {c : Call} {own : Outcome}
    {slots : List SlotFacts} (hr : e.retaining c.callee = true)
    (h : readerOutcome e c own slots = .applied) :
    own = .applied ∧ ∀ s ∈ slots, ∃ cs, s = .closed cs ∧ ∀ k ∈ cs, k = .identity := by
  cases hu : unmet e c with
  | true => rw [an_unmet_premise_decides_no_slot hr hu] at h; cases h
  | false =>
      rw [a_met_premise_defers_to_the_slots hr hu] at h
      have hall := combine_applied h
      refine ⟨hall own (by simp), ?_⟩
      intro s hs
      exact decideSlot_applied (hall (decideSlot s) (by simp [List.mem_map]; exact Or.inr ⟨s, hs, rfl⟩))

theorem combine_refused {os : List Outcome} {o : Outcome} (ho : o ∈ os) (hk : o ≠ .applied) :
    combine os = .refused := by
  unfold combine
  cases hall : os.all (fun o => o == .applied) with
  | false => rfl
  | true =>
      have := List.all_eq_true.mp hall o ho
      simp at this
      exact absurd this hk

theorem a_refused_slot_refuses {e : Environment} {c : Call} {own : Outcome}
    {slots : List SlotFacts} {s : SlotFacts} (hr : e.retaining c.callee = true)
    (hu : unmet e c = false) (hs : s ∈ slots) (hk : decideSlot s = .refused) :
    readerOutcome e c own slots = .refused := by
  rw [a_met_premise_defers_to_the_slots hr hu]
  exact combine_refused (o := decideSlot s) (by simp [List.mem_map]; exact Or.inr ⟨s, hs, rfl⟩)
    (by rw [hk]; decide)

theorem a_slot_without_run_facts_refuses {e : Environment} {c : Call} {own : Outcome}
    {slots : List SlotFacts} (hr : e.retaining c.callee = true) (hu : unmet e c = false)
    (hs : SlotFacts.noRun ∈ slots) : readerOutcome e c own slots = .refused :=
  a_refused_slot_refuses hr hu hs rfl

theorem an_unregistered_slot_refuses {e : Environment} {c : Call} {own : Outcome}
    {slots : List SlotFacts} (hr : e.retaining c.callee = true) (hu : unmet e c = false)
    (hs : SlotFacts.unregistered ∈ slots) : readerOutcome e c own slots = .refused :=
  a_refused_slot_refuses hr hu hs rfl

theorem an_empty_closed_slot_invokes_nothing : decideSlot (.closed []) = .applied := rfl

/-- The premise adds no candidate: at a call it is met, the outcome is the
one the same declaration has with no premise at all. -/
theorem the_premise_adds_no_candidate {e : Environment} {c : Call} {own : Outcome}
    {slots : List SlotFacts} (hu : unmet e c = false) :
    readerOutcome e c own slots =
      readerOutcome { e with premises := fun _ => [], poisoned := false } c own slots := by
  unfold readerOutcome renamed
  have hu' : unmet { e with premises := fun _ => [], poisoned := false } c = false := by
    unfold unmet; simp
  simp [hu, hu']

/-- Stating more slots never improves the outcome. -/
theorem stating_more_slots_never_improves {e : Environment} {c : Call} {own : Outcome}
    {slots more : List SlotFacts} (hr : e.retaining c.callee = true)
    (h : readerOutcome e c own (slots ++ more) = .applied) :
    readerOutcome e c own slots = .applied := by
  cases hu : unmet e c with
  | true => rw [an_unmet_premise_decides_no_slot hr hu] at h; cases h
  | false =>
      rw [a_met_premise_defers_to_the_slots hr hu] at h
      rw [a_met_premise_defers_to_the_slots hr hu]
      have hall := combine_applied h
      unfold combine
      have : (own :: slots.map decideSlot).all (fun o => o == .applied) = true := by
        apply List.all_eq_true.mpr
        intro o ho
        have := hall o (by
          simp only [List.mem_cons, List.map_append, List.mem_append] at ho ⊢
          rcases ho with ho | ho
          · exact Or.inl ho
          · exact Or.inr (Or.inl ho))
        simp [this]
      simp [this]

/-! ### The readers' vectors (`pemEnv`: `PEM_read_bio_X509`, premise `x = 0`) -/

/-- The 14 BIO slots, each closed and empty (the measured run). -/
def emptySlots : List SlotFacts := List.replicate 14 (.closed [])

theorem reader_shape_is_admitted :
    admits [{ index := 1, literal := [0] }] (.invoked :: List.replicate 14 .slotInvoked) =
      true := by decide

theorem reader_shape_was_refused_before :
    admitsOld [{ index := 1, literal := [0] }] (.invoked :: List.replicate 14 .slotInvoked) =
      false := by decide

theorem pkcs12_shape_is_admitted :
    admits [{ index := 1, literal := [0] }] (List.replicate 14 .slotInvoked) = true := by
  decide

theorem a_plain_member_invocation_still_refused :
    admits [{ index := 1, literal := [0] }] (.member :: List.replicate 14 .slotInvoked) =
      false := by decide

/-- Within the premise, a null `cb` (the null default applies) and every BIO
slot closed and empty: the declaration applies. -/
theorem reader_null_callback_empty_slots_apply :
    readerOutcome pemEnv pemNullX .applied emptySlots = .applied := by decide

theorem reader_reuse_arm_decides_no_slot :
    readerOutcome pemEnv pemReuse .applied (.closed [.uncontracted] :: emptySlots) =
      .undeclared := by decide

theorem reader_uncontracted_bio_candidate_refuses :
    readerOutcome pemEnv pemNullX .applied (.closed [.uncontracted] :: emptySlots) =
      .refused := by decide

theorem reader_effectful_bio_method_refuses :
    readerOutcome pemEnv pemNullX .applied (.closed [.effectful] :: emptySlots) =
      .refused := by decide

theorem reader_unregistered_key_refuses :
    readerOutcome pemEnv pemNullX .applied (.unregistered :: emptySlots) = .refused := by
  decide

theorem reader_without_run_facts_refuses :
    readerOutcome pemEnv pemNullX .applied (List.replicate 14 .noRun) = .refused := by decide

end QCore.OpensslBioReaders
