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

import QCore.ExternCall

/-! Card `UNSAFE-LINKAGE`: linking an external library that has **no contract
at all**, under an explicit recorded waiver.

This is a model of the parts card `UNSAFE-LINKAGE` added to
`crates/qcore-core/src/extern_call.rs`: the callee trust marker, the closed
unchecked admission table, the derived effect row of an uncontracted callee, and
the assumption inventory a certificate derives from a callee list. The source
parser, the driver's identity gates and the C backend are separate Rust
boundaries, recorded in `docs/reports/wave18-unsafe-linkage/README.md`.

The ABI vocabulary, the bridged parameter forms and the custody vector are
**imported** from `QCore.ExternCall` rather than restated, so there is one table
in this model and not two that could drift apart. `QCore.ExternCall.AbiType` is
the collapsed projection that model already uses (a scalar, the two flat pointer
spellings, the pointer-to-pointer projection, `unit`), so `signature` below
models the *shape* of the rendered declaration and not the Rust rendering byte
for byte; what the theorems are about is that the record exists, names the
callee, and cannot be dropped.

What is established here:

* **assumption-carrying totality** — no callee list containing an uncontracted
  callee derives an assumption inventory that omits it, and no uncontracted
  callee is ever published as a contracted one;
* **the default is unchanged** — a list with no uncontracted callee derives
  exactly the inventory it derived before, and carries no waiver premise;
* **the admitted surface is custody-free** — every pointer position and `unit`
  parameter refuses by its own named constructor, the scalar table is inhabited,
  and an admitted unchecked callee's derived custody vector is empty;
* **the derived row is maximal** — an uncontracted callee derives every effect
  this profile can name, and a narrower declared row refuses it.

What is **not** established, here or anywhere: anything whatever about what an
uncontracted callee does. That is the premise, and printing it is the point. -/

namespace QCore.UncheckedLinkage

open QCore.ExternCall (AbiType CallParam Custody Effects derivedCustody admits isPointer)

/-- How much is known about a callee. The two constructors are a *total* match
everywhere a callee is consulted, which is what makes the assumption inventory
below total: there is no third arm that could contribute no premise. -/
inductive Trust where
  /-- A resource-contract v1 declaration was loaded and admitted. -/
  | contracted
  /-- No contract exists. The caller declared an ABI signature and nothing
  else. -/
  | uncheckedLinkage
  deriving DecidableEq, Repr

/-- One admitted callee, in the fields this card's theorems are about. -/
structure Callee where
  name : String
  trust : Trust
  parameters : List CallParam
  resultAbi : AbiType
  /-- Whether the callee establishes fresh custody. False for every unchecked
  callee by construction of `unchecked`, which is proved below rather than
  assumed. -/
  acquires : Bool
  deriving DecidableEq, Repr

/-- Why an `unsafe extern fn` declaration is still not callable. Each
constructor names one excluded shape; there is no catch-all, so widening the
unchecked surface means deleting a refusal on purpose. -/
inductive Refusal where
  | pointerParameter (index : Nat) (t : AbiType)
  | pointerResult (t : AbiType)
  | unitParameter (index : Nat)
  deriving DecidableEq, Repr

/-- Admission of the declared parameter list, left to right. A pointer in any
position and any spelling refuses: no assumption makes a capability handed to
unknown code checkable, so there is no premise text to record for it and this
card's whole discipline is that premises are recorded. -/
def admitParamsFrom : Nat → List AbiType → Except Refusal (List CallParam)
  | _, [] => .ok []
  | i, t :: rest =>
    if t = .unit then .error (.unitParameter i)
    else if isPointer t then .error (.pointerParameter i t)
    else match admitParamsFrom (i + 1) rest with
      | .ok ps => .ok (.data t :: ps)
      | .error e => .error e

