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

import QCore.SettledLeakJoined

/-!
# A leafless mark is decided from settled states, with an `Unknown` reason when unwitnessed (card SETTLED-LEAK-MARK-LEAFLESS)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_mark_leafless.rs`, gateway rule 84 amended
(user decision of 2026-11-28, option (b)).

Rule 84 decides a marked leak report beside a blocking leaf, `Unknown` or
`Pending` (`QCore.SettledLeakJoined.decisionAt`). Beside no blocking leaf the
mark comes from a modelled effect, not a refusal (rule 43's
`ElementsReleased`), and rule 74c's held report decided the identity, so the
leak was published exactly when some iterate had claimed it
(`QCore.SettledLeakJoined.a_mark_beside_no_blocking_leaf_is_left_to_rule_74c`).

This card decides such an identity when the settled exit would claim it but
for the mark (`claimable`: live, not exempt, no reference family). It is
published as a leak exactly when rule 84's witness carries it, and otherwise
the function records the `Unknown` reason "mark without refusal".

## What is proved

* **The reason is `Unknown`**: never `Safe`, never `Unsafe`
  (`the_reason_is_unknown`, `the_reason_is_never_safe_or_unsafe`).
* **A decided candidate is never `Safe`**: it leaves either a leak or the
  reason (`a_leafless_decision_is_never_safe`); a withdrawal in a row with no
  other leaf is exactly `Unknown`, never `Unsafe`
  (`a_withdrawal_in_a_leafless_row_is_unknown`).
* **Schedule independence**: two schedules of the same runs decide the same
  whenever the function carries a blocking leaf, the identity is claimable, or
  the settled exit is unmarked (`outcome_is_order_independent`), where the
  base decision read the schedule (`the_leafless_case_read_the_schedule`).
* **Agreement with the existing rule**: beside a blocking leaf (a refusal or
  a hold) the reason never fires and the decision is rule 84's
  (`agrees_with_rule_84_beside_a_blocking_leaf`).
* **The witness requirement stands**: a witnessed candidate is published,
  as rule 84 publishes a refusal-free path (`a_witnessed_mark_is_published`).
* **The fixtures**: the open arm beside a drained arm publishes in both
  orders; the open arm through a retained loop exit records the reason in
  both orders (`both_orders_publish_the_open_arm`,
  `both_orders_record_the_reason`). `outcomeVectors` is the table the Rust
  `OUTCOME_VECTORS` reads (`outcome_vectors_hold`).

## What this is not

A proof that the Rust's `claimable` and witness are computed faithfully (rule
84's trust boundary 28, pinned by
`crates/qcore-c-gateway/tests/unit/legacy_mark_leafless.rs` and the fixtures
in `crates/qcore-c-gateway/tests/cross_unit_elements_released/mark_leafless.rs`),
nor a claim about any upstream library. An identity the settled exit does
not claim (exempt, not live) is outside this rule and left to rule 74c
(`an_unclaimable_mark_is_left_to_rule_74c`).
-/

namespace QCore.SettledLeakLeafless

open QCore.VerdictSemigroup (V fold)
open QCore.SettledLeakJoined

/-- Whether this card decides a marked identity (`leafless` in the Rust): no
blocking leaf, a mark at the settled exit, and the exit would claim it but for
the mark. -/
def leafless (blockingLeaf marked claimable : Bool) : Bool := !blockingLeaf && marked && claimable

/-- A candidate's outcome (`outcome` in the Rust): (leak published, reason
recorded). -/
def outcome (witnessed : Bool) : Bool × Bool := (witnessed, !witnessed)

/-- The whole decision at one (exit, identity): rule 84 beside a blocking
leaf, this card for a claimable leafless mark, rule 74c's held report
otherwise. Returns (leak published, reason recorded). -/
def decideAt (unknownLeaf pendingLeaf claimable : Bool) (order : List KRun) : Bool × Bool :=
  if leafless (blocking unknownLeaf pendingLeaf) (kMarked order) claimable then
    outcome (kWitnessed order)
  else
    (decisionAt unknownLeaf pendingLeaf order, false)

