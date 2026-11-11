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

import QCore.DerivedResultInference
import QCore.NullTriagePaths

/-!
Rule 36: exact original borrowed-formal returns. This models the new source
conjunction, flow classifier, exit observation and custody-preserving transfer.
`exposure` is the existing unresolved-effect cascade's complete exposure set;
this rule changes no cascade or resource status. Faithful AST/source extraction,
state interpretation and Rust correspondence remain premise 28. The caller
uses DerivedResultInference's existing application and invalidation theorems.
-/
namespace QCore.ExactFormalReturn
open DerivedResultInference

structure SourceFacts where
  pointerABI : Bool
  formalIndex : Bool
  originalReference : Bool
  stableFormal : Bool
  completeSpans : Bool
  matchingOccurrences : Bool
  uniqueValueBindings : Bool
  deriving DecidableEq, Repr

def sourceAdmits (f : SourceFacts) : Bool :=
  f.pointerABI && f.formalIndex && f.originalReference && f.stableFormal &&
    f.completeSpans && f.matchingOccurrences && f.uniqueValueBindings

theorem missing_source_fact_refuses (f : SourceFacts)
    (h : f.pointerABI = false ∨ f.formalIndex = false ∨ f.originalReference = false ∨
      f.stableFormal = false ∨ f.completeSpans = false ∨
      f.matchingOccurrences = false ∨ f.uniqueValueBindings = false) :
    sourceAdmits f = false := by
  rcases h with h | h | h | h | h | h | h <;> simp [sourceAdmits, h]

structure Pointer where
  targets : List Nat
  unknown : Bool := false
  identity : Bool := true
  mayNull : Bool := true
  deriving DecidableEq, Repr

inductive Flow where
  | null
  | exact (site : Nat)
  | refuse
  deriving DecidableEq, Repr

structure State (α : Type) where
  pointer : Option Pointer
  ledger : List (Nat × Nat)
  unresolved : List Nat := []
  unmodeled : Bool := false
  diagnostics : List Nat := []
  frame : α
  deriving DecidableEq, Repr

def classify (s : State α) (bases : List (Nat × Nat)) (parameter : Nat) : Flow :=
  match s.pointer with
  | none => .refuse
  | some p =>
    if p.unknown || !p.identity || !s.unresolved.isEmpty then .refuse
    else match p.targets with
      | [] => if p.mayNull then .null else .refuse
      | [site] =>
        if bases.lookup site == some parameter && !(s.ledger.any (fun x => x.1 == site))
        then .exact site else .refuse
      | _ => .refuse

def observe : Flow → Returned
  | .null => .null
  | .exact site => .derived site .exact false false
  | .refuse => .ambiguous

def transfer (s : State α) (bases : List (Nat × Nat)) (parameter : Nat)
    (exposure : List Nat) : State α :=
  match classify s bases parameter with
  | .refuse =>
      { s with
        unresolved := s.unresolved ++ exposure
        unmodeled := true
        diagnostics := 0 :: s.diagnostics }
  | _ => s

theorem admitted_transfer_preserves_every_field (s : State α) (bases : List (Nat × Nat))
    (parameter : Nat) (exposure : List Nat) (h : classify s bases parameter ≠ .refuse) :
    transfer s bases parameter exposure = s := by
  unfold transfer
  split <;> simp_all

theorem every_transfer_preserves_owed_resources (s : State α) (bases : List (Nat × Nat))
    (parameter : Nat) (exposure : List Nat) :
    (transfer s bases parameter exposure).ledger = s.ledger := by
  unfold transfer
  split <;> rfl

theorem every_transfer_preserves_pointer_and_frame (s : State α) (bases : List (Nat × Nat))
    (parameter : Nat) (exposure : List Nat) :
    (transfer s bases parameter exposure).pointer = s.pointer ∧
    (transfer s bases parameter exposure).frame = s.frame := by
  unfold transfer
  split <;> exact ⟨rfl, rfl⟩

theorem refusal_keeps_prior_obligations_and_adds_unknown (s : State α)
    (bases : List (Nat × Nat)) (parameter : Nat) (exposure : List Nat)
    (h : classify s bases parameter = .refuse) :
    (transfer s bases parameter exposure).unresolved = s.unresolved ++ exposure ∧
    (transfer s bases parameter exposure).unmodeled = true ∧
    (transfer s bases parameter exposure).diagnostics = 0 :: s.diagnostics := by
  simp [transfer, h]

