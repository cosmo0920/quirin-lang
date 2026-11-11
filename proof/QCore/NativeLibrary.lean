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

import QCore.UncheckedLinkage

/-! Card `UNSAFE-LINKAGE-ARTIFACT`: the `source.native-library.v1` manifest, in
the slice that lets a waiver say **which build** it was waived from.

This is a model of the parts this card added to
`crates/qcore-core/src/native_library.rs` and `extern_call.rs`: the manifest's
symbol binding, the contract-less entry gate, the declaration-versus-manifest
type gate, and the premise inventory a certificate derives once a manifest is in
the picture. The manifest's *concrete syntax* and its total-or-reject loader are
a separate Rust boundary, tested there; what is modelled here is what the
loaded manifest means.

The admission table, the callee record and the rendered signature are
**imported** from `QCore.UncheckedLinkage` rather than restated, exactly as that
module imports its ABI vocabulary from `QCore.ExternCall`. There is one table in
this model and not three that could drift apart — which is also how the central
claim below is even statable: *with the manifest in the model, there is still no
assumption-free path through an unsafe call*.

What is established here:

* **totality survives the manifest** — every uncontracted callee still
  contributes a premise naming it, whether or not a manifest is selected, and
  no assignment of residences to callees can drop one;
* **the manifest sharpens and only sharpens** — an unmanifested waiver keeps the
  bare record byte for byte, a manifested one names the artifact, a contracted
  callee names none, and the three arms are a total match;
* **the admitted surface does not move** — the same closed table decides with
  and without a manifest, so a manifest cannot buy a pointer;
* **disagreement refuses rather than withdrawing** — a symbol its manifest
  contradicts is refused, and a symbol its manifest names is never admitted
  under the weaker bare premise.

What is **not** established, here or anywhere: anything about what an
uncontracted callee does, or that the linker used the artifact the manifest
names. The identity is a stated premise; that is the whole claim. -/

namespace QCore.NativeLibrary

open QCore.ExternCall (AbiType isPointer)
open QCore.UncheckedLinkage (Callee Trust Refusal unchecked signature)

/-! ## The manifest -/

/-- The data model the artifact was built under. One constructor in v1, which is
what makes `LOADER-ABI-LP64` an explicit statement rather than a reader's
assumption. -/
inductive DataModel where
  | lp64
  deriving DecidableEq, Repr

/-- No unwinding crosses the boundary. Stated, never checked. -/
inductive Unwinding where
  | abort
  deriving DecidableEq, Repr

/-- Every allocation the library hands out is released through the library's own
entry points. A contract-less entry satisfies this vacuously: it cannot
acquire. -/
inductive AllocatorCustody where
  | closed
  deriving DecidableEq, Repr

/-- The thread posture. Currently vacuous — extern calls stay refused in
concurrent contexts — but stated, because the manifest is a declaration in which
it *can* be stated, which a bare waiver is not. -/
inductive ThreadPosture where
  | singleCaller
  deriving DecidableEq, Repr

/-- The identity of one **exported header**: the reference a manifest spells and
the digest of the text that reference resolves to (card
`NATIVE-LIBRARY-HEADER`).

Unlike `Identity` below, this one is *verified* rather than declared: the header
is read at check time and its text decides admission, so measuring it is both
possible and honest. Whether the linker used the artifact stays the builder's
act, and stays a pin. -/
structure HeaderIdentity where
  reference : String
  digest : String
  deriving DecidableEq, Repr

/-- The stated identity of one library artifact. Every field is declared text:
nothing here is probed, hashed or read from a filesystem. -/
structure Identity where
  library : String
  version : String
  artifact : String
  hash : String
  deriving DecidableEq, Repr

/-- The four ABI premise fields, all mandatory, none checked beyond the closed
value vocabulary. -/
structure Premises where
  dataModel : DataModel
  unwinding : Unwinding
  allocatorCustody : AllocatorCustody
  threadPosture : ThreadPosture
  deriving DecidableEq, Repr

/-- What a manifest-resident symbol inherits: the identity of the build it comes
from and the premises that build was produced under. -/
structure Residence where
  identity : Identity
  premises : Premises
  /-- The declared contract module a `contract fn` entry bound this symbol to
  (card `NATIVE-LIBRARY-CONTRACT-SLICE`, level L3), and `none` for the
  contract-less entry of this module. A contract-less residence therefore never
  names a module, which is what keeps a level-L1 premise exactly what it was. -/
  contract : Option String
  /-- The exported header this symbol's declaration was **checked against**
  (card `NATIVE-LIBRARY-HEADER`), and `none` for a manifest that names no
  header — which is every manifest of the two cards below this one.

  Its trust status differs from every other field here, and a reader of a
  premise must be able to tell: `identity` is *declared* and nothing opens it,
  while this reference names a text that was read and this digest was measured
  over it. -/
  header : Option HeaderIdentity
  deriving DecidableEq, Repr