/-- The reason's leaf. -/
def reasonLeaf : Leaf := ⟨.unknown, "mark-without-refusal"⟩

theorem the_reason_is_unknown : reasonLeaf.verdict = .unknown := rfl

theorem the_reason_is_never_safe_or_unsafe :
    reasonLeaf.verdict ≠ .safe ∧ reasonLeaf.verdict ≠ .provisionalUnsafe := by
  decide

/-- The settle step of a candidate: the function's leaves are kept, and the
leak or the reason is appended. -/
def settleLeafless (leaves : List Leaf) (witnessed : Bool) : List Leaf :=
  leaves ++ (if (outcome witnessed).1 then [⟨.provisionalUnsafe, "leak"⟩] else []) ++
    (if (outcome witnessed).2 then [reasonLeaf] else [])

/-- **Never `Safe`.** A decided candidate leaves a leak or the reason. -/
theorem a_leafless_decision_is_never_safe (leaves : List Leaf) (witnessed : Bool) :
    fold ((settleLeafless leaves witnessed).map (·.verdict)) ≠ .safe := by
  cases witnessed
  · apply QCore.VerdictSemigroup.a_non_safe_link_keeps_the_chain_non_safe (x := V.unknown)
    · simp [settleLeafless, outcome, reasonLeaf]
    · decide
  · apply QCore.VerdictSemigroup.a_non_safe_link_keeps_the_chain_non_safe
      (x := V.provisionalUnsafe)
    · simp [settleLeafless, outcome]
    · decide

/-- **A withdrawal is `Unknown`, never `Unsafe`.** In a row whose other
leaves are all `Safe` (a leafless row), an unwitnessed candidate folds to
exactly `Unknown`. -/
theorem a_withdrawal_in_a_leafless_row_is_unknown (leaves : List Leaf)
    (safe : ∀ l ∈ leaves, l.verdict = .safe) :
    fold ((settleLeafless leaves false).map (·.verdict)) = .unknown := by
  have h : fold (leaves.map (·.verdict)) = .safe :=
    (QCore.VerdictSemigroup.fold_eq_safe_iff _).mpr (by
      intro x hx
      obtain ⟨l, lm, rfl⟩ := List.mem_map.mp hx
      exact safe l lm)
  simp only [settleLeafless, outcome, Bool.not_false, ite_true, Bool.false_eq_true, ite_false,
    List.append_nil, List.map_append, QCore.VerdictSemigroup.fold_append, h]
  decide

/-- A witnessed candidate is published, as rule 84 publishes a refusal-free
path. -/
theorem a_witnessed_mark_is_published (claimable : Bool) (order : List KRun)
    (marked : kMarked order = true) (cl : claimable = true) (w : kWitnessed order = true) :
    decideAt false false claimable order = (true, false) := by
  simp [decideAt, leafless, blocking, marked, cl, w, outcome]

/-- **Agreement.** Beside a blocking leaf the reason never fires and the
decision is rule 84's (`decisionAt`). -/
theorem agrees_with_rule_84_beside_a_blocking_leaf (unknownLeaf pendingLeaf claimable : Bool)
    (order : List KRun) (b : blocking unknownLeaf pendingLeaf = true) :
    decideAt unknownLeaf pendingLeaf claimable order =
      (decisionAt unknownLeaf pendingLeaf order, false) := by
  simp [decideAt, leafless, b]

