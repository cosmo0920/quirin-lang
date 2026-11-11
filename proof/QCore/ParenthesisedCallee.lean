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
# A callee that is not spelled `NAME (` (card W45-PARENTHESISED-CALLEE)

The abstract rule implemented by `legacy_c::parenthesised_callee`
(`normalise_unit`, `opaque_call`, `Parser::refuse_opaque_call`,
`Parser::classify_for_clause`) in `crates/qcore-c-gateway`, gateway
constraint 10c.

In C, `(f)(x)` calls `f`: a parenthesised primary expression designates what
its operand designates (C11 6.5.1p5), so the parentheses only suppress a
function-like macro. Before the card every call reader of the gateway keyed on
the pair `NAME (`, and a statement whose callee was spelled any other way fell
through to the classifier's final `NoOp`.

The model:

* `Callee` is a callee expression: a name, a parenthesised callee, or any
  other expression (a conditional, a cast, an address, a call's result).
* `designates` is what C calls: a name designates itself and parentheses
  designate their operand; any other expression designates nothing the
  projection can name.
* `norm` is rule 1, the rewrite `(`ᵏ `NAME` `)`ᵏ ↦ `NAME`.
* `lower` is the classifier after the card: a call through a callee that
  normalises to a name is that name's call; any other call is refused
  (rule 2); a statement with no call is `noOp`. `lowerBefore` is the
  classifier before the card, for the non-vacuity statements.
* `guard` is rule 2's hook over an arbitrary classification, and `forClause`
  is rule 3's.

## What this is not

A model of the **decision**, as the gateway's other Lean modules are. Faithful
token extraction — that the lexical reader recognises exactly the
parenthesised-name callees and the other callee expressions of the C it reads,
and never mistakes a cast, a control header, an argument list or a
declarator for one — and the correspondence with the compiled Rust are review
premises (trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/parenthesised_callee.rs` and the unit tests of
`legacy_parenthesised_callee.rs`.
-/

namespace QCore.ParenthesisedCallee

/-- A callee expression. -/
inductive Callee where
  | name (f : Nat)
  | paren (c : Callee)
  | other
  deriving DecidableEq, Repr

/-- What C calls through a callee expression, as far as a name can say. -/
def designates : Callee → Option Nat
  | .name f => some f
  | .paren c => designates c
  | .other => none

/-- Rule 1: parentheses around a callee are removed. -/
def norm : Callee → Callee
  | .name f => .name f
  | .paren c => norm c
  | .other => .other

/-- `k` pairs of parentheses around a callee. -/
def parens : Nat → Callee → Callee
  | 0, c => c
  | k + 1, c => .paren (parens k c)

/-- An expression statement, as the classifier sees it. -/
inductive Stmt where
  /-- A call through `callee` with actuals `args`. -/
  | call (callee : Callee) (args : List Nat)
  /-- A statement the classifier positively recognised as call-free. -/
  | callFree
  deriving DecidableEq, Repr

/-- A statement's lowering. -/
inductive Lowering where
  | contractCall (f : Nat) (args : List Nat)
  | noOp
  | refused
  deriving DecidableEq, Repr

/-- The classifier after the card. -/
def lower : Stmt → Lowering
  | .call c args =>
      match norm c with
      | .name f => .contractCall f args
      | _ => .refused
  | .callFree => .noOp

/-- The classifier before the card: only a bare name was a call, and every
other statement reached the final `NoOp`. -/
def lowerBefore : Stmt → Lowering
  | .call (.name f) args => .contractCall f args
  | _ => .noOp

/-! ## Rule 1 preserves what C calls -/

theorem norm_designates (c : Callee) : designates (norm c) = designates c := by
  induction c with
  | name f => rfl
  | paren c ih => simpa [norm, designates] using ih
  | other => rfl

theorem norm_parens (k : Nat) (c : Callee) : norm (parens k c) = norm c := by
  induction k with
  | zero => rfl
  | succ k ih => simpa [parens, norm] using ih

theorem norm_idempotent (c : Callee) : norm (norm c) = norm c := by
  induction c with
  | name f => rfl
  | paren c ih => simpa [norm] using ih
  | other => rfl

/-- `norm` yields a name exactly when C's callee designates that name. -/
theorem norm_is_name_iff (c : Callee) (f : Nat) :
    norm c = .name f ↔ designates c = some f := by
  induction c with
  | name g =>
      simp [norm, designates]
  | paren c ih => simpa [norm, designates] using ih
  | other => simp [norm, designates]

/-! ## `(f)(a)` and `f(a)` have the same effect -/

theorem a_parenthesised_name_is_the_name (k f : Nat) (args : List Nat) :
    lower (.call (parens k (.name f)) args) = lower (.call (.name f) args) := by
  simp [lower, norm_parens]

theorem parentheses_never_change_a_lowering (c : Callee) (args : List Nat) :
    lower (.call (.paren c) args) = lower (.call c args) := by
  simp [lower, norm]

/-- An admitted call is a call of exactly the function C calls. -/
theorem an_admitted_call_is_the_designated_call (c : Callee) (args : List Nat)
    (g : Nat) (actuals : List Nat)
    (h : lower (.call c args) = .contractCall g actuals) :
    designates c = some g ∧ actuals = args := by
  simp only [lower] at h
  cases hn : norm c with
  | name g' =>
      rw [hn] at h
      injection h with hg ha
      subst hg
      exact ⟨(norm_is_name_iff c g').mp hn, ha.symm⟩
  | paren c' => rw [hn] at h; cases h
  | other => rw [hn] at h; cases h

/-! ## Every call is a classified effect or a refusal, never a no-op -/

theorem a_call_is_never_a_no_op (c : Callee) (args : List Nat) :
    lower (.call c args) ≠ .noOp := by
  simp only [lower]
  split <;> simp

/-- The invariant, stated over every statement: a lowering is `noOp` only for
a statement the classifier recognised as call-free. -/
theorem a_no_op_is_call_free (s : Stmt) (h : lower s = .noOp) : s = .callFree := by
  cases s with
  | call c args => exact absurd h (a_call_is_never_a_no_op c args)
  | callFree => rfl

/-- A call through a callee that designates no name is refused. -/
theorem an_opaque_callee_is_refused (c : Callee) (args : List Nat)
    (h : designates c = none) : lower (.call c args) = .refused := by
  simp only [lower]
  cases hn : norm c with
  | name f =>
      have := (norm_is_name_iff c f).mp hn
      rw [h] at this
      cases this
  | paren c' => rfl
  | other => rfl

/-- On a bare name the card decides exactly as before. -/
theorem a_bare_name_is_unchanged (f : Nat) (args : List Nat) :
    lower (.call (.name f) args) = lowerBefore (.call (.name f) args) := rfl

/-! ## Non-vacuity: the hole the card closes existed -/

theorem the_parenthesised_call_was_a_no_op (f : Nat) (args : List Nat) :
    lowerBefore (.call (.paren (.name f)) args) = .noOp := rfl

theorem the_parenthesised_call_is_now_the_call (f : Nat) (args : List Nat) :
    lower (.call (.paren (.name f)) args) = .contractCall f args := rfl

theorem an_opaque_call_was_a_no_op (args : List Nat) :
    lowerBefore (.call .other args) = .noOp := rfl

/-! ## Rules 2 and 3 only refuse -/

/-- Rule 2's hook over any classification: an admission of a statement that
applies a call to a callee that is not a name is replaced by the refusal; a
refusal is kept as it is. -/
def guard (blocked : Bool) (base : Lowering) : Lowering :=
  if blocked && base != .refused then .refused else base

theorem guard_never_admits_a_refusal (blocked : Bool) :
    guard blocked .refused = .refused := by
  cases blocked <;> rfl

theorem guard_admits_only_what_the_base_admits (blocked : Bool) (base l : Lowering)
    (h : guard blocked base = l) (hl : l ≠ .refused) : l = base := by
  unfold guard at h
  split at h
  · exact absurd h.symm hl
  · exact h.symm

theorem guard_refuses_an_opaque_admission (base : Lowering) :
    guard true base = .refused := by
  unfold guard
  cases base <;> rfl

theorem guard_is_the_identity_without_an_opaque_call (base : Lowering) :
    guard false base = base := by
  unfold guard
  simp

/-- Rule 3: a `for` clause whose stripped spelling is not faithful to its
original (a bracket, or a group that wraps more than one name, or is an
argument list, a callee or a cast) is refused unless its stripped
classification already was. `grouped` is that unfaithfulness, decided
lexically (`stripping_is_faithful`). -/
def forClause (grouped : Bool) (stripped : Lowering) : Lowering :=
  guard grouped stripped

theorem a_grouped_for_clause_is_never_admitted (stripped : Lowering) :
    forClause true stripped = .refused :=
  guard_refuses_an_opaque_admission stripped

theorem an_ungrouped_for_clause_is_unchanged (stripped : Lowering) :
    forClause false stripped = stripped :=
  guard_is_the_identity_without_an_opaque_call stripped

/-! ## The verdict: a body with a refused statement is never `Safe` -/

inductive Verdict where
  | safe
  | unknown
  deriving DecidableEq, Repr

/-- The part of the verdict this card touches: a refused node keeps the body
out of `Safe` (the `Unsupported` diagnostic `classify_diagnostics` reads). -/
def verdict (body : List Lowering) : Verdict :=
  if body.contains .refused then .unknown else .safe

theorem an_opaque_call_keeps_the_body_out_of_safe (body : List Lowering)
    (args : List Nat) (c : Callee) (h : designates c = none) :
    verdict (body ++ [lower (.call c args)]) = .unknown := by
  rw [an_opaque_callee_is_refused c args h]
  simp [verdict]

/-- The measured counterexample: before the card, a parenthesised second
release lowered to nothing, so a body whose other statements were admitted was
`Safe`; after it the second release is the release. -/
theorem the_double_release_is_no_longer_hidden (f p : Nat) :
    lowerBefore (.call (.paren (.name f)) [p]) = .noOp ∧
      lower (.call (.paren (.name f)) [p]) = lower (.call (.name f) [p]) :=
  ⟨rfl, rfl⟩

end QCore.ParenthesisedCallee
