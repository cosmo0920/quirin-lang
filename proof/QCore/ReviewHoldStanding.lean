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

import QCore.VerdictSemigroup

/-!
# A held call never masks a reason that would stand without the hold

Card **W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL** (gateway rule 10i as
amended; language specification §L21.2). The model of
`crates/qcore-c-gateway/src/legacy_review_standing.rs`.

A call held for a human act (a declaration awaiting review, or withheld for a
pin mismatch) refuses its statement with a `Pending` reason that **replaces**
the statement's other reasons, and the held call has no effect, so nothing the
accepted declaration would make of the path is seen. Before the card a row
whose recorded reasons were only held calls was `Pending` although accepting
the declaration would leave it `Unknown` or alleging a violation.

The fix analyses the function a second time with every held declaration in
force (`resolved`) and joins each `Pending` row with every reason above
`Pending` there, recorded as an `Unknown` standing reason (`standing`): a
violation of the second analysis is `cap`ped to `Unknown`, since it rests on a
declaration nobody has accepted.

## What is proved

* **The published verdict** (`publish`, `Row.published`) is never below the
  held analysis', moves only a `Pending` row, never reaches `Safe` from a held
  row and never `Unsafe` through the second analysis.
* **Pending only from the hold.** A published `Pending` has every reason of
  both analyses `Safe` or `Pending`, and a held link for a human-resolvable
  reason.
* **A held call never removes another reason.** Every reason of the second
  analysis is, capped, below the published verdict of a row the hold touched;
  so is the second analysis' whole verdict.
* **The Rust fold is `publish`.** Folding the held reasons with the standing
  reasons appended (`Row.settled`, what `settle` records) is exactly
  `Row.published`.
* **The concrete reason** (`ConcreteReason`, card
  W47-REVIEW-HOLD-CONCRETE-REASON). A row holding a call names every marker
  of every held declaration, reason verbatim, with its category; nothing
  else is named, an unnamed hold publishes nothing, the category and the
  reasons never change a verdict, and composition keeps every reason.
