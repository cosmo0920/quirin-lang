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

import QCore.ContractDependencies

/-!
# Per-clause body standing (card VERDICT-BODY-STANDING-CLAUSE-SCOPE, rules 13i–13k)

The abstract rules implemented by
`crates/qcore-c-gateway/src/contract_clause_standing.rs` (the clauses of a
declaration and what each piece of body evidence covers) and
`crates/qcore-audit/src/contract_dependencies.rs` (`clause_standing_of`, the
run's per-clause standing and the Safe roll-up), contract specification C06.5.

* **Coverage (rule 13i).** An equal W4 summary and the `entry` check compare
  the custody projection: every parameter and result clause. A structural
  check (rules 11r, 12w, 13a, 13c, 13d) covers only the clause kinds it reads.
  Nothing covers a container or counted-user clause, a retention, a holder
  reset, a named premise, a variadic tail, a no-return clause or an unreadable
  text.
* **Standing (rule 13j).** Under the previous card's evidence standing
  (`QCore.ContractDependencies.standing`: every examining unit validated, one
  at least), a declaration is `validated` only when it has a clause and every
  clause is covered in every unit; `partlyValidated` when some clause is;
  otherwise a `declaredAssumption`; `held` passes through.
* **Roll-up (rule 13k).** A Safe row rests only on validated declarations when
  every declaration it rests on is validated under this standing; that set of
  rows is a subset of the previous card's.

Theorems:
* coverage: `a_structural_check_covers_only_what_it_reads`,
  `a_structural_check_never_covers_a_formal`, `nothing_covers_these_clauses`;
* standing: `validated_needs_every_clause`,
  `an_uncovered_clause_is_never_validated`,
  `a_partly_validated_declaration_has_an_uncovered_clause`,
  `a_declaration_with_a_premise_is_never_validated`,
  `structural_evidence_alone_never_validates_a_formal`,
  `a_held_declaration_is_never_validated`,
  `validated_implies_previously_validated`;
* roll-up: `the_validated_only_rollup_is_a_subset`;
* the shared vectors: `coverage_vectors_hold`, `clause_standing_vectors_hold`
  (bound to the Rust by `crates/qcore-c-gateway/tests/contract_clause_standing.rs`
  and `crates/qcore-audit/tests/contract_dependencies_report.rs`).

## What this is not

A model over clause kinds. That the gateway's checks read exactly the clause
kinds this table assigns them (the W4 comparison is the custody projection's
equality; rule 13a's hook reads slot moves, derived out slots and the
function map's borrowed results, and so on) is a review premise (trust
boundary 28), pinned by the Rust tests named above.
-/

namespace QCore.BodyClauseStanding

open QCore.ContractDependencies (Outcome unitOutcome)

/-- What reads a clause. -/
inductive ClauseKind where
  | parameter | result | borrowedResult | referenceResult | container | countedUser
  | retention | guard | holderReset | slotMove | outDerived | reference | exitClass
  | premise | variadicTail | noReturn | callback | callbackChecked | arm | memberState
  | unreadable | lifetimeProtocol
  deriving DecidableEq, Repr

def allKinds : List ClauseKind :=
  [.parameter, .result, .borrowedResult, .referenceResult, .container, .countedUser,
   .retention, .guard, .holderReset, .slotMove, .outDerived, .reference, .exitClass,
   .premise, .variadicTail, .noReturn, .callback, .callbackChecked, .arm, .memberState,
   .unreadable, .lifetimeProtocol]

/-- The clauses of the custody projection. -/
def ClauseKind.custody : ClauseKind → Bool
  | .parameter | .result | .borrowedResult | .referenceResult => true
  | _ => false

/-- The body evidence of rule 13g; card DECLARED-ASSUMPTION-BODY-CHECKS adds
`r13m`, the declaration's own `entry` check on a body whose W4 summary only
refines it (`QCore.DeclaredEntryCheck`); card CG22-CALLBACK-REGISTRY-CONTRACT
adds `r13y`, the callback body check (`QCore.CallbackContracts`); card
CG22-OP-CALLBACKS-AND-SERVED-QUEUES adds `r12o`, the arm check of a
type-switched release (`QCore.OpCallbacks`); card OP-CONSTRUCTOR-DECLARATIONS
adds `r14e`, the result-state check of a constructor's body
(`QCore.OpCallbacks`); card OP-CALLBACK-MEMBER-PATH-RELEASE adds `r30`, the
member-path check of a callback candidate's body (`QCore.MemberPathRelease`). -/
inductive Evidence where
  | w4 | entry | r11r | r12w | r13a | r13c | r13d | r13m | r13y | r12o | r14e | r30
  deriving DecidableEq, Repr

/-- Whether a piece of evidence covers a clause kind (rule 13i). -/
def covers : Evidence → ClauseKind → Bool
  | .w4, k => k.custody
  | .entry, k => k.custody
  | .r13m, k => k.custody
  | .r11r, k => k == .exitClass
  | .r12w, k => k == .guard
  | .r13a, k => k == .slotMove || k == .outDerived || k == .borrowedResult
  | .r13c, k => k == .reference
  | .r13d, k => k == .referenceResult
  | .r13y, k => k == .callbackChecked
  | .r12o, k => k == .arm
  | .r14e, k => k == .memberState
  | .r30, k => k == .memberState

/-- The structural checks. -/
def Evidence.structural : Evidence → Bool
  | .w4 | .entry | .r13m => false
  | _ => true

/-- The clause kinds a structural check reads. -/
def reads : Evidence → List ClauseKind
  | .w4 | .entry | .r13m => allKinds.filter ClauseKind.custody
  | .r11r => [.exitClass]
  | .r12w => [.guard]
  | .r13a => [.slotMove, .outDerived, .borrowedResult]
  | .r13c => [.reference]
  | .r13d => [.referenceResult]
  | .r13y => [.callbackChecked]
  | .r12o => [.arm]
  | .r14e => [.memberState]
  | .r30 => [.memberState]

/-- A structural check's evidence never covers a clause of another kind. -/
theorem a_structural_check_covers_only_what_it_reads (e : Evidence) (k : ClauseKind)
    (c : covers e k = true) : k ∈ reads e := by
  cases e <;> cases k <;> simp_all [covers, reads, allKinds, ClauseKind.custody]

/-- In particular a structural check never covers a formal's mode: the
`resource_borrowed` formals beside the clause it reads stay uncovered. -/
theorem a_structural_check_never_covers_a_formal (e : Evidence)
    (s : e.structural = true) : covers e .parameter = false := by
  cases e <;> simp_all [Evidence.structural, covers]

/-- No evidence covers these clauses. -/
theorem legacy_evidence_never_validates_a_lifetime_protocol (e : Evidence) :
    covers e .lifetimeProtocol = false := by
  cases e <;> rfl

theorem nothing_covers_these_clauses (e : Evidence) :
    covers e .container = false ∧ covers e .countedUser = false ∧
      covers e .retention = false ∧ covers e .holderReset = false ∧
      covers e .premise = false ∧ covers e .variadicTail = false ∧
      covers e .noReturn = false ∧ covers e .callback = false ∧
      covers e .unreadable = false := by
  cases e <;> decide

/-! ## Standing (rule 13j) -/

/-- One unit that examined a body of the declared function: whether a check of
it failed there, and the evidence that validated it there. -/
structure UnitView where
  failed : Bool
  evidence : List Evidence

/-- The previous card's outcome of the unit. -/
def UnitView.outcome (u : UnitView) : Outcome :=
  unitOutcome (!u.evidence.isEmpty) u.failed

/-- The unit's evidence covers a clause of kind `k`. -/
def coveredIn (k : ClauseKind) (u : UnitView) : Bool :=
  !u.failed && u.evidence.any (covers · k)

/-- A clause covered in every unit. -/
def everywhere (us : List UnitView) (k : ClauseKind) : Bool :=
  us.all (coveredIn k)

inductive Standing where
  | validated | partlyValidated | declaredAssumption | held
  deriving DecidableEq, Repr

/-- The previous card's evidence standing is `validated`. -/
def evidenceValidated (us : List UnitView) : Bool :=
  QCore.ContractDependencies.standing false (us.map UnitView.outcome) ==
    QCore.ContractDependencies.Standing.validated

/-- A declaration's standing for the run, from its clause kinds and the units
that examined its body. -/
def standing (held : Bool) (cs : List ClauseKind) (us : List UnitView) : Standing :=
  if held then .held
  else if !evidenceValidated us then .declaredAssumption
  else if !cs.isEmpty && cs.all (everywhere us) then .validated
  else if cs.any (everywhere us) then .partlyValidated
  else .declaredAssumption

theorem a_held_declaration_is_never_validated (cs : List ClauseKind) (us : List UnitView) :
    standing true cs us ≠ .validated := by
  simp [standing]

/-- `standing` is `validated` exactly when the declaration is not held, its
evidence standing is validated, and it has a clause and every clause is covered
in every unit. -/
theorem standing_eq_validated {h : Bool} {cs : List ClauseKind} {us : List UnitView} :
    standing h cs us = .validated ↔
      h = false ∧ evidenceValidated us = true ∧
        (!cs.isEmpty && cs.all (everywhere us)) = true := by
  unfold standing
  cases h <;> cases e : evidenceValidated us <;>
    cases a : (!cs.isEmpty && cs.all (everywhere us)) <;>
    cases b : cs.any (everywhere us) <;> simp

/-- `standing` is `partlyValidated` exactly when the evidence standing is
validated, not every clause is covered everywhere, and some clause is. -/
theorem standing_eq_partly {h : Bool} {cs : List ClauseKind} {us : List UnitView} :
    standing h cs us = .partlyValidated ↔
      h = false ∧ evidenceValidated us = true ∧
        (!cs.isEmpty && cs.all (everywhere us)) = false ∧ cs.any (everywhere us) = true := by
  unfold standing
  cases h <;> cases e : evidenceValidated us <;>
    cases a : (!cs.isEmpty && cs.all (everywhere us)) <;>
    cases b : cs.any (everywhere us) <;> simp

/-- A declaration is validated only if every clause is covered in every unit
that examined its body. -/
theorem validated_needs_every_clause {h : Bool} {cs : List ClauseKind} {us : List UnitView}
    (v : standing h cs us = .validated) :
    ∀ k ∈ cs, ∀ u ∈ us, coveredIn k u = true := by
  intro k hk u hu
  have a := (standing_eq_validated.mp v).2.2
  have all : cs.all (everywhere us) = true := by
    cases c : cs.all (everywhere us)
    · rw [c] at a; simp at a
    · rfl
  have ek := List.all_eq_true.mp all k hk
  exact List.all_eq_true.mp ek u hu

theorem an_uncovered_clause_is_never_validated {h : Bool} {cs : List ClauseKind}
    {us : List UnitView} {k : ClauseKind} {u : UnitView}
    (hk : k ∈ cs) (hu : u ∈ us) (uncovered : coveredIn k u = false) :
    standing h cs us ≠ .validated := by
  intro v
  have := validated_needs_every_clause v k hk u hu
  rw [uncovered] at this
  cases this

theorem a_partly_validated_declaration_has_an_uncovered_clause {h : Bool}
    {cs : List ClauseKind} {us : List UnitView}
    (p : standing h cs us = .partlyValidated) :
    ∃ k ∈ cs, ∃ u ∈ us, coveredIn k u = false := by
  obtain ⟨_, _, a, b⟩ := standing_eq_partly.mp p
  have ne : cs ≠ [] := by
    intro empty
    subst empty
    simp at b
  have notAll : ¬ (∀ k ∈ cs, everywhere us k = true) := by
    intro all
    rw [List.all_eq_true.mpr all] at a
    cases c : cs.isEmpty
    · rw [c] at a; simp at a
    · exact ne (List.isEmpty_iff.mp c)
  apply Classical.byContradiction
  intro none
  apply notAll
  intro k hk
  apply List.all_eq_true.mpr
  intro u hu
  cases c : coveredIn k u
  · exact absurd ⟨k, hk, u, hu, c⟩ none
  · rfl

/-- The evidence standing is a precondition of the clause standing. -/
theorem validated_needs_the_evidence_standing {h : Bool} {cs : List ClauseKind}
    {us : List UnitView} (v : standing h cs us = .validated) : evidenceValidated us = true :=
  (standing_eq_validated.mp v).2.1

/-- Every declaration validated under this card was validated under the
previous card's rule: the validated set only shrinks. -/
theorem validated_implies_previously_validated {h : Bool} {cs : List ClauseKind}
    {us : List UnitView} (v : standing h cs us = .validated) :
    QCore.ContractDependencies.standing h (us.map UnitView.outcome) =
      QCore.ContractDependencies.Standing.validated := by
  cases h
  · have e := validated_needs_the_evidence_standing v
    simpa [evidenceValidated] using e
  · exact absurd v (a_held_declaration_is_never_validated cs us)

/-- No evidence covers a named premise, so a premised declaration is never
validated. -/
theorem a_declaration_with_a_premise_is_never_validated {h : Bool} {cs : List ClauseKind}
    {us : List UnitView} (hk : .premise ∈ cs) : standing h cs us ≠ .validated := by
  intro v
  have e := validated_needs_the_evidence_standing v
  have ne : us ≠ [] := by
    intro empty
    subst empty
    simp [evidenceValidated, QCore.ContractDependencies.standing] at e
  obtain ⟨u, hu⟩ := List.exists_mem_of_ne_nil us ne
  have := validated_needs_every_clause v _ hk u hu
  simp [coveredIn] at this
  obtain ⟨_, e', _, c⟩ := this
  have := (nothing_covers_these_clauses e').2.2.2.2.1
  rw [this] at c
  cases c

/-- Structural evidence alone never validates a declaration with a formal. -/
theorem structural_evidence_alone_never_validates_a_formal {h : Bool}
    {cs : List ClauseKind} {us : List UnitView}
    (alone : ∀ u ∈ us, ∀ e ∈ u.evidence, e.structural = true)
    (hk : .parameter ∈ cs) : standing h cs us ≠ .validated := by
  intro v
  have e := validated_needs_the_evidence_standing v
  have ne : us ≠ [] := by
    intro empty
    subst empty
    simp [evidenceValidated, QCore.ContractDependencies.standing] at e
  obtain ⟨u, hu⟩ := List.exists_mem_of_ne_nil us ne
  have := validated_needs_every_clause v _ hk u hu
  simp [coveredIn] at this
  obtain ⟨_, e', he', c⟩ := this
  rw [a_structural_check_never_covers_a_formal e' (alone u hu e' he')] at c
  cases c

/-! ## Roll-up (rule 13k) -/

/-- A declaration a Safe row rests on. -/
structure Decl where
  held : Bool
  clauses : List ClauseKind
  units : List UnitView

/-- The row rests only on validated declarations, under this card. -/
def onlyValidated (row : List Decl) : Prop :=
  row ≠ [] ∧ ∀ d ∈ row, standing d.held d.clauses d.units = .validated

/-- The same under the previous card. -/
def previouslyOnlyValidated (row : List Decl) : Prop :=
  row ≠ [] ∧ ∀ d ∈ row,
    QCore.ContractDependencies.standing d.held (d.units.map UnitView.outcome) =
      QCore.ContractDependencies.Standing.validated

theorem the_validated_only_rollup_is_a_subset {row : List Decl} (only : onlyValidated row) :
    previouslyOnlyValidated row :=
  ⟨only.1, fun d hd => validated_implies_previously_validated (only.2 d hd)⟩

/-! ## Shared vectors

Bound to the Rust: `coverageVectors` against
`contract_clause_standing::covers` (every evidence spelling against every
clause kind) by `crates/qcore-c-gateway/tests/contract_clause_standing.rs`;
`clauseStandingVectors` against the audit's fold and `clause_standing_of` by
`crates/qcore-audit/tests/contract_dependencies_report.rs`. -/

def ClauseKind.spelling : ClauseKind → String
  | .parameter => "parameter"
  | .result => "result"
  | .borrowedResult => "borrowedResult"
  | .referenceResult => "referenceResult"
  | .container => "container"
  | .countedUser => "countedUser"
  | .retention => "retention"
  | .guard => "guard"
  | .holderReset => "holderReset"
  | .slotMove => "slotMove"
  | .outDerived => "outDerived"
  | .reference => "reference"
  | .exitClass => "exitClass"
  | .premise => "premise"
  | .variadicTail => "variadicTail"
  | .noReturn => "noReturn"
  | .callback => "callback"
  | .callbackChecked => "callbackChecked"
  | .arm => "arm"
  | .memberState => "memberState"
  | .unreadable => "unreadable"
  | .lifetimeProtocol => "lifetimeProtocol"

def Evidence.spelling : Evidence → String
  | .w4 => "W4"
  | .entry => "entry"
  | .r11r => "11r"
  | .r12w => "12w"
  | .r13a => "13a"
  | .r13c => "13c"
  | .r13d => "13d"
  | .r13m => "13m"
  | .r13y => "13y"
  | .r12o => "12o"
  | .r14e => "14e"
  | .r30 => "30"

/-- (evidence, the clause kinds it covers). -/
def coverageVectors : List (String × List String) := [
  ("W4", ["parameter", "result", "borrowedResult", "referenceResult"]),
  ("entry", ["parameter", "result", "borrowedResult", "referenceResult"]),
  ("11r", ["exitClass"]),
  ("12w", ["guard"]),
  ("13a", ["slotMove", "outDerived", "borrowedResult"]),
  ("13c", ["reference"]),
  ("13d", ["referenceResult"]),
  ("13m", ["parameter", "result", "borrowedResult", "referenceResult"]),
  ("13y", ["callbackChecked"]),
  ("12o", ["arm"]),
  ("14e", ["memberState"]),
  ("30", ["memberState"])
]

def allEvidence : List Evidence :=
  [.w4, .entry, .r11r, .r12w, .r13a, .r13c, .r13d, .r13m, .r13y, .r12o, .r14e, .r30]

theorem coverage_vectors_hold :
    (allEvidence.zip coverageVectors).all (fun (e, v) =>
      e.spelling == v.1 &&
        allKinds.all (fun k => covers e k == v.2.contains k.spelling)) = true := by
  decide

/-- A unit of a vector: `failed`, `none` (examined, no evidence) or the
evidence spellings joined by `+`. -/
def unitOf : String → UnitView
  | "failed" => ⟨true, []⟩
  | "none" => ⟨false, []⟩
  | "W4" => ⟨false, [.w4]⟩
  | "entry" => ⟨false, [.entry]⟩
  | "11r" => ⟨false, [.r11r]⟩
  | "12w" => ⟨false, [.r12w]⟩
  | "13a" => ⟨false, [.r13a]⟩
  | "13c" => ⟨false, [.r13c]⟩
  | "13d" => ⟨false, [.r13d]⟩
  | "W4+13a" => ⟨false, [.w4, .r13a]⟩
  | "W4+13d" => ⟨false, [.w4, .r13d]⟩
  | "entry+13d" => ⟨false, [.entry, .r13d]⟩
  | "13m" => ⟨false, [.r13m]⟩
  | "13m+13a" => ⟨false, [.r13m, .r13a]⟩
  | "13y" => ⟨false, [.r13y]⟩
  | "12o" => ⟨false, [.r12o]⟩
  | "14e" => ⟨false, [.r14e]⟩
  | "30" => ⟨false, [.r30]⟩
  | _ => ⟨true, []⟩

def kindOf (s : String) : ClauseKind :=
  (allKinds.find? (·.spelling == s)).getD .unreadable

def Standing.spelling : Standing → String
  | .validated => "validated"
  | .partlyValidated => "partly-validated"
  | .declaredAssumption => "declared-assumption"
  | .held => "held"

/-- (held, the declaration's clause kinds, its units, standing). -/
def clauseStandingVectors : List (Bool × List String × List String × String) := [
  (false, ["parameter", "result"], ["W4", "entry"], "validated"),
  (false, ["parameter", "reference", "result"], ["13c"], "partly-validated"),
  (false, ["parameter", "reference", "result"], ["13c", "13c"], "partly-validated"),
  (false, ["parameter", "borrowedResult"], ["W4+13a", "13a"], "partly-validated"),
  (false, ["parameter", "borrowedResult"], ["W4+13a", "W4"], "validated"),
  (false, ["parameter", "container"], ["W4"], "partly-validated"),
  (false, ["parameter", "result", "premise"], ["W4"], "partly-validated"),
  (false, ["parameter", "result"], ["W4", "none"], "declared-assumption"),
  (false, ["parameter", "result"], ["W4", "failed"], "declared-assumption"),
  (false, ["parameter", "result"], [], "declared-assumption"),
  (false, ["retention", "parameter"], ["12w"], "declared-assumption"),
  (false, ["exitClass", "exitClass"], ["11r"], "validated"),
  (false, ["parameter", "referenceResult"], ["entry+13d"], "validated"),
  (false, ["parameter", "referenceResult"], ["13d"], "partly-validated"),
  (true, ["parameter", "result"], ["W4"], "held"),
  (false, ["parameter", "result"], ["13m", "W4", "13m"], "validated"),
  (false, ["parameter", "result"], ["13m", "failed"], "declared-assumption"),
  (false, ["parameter", "borrowedResult"], ["13m+13a", "13m"], "validated"),
  (false, ["parameter", "container"], ["13m"], "partly-validated"),
  (false, ["parameter", "callbackChecked", "result"], ["13y"], "partly-validated"),
  (false, ["parameter", "callback", "result"], ["W4"], "partly-validated"),
  (false, ["parameter", "arm", "arm", "callback"], ["12o"], "partly-validated"),
  (false, ["parameter", "memberState", "memberState", "result"], ["14e"], "partly-validated"),
  (false, ["parameter", "memberState", "result"], ["W4"], "partly-validated")
]

theorem clause_standing_vectors_hold :
    clauseStandingVectors.all (fun v =>
      (standing v.1 (v.2.1.map kindOf) (v.2.2.1.map unitOf)).spelling == v.2.2.2) = true := by
  decide

end QCore.BodyClauseStanding
