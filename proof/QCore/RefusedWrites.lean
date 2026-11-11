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
# What a refused statement destroys in the binding environment (card `AOM-BINDING-POISON`)

Card `AOM-DESTROY-SIBLINGS` landed a faithful release contract for
`flb_input_chunk_destroy_direct_routes` and measured, at all thirteen of its
call sites, that the declaration's *discharging* arm never fired. The reason was
not the declaration and not nullability. Every refused statement lowered to
`LegacyInstruction::Unknown`, and `legacy_c::transfer`'s arm for it ran

```rust
for pointer in state.pointers.values_mut() {
    pointer.unknown = true;
    pointer.may_be_null = true;
}
```

— it poisoned **every** binding in the state, including bindings whose names the
refused text does not so much as write. Three unrelated stores into
`result[index].…` therefore cost the acquisition at preprocessed line 19357 its
*name*, so six leak leaves retired by **withdrawal by refusal** rather than by
discharge. That residual was named `AOM-SIBLINGS-BINDING-POISON`.

This module models the repair —
`crates/qcore-c-gateway/src/legacy_c.rs`'s `RefusedWrites`, `may_write_names`
and `frame_protected_locals`, together with the `Unknown` arm of `transfer`.

## What is and is not changed

Nothing about the *refusal* moves: the `Unsupported` diagnostic still stands,
the path still carries `unmodeled`, and every exit reached through a refusal
still publishes `ExitCustody::Unknown`, so no W4 summary can be strengthened by
anything here. `state.resources` is not touched by this arm and never was. The
only thing bounded is the collateral damage to bindings, and it is bounded by an
explicit **allow-list**: `RefusedWrites.sparing` carries the places the refusal
spares, and everything outside that list — every place the projection has not
bound yet included — is poisoned exactly as before.

## The three premises

A place is spared only when it is a plain name (not a member, subscript or
`*out` slot), the frame reader recognised it as an automatic object of this body
whose address no `&` in the body takes, and the refused text writes it nowhere.
`spared_places_are_unwritable` is that conjunction; each conjunct is shown
load-bearing by a fixture that moves when it is dropped.

## Premises, not theorems

That the Rust readers extract the declaration runs, the static-storage names,
the file-scope objects, the address-of operands and the write positions
faithfully from preprocessed C is a **premise**, exactly as in
`QCore.PublicationCustody`, `QCore.BorrowedActual` and `QCore.BorrowLiveness`.
So is the no-undefined-behaviour reading of the C object model on which
"a write reaches only what it names or what an address let it reach" rests —
the same premise the projection already makes when it models `free(p)` as
releasing exactly `p`'s site.
-/

namespace QCore.RefusedWrites

/-- An abstract name, as the projection spells a place. -/
abbrev Name := Nat

/-- An abstract acquisition identity. -/
abbrev Site := Nat

/-- One binding, as `AbstractPointer` records it. `unknown` and `mayBeNull` are
    the two fields the poison sets; `targets` it never touched. -/
structure Binding where
  targets : List Site
  unknown : Bool
  mayBeNull : Bool
deriving DecidableEq, Repr

/-- A place, as the projection spells one.

    `canonical_place` and `direct_assignment_destination` build exactly these
    shapes. Only `plain` denotes a *slot*; every other shape denotes storage
    that some lvalue other than the spelling can reach, which is why the repair
    never spares one. -/
inductive Place where
  /-- `NAME` -/
  | plain (n : Name)
  /-- `NAME.f`, `NAME->f`, `NAME[0]` and any chain of those -/
  | selection (base : Name)
  /-- `*NAME`, the out-parameter slot -/
  | indirect (base : Name)
deriving DecidableEq, Repr

/-- The name sets the frame reader produces for one function body.

    Each is one reader's output over the preprocessed token stream:
    `publication::body_declarations` for the first two,
    `publication::file_scope_objects` over the whole translation unit for the
    third, the definition's parameter list for the fourth, and the `&` scan over
    the body for the last. No declared type, qualifier or per-site status
    appears here. -/
structure Frame where
  bodyDeclared : Name → Bool
  staticLocal : Name → Bool
  fileScope : Name → Bool
  formal : Name → Bool
  addressTaken : Name → Bool

namespace Frame

/-- Whether the slot `n` names is one no lvalue but the name itself designates.

    A `static` local outlives the call exactly as a file-scope object does, so a
    callee can reach it; a name that is also a file-scope object is dropped
    whatever the declaration reader said, which is what keeps a misread
    declaration run (`a * b;` contributing `b`) from sparing a global; and a
    name any `&` in the body reaches is dropped outright. -/
def protects (fr : Frame) (n : Name) : Bool :=
  (fr.bodyDeclared n || fr.formal n)
    && !fr.staticLocal n
    && !fr.fileScope n
    && !fr.addressTaken n

end Frame

