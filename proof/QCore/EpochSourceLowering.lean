import QCore.EpochAccounting

/-! Source lowering for the flat-epoch accounting profile (card T08-EPOCH-SRC).

`QCore.EpochAccounting` models the checked bound of one flat epoch. This module
models the *source form* that card T08-EPOCH-SRC added in front of it — the
lexical `thread scope NAME epoch { … }` declaration of
`qcore_frontend::source_epoch` — and the lowering from that declaration to the
epoch the checker sees.

What is proved here is what the lowering is responsible for, and nothing more:

* **Transcription.** Every declared figure lands in its own slot: the worker
  count, each worker's peak, effort, credit and result, the declared overhead
  and the allocation. A declaration that reads a cell twice, writes shared
  storage, nests a worker, offers a measurement as the declared overhead,
  claims a result ends at completion, or omits the creator's join is *not
  admitted*, so it never reaches the bound at all.
* **The accounting rules survive the lowering.** Two workers reading one
  declared cell lower to an entry allocation that counts it once, an unread
  cell contributes nothing, and two declared peaks lower to their sum — whose
  componentwise maximum is strictly smaller.
* **Admission implies the model bound.** An admitted declaration lowers to a
  *wellformed* epoch, so `EpochAccounting.residency_is_bounded_by_the_epoch_peak`
  applies to it: no configuration of a declaration this source form admits
  exceeds the checked bound.
* **The latency claim buys nothing.** `bound effort as latency;` is recorded on
  the declaration and ignored by the lowering: the epoch it accompanies is the
  same epoch, and `EpochAccounting.no_function_of_the_work_total_is_the_finish_time`
  still applies to it.

What is **not** proved here: that the declared bytes, effort, credits or
overhead describe any real allocation, allocator or runtime (they are declared
premises of the profile), that the Rust lowering is this function (the
transcription is by inspection, as in `EpochAccounting`), or anything at all
about execution — this source route emits no C and runs nothing. -/
namespace QCore.EpochSourceLowering

open QCore
open QCore.EpochAccounting

/-- The two axes of the source form: `memory` carries every byte figure,
`effort` carries the work quantity. -/
def memoryAxis : Axis := 0
def effortAxis : Axis := 1

def bytesOf (amount : Nat) : QuantVector :=
  fun axis => if axis = memoryAxis then amount else 0

def effortOf (amount : Nat) : QuantVector :=
  fun axis => if axis = effortAxis then amount else 0

/-- One credit, on both axes. -/
def pairOf (bytes effort : Nat) : QuantVector :=
  fun axis => if axis = memoryAxis then bytes else if axis = effortAxis then effort else 0

theorem bytesOf_le {smaller larger : Nat} (le : smaller ≤ larger) :
    QuantVector.Le (bytesOf smaller) (bytesOf larger) := by
  intro axis
  by_cases memory : axis = memoryAxis <;> simp [bytesOf, memory, le]

/-- What the creator's join clause says happens to a retained result. `close`
is the claim that completion reclaimed it, representable precisely so it can be
refused. -/
inductive Disposal where
  | move
  | drop
  | close
  deriving DecidableEq, Repr

/-- One clause of a worker block. `write` and `nested` are the two shapes the
profile refuses rather than approximates. -/
inductive Clause where
  | read (cell : Nat)
  | write (cell : Nat)
  | peak (bytes : Nat)
  | effort (units : Nat)
  | credit (bytes effort : Nat)
  | retained (bytes : Nat)
  | join (disposal : Disposal)
  | nested
  deriving DecidableEq, Repr

structure Worker where
  clauses : List Clause
  deriving Repr

structure CellDecl where
  id : Nat
  bytes : Nat
  deriving Repr

/-- Where the overhead figures came from. Only `declared` is admitted. -/
inductive Basis where
  | declared
  | measured (runs : Nat)
  deriving DecidableEq, Repr

/-- One `thread scope NAME epoch { … }` declaration. -/
structure Decl where
  cells : List CellDecl
  allocationBytes : Nat
  allocationEffort : Nat
  perEpoch : Nat
  perTask : Nat
  basis : Basis
  /-- The module wrote `bound effort as latency;`. -/
  latency : Bool
  workers : List Worker
  deriving Repr

-- ---------------------------------------------------------------------------
-- Reading the clauses of one worker.
-- ---------------------------------------------------------------------------

def readsOf : List Clause → List Nat
  | [] => []
  | .read cell :: rest => cell :: readsOf rest
  | _ :: rest => readsOf rest

