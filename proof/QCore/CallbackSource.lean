namespace QCore.CallbackSource

/-! Symbol provenance for the bounded callback source-path projection.
The executable account erases C expressions to their symbol references and
producer type tags. It does not prove Clang/Rust import, C evaluation order,
integer arithmetic, memory bounds, external call contracts, or source/runtime
refinement. Both alternatives of each imported condition remain explicit. -/

structure Reference where
  operation : Nat
  typeTag : String
  deriving DecidableEq, Repr

structure Record where
  references : List Reference
  produces : Option String
  deriving DecidableEq, Repr

def referenceValid (prior : List (Option String)) (reference : Reference) : Bool :=
  decide (prior[reference.operation]? = some (some reference.typeTag))

def recordValid (prior : List (Option String)) (record : Record) : Bool :=
  record.references.all (referenceValid prior)

def appendRecord (prior : List (Option String)) (record : Record) :
    Option (List (Option String)) :=
  if recordValid prior record then some (prior ++ [record.produces]) else none

theorem accepted_record_has_typed_producers
    {prior next : List (Option String)} {record : Record}
    (accepted : appendRecord prior record = some next) :
    ∀ reference ∈ record.references,
      prior[reference.operation]? = some (some reference.typeTag) := by
  unfold appendRecord at accepted
  split at accepted
  · rename_i valid
    intro reference member
    have present := List.all_eq_true.mp valid reference member
    simpa [referenceValid] using present
  · contradiction

theorem accepted_record_rejects_forward_references
    {prior next : List (Option String)} {record : Record}
    (accepted : appendRecord prior record = some next) :
    ∀ reference ∈ record.references, reference.operation < prior.length := by
  intro reference member
  have producer := accepted_record_has_typed_producers accepted reference member
  obtain ⟨bounded, _⟩ := List.getElem?_eq_some_iff.mp producer
  exact bounded

theorem accepted_record_appends_exactly_its_producer
    {prior next : List (Option String)} {record : Record}
    (accepted : appendRecord prior record = some next) :
    next = prior ++ [record.produces] := by
  unfold appendRecord at accepted
  split at accepted
  · cases accepted
    rfl
  · contradiction

theorem wrong_type_reference_is_rejected
    (prior : List (Option String)) (reference : Reference)
    (mismatch : prior[reference.operation]? ≠ some (some reference.typeTag)) :
    referenceValid prior reference = false := by
  simp [referenceValid, mismatch]

-- A return/write/guard record does not produce a read or call result.
theorem nonproducing_record_cannot_supply_symbol (prior : List (Option String))
    (typeTag : String) :
    referenceValid (prior ++ [none]) { operation := prior.length, typeTag } = false := by
  simp [referenceValid]

def branchPaths (prior : List Bool) : List (List Bool) :=
  [prior ++ [true], prior ++ [false]]

theorem branch_preserves_taken_alternative (prior : List Bool) :
    prior ++ [true] ∈ branchPaths prior := by
  simp [branchPaths]

theorem branch_preserves_not_taken_alternative (prior : List Bool) :
    prior ++ [false] ∈ branchPaths prior := by
  simp [branchPaths]

/-! Abstract lexical storage boundary for array/record path projections.
The C layout and initialization semantics remain obligations of consumers. -/
def startStorage (live : List Nat) (slot : Nat) : Option (List Nat) :=
  if slot ∈ live then none else some (slot :: live)

def endStorage (live : List Nat) (slot : Nat) : List Nat :=
  live.filter (fun candidate => candidate != slot)

theorem accepted_storage_start_is_fresh
    {live next : List Nat} {slot : Nat}
    (accepted : startStorage live slot = some next) : slot ∉ live := by
  unfold startStorage at accepted
  split at accepted
  · contradiction
  · assumption

theorem accepted_storage_start_retains_existing
    {live next : List Nat} {slot : Nat}
    (accepted : startStorage live slot = some next) :
    ∀ prior ∈ live, prior ∈ next := by
  unfold startStorage at accepted
  split at accepted
  · contradiction
  · cases accepted
    intro prior member
    exact List.mem_cons_of_mem slot member

theorem storage_end_removes_identity (live : List Nat) (slot : Nat) :
    slot ∉ endStorage live slot := by
  simp [endStorage]

theorem storage_end_preserves_other_identity
    (live : List Nat) (slot other : Nat) (different : other ≠ slot)
    (present : other ∈ live) : other ∈ endStorage live slot := by
  simp [endStorage, different, present]

end QCore.CallbackSource
