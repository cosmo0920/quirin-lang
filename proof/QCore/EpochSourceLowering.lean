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

-- ===========================================================================
-- Composition with a checked task region (card T08-COMPOSE).
-- ===========================================================================

/-! `source.epoch.tasks.v1` puts the same declaration in front of a **real**
checked task region instead of in front of declared worker shapes. The rows of
the plan must then agree with what the checked region actually does, and the
agreement relation is what is modeled here.

Two halves, kept apart on purpose:

* **Matching.** Rows naming worker `w`, in order, correspond one-to-one with the
  region's creations of `w`, in order. Both directions are checked, per worker,
  so an over-declared row cannot be paid for by an undeclared creation.
* **Per pair.** A matched row and creation must agree on the ordered capture
  list and on the result: `retained` is declared exactly when the creation
  retains one, and a declared `join` must equal the disposal the region
  performs. An elided `join` is *derived* from the region.

The payoff is the last theorem: an agreed composition builds a declaration the
existing `admits` predicate accepts, so
`every_configuration_of_an_admitted_declaration_is_bounded` applies to it
unchanged. Agreement is not a bound; it is what makes the bound be about the
region that runs.

Not modeled here, exactly as in the rest of this file: that the Rust derivation
is this function (transcription is by inspection), and that any declared byte or
effort figure describes a real allocation. -/

/-- A capture as either side describes it. The composed profile admits scalar
copies and owned moves; a read loan has no composed spelling. -/
inductive Capture where
  | copy
  | move
  deriving DecidableEq, Repr

/-- One creation, as the checked task IR describes it. Every field of this is
read out of the region; none of it is read out of the plan. -/
structure LedgerSpawn where
  worker : Nat
  captures : List Capture
  /-- The spawned worker's checked result type is an owned resource. -/
  retainsResult : Bool
  /-- What the creator then does with it. `none` exactly when nothing is
  retained; never `close`, because completion is not a disposal the region can
  perform. -/
  disposal : Option Disposal
  deriving DecidableEq, Repr

/-- One row of the composed plan. `retained` and `join` are the two facts the
plan may still spell; everything else about the shape is derived. -/
structure Row where
  worker : Nat
  captures : List Capture
  peak : Nat
  effort : Nat
  creditBytes : Nat
  creditEffort : Nat
  retained : Option Nat
  join : Option Disposal
  deriving DecidableEq, Repr

/-- A creation the region could actually have produced: it retains a result
exactly when it has a disposal, and never claims completion reclaimed it. -/
def ledgerWellformed (spawn : LedgerSpawn) : Bool :=
  (spawn.disposal.isSome == spawn.retainsResult)
    && (match spawn.disposal with
        | some .close => false
        | _ => true)

/-- The result half of the agreement. -/
def resultAgrees (row : Row) (spawn : LedgerSpawn) : Bool :=
  match row.retained, spawn.retainsResult, spawn.disposal with
  | some _, true, some derived =>
      match row.join with
      | none => true
      | some declared => declared == derived
  | none, false, _ => row.join.isNone
  | _, _, _ => false

/-- One row agrees with one creation. -/
def rowAgrees (row : Row) (spawn : LedgerSpawn) : Bool :=
  (row.worker == spawn.worker) && (row.captures == spawn.captures) && resultAgrees row spawn

def rowsFor (name : Nat) (rows : List Row) : List Row :=
  rows.filter (fun row => row.worker == name)

def spawnsFor (name : Nat) (ledger : List LedgerSpawn) : List LedgerSpawn :=
  ledger.filter (fun spawn => spawn.worker == name)

/-- One worker's rows and creations correspond exactly, in order. -/
def workerAgrees (name : Nat) (rows : List Row) (ledger : List LedgerSpawn) : Bool :=
  ((rowsFor name rows).length == (spawnsFor name ledger).length)
    && ((rowsFor name rows).zip (spawnsFor name ledger)).all
        (fun pair => rowAgrees pair.1 pair.2)

/-- The whole agreement relation. Every worker named by *either* side is
checked, so a name only the plan mentions and a name only the region creates are
both caught. Checking a name twice is harmless: the per-worker test does not
depend on how often it is asked. -/
def agrees (rows : List Row) (ledger : List LedgerSpawn) : Bool :=
  (rows.map Row.worker ++ ledger.map LedgerSpawn.worker).all
    (fun name => workerAgrees name rows ledger)

-- ---------------------------------------------------------------------------
-- The matching half: both directions, per worker.
-- ---------------------------------------------------------------------------

