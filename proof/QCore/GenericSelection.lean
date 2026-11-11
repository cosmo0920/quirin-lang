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
# C11 generic selection, read by the selection-invariant route

Card **W44-C11-GENERIC-SELECTION**. C11 §6.5.1.1 gives
`_Generic (controlling-expression, T1: e1, …, default: ed)` this meaning: the
controlling expression is **not evaluated**; its type selects exactly one
association at translation time; and the result is the selected expression,
evaluated. The analyzer has no C type checker, so it cannot say which
association the compiler selects.

glibc 2.43's C23 const-preserving string macros expand to the one shape the
corpus contains:

```text
_Generic (0 ? (S) : (void *) 1, const void *: (const char *) (strchr (S, C)),
          default: strchr (S, C))
```

Its associations are the same expression up to a pointer cast. The card's
reader, `legacy_generic_selection::resolve_in_body`, admits a selection only
when every association is one common expression `E` after removing enclosing
parentheses and pointer casts, and lowers the whole selection to `E`, dropping
the controlling expression. When any cast was removed, `E` must also be a call
of a function the unit declares with an object-pointer result, so every
removed cast converts a pointer to a pointer. Anything else is left exactly as
it was, and the ordinary classifier refuses it.

## The model

`Expr` is an abstract C expression: an opaque `leaf` (a call, a name, a
literal: whatever the ordinary classifier reads), a `cast` to a type label, a
parenthesis, a binary operator, and a generic selection. `eval` is a
deterministic evaluation to an effect trace and a value; `none` is an
execution C does not define (a selection no association matches is a
constraint violation, so a compiled program never reaches it). The selection
itself is `Sem.select`, a function of the controlling expression and the
association keys only, standing for the compiler's typing, which this model
does not reproduce and does not need to.

## What is proved

* **The controlling expression is unevaluated** — two selections that differ
  only in their controlling expression, and select the same association,
  evaluate identically (`the_control_is_unevaluated`); the control's own
  effects never appear (`a_controls_effect_never_appears`).
* **Exactly the selected association is evaluated**
  (`exactly_the_selected_association_is_evaluated`).
* **Selection invariance (route 1)** — when the reader admits a selection,
  every association that evaluates yields the common core's outcome
  (`an_admitted_association_evaluates_as_the_core`), so the lowered effect is
  independent of which association the compiler selects
  (`the_lowered_effect_is_independent_of_the_selection`).
* **The lowering is C evaluation** — `resolve` rewrites every admitted
  selection, innermost first, and preserves every defined outcome of every
  expression, whatever the compiler's selection (`resolve_preserves_outcomes`).
  A refused selection is left untouched (`a_refused_selection_is_unchanged`).
* **What refuses** — associations that differ after the removals
  (`differing_associations_refuse`), a removed cast when the core is not a
  declared pointer result (`a_cast_without_a_pointer_core_refuses`), a
  non-pointer cast (`a_non_pointer_cast_is_not_removed`), two `default`
  associations and an empty association list.
* **Non-vacuity** — the glibc shape is admitted in its const and non-const
  forms and resolves to the call; a nested selection resolves innermost first;
  a non-invariant selection's two associations have different outcomes, so no
  single lowering is right for both; an integer cast changes the value, so the
  pointer-result premise is load-bearing; and a controlling expression with a
  side effect contributes nothing.

## What this is not

Two premises are stated, not proved (trust boundary 28). **Value
preservation**: a pointer-to-object-pointer conversion leaves the object the
pointer designates unchanged (C11 §6.3.2.3p2 and p7 under the no-undefined-
behaviour object model the gateway already assumes, `AOM-POISON-NO-UB`); it is
the hypothesis `ValuePreserving`. **Declaration faithfulness**: a call of a
function the unit's C declaration types as returning an object pointer yields a
pointer; it is the hypothesis `PointerResults`, the same token reader premise
constraint 8's ABI gate rests on. That the token reader parses a cast, its
operand and the association list as this AST does is the extraction premise:
it is why the Rust reader removes a cast only when its operand is a
parenthesised group, a direct call or a single name.
-/

namespace QCore.GenericSelection

