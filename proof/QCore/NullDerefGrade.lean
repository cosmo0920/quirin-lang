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

import QCore.NullDeref
import QCore.UsageDemand

/-!
# The grounding of `quant null-deref #B;` in usage grades (card NULL-DEREF-QTT-GROUNDING)

USER DECISION 2026-10-11 ("We need grounding."): `quant null-deref #B;` is the
contract-level projection of a **§L07 usage grade**, read with **§L08–L09
access and identity**; it is not a §L06 index or dimension and not a `credit`,
`live` or `cost` axis. Normative text: contract specification C09.3, language
specification §L07, grammar G05d.

* **The grade of one path** (`pathGrade`). Each dereference of the formal's
  entry value is one use of its pointee: §L07's sequential demands add, so a
  path's demand is one per dereference made before a NULL test or an
  assignment of the formal closes the entry value's account (after a test the
  body holds its own evidence; after an assignment the formal names another
  value). An offset `B ± n` is the same value (C11 6.5.6p8) and adds nothing.
* **The grade of a body** (`bodyGrade`): the least enclosing interval of its
  terminating paths' grades (§L07's join of mutually exclusive branches);
  none when no path terminates.
* **Identity.** A use needs an access share of a live identity (§L08's
  `FULL = 1024` shares; §L09: a raw pointer carries no custody). NULL has no
  identity, so no share exists for it: a grade whose lower bound is at least
  one requires the actual to be an existing identity (`requiresIdentity`,
  `admits`); a lower bound of 0 requires nothing.

The theorems equate the existing null-dereference semantics
(`QCore.NullDeref`, the Rust `qcore_core::null_deref` and the walk's `First`
sets) with this reading:

* `relevant_accepts_body_iff_inferred`, `necessary_iff_relevant`: a `relevant`
  (lower bound ≥ 1) demand on every terminating path is exactly the walk's
  inference and the clause's callee validation (`necessary`);
* `zero_lower_imposes_nothing`, `a_path_without_a_first_dereference_infers_no_clause`:
  a lower bound of 0 imposes no caller obligation, so no clause;
* `moved_question_is_sequential_addition`: a met demand carries the identity
  forward (decision 1's moved question);
* `branch_join_matches`, `requiresIdentity_join`, `established_join`: the
  grade join matches the walk's path join;
* `caller_obligation_is_grade_admission`: `QCore.NullDeref`'s caller
  obligation is grade admission against an existing identity.

## The one-sided body check (a divergence from §L07's admission, not a change)

§L07 admits a declared grade against a body only when it contains the body's
interval; a usage checker rejects `one` on an unused parameter
(`QCore.UsageDemand.unused_one_parameter_rejected`). The null clause's body
check does not reject: a declared clause whose body grade has lower bound 0 is
`stronger-than-necessary` (USER DECISION 2026-10-10, decision 2), and the
callee still reads `B` non-null at entry. That is sound because the clause
spends nothing: it asks the caller for an existing identity, a precondition,
and the body's reading at entry is sound for every caller that meets it
(`stronger_than_necessary_is_still_sound`). Shares are borrowed, not
consumed, so asking for one more than the body uses costs no conservation law.

## What this is not

A model of C or of the walk: that the walk's `First` sets are this abstraction
of its paths (a dereference of a value whose origin is the formal marks it, a
NULL test of it marks it tested, an assignment drops the origin, joins union
the sets, exits collect them) is a review premise (trust boundary 28), pinned
by `firstVectors`, `inferredVectors` and `gradeVectors`, which bind the Rust
(`crates/qcore-core/tests/null_deref.rs`,
`crates/qcore-core/tests/null_deref_grade.rs`). Paths are finite event lists;
a loop is its unrolled paths, and an upper bound the walk does not track
(`inf`) never enters an admission, which reads only the lower bound.
-/

namespace QCore.NullDerefGrade

open QCore.NullDeref (Event Nullness Outcome First check discharged afterDereference
  walk step run sound noAssign firstIsDeref inferred firstAfter hasMoved entry)

/-! ## Identity: what a grade asks of an actual -/

/-- A grade requires an existing identity when its lower bound is at least one
(§L08–L09: a use needs a share of a live identity; NULL has none). -/
def requiresIdentity (g : Usage) : Bool := decide (1 ≤ g.lower)

/-- Grade admission of an actual (`identityExists`: the actual is not NULL). -/
def admits (g : Usage) (identityExists : Bool) : Bool := !requiresIdentity g || identityExists

/-- **A declared `relevant` use admits an actual only if its identity exists.** -/
theorem relevant_admits_only_an_existing_identity (e : Bool) :
    admits Usage.relevant e = e := by
  cases e <;> rfl

/-- `relevant` refines exactly the grades whose lower bound is at least one. -/
theorem refines_relevant_iff (g : Usage) : g.Refines Usage.relevant ↔ 1 ≤ g.lower := by
  rcases g with ⟨lower, upper⟩
  constructor
  · intro h; exact h.1
  · intro h
    refine ⟨h, ?_⟩
    cases upper <;> trivial

theorem relevant_accepts_iff (g : Usage) (hv : g.Valid) :
    Usage.relevant.Accepts g ↔ 1 ≤ g.lower := by
  constructor
  · intro h; exact (refines_relevant_iff g).mp h.2
  · intro h; exact ⟨hv, (refines_relevant_iff g).mpr h⟩

theorem requiresIdentity_iff (g : Usage) : requiresIdentity g = true ↔ 1 ≤ g.lower := by
  simp [requiresIdentity]

/-! ## The grade of one path -/

/-- The demand one path puts on the formal's entry value (§L07 sequential
addition), closed by a NULL test or an assignment; an offset adds nothing. -/
def pathGrade : List Event → Usage
  | [] => Usage.zero
  | .deref _ :: rest => Usage.add Usage.one (pathGrade rest)
  | .offset :: rest => pathGrade rest
  | .assign _ :: _ => Usage.zero
  | .nonNullArm :: _ => Usage.zero
  | .nullArm :: _ => Usage.zero

