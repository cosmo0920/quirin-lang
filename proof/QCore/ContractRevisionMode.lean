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
# Revision modes of declared contracts

Card **W45-CONTRACT-REVISION-MODE** (contract spec §C06.2). A contract, or one
clause of it, may say `revision pinned` or `revision flexible`; **pinned is the
default**. A pinned contract is in force only where the audited source matches
the identity its lock records; every mismatch is withheld, and each call that
relies on it is `Unknown` under the reason code `contract-pin-mismatch`. A
flexible contract binds by name and declared signature only: on a function
whose body the run analyses the body is checked against it as usual, and on a
trusted function it is an explicit assumption of the run.

This is the model of that decision
(`crates/qcore-contract/src/contract_revision.rs`, which reads the markers;
`crates/qcore-audit/src/contract_revision.rs`, which compares pins and
withholds mismatches; `crates/qcore-c-gateway/src/legacy_pin_hold.rs`, which
holds the call sites; `crates/qcore-schemas/src/reason.rs`, which names the
unit reason).

## What is proved

* **The default is pinned**, and one pinned clause pins the whole contract,
  because a contract is applied as one conjunction.
* **A mismatched pin is not in force**: it discharges nothing and accuses
  nothing, so a verdict that rests on it is neither `Safe` nor `Unsafe`, and
  its reason is an analyser-subject category that never supports `Unsafe`.
* **Several reviewed identities**: a pinned contract is in force exactly when
  its text is the pinned one and the audited identity is one of the reviewed
  identities; adding an identity disturbs none; a contract edit voids every
  identity; pinned at `a` and `b` it holds at both and nowhere else.
* **Re-pinning restores verdicts exactly**: an environment whose mismatches are
  re-pinned (the audited identity added) gives every obligation the verdict of
  the same environment with every contract in force, which is the verdict the
  matching pins gave.
* **Flexible re-verification**: a flexible contract is in force at every
  source identity, and a body the run analyses is checked against exactly the
  declared clauses at every identity — the mode never skips the check. A
  flexible contract on a trusted function is always listed as an assumption.
* **The unit reason names the pin**: a projection summary naming a call held
  for a pin mismatch classifies as `contractPinMismatch`, never as a source
  category; a held review is named first.

## What this is not

Identities are abstract values compared for equality: that the lock digest is
a faithful identity of a function's text is the scanner's boundary
(`crates/qcore-audit/src/c_identity.rs`), pinned by its tests. Which
obligations a clause discharges or accuses is a premise supplied by the
analysis. That the gateway's rewrite reaches every call site is pinned by
`crates/qcore-c-gateway/tests/contract_pin_hold.rs`.
-/

namespace QCore.ContractRevisionMode

open QCore.ReasonCategory

/-- A revision mode. -/
inductive Mode where
  | pinned
  | flexible
  deriving DecidableEq, Repr

/-- One clause: what it discharges and what it accuses, and its own marker. -/
structure Clause (O : Type) where
  discharges : O → Bool
  accuses : O → Bool
  marker : Option Mode

/-- One declaration over identities `I`: its clauses, its contract-level
marker, whether its contract text is the one its lock entry was pinned with,
the reviewed identities that entry records (each a source identity reviewed
at a named revision; empty without a lock or entry), the identity of its
function in the audited source, and whether the run analyses that
function's body. -/
structure Declaration (O I : Type) where
  clauses : List (Clause O)
  contractMarker : Option Mode
  textMatches : Bool
  reviewed : List I
  audited : I
  hasBody : Bool

namespace Declaration

variable {O I : Type} [DecidableEq I]

/-- The contract-level mode: the marker, else the default. -/
def contractMode (d : Declaration O I) : Mode := d.contractMarker.getD .pinned

/-- The effective mode: flexible exactly when the contract-level mode is
flexible and no clause is marked pinned. -/
def mode (d : Declaration O I) : Mode :=
  if d.contractMode = .flexible ∧ ¬ d.clauses.any (·.marker = some .pinned) then .flexible
  else .pinned

/-- In force: a flexible declaration always; a pinned one only where its
contract text is the pinned one and the audited identity is one of the
reviewed identities. -/
def inForce (d : Declaration O I) : Bool :=
  match d.mode with
  | .flexible => true
  | .pinned => d.textMatches && d.reviewed.contains d.audited

/-- What a consumer may apply: every clause, or nothing. -/
def applied (d : Declaration O I) : Option (List (Clause O)) :=
  if d.inForce then some d.clauses else none

