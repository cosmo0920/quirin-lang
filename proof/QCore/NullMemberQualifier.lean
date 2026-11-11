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
# One member clause, three kinds (card NONNULL-MEMBER-UNIFIED-QUALIFIER, rules 14t, 14x, 14z)

The abstract rule implemented by `crates/qcore-contract/src/bounds_contract.rs`
(`Reader::clause`, `member_qualifier`; the premise placement check of
`load_function`), contract specification C09.3 and grammar G05e/G05f, at the
user's decision of 2026-10-16 ("As shown above", "Migrate, one spelling", "Yes,
all kinds"):

* plain  `requires nonnull_member(TAG, FIELD);`
* stable `requires nonnull_member(TAG, FIELD) where quant #scope [[stable]];`
* scope  `requires nonnull_member(TAG, FIELD, P) where quant #scope [[scope]];`

`P` is required with `[[scope]]` and rejected otherwise; any other qualifier
word is rejected; the retired words `nonnull_member_stable` /
`nonnull_member_scope` are rejected with a diagnostic naming the new form. No
alias.

**Semantics are unchanged.** Each kind loads to exactly the fact its retired
spelling loaded to (`NonnullMember`, `NonnullMemberStable`,
`NonnullMemberScope`), and every admitted merged clause is the migration of
exactly one retired clause, so the merged surface and the three old forms
denote the same facts.

**The premise beside every kind.** A declaration's `resource_premise` is
admitted exactly when it states a member fact of any kind.

Theorems:
* `the_merged_surface_loads_the_old_fact` (migration preserves the fact);
* `every_admitted_clause_is_a_migrated_one` (nothing new is admitted);
* `migration_is_injective` (two old clauses never share a merged spelling);
* `the_merged_surface_is_equivalent_to_the_old_forms`;
* `a_retired_word_is_rejected_naming_the_new_form`;
* `a_parameter_needs_the_scope_qualifier`, `the_scope_qualifier_needs_a_parameter`,
  `an_unknown_qualifier_is_rejected`;
* `a_premise_stands_beside_every_kind`, `a_premise_needs_a_member_fact`;
* `qualifier_vectors_hold` (the shared examples of
  `crates/qcore-contract/tests/nonnull_member_qualifier.rs`).

## What this is not

A model over parsed tokens. That the Rust reader maps a token sequence to this
`Surface` (the head word, the argument count, the qualifier word inside
`where quant #scope [[…]]`) is a review premise (trust boundary 28), pinned by
the Rust tests named above.
-/

namespace QCore.NullMemberQualifier

/-- The member facts the null property reads (rules 14t, 14x, 14z). -/
inductive Fact where
  | member (tag field : String)
  | stable (tag field : String)
  | scope (tag field parameter : String)
deriving DecidableEq, Repr

/-- The three clauses before the merge, by their words. -/
inductive OldClause where
  /-- `nonnull_member(TAG, FIELD)` -/
  | nonnullMember (tag field : String)
  /-- `nonnull_member_stable(TAG, FIELD)` -/
  | nonnullMemberStable (tag field : String)
  /-- `nonnull_member_scope(TAG, FIELD, P)` -/
  | nonnullMemberScope (tag field parameter : String)
deriving DecidableEq, Repr

/-- The fact each old clause loaded to. -/
def OldClause.fact : OldClause → Fact
  | .nonnullMember t f => .member t f
  | .nonnullMemberStable t f => .stable t f
  | .nonnullMemberScope t f p => .scope t f p

/-- A clause's head word. -/
inductive Head where
  | nonnullMember
  | retiredStable
  | retiredScope
deriving DecidableEq, Repr

/-- The word inside `where quant #scope [[…]]`. -/
inductive Qualifier where
  | stable
  | scope
  | other (word : String)
deriving DecidableEq, Repr

/-- A clause as the reader sees it: head, arguments, qualifier. -/
structure Surface where
  head : Head
  args : List String
  qualifier : Option Qualifier
deriving DecidableEq, Repr