/-- The closed unchecked admission table: scalars by value in, a scalar or
`unit` out, and nothing else. Deterministic, exactly as contract admission is,
so a certificate consumer can repeat it. -/
def unchecked (name : String) (ps : List AbiType) (result : AbiType) :
    Except Refusal Callee :=
  match admitParamsFrom 0 ps with
  | .error e => .error e
  | .ok bridged =>
    if isPointer result then .error (.pointerResult result)
    else .ok { name := name, trust := .uncheckedLinkage, parameters := bridged,
               resultAbi := result, acquires := false }

/-- The rendered declared ABI of a callee: the second half of the waiver record,
so a premise names *this identity with this signature* rather than a bare name. -/
def abiName : AbiType → String
  | .scalar => "scalar"
  | .ptrMut => "ptr<mut>"
  | .ptrConst => "ptr<const>"
  | .ptrMutPtrMut => "ptr<mut, ptr<mut>>"
  | .unit => "void"

def paramAbi : CallParam → AbiType
  | .data t | .borrowed t => t
  | .consumed | .stored _ | .owner | .released | .reallocSubject => .ptrMut
  | .outSlot => .ptrMutPtrMut

def signature (k : Callee) : String :=
  "(" ++ String.intercalate ", " (k.parameters.map (fun p => abiName (paramAbi p)))
    ++ ") -> " ++ abiName k.resultAbi

/-- A premise a certificate carries. -/
inductive Assumption where
  /-- The callee's declared contract is trusted (`ContractTrust::Declared`). -/
  | declaredContract (name : String)
  /-- The callee has no contract at all: this identity, with this declared ABI,
  was linked and not checked. -/
  | uncheckedLinkage (name : String) (signature : String)
  deriving DecidableEq, Repr

/-- The premise one callee contributes. A total match on its trust, so every
callee contributes exactly one premise and no callee contributes none. -/
def assumptionFor (k : Callee) : Assumption :=
  match k.trust with
  | .contracted => .declaredContract k.name
  | .uncheckedLinkage => .uncheckedLinkage k.name (signature k)

/-- The inventory a certificate derives from its own callee list. -/
def assumptions (ks : List Callee) : List Assumption := ks.map assumptionFor

/-! ## Assumption-carrying totality

The sentence this card exists for: there is no path from a program containing an
uncontracted callee to an inventory that does not mention it. -/

/-- **The totality theorem.** Every uncontracted callee of a list appears, by
name and declared signature, in the inventory that list derives. -/
theorem assumption_totality (ks : List Callee) (k : Callee) (member : k ∈ ks)
    (waived : k.trust = .uncheckedLinkage) :
    Assumption.uncheckedLinkage k.name (signature k) ∈ assumptions ks := by
  have : assumptionFor k = Assumption.uncheckedLinkage k.name (signature k) := by
    simp [assumptionFor, waived]
  exact this ▸ List.mem_map_of_mem member

/-- Every callee contributes a premise: the inventory is exactly as long as the
callee list, so none can be silently skipped. -/
theorem every_callee_contributes_exactly_one_premise (ks : List Callee) :
    (assumptions ks).length = ks.length := by
  simp [assumptions]

/-- An uncontracted callee is never published as a contracted one. This is the
substitution that would look like evidence, so it is refused by construction of
the match rather than by a check. -/
theorem an_uncontracted_callee_is_never_published_as_contracted
    (k : Callee) (waived : k.trust = .uncheckedLinkage) (n : String) :
    assumptionFor k ≠ .declaredContract n := by
  simp [assumptionFor, waived]

/-- And the converse: a contracted callee never contributes a waiver premise, so
the inventory cannot overstate what was waived either. -/
theorem a_contracted_callee_contributes_no_waiver
    (k : Callee) (contracted : k.trust = .contracted) (n s : String) :
    assumptionFor k ≠ .uncheckedLinkage n s := by
  simp [assumptionFor, contracted]

