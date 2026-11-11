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
# Typed scalar accesses through pointers (card W43-INDIRECT-ACCESS-GENERAL)

The abstract rule implemented by
`typed_field_access::typed_scalar_statement`,
`LegacyInstruction::IndirectScalarAccess`, `indirect_scalar_base_resolves` and
`report_indirect_scalar_violations` in `crates/qcore-c-gateway`.

A statement whose every memory access is a typed scalar access (`*B`, `B[I]`,
`B->F`, `S.F`) and whose destination and value are scalar is admitted **only
after the existing classifier refused it**. It is custody-neutral: it changes
no resource status, no nullable or unresolved-effect marker and no custody or
field-link map, and it binds no name to a new identity. What it keeps is the
dereference obligation of each base, decided on one binding:

* no binding on any reaching path — admitted;
* a non-null binding with no tracked target — admitted;
* exactly one known, non-null target whose status is exactly live, with no
  nullable or unresolved-effect marker — admitted, identity or derived alike;
* everything else — refused with an unknown-alias reason.

A base settled on one identity whose status carries the released bit is a
`useAfterRelease` candidate. A write through `*B` or `B[I]` poisons every
binding outside the refusal's frame, as the refusal it replaces did.

## What this is not

A model of the **decision**, as the gateway's other Lean modules are. Faithful
C token/type extraction and the correspondence with the compiled Rust are
review premises (trust boundary 28). The model does not decide bounds: a
subscript's index is read as a scalar and nothing is claimed about the
object's extent. That the store cannot rewrite a pointer's representation
outside the frame is the C object model under the no-undefined-behaviour
reading constraint 10a already rests on, stated here as the frame, not proved.
-/

namespace QCore.IndirectScalarAccess

/-- A resource status as the Rust bit set carries it: several bits may be set
after a join, and "live" means exactly the live bit. -/
structure Flags where
  live : Bool
  released : Bool
  escaped : Bool
  deriving DecidableEq, Repr

def Flags.exactlyLive (flags : Flags) : Bool :=
  flags.live && !flags.released && !flags.escaped

/-- How a binding relates to its targets. -/
inductive Derivation where
  | identity
  | derived
  | mixed
  deriving DecidableEq, Repr

structure Binding where
  targets : List Nat
  unknown : Bool
  mayBeNull : Bool
  derivation : Derivation
  deriving DecidableEq, Repr

/-- The resource half of the projection that this rule must not touch is
represented by opaque counters, as `QCore.ScalarMemberStore` does. -/
structure Rest where
  custody : Nat
  fieldLinks : Nat
  contained : Nat
  deriving DecidableEq, Repr

structure State where
  binding : Nat → Option Binding
  status : Nat → Option Flags
  nullable : Nat → Bool
  unresolved : Nat → Bool
  rest : Rest

inductive Diagnostic where
  | unknownAlias (base : Nat)
  | useAfterRelease (site : Nat)
  deriving DecidableEq, Repr

/-- The dereference obligation of one base (`indirect_scalar_base_resolves`). -/
def resolves (s : State) (base : Nat) : Bool :=
  match s.binding base with
  | none => true
  | some b =>
      if b.mayBeNull || b.derivation == .mixed then false
      else
        match b.targets with
        | [] => b.unknown
        | [site] =>
            !b.unknown && !s.nullable site && !s.unresolved site &&
              (match s.status site with
               | some flags => flags.exactlyLive
               | none => false)
        | _ => false

/-- The accusing reading from the settled state
(`report_indirect_scalar_violations`). -/
def accuses (s : State) (base : Nat) : Option Nat :=
  match s.binding base with
  | some ⟨[site], false, _, _⟩ =>
      if s.unresolved site then none
      else
        match s.status site with
        | some flags => if flags.released then some site else none
        | none => none
  | _ => none

