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
import QCore.ContractDeferredReview

/-!
# The four-value verdict semigroup

Card **W45-USER-CONTRACT-PLUGINS** (language specification §L21.2, contract
specification §C11.1). Verdicts are `Safe < Pending < Unknown < Unsafe`, and
every roll-up — a function over its diagnostics and the contracts it applies, a
unit over its functions, a chain over its links — is the **join** (the
maximum). This is the model of `crates/qcore-schemas/src/verdict.rs` (the
algebra), `crates/qcore-c-gateway/src/legacy_verdict.rs` (which diagnostic is
which value) and `crates/qcore-verifier-host/src/plugin_firewall.rs` (which
checker-plugin answer is which value).

## What is proved

* **The algebra.** The join is associative, commutative and idempotent with
  `Safe` as its identity, and the fold of a chain is the join of its parts; the
  reasoned verdict (a value with a multiset of reasons) is again associative
  with `Safe` and no reason as its identity, and commutative up to the order
  of reasons.
* **Monotonicity.** Adding a link to a chain never lowers its verdict, and a
  chain is `Safe` exactly when every link is.
* **Pending is fail-closed.** A chain with a `Pending` link is never `Safe`, and
  a chain is `Pending` only when some link was withheld for a named
  human-resolvable reason (`isHumanResolvable`).
* **Agreement with the historical roll-up.** Read through the projection that
  sends `Pending` to the three-valued `Unknown`, the four-valued fold is exactly
  the three-way decision the gateway made before the card (`Unsafe` if any, else
  `Unknown` if any, else `Safe`); on inputs with no `Pending` the two coincide.
  The deferred-review model's `Unknown (awaitingHumanReview)` verdict is the new
  `Pending`.
* **The plugin firewall.** A plugin's `Safe` is `Safe` only with a certificate
  the audit's own checker accepted, and otherwise `Pending`; its `Unsafe` is
  `Unsafe` only with a witness the audit's own checker validated, and otherwise
  `Pending`; a refuted certificate or witness, an `Unknown` answer and every
  failure (crash, timeout, quota breach, protocol error, mismatch) are
  `Unknown`.
* **Contributions** (card W48-PLUGIN-VERDICT-PRESERVATION). A composed row
  keeps every contributor's `(source, answer, verdict, reasons)` record and
  publishes their join; composition keeps every record, projection by source
  recovers each one, and a plugin's own answer, value and reasons are
  recovered from any row holding them (the model of
  `crates/qcore-audit/src/plugin_verdicts.rs`).

## What this is not

Which diagnostic a Rust analysis emits, and whether a trusted checker's replay
is sound, are premises here. That the Rust fold is this fold is pinned by
`crates/qcore-c-gateway/tests/verdict_fold.rs` and
`crates/qcore-schemas/src/verdict.rs`'s tests; that the host classifies as
`firewall` does by `crates/qcore-verifier-host/tests/plugin_firewall.rs`.
-/

namespace QCore.VerdictSemigroup

open QCore.ReasonCategory

/-- The four-value verdict. -/
inductive V where
  | safe
  | pending
  | unknown
  | provisionalUnsafe
  deriving DecidableEq, Repr

namespace V

/-- The order `Safe < Pending < Unknown < Unsafe`, as a rank. -/
def rank : V → Nat
  | .safe => 0
  | .pending => 1
  | .unknown => 2
  | .provisionalUnsafe => 3

theorem rank_injective {a b : V} (h : a.rank = b.rank) : a = b := by
  cases a <;> cases b <;> simp_all [rank]

/-- The composition: the larger value. -/
def join (a b : V) : V := if a.rank ≤ b.rank then b else a

end V

open V

theorem join_rank (a b : V) : (join a b).rank = max a.rank b.rank := by
  cases a <;> cases b <;> decide

theorem join_assoc (a b c : V) : join (join a b) c = join a (join b c) := by
  cases a <;> cases b <;> cases c <;> decide

theorem join_comm (a b : V) : join a b = join b a := by
  cases a <;> cases b <;> decide

theorem join_idem (a : V) : join a a = a := by
  cases a <;> decide

theorem safe_join (a : V) : join .safe a = a := by
  cases a <;> decide

theorem join_safe (a : V) : join a .safe = a := by
  cases a <;> decide

