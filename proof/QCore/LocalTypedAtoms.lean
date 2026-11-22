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

import QCore.QuantProposition
import QCore.LocalSourceBindings
import QCore.PluginSourceWitnesses

/-!
# Source-local hypotheses as typed atoms (card SP6-SYMBOLIC-HYPOTHESIS-ATOMS)

Two source-local checkers state their bounded hypotheses in their own
standalone graphs: `quirin-source-analysis::cjson_local_binding`
(`finite_detach_post`, `finite_entry_equations`,
`incomplete_fixed_case_domain`, bound to `QCore.LocalSourceBindings`) and
`quirin-source-analysis::plugin_source_witnesses` (four fixture-trace atoms,
bound to `QCore.PluginSourceWitnesses`). They were English sentences
(`resource_hypothesis(ID, "...")`, retired: `QF002`); each is now a typed atom
(C04.18.7). For each, an interpretation of its parameters by the bound model
(`finite`, `trace`) makes the atom hold exactly when the bound proposition
does (`*_atom_iff`). The Rust unit tests `typed_atoms` of
`crates/quirin-source-analysis/src/cjson_local_binding.rs` and
`crates/quirin-source-analysis/src/plugin_source_witnesses.rs` bind each
formula below to the checker's source.

Nothing here establishes a native execution, a source extraction or a
complete trace: the propositions stay bounded, source-local evidence.
-/

namespace QCore.LocalTypedAtoms

open QCore.QuantProposition
open QCore.CjsonRepresentationPremises
open QCore.LocalSourceBindings

/-! ## The cJSON native domain -/

/-- Recorded entry snapshots, post-field comparisons and case keys. -/
inductive FiniteCarrier where
  | snapshot (e : Environment)
  | comparison (b : Bool)
  | key (k : Nat × Option Nat)

def finiteSort (s : String) (v : SemanticValue FiniteCarrier) : Prop :=
  (s = "snapshot" ∧ ∃ e, v = .opaque (.snapshot e)) ∨
  (s = "comparison" ∧ ∃ b, v = .opaque (.comparison b)) ∨
  (s = "case_key" ∧ ∃ k, v = .opaque (.key k))

def finiteRelation (d : NativeDomain) (name : String) :
    List (SemanticValue FiniteCarrier) → Prop
  | [.opaque (.snapshot e)] =>
      (name = "recorded" ∧ e ∈ d.entries) ∨ (name = "detach_chain" ∧ chain e) ∨
      (name = "detach_distinct" ∧ distinct e) ∨ (name = "detach_empty" ∧ empty e) ∨
      (name = "detach_first_prev" ∧ firstPrev e) ∨ (name = "detach_last_next" ∧ lastNext e) ∨
      (name = "detach_link_targets" ∧ linkTargets e) ∨
      (name = "detach_parent_outside" ∧ parentOutside e) ∨
      (name = "detach_predecessor" ∧ predecessor e)
  | [.opaque (.comparison b)] =>
      (name = "post_comparison" ∧ b ∈ d.postPredicates) ∨ (name = "holds" ∧ b = true)
  | _ => False

def finiteFunction (d : NativeDomain) (name : String) :
    List (SemanticValue FiniteCarrier) → Option (SemanticValue FiniteCarrier)
  | [.opaque (.key k)] =>
      if name = "recorded_count" then some (.integer (d.caseKeys.count k))
      else if name = "required_count" then some (.integer (canonicalCases.count k))
      else none
  | _ => none

/-- Each parameter read off the native domain the checker binds. -/
def finite (d : NativeDomain) : Interpretation FiniteCarrier where
  sortHolds := finiteSort
  relation := finiteRelation d
  function := finiteFunction d

def noValues : String → SemanticValue FiniteCarrier := fun _ => .integer 0

def finiteDetachPostAtom : Expr := (.forall_ "c" "comparison" (.implies (.predicate "post_comparison" [(.var "c")]) (.predicate "holds" [(.var "c")])))