def discharges (d : Declaration O I) (o : O) : Bool :=
  match d.applied with
  | none => false
  | some cs => cs.any (·.discharges o)

def accuses (d : Declaration O I) (o : O) : Bool :=
  match d.applied with
  | none => false
  | some cs => cs.any (·.accuses o)

def touches (d : Declaration O I) (o : O) : Bool :=
  d.clauses.any (fun c => c.discharges o || c.accuses o)

/-- A pin mismatch: pinned, and not in force. -/
def mismatched (d : Declaration O I) : Bool := d.mode = .pinned && !d.inForce

/-- Re-pin after review (`quirin-audit pin`): the audited identity is added
beside the identities reviewed at other revisions, which are kept while the
contract text is the pinned one and voided when it changed; the lock then
records the current text. -/
def repin (d : Declaration O I) : Declaration O I :=
  { d with
    reviewed := d.audited :: (if d.textMatches then d.reviewed else [])
    textMatches := true }

/-- Add one reviewed identity, keeping every other. -/
def addIdentity (d : Declaration O I) (i : I) : Declaration O I :=
  { d with reviewed := i :: d.reviewed }

/-- The body check of a function the run analyses: against the clauses in
force, or against nothing when the contract is withheld (the body is then an
undeclared definition). `none` when no body is analysed. -/
def bodyCheck {R : Type} (check : List (Clause O) → R) (d : Declaration O I) : Option R :=
  if d.hasBody then some (check (d.applied.getD [])) else none

end Declaration

/-- A verdict and, for `Unknown`, its reason category. -/
inductive Verdict where
  | safe
  | provisionalUnsafe
  | unknown (reason : Option Category)
  deriving DecidableEq, Repr

variable {O I : Type} [DecidableEq I]

/-- The verdict of one obligation: an accusation by an applied clause is
`Unsafe`, a discharge with no accusation `Safe`; otherwise `Unknown`, named
`contractPinMismatch` when a mismatched declaration touches the obligation. -/
def verdict (env : List (Declaration O I)) (o : O) : Verdict :=
  if env.any (·.accuses o) then .provisionalUnsafe
  else if env.any (·.discharges o) then .safe
  else if env.any (fun d => d.mismatched && d.touches o) then .unknown (some .contractPinMismatch)
  else .unknown none

/-- The verdict with every declaration in force, whatever its pin. -/
def verdictInForce (env : List (Declaration O I)) (o : O) : Verdict :=
  if env.any (fun d => d.clauses.any (·.accuses o)) then .provisionalUnsafe
  else if env.any (fun d => d.clauses.any (·.discharges o)) then .safe
  else .unknown none

/-! ## The default is pinned -/

omit [DecidableEq I] in
/-- With no marker anywhere, a declaration is pinned. -/
theorem the_default_is_pinned (d : Declaration O I) (noContractMarker : d.contractMarker = none) :
    d.mode = .pinned := by
  simp [Declaration.mode, Declaration.contractMode, noContractMarker]

omit [DecidableEq I] in
/-- One clause marked pinned pins the whole declaration, whatever the
contract-level marker says. -/
theorem one_pinned_clause_pins_the_contract (d : Declaration O I) {c : Clause O}
    (member : c ∈ d.clauses) (pinned : c.marker = some .pinned) : d.mode = .pinned := by
  have any : d.clauses.any (·.marker = some .pinned) = true := by
    simp only [List.any_eq_true, decide_eq_true_eq]
    exact ⟨c, member, pinned⟩
  simp [Declaration.mode, any]

omit [DecidableEq I] in
/-- A flexible contract-level marker with no pinned clause makes it flexible. -/
theorem a_flexible_contract_without_pinned_clauses_is_flexible (d : Declaration O I)
    (flexible : d.contractMarker = some .flexible)
    (noPinned : ∀ c ∈ d.clauses, c.marker ≠ some .pinned) : d.mode = .flexible := by
  have none : d.clauses.any (·.marker = some .pinned) = false := by
    rw [List.any_eq_false]
    intro c member
    simpa using noPinned c member
  simp [Declaration.mode, Declaration.contractMode, flexible, none]

/-! ## A mismatched pin is not in force -/

/-- **In force iff the text matches and some identity matches**, for a
pinned declaration. -/
theorem a_pin_is_in_force_iff_its_text_and_some_identity_match (d : Declaration O I)
    (pinned : d.mode = .pinned) :
    d.inForce = true ↔ d.textMatches = true ∧ d.audited ∈ d.reviewed := by
  simp [Declaration.inForce, pinned]