/-- One admitted statement: the bases it dereferences, the declared scalar it
writes (if any), and, for a write through `*B`/`B[I]`, the names its frame
spares. -/
structure Access where
  bases : List Nat
  destination : Option Nat
  frame : Option (Nat → Bool)

def poison (b : Binding) : Binding :=
  { b with unknown := true, mayBeNull := true }

/-- The bindings after the statement: the scalar destination is forgotten and
every binding outside a write's frame is poisoned. -/
def rebind (s : State) (a : Access) (name : Nat) : Option Binding :=
  if a.destination = some name then none
  else
    match a.frame, s.binding name with
    | some spares, some b => if spares name then some b else some (poison b)
    | _, b => b

def refusals (s : State) (a : Access) : List Diagnostic :=
  (a.bases.filter (fun base => !resolves s base)).map Diagnostic.unknownAlias

def transfer (s : State) (a : Access) : State × List Diagnostic :=
  ({ s with binding := rebind s a }, refusals s a)

/-- A function is `Safe` only with no diagnostic at all. -/
def safe (diagnostics : List Diagnostic) : Bool := diagnostics.isEmpty

/-! ## Custody neutrality -/

theorem the_statuses_are_unchanged (s : State) (a : Access) :
    (transfer s a).1.status = s.status := rfl

theorem the_markers_are_unchanged (s : State) (a : Access) :
    (transfer s a).1.nullable = s.nullable ∧ (transfer s a).1.unresolved = s.unresolved :=
  ⟨rfl, rfl⟩

theorem custody_and_field_links_are_unchanged (s : State) (a : Access) :
    (transfer s a).1.rest = s.rest := rfl

/-- No name is bound to a new identity: every binding after the statement
carries exactly the targets it carried before. -/
theorem no_binding_gains_a_target (s : State) (a : Access) (name : Nat) (b : Binding)
    (h : (transfer s a).1.binding name = some b) :
    ∃ before, s.binding name = some before ∧ b.targets = before.targets := by
  simp only [transfer, rebind] at h
  split at h
  · simp at h
  · split at h
    · rename_i spares before _ hb
      split at h
      · exact ⟨before, hb, by simp_all⟩
      · exact ⟨before, hb, by cases h; rfl⟩
    · rename_i hcase
      exact ⟨b, h, rfl⟩

/-- A spared name other than the destination keeps its binding exactly. -/
theorem a_spared_binding_is_untouched (s : State) (a : Access) (name : Nat)
    (spares : Nat → Bool) (hframe : a.frame = some spares) (hspared : spares name = true)
    (hdest : a.destination ≠ some name) :
    (transfer s a).1.binding name = s.binding name := by
  simp only [transfer, rebind, hdest, hframe]
  cases hb : s.binding name <;> simp [hspared]

/-- A read or a member store (no frame) keeps every binding but its scalar
destination. -/
theorem without_a_frame_only_the_destination_changes (s : State) (a : Access) (name : Nat)
    (hframe : a.frame = none) (hdest : a.destination ≠ some name) :
    (transfer s a).1.binding name = s.binding name := by
  simp [transfer, rebind, hdest, hframe]

/-! ## The dereference obligation -/

theorem an_unbound_base_resolves (s : State) (base : Nat) (h : s.binding base = none) :
    resolves s base = true := by
  simp [resolves, h]

theorem a_nullable_base_refuses (s : State) (base : Nat) (b : Binding)
    (h : s.binding base = some b) (hn : b.mayBeNull = true) :
    resolves s base = false := by
  simp [resolves, h, hn]

theorem a_mixed_base_refuses (s : State) (base : Nat) (b : Binding)
    (h : s.binding base = some b) (hm : b.derivation = .mixed) :
    resolves s base = false := by
  simp [resolves, h, hm]

theorem an_ambiguous_base_refuses (s : State) (base : Nat) (b : Binding) (x y : Nat)
    (rest : List Nat) (h : s.binding base = some b) (ht : b.targets = x :: y :: rest) :
    resolves s base = false := by
  simp [resolves, h, ht]

