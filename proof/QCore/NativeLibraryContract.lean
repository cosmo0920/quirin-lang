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

import QCore.NativeLibrary

/-! Card `NATIVE-LIBRARY-CONTRACT-SLICE`: the **contracted** half of the
`source.native-library.v1` manifest — level L3 of the linkage spectrum, where a
library's pointer symbols belong.

Card `UNSAFE-LINKAGE` established that pointers cross exactly where a contract
exists to state their premise; card `UNSAFE-LINKAGE-ARTIFACT` built the manifest
but refused a `contract fn` entry by name. This module models what lifting that
refusal means: a manifest entry binds a symbol to a **declared contract module**
and to the artifact identity, so a contracted extern call resolves through the
manifest and inherits the build.

The vocabulary is **imported** from `QCore.NativeLibrary` — the identity, the
ABI premises, the residence record and the premise inductive — so there is one
premise vocabulary in this model and not two that could drift. The contract
machinery itself is not modelled here at all, and deliberately: the manifest
*selects and attributes*, it creates no contract semantics, and what a declared
contract means is `QCore.ExternCall`'s business, unchanged.

What is established here:

* **the premises stack** — a manifest-bound contracted call carries the declared
  contract premise *and* the artifact premise, and the artifact premise never
  replaces the contract one;
* **there is no premise-free path** through a manifest-bound contracted call;
* **an unmanifested contract does not move** — its premise list is exactly the
  one-element list it was;
* **a contradicted binding withdraws**, and the withdrawal admits nothing, so it
  is monotone in the refusing direction. The direction is the whole argument and
  it is stated as a theorem: at level L1 the weaker reading below a withdrawn
  binding is the bare waiver, which is *accepted*, so that card refuses instead;
  here the weaker reading is the uncontracted callee, which *refuses*;
* **the binding is total in both directions** — an entry whose contract the
  environment does not declare, an entry naming another module, and a contract
  in the slice that no entry binds each make the slice non-total;
* **the waiver mirror needs no gate** — slice totality already implies that a
  manifest-contracted symbol is declared by the environment, which is what makes
  the `Q034` refusal reach it.

What is **not** established, here or anywhere: that the declared contract is
true of the named artifact, or that the linker used it. Both stay premises. -/

namespace QCore.NativeLibraryContract

open QCore.ExternCall (AbiType isPointer)
open QCore.NativeLibrary (Identity Premises Residence HeaderIdentity Premise waives)

/-! ## The contract slice -/

/-- One contract slice entry: a symbol, the artifact's exported header
declaration, and the declared contract module the symbol is read through.
Pointers are admitted in every position — that is the whole point of the level,
and the reason `QCore.NativeLibrary.entryAdmissible` refuses them for a
contract-less entry is that such an entry has no modes in which to state what a
pointer may be used for. This one does. -/
structure Entry where
  symbol : String
  parameters : List AbiType
  result : AbiType
  contractModule : String
  deriving DecidableEq, Repr

/-- A loaded manifest, at this card's slice. -/
structure Manifest where
  identity : Identity
  premises : Premises
  /-- The exported header this manifest binds (card `NATIVE-LIBRARY-HEADER`).
  `none` is this card's world, and `QCore.NativeLibraryHeader` is where a bound
  one is decided. -/
  header : Option HeaderIdentity
  entries : List Entry
  deriving DecidableEq, Repr

/-- Resolve one symbol against a manifest. -/
def Manifest.binding (m : Manifest) (symbol : String) : Option Entry :=
  m.entries.find? (fun e => e.symbol == symbol)

/-- The manifest's **slice**: the contract modules its entries name. Derived
rather than declared separately, because a second spelling would be a second
place the slice could be wrong. -/
def Manifest.slice (m : Manifest) : List String :=
  m.entries.map Entry.contractModule

/-- The residence one contract slice entry contributes. It names the contract
module, which is what makes the published premise read *contracted, per this
contract, from this build*. -/
def Manifest.residence (m : Manifest) (e : Entry) : Residence :=
  ⟨m.identity, m.premises, some e.contractModule, m.header⟩