def peakOf : List Clause → Nat
  | [] => 0
  | .peak bytes :: _ => bytes
  | _ :: rest => peakOf rest

def effortClauseOf : List Clause → Nat
  | [] => 0
  | .effort units :: _ => units
  | _ :: rest => effortClauseOf rest

def retainedOf : List Clause → Nat
  | [] => 0
  | .retained bytes :: _ => bytes
  | _ :: rest => retainedOf rest

def creditOf : List Clause → Option (Nat × Nat)
  | [] => none
  | .credit bytes effort :: _ => some (bytes, effort)
  | _ :: rest => creditOf rest

def joinOf : List Clause → Option Disposal
  | [] => none
  | .join disposal :: _ => some disposal
  | _ :: rest => joinOf rest

def countOf (keep : Clause → Bool) : List Clause → Nat
  | [] => 0
  | clause :: rest => (if keep clause then 1 else 0) + countOf keep rest

def isPeak : Clause → Bool | .peak _ => true | _ => false
def isEffort : Clause → Bool | .effort _ => true | _ => false
def isCredit : Clause → Bool | .credit _ _ => true | _ => false
def isRetained : Clause → Bool | .retained _ => true | _ => false
def isJoin : Clause → Bool | .join _ => true | _ => false

/-- No clause outside the admitted set of shapes. -/
def modeled : Clause → Bool
  | .write _ => false
  | .nested => false
  | _ => true

def distinct : List Nat → Bool
  | [] => true
  | cell :: rest => !rest.contains cell && distinct rest

-- ---------------------------------------------------------------------------
-- Admission. Exactly the grammar's own rules plus the core's per-task ones
-- that this lowering can decide before the checker runs.
-- ---------------------------------------------------------------------------

def workerAdmits (cells : List CellDecl) (worker : Worker) : Bool :=
  worker.clauses.all modeled
    && distinct (readsOf worker.clauses)
    && (readsOf worker.clauses).all (fun cell => cells.any (fun declared => declared.id == cell))
    && countOf isCredit worker.clauses == 1
    && countOf isPeak worker.clauses ≤ 1
    && countOf isEffort worker.clauses ≤ 1
    && countOf isRetained worker.clauses ≤ 1
    && countOf isJoin worker.clauses == 1
    && (match joinOf worker.clauses with
        | some .close => false
        | some _ => true
        | none => false)
    && retainedOf worker.clauses ≤ peakOf worker.clauses

def admits (declaration : Decl) : Bool :=
  (match declaration.basis with
   | .declared => true
   | .measured _ => false)
    && !declaration.workers.isEmpty
    && declaration.workers.all (workerAdmits declaration.cells)

abbrev Admitted (declaration : Decl) : Prop := admits declaration = true

-- ---------------------------------------------------------------------------
-- The lowering itself.
-- ---------------------------------------------------------------------------

/-- A declared cell enters the entry allocation only if some worker reads it. -/
def referenced (declaration : Decl) (cell : Nat) : Bool :=
  declaration.workers.any (fun worker => (readsOf worker.clauses).contains cell)

def lowerCells (declaration : Decl) : List Cell :=
  (declaration.cells.filter (fun cell => referenced declaration cell.id)).map
    (fun cell => { id := cell.id, footprint := bytesOf cell.bytes })

def lowerWorker (worker : Worker) : Task :=
  { privatePeak := bytesOf (peakOf worker.clauses)
    retained := bytesOf (retainedOf worker.clauses)
    transferred :=
      match joinOf worker.clauses with
      | some .move => bytesOf (retainedOf worker.clauses)
      | _ => QuantVector.zero
    work := effortOf (effortClauseOf worker.clauses)
    credit :=
      match creditOf worker.clauses with
      | some (bytes, effort) => pairOf bytes effort
      | none => QuantVector.zero }

def lower (declaration : Decl) : Epoch :=
  { cells := lowerCells declaration
    tasks := declaration.workers.map lowerWorker
    perTask := bytesOf declaration.perTask
    perEpoch := bytesOf declaration.perEpoch
    allocation := pairOf declaration.allocationBytes declaration.allocationEffort }

-- ---------------------------------------------------------------------------
-- Transcription: every declared figure lands in its own slot.
-- ---------------------------------------------------------------------------

theorem lower_preserves_the_worker_count (declaration : Decl) :
    (lower declaration).tasks.length = declaration.workers.length := by
  simp [lower]

theorem lower_preserves_each_declared_peak (worker : Worker) :
    (lowerWorker worker).privatePeak = bytesOf (peakOf worker.clauses) := rfl

