import QCore.ConditionCallDesugar

/-! # The `__builtin_expect` wrapper in a controlling expression (card BUILTIN-EXPECT-NULL-TEST)

`likely(e)` and `unlikely(e)` expand, in the unit's own preprocessed text, to
`__builtin_expect((e), 1)` and `__builtin_expect((e), 0)`. GCC and Clang
document the builtin's value as its first argument; the second is a hint to
the optimiser and changes no value. As a call, C evaluates both arguments, in
an unspecified order (C11 6.5.2.2p10).

Rule 5h's Boolean reading (`legacy_condition_desugar.rs`, `parse`) reads the
call as its operand when the hint is one token of decimal digits, and refuses it
otherwise. This module states that reading over the Boolean trees of
`QCore.ConditionCallDesugar` and proves:

* the wrapper's value is its operand's value, whatever the hint
  (`the_wrapper_has_the_operand_value`);
* a hint with no evaluation event makes the wrapped and the bare expression
  evaluate identically, for every leaf assignment and either argument order
  (`the_reading_is_c_evaluation`), so the desugared plan of the reading is C's
  evaluation of the wrapped expression (`the_desugared_reading_is_c_evaluation`)
  and every null or comparison leaf refines exactly as its bare test does
  (`a_leaf_decides_as_its_bare_test`);
* a hint with an evaluation event refuses the whole expression
  (`an_effectful_hint_refuses`, `a_nested_effectful_hint_refuses`), and so
  does a refused operand (`a_refused_operand_refuses`);
* the hint spellings the Rust admits are exactly the digit-sequence
  literals, every one effect-free (`admitted_spellings_are_literals`,
  `vectors_match`); a suffixed or prefixed integer literal (`0L`, `0x0`)
  is effect-free too but is not admitted, a refusal on the safe side.

The C reader, the token-level hint classification and the Rust-to-Lean
correspondence remain review premises (standing constraint 28); the Rust
fixtures of `crates/qcore-c-gateway/tests/builtin_expect_null_test.rs`
exercise them on the bounded shapes.
-/

namespace QCore.BuiltinExpectCondition

open ConditionCallDesugar

/-- A controlling expression that may carry the wrapper. `expect e hint` is
`__builtin_expect(e, hint)`, with `hint` the evaluation events of the hint
argument (empty for an integer literal). -/
inductive Wrapped (α : Type) where
  | leaf : Nat → List α → Wrapped α
  | and : Wrapped α → Wrapped α → Wrapped α
  | or : Wrapped α → Wrapped α → Wrapped α
  | not : Wrapped α → Wrapped α
  | choose : Wrapped α → Wrapped α → Wrapped α → Wrapped α
  | expect : Wrapped α → List α → Wrapped α
  deriving Repr

/-- C's evaluation of a wrapped expression for the leaf values `v`. The
connectives short-circuit as in `ConditionCallDesugar.eval`. The wrapper
evaluates both arguments, the hint first when `hintFirst` (the order is
unspecified), and has its operand's value. -/
def evalW {α : Type} (v : Nat → Bool) (hintFirst : Bool) : Wrapped α → List α × Bool
  | .leaf i calls => (calls, v i)
  | .and a b =>
      let (ta, ra) := evalW v hintFirst a
      if ra then
        let (tb, rb) := evalW v hintFirst b
        (ta ++ tb, rb)
      else (ta, false)
  | .or a b =>
      let (ta, ra) := evalW v hintFirst a
      if ra then (ta, true)
      else
        let (tb, rb) := evalW v hintFirst b
        (ta ++ tb, rb)
  | .not a =>
      let (ta, ra) := evalW v hintFirst a
      (ta, !ra)
  | .choose c a b =>
      let (tc, rc) := evalW v hintFirst c
      if rc then
        let (t, r) := evalW v hintFirst a
        (tc ++ t, r)
      else
        let (t, r) := evalW v hintFirst b
        (tc ++ t, r)
  | .expect e hint =>
      let (t, r) := evalW v hintFirst e
      (if hintFirst then hint ++ t else t ++ hint, r)