/-- One declaration of the selected contract environment, projected to what the
gate reads: the name, the module that declares it, and its ABI projection. -/
structure Declaration where
  name : String
  module : String
  parameters : List AbiType
  result : AbiType
  deriving DecidableEq, Repr

/-- The selected contract environment. -/
abbrev Environment := List Declaration

/-- Design §4.1, forward direction: an entry resolves to a declared contract, in
the module it names. The module reference is compared **exactly** — a contract
in another module is another contract, and resolving by bare symbol name would
let a manifest attribute a build to a declaration it never named. -/
def entryResolves (env : Environment) (e : Entry) : Bool :=
  match env.find? (fun d => d.name == e.symbol) with
  | none => false
  | some d => d.module == e.contractModule

/-- Design §4.1, both directions. The second conjunct is the reverse one: every
declaration of a module *in the slice* is bound by an entry. The slice is the
library's exported surface, so a declaration inside it that no entry names makes
the binding partial — and a partial binding is not one. -/
def sliceTotal (env : Environment) (m : Manifest) : Bool :=
  m.entries.all (entryResolves env) &&
    env.all fun d =>
      !m.slice.contains d.module || m.entries.any (fun e => e.symbol == d.name)

/-- The type gate of design §4.2 at the contracted setting: the declared
contract's own ABI projection must equal the manifest's exported header
declaration. -/
def agrees (e : Entry) (d : Declaration) : Bool :=
  e.parameters == d.parameters && e.result == d.result

/-! ## Admission, and withdrawal -/

/-- What binding one contracted declaration through a manifest produces. Three
constructors and no catch-all: no manifest names it; a manifest names it and
agrees; or a manifest names it and contradicts it, which **withdraws** the
declaration for this unit. -/
inductive Outcome where
  /-- No selected manifest binds this symbol. The contracted route is exactly
  what it was. -/
  | unmanifested
  /-- A manifest binds it and the two declarations agree. -/
  | resident (e : Entry) (r : Residence)
  /-- A manifest binds it and contradicts it: the declaration is withdrawn for
  this unit and the call sites become uncontracted. -/
  | withdrawn (e : Entry)
  deriving DecidableEq, Repr

def admit (m? : Option Manifest) (d : Declaration) : Outcome :=
  match m? with
  | none => .unmanifested
  | some m =>
    match m.binding d.name with
    | none => .unmanifested
    | some e => if agrees e d then .resident e (m.residence e) else .withdrawn e

/-- Whether the unit ends up with a contracted callee for this declaration. A
withdrawal produces none — the weaker reading below it is the **uncontracted**
callee, which refuses. -/
def admits : Outcome → Bool
  | .unmanifested => true
  | .resident _ _ => true
  | .withdrawn _ => false

/-! ## The premise inventory: stacking -/

/-- The premises one contracted declaration contributes.

The `resident` arm is the card: **two** premises, not one replacing another. The
declared contract says what the code does; the artifact premise says which code
it is; neither is derivable from the other, so a certificate carries both. This
is the one deliberate asymmetry with level L1, where the sharpened waiver
*replaces* the bare one — there the manifest restates the very thing the bare
premise carried, and one record is the whole statement. -/
def premisesFor (d : Declaration) : Outcome → List Premise
  | .unmanifested => [.declaredContract d.name]
  | .resident e r => [.declaredContract d.name, .manifestResidentContract d.name e.contractModule r]
  | .withdrawn _ => []

/-- **The unmanifested contracted route is byte-unchanged.** -/
theorem an_unmanifested_contract_keeps_its_one_premise (d : Declaration) :
    premisesFor d .unmanifested = [.declaredContract d.name] := rfl

/-- **The premises stack.** A manifest-bound contracted call carries both. -/
theorem a_manifest_bound_call_stacks_both_premises (d : Declaration) (e : Entry)
    (r : Residence) :
    premisesFor d (.resident e r) =
      [.declaredContract d.name, .manifestResidentContract d.name e.contractModule r] := rfl