/-- A pinned declaration whose text changed, or whose audited identity is
none of the reviewed ones, is not in force and applies nothing. -/
theorem a_mismatched_pin_is_not_in_force (d : Declaration O I) (pinned : d.mode = .pinned)
    (differs : d.textMatches = false ∨ d.audited ∉ d.reviewed) :
    d.inForce = false ∧ d.applied = none ∧ d.mismatched = true := by
  have notInForce : d.inForce = false := by
    cases differs with
    | inl text => simp [Declaration.inForce, pinned, text]
    | inr absent => simp [Declaration.inForce, pinned, absent]
  exact ⟨notInForce, by simp [Declaration.applied, notInForce],
    by simp [Declaration.mismatched, pinned, notInForce]⟩

/-- A mismatched declaration discharges nothing and accuses nothing. -/
theorem a_mismatched_pin_contributes_nothing (d : Declaration O I)
    (mismatched : d.mismatched = true) (o : O) :
    d.discharges o = false ∧ d.accuses o = false := by
  have notInForce : d.inForce = false := by
    simp only [Declaration.mismatched, Bool.and_eq_true, Bool.not_eq_eq_eq_not,
      Bool.not_true] at mismatched
    exact mismatched.2
  simp [Declaration.discharges, Declaration.accuses, Declaration.applied, notInForce]

/-- Whatever is applied is the whole clause list. -/
theorem applied_is_all_or_nothing (d : Declaration O I) {cs : List (Clause O)}
    (applied : d.applied = some cs) : cs = d.clauses ∧ d.inForce = true := by
  unfold Declaration.applied at applied
  by_cases inForce : d.inForce = true
  · simp only [inForce, ↓reduceIte, Option.some.injEq] at applied
    exact ⟨applied.symm, inForce⟩
  · simp [inForce] at applied

theorem Declaration.contributes_only_what_it_touches (d : Declaration O I) (o : O) :
    (d.discharges o = true → d.touches o = true) ∧
      (d.accuses o = true → d.touches o = true) := by
  constructor
  · intro hit
    unfold Declaration.discharges at hit
    split at hit
    · simp at hit
    · rename_i cs applied
      have ⟨same, _⟩ := applied_is_all_or_nothing d applied
      subst same
      simp only [List.any_eq_true] at hit
      obtain ⟨c, member, yes⟩ := hit
      simp only [Declaration.touches, List.any_eq_true, Bool.or_eq_true]
      exact ⟨c, member, Or.inl yes⟩
  · intro hit
    unfold Declaration.accuses at hit
    split at hit
    · simp at hit
    · rename_i cs applied
      have ⟨same, _⟩ := applied_is_all_or_nothing d applied
      subst same
      simp only [List.any_eq_true] at hit
      obtain ⟨c, member, yes⟩ := hit
      simp only [Declaration.touches, List.any_eq_true, Bool.or_eq_true]
      exact ⟨c, member, Or.inr yes⟩

/-- If every declaration that touches `o` is a mismatched pin and at least one
does, the verdict of `o` is `Unknown` with reason `contractPinMismatch`. -/
theorem a_verdict_resting_on_a_mismatched_pin_is_unknown (env : List (Declaration O I)) (o : O)
    (only : ∀ d ∈ env, d.touches o = true → d.mismatched = true)
    (some_mismatched : ∃ d ∈ env, d.mismatched = true ∧ d.touches o = true) :
    verdict env o = .unknown (some .contractPinMismatch) := by
  have noAccusation : env.any (·.accuses o) = false := by
    rw [List.any_eq_false]
    intro d member accused
    have touched := (d.contributes_only_what_it_touches o).2 accused
    have := (a_mismatched_pin_contributes_nothing d (only d member touched) o).2
    simp [this] at accused
  have noDischarge : env.any (·.discharges o) = false := by
    rw [List.any_eq_false]
    intro d member discharged
    have touched := (d.contributes_only_what_it_touches o).1 discharged
    have := (a_mismatched_pin_contributes_nothing d (only d member touched) o).1
    simp [this] at discharged
  have held : env.any (fun d => d.mismatched && d.touches o) = true := by
    obtain ⟨d, member, mismatched, touched⟩ := some_mismatched
    simp only [List.any_eq_true, Bool.and_eq_true]
    exact ⟨d, member, mismatched, touched⟩
  simp [verdict, noAccusation, noDischarge, held]