theorem le_join_left (a b : V) : a.rank ≤ (join a b).rank := by
  rw [join_rank]; exact Nat.le_max_left _ _

theorem le_join_right (a b : V) : b.rank ≤ (join a b).rank := by
  rw [join_rank]; exact Nat.le_max_right _ _

/-- The fold of a chain's links; `Safe`, the identity, for none. -/
def fold : List V → V
  | [] => .safe
  | x :: xs => join x (fold xs)

theorem fold_nil : fold [] = .safe := rfl

theorem fold_append (xs ys : List V) : fold (xs ++ ys) = join (fold xs) (fold ys) := by
  induction xs with
  | nil => simp [fold, safe_join]
  | cons x xs ih => simp [fold, ih, join_assoc]

/-- The fold does not depend on the order of the links. -/
theorem fold_append_comm (xs ys : List V) : fold (xs ++ ys) = fold (ys ++ xs) := by
  rw [fold_append, fold_append, join_comm]

/-- Monotonicity: adding links anywhere never lowers the verdict. -/
theorem adding_a_link_never_lowers (xs ys : List V) :
    (fold xs).rank ≤ (fold (xs ++ ys)).rank := by
  rw [fold_append]; exact le_join_left _ _

theorem every_link_is_below_the_fold {x : V} {xs : List V} (member : x ∈ xs) :
    x.rank ≤ (fold xs).rank := by
  induction xs with
  | nil => simp at member
  | cons y ys ih =>
    simp only [List.mem_cons] at member
    rcases member with rfl | tail
    · exact le_join_left _ _
    · exact Nat.le_trans (ih tail) (le_join_right _ _)

/-- A chain is `Safe` exactly when every link is. -/
theorem fold_eq_safe_iff (xs : List V) : fold xs = .safe ↔ ∀ x ∈ xs, x = .safe := by
  induction xs with
  | nil => simp [fold]
  | cons x xs ih =>
    simp only [fold, List.mem_cons, forall_eq_or_imp]
    constructor
    · intro h
      have hx : x = .safe := by
        have := le_join_left x (fold xs); rw [h] at this
        cases x <;> simp_all [rank]
      subst hx
      rw [safe_join] at h
      exact ⟨rfl, ih.mp h⟩
    · rintro ⟨rfl, rest⟩
      rw [safe_join]; exact ih.mpr rest

/-- `Pending` is never folded into `Safe`. -/
theorem pending_never_folds_to_safe {xs : List V} (member : V.pending ∈ xs) :
    fold xs ≠ .safe := by
  intro h
  have := (fold_eq_safe_iff xs).mp h _ member
  cases this

/-- Nothing that is not `Safe` is ever folded into `Safe`. -/
theorem a_non_safe_link_keeps_the_chain_non_safe {x : V} {xs : List V} (member : x ∈ xs)
    (notSafe : x ≠ .safe) : fold xs ≠ .safe :=
  fun h => notSafe ((fold_eq_safe_iff xs).mp h x member)

/-! ## Reasons -/

/-- A verdict with the reasons that produced it, kept as a list whose order
does not matter (a multiset). -/
structure Reasoned (R : Type) where
  value : V
  reasons : List R

namespace Reasoned

variable {R : Type}

def identity : Reasoned R := ⟨.safe, []⟩

def join (a b : Reasoned R) : Reasoned R := ⟨V.join a.value b.value, a.reasons ++ b.reasons⟩

theorem join_assoc (a b c : Reasoned R) : join (join a b) c = join a (join b c) := by
  simp [join, QCore.VerdictSemigroup.join_assoc]

theorem identity_join (a : Reasoned R) : join identity a = a := by
  simp [join, identity, safe_join]

theorem join_identity (a : Reasoned R) : join a identity = a := by
  simp [join, identity, join_safe]

/-- Commutative on the value, and on the reasons up to their order. -/
theorem join_comm_value (a b : Reasoned R) : (join a b).value = (join b a).value := by
  simp [join, QCore.VerdictSemigroup.join_comm]

theorem join_comm_reasons (a b : Reasoned R) :
    (join a b).reasons.Perm (join b a).reasons := by
  simp [join, List.perm_append_comm]