/-- **The contract premise is never replaced.** Whatever the manifest does, an
admitted contracted callee still publishes the premise that its behaviour is a
declaration — which is the sentence a reader of a `Safe` needs. -/
theorem the_contract_premise_is_never_replaced (d : Declaration) (o : Outcome)
    (admitted : admits o = true) :
    Premise.declaredContract d.name ∈ premisesFor d o := by
  cases o with
  | unmanifested => simp [premisesFor]
  | resident e r => simp [premisesFor]
  | withdrawn e => simp [admits] at admitted

/-- **There is no premise-free path through a manifest-bound contracted call.**
Every admitted outcome contributes at least one premise, and a resident one
contributes exactly two. -/
theorem there_is_no_premise_free_path (d : Declaration) (o : Outcome)
    (admitted : admits o = true) : premisesFor d o ≠ [] := by
  cases o with
  | unmanifested => simp [premisesFor]
  | resident e r => simp [premisesFor]
  | withdrawn e => simp [admits] at admitted

theorem a_resident_call_contributes_exactly_two_premises (d : Declaration) (e : Entry)
    (r : Residence) : (premisesFor d (.resident e r)).length = 2 := rfl

/-- The artifact premise names the contract module *and* the build, so a reader
of a certificate can tell which declaration was read against which artifact. -/
theorem the_artifact_premise_names_the_contract_and_the_build (d : Declaration)
    (m : Manifest) (e : Entry) :
    Premise.manifestResidentContract d.name e.contractModule (m.residence e) ∈
      premisesFor d (.resident e (m.residence e)) := by
  simp [premisesFor]

/-- A contracted premise waives nothing, so the manifest cannot make a contract
look like a waiver or the reverse. -/
theorem a_contracted_premise_waives_nothing (d : Declaration) (e : Entry) (r : Residence) :
    ∀ p ∈ premisesFor d (.resident e r), waives p = none := by
  intro p member
  simp only [premisesFor, List.mem_cons, List.not_mem_nil, or_false] at member
  rcases member with rfl | rfl <;> simp [waives]

/-- And the residence a contracted binding publishes always names its module,
where a contract-less one never does — the two levels are distinguishable by
inspection. -/
theorem a_contracted_residence_names_its_module (m : Manifest) (e : Entry) :
    (m.residence e).contract = some e.contractModule := rfl

theorem a_contract_less_residence_names_none (m : QCore.NativeLibrary.Manifest) :
    m.residence.contract = none := rfl

/-! ## Withdrawal, and why the direction differs from level L1 -/

/-- A manifest that contradicts the declared contract **withdraws** it. -/
theorem a_contradicted_binding_withdraws (m : Manifest) (d : Declaration) (e : Entry)
    (binding : m.binding d.name = some e) (disagrees : agrees e d = false) :
    admit (some m) d = .withdrawn e := by
  simp [admit, binding, disagrees]

/-- **The withdrawal admits nothing**, which is what makes it the refusing
direction: the reading below a withdrawn contract is the uncontracted callee,
and this profile has no contracted callee left to publish for it. -/
theorem a_withdrawal_admits_nothing (e : Entry) : admits (.withdrawn e) = false := rfl

theorem a_withdrawal_publishes_no_premise (d : Declaration) (e : Entry) :
    premisesFor d (.withdrawn e) = [] := rfl

/-- The unmanifested reading of a contracted declaration is always an
admission — which is what makes the next theorem a *monotonicity* claim rather
than a tautology about an empty set. -/
theorem the_unmanifested_reading_is_always_admitted (d : Declaration) :
    admits (admit none d) = true := rfl

