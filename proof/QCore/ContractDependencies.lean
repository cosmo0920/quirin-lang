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

/-!
# Verdict contract dependencies (card VERDICT-CONTRACT-DEPENDENCIES, rules 13f–13h)

The abstract rules implemented by `crates/qcore-c-gateway/src/contract_dependencies.rs`
(the per-row dependency set and the per-unit body evidence) and
`crates/qcore-audit/src/contract_dependencies.rs` (the row field and the run's
body standing), contract specification C06 and C04.18.

* **The set (rule 13f).** A function's own work consults names: the lookups its
  CFG builds and inferences made (bounded by the names its body spells), every
  callee its solved CFG applies a contract of, and its own name (its entry
  contract). A consulted name the unit gave an in-unit summary is not a
  declaration: the summary's own consultations are the consulting row's too,
  transitively. The row's dependency set is every **declared** name reached so.
* **Body standing (rule 13g).** One unit's outcome for a declaration whose body
  it examined is `failed` when a check of the body failed there, `validated`
  when positive evidence validated it there, `unvalidated` otherwise. For the
  run, a declaration is `validated` only when it has at least one outcome and
  every outcome is `validated`; one held for a human act is `held`; every other
  is a `declaredAssumption` (C06): no examined body, or one that did not
  validate it.
* **Evidence only (rule 13h).** The published row is the verdict row with the
  set beside it: the set is never an input of the verdict.

Theorems:
* completeness: `an_applied_call_is_a_dependency`,
  `a_consulted_declaration_is_a_dependency`, `the_entry_contract_is_a_dependency`;
* propagation: `a_summary_dependency_propagates`,
  `a_summarys_applied_call_is_a_dependency`;
* no verdict moves: `publishing_the_set_changes_no_verdict`,
  `two_sets_publish_the_same_verdict_row`;
* labelling: `a_declared_assumption_is_never_validated`,
  `no_examined_body_is_a_declared_assumption`,
  `an_unvalidated_body_is_never_validated`, `a_failed_body_is_never_validated`,
  `validated_needs_evidence_in_every_unit`, `a_failure_wins_in_its_unit`,
  `a_held_declaration_is_never_validated`;
* the shared vectors: `outcome_vectors_hold`, `standing_vectors_hold`,
  `closure_vectors_hold` (bound to the Rust by
  `crates/qcore-c-gateway/tests/contract_dependencies.rs`).

## What this is not

A model over abstract names. That the gateway's lookups, body spellings and CFG
instructions are what this calls `consulted`, `applied` and the own name, and
the correspondence with the compiled Rust, are review premises (trust boundary
28), pinned by the Rust tests named above.
-/

namespace QCore.ContractDependencies

/-! ## The set (rule 13f) -/

/-- One unit's record: per function, the names its work consulted and the
callees its solved CFG applies a contract of; the names given an in-unit
summary; and the declared names. -/
structure Unit where
  consulted : String → List String
  applied : String → List String
  inferred : List String
  declared : List String

/-- Every spelling a function's own work consulted: its own name (its entry
contract), its lookups and its CFG's applied callees. -/
def direct (u : Unit) (f : String) : List String :=
  f :: (u.consulted f ++ u.applied f)

/-- `g`'s summary is consulted by `f`'s work, transitively. -/
inductive Reaches (u : Unit) : String → String → Prop
  | refl (f : String) : Reaches u f f
  | step {f g h : String} : Reaches u f g → h ∈ direct u g → h ∈ u.inferred → Reaches u f h

/-- `d` is in `f`'s dependency set. -/
def InDeps (u : Unit) (f d : String) : Prop :=
  ∃ g, Reaches u f g ∧ d ∈ direct u g ∧ d ∈ u.declared

theorem reaches_trans {u : Unit} {f g h : String} (fg : Reaches u f g) (gh : Reaches u g h) :
    Reaches u f h := by
  induction gh with
  | refl => exact fg
  | step _ hd hi ih => exact Reaches.step ih hd hi

/-- A declared callee the row's solved CFG applies is in its set: the
completeness the verdict needs (every contract the solver applied). -/
theorem an_applied_call_is_a_dependency {u : Unit} {f d : String}
    (applied : d ∈ u.applied f) (declared : d ∈ u.declared) : InDeps u f d :=
  ⟨f, Reaches.refl f, List.mem_cons_of_mem _ (List.mem_append_right _ applied), declared⟩

