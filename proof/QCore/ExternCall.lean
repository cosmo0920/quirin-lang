/-! Card `source.extern-call.v1`: the bounded extern-call model.

This is a model of `crates/qcore-core/src/extern_call.rs`. It establishes four
things about the *model*: which resource-contract v1 shapes are callable, that
the derived summary is a function of the contract, that applying a derived
summary changes the ledger by exactly the declared delta, and that the
nullable-owned-return adapter has exactly two exits.

Two premises are explicit and are **not** discharged here or anywhere else in
this repository: that a callee's C implementation satisfies its declared
contract, and that the Rust extraction of a source module agrees with the
transcription below. A conclusion drawn through an extern call is valid only
relative to the declaration (`ContractTrust::Declared`, contract spec §C06). -/
namespace QCore.ExternCall

/-- The C ABI projection of contract spec §C04.1, collapsed to the distinctions
this model uses: a scalar carries data, a pointer may carry custody, and the
pointer-to-pointer projection exists only for the excluded `resource_out`. -/
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

/-- The admitted v1 bridging of one parameter (design §3.2). -/
inductive CallParam where
  | data (t : AbiType)
  | borrowed (t : AbiType)
  | consumed
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
  deriving DecidableEq, Repr

def isPointer : AbiType → Bool
  | .ptrMut | .ptrConst | .ptrMutPtrMut => true
  | .scalar | .unit => false

def isMutablePointer : AbiType → Bool
  | .ptrMut | .ptrMutPtrMut => true
  | .ptrConst | .scalar | .unit => false

/-- Admission of one parameter. Every excluded mode returns `none` by its own
match arm; there is no catch-all that could silently absorb a new mode. -/
def bridgeParam : AbiType → ParamMode → Option CallParam
  | .unit, _ => none
  | t, .borrowed => if isPointer t then
      (if t = .ptrMutPtrMut then none else some (.borrowed t))
    else some (.data t)
  | t, .consumed nullable =>
      if !isPointer t then none
      else if nullable then none
      else if t = .ptrMut then some .consumed else none
  | _, .owned _ => none
  | _, .released _ => none
  | _, .out _ => none
  | _, .stored _ _ => none
  | _, .removed _ _ => none

def bridgeParams : List (AbiType × ParamMode) → Option (List CallParam)
  | [] => some []
  | (t, m) :: rest =>
    match bridgeParam t m, bridgeParams rest with
    | some p, some ps => some (p :: ps)
    | _, _ => none

def bridgeResult : AbiType → ReturnMode → Option CallResult
  | t, .noCustody => some (.data t)
  | _, .owned false => none
  | t, .owned true => if t = .ptrMut then some .ownedNullable else none
  | _, .reallocated _ _ => none

def bridge (c : Contract) : Option Callee :=
  match bridgeParams c.parameters, bridgeResult c.result c.resultMode with
  | some ps, some r => some ⟨ps, r, c.result⟩
  | _, _ => none

/-! ## Refusal completeness for every non-admitted C04 mode (design §3.1). -/

theorem owned_parameter_refuses (t : AbiType) (n : Bool) (rest : List (AbiType × ParamMode))
    (r : AbiType) (rm : ReturnMode) : bridge ⟨(t, .owned n) :: rest, r, rm⟩ = none := by
  cases t <;> simp [bridge, bridgeParams, bridgeParam]

theorem released_parameter_refuses (t : AbiType) (n : Bool) (rest : List (AbiType × ParamMode))
    (r : AbiType) (rm : ReturnMode) : bridge ⟨(t, .released n) :: rest, r, rm⟩ = none := by
  cases t <;> simp [bridge, bridgeParams, bridgeParam]

theorem out_parameter_refuses (t : AbiType) (n : Bool) (rest : List (AbiType × ParamMode))
    (r : AbiType) (rm : ReturnMode) : bridge ⟨(t, .out n) :: rest, r, rm⟩ = none := by
  cases t <;> simp [bridge, bridgeParams, bridgeParam]

theorem stored_parameter_refuses (t : AbiType) (o : Nat) (n : Bool)
    (rest : List (AbiType × ParamMode)) (r : AbiType) (rm : ReturnMode) :
    bridge ⟨(t, .stored o n) :: rest, r, rm⟩ = none := by
  cases t <;> simp [bridge, bridgeParams, bridgeParam]