/-- Composition keeps every reason. -/
theorem join_keeps_reasons (a b : Reasoned R) (r : R) (member : r ∈ a.reasons ∨ r ∈ b.reasons) :
    r ∈ (join a b).reasons := by
  simp [join]; exact member

end Reasoned

/-! ## Where a value comes from: the gateway's links -/

/-- One link of a function's chain, as the gateway classifies a diagnostic or
an applied contract: nothing open, a call held because its contract was
withheld (for a reason category), an unresolved construct, or a validated
violation. -/
inductive Link where
  | ok
  | held (reason : Category)
  | unresolved (reason : Category)
  | violation
  deriving DecidableEq, Repr

/-- The value of one link. A held call is `Pending` exactly when its reason is
human-resolvable; `unresolved` is `Unknown` whatever its category. -/
def Link.value : Link → V
  | .ok => .safe
  | .held reason => if Category.isHumanResolvable reason then .pending else .unknown
  | .unresolved _ => .unknown
  | .violation => .provisionalUnsafe

theorem join_eq_pending {a b : V} (h : join a b = .pending) :
    a = .pending ∨ (a = .safe ∧ b = .pending) := by
  cases a <;> cases b <;> simp_all [V.join, rank]

theorem join_eq_unsafe {a b : V} (h : join a b = .provisionalUnsafe) :
    a = .provisionalUnsafe ∨ b = .provisionalUnsafe := by
  cases a <;> cases b <;> simp_all [V.join, rank]

theorem Link.value_eq_pending {l : Link} (h : l.value = .pending) :
    ∃ reason, l = .held reason ∧ Category.isHumanResolvable reason = true := by
  cases l with
  | held reason =>
    cases hr : Category.isHumanResolvable reason
    · simp [Link.value, hr] at h
    · exact ⟨reason, rfl, hr⟩
  | _ => simp [Link.value] at h

theorem Link.value_eq_unsafe {l : Link} (h : l.value = .provisionalUnsafe) :
    l = .violation := by
  cases l with
  | held reason =>
    cases hr : Category.isHumanResolvable reason <;> simp [Link.value, hr] at h
  | violation => rfl
  | _ => simp [Link.value] at h

/-- `Pending` only from a named human-resolvable reason: a chain whose value is
`Pending` has a link held for a human-resolvable category. -/
theorem pending_only_from_a_human_resolvable_reason (ls : List Link)
    (pending : fold (ls.map Link.value) = .pending) :
    ∃ reason, Link.held reason ∈ ls ∧ Category.isHumanResolvable reason = true := by
  induction ls with
  | nil => simp [fold] at pending
  | cons l ls ih =>
    simp only [List.map_cons, fold] at pending
    rcases join_eq_pending pending with here | ⟨_, rest⟩
    · obtain ⟨reason, rfl, resolvable⟩ := Link.value_eq_pending here
      exact ⟨reason, List.mem_cons.mpr (Or.inl rfl), resolvable⟩
    · obtain ⟨reason, member, resolvable⟩ := ih rest
      exact ⟨reason, List.mem_cons.mpr (Or.inr member), resolvable⟩

/-- `Unsafe` only from a validated violation. -/
theorem unsafe_only_from_a_violation (ls : List Link)
    (isUnsafe : fold (ls.map Link.value) = .provisionalUnsafe) : Link.violation ∈ ls := by
  induction ls with
  | nil => simp [fold] at isUnsafe
  | cons l ls ih =>
    simp only [List.map_cons, fold] at isUnsafe
    rcases join_eq_unsafe isUnsafe with here | rest
    · exact List.mem_cons.mpr (Or.inl (Link.value_eq_unsafe here).symm)
    · exact List.mem_cons.mpr (Or.inr (ih rest))

/-! ## Agreement with the historical three-way roll-up -/

/-- The three values the gateway published before the card. -/
inductive V3 where
  | safe
  | unknown
  | provisionalUnsafe
  deriving DecidableEq, Repr

/-- The historical decision: `Unsafe` if any, else `Unknown` if any, else
`Safe`. -/
def rollup3 (xs : List V3) : V3 :=
  if V3.provisionalUnsafe ∈ xs then .provisionalUnsafe
  else if V3.unknown ∈ xs then .unknown else .safe

