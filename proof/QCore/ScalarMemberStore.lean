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

import QCore.AggregateStore

/-! # Bounded scalar member stores (card W42-INDIRECT-ACCESS)

This is the abstract rule implemented by
`typed_field_access::scalar_member_store_base` and
`LegacyInstruction::ScalarMemberStore`.  It models one exact
`BASE->FIELD = RHS` store.  Syntax and declarations establish that the member
and RHS carry no tracked identity; the flow check separately requires one live,
settled, non-null base with no unresolved effect and preserves the existing
may-alias obligation.

Faithful C token/type extraction and correspondence with the compiled Rust are
review premises under standing constraint 28.  This module proves properties
of the bounded rule, not extraction of arbitrary C.
-/

namespace QCore.ScalarMemberStore

/-- Declaration and syntax premises checked before lowering. -/
structure Candidate where
  exactShape : Bool
  readableDeclarations : Bool
  recordPointerBase : Bool
  trackedBase : Bool
  scalarField : Bool
  scalarOnlyRhs : Bool
  rhsHasMutation : Bool
  deriving DecidableEq, Repr

/-- Flow-sensitive premises checked at the instruction. -/
structure Flow where
  oneTarget : Bool
  known : Bool
  nonnull : Bool
  live : Bool
  unresolved : Bool
  aliasSafe : Bool
  deriving DecidableEq, Repr

def classifies (candidate : Candidate) : Bool :=
  candidate.exactShape && candidate.readableDeclarations &&
    candidate.recordPointerBase && candidate.trackedBase &&
    candidate.scalarField && candidate.scalarOnlyRhs && !candidate.rhsHasMutation

def resolves (flow : Flow) : Bool :=
  flow.oneTarget && flow.known && flow.nonnull && flow.live &&
    !flow.unresolved && flow.aliasSafe

def admits (candidate : Candidate) (flow : Flow) : Bool :=
  classifies candidate && resolves flow

/-- Only the maps relevant to the resource projection are represented.  An
exact scalar member store changes none of them, including when its flow gate
refuses and records an unknown-alias reason. -/
structure Projection where
  pointers : Nat
  custody : Nat
  fieldLinks : Nat
  scalarFacts : Nat
  deriving DecidableEq, Repr

structure TransferResult where
  projection : Projection
  admitted : Bool
  unknownAlias : Bool
  deriving DecidableEq, Repr

def transfer (candidate : Candidate) (flow : Flow) (state : Projection) : TransferResult :=
  { projection := state
    admitted := admits candidate flow
    unknownAlias := classifies candidate && !resolves flow }

theorem an_admitted_store_has_exact_shape (candidate : Candidate) (flow : Flow)
    (h : admits candidate flow = true) : candidate.exactShape = true := by
  simp_all [admits, classifies, resolves]

theorem an_admitted_store_has_a_scalar_field (candidate : Candidate) (flow : Flow)
    (h : admits candidate flow = true) : candidate.scalarField = true := by
  simp_all [admits, classifies, resolves]

theorem an_admitted_store_has_no_rhs_mutation (candidate : Candidate) (flow : Flow)
    (h : admits candidate flow = true) : candidate.rhsHasMutation = false := by
  simp_all [admits, classifies, resolves]

theorem a_pointer_field_refuses (candidate : Candidate) (flow : Flow)
    (h : candidate.scalarField = false) : admits candidate flow = false := by
  simp [admits, classifies, h]

theorem an_rhs_mutation_refuses (candidate : Candidate) (flow : Flow)
    (h : candidate.rhsHasMutation = true) : admits candidate flow = false := by
  simp [admits, classifies, h]

theorem an_ambiguous_base_refuses (candidate : Candidate) (flow : Flow)
    (h : flow.oneTarget = false) : admits candidate flow = false := by
  simp [admits, resolves, h]

theorem a_nullable_base_refuses (candidate : Candidate) (flow : Flow)
    (h : flow.nonnull = false) : admits candidate flow = false := by
  simp [admits, resolves, h]

theorem a_released_base_refuses (candidate : Candidate) (flow : Flow)
    (h : flow.live = false) : admits candidate flow = false := by
  simp [admits, resolves, h]

theorem an_unresolved_base_refuses (candidate : Candidate) (flow : Flow)
    (h : flow.unresolved = true) : admits candidate flow = false := by
  simp [admits, resolves, h]

theorem an_alias_obligation_refuses (candidate : Candidate) (flow : Flow)
    (h : flow.aliasSafe = false) : admits candidate flow = false := by
  simp [admits, resolves, h]

theorem a_successful_store_preserves_the_projection
    (candidate : Candidate) (flow : Flow) (state : Projection)
    (_h : admits candidate flow = true) :
    (transfer candidate flow state).projection = state := by
  rfl

theorem a_flow_refused_store_preserves_the_projection
    (candidate : Candidate) (flow : Flow) (state : Projection)
    (_hc : classifies candidate = true) (_hr : resolves flow = false) :
    (transfer candidate flow state).projection = state := by
  rfl

theorem a_flow_refused_store_records_unknown_alias
    (candidate : Candidate) (flow : Flow) (state : Projection)
    (hc : classifies candidate = true) (hr : resolves flow = false) :
    (transfer candidate flow state).unknownAlias = true := by
  simp [transfer, hc, hr]

theorem the_bounded_rule_is_not_vacuous :
    admits ⟨true, true, true, true, true, true, false⟩
      ⟨true, true, true, true, false, true⟩ = true := by
  decide

end QCore.ScalarMemberStore