/-- A path's grade is the exact demand `[n, n]`. -/
theorem pathGrade_exact : ∀ evs : List Event, ∃ n, pathGrade evs = ⟨n, .finite n⟩
  | [] => ⟨0, rfl⟩
  | .deref _ :: rest => by
      obtain ⟨n, hn⟩ := pathGrade_exact rest
      exact ⟨1 + n, by simp [pathGrade, hn, Usage.add, Usage.one, UsageUpper.add]⟩
  | .offset :: rest => pathGrade_exact rest
  | .assign _ :: _ => ⟨0, rfl⟩
  | .nonNullArm :: _ => ⟨0, rfl⟩
  | .nullArm :: _ => ⟨0, rfl⟩

theorem pathGrade_valid (evs : List Event) : (pathGrade evs).Valid := by
  obtain ⟨n, hn⟩ := pathGrade_exact evs
  rw [hn]
  exact Nat.le_refl n

/-- **A path demands the identity exactly when it dereferences first.** -/
theorem pathGrade_lower_pos_iff : ∀ evs : List Event,
    1 ≤ (pathGrade evs).lower ↔ firstIsDeref evs = true
  | [] => by simp [pathGrade, firstIsDeref, Usage.zero]
  | .deref _ :: rest => by
      simp only [pathGrade, firstIsDeref, Usage.add, Usage.one, iff_true]
      omega
  | .offset :: rest => by
      simp only [pathGrade, firstIsDeref]
      exact pathGrade_lower_pos_iff rest
  | .assign _ :: _ => by simp [pathGrade, firstIsDeref, Usage.zero]
  | .nonNullArm :: _ => by simp [pathGrade, firstIsDeref, Usage.zero]
  | .nullArm :: _ => by simp [pathGrade, firstIsDeref, Usage.zero]

theorem relevant_accepts_path_iff (evs : List Event) :
    Usage.relevant.Accepts (pathGrade evs) ↔ firstIsDeref evs = true :=
  (relevant_accepts_iff _ (pathGrade_valid evs)).trans (pathGrade_lower_pos_iff evs)

/-! ## The walk's abstraction of a path: `First` -/

/-- The walk's `First` for one path: a dereference of the entry value first is
`dereferenced`, a NULL test first is `other`, an assignment drops the formal's
origin before anything was attributed to it (`pending`); offsets keep it. -/
def firstOf : List Event → First
  | [] => .pending
  | .deref _ :: _ => .dereferenced
  | .offset :: rest => firstOf rest
  | .assign _ :: _ => .pending
  | .nonNullArm :: _ => .other
  | .nullArm :: _ => .other