/-- Relative exactness includes a NULL base and a base at any interior offset. -/
theorem exact_observation_denotes_original_value (base result : Val) (site : Nat) :
    denotes base (observe (.exact site)) result ↔ result = base := by
  simp [observe, denotes]

theorem null_base_is_never_promoted_to_nonnull (site : Nat) (result : Val)
    (h : denotes .null (observe (.exact site)) result) : result = .null := by
  exact (exact_observation_denotes_original_value .null result site).mp h

theorem interior_base_offset_is_preserved (object offset site : Nat) (result : Val)
    (h : denotes (.ptr object offset) (observe (.exact site)) result) :
    result = .ptr object offset := by
  exact (exact_observation_denotes_original_value (.ptr object offset) result site).mp h

theorem refused_exit_never_validates (spec : Spec) (base : Nat) (touched : Bool) :
    exitMatches spec base ⟨observe .refuse, touched⟩ = false := by
  simp [exitMatches, observe, returnedMatches]

theorem prior_unknown_cannot_be_cleared (spec : Spec) (base : Nat) (exits : List Exit) :
    validates spec base false exits = false := by
  rfl

/-- The path search's bounded reading: false is known NULL, true non-null;
none cuts the path as unresolved. No absolute non-null fact is guessed. -/
def witnessNull (isExact : Bool) : Option Bool → Option Bool
  | some true => some true
  | some false => if isExact then some false else none
  | none => none

theorem exact_witness_preserves_null : witnessNull true (some false) = some false := by
  rfl

theorem unknown_derived_base_cannot_prove_infeasible (isExact : Bool) :
    witnessNull isExact none = none ∧
    QCore.NullTriagePaths.conclude false false false = "unresolved" := by
  exact ⟨rfl, rfl⟩

inductive Result where
  | neutral
  | other
  | derived (spec : Spec)
  deriving DecidableEq, Repr

structure Contract where
  parameters : List Nat
  result : Result
  deriving DecidableEq, Repr

/-- A validated candidate refines only a neutral result and exactly the same
custody vector. `validated` stands for the unchanged fresh all-exit gate. -/
def refine (old : Contract) (candidate : Contract) (sourceChecked validated : Bool) : Contract :=
  if old.result = .neutral ∧ old.parameters = candidate.parameters ∧
      sourceChecked = true ∧ validated = true then candidate else old

theorem nonneutral_result_retains_precedence (old candidate : Contract)
    (sourceChecked validated : Bool) (h : old.result ≠ .neutral) :
    refine old candidate sourceChecked validated = old := by
  simp [refine, h]

theorem failed_validation_retains_old_summary (old candidate : Contract) (sourceChecked : Bool) :
    refine old candidate sourceChecked false = old := by
  simp [refine]

theorem refinement_preserves_parameter_custody (old candidate : Contract)
    (sourceChecked validated : Bool) :
    (refine old candidate sourceChecked validated).parameters = old.parameters := by
  unfold refine
  split
  next h =>
    exact h.2.1.symm
  next => rfl

-- Same fourteen concrete classifier cases as the Rust flow vector test.
def initial : State Unit :=
  { pointer := some { targets := [7] }, ledger := [(20, 1)], frame := () }
def bases : List (Nat × Nat) := [(7, 1)]
def flowVectors : List Flow :=
  [ classify initial bases 1,
    classify { initial with pointer := some { targets := [] } } bases 1,
    classify { initial with pointer := some { targets := [], mayNull := false } } bases 1,
    classify { initial with pointer := none } bases 1,
    classify { initial with pointer := some { targets := [7], unknown := true } } bases 1,
    classify { initial with pointer := some { targets := [7, 8] } } bases 1,
    classify { initial with pointer := some { targets := [8] } } bases 1,
    classify { initial with pointer := some { targets := [7], identity := false } } bases 1,
    classify { initial with pointer := some { targets := [7], identity := false } } bases 1,
    classify { initial with ledger := [(7, 1), (20, 1)] } bases 1,
    classify { initial with ledger := [(7, 2), (20, 1)] } bases 1,
    classify { initial with ledger := [(7, 4), (20, 1)] } bases 1,
    classify { initial with unresolved := [20] } bases 1,
    classify initial bases 0 ]

theorem shared_flow_vectors :
    flowVectors = [.exact 7, .null, .refuse, .refuse, .refuse, .refuse, .refuse,
      .refuse, .refuse, .refuse, .refuse, .refuse, .refuse, .refuse] := by
  decide

end QCore.ExactFormalReturn