/-- One contract-less entry: a symbol, its exported header declaration's ABI
projection, and **no** contract — therefore no modes and no effects. -/
structure Entry where
  symbol : String
  parameters : List AbiType
  result : AbiType
  deriving DecidableEq, Repr

/-- A loaded manifest. -/
structure Manifest where
  identity : Identity
  premises : Premises
  /-- The exported header this manifest binds, and its declared digest (card
  `NATIVE-LIBRARY-HEADER`). `none` is the default and is exactly the world of
  this module and of `QCore.NativeLibraryContract`. -/
  header : Option HeaderIdentity
  entries : List Entry
  deriving DecidableEq, Repr

/-- The residence every entry of a manifest inherits. -/
def Manifest.residence (m : Manifest) : Residence := ⟨m.identity, m.premises, none, m.header⟩

/-- Resolve one symbol against a manifest. -/
def Manifest.binding (m : Manifest) (symbol : String) : Option Entry :=
  m.entries.find? (fun e => e.symbol == symbol)

/-- The contract-less entry gate of the loader: an entry that declares a pointer
in either direction, or a `unit` parameter, is not a contract-less entry at all.
A contract-less entry has no modes in which to state what a pointer may be used
for, and an artifact identity says *which code*, not what it does with your
pointer. -/
def entryAdmissible (e : Entry) : Bool :=
  e.parameters.all (fun t => !isPointer t && t != AbiType.unit) && !isPointer e.result

/-- The type gate of design §4.2 at the contract-less setting: the waived
declaration's ABI projection must equal the manifest's exported header
declaration. -/
def agrees (e : Entry) (ps : List AbiType) (result : AbiType) : Bool :=
  e.parameters == ps && e.result == result

/-! ## Admission with a manifest in the picture -/

/-- One admitted callee together with the manifest it is resident in, if any. -/
structure Resident where
  callee : Callee
  residence : Option Residence
  deriving DecidableEq, Repr

/-- The outcome of admitting one `unsafe extern fn` declaration. Three
constructors and no catch-all: the surface refusal is the wave-18 one, unmoved;
the disagreement refusal is this card's; and admission carries the residence
when, and only when, a manifest named the symbol. -/
inductive Outcome where
  | admitted (r : Resident)
  | refusedSurface (e : Refusal)
  | refusedDisagreement (e : Entry)
  deriving DecidableEq, Repr

/-- Admission. The **closed unchecked table decides first and unchanged**, so a
manifest demonstrably cannot widen what may cross; only then is the manifest
consulted, and only to sharpen or to refuse. -/
def admit (m? : Option Manifest) (name : String) (ps : List AbiType)
    (result : AbiType) : Outcome :=
  match unchecked name ps result with
  | .error e => .refusedSurface e
  | .ok k =>
    match m? with
    | none => .admitted ⟨k, none⟩
    | some m =>
      match m.binding name with
      | none => .admitted ⟨k, none⟩
      | some entry =>
        if agrees entry ps result then .admitted ⟨k, some m.residence⟩
        else .refusedDisagreement entry

/-! ## The premise inventory -/

/-- A premise a certificate carries. The waived arm is split in two so that a
bare waiver stays a valid *weaker* statement rather than becoming a malformed
sharpened one, and so that a reader tells them apart by inspection. -/
inductive Premise where
  /-- The callee's declared contract is trusted. -/
  | declaredContract (name : String)
  /-- No contract, and **no build was named**: the wave-18 record, unchanged. -/
  | uncheckedLinkage (name signature : String)
  /-- No contract, from this build, under these stated ABI premises. -/
  | manifestUncheckedLinkage (name signature : String) (residence : Residence)
  /-- Card `NATIVE-LIBRARY-CONTRACT-SLICE`, level L3: the callee **has** a
  declared contract and a manifest binds its symbol to that contract and to a
  named build. It is carried *beside* `declaredContract`, never in place of it —
  the contract premise says what the code does, this one says which code it is,
  and neither is derivable from the other. Modelled in the same inductive as the
  three above so there is one premise vocabulary and not two that could
  drift. -/
  | manifestResidentContract (name contractModule : String) (residence : Residence)
  deriving DecidableEq, Repr