/-- The corollary the card states: a dependant of a mismatched pin is neither
`Safe` nor `Unsafe`. -/
theorem a_mismatched_dependency_is_neither_safe_nor_unsafe (env : List (Declaration O I)) (o : O)
    (only : ∀ d ∈ env, d.touches o = true → d.mismatched = true)
    (some_mismatched : ∃ d ∈ env, d.mismatched = true ∧ d.touches o = true) :
    verdict env o ≠ .safe ∧ verdict env o ≠ .provisionalUnsafe := by
  rw [a_verdict_resting_on_a_mismatched_pin_is_unknown env o only some_mismatched]
  exact ⟨by simp, by simp⟩

/-- The reason it carries says nothing about the analysed source and can never
support a provisional `Unsafe`. -/
theorem the_pin_reason_never_supports_unsafe :
    Category.maySupportUnsafe .contractPinMismatch = false ∧
      Category.subject .contractPinMismatch = .analyser := ⟨rfl, rfl⟩

/-! ## Several reviewed identities -/

/-- A contract edit voids every identity: whatever is reviewed and audited, a
pinned declaration whose text changed is not in force. -/
theorem a_contract_edit_voids_every_identity (d : Declaration O I) (pinned : d.mode = .pinned)
    (edited : d.textMatches = false) (reviewed : List I) (audited : I) :
    ({ d with reviewed := reviewed, audited := audited } : Declaration O I).inForce = false := by
  have mode : ({ d with reviewed := reviewed, audited := audited } : Declaration O I).mode =
      .pinned := by
    simpa [Declaration.mode, Declaration.contractMode] using pinned
  unfold Declaration.inForce
  rw [mode]
  simp [edited]

/-- Adding an identity disturbs none: wherever the declaration was in force,
it still is, and it is also in force at the added identity. -/
theorem adding_an_identity_keeps_every_reviewed_one (d : Declaration O I) (i : I)
    (inForce : d.inForce = true) (text : d.textMatches = true) :
    (d.addIdentity i).inForce = true ∧
      ({ d.addIdentity i with audited := i } : Declaration O I).inForce = true := by
  have mode : (d.addIdentity i).mode = d.mode := by
    simp [Declaration.addIdentity, Declaration.mode, Declaration.contractMode]
  have modeAt : ({ d.addIdentity i with audited := i } : Declaration O I).mode = d.mode := by
    simp [Declaration.addIdentity, Declaration.mode, Declaration.contractMode]
  constructor
  · unfold Declaration.inForce at inForce ⊢
    rw [mode]
    split
    · rfl
    · rename_i pinned
      rw [pinned] at inForce
      simp only [Bool.and_eq_true, List.contains_iff_mem] at inForce
      simp [Declaration.addIdentity, inForce.1, inForce.2]
  · unfold Declaration.inForce
    rw [modeAt]
    split
    · rfl
    · simp [Declaration.addIdentity, text]

/-- Pinned at two reviewed identities `a` and `b` (the text unchanged): in
force at both, and out of force at any `c` that is neither. -/
theorem pinned_at_two_revisions_holds_at_both_and_nowhere_else (d : Declaration O I)
    (pinned : d.mode = .pinned) (text : d.textMatches = true) (a b c : I)
    (neither : c ≠ a ∧ c ≠ b) :
    ({ d with reviewed := [a, b], audited := a } : Declaration O I).inForce = true ∧
      ({ d with reviewed := [a, b], audited := b } : Declaration O I).inForce = true ∧
      ({ d with reviewed := [a, b], audited := c } : Declaration O I).inForce = false := by
  have mode : ∀ r x, ({ d with reviewed := r, audited := x } : Declaration O I).mode =
      .pinned := by
    intro r x
    simpa [Declaration.mode, Declaration.contractMode] using pinned
  refine ⟨?_, ?_, ?_⟩ <;> unfold Declaration.inForce <;> rw [mode] <;>
    simp [text, neither.1, neither.2]

/-! ## Re-pinning restores verdicts exactly -/

/-- A re-pinned declaration is in force and applies all of its clauses. -/
theorem Declaration.repin_in_force (d : Declaration O I) :
    d.repin.inForce = true ∧ d.repin.applied = some d.clauses := by
  have inForce : d.repin.inForce = true := by
    unfold Declaration.inForce
    split
    · rfl
    · simp [Declaration.repin]
  refine ⟨inForce, ?_⟩
  unfold Declaration.applied
  split
  · rfl
  · rename_i notInForce
    exact absurd inForce notInForce

