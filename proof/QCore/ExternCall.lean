/-! Card `source.extern-call.v1`: the bounded extern-call model, extended by
card `EXTERN-F1` with the `resource_out` and `resource_stored` call modes.

This is a model of `crates/qcore-core/src/extern_call.rs`. It establishes four
things about the *model*: which resource-contract v1 shapes are callable, that
the derived summary is a function of the contract, that applying a derived
summary changes the ledger by exactly the declared delta, and that the
acquisition adapter has exactly two exits — for both admitted acquisition
sources, an owned nullable return and a declared out slot.

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
  deriving DecidableEq, Repr

/-- The admitted v1 bridging of the result (design §3.2). -/
inductive CallResult where
  | data (t : AbiType)
  | ownedNullable
  deriving DecidableEq, Repr

structure Callee where
  parameters : List CallParam
  result : CallResult
  resultAbi : AbiType
  /-- Whether the declaration carries the single admitted out slot. -/
  outSlot : Bool
  deriving DecidableEq, Repr

/-- The two admitted acquisition sources, and there are only two. -/
def acquires (k : Callee) : Bool := k.result == .ownedNullable || k.outSlot

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

/-- Admission of one parameter at index `i` of `ps`. Every excluded mode
returns `none` by its own match arm; there is no catch-all that could silently
absorb a new mode. -/
def bridgeParamAt (ps : List (AbiType × ParamMode)) (i : Nat) :
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
  | _, .released _ => none
  | _, .removed _ _ => none

def bridgeParamsFrom (ps : List (AbiType × ParamMode)) :
    Nat → List (AbiType × ParamMode) → Option (List CallParam)
  | _, [] => some []
  | i, (t, m) :: rest =>
    match bridgeParamAt ps i t m, bridgeParamsFrom ps (i + 1) rest with
    | some p, some qs => some (p :: qs)
    | _, _ => none

def bridgeResult : AbiType → ReturnMode → Option CallResult
  | t, .noCustody => some (.data t)
  | _, .owned false => none
  | t, .owned true => if t = .ptrMut then some .ownedNullable else none
  | _, .reallocated _ _ => none

def outCount (ks : List CallParam) : Nat := (ks.filter (· == .outSlot)).length

def hasStored (ks : List CallParam) : Bool :=
  ks.any fun k => match k with
    | .stored _ => true
    | _ => false

/-- Admit one complete declaration, or refuse it. Beyond per-parameter
admission there are exactly two contract-level rules, both of which keep the
adapter binding exactly one identity: at most one acquisition source, and no
store beside one. -/
def bridge (c : Contract) : Option Callee :=
  match bridgeParamsFrom c.parameters 0 c.parameters, bridgeResult c.result c.resultMode with
  | some ps, some r =>
    if outCount ps > 1 then none
    else if outCount ps = 1 ∧ r = .ownedNullable then none
    else if hasStored ps ∧ (r = .ownedNullable ∨ outCount ps = 1) then none
    else some ⟨ps, r, c.result, outCount ps = 1⟩
  | _, _ => none

/-! ## Refusal completeness for every non-admitted C04 mode. -/

theorem released_parameter_refuses (t : AbiType) (n : Bool) (rest : List (AbiType × ParamMode))
    (r : AbiType) (rm : ReturnMode) : bridge ⟨(t, .released n) :: rest, r, rm⟩ = none := by
  cases t <;> simp [bridge, bridgeParamsFrom, bridgeParamAt]

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

theorem reallocated_return_refuses (ps : List (AbiType × ParamMode)) (r : AbiType)
    (p : Nat) (n : Bool) : bridge ⟨ps, r, .reallocated p n⟩ = none := by
  simp [bridge, bridgeResult]

theorem owned_result_requires_a_mutable_pointer (ps : List (AbiType × ParamMode))
    (r : AbiType) (h : r ≠ .ptrMut) : bridge ⟨ps, r, .owned true⟩ = none := by
  simp [bridge, bridgeResult, h]

/-! ## `resource_owned` is admitted in designated owner position and nowhere
else. The card that admits `resource_stored` is forced to admit it there,
because contract spec §C04.2 requires a stored parameter's owner to be a
distinct `Owned` parameter; every other position keeps its refusal. -/

theorem an_owned_parameter_outside_owner_position_refuses
    (ps : List (AbiType × ParamMode)) (i : Nat) (t : AbiType) (n : Bool)
    (h : isDesignatedOwner ps i = false) : bridgeParamAt ps i t (.owned n) = none := by
  cases t <;> simp [bridgeParamAt, isPointer, h]