/-- Agreement is decided per worker, so it entails that every worker named by
either side has as many rows as creations. This is the whole matching half. -/
theorem agrees_implies_the_counts_match {rows : List Row} {ledger : List LedgerSpawn}
    {name : Nat}
    (named : name ∈ rows.map Row.worker ++ ledger.map LedgerSpawn.worker)
    (holds : agrees rows ledger = true) :
    (rowsFor name rows).length = (spawnsFor name ledger).length := by
  have single : workerAgrees name rows ledger = true :=
    List.all_eq_true.mp holds name named
  have parts := (Bool.and_eq_true _ _).mp single
  exact (beq_iff_eq).mp parts.1

theorem a_creation_names_its_worker {ledger : List LedgerSpawn} {rows : List Row} {name : Nat}
    (created : spawnsFor name ledger ≠ []) :
    name ∈ rows.map Row.worker ++ ledger.map LedgerSpawn.worker := by
  obtain ⟨spawn, member⟩ : ∃ spawn, spawn ∈ spawnsFor name ledger := by
    cases sites : spawnsFor name ledger with
    | nil => exact absurd sites created
    | cons head _ => exact ⟨head, by simp⟩
  have inside : spawn ∈ ledger ∧ (spawn.worker == name) = true := by
    simpa [spawnsFor, List.mem_filter] using member
  exact List.mem_append_right _
    (List.mem_map.mpr ⟨spawn, inside.1, (beq_iff_eq).mp inside.2⟩)

theorem a_row_names_its_worker {ledger : List LedgerSpawn} {rows : List Row} {name : Nat}
    (declared : rowsFor name rows ≠ []) :
    name ∈ rows.map Row.worker ++ ledger.map LedgerSpawn.worker := by
  obtain ⟨row, member⟩ : ∃ row, row ∈ rowsFor name rows := by
    cases declaredRows : rowsFor name rows with
    | nil => exact absurd declaredRows declared
    | cons head _ => exact ⟨head, by simp⟩
  have inside : row ∈ rows ∧ (row.worker == name) = true := by
    simpa [rowsFor, List.mem_filter] using member
  exact List.mem_append_left _
    (List.mem_map.mpr ⟨row, inside.1, (beq_iff_eq).mp inside.2⟩)

/-- A creation the plan never declares is refused. -/
theorem a_creation_with_no_row_is_not_agreed
    {rows : List Row} {ledger : List LedgerSpawn} {name : Nat}
    (declared : rowsFor name rows = [])
    (created : spawnsFor name ledger ≠ []) : agrees rows ledger = false := by
  cases holds : agrees rows ledger with
  | false => rfl
  | true =>
    have counts := agrees_implies_the_counts_match
      (rows := rows) (a_creation_names_its_worker created) holds
    rw [declared] at counts
    exact absurd (List.eq_nil_of_length_eq_zero counts.symm) created

/-- A row for a worker the region never creates is refused. -/
theorem a_row_with_no_creation_is_not_agreed
    {rows : List Row} {ledger : List LedgerSpawn} {name : Nat}
    (declared : rowsFor name rows ≠ [])
    (created : spawnsFor name ledger = []) : agrees rows ledger = false := by
  cases holds : agrees rows ledger with
  | false => rfl
  | true =>
    have counts := agrees_implies_the_counts_match
      (ledger := ledger) (a_row_names_its_worker declared) holds
    rw [created] at counts
    exact absurd (List.eq_nil_of_length_eq_zero counts) declared

-- ---------------------------------------------------------------------------
-- Worked compositions: the owned round trip and the copy-only one.
-- ---------------------------------------------------------------------------

/-- The owned round trip: one creation that moves an owned resource in and hands
one back, which the creator releases at the join. -/
def ownedSpawn : LedgerSpawn :=
  { worker := 1, captures := [.move], retainsResult := true, disposal := some .drop }

def ownedRow : Row :=
  { worker := 1, captures := [.move], peak := 1024, effort := 10
    creditBytes := 4096, creditEffort := 100, retained := some 64, join := some .drop }

/-- The copy-only round trip: one scalar copied in and nothing retained. -/
def copySpawn : LedgerSpawn :=
  { worker := 2, captures := [.copy], retainsResult := false, disposal := none }

def copyRow : Row :=
  { worker := 2, captures := [.copy], peak := 256, effort := 4
    creditBytes := 512, creditEffort := 20, retained := none, join := none }

theorem the_worked_ledger_entries_are_wellformed :
    ledgerWellformed ownedSpawn = true ∧ ledgerWellformed copySpawn = true := by
  decide

/-- Non-vacuity, in both shapes: these compositions really are agreed, so the
refusals below are not proved against a relation that refuses everything. -/
theorem the_owned_round_trip_is_agreed : agrees [ownedRow] [ownedSpawn] = true := by
  decide

theorem the_copy_only_round_trip_is_agreed : agrees [copyRow] [copySpawn] = true := by
  decide

theorem a_two_worker_composition_is_agreed :
    agrees [ownedRow, copyRow] [ownedSpawn, copySpawn] = true := by
  decide

