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
# Typed indirect reads in the actuals of a contracted call
(card W44-INDIRECT-CALL-ARGUMENT-ACCESS)

The abstract rule implemented by `legacy_call_argument_access.rs` (the
recogniser, `Position`, the probe `Parser::call_argument_access` and
`refused_reads`) and its hooks in `legacy_c.rs` in `crates/qcore-c-gateway`.

A simple statement the lexical indirect-access gate refused, whose first call
is contracted, is classified again with some whole actuals leaving the gate:

* an actual is a **candidate** only when it is a typed read (a scalar value
  carrying typed scalar accesses, or a pointer read `B[I]`, `*B`, `B->F`,
  `S.F`) **and** it sits at a declared `Borrowed` position or in a declared
  variadic data tail. A custody-taking position (`Owned`, `Consumed`,
  `Released`, `Out`, `Stored`, `Removed`) never admits: the callee would take
  custody of an object the projection cannot name;
* the rule applies only when that second classification is a contracted call
  (or an element store around one) that used a candidate; otherwise the
  original refusal is kept;
* the admitted statement lowers to a block: the typed read of
  `QCore.IndirectScalarAccess` over every base the used actuals dereference,
  then the reads' second obligation (`ArgumentReads`), then the call.

The second obligation refuses (`UnknownAlias`) a name that is bound to an
identity carried by an integer (a tracked descriptor is custody, never data),
a pointer-valued read through a base whose target owns sub-custody (a field
link, a contained child or an indexed collection: the value read may be a
tracked object the read does not name), and a pointer-valued read whose own
place is bound. Both read nodes are custody-neutral, so the call sees exactly
the state it saw before the card.

## What this is not

A model of the **decision**, as the gateway's other Lean modules are. Faithful
C token/type extraction and the correspondence with the compiled Rust are
review premises (trust boundary 28). C evaluates every actual before the call
(C11 6.5.2.2p10); that the reads therefore precede the call's effects is the
sequencing this model states, not proves. Bounds are not decided.
-/

namespace QCore.CallArgumentAccess

open QCore.IndirectScalarAccess (State Binding Flags resolves accuses)

/-! ## Positions -/

/-- What the callee's declaration says about one argument index. -/
inductive Position where
  | borrowed
  | tail
  | owned
  | consumed
  | released
  | out
  | stored
  | removed
  | beyond
  | uncontracted
  deriving DecidableEq, Repr

/-- `Position::admits`: only a declared `Borrowed` position or a declared data
tail admits a read. -/
def Position.admits : Position → Bool
  | .borrowed => true
  | .tail => true
  | _ => false

/-- The custody-taking positions. -/
def Position.takesCustody : Position → Bool
  | .owned | .consumed | .released | .out | .stored | .removed => true
  | _ => false

theorem a_custody_position_never_admits (p : Position) (h : p.takesCustody = true) :
    p.admits = false := by
  cases p <;> simp_all [Position.takesCustody, Position.admits]

theorem an_uncontracted_callee_never_admits : Position.uncontracted.admits = false := rfl

theorem an_excess_actual_without_a_tail_never_admits : Position.beyond.admits = false := rfl

/-! ## Actuals -/

/-- What the recogniser answered for one actual. -/
inductive Shape where
  /-- A scalar value carrying typed scalar accesses over `bases`. -/
  | scalar (bases : List Nat)
  /-- A pointer read through `base` (none for `S.F`), whose own place, when it
  spells one, is `place`. -/
  | pointer (base : Option Nat) (place : Option Nat)
  /-- Refused by the recogniser. -/
  | refused
  deriving DecidableEq, Repr

structure Actual where
  position : Position
  /-- The actual reads memory the gate refuses. -/
  access : Bool
  shape : Shape
  /-- Every name the actual reads. -/
  names : List Nat
  deriving DecidableEq, Repr

def Shape.recognised : Shape → Bool
  | .refused => false
  | _ => true

/-- A candidate: a recognised typed read at an admitting position. -/
def Actual.candidate (a : Actual) : Bool :=
  a.access && a.shape.recognised && a.position.admits