* **The other holds.** A hold whose reason is not human-resolvable
  (`qcore_refuted_by_body__`, the ABI and mode withdrawals, the literal
  premise's marks) is `Unknown`, which absorbs every refusal it preempts: the
  preemption moves attribution only.

## What this is not

Which reasons each Rust analysis records, and that the second analysis is the
held one with the holds lifted, are premises here (pinned by
`crates/qcore-c-gateway/tests/review_hold_standing.rs`). The cross-unit
refutation of a declaration once accepted is outside the second analysis
(`W45-REVIEW-HOLD-CROSS-UNIT-REFUTATION`).
-/

namespace QCore.ReviewHoldStanding

open QCore.ReasonCategory
open QCore.VerdictSemigroup
open QCore.VerdictSemigroup.V

/-- A reason of the analysis with every held declaration in force, as it may
stand beside a held call: a violation there is not validated while the
declaration is held, so it counts as `Unknown`. -/
def cap : V → V
  | .provisionalUnsafe => .unknown
  | v => v

theorem cap_rank_le (v : V) : (cap v).rank ≤ 2 := by
  cases v <;> decide

theorem cap_of_rank_le {v : V} (h : v.rank ≤ 2) : cap v = v := by
  cases v <;> simp_all [cap, V.rank]

theorem cap_join (a b : V) : cap (join a b) = join (cap a) (cap b) := by
  cases a <;> cases b <;> decide

theorem cap_ne_unsafe (v : V) : cap v ≠ .provisionalUnsafe := by
  cases v <;> decide

/-- The published verdict of a row whose held analysis is `h` and whose
analysis with every held declaration in force is `r`. -/
def publish (h r : V) : V := if h = .pending then join .pending (cap r) else h

theorem publish_never_lowers (h r : V) : h.rank ≤ (publish h r).rank := by
  unfold publish
  split
  · subst_vars; exact le_join_left _ _
  · exact Nat.le_refl _

theorem only_a_pending_row_moves {h : V} (r : V) (notPending : h ≠ .pending) :
    publish h r = h := by
  simp [publish, notPending]

theorem a_held_row_is_never_published_safe {h : V} (r : V) (held : h = .pending) :
    publish h r ≠ .safe := by
  subst held
  cases r <;> simp [publish, cap, V.join, V.rank]

theorem the_second_analysis_never_publishes_unsafe {h r : V}
    (isUnsafe : publish h r = .provisionalUnsafe) : h = .provisionalUnsafe := by
  unfold publish at isUnsafe
  split at isUnsafe
  · subst_vars; cases r <;> simp_all [cap, V.join, V.rank]
  · exact isUnsafe

/-- `Pending` is published only when the held analysis is `Pending` and the
second analysis is `Safe` or itself only held. -/
theorem pending_only_if_the_resolution_is_safe {h r : V} (pending : publish h r = .pending) :
    h = .pending ∧ (r = .safe ∨ r = .pending) := by
  unfold publish at pending
  split at pending
  · rename_i held
    refine ⟨held, ?_⟩
    cases r <;> simp_all [cap, V.join, V.rank]
  · rename_i notPending; exact absurd pending notPending

/-- The published verdict of a row the hold touched is never below the
second analysis' capped verdict. -/
theorem never_below_the_resolution {h : V} (r : V) (touched : h ≠ .safe) :
    (cap r).rank ≤ (publish h r).rank := by
  unfold publish
  split
  · exact le_join_right _ _
  · have := cap_rank_le r
    cases h <;> simp_all [V.rank] <;> omega

/-! ## The other holds -/

/-- A hold whose reason is not human-resolvable is `Unknown`, and it absorbs
every refusal it preempts: holding the statement alone and holding it beside
those refusals fold to the same verdict. -/
theorem an_unknown_hold_absorbs_every_refusal {c : Category}
    (notResolvable : Category.isHumanResolvable c = false) (s : List V)
    (refusals : ∀ v ∈ s, v.rank ≤ 2) :
    fold ((Link.held c).value :: s) = fold [(Link.held c).value] := by
  have value : (Link.held c).value = .unknown := by simp [Link.value, notResolvable]
  rw [value]
  induction s with
  | nil => rfl
  | cons x xs ih =>
    have hx := refusals x (List.mem_cons.mpr (Or.inl rfl))
    have rest : ∀ v ∈ xs, v.rank ≤ 2 := fun v m => refusals v (List.mem_cons.mpr (Or.inr m))
    have ih := ih rest
    simp only [fold] at ih ⊢
    cases x <;> simp_all [V.join, V.rank] <;> cases hf : fold xs <;>
      simp_all

/-! ## The row: the held reasons, the standing reasons and their fold -/

/-- The standing reasons `settle` records for a second analysis' reasons:
one `Unknown` for every reason above `Pending`. -/
def standing (rs : List V) : List V :=
  (rs.filter (fun v => decide (2 ≤ v.rank))).map (fun _ => V.unknown)

theorem fold_standing_rank_le (rs : List V) : (fold (standing rs)).rank ≤ 2 := by
  induction rs with
  | nil => decide
  | cons x xs ih =>
    unfold standing at *
    by_cases hx : 2 ≤ x.rank
    · simp only [List.filter_cons, hx, decide_true, ite_true, List.map_cons, fold]
      rw [join_rank]; exact Nat.max_le.mpr ⟨by decide, ih⟩
    · simp only [List.filter_cons, hx, decide_false]
      simpa using ih

theorem join_unknown_of_rank_le {a : V} (h : a.rank ≤ 2) : join .unknown a = .unknown := by
  cases a <;> simp_all [V.join, V.rank]

/-- Joined with `Pending`, the standing reasons fold to the capped second
analysis. -/
theorem pending_join_standing (rs : List V) :
    join .pending (fold (standing rs)) = join .pending (cap (fold rs)) := by
  induction rs with
  | nil => decide
  | cons x xs ih =>
    have bound := fold_standing_rank_le xs
    unfold standing at ih bound ⊢
    by_cases hx : 2 ≤ x.rank
    · simp only [List.filter_cons, hx, decide_true, ite_true, List.map_cons, fold]
      rw [cap_join]
      have capx : cap x = .unknown := by cases x <;> simp_all [cap, V.rank]
      rw [capx, join_unknown_of_rank_le bound, join_unknown_of_rank_le (cap_rank_le _)]
    · simp only [List.filter_cons, hx, decide_false]
      simp only [fold]
      rw [cap_join, ← join_assoc]
      have absorbed : join .pending (cap x) = .pending := by
        cases x <;> simp_all [cap, V.join, V.rank]
      rw [absorbed]
      simpa using ih

/-- One function: the reasons its held analysis records and the reasons its
analysis with every held declaration in force records. -/
structure Row where
  held : List Link
  resolved : List Link

namespace Row

def heldVerdict (row : Row) : V := fold (row.held.map Link.value)

def resolvedVerdict (row : Row) : V := fold (row.resolved.map Link.value)

def published (row : Row) : V := publish row.heldVerdict row.resolvedVerdict

/-- What `settle` records: a `Pending` row's reasons with the standing
reasons appended; every other row unchanged. -/
def settled (row : Row) : List V :=
  if row.heldVerdict = .pending then
    row.held.map Link.value ++ standing (row.resolved.map Link.value)
  else row.held.map Link.value

/-- The Rust fold over the settled reasons is the published verdict. -/
theorem the_settled_fold_is_published (row : Row) : fold row.settled = row.published := by
  unfold settled published publish
  split
  · rename_i held
    rw [fold_append]
    change join row.heldVerdict _ = _
    rw [held, pending_join_standing]
    rfl
  · rfl

theorem value_rank_le_of_fold {x : V} {xs : List V} (member : x ∈ xs) :
    x.rank ≤ (fold xs).rank :=
  every_link_is_below_the_fold member

/-- A row is published `Pending` only if every reason of both analyses is
`Safe` or `Pending`, and a held link names a human-resolvable reason. -/
theorem pending_only_if_every_reason_is_a_hold (row : Row) (pending : row.published = .pending) :
    (∀ l ∈ row.held, l.value = .safe ∨ l.value = .pending) ∧
      (∀ l ∈ row.resolved, l.value = .safe ∨ l.value = .pending) ∧
      ∃ reason, Link.held reason ∈ row.held ∧ Category.isHumanResolvable reason = true := by
  obtain ⟨held, resolved⟩ := pending_only_if_the_resolution_is_safe pending
  have low : ∀ (ls : List Link), (fold (ls.map Link.value) = .safe ∨
      fold (ls.map Link.value) = .pending) → ∀ l ∈ ls, l.value = .safe ∨ l.value = .pending := by
    intro ls h l member
    have := value_rank_le_of_fold (List.mem_map_of_mem (f := Link.value) member)
    rcases h with h | h <;> rw [h] at this <;> cases hv : l.value <;> simp_all [V.rank]
  refine ⟨low row.held (Or.inr held), low row.resolved resolved, ?_⟩
  exact pending_only_from_a_human_resolvable_reason row.held held

/-- A held call never removes another reason: on a row the hold touched,
every reason the second analysis records is, capped, below the published
verdict. -/
theorem a_held_call_never_removes_another_reason (row : Row) (touched : row.heldVerdict ≠ .safe)
    {l : Link} (member : l ∈ row.resolved) : (cap l.value).rank ≤ row.published.rank := by
  have below : (cap l.value).rank ≤ (cap row.resolvedVerdict).rank := by
    have := value_rank_le_of_fold (List.mem_map_of_mem (f := Link.value) member)
    have mono : ∀ a b : V, a.rank ≤ b.rank → (cap a).rank ≤ (cap b).rank := by
      intro a b hab; cases a <;> cases b <;> simp_all [cap, V.rank]
    exact mono _ _ this
  exact Nat.le_trans below (never_below_the_resolution _ touched)

/-- The published verdict of a row the hold touched is never below the
verdict of its analysis without the hold's rewrite (capped). -/
theorem never_below_the_verdict_without_the_rewrite (row : Row)
    (touched : row.heldVerdict ≠ .safe) :
    (cap row.resolvedVerdict).rank ≤ row.published.rank :=
  never_below_the_resolution _ touched

theorem never_lowers_the_held_verdict (row : Row) :
    row.heldVerdict.rank ≤ row.published.rank :=
  publish_never_lowers _ _

/-- The defect's shape: one held call whose statement also carries a refusal
that stands with the declaration in force. Before the card the row was
`Pending`; it is published `Unknown`. -/
theorem a_held_call_beside_a_standing_refusal_is_unknown (c : Category) (refusal : Category)
    (resolvable : Category.isHumanResolvable c = true) :
    (Row.mk [Link.held c] [Link.unresolved refusal]).heldVerdict = .pending ∧
      (Row.mk [Link.held c] [Link.unresolved refusal]).published = .unknown := by
  constructor
  · simp [heldVerdict, fold, Link.value, resolvable, join_safe]
  · simp [published, publish, heldVerdict, resolvedVerdict, fold, Link.value, resolvable,
      join_safe, cap]
    decide

/-- A violation the accepted declaration would allege keeps the row
`Unknown`: never `Pending`, never `Unsafe`. -/
theorem an_alleged_violation_is_unknown (c : Category)
    (resolvable : Category.isHumanResolvable c = true) :
    (Row.mk [Link.held c] [Link.violation]).published = .unknown := by
  simp [published, publish, heldVerdict, resolvedVerdict, fold, Link.value, resolvable,
    join_safe, cap]
  decide

end Row

/-! ## The concrete reason of every review hold, on the row it holds

Card **W47-REVIEW-HOLD-CONCRETE-REASON** (gateway rule 10i as amended,
contract specification §C06.1; user decision 2026-09-29). The model of
`crates/qcore-audit/src/review_hold_reasons.rs` and of the loader's marker
reading (`review_marker` in `crates/qcore-contract/src/contract_loader.rs`).

A row holding a call to a declaration awaiting review publishes, beside its
verdict, every marker of every held declaration: the reason verbatim and a
category read from the marker's structure (a trailing
`rewrite_before_acceptance`). Proved: the loader refuses a blank reason and
keeps a loaded one verbatim; the published reasons contain every held
declaration's every marker and nothing the environment does not declare; a
held callee with no withheld declaration or no marker publishes nothing
(fail-closed); the verdict is the fold of the row's values whatever its
reasons and categories, so adding reasons never changes it and a published
`Pending` row stays `Pending` with every hold named; composing rows (the
report merge, the checkpoint composition) keeps every row's reasons.