def finiteEntryEquationsAtom : Expr := (.forall_ "e" "snapshot" (.implies (.predicate "recorded" [(.var "e")]) (.and (.and (.and (.and (.and (.and (.and (.predicate "detach_chain" [(.var "e")]) (.predicate "detach_distinct" [(.var "e")])) (.predicate "detach_empty" [(.var "e")])) (.predicate "detach_first_prev" [(.var "e")])) (.predicate "detach_last_next" [(.var "e")])) (.predicate "detach_link_targets" [(.var "e")])) (.predicate "detach_parent_outside" [(.var "e")])) (.predicate "detach_predecessor" [(.var "e")]))))

def incompleteFixedCaseDomainAtom : Expr := (.exists_ "k" "case_key" (.not (.equal (.apply "recorded_count" [(.var "k")]) (.apply "required_count" [(.var "k")]))))

section Finite
variable (d : NativeDomain)

@[simp] theorem finite_relation : (finite d).relation = finiteRelation d := rfl
@[simp] theorem finite_function : (finite d).function = finiteFunction d := rfl

@[simp] theorem finite_sort_snapshot (v : SemanticValue FiniteCarrier) :
    sortMember (finite d) "snapshot" v ↔ ∃ e, v = .opaque (.snapshot e) := by
  simp [sortMember, finite, finiteSort]

@[simp] theorem finite_sort_comparison (v : SemanticValue FiniteCarrier) :
    sortMember (finite d) "comparison" v ↔ ∃ b, v = .opaque (.comparison b) := by
  simp [sortMember, finite, finiteSort]

@[simp] theorem finite_sort_key (v : SemanticValue FiniteCarrier) :
    sortMember (finite d) "case_key" v ↔ ∃ k, v = .opaque (.key k) := by
  simp [sortMember, finite, finiteSort]

theorem finite_detach_post_atom_iff :
    holds (finite d) noValues finiteDetachPostAtom ↔ finiteDetachPost d := by
  simp [finiteDetachPostAtom, holds, evaluate, finiteRelation, QuantProposition.bind,
    finiteDetachPost]

theorem finite_entry_equations_atom_iff :
    holds (finite d) noValues finiteEntryEquationsAtom ↔ finiteEntryEquations d := by
  simp [finiteEntryEquationsAtom, holds, evaluate, finiteRelation, QuantProposition.bind,
    finiteEntryEquations, and_assoc]

theorem incomplete_fixed_case_domain_atom_iff :
    holds (finite d) noValues incompleteFixedCaseDomainAtom ↔ incompleteFixedCaseDomain d := by
  simp [incompleteFixedCaseDomainAtom, holds, evaluate, finiteFunction, QuantProposition.bind,
    incompleteFixedCaseDomain, List.perm_iff_count]
  constructor
  · rintro ⟨_, ⟨a, b, rfl⟩, h⟩
    exact ⟨a, b, fun heq => h _ rfl (by simp [heq])⟩
  · rintro ⟨a, b, h⟩
    refine ⟨_, ⟨a, b, rfl⟩, ?_⟩
    intro x hx hy
    simp only [Option.some.injEq] at hx hy
    subst hx
    simp only [SemanticValue.integer.injEq, Int.natCast_inj] at hy
    exact h hy.symm

end Finite

/-! ## The plugin fixture traces -/

open QCore.PluginSourceWitnesses (DuplicateNoLoaderCalls DuplicateCompleted GoCreatorCompleted
  GoCreatorImpliesRegistration GoRegistrationSucceeded)

/-- A trace's markers. -/
inductive Marker where
  | marker (s : String)

/-- `#at(i)` is the trace's marker at index `i` (none outside it);
`#length()` its length. -/
def traceFunction (trace : List String) (name : String) :
    List (SemanticValue Marker) → Option (SemanticValue Marker)
  | [] => if name = "length" then some (.integer trace.length) else none
  | [.integer i] =>
      if name = "at" ∧ 0 ≤ i then (trace[i.toNat]?).map (fun s => .opaque (.marker s)) else none
  | _ => none