def Shape.bases : Shape → List Nat
  | .scalar bases => bases
  | .pointer (some base) _ => [base]
  | _ => []

def Shape.pointerBases : Shape → List Nat
  | .pointer (some base) _ => [base]
  | _ => []

def Shape.pointerPlaces : Shape → List Nat
  | .pointer _ (some place) => [place]
  | _ => []

/-! ## The classifier -/

/-- One refused statement as the rule sees it. -/
structure Statement where
  /-- The refusal is the lexical gate's. -/
  gateRefused : Bool
  /-- The statement reads a tracked descriptor name. -/
  readsDescriptor : Bool
  /-- The first call's callee has a contract in force. -/
  contracted : Bool
  actuals : List Actual
  /-- The second classification, with the used candidates admitted, is a
  contracted call (or an element store around one). -/
  probeIsCall : Bool
  deriving DecidableEq, Repr

/-- The actuals the second classification used. -/
def Statement.used (s : Statement) : List Actual := s.actuals.filter Actual.candidate

def Statement.admitted (s : Statement) : Bool :=
  s.gateRefused && !s.readsDescriptor && s.contracted && !s.used.isEmpty && s.probeIsCall

/-- An instruction of the lowering. -/
inductive Instr where
  | refusal
  /-- `IndirectScalarAccess` with no destination and no frame. -/
  | typedRead (bases : List Nat)
  /-- `ArgumentReads`. -/
  | argumentReads (names pointerBases pointerPlaces : List Nat)
  | call
  deriving DecidableEq, Repr

def Statement.bases (s : Statement) : List Nat := (s.used.map fun a => a.shape.bases).flatten
def Statement.names (s : Statement) : List Nat := (s.used.map Actual.names).flatten
def Statement.pointerBases (s : Statement) : List Nat :=
  (s.used.map fun a => a.shape.pointerBases).flatten
def Statement.pointerPlaces (s : Statement) : List Nat :=
  (s.used.map fun a => a.shape.pointerPlaces).flatten

/-- The lowering: the original refusal unless admitted; otherwise the reads,
then the call. (The Rust omits an empty read node; an empty one changes
nothing, so the model keeps both.) -/
def lower (s : Statement) : List Instr :=
  if s.admitted then
    [.typedRead s.bases, .argumentReads s.names s.pointerBases s.pointerPlaces, .call]
  else [.refusal]

theorem a_statement_the_gate_did_not_refuse_keeps_its_lowering (s : Statement)
    (h : s.gateRefused = false) : lower s = [.refusal] := by
  simp [lower, Statement.admitted, h]

theorem a_descriptor_read_keeps_the_refusal (s : Statement) (h : s.readsDescriptor = true) :
    lower s = [.refusal] := by
  simp [lower, Statement.admitted, h]

theorem an_uncontracted_call_keeps_the_refusal (s : Statement) (h : s.contracted = false) :
    lower s = [.refusal] := by
  simp [lower, Statement.admitted, h]

theorem a_declined_probe_keeps_the_refusal (s : Statement) (h : s.probeIsCall = false) :
    lower s = [.refusal] := by
  simp [lower, Statement.admitted, h]

/-- No actual at a custody-taking position is ever used. -/
theorem a_used_actual_is_at_an_admitting_position (s : Statement) (a : Actual)
    (h : a ∈ s.used) : a.position.admits = true := by
  simp only [Statement.used, List.mem_filter, Actual.candidate, Bool.and_eq_true] at h
  exact h.2.2

theorem a_custody_position_is_never_used (s : Statement) (a : Actual) (h : a ∈ s.used) :
    a.position.takesCustody = false := by
  have hp := a_used_actual_is_at_an_admitting_position s a h
  cases hpos : a.position <;> simp_all [Position.admits, Position.takesCustody]

/-- The reads precede the call. -/
theorem the_reads_precede_the_call (s : Statement) (h : s.admitted = true) :
    lower s = [.typedRead s.bases, .argumentReads s.names s.pointerBases s.pointerPlaces, .call] := by
  simp [lower, h]

