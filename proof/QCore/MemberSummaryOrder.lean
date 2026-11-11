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

import QCore.OpCallbacks

/-!
# A formal released through a decided member invocation (card RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN)

Gateway rule 12n, amended. A body that releases a **formal** through a
declaration whose member invocation (`resource_callback_member_invoked`) is
decided is bound to that formal only by its own W4 summary, so the member
decision is taken twice: once when the summary is derived, once in the
reported run. A tracked local needs no summary and reads the reported
decision alone. Before the card the derivation order (`derive_custody_summaries`,
`callback_route::callback_edges`) had no edge to the member's candidates,
which are stores elsewhere in the unit, so a caller could be derived before
its candidates: their class was `missing` there, the call refused, no summary
was derived, and the reported run read the release of the unbound formal as
`UnknownAlias` although it decided the member exactly as for a local
(`an_uninstalled_candidate_refuses_at_summary_time`).

The narrowing adds an edge to every function of the unit's closed member set
(`a_closed_member_candidate_precedes_its_caller`; an open set adds none,
`an_open_member_adds_no_edge`). With every candidate summarised first, the
derivation-time decision is the reported one and the formal reads as the
local (`a_formal_reads_as_the_local_once_candidates_precede`). It never
widens: the formal is never `Safe` where the local is not
(`the_formal_is_never_safe_where_the_local_is_not`), and a candidate that
could alias the formal — one whose own contract is not the identity, e.g. it
releases what it receives or a global, or has no contract — keeps the formal
non-`Safe` whatever the order (`a_candidate_that_could_alias_keeps_the_formal_unsafe`).

Abstract rule theorems, not extraction of the Rust analyzer
(`crates/qcore-c-gateway/src/legacy_callback_route.rs`, `callback_edges`);
the controls are `tests/cg22_op_callbacks.rs`.
-/

namespace QCore.MemberSummaryOrder

open QCore.CallbackContracts (Decision)
open QCore.OpCallbacks (Closure SuppliedClass memberDecision decideSupplied)

/-- The derivation edges one called member adds: every function of its
closed set, none for an open one. -/
def memberEdges : Closure → List String
  | .closed names => names
  | .opened => []

/-- A body's derivation edges: its direct callees (and rule 13u's named
candidates), then each called member's closed set. -/
def derivationEdges (direct : List String) (members : List Closure) : List String :=
  direct ++ members.flatMap memberEdges

/-- A candidate's class while a body's summary is derived: its own summary's
class once installed, `missing` before. -/
def classAt (installed : List String) (summary : String → SuppliedClass)
    (x : String) : SuppliedClass :=
  if installed.contains x then summary x else .missing

/-- A tracked local released through the call: the reported decision alone. -/
def localSafe (reported : Decision) : Bool := reported == .identity

/-- A formal released through the call: bound only by the body's summary,
which needs the derivation-time decision admitted, then the reported one. -/
def formalSafe (atSummary reported : Decision) : Bool :=
  atSummary == .identity && reported == .identity

/-- **A closed member's candidate is summarised before its caller.** -/
theorem a_closed_member_candidate_precedes_its_caller {direct names : List String}
    {members : List Closure} {x : String}
    (hm : Closure.closed names ∈ members) (hx : x ∈ names) :
    x ∈ derivationEdges direct members := by
  unfold derivationEdges
  exact List.mem_append_right _ (List.mem_flatMap.mpr ⟨_, hm, hx⟩)

/-- An open member adds no edge (its call refuses in every run). -/
theorem an_open_member_adds_no_edge : memberEdges .opened = ([] : List String) := rfl

/-- Once every candidate is installed, the derivation-time classes are the
reported ones. -/
theorem installed_classes_agree {installed names : List String}
    {summary : String → SuppliedClass}
    (h : ∀ x ∈ names, installed.contains x = true) :
    names.map (classAt installed summary) = names.map summary := by
  apply List.map_congr_left
  intro x hx
  unfold classAt
  rw [h x hx]
  rfl

