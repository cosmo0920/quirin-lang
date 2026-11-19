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

import QCore.OpensslHandshakeSlots

/-!
# A BIO's method table and callback as opaque slots (card OPENSSL-BIO-METHODS-AND-CALLBACKS)

Gateway rule 12n and contract specification C04.19.3 (the opaque slot,
`QCore.OpaqueCallbackSlots`), C04.18 (named premises), C04.19 (amended in
place by OPENSSL-FREE-SESSION-REMOVE-INVOCATION: a slot invocation beside a
reference drop, `QCore.OpensslFreeInvocations`). Abstract rule theorems over the checker
`providers/quirin-provider-openssl/src/check.rs` (`expected_clauses`,
`slot_writers_declared`) and the crate's declarations, not an extraction of
them.

* **A method slot's set is what the setters are handed, beside the
  built-ins.** A function a BIO's method table holds was handed to a
  `BIO_meth_set_*` setter of some table, or is a function of a method the
  library defines itself (`Origin`). The run's set is the first kind
  (`runSet`), so every setter-written function is a candidate
  (`a_setter_written_function_is_a_candidate`), and a held function outside
  the set is a built-in (`a_held_function_outside_the_set_is_builtin`).
* **What each premise covers.** The narrowed unstated-slots premise no
  longer covers a BIO slot, the library-value premise covers only the
  built-ins, and the free premise covers only what a BIO's last drop runs
  (`covers`). So a declaration is accepted only where every BIO slot it runs
  is stated (`a_bio_slot_must_be_stated`), a built-in method it runs is
  covered only by `openssl_builtin_bio_methods`
  (`a_builtin_method_needs_its_premise`). Card
  OPENSSL-FREE-SESSION-REMOVE-INVOCATION: what a BIO's last drop runs is its
  callback and destroy slots, stated beside the drop, and the free premise
  covers only the ex_data free callbacks
  (`the_free_premise_covers_only_ex_data`), so a BIO's free callbacks are
  stated (`a_bio_slot_must_be_stated`).
* **Stating the BIO slots never improves a verdict**
  (`stating_the_bio_slots_never_improves_a_verdict`, by
  `QCore.OpensslHandshakeSlots`), and a row is `Safe` only where each new
  invocation decides the identity.

## What this is not

That the pages' NAME sections are the library's complete writer lists, that
`BIO_METHOD` is opaque to every unit of the run (checked per run: a unit
defining `struct bio_method_st` refuses the slot), that the handshake and I/O
calls run no BIO operation outside the stated keys, and that the built-in
methods behave as the premise says are premises, pinned by the crate's
checker tests and `crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`.
-/

namespace QCore.OpensslBioSlots

open QCore.CallbackContracts (Decision)
open QCore.OpensslUnstatedSlots (rowVerdict)
open QCore.VerdictSemigroup
open QCore.VerdictSemigroup.V

/-! ## A method slot's set is what the setters are handed, beside the built-ins -/

/-- Where a function a BIO's method table holds came from. -/
inductive Origin where
  | setter (f : String)
  | builtin (f : String)
  deriving DecidableEq, Repr

/-- The run's set of a method slot: what its setters are handed. -/
def runSet : List Origin → List String
  | [] => []
  | .setter f :: os => f :: runSet os
  | .builtin _ :: os => runSet os

theorem a_setter_written_function_is_a_candidate {os : List Origin} {f : String}
    (h : Origin.setter f ∈ os) : f ∈ runSet os := by
  induction os with
  | nil => cases h
  | cons o os ih =>
      rcases List.mem_cons.mp h with heq | hmem
      · subst heq
        simp [runSet]
      · cases o <;> simp [runSet, ih hmem]

theorem a_held_function_outside_the_set_is_builtin {os : List Origin} {o : Origin}
    (h : o ∈ os) (hn : ∀ f, o = .setter f → f ∉ runSet os) : ∃ f, o = .builtin f := by
  cases o with
  | setter f => exact absurd (a_setter_written_function_is_a_candidate h) (hn f rfl)
  | builtin f => exact ⟨f, rfl⟩

/-! ## What each premise covers -/

/-- What a call may run: a slot (by key; a BIO's last drop runs its callback
with `BIO_CB_FREE` and its destroy function, both slots), a built-in
method's function, or an ex_data callback. -/
inductive Runnable where
  | slot (key : String)
  | builtinMethod
  | exData
  deriving DecidableEq, Repr

/-- The premises a declaration may name. -/
inductive Premise where
  | unstatedSlots
  | builtinBioMethods
  | bioFreeCallbacks
  | exData
  deriving DecidableEq, Repr