theorem removed_parameter_refuses (t : AbiType) (o : Nat) (n : Bool)
    (rest : List (AbiType × ParamMode)) (r : AbiType) (rm : ReturnMode) :
    bridge ⟨(t, .removed o n) :: rest, r, rm⟩ = none := by
  cases t <;> simp [bridge, bridgeParams, bridgeParam]

theorem nullable_consumed_parameter_refuses (t : AbiType) (rest : List (AbiType × ParamMode))
    (r : AbiType) (rm : ReturnMode) : bridge ⟨(t, .consumed true) :: rest, r, rm⟩ = none := by
  cases t <;> simp [bridge, bridgeParams, bridgeParam, isPointer]

theorem scalar_custody_refuses (rest : List (AbiType × ParamMode)) (r : AbiType)
    (rm : ReturnMode) : bridge ⟨(.scalar, .consumed false) :: rest, r, rm⟩ = none := by
  simp [bridge, bridgeParams, bridgeParam, isPointer]

theorem borrowing_a_pointer_to_pointer_refuses (rest : List (AbiType × ParamMode))
    (r : AbiType) (rm : ReturnMode) : bridge ⟨(.ptrMutPtrMut, .borrowed) :: rest, r, rm⟩ = none := by
  simp [bridge, bridgeParams, bridgeParam, isPointer]

theorem consuming_a_const_pointer_refuses (rest : List (AbiType × ParamMode))
    (r : AbiType) (rm : ReturnMode) :
    bridge ⟨(.ptrConst, .consumed false) :: rest, r, rm⟩ = none := by
  simp [bridge, bridgeParams, bridgeParam, isPointer]

theorem nonnull_owned_return_refuses (ps : List (AbiType × ParamMode)) (r : AbiType) :
    bridge ⟨ps, r, .owned false⟩ = none := by
  simp [bridge, bridgeResult]

theorem reallocated_return_refuses (ps : List (AbiType × ParamMode)) (r : AbiType)
    (p : Nat) (n : Bool) : bridge ⟨ps, r, .reallocated p n⟩ = none := by
  simp [bridge, bridgeResult]

theorem owned_result_requires_a_mutable_pointer (ps : List (AbiType × ParamMode))
    (r : AbiType) (h : r ≠ .ptrMut) : bridge ⟨ps, r, .owned true⟩ = none := by
  simp [bridge, bridgeResult, h]

/-- Non-vacuity: the admitted table is inhabited, so the refusals above are not
refusing everything. -/
theorem the_admitted_table_is_inhabited :
    bridge ⟨[(.scalar, .borrowed), (.ptrMut, .consumed false)], .ptrMut, .owned true⟩ =
      some ⟨[.data .scalar, .consumed], .ownedNullable, .ptrMut⟩ := by
  simp [bridge, bridgeParams, bridgeParam, bridgeResult, isPointer]

/-! ## Derivation is a function of the contract (design §3.4). -/

theorem admission_is_deterministic {c : Contract} {k₁ k₂ : Callee}
    (h₁ : bridge c = some k₁) (h₂ : bridge c = some k₂) : k₁ = k₂ := by
  rw [h₁] at h₂; exact Option.some.inj h₂

/-- Ownership behavior of one resource-bearing argument. Data parameters carry
no custody and are absent from the derived vector. -/
inductive Custody where
  | borrowed
  | consumed
  deriving DecidableEq, Repr

def derivedCustody : List CallParam → List Custody
  | [] => []
  | .data _ :: rest => derivedCustody rest
  | .borrowed _ :: rest => .borrowed :: derivedCustody rest
  | .consumed :: rest => .consumed :: derivedCustody rest

structure Effects where
  io : Bool
  alloc : Bool
  mutates : Bool
  deriving DecidableEq, Repr

def paramMutates : CallParam → Bool
  | .data t | .borrowed t => isMutablePointer t
  | .consumed => true

def derivedEffects (k : Callee) : Effects :=
  { io := true
    alloc := k.result = .ownedNullable
    mutates := k.parameters.any paramMutates }

