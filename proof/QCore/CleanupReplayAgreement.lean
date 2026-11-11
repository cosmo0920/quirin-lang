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

namespace QCore.CleanupReplayAgreement

/-! Admission of a transported symbolic cleanup-replay proposal (card `P04-B2`).

Model of the **fourth** transported source vocabulary and of
`qcore_verification::cleanup_replay_source`'s `WitnessChecker::check` body.

Four things are modelled, and nothing else:

1. **The value lattice is closed.** It has exactly the nine constructors the
   trusted checker's own value domain has, with no catch-all: an enumeration of
   all nine is complete and duplicate-free, so a producer cannot report a value
   over a domain the core does not share.
2. **The step vocabulary is closed.** A bare value-free return, a structured
   loop, a nonreturning termination, an expression outside the closed list and
   a callee outside the profile's own three declared names each have **no**
   projection, and one refused step refuses the whole path.
3. **The sampled prestate domain is fixed by the family.** A proposal that
   answers a shorter, longer, reordered or foreign domain is refused, so a
   producer cannot answer a smaller question.
4. **Agreement discharges nothing.** Admission requires, in this order, that
   the transported profile *be* the contract's own declaration, that the
   prerequisite be re-established by the core's own re-derivation and
   re-evaluation, that the core's own replay succeed, and only then that the
   proposal equal what the core computed.

Everything the core cannot decide — whether a descriptor was released, a
mapping torn down or a file deleted, and the `close`/`munmap`/`unlink` ABI
bindings — stays a declared premise of the pinned contract and is outside every
statement here. A recorded call is a recorded call. -/

/-! ### The closed value lattice -/

/-- The symbolic value domain the replay evaluates over: exactly the nine
constructors of the trusted checker's own `Value`, in the same order. There is
deliberately no `unknown`/`other` constructor — an unresolved value is a
refusal, not an opaque token. -/
inductive Lattice where
  | null
  | root
  | pointer (field : Nat)
  | integer (value : Int)
  | symbol (field : Nat)
  | errnoPointer
  | errno
  | text
  | reportResult
  deriving DecidableEq, Repr

/-- The nine constructor tags, as a finite enumeration used to state closure. -/
inductive Tag where
  | null | root | pointer | integer | symbol | errnoPointer | errno | text | reportResult
  deriving DecidableEq, Repr

def tag : Lattice → Tag
  | .null => .null
  | .root => .root
  | .pointer _ => .pointer
  | .integer _ => .integer
  | .symbol _ => .symbol
  | .errnoPointer => .errnoPointer
  | .errno => .errno
  | .text => .text
  | .reportResult => .reportResult

def allTags : List Tag :=
  [.null, .root, .pointer, .integer, .symbol, .errnoPointer, .errno, .text, .reportResult]

/-- The lattice has exactly nine constructors. -/
theorem the_lattice_has_nine_constructors : allTags.length = 9 := by rfl

/-- No constructor is listed twice, so "nine" is nine distinct forms. -/
theorem the_nine_constructors_are_distinct : allTags.Nodup := by decide

/-- The enumeration is complete: every lattice value has one of the nine tags,
so there is no catch-all escape hatch a producer could answer through. -/
theorem the_lattice_enumeration_is_complete (v : Lattice) : tag v ∈ allTags := by
  cases v <;> simp [tag, allTags]

/-- Truth is decided only where the lattice decides it: a null pointer is
false, the record and a live field pointer are true, an integer is its own
truth, and every symbolic value is unresolved. -/
def truth : Lattice → Option Bool
  | .null => some false
  | .root => some true
  | .pointer _ => some true
  | .integer n => some (n ≠ 0)
  | _ => none

/-- The symbolic values have no truth value at all, so a guard over one refuses
the path rather than guessing a branch. -/
theorem symbolic_values_have_no_truth :
    truth .errnoPointer = none ∧ truth .errno = none ∧ truth .text = none ∧
      truth (.symbol 0) = none ∧ truth .reportResult = none := by
  refine ⟨rfl, rfl, rfl, rfl, rfl⟩