theorem an_unknown_base_with_a_target_refuses (s : State) (base : Nat) (b : Binding)
    (site : Nat) (h : s.binding base = some b) (ht : b.targets = [site])
    (hu : b.unknown = true) :
    resolves s base = false := by
  simp [resolves, h, ht, hu]

theorem a_released_base_refuses (s : State) (base : Nat) (b : Binding) (site : Nat)
    (flags : Flags) (h : s.binding base = some b) (ht : b.targets = [site])
    (hs : s.status site = some flags) (hr : flags.released = true) :
    resolves s base = false := by
  simp [resolves, h, ht, hs, Flags.exactlyLive, hr]

theorem an_unresolved_base_refuses (s : State) (base : Nat) (b : Binding) (site : Nat)
    (h : s.binding base = some b) (ht : b.targets = [site]) (hu : s.unresolved site = true) :
    resolves s base = false := by
  simp [resolves, h, ht, hu]

/-- An admitted tracked base denotes exactly one live identity. -/
theorem a_resolved_tracked_base_is_one_live_identity (s : State) (base : Nat) (b : Binding)
    (site : Nat) (h : s.binding base = some b) (ht : b.targets = [site])
    (hr : resolves s base = true) :
    b.unknown = false ∧ b.mayBeNull = false ∧ s.unresolved site = false ∧
      ∃ flags, s.status site = some flags ∧ flags.exactlyLive = true := by
  simp only [resolves, h, ht] at hr
  cases hn : b.mayBeNull <;> cases hm : (b.derivation == .mixed) <;>
    simp_all
  cases hs : s.status site <;> simp_all

/-- The accusing arm and the admitting arm never meet on one base. -/
theorem a_resolved_base_is_never_accused (s : State) (base : Nat)
    (hr : resolves s base = true) : accuses s base = none := by
  unfold accuses
  split
  · rename_i site mb d hb
    simp only [resolves, hb] at hr
    cases hmb : mb <;> cases hd : (d == Derivation.mixed) <;> simp_all
    · cases hs : s.status site <;> simp_all [Flags.exactlyLive]
  · rfl

/-- An accusation names a settled identity whose status carries the released
bit and whose fate the projection committed to. -/
theorem an_accusation_names_a_released_settled_identity (s : State) (base site : Nat)
    (h : accuses s base = some site) :
    s.unresolved site = false ∧ ∃ flags, s.status site = some flags ∧ flags.released = true := by
  unfold accuses at h
  split at h
  · rename_i site' mb d hb
    split at h
    · simp at h
    · rename_i hu
      split at h
      · rename_i flags hs
        split at h
        · rename_i hrel
          cases h
          exact ⟨by simpa using hu, flags, hs, hrel⟩
        · simp at h
      · simp at h
  · simp at h

/-! ## Verdicts -/

/-- A statement whose every base resolves adds no diagnostic. -/
theorem an_admitted_statement_adds_no_diagnostic (s : State) (a : Access)
    (h : ∀ base ∈ a.bases, resolves s base = true) : (transfer s a).2 = [] := by
  simp only [transfer, refusals]
  rw [List.filter_eq_nil_iff.mpr]
  · rfl
  · intro base hmem
    simp [h base hmem]

/-- A refused base always leaves a reason, so the function cannot be `Safe`. -/
theorem a_refusal_is_never_safe (s : State) (a : Access) (base : Nat)
    (hmem : base ∈ a.bases) (hr : resolves s base = false) :
    safe (transfer s a).2 = false := by
  simp only [safe, transfer, refusals, List.isEmpty_eq_false_iff]
  intro hnil
  have : base ∈ a.bases.filter (fun base => !resolves s base) := by
    simp [hmem, hr]
  rw [List.map_eq_nil_iff] at hnil
  simp [hnil] at this