/-- **Withdrawal is monotone in the refusing direction.** The only way a manifest
changes the outcome of a contracted declaration is by withdrawing it: whatever it
refuses, it refuses *by withdrawal*, and there is no manifest and no declaration
for which selecting the manifest admits something the unmanifested route did
not. -/
theorem a_manifest_can_only_withdraw (m? : Option Manifest) (d : Declaration)
    (refused : admits (admit m? d) = false) : ∃ e : Entry, admit m? d = .withdrawn e := by
  cases manifest : m? with
  | none => simp [manifest, admit, admits] at refused
  | some m =>
    cases binding : m.binding d.name with
    | none => simp [manifest, binding, admit, admits] at refused
    | some e =>
      by_cases agreement : agrees e d = true
      · simp [manifest, binding, agreement, admit, admits] at refused
      · exact ⟨e, by simp [binding, agreement, admit]⟩

/-- The same sentence over a whole environment, which is the way a reader checks
it: every name a selection admits was already admitted without it. -/
def admittedNames (m? : Option Manifest) (env : Environment) : List String :=
  (env.filter fun d => admits (admit m? d)).map Declaration.name

theorem withdrawal_only_removes_admissions (m? : Option Manifest) (env : Environment)
    (n : String) (member : n ∈ admittedNames m? env) : n ∈ admittedNames none env := by
  simp only [admittedNames, List.mem_map, List.mem_filter] at member ⊢
  obtain ⟨d, ⟨inEnv, _⟩, name⟩ := member
  exact ⟨d, ⟨inEnv, the_unmanifested_reading_is_always_admitted d⟩, name⟩

/-- **The direction argument, made formal.** One rule decides both cards:
*withdraw only where the weaker admissible reading refuses.* At level L3 the
weaker reading below a withdrawn binding publishes nothing — it refuses. At
level L1 the weaker reading is the bare waiver, and it is an **admission**: the
same unmanifested call is admitted there. The two cards therefore land on
opposite dispositions from the same rule, and neither is a shortcut. -/
theorem the_weaker_reading_refuses_here_and_is_admitted_at_level_one
    (d : Declaration) (e : Entry) (identity : Identity) (ps : Premises) :
    premisesFor d (.withdrawn e) = [] ∧
      QCore.NativeLibrary.admit
          (some ⟨identity, ps, none, [⟨"other", [.scalar], .scalar⟩]⟩) "k" [.scalar] .scalar =
        .admitted ⟨{ name := "k", trust := .uncheckedLinkage,
                     parameters := [.data .scalar], resultAbi := .scalar,
                     acquires := false }, none⟩ := by
  refine ⟨rfl, ?_⟩
  simp [QCore.NativeLibrary.admit, QCore.UncheckedLinkage.unchecked,
        QCore.UncheckedLinkage.admitParamsFrom, isPointer, QCore.NativeLibrary.Manifest.binding]

/-! ## Binding totality, in both directions -/

/-- An entry whose symbol the environment does not declare makes the slice
non-total. -/
theorem an_unresolved_entry_is_not_total (identity : Identity) (ps : Premises) :
    let m : Manifest := ⟨identity, ps, none, [⟨"k_alloc", [.scalar], .ptrMut, "lib"⟩]⟩
    sliceTotal [] m = false := by
  simp [sliceTotal, entryResolves]

/-- An entry naming another module makes it non-total too: the reference is
compared exactly, never resolved by bare symbol name. -/
theorem an_entry_in_another_module_is_not_total (identity : Identity) (ps : Premises) :
    let m : Manifest := ⟨identity, ps, none, [⟨"k_alloc", [.scalar], .ptrMut, "lib"⟩]⟩
    sliceTotal [⟨"k_alloc", "elsewhere", [.scalar], .ptrMut⟩] m = false := by
  simp [sliceTotal, entryResolves]

/-- And the reverse direction: a contract declared in a module of the slice that
no entry binds. -/
theorem an_unbound_contract_in_the_slice_is_not_total (identity : Identity) (ps : Premises) :
    let m : Manifest := ⟨identity, ps, none, [⟨"k_alloc", [.scalar], .ptrMut, "lib"⟩]⟩
    sliceTotal [⟨"k_alloc", "lib", [.scalar], .ptrMut⟩,
                ⟨"k_free", "lib", [.ptrMut], .unit⟩] m = false := by
  simp [sliceTotal, entryResolves, Manifest.slice]