/-- Which callee a premise waives, if it waives one. -/
def waives : Premise → Option String
  | .declaredContract _ => none
  | .uncheckedLinkage n _ => some n
  | .manifestUncheckedLinkage n _ _ => some n
  -- A contracted callee waives nothing, manifest-bound or not.
  | .manifestResidentContract _ _ _ => none

/-- The premise one resident callee contributes: a **total match** on its trust
and, within the waived arm, on whether a manifest named it. Three arms, and no
fourth that contributes nothing. -/
def premiseFor (r : Resident) : Premise :=
  match r.callee.trust with
  | .contracted => .declaredContract r.callee.name
  | .uncheckedLinkage =>
    match r.residence with
    | none => .uncheckedLinkage r.callee.name (signature r.callee)
    | some d => .manifestUncheckedLinkage r.callee.name (signature r.callee) d

/-- The inventory a certificate derives from its own resident callee list. -/
def premises (rs : List Resident) : List Premise := rs.map premiseFor

/-! ## Totality survives the manifest

The sentence this card exists for, restated with the manifest in the model:
there is still no path from a program containing an uncontracted callee to an
inventory that does not mention it. -/

/-- **The totality theorem, with the manifest in the model.** Every uncontracted
resident of a list appears in the inventory that list derives, and the premise it
contributes names it — whether or not a manifest sharpened it. -/
theorem manifest_assumption_totality (rs : List Resident) (r : Resident)
    (member : r ∈ rs) (waived : r.callee.trust = .uncheckedLinkage) :
    premiseFor r ∈ premises rs ∧ waives (premiseFor r) = some r.callee.name := by
  refine ⟨List.mem_map_of_mem member, ?_⟩
  cases residence : r.residence <;> simp [premiseFor, waived, residence, waives]

/-- **There is no assumption-free path through an unsafe call.** A waived
callee's premise is never the contracted one, in either residence case, so no
manifest can launder a waiver into a contract. -/
theorem there_is_no_assumption_free_path (r : Resident)
    (waived : r.callee.trust = .uncheckedLinkage) (n : String) :
    premiseFor r ≠ .declaredContract n := by
  cases residence : r.residence <;> simp [premiseFor, waived, residence]

/-- Every resident contributes a premise, so none can be silently skipped. -/
theorem every_resident_contributes_exactly_one_premise (rs : List Resident) :
    (premises rs).length = rs.length := by
  simp [premises]

/-- **No assignment of residences to callees can drop a premise.** Quantified
over an arbitrary residence assignment — including one that names some callees
and not others — so this is not a statement about one selected manifest. -/
theorem the_manifest_never_removes_a_premise (assign : Callee → Option Residence)
    (ks : List Callee) (k : Callee) (member : k ∈ ks)
    (waived : k.trust = .uncheckedLinkage) :
    ∃ p ∈ premises (ks.map (fun k => ⟨k, assign k⟩)), waives p = some k.name := by
  refine ⟨premiseFor ⟨k, assign k⟩, ?_, ?_⟩
  · exact List.mem_map_of_mem (List.mem_map_of_mem member)
  · cases assign k with
    | none => simp [premiseFor, waived, waives]
    | some d => simp [premiseFor, waived, waives]

/-- No premise appears out of nowhere: a waiver in the inventory names a resident
of the list that really is uncontracted. -/
theorem a_premise_names_a_resident_of_the_list (rs : List Resident) (p : Premise)
    (n : String) (present : p ∈ premises rs) (waiver : waives p = some n) :
    ∃ r ∈ rs, r.callee.trust = .uncheckedLinkage ∧ r.callee.name = n := by
  simp only [premises, List.mem_map] at present
  obtain ⟨r, member, eq⟩ := present
  subst eq
  cases trust : r.callee.trust with
  | contracted => simp [premiseFor, trust, waives] at waiver
  | uncheckedLinkage =>
    refine ⟨r, member, trust, ?_⟩
    cases residence : r.residence with
    | none =>
      simp only [premiseFor, trust, residence, waives, Option.some.injEq] at waiver
      exact waiver
    | some d =>
      simp only [premiseFor, trust, residence, waives, Option.some.injEq] at waiver
      exact waiver

/-! ## The manifest sharpens, and only sharpens -/

/-- **The default is unchanged.** A waiver no manifest names contributes exactly
the record card `UNSAFE-LINKAGE` published — the absence of an artifact field is
itself part of the trust statement: no build was named. -/
theorem an_unmanifested_waiver_keeps_the_bare_premise (r : Resident)
    (waived : r.callee.trust = .uncheckedLinkage) (bare : r.residence = none) :
    premiseFor r = .uncheckedLinkage r.callee.name (signature r.callee) := by
  simp [premiseFor, waived, bare]

