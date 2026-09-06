namespace QCore.VerifierEvidenceV2

/-! Reduced executable W3 observed-contract replay. Required hook identities
and predicates belong to the core task. Provider records contain values only.
This model proves the nonempty denominator and precondition-counting boundary;
it neither authenticates runtime observations nor models the full Rust scalar
predicate parser, hashes, process transport or lifecycle source importer. -/

structure Task where
  complete : Bool
  requiredHooks : List Nat
  expectedOutput : Nat
  observationsTrusted : Bool
  observedRunsOnly : Bool
  deriving DecidableEq, Repr

structure Observation where
  hook : Nat
  input : Nat
  output : Nat
  deriving DecidableEq, Repr

def meetsPrecondition (record : Observation) : Bool := decide (record.input > 0)
def meetsPostcondition (task : Task) (record : Observation) : Bool :=
  decide (record.output = task.expectedOutput)

def counts (task : Task) (hook : Nat) (record : Observation) : Bool :=
  decide (record.hook = hook) && meetsPrecondition record && meetsPostcondition task record

def Accepts (task : Task) (records : List Observation) : Prop :=
  task.complete = true ∧ task.requiredHooks ≠ [] ∧
  task.requiredHooks.all (fun hook => records.any (counts task hook)) = true ∧
  records.all (fun record =>
    !(meetsPrecondition record) || meetsPostcondition task record) = true ∧
  task.observationsTrusted = true ∧ task.observedRunsOnly = true

instance (task : Task) (records : List Observation) : Decidable (Accepts task records) := by
  unfold Accepts
  infer_instance

def replay (task : Task) (records : List Observation) : Bool :=
  decide (Accepts task records)

theorem accepted_has_complete_nonempty_inventory
    (task : Task) (records : List Observation) (accepted : replay task records = true) :
    task.complete = true ∧ task.requiredHooks ≠ [] := by
  have valid : Accepts task records := of_decide_eq_true accepted
  exact ⟨valid.1, valid.2.1⟩

theorem accepted_hook_has_recomputed_observation
    (task : Task) (records : List Observation) (accepted : replay task records = true)
    (hook : Nat) (required : hook ∈ task.requiredHooks) :
    ∃ record ∈ records, record.hook = hook ∧ record.input > 0 ∧
      record.output = task.expectedOutput := by
  have valid : Accepts task records := of_decide_eq_true accepted
  have found := List.all_eq_true.mp valid.2.2.1 hook required
  obtain ⟨record, member, counted⟩ := List.any_eq_true.mp found
  refine ⟨record, member, ?_⟩
  simpa [counts, meetsPrecondition, meetsPostcondition, and_assoc] using counted

theorem incomplete_inventory_rejects (task : Task) (records : List Observation)
    (incomplete : task.complete = false) : replay task records = false := by
  simp [replay, Accepts, incomplete]

theorem empty_inventory_rejects (task : Task) (records : List Observation)
    (empty : task.requiredHooks = []) : replay task records = false := by
  simp [replay, Accepts, empty]

theorem false_precondition_never_counts (task : Task) (hook : Nat)
    (record : Observation) (outside : record.input = 0) :
    counts task hook record = false := by
  simp [counts, meetsPrecondition, outside]

/-- Record the conditional trust requirement without overriding explicit refusal. -/
def applyObservationTrust (declared : Option Bool) (task : Task) : Task :=
  { task with
    observationsTrusted := declared.getD true
    observedRunsOnly := true
    complete := task.complete && declared.getD true }

theorem accepted_requires_observation_trust_and_bounds
    (task : Task) (records : List Observation) (accepted : replay task records = true) :
    task.observationsTrusted = true ∧ task.observedRunsOnly = true := by
  have valid : Accepts task records := of_decide_eq_true accepted
  exact valid.2.2.2.2

theorem explicit_refusal_is_preserved (task : Task) :
    (applyObservationTrust (some false) task).observationsTrusted = false := by
  rfl

theorem explicit_refusal_derivation_rejects (task : Task) (records : List Observation) :
    replay (applyObservationTrust (some false) task) records = false := by
  simp [applyObservationTrust, replay, Accepts]

theorem absent_flag_names_conditional_trust (task : Task) :
    (applyObservationTrust none task).observationsTrusted = true := by
  rfl

end QCore.VerifierEvidenceV2
