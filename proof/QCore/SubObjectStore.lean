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

import QCore.IndirectScalarAccess

/-!
# Stores into a sub-object through a resolved base (card W45-SUB-OBJECT-STORES)

The abstract rule implemented by `legacy_c::sub_object_store`
(`read_destination`, `read_value`, `transfer`, `bases_resolve`,
`stores_no_identity`, `call_runs`) and `Parser::sub_object_store_statement` in
`crates/qcore-c-gateway`.

A statement the lexical gate refused as a store into a sub-object (`R->F`,
`R[I].F`, `(R)->F`, `S.F`, `(&S)->F`, `A[I].F`) is lowered, when its root and
member resolve through the declaration reader, to a store decided on the state
at the node:

* the dereferenced base must satisfy W43's obligation (`resolves`), and for a
  pointer member it must carry no tracked target (a tracked aggregate's member
  may hold a field link only O04-B08's store rewrites);
* a scalar, null or literal value is custody-neutral; the member's place
  binding is forgotten;
* a named pointer value (a stored call's result is bound to a temporary first)
  is admitted only when it stores no identity this function tracks: no binding,
  or a binding with no target that is not unknown;
* everything else is the fallback, the gate's refusal: it reports, changes no
  status, and the refused hand-on rule (card W44-LEAK-PIN-AUDIT) marks the
  value's identities unresolved.

The classifier never reads a formal's pointer as a stored value, and reads a
stored call only when its contract states no result custody. So no admitted
store moves custody: the store of an owned identity into a record the caller
reaches is exactly the transfer O04-B08 and W44-LEAK-PIN-AUDIT refuse, because
no summary mode states it and a caller's `Safe` would rest on it unseen.

## What this is not

A model of the **decision**, as the gateway's other Lean modules are. Faithful
token and declaration extraction and the correspondence with the compiled Rust
are review premises (trust boundary 28). That a member store writes that one
member and nothing else is the C object model under the no-undefined-behaviour
reading constraint 10a rests on, stated here, not proved.
-/

namespace QCore.SubObjectStore

open QCore.IndirectScalarAccess

/-- The destination: the dereferenced base (none for this function's own
record), the member's place, and whether the member holds a pointer. -/
structure Destination where
  base : Option Nat
  place : Nat
  pointerMember : Bool

/-- The value after classification (a call result is its temporary's name). -/
inductive Value where
  | scalar
  | untracked
  | name (n : Nat)

def tracked (s : State) (n : Nat) : Bool :=
  match s.binding n with
  | some b => !b.targets.isEmpty
  | none => false

/-- Step 1: the base premises (`bases_resolve`). -/
def baseOk (s : State) (d : Destination) : Bool :=
  match d.base with
  | none => true
  | some b => resolves s b && !(d.pointerMember && tracked s b)

/-- Step 3 (`stores_no_identity`): the named value stores no identity this
function tracks. -/
def storesNoIdentity (s : State) (n : Nat) : Bool :=
  match s.binding n with
  | none => true
  | some b => b.targets.isEmpty && !b.unknown

def forget (s : State) (place : Nat) : State :=
  { s with binding := fun x => if x = place then none else s.binding x }

inductive Outcome where
  | admitted (s : State)
  | refused

/-- The store's transfer, fallback excluded. -/
def store (s : State) (d : Destination) (v : Value) : Outcome :=
  if !baseOk s d then .refused
  else
    match v with
    | .scalar | .untracked => .admitted (forget s d.place)
    | .name n => if storesNoIdentity s n then .admitted (forget s d.place) else .refused

/-- The fallback, as far as custody goes: the refused hand-on rule marks every
target of the stored value unresolved and changes no status. -/
def handOn (s : State) (v : Value) : State :=
  match v with
  | .name n =>
      match s.binding n with
      | some b => { s with unresolved := fun x => s.unresolved x || b.targets.contains x }
      | none => s
  | _ => s

/-- The classifier's value premises: a formal's pointer is never a stored
value, and a stored call is read only with no result custody. -/
def admitsName (formal : Nat → Bool) (n : Nat) : Bool := !formal n

inductive ResultMode where
  | none
  | owned
  | derived
  | reallocated
  deriving DecidableEq, Repr

def admitsCall (result : ResultMode) (everyParameterBorrowed : Bool) : Bool :=
  result == .none && everyParameterBorrowed

/-! ## What an admitted store leaves -/

theorem store_admitted (s s' : State) (d : Destination) (v : Value)
    (h : store s d v = .admitted s') : baseOk s d = true ∧ s' = forget s d.place := by
  unfold store at h
  by_cases hbase : baseOk s d = true
  · refine ⟨hbase, ?_⟩
    simp only [hbase, Bool.not_true, Bool.false_eq_true, ite_false] at h
    cases v with
    | scalar => simp only [Outcome.admitted.injEq] at h; exact h.symm
    | untracked => simp only [Outcome.admitted.injEq] at h; exact h.symm
    | name n =>
      by_cases hn : storesNoIdentity s n = true
      · simp only [hn, ite_true, Outcome.admitted.injEq] at h; exact h.symm
      · simp [hn] at h
  · simp [hbase] at h

/-- Custody neutrality: no admitted store changes a status, a marker or the
custody and field-link maps. -/
theorem an_admitted_store_moves_no_custody (s s' : State) (d : Destination) (v : Value)
    (h : store s d v = .admitted s') :
    s'.status = s.status ∧ s'.unresolved = s.unresolved ∧ s'.nullable = s.nullable ∧
      s'.rest = s.rest := by
  rcases store_admitted s s' d v h with ⟨_, rfl⟩
  exact ⟨rfl, rfl, rfl, rfl⟩

/-- The member's old fact is forgotten. -/
theorem an_admitted_store_forgets_the_member (s s' : State) (d : Destination) (v : Value)
    (h : store s d v = .admitted s') : s'.binding d.place = none := by
  rcases store_admitted s s' d v h with ⟨_, rfl⟩
  simp [forget]

/-- Constraint 5i: the store writes the member's place and no name. -/
theorem an_admitted_store_rebinds_no_other_name (s s' : State) (d : Destination)
    (v : Value) (x : Nat) (hx : x ≠ d.place) (h : store s d v = .admitted s') :
    s'.binding x = s.binding x := by
  rcases store_admitted s s' d v h with ⟨_, rfl⟩
  simp [forget, hx]

/-! ## The base premises -/

theorem an_unresolved_base_refuses (s : State) (d : Destination) (v : Value)
    (b : Nat) (hb : d.base = some b) (hr : resolves s b = false) :
    store s d v = .refused := by
  simp [store, baseOk, hb, hr]

theorem a_pointer_store_through_a_tracked_base_refuses (s : State)
    (d : Destination) (v : Value) (b : Nat) (hb : d.base = some b)
    (hp : d.pointerMember = true) (ht : tracked s b = true) :
    store s d v = .refused := by
  simp [store, baseOk, hb, hp, ht]

/-! ## A tracked identity is never stored -/

/-- Any value bound to a target — an owned identity, a caller's site, a
descriptor, a derived pointer, whatever the destination — keeps the refusal:
no `Safe` rests on a store that moves custody unseen. -/
theorem a_stored_identity_keeps_the_refusal (s : State) (d : Destination) (n site : Nat)
    (b : Binding) (hb : s.binding n = some b) (hmem : site ∈ b.targets) :
    store s d (.name n) = .refused := by
  have hne : b.targets.isEmpty = false := by
    cases ht : b.targets with
    | nil => simp [ht] at hmem
    | cons _ _ => rfl
  simp [store, storesNoIdentity, hb, hne]

/-- An unknown (poisoned) binding may denote any identity: refused. -/
theorem an_unknown_value_refuses (s : State) (d : Destination) (n : Nat) (b : Binding)
    (hb : s.binding n = some b) (hu : b.unknown = true) :
    store s d (.name n) = .refused := by
  simp [store, storesNoIdentity, hb, hu]

/-- A value no reaching path bound stores no tracked identity. -/
theorem an_unbound_value_is_neutral (s : State) (d : Destination) (n : Nat)
    (hbase : baseOk s d = true) (hb : s.binding n = none) :
    store s d (.name n) = .admitted (forget s d.place) := by
  simp [store, storesNoIdentity, hbase, hb]

/-- A formal's pointer is never read as a stored value. -/
theorem a_formal_value_keeps_the_refusal (formal : Nat → Bool) (n : Nat)
    (h : formal n = true) : admitsName formal n = false := by
  simp [admitsName, h]

/-- An owned, derived or reallocated call result is never read as a stored
value. -/
theorem a_custody_result_keeps_the_refusal (result : ResultMode) (borrowed : Bool)
    (h : result ≠ .none) : admitsCall result borrowed = false := by
  cases result <;> simp_all [admitsCall]

/-! ## The refused store still hands on -/

/-- The fallback discharges nothing: no status changes. -/
theorem the_fallback_changes_no_status (s : State) (v : Value) :
    (handOn s v).status = s.status := by
  unfold handOn
  split
  · split <;> rfl
  · rfl

/-- The fallback marks every target of the stored value unresolved, so no leak
is claimed for an identity a refused store may have stored. -/
theorem the_fallback_marks_the_stored_identity (s : State) (n site : Nat) (b : Binding)
    (hb : s.binding n = some b) (hmem : site ∈ b.targets) :
    (handOn s (.name n)).unresolved site = true := by
  simp [handOn, hb, hmem]

/-! ## Non-vacuity -/

/-- Name 0 holds the live site 7; name 3 is the formal base, never bound. -/
def owner : State :=
  { binding := fun n => if n = 0 then some ⟨[7], false, false, .identity⟩ else none
    status := fun site => if site = 7 then some ⟨true, false, false⟩ else none
    nullable := fun _ => false
    unresolved := fun _ => false
    rest := ⟨0, 0, 0⟩ }

def callersRecord : Destination := ⟨some 3, 30, true⟩

theorem the_formal_base_resolves : baseOk owner callersRecord = true := by decide

theorem the_owned_pointer_into_the_callers_record_is_refused :
    storesNoIdentity owner 0 = false := by decide

theorem an_unbound_pointer_stores_no_identity : storesNoIdentity owner 5 = true := by decide

theorem a_scalar_store_is_admitted :
    ∃ s', store owner callersRecord .scalar = .admitted s' := ⟨_, rfl⟩

theorem a_none_result_call_is_read : admitsCall .none true = true := by decide

end QCore.SubObjectStore