/-- A manifest-resident waiver names the build it was waived from, with the
identity and the ABI premises of that build's manifest. -/
theorem a_manifested_waiver_names_its_artifact (r : Resident) (d : Residence)
    (waived : r.callee.trust = .uncheckedLinkage) (resident : r.residence = some d) :
    premiseFor r = .manifestUncheckedLinkage r.callee.name (signature r.callee) d := by
  simp [premiseFor, waived, resident]

/-- The two waived records are never confusable: a bare premise is not a
sharpened one whatever residence is offered for comparison. -/
theorem a_bare_premise_is_never_a_sharpened_one (n s : String) (d : Residence) :
    Premise.uncheckedLinkage n s ≠ Premise.manifestUncheckedLinkage n s d := by
  simp

/-- **A contracted callee names no artifact**, even where a manifest would offer
one: the slice this card landed binds contract-less entries only, so a declared
contract can never acquire an artifact premise it was not measured against. -/
theorem a_contracted_callee_names_no_artifact (r : Resident)
    (contracted : r.callee.trust = .contracted) (n s : String) (d : Residence) :
    premiseFor r ≠ .manifestUncheckedLinkage n s d := by
  simp [premiseFor, contracted]

/-- And a contracted callee contributes no waiver of either kind, so the
inventory cannot overstate what was waived. -/
theorem a_contracted_callee_contributes_no_waiver (r : Resident)
    (contracted : r.callee.trust = .contracted) :
    waives (premiseFor r) = none := by
  simp [premiseFor, contracted, waives]

/-! ## The admitted surface does not move -/

/-- **A manifest cannot widen what may cross.** Whatever the closed table
refuses, admission refuses with the same named refusal, for every manifest —
including one that names the symbol and agrees about nothing. -/
theorem a_refused_surface_stays_refused_under_every_manifest
    (m? : Option Manifest) (name : String) (ps : List AbiType) (result : AbiType)
    (e : Refusal) (refused : unchecked name ps result = .error e) :
    admit m? name ps result = .refusedSurface e := by
  simp [admit, refused]

/-- In particular a pointer parameter refuses under every manifest, in every
spelling — `ptr<const, u8>` included, deliberately. An artifact hash says which
code, not what it does with your pointer. -/
theorem a_manifest_cannot_buy_a_pointer_parameter (m? : Option Manifest)
    (name : String) (result : AbiType) :
    admit m? name [.ptrConst] result = .refusedSurface (.pointerParameter 0 .ptrConst) := by
  simp [admit, unchecked, QCore.UncheckedLinkage.admitParamsFrom, isPointer]

theorem a_manifest_cannot_buy_a_pointer_result (m? : Option Manifest) (name : String) :
    admit m? name [.scalar] .ptrMut = .refusedSurface (.pointerResult .ptrMut) := by
  simp [admit, unchecked, QCore.UncheckedLinkage.admitParamsFrom, isPointer]

/-- Nor could a manifest have declared one: a contract-less entry carrying a
pointer is not an admissible entry at all. -/
theorem a_pointer_entry_is_not_an_admissible_contract_less_entry
    (symbol : String) (result : AbiType) :
    entryAdmissible ⟨symbol, [.ptrConst], result⟩ = false := by
  simp [entryAdmissible, isPointer]

theorem a_pointer_result_entry_is_not_admissible (symbol : String) :
    entryAdmissible ⟨symbol, [.scalar], .ptrMut⟩ = false := by
  simp [entryAdmissible, isPointer]

/-- The entry gate is not vacuous: a scalar entry is admissible. -/
theorem a_scalar_entry_is_admissible (symbol : String) :
    entryAdmissible ⟨symbol, [.scalar, .scalar], .scalar⟩ = true := by
  simp [entryAdmissible, isPointer]

/-- Anything admitted was admitted by the closed table, so every property that
table establishes — custody-freedom, acquiring nothing, being marked
uncontracted — carries over to a manifest-resident callee unchanged. -/
theorem an_admitted_resident_was_admitted_by_the_closed_table
    (m? : Option Manifest) (name : String) (ps : List AbiType) (result : AbiType)
    (r : Resident) (admitted : admit m? name ps result = .admitted r) :
    unchecked name ps result = .ok r.callee := by
  unfold admit at admitted
  cases table : unchecked name ps result with
  | error e => simp [table] at admitted
  | ok k =>
    simp only [table] at admitted
    cases manifest : m? with
    | none => simp [manifest] at admitted; simp [← admitted]
    | some m =>
      simp only [manifest] at admitted
      cases binding : m.binding name with
      | none => simp [binding] at admitted; simp [← admitted]
      | some entry =>
        simp only [binding] at admitted
        by_cases agreement : agrees entry ps result = true
        · simp [agreement] at admitted; simp [← admitted]
        · simp [agreement] at admitted