/-- Each rejection names its diagnostic. -/
inductive Rejection where
  /-- `nonnull-member-retired-spelling`: names the merged kind to write. -/
  | retiredSpelling (kind : Qualifier)
  /-- `nonnull-member-scope-needs-parameter` -/
  | scopeNeedsParameter
  /-- `nonnull-member-parameter-needs-scope` -/
  | parameterNeedsScope
  /-- `nonnull-member-qualifier-unknown` -/
  | unknownQualifier (word : String)
  /-- `nonnull-member-shape` -/
  | shape
deriving DecidableEq, Repr

/-- The reader. -/
def load : Surface → Except Rejection Fact
  | ⟨.retiredStable, _, _⟩ => .error (.retiredSpelling .stable)
  | ⟨.retiredScope, _, _⟩ => .error (.retiredSpelling .scope)
  | ⟨.nonnullMember, _, some (.other w)⟩ => .error (.unknownQualifier w)
  | ⟨.nonnullMember, [t, f], none⟩ => .ok (.member t f)
  | ⟨.nonnullMember, [t, f], some .stable⟩ => .ok (.stable t f)
  | ⟨.nonnullMember, [_, _], some .scope⟩ => .error .scopeNeedsParameter
  | ⟨.nonnullMember, [t, f, p], some .scope⟩ => .ok (.scope t f p)
  | ⟨.nonnullMember, [_, _, _], _⟩ => .error .parameterNeedsScope
  | ⟨.nonnullMember, _, _⟩ => .error .shape

/-- The migration: every old clause's merged spelling. -/
def migrate : OldClause → Surface
  | .nonnullMember t f => ⟨.nonnullMember, [t, f], none⟩
  | .nonnullMemberStable t f => ⟨.nonnullMember, [t, f], some .stable⟩
  | .nonnullMemberScope t f p => ⟨.nonnullMember, [t, f, p], some .scope⟩

theorem the_merged_surface_loads_the_old_fact (o : OldClause) :
    load (migrate o) = .ok o.fact := by
  cases o <;> rfl

theorem every_admitted_clause_is_a_migrated_one (s : Surface) (fact : Fact)
    (h : load s = .ok fact) : ∃ o, migrate o = s ∧ o.fact = fact := by
  rcases s with ⟨head, args, qualifier⟩
  cases head with
  | retiredStable => simp [load] at h
  | retiredScope => simp [load] at h
  | nonnullMember =>
    match args, qualifier, h with
    | [t, f], none, h =>
      simp [load] at h
      exact ⟨.nonnullMember t f, rfl, h⟩
    | [t, f], some .stable, h =>
      simp [load] at h
      exact ⟨.nonnullMemberStable t f, rfl, h⟩
    | [t, f, p], some .scope, h =>
      simp [load] at h
      exact ⟨.nonnullMemberScope t f p, rfl, h⟩
    | [], q, h => cases q <;> (try rename_i q; try cases q) <;> simp [load] at h
    | [_], q, h => cases q <;> (try rename_i q; try cases q) <;> simp [load] at h
    | [_, _], some .scope, h => simp [load] at h
    | [_, _], some (.other _), h => simp [load] at h
    | [_, _, _], none, h => simp [load] at h
    | [_, _, _], some .stable, h => simp [load] at h
    | [_, _, _], some (.other _), h => simp [load] at h
    | _ :: _ :: _ :: _ :: _, q, h => cases q <;> (try rename_i q; try cases q) <;> simp [load] at h

theorem migration_is_injective (a b : OldClause) (h : migrate a = migrate b) : a = b := by
  cases a <;> cases b <;> simp_all [migrate]

/-- Equivalence: a merged clause is admitted with a fact exactly when it is
the migration of an old clause that loaded to that fact. -/
theorem the_merged_surface_is_equivalent_to_the_old_forms (s : Surface) (fact : Fact) :
    load s = .ok fact ↔ ∃ o, migrate o = s ∧ o.fact = fact := by
  constructor
  · exact every_admitted_clause_is_a_migrated_one s fact
  · rintro ⟨o, rfl, rfl⟩
    exact the_merged_surface_loads_the_old_fact o