Premises stated, not proved: the held callees are exactly the ones the
gateway's minted refusal names (rule 10i), and a checkpoint stores and
renders a row's members verbatim (pinned by
`crates/qcore-audit/tests/checkpoint_cli.rs`). -/

namespace ConcreteReason

/-- What resolving a marker takes, read from its structure. -/
inductive Disposition where
  | acceptableAsWrittenAfterReview
  | mustBeRewrittenBeforeAcceptance
  deriving DecidableEq, Repr

/-- One review marker as the loader records it: its reason text and whether
it ends with `rewrite_before_acceptance`. -/
structure Marker where
  reason : String
  rewrite : Bool
  deriving DecidableEq, Repr

def Marker.category (m : Marker) : Disposition :=
  if m.rewrite then .mustBeRewrittenBeforeAcceptance else .acceptableAsWrittenAfterReview

theorem Marker.category_eq_rewrite_iff (m : Marker) :
    m.category = .mustBeRewrittenBeforeAcceptance ↔ m.rewrite = true := by
  unfold Marker.category
  cases m.rewrite <;> simp

/-- A reason no one could read: empty after trimming (every character, if
any, is white space). -/
def blank (reason : String) : Bool := reason.all Char.isWhitespace

/-- The loader's reading of one marker: a blank reason refuses (the module
is rejected whole); otherwise the reason is kept verbatim. -/
def load (reason : String) (rewrite : Bool) : Option Marker :=
  if blank reason then none else some ⟨reason, rewrite⟩