/-- A declared name the row's own work looked up is in its set. -/
theorem a_consulted_declaration_is_a_dependency {u : Unit} {f d : String}
    (consulted : d ∈ u.consulted f) (declared : d ∈ u.declared) : InDeps u f d :=
  ⟨f, Reaches.refl f, List.mem_cons_of_mem _ (List.mem_append_left _ consulted), declared⟩

/-- A declared function's own declaration (its entry contract) is in its set. -/
theorem the_entry_contract_is_a_dependency {u : Unit} {f : String}
    (declared : f ∈ u.declared) : InDeps u f f :=
  ⟨f, Reaches.refl f, List.mem_cons_self, declared⟩

/-- A summary's dependencies are its consulting row's. -/
theorem a_summary_dependency_propagates {u : Unit} {f g d : String}
    (consults : g ∈ direct u f) (summary : g ∈ u.inferred) (dep : InDeps u g d) :
    InDeps u f d := by
  obtain ⟨h, gh, hd, hdecl⟩ := dep
  exact ⟨h, reaches_trans (Reaches.step (Reaches.refl f) consults summary) gh, hd, hdecl⟩

/-- In particular a declared callee a consulted summary's CFG applies. -/
theorem a_summarys_applied_call_is_a_dependency {u : Unit} {f g d : String}
    (consults : g ∈ direct u f) (summary : g ∈ u.inferred) (applied : d ∈ u.applied g)
    (declared : d ∈ u.declared) : InDeps u f d :=
  a_summary_dependency_propagates consults summary (an_applied_call_is_a_dependency applied declared)

/-! ## No verdict moves (rule 13h) -/

inductive Verdict where
  | safe | pending | unknown | provisionalUnsafe
  deriving DecidableEq, Repr

/-- A verdict row: what the analysis decided. -/
structure Row where
  status : Verdict
  diagnostics : List String
  deriving DecidableEq, Repr

/-- A published row: the verdict row with the dependency set beside it. -/
structure Published where
  row : Row
  dependencies : List String

/-- Publishing writes the set beside the row and reads nothing from it. -/
def publish (r : Row) (deps : List String) : Published := ⟨r, deps⟩

/-- The verdict-only projection. -/
def project (p : Published) : Row := p.row

theorem publishing_the_set_changes_no_verdict (r : Row) (deps : List String) :
    (project (publish r deps)).status = r.status ∧
      (project (publish r deps)).diagnostics = r.diagnostics :=
  ⟨rfl, rfl⟩

theorem two_sets_publish_the_same_verdict_row (r : Row) (a b : List String) :
    project (publish r a) = project (publish r b) := rfl

/-! ## Body standing (rule 13g) -/

/-- What one unit's bodies made of a declaration they define. -/
inductive Outcome where
  | validated | unvalidated | failed
  deriving DecidableEq, Repr

/-- One unit's outcome: a failure wins; validation needs evidence. -/
def unitOutcome (validated failed : Bool) : Outcome :=
  if failed then .failed else if validated then .validated else .unvalidated

/-- A declaration's standing for the run. -/
inductive Standing where
  | validated | declaredAssumption | held
  deriving DecidableEq, Repr

def standing (held : Bool) (outcomes : List Outcome) : Standing :=
  if held then .held
  else if outcomes ≠ [] ∧ outcomes.all (· == .validated) then .validated
  else .declaredAssumption

theorem a_failure_wins_in_its_unit (v : Bool) : unitOutcome v true = .failed := by
  simp [unitOutcome]

theorem no_examined_body_is_a_declared_assumption :
    standing false [] = .declaredAssumption := by
  simp [standing]

theorem a_held_declaration_is_never_validated (os : List Outcome) :
    standing true os ≠ .validated := by
  simp [standing]

theorem validated_needs_evidence_in_every_unit {h : Bool} {os : List Outcome}
    (v : standing h os = .validated) : os ≠ [] ∧ ∀ o ∈ os, o = .validated := by
  unfold standing at v
  cases h <;> simp at v
  obtain ⟨ne, all⟩ := v
  exact ⟨ne, fun o mem => by simpa using all o mem⟩

theorem an_unvalidated_body_is_never_validated {h : Bool} {os : List Outcome}
    (mem : .unvalidated ∈ os) : standing h os ≠ .validated := by
  intro v
  have := (validated_needs_evidence_in_every_unit v).2 _ mem
  cases this