/-- Non-vacuity: the decided half really is decided, and the eligibility
distinction the replay turns on — a reset integer versus a live pointer — is
visible in the truth function. -/
theorem the_decided_truths_are_decided :
    truth .null = some false ∧ truth .root = some true ∧
      truth (.pointer 0) = some true ∧ truth (.integer 0) = some false ∧
      truth (.integer (-1)) = some true := by
  refine ⟨rfl, rfl, rfl, rfl, rfl⟩

/-! ### The closed step vocabulary -/

/-- A step the projection can represent: exactly the eight forms the replay
executes. -/
inductive Step where
  | read
  | write
  | localStart
  | localWrite
  | localEnd
  | guard
  | call (callee : Nat)
  | returnValue
  deriving DecidableEq, Repr

/-- What an importer may encounter. The last four constructors stand for the
forms the projector refuses **by name**: a value-free return (a cleanup path
without a terminal `ReturnValue` is not a complete path in either family), a
structured loop, a declared nonreturning termination, and an expression form
outside the closed expression vocabulary. -/
inductive SourceStep where
  | read
  | write
  | localStart
  | localWrite
  | localEnd
  | guard
  | call (callee : Nat)
  | returnValue
  | bareReturn
  | scanLoop
  | terminate
  | outsideExpressionVocabulary
  deriving DecidableEq, Repr

/-- The closed callee table is the profile's own three declared names — the
syscall, the errno accessor and the diagnostic helper — so it is
contract-derived rather than a caller assertion. -/
structure Callees where
  syscall : Nat
  errno : Nat
  report : Nat
  deriving DecidableEq, Repr

def admits (t : Callees) (c : Nat) : Bool :=
  c == t.syscall || c == t.errno || c == t.report

def projectStep (t : Callees) : SourceStep → Option Step
  | .read => some .read
  | .write => some .write
  | .localStart => some .localStart
  | .localWrite => some .localWrite
  | .localEnd => some .localEnd
  | .guard => some .guard
  | .call c => if admits t c then some (.call c) else none
  | .returnValue => some .returnValue
  | .bareReturn => none
  | .scanLoop => none
  | .terminate => none
  | .outsideExpressionVocabulary => none

/-- Project a whole path. One refused step refuses the whole projection; there
is no partial encoding and no fallback. -/
def project (t : Callees) : List SourceStep → Option (List Step)
  | [] => some []
  | s :: rest =>
    match projectStep t s, project t rest with
    | some o, some os => some (o :: os)
    | _, _ => none

theorem a_bare_return_has_no_projection (t : Callees) :
    projectStep t .bareReturn = none := rfl

theorem a_scan_loop_has_no_projection (t : Callees) :
    projectStep t .scanLoop = none := rfl

theorem a_termination_has_no_projection (t : Callees) :
    projectStep t .terminate = none := rfl

theorem an_expression_outside_the_vocabulary_has_no_projection (t : Callees) :
    projectStep t .outsideExpressionVocabulary = none := rfl

/-- A callee outside the profile's own three declared names has no projection,
so no general call vocabulary is opened. -/
theorem an_undeclared_callee_has_no_projection {t : Callees} {c : Nat}
    (outside : admits t c = false) : projectStep t (.call c) = none := by
  simp [projectStep, outside]

/-- One refused step refuses the whole path. -/
theorem a_refused_step_refuses_the_path {t : Callees} {s : SourceStep} {rest : List SourceStep}
    (refused : projectStep t s = none) : project t (s :: rest) = none := by
  simp [project, refused]

theorem a_refused_tail_refuses_the_path {t : Callees} {s : SourceStep} {rest : List SourceStep}
    (refused : project t rest = none) : project t (s :: rest) = none := by
  unfold project
  rw [refused]
  cases projectStep t s <;> rfl

/-- Non-vacuity: a real cleanup shape projects — a guarded field read, the
declared syscall, its stored result, the guard on it and the terminal return. -/
theorem a_real_cleanup_shape_projects :
    project ⟨7, 8, 9⟩ [.guard, .read, .call 7, .localWrite, .guard, .write, .returnValue] =
      some [.guard, .read, .call 7, .localWrite, .guard, .write, .returnValue] := by
  rfl

/-- …and the same shape with a bare return instead of a value return does not. -/
theorem the_same_shape_with_a_bare_return_does_not_project :
    project ⟨7, 8, 9⟩ [.guard, .read, .call 7, .localWrite, .guard, .write, .bareReturn] =
      none := by
  rfl