theorem a_blank_reason_is_refused {reason : String} (rewrite : Bool)
    (isBlank : blank reason = true) : load reason rewrite = none := by
  simp [load, isBlank]

theorem a_loaded_marker_keeps_its_reason_verbatim {reason : String} {rewrite : Bool}
    {m : Marker} (loaded : load reason rewrite = some m) :
    m.reason = reason ∧ blank m.reason = false ∧ m.rewrite = rewrite := by
  unfold load at loaded
  by_cases b : blank reason = true
  · simp [b] at loaded
  · simp [b] at loaded
    subst loaded
    exact ⟨rfl, by simpa using b, rfl⟩

/-- One published hold: the held declaration and one of its markers. -/
structure Hold where
  declaration : String
  marker : Marker
  deriving DecidableEq, Repr

/-- The environment: the markers of each declaration withheld for review,
`none` for a name it does not withhold. -/
abbrev Env := String → Option (List Marker)

/-- `row_review_holds`: every marker of every held callee, in order; `none`
when some callee is not withheld or is withheld with no marker. -/
def holdsOf (env : Env) : List String → Option (List Hold)
  | [] => some []
  | d :: ds =>
    match env d, holdsOf env ds with
    | some (m :: ms), some rest => some ((m :: ms).map (Hold.mk d) ++ rest)
    | _, _ => none