/-- Every base a used actual dereferences carries the typed read's obligation. -/
theorem every_used_base_is_obligated (s : Statement) (a : Actual) (base : Nat)
    (ha : a ∈ s.used) (hb : base ∈ a.shape.bases) : base ∈ s.bases := by
  simp only [Statement.bases, List.mem_flatten, List.mem_map]
  exact ⟨a.shape.bases, ⟨a, ha, rfl⟩, hb⟩

/-! ## The second obligation -/

/-- The flow facts the second obligation reads, beside the typed read's state. -/
structure Facts where
  state : State
  /-- The site is an identity carried by an integer (a tracked descriptor). -/
  integerKind : Nat → Bool
  /-- The site owns sub-custody: a field link, a contained child or an indexed
  collection. -/
  subCustody : Nat → Bool
  /-- The place (a composite name such as `p->f`) is bound. -/
  placeBound : Nat → Bool

def targets (f : Facts) (name : Nat) : List Nat :=
  match f.state.binding name with
  | some b => b.targets
  | none => []

/-- `carries_integer_identity`. -/
def carriesInteger (f : Facts) (name : Nat) : Bool := (targets f name).any f.integerKind

def ownsSubCustody (f : Facts) (base : Nat) : Bool := (targets f base).any f.subCustody

/-- `refused_reads`: every name the second obligation refuses. -/
def refusedReads (f : Facts) (names pointerBases pointerPlaces : List Nat) : List Nat :=
  names.filter (carriesInteger f) ++ pointerBases.filter (ownsSubCustody f) ++
    pointerPlaces.filter f.placeBound

inductive Diagnostic where
  | unknownAlias (name : Nat)
  | useAfterRelease (site : Nat)
  deriving DecidableEq, Repr

/-- The diagnostics of the two read nodes, from the state before the call. -/
def readDiagnostics (f : Facts) (s : Statement) : List Diagnostic :=
  (s.bases.filter (fun base => !resolves f.state base)).map Diagnostic.unknownAlias ++
    (refusedReads f s.names s.pointerBases s.pointerPlaces).map Diagnostic.unknownAlias

/-- Both read nodes are custody-neutral: the state the call sees is the state
before the statement. (The typed read has no destination and no frame.) -/
def afterReads (f : Facts) (_s : Statement) : Facts := f

theorem the_call_sees_the_state_before_the_statement (f : Facts) (s : Statement) :
    (afterReads f s).state = f.state := rfl

theorem the_typed_read_changes_no_binding (f : Facts) (s : Statement) :
    (QCore.IndirectScalarAccess.transfer f.state ⟨s.bases, none, none⟩).1.binding =
      f.state.binding := by
  funext name
  exact QCore.IndirectScalarAccess.without_a_frame_only_the_destination_changes f.state
    ⟨s.bases, none, none⟩ name rfl (by simp)

theorem a_descriptor_value_refuses (f : Facts) (names pb pp : List Nat) (name : Nat)
    (hmem : name ∈ names) (h : carriesInteger f name = true) :
    name ∈ refusedReads f names pb pp := by
  simp [refusedReads, hmem, h]

theorem a_pointer_read_through_an_owner_of_sub_custody_refuses (f : Facts)
    (names pb pp : List Nat) (base : Nat) (hmem : base ∈ pb) (h : ownsSubCustody f base = true) :
    base ∈ refusedReads f names pb pp := by
  simp [refusedReads, hmem, h]

theorem a_pointer_read_of_a_bound_place_refuses (f : Facts) (names pb pp : List Nat)
    (place : Nat) (hmem : place ∈ pp) (h : f.placeBound place = true) :
    place ∈ refusedReads f names pb pp := by
  simp [refusedReads, hmem, h]

/-- An unbound base carries no sub-custody and no integer identity. -/
theorem an_unbound_name_is_never_refused_by_the_second_obligation (f : Facts) (name : Nat)
    (h : f.state.binding name = none) :
    carriesInteger f name = false ∧ ownsSubCustody f name = false := by
  simp [carriesInteger, ownsSubCustody, targets, h]

