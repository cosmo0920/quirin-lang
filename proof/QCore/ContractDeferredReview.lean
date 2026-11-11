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

import QCore.ReasonCategory

/-!
# Contracts awaiting human review

Card **W45-CONTRACT-DEFERRED-REVIEW** (contract spec §C06). A contract module
may mark a declaration, or one of its clauses, as awaiting human review
(`resource_review_pending`). The loader validates the declaration in full and
then **withholds** it: a pending declaration discharges nothing and accuses
nothing, and every call site that would apply it is `Unknown` under the reason
code `awaiting-human-review`.

This is the model of that decision
(`crates/qcore-contract/src/contract_loader.rs`, which withholds;
`crates/qcore-c-gateway/src/legacy_review_hold.rs`, which holds the call sites;
`crates/qcore-schemas/src/reason.rs`, which names the unit reason).

## What is proved

* **Withholding is total over the declaration.** A declaration with a deferred
  clause is pending exactly as a declaration marked as a whole is; the loader
  applies a declaration's clauses all together or not at all, so no deferred
  clause is ever dropped while the rest decide.
* **A deferred clause contributes no discharge and no accusation.** Every
  obligation whose verdict could only rest on a pending declaration is neither
  `Safe` nor `Unsafe`, and its reason is `awaitingHumanReview`, a category that
  says nothing about the analysed source and never supports `Unsafe`.
* **Monotonicity of review.** Removing deferral from any set of clauses only
  ever adds contributions, and cannot move the verdict of an obligation no newly
  effective clause touches: a `Safe` verdict there stays `Safe`.
* **Inheritance propagates a deferral.** An inherited declaration carries its
  parent's markers clause by clause, so it is pending exactly when its parent
  is, through any chain.
* **The unit reason names the review first.** A projection summary naming at
  least one held call classifies as `awaitingHumanReview`, never as a source
  category.

## What this is not

The clause contents are abstract: which obligations a clause discharges or
accuses is a premise supplied by the analysis, not modelled here. That the
gateway's token rewrite reaches every call site is a Rust boundary pinned by
`crates/qcore-c-gateway/tests/contract_deferred_review.rs`.
-/

namespace QCore.ContractDeferredReview

open QCore.ReasonCategory

/-- One clause: what it discharges and what it accuses, over obligations `O`,
and whether it is wrapped in a clause-level review marker. -/
structure Clause (O : Type) where
  discharges : O → Bool
  accuses : O → Bool
  deferred : Bool

/-- One declaration: its clauses, in order, and whether it carries a
contract-level review marker. -/
structure Declaration (O : Type) where
  clauses : List (Clause O)
  contractDeferred : Bool

namespace Declaration

variable {O : Type}

/-- Pending review: a contract-level marker, or any clause-level one. -/
def pending (d : Declaration O) : Bool :=
  d.contractDeferred || d.clauses.any (·.deferred)

/-- What the loader lets a consumer apply: every clause, or nothing. -/
def applied (d : Declaration O) : Option (List (Clause O)) :=
  if d.pending then none else some d.clauses

/-- The obligations a declaration's applied clauses discharge. -/
def discharges (d : Declaration O) (o : O) : Bool :=
  match d.applied with
  | none => false
  | some cs => cs.any (·.discharges o)

/-- The obligations a declaration's applied clauses accuse. -/
def accuses (d : Declaration O) (o : O) : Bool :=
  match d.applied with
  | none => false
  | some cs => cs.any (·.accuses o)

/-- Whether any clause of the declaration, applied or not, touches `o`. -/
def touches (d : Declaration O) (o : O) : Bool :=
  d.clauses.any (fun c => c.discharges o || c.accuses o)

end Declaration

/-- A verdict and, for `Unknown`, its reason category (`none` when the
obligation is simply undecided by any declaration). -/
inductive Verdict where
  | safe
  | provisionalUnsafe
  | unknown (reason : Option Category)
  deriving DecidableEq, Repr