-- Abstract expressions. Type labels and opaque leaves are named by `Nat`.
mutual
/-- An abstract C expression. -/
inductive Expr where
  /-- Any expression the ordinary classifier reads: a call, a name, a literal. -/
  | leaf (b : Nat)
  /-- `( T ) e`. -/
  | cast (t : Nat) (e : Expr)
  /-- `( e )`. -/
  | paren (e : Expr)
  /-- A binary operator. -/
  | op (o : Nat) (l r : Expr)
  /-- `_Generic ( control , assocs )`. -/
  | generic (control : Expr) (assocs : Assocs)
  deriving DecidableEq, Repr

/-- The association list; key `none` is `default`. -/
inductive Assocs where
  | nil
  | cons (key : Option Nat) (e : Expr) (rest : Assocs)
  deriving DecidableEq, Repr
end

/-- An effect trace and a value. -/
structure Outcome where
  effects : List Nat
  value : Nat
  deriving DecidableEq, Repr

/-- The environment an evaluation is relative to. -/
structure Sem where
  /-- An opaque expression's outcome; `none` when C does not define one. -/
  leafEval : Nat → Option Outcome
  /-- The value a cast to type label `t` produces. -/
  castValue : Nat → Nat → Nat
  /-- A binary operator's outcome from its operands' (sequenced left first;
  the order is immaterial to every statement below). -/
  combine : Nat → Outcome → Outcome → Outcome
  /-- The compiler's selection: from the controlling expression and the keys. -/
  select : Expr → List (Option Nat) → Nat
  /-- Whether a value is a pointer. -/
  isPointer : Nat → Bool

/-- The keys of an association list, in order. -/
def keys : Assocs → List (Option Nat)
  | .nil => []
  | .cons k _ rest => k :: keys rest

mutual
/-- C evaluation. -/
def eval (s : Sem) : Expr → Option Outcome
  | .leaf b => s.leafEval b
  | .cast t e => (eval s e).map fun o => { o with value := s.castValue t o.value }
  | .paren e => eval s e
  | .op o l r =>
      match eval s l, eval s r with
      | some a, some b => some (s.combine o a b)
      | _, _ => none
  | .generic c as => evalAt s (s.select c (keys as)) as

/-- Evaluate the association at index `i`, and nothing else. -/
def evalAt (s : Sem) : Nat → Assocs → Option Outcome
  | _, .nil => none
  | 0, .cons _ e _ => eval s e
  | n + 1, .cons _ _ rest => evalAt s n rest
end

/-- The association expression at index `i`. -/
def nth : Nat → Assocs → Option Expr
  | _, .nil => none
  | 0, .cons _ e _ => some e
  | n + 1, .cons _ _ rest => nth n rest

/-! ## The controlling expression and the selected association -/

/-- **Exactly the selected association is evaluated.** -/
theorem exactly_the_selected_association_is_evaluated (s : Sem) :
    ∀ (as : Assocs) (i : Nat) (o : Outcome), evalAt s i as = some o →
      ∃ e, nth i as = some e ∧ eval s e = some o
  | .nil, _, _, h => by simp [evalAt] at h
  | .cons _ e _, 0, _, h => ⟨e, rfl, by simpa [evalAt] using h⟩
  | .cons _ _ rest, n + 1, o, h => by
      simp only [evalAt] at h
      exact exactly_the_selected_association_is_evaluated s rest n o h