theorem lower_preserves_each_declared_effort (worker : Worker) :
    (lowerWorker worker).work = effortOf (effortClauseOf worker.clauses) := rfl

theorem lower_preserves_each_declared_credit (worker : Worker) (bytes effort : Nat)
    (declared : creditOf worker.clauses = some (bytes, effort)) :
    (lowerWorker worker).credit = pairOf bytes effort := by
  simp [lowerWorker, declared]

theorem lower_preserves_the_declared_overhead (declaration : Decl) :
    (lower declaration).perEpoch = bytesOf declaration.perEpoch
      ∧ (lower declaration).perTask = bytesOf declaration.perTask := ⟨rfl, rfl⟩

/-- The declared overhead scales with the number of workers, not with anything
the lowering invents. -/
theorem lower_scales_the_overhead_with_the_worker_count (declaration : Decl) :
    overhead (lower declaration)
      = QuantVector.add (bytesOf declaration.perEpoch)
          (QuantVector.scale declaration.workers.length (bytesOf declaration.perTask)) := by
  simp [overhead, lower]

/-- The bytes and the effort never mix: a declared peak is on one axis only. -/
theorem a_declared_peak_carries_no_effort (worker : Worker) :
    (lowerWorker worker).privatePeak effortAxis = 0 := by
  simp [lowerWorker, bytesOf, memoryAxis, effortAxis]

theorem declared_effort_carries_no_bytes (worker : Worker) :
    (lowerWorker worker).work memoryAxis = 0 := by
  simp [lowerWorker, effortOf, memoryAxis, effortAxis]

/-- `join(move)` is the only clause that leaves a result live under the creator
after the join; `join(drop)` releases it there. -/
theorem the_join_clause_decides_what_survives (worker : Worker) :
    ((joinOf worker.clauses = some .move) →
        (lowerWorker worker).transferred = bytesOf (retainedOf worker.clauses))
      ∧ ((joinOf worker.clauses = some .drop) →
        (lowerWorker worker).transferred = QuantVector.zero) := by
  constructor <;> intro clause <;> simp [lowerWorker, clause]

-- ---------------------------------------------------------------------------
-- The accounting rules survive the lowering.
-- ---------------------------------------------------------------------------

/-- The worked example of CT35: one declared 4096-byte cell read by two workers
lowers to an entry allocation of 4096, not 8192. -/
def twoReaders : Decl :=
  { cells := [{ id := 1, bytes := 4096 }]
    allocationBytes := 65536, allocationEffort := 1000
    perEpoch := 512, perTask := 128, basis := .declared, latency := false
    workers :=
      [{ clauses := [.read 1, .peak 1024, .effort 10, .credit 4096 100, .join .drop] },
       { clauses := [.read 1, .peak 1024, .effort 10, .credit 4096 100, .join .drop] }] }

theorem two_readers_of_one_cell_lower_to_one_footprint :
    unionOf (lower twoReaders).cells = bytesOf 4096 := by
  funext axis
  simp [lower, lowerCells, twoReaders, referenced, readsOf, unionOf, QuantVector.add,
    QuantVector.zero]

/-- And the epoch that declaration lowers to is admitted, so the figure above is
a figure the checker really sees. -/
theorem the_two_reader_declaration_is_admitted : Admitted twoReaders := by
  decide

/-- A cell no worker reads is not part of the entry allocation: declaring
storage is not using it. -/
theorem an_unread_cell_is_not_in_the_entry_allocation :
    unionOf (lower { twoReaders with
      cells := twoReaders.cells ++ [{ id := 2, bytes := 8192 }] }).cells
      = unionOf (lower twoReaders).cells := by
  funext axis
  simp [lower, lowerCells, twoReaders, referenced, readsOf, unionOf, QuantVector.add,
    QuantVector.zero]

/-- A worker added as a second reader of an already-read cell changes nothing
about the entry allocation. The union is fixed by the identities present. -/
theorem a_second_reader_does_not_grow_the_entry_allocation :
    unionOf (lower { twoReaders with
      workers := twoReaders.workers ++
        [{ clauses := [.read 1, .peak 1, .credit 4096 100, .join .drop] }] }).cells
      = unionOf (lower twoReaders).cells := by
  funext axis
  simp [lower, lowerCells, twoReaders, referenced, readsOf, unionOf, QuantVector.add,
    QuantVector.zero]

