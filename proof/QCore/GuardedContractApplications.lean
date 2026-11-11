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

import QCore.GuardedDecomposition
import QCore.CjsonRepresentationPremises
import QCore.PluginPremiseDecomposition

/-! Mathematical propositions for the two actual-contract sidecar applications.
They remain proposed: no theorem binds these environments to imported C. -/
namespace QCore.GuardedContractApplications
namespace Cjson
open CjsonRepresentationPremises

def representation (e : Environment) : Prop :=
  chain e ∧ distinct e ∧ empty e ∧ firstPrev e ∧ lastNext e ∧
  linkTargets e ∧ noWriter e ∧ parentOutside e ∧ predecessor e

structure Call where
  entry : Environment
  after : Environment
  item : Nat
  nullParent : Bool
  nullItem : Bool

/-- The actual early-return guard has a partial check, not a membership oracle. -/
def rawFailure (c : Call) : Prop :=
  c.nullParent = true ∨ c.nullItem = true ∨
    (c.entry.child ≠ some c.item ∧ c.entry.prev c.item = none)

def invalidMembership (c : Call) : Prop :=
  c.nullParent = false ∧ c.nullItem = false ∧ ¬ c.entry.ownedChild c.item

def prerequisites (c : Call) : Prop :=
  representation c.entry ∧ c.entry.ownedChild c.item ∧
  c.nullParent = false ∧ c.nullItem = false

def remainingListValid (c : Call) : Prop :=
  representation c.after ∧ c.after.children = c.entry.children.erase c.item ∧
  c.after.next c.item = none ∧ c.after.prev c.item = none

def originalListUnchanged (c : Call) : Prop :=
  c.after.child = c.entry.child ∧ c.after.next = c.entry.next ∧
  c.after.prev = c.entry.prev

/-- Even a nonmember is not rejected by the raw guard if its prev is nonnull.
Thus the sidecar's forbidden obligation is a caller requirement, not an effect
or a complete membership test claimed of the C implementation. -/
theorem nonnull_prev_defeats_raw_rejection (c : Call)
    (hp : c.nullParent = false) (hi : c.nullItem = false)
    (prev : c.entry.prev c.item ≠ none) : ¬ rawFailure c := by
  simp [rawFailure, hp, hi, prev]

theorem admitted_membership_excludes_positive_invalidity (c : Call)
    (admitted : prerequisites c) : ¬ invalidMembership c := by
  intro invalid
  exact invalid.2.2 admitted.2.1
end Cjson

namespace Plugin
open PluginPremiseDecomposition

def registeredCase (e : Environment) (ev : Event) : Prop :=
  e.memberCandidate ev.member ev.function ∧
  ∃ reg ∈ e.trace, reg.action = .register ∧ reg.function = ev.function ∧
    reg.member = ev.member ∧ reg.time < ev.time ∧ reg.result = 0 ∧
    reg.publicSetter = true

def defaultCase (e : Environment) (ev : Event) : Prop :=
  e.memberCandidate ev.member ev.function ∧ e.staticDefault ev.member ev.function

def lifetimePostcondition (e : Environment) (ev : Event) : Prop :=
  e.codeLive ev.function ev.time ∧ e.contextLive ev.opaqueId ev.time ∧
  unloadOrder e ev.plugin

def unregistered (e : Environment) (ev : Event) : Prop :=
  ¬ registeredCase e ev ∧ ¬ defaultCase e ev

/-- The existing declared candidate proposition has both explicit alternatives;
neither a successful dlsym nor an observed callback supplies this antecedent. -/
theorem candidate_premise_decomposes_both_alternatives (e : Environment) (ev : Event)
    (candidates : callbackCandidates e) (member : ev ∈ e.trace)
    (invokes : ev.action = .invoke) : registeredCase e ev ∨ defaultCase e ev := by
  obtain ⟨candidate, alternatives⟩ := candidates ev member invokes
  rcases alternatives with default | registered
  · exact Or.inr ⟨candidate, default⟩
  · exact Or.inl ⟨candidate, registered⟩

/-- Lifetime is independently supplied, including for a default candidate. -/
theorem lifetime_requires_its_own_premise (e : Environment) (ev : Event)
    (live : codeContextLifetime e ev.plugin) (ordered : unloadOrder e ev.plugin)
    (member : ev ∈ e.trace) (invokes : ev.action = .invoke) :
    lifetimePostcondition e ev := by
  obtain ⟨code, context⟩ := live ev member rfl (Or.inl invokes)
  exact ⟨code, context, ordered⟩

end Plugin
end QCore.GuardedContractApplications
