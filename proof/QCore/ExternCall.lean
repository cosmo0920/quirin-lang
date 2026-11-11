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

/-! Card `source.extern-call.v1`: the bounded extern-call model, extended by
card `EXTERN-F1` with the `resource_out` and `resource_stored` call modes and by
card `EXTERN-MODES` with `resource_released` and
`resource_return_reallocated`.

This is a model of `crates/qcore-core/src/extern_call.rs`. It establishes four
things about the *model*: which resource-contract v1 shapes are callable, that
the derived summary is a function of the contract, that applying a derived
summary changes the ledger by exactly the declared delta, and that the
acquisition adapter has exactly two exits — for each of the three admitted
acquisition sources, an owned nullable return, a declared out slot and a
declared reallocation.

Two premises are explicit and are **not** discharged here or anywhere else in
this repository: that a callee's C implementation satisfies its declared
contract, and that the Rust extraction of a source module agrees with the
transcription below. A conclusion drawn through an extern call is valid only
relative to the declaration (`ContractTrust::Declared`, contract spec §C06).

A third premise is specific to `resource_stored`: a stored identity leaves the
caller's ledger permanently, and that its declared owner keeps and eventually
releases it is asserted by the declaration alone. The model below records the
departure; it proves nothing about the owner. -/
namespace QCore.ExternCall

/-- The C ABI projection of contract spec §C04.1, collapsed to the distinctions
this model uses: a scalar carries data, a flat pointer may carry custody, and
the pointer-to-pointer projection exists only for `resource_out`. -/
inductive AbiType where
  | scalar
  | ptrMut
  | ptrConst
  | ptrMutPtrMut
  | unit
  deriving DecidableEq, Repr

/-- Every resource-contract v1 parameter mode (§C04.2). Listing all seven is
what makes the exclusion theorems below exhaustive rather than partial. -/
inductive ParamMode where
  | borrowed
  | owned (nullable : Bool)
  | consumed (nullable : Bool)
  | released (nullable : Bool)
  | out (nullable : Bool)
  | stored (owner : Nat) (nullable : Bool)
  | removed (owner : Nat) (nullable : Bool)
  deriving DecidableEq, Repr

/-- Every resource-contract v1 result mode (§C04.3). -/
inductive ReturnMode where
  | noCustody
  | owned (nullable : Bool)
  | reallocated (parameter : Nat) (nullable : Bool)
  deriving DecidableEq, Repr

structure Contract where
  parameters : List (AbiType × ParamMode)
  result : AbiType
  resultMode : ReturnMode
  deriving DecidableEq, Repr

/-- The admitted v1 bridging of one parameter (design §3.2, extended by
`EXTERN-F1`). -/
inductive CallParam where
  | data (t : AbiType)
  | borrowed (t : AbiType)
  | consumed
  /-- `resource_stored(p, owner, nonnull)`: custody leaves the caller and is
  indexed beneath the parameter at `owner`. -/
  | stored (owner : Nat)
  /-- `resource_owned(p, nonnull)` in designated owner position: the caller
  supplies live custody and keeps the identity. -/
  | owner
  /-- `resource_out(p, nullable)`: the callee writes fresh custody through the
  slot, under the checked adapter. -/
  | outSlot
  /-- `resource_released(p, nonnull)` (card `EXTERN-MODES`): the declared
  contract discharges the obligation. The caller-side delta is the same as
  `consumed`; what differs is the claim, which contract spec §C04.2 warns can
  invalidate a safety conclusion when false. -/
  | released
  /-- The subject of `resource_return_reallocated(p, nullable)` (card
  `EXTERN-MODES`): a `resource_owned` parameter the *result* mode designates.
  It is preserved on the failure arm and replaced exactly once on the success
  arm. -/
  | reallocSubject
  deriving DecidableEq, Repr

/-- The admitted v1 bridging of the result (design §3.2). -/
inductive CallResult where
  | data (t : AbiType)
  | ownedNullable
  /-- `resource_return_reallocated(p, nullable)` (card `EXTERN-MODES`). -/
  | reallocatedNullable (parameter : Nat)
  deriving DecidableEq, Repr

/-- Whether the result mode is an acquisition source. -/
def resultAcquires : CallResult → Bool
  | .ownedNullable => true
  | .reallocatedNullable _ => true
  | .data _ => false

structure Callee where
  parameters : List CallParam
  result : CallResult
  resultAbi : AbiType
  /-- Whether the declaration carries the single admitted out slot. -/
  outSlot : Bool
  /-- Whether the declaration carries the single admitted reallocation subject. -/
  reallocSubject : Bool
  deriving DecidableEq, Repr

/-- The three admitted acquisition sources, and there are only three. Each is
exclusive with the others, so the adapter always binds exactly one identity. -/
def acquires (k : Callee) : Bool := resultAcquires k.result || k.outSlot

def isPointer : AbiType → Bool
  | .ptrMut | .ptrConst | .ptrMutPtrMut => true
  | .scalar | .unit => false

def isMutablePointer : AbiType → Bool
  | .ptrMut | .ptrMutPtrMut => true
  | .ptrConst | .scalar | .unit => false

/-- Whether some `resource_stored` parameter of this contract designates `i` as
its owner. Owner position is a property of the whole declaration, which is why
per-parameter admission below is indexed. -/
def isDesignatedOwner (ps : List (AbiType × ParamMode)) (i : Nat) : Bool :=
  ps.any fun p => match p.2 with
    | .stored o _ => o == i
    | _ => false

/-- The parameter the result mode designates as a reallocation subject, if any.
Like owner position, subject position is a property of the whole declaration —
here of its *result* — which is why per-parameter admission is indexed. -/
def subjectOf : ReturnMode → Option Nat
  | .reallocated p _ => some p
  | _ => none

/-- Admission of one parameter at index `i` of `ps`, given the subject the
result mode designates. Every excluded mode returns `none` by its own match
arm; there is no catch-all that could silently absorb a new mode. -/
def bridgeParamAt (ps : List (AbiType × ParamMode)) (subject : Option Nat) (i : Nat) :
    AbiType → ParamMode → Option CallParam
  | .unit, _ => none
  | t, .borrowed => if isPointer t then
      (if t = .ptrMutPtrMut then none else some (.borrowed t))
    else some (.data t)
  | t, .consumed nullable =>
      if !isPointer t then none
      else if nullable then none
      else if t = .ptrMut then some .consumed else none
  | t, .owned nullable =>
      if !isPointer t then none
      else if subject = some i then
        (if nullable then none
         else if t = .ptrMut then some .reallocSubject else none)
      else if !isDesignatedOwner ps i then none
      else if nullable then none
      else if t = .ptrMut then some .owner else none
  | t, .out nullable =>
      if !isPointer t then none
      else if !nullable then none
      else if t = .ptrMutPtrMut then some .outSlot else none
  | t, .stored owner nullable =>
      if !isPointer t then none
      else if nullable then none
      else if t ≠ .ptrMut then none
      else if owner = i then none
      else match ps[owner]? with
        | some (_, .owned _) => some (.stored owner)
        | _ => none
  | t, .released nullable =>
      if !isPointer t then none
      else if nullable then none
      else if t = .ptrMut then some .released else none
  | _, .removed _ _ => none

def bridgeParamsFrom (ps : List (AbiType × ParamMode)) (subject : Option Nat) :
    Nat → List (AbiType × ParamMode) → Option (List CallParam)
  | _, [] => some []
  | i, (t, m) :: rest =>
    match bridgeParamAt ps subject i t m, bridgeParamsFrom ps subject (i + 1) rest with
    | some p, some qs => some (p :: qs)
    | _, _ => none

/-- Result admission. The reallocation arm carries contract spec §C04.3's own
well-formedness rule — the subject must be a parameter of this declaration and
must be `Owned` — because both are properties of the whole contract rather than
of any one parameter. -/
def bridgeResult (ps : List (AbiType × ParamMode)) : AbiType → ReturnMode → Option CallResult
  | t, .noCustody => some (.data t)
  | _, .owned false => none
  | t, .owned true => if t = .ptrMut then some .ownedNullable else none
  | _, .reallocated _ false => none
  | t, .reallocated p true =>
      match ps[p]? with
      | some (_, .owned _) => if t = .ptrMut then some (.reallocatedNullable p) else none
      | _ => none