/-- A declaration **outside** the slice is not the manifest's business, so it is
not demanded. The gate is total on the slice, not on the environment. -/
theorem a_declaration_outside_the_slice_is_not_demanded (identity : Identity) (ps : Premises) :
    let m : Manifest := ⟨identity, ps, none, [⟨"k_alloc", [.scalar], .ptrMut, "lib"⟩]⟩
    sliceTotal [⟨"k_alloc", "lib", [.scalar], .ptrMut⟩,
                ⟨"k_free", "elsewhere", [.ptrMut], .unit⟩] m = true := by
  simp [sliceTotal, entryResolves, Manifest.slice]

/-- The gate is not vacuous: a slice that binds exactly its module's contracts
is total. -/
theorem a_matching_slice_is_total (identity : Identity) (ps : Premises) :
    let m : Manifest :=
      ⟨identity, ps, none, [⟨"k_alloc", [.scalar], .ptrMut, "lib"⟩,
                      ⟨"k_free", [.ptrMut], .unit, "lib"⟩]⟩
    sliceTotal [⟨"k_alloc", "lib", [.scalar], .ptrMut⟩,
                ⟨"k_free", "lib", [.ptrMut], .unit⟩] m = true := by
  simp [sliceTotal, entryResolves, Manifest.slice]

/-- **The waiver mirror needs no gate of its own.** Slice totality already
implies that a symbol a manifest binds to a contract is declared by the selected
environment — which is exactly the condition the existing `Q034` refusal reads.
The third face of the compare-where-both-are-visible rule is therefore a theorem
of the other two gates, not a fourth diagnostic
(`CONTRACT-SLICE-WAIVER-MIRROR`). -/
theorem a_manifest_contracted_symbol_is_declared_by_the_environment
    (env : Environment) (m : Manifest) (name : String) (e : Entry)
    (total : sliceTotal env m = true) (binding : m.binding name = some e) :
    ∃ d ∈ env, d.name = e.symbol ∧ d.module = e.contractModule := by
  have member : e ∈ m.entries := by
    have := List.find?_some binding
    exact List.mem_of_find?_eq_some binding
  have resolves : entryResolves env e = true := by
    simp only [sliceTotal, Bool.and_eq_true] at total
    exact List.all_eq_true.mp total.1 e member
  unfold entryResolves at resolves
  cases found : env.find? (fun d => d.name == e.symbol) with
  | none => simp [found] at resolves
  | some d =>
    simp only [found] at resolves
    refine ⟨d, List.mem_of_find?_eq_some found, ?_, ?_⟩
    · have := List.find?_some found
      simpa using this
    · simpa using resolves

/-! ## Closed allocator custody

Design §4.4's structural check, which was **vacuous** while every manifest entry
was contract-less — a waiver cannot acquire — and becomes real exactly here. -/

/-- What one bound contract does to custody, projected to the two facts the check
reads. -/
structure Shape where
  acquires : Bool
  discharges : Bool
  deriving DecidableEq, Repr

/-- A manifest's slice is closed when it cannot acquire, or when something in it
discharges. §4.4 says "for the same resource shape"; §C04 projects every resource
to one shape, so the shape match is vacuous in v1 and this compares the slice as
a whole. -/
def closedCustody (shape : String → Shape) (m : Manifest) : Bool :=
  !m.entries.any (fun e => (shape e.symbol).acquires) ||
    m.entries.any fun e => (shape e.symbol).discharges

/-- **The check is sound in the sense §4.4 asks for**: a slice it admits, and
that can establish custody at all, contains somewhere to discharge it. -/
theorem an_admitted_slice_discharges_what_it_can_acquire (shape : String → Shape)
    (m : Manifest) (closed : closedCustody shape m = true)
    (e : Entry) (member : e ∈ m.entries) (acquiring : (shape e.symbol).acquires = true) :
    ∃ d ∈ m.entries, (shape d.symbol).discharges = true := by
  simp only [closedCustody, Bool.or_eq_true, Bool.not_eq_true', List.any_eq_false,
    List.any_eq_true] at closed
  rcases closed with none | some
  · exact absurd acquiring (by simpa using none e member)
  · obtain ⟨d, member, discharges⟩ := some
    exact ⟨d, member, discharges⟩