/-- **The default is unchanged.** A callee list with no uncontracted member
derives exactly the declared-contract inventory it derived before this card. -/
theorem the_contracted_inventory_is_unchanged (ks : List Callee)
    (allContracted : ∀ k ∈ ks, k.trust = .contracted) :
    assumptions ks = ks.map (fun k => .declaredContract k.name) := by
  simp only [assumptions]
  apply List.map_congr_left
  intro k member
  simp [assumptionFor, allContracted k member]

/-- No waiver premise appears out of nowhere: a waiver in the inventory names a
callee of the list that really is uncontracted. -/
theorem a_waiver_premise_names_an_uncontracted_callee
    (ks : List Callee) (n s : String)
    (present : Assumption.uncheckedLinkage n s ∈ assumptions ks) :
    ∃ k ∈ ks, k.trust = .uncheckedLinkage ∧ k.name = n := by
  simp only [assumptions, List.mem_map] at present
  obtain ⟨k, member, eq⟩ := present
  refine ⟨k, member, ?_, ?_⟩ <;>
    · cases trust : k.trust with
      | contracted => simp [assumptionFor, trust] at eq
      | uncheckedLinkage => simp only [assumptionFor, trust] at eq
                            simp_all

/-- A list with no uncontracted callee carries no waiver premise at all. -/
theorem no_waiver_without_an_uncontracted_callee (ks : List Callee)
    (allContracted : ∀ k ∈ ks, k.trust = .contracted) (n s : String) :
    Assumption.uncheckedLinkage n s ∉ assumptions ks := by
  intro present
  obtain ⟨k, member, waived, _⟩ := a_waiver_premise_names_an_uncontracted_callee ks n s present
  simp [allContracted k member] at waived

/-! ## The admitted surface carries no custody -/

/-- Any pointer parameter refuses, in any position. -/
theorem a_pointer_parameter_refuses (name : String) (before : List AbiType)
    (t : AbiType) (rest : List AbiType) (result : AbiType)
    (pointer : isPointer t = true) (scalarPrefix : ∀ u ∈ before, isPointer u = false)
    (noUnitPrefix : ∀ u ∈ before, u ≠ .unit) :
    ∃ e, unchecked name (before ++ t :: rest) result = .error e := by
  suffices h : ∀ i, ∃ e, admitParamsFrom i (before ++ t :: rest) = .error e by
    obtain ⟨e, he⟩ := h 0
    exact ⟨e, by simp [unchecked, he]⟩
  intro i
  induction before generalizing i with
  | nil =>
    refine ⟨.pointerParameter i t, ?_⟩
    have : t ≠ .unit := by
      intro eq; rw [eq] at pointer; simp [isPointer] at pointer
    simp [admitParamsFrom, this, pointer]
  | cons u us ih =>
    have notUnit : u ≠ .unit := noUnitPrefix u (by simp)
    have notPointer : isPointer u = false := scalarPrefix u (by simp)
    obtain ⟨e, he⟩ := ih (fun v hv => scalarPrefix v (by simp [hv]))
      (fun v hv => noUnitPrefix v (by simp [hv])) (i + 1)
    exact ⟨e, by simp [admitParamsFrom, notUnit, notPointer, he]⟩

/-- Each pointer spelling refuses on its own, including the read-only one. -/
theorem a_flat_mutable_pointer_parameter_refuses (name : String) (result : AbiType) :
    unchecked name [.ptrMut] result = .error (.pointerParameter 0 .ptrMut) := by
  simp [unchecked, admitParamsFrom, isPointer]

/-- `ptr<const, u8>` is refused **deliberately**, not by oversight: nothing
bounds the read, nothing ends the loan at the return, `const` constrains the
declaration rather than the implementation, and nothing stops the callee
publishing the pointer. See `docs/design/UNSAFE_LINKAGE_V1.md` §3.1. -/
theorem a_read_only_pointer_parameter_refuses (name : String) (result : AbiType) :
    unchecked name [.ptrConst] result = .error (.pointerParameter 0 .ptrConst) := by
  simp [unchecked, admitParamsFrom, isPointer]