def outCount (ks : List CallParam) : Nat := (ks.filter (· == .outSlot)).length

def hasStored (ks : List CallParam) : Bool :=
  ks.any fun k => match k with
    | .stored _ => true
    | _ => false

def hasReallocSubject (ks : List CallParam) : Bool := ks.any (· == .reallocSubject)

/-- Admit one complete declaration, or refuse it. Beyond per-parameter
admission there are exactly two contract-level rules, both of which keep the
adapter binding exactly one identity: at most one acquisition source, and no
store beside one. -/
def bridge (c : Contract) : Option Callee :=
  match bridgeParamsFrom c.parameters (subjectOf c.resultMode) 0 c.parameters,
      bridgeResult c.parameters c.result c.resultMode with
  | some ps, some r =>
    if outCount ps > 1 then none
    else if outCount ps = 1 ∧ resultAcquires r = true then none
    else if hasStored ps ∧ (resultAcquires r = true ∨ outCount ps = 1) then none
    else some ⟨ps, r, c.result, outCount ps = 1, hasReallocSubject ps⟩
  | _, _ => none

/-! ## Refusal completeness for every non-admitted C04 mode. -/

/-! Card `EXTERN-MODES` retires `released_parameter_refuses` by name: the mode
is admitted. Its load-bearing halves survive as `nullable_released_parameter_refuses`
and `releasing_other_than_a_flat_mutable_pointer_refuses` below, and the mode's
admission is pinned non-vacuously by `the_release_table_is_inhabited`. -/

theorem nullable_released_parameter_refuses (ps : List (AbiType × ParamMode))
    (subject : Option Nat) (i : Nat) (t : AbiType) :
    bridgeParamAt ps subject i t (.released true) = none := by
  cases t <;> simp [bridgeParamAt, isPointer]

theorem releasing_other_than_a_flat_mutable_pointer_refuses
    (ps : List (AbiType × ParamMode)) (subject : Option Nat) (i : Nat) (t : AbiType) (n : Bool)
    (h : t ≠ .ptrMut) : bridgeParamAt ps subject i t (.released n) = none := by
  cases t <;> simp_all [bridgeParamAt, isPointer]

theorem removed_parameter_refuses (t : AbiType) (o : Nat) (n : Bool)
    (rest : List (AbiType × ParamMode)) (r : AbiType) (rm : ReturnMode) :
    bridge ⟨(t, .removed o n) :: rest, r, rm⟩ = none := by
  cases t <;> simp [bridge, bridgeParamsFrom, bridgeParamAt]

theorem nullable_consumed_parameter_refuses (t : AbiType) (rest : List (AbiType × ParamMode))
    (r : AbiType) (rm : ReturnMode) : bridge ⟨(t, .consumed true) :: rest, r, rm⟩ = none := by
  cases t <;> simp [bridge, bridgeParamsFrom, bridgeParamAt, isPointer]

theorem scalar_custody_refuses (rest : List (AbiType × ParamMode)) (r : AbiType)
    (rm : ReturnMode) : bridge ⟨(.scalar, .consumed false) :: rest, r, rm⟩ = none := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, isPointer]

theorem borrowing_a_pointer_to_pointer_refuses (rest : List (AbiType × ParamMode))
    (r : AbiType) (rm : ReturnMode) : bridge ⟨(.ptrMutPtrMut, .borrowed) :: rest, r, rm⟩ = none := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, isPointer]

theorem consuming_a_const_pointer_refuses (rest : List (AbiType × ParamMode))
    (r : AbiType) (rm : ReturnMode) :
    bridge ⟨(.ptrConst, .consumed false) :: rest, r, rm⟩ = none := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, isPointer]

theorem nonnull_owned_return_refuses (ps : List (AbiType × ParamMode)) (r : AbiType) :
    bridge ⟨ps, r, .owned false⟩ = none := by
  simp [bridge, bridgeResult]

/-! Card `EXTERN-MODES` retires `reallocated_return_refuses` by name: the mode
is admitted. Its load-bearing halves survive as the four refusals below, and
the mode's admission is pinned non-vacuously by
`the_reallocation_table_is_inhabited`. -/

theorem nonnull_reallocated_return_refuses (ps : List (AbiType × ParamMode)) (r : AbiType)
    (p : Nat) : bridge ⟨ps, r, .reallocated p false⟩ = none := by
  simp [bridge, bridgeResult]

/-- Contract spec §C04.3: the subject must be a parameter of this declaration. -/
theorem a_reallocation_subject_out_of_range_refuses (r : AbiType) (p : Nat) :
    bridge ⟨[], r, .reallocated p true⟩ = none := by
  simp [bridge, bridgeResult, bridgeParamsFrom]

/-- Contract spec §C04.3: the subject must be `Owned`. -/
theorem a_reallocation_subject_that_is_not_owned_refuses (r : AbiType) :
    bridge ⟨[(.ptrMut, .borrowed)], r, .reallocated 0 true⟩ = none := by
  simp [bridge, bridgeResult, bridgeParamsFrom, bridgeParamAt, isPointer]

theorem a_reallocated_result_on_other_than_a_flat_mutable_pointer_refuses
    (ps : List (AbiType × ParamMode)) (r : AbiType) (p : Nat) (h : r ≠ .ptrMut) :
    bridge ⟨ps, r, .reallocated p true⟩ = none := by
  cases hp : ps[p]? with
  | none => simp [bridge, bridgeResult, hp]
  | some entry =>
    obtain ⟨t, m⟩ := entry
    cases m <;> simp [bridge, bridgeResult, hp, h]

/-- A nullable subject supplies no live custody, so there is nothing for the
success arm to replace exactly once. -/
theorem a_nullable_reallocation_subject_refuses (ps : List (AbiType × ParamMode))
    (i : Nat) (t : AbiType) : bridgeParamAt ps (some i) i t (.owned true) = none := by
  cases t <;> simp [bridgeParamAt, isPointer]

theorem a_reallocation_subject_on_other_than_a_flat_mutable_pointer_refuses
    (ps : List (AbiType × ParamMode)) (i : Nat) (t : AbiType) (n : Bool) (h : t ≠ .ptrMut) :
    bridgeParamAt ps (some i) i t (.owned n) = none := by
  cases t <;> simp_all [bridgeParamAt, isPointer]

theorem owned_result_requires_a_mutable_pointer (ps : List (AbiType × ParamMode))
    (r : AbiType) (h : r ≠ .ptrMut) : bridge ⟨ps, r, .owned true⟩ = none := by
  simp [bridge, bridgeResult, h]

/-! ## `resource_owned` is admitted in designated owner position and nowhere
else. The card that admits `resource_stored` is forced to admit it there,
because contract spec §C04.2 requires a stored parameter's owner to be a
distinct `Owned` parameter; every other position keeps its refusal. -/

/-- Card `EXTERN-MODES` adds the second designated position — a reallocation
subject — without widening the first: an owned parameter that is neither a
declared subject nor a store owner still refuses. -/
theorem an_owned_parameter_outside_owner_position_refuses
    (ps : List (AbiType × ParamMode)) (subject : Option Nat) (i : Nat) (t : AbiType) (n : Bool)
    (hs : subject ≠ some i) (h : isDesignatedOwner ps i = false) :
    bridgeParamAt ps subject i t (.owned n) = none := by
  cases t <;> simp [bridgeParamAt, isPointer, h, hs]

theorem owned_parameter_alone_refuses (t : AbiType) (n : Bool) (r : AbiType)
    (rm : ReturnMode) (hs : subjectOf rm = none) : bridge ⟨[(t, .owned n)], r, rm⟩ = none := by
  cases t <;>
    simp [bridge, bridgeParamsFrom, bridgeParamAt, isPointer, isDesignatedOwner, hs]

theorem a_nullable_owner_refuses (ps : List (AbiType × ParamMode)) (subject : Option Nat)
    (i : Nat) (t : AbiType) : bridgeParamAt ps subject i t (.owned true) = none := by
  cases t <;> by_cases hs : subject = some i <;> simp [bridgeParamAt, isPointer, hs]