def traceInterp (trace : List String) : Interpretation Marker where
  sortHolds := fun _ _ => False
  relation := fun _ _ => False
  function := traceFunction trace

/-- The marker parameters, each the fixture marker it names. -/
def markers (name : String) : SemanticValue Marker :=
  if name = "begin" then .opaque (.marker "begin")
  else if name = "duplicate_test" then .opaque (.marker "duplicate_test")
  else if name = "return_zero" then .opaque (.marker "return:0")
  else if name = "return_nonnull" then .opaque (.marker "return:nonnull")
  else if name = "register_success" then .opaque (.marker "register:success")
  else if name = "end" then .opaque (.marker "end")
  else .integer 0

def duplicateNoLoaderCallsAtom : Expr := (.and (.and (.and (.and (.equal (.apply "length" []) (.integer 4)) (.equal (.apply "at" [(.integer 0)]) (.var "begin"))) (.equal (.apply "at" [(.integer 1)]) (.var "duplicate_test"))) (.equal (.apply "at" [(.integer 2)]) (.var "return_zero"))) (.equal (.apply "at" [(.integer 3)]) (.var "end")))

def duplicateCompletedAtom : Expr := (.and (.exists_ "i" "integer" (.and (.and (.compare .lessEqual (.integer 0) (.var "i")) (.compare .less (.var "i") (.apply "length" []))) (.equal (.apply "at" [(.var "i")]) (.var "return_zero")))) (.and (.compare .less (.integer 0) (.apply "length" [])) (.equal (.apply "at" [(.sub (.apply "length" []) (.integer 1))]) (.var "end"))))

def goCreatorCompletedAtom : Expr := (.and (.exists_ "i" "integer" (.and (.and (.compare .lessEqual (.integer 0) (.var "i")) (.compare .less (.var "i") (.apply "length" []))) (.equal (.apply "at" [(.var "i")]) (.var "return_nonnull")))) (.and (.compare .less (.integer 0) (.apply "length" [])) (.equal (.apply "at" [(.sub (.apply "length" []) (.integer 1))]) (.var "end"))))

def goCreatorImpliesRegistrationAtom : Expr := (.implies (.exists_ "i" "integer" (.and (.and (.compare .lessEqual (.integer 0) (.var "i")) (.compare .less (.var "i") (.apply "length" []))) (.equal (.apply "at" [(.var "i")]) (.var "return_nonnull")))) (.exists_ "j" "integer" (.and (.and (.compare .lessEqual (.integer 0) (.var "j")) (.compare .less (.var "j") (.apply "length" []))) (.equal (.apply "at" [(.var "j")]) (.var "register_success")))))

section Traces
variable (trace : List String)

@[simp] theorem exists_integer_value (P : SemanticValue Marker → Prop) :
    (∃ v, (∃ i, v = .integer i) ∧ P v) ↔ ∃ i : Int, P (.integer i) :=
  ⟨fun ⟨_, ⟨i, rfl⟩, h⟩ => ⟨i, h⟩, fun ⟨i, h⟩ => ⟨_, ⟨i, rfl⟩, h⟩⟩

/-- An in-range index naming a marker is membership. -/
theorem indexed_marker_iff (m : String) :
    (∃ i : Int, 0 ≤ i ∧ i < trace.length ∧ trace[i.toNat]? = some m) ↔ m ∈ trace := by
  constructor
  · rintro ⟨i, _, _, h⟩
    exact List.mem_of_getElem? h
  · intro h
    obtain ⟨n, hn, he⟩ := List.getElem_of_mem h
    refine ⟨n, by omega, by omega, ?_⟩
    simp only [Int.toNat_natCast]
    exact (List.getElem?_eq_getElem hn).trans (by rw [he])

