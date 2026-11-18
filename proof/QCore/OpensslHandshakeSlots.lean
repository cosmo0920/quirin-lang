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

import QCore.OpensslUnstatedSlots

/-!
# The OpenSSL handshake slots whose setters were undeclared (card OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS)

Gateway rule 12n and contract specification C04.19.3 (the opaque slot,
`QCore.OpaqueCallbackSlots`), C04.18 (named premises). Abstract rule
theorems over the checker `providers/quirin-provider-openssl/src/check.rs`
(`registered_slots`, `slot_writers_declared`) and the crate's declarations,
not an extraction of them.

* **A setter may write several slots.** An entry registers the slot its
  shape names and each further registration it states
  (`SSL_CTX_add_custom_ext`'s `add_cb`, `free_cb`, `parse_cb`). The checker
  accepts a slot's writers only when every writer is an entry registering
  the slot and every entry registering it is a listed writer
  (`writersAccepted`). So a further registration is a writer of its slot
  like the shape's (`a_further_registration_is_a_registration`); an entry
  registering a slot it is not listed for refuses
  (`an_unlisted_registering_entry_refuses`), and so does a listed writer
  whose entry dropped that registration
  (`a_writer_without_the_registration_refuses`).
* **A slot the library writes through no function pointer formal is never
  accepted.** `SSL_CTX_enable_ct` writes the CT validation slot with the
  library's own callback: its entry, if any, registers nothing, so a slot
  listing it as a writer refuses
  (`a_library_written_slot_is_never_accepted`), and a declaration that may
  run such a slot is covered only by the unstated-slots premise
  (`an_unstated_slot_needs_the_premise`).
* **Stating more slots never improves a verdict**
  (`stating_more_invocations_never_improves_a_verdict`, by
  `QCore.OpensslUnstatedSlots`): every new invocation is one more decision
  joined into the row, and the row is `Safe` only where each decides the
  identity (`a_row_is_safe_only_if_every_new_invocation_decides`).

## What this is not

That the pages' NAME sections are the library's complete writer lists, that
the handshake and I/O calls run every slot their declarations state, and
that the Rust checker computes `writersAccepted` are premises, pinned by the
crate's checker tests and
`crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`.
-/

namespace QCore.OpensslHandshakeSlots

open QCore.CallbackContracts (Decision)
open QCore.OpensslUnstatedSlots (Runnable Premise covered wellCovered rowVerdict)
open QCore.VerdictSemigroup
open QCore.VerdictSemigroup.V

/-! ## A setter may write several slots -/

/-- A documented entry: its function, the slot its shape registers (if any)
and its further registrations. -/
structure Entry where
  function : String
  shapeSlot : Option String
  further : List String
  deriving DecidableEq, Repr

/-- `registered_slots`: the further registrations, then the shape's. -/
def registeredSlots (e : Entry) : List String := e.further ++ e.shapeSlot.toList

/-- Whether an entry registers `slot`. -/
def registers (e : Entry) (slot : String) : Bool := (registeredSlots e).contains slot

/-- `slot_writers_declared`'s writer half: every listed writer is an entry
registering the slot, and every entry registering it is a listed writer. -/
def writersAccepted (entries : List Entry) (slot : String) (writers : List String) : Bool :=
  writers.all (fun w => entries.any fun e => e.function == w && registers e slot) &&
    entries.all (fun e => !registers e slot || writers.contains e.function)

theorem a_further_registration_is_a_registration {e : Entry} {slot : String}
    (h : slot ∈ e.further) : registers e slot = true := by
  simp [registers, registeredSlots, h]

theorem the_shape_registration_is_a_registration {e : Entry} {slot : String}
    (h : e.shapeSlot = some slot) : registers e slot = true := by
  simp [registers, registeredSlots, h]

theorem an_unlisted_registering_entry_refuses {entries : List Entry} {slot : String}
    {writers : List String} {e : Entry} (he : e ∈ entries) (hr : registers e slot = true)
    (hw : e.function ∉ writers) : writersAccepted entries slot writers = false := by
  simp only [writersAccepted, Bool.and_eq_false_iff, List.all_eq_false]
  refine Or.inr ⟨e, he, ?_⟩
  simp [hr, hw]

theorem a_writer_without_the_registration_refuses {entries : List Entry} {slot : String}
    {writers : List String} {w : String} (hw : w ∈ writers)
    (hn : ∀ e ∈ entries, e.function = w → registers e slot = false) :
    writersAccepted entries slot writers = false := by
  simp only [writersAccepted, Bool.and_eq_false_iff, List.all_eq_false]
  refine Or.inl ⟨w, hw, ?_⟩
  simp only [List.any_eq_true, Bool.and_eq_true, beq_iff_eq, not_exists, not_and]
  intro e he hf hr
  rw [hn e he hf] at hr
  cases hr

theorem every_registering_entry_is_a_writer {entries : List Entry} {slot : String}
    {writers : List String} (h : writersAccepted entries slot writers = true) {e : Entry}
    (he : e ∈ entries) (hr : registers e slot = true) : e.function ∈ writers := by
  simp only [writersAccepted, Bool.and_eq_true, List.all_eq_true] at h
  have := h.2 e he
  simpa [hr, List.contains_iff_mem] using this

theorem every_writer_registers_the_slot {entries : List Entry} {slot : String}
    {writers : List String} (h : writersAccepted entries slot writers = true) {w : String}
    (hw : w ∈ writers) : ∃ e ∈ entries, e.function = w ∧ registers e slot = true := by
  simp only [writersAccepted, Bool.and_eq_true, List.all_eq_true] at h
  have := h.1 w hw
  simp only [List.any_eq_true, Bool.and_eq_true, beq_iff_eq] at this
  exact this

/-! ## A slot the library writes is never accepted -/

/-- A writer that takes no function pointer: every entry of it registers
nothing (`SSL_CTX_enable_ct`, `SSL_enable_ct`). -/
def libraryWriter (entries : List Entry) (w : String) : Prop :=
  ∀ e ∈ entries, e.function = w → registeredSlots e = []

theorem a_library_written_slot_is_never_accepted {entries : List Entry} {slot : String}
    {writers : List String} {w : String} (hw : w ∈ writers) (hl : libraryWriter entries w) :
    writersAccepted entries slot writers = false :=
  a_writer_without_the_registration_refuses hw fun e he hf => by
    simp [registers, hl e he hf]

/-- A slot a declaration may run but does not state is covered only by the
unstated-slots premise. -/
theorem an_unstated_slot_needs_the_premise {runs : List Runnable} {stated : List String}
    {premise : Premise} (h : wellCovered runs stated premise = true) {key : String}
    (hk : Runnable.slot key ∈ runs) (hs : key ∉ stated) : premise = .unstatedSlots := by
  by_cases hp : premise = .unstatedSlots
  · exact hp
  · exact absurd
      (QCore.OpensslUnstatedSlots.dropping_the_slot_premise_needs_every_slot_stated hp h hk) hs

/-! ## Stating more slots never improves a verdict -/

theorem stating_more_invocations_never_improves_a_verdict (own : V) (ds more : List Decision)
    (p : Premise) (h : rowVerdict own (ds ++ more) p = .safe) : rowVerdict own ds p = .safe := by
  induction more generalizing ds with
  | nil => simpa using h
  | cons d more ih =>
      have h' : rowVerdict own ((ds ++ [d]) ++ more) p = .safe := by
        simpa [List.append_assoc] using h
      exact QCore.OpensslUnstatedSlots.stating_an_invocation_never_improves_a_verdict own ds d p
        (ih (ds ++ [d]) h')

theorem a_row_is_safe_only_if_every_new_invocation_decides {own : V}
    {ds more : List Decision} {p : Premise} (h : rowVerdict own (ds ++ more) p = .safe) :
    ∀ d ∈ more, d = .identity := fun d hd =>
  QCore.OpensslUnstatedSlots.a_row_is_safe_only_if_every_invocation_decides h d
    (List.mem_append_right ds hd)

end QCore.OpensslHandshakeSlots