theorem an_owner_on_other_than_a_flat_mutable_pointer_refuses
    (ps : List (AbiType × ParamMode)) (subject : Option Nat) (i : Nat) (t : AbiType) (n : Bool)
    (h : t ≠ .ptrMut) : bridgeParamAt ps subject i t (.owned n) = none := by
  cases t <;> by_cases hs : subject = some i <;> simp_all [bridgeParamAt, isPointer]

/-! ## `resource_out` (card `EXTERN-F1`). -/

theorem a_nonnull_out_parameter_refuses (ps : List (AbiType × ParamMode))
    (subject : Option Nat) (i : Nat) (t : AbiType) :
    bridgeParamAt ps subject i t (.out false) = none := by
  cases t <;> simp [bridgeParamAt, isPointer]

theorem an_out_parameter_on_other_than_the_pointer_to_pointer_projection_refuses
    (ps : List (AbiType × ParamMode)) (subject : Option Nat) (i : Nat) (t : AbiType) (n : Bool)
    (h : t ≠ .ptrMutPtrMut) : bridgeParamAt ps subject i t (.out n) = none := by
  cases t <;> simp_all [bridgeParamAt, isPointer]

theorem two_out_slots_refuse (r : AbiType) (rm : ReturnMode) :
    bridge ⟨[(.ptrMutPtrMut, .out true), (.ptrMutPtrMut, .out true)], r, rm⟩ = none := by
  cases rm with
  | noCustody => simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount]
  | owned n =>
    cases n <;> cases r <;>
      simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount]
  | reallocated p n =>
    -- The subject must be `Owned`; both parameters here are out slots, so the
    -- lookup either misses or finds a mode the result admission refuses.
    match p, n with
    | 0, false | 1, false | 0, true | 1, true =>
      simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer]
    | (_ + 2), false | (_ + 2), true =>
      simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer]

theorem an_out_slot_beside_an_owned_return_refuses :
    bridge ⟨[(.ptrMutPtrMut, .out true)], .ptrMut, .owned true⟩ = none := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount,
    resultAcquires]

/-- Non-vacuity for the out mode. -/
theorem the_out_table_is_inhabited :
    bridge ⟨[(.ptrMutPtrMut, .out true), (.scalar, .borrowed)], .scalar, .noCustody⟩ =
      some ⟨[.outSlot, .data .scalar], .data .scalar, .scalar, true, false⟩ := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount, hasStored,
    resultAcquires, hasReallocSubject]

/-! ## `resource_stored` (card `EXTERN-F1`). -/

theorem a_nullable_stored_parameter_refuses (ps : List (AbiType × ParamMode))
    (subject : Option Nat) (i o : Nat) (t : AbiType) :
    bridgeParamAt ps subject i t (.stored o true) = none := by
  cases t <;> simp [bridgeParamAt, isPointer]

theorem a_stored_parameter_on_other_than_a_flat_mutable_pointer_refuses
    (ps : List (AbiType × ParamMode)) (subject : Option Nat) (i o : Nat) (t : AbiType) (n : Bool)
    (h : t ≠ .ptrMut) : bridgeParamAt ps subject i t (.stored o n) = none := by
  cases t <;> simp_all [bridgeParamAt, isPointer]

theorem a_stored_parameter_beneath_itself_refuses (ps : List (AbiType × ParamMode))
    (subject : Option Nat) (i : Nat) (t : AbiType) (n : Bool) :
    bridgeParamAt ps subject i t (.stored i n) = none := by
  cases t <;> cases n <;> simp [bridgeParamAt, isPointer]

theorem a_stored_parameter_beneath_a_non_owned_parameter_refuses :
    bridge ⟨[(.ptrMut, .borrowed), (.ptrMut, .stored 0 false)], .unit, .noCustody⟩ = none := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer]

theorem a_stored_parameter_beside_an_owned_return_refuses :
    bridge ⟨[(.ptrMut, .owned false), (.ptrMut, .stored 0 false)], .ptrMut, .owned true⟩ =
      none := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, isDesignatedOwner,
    outCount, hasStored, subjectOf, resultAcquires]

/-- Non-vacuity for the store mode, and the shape contract spec §C04.2 forces:
the owner is a distinct `resource_owned` parameter of the same contract. -/
theorem the_store_table_is_inhabited :
    bridge ⟨[(.ptrMut, .owned false), (.ptrMut, .stored 0 false)], .unit, .noCustody⟩ =
      some ⟨[.owner, .stored 0], .data .unit, .unit, false, false⟩ := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, isDesignatedOwner,
    outCount, hasStored, subjectOf, resultAcquires, hasReallocSubject]

/-- Non-vacuity, unchanged from the v1 card. -/
theorem the_admitted_table_is_inhabited :
    bridge ⟨[(.scalar, .borrowed), (.ptrMut, .consumed false)], .ptrMut, .owned true⟩ =
      some ⟨[.data .scalar, .consumed], .ownedNullable, .ptrMut, false, false⟩ := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount, hasStored,
    resultAcquires, hasReallocSubject]

/-! ## Derivation is a function of the contract (design §3.4). -/

theorem admission_is_deterministic {c : Contract} {k₁ k₂ : Callee}
    (h₁ : bridge c = some k₁) (h₂ : bridge c = some k₂) : k₁ = k₂ := by
  rw [h₁] at h₂; exact Option.some.inj h₂

/-- Ownership behavior of one resource-bearing argument. Data parameters carry
no custody, and an out slot carries no *caller* identity, so both are absent
from the derived vector. -/
inductive Custody where
  | borrowed
  | consumed
  | stored
  | owner
  /-- Card `EXTERN-MODES`: the declared contract discharges the obligation. -/
  | released
  /-- Card `EXTERN-MODES`: the identity is replaced on the success arm and
  preserved on the failure arm. -/
  | reallocated
  deriving DecidableEq, Repr

def derivedCustody : List CallParam → List Custody
  | [] => []
  | .data _ :: rest => derivedCustody rest
  | .outSlot :: rest => derivedCustody rest
  | .borrowed _ :: rest => .borrowed :: derivedCustody rest
  | .consumed :: rest => .consumed :: derivedCustody rest
  | .stored _ :: rest => .stored :: derivedCustody rest
  | .owner :: rest => .owner :: derivedCustody rest
  | .released :: rest => .released :: derivedCustody rest
  | .reallocSubject :: rest => .reallocated :: derivedCustody rest

theorem an_out_slot_carries_no_caller_custody (rest : List CallParam) :
    derivedCustody (.outSlot :: rest) = derivedCustody rest := rfl

theorem a_store_and_its_owner_both_carry_custody (rest : List CallParam) :
    derivedCustody (.owner :: .stored 0 :: rest) =
      .owner :: .stored :: derivedCustody rest := rfl

structure Effects where
  io : Bool
  alloc : Bool
  mutates : Bool
  deriving DecidableEq, Repr

def paramMutates : CallParam → Bool
  | .data t | .borrowed t => isMutablePointer t
  | .consumed | .stored _ | .owner | .outSlot | .released | .reallocSubject => true

def derivedEffects (k : Callee) : Effects :=
  { io := true
    alloc := acquires k
    mutates := k.parameters.any paramMutates }

theorem every_derived_row_contains_io (k : Callee) : (derivedEffects k).io = true := rfl

theorem alloc_is_derived_exactly_for_an_acquisition (k : Callee) :
    (derivedEffects k).alloc = true ↔ (resultAcquires k.result = true ∨ k.outSlot = true) := by
  simp [derivedEffects, acquires]

/-- Card `EXTERN-MODES`: a reallocation is an acquisition source, so it derives
`alloc` exactly as the other two do. -/
theorem alloc_is_derived_for_a_reallocation (k : Callee) (p : Nat)
    (h : k.result = .reallocatedNullable p) : (derivedEffects k).alloc = true := by
  simp [derivedEffects, acquires, resultAcquires, h]

theorem alloc_is_derived_for_an_out_slot (k : Callee) (h : k.outSlot = true) :
    (derivedEffects k).alloc = true := by
  simp [derivedEffects, acquires, h]