/-- Every marker of every held declaration is published. -/
theorem holdsOf_names_every_marker {env : Env} {ds : List String} {hs : List Hold}
    (published : holdsOf env ds = some hs) {d : String} (held : d ∈ ds) {ms : List Marker}
    (withheld : env d = some ms) {m : Marker} (marked : m ∈ ms) : Hold.mk d m ∈ hs := by
  induction ds generalizing hs with
  | nil => simp at held
  | cons e es ih =>
    cases he : env e with
    | none => simp [holdsOf, he] at published
    | some es' =>
      cases es' with
      | nil => simp [holdsOf, he] at published
      | cons n ns =>
        cases hr : holdsOf env es with
        | none => simp [holdsOf, he, hr] at published
        | some rest =>
          simp only [holdsOf, he, hr, Option.some.injEq] at published
          subst published
          rcases List.mem_cons.mp held with same | tail
          · subst same
            rw [he, Option.some.injEq] at withheld
            subst withheld
            exact List.mem_append_left _ (List.mem_map.mpr ⟨m, marked, rfl⟩)
          · exact List.mem_append_right _ (ih hr tail)

/-- Nothing is published that the environment does not declare. -/
theorem holdsOf_invents_nothing {env : Env} {ds : List String} {hs : List Hold}
    (published : holdsOf env ds = some hs) {h : Hold} (member : h ∈ hs) :
    h.declaration ∈ ds ∧ ∃ ms, env h.declaration = some ms ∧ h.marker ∈ ms := by
  induction ds generalizing hs with
  | nil =>
    simp only [holdsOf, Option.some.injEq] at published
    subst published
    simp at member
  | cons e es ih =>
    cases he : env e with
    | none => simp [holdsOf, he] at published
    | some es' =>
      cases es' with
      | nil => simp [holdsOf, he] at published
      | cons n ns =>
        cases hr : holdsOf env es with
        | none => simp [holdsOf, he, hr] at published
        | some rest =>
          simp only [holdsOf, he, hr, Option.some.injEq] at published
          subst published
          rcases List.mem_append.mp member with here | later
          · obtain ⟨m, marked, rfl⟩ := List.mem_map.mp here
            exact ⟨List.mem_cons_self .., n :: ns, he, marked⟩
          · obtain ⟨inTail, ms, withheld, marked⟩ := ih hr later
            exact ⟨List.mem_cons_of_mem _ inTail, ms, withheld, marked⟩

/-- Fail-closed: a held callee the environment does not withhold, or
withholds with no marker, publishes no row at all. -/
theorem holdsOf_refuses_an_unnamed_hold {env : Env} {ds : List String} {d : String}
    (held : d ∈ ds) (unnamed : env d = none ∨ env d = some []) : holdsOf env ds = none := by
  induction ds with
  | nil => simp at held
  | cons e es ih =>
    rcases List.mem_cons.mp held with same | tail
    · subst same
      rcases unnamed with u | u <;> simp [holdsOf, u]
    · have rest := ih tail
      cases he : env e with
      | none => simp [holdsOf, he]
      | some es' =>
        cases es' with
        | nil => simp [holdsOf, he]
        | cons n ns => simp [holdsOf, he, rest]

