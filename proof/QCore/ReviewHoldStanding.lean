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

/-- The observed shape: one held call whose statement also carries a refusal
that stands with the declaration in force. Before the card the row was
`Pending`; it is published `Unknown`. -/
theorem the_observed_shape_is_unknown (c : Category) (refusal : Category)
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

end QCore.ReviewHoldStanding
