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
# A qualified pointer declarator is a type group (card TYPE-GROUP-QUALIFIED-POINTER, rule 74u)

`cast_semantics::type_group_open` decides whether a parenthesised group is a
type name, so that family S of `legacy_return_family` reads through it (rule
12h(4)'s member count, rule 74m's through count, rule 74r's starred base). At
the base it accepted words followed only by trailing `*`s (`oldAccepts`), so
`( long * const )` was no group: `*(long * const)p` made no base and a null
untracked `p` was `Safe` (`old_misses_the_qualified_group`).

Rule 74u reads C11 6.7.7's abstract declarator `pointer`: words, then any
number of `*`, each followed by a possibly empty type-qualifier list
(`accepts`). It is exactly the grammar `TypeName` (`accepts_iff`, through
C11 6.7.6.1's right-recursive `Pointer`), and it accepts every group the base
accepted (`old_le_new`).

Every consumer only refuses more when it reads through more groups:

* the count through groups is monotone in the recogniser (`count_mono`), so a
  member or subscript root (count zero) and a starred may-bind root (count
  one both without and with the groups, under every reading) are admitted at
  the tip only where they were at the base (`member_admission_shrinks`,
  `starred_admission_shrinks`);
* rule 74r's reading through groups is monotone (`through_mono`), so the bases
  only grow (`bases_grow`);
* family S's admission (every may-bind root admitted, every base's
  obligation discharged) at the tip implies it at the base
  (`the_fix_only_refuses_more`, instantiated as `qualified_only_refuses_more`).

## What this is not

A model of the token scan or of the flow check. Each token's class (a `*`, a
type qualifier, another word, anything else) and each step back from a root
are the Rust reader's output (trust boundary 28), bound by
`typeGroupVectors`; a base's obligation is a predicate the flow check decides.
-/

namespace QCore.TypeGroupQualifiedPointer

/-! ## The recogniser -/

/-- One token inside a group's parentheses. -/
inductive Tok where
  /-- An identifier; `qual` when it is one of `POINTER_QUALIFIERS`. -/
  | ident (qual : Bool)
  /-- A `*`. -/
  | star
  /-- Anything else. -/
  | other
  deriving DecidableEq, Repr

/-- After the first `*`: only `*`s and type qualifiers. -/
def ptrs : List Tok → Bool
  | [] => true
  | .star :: r => ptrs r
  | .ident true :: r => ptrs r
  | _ => false

/-- The specifier-qualifier list, then the pointer part. -/
def specs : List Tok → Bool
  | [] => true
  | .ident _ :: r => specs r
  | .star :: r => ptrs r
  | .other :: _ => false

/-- Rule 74u's `type_group`: at least one word, then `( * qualifier* )*`. -/
def accepts : List Tok → Bool
  | .ident _ :: r => specs r
  | _ => false

/-- Only `*`s. -/
def allStars : List Tok → Bool
  | [] => true
  | .star :: r => allStars r
  | _ => false

/-- The base's words, then only trailing `*`s. -/
def oldSpecs : List Tok → Bool
  | [] => true
  | .ident _ :: r => oldSpecs r
  | .star :: r => allStars r
  | .other :: _ => false

/-- The base's `type_group_open`: at least one word, then only `*`s. -/
def oldAccepts : List Tok → Bool
  | .ident _ :: r => oldSpecs r
  | _ => false

theorem allStars_ptrs : ∀ l, allStars l = true → ptrs l = true
  | [], _ => rfl
  | .star :: r, h => by simp only [allStars, ptrs] at h ⊢; exact allStars_ptrs r h
  | .ident _ :: _, h => by simp [allStars] at h
  | .other :: _, h => by simp [allStars] at h

theorem oldSpecs_specs : ∀ l, oldSpecs l = true → specs l = true
  | [], _ => rfl
  | .ident _ :: r, h => by simp only [oldSpecs, specs] at h ⊢; exact oldSpecs_specs r h
  | .star :: r, h => by simp only [oldSpecs, specs] at h ⊢; exact allStars_ptrs r h
  | .other :: _, h => by simp [oldSpecs] at h