/-! ### The fixed sampled prestate domains -/

/-- The record's eligibility state at entry, for the native cleanup family. -/
inductive Entry where
  | isNull
  | absent
  | present
  deriving DecidableEq, Repr

/-- One sampled prestate. The inhabitants of each variant are fixed by the
family, so a producer cannot answer over a smaller or differently sampled
domain. -/
inductive Prestate where
  | nativeCleanupOutcome (entry : Entry) (syscallResult : Option Int)
  | guardedDelete (descriptorActive mappingActive : Bool) (syscallResult : Option Int)
  deriving DecidableEq, Repr

inductive Family where
  | nativeCleanupOutcome
  | guardedDelete
  deriving DecidableEq, Repr

/-- The complete sampled domain of each family, in the exact order the trusted
checker enumerates it. -/
def domain : Family → List Prestate
  | .nativeCleanupOutcome =>
    [ .nativeCleanupOutcome .isNull none,
      .nativeCleanupOutcome .absent none,
      .nativeCleanupOutcome .present (some (-1)),
      .nativeCleanupOutcome .present (some 0) ]
  | .guardedDelete =>
    [ .guardedDelete true false none,
      .guardedDelete true true none,
      .guardedDelete false true none,
      .guardedDelete false false (some (-1)),
      .guardedDelete false false (some 0) ]

theorem the_outcome_family_samples_four_prestates :
    (domain .nativeCleanupOutcome).length = 4 := by rfl

theorem the_delete_family_samples_five_prestates :
    (domain .guardedDelete).length = 5 := by rfl

theorem each_sampled_domain_is_duplicate_free :
    (domain .nativeCleanupOutcome).Nodup ∧ (domain .guardedDelete).Nodup := by
  refine ⟨by decide, by decide⟩

/-- The two families' domains are disjoint, so no case of one can be read as a
case of the other. -/
theorem the_two_domains_are_disjoint :
    ∀ p ∈ domain .nativeCleanupOutcome, p ∉ domain .guardedDelete := by decide

/-! ### Admission -/

/-- The record shape the replay evolves, as a transported claim. Only equality
with the contract's own declaration matters here. -/
structure Profile where
  content : Nat
  deriving DecidableEq, Repr

/-- One case of the proposed replay: the prestate it answers, the lattice state
it started from, and the figures the replay computed. -/
structure Case where
  prestate : Prestate
  initialFields : List Lattice
  selectedPath : Nat
  returned : Int
  syscallCalls : Nat
  fieldWrites : List Nat
  preserved : Bool
  reset : Bool
  deriving DecidableEq, Repr

/-- The proposed replay of one body over its family's complete sampled domain. -/
structure Replay where
  family : Family
  cases : List Case
  deriving DecidableEq, Repr

/-- The prestate vector a replay answers. -/
def answered (r : Replay) : List Prestate := r.cases.map Case.prestate

/-- A replay is well-formed only when it answers exactly its family's fixed
sampled domain, in order. -/
def answersItsDomain (r : Replay) : Bool := answered r == domain r.family

structure Task where
  digest : Nat
  deriving DecidableEq, Repr

structure Witness where
  taskDigest : Nat
  proposal : Replay
  deriving DecidableEq, Repr

structure Prerequisite where
  semanticProfile : Nat
  taskDigest : Nat
  obligationId : Nat
  deriving DecidableEq, Repr

/-- The identity the core derives for itself from its own prerequisite task. -/
def derivedIdentity (profile : Nat) (prereq : Task) (obligation : Nat) : Prerequisite :=
  { semanticProfile := profile, taskDigest := prereq.digest, obligationId := obligation }

/-- The whole admission context, all of it derived by the trusted core.

* `declared` is the profile the core reads out of its **own** pinned contract;
* `evaluate` is the core running its **own** unedited replay and its own
  contract evaluation over its own import — `none` is a refusal;
* `prerequisiteEvaluate` is the prerequisite's own evaluation, re-run by the
  core; `none` is a refusal.

Nothing a producer sends appears in this structure. -/
structure Context where
  own : Task
  declared : Profile
  declaredFamily : Family
  prerequisiteTask : Task
  prerequisiteProfile : Nat
  prerequisiteObligation : Nat
  evaluate : Task → Option Replay
  prerequisiteEvaluate : Task → Bool