theorem mut_is_derived_exactly_for_a_mutating_parameter (k : Callee) :
    (derivedEffects k).mutates = true ↔ ∃ p ∈ k.parameters, paramMutates p = true := by
  simp [derivedEffects]

theorem a_const_loan_of_a_scalar_call_derives_no_mutation
    (k : Callee) (h : k.parameters = [.data .scalar, .borrowed .ptrConst]) :
    (derivedEffects k).mutates = false := by
  simp [derivedEffects, h, paramMutates, isMutablePointer]

/-- The caller's declared row must admit the derived one; a declared row is
never widened to fit. -/
def admits (declared derived : Effects) : Bool :=
  (!derived.io || declared.io) && (!derived.alloc || declared.alloc) &&
    (!derived.mutates || declared.mutates)

theorem an_empty_declared_row_admits_no_extern_call (k : Callee) :
    admits ⟨false, false, false⟩ (derivedEffects k) = false := by
  simp [admits, derivedEffects]

theorem a_row_without_alloc_refuses_an_acquiring_call (k : Callee)
    (h : k.result = .ownedNullable) :
    admits ⟨true, false, true⟩ (derivedEffects k) = false := by
  simp [admits, derivedEffects, acquires, resultAcquires, h]

theorem a_row_without_alloc_refuses_a_reallocating_call (k : Callee) (p : Nat)
    (h : k.result = .reallocatedNullable p) :
    admits ⟨true, false, true⟩ (derivedEffects k) = false := by
  simp [admits, derivedEffects, acquires, resultAcquires, h]

theorem a_row_without_alloc_refuses_an_out_writing_call (k : Callee)
    (h : k.outSlot = true) :
    admits ⟨true, false, true⟩ (derivedEffects k) = false := by
  simp [admits, derivedEffects, acquires, h]

/-! ## The call step and its custody delta. -/

/-- The caller's live identities. A list, so the theorems below can state the
delta exactly rather than only as a count. -/
abbrev Ledger := List Nat

def removeAll : Ledger → List Nat → Ledger
  | live, [] => live
  | live, x :: rest => removeAll (live.filter (· != x)) rest

/-- The two exits of a call. There is no third constructor, which is the whole
content of adapter totality below. -/
inductive Outcome where
  | success (live : Ledger) (bound : Option Nat)
  | errorExit (live : Ledger)
  deriving DecidableEq, Repr

/-- One extern call. `departing` are the identities the declaration removes
from the caller's ledger — those passed to `consumed` parameters and those
passed to `stored` parameters, which leave by exactly the same delta — `fresh`
is the identity an acquisition would bind through either source, and `succeeds`
is the fixture's literal outcome control. -/
def stepCall (live : Ledger) (k : Callee) (departing : List Nat) (fresh : Nat)
    (succeeds : Bool) : Outcome :=
  let remaining := removeAll live departing
  if acquires k then
    if succeeds then .success (fresh :: remaining) (some fresh)
    else .errorExit remaining
  else .success remaining none

theorem adapter_totality (live : Ledger) (k : Callee) (departing : List Nat) (fresh : Nat)
    (succeeds : Bool) :
    (∃ l b, stepCall live k departing fresh succeeds = .success l b) ∨
      (∃ l, stepCall live k departing fresh succeeds = .errorExit l) := by
  unfold stepCall
  by_cases h : acquires k = true
  · simp only [h, ite_true]
    cases succeeds with
    | false => exact Or.inr ⟨_, rfl⟩
    | true => exact Or.inl ⟨_, _, rfl⟩
  · simp only [h]
    exact Or.inl ⟨_, _, rfl⟩

theorem a_data_call_never_binds_an_identity (live : Ledger) (k : Callee)
    (departing : List Nat) (fresh : Nat) (succeeds : Bool) (t : AbiType)
    (hr : k.result = .data t) (ho : k.outSlot = false) :
    stepCall live k departing fresh succeeds = .success (removeAll live departing) none := by
  simp [stepCall, acquires, resultAcquires, hr, ho]

theorem a_data_call_never_takes_the_error_arm (live : Ledger) (k : Callee)
    (departing : List Nat) (fresh : Nat) (succeeds : Bool) (t : AbiType)
    (hr : k.result = .data t) (ho : k.outSlot = false) (l : Ledger) :
    stepCall live k departing fresh succeeds ≠ .errorExit l := by
  simp [stepCall, acquires, resultAcquires, hr, ho]

/-- A store binds nothing: `resource_stored` is admitted with no owned return,
so a storing call is a departure and never an acquisition. -/
theorem a_store_binds_no_identity (live : Ledger) (k : Callee) (departing : List Nat)
    (fresh : Nat) (succeeds : Bool) (t : AbiType) (hr : k.result = .data t)
    (ho : k.outSlot = false) (_hs : hasStored k.parameters = true) :
    stepCall live k departing fresh succeeds = .success (removeAll live departing) none := by
  simp [stepCall, acquires, resultAcquires, hr, ho]

theorem a_failed_acquisition_binds_nothing (live : Ledger) (k : Callee)
    (departing : List Nat) (fresh : Nat) (h : acquires k = true) :
    stepCall live k departing fresh false = .errorExit (removeAll live departing) := by
  simp [stepCall, h]

theorem a_successful_acquisition_binds_exactly_the_fresh_identity (live : Ledger)
    (k : Callee) (departing : List Nat) (fresh : Nat) (h : acquires k = true) :
    stepCall live k departing fresh true = .success (fresh :: removeAll live departing)
      (some fresh) := by
  simp [stepCall, h]

/-- An out write is an acquisition on the success arm and on no other: the
declared slot materializes a binding exactly under the success control, and the
failure arm binds nothing. -/
theorem an_out_write_is_an_acquisition_only_on_success (live : Ledger) (k : Callee)
    (departing : List Nat) (fresh : Nat) (h : k.outSlot = true) :
    stepCall live k departing fresh true = .success (fresh :: removeAll live departing)
      (some fresh) ∧
    stepCall live k departing fresh false = .errorExit (removeAll live departing) := by
  constructor <;> simp [stepCall, acquires, h]

/-- Conservation: whichever arm is taken, every identity the call did not
remove survives, and nothing else is removed. -/
theorem the_custody_delta_is_exactly_the_declared_one (live : Ledger) (k : Callee)
    (departing : List Nat) (fresh : Nat) (succeeds : Bool) (x : Nat)
    (kept : x ∉ departing) (present : x ∈ live) :
    x ∈ (match stepCall live k departing fresh succeeds with
          | .success l _ => l
          | .errorExit l => l) := by
  have base : ∀ (cs : List Nat) (ls : Ledger), x ∉ cs → x ∈ ls → x ∈ removeAll ls cs := by
    intro cs
    induction cs with
    | nil => intro ls _ h; simpa [removeAll] using h
    | cons c rest ih =>
      intro ls hx h
      have hne : x ≠ c := by
        intro hEq; exact hx (by simp [hEq])
      have hrest : x ∉ rest := fun hm => hx (by simp [hm])
      exact ih _ hrest (by simp [List.mem_filter, h, hne])
  have surviving := base departing live kept present
  unfold stepCall
  by_cases h : acquires k = true
  · simp only [h, ite_true]
    cases succeeds with
    | false => simpa using surviving
    | true => simp [List.mem_cons, surviving]
  · simp only [h]
    simpa using surviving