variable {O : Type}

/-- The verdict of one obligation over an environment of declarations: an
accusation by an applied clause is `Unsafe`, a discharge with no accusation is
`Safe`; otherwise `Unknown`, named `awaitingHumanReview` when some pending
declaration touches the obligation. -/
def verdict (env : List (Declaration O)) (o : O) : Verdict :=
  if env.any (·.accuses o) then .provisionalUnsafe
  else if env.any (·.discharges o) then .safe
  else if env.any (fun d => d.pending && d.touches o) then .unknown (some .awaitingHumanReview)
  else .unknown none

/-! ## Withholding is total over the declaration -/

/-- A clause-level marker withholds the declaration exactly as a contract-level
marker does. -/
theorem a_deferred_clause_withholds_the_declaration (d : Declaration O)
    {c : Clause O} (member : c ∈ d.clauses) (deferred : c.deferred = true) :
    d.pending = true ∧ d.applied = none := by
  have pending : d.pending = true := by
    simp only [Declaration.pending, Bool.or_eq_true, List.any_eq_true]
    exact Or.inr ⟨c, member, deferred⟩
  exact ⟨pending, by simp [Declaration.applied, pending]⟩

/-- No partial application: whatever is applied is the whole clause list, and
none of it is deferred. A deferred clause is therefore never dropped while the
remaining clauses decide. -/
theorem applied_is_all_or_nothing (d : Declaration O) {cs : List (Clause O)}
    (applied : d.applied = some cs) :
    cs = d.clauses ∧ d.contractDeferred = false ∧ ∀ c ∈ cs, c.deferred = false := by
  unfold Declaration.applied at applied
  by_cases pending : d.pending = true
  · simp [pending] at applied
  · simp only [pending, Bool.false_eq_true, ↓reduceIte, Option.some.injEq] at applied
    subst applied
    simp only [Declaration.pending, Bool.or_eq_true, List.any_eq_true, not_or,
      not_exists, not_and, Bool.not_eq_true] at pending
    exact ⟨rfl, pending.1, fun c member => by
      simpa using pending.2 c member⟩

/-- A pending declaration discharges nothing and accuses nothing. -/
theorem a_pending_declaration_contributes_nothing (d : Declaration O)
    (pending : d.pending = true) (o : O) :
    d.discharges o = false ∧ d.accuses o = false := by
  simp [Declaration.discharges, Declaration.accuses, Declaration.applied, pending]

/-! ## A verdict that depends on a deferred clause is neither Safe nor Unsafe -/

/-- If every declaration that touches `o` is pending and at least one does, the
verdict of `o` is `Unknown` with reason `awaitingHumanReview` — neither `Safe`
nor `Unsafe`. -/
theorem a_verdict_resting_on_a_deferred_clause_is_unknown
    (env : List (Declaration O)) (o : O)
    (only : ∀ d ∈ env, d.touches o = true → d.pending = true)
    (some_pending : ∃ d ∈ env, d.pending = true ∧ d.touches o = true) :
    verdict env o = .unknown (some .awaitingHumanReview) := by
  have noAccusation : env.any (·.accuses o) = false := by
    rw [List.any_eq_false]
    intro d member accused
    have touched : d.touches o = true := by
      unfold Declaration.accuses at accused
      split at accused
      · simp at accused
      · rename_i cs applied
        have ⟨same, _, _⟩ := applied_is_all_or_nothing d applied
        subst same
        simp only [List.any_eq_true] at accused
        obtain ⟨c, member, hit⟩ := accused
        simp only [Declaration.touches, List.any_eq_true, Bool.or_eq_true]
        exact ⟨c, member, Or.inr hit⟩
    have := (a_pending_declaration_contributes_nothing d (only d member touched) o).2
    simp [this] at accused
  have noDischarge : env.any (·.discharges o) = false := by
    rw [List.any_eq_false]
    intro d member discharged
    have touched : d.touches o = true := by
      unfold Declaration.discharges at discharged
      split at discharged
      · simp at discharged
      · rename_i cs applied
        have ⟨same, _, _⟩ := applied_is_all_or_nothing d applied
        subst same
        simp only [List.any_eq_true] at discharged
        obtain ⟨c, member, hit⟩ := discharged
        simp only [Declaration.touches, List.any_eq_true, Bool.or_eq_true]
        exact ⟨c, member, Or.inl hit⟩
    have := (a_pending_declaration_contributes_nothing d (only d member touched) o).1
    simp [this] at discharged
  have held : env.any (fun d => d.pending && d.touches o) = true := by
    obtain ⟨d, member, pending, touched⟩ := some_pending
    simp only [List.any_eq_true, Bool.and_eq_true]
    exact ⟨d, member, pending, touched⟩
  simp [verdict, noAccusation, noDischarge, held]