/-- A slice that acquires and never discharges is refused: such a library could
only leak or lie. -/
theorem an_open_custody_slice_is_refused (identity : Identity) (ps : Premises) :
    let m : Manifest := ⟨identity, ps, none, [⟨"k_alloc", [.scalar], .ptrMut, "lib"⟩]⟩
    closedCustody (fun _ => ⟨true, false⟩) m = false := by
  simp [closedCustody]

/-- The check is not vacuous in either direction: a discharge beside the
acquisition closes it, and a slice that cannot acquire is closed anyway — which
is the sense in which the contract-less level satisfied it by construction. -/
theorem a_discharging_slice_is_closed (identity : Identity) (ps : Premises) :
    let m : Manifest := ⟨identity, ps, none, [⟨"k_alloc", [.scalar], .ptrMut, "lib"⟩,
                                        ⟨"k_free", [.ptrMut], .unit, "lib"⟩]⟩
    closedCustody (fun s => if s == "k_free" then ⟨false, true⟩ else ⟨true, false⟩) m = true := by
  simp [closedCustody]

theorem a_slice_that_cannot_acquire_is_closed (identity : Identity) (ps : Premises)
    (entries : List Entry) :
    closedCustody (fun _ => ⟨false, false⟩) ⟨identity, ps, none, entries⟩ = true := by
  simp [closedCustody]

/-! ## Non-vacuity -/

/-- **A contract slice entry may declare a pointer**, which is the level's whole
reason to exist: the contract-less entry gate of `QCore.NativeLibrary` refuses
the same spelling, and this route admits it and publishes both premises. -/
theorem a_pointer_symbol_binds_here_and_never_as_a_waiver
    (identity : Identity) (ps : Premises) :
    let e : Entry := ⟨"k_alloc", [.scalar], .ptrMut, "lib"⟩
    let m : Manifest := ⟨identity, ps, none, [e]⟩
    let d : Declaration := ⟨"k_alloc", "lib", [.scalar], .ptrMut⟩
    admit (some m) d = .resident e (m.residence e) ∧
      QCore.NativeLibrary.entryAdmissible ⟨"k_alloc", [.scalar], .ptrMut⟩ = false := by
  refine ⟨?_, ?_⟩
  · simp [admit, Manifest.binding, agrees]
  · simp [QCore.NativeLibrary.entryAdmissible, isPointer]

/-- A concrete agreeing binding, with the two premises it publishes. -/
theorem the_contract_slice_route_admits_and_stacks (identity : Identity) (ps : Premises) :
    let e : Entry := ⟨"k_alloc", [.scalar], .ptrMut, "lib"⟩
    let m : Manifest := ⟨identity, ps, none, [e]⟩
    let d : Declaration := ⟨"k_alloc", "lib", [.scalar], .ptrMut⟩
    premisesFor d (admit (some m) d) =
      [.declaredContract "k_alloc",
       .manifestResidentContract "k_alloc" "lib" ⟨identity, ps, some "lib", none⟩] := by
  simp [admit, Manifest.binding, agrees, premisesFor, Manifest.residence]

/-- A symbol no manifest binds keeps the single contract premise beside it. -/
theorem an_unbound_symbol_keeps_the_single_premise (identity : Identity) (ps : Premises) :
    let m : Manifest := ⟨identity, ps, none, [⟨"other", [.scalar], .ptrMut, "lib"⟩]⟩
    let d : Declaration := ⟨"k_alloc", "lib", [.scalar], .ptrMut⟩
    premisesFor d (admit (some m) d) = [.declaredContract "k_alloc"] := by
  simp [admit, Manifest.binding, premisesFor]

end QCore.NativeLibraryContract