theorem a_departing_identity_does_not_survive (live : Ledger) (k : Callee)
    (departing : List Nat) (fresh : Nat) (succeeds : Bool) (x : Nat)
    (moved : x ∈ departing) (distinct : fresh ≠ x) :
    x ∉ (match stepCall live k departing fresh succeeds with
          | .success l _ => l
          | .errorExit l => l) := by
  have base : ∀ (cs : List Nat) (ls : Ledger), x ∈ cs → x ∉ removeAll ls cs := by
    intro cs
    induction cs with
    | nil => intro _ h; cases h
    | cons c rest ih =>
      intro ls hx
      by_cases hEq : x = c
      · subst hEq
        have gone : ∀ (l : Ledger), x ∉ l.filter (· != x) := by
          intro l hm
          have := (List.mem_filter.mp hm).2
          simp at this
        have step : ∀ (cs' : List Nat) (l : Ledger), x ∉ l → x ∉ removeAll l cs' := by
          intro cs'
          induction cs' with
          | nil => intro l h; simpa [removeAll] using h
          | cons c' rest' ih' =>
            intro l h hm
            exact ih' _ (fun hmem => h (List.mem_filter.mp hmem).1) hm
        exact step rest _ (gone ls)
      · have : x ∈ rest := by
          rcases List.mem_cons.mp hx with h | h
          · exact absurd h hEq
          · exact h
        exact ih _ this
  have gone := base departing live moved
  unfold stepCall
  by_cases h : acquires k = true
  · simp only [h, ite_true]
    cases succeeds with
    | false => simpa using gone
    | true => simp [List.mem_cons, gone, Ne.symm distinct]
  · simp only [h]
    simpa using gone

/-- A consumed identity does not survive: the v1 statement, now a corollary of
the shared departure rule. -/
theorem a_consumed_identity_does_not_survive (live : Ledger) (k : Callee)
    (consumed : List Nat) (fresh : Nat) (succeeds : Bool) (x : Nat)
    (moved : x ∈ consumed) (distinct : fresh ≠ x) :
    x ∉ (match stepCall live k consumed fresh succeeds with
          | .success l _ => l
          | .errorExit l => l) :=
  a_departing_identity_does_not_survive live k consumed fresh succeeds x moved distinct

/-- A stored identity does not survive in the caller's ledger. What becomes of
it beneath the declared owner is the premise this model records and does not
discharge. -/
theorem a_stored_identity_does_not_survive (live : Ledger) (k : Callee)
    (consumed stored : List Nat) (fresh : Nat) (succeeds : Bool) (x : Nat)
    (moved : x ∈ stored) (distinct : fresh ≠ x) :
    x ∉ (match stepCall live k (consumed ++ stored) fresh succeeds with
          | .success l _ => l
          | .errorExit l => l) :=
  a_departing_identity_does_not_survive live k (consumed ++ stored) fresh succeeds x
    (List.mem_append.mpr (Or.inr moved)) distinct

/-- The declared owner is a loan, so it survives the store that names it. -/
theorem the_owner_survives_a_store (live : Ledger) (k : Callee)
    (stored : List Nat) (fresh : Nat) (succeeds : Bool) (owner : Nat)
    (kept : owner ∉ stored) (present : owner ∈ live) :
    owner ∈ (match stepCall live k stored fresh succeeds with
              | .success l _ => l
              | .errorExit l => l) :=
  the_custody_delta_is_exactly_the_declared_one live k stored fresh succeeds owner kept present


/-! ## Card `EXTERN-ERR-REL`: the adapter's error-arm disposition discipline.

`EXTERN-F1` admitted a raise only where nothing else was live, which made
`resource_stored` unreachable from a single source module: a store needs a
child and a distinct live owner. The arm below is not a new custody rule. The
sequential lowering `qcore_core::cleanup_lowering` already releases every
identity live before a failing acquisition, in reverse acquisition order, on
that acquisition's cleanup edge; what the profile lacked was a *rendering* of
that edge, because an extern-acquired identity is released only by calling a
declared `resource_consumed` callee. The arm names those calls, and admission
requires them to be exactly that edge.

The narrowing is stated as such: `an_empty_arm_with_live_custody_refuses` keeps
the v1 refusal, and `an_abandoned_identity_refuses` generalises it — an
identity that reaches the raise still live is still refused. -/

/-- One disposition on an error arm: the callee it calls and the identity that
call discharges. -/
structure Disposition where
  callee : Callee
  released : Nat
  deriving DecidableEq, Repr

/-- The identities an arm releases, in the order it names them. -/
def armReleases (ds : List Disposition) : List Nat := ds.map (·.released)

/-- A disposition is a release and nothing else: it may not acquire (that would
open an obligation on a path taken because something already failed, with no
adapter of its own) and may not store (that would convert an obligation into a
declared premise instead of discharging it). -/
def isRelease (d : Disposition) : Bool :=
  !acquires d.callee && !hasStored d.callee.parameters

/-- Admission of one error arm over the identities live at the acquisition.
Equality with `live.reverse` is exactly the cleanup edge: the same identities,
each once, in reverse acquisition order. -/
def armAdmits (live : Ledger) (ds : List Disposition) : Bool :=
  ds.all isRelease && decide (armReleases ds = live.reverse)

/-- The adapter with its arm. `none` is a refusal; the success arm keeps every
identity the arm would have released, because the arm is a *path*. -/
def stepAdapter (live : Ledger) (k : Callee) (fresh : Nat) (succeeds : Bool)
    (ds : List Disposition) : Option Outcome :=
  if !acquires k then none
  else if !armAdmits live ds then none
  else if succeeds then some (.success (fresh :: live) (some fresh))
  else some (.errorExit [])

/-- Conservation: under an admitted arm, every identity live at the acquisition
is released by that arm. Nothing is abandoned. -/
theorem error_arm_conservation (live : Ledger) (ds : List Disposition)
    (h : armAdmits live ds = true) (x : Nat) (hx : x ∈ live) : x ∈ armReleases ds := by
  have hEq : armReleases ds = live.reverse :=
    of_decide_eq_true ((Bool.and_eq_true _ _).mp h).2
  rw [hEq]
  exact List.mem_reverse.mpr hx

/-- The other direction, which is the refusal: an identity the arm does not
release is refused rather than silently carried past the raise. -/
theorem an_abandoned_identity_refuses (live : Ledger) (ds : List Disposition)
    (x : Nat) (hx : x ∈ live) (hout : x ∉ armReleases ds) : armAdmits live ds = false := by
  cases h : armAdmits live ds with
  | false => rfl
  | true => exact absurd (error_arm_conservation live ds h x hx) hout

/-- Nothing survives the raise: an admitted arm leaves an empty residual, which
is the same statement as conservation read on the error exit. -/
theorem an_admitted_error_exit_is_empty (live : Ledger) (k : Callee) (fresh : Nat)
    (ds : List Disposition) (h : acquires k = true) (ha : armAdmits live ds = true) :
    stepAdapter live k fresh false ds = some (.errorExit []) := by
  simp [stepAdapter, h, ha]

/-- The v1 refusal, kept: an arm that disposes of nothing while something is
live still refuses. This is the theorem `EXTERN-F1` relied on, narrowed to
empty arms rather than deleted. -/
theorem an_empty_arm_with_live_custody_refuses (live : Ledger) (h : live ≠ []) :
    armAdmits live [] = false := by
  have hne : armReleases [] ≠ live.reverse := by
    intro hEq
    have h0 : live.reverse = [] := by simpa [armReleases] using hEq.symm
    exact h (by simpa using congrArg List.reverse h0)
  simp [armAdmits, hne]

/-- The v1 shape, kept admitted: an arm that disposes of nothing when nothing
else is live is exactly what `source.extern-call.v1` already accepted. -/
theorem an_empty_arm_with_nothing_live_is_admitted : armAdmits [] [] = true := by
  decide

/-- An acquiring disposition refuses, whatever the ledger. -/
theorem an_acquiring_disposition_refuses (live : Ledger) (d : Disposition)
    (rest : List Disposition) (h : acquires d.callee = true) :
    armAdmits live (d :: rest) = false := by
  simp [armAdmits, isRelease, h]

/-- A storing disposition refuses, whatever the ledger. -/
theorem a_storing_disposition_refuses (live : Ledger) (d : Disposition)
    (rest : List Disposition) (h : hasStored d.callee.parameters = true) :
    armAdmits live (d :: rest) = false := by
  simp [armAdmits, isRelease, h]

/-- An arm longer than the live set cannot equal its reverse, so a repeated
disposition — the double release — refuses on length alone. -/
theorem an_arm_of_the_wrong_length_refuses (live : Ledger) (ds : List Disposition)
    (h : ds.length ≠ live.length) : armAdmits live ds = false := by
  have hne : armReleases ds ≠ live.reverse := by
    intro hEq
    apply h
    have := congrArg List.length hEq
    simpa [armReleases] using this
  simp [armAdmits, hne]

/-- The double release, as a corollary: one live identity, two dispositions. -/
theorem a_repeated_disposition_refuses (x : Nat) (d : Disposition) :
    armAdmits [x] [d, d] = false :=
  an_arm_of_the_wrong_length_refuses [x] [d, d] (by simp)

/-- A disposition of an identity that is not live refuses: the released list
would contain a member the reversed ledger does not. -/
theorem a_disposition_of_a_departed_identity_refuses (live : Ledger)
    (ds : List Disposition) (x : Nat) (hin : x ∈ armReleases ds) (hout : x ∉ live) :
    armAdmits live ds = false := by
  have hne : armReleases ds ≠ live.reverse := by
    intro hEq
    exact hout (List.mem_reverse.mp (hEq ▸ hin))
  simp [armAdmits, hne]

/-- The order is the cleanup edge's, not the handler's choice: acquisition
order refuses whenever it differs from reverse acquisition order. -/
theorem an_arm_in_the_wrong_order_refuses (live : Ledger) (ds : List Disposition)
    (h : armReleases ds ≠ live.reverse) : armAdmits live ds = false := by
  simp [armAdmits, h]

/-- Concretely, over two live identities: acquisition order refuses and reverse
acquisition order is admitted, so the rule is about order and not about arms of
length two. -/
theorem two_dispositions_must_run_in_reverse_acquisition_order
    (a b : Disposition) (ha : a.released = 1) (hb : b.released = 2)
    (hra : isRelease a = true) (hrb : isRelease b = true) :
    armAdmits [1, 2] [a, b] = false ∧ armAdmits [1, 2] [b, a] = true := by
  constructor
  · apply an_arm_in_the_wrong_order_refuses
    simp [armReleases, ha, hb]
  · simp [armAdmits, armReleases, ha, hb, hra, hrb]

/-- The arm is a path, not a place: on the success arm the ledger keeps every
identity the arm would have released, which is exactly why a store two
statements later can still name its owner. -/
theorem the_success_arm_keeps_every_dispositioned_identity (live : Ledger) (k : Callee)
    (fresh : Nat) (ds : List Disposition) (h : acquires k = true)
    (ha : armAdmits live ds = true) (x : Nat) (hx : x ∈ live) :
    ∃ l b, stepAdapter live k fresh true ds = some (.success l b) ∧ x ∈ l := by
  refine ⟨fresh :: live, some fresh, ?_, ?_⟩
  · simp [stepAdapter, h, ha]
  · exact List.mem_cons_of_mem _ hx

/-- Adapter totality survives the arm: an admitted acquisition still has exactly
two exits, and a refused arm has none rather than a third. -/
theorem adapter_with_arm_totality (live : Ledger) (k : Callee) (fresh : Nat)
    (succeeds : Bool) (ds : List Disposition) (h : acquires k = true) :
    stepAdapter live k fresh succeeds ds = none ∨
      (∃ l b, stepAdapter live k fresh succeeds ds = some (.success l b)) ∨
      (∃ l, stepAdapter live k fresh succeeds ds = some (.errorExit l)) := by
  unfold stepAdapter
  cases ha : armAdmits live ds with
  | false => exact Or.inl (by simp [h])
  | true =>
    cases succeeds with
    | false => exact Or.inr (Or.inr ⟨[], by simp [h]⟩)
    | true => exact Or.inr (Or.inl ⟨fresh :: live, some fresh, by simp [h]⟩)

/-! ### STORE-REACH: the store is reachable, and non-vacuously so.

`EXTERN-F1` recorded `resource_stored` as admitted by the bridging table but
unreachable from a single source module. The witness below is the shape that
was missing: a second acquisition over one live identity, admitted because its
arm releases that identity, leaving two live identities for the store the
bridging table already accepted. -/

/-- The consuming callee an arm disposes through, in the model's vocabulary. -/
def releaseCallee : Callee := ⟨[.consumed], .data .unit, .unit, false, false⟩

/-- The storing callee contract spec §C04.2 forces: a distinct declared owner. -/
def attachCallee : Callee := ⟨[.owner, .stored 0], .data .unit, .unit, false, false⟩

theorem the_release_callee_is_a_disposition : isRelease ⟨releaseCallee, 1⟩ = true := by
  decide

theorem the_attach_callee_is_not_a_disposition : isRelease ⟨attachCallee, 1⟩ = false := by
  decide

/-- The second acquisition over one live identity is admitted exactly because
its arm releases that identity. -/
theorem a_second_acquisition_is_admitted_when_its_arm_releases_the_first :
    stepAdapter [1] ⟨[], .ownedNullable, .ptrMut, false, false⟩ 2 true [⟨releaseCallee, 1⟩] =
      some (.success [2, 1] (some 2)) := by
  decide

/-- And is refused without it, which is EXTERN-F1's measured boundary reproduced
rather than deleted. -/
theorem the_same_acquisition_with_an_empty_arm_refuses :
    stepAdapter [1] ⟨[], .ownedNullable, .ptrMut, false, false⟩ 2 true [] = none := by
  decide

/-- STORE-REACH, non-vacuity: the two live identities the previous theorem
produces are exactly the child and the distinct owner the admitted store
consumes, and the store's contract bridges to that callee. -/
theorem the_store_is_reachable_from_two_live_identities :
    bridge ⟨[(.ptrMut, .owned false), (.ptrMut, .stored 0 false)], .unit, .noCustody⟩ =
        some attachCallee ∧
      stepCall [2, 1] attachCallee [2] 0 true = .success [1] none := by
  constructor
  · simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, isDesignatedOwner,
      outCount, hasStored, attachCallee, subjectOf, resultAcquires, hasReallocSubject]
  · decide