/-- The worked example of CT36: declared peaks of 3072 and 5120 lower to a
summed 8192. -/
def twoStages : Decl :=
  { cells := []
    allocationBytes := 65536, allocationEffort := 1000
    perEpoch := 512, perTask := 128, basis := .declared, latency := false
    workers :=
      [{ clauses := [.peak 3072, .effort 10, .credit 8192 100, .join .drop] },
       { clauses := [.peak 5120, .effort 20, .credit 8192 100, .join .drop] }] }

theorem the_two_stage_declaration_is_admitted : Admitted twoStages := by
  decide

theorem declared_peaks_lower_to_their_sum :
    sumBy Task.privatePeak (lower twoStages).tasks memoryAxis = 8192 := by
  simp [lower, twoStages, lowerWorker, sumBy, peakOf, bytesOf, QuantVector.add,
    QuantVector.zero, memoryAxis]

/-- And the componentwise maximum of the same declaration is strictly smaller,
so a maximized reading of this source is an undercount, not a rounding. -/
theorem the_maximum_of_the_declared_peaks_undercounts :
    maxBy Task.privatePeak (lower twoStages).tasks memoryAxis
      < sumBy Task.privatePeak (lower twoStages).tasks memoryAxis := by
  simp [lower, twoStages, lowerWorker, sumBy, maxBy, peakOf, bytesOf, QuantVector.add,
    QuantVector.join, QuantVector.zero, memoryAxis]

/-- The epoch peak of a lowered declaration is exactly the union, plus the
summed peaks, plus the declared overhead — the bound of the model, on the
figures of the source. -/
theorem the_lowered_bound_is_the_models_bound (declaration : Decl) :
    epochPeak (lower declaration)
      = QuantVector.add (unionOf (lower declaration).cells)
          (QuantVector.add (sumBy Task.privatePeak (lower declaration).tasks)
            (overhead (lower declaration))) := rfl

-- ---------------------------------------------------------------------------
-- Admission implies the model bound.
-- ---------------------------------------------------------------------------

theorem worker_admits_retained_within_peak {cells : List CellDecl} {worker : Worker}
    (admitted : workerAdmits cells worker = true) :
    retainedOf worker.clauses ≤ peakOf worker.clauses := by
  simp [workerAdmits, Bool.and_eq_true] at admitted
  exact admitted.2

theorem lowered_worker_is_wellformed {cells : List CellDecl} {worker : Worker}
    (admitted : workerAdmits cells worker = true) : (lowerWorker worker).Wellformed := by
  refine ⟨?_, bytesOf_le (worker_admits_retained_within_peak admitted)⟩
  intro axis
  cases disposal : joinOf worker.clauses with
  | none => simp [lowerWorker, disposal, QuantVector.zero]
  | some value =>
    cases value with
    | move => simp [lowerWorker, disposal]
    | drop => simp [lowerWorker, disposal, QuantVector.zero]
    | close => simp [lowerWorker, disposal, QuantVector.zero]

/-- The payoff: a declaration this source form admits lowers to a wellformed
epoch, so every theorem of `EpochAccounting` about wellformed epochs applies to
it. Nothing in the lowering can produce an epoch the model does not cover. -/
theorem an_admitted_declaration_lowers_to_a_wellformed_epoch {declaration : Decl}
    (admitted : Admitted declaration) : (lower declaration).Wellformed := by
  intro task member
  simp only [lower, List.mem_map] at member
  obtain ⟨worker, worker_member, lowered⟩ := member
  have basis_and_all := (Bool.and_eq_true _ _).mp admitted
  have single := List.all_eq_true.mp basis_and_all.2 worker worker_member
  exact lowered ▸ lowered_worker_is_wellformed single

/-- Therefore no configuration of an admitted declaration exceeds its bound.
This is the source-level statement of `residency_is_bounded_by_the_epoch_peak`. -/
theorem every_configuration_of_an_admitted_declaration_is_bounded {declaration : Decl}
    (admitted : Admitted declaration) (phases : List Phase) :
    QuantVector.Le (residency (lower declaration) phases) (epochPeak (lower declaration)) :=
  residency_is_bounded_by_the_epoch_peak
    (an_admitted_declaration_lowers_to_a_wellformed_epoch admitted) phases

/-- And the all-running configuration of a lowered declaration attains it, so
the bound is the epoch's own number rather than a slack one. -/
theorem the_all_running_configuration_of_a_lowered_declaration_attains_its_bound
    (declaration : Decl) :
    residency (lower declaration) (allPhases .running (lower declaration).tasks)
      = epochPeak (lower declaration) :=
  the_all_running_configuration_attains_the_bound (lower declaration)

-- ---------------------------------------------------------------------------
-- What the source form refuses.
-- ---------------------------------------------------------------------------