/-- Every group the base read is read. -/
theorem old_le_new (g : List Tok) (h : oldAccepts g = true) : accepts g = true := by
  match g, h with
  | .ident _ :: r, h => simpa [oldAccepts, accepts] using oldSpecs_specs r h

/-! ## The grammar (C11 6.7.6.1, 6.7.7) -/

/-- A type-qualifier list. -/
def Quals (qs : List Tok) : Prop := ∀ q ∈ qs, q = .ident true

/-- C11 6.7.6.1 `pointer`: `* type-qualifier-list_opt` or
`* type-qualifier-list_opt pointer`. -/
inductive Pointer : List Tok → Prop where
  | one (qs : List Tok) : Quals qs → Pointer (.star :: qs)
  | more (qs p : List Tok) : Quals qs → Pointer p → Pointer (.star :: (qs ++ p))

/-- A specifier-qualifier list, read as words. -/
def Words (ws : List Tok) : Prop := ∀ w ∈ ws, ∃ b, w = .ident b

/-- C11 6.7.7 `type-name` without a direct abstract declarator:
`specifier-qualifier-list pointer_opt`. -/
def TypeName (g : List Tok) : Prop :=
  ∃ ws p, ws ≠ [] ∧ Words ws ∧ (p = [] ∨ Pointer p) ∧ g = ws ++ p

/-- The alphabet after a `*`. -/
def PtrAlpha (l : List Tok) : Prop := ∀ t ∈ l, t = .star ∨ t = .ident true

theorem ptrs_iff : ∀ l, ptrs l = true ↔ PtrAlpha l
  | [] => by simp [ptrs, PtrAlpha]
  | .star :: r => by
      simp only [ptrs, ptrs_iff r, PtrAlpha, List.mem_cons, forall_eq_or_imp]; simp
  | .ident true :: r => by
      simp only [ptrs, ptrs_iff r, PtrAlpha, List.mem_cons, forall_eq_or_imp]; simp
  | .ident false :: r => by simp [ptrs, PtrAlpha]
  | .other :: r => by simp [ptrs, PtrAlpha]

theorem pointer_head : ∀ {l}, Pointer l → ∃ r, l = .star :: r
  | _, .one qs _ => ⟨qs, rfl⟩
  | _, .more qs p _ _ => ⟨qs ++ p, rfl⟩

theorem pointer_alpha : ∀ {l}, Pointer l → PtrAlpha l
  | _, .one qs h => by
      intro t ht
      rcases List.mem_cons.mp ht with rfl | ht
      · exact Or.inl rfl
      · exact Or.inr (h t ht)
  | _, .more qs p h hp => by
      intro t ht
      rcases List.mem_cons.mp ht with rfl | ht
      · exact Or.inl rfl
      · rcases List.mem_append.mp ht with ht | ht
        · exact Or.inr (h t ht)
        · exact pointer_alpha hp t ht