theorem a_failed_body_is_never_validated {h : Bool} {os : List Outcome}
    (mem : .failed ∈ os) : standing h os ≠ .validated := by
  intro v
  have := (validated_needs_evidence_in_every_unit v).2 _ mem
  cases this

/-- A declared assumption — no examined body, or a body some unit did not
validate — is never labelled validated. -/
theorem a_declared_assumption_is_never_validated {h : Bool} {os : List Outcome}
    (assumption : os = [] ∨ (∃ o ∈ os, o ≠ .validated)) : standing h os ≠ .validated := by
  intro v
  have ⟨ne, all⟩ := validated_needs_evidence_in_every_unit v
  rcases assumption with empty | ⟨o, mem, other⟩
  · exact ne empty
  · exact other (all o mem)

/-! ## Shared vectors

Bound to the Rust by `crates/qcore-c-gateway/tests/contract_dependencies.rs`:
`outcomeVectors` against `UnitRecord::body_outcomes`, `standingVectors`
against the audit's run standing (`body_standing_of`), `closureVectors`
against `row_dependencies` over `exampleUnit`. -/

/-- (validated here, failed here, outcome). -/
def outcomeVectors : List (Bool × Bool × String) := [
  (false, false, "unvalidated"),
  (true, false, "validated"),
  (false, true, "failed"),
  (true, true, "failed")
]

def Outcome.spelling : Outcome → String
  | .validated => "validated"
  | .unvalidated => "unvalidated"
  | .failed => "failed"

theorem outcome_vectors_hold :
    outcomeVectors.all (fun v => (unitOutcome v.1 v.2.1).spelling == v.2.2) = true := by
  decide

/-- (held, validated units, unvalidated units, failed units, standing). -/
def standingVectors : List (Bool × Nat × Nat × Nat × String) := [
  (false, 0, 0, 0, "declared-assumption"),
  (false, 2, 0, 0, "validated"),
  (false, 2, 1, 0, "declared-assumption"),
  (false, 1, 0, 1, "declared-assumption"),
  (false, 0, 3, 0, "declared-assumption"),
  (true, 2, 0, 0, "held")
]

def Standing.spelling : Standing → String
  | .validated => "validated"
  | .declaredAssumption => "declared-assumption"
  | .held => "held"

def outcomesOf (v u f : Nat) : List Outcome :=
  List.replicate v .validated ++ List.replicate u .unvalidated ++ List.replicate f .failed

theorem standing_vectors_hold :
    standingVectors.all (fun v =>
      (standing v.1 (outcomesOf v.2.1 v.2.2.1 v.2.2.2.1)).spelling == v.2.2.2.2) = true := by
  decide

/-- The example unit of the closure vectors: `f` consults the summary `g` and
the declaration `free`; `g` consults the summary `h` and applies `close`; `h`
applies `fclose`; `k` is consulted by nobody. `local` is a name `f` looked up
that nothing declares. -/
def exampleUnit : Unit where
  consulted := fun
    | "f" => ["g", "free", "local"]
    | "g" => ["h"]
    | _ => []
  applied := fun
    | "g" => ["close"]
    | "h" => ["fclose"]
    | "k" => ["malloc"]
    | _ => []
  inferred := ["g", "h"]
  declared := ["free", "close", "fclose", "malloc"]

/-- The summaries `f` reaches, by bounded search (fuel = the unit's
summaries plus one). -/
def reach (u : Unit) : Nat → List String → List String
  | 0, seen => seen
  | n + 1, seen =>
    let next := seen.flatMap (direct u) |>.filter (fun g => g ∈ u.inferred && !(g ∈ seen))
    if next = [] then seen else reach u n (seen ++ next.eraseDups)

/-- The dependency set the bounded search computes. -/
def deps (u : Unit) (f : String) : List String :=
  ((reach u (u.inferred.length + 1) [f]).flatMap (direct u)).filter (· ∈ u.declared) |>.eraseDups

/-- (function, its dependency set). -/
def closureVectors : List (String × List String) := [
  ("f", ["close", "fclose", "free"]),
  ("g", ["close", "fclose"]),
  ("h", ["fclose"]),
  ("k", ["malloc"])
]

/-- The same names, each once. -/
def sameSet (xs ys : List String) : Bool :=
  xs.all (· ∈ ys) && ys.all (· ∈ xs) && xs.length == ys.length

theorem closure_vectors_hold :
    closureVectors.all (fun v => sameSet (deps exampleUnit v.1) v.2) = true := by
  decide

end QCore.ContractDependencies