/-- A row holding at least one call names at least one concrete reason. -/
theorem holdsOf_nonempty {env : Env} {d : String} {ds : List String} {hs : List Hold}
    (published : holdsOf env (d :: ds) = some hs) : hs ≠ [] := by
  cases he : env d with
  | none => simp [holdsOf, he] at published
  | some ms =>
    cases ms with
    | nil => simp [holdsOf, he] at published
    | cons n ns =>
      cases hr : holdsOf env ds with
      | none => simp [holdsOf, he, hr] at published
      | some rest =>
        simp only [holdsOf, he, hr, Option.some.injEq] at published
        subst published
        simp

/-- A published row: the values its verdict folds and the holds it lists. -/
structure Published where
  values : List V
  holds : List Hold

def Published.verdict (p : Published) : V := fold p.values

/-- `annotate_unit` on one row. -/
def annotate (env : Env) (callees : List String) (values : List V) : Option Published :=
  (holdsOf env callees).map (Published.mk values)

theorem annotate_holds {env : Env} {cs : List String} {values : List V} {p : Published}
    (annotated : annotate env cs values = some p) :
    p.values = values ∧ holdsOf env cs = some p.holds := by
  unfold annotate at annotated
  cases h : holdsOf env cs with
  | none => simp [h] at annotated
  | some hs =>
    simp only [h, Option.map_some, Option.some.injEq] at annotated
    subst annotated
    exact ⟨rfl, rfl⟩

/-- The annotation never changes a verdict. -/
theorem annotation_keeps_the_verdict {env : Env} {cs : List String} {values : List V}
    {p : Published} (annotated : annotate env cs values = some p) :
    p.verdict = fold values := by
  unfold Published.verdict
  rw [(annotate_holds annotated).1]

/-- Adding reasons never changes the verdict value. -/
theorem adding_reasons_never_changes_the_verdict (p : Published) (extra : List Hold) :
    (Published.mk p.values (p.holds ++ extra)).verdict = fold p.values := by
  rfl