/-- The corollary the card states: such a verdict is neither `Safe` nor
`Unsafe`. -/
theorem a_deferred_dependency_is_neither_safe_nor_unsafe
    (env : List (Declaration O)) (o : O)
    (only : ∀ d ∈ env, d.touches o = true → d.pending = true)
    (some_pending : ∃ d ∈ env, d.pending = true ∧ d.touches o = true) :
    verdict env o ≠ .safe ∧ verdict env o ≠ .provisionalUnsafe := by
  rw [a_verdict_resting_on_a_deferred_clause_is_unknown env o only some_pending]
  exact ⟨by simp, by simp⟩

/-- The reason it carries says nothing about the analysed source and can never
support a provisional `Unsafe`. -/
theorem the_review_reason_never_supports_unsafe :
    Category.maySupportUnsafe .awaitingHumanReview = false ∧
      Category.subject .awaitingHumanReview = .analyser := ⟨rfl, rfl⟩

/-! ## Monotonicity of review -/

/-- Resolve the review of a declaration: drop its contract-level marker and
unwrap every clause-level one. -/
def Declaration.resolve (d : Declaration O) : Declaration O :=
  { clauses := d.clauses.map (fun c => { c with deferred := false }),
    contractDeferred := false }

/-- Resolve the reviews of the declarations `chosen` selects. -/
def resolveWhere (chosen : Declaration O → Bool) (env : List (Declaration O)) :
    List (Declaration O) :=
  env.map (fun d => if chosen d then d.resolve else d)

theorem Declaration.resolve_not_pending (d : Declaration O) : d.resolve.pending = false := by
  simp [Declaration.pending, Declaration.resolve, List.any_map, Function.comp_def]

theorem Declaration.resolve_touches (d : Declaration O) (o : O) :
    d.resolve.touches o = d.touches o := by
  simp [Declaration.touches, Declaration.resolve, List.any_map, Function.comp_def]

/-- A declaration's contribution to an obligation it does not touch is empty,
pending or not. -/
theorem Declaration.untouched_contributes_nothing (d : Declaration O) (o : O)
    (untouched : d.touches o = false) : d.discharges o = false ∧ d.accuses o = false := by
  simp only [Declaration.touches, List.any_eq_false, Bool.or_eq_true, not_or,
    Bool.not_eq_true] at untouched
  constructor
  · unfold Declaration.discharges
    split
    · rfl
    · rename_i cs applied
      have ⟨same, _, _⟩ := applied_is_all_or_nothing d applied
      subst same
      rw [List.any_eq_false]
      intro c member
      simp [(untouched c member).1]
  · unfold Declaration.accuses
    split
    · rfl
    · rename_i cs applied
      have ⟨same, _, _⟩ := applied_is_all_or_nothing d applied
      subst same
      rw [List.any_eq_false]
      intro c member
      simp [(untouched c member).2]