/-! ## Card `EXTERN-MODES`: `resource_released` and `resource_return_reallocated`.

The two modes this card admits differ in what they let the model *say*, not in
how much it believes. A release has the same caller-side ledger delta as a
consumption, so nothing about accounting moves; what moves is that the model can
now distinguish the declaration that discharges an obligation from the one that
relocates it, which contract spec §C04.2 separates and warns about. A
reallocation is the one departing mode whose *failure* arm §C04.3 states, which
is exactly why its subject stays on the error arm's live list and must be
dispositioned there — the same abandonment rule, not a relaxation of it. -/

/-- Non-vacuity for the release mode. -/
theorem the_release_table_is_inhabited :
    bridge ⟨[(.ptrMut, .released false)], .unit, .noCustody⟩ =
      some ⟨[.released], .data .unit, .unit, false, false⟩ := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount, hasStored,
    resultAcquires, hasReallocSubject]

/-- Non-vacuity for the reallocation mode, in the shape contract spec §C04.3
forces: the subject is a `resource_owned` parameter of the same contract. -/
theorem the_reallocation_table_is_inhabited :
    bridge ⟨[(.ptrMut, .owned false), (.scalar, .borrowed)], .ptrMut, .reallocated 0 true⟩ =
      some ⟨[.reallocSubject, .data .scalar], .reallocatedNullable 0, .ptrMut, false, true⟩ := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount, hasStored,
    resultAcquires, hasReallocSubject, subjectOf]

/-- A reallocation is an acquisition source, so it is exclusive with the other
two: the adapter still binds exactly one identity. -/
theorem a_reallocation_beside_an_out_slot_refuses :
    bridge ⟨[(.ptrMut, .owned false), (.ptrMutPtrMut, .out true)], .ptrMut, .reallocated 0 true⟩ =
      none := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount,
    resultAcquires, subjectOf]

theorem a_reallocating_callee_acquires (k : Callee) (p : Nat)
    (h : k.result = .reallocatedNullable p) : acquires k = true := by
  simp [acquires, resultAcquires, h]

theorem a_release_carries_its_own_custody (rest : List CallParam) :
    derivedCustody (.released :: rest) = .released :: derivedCustody rest := rfl

theorem a_reallocation_subject_carries_its_own_custody (rest : List CallParam) :
    derivedCustody (.reallocSubject :: rest) = .reallocated :: derivedCustody rest := rfl

/-- A declared release is an admitted error-arm disposition. This is what the
arm was always describing: `cleanup_lowering` releases, and before this card the
profile could spell that only as a transfer. -/
def freeCallee : Callee := ⟨[.released], .data .unit, .unit, false, false⟩