/-- Re-pinning every declaration gives every obligation the verdict of the same
environment with every contract in force. -/
theorem repinning_restores_the_verdict_in_force (env : List (Declaration O I)) (o : O) :
    verdict (env.map Declaration.repin) o = verdictInForce env o := by
  have applied : ∀ d : Declaration O I, d.repin.applied = some d.clauses :=
    fun d => d.repin_in_force.2
  have notMismatched : ∀ d : Declaration O I, d.repin.mismatched = false := by
    intro d
    simp [Declaration.mismatched, d.repin_in_force.1]
  simp [verdict, verdictInForce, List.any_map, Function.comp_def, Declaration.accuses,
    Declaration.discharges, applied, notMismatched]

/-- Where every pin matches, the verdict already is the in-force verdict: the
matching pins of the earlier revision gave exactly this. -/
theorem matching_pins_give_the_verdict_in_force (env : List (Declaration O I)) (o : O)
    (matching : ∀ d ∈ env, d.inForce = true) :
    verdict env o = verdictInForce env o := by
  have applied : ∀ d ∈ env, d.applied = some d.clauses := by
    intro d member
    simp [Declaration.applied, matching d member]
  have notMismatched : ∀ d ∈ env, d.mismatched = false := by
    intro d member
    simp [Declaration.mismatched, matching d member]
  have accuses : env.any (·.accuses o) = env.any (fun d => d.clauses.any (·.accuses o)) := by
    apply Bool.eq_iff_iff.mpr
    simp only [List.any_eq_true]
    constructor
    · rintro ⟨d, member, hit⟩
      refine ⟨d, member, ?_⟩
      simpa [Declaration.accuses, applied d member] using hit
    · rintro ⟨d, member, hit⟩
      refine ⟨d, member, ?_⟩
      simpa [Declaration.accuses, applied d member] using hit
  have discharges :
      env.any (·.discharges o) = env.any (fun d => d.clauses.any (·.discharges o)) := by
    apply Bool.eq_iff_iff.mpr
    simp only [List.any_eq_true]
    constructor
    · rintro ⟨d, member, hit⟩
      refine ⟨d, member, ?_⟩
      simpa [Declaration.discharges, applied d member] using hit
    · rintro ⟨d, member, hit⟩
      refine ⟨d, member, ?_⟩
      simpa [Declaration.discharges, applied d member] using hit
  have held : env.any (fun d => d.mismatched && d.touches o) = false := by
    rw [List.any_eq_false]
    intro d member
    simp [notMismatched d member]
  unfold verdict verdictInForce
  rw [accuses, discharges, held]
  simp

/-- **Removing a mismatch restores verdicts exactly**: re-pinning the
declarations of an environment whose pins matched at an earlier revision gives
every obligation the verdict those matching pins gave. -/
theorem removing_a_mismatch_restores_verdicts_exactly (env before : List (Declaration O I))
    (o : O) (sameClauses : env.map (·.clauses) = before.map (·.clauses))
    (matched : ∀ d ∈ before, d.inForce = true) :
    verdict (env.map Declaration.repin) o = verdict before o := by
  rw [repinning_restores_the_verdict_in_force, matching_pins_give_the_verdict_in_force before o matched]
  have any_eq : ∀ (f : List (Clause O) → Bool),
      env.any (fun d => f d.clauses) = before.any (fun d => f d.clauses) := by
    intro f
    have := congrArg (fun l => l.any f) sameClauses
    simpa [List.any_map, Function.comp_def] using this
  simp only [verdictInForce]
  rw [any_eq (fun cs => cs.any (·.accuses o)), any_eq (fun cs => cs.any (·.discharges o))]

/-! ## Flexible re-verification -/

/-- A flexible declaration is in force at every source identity and whatever
its lock says. -/
theorem a_flexible_contract_is_in_force_at_every_identity (d : Declaration O I)
    (flexible : d.mode = .flexible) (audited : I) (reviewed : List I) (text : Bool) :
    ({ d with audited := audited, reviewed := reviewed, textMatches := text } :
      Declaration O I).inForce = true := by
  have : ({ d with audited := audited, reviewed := reviewed, textMatches := text } :
      Declaration O I).mode = .flexible := by
    simpa [Declaration.mode, Declaration.contractMode] using flexible
  simp [Declaration.inForce, this]