/-- Resolving reviews only adds contributions: every discharge and accusation of
the original environment survives. -/
theorem resolving_only_adds_contributions (chosen : Declaration O → Bool)
    (env : List (Declaration O)) (o : O) :
    (env.any (·.discharges o) = true →
        (resolveWhere chosen env).any (·.discharges o) = true) ∧
      (env.any (·.accuses o) = true →
        (resolveWhere chosen env).any (·.accuses o) = true) := by
  have keep : ∀ d : Declaration O, (d.discharges o = true →
      (if chosen d then d.resolve else d).discharges o = true) ∧
      (d.accuses o = true → (if chosen d then d.resolve else d).accuses o = true) := by
    intro d
    by_cases pick : chosen d = true
    · by_cases pending : d.pending = true
      · have := a_pending_declaration_contributes_nothing d pending o
        simp [this]
      · have undeferred : d.resolve = d := by
          simp only [Declaration.pending, Bool.or_eq_true, List.any_eq_true, not_or,
            not_exists, not_and, Bool.not_eq_true] at pending
          cases d with
          | mk clauses contractDeferred =>
            simp only [Declaration.resolve, Declaration.mk.injEq]
            refine ⟨?_, pending.1.symm⟩
            conv => rhs; rw [← List.map_id clauses]
            apply List.map_congr_left
            intro c member
            cases c with
            | mk dis acc deferred =>
              have := pending.2 _ member
              simp_all
        simp [pick, undeferred]
    · simp [pick]
  constructor
  · intro held
    simp only [List.any_eq_true, resolveWhere, List.mem_map] at held ⊢
    obtain ⟨d, member, hit⟩ := held
    exact ⟨_, ⟨d, member, rfl⟩, (keep d).1 hit⟩
  · intro held
    simp only [List.any_eq_true, resolveWhere, List.mem_map] at held ⊢
    obtain ⟨d, member, hit⟩ := held
    exact ⟨_, ⟨d, member, rfl⟩, (keep d).2 hit⟩

/-- **Monotonicity.** Resolving the reviews of any set of declarations cannot
move a `Safe` verdict on an obligation that none of the resolved declarations
touches. -/
theorem resolving_reviews_keeps_unrelated_safe (chosen : Declaration O → Bool)
    (env : List (Declaration O)) (o : O)
    (unrelated : ∀ d ∈ env, chosen d = true → d.touches o = false)
    (safe : verdict env o = .safe) :
    verdict (resolveWhere chosen env) o = .safe := by
  -- Every declaration contributes to `o` exactly as it did before.
  have same : ∀ d ∈ env, (if chosen d then d.resolve else d).discharges o = d.discharges o ∧
      (if chosen d then d.resolve else d).accuses o = d.accuses o := by
    intro d member
    by_cases pick : chosen d = true
    · have untouched := unrelated d member pick
      have before := d.untouched_contributes_nothing o untouched
      have after := d.resolve.untouched_contributes_nothing o
        (by rw [Declaration.resolve_touches]; exact untouched)
      simp [pick, before, after]
    · simp [pick]
  have accuses : (resolveWhere chosen env).any (·.accuses o) = env.any (·.accuses o) := by
    simp only [resolveWhere, List.any_map, Function.comp_def]
    apply Bool.eq_iff_iff.mpr
    simp only [List.any_eq_true]
    constructor
    · rintro ⟨d, member, hit⟩
      exact ⟨d, member, by rw [← (same d member).2]; exact hit⟩
    · rintro ⟨d, member, hit⟩
      exact ⟨d, member, by rw [(same d member).2]; exact hit⟩
  have discharges :
      (resolveWhere chosen env).any (·.discharges o) = env.any (·.discharges o) := by
    simp only [resolveWhere, List.any_map, Function.comp_def]
    apply Bool.eq_iff_iff.mpr
    simp only [List.any_eq_true]
    constructor
    · rintro ⟨d, member, hit⟩
      exact ⟨d, member, by rw [← (same d member).1]; exact hit⟩
    · rintro ⟨d, member, hit⟩
      exact ⟨d, member, by rw [(same d member).1]; exact hit⟩
  unfold verdict at safe ⊢
  rw [accuses, discharges]
  by_cases accused : env.any (·.accuses o) = true
  · simp [accused] at safe
  · by_cases discharged : env.any (·.discharges o) = true
    · simp [accused, discharged]
    · exfalso
      revert safe
      simp only [accused, discharged, Bool.false_eq_true, ↓reduceIte]
      split <;> simp