theorem a_pointer_to_pointer_parameter_refuses (name : String) (result : AbiType) :
    unchecked name [.ptrMutPtrMut] result = .error (.pointerParameter 0 .ptrMutPtrMut) := by
  simp [unchecked, admitParamsFrom, isPointer]

theorem a_unit_parameter_refuses (name : String) (result : AbiType) :
    unchecked name [.unit] result = .error (.unitParameter 0) := by
  simp [unchecked, admitParamsFrom]

theorem a_pointer_result_refuses (name : String) :
    unchecked name [.scalar] .ptrMut = .error (.pointerResult .ptrMut) := by
  simp [unchecked, admitParamsFrom, isPointer]

theorem a_read_only_pointer_result_refuses (name : String) :
    unchecked name [.scalar] .ptrConst = .error (.pointerResult .ptrConst) := by
  simp [unchecked, admitParamsFrom, isPointer]

/-- The refusals above are not vacuous: the scalar table admits. -/
theorem the_unchecked_table_is_inhabited (name : String) :
    unchecked name [.scalar, .scalar] .scalar =
      .ok { name := name, trust := .uncheckedLinkage,
            parameters := [.data .scalar, .data .scalar],
            resultAbi := .scalar, acquires := false } := by
  simp [unchecked, admitParamsFrom, isPointer]

theorem a_unit_result_is_admitted (name : String) :
    unchecked name [] .unit =
      .ok { name := name, trust := .uncheckedLinkage, parameters := [],
            resultAbi := .unit, acquires := false } := by
  simp [unchecked, admitParamsFrom, isPointer]

private theorem admitted_parameters_are_all_data :
    ∀ (i : Nat) (ps : List AbiType) (qs : List CallParam),
      admitParamsFrom i ps = .ok qs → derivedCustody qs = []
  | _, [], qs, eq => by simp [admitParamsFrom] at eq; subst eq; rfl
  | i, t :: rest, qs, eq => by
    unfold admitParamsFrom at eq
    by_cases isUnit : t = .unit
    · simp [isUnit] at eq
    · cases pointer : isPointer t with
      | true => simp [isUnit, pointer] at eq
      | false =>
        simp only [isUnit, pointer, ite_false, Bool.false_eq_true] at eq
        cases inner : admitParamsFrom (i + 1) rest with
        | error e => simp [inner] at eq
        | ok rs =>
          simp only [inner, Except.ok.injEq] at eq
          subst eq
          simp [derivedCustody, admitted_parameters_are_all_data (i + 1) rest rs inner]

/-- **Custody-freedom.** An admitted unchecked callee's parameters are all data,
so the custody vector the caller's passes consume is empty. The caller was
handed nothing to hold, which is the whole soundness argument for the v1
surface. -/
theorem an_admitted_unchecked_callee_carries_no_custody
    (name : String) (ps : List AbiType) (result : AbiType) (k : Callee)
    (admitted : unchecked name ps result = .ok k) :
    derivedCustody k.parameters = [] := by
  unfold unchecked at admitted
  cases bridged : admitParamsFrom 0 ps with
  | error e => simp [bridged] at admitted
  | ok qs =>
    simp only [bridged] at admitted
    cases pointer : isPointer result with
    | true => simp [pointer] at admitted
    | false =>
      simp only [pointer, ite_false, Bool.false_eq_true, Except.ok.injEq] at admitted
      subst admitted
      exact admitted_parameters_are_all_data 0 ps qs bridged