theorem the_free_callee_is_a_disposition : isRelease ⟨freeCallee, 1⟩ = true := by
  decide

theorem an_arm_rendered_by_a_declared_release_is_admitted :
    armAdmits [1] [⟨freeCallee, 1⟩] = true := by
  decide

/-- The adapter of a declared reallocation. Contract spec §C04.3 fixes both
arms: a null failure preserves the incoming custody, so the subject is on the
arm's live list, and a successful replacement consumes the old resource exactly
once, so it is removed only there. -/
def stepRealloc (live : Ledger) (subject fresh : Nat) (succeeds : Bool)
    (ds : List Disposition) : Option Outcome :=
  if !armAdmits live ds then none
  else if succeeds then some (.success (fresh :: removeAll live [subject]) (some fresh))
  else some (.errorExit [])

theorem reallocation_adapter_totality (live : Ledger) (subject fresh : Nat) (succeeds : Bool)
    (ds : List Disposition) :
    stepRealloc live subject fresh succeeds ds = none ∨
      (∃ l b, stepRealloc live subject fresh succeeds ds = some (.success l b)) ∨
      (∃ l, stepRealloc live subject fresh succeeds ds = some (.errorExit l)) := by
  unfold stepRealloc
  by_cases h : armAdmits live ds = true
  · simp only [h, Bool.not_true, Bool.false_eq_true, ite_false]
    cases succeeds with
    | false => exact Or.inr (Or.inr ⟨_, rfl⟩)
    | true => exact Or.inr (Or.inl ⟨_, _, rfl⟩)
  · simp [h]

theorem a_removed_identity_is_gone (live : Ledger) (x : Nat) : x ∉ removeAll live [x] := by
  simp [removeAll, List.mem_filter]

/-- The success arm: the subject is replaced exactly once. It is not in the
resulting ledger, the established identity is, and nothing else moved. -/
theorem a_successful_reallocation_replaces_its_subject (live : Ledger) (subject fresh : Nat)
    (ds : List Disposition) (h : armAdmits live ds = true) (hne : fresh ≠ subject) :
    ∃ l, stepRealloc live subject fresh true ds = some (.success l (some fresh)) ∧
      fresh ∈ l ∧ subject ∉ l := by
  refine ⟨fresh :: removeAll live [subject], by simp [stepRealloc, h], by simp, ?_⟩
  simp only [List.mem_cons, not_or]
  exact ⟨fun hEq => hne hEq.symm, a_removed_identity_is_gone live subject⟩

/-- The failure arm: nothing is bound, and the residual is empty — which is only
possible because the arm released the preserved subject too. -/
theorem a_failed_reallocation_binds_nothing (live : Ledger) (subject fresh : Nat)
    (ds : List Disposition) (h : armAdmits live ds = true) :
    stepRealloc live subject fresh false ds = some (.errorExit []) := by
  simp [stepRealloc, h]

/-- The half of §C04.3 that makes a reallocation different from every other
departing mode, stated as a fact about the arm: a subject live at the attempt is
released by the admitted arm. -/
theorem a_preserved_subject_is_released_by_the_arm (live : Ledger) (ds : List Disposition)
    (subject : Nat) (h : armAdmits live ds = true) (hin : subject ∈ live) :
    subject ∈ armReleases ds :=
  error_arm_conservation live ds h subject hin

/-- And the refusing direction, which is the pin: an arm that ignores the
preserved subject is refused, so the checker cannot silently write a leak on the
path §C04.3 says preserved the custody. -/
theorem a_failed_reallocation_whose_arm_ignores_the_subject_refuses (live : Ledger)
    (subject fresh : Nat) (succeeds : Bool) (ds : List Disposition) (hin : subject ∈ live)
    (hout : subject ∉ armReleases ds) : stepRealloc live subject fresh succeeds ds = none := by
  simp [stepRealloc, an_abandoned_identity_refuses live ds subject hin hout]

/-- The asymmetry, in one statement. An ordinary departing identity is off the
ledger before the arm runs, so the arm neither may nor must dispose of it; a
reallocation subject is still on it. `stepCall`'s `departing` and `stepRealloc`'s
`subject` are therefore different positions, not two spellings of one. -/
theorem a_departing_identity_is_absent_where_a_subject_is_present (live : Ledger) (x : Nat)
    (hin : x ∈ live) : x ∉ removeAll live [x] ∧ x ∈ live :=
  ⟨a_removed_identity_is_gone live x, hin⟩

/-- Non-vacuity for the whole reallocation round trip: one live identity, a
successful replacement, and a release of the result. -/
theorem a_reallocation_round_trip_is_admitted :
    stepRealloc [1] 1 2 true [⟨freeCallee, 1⟩] = some (.success [2] (some 2)) := by
  decide

/-- And its failure arm, which releases the subject the declaration preserved. -/
theorem a_failed_reallocation_round_trip_is_admitted :
    stepRealloc [1] 1 2 false [⟨freeCallee, 1⟩] = some (.errorExit []) := by
  decide

/-- The same failure arm with nothing disposed refuses, so the admission above
is not an artefact of an arm that never had to do anything. -/
theorem a_failed_reallocation_round_trip_with_an_empty_arm_refuses :
    stepRealloc [1] 1 2 false [] = none := by
  decide

/-! ## Card `PIPE-SCALAR-RESULT`: binding a non-acquiring call's scalar result.

The widening this card lands is one predicate, [`bindsScalar`], and one new
namespace. A call whose declared result is a **custody-free scalar** may bind
that result to a local, and a bound scalar may be passed where a declaration
writes a scalar parameter. Neither form touches the ledger.

The property to establish is therefore not a custody rule but its absence, and
it has two halves. *Refusal completeness*: every acquisition source, `unit` and
every pointer keep the refusal they had, so the mandatory adapter is still the
one spelling of an acquisition. *Non-interference*: a scalar binding leaves the
ledger exactly as the discarding call would, adds nothing to it, and can never
name an identity — which is the sentence "a scalar binding must not launder a
resource identity", proved rather than asserted.

Scalar names and identities are both `Nat` in this model and are kept in
separate fields; the collision refusal in [`stepBind`] is what makes that
separation a *statement about source*, since a name the reader could write into
either position is refused before it can mean two things. -/

/-- Whether a call to this callee may bind its result to a scalar local.

Deliberately narrow, and each clause is load-bearing: `acquires` excludes all
three acquisition sources, `unit` has no value to bind, and a pointer result
outside `resource_return_owned`/`resource_return_reallocated` is an address the
declaration says nothing about. -/
def bindsScalar (k : Callee) : Bool :=
  !acquires k &&
    (match k.result with
     | .data t => !isPointer t && t != .unit
     | .ownedNullable => false
     | .reallocatedNullable _ => false)

/-- Non-vacuity: the scalar-binding table is inhabited, and by exactly the shape
`qcore_extern_sum` declares — scalar and pointer loans in, a scalar out. -/
theorem the_scalar_binding_table_is_inhabited :
    bindsScalar ⟨[.borrowed .ptrConst, .data .scalar], .data .scalar, .scalar, false, false⟩
      = true := by
  decide

/-- And it is reached through the admission table rather than only postulated:
the contract that bridges to that callee binds a scalar. -/
theorem a_bridged_scalar_returning_contract_binds_its_result :
    (bridge ⟨[(.ptrConst, .borrowed), (.scalar, .borrowed)], .scalar, .noCustody⟩).map
        bindsScalar = some true := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount, hasStored,
    resultAcquires, hasReallocSubject, bindsScalar, acquires]

/-! ### Refusal completeness: nothing an acquisition or a non-scalar can bind -/

/-- An owned nullable return binds an identity, not a scalar. -/
theorem an_owned_return_binds_no_scalar (k : Callee) (h : k.result = .ownedNullable) :
    bindsScalar k = false := by
  simp [bindsScalar, h]

/-- A declared reallocation binds an identity, not a scalar. -/
theorem a_reallocating_return_binds_no_scalar (k : Callee) (p : Nat)
    (h : k.result = .reallocatedNullable p) : bindsScalar k = false := by
  simp [bindsScalar, h]