theorem owned_parameter_alone_refuses (t : AbiType) (n : Bool) (r : AbiType)
    (rm : ReturnMode) : bridge ⟨[(t, .owned n)], r, rm⟩ = none := by
  cases t <;> simp [bridge, bridgeParamsFrom, bridgeParamAt, isPointer, isDesignatedOwner]

theorem a_nullable_owner_refuses (ps : List (AbiType × ParamMode)) (i : Nat) (t : AbiType) :
    bridgeParamAt ps i t (.owned true) = none := by
  cases t <;> simp [bridgeParamAt, isPointer]

theorem an_owner_on_other_than_a_flat_mutable_pointer_refuses
    (ps : List (AbiType × ParamMode)) (i : Nat) (t : AbiType) (n : Bool)
    (h : t ≠ .ptrMut) : bridgeParamAt ps i t (.owned n) = none := by
  cases t <;> simp_all [bridgeParamAt, isPointer]

/-! ## `resource_out` (card `EXTERN-F1`). -/

theorem a_nonnull_out_parameter_refuses (ps : List (AbiType × ParamMode)) (i : Nat)
    (t : AbiType) : bridgeParamAt ps i t (.out false) = none := by
  cases t <;> simp [bridgeParamAt, isPointer]

theorem an_out_parameter_on_other_than_the_pointer_to_pointer_projection_refuses
    (ps : List (AbiType × ParamMode)) (i : Nat) (t : AbiType) (n : Bool)
    (h : t ≠ .ptrMutPtrMut) : bridgeParamAt ps i t (.out n) = none := by
  cases t <;> simp_all [bridgeParamAt, isPointer]

theorem two_out_slots_refuse (r : AbiType) (rm : ReturnMode) :
    bridge ⟨[(.ptrMutPtrMut, .out true), (.ptrMutPtrMut, .out true)], r, rm⟩ = none := by
  cases rm with
  | noCustody => simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount]
  | owned n =>
    cases n <;> cases r <;>
      simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount]
  | reallocated _ _ => simp [bridge, bridgeResult]

theorem an_out_slot_beside_an_owned_return_refuses :
    bridge ⟨[(.ptrMutPtrMut, .out true)], .ptrMut, .owned true⟩ = none := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount]

/-- Non-vacuity for the out mode. -/
theorem the_out_table_is_inhabited :
    bridge ⟨[(.ptrMutPtrMut, .out true), (.scalar, .borrowed)], .scalar, .noCustody⟩ =
      some ⟨[.outSlot, .data .scalar], .data .scalar, .scalar, true⟩ := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount, hasStored]

/-! ## `resource_stored` (card `EXTERN-F1`). -/

theorem a_nullable_stored_parameter_refuses (ps : List (AbiType × ParamMode)) (i o : Nat)
    (t : AbiType) : bridgeParamAt ps i t (.stored o true) = none := by
  cases t <;> simp [bridgeParamAt, isPointer]

theorem a_stored_parameter_on_other_than_a_flat_mutable_pointer_refuses
    (ps : List (AbiType × ParamMode)) (i o : Nat) (t : AbiType) (n : Bool)
    (h : t ≠ .ptrMut) : bridgeParamAt ps i t (.stored o n) = none := by
  cases t <;> simp_all [bridgeParamAt, isPointer]

theorem a_stored_parameter_beneath_itself_refuses (ps : List (AbiType × ParamMode)) (i : Nat)
    (t : AbiType) (n : Bool) : bridgeParamAt ps i t (.stored i n) = none := by
  cases t <;> cases n <;> simp [bridgeParamAt, isPointer]

theorem a_stored_parameter_beneath_a_non_owned_parameter_refuses :
    bridge ⟨[(.ptrMut, .borrowed), (.ptrMut, .stored 0 false)], .unit, .noCustody⟩ = none := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer]

theorem a_stored_parameter_beside_an_owned_return_refuses :
    bridge ⟨[(.ptrMut, .owned false), (.ptrMut, .stored 0 false)], .ptrMut, .owned true⟩ =
      none := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, isDesignatedOwner,
    outCount, hasStored]

/-- Non-vacuity for the store mode, and the shape contract spec §C04.2 forces:
the owner is a distinct `resource_owned` parameter of the same contract. -/
theorem the_store_table_is_inhabited :
    bridge ⟨[(.ptrMut, .owned false), (.ptrMut, .stored 0 false)], .unit, .noCustody⟩ =
      some ⟨[.owner, .stored 0], .data .unit, .unit, false⟩ := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, isDesignatedOwner,
    outCount, hasStored]