/-- An unchecked callee establishes no fresh custody either, so it takes no
acquisition adapter and binds no identity. -/
theorem an_admitted_unchecked_callee_acquires_nothing
    (name : String) (ps : List AbiType) (result : AbiType) (k : Callee)
    (admitted : unchecked name ps result = .ok k) : k.acquires = false := by
  unfold unchecked at admitted
  cases bridged : admitParamsFrom 0 ps with
  | error e => simp [bridged] at admitted
  | ok qs =>
    simp only [bridged] at admitted
    cases pointer : isPointer result with
    | true => simp [pointer] at admitted
    | false =>
      simp only [pointer, ite_false, Bool.false_eq_true, Except.ok.injEq] at admitted
      subst admitted; rfl

/-- And it really is marked as uncontracted, so `assumption_totality` applies to
everything this table admits. -/
theorem an_admitted_unchecked_callee_is_marked
    (name : String) (ps : List AbiType) (result : AbiType) (k : Callee)
    (admitted : unchecked name ps result = .ok k) : k.trust = .uncheckedLinkage := by
  unfold unchecked at admitted
  cases bridged : admitParamsFrom 0 ps with
  | error e => simp [bridged] at admitted
  | ok qs =>
    simp only [bridged] at admitted
    cases pointer : isPointer result with
    | true => simp [pointer] at admitted
    | false =>
      simp only [pointer, ite_false, Bool.false_eq_true, Except.ok.injEq] at admitted
      subst admitted; rfl

/-! ## The derived effect row is maximal -/

/-- Nothing is known about an uncontracted callee, so its row cannot honestly be
narrower than everything this profile can name. A contracted callee keeps its
existing row exactly. -/
def derivedEffects (k : Callee) : Effects :=
  match k.trust with
  | .uncheckedLinkage => { io := true, alloc := true, mutates := true }
  | .contracted =>
    { io := true, alloc := k.acquires,
      mutates := k.parameters.any QCore.ExternCall.paramMutates }

theorem an_unchecked_row_is_maximal (k : Callee) (waived : k.trust = .uncheckedLinkage) :
    derivedEffects k = { io := true, alloc := true, mutates := true } := by
  simp [derivedEffects, waived]

/-- The contracted row is byte-for-byte the one `QCore.ExternCall` derives, so
this card widened nothing for a callee that has a contract. -/
theorem a_contracted_row_is_the_existing_one (k : Callee)
    (contracted : k.trust = .contracted) :
    derivedEffects k =
      { io := true, alloc := k.acquires,
        mutates := k.parameters.any QCore.ExternCall.paramMutates } := by
  simp [derivedEffects, contracted]

/-- An entry that declares less than the whole row refuses an unchecked call, so
the waiver is visible in the entry's own signature and not only at the
declaration. All three proper sub-rows are shown, not one convenient one. -/
theorem a_narrower_declared_row_refuses_an_unchecked_call
    (k : Callee) (waived : k.trust = .uncheckedLinkage) :
    admits ⟨true, false, false⟩ (derivedEffects k) = false
      ∧ admits ⟨true, true, false⟩ (derivedEffects k) = false
      ∧ admits ⟨true, false, true⟩ (derivedEffects k) = false := by
  simp [admits, derivedEffects, waived]

theorem an_empty_declared_row_admits_no_unchecked_call (k : Callee)
    (waived : k.trust = .uncheckedLinkage) :
    admits ⟨false, false, false⟩ (derivedEffects k) = false := by
  simp [admits, derivedEffects, waived]

/-- The whole row does admit it, so the refusals above are not vacuous. -/
theorem the_whole_declared_row_admits_an_unchecked_call (k : Callee)
    (waived : k.trust = .uncheckedLinkage) :
    admits ⟨true, true, true⟩ (derivedEffects k) = true := by
  simp [admits, derivedEffects, waived]

/-! ## Premise agreement

The model maps equal callees to equal premises. Admission is a pure function;
that restriction does not independently prove Rust certificate replay. -/

theorem the_premise_is_a_function_of_the_callee (j k : Callee) (same : j = k) :
    assumptionFor j = assumptionFor k := by rw [same]

end QCore.UncheckedLinkage