/-- The projection that reads `Pending` as the `Unknown` it used to be. -/
def toThree : V → V3
  | .safe => .safe
  | .pending => .unknown
  | .unknown => .unknown
  | .provisionalUnsafe => .provisionalUnsafe

/-- The embedding of the historical values. -/
def ofThree : V3 → V
  | .safe => .safe
  | .unknown => .unknown
  | .provisionalUnsafe => .provisionalUnsafe

theorem rollup3_cons (a : V3) (xs : List V3) : rollup3 (a :: xs) = rollup3 [a, rollup3 xs] := by
  unfold rollup3
  cases a <;> by_cases hu : V3.provisionalUnsafe ∈ xs <;> by_cases hk : V3.unknown ∈ xs <;>
    simp [hu, hk]

theorem toThree_join (a b : V) : toThree (join a b) = rollup3 [toThree a, toThree b] := by
  cases a <;> cases b <;> decide

theorem ofThree_join (a b : V3) : join (ofThree a) (ofThree b) = ofThree (rollup3 [a, b]) := by
  cases a <;> cases b <;> decide

/-- The four-valued fold refines the historical roll-up: projected, it is the
three-way decision over the projected links. -/
theorem fold_refines_the_historical_rollup (xs : List V) :
    toThree (fold xs) = rollup3 (xs.map toThree) := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    simp only [fold, List.map_cons]
    rw [toThree_join, ih, ← rollup3_cons]

/-- On links with no `Pending`, the new fold is the historical value exactly. -/
theorem fold_is_the_historical_rollup_without_pending (xs : List V3) :
    fold (xs.map ofThree) = ofThree (rollup3 xs) := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    simp only [List.map_cons, fold]
    rw [ih, ofThree_join, ← rollup3_cons]

/-! ## The deferred-review model, read in four values -/

/-- A deferred-review verdict as a four-value verdict: an `Unknown` whose
reason is human-resolvable is `Pending`. -/
def ofDeferred : QCore.ContractDeferredReview.Verdict → V
  | .safe => .safe
  | .provisionalUnsafe => .provisionalUnsafe
  | .unknown (some reason) => if Category.isHumanResolvable reason then .pending else .unknown
  | .unknown none => .unknown

/-- A verdict resting only on declarations awaiting review is `Pending`. -/
theorem a_verdict_resting_on_a_deferred_clause_is_pending {O : Type}
    (env : List (QCore.ContractDeferredReview.Declaration O)) (o : O)
    (only : ∀ d ∈ env, d.touches o = true → d.pending = true)
    (some_pending : ∃ d ∈ env, d.pending = true ∧ d.touches o = true) :
    ofDeferred (QCore.ContractDeferredReview.verdict env o) = .pending := by
  rw [QCore.ContractDeferredReview.a_verdict_resting_on_a_deferred_clause_is_unknown env o
    only some_pending]
  simp [ofDeferred, Category.isHumanResolvable]

/-! ## The checker-plugin firewall -/

/-- What the audit's own trusted checker said about offered evidence. -/
inductive Check where
  | noChecker
  | accepted
  | refuted
  deriving DecidableEq, Repr

/-- The evidence a plugin attached to its proposal. -/
inductive Evidence where
  | none
  | offered (check : Check)
  deriving DecidableEq, Repr

/-- A plugin's proposal. -/
inductive Proposal where
  | safe (evidence : Evidence)
  | violated (evidence : Evidence)
  | unknown
  deriving DecidableEq, Repr

/-- A dispatch: an answer that passed the request, transport and envelope
gates, or a failure (a gate, a crash, a timeout, a quota breach, a protocol
error or a mismatch). -/
inductive Outcome where
  | answered (proposal : Proposal)
  | failure
  deriving DecidableEq, Repr

namespace Outcome

/-- The four evidence shapes. -/
def evidences : List Evidence :=
  [.none, .offered .noChecker, .offered .accepted, .offered .refuted]

/-- Every outcome, so the firewall's properties are decided exhaustively. -/
def all : List Outcome :=
  .failure :: .answered .unknown ::
    (evidences.map (fun e => Outcome.answered (.safe e)) ++
      evidences.map (fun e => Outcome.answered (.violated e)))

theorem all_is_complete (o : Outcome) : o ∈ all := by
  rcases o with ((_ | c) | (_ | c) | _) | _ <;> (try rcases c with _ | _ | _) <;>
    simp [all, evidences]