/-- The events the walk attributes to the formal's origin, by the names the
Rust's `First` transitions read (`firstVectors`): an assignment ends them. -/
def originEvents : List Event → List String
  | [] => []
  | .deref _ :: rest => "deref" :: originEvents rest
  | .offset :: rest => originEvents rest
  | .assign _ :: _ => []
  | .nonNullArm :: rest => "test" :: originEvents rest
  | .nullArm :: rest => "test" :: originEvents rest

private def firstStep (f : First) (e : String) : First :=
  if e == "deref" then f.deref else f.test

private theorem fold_dereferenced : ∀ es : List String,
    es.foldl firstStep .dereferenced = .dereferenced
  | [] => rfl
  | e :: es => by
      simp only [List.foldl]
      have : firstStep .dereferenced e = .dereferenced := by
        unfold firstStep; split <;> rfl
      rw [this]; exact fold_dereferenced es

private theorem fold_other : ∀ es : List String, es.foldl firstStep .other = .other
  | [] => rfl
  | e :: es => by
      simp only [List.foldl]
      have : firstStep .other e = .other := by
        unfold firstStep; split <;> rfl
      rw [this]; exact fold_other es

/-- **`firstOf` is the Rust's `First` fold over the origin's events.** -/
theorem firstOf_is_firstAfter : ∀ evs : List Event, firstOf evs = firstAfter (originEvents evs)
  | [] => rfl
  | .deref _ :: rest => by
      show First.dereferenced = (originEvents rest).foldl firstStep (firstStep .pending "deref")
      have : firstStep .pending "deref" = .dereferenced := by decide
      rw [this, fold_dereferenced]
  | .offset :: rest => firstOf_is_firstAfter rest
  | .assign _ :: _ => rfl
  | .nonNullArm :: rest => by
      show First.other = (originEvents rest).foldl firstStep (firstStep .pending "test")
      have : firstStep .pending "test" = .other := by decide
      rw [this, fold_other]
  | .nullArm :: rest => by
      show First.other = (originEvents rest).foldl firstStep (firstStep .pending "test")
      have : firstStep .pending "test" = .other := by decide
      rw [this, fold_other]

theorem firstOf_dereferenced_iff : ∀ evs : List Event,
    firstOf evs = .dereferenced ↔ firstIsDeref evs = true
  | [] => by simp [firstOf, firstIsDeref]
  | .deref _ :: _ => by simp [firstOf, firstIsDeref]
  | .offset :: rest => firstOf_dereferenced_iff rest
  | .assign _ :: _ => by simp [firstOf, firstIsDeref]
  | .nonNullArm :: _ => by simp [firstOf, firstIsDeref]
  | .nullArm :: _ => by simp [firstOf, firstIsDeref]

/-! ## The grade of a body: the join of its terminating paths -/