/-- A declared out slot binds an identity, not a scalar — even though its
*result* is a scalar status code, which is exactly the case a rule written over
the result alone would have got wrong. -/
theorem an_out_writing_call_binds_no_scalar (k : Callee) (h : k.outSlot = true) :
    bindsScalar k = false := by
  simp [bindsScalar, acquires, h]

/-- The general statement of the three above: no acquisition binds a scalar, so
the mandatory adapter remains the one spelling of an acquisition. -/
theorem an_acquiring_callee_binds_no_scalar (k : Callee) (h : acquires k = true) :
    bindsScalar k = false := by
  simp [bindsScalar, h]

/-- A `unit` result binds nothing: there is no value. -/
theorem a_unit_result_binds_no_scalar (k : Callee) (h : k.result = .data .unit) :
    bindsScalar k = false := by
  simp [bindsScalar, h, isPointer]

/-- A pointer result declared with no custody is not bindable either. Refusing
it is what keeps an unmodelled address out of the value namespace. -/
theorem a_pointer_result_binds_no_scalar (k : Callee) (t : AbiType)
    (hr : k.result = .data t) (hp : isPointer t = true) : bindsScalar k = false := by
  simp [bindsScalar, hr, hp]

/-! ### Non-interference: the value namespace and the ledger do not meet -/

/-- The state of a body under this profile: the linear ledger, and the scalar
names bound so far. Two fields, never one — every theorem below is a statement
about the fact that no operation moves a name between them. -/
structure Env where
  live : Ledger
  scalars : List Nat
  deriving DecidableEq, Repr

/-- Binding a non-acquiring call's scalar result.

`departing` are the identities the same call's declared parameter modes remove,
exactly as in `stepCall`: a scalar-result callee may still consume, store or
release, because nothing in the bridging table couples the result mode to the
parameter modes. What the binding itself contributes is one name in `scalars`
and nothing at all in `live`.

`none` is a refusal: the callee does not bind a scalar, or the name is already a
live identity — the cross-namespace shadow, refused rather than resolved by
lookup order. -/
def stepBind (e : Env) (k : Callee) (departing : List Nat) (n : Nat) : Option Env :=
  if !bindsScalar k then none
  else if e.live.contains n then none
  else some ⟨removeAll e.live departing, n :: e.scalars⟩

/-- A scalar binding that carries no departing argument leaves the ledger
**byte for byte** as it found it. This is the card's central claim. -/
theorem a_scalar_binding_leaves_the_ledger_unchanged (e : Env) (k : Callee) (n : Nat)
    (h : bindsScalar k = true) (hn : n ∉ e.live) :
    (stepBind e k [] n).map (·.live) = some e.live := by
  simp [stepBind, h, hn, removeAll]

/-- And with departing arguments it changes the ledger by **exactly** what the
discarding call would: `stepBind`'s ledger is `stepCall`'s, so the binding
itself contributes no custody delta at all. -/
theorem a_scalar_binding_has_the_discarding_calls_ledger (e : Env) (k : Callee)
    (departing : List Nat) (n : Nat) (h : bindsScalar k = true)
    (hn : n ∉ e.live) (fresh : Nat) (succeeds : Bool) :
    (stepBind e k departing n).map (·.live)
      = some (match stepCall e.live k departing fresh succeeds with
              | .success l _ => l
              | .errorExit l => l) := by
  have hacq : acquires k = false := by
    by_cases ha : acquires k = true
    · simp [an_acquiring_callee_binds_no_scalar k ha] at h
    · simpa using ha
  simp [stepBind, stepCall, h, hn, hacq]

/-- Nothing enters the ledger: every identity live after a scalar binding was
live before it. There is no arm on which the bound name becomes an identity. -/
theorem a_scalar_binding_introduces_no_identity (e : Env) (k : Callee)
    (departing : List Nat) (n : Nat) (e' : Env) (h : stepBind e k departing n = some e')
    (x : Nat) (hx : x ∈ e'.live) : x ∈ e.live := by
  have base : ∀ (cs : List Nat) (ls : Ledger), x ∈ removeAll ls cs → x ∈ ls := by
    intro cs
    induction cs with
    | nil => intro ls h; simpa [removeAll] using h
    | cons c rest ih =>
      intro ls hm
      exact (List.mem_filter.mp (ih _ hm)).1
  by_cases hb : bindsScalar k = true
  · by_cases hn : n ∈ e.live
    · simp [stepBind, hb, hn] at h
    · simp only [stepBind, hb, hn, Bool.not_true, Bool.false_eq_true, ite_false,
        Option.some.injEq, List.elem_iff] at h
      subst h
      exact base departing e.live hx
  · simp [stepBind, hb] at h

/-- The bound name is never an identity of the resulting ledger. Stated as the
laundering theorem it is: after `let n = f(x);` the name `n` denotes data, and
no later `move n` can find it in the ledger. -/
theorem a_scalar_binding_never_names_a_live_identity (e : Env) (k : Callee)
    (departing : List Nat) (n : Nat) (e' : Env) (h : stepBind e k departing n = some e') :
    n ∉ e'.live := by
  intro hin
  have hbefore := a_scalar_binding_introduces_no_identity e k departing n e' h n hin
  by_cases hb : bindsScalar k = true
  · by_cases hn : n ∈ e.live
    · simp [stepBind, hb, hn] at h
    · exact hn hbefore
  · simp [stepBind, hb] at h

/-- The cross-namespace shadow refuses: a scalar `let` whose name is already a
live identity has no admitted step. Fail-closed rather than resolved by lookup
order — the machine could tell the two apart, and a reader could not. -/
theorem a_scalar_binding_over_a_live_identity_refuses (e : Env) (k : Callee)
    (departing : List Nat) (n : Nat) (h : n ∈ e.live) :
    stepBind e k departing n = none := by
  by_cases hb : bindsScalar k = true <;> simp [stepBind, hb, h]

/-- An acquiring call has no scalar-binding step at all, so the widening cannot
be used as a second spelling of an acquisition. -/
theorem an_acquisition_has_no_scalar_binding_step (e : Env) (k : Callee)
    (departing : List Nat) (n : Nat) (h : acquires k = true) :
    stepBind e k departing n = none := by
  simp [stepBind, an_acquiring_callee_binds_no_scalar k h]

/-- A scalar actual reaches a `data` parameter, and `derivedCustody` drops every
`data` parameter — so passing a bound scalar contributes nothing to the derived
summary the caller's passes consume. The argument side of non-interference. -/
theorem a_scalar_actual_carries_no_custody (t : AbiType) (rest : List CallParam) :
    derivedCustody (.data t :: rest) = derivedCustody rest := rfl

/-- The refusal surface of the scalar binding, exhibited as one list so a future
widening has to change this theorem rather than slip past it: an owned return,
a reallocation, an out slot, a `unit` result and a pointer result. -/
theorem refused_scalar_bindings_are_exactly_these :
    bindsScalar ⟨[], .ownedNullable, .ptrMut, false, false⟩ = false ∧
    bindsScalar ⟨[.reallocSubject], .reallocatedNullable 0, .ptrMut, false, true⟩ = false ∧
    bindsScalar ⟨[.outSlot], .data .scalar, .scalar, true, false⟩ = false ∧
    bindsScalar ⟨[], .data .unit, .unit, false, false⟩ = false ∧
    bindsScalar ⟨[], .data .ptrMut, .ptrMut, false, false⟩ = false ∧
    bindsScalar ⟨[], .data .scalar, .scalar, false, false⟩ = true := by
  refine ⟨by decide, by decide, by decide, by decide, by decide, by decide⟩

/-- Non-vacuity for the whole round trip, in the shape the fixture pins: a
scalar binding over a live identity's ledger, then a second binding that reads
it, and the identity untouched throughout. -/
theorem a_scalar_chain_leaves_one_live_identity :
    (stepBind ⟨[1], []⟩ ⟨[.borrowed .ptrConst, .data .scalar], .data .scalar, .scalar,
        false, false⟩ [] 7).bind
      (fun e => stepBind e ⟨[.data .scalar, .data .scalar], .data .scalar, .scalar,
        false, false⟩ [] 8)
      = some ⟨[1], [8, 7]⟩ := by
  decide

end QCore.ExternCall