/-- And the same two rows agree with the same two creations written in the other
order: the correspondence is per worker, not positional across workers. -/
theorem the_two_worker_composition_is_agreed_in_either_order :
    agrees [copyRow, ownedRow] [ownedSpawn, copySpawn] = true := by
  decide

-- ---------------------------------------------------------------------------
-- The per-pair half, by measurement on the worked compositions.
-- ---------------------------------------------------------------------------

theorem an_undeclared_creation_is_refused : agrees [] [ownedSpawn] = false := by decide

theorem an_over_declared_row_is_refused : agrees [ownedRow] [] = false := by decide

theorem a_second_row_for_one_creation_is_refused :
    agrees [ownedRow, ownedRow] [ownedSpawn] = false := by decide

theorem a_second_creation_for_one_row_is_refused :
    agrees [ownedRow] [ownedSpawn, ownedSpawn] = false := by decide

theorem a_missing_capture_is_refused :
    agrees [{ ownedRow with captures := [] }] [ownedSpawn] = false := by decide

theorem an_extra_capture_is_refused :
    agrees [{ ownedRow with captures := [.move, .move] }] [ownedSpawn] = false := by decide

theorem a_wrong_capture_mode_is_refused :
    agrees [{ ownedRow with captures := [.copy] }] [ownedSpawn] = false := by decide

theorem a_declared_result_the_creation_does_not_retain_is_refused :
    agrees [{ copyRow with retained := some 8 }] [copySpawn] = false := by decide

theorem a_retained_creation_with_no_declared_result_is_refused :
    agrees [{ ownedRow with retained := none, join := none }] [ownedSpawn] = false := by decide

theorem a_diverging_disposal_is_refused :
    agrees [{ ownedRow with join := some .move }] [ownedSpawn] = false := by decide

theorem a_completion_disposal_is_refused :
    agrees [{ ownedRow with join := some .close }] [ownedSpawn] = false := by decide

theorem a_disposal_on_a_row_that_retains_nothing_is_refused :
    agrees [{ copyRow with join := some .drop }] [copySpawn] = false := by decide

/-- The derivation: an elided `join` agrees, and what it agrees with is the
region's own disposal. Declaring the same thing is admitted too, and declaring
anything else is not — so eliding is a derivation, not a hole. -/
theorem an_elided_join_agrees_and_is_the_regions_disposal :
    agrees [{ ownedRow with join := none }] [ownedSpawn] = true
      ∧ agrees [{ ownedRow with join := ownedSpawn.disposal }] [ownedSpawn] = true
      ∧ ∀ other : Disposal, other ≠ .drop →
          agrees [{ ownedRow with join := some other }] [ownedSpawn] = false := by
  refine ⟨by decide, by decide, ?_⟩
  intro other different
  cases other with
  | move => decide
  | drop => exact absurd rfl different
  | close => decide

-- ---------------------------------------------------------------------------
-- From agreement to the model bound.
-- ---------------------------------------------------------------------------

/-- The disposal the composition records: the region's own, and — for a creation
that retains nothing — the release of a zero result, which is what
`TaskResult::ReleasedInWorker` accounts for on the Rust side. -/
def derivedDisposal (spawn : LedgerSpawn) : Disposal :=
  match spawn.disposal with
  | some derived => derived
  | none => .drop

/-- One agreed pair becomes one worker of the declaration the existing lowering
already understands: the plan's figures, and the region's disposal. -/
def composeWorker (row : Row) (spawn : LedgerSpawn) : Worker :=
  { clauses :=
      [ .peak row.peak
      , .effort row.effort
      , .credit row.creditBytes row.creditEffort
      , .retained (row.retained.getD 0)
      , .join (derivedDisposal spawn) ] }

/-- Rows whose declared result fits the peak it was accounted for. This is the
core's own per-task rule; the composition does not weaken it. -/
def rowFits (row : Row) : Bool := row.retained.getD 0 ≤ row.peak

def composeDecl (allocationBytes allocationEffort perEpoch perTask : Nat)
    (pairs : List (Row × LedgerSpawn)) : Decl :=
  { cells := []
    allocationBytes := allocationBytes
    allocationEffort := allocationEffort
    perEpoch := perEpoch
    perTask := perTask
    basis := .declared
    latency := false
    workers := pairs.map (fun pair => composeWorker pair.1 pair.2) }

theorem compose_preserves_each_declared_figure (row : Row) (spawn : LedgerSpawn) :
    peakOf (composeWorker row spawn).clauses = row.peak
      ∧ effortClauseOf (composeWorker row spawn).clauses = row.effort
      ∧ creditOf (composeWorker row spawn).clauses = some (row.creditBytes, row.creditEffort)
      ∧ retainedOf (composeWorker row spawn).clauses = row.retained.getD 0 :=
  ⟨rfl, rfl, rfl, rfl⟩