/-- The body's grade: the least enclosing interval of its terminating paths'
grades (§L07's branch join); `none` when no path terminates. -/
def bodyGrade : List (List Event) → Option Usage
  | [] => none
  | p :: ps =>
      match bodyGrade ps with
      | none => some (pathGrade p)
      | some g => some (Usage.join (pathGrade p) g)

theorem bodyGrade_none_iff : ∀ paths : List (List Event), bodyGrade paths = none ↔ paths = []
  | [] => by simp [bodyGrade]
  | p :: ps => by
      simp only [bodyGrade]
      cases bodyGrade ps <;> simp

theorem bodyGrade_valid : ∀ (paths : List (List Event)) (g : Usage),
    bodyGrade paths = some g → g.Valid
  | [], _, h => by simp [bodyGrade] at h
  | p :: ps, g, h => by
      simp only [bodyGrade] at h
      cases hb : bodyGrade ps with
      | none =>
          rw [hb] at h; cases h; exact pathGrade_valid p
      | some g' =>
          rw [hb] at h; cases h
          exact Usage.join_valid (pathGrade_valid p) (bodyGrade_valid ps g' hb)

/-- The join's lower bound is the minimum. -/
theorem join_lower (a b : Usage) : (Usage.join a b).lower = Nat.min a.lower b.lower := rfl

theorem bodyGrade_lower_pos_iff : ∀ (paths : List (List Event)) (g : Usage),
    bodyGrade paths = some g → (1 ≤ g.lower ↔ ∀ p ∈ paths, firstIsDeref p = true)
  | [], _, h => by simp [bodyGrade] at h
  | p :: ps, g, h => by
      simp only [bodyGrade] at h
      cases hb : bodyGrade ps with
      | none =>
          rw [hb] at h; cases h
          have hps : ps = [] := (bodyGrade_none_iff ps).mp hb
          subst hps
          simp [pathGrade_lower_pos_iff]
      | some g' =>
          rw [hb] at h; cases h
          rw [join_lower, Nat.le_min, pathGrade_lower_pos_iff, bodyGrade_lower_pos_iff ps g' hb]
          simp

theorem inferred_iff (fs : List First) :
    inferred fs = true ↔ fs ≠ [] ∧ ∀ f ∈ fs, f = .dereferenced := by
  simp [inferred, List.all_eq_true]

/-- **A `relevant` demand on every terminating path is exactly the walk's
inference.** -/
theorem relevant_accepts_body_iff_inferred (paths : List (List Event)) :
    (∃ g, bodyGrade paths = some g ∧ Usage.relevant.Accepts g) ↔
      inferred (paths.map firstOf) = true := by
  rw [inferred_iff]
  constructor
  · rintro ⟨g, hg, ha⟩
    have hne : paths ≠ [] := by
      intro h; subst h; simp [bodyGrade] at hg
    have hall := (bodyGrade_lower_pos_iff paths g hg).mp
      ((relevant_accepts_iff g (bodyGrade_valid paths g hg)).mp ha)
    refine ⟨by simpa using hne, ?_⟩
    intro f hf
    obtain ⟨p, hp, rfl⟩ := List.mem_map.mp hf
    exact (firstOf_dereferenced_iff p).mpr (hall p hp)
  · rintro ⟨hne, hall⟩
    have hne' : paths ≠ [] := by simpa using hne
    cases hg : bodyGrade paths with
    | none => exact absurd ((bodyGrade_none_iff paths).mp hg) hne'
    | some g =>
        refine ⟨g, rfl, (relevant_accepts_iff g (bodyGrade_valid paths g hg)).mpr ?_⟩
        refine (bodyGrade_lower_pos_iff paths g hg).mpr ?_
        intro p hp
        exact (firstOf_dereferenced_iff p).mp (hall _ (List.mem_map_of_mem hp))

/-- The Rust's `clause_standing` on the exit set. -/
def standing (exits : List First) : String :=
  if inferred exits then "necessary" else "stronger-than-necessary"

/-- **The clause's callee validation is `relevant`'s admission of the body's
grade**: `necessary` exactly when `relevant` admits it. -/
theorem necessary_iff_relevant (paths : List (List Event)) :
    standing (paths.map firstOf) = "necessary" ↔
      ∃ g, bodyGrade paths = some g ∧ Usage.relevant.Accepts g := by
  rw [relevant_accepts_body_iff_inferred]
  unfold standing
  cases inferred (paths.map firstOf) <;> decide

/-- **Callee validation is semantic necessity**: when `relevant` admits the
body's grade, no terminating execution of any of its paths starts with the
formal NULL (`QCore.NullDeref.an_inferred_clause_is_necessary`). -/
theorem relevant_body_needs_the_identity (paths : List (List Event)) (g : Usage)
    (hg : bodyGrade paths = some g) (ha : Usage.relevant.Accepts g) :
    ∀ p ∈ paths, ∀ oracle, run true p oracle = none := by
  have hall := (bodyGrade_lower_pos_iff paths g hg).mp
    ((relevant_accepts_iff g (bodyGrade_valid paths g hg)).mp ha)
  intro p hp oracle
  exact QCore.NullDeref.an_inferred_clause_is_necessary p oracle (hall p hp)

/-! ## A lower bound of 0 imposes no caller obligation -/

/-- **A lower bound of 0 asks nothing of the actual and is no clause.** -/
theorem zero_lower_imposes_nothing (g : Usage) (h : g.lower = 0) :
    (∀ e, admits g e = true) ∧ ¬ g.Refines Usage.relevant := by
  refine ⟨fun e => by simp [admits, requiresIdentity, h], ?_⟩
  rw [refines_relevant_iff, h]
  omega

/-- `many`, the grade of an unannotated parameter, admits every actual and
every valid body grade. -/
theorem many_imposes_nothing :
    (∀ e, admits Usage.many e = true) ∧ ∀ g : Usage, g.Valid → Usage.many.Accepts g := by
  refine ⟨fun e => by simp [admits, requiresIdentity, Usage.many], ?_⟩
  intro g hv
  refine ⟨hv, Nat.zero_le _, ?_⟩
  cases g.upper <;> trivial

/-- **One path that does not dereference first leaves the body's lower bound
at 0: no clause is inferred.** -/
theorem a_path_without_a_first_dereference_infers_no_clause (paths : List (List Event))
    (p : List Event) (hp : p ∈ paths) (hf : firstIsDeref p = false) :
    inferred (paths.map firstOf) = false ∧
      ∀ g, bodyGrade paths = some g → g.lower = 0 ∧ ∀ e, admits g e = true := by
  have hni : ¬ inferred (paths.map firstOf) = true := by
    intro hi
    obtain ⟨g, hg, ha⟩ := (relevant_accepts_body_iff_inferred paths).mpr hi
    have := (bodyGrade_lower_pos_iff paths g hg).mp
      ((relevant_accepts_iff g (bodyGrade_valid paths g hg)).mp ha) p hp
    simp [hf] at this
  refine ⟨by simpa using hni, ?_⟩
  intro g hg
  have hz : g.lower = 0 := by
    have hlt : ¬ 1 ≤ g.lower := by
      intro hle
      have := (bodyGrade_lower_pos_iff paths g hg).mp hle p hp
      simp [hf] at this
    omega
  exact ⟨hz, (zero_lower_imposes_nothing g hz).1⟩

/-! ## Clause positions are demand substitution -/

/-- **Handing `B` to a clause position is a `relevant` use of it**: §L07's
demand substitution scales the argument's demand (`one`) by the parameter's
grade, and any demand added to it requires the identity (rule 14p counts the
hand-off as a dereference). -/
theorem a_clause_position_demands_the_identity (before : Usage) :
    requiresIdentity (Usage.add before (Usage.mul Usage.relevant Usage.one)) = true := by
  simp [requiresIdentity, Usage.add, Usage.mul, Usage.relevant, Usage.one]

/-- An unannotated position (`many`) demands nothing of the identity. -/
theorem an_unannotated_position_demands_nothing :
    requiresIdentity (Usage.mul Usage.many Usage.one) = false := by
  simp [requiresIdentity, Usage.mul, Usage.many, Usage.one]

/-! ## A met demand carries the identity forward: the moved question -/

private theorem walk_none : ∀ evs : List Event, walk none evs = none
  | [] => rfl
  | _ :: rest => by simp only [walk, List.foldl, step]; exact walk_none rest

private theorem walk_cons (x : Option Nullness) (e : Event) (rest : List Event) :
    walk x (e :: rest) = walk (step x e) rest := rfl

/-- A non-null value keeps exactly its reading along a path with no assignment. -/
theorem nonNull_kept : ∀ (rest : List Event) (ps : List QCore.NullDeref.Proof) (n' : Nullness),
    noAssign rest = true → walk (some (.nonNull ps)) rest = some n' → n' = .nonNull ps
  | [], _, _, _, h => by simp [walk] at h; exact h.symm
  | .assign _ :: _, _, _, hna, _ => by simp [noAssign] at hna
  | .deref _ :: rest, ps, n', hna, h => by
      rw [walk_cons] at h
      exact nonNull_kept rest ps n' hna h
  | .offset :: rest, ps, n', hna, h => by
      rw [walk_cons] at h
      exact nonNull_kept rest ps n' hna h
  | .nonNullArm :: rest, ps, n', hna, h => by
      rw [walk_cons] at h
      exact nonNull_kept rest ps n' hna h
  | .nullArm :: rest, ps, n', _, h => by
      rw [walk_cons] at h
      simp only [step, QCore.NullDeref.refine] at h
      rw [walk_none] at h
      cases h

/-- The demand on the identity since its last assignment (a NULL test changes
no identity, so it does not close this account). -/
def useGrade : List Event → Usage
  | [] => Usage.zero
  | .deref _ :: rest => Usage.add Usage.one (useGrade rest)
  | .assign _ :: _ => Usage.zero
  | _ :: rest => useGrade rest

/-- Sequential addition's identity requirement is the disjunction of its
summands': existence is required once, however many uses follow. -/
theorem requiresIdentity_add (a b : Usage) :
    requiresIdentity (Usage.add a b) = (requiresIdentity a || requiresIdentity b) := by
  have h : (1 ≤ a.lower + b.lower) ↔ (1 ≤ a.lower ∨ 1 ≤ b.lower) := by omega
  simp only [requiresIdentity, Usage.add]
  rw [← Bool.decide_or]
  exact decide_eq_decide.mpr h

/-- **Decision 1's moved question is §L07 sequential demand addition.** After a
dereference at `s` and no assignment, a later dereference at `t` adds to a
demand that already requires the identity (the requirement is the first use's,
unchanged by the addition), and the walk discharges the later check by the
first: its value is the one the first dereference left, `moved` to `s` when the
entry value was undecided. -/
theorem moved_question_is_sequential_addition (n : Nullness) (s t : Nat)
    (rest : List Event) (r : Bool) (n' : Nullness) (hna : noAssign rest = true)
    (h : walk (some n) (.deref s :: rest) = some n') :
    useGrade (.deref s :: (rest ++ [.deref t])) =
        Usage.add Usage.one (useGrade (rest ++ [.deref t])) ∧
      requiresIdentity (useGrade (.deref s :: (rest ++ [.deref t]))) =
        requiresIdentity (useGrade [.deref s]) ∧
      n' = afterDereference n s ∧
      discharged (check n' r) = true ∧
      ((∀ ps, n ≠ .nonNull ps) → check n' r = .moved) := by
  have hn' : n' = afterDereference n s := by
    rw [walk_cons] at h
    simp only [step] at h
    cases n with
    | nonNull ps => exact nonNull_kept rest ps n' hna h
    | null => exact nonNull_kept rest _ n' hna h
    | maybe _ => exact nonNull_kept rest _ n' hna h
  refine ⟨rfl, ?_, hn', ?_, ?_⟩
  · show requiresIdentity (Usage.add Usage.one _) = requiresIdentity (Usage.add Usage.one Usage.zero)
    rw [requiresIdentity_add, requiresIdentity_add]
    simp [requiresIdentity, Usage.one]
  · subst hn'
    cases n with
    | nonNull ps =>
        simp only [afterDereference, check]
        split <;> rfl
    | null => rfl
    | maybe _ => rfl
  · intro hnn
    subst hn'
    cases n with
    | nonNull ps => exact absurd rfl (hnn ps)
    | null => rfl
    | maybe _ => rfl

/-! ## The branch join of grades matches the walk's path join -/

theorem requiresIdentity_join (a b : Usage) :
    requiresIdentity (Usage.join a b) = (requiresIdentity a && requiresIdentity b) := by
  simp only [requiresIdentity, join_lower]
  apply Bool.eq_iff_iff.mpr
  simp only [decide_eq_true_eq, Bool.and_eq_true, Nat.le_min]

/-- A reading establishes the identity when it is non-null. -/
def established : Nullness → Bool
  | .nonNull _ => true
  | _ => false

/-- The walk's nullness join establishes the identity exactly when both arms
do, as the grade join requires it exactly when both arms do. -/
theorem established_join (a b : Nullness) :
    established (QCore.NullDeref.join a b) = (established a && established b) := by
  cases a <;> cases b <;> rfl

theorem relevant_admits_join_iff (a b : Usage) (ha : a.Valid) (hb : b.Valid) :
    Usage.relevant.Accepts (Usage.join a b) ↔
      Usage.relevant.Accepts a ∧ Usage.relevant.Accepts b := by
  rw [relevant_accepts_iff _ (Usage.join_valid ha hb), relevant_accepts_iff _ ha,
    relevant_accepts_iff _ hb, join_lower, Nat.le_min]

theorem inferred_append (xs ys : List First) (hx : xs ≠ []) (hy : ys ≠ []) :
    inferred (xs ++ ys) = (inferred xs && inferred ys) := by
  apply Bool.eq_iff_iff.mpr
  rw [Bool.and_eq_true, inferred_iff, inferred_iff, inferred_iff]
  constructor
  · rintro ⟨_, hall⟩
    exact ⟨⟨hx, fun f hf => hall f (List.mem_append_left _ hf)⟩,
      ⟨hy, fun f hf => hall f (List.mem_append_right _ hf)⟩⟩
  · rintro ⟨⟨_, h1⟩, ⟨_, h2⟩⟩
    refine ⟨by simp [hx], fun f hf => ?_⟩
    rcases List.mem_append.mp hf with h | h
    · exact h1 f h
    · exact h2 f h

private theorem max_assoc (a b c : UsageUpper) :
    UsageUpper.max (UsageUpper.max a b) c = UsageUpper.max a (UsageUpper.max b c) := by
  cases a <;> cases b <;> cases c <;> simp [UsageUpper.max, Nat.max_assoc]

theorem join_assoc (a b c : Usage) :
    Usage.join (Usage.join a b) c = Usage.join a (Usage.join b c) := by
  simp only [Usage.join, Nat.min_assoc, max_assoc]

theorem bodyGrade_append : ∀ (ps qs : List (List Event)) (a b : Usage),
    bodyGrade ps = some a → bodyGrade qs = some b → bodyGrade (ps ++ qs) = some (Usage.join a b)
  | [], _, _, _, h, _ => by simp [bodyGrade] at h
  | p :: ps, qs, a, b, ha, hb => by
      simp only [bodyGrade] at ha
      simp only [List.cons_append, bodyGrade]
      cases hps : bodyGrade ps with
      | none =>
          rw [hps] at ha; cases ha
          have : ps = [] := (bodyGrade_none_iff ps).mp hps
          subst this
          simp [hb]
      | some a' =>
          rw [hps] at ha; cases ha
          rw [bodyGrade_append ps qs a' b hps hb, join_assoc]

/-- **The branch join of grades matches the walk's path join**: the paths of
two arms join as the least enclosing interval of their grades; `relevant`
admits it exactly when the walk's union of the arms' `First` sets infers the
clause, which is exactly when each arm's does. -/
theorem branch_join_matches (ps qs : List (List Event)) (hp : ps ≠ []) (hq : qs ≠ []) :
    ∃ a b, bodyGrade ps = some a ∧ bodyGrade qs = some b ∧
      bodyGrade (ps ++ qs) = some (Usage.join a b) ∧
      (Usage.relevant.Accepts (Usage.join a b) ↔ inferred ((ps ++ qs).map firstOf) = true) ∧
      inferred ((ps ++ qs).map firstOf) = (inferred (ps.map firstOf) && inferred (qs.map firstOf)) := by
  cases ha : bodyGrade ps with
  | none => exact absurd ((bodyGrade_none_iff ps).mp ha) hp
  | some a =>
      cases hb : bodyGrade qs with
      | none => exact absurd ((bodyGrade_none_iff qs).mp hb) hq
      | some b =>
          have hj := bodyGrade_append ps qs a b ha hb
          refine ⟨a, b, rfl, rfl, hj, ?_, ?_⟩
          · rw [← relevant_accepts_body_iff_inferred, hj]
            simp
          · rw [List.map_append]
            exact inferred_append _ _ (by simpa using hp) (by simpa using hq)

/-! ## The caller obligation is grade admission against an existing identity -/

/-- **`QCore.NullDeref`'s caller obligation is grade admission**: the check of
an actual at a clause position is discharged exactly when every concrete value
the walk's reading admits (`true` = NULL) is an existing identity that
`relevant` admits. -/
theorem caller_obligation_is_grade_admission (actual : Nullness) (r : Bool) :
    discharged (check actual r) = true ↔
      ∀ v, sound actual v → admits Usage.relevant (!v) = true := by
  constructor
  · intro h v hs
    rw [QCore.NullDeref.check_sound actual r v hs h]
    rfl
  · intro h
    cases actual with
    | nonNull ps =>
        simp only [check]
        split <;> rfl
    | null =>
        have := h true rfl
        simp [admits, requiresIdentity, Usage.relevant] at this
    | maybe s =>
        have := h true trivial
        simp [admits, requiresIdentity, Usage.relevant] at this

/-- The callee's entry reading under its clause is an existing identity for
every value it is sound for: what `relevant` admits. -/
theorem entry_is_an_admitted_identity (v : Bool) (hs : sound entry v) :
    admits Usage.relevant (!v) = true := by
  simp only [entry, sound] at hs
  subst hs
  rfl

/-- `QCore.NullDeref.caller_obligation_establishes_the_entry`, recovered from
grade admission: a caller whose actual `relevant` admits supplies a value the
callee's entry reading is sound for. -/
theorem admission_establishes_the_entry (actual : Nullness) (r : Bool) (v : Bool)
    (hs : sound actual v) (h : discharged (check actual r) = true) : sound entry v := by
  have ha := (caller_obligation_is_grade_admission actual r).mp h v hs
  rw [relevant_admits_only_an_existing_identity] at ha
  simp only [entry, sound]
  simpa using ha

/-- **A declared clause stronger than its body is still sound** (the one-sided
body check): a `stronger-than-necessary` body's grade is one `relevant` does
not admit (a strict §L07 checker would reject the declaration), yet every
caller meeting the obligation supplies an existing identity, so the callee's
entry reading holds; the clause asks for a share it need not use, and spends
nothing. -/
theorem stronger_than_necessary_is_still_sound (paths : List (List Event))
    (hs : standing (paths.map firstOf) = "stronger-than-necessary") :
    ¬ (∃ g, bodyGrade paths = some g ∧ Usage.relevant.Accepts g) ∧
      ∀ (actual : Nullness) (r v : Bool), sound actual v →
        discharged (check actual r) = true → sound entry v := by
  refine ⟨fun h => ?_, admission_establishes_the_entry⟩
  have hn := (necessary_iff_relevant paths).mpr h
  rw [hs] at hn
  exact absurd hn (by decide)

/-! ## Vectors bound to the Rust -/

/-- A representative grade of one exit's `First`: a path that dereferenced
first demands at least one use (`one`); any other demands none (`zero`). -/
def firstGrade : First → Usage
  | .dereferenced => Usage.one
  | _ => Usage.zero

/-- The abstraction keeps exactly the identity requirement of every path. -/
theorem firstGrade_requires_iff (evs : List Event) :
    requiresIdentity (firstGrade (firstOf evs)) = requiresIdentity (pathGrade evs) := by
  apply Bool.eq_iff_iff.mpr
  rw [requiresIdentity_iff, requiresIdentity_iff, pathGrade_lower_pos_iff,
    ← firstOf_dereferenced_iff]
  cases firstOf evs <;> simp [firstGrade, Usage.one, Usage.zero]

/-- The joined representative grade of a non-empty exit list. -/
def exitGrade : List First → Option Usage
  | [] => none
  | f :: fs =>
      match exitGrade fs with
      | none => some (firstGrade f)
      | some g => some (Usage.join (firstGrade f) g)

def firstOfName : String → First
  | "dereferenced" => .dereferenced
  | "other" => .other
  | _ => .pending

/-- Exit `First`s, the joined grade's lower bound, `relevant`'s admission of it
and the standing: the Rust's `GRADE_VECTORS`
(`crates/qcore-core/tests/null_deref_grade.rs`). -/
def gradeVectors : List (List String × Nat × Bool × String) := [
  (["dereferenced"], 1, true, "necessary"),
  (["dereferenced", "dereferenced"], 1, true, "necessary"),
  (["pending"], 0, false, "stronger-than-necessary"),
  (["other"], 0, false, "stronger-than-necessary"),
  (["dereferenced", "other"], 0, false, "stronger-than-necessary"),
  (["dereferenced", "pending"], 0, false, "stronger-than-necessary"),
  (["other", "pending"], 0, false, "stronger-than-necessary")
]

theorem grade_vectors_hold :
    gradeVectors.all (fun (es, lower, admitted, name) =>
      match exitGrade (es.map firstOfName) with
      | none => false
      | some g =>
          g.lower == lower &&
            decide (Usage.relevant.Accepts g) == admitted &&
            inferred (es.map firstOfName) == admitted &&
            standing (es.map firstOfName) == name) = true := by
  decide

end QCore.NullDerefGrade