/-- The category never enters the verdict: two environments that differ in
anything (a disposition included) annotate a row to the same verdict. -/
theorem the_category_never_changes_the_verdict {env env' : Env} {cs : List String}
    {values : List V} {p p' : Published} (annotated : annotate env cs values = some p)
    (annotated' : annotate env' cs values = some p') : p.verdict = p'.verdict := by
  rw [annotation_keeps_the_verdict annotated, annotation_keeps_the_verdict annotated']

/-- The reasons of a published row contain every held declaration's every
marker, reason verbatim and category with it. -/
theorem every_hold_names_its_concrete_reason {env : Env} {cs : List String}
    {values : List V} {p : Published} (annotated : annotate env cs values = some p)
    {d : String} (held : d ∈ cs) {ms : List Marker} (withheld : env d = some ms)
    {m : Marker} (marked : m ∈ ms) : Hold.mk d m ∈ p.holds :=
  holdsOf_names_every_marker (annotate_holds annotated).2 held withheld marked

/-- Two holds on one row: both are listed. -/
theorem two_holds_on_one_row_list_both {env : Env} {d₁ d₂ : String} {m₁ m₂ : Marker}
    {values : List V} {p : Published} (annotated : annotate env [d₁, d₂] values = some p)
    (first : env d₁ = some [m₁]) (second : env d₂ = some [m₂]) :
    Hold.mk d₁ m₁ ∈ p.holds ∧ Hold.mk d₂ m₂ ∈ p.holds :=
  ⟨every_hold_names_its_concrete_reason annotated (by simp) first (by simp),
    every_hold_names_its_concrete_reason annotated (by simp) second (by simp)⟩

/-- Fail-closed: a row holding a call whose reason the environment cannot
name is not published. -/
theorem an_unnamed_hold_is_not_published {env : Env} {cs : List String} {values : List V}
    {d : String} (held : d ∈ cs) (unnamed : env d = none ∨ env d = some []) :
    annotate env cs values = none := by
  simp [annotate, holdsOf_refuses_an_unnamed_hold held unnamed]

/-- Every published reason was declared, loaded non-blank, and is kept verbatim. -/
theorem every_published_reason_is_a_loaded_one {env : Env}
    (loaded : ∀ d ms, env d = some ms → ∀ m ∈ ms, blank m.reason = false)
    {cs : List String} {values : List V} {p : Published}
    (annotated : annotate env cs values = some p) {h : Hold} (member : h ∈ p.holds) :
    h.declaration ∈ cs ∧ blank h.marker.reason = false := by
  obtain ⟨held, ms, withheld, marked⟩ :=
    holdsOf_invents_nothing (annotate_holds annotated).2 member
  exact ⟨held, loaded _ _ withheld _ marked⟩

/-- The user decision, end to end: a row the held-call standing rule
publishes `Pending` (`Row.published`), annotated over what `settle` records
(`Row.settled`), stays `Pending` and names every hold's concrete reason,
and a row holding at least one call names at least one. -/
theorem a_pending_row_stays_pending_and_names_every_hold (row : Row) {env : Env}
    {cs : List String} {p : Published} (annotated : annotate env cs row.settled = some p)
    (pending : row.published = .pending) :
    p.verdict = .pending ∧
      (∀ d ∈ cs, ∀ ms, env d = some ms → ∀ m ∈ ms, Hold.mk d m ∈ p.holds) ∧
      (cs ≠ [] → p.holds ≠ []) := by
  refine ⟨?_, fun d held ms withheld m marked =>
    every_hold_names_its_concrete_reason annotated held withheld marked, ?_⟩
  · rw [annotation_keeps_the_verdict annotated, Row.the_settled_fold_is_published, pending]
  · intro nonempty
    obtain ⟨d, ds, rfl⟩ := List.exists_cons_of_ne_nil nonempty
    exact holdsOf_nonempty (annotate_holds annotated).2

/-! ### Composition keeps every row's reasons -/

def Published.reasoned (p : Published) : Reasoned Hold := ⟨p.verdict, p.holds⟩

/-- Rows composed into a unit, a report or a checkpoint composition: the join. -/
def compose : List (Reasoned Hold) → Reasoned Hold
  | [] => Reasoned.identity
  | r :: rs => Reasoned.join r (compose rs)

theorem compose_value (rs : List (Reasoned Hold)) :
    (compose rs).value = fold (rs.map Reasoned.value) := by
  induction rs with
  | nil => rfl
  | cons r rs ih => simp [compose, Reasoned.join, fold, ih]

theorem compose_append (a b : List (Reasoned Hold)) :
    compose (a ++ b) = Reasoned.join (compose a) (compose b) := by
  induction a with
  | nil => simp [compose, Reasoned.identity_join]
  | cons r rs ih => simp [compose, ih, Reasoned.join_assoc]

/-- A report merge (`AuditReport::absorb`) or a checkpoint composition never
loses a row's reason. -/
theorem composition_never_loses_a_reason {rs : List (Reasoned Hold)} {r : Reasoned Hold}
    (member : r ∈ rs) {h : Hold} (named : h ∈ r.reasons) : h ∈ (compose rs).reasons := by
  induction rs with
  | nil => simp at member
  | cons s ss ih =>
    rcases List.mem_cons.mp member with same | later
    · subst same
      exact Reasoned.join_keeps_reasons _ _ h (Or.inl named)
    · exact Reasoned.join_keeps_reasons _ _ h (Or.inr (ih later))

/-- Merging two reports keeps both reports' reasons. -/
theorem a_merge_keeps_both_reports_reasons (a b : List (Reasoned Hold)) :
    (compose (a ++ b)).reasons = (compose a).reasons ++ (compose b).reasons := by
  rw [compose_append]
  rfl

end ConcreteReason

end QCore.ReviewHoldStanding