/-- Re-establishing the prerequisite: the claimed triple must be the identity
the core derives, and the prerequisite's own evaluation must then succeed.
Nothing the claim carries is read as a conclusion. -/
def revalidate (c : Context) (claimed : Prerequisite) : Bool :=
  claimed == derivedIdentity c.prerequisiteProfile c.prerequisiteTask c.prerequisiteObligation &&
    c.prerequisiteEvaluate c.prerequisiteTask

/-- The implemented admission rule, in the order the Rust body checks it. -/
def admitsWitness (c : Context) (presented : Task) (transported : Profile) (family : Family)
    (claimed : Prerequisite) (w : Witness) : Bool :=
  presented == c.own &&
  transported == c.declared &&
  family == c.declaredFamily &&
  revalidate c claimed &&
  w.taskDigest == c.own.digest &&
  (match c.evaluate c.own with
   | none => false
   | some replay => w.proposal == replay && answersItsDomain replay)

/-! #### Fail-closed directions -/

/-- Agreement alone discharges nothing: when the core's own replay and contract
evaluation refuse, no witness whatsoever is admitted. -/
theorem a_refusing_core_admits_nothing {c : Context} {presented : Task} {p : Profile}
    {f : Family} {q : Prerequisite} {w : Witness} (refused : c.evaluate c.own = none) :
    admitsWitness c presented p f q w = false := by
  simp [admitsWitness, refused]

/-- A transported profile that is not the contract's own declaration is refused
before anything is replayed. -/
theorem a_profile_that_is_not_the_declaration_is_refused {c : Context} {presented : Task}
    {p : Profile} {f : Family} {q : Prerequisite} {w : Witness}
    (undeclared : p ≠ c.declared) :
    admitsWitness c presented p f q w = false := by
  simp [admitsWitness, undeclared]

/-- A family that is not the one the contract's eligibility vocabulary declares
is refused, in both directions. -/
theorem a_family_that_is_not_the_declared_one_is_refused {c : Context} {presented : Task}
    {p : Profile} {f : Family} {q : Prerequisite} {w : Witness}
    (undeclared : f ≠ c.declaredFamily) :
    admitsWitness c presented p f q w = false := by
  simp [admitsWitness, undeclared]

/-- A claimed prerequisite that is not the one the core derives is refused. -/
theorem a_foreign_prerequisite_is_refused {c : Context} {claimed : Prerequisite}
    (foreign :
      claimed ≠ derivedIdentity c.prerequisiteProfile c.prerequisiteTask
        c.prerequisiteObligation) :
    revalidate c claimed = false := by
  simp [revalidate, foreign]

/-- Re-validation always re-runs the prerequisite's own evaluation, so a
prerequisite whose contract is false can never be established, whatever
identity is claimed. -/
theorem a_refusing_prerequisite_is_never_established {c : Context} {claimed : Prerequisite}
    (refused : c.prerequisiteEvaluate c.prerequisiteTask = false) :
    revalidate c claimed = false := by
  simp [revalidate, refused]

/-- …and a dependent over a refusing prerequisite is therefore never admitted,
even when its own contract and its own reading are impeccable. -/
theorem a_dependent_over_a_refusing_prerequisite_is_refused {c : Context} {presented : Task}
    {p : Profile} {f : Family} {q : Prerequisite} {w : Witness}
    (refused : c.prerequisiteEvaluate c.prerequisiteTask = false) :
    admitsWitness c presented p f q w = false := by
  simp [admitsWitness, revalidate, refused]

/-- Every field of the prerequisite identity is load-bearing. -/
theorem every_prerequisite_field_is_load_bearing {c : Context} {claimed : Prerequisite}
    (established : revalidate c claimed = true) :
    claimed.semanticProfile = c.prerequisiteProfile ∧
      claimed.taskDigest = c.prerequisiteTask.digest ∧
      claimed.obligationId = c.prerequisiteObligation := by
  unfold revalidate at established
  simp only [Bool.and_eq_true, beq_iff_eq] at established
  rw [established.1]
  exact ⟨rfl, rfl, rfl⟩