/-- What a refusal may do to the binding environment.

    `unbounded` is the pre-repair relation and the answer for every refusal that
    does not stand for exactly one statement whose complete token text was read
    — a `goto` to an unresolved label, a malformed for-loop clause, a `break` or
    `continue` outside its construct, a stray or duplicate label, and every
    refused `return`, branch, loop, `switch` or `case` *condition*. -/
inductive RefusedWrites where
  | unbounded
  /-- The closed list of places this refusal spares. -/
  | sparing (spared : List Place)
deriving Repr

namespace RefusedWrites

/-- Whether this refusal poisons the binding of `p`.

    Fail-closed by construction in both directions it could be wrong: an
    unbounded refusal poisons everything, and a bounded one poisons everything
    outside a list it built positively. -/
def poisons : RefusedWrites → Place → Bool
  | unbounded, _ => true
  | sparing spared, p => !spared.contains p

end RefusedWrites

/-- The whole abstract state this arm can touch: the binding environment. -/
abbrev State := List (Place × Binding)

/-- The poison, as `transfer`'s `Unknown` arm performs it. -/
def poison (w : RefusedWrites) (st : State) : State :=
  st.map fun entry =>
    if w.poisons entry.fst then
      (entry.fst, { entry.snd with unknown := true, mayBeNull := true })
    else
      entry

/-! ## 1. The pre-repair relation is the degenerate case

The repair replaced nothing: `unbounded` *is* the old blanket poison, on every
state, so any refusal this card does not positively admit behaves exactly as it
did before. -/

/-- The old rule, written out: set both flags on every binding. -/
def blanketPoison (st : State) : State :=
  st.map fun entry => (entry.fst, { entry.snd with unknown := true, mayBeNull := true })

theorem unbounded_is_the_blanket_poison (st : State) :
    poison RefusedWrites.unbounded st = blanketPoison st := by
  simp [poison, blanketPoison, RefusedWrites.poisons]

theorem an_empty_sparing_list_is_also_the_blanket_poison (st : State) :
    poison (RefusedWrites.sparing []) st = blanketPoison st := by
  simp [poison, blanketPoison, RefusedWrites.poisons]

/-! ## 2. The three premises of a spared place

`spares` is the implemented conjunction: a plain name, frame-protected, and not
written by the refused text. -/

/-- The write set the statement's own text yields (`may_write_names`), as a
    predicate. Over-approximating it is sound and only costs precision, which is
    why the Rust reader defaults every operator run it does not recognise to
    *written*. -/
abbrev Written := Name → Bool

/-- The implemented rule, as `Parser::refused_writes` computes it: spare exactly
    the frame-protected plain names this text does not write. -/
def spares (fr : Frame) (written : Written) : Place → Bool
  | Place.plain n => fr.protects n && !written n
  | Place.selection _ => false
  | Place.indirect _ => false

/-- A selection place is never spared: it denotes storage an unrelated write can
    reach. -/
theorem a_selection_place_is_never_spared (fr : Frame) (w : Written) (b : Name) :
    spares fr w (Place.selection b) = false := by
  simp [spares]

/-- Nor is an out-parameter slot. -/
theorem an_indirect_place_is_never_spared (fr : Frame) (w : Written) (b : Name) :
    spares fr w (Place.indirect b) = false := by
  simp [spares]

/-- The conjunction, stated positively: a spared place is a plain name that the
    frame protects and the text does not write. -/
theorem spared_places_are_unwritable (fr : Frame) (w : Written) (p : Place) :
    spares fr w p = true →
      ∃ n, p = Place.plain n ∧ fr.protects n = true ∧ w n = false := by
  cases p with
  | plain n =>
      intro h
      refine ⟨n, rfl, ?_, ?_⟩ <;> simp [spares] at h <;> simp [h]
  | selection b => intro h; simp [spares] at h
  | indirect b => intro h; simp [spares] at h

/-- A name the text writes is poisoned, whatever the frame says. -/
theorem a_written_name_is_never_spared (fr : Frame) (w : Written) (n : Name)
    (hw : w n = true) : spares fr w (Place.plain n) = false := by
  simp [spares, hw]

/-- A `static` local is poisoned, whatever the text says: its object outlives
    the call. -/
theorem a_static_local_is_never_spared (fr : Frame) (w : Written) (n : Name)
    (hs : fr.staticLocal n = true) : spares fr w (Place.plain n) = false := by
  simp [spares, Frame.protects, hs]

/-- So is a file-scope object. -/
theorem a_file_scope_object_is_never_spared (fr : Frame) (w : Written) (n : Name)
    (hf : fr.fileScope n = true) : spares fr w (Place.plain n) = false := by
  simp [spares, Frame.protects, hf]

/-- So is any name a `&` in the body reaches. -/
theorem an_address_taken_name_is_never_spared (fr : Frame) (w : Written) (n : Name)
    (ha : fr.addressTaken n = true) : spares fr w (Place.plain n) = false := by
  simp [spares, Frame.protects, ha]