/-- **The defect.** A candidate not yet summarised when the caller's summary
is derived refuses the call there, so a formal is never bound. -/
theorem an_uninstalled_candidate_refuses_at_summary_time
    {installed names : List String} {summary : String → SuppliedClass} {x : String}
    (hx : x ∈ names) (hn : installed.contains x = false) :
    memberDecision (.closed names) (classAt installed summary) = .refused := by
  have hm : (names.map (classAt installed summary)).any (· == .missing) = true :=
    List.any_eq_true.mpr ⟨.missing, List.mem_map.mpr ⟨x, hx, by unfold classAt; rw [hn]; rfl⟩, by decide⟩
  show decideSupplied false (names.map (classAt installed summary)) = .refused
  unfold decideSupplied
  simp [hm]

/-- **The narrowing.** With every candidate summarised first, the formal
reads exactly as the tracked local. -/
theorem a_formal_reads_as_the_local_once_candidates_precede
    {installed names : List String} {summary : String → SuppliedClass}
    (h : ∀ x ∈ names, installed.contains x = true) :
    formalSafe (memberDecision (.closed names) (classAt installed summary))
        (memberDecision (.closed names) summary)
      = localSafe (memberDecision (.closed names) summary) := by
  have e : memberDecision (.closed names) (classAt installed summary)
      = memberDecision (.closed names) summary := by
    show decideSupplied false (names.map (classAt installed summary))
      = decideSupplied false (names.map summary)
    rw [installed_classes_agree h]
  rw [e]
  generalize memberDecision (.closed names) summary = d
  cases d <;> rfl

/-- **Never wider.** A formal is `Safe` through the call only where a tracked
local is. -/
theorem the_formal_is_never_safe_where_the_local_is_not {a r : Decision}
    (h : formalSafe a r = true) : localSafe r = true := by
  revert h
  cases a <;> cases r <;> decide

/-- **A candidate that could alias the formal keeps it non-`Safe`.** A
candidate whose own contract is not the identity (it releases what it
receives or a global, keeps something, or has no contract) refuses the
reported decision, whatever the derivation order. -/
theorem a_candidate_that_could_alias_keeps_the_formal_unsafe (atSummary : Decision)
    {names : List String} {summary : String → SuppliedClass} {x : String}
    (hx : x ∈ names) (hi : summary x ≠ .identity) (ht : summary x ≠ .takesSupplied) :
    formalSafe atSummary (memberDecision (.closed names) summary) = false := by
  have hne : memberDecision (.closed names) summary ≠ .identity := by
    intro h
    have h' : decideSupplied false (names.map summary) = .identity := h
    rcases QCore.OpCallbacks.an_admitted_decision_takes_or_is_identity h' (summary x)
        (List.mem_map.mpr ⟨x, hx, rfl⟩) with e | e
    · exact hi e
    · exact ht e
  unfold formalSafe
  generalize memberDecision (.closed names) summary = d at hne ⊢
  cases d <;> simp_all

/-- The card's fixture: `handler` the identity, `freeing` (it releases what
it receives) not. -/
def fixtureClass (x : String) : SuppliedClass :=
  if x = "handler" then .identity else .effectful

/-- The fixture's rows: the formal before the order (not `Safe`), after it
(`Safe`), the tracked local (`Safe`), and the aliasing candidate (not `Safe`). -/
theorem fixture_vectors_hold :
    formalSafe (memberDecision (.closed ["handler"]) (classAt [] fixtureClass))
        (memberDecision (.closed ["handler"]) fixtureClass) = false ∧
    formalSafe (memberDecision (.closed ["handler"]) (classAt ["handler"] fixtureClass))
        (memberDecision (.closed ["handler"]) fixtureClass) = true ∧
    localSafe (memberDecision (.closed ["handler"]) fixtureClass) = true ∧
    formalSafe (memberDecision (.closed ["freeing"]) (classAt ["freeing"] fixtureClass))
        (memberDecision (.closed ["freeing"]) fixtureClass) = false := by
  decide

end QCore.MemberSummaryOrder