theorem pointer_cons_qual {r : List Tok} (h : Pointer (.star :: r)) :
    Pointer (.star :: .ident true :: r) := by
  cases h with
  | one _ hq =>
      exact .one (.ident true :: r) (by
        intro q hq'
        rcases List.mem_cons.mp hq' with rfl | hq'
        · rfl
        · exact hq q hq')
  | more qs p hq hp =>
      exact .more (.ident true :: qs) p (by
        intro q hq'
        rcases List.mem_cons.mp hq' with rfl | hq'
        · rfl
        · exact hq q hq') hp

theorem pointer_of_alpha : ∀ r, PtrAlpha r → Pointer (.star :: r)
  | [], _ => .one [] (by intro q hq; simp at hq)
  | t :: r, h => by
      have hr : PtrAlpha r := fun u hu => h u (List.mem_cons_of_mem _ hu)
      have ih := pointer_of_alpha r hr
      rcases h t (List.mem_cons_self ..) with rfl | rfl
      · exact .more [] (.star :: r) (by intro q hq; simp at hq) ih
      · exact pointer_cons_qual ih

theorem pointer_iff (r : List Tok) : Pointer (.star :: r) ↔ PtrAlpha r := by
  constructor
  · intro h t ht; exact pointer_alpha h t (List.mem_cons_of_mem _ ht)
  · exact pointer_of_alpha r

/-- `specs` reads a possibly empty word list then an optional pointer. -/
theorem specs_iff : ∀ l, specs l = true ↔
    ∃ ws p, Words ws ∧ (p = [] ∨ Pointer p) ∧ l = ws ++ p
  | [] => by
      simp only [specs, true_iff]
      exact ⟨[], [], by intro w hw; simp at hw, Or.inl rfl, rfl⟩
  | .ident b :: r => by
      simp only [specs]
      rw [specs_iff r]
      constructor
      · rintro ⟨ws, p, hw, hp, rfl⟩
        refine ⟨.ident b :: ws, p, ?_, hp, rfl⟩
        intro w hw'
        rcases List.mem_cons.mp hw' with rfl | hw'
        · exact ⟨b, rfl⟩
        · exact hw w hw'
      · rintro ⟨ws, p, hw, hp, he⟩
        cases ws with
        | nil =>
            simp only [List.nil_append] at he
            rcases hp with rfl | hp
            · simp at he
            · obtain ⟨r', hr'⟩ := pointer_head hp
              rw [hr'] at he; simp at he
        | cons w ws =>
            simp only [List.cons_append, List.cons.injEq] at he
            exact ⟨ws, p, fun u hu => hw u (List.mem_cons_of_mem _ hu), hp, he.2⟩
  | .star :: r => by
      simp only [specs]
      rw [ptrs_iff r]
      constructor
      · intro h
        exact ⟨[], .star :: r, by intro w hw; simp at hw,
          Or.inr ((pointer_iff r).mpr h), rfl⟩
      · rintro ⟨ws, p, hw, hp, he⟩
        cases ws with
        | nil =>
            simp only [List.nil_append] at he
            subst he
            rcases hp with hp | hp
            · simp at hp
            · exact (pointer_iff r).mp hp
        | cons w ws =>
            simp only [List.cons_append, List.cons.injEq] at he
            obtain ⟨b, hb⟩ := hw w (List.mem_cons_self ..)
            rw [hb] at he; simp at he
  | .other :: r => by
      simp only [specs, Bool.false_eq_true, false_iff, not_exists, not_and]
      intro ws p hw hp he
      cases ws with
      | nil =>
          simp only [List.nil_append] at he
          rcases hp with rfl | hp
          · simp at he
          · obtain ⟨r', hr'⟩ := pointer_head hp
            rw [hr'] at he; simp at he
      | cons w ws =>
          simp only [List.cons_append, List.cons.injEq] at he
          obtain ⟨b, hb⟩ := hw w (List.mem_cons_self ..)
          rw [hb] at he; simp at he

/-- The recogniser is exactly the grammar. -/
theorem accepts_iff (g : List Tok) : accepts g = true ↔ TypeName g := by
  unfold TypeName
  match g with
  | [] =>
      simp only [accepts, Bool.false_eq_true, false_iff, not_exists, not_and]
      intro ws p hne _ _ he
      cases ws with
      | nil => exact hne rfl
      | cons _ _ => simp at he
  | .star :: r =>
      simp only [accepts, Bool.false_eq_true, false_iff, not_exists, not_and]
      intro ws p hne hw _ he
      cases ws with
      | nil => exact hne rfl
      | cons w ws =>
          simp only [List.cons_append, List.cons.injEq] at he
          obtain ⟨b, hb⟩ := hw w (List.mem_cons_self ..)
          rw [hb] at he; simp at he
  | .other :: r =>
      simp only [accepts, Bool.false_eq_true, false_iff, not_exists, not_and]
      intro ws p hne hw _ he
      cases ws with
      | nil => exact hne rfl
      | cons w ws =>
          simp only [List.cons_append, List.cons.injEq] at he
          obtain ⟨b, hb⟩ := hw w (List.mem_cons_self ..)
          rw [hb] at he; simp at he
  | .ident b :: r =>
      simp only [accepts]
      rw [specs_iff r]
      constructor
      · rintro ⟨ws, p, hw, hp, rfl⟩
        refine ⟨.ident b :: ws, p, by simp, ?_, hp, rfl⟩
        intro w hw'
        rcases List.mem_cons.mp hw' with rfl | hw'
        · exact ⟨b, rfl⟩
        · exact hw w hw'
      · rintro ⟨ws, p, _, hw, hp, he⟩
        cases ws with
        | nil =>
            simp only [List.nil_append] at he
            rcases hp with rfl | hp
            · simp at he
            · obtain ⟨r', hr'⟩ := pointer_head hp
              rw [hr'] at he; simp at he
        | cons w ws =>
            simp only [List.cons_append, List.cons.injEq] at he
            exact ⟨ws, p, fun u hu => hw u (List.mem_cons_of_mem _ hu), hp, he.2⟩

/-- A qualifier after a `*` is a group now and was not at the base. -/
theorem the_qualified_group_is_new :
    accepts [.ident false, .star, .ident true] = true ∧
      oldAccepts [.ident false, .star, .ident true] = false := by decide

/-- A word after a `*` that is no qualifier (`( a * b )`) is still no group. -/
theorem a_product_is_no_group :
    accepts [.ident false, .star, .ident false] = false := by decide

/-! ## The consumers -/

/-- One step back from a root, nearest first. -/
inductive Item where
  /-- A `*`, `unary` under the reading counted (rule 74h's `operator_at`,
  which `type_group_open` does not decide). -/
  | star (unary : Bool)
  /-- A `(`. -/
  | opening
  /-- A `)` closing the group `g`. -/
  | close (g : List Tok)
  /-- Anything else: the walk stops. -/
  | other
  deriving DecidableEq, Repr

/-- `stars_before` under a recogniser `rec`: the unary `*`s reached through
`(` and through groups `rec` accepts; a `*` behind more than `closing`
opening parentheses stops it. With `rec` constantly false this is the count
without `through` (rule 74m's inner count). -/
def count (rec : List Tok → Bool) (closing : Nat) : Nat → List Item → Nat
  | _, [] => 0
  | opened, .star true :: rest =>
      if closing < opened then 0 else count rec closing opened rest + 1
  | _, .star false :: _ => 0
  | opened, .opening :: rest => count rec closing (opened + 1) rest
  | opened, .close g :: rest => if rec g then count rec closing opened rest else 0
  | _, .other :: _ => 0

/-- A recogniser that accepts at least what another does. -/
def Le (rec rec' : List Tok → Bool) : Prop := ∀ g, rec g = true → rec' g = true

theorem count_mono {rec rec' : List Tok → Bool} (h : Le rec rec') (closing : Nat) :
    ∀ opened items, count rec closing opened items ≤ count rec' closing opened items
  | _, [] => Nat.le_refl _
  | opened, .star true :: rest => by
      simp only [count]
      split
      · exact Nat.le_refl _
      · exact Nat.succ_le_succ (count_mono h closing opened rest)
  | _, .star false :: _ => Nat.le_refl _
  | opened, .opening :: rest => by
      simp only [count]; exact count_mono h closing (opened + 1) rest
  | opened, .close g :: rest => by
      simp only [count]
      cases hg : rec g with
      | false => simp
      | true =>
          simp only [h g hg, ite_true]
          exact count_mono h closing opened rest
  | _, .other :: _ => Nat.le_refl _

/-- The recogniser that accepts nothing (no `through`). -/
def none : List Tok → Bool := fun _ => false

theorem none_le (rec : List Tok → Bool) : Le none rec := by
  intro g hg; simp [none] at hg

/-- Rule 12h(4)'s count before a member or subscript root (`closing` is
`usize::MAX` there, any `bound` here); the root is admitted when it is zero. -/
def memberCount (rec : List Tok → Bool) (bound : Nat) (items : List Item) : Nat :=
  count rec bound 0 items

theorem member_admission_shrinks {rec rec' : List Tok → Bool} (h : Le rec rec')
    (bound : Nat) (items : List Item) (hn : memberCount rec' bound items = 0) :
    memberCount rec bound items = 0 :=
  Nat.eq_zero_of_le_zero (hn ▸ count_mono h bound 0 items)

/-- Rule 74m: a starred may-bind root is admitted when, under every reading
(each reading's items), the count is one without and with the groups. -/
def starredAdmitted (rec : List Tok → Bool) (closing : Nat)
    (readings : List (List Item)) : Prop :=
  ∀ items ∈ readings, count none closing 0 items = 1 ∧ count rec closing 0 items = 1

theorem starred_admission_shrinks {rec rec' : List Tok → Bool} (h : Le rec rec')
    (closing : Nat) (readings : List (List Item))
    (ha : starredAdmitted rec' closing readings) : starredAdmitted rec closing readings := by
  intro items hi
  obtain ⟨hinner, hthrough⟩ := ha items hi
  refine ⟨hinner, ?_⟩
  have lo := count_mono (none_le rec) closing 0 items
  have hi' := count_mono h closing 0 items
  omega

/-- Rule 74r's reading back through `(` and accepted groups to a `*`. -/
def through (rec : List Tok → Bool) : List Item → Bool
  | [] => false
  | .star u :: _ => u
  | .opening :: rest => through rec rest
  | .close g :: rest => rec g && through rec rest
  | .other :: _ => false

theorem through_mono {rec rec' : List Tok → Bool} (h : Le rec rec') :
    ∀ items, through rec items = true → through rec' items = true
  | [], hi => hi
  | .star _ :: _, hi => hi
  | .opening :: rest, hi => through_mono h rest hi
  | .close g :: rest, hi => by
      simp only [through, Bool.and_eq_true] at hi ⊢
      exact ⟨h g hi.1, through_mono h rest hi.2⟩
  | .other :: _, hi => hi

/-- An untracked root is a base when the old test (a `*` right before it,
which no group decides) or the reading through groups says so. -/
def based (rec : List Tok → Bool) (old : Bool) (items : List Item) : Bool :=
  old || through rec items

theorem bases_grow {rec rec' : List Tok → Bool} (h : Le rec rec') (old : Bool)
    (items : List Item) (hb : based rec old items = true) : based rec' old items = true := by
  simp only [based, Bool.or_eq_true] at hb ⊢
  exact hb.imp id (through_mono h items)

/-- A root of a `return` expression family S reads, as the reader classes it. -/
inductive Root where
  /-- An untracked root: its old starred test and its steps back. -/
  | untracked (old : Bool) (items : List Item)
  /-- A may-bind starred root: its `closing` and each reading's steps. -/
  | starred (closing : Nat) (readings : List (List Item))
  /-- A may-bind member or subscript root under the cast reading. -/
  | member (bound : Nat) (items : List Item)

/-- Family S's admission under a recogniser: every may-bind root is
admitted, and every base's obligation (`ok`, the flow check's) holds. -/
def Admits (rec : List Tok → Bool) (ok : Root → Prop) (roots : List Root) : Prop :=
  ∀ r ∈ roots, match r with
    | .untracked old items => based rec old items = true → ok r
    | .starred closing readings => starredAdmitted rec closing readings ∧ ok r
    | .member bound items => memberCount rec bound items = 0 ∧ ok r

/-- Recognising more groups only refuses more. -/
theorem the_fix_only_refuses_more {rec rec' : List Tok → Bool} (h : Le rec rec')
    (ok : Root → Prop) (roots : List Root) (ha : Admits rec' ok roots) :
    Admits rec ok roots := by
  intro r hr
  have := ha r hr
  match r, this with
  | .untracked old items, hu => exact fun hb => hu (bases_grow h old items hb)
  | .starred closing readings, ⟨hs, hok⟩ =>
      exact ⟨starred_admission_shrinks h closing readings hs, hok⟩
  | .member bound items, ⟨hm, hok⟩ =>
      exact ⟨member_admission_shrinks h bound items hm, hok⟩

/-- The card's instance: whatever the tip admits, the base admitted. No
`Unknown` moves to `Safe`. -/
theorem qualified_only_refuses_more (ok : Root → Prop) (roots : List Root)
    (ha : Admits accepts ok roots) : Admits oldAccepts ok roots :=
  the_fix_only_refuses_more old_le_new ok roots ha

/-- `* ( long * const ) p` for an untracked `p`: no base at the base, a base
at the tip (the old test sees `)` before `p`). -/
theorem old_misses_the_qualified_group :
    based oldAccepts false [.close [.ident false, .star, .ident true], .star true] = false ∧
      based accepts false [.close [.ident false, .star, .ident true], .star true] = true := by
  decide

/-- The count's own witness, `* ( long * const ) ( long * ) * q` for a
may-bind `q`: one `*` without the groups, and through them one at the base
(it stopped at the qualified group) but two at the tip. The model omits
family S's other refusals: the reader already refuses this spelling, since a
`(` right after a group `is_cast_group` does not resolve is read as a call,
and a `*` right after a qualified group is undecided (rule 74h). -/
theorem a_second_dereference_refuses :
    starredAdmitted oldAccepts 0
        [[.star true, .close [.ident false, .star],
          .close [.ident false, .star, .ident true], .star true]] ∧
      ¬ starredAdmitted accepts 0
        [[.star true, .close [.ident false, .star],
          .close [.ident false, .star, .ident true], .star true]] := by
  constructor
  · intro items hi
    simp only [List.mem_singleton] at hi
    subst hi
    decide
  · intro ha
    have := (ha _ (List.mem_singleton_self _)).2
    revert this
    decide

/-- `* ( long * const ) q -> f` for a may-bind `q`: rule 12h(4)'s count is
zero at the base and one at the tip, so the member root now refuses. -/
theorem a_member_root_through_the_group_refuses :
    memberCount oldAccepts 1000 [.close [.ident false, .star, .ident true], .star true] = 0 ∧
      memberCount accepts 1000 [.close [.ident false, .star, .ident true], .star true] = 1 := by
  decide

/-! ## The token reader's vectors -/

/-- Abbreviations for the vectors: a word, a qualifier, a `*`, anything else. -/
abbrev w : Tok := .ident false
abbrev q : Tok := .ident true
abbrev s : Tok := .star
abbrev o : Tok := .other

/-- `(spelling between the parentheses, tokens, base reading, tip reading)`,
read by `crates/qcore-c-gateway/tests/unit/cast_semantics_type_group.rs`. -/
def typeGroupVectors : List (String × List Tok × Bool × Bool) := [
  ("long", [w], true, true),
  ("long *", [w, s], true, true),
  ("unsigned long * *", [w, w, s, s], true, true),
  ("const long *", [q, w, s], true, true),
  ("const char *", [q, w, s], true, true),
  ("struct s *", [w, w, s], true, true),
  ("long * const", [w, s, q], false, true),
  ("long * volatile", [w, s, q], false, true),
  ("long * restrict", [w, s, q], false, true),
  ("long * __restrict", [w, s, q], false, true),
  ("long * const volatile", [w, s, q, q], false, true),
  ("long * _Nonnull", [w, s, q], false, true),
  ("long * _Atomic", [w, s, q], false, true),
  ("const char * const", [q, w, s, q], false, true),
  ("const char * const *", [q, w, s, q, s], false, true),
  ("const char * const * const", [q, w, s, q, s, q], false, true),
  ("char * * const", [w, s, s, q], false, true),
  ("long * const *", [w, s, q, s], false, true),
  ("a * b", [w, s, w], false, false),
  ("* const", [s, q], false, false),
  ("a + b", [w, o, w], false, false),
  ("", [], false, false)
]

theorem type_group_vectors_hold :
    typeGroupVectors.all (fun (_, toks, old, new) =>
      oldAccepts toks == old && accepts toks == new) = true := by decide

/-- On every vector the tip reads at least what the base read. -/
theorem vectors_only_add :
    typeGroupVectors.all (fun (_, _, old, new) => !old || new) = true := by decide

end QCore.TypeGroupQualifiedPointer
