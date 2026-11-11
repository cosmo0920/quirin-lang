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

import QCore.OuterContract

/-!
Runtime list candidate contracts (RK-PLUGIN-INTERCEPTOR-CANDIDATES).
`outer_candidates.rs` validates the supported inventory, exact digest citation
and reviewed source bytes before the outer envelope is in force. A candidate
is a restriction for a human reviewing deployed callbacks, never a static
closure proof. The lifecycle and neutral-custody predicates below state the
review obligation; the analyzer does not execute external callbacks or infer
these predicates from their signatures. Faithful source review, extraction and
Rust/model correspondence remain declared premises (rule 28).
-/
namespace QCore.OuterListCandidates

structure Identity where
  listKey : String
  elementRecord : String
  unionArm : String
  signature : String
  deriving DecidableEq

/-- Distinct lists stay distinct even if a C union shares their storage. -/
theorem different_lists_have_different_identities (a b : Identity)
    (h : a.listKey ≠ b.listKey) : a ≠ b := by
  intro heq
  exact h (congrArg Identity.listKey heq)

structure PinEvidence where
  supported : Bool
  digestBound : Bool
  sourceMatches : Bool
  deriving DecidableEq

def inForce (e : PinEvidence) : Bool := e.supported && e.digestBound && e.sourceMatches

theorem an_unbound_inventory_is_never_in_force (e : PinEvidence)
    (h : e.digestBound = false) : inForce e = false := by
  simp [inForce, h]

theorem changed_source_is_never_in_force (e : PinEvidence)
    (h : e.sourceMatches = false) : inForce e = false := by
  simp [inForce, h]

/-- Zero is the public setter's success; a failed/duplicate call adds no method.
A configuration setter has no client initialization check. Client setters may
be called only from on_new, before initialization, on a non-share client.
Neutral is the explicit human-reviewed custody restriction, not a C type fact. -/
def reviewScope (result : Int) (configuration onNew initialized share neutral liveThroughLastInvocation : Bool) : Prop :=
  result = 0 ∧ (configuration = true ∨ (onNew = true ∧ initialized = false ∧ share = false)) ∧ neutral = true ∧ liveThroughLastInvocation = true

theorem a_reviewed_registration_has_success (result : Int)
    (configuration onNew initialized share neutral liveThroughLastInvocation : Bool)
    (h : reviewScope result configuration onNew initialized share neutral liveThroughLastInvocation) : result = 0 := h.1

theorem a_client_registration_precedes_initialization (result : Int)
    (onNew initialized share neutral liveThroughLastInvocation : Bool)
    (h : reviewScope result false onNew initialized share neutral liveThroughLastInvocation) :
    onNew = true ∧ initialized = false ∧ share = false := by
  simpa using h.2.1

theorem an_unreviewed_callback_is_outside_the_candidate (result : Int)
    (configuration onNew initialized share liveThroughLastInvocation : Bool) :
    ¬ reviewScope result configuration onNew initialized share false liveThroughLastInvocation := by
  simp [reviewScope]

/-- Retaining a callback or opaque context cannot silently forget its lifetime. -/
theorem a_reviewed_registration_keeps_its_context_live (result : Int)
    (configuration onNew initialized share neutral liveThroughLastInvocation : Bool)
    (h : reviewScope result configuration onNew initialized share neutral liveThroughLastInvocation) :
    liveThroughLastInvocation = true := h.2.2.2

open QCore.OuterContract

/-- A valid inventory is still conditional on review, so its runtime contribution
is Pending. An unsupported, unbound or stale inventory cannot cover a fact. -/
def contribution (e : PinEvidence) : Verdict := if inForce e then .pending else .unknown

theorem a_candidate_never_contributes_safe (e : PinEvidence) : contribution e ≠ .safe := by
  simp only [contribution]
  split <;> decide

theorem an_in_force_candidate_awaits_review (e : PinEvidence)
    (h : inForce e = true) : contribution e = .pending := by
  simp [contribution, h]

theorem an_unsupported_candidate_is_unknown (e : PinEvidence)
    (h : e.supported = false) : contribution e = .unknown := by
  simp [contribution, inForce, h]

/-- Candidate loading does not participate in the static callback registry. -/
def staticProjection (_ : List Identity) (verdict : Verdict) : Verdict := verdict

theorem candidates_preserve_the_static_verdict (inventory : List Identity) (v : Verdict) :
    staticProjection inventory v = v := rfl

end QCore.OuterListCandidates