/-- The reading: a wrapper whose hint has no evaluation event is its operand;
any other wrapper refuses (`none`), and a refusal anywhere refuses the whole
expression. -/
def read {α : Type} : Wrapped α → Option (Cond α)
  | .leaf i calls => some (.leaf i calls)
  | .and a b => do
      let a ← read a
      let b ← read b
      pure (.and a b)
  | .or a b => do
      let a ← read a
      let b ← read b
      pure (.or a b)
  | .not a => do
      let a ← read a
      pure (.not a)
  | .choose c a b => do
      let c ← read c
      let a ← read a
      let b ← read b
      pure (.choose c a b)
  | .expect e [] => read e
  | .expect _ (_ :: _) => none

/-- The wrapper has its operand's value, whatever the hint and the order. -/
theorem the_wrapper_has_the_operand_value {α : Type} (v : Nat → Bool) (hintFirst : Bool)
    (e : Wrapped α) (hint : List α) :
    (evalW v hintFirst (.expect e hint)).2 = (evalW v hintFirst e).2 := by
  simp only [evalW]

/-- **The reading is C's evaluation.** When the reading admits an expression,
the bare tree it yields performs exactly the events C performs for the wrapped
expression, in C's order, and has its value, for every leaf assignment and
either argument order of every wrapper. -/
theorem the_reading_is_c_evaluation {α : Type} (v : Nat → Bool) (hintFirst : Bool) :
    ∀ (w : Wrapped α) (c : Cond α), read w = some c → eval v c = evalW v hintFirst w := by
  intro w
  induction w with
  | leaf i calls =>
      intro c h
      simp only [read, Option.some.injEq] at h
      subst h
      simp [eval, evalW]
  | and a b iha ihb =>
      intro c h
      simp only [read, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
        Option.some.injEq] at h
      obtain ⟨ca, ha, cb, hb, rfl⟩ := h
      simp only [eval, evalW, iha ca ha, ihb cb hb]
  | or a b iha ihb =>
      intro c h
      simp only [read, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
        Option.some.injEq] at h
      obtain ⟨ca, ha, cb, hb, rfl⟩ := h
      simp only [eval, evalW, iha ca ha, ihb cb hb]
  | not a iha =>
      intro c h
      simp only [read, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
        Option.some.injEq] at h
      obtain ⟨ca, ha, rfl⟩ := h
      simp only [eval, evalW, iha ca ha]
  | choose k a b ihk iha ihb =>
      intro c h
      simp only [read, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
        Option.some.injEq] at h
      obtain ⟨ck, hk, ca, ha, cb, hb, rfl⟩ := h
      simp only [eval, evalW, ihk ck hk, iha ca ha, ihb cb hb]
  | expect e hint ihe =>
      intro c h
      cases hint with
      | nil =>
          simp only [read] at h
          rw [ihe c h]
          simp only [evalW, List.nil_append, List.append_nil]
          cases hintFirst <;> rfl
      | cons x rest => simp [read] at h

/-- The desugared plan of an admitted reading applies exactly the events C
evaluates for the wrapped expression and leaves by the exit its value selects
(composition with `the_desugaring_is_c_evaluation`). -/
theorem the_desugared_reading_is_c_evaluation {α : Type} (v : Nat → Bool) (hintFirst : Bool)
    (w : Wrapped α) (c : Cond α) (h : read w = some c) :
    run v (desugar c) = evalW v hintFirst w := by
  rw [the_desugaring_is_c_evaluation, the_reading_is_c_evaluation v hintFirst w c h]