/-! ## The classifier: consulted only after a refusal -/

/-- The syntactic premises `typed_scalar_statement` checks, as booleans. -/
structure Candidate where
  noCall : Bool
  noUnaryAddress : Bool
  noComma : Bool
  noNestedWrite : Bool
  everyAccessTyped : Bool
  scalarDestination : Bool
  scalarValue : Bool
  incrementsThePointer : Bool
  deriving DecidableEq, Repr

def admitsSyntax (c : Candidate) : Bool :=
  c.noCall && c.noUnaryAddress && c.noComma && c.noNestedWrite && c.everyAccessTyped &&
    c.scalarDestination && c.scalarValue && !c.incrementsThePointer

/-- The wrapped classifier: the existing answer unless it refused and the typed
route admits the statement. -/
def classify {Instr : Type} (base : Instr) (refused : Bool) (c : Candidate)
    (route : Instr) : Instr :=
  if refused && admitsSyntax c then route else base

/-- Conservativity: a statement the existing classifier admits keeps its exact
lowering, whatever the route would have said. -/
theorem an_admitted_statement_keeps_its_lowering {Instr : Type} (base route : Instr)
    (c : Candidate) : classify base false c route = base := by
  simp [classify]

theorem a_call_keeps_the_refusal {Instr : Type} (base route : Instr) (c : Candidate)
    (h : c.noCall = false) : classify base true c route = base := by
  simp [classify, admitsSyntax, h]

theorem an_address_keeps_the_refusal {Instr : Type} (base route : Instr) (c : Candidate)
    (h : c.noUnaryAddress = false) : classify base true c route = base := by
  simp [classify, admitsSyntax, h]

theorem an_untyped_access_keeps_the_refusal {Instr : Type} (base route : Instr)
    (c : Candidate) (h : c.everyAccessTyped = false) : classify base true c route = base := by
  simp [classify, admitsSyntax, h]

theorem a_pointer_value_keeps_the_refusal {Instr : Type} (base route : Instr) (c : Candidate)
    (h : c.scalarValue = false) : classify base true c route = base := by
  simp [classify, admitsSyntax, h]

theorem a_pointer_destination_keeps_the_refusal {Instr : Type} (base route : Instr)
    (c : Candidate) (h : c.scalarDestination = false) : classify base true c route = base := by
  simp [classify, admitsSyntax, h]

theorem incrementing_the_pointer_keeps_the_refusal {Instr : Type} (base route : Instr)
    (c : Candidate) (h : c.incrementsThePointer = true) : classify base true c route = base := by
  simp [classify, admitsSyntax, h]

/-! ## Non-vacuity -/

def liveState : State :=
  { binding := fun n => if n = 0 then some ⟨[7], false, false, .identity⟩ else none
    status := fun site => if site = 7 then some ⟨true, false, false⟩ else none
    nullable := fun _ => false
    unresolved := fun _ => false
    rest := ⟨1, 2, 3⟩ }

def releasedState : State :=
  { liveState with status := fun site => if site = 7 then some ⟨false, true, false⟩ else none }

def derivedUntracked : State :=
  { liveState with binding := fun n => if n = 0 then some ⟨[], true, false, .derived⟩ else none }

theorem a_live_identity_is_admitted : resolves liveState 0 = true := by decide

theorem an_unbound_formal_is_admitted : resolves liveState 5 = true := by decide

theorem a_guarded_untracked_derived_pointer_is_admitted :
    resolves derivedUntracked 0 = true := by decide

theorem a_released_identity_is_refused_and_accused :
    resolves releasedState 0 = false ∧ accuses releasedState 0 = some 7 := by decide

theorem the_live_identity_is_not_accused : accuses liveState 0 = none := by decide

theorem a_full_syntax_candidate_is_admitted :
    admitsSyntax ⟨true, true, true, true, true, true, true, false⟩ = true := by decide

end QCore.IndirectScalarAccess