/-- What a premise covers; `bio` tells a BIO key (`bio_st.*`,
`bio_method_st.*`). The narrowed unstated-slots premise covers no BIO key. -/
def covers (bio : String → Bool) : Premise → Runnable → Bool
  | .unstatedSlots, .slot key => !bio key
  | .unstatedSlots, .exData => true
  | .builtinBioMethods, .builtinMethod => true
  | .bioFreeCallbacks, .exData => true
  | .exData, .exData => true
  | _, _ => false

/-- A callback is covered when the declaration states it (a slot) or a
premise it names covers it. -/
def covered (bio : String → Bool) (stated : List String) (ps : List Premise) :
    Runnable → Bool
  | .slot key => stated.contains key || ps.any (covers bio · (.slot key))
  | r => ps.any (covers bio · r)

/-- A declaration is accepted only where every callback it may run is
covered. -/
def wellCovered (bio : String → Bool) (runs : List Runnable) (stated : List String)
    (ps : List Premise) : Bool :=
  runs.all (covered bio stated ps)

theorem no_premise_covers_a_bio_slot {bio : String → Bool} {key : String}
    (hb : bio key = true) (p : Premise) : covers bio p (.slot key) = false := by
  cases p <;> simp [covers, hb]

theorem a_bio_slot_must_be_stated {bio : String → Bool} {runs : List Runnable}
    {stated : List String} {ps : List Premise} (h : wellCovered bio runs stated ps = true)
    {key : String} (hk : Runnable.slot key ∈ runs) (hb : bio key = true) : key ∈ stated := by
  simp only [wellCovered, List.all_eq_true] at h
  have := h _ hk
  simp only [covered, Bool.or_eq_true, List.any_eq_true, no_premise_covers_a_bio_slot hb,
    Bool.false_eq_true, and_false, exists_false, or_false, List.contains_iff_mem] at this
  exact this

theorem a_builtin_method_needs_its_premise {bio : String → Bool} {runs : List Runnable}
    {stated : List String} {ps : List Premise} (h : wellCovered bio runs stated ps = true)
    (hr : Runnable.builtinMethod ∈ runs) : Premise.builtinBioMethods ∈ ps := by
  simp only [wellCovered, List.all_eq_true] at h
  have := h _ hr
  simp only [covered, List.any_eq_true] at this
  obtain ⟨p, hp, hc⟩ := this
  cases p <;> simp_all [covers]

/-- Card OPENSSL-FREE-SESSION-REMOVE-INVOCATION: the narrowed free premise
covers the ex_data free callbacks and nothing else. -/
theorem the_free_premise_covers_only_ex_data {bio : String → Bool} {r : Runnable}
    (h : covers bio .bioFreeCallbacks r = true) : r = .exData := by
  cases r <;> simp_all [covers]

/-- The narrowed premise alone never accepts a declaration running a BIO
slot it does not state. -/
theorem the_narrowed_premise_never_covers_a_bio_slot {bio : String → Bool} {key : String}
    (hb : bio key = true) (hs : key ∉ stated) :
    covered bio stated [.unstatedSlots] (.slot key) = false := by
  simp [covered, covers, hb, hs]

/-! ## A BIO's drop states its slots

Card OPENSSL-FREE-SESSION-REMOVE-INVOCATION: C04.19 admits slot invocations
beside a reference drop (`QCore.OpensslFreeInvocations.admitted`), so a
dropping declaration that may run a BIO's callback or destroy slot is
accepted only where it states it, whatever premise it names. -/

theorem a_bio_drop_states_its_slots {bio : String → Bool} {runs : List Runnable}
    {stated : List String} {ps : List Premise} (h : wellCovered bio runs stated ps = true)
    {key : String} (hk : Runnable.slot key ∈ runs) (hb : bio key = true) : key ∈ stated :=
  a_bio_slot_must_be_stated h hk hb

/-! ## Stating the BIO slots never improves a verdict -/

theorem stating_the_bio_slots_never_improves_a_verdict (own : V) (ds bio : List Decision)
    (p : QCore.OpensslUnstatedSlots.Premise) (h : rowVerdict own (ds ++ bio) p = .safe) :
    rowVerdict own ds p = .safe :=
  QCore.OpensslHandshakeSlots.stating_more_invocations_never_improves_a_verdict own ds bio p h

theorem a_row_is_safe_only_if_every_bio_invocation_decides {own : V}
    {ds bio : List Decision} {p : QCore.OpensslUnstatedSlots.Premise}
    (h : rowVerdict own (ds ++ bio) p = .safe) : ∀ d ∈ bio, d = .identity :=
  QCore.OpensslHandshakeSlots.a_row_is_safe_only_if_every_new_invocation_decides h

end QCore.OpensslBioSlots