/-- **Schedule independence.** Two schedules of the same runs decide the same
(leak and reason), whenever the function carries a blocking leaf, the
identity is claimable, or the settled exit is unmarked. -/
theorem outcome_is_order_independent (unknownLeaf pendingLeaf claimable : Bool)
    (o₁ o₂ : List KRun) (perm : o₁.Perm o₂)
    (scope : blocking unknownLeaf pendingLeaf = true ∨ claimable = true ∨
      kMarked o₁ = false) :
    decideAt unknownLeaf pendingLeaf claimable o₁ = decideAt unknownLeaf pendingLeaf claimable o₂ := by
  have hm : kMarked o₁ = kMarked o₂ := any_perm _ perm
  have hw : kWitnessed o₁ = kWitnessed o₂ := any_perm _ perm
  have rest : decisionAt unknownLeaf pendingLeaf o₁ = decisionAt unknownLeaf pendingLeaf o₂ ∨
      leafless (blocking unknownLeaf pendingLeaf) (kMarked o₁) claimable = true := by
    cases hb : blocking unknownLeaf pendingLeaf
    · cases hk : kMarked o₁
      · left
        apply publication_is_order_independent_beside_a_hold _ _ _ _ perm
        right; right; exact hk
      · cases hc : claimable
        · rcases scope with h | h | h
          · simp [hb] at h
          · simp [hc] at h
          · simp [hk] at h
        · right; simp [leafless]
    · left
      apply publication_is_order_independent_beside_a_hold _ _ _ _ perm
      unfold blocking at hb
      cases hu : unknownLeaf
      · right; left; simpa [hu] using hb
      · left; rfl
  unfold decideAt
  rw [← hm, ← hw]
  rcases rest with h | h
  · by_cases hl : leafless (blocking unknownLeaf pendingLeaf) (kMarked o₁) claimable = true
    · simp [hl]
    · simp [hl, h]
  · simp [h]

/-- Without a blocking leaf and with an identity the exit does not claim, the
decision is rule 74c's held report, as before. -/
theorem an_unclaimable_mark_is_left_to_rule_74c (order : List KRun)
    (marked : kMarked order = true) :
    decideAt false false false order = (held order, false) := by
  simp [decideAt, leafless, a_mark_beside_no_blocking_leaf_is_left_to_rule_74c order marked]

/-- A drained arm: rule 43's modelled mark, no refusal. -/
def drainedArm : KRun := ⟨[⟨true, none⟩], false⟩

/-- An open arm through a retained loop exit edge: `unmodeled` blocks the
replay as a mark does, with no refusal leaf at the settle point. -/
def retainedArm : KRun := ⟨[⟨true, none⟩], true⟩

/-- **The hazard, reproduced.** Beside no blocking leaf, the base decision
was rule 74c's held report: the open arm first was published, the drained
arm first was not. -/
theorem the_leafless_case_read_the_schedule :
    decisionAt false false [avoidingArm, drainedArm] = true ∧
      decisionAt false false [drainedArm, avoidingArm] = false := by
  decide

/-- The open, refusal-free arm is witnessed: both orders publish. -/
theorem both_orders_publish_the_open_arm :
    decideAt false false true [avoidingArm, drainedArm] = (true, false) ∧
      decideAt false false true [drainedArm, avoidingArm] = (true, false) := by
  decide

/-- The open arm through a retained exit is not witnessed: both orders
record the reason and publish no leak. -/
theorem both_orders_record_the_reason :
    decideAt false false true [retainedArm, drainedArm] = (false, true) ∧
      decideAt false false true [drainedArm, retainedArm] = (false, true) := by
  decide

/-- `(blocking leaf, marked, claimable, witnessed, decided, leak, reason)`;
the Rust `OUTCOME_VECTORS` is read from this table by
`tests/unit/legacy_mark_leafless.rs`. A row this card does not decide
records neither. -/
def outcomeVectors : List (Bool × Bool × Bool × Bool × Bool × Bool × Bool) := [
  (false, true, true, true, true, true, false),
  (false, true, true, false, true, false, true),
  (false, true, false, false, false, false, false),
  (false, false, true, false, false, false, false),
  (true, true, true, true, false, false, false),
  (true, true, true, false, false, false, false)
]

theorem outcome_vectors_hold :
    outcomeVectors.all (fun r =>
      let decided := leafless r.1 r.2.1 r.2.2.1
      let o := if decided then outcome r.2.2.2.1 else (false, false)
      decided == r.2.2.2.2.1 && o.1 == r.2.2.2.2.2.1 && o.2 == r.2.2.2.2.2.2) = true := by
  decide

end QCore.SettledLeakLeafless