theorem a_retired_word_is_rejected_naming_the_new_form (args : List String)
    (q : Option Qualifier) :
    load ⟨.retiredStable, args, q⟩ = .error (.retiredSpelling .stable) ∧
      load ⟨.retiredScope, args, q⟩ = .error (.retiredSpelling .scope) :=
  ⟨rfl, rfl⟩

theorem a_parameter_needs_the_scope_qualifier (t f p : String) (q : Option Qualifier)
    (hq : q ≠ some .scope) (fact : Fact) :
    load ⟨.nonnullMember, [t, f, p], q⟩ ≠ .ok fact := by
  match q, hq with
  | none, _ => simp [load]
  | some .stable, _ => simp [load]
  | some .scope, hq => exact absurd rfl hq
  | some (.other _), _ => simp [load]

theorem the_scope_qualifier_needs_a_parameter (args : List String) (fact : Fact)
    (h : load ⟨.nonnullMember, args, some .scope⟩ = .ok fact) : args.length = 3 := by
  match args, h with
  | [_, _, _], _ => rfl
  | [], h => simp [load] at h
  | [_], h => simp [load] at h
  | [_, _], h => simp [load] at h
  | _ :: _ :: _ :: _ :: _, h => simp [load] at h

theorem an_unknown_qualifier_is_rejected (args : List String) (w : String) :
    load ⟨.nonnullMember, args, some (.other w)⟩ = .error (.unknownQualifier w) := rfl

/-- A bounds declaration's clauses, as far as the premise placement reads them. -/
inductive Clause where
  | fact (f : Fact)
  | premise (name : String)
  /-- `quant null-deref`, a bounds clause, a result declaration. -/
  | other
deriving DecidableEq, Repr

/-- The placement check: a premise stands only in a declaration that also
states a member fact, of any kind. -/
def premiseAdmitted (clauses : List Clause) : Bool :=
  !(clauses.any (fun c => match c with | .premise _ => true | _ => false)) ||
    clauses.any (fun c => match c with | .fact _ => true | _ => false)

theorem a_premise_stands_beside_every_kind (f : Fact) (n : String) (rest : List Clause) :
    premiseAdmitted (.fact f :: .premise n :: rest) = true := by
  simp [premiseAdmitted]

theorem a_premise_needs_a_member_fact (n : String) (rest : List Clause)
    (h : ∀ c ∈ rest, ∀ f, c ≠ .fact f) :
    premiseAdmitted (.premise n :: rest) = false := by
  simp only [premiseAdmitted, List.any_cons, Bool.true_or, Bool.not_true, Bool.false_or]
  apply List.any_eq_false.mpr
  intro c hc
  cases c with
  | fact f => exact absurd rfl (h _ hc f)
  | premise _ => simp
  | other => simp

/-- The shared examples. -/
theorem qualifier_vectors_hold :
    load ⟨.nonnullMember, ["S", "next"], none⟩ = .ok (.member "S" "next") ∧
    load ⟨.nonnullMember, ["S", "next"], some .stable⟩ = .ok (.stable "S" "next") ∧
    load ⟨.nonnullMember, ["S", "next", "s"], some .scope⟩ = .ok (.scope "S" "next" "s") ∧
    load ⟨.nonnullMember, ["S", "next"], some .scope⟩ = .error .scopeNeedsParameter ∧
    load ⟨.nonnullMember, ["S", "next", "s"], none⟩ = .error .parameterNeedsScope ∧
    load ⟨.nonnullMember, ["S", "next", "s"], some .stable⟩ = .error .parameterNeedsScope ∧
    load ⟨.nonnullMember, ["S", "next"], some (.other "global")⟩ =
      .error (.unknownQualifier "global") ∧
    load ⟨.retiredStable, ["S", "next"], none⟩ = .error (.retiredSpelling .stable) ∧
    load ⟨.retiredScope, ["S", "next", "s"], none⟩ = .error (.retiredSpelling .scope) ∧
    premiseAdmitted [.fact (.member "S" "next"), .premise "no_plugin"] = true ∧
    premiseAdmitted [.premise "no_plugin", .other] = false :=
  ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

end QCore.NullMemberQualifier