end Outcome

/-- The firewall (`PluginClaim::verdict ∘ verify_plugin_claim`). -/
def firewall : Outcome → V
  | .failure => .unknown
  | .answered .unknown => .unknown
  | .answered (.safe .none) => .pending
  | .answered (.safe (.offered .noChecker)) => .pending
  | .answered (.safe (.offered .accepted)) => .safe
  | .answered (.safe (.offered .refuted)) => .unknown
  | .answered (.violated .none) => .pending
  | .answered (.violated (.offered .noChecker)) => .pending
  | .answered (.violated (.offered .accepted)) => .provisionalUnsafe
  | .answered (.violated (.offered .refuted)) => .unknown

/-- A plugin's `Safe` counts only with a certificate the audit accepted. -/
theorem a_plugin_is_safe_only_with_an_accepted_certificate (o : Outcome) :
    firewall o = .safe ↔ o = .answered (.safe (.offered .accepted)) := by
  have exhaustive : ∀ o ∈ Outcome.all,
      (firewall o = .safe ↔ o = .answered (.safe (.offered .accepted))) := by decide
  exact exhaustive o (Outcome.all_is_complete o)

/-- A plugin's `Unsafe` counts only with a witness the audit validated. -/
theorem a_plugin_is_unsafe_only_with_a_validated_witness (o : Outcome) :
    firewall o = .provisionalUnsafe ↔ o = .answered (.violated (.offered .accepted)) := by
  have exhaustive : ∀ o ∈ Outcome.all,
      (firewall o = .provisionalUnsafe ↔ o = .answered (.violated (.offered .accepted))) := by
    decide
  exact exhaustive o (Outcome.all_is_complete o)

/-- An uncertified `Safe` claim — no certificate, or none the audit can
check — is `Pending`. -/
theorem an_uncertified_safe_claim_is_pending :
    firewall (.answered (.safe .none)) = .pending ∧
      firewall (.answered (.safe (.offered .noChecker))) = .pending := ⟨rfl, rfl⟩

/-- An uncertified `Unsafe` claim — no witness, or none the audit can
validate — is `Pending`. -/
theorem an_uncertified_unsafe_claim_is_pending :
    firewall (.answered (.violated .none)) = .pending ∧
      firewall (.answered (.violated (.offered .noChecker))) = .pending := ⟨rfl, rfl⟩

/-- A plugin failure is `Unknown`, and so are a refuted certificate or witness
and an `Unknown` answer. -/
theorem a_plugin_failure_is_unknown :
    firewall .failure = .unknown ∧ firewall (.answered .unknown) = .unknown ∧
      firewall (.answered (.safe (.offered .refuted))) = .unknown ∧
      firewall (.answered (.violated (.offered .refuted))) = .unknown :=
  ⟨rfl, rfl, rfl, rfl⟩

/-- A plugin claim is `Pending` only when nothing was checked: no certificate
or witness, or none the audit has a checker for. -/
theorem a_plugin_is_pending_only_when_nothing_was_checked (o : Outcome)
    (pending : firewall o = .pending) :
    o = .answered (.safe .none) ∨ o = .answered (.safe (.offered .noChecker)) ∨
      o = .answered (.violated .none) ∨ o = .answered (.violated (.offered .noChecker)) := by
  have exhaustive : ∀ o ∈ Outcome.all, firewall o = .pending →
      (o = .answered (.safe .none) ∨ o = .answered (.safe (.offered .noChecker)) ∨
        o = .answered (.violated .none) ∨ o = .answered (.violated (.offered .noChecker))) := by
    decide
  exact exhaustive o (Outcome.all_is_complete o) pending

/-- A chain with an uncertified plugin link is never `Safe`, whatever the rest
of the chain says. -/
theorem an_uncertified_plugin_link_keeps_the_chain_non_safe (xs : List V) (o : Outcome)
    (uncertified : firewall o = .pending) : fold (firewall o :: xs) ≠ .safe :=
  pending_never_folds_to_safe (by rw [uncertified]; exact List.mem_cons.mpr (Or.inl rfl))

/-! ## Contributions: who said what (card W48-PLUGIN-VERDICT-PRESERVATION)