/-- A released base leaves a reason (the typed read's refusal), so the function
cannot be `Safe`, and the settled state accuses it. -/
theorem a_released_base_refuses_and_is_accused (f : Facts) (s : Statement) (base site : Nat)
    (b : Binding) (flags : Flags) (hmem : base ∈ s.bases) (hb : f.state.binding base = some b)
    (ht : b.targets = [site]) (hk : b.unknown = false) (hs : f.state.status site = some flags)
    (hr : flags.released = true) (hu : f.state.unresolved site = false) :
    Diagnostic.unknownAlias base ∈ readDiagnostics f s ∧ accuses f.state base = some site := by
  constructor
  · have hres : resolves f.state base = false :=
      QCore.IndirectScalarAccess.a_released_base_refuses f.state base b site flags hb ht hs hr
    simp [readDiagnostics, hmem, hres]
  · cases b with
    | mk targets unknown mayBeNull derivation =>
      simp only at ht hk
      subst ht hk
      simp [accuses, hb, hu, hs, hr]

/-- With every base resolved and nothing refused, the reads add no diagnostic. -/
theorem resolved_reads_add_no_diagnostic (f : Facts) (s : Statement)
    (hb : ∀ base ∈ s.bases, resolves f.state base = true)
    (hr : refusedReads f s.names s.pointerBases s.pointerPlaces = []) :
    readDiagnostics f s = [] := by
  simp only [readDiagnostics, hr, List.map_nil, List.append_nil, List.map_eq_nil_iff,
    List.filter_eq_nil_iff]
  intro base hmem
  simp [hb base hmem]

/-- A refused read always leaves a reason, so the function cannot be `Safe`. -/
theorem a_refused_read_is_never_safe (f : Facts) (s : Statement) (name : Nat)
    (h : name ∈ refusedReads f s.names s.pointerBases s.pointerPlaces) :
    (readDiagnostics f s).isEmpty = false := by
  simp only [readDiagnostics, List.isEmpty_eq_false_iff]
  intro hnil
  rw [List.append_eq_nil_iff] at hnil
  have := hnil.2
  rw [List.map_eq_nil_iff] at this
  simp [this] at h

/-! ## Non-vacuity -/

def liveFacts : Facts :=
  { state := QCore.IndirectScalarAccess.liveState
    integerKind := fun _ => false
    subCustody := fun _ => false
    placeBound := fun _ => false }

def linkedFacts : Facts := { liveFacts with subCustody := fun site => site = 7 }

def descriptorFacts : Facts := { liveFacts with integerKind := fun site => site = 7 }

/-- `strdup(src[i])` over an unbound formal `src` (name 5), at a `Borrowed`
position of a contracted call whose probe is a call. -/
def strdupElement : Statement :=
  { gateRefused := true
    readsDescriptor := false
    contracted := true
    actuals := [⟨.borrowed, true, .pointer (some 5) none, [5, 6]⟩]
    probeIsCall := true }

/-- The same actual handed to a releasing position. -/
def releasedElement : Statement :=
  { strdupElement with actuals := [⟨.released, true, .pointer (some 5) none, [5, 6]⟩] }

theorem the_brief_example_is_admitted :
    lower strdupElement = [.typedRead [5], .argumentReads [5, 6] [5] [], .call] := by decide

theorem the_brief_example_adds_no_diagnostic : readDiagnostics liveFacts strdupElement = [] := by
  decide

theorem a_releasing_position_keeps_the_refusal : lower releasedElement = [.refusal] := by decide

/-- A pointer read through the live identity (name 0, site 7) whose target owns
a field link refuses; the same read with no link is admitted. -/
def ownerRead : Statement :=
  { strdupElement with actuals := [⟨.borrowed, true, .pointer (some 0) none, [0]⟩] }

theorem a_linked_owner_refuses_the_pointer_read :
    readDiagnostics linkedFacts ownerRead = [.unknownAlias 0] := by decide

theorem an_unlinked_live_owner_admits_the_pointer_read :
    readDiagnostics liveFacts ownerRead = [] := by decide

theorem a_scalar_read_of_a_descriptor_carrier_refuses :
    readDiagnostics descriptorFacts
      { strdupElement with actuals := [⟨.tail, true, .scalar [], [0]⟩] } = [.unknownAlias 0] := by
  decide

end QCore.CallArgumentAccess