/-- The composed epoch's task count is the region's creation count. No source
form declares it, so no plan can disagree with it. -/
theorem the_composed_task_count_is_the_regions
    (allocationBytes allocationEffort perEpoch perTask : Nat)
    (pairs : List (Row × LedgerSpawn)) :
    (lower (composeDecl allocationBytes allocationEffort perEpoch perTask pairs)).tasks.length
      = pairs.length := by
  simp [lower, composeDecl]

/-- A composed epoch has no shared storage at all, so its entry allocation is
zero: `T08-COMPOSE-SHARED` is a structural fact of this composition, not a
figure a plan could supply. -/
theorem a_composed_epoch_has_no_entry_allocation
    (allocationBytes allocationEffort perEpoch perTask : Nat)
    (pairs : List (Row × LedgerSpawn)) :
    (lower (composeDecl allocationBytes allocationEffort perEpoch perTask pairs)).cells = [] := by
  simp [lower, lowerCells, composeDecl]

theorem a_composed_worker_is_admitted {row : Row} {spawn : LedgerSpawn}
    (wellformed : ledgerWellformed spawn = true) (fits : rowFits row = true) :
    workerAdmits [] (composeWorker row spawn) = true := by
  have disposal : derivedDisposal spawn ≠ .close := by
    cases value : spawn.disposal with
    | none => simp [derivedDisposal, value]
    | some inner =>
      cases inner with
      | move => simp [derivedDisposal, value]
      | drop => simp [derivedDisposal, value]
      | close => simp [ledgerWellformed, value] at wellformed
  have bound : row.retained.getD 0 ≤ row.peak := by simpa [rowFits] using fits
  cases value : derivedDisposal spawn with
  | move => simp [workerAdmits, composeWorker, value, readsOf, distinct, countOf, isCredit,
      isPeak, isEffort, isRetained, isJoin, modeled, joinOf, retainedOf, peakOf, bound]
  | drop => simp [workerAdmits, composeWorker, value, readsOf, distinct, countOf, isCredit,
      isPeak, isEffort, isRetained, isJoin, modeled, joinOf, retainedOf, peakOf, bound]
  | close => exact absurd value disposal

/-- The payoff. A nonempty composition of pairs that are individually
wellformed and fitting builds a declaration the *existing* admission predicate
accepts, so every theorem this file already proved about admitted declarations
applies to a composed epoch unchanged. -/
theorem an_agreed_composition_is_admitted
    {allocationBytes allocationEffort perEpoch perTask : Nat}
    {pairs : List (Row × LedgerSpawn)} (nonempty : pairs ≠ [])
    (wellformed : ∀ pair ∈ pairs, ledgerWellformed pair.2 = true)
    (fitting : ∀ pair ∈ pairs, rowFits pair.1 = true) :
    Admitted (composeDecl allocationBytes allocationEffort perEpoch perTask pairs) := by
  refine (Bool.and_eq_true _ _).mpr ⟨(Bool.and_eq_true _ _).mpr ⟨rfl, ?_⟩, ?_⟩
  · cases value : pairs with
    | nil => exact absurd value nonempty
    | cons _ _ => simp [composeDecl]
  · refine List.all_eq_true.mpr ?_
    intro worker member
    have inside : worker ∈ (composeDecl allocationBytes allocationEffort perEpoch perTask
        pairs).workers := member
    simp only [composeDecl, List.mem_map] at inside
    obtain ⟨pair, pair_member, composed⟩ := inside
    exact composed ▸ a_composed_worker_is_admitted (wellformed pair pair_member)
      (fitting pair pair_member)

/-- And therefore no configuration of a composed epoch exceeds its bound. The
composition adds a relation to the region; it does not add a way past the
accounting. -/
theorem every_configuration_of_a_composed_epoch_is_bounded
    {allocationBytes allocationEffort perEpoch perTask : Nat}
    {pairs : List (Row × LedgerSpawn)} (nonempty : pairs ≠ [])
    (wellformed : ∀ pair ∈ pairs, ledgerWellformed pair.2 = true)
    (fitting : ∀ pair ∈ pairs, rowFits pair.1 = true) (phases : List Phase) :
    QuantVector.Le
      (residency (lower (composeDecl allocationBytes allocationEffort perEpoch perTask pairs))
        phases)
      (epochPeak (lower (composeDecl allocationBytes allocationEffort perEpoch perTask pairs))) :=
  every_configuration_of_an_admitted_declaration_is_bounded
    (an_agreed_composition_is_admitted nonempty wellformed fitting) phases

/-- The worked owned composition, end to end: agreed, admitted, and bounded. -/
theorem the_owned_composition_is_admitted :
    Admitted (composeDecl 65536 1000 512 128 [(ownedRow, ownedSpawn)]) := by
  decide

end QCore.EpochSourceLowering