/-- **The controlling expression is unevaluated**: only the selection it
determines matters, never its outcome. -/
theorem the_control_is_unevaluated (s : Sem) (c c' : Expr) (as : Assocs)
    (same : s.select c (keys as) = s.select c' (keys as)) :
    eval s (.generic c as) = eval s (.generic c' as) := by
  simp only [eval, same]

/-! ## The reader -/

/-- Remove enclosing parentheses and casts to pointer type labels. The flag
records whether a cast was removed. -/
def strip (pt : Nat → Bool) : Expr → Expr × Bool
  | .paren e => strip pt e
  | .cast t e => if pt t then ((strip pt e).1, true) else (.cast t e, false)
  | e => (e, false)

/-- Every association's stripped core and cast flag. -/
def cores (pt : Nat → Bool) : Assocs → List (Expr × Bool)
  | .nil => []
  | .cons _ e rest => strip pt e :: cores pt rest

/-- The number of `default` associations. -/
def defaults : Assocs → Nat
  | .nil => 0
  | .cons none _ rest => defaults rest + 1
  | .cons (some _) _ rest => defaults rest

/-- The selection-invariant reader. `pt` says a type label is a pointer type;
`pr` says an expression is a call of a function declared with an
object-pointer result. -/
def reader (pt : Nat → Bool) (pr : Expr → Bool) (as : Assocs) : Option Expr :=
  match cores pt as with
  | [] => none
  | (c0, _) :: _ =>
    if (cores pt as).all (fun p => decide (p.1 = c0)) && decide (defaults as ≤ 1) then
      if (cores pt as).any (fun p => p.2) then (if pr c0 then some c0 else none) else some c0
    else none

mutual
/-- Resolve every admitted selection, innermost first. The controlling
expression is never rewritten; a refused selection is left as it was. -/
def resolve (pt : Nat → Bool) (pr : Expr → Bool) : Expr → Expr
  | .leaf b => .leaf b
  | .cast t e => .cast t (resolve pt pr e)
  | .paren e => .paren (resolve pt pr e)
  | .op o l r => .op o (resolve pt pr l) (resolve pt pr r)
  | .generic c as =>
      match reader pt pr (resolveAssocs pt pr as) with
      | some core => core
      | none => .generic c as

def resolveAssocs (pt : Nat → Bool) (pr : Expr → Bool) : Assocs → Assocs
  | .nil => .nil
  | .cons k e rest => .cons k (resolve pt pr e) (resolveAssocs pt pr rest)
end

/-! ## The premises -/

/-- A cast to a pointer type label leaves a pointer value unchanged. -/
def ValuePreserving (s : Sem) (pt : Nat → Bool) : Prop :=
  ∀ t v, pt t = true → s.isPointer v = true → s.castValue t v = v

/-- An expression `pr` accepts yields a pointer whenever it yields anything. -/
def PointerResults (s : Sem) (pr : Expr → Bool) : Prop :=
  ∀ e o, pr e = true → eval s e = some o → s.isPointer o.value = true

/-! ## Soundness -/

/-- Removing parentheses and pointer casts preserves the outcome, provided the
core yields a pointer whenever a cast was removed. -/
theorem strip_preserves (s : Sem) (pt : Nat → Bool) (hv : ValuePreserving s pt) :
    ∀ (e : Expr) (o : Outcome), eval s e = some o →
      ((strip pt e).2 = true →
        ∀ o', eval s (strip pt e).1 = some o' → s.isPointer o'.value = true) →
      eval s (strip pt e).1 = some o
  | .leaf _, _, h, _ => by simpa [strip] using h
  | .op _ _ _, _, h, _ => by simpa [strip] using h
  | .generic _ _, _, h, _ => by simpa [strip] using h
  | .paren e, o, h, hp => by
      simp only [strip] at hp ⊢
      exact strip_preserves s pt hv e o (by simpa [eval] using h) hp
  | .cast t e, o, h, hp => by
      by_cases ht : pt t = true
      · simp only [strip, ht, ↓reduceIte] at hp ⊢
        simp only [eval] at h
        cases he : eval s e with
        | none => simp [he] at h
        | some o' =>
          simp only [he, Option.map_some, Option.some.injEq] at h
          have hcore : eval s (strip pt e).1 = some o' :=
            strip_preserves s pt hv e o' he (fun _ => hp trivial)
          rw [hcore, ← h, hv t o'.value ht (hp trivial o' hcore)]
      · have hf : pt t = false := by simpa using ht
        simp only [strip, hf, Bool.false_eq_true, ↓reduceIte]
        exact h

theorem strip_mem_cores (pt : Nat → Bool) :
    ∀ (as : Assocs) (i : Nat) (e : Expr), nth i as = some e → strip pt e ∈ cores pt as
  | .nil, _, _, h => by simp [nth] at h
  | .cons _ e' _, 0, e, h => by
      simp only [nth, Option.some.injEq] at h
      subst h
      simp [cores]
  | .cons _ _ rest, n + 1, e, h => by
      simp only [nth] at h
      simp only [cores, List.mem_cons]
      exact Or.inr (strip_mem_cores pt rest n e h)

/-- What an admission states: every association strips to the core, and a
removed cast implies a declared pointer result. -/
theorem reader_admits (pt : Nat → Bool) (pr : Expr → Bool) (as : Assocs) (core : Expr)
    (h : reader pt pr as = some core) :
    (∀ p ∈ cores pt as, p.1 = core) ∧
      ((∃ p ∈ cores pt as, p.2 = true) → pr core = true) := by
  unfold reader at h
  split at h
  · simp at h
  · rename_i c0 flag rest hcs
    rw [hcs]
    split at h
    · rename_i hall
      simp only [hcs, Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq] at hall
      split at h
      · rename_i hany
        split at h
        · rename_i hpr
          simp only [Option.some.injEq] at h
          subst h
          exact ⟨hall.1, fun _ => hpr⟩
        · simp at h
      · rename_i hany
        simp only [Option.some.injEq] at h
        subst h
        refine ⟨hall.1, fun ⟨p, hp, hp2⟩ => ?_⟩
        rw [hcs] at hany
        exact absurd (List.any_eq_true.mpr ⟨p, hp, hp2⟩) hany
    · simp at h

/-- **Selection invariance.** When the reader admits a selection, whichever
association the compiler selects evaluates exactly as the common core. -/
theorem an_admitted_association_evaluates_as_the_core (s : Sem) (pt : Nat → Bool)
    (pr : Expr → Bool) (hv : ValuePreserving s pt) (hp : PointerResults s pr)
    (as : Assocs) (core : Expr) (h : reader pt pr as = some core)
    (i : Nat) (o : Outcome) (hi : evalAt s i as = some o) : eval s core = some o := by
  obtain ⟨e, he, heval⟩ := exactly_the_selected_association_is_evaluated s as i o hi
  have hmem := strip_mem_cores pt as i e he
  obtain ⟨hcore, hcast⟩ := reader_admits pt pr as core h
  have hc : (strip pt e).1 = core := hcore _ hmem
  have := strip_preserves s pt hv e o heval (fun hflag o' ho' => by
    have hprc : pr core = true := hcast ⟨_, hmem, hflag⟩
    rw [hc] at ho'
    exact hp core o' hprc ho')
  rwa [hc] at this

/-- **The lowered effect is independent of the selection.** -/
theorem the_lowered_effect_is_independent_of_the_selection (s : Sem) (pt : Nat → Bool)
    (pr : Expr → Bool) (hv : ValuePreserving s pt) (hp : PointerResults s pr)
    (as : Assocs) (core : Expr) (h : reader pt pr as = some core)
    (i j : Nat) (o o' : Outcome) (hi : evalAt s i as = some o) (hj : evalAt s j as = some o') :
    o = o' := by
  have a := an_admitted_association_evaluates_as_the_core s pt pr hv hp as core h i o hi
  have b := an_admitted_association_evaluates_as_the_core s pt pr hv hp as core h j o' hj
  rw [a] at b
  exact Option.some.inj b

mutual
/-- **The lowering is C evaluation.** Resolving every admitted selection,
innermost first, preserves every defined outcome, whatever the compiler
selects. -/
theorem resolve_preserves_outcomes (s : Sem) (pt : Nat → Bool) (pr : Expr → Bool)
    (hv : ValuePreserving s pt) (hp : PointerResults s pr) :
    ∀ (e : Expr) (o : Outcome), eval s e = some o → eval s (resolve pt pr e) = some o
  | .leaf _, _, h => by simpa [resolve] using h
  | .cast t e, o, h => by
      simp only [resolve, eval] at h ⊢
      cases he : eval s e with
      | none => simp [he] at h
      | some o' =>
        rw [resolve_preserves_outcomes s pt pr hv hp e o' he]
        simpa [he] using h
  | .paren e, o, h => by
      simp only [resolve, eval] at h ⊢
      exact resolve_preserves_outcomes s pt pr hv hp e o h
  | .op x l r, o, h => by
      simp only [resolve, eval] at h ⊢
      cases hl : eval s l with
      | none => simp [hl] at h
      | some a =>
        cases hr : eval s r with
        | none => simp [hl, hr] at h
        | some b =>
          rw [resolve_preserves_outcomes s pt pr hv hp l a hl,
            resolve_preserves_outcomes s pt pr hv hp r b hr]
          simpa [hl, hr] using h
  | .generic c as, o, h => by
      simp only [resolve]
      split
      · rename_i core hcore
        simp only [eval] at h
        exact an_admitted_association_evaluates_as_the_core s pt pr hv hp _ core hcore _ o
          (resolve_assocs_preserves_outcomes s pt pr hv hp as _ o h)
      · exact h

theorem resolve_assocs_preserves_outcomes (s : Sem) (pt : Nat → Bool) (pr : Expr → Bool)
    (hv : ValuePreserving s pt) (hp : PointerResults s pr) :
    ∀ (as : Assocs) (i : Nat) (o : Outcome),
      evalAt s i as = some o → evalAt s i (resolveAssocs pt pr as) = some o
  | .nil, _, _, h => by simp [evalAt] at h
  | .cons _ e _, 0, o, h => by
      simp only [resolveAssocs, evalAt] at h ⊢
      exact resolve_preserves_outcomes s pt pr hv hp e o h
  | .cons _ _ rest, n + 1, o, h => by
      simp only [resolveAssocs, evalAt] at h ⊢
      exact resolve_assocs_preserves_outcomes s pt pr hv hp rest n o h
end

/-- A refused selection is left exactly as it was. -/
theorem a_refused_selection_is_unchanged (pt : Nat → Bool) (pr : Expr → Bool)
    (c : Expr) (as : Assocs) (h : reader pt pr (resolveAssocs pt pr as) = none) :
    resolve pt pr (.generic c as) = .generic c as := by
  simp [resolve, h]

/-- The reader never reads the controlling expression: an admitted selection
resolves to the same core whatever its control. -/
theorem the_reader_ignores_the_control (pt : Nat → Bool) (pr : Expr → Bool)
    (c c' : Expr) (as : Assocs) (core : Expr)
    (h : reader pt pr (resolveAssocs pt pr as) = some core) :
    resolve pt pr (.generic c as) = core ∧ resolve pt pr (.generic c' as) = core := by
  simp [resolve, h]

/-! ## What refuses -/

/-- Two associations whose cores differ refuse. -/
theorem differing_associations_refuse (pt : Nat → Bool) (pr : Expr → Bool) (as : Assocs)
    (p q : Expr × Bool) (hp : p ∈ cores pt as) (hq : q ∈ cores pt as) (hne : p.1 ≠ q.1) :
    reader pt pr as = none := by
  cases h : reader pt pr as with
  | none => rfl
  | some core =>
    obtain ⟨hall, _⟩ := reader_admits pt pr as core h
    exact absurd ((hall p hp).trans (hall q hq).symm) hne

/-- A removed cast whose core is not a declared pointer result refuses. -/
theorem a_cast_without_a_pointer_core_refuses (pt : Nat → Bool) (pr : Expr → Bool)
    (as : Assocs) (p : Expr × Bool) (hp : p ∈ cores pt as) (hcast : p.2 = true)
    (hpr : pr p.1 = false) : reader pt pr as = none := by
  cases h : reader pt pr as with
  | none => rfl
  | some core =>
    obtain ⟨hall, hc⟩ := reader_admits pt pr as core h
    have := hc ⟨p, hp, hcast⟩
    rw [← hall p hp, hpr] at this
    exact absurd this (by decide)

/-- A cast to a type label that is not a pointer type is never removed. -/
theorem a_non_pointer_cast_is_not_removed (pt : Nat → Bool) (t : Nat) (e : Expr)
    (h : pt t = false) : strip pt (.cast t e) = (.cast t e, false) := by
  simp [strip, h]

/-- Two `default` associations refuse. -/
theorem two_defaults_refuse (pt : Nat → Bool) (pr : Expr → Bool) (as : Assocs)
    (h : 1 < defaults as) : reader pt pr as = none := by
  unfold reader
  split
  · rfl
  · have : decide (defaults as ≤ 1) = false := by simp; omega
    simp [this]

/-- An empty association list refuses. -/
theorem no_association_refuses (pt : Nat → Bool) (pr : Expr → Bool) :
    reader pt pr .nil = none := rfl

/-! ## Non-vacuity: the glibc shape and its neighbours

Type labels: `1` is `const char *`, `2` is `const void *` (the key), `3` is
`long` (not a pointer). Leaves: `10` is `strchr (S, C)`, `11` the controlling
expression `0 ? (S) : (void *) 1`, `12` a different call, `13` an integer call
such as `strlen (S)`. -/

/-- Pointer type labels. -/
def ptEx (t : Nat) : Bool := t == 1 || t == 2

/-- The calls declared with an object-pointer result. -/
def prEx (e : Expr) : Bool := e == .leaf 10 || e == .leaf 12

/-- The glibc expansion `__glibc_const_generic (S, const char *, strchr (S, C))`. -/
def glibc : Expr :=
  .generic (.leaf 11) (.cons (some 2) (.cast 1 (.paren (.leaf 10))) (.cons none (.leaf 10) .nil))

/-- A selection whose associations are different calls. -/
def differing : Expr :=
  .generic (.leaf 11) (.cons (some 2) (.leaf 12) (.cons none (.leaf 10) .nil))

/-- The glibc shape around an integer call. -/
def integerCast : Expr :=
  .generic (.leaf 11) (.cons (some 2) (.cast 1 (.paren (.leaf 13))) (.cons none (.leaf 13) .nil))

/-- A selection whose associations are both the glibc selection. -/
def nested : Expr :=
  .generic (.leaf 12) (.cons (some 2) (.cast 1 (.paren glibc)) (.cons none glibc .nil))

/-- A semantics: the controlling expression has a side effect (`99`); a cast
to a pointer label leaves a pointer (`≥ 1000`) unchanged and turns an integer
into a different value. `sel` is the association the compiler selects: `0`
for a `const` argument, `1` otherwise. -/
def semEx (sel : Nat) : Sem where
  leafEval
    | 10 => some ⟨[5], 1000⟩
    | 11 => some ⟨[99], 0⟩
    | 12 => some ⟨[6], 2000⟩
    | 13 => some ⟨[7], 7⟩
    | _ => none
  castValue _ v := if v < 1000 then v + 1000 else v
  combine _ a b := ⟨a.effects ++ b.effects, a.value + b.value⟩
  select _ _ := sel
  isPointer v := decide (1000 ≤ v)

theorem semEx_value_preserving (sel : Nat) : ValuePreserving (semEx sel) ptEx := by
  intro t v _ hp
  simp only [semEx, decide_eq_true_eq] at hp
  simp only [semEx]
  split
  · omega
  · rfl

theorem semEx_pointer_results (sel : Nat) : PointerResults (semEx sel) prEx := by
  intro e o hpr he
  simp only [prEx, Bool.or_eq_true, beq_iff_eq] at hpr
  rcases hpr with rfl | rfl
  · simp only [eval, semEx, Option.some.injEq] at he
    subst he
    simp [semEx]
  · simp only [eval, semEx, Option.some.injEq] at he
    subst he
    simp [semEx]

/-- The glibc shape is admitted and resolves to the call. -/
theorem the_glibc_shape_resolves_to_the_call : resolve ptEx prEx glibc = .leaf 10 := by
  decide

/-- Its `const` selection (the cast) and its plain selection (the call)
evaluate identically, and as the call. -/
theorem both_glibc_selections_are_the_call :
    eval (semEx 0) glibc = some ⟨[5], 1000⟩ ∧ eval (semEx 1) glibc = some ⟨[5], 1000⟩ ∧
      eval (semEx 0) (.leaf 10) = some ⟨[5], 1000⟩ := by
  decide

/-- **The controlling expression's side effect never appears**, whichever
association is selected. -/
theorem a_controls_effect_never_appears :
    (eval (semEx 11) (.leaf 11)).map Outcome.effects = some [99] ∧
      (eval (semEx 0) glibc).map Outcome.effects = some [5] ∧
      (eval (semEx 1) glibc).map Outcome.effects = some [5] := by
  decide

/-- A nested selection resolves, innermost first, to the call. -/
theorem a_nested_selection_resolves_to_the_call : resolve ptEx prEx nested = .leaf 10 := by
  decide

/-- The differing selection is refused, and it has to be: its two selections
have different outcomes, so no single lowering is right for both. -/
theorem a_non_invariant_selection_is_refused_and_must_be :
    resolve ptEx prEx differing = differing ∧
      eval (semEx 0) differing ≠ eval (semEx 1) differing := by
  decide

/-- The integer-cast selection is refused, and it has to be: the cast changes
the integer's value, so the pointer-result premise is load-bearing. -/
theorem an_integer_cast_is_refused_and_must_be :
    resolve ptEx prEx integerCast = integerCast ∧
      eval (semEx 0) integerCast ≠ eval (semEx 1) integerCast := by
  decide

/-- The two premises hold of the example semantics, so
`resolve_preserves_outcomes` applies to it and is not vacuous. -/
theorem the_example_satisfies_the_premises (sel : Nat) :
    ValuePreserving (semEx sel) ptEx ∧ PointerResults (semEx sel) prEx :=
  ⟨semEx_value_preserving sel, semEx_pointer_results sel⟩

end QCore.GenericSelection