/-- **A checked body is still checked**: a body the run analyses under a
flexible contract is checked against exactly the declared clauses, at every
source identity and whatever the lock says — the mode never skips the check. -/
theorem a_checked_body_is_still_checked {R : Type} (check : List (Clause O) → R)
    (d : Declaration O I) (flexible : d.mode = .flexible) (body : d.hasBody = true)
    (audited : I) (reviewed : List I) (text : Bool) :
    Declaration.bodyCheck check
        ({ d with audited := audited, reviewed := reviewed, textMatches := text } :
          Declaration O I)
      = some (check d.clauses) := by
  have inForce :=
    a_flexible_contract_is_in_force_at_every_identity d flexible audited reviewed text
  have hasBody :
      ({ d with audited := audited, reviewed := reviewed, textMatches := text } :
        Declaration O I).hasBody = true := body
  unfold Declaration.bodyCheck Declaration.applied
  split
  · first
      | rfl
      | (split
         · rfl
         · rename_i notInForce
           exact absurd inForce notInForce)
  · rename_i noBody
    exact absurd hasBody noBody

/-- And it is the check a matching pin gives: a flexible contract checks the
body exactly as the same contract pinned and in force does. -/
theorem flexible_and_matching_pinned_check_alike {R : Type} (check : List (Clause O) → R)
    (flexible pinned : Declaration O I) (sameClauses : flexible.clauses = pinned.clauses)
    (isFlexible : flexible.mode = .flexible) (inForce : pinned.inForce = true)
    (bodies : flexible.hasBody = true ∧ pinned.hasBody = true) :
    Declaration.bodyCheck check flexible = Declaration.bodyCheck check pinned := by
  have flexibleInForce : flexible.inForce = true := by
    simp [Declaration.inForce, isFlexible]
  simp [Declaration.bodyCheck, Declaration.applied, flexibleInForce, inForce, bodies,
    sameClauses]

/-- The assumptions a run lists: every flexible declaration whose body it does
not analyse. -/
def assumptions (env : List (Declaration O I)) : List (Declaration O I) :=
  env.filter (fun d => d.mode = .flexible && !d.hasBody)

/-- A flexible contract on a trusted function never applies silently: it is in
force, and it is listed as an assumption of the run. -/
theorem a_trusted_flexible_contract_is_a_listed_assumption (env : List (Declaration O I))
    (d : Declaration O I) (member : d ∈ env) (flexible : d.mode = .flexible)
    (trusted : d.hasBody = false) :
    d.inForce = true ∧ d ∈ assumptions env := by
  refine ⟨by simp [Declaration.inForce, flexible], ?_⟩
  simp [assumptions, List.mem_filter, member, flexible, trusted]

/-! ## The unit reason names the pin -/

/-- The projection summary's classification: calls held for review first, then
calls held for a pin mismatch, then an unsupported operation, then residual
custody. -/
def classifyProjection (unsupported review pinned : Nat) : Category :=
  if review > 0 then .awaitingHumanReview
  else if pinned > 0 then .contractPinMismatch
  else if unsupported > 0 then .unsupportedSyntax
  else .residualCustody

/-- A summary naming a call held for a pin mismatch (and none held for review)
is `contractPinMismatch`, never a statement about the analysed source. -/
theorem a_pin_held_summary_is_never_a_source_statement (unsupported pinned : Nat)
    (positive : pinned > 0) :
    classifyProjection unsupported 0 pinned = .contractPinMismatch ∧
      Category.describesAnalysedSource (classifyProjection unsupported 0 pinned) = false := by
  simp [classifyProjection, positive, Category.describesAnalysedSource, Category.subject]

/-- Whenever any call is held, the summary says nothing about the source. -/
theorem a_held_summary_says_nothing_about_the_source (unsupported review pinned : Nat)
    (held : review > 0 ∨ pinned > 0) :
    Category.describesAnalysedSource (classifyProjection unsupported review pinned) = false := by
  unfold classifyProjection
  by_cases r : review > 0
  · simp [r, Category.describesAnalysedSource, Category.subject]
  · have p : pinned > 0 := by omega
    simp [r, p, Category.describesAnalysedSource, Category.subject]

/-- And a summary with no held call classifies exactly as before the card. -/
theorem a_summary_without_held_calls_is_unchanged (unsupported : Nat) :
    classifyProjection unsupported 0 0 =
      (if unsupported > 0 then .unsupportedSyntax else .residualCustody) := by
  simp [classifyProjection]

end QCore.ContractRevisionMode