/-- A prerequisite established for one task never establishes another. -/
theorem a_prerequisite_does_not_cross_tasks {profile : Nat} {a b : Task} {obligation : Nat}
    (different : a.digest ≠ b.digest) :
    derivedIdentity profile a obligation ≠ derivedIdentity profile b obligation := by
  simp [derivedIdentity, different]

/-- A witness bound to another task is refused before any replay. -/
theorem a_cross_task_witness_is_refused {c : Context} {presented : Task} {p : Profile}
    {f : Family} {q : Prerequisite} {w : Witness} (other : w.taskDigest ≠ c.own.digest) :
    admitsWitness c presented p f q w = false := by
  simp [admitsWitness, other]

/-- A task the checker did not derive is refused even with a real witness. -/
theorem a_foreign_task_is_refused {c : Context} {presented : Task} {p : Profile}
    {f : Family} {q : Prerequisite} {w : Witness} (foreign : presented ≠ c.own) :
    admitsWitness c presented p f q w = false := by
  simp [admitsWitness, foreign]

/-! #### What an admission means -/

/-- Everything an admission establishes, extracted once: the core ran its own
replay, the proposal *is* that replay, and that replay answers its family's
whole sampled domain. -/
theorem an_admission_is_the_cores_own_replay {c : Context} {presented : Task} {p : Profile}
    {f : Family} {q : Prerequisite} {w : Witness}
    (admitted : admitsWitness c presented p f q w = true) :
    ∃ replay, c.evaluate c.own = some replay ∧ w.proposal = replay ∧
      answersItsDomain replay = true := by
  unfold admitsWitness at admitted
  cases h : c.evaluate c.own with
  | none => rw [h] at admitted; simp at admitted
  | some replay =>
    rw [h] at admitted
    simp only [Bool.and_eq_true, beq_iff_eq] at admitted
    exact ⟨replay, rfl, admitted.2.1, admitted.2.2⟩

/-- Every admission is backed by a replay the core ran itself. -/
theorem an_admitted_witness_has_a_core_replay {c : Context} {presented : Task} {p : Profile}
    {f : Family} {q : Prerequisite} {w : Witness}
    (admitted : admitsWitness c presented p f q w = true) :
    ∃ replay, c.evaluate c.own = some replay := by
  obtain ⟨replay, evaluated, _, _⟩ := an_admission_is_the_cores_own_replay admitted
  exact ⟨replay, evaluated⟩

/-- An admitted proposal is exactly the replay the core computed; a producer
never contributes content, only agreement. -/
theorem only_the_core_replay_is_admitted {c : Context} {presented : Task} {p : Profile}
    {f : Family} {q : Prerequisite} {w : Witness} {replay : Replay}
    (evaluated : c.evaluate c.own = some replay)
    (admitted : admitsWitness c presented p f q w = true) :
    w.proposal = replay := by
  obtain ⟨other, evaluatedOther, same, _⟩ := an_admission_is_the_cores_own_replay admitted
  rw [evaluated] at evaluatedOther
  exact same.trans (Option.some.inj evaluatedOther.symm)

/-- Two admitted witnesses cannot disagree. -/
theorem admitted_witnesses_never_disagree {c : Context} {presented : Task} {p : Profile}
    {f : Family} {q : Prerequisite} {v w : Witness}
    (a : admitsWitness c presented p f q v = true)
    (b : admitsWitness c presented p f q w = true) :
    v.proposal = w.proposal := by
  obtain ⟨replay, evaluated⟩ := an_admitted_witness_has_a_core_replay a
  rw [only_the_core_replay_is_admitted evaluated a, only_the_core_replay_is_admitted evaluated b]

/-- Changing what the core computes retires the witness. -/
theorem a_changed_core_replay_retires_the_witness {c : Context} {presented : Task} {p : Profile}
    {f : Family} {q : Prerequisite} {w : Witness} {before after : Replay}
    (evaluated : c.evaluate c.own = some before)
    (admitted : admitsWitness c presented p f q w = true) (moved : after ≠ before) :
    w.proposal ≠ after := by
  rw [only_the_core_replay_is_admitted evaluated admitted]
  exact fun h => moved h.symm