theorem every_derived_row_contains_io (k : Callee) : (derivedEffects k).io = true := rfl

theorem alloc_is_derived_exactly_for_an_owned_return (k : Callee) :
    (derivedEffects k).alloc = true ↔ k.result = .ownedNullable := by
  simp [derivedEffects]

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
  simp [admits, derivedEffects, h]

/-! ## The call step and its custody delta (design §3.4, §3.3). -/

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

/-- One extern call. `consumed` are the identities passed to `consumed`
parameters, `fresh` is the identity an owned return would bind, and `succeeds`
is the fixture's literal outcome control. -/
def stepCall (live : Ledger) (k : Callee) (consumed : List Nat) (fresh : Nat)
    (succeeds : Bool) : Outcome :=
  let remaining := removeAll live consumed
  match k.result with
  | .data _ => .success remaining none
  | .ownedNullable =>
    if succeeds then .success (fresh :: remaining) (some fresh)
    else .errorExit remaining

theorem adapter_totality (live : Ledger) (k : Callee) (consumed : List Nat) (fresh : Nat)
    (succeeds : Bool) :
    (∃ l b, stepCall live k consumed fresh succeeds = .success l b) ∨
      (∃ l, stepCall live k consumed fresh succeeds = .errorExit l) := by
  unfold stepCall
  cases k.result with
  | data t => exact Or.inl ⟨_, _, rfl⟩
  | ownedNullable =>
    cases succeeds with
    | false => exact Or.inr ⟨_, rfl⟩
    | true => exact Or.inl ⟨_, _, rfl⟩

theorem a_data_call_never_binds_an_identity (live : Ledger) (k : Callee)
    (consumed : List Nat) (fresh : Nat) (succeeds : Bool) (t : AbiType)
    (h : k.result = .data t) :
    stepCall live k consumed fresh succeeds = .success (removeAll live consumed) none := by
  simp [stepCall, h]

theorem a_data_call_never_takes_the_error_arm (live : Ledger) (k : Callee)
    (consumed : List Nat) (fresh : Nat) (succeeds : Bool) (t : AbiType)
    (h : k.result = .data t) (l : Ledger) :
    stepCall live k consumed fresh succeeds ≠ .errorExit l := by
  simp [stepCall, h]

theorem a_failed_acquisition_binds_nothing (live : Ledger) (k : Callee)
    (consumed : List Nat) (fresh : Nat) (h : k.result = .ownedNullable) :
    stepCall live k consumed fresh false = .errorExit (removeAll live consumed) := by
  simp [stepCall, h]

theorem a_successful_acquisition_binds_exactly_the_fresh_identity (live : Ledger)
    (k : Callee) (consumed : List Nat) (fresh : Nat) (h : k.result = .ownedNullable) :
    stepCall live k consumed fresh true = .success (fresh :: removeAll live consumed)
      (some fresh) := by
  simp [stepCall, h]

/-- Conservation: whichever arm is taken, every identity the call did not
consume survives, and nothing else is removed. -/
theorem the_custody_delta_is_exactly_the_declared_one (live : Ledger) (k : Callee)
    (consumed : List Nat) (fresh : Nat) (succeeds : Bool) (x : Nat)
    (kept : x ∉ consumed) (present : x ∈ live) :
    x ∈ (match stepCall live k consumed fresh succeeds with
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
  have surviving := base consumed live kept present
  unfold stepCall
  cases k.result with
  | data t => simpa using surviving
  | ownedNullable =>
    cases succeeds with
    | false => simpa using surviving
    | true => simp [List.mem_cons, surviving]

theorem a_consumed_identity_does_not_survive (live : Ledger) (k : Callee)
    (consumed : List Nat) (fresh : Nat) (succeeds : Bool) (x : Nat)
    (moved : x ∈ consumed) (distinct : fresh ≠ x) :
    x ∉ (match stepCall live k consumed fresh succeeds with
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
  have gone := base consumed live moved
  unfold stepCall
  cases k.result with
  | data t => simpa using gone
  | ownedNullable =>
    cases succeeds with
    | false => simpa using gone
    | true => simp [List.mem_cons, gone, Ne.symm distinct]

end QCore.ExternCall