A composed row keeps every contributor's record — its source (the host, the
plugin run, a plugin), what it answered before the firewall classified it,
the value it contributes and its reasons — and *derives* its published value
and reasons from them. The published value is the join of the contributed
values, the published reasons the union of the contributed reasons, and
composing two rows (a unit over its functions, a checkpoint's units, a
report's rows) concatenates their records. So no composition erases a
contributor: each source's own records are recovered from the composed row by
projection, and that source's records and everyone else's recompose the
published value. -/

/-- One contributor's record in a composed row. `A` is what the contributor
answered before classification (for a plugin, its firewall outcome). -/
structure Contribution (S A R : Type) where
  source : S
  answer : A
  value : V
  reasons : List R

namespace Contribution

variable {S A R : Type}

/-- The record read as a reasoned verdict, forgetting who said it. -/
def toReasoned (c : Contribution S A R) : Reasoned R := ⟨c.value, c.reasons⟩

end Contribution

namespace Reasoned

variable {R : Type}

/-- The fold of reasoned verdicts; the identity for none. -/
def fold : List (Reasoned R) → Reasoned R
  | [] => identity
  | x :: xs => join x (fold xs)

end Reasoned

/-- The fold does not depend on the order of the links. -/
theorem fold_perm {xs ys : List V} (h : xs.Perm ys) : fold xs = fold ys := by
  induction h with
  | nil => rfl
  | cons x _ ih => simp [fold, ih]
  | swap x y l =>
    simp only [fold]
    rw [← join_assoc, join_comm y x, join_assoc]
  | trans _ _ ih₁ ih₂ => exact ih₁.trans ih₂

/-- A composed row: the records it keeps. Its value and reasons are derived. -/
structure Row (S A R : Type) where
  contributions : List (Contribution S A R)

namespace Row

variable {S A R : Type}

/-- The published value: the join of every contributed value. -/
def value (r : Row S A R) : V := fold (r.contributions.map Contribution.value)

/-- The published reasons: every contributed reason, with multiplicity. -/
def reasons (r : Row S A R) : List R := r.contributions.flatMap Contribution.reasons

/-- The published reasoned verdict. -/
def published (r : Row S A R) : Reasoned R := ⟨r.value, r.reasons⟩

/-- Composition of rows keeps both rows' records. -/
def join (a b : Row S A R) : Row S A R := ⟨a.contributions ++ b.contributions⟩

/-- One source's own records, recovered from a composed row. -/
def project [DecidableEq S] (r : Row S A R) (s : S) : Row S A R :=
  ⟨r.contributions.filter (fun c => decide (c.source = s))⟩

/-- Every other source's records. -/
def rest [DecidableEq S] (r : Row S A R) (s : S) : Row S A R :=
  ⟨r.contributions.filter (fun c => !decide (c.source = s))⟩

/-- **The published value is the join**: a row publishes exactly the
reasoned fold of its contributors' records. -/
theorem published_is_the_fold (r : Row S A R) :
    r.published = Reasoned.fold (r.contributions.map Contribution.toReasoned) := by
  rcases r with ⟨cs⟩
  induction cs with
  | nil => rfl
  | cons c cs ih =>
    simp only [published, value, reasons, List.map_cons, fold, List.flatMap_cons,
      Reasoned.fold] at ih ⊢
    rw [← ih]
    rfl

/-- Composing rows joins their published verdicts. -/
theorem join_published (a b : Row S A R) :
    (join a b).published = Reasoned.join a.published b.published := by
  simp [join, published, value, reasons, Reasoned.join, List.map_append, fold_append,
    List.flatMap_append]

theorem join_assoc (a b c : Row S A R) : join (join a b) c = join a (join b c) := by
  simp [join]

/-- Composition never lowers a row. -/
theorem value_le_join_left (a b : Row S A R) : a.value.rank ≤ (join a b).value.rank := by
  simp only [join, value, List.map_append, fold_append]; exact le_join_left _ _

theorem value_le_join_right (a b : Row S A R) : b.value.rank ≤ (join a b).value.rank := by
  simp only [join, value, List.map_append, fold_append]; exact le_join_right _ _

/-- **Composition preserves every record**: a record of either row is a
record of the composed row. -/
theorem join_keeps_every_record (a b : Row S A R) (c : Contribution S A R)
    (member : c ∈ a.contributions ∨ c ∈ b.contributions) :
    c ∈ (join a b).contributions := by
  simp [join]; exact member

/-- A contributor's value is below the published value, and its reasons are
published. -/
theorem a_record_is_below_the_row (r : Row S A R) {c : Contribution S A R}
    (member : c ∈ r.contributions) :
    c.value.rank ≤ r.value.rank ∧ ∀ x ∈ c.reasons, x ∈ r.reasons := by
  refine ⟨every_link_is_below_the_fold (List.mem_map_of_mem member), ?_⟩
  intro x hx
  exact List.mem_flatMap.mpr ⟨c, member, hx⟩

/-- A row is `Safe` exactly when every contributor is. -/
theorem value_eq_safe_iff (r : Row S A R) :
    r.value = .safe ↔ ∀ c ∈ r.contributions, c.value = .safe := by
  rw [value, fold_eq_safe_iff]
  simp

variable [DecidableEq S]

/-- **Projection recovers a source's own records**, every one, from any
composition: projection distributes over composition. -/
theorem project_join (a b : Row S A R) (s : S) :
    (join a b).project s = join (a.project s) (b.project s) := by
  simp [join, project, List.filter_append]

/-- A record is recovered under its own source. -/
theorem project_recovers (r : Row S A R) {c : Contribution S A R}
    (member : c ∈ r.contributions) : c ∈ (r.project c.source).contributions := by
  simp [project, member]

/-- Everything projected under `s` is a record of the row from `s`. -/
theorem project_only_from_the_source (r : Row S A R) (s : S) {c : Contribution S A R}
    (member : c ∈ (r.project s).contributions) : c ∈ r.contributions ∧ c.source = s := by
  simpa [project] using member

/-- **Recovery recomposes**: a source's own records and everyone else's
recompose the published value, and the published reasons up to order. -/
theorem project_and_rest_recompose (r : Row S A R) (s : S) :
    (join (r.project s) (r.rest s)).value = r.value ∧
      (join (r.project s) (r.rest s)).reasons.Perm r.reasons := by
  have perm : (r.contributions.filter (fun c => decide (c.source = s)) ++
      r.contributions.filter (fun c => !decide (c.source = s))).Perm r.contributions :=
    List.filter_append_perm _ _
  constructor
  · simp only [join, project, rest, value]
    exact fold_perm (perm.map _)
  · simp only [join, project, rest, reasons]
    exact perm.flatMap_right _

end Row

/-! ### Plugins in a composed row

A plugin's record carries its firewall outcome as its answer and the
firewall's value as its value; a host or run record carries no outcome. -/

/-- A plugin's record: what it answered, and the value the firewall gives it. -/
def pluginRecord {S R : Type} (s : S) (o : Outcome) (reasons : List R) :
    Contribution S (Option Outcome) R :=
  ⟨s, some o, firewall o, reasons⟩

/-- A host (or run) record: no plugin answer. -/
def hostRecord {S R : Type} (s : S) (v : V) (reasons : List R) :
    Contribution S (Option Outcome) R :=
  ⟨s, none, v, reasons⟩

section Plugins

variable {S R : Type}

/-- **A plugin's own verdict and reasons are recovered**: projecting a composed
row under the plugin's source finds its answer, its firewall value and its
reasons. -/
theorem a_plugin_record_is_recovered [DecidableEq S] (r : Row S (Option Outcome) R) (s : S)
    (o : Outcome)
    (rs : List R) (member : pluginRecord s o rs ∈ r.contributions) :
    ∃ c ∈ (r.project s).contributions,
      c.answer = some o ∧ c.value = firewall o ∧ c.reasons = rs :=
  ⟨_, Row.project_recovers r member, rfl, rfl, rfl⟩

/-- An uncertified plugin claim keeps the composed row from `Safe`. -/
theorem an_uncertified_plugin_keeps_the_row_non_safe (r : Row S (Option Outcome) R) (s : S)
    (o : Outcome) (rs : List R) (member : pluginRecord s o rs ∈ r.contributions)
    (uncertified : firewall o = .pending) : r.value ≠ .safe := by
  intro h
  have := (Row.value_eq_safe_iff r).mp h _ member
  simp [pluginRecord, uncertified] at this

/-- No `Unknown` contributor flips to `Safe` or `Pending`: a row with an
`Unknown` record is `Unknown` or `Unsafe`. -/
theorem an_unknown_record_keeps_the_row_at_least_unknown (r : Row S (Option Outcome) R)
    {c : Contribution S (Option Outcome) R} (member : c ∈ r.contributions)
    (unknown : c.value = .unknown) : r.value = .unknown ∨ r.value = .provisionalUnsafe := by
  have le := (Row.a_record_is_below_the_row r member).1
  rw [unknown] at le
  cases h : r.value <;> simp_all [rank]

/-- A failing plugin is fail-closed in any row. -/
theorem a_failing_plugin_keeps_the_row_at_least_unknown (r : Row S (Option Outcome) R)
    (s : S) (rs : List R) (member : pluginRecord s .failure rs ∈ r.contributions) :
    r.value = .unknown ∨ r.value = .provisionalUnsafe :=
  an_unknown_record_keeps_the_row_at_least_unknown r member rfl

/-- A row with a plugin record is `Safe` only if that plugin's `Safe` was
certified. -/
theorem a_row_with_a_plugin_is_safe_only_if_certified (r : Row S (Option Outcome) R) (s : S)
    (o : Outcome) (rs : List R) (member : pluginRecord s o rs ∈ r.contributions)
    (safe : r.value = .safe) : o = .answered (.safe (.offered .accepted)) :=
  (a_plugin_is_safe_only_with_an_accepted_certificate o).mp
    ((Row.value_eq_safe_iff r).mp safe _ member)

/-! #### Negative controls -/

/-- Two plugins disagreeing — one certified `Safe`, one failed — beside a
`Safe` host: the row is `Unknown`, and each plugin's own record is recovered. -/
theorem two_plugins_disagreeing [DecidableEq S] (host p q : S) (hp : p ≠ host) (hq : q ≠ host)
    (pq : p ≠ q)
    (rp rq : List R) :
    Row.value (⟨[hostRecord host .safe [],
        pluginRecord p (.answered (.safe (.offered .accepted))) rp,
        pluginRecord q .failure rq]⟩ : Row S (Option Outcome) R) = .unknown ∧
      (Row.project (⟨[hostRecord host .safe [],
        pluginRecord p (.answered (.safe (.offered .accepted))) rp,
        pluginRecord q .failure rq]⟩ : Row S (Option Outcome) R) p).contributions =
        [pluginRecord p (.answered (.safe (.offered .accepted))) rp] ∧
      (Row.project (⟨[hostRecord host .safe [],
        pluginRecord p (.answered (.safe (.offered .accepted))) rp,
        pluginRecord q .failure rq]⟩ : Row S (Option Outcome) R) q).contributions =
        [pluginRecord q .failure rq] := by
  refine ⟨rfl, ?_, ?_⟩
  · simp [Row.project, hostRecord, pluginRecord, Ne.symm hp, Ne.symm pq]
  · simp [Row.project, hostRecord, pluginRecord, Ne.symm hq, pq]

/-- A plugin's `Unsafe` beside a host `Safe`: `Pending` while uncertified,
provisional `Unsafe` only with a validated witness; never `Safe`. -/
theorem a_plugin_unsafe_beside_a_host_safe (host p : S) (rs : List R) :
    Row.value (⟨[hostRecord host .safe [], pluginRecord p (.answered (.violated .none)) rs]⟩ :
        Row S (Option Outcome) R) = .pending ∧
      Row.value (⟨[hostRecord host .safe [],
          pluginRecord p (.answered (.violated (.offered .noChecker))) rs]⟩ :
        Row S (Option Outcome) R) = .pending ∧
      Row.value (⟨[hostRecord host .safe [],
          pluginRecord p (.answered (.violated (.offered .accepted))) rs]⟩ :
        Row S (Option Outcome) R) = .provisionalUnsafe :=
  ⟨rfl, rfl, rfl⟩

/-- A failing plugin beside a host `Safe` is `Unknown`. -/
theorem a_failing_plugin_beside_a_host_safe (host p : S) (rs : List R) :
    Row.value (⟨[hostRecord host .safe [], pluginRecord p .failure rs]⟩ :
        Row S (Option Outcome) R) = .unknown := rfl

end Plugins

end QCore.VerdictSemigroup