/-! ## Disagreement refuses, and never withdraws -/

/-- A manifest that contradicts the declaration refuses it. -/
theorem a_disagreeing_manifest_refuses (m : Manifest) (name : String)
    (ps : List AbiType) (result : AbiType) (k : Callee) (entry : Entry)
    (table : unchecked name ps result = .ok k)
    (binding : m.binding name = some entry)
    (disagrees : agrees entry ps result = false) :
    admit (some m) name ps result = .refusedDisagreement entry := by
  simp [admit, table, binding, disagrees]

/-- **Refusal, not withdrawal — the theorem that makes it a refusal.** A symbol
its manifest names is *never* admitted under the weaker bare premise. Falling
back to it would reward a contradiction with a premise that says less about the
artifact, so there is no such outcome to reach. -/
theorem a_named_symbol_is_never_admitted_bare (m : Manifest) (name : String)
    (ps : List AbiType) (result : AbiType) (entry : Entry) (r : Resident)
    (binding : m.binding name = some entry)
    (admitted : admit (some m) name ps result = .admitted r) :
    r.residence = some m.residence := by
  unfold admit at admitted
  cases table : unchecked name ps result with
  | error e => simp [table] at admitted
  | ok k =>
    simp only [table] at admitted
    simp only [binding] at admitted
    by_cases agreement : agrees entry ps result = true
    · simp [agreement] at admitted; simp [← admitted]
    · simp [agreement] at admitted

/-- Agreement is exactly declaration-equals-header, so the gate is a comparison
and not an approximation of one. -/
theorem agreement_is_declaration_equals_header (e : Entry) (ps : List AbiType)
    (result : AbiType) :
    agrees e ps result = true ↔ e.parameters = ps ∧ e.result = result := by
  simp [agrees]

/-! ## Premise agreement

Binding and admission are pure functions in this model. The theorem below
compares the premises of two admitted results; it does not prove that a Rust
certificate consumer faithfully repeats either computation. -/

/-- And the published premise is a function of the admission, so two
rederivations of the same declaration publish the same premise. -/
theorem the_published_premise_is_a_function_of_the_admission
    (m? : Option Manifest) (name : String) (ps : List AbiType) (result : AbiType)
    (r₁ r₂ : Resident)
    (first : admit m? name ps result = .admitted r₁)
    (second : admit m? name ps result = .admitted r₂) :
    premiseFor r₁ = premiseFor r₂ := by
  rw [first] at second
  simp at second
  rw [second]

/-! ## Non-vacuity

The refusals above are refusals of something reachable: the manifest route
admits, and what it admits carries the sharpened premise. -/

/-- A concrete manifest-resident admission, with the premise it publishes. -/
theorem the_manifest_route_admits_and_names_the_artifact
    (identity : Identity) (premises : Premises) :
    let m : Manifest :=
      ⟨identity, premises, none, [⟨"k", [.scalar], .scalar⟩]⟩
    admit (some m) "k" [.scalar] .scalar =
      .admitted ⟨{ name := "k", trust := .uncheckedLinkage,
                   parameters := [.data .scalar], resultAbi := .scalar,
                   acquires := false },
                 some ⟨identity, premises, none, none⟩⟩ := by
  simp [admit, unchecked, QCore.UncheckedLinkage.admitParamsFrom, isPointer,
        Manifest.binding, Manifest.residence, agrees]

/-- A symbol no manifest names is admitted too, and keeps the bare record. -/
theorem an_unnamed_symbol_is_admitted_bare
    (identity : Identity) (premises : Premises) :
    let m : Manifest :=
      ⟨identity, premises, none, [⟨"other", [.scalar], .scalar⟩]⟩
    admit (some m) "k" [.scalar] .scalar =
      .admitted ⟨{ name := "k", trust := .uncheckedLinkage,
                   parameters := [.data .scalar], resultAbi := .scalar,
                   acquires := false }, none⟩ := by
  simp [admit, unchecked, QCore.UncheckedLinkage.admitParamsFrom, isPointer,
        Manifest.binding]

end QCore.NativeLibrary
