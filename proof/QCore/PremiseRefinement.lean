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

import QCore.HypothesisGraph
import QCore.CjsonRepresentationPremises

/-! Neutral proposed-conjunction attachment. Reference identity, assumed
propositions and independently held contexts are deliberately separate. -/
namespace QCore.PremiseRefinement
open HypothesisGraph

structure Attachment where
  original : Premise
  proposed : List Atom

def erase (a : Attachment) : Premise := a.original

def report (ctx : Context) (legacy : List Atom) (a : Attachment) :
    Disposition × Disposition := (compose ctx legacy, compose ctx a.proposed)

theorem exact_erasure (a : Attachment) : erase a = a.original := rfl

theorem changing_refinement_preserves_legacy (ctx : Context) (legacy : List Atom)
    (a b : Attachment) : (report ctx legacy a).1 = (report ctx legacy b).1 := rfl

theorem refinement_support_requires_all (ctx : Context) (legacy : List Atom)
    (a : Attachment) : (report ctx legacy a).2 = .supported ↔
      supported ctx a.proposed = true :=
  composition_support_needs_every_atom ctx a.proposed

def boundReport (expected held : Context) (legacy : List Atom) (a : Attachment) :
    Option (Disposition × Disposition) :=
  if held = expected then some (report expected legacy a) else none

theorem foreign_context_refuses (expected held : Context) (legacy : List Atom)
    (a : Attachment) (h : held ≠ expected) : boundReport expected held legacy a = none := by
  simp [boundReport, h]

theorem supported_refinement_does_not_support_legacy (ctx : Context) (p : Premise) :
    report ctx [⟨0, ctx, .open, .declared⟩] ⟨p, [⟨1, ctx, .supported, .declared⟩]⟩ =
      (.open, .supported) := by
  simp [report, compose, supported] <;> decide

theorem refuted_refinement_does_not_refute_legacy (ctx : Context) (p : Premise) :
    report ctx [⟨0, ctx, .open, .declared⟩] ⟨p, [⟨1, ctx, .refuted, .declared⟩]⟩ =
      (.open, .refuted) := by
  simp [report, compose, supported] <;> decide

/-- Dependencies are the actual direct/summary declaration closure, independently
provided; matching a name alone cannot transport evidence across contexts. -/
def impacted (expected held : Context) (declaration : String) (dependencies : List String) : Bool :=
  decide (expected = held ∧ declaration ∈ dependencies)

theorem impact_is_exact_context_and_dependency (expected held : Context)
    (declaration : String) (dependencies : List String) :
    impacted expected held declaration dependencies = true ↔
      expected = held ∧ declaration ∈ dependencies := by
  simp [impacted]

theorem unrelated_record_unchanged (expected held : Context)
    (declaration : String) (dependencies : List String)
    (h : declaration ∉ dependencies) : impacted expected held declaration dependencies = false := by
  simp [impacted, h]

/-- A model reference matching its expected digest is compatible with an
    environment that refutes noWriter. Reference integrity cannot establish
    the environmental antecedent or imported C correspondence. -/
structure BoundReference where
  modelDigest : String
  environment : CjsonRepresentationPremises.Environment

def referenceIntegrity (expected : String) (r : BoundReference) : Prop :=
  r.modelDigest = expected

theorem reference_identity_does_not_establish_no_writer (expected : String) :
    ∃ r : BoundReference, referenceIntegrity expected r ∧
      ¬ CjsonRepresentationPremises.noWriter r.environment := by
  let step : CjsonRepresentationPremises.Step := ⟨1, true⟩
  let e : CjsonRepresentationPremises.Environment :=
    ⟨0, 2, [], fun _ => False, fun _ => True, none, fun _ => none,
      fun _ => none, [step]⟩
  refine ⟨⟨expected, e⟩, rfl, ?_⟩
  exact CjsonRepresentationPremises.a_writing_step_refutes_no_writer e step
    (by simp [e]) (by decide) rfl

end QCore.PremiseRefinement