/-- Non-vacuity, unchanged from the v1 card. -/
theorem the_admitted_table_is_inhabited :
    bridge ⟨[(.scalar, .borrowed), (.ptrMut, .consumed false)], .ptrMut, .owned true⟩ =
      some ⟨[.data .scalar, .consumed], .ownedNullable, .ptrMut, false⟩ := by
  simp [bridge, bridgeParamsFrom, bridgeParamAt, bridgeResult, isPointer, outCount, hasStored]

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
  deriving DecidableEq, Repr

def derivedCustody : List CallParam → List Custody
  | [] => []
  | .data _ :: rest => derivedCustody rest
  | .outSlot :: rest => derivedCustody rest
  | .borrowed _ :: rest => .borrowed :: derivedCustody rest
  | .consumed :: rest => .consumed :: derivedCustody rest
  | .stored _ :: rest => .stored :: derivedCustody rest
  | .owner :: rest => .owner :: derivedCustody rest

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
  | .consumed | .stored _ | .owner | .outSlot => true

def derivedEffects (k : Callee) : Effects :=
  { io := true
    alloc := acquires k
    mutates := k.parameters.any paramMutates }

theorem every_derived_row_contains_io (k : Callee) : (derivedEffects k).io = true := rfl

theorem alloc_is_derived_exactly_for_an_acquisition (k : Callee) :
    (derivedEffects k).alloc = true ↔ (k.result = .ownedNullable ∨ k.outSlot = true) := by
  simp [derivedEffects, acquires]

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
  simp [admits, derivedEffects, acquires, h]

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
  · simp only [h, if_true]
    cases succeeds with
    | false => exact Or.inr ⟨_, rfl⟩
    | true => exact Or.inl ⟨_, _, rfl⟩
  · simp only [h]
    exact Or.inl ⟨_, _, rfl⟩

theorem a_data_call_never_binds_an_identity (live : Ledger) (k : Callee)
    (departing : List Nat) (fresh : Nat) (succeeds : Bool) (t : AbiType)
    (hr : k.result = .data t) (ho : k.outSlot = false) :
    stepCall live k departing fresh succeeds = .success (removeAll live departing) none := by
  simp [stepCall, acquires, hr, ho]

theorem a_data_call_never_takes_the_error_arm (live : Ledger) (k : Callee)
    (departing : List Nat) (fresh : Nat) (succeeds : Bool) (t : AbiType)
    (hr : k.result = .data t) (ho : k.outSlot = false) (l : Ledger) :
    stepCall live k departing fresh succeeds ≠ .errorExit l := by
  simp [stepCall, acquires, hr, ho]

/-- A store binds nothing: `resource_stored` is admitted with no owned return,
so a storing call is a departure and never an acquisition. -/
theorem a_store_binds_no_identity (live : Ledger) (k : Callee) (departing : List Nat)
    (fresh : Nat) (succeeds : Bool) (t : AbiType) (hr : k.result = .data t)
    (ho : k.outSlot = false) (_hs : hasStored k.parameters = true) :
    stepCall live k departing fresh succeeds = .success (removeAll live departing) none := by
  simp [stepCall, acquires, hr, ho]

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
  · simp only [h, if_true]
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
  · simp only [h, if_true]
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
def releaseCallee : Callee := ⟨[.consumed], .data .unit, .unit, false⟩

/-- The storing callee contract spec §C04.2 forces: a distinct declared owner. -/
def attachCallee : Callee := ⟨[.owner, .stored 0], .data .unit, .unit, false⟩

theorem the_release_callee_is_a_disposition : isRelease ⟨releaseCallee, 1⟩ = true := by
  decide

theorem the_attach_callee_is_not_a_disposition : isRelease ⟨attachCallee, 1⟩ = false := by
  decide

/-- The second acquisition over one live identity is admitted exactly because
its arm releases that identity. -/
theorem a_second_acquisition_is_admitted_when_its_arm_releases_the_first :
    stepAdapter [1] ⟨[], .ownedNullable, .ptrMut, false⟩ 2 true [⟨releaseCallee, 1⟩] =
      some (.success [2, 1] (some 2)) := by
  decide

/-- And is refused without it, which is EXTERN-F1's measured boundary reproduced
rather than deleted. -/
theorem the_same_acquisition_with_an_empty_arm_refuses :
    stepAdapter [1] ⟨[], .ownedNullable, .ptrMut, false⟩ 2 true [] = none := by
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
      outCount, hasStored, attachCallee]
  · decide

end QCore.ExternCall