/-- And so is a name the reader never recognised as an object of this body:
    absence of evidence spares nothing. -/
theorem an_unrecognised_name_is_never_spared (fr : Frame) (w : Written) (n : Name)
    (hd : fr.bodyDeclared n = false) (hp : fr.formal n = false) :
    spares fr w (Place.plain n) = false := by
  simp [spares, Frame.protects, hd, hp]

/-! ## 3. Monotonicity — the repair only ever *removes* poison

Nothing here can poison a binding the old rule left alone, and a refusal that
spares nothing is exactly the old rule (§1). -/

/-- Every binding the repaired rule poisons, the blanket rule poisoned too.
    Contrapositively: the repair can only spare. -/
theorem the_repair_only_removes_poison (w : RefusedWrites) (p : Place) :
    w.poisons p = true → (RefusedWrites.unbounded : RefusedWrites).poisons p = true := by
  intro _
  simp [RefusedWrites.poisons]

/-- Recognising *fewer* names as protected can only poison more. Stated as the
    pointwise implication on `spares`, which is what makes every fail-closed
    default above (an unreadable body yields the empty frame) safe by
    construction. -/
theorem a_weaker_frame_spares_less (fr fr' : Frame) (w : Written) (p : Place)
    (h : ∀ n, fr'.protects n = true → fr.protects n = true) :
    spares fr' w p = true → spares fr w p = true := by
  cases p with
  | plain n =>
      intro hp
      simp [spares] at hp ⊢
      exact ⟨h n hp.1, hp.2⟩
  | selection b => intro hp; simp [spares] at hp
  | indirect b => intro hp; simp [spares] at hp

/-! ## 4. The refusal itself is untouched

The two facts that make this repair unable to promote anything. Both are
properties of `transfer`'s arm as a whole rather than of the poison: the arm
reports its `Unsupported` diagnostic before it poisons anything, and it writes
no resource state at all. -/

/-- The status a refusal publishes, as `classify_diagnostics` reads it. -/
inductive Status where
  | safe
  | unknown
  | violation
deriving DecidableEq, Repr

/-- A function carrying a refusal is at best `unknown`, whatever the binding
    environment says: the `Unsupported` diagnostic is emitted unconditionally,
    and `unknown_found` in `classify_diagnostics` covers it. -/
def statusWithARefusal (hasViolation : Bool) : Status :=
  if hasViolation then Status.violation else Status.unknown

theorem a_refusal_never_certifies (v : Bool) : statusWithARefusal v ≠ Status.safe := by
  cases v <;> simp [statusWithARefusal]

/-- The poison does not read or write resource states, so a refusal moves no
    site's disposition in either direction. Modelled by the state being a pair
    whose second component `poison` cannot mention. -/
theorem the_poison_moves_no_resource_state
    (w : RefusedWrites) (st : State) (resources : List (Site × Nat)) :
    (poison w st, resources).snd = resources := rfl

/-! ## 5. Non-vacuity — the rule both spares and poisons

An empty model that spared nothing would satisfy everything above, so both
directions are exhibited. The fixture is card `AOM-DESTROY-SIBLINGS`' own:
`result` acquired, then a refused `result[index].id = read_id;`, which writes
the selection and reads the base. -/

/-- `result`. -/
def result : Name := 0

/-- `view`, whose address `input_chunk_metadata_view(ic, &view)` takes. -/
def view : Name := 1

/-- The frame of `flb_input_chunk_get_direct_routes`, abstracted: both names are
    declared automatic locals, and only `view` is address-taken. -/
def routesFrame : Frame where
  bodyDeclared := fun n => n == result || n == view
  staticLocal := fun _ => false
  fileScope := fun _ => false
  formal := fun _ => false
  addressTaken := fun n => n == view

/-- The write set of `result[index].id = read_id;`: the subscript base is read,
    not written. -/
def identifierStoreWrites : Written := fun _ => false

theorem the_acquisition_survives_the_identifier_loop :
    spares routesFrame identifierStoreWrites (Place.plain result) = true := by
  decide

theorem the_address_taken_aggregate_does_not :
    spares routesFrame identifierStoreWrites (Place.plain view) = false := by
  decide

/-- And the member place the store actually writes is poisoned, so nothing about
    `result`'s *contents* is spared. -/
theorem the_written_member_is_still_poisoned :
    spares routesFrame identifierStoreWrites (Place.selection result) = false := by
  decide

/-- The state-level statement of the same fixture: the binding of `result`
    keeps its targets and both flags, and the member binding loses both. -/
theorem the_fixture_state_spares_exactly_one_binding :
    poison
        (RefusedWrites.sparing [Place.plain result])
        [(Place.plain result, ⟨[7], false, false⟩),
         (Place.selection result, ⟨[9], false, false⟩)]
      = [(Place.plain result, ⟨[7], false, false⟩),
         (Place.selection result, ⟨[9], true, true⟩)] := by
  decide

end QCore.RefusedWrites