/-- The last marker is the one at `length - 1`. -/
theorem last_marker_iff (m : String) :
    (0 < (trace.length : Int) ∧ trace[((trace.length : Int) - 1).toNat]? = some m) ↔
      trace.getLast? = some m := by
  rw [List.getLast?_eq_getElem?]
  constructor
  · rintro ⟨_, h⟩
    have : ((trace.length : Int) - 1).toNat = trace.length - 1 := by omega
    rwa [this] at h
  · intro h
    have hl : 0 < trace.length := by
      cases trace with
      | nil => simp at h
      | cons _ _ => simp
    have : ((trace.length : Int) - 1).toNat = trace.length - 1 := by omega
    exact ⟨by omega, by rwa [this]⟩

theorem duplicate_no_loader_calls_atom_iff :
    holds (traceInterp trace) markers duplicateNoLoaderCallsAtom ↔ DuplicateNoLoaderCalls trace := by
  simp [duplicateNoLoaderCallsAtom, holds, evaluate, traceInterp, traceFunction, markers,
    DuplicateNoLoaderCalls]
  constructor
  · rintro ⟨⟨⟨⟨hl, h0⟩, h1⟩, h2⟩, h3⟩
    have hl' : trace.length = 4 := by omega
    rcases trace with _ | ⟨a, _ | ⟨b, _ | ⟨c, _ | ⟨e, _ | ⟨f, t⟩⟩⟩⟩⟩
    all_goals simp at hl'
    simp at h0 h1 h2 h3
    subst h0 h1 h2 h3
    rfl
  · rintro rfl
    simp

theorem duplicate_completed_atom_iff :
    holds (traceInterp trace) markers duplicateCompletedAtom ↔ DuplicateCompleted trace := by
  rw [DuplicateCompleted, ← indexed_marker_iff, ← last_marker_iff]
  simp [duplicateCompletedAtom, holds, evaluate, traceInterp, traceFunction, markers,
    QuantProposition.bind, sortMember, ordered, integerBinary]
  constructor
  · rintro ⟨⟨i, ⟨h0, hl⟩, _, he⟩, h2, _, h3⟩
    exact ⟨⟨i, h0, hl, he⟩, h2, h3⟩
  · rintro ⟨⟨i, h0, hl, he⟩, h2, h3⟩
    exact ⟨⟨i, ⟨h0, hl⟩, h0, he⟩, h2, by omega, h3⟩

theorem go_creator_completed_atom_iff :
    holds (traceInterp trace) markers goCreatorCompletedAtom ↔ GoCreatorCompleted trace := by
  rw [GoCreatorCompleted, ← indexed_marker_iff, ← last_marker_iff]
  simp [goCreatorCompletedAtom, holds, evaluate, traceInterp, traceFunction, markers,
    QuantProposition.bind, sortMember, ordered, integerBinary]
  constructor
  · rintro ⟨⟨i, ⟨h0, hl⟩, _, he⟩, h2, _, h3⟩
    exact ⟨⟨i, h0, hl, he⟩, h2, h3⟩
  · rintro ⟨⟨i, h0, hl, he⟩, h2, h3⟩
    exact ⟨⟨i, ⟨h0, hl⟩, h0, he⟩, h2, by omega, h3⟩

theorem go_creator_implies_registration_atom_iff :
    holds (traceInterp trace) markers goCreatorImpliesRegistrationAtom ↔
      GoCreatorImpliesRegistration trace := by
  rw [GoCreatorImpliesRegistration, GoRegistrationSucceeded, ← indexed_marker_iff,
    ← indexed_marker_iff]
  simp [goCreatorImpliesRegistrationAtom, holds, evaluate, traceInterp, traceFunction, markers,
    QuantProposition.bind, sortMember, ordered]
  constructor
  · intro h i h0 hl he
    obtain ⟨j, ⟨j0, jl⟩, _, je⟩ := h i h0 hl h0 he
    exact ⟨j, j0, jl, je⟩
  · intro h i h0 hl _ he
    obtain ⟨j, j0, jl, je⟩ := h i h0 hl he
    exact ⟨j, ⟨j0, jl⟩, j0, je⟩

end Traces

end QCore.LocalTypedAtoms