/-! ## Inheritance propagates a deferral -/

/-- Contract inheritance (§C15): the child's clauses are the parent's, in order,
each renamed by position — a renaming changes what a clause names, never whether
it is deferred. The contract-level marker is a clause of the parent and is
copied the same way. -/
def inherit (rename : Clause O → Clause O) (parent : Declaration O) : Declaration O :=
  { clauses := parent.clauses.map rename, contractDeferred := parent.contractDeferred }

/-- A child is pending exactly when its parent is. -/
theorem inheritance_propagates_pending (rename : Clause O → Clause O)
    (preserves : ∀ c, (rename c).deferred = c.deferred) (parent : Declaration O) :
    (inherit rename parent).pending = parent.pending := by
  simp [inherit, Declaration.pending, List.any_map, Function.comp_def, preserves]

/-- Clause by clause: the child's `i`-th clause is deferred exactly when the
parent's is. -/
theorem inheritance_propagates_each_clause (rename : Clause O → Clause O)
    (preserves : ∀ c, (rename c).deferred = c.deferred) (parent : Declaration O) :
    (inherit rename parent).clauses.map (·.deferred) =
      parent.clauses.map (·.deferred) := by
  simp [inherit, List.map_map, Function.comp_def, preserves]

/-- A child of a deferred parent is withheld: it contributes nothing either. -/
theorem an_inherited_deferral_withholds_the_child (rename : Clause O → Clause O)
    (preserves : ∀ c, (rename c).deferred = c.deferred) (parent : Declaration O)
    (pending : parent.pending = true) (o : O) :
    (inherit rename parent).applied = none ∧
      (inherit rename parent).discharges o = false ∧
      (inherit rename parent).accuses o = false := by
  have child : (inherit rename parent).pending = true := by
    rw [inheritance_propagates_pending rename preserves]; exact pending
  exact ⟨by simp [Declaration.applied, child],
    a_pending_declaration_contributes_nothing _ child o⟩

/-- Through a chain: inheriting twice is pending exactly when the root is. -/
theorem inheritance_chains_propagate_pending (first second : Clause O → Clause O)
    (preservesFirst : ∀ c, (first c).deferred = c.deferred)
    (preservesSecond : ∀ c, (second c).deferred = c.deferred) (root : Declaration O) :
    (inherit second (inherit first root)).pending =
      root.pending := by
  rw [inheritance_propagates_pending second preservesSecond,
    inheritance_propagates_pending first preservesFirst]

/-! ## The unit reason names the review first -/

/-- The projection summary's classification
(`ReasonCategory::classify` on `resource projection unresolved: …`): held calls
first, then an unsupported operation, then residual custody. -/
def classifyProjection (unsupported held : Nat) : Category :=
  if held > 0 then .awaitingHumanReview
  else if unsupported > 0 then .unsupportedSyntax
  else .residualCustody

/-- A summary naming a held call is never a statement about the analysed source,
whatever else it counts. -/
theorem a_held_summary_is_never_a_source_statement (unsupported held : Nat)
    (positive : held > 0) :
    classifyProjection unsupported held = .awaitingHumanReview ∧
      Category.describesAnalysedSource (classifyProjection unsupported held) = false := by
  simp [classifyProjection, positive, Category.describesAnalysedSource, Category.subject]

/-- And a summary with no held call classifies exactly as before the card. -/
theorem a_summary_without_held_calls_is_unchanged (unsupported : Nat) :
    classifyProjection unsupported 0 =
      (if unsupported > 0 then .unsupportedSyntax else .residualCustody) := by
  simp [classifyProjection]

end QCore.ContractDeferredReview