/-- A null or comparison test under the wrapper decides through the same
leaf, with the same events and the same exit, as the bare test: the plan of
`__builtin_expect(t, K)` is the plan of `t`. -/
theorem a_leaf_decides_as_its_bare_test {α : Type} (i : Nat) (calls : List α) :
    read (Wrapped.expect (.leaf i calls) ([] : List α)) = some (.leaf i calls) ∧
      read (Wrapped.not (.expect (.leaf i calls) ([] : List α))) = some (.not (.leaf i calls)) := by
  simp [read]

/-- The argument order of an admitted wrapper is unobservable. -/
theorem the_argument_order_is_unobservable {α : Type} (v : Nat → Bool) (w : Wrapped α)
    (c : Cond α) (h : read w = some c) :
    evalW v true w = evalW v false w := by
  rw [← the_reading_is_c_evaluation v true w c h, ← the_reading_is_c_evaluation v false w c h]

/-- A hint with an evaluation event refuses the wrapper. -/
theorem an_effectful_hint_refuses {α : Type} (e : Wrapped α) (x : α) (rest : List α) :
    read (.expect e (x :: rest)) = none := by
  simp [read]

/-- A refused operand refuses the wrapper, whatever its hint. -/
theorem a_refused_operand_refuses {α : Type} (e : Wrapped α) (hint : List α)
    (h : read e = none) : read (.expect e hint) = none := by
  cases hint <;> simp [read, h]

/-- One refusing wrapper anywhere under a connective refuses the expression. -/
theorem a_nested_effectful_hint_refuses {α : Type} (a e : Wrapped α) (x : α) (rest : List α) :
    read (.and a (.expect e (x :: rest))) = none ∧
      read (.or (.expect e (x :: rest)) a) = none := by
  exact ⟨by simp [read], by simp [read]⟩

/-- Without the hint's event an effectful wrapper would drop it: the wrapped
expression performs the hint's event, the bare operand does not. This is why
the reading refuses it. -/
theorem an_effectful_hint_is_observable {α : Type} (v : Nat → Bool) (i : Nat) (x : α) :
    (evalW v false (Wrapped.expect (.leaf i []) [x])).1 = [x] ∧
      (eval v (Cond.leaf i ([] : List α))).1 = [] := by
  simp [evalW, eval]

/-! ## The hint spellings the Rust admits

`parse` in `legacy_condition_desugar.rs` admits a hint that is, after its
enclosing parentheses, exactly one token of ASCII digits; a literal has no
evaluation event (C11 6.5.1, 6.4.4.1). The Rust fixture
`the_hint_spellings_bind_the_admission_vector` binds `vectors` through the
reader itself. -/

/-- A hint's spelling, as the token classifier sees it. -/
inductive HintSpelling where
  /-- one token of decimal digits, optionally parenthesised (`0`, `(1)`) -/
  | digits
  /-- one integer-literal token with a hex/octal prefix or a `u`/`l` suffix
  (`0L`, `1UL`, `0x0`) -/
  | suffixedOrPrefixed
  /-- an identifier, a member or element read -/
  | name
  /-- a call, an assignment, an increment or decrement -/
  | effectful
  /-- anything else (a cast, an operator expression, a floating literal) -/
  | other
  deriving DecidableEq, Repr

def admitted : HintSpelling → Bool
  | .digits => true
  | _ => false

/-- The evaluation events of a spelling's hint, as far as the reading may
assume: none for a literal, possibly some for anything else. -/
def mayHaveEvents : HintSpelling → Bool
  | .digits => false
  | .suffixedOrPrefixed => false
  | _ => true

theorem admitted_spellings_are_literals (s : HintSpelling) (h : admitted s = true) :
    mayHaveEvents s = false := by
  cases s <;> simp_all [admitted, mayHaveEvents]

/-- The admission vector the Rust fixture binds, in constructor order. -/
def vectors : List Bool := [true, false, false, false, false]

theorem vectors_match :
    [HintSpelling.digits, .suffixedOrPrefixed, .name, .effectful, .other].map admitted =
      vectors := by
  decide

end QCore.BuiltinExpectCondition