/-- A worker whose result outlives its task but exceeds the peak that task was
accounted for. -/
theorem an_over_retained_worker_is_not_admitted :
    ¬ Admitted { twoStages with
      workers := [{ clauses := [.peak 128, .retained 256, .credit 8192 100, .join .move] }] } := by
  decide

/-- The claim that a result stopped being live at completion. -/
theorem a_close_join_is_not_admitted :
    ¬ Admitted { twoStages with
      workers := [{ clauses := [.peak 1024, .retained 256, .credit 8192 100, .join .close] }] } := by
  decide

/-- A worker the creator never joins. -/
theorem a_worker_without_a_join_is_not_admitted :
    ¬ Admitted { twoStages with
      workers := [{ clauses := [.peak 1024, .retained 256, .credit 8192 100] }] } := by
  decide

/-- A worker with no credit clause: its share of the allocation is not a figure
the lowering may invent. -/
theorem a_worker_without_a_credit_is_not_admitted :
    ¬ Admitted { twoStages with
      workers := [{ clauses := [.peak 1024, .join .drop] }] } := by
  decide

/-- The same shared cell named twice by one worker. Refused rather than
deduplicated: a repeat is evidence the plan is summing. -/
theorem a_repeated_shared_entry_is_not_admitted :
    ¬ Admitted { twoReaders with
      workers := [{ clauses := [.read 1, .read 1, .peak 1024, .credit 4096 100, .join .drop] }] } := by
  decide

/-- A shared entry that names no declared cell. -/
theorem an_undeclared_shared_entry_is_not_admitted :
    ¬ Admitted { twoReaders with
      workers := [{ clauses := [.read 7, .peak 1024, .credit 4096 100, .join .drop] }] } := by
  decide

/-- A worker that writes shared storage: this profile admits shared reads. -/
theorem a_shared_write_is_not_admitted :
    ¬ Admitted { twoReaders with
      workers := [{ clauses := [.write 1, .peak 1024, .credit 4096 100, .join .drop] }] } := by
  decide

/-- A worker that creates a worker: the epoch would not be flat. -/
theorem a_nested_worker_is_not_admitted :
    ¬ Admitted { twoStages with
      workers := [{ clauses := [.peak 1024, .credit 8192 100, .join .drop, .nested] }] } := by
  decide

/-- A native measurement offered as the declared overhead. An observation of
some runs bounds those runs, not every admitted epoch. -/
theorem a_measured_overhead_basis_is_not_admitted :
    ¬ Admitted { twoStages with basis := .measured 30 } := by
  decide

/-- An epoch with no workers at all. -/
theorem an_empty_epoch_is_not_admitted :
    ¬ Admitted { twoStages with workers := [] } := by
  decide

-- ---------------------------------------------------------------------------
-- Credits, and the latency claim.
-- ---------------------------------------------------------------------------

theorem declared_credits_lower_to_the_granted_total :
    grantedCredits (lower twoStages) memoryAxis = 16384 := by
  simp [lower, twoStages, lowerWorker, grantedCredits, sumBy, creditOf, pairOf,
    QuantVector.add, QuantVector.zero, memoryAxis]

/-- The worked example of CT38: two child credits that split a total to more
than the total are not covered by it, whatever each child's own check says. -/
theorem credits_that_split_the_total_to_more_than_the_total_are_refused :
    ¬ QuantVector.Le (grantedCredits (lower { twoStages with allocationBytes := 8192 }))
        (lower { twoStages with allocationBytes := 8192 }).allocation := by
  refine credits_beyond_the_allocation_are_not_covered (axis := memoryAxis) ?_
  simp [lower, twoStages, lowerWorker, grantedCredits, sumBy, creditOf, pairOf,
    QuantVector.add, QuantVector.zero, memoryAxis]

/-- The worked example of CT39, first half: the latency claim is recorded on the
declaration and changes no lowered quantity. The epoch it accompanies is the
same epoch. -/
theorem the_latency_claim_changes_no_lowered_quantity (declaration : Decl) :
    lower { declaration with latency := true } = lower declaration := rfl

/-- The second half: there is still no schedule anywhere in the model, so the
work total of a lowered declaration is not a finish time. This is
`EpochAccounting.no_function_of_the_work_total_is_the_finish_time` restated
where the source form can see it. -/
theorem no_lowered_declaration_yields_a_finish_time :
    ¬ ∃ latency : Nat → Nat,
        ∀ works starts : List Nat, latency (totalWork works) = finishTime starts works :=
  no_function_of_the_work_total_is_the_finish_time

end QCore.EpochSourceLowering