/-- Every admitted proposal answers exactly its family's fixed sampled domain,
in order: a shorter, longer, reordered or foreign domain is never admitted. -/
theorem an_admitted_proposal_answers_its_whole_domain {c : Context} {presented : Task}
    {p : Profile} {f : Family} {q : Prerequisite} {w : Witness}
    (admitted : admitsWitness c presented p f q w = true) :
    answered w.proposal = domain w.proposal.family := by
  obtain ⟨replay, _, same, whole⟩ := an_admission_is_the_cores_own_replay admitted
  rw [same]
  simpa [answersItsDomain] using whole

/-- The case count follows, so a proposal cannot answer fewer prestates than
its family samples. -/
theorem an_admitted_proposal_has_its_domains_case_count {c : Context} {presented : Task}
    {p : Profile} {f : Family} {q : Prerequisite} {w : Witness}
    (admitted : admitsWitness c presented p f q w = true) :
    w.proposal.cases.length = (domain w.proposal.family).length := by
  have h := an_admitted_proposal_answers_its_whole_domain admitted
  have : (answered w.proposal).length = (domain w.proposal.family).length := by rw [h]
  simpa [answered] using this

/-- An admitted outcome-family proposal has exactly four cases. -/
theorem an_admitted_outcome_proposal_has_four_cases {c : Context} {presented : Task}
    {p : Profile} {f : Family} {q : Prerequisite} {w : Witness}
    (admitted : admitsWitness c presented p f q w = true)
    (outcome : w.proposal.family = .nativeCleanupOutcome) :
    w.proposal.cases.length = 4 := by
  rw [an_admitted_proposal_has_its_domains_case_count admitted, outcome]
  rfl

/-- An admitted delete-family proposal has exactly five. -/
theorem an_admitted_delete_proposal_has_five_cases {c : Context} {presented : Task}
    {p : Profile} {f : Family} {q : Prerequisite} {w : Witness}
    (admitted : admitsWitness c presented p f q w = true)
    (delete : w.proposal.family = .guardedDelete) :
    w.proposal.cases.length = 5 := by
  rw [an_admitted_proposal_has_its_domains_case_count admitted, delete]
  rfl

/-- A proposal whose case vector does not answer the family's domain is refused
even when every other check passes — this is the fail-closed direction of the
sampled-domain rule. -/
theorem a_proposal_over_another_domain_is_refused {c : Context} {presented : Task}
    {p : Profile} {f : Family} {q : Prerequisite} {w : Witness} {replay : Replay}
    (evaluated : c.evaluate c.own = some replay)
    (elsewhere : answered replay ≠ domain replay.family) :
    admitsWitness c presented p f q w = false := by
  unfold admitsWitness
  rw [evaluated]
  have : answersItsDomain replay = false := by
    simpa [answersItsDomain] using elsewhere
  simp [this]

/-! #### Non-vacuity -/

/-- The core's own replay, over its own declared profile and family, with the
identity it derives for its own prerequisite, is admitted. Without this the
fail-closed theorems above would hold of a route that admits nothing. -/
theorem the_core_replay_is_admitted {c : Context} {replay : Replay}
    (evaluated : c.evaluate c.own = some replay)
    (whole : answersItsDomain replay = true)
    (prerequisiteHolds : c.prerequisiteEvaluate c.prerequisiteTask = true) :
    admitsWitness c c.own c.declared c.declaredFamily
      (derivedIdentity c.prerequisiteProfile c.prerequisiteTask c.prerequisiteObligation)
      { taskDigest := c.own.digest, proposal := replay } = true := by
  simp [admitsWitness, revalidate, evaluated, whole, prerequisiteHolds]

/-- Non-vacuity of the domain rule itself: the outcome family's own domain
answers it, and the delete family's domain does not. -/
theorem the_domain_rule_is_not_vacuous :
    answersItsDomain
      { family := .nativeCleanupOutcome,
        cases := (domain .nativeCleanupOutcome).map (fun p =>
          { prestate := p, initialFields := [], selectedPath := 0, returned := 0,
            syscallCalls := 0, fieldWrites := [], preserved := true, reset := false }) } = true ∧
    answersItsDomain
      { family := .nativeCleanupOutcome,
        cases := (domain .guardedDelete).map (fun p =>
          { prestate := p, initialFields := [], selectedPath := 0, returned := 0,
            syscallCalls := 0, fieldWrites := [], preserved := true, reset := false }) } =
      false := by
  refine ⟨by decide, by decide⟩

end QCore.CleanupReplayAgreement
