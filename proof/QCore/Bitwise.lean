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

import QCore.Connectives

/-! The integer tier, unary negation and the constant bindings of
`source.branch.v1` (card `BITWISE-CONDITIONS`).

This module unifies the three follow-ons `OR-AND-LOWERING` and `IF-STATEMENT`
left behind — `BITWISE-OPERATORS`, `IF-CONNECTIVE-NEGATION` and
`IF-CONDITION-BINDINGS` — and, like `QCore.Connectives`, it adds **no**
semantics to the branch construct. Everything is stated against
`QCore.BranchJoin`'s own `Op`, `Seq` and interpreter, so the theorems are about
the model card `IF-STATEMENT` proved rather than a paraphrase of it.

Four families:

* **The operators.** `|`, `^` and `&` on a 32-bit pattern, characterized
  bit-by-bit against Lean's own `BitVec` operations rather than against a fresh
  definition, plus the totality that let this family be admitted where a shift
  could not: `|`, `^` and `&` are defined in C for *every* pair of operands,
  and a shift is not.
* **The negation.** `!` is eliminated by an **arm swap**, so `!A` is the branch
  a reader would have written with the arms exchanged, `!!A` is literally `A`'s
  own lowering, and no checked model ever sees a negation.
* **The precedence, and the two refusals.** A recursive-descent model with
  explicit fuel over a **two-sorted** grammar. The card's headline decision is
  proved here as two refusals rather than as a ranking: a precedence is declared
  only where it agrees with C, and where this tier's sorts would force a
  different ranking than C's — an ungrouped bitwise comparison operand, and an
  ungrouped operand of `!` — the spelling is refused.
* **The constant bindings.** A name bound by `let NAME = INTEGER;` is
  substituted by its value at parse time, so a condition that reads one is the
  condition that writes the literal out. The custody rule is stated at the
  position it protects: a resource name is refused, and no binding of this tier
  can ever name an identity.

**Not proved:** that the Rust parser builds these trees from real source the way
the model says (a tested Rust boundary, per the `PipeDesugaring` / `BranchJoin` /
`Connectives` precedent); that a C compiler computes `|`, `^` and `&` as
`BitVec` does (that is what the sanitized native run cross-checks); and anything
about a *run-time* condition, which stays follow-on `IF-RUNTIME-CONDITION`. -/
namespace QCore.Bitwise

open QCore.BranchJoin

/-! ## 1. The three bitwise operators -/

/-- The closed bitwise vocabulary of `source.branch.v1`'s integer tier.

Closed at **three**, because those are exactly the three single-byte spellings
card `OR-AND-LOWERING` freed when it made both logical connectives
double-character. A shift, an arithmetic operator and a complement are each a
separate decision and each keeps a named refusal. -/
inductive BitwiseOp where
  | or
  | xor
  | and
  deriving DecidableEq, Repr

/-- The operation itself, on the 32-bit pattern an `i32` carries. Defined by
delegation to Lean's own `BitVec` operations, so the truth tables below are
theorems about the standard operations rather than about a private definition
that could quietly disagree with one. -/
def BitwiseOp.apply : BitwiseOp → BitVec 32 → BitVec 32 → BitVec 32
  | .or, left, right => left ||| right
  | .xor, left, right => left ^^^ right
  | .and, left, right => left &&& right

/-- **The truth tables**, bit by bit. Each output bit is the ordinary Boolean
operation on the corresponding input bits — which is what "bitwise or" means,
stated rather than assumed. -/
theorem the_operators_are_their_truth_tables (left right : BitVec 32) (index : Nat) :
    (BitwiseOp.or.apply left right).getLsbD index
        = (left.getLsbD index || right.getLsbD index) ∧
    (BitwiseOp.xor.apply left right).getLsbD index
        = xor (left.getLsbD index) (right.getLsbD index) ∧
    (BitwiseOp.and.apply left right).getLsbD index
        = (left.getLsbD index && right.getLsbD index) := by
  refine ⟨?_, ?_, ?_⟩ <;> simp [BitwiseOp.apply]

/-- The three are **different** operations, so the table above is not vacuous
and no two of the spellings could be conflated. -/
theorem the_three_operators_are_distinct :
    BitwiseOp.or.apply 1 3 ≠ BitwiseOp.xor.apply 1 3 ∧
    BitwiseOp.xor.apply 1 3 ≠ BitwiseOp.and.apply 1 3 ∧
    BitwiseOp.or.apply 1 3 ≠ BitwiseOp.and.apply 1 3 := by
  refine ⟨?_, ?_, ?_⟩ <;> decide

/-- **Totality**, and the reason this family could be admitted at all: every
operand pair has a value, in the model and in C. C leaves `|`, `^` and `&`
defined for all operands and leaves a shift undefined for some, which is why
this card spends three reservations and not five. -/
theorem the_operators_are_total (op : BitwiseOp) (left right : BitVec 32) :
    ∃ value, op.apply left right = value :=
  ⟨op.apply left right, rfl⟩

/-- Each operator is commutative and associative, which is what makes the
declared left-associativity of a homogeneous chain unobservable — the same
finding card `OR-AND-LOWERING` recorded for `||` and `&&`, here for the
integers. The declaration is therefore load-bearing only where the three
operators **mix**. -/
theorem each_operator_is_commutative_and_associative (op : BitwiseOp) (a b c : BitVec 32) :
    op.apply a b = op.apply b a ∧
    op.apply (op.apply a b) c = op.apply a (op.apply b c) := by
  cases op
  · exact ⟨BitVec.or_comm a b, BitVec.or_assoc a b c⟩
  · exact ⟨BitVec.xor_comm a b, BitVec.xor_assoc a b c⟩
  · exact ⟨BitVec.and_comm a b, BitVec.and_assoc a b c⟩

/-- …and the *mixture* is not: `&` and `|` do not commute past each other, so
the ranking below is a real decision. -/
theorem the_operators_do_not_commute_with_each_other :
    BitwiseOp.or.apply (BitwiseOp.and.apply 1 2) 3
      ≠ BitwiseOp.and.apply 1 (BitwiseOp.or.apply 2 3) := by decide

/-! ## 2. The integer tier -/

/-- One comparison operand. Closed over 32-bit literals: a name written in a
condition is substituted for the literal its constant binding carries (§5), so
nothing here can denote a resource identity, a call result or any run-time
value. Card `IF-STATEMENT`'s single-valued observation therefore still holds. -/
inductive IntExpr where
  | lit (value : BitVec 32)
  | bitwise (op : BitwiseOp) (left right : IntExpr)
  deriving DecidableEq, Repr

/-- Total and state-free: every operand of the tier has exactly one value, and
computing it reads nothing. -/
def IntExpr.evaluate : IntExpr → BitVec 32
  | .lit value => value
  | .bitwise op left right => op.apply left.evaluate right.evaluate

/-- The shape card `IF-STATEMENT` shipped. -/
def IntExpr.isLiteral : IntExpr → Bool
  | .lit _ => true
  | .bitwise _ _ _ => false

/- Totality and absence of state inputs are properties of the definitions
above. Neither existentially naming `evaluate`'s result nor equating that result
with itself supplies independent evidence. The fold/value correspondence below
is the proof obligation connecting this tier to the branch model. -/

/-- A condition of the widened tier: the two leaves card `IF-STATEMENT` shipped,
with each comparison operand widened from a literal to the integer tier. -/
inductive Cond where
  | lit (value : Bool)
  | cmp (left : IntExpr) (op : CompareOp) (right : IntExpr)
  deriving DecidableEq, Repr

/-- The comparison is on the **signed** reading of the pattern, which is the
`i32` the source writes. -/
def Cond.evaluate : Cond → Bool
  | .lit value => value
  | .cmp left op right => op.apply left.evaluate.toInt right.evaluate.toInt

/-! ### 2.1 Neutrality: the shipped tier is unmoved

The bridge back to `BranchJoin.Condition`, so "a program that writes no bit
operation is unmoved" is a theorem rather than a promise. -/

/-- **The fold.** The `BranchJoin` condition a widened condition decides to, by
evaluating each operand. The Rust deliberately does *not* fold — it carries the
operand tree into `qcore-core` so the emitted C computes the bit operation
itself, which is what keeps the reference / CFG / native agreement a cross-check
rather than a tautology — but folding is **value-preserving**, which is what
lets every theorem below be stated against `BranchJoin`'s own condition type.
-/
def Cond.folded : Cond → Condition
  | .lit value => .lit value
  | .cmp left op right => .cmp left.evaluate.toInt op right.evaluate.toInt

theorem the_fold_preserves_the_value (condition : Cond) :
    condition.evaluate = condition.folded.evaluate := by
  cases condition <;> rfl

/-- The `BranchJoin` condition a widened condition *is*, when it happens to be
one of the shapes card `IF-STATEMENT` shipped. -/
def Cond.shipped : Cond → Option Condition
  | .lit value => some (.lit value)
  | .cmp (.lit left) op (.lit right) => some (.cmp left.toInt op right.toInt)
  | .cmp _ _ _ => none

/-- **Neutrality.** Where a widened condition has a shipped spelling, the two
evaluate identically: this card moves no verdict for a program that writes no
bit operation. -/
theorem the_shipped_tier_evaluates_unchanged (condition : Cond) (shipped : Condition) :
    condition.shipped = some shipped → condition.evaluate = shipped.evaluate := by
  cases condition with
  | lit value => intro h; cases h; rfl
  | cmp left op right =>
      cases left with
      | lit leftValue =>
          cases right with
          | lit rightValue => intro h; cases h; rfl
          | bitwise _ _ _ => intro h; exact absurd h (by simp [Cond.shipped])
      | bitwise _ _ _ => intro h; exact absurd h (by simp [Cond.shipped])

/-- …and the claim is not vacuous: a bitwise operand has **no** shipped
spelling, which is exactly what makes it new. -/
theorem a_bitwise_operand_has_no_shipped_spelling
    (op : BitwiseOp) (left right : IntExpr) (relation : CompareOp) (other : IntExpr) :
    (Cond.cmp (.bitwise op left right) relation other).shipped = none := rfl

/-- A worked instance of both halves at once: the masked comparison this card
exists to make writable, beside the literal comparison that is unmoved. -/
theorem the_masked_comparison_is_admitted_and_true :
    (Cond.cmp (.bitwise .or (.lit 3) (.lit 4)) .eq (.lit 7)).evaluate = true ∧
    (Cond.cmp (.lit 3) .lt (.lit 5)).shipped = some (.cmp 3 .lt 5) := by
  refine ⟨?_, ?_⟩ <;> decide

/-! ## 3. Negation, as the arm swap -/

/-- A condition tree after this card: card `OR-AND-LOWERING`'s tree with one
constructor added. The leaves are `BranchJoin`'s own conditions, so every
theorem below composes directly with that module's validator and interpreter;
the integer tier of §2 is what a front end resolves *into* such a leaf. -/
inductive Tree where
  | leaf (condition : Cond)
  | not (inner : Tree)
  | and (left right : Tree)
  | or (left right : Tree)
  deriving DecidableEq, Repr

/-- The ordinary reading, used only as the oracle the lowering is proved to
agree with. -/
def Tree.value : Tree → Bool
  | .leaf condition => condition.evaluate
  | .not inner => !inner.value
  | .and left right => left.value && right.value
  | .or left right => left.value || right.value

/-- Whether a tree uses the new constructor at all. -/
def Tree.negationFree : Tree → Bool
  | .leaf _ => true
  | .not _ => false
  | .and left right => left.negationFree && right.negationFree
  | .or left right => left.negationFree && right.negationFree

/-- The lowering. Only one line is new, and it is the whole of the negation
card: **`!` exchanges the arms.**

```text
lower (!A)     T E   =   lower A E T
lower (A || B) T E   =   if (A) { T } else { if (B) { T } else { E } }
lower (A && B) T E   =   if (A) { if (B) { T } else { E } } else { E }
```
-/
def lower : Tree → Seq → Seq → Op
  | .leaf condition, thenArm, elseArm => .branch condition.folded thenArm elseArm
  | .not inner, thenArm, elseArm => lower inner elseArm thenArm
  | .and left right, thenArm, elseArm =>
      lower left (Connectives.only (lower right thenArm elseArm)) elseArm
  | .or left right, thenArm, elseArm =>
      lower left thenArm (Connectives.only (lower right thenArm elseArm))

/-- **The negation is the arm swap.** Stated on its own because it is the whole
mechanism: no new operation, no new condition constructor, no new rule. -/
theorem the_negation_is_the_arm_swap (inner : Tree) (thenArm elseArm : Seq) :
    lower (.not inner) thenArm elseArm = lower inner elseArm thenArm := rfl

/-- The model counterpart of the card's byte-equality pin: `if (!C) T else E`
lowers to exactly the branch a reader writes as `if (C) E else T`. -/
theorem a_negated_leaf_is_its_hand_swapped_twin
    (condition : Cond) (thenArm elseArm : Seq) :
    lower (.not (.leaf condition)) thenArm elseArm
      = .branch condition.folded elseArm thenArm := rfl

/-- `!!A` is **literally** `A`'s own lowering — the same term, so every verdict
is shared by definition rather than by agreement. -/
theorem a_double_negation_is_the_identity_lowering (inner : Tree) (thenArm elseArm : Seq) :
    lower (.not (.not inner)) thenArm elseArm = lower inner thenArm elseArm := rfl

/-- Negation is an involution on values too, and De Morgan holds — so the arm
swap is the *same* rewrite a De Morgan pass would have performed, and this card
did not have to choose between them. -/
theorem the_negation_obeys_de_morgan (left right : Tree) :
    (Tree.not (.and left right)).value = (Tree.or (.not left) (.not right)).value ∧
    (Tree.not (.or left right)).value = (Tree.and (.not left) (.not right)).value := by
  refine ⟨?_, ?_⟩ <;> simp [Tree.value]

/-- **Neutrality against the connective card.** On a negation-free tree this
lowering is `QCore.Connectives.lower`, operation for operation: the card adds a
constructor and moves nothing. -/
def Tree.toConnectives : Tree → Option Connectives.Tree
  | .leaf condition => some (.leaf condition.folded)
  | .not _ => none
  | .and left right =>
      match left.toConnectives, right.toConnectives with
      | some left, some right => some (.and left right)
      | _, _ => none
  | .or left right =>
      match left.toConnectives, right.toConnectives with
      | some left, some right => some (.or left right)
      | _, _ => none

theorem a_negation_free_tree_lowers_exactly_as_before
    (tree : Tree) (previous : Connectives.Tree) (thenArm elseArm : Seq) :
    tree.toConnectives = some previous →
      lower tree thenArm elseArm = Connectives.lower previous thenArm elseArm := by
  induction tree generalizing previous thenArm elseArm with
  | leaf condition => intro h; cases h; rfl
  | not inner => intro h; exact absurd h (by simp [Tree.toConnectives])
  | and left right ihLeft ihRight =>
      intro h
      rw [Tree.toConnectives] at h
      cases hLeft : left.toConnectives with
      | none => rw [hLeft] at h; simp at h
      | some leftTree =>
          cases hRight : right.toConnectives with
          | none => rw [hLeft, hRight] at h; simp at h
          | some rightTree =>
              rw [hLeft, hRight] at h
              simp only [Option.some.injEq] at h
              subst h
              rw [lower, Connectives.lower, ihLeft _ _ _ hLeft, ihRight _ _ _ hRight]
  | or left right ihLeft ihRight =>
      intro h
      rw [Tree.toConnectives] at h
      cases hLeft : left.toConnectives with
      | none => rw [hLeft] at h; simp at h
      | some leftTree =>
          cases hRight : right.toConnectives with
          | none => rw [hLeft, hRight] at h; simp at h
          | some rightTree =>
              rw [hLeft, hRight] at h
              simp only [Option.some.injEq] at h
              subst h
              rw [lower, Connectives.lower, ihLeft _ _ _ hLeft, ihRight _ _ _ hRight]

/-- …and the claim is not vacuous: a negation-free tree is exactly one with a
previous spelling. -/
theorem a_tree_has_a_previous_spelling_exactly_when_it_is_negation_free (tree : Tree) :
    tree.negationFree = true ↔ ∃ previous, tree.toConnectives = some previous := by
  induction tree with
  | leaf condition => exact ⟨fun _ => ⟨.leaf condition.folded, rfl⟩, fun _ => rfl⟩
  | not inner =>
      exact ⟨fun h => by simp [Tree.negationFree] at h,
        fun h => by obtain ⟨_, h⟩ := h; simp [Tree.toConnectives] at h⟩
  | and left right ihLeft ihRight =>
      constructor
      · intro h
        simp only [Tree.negationFree, Bool.and_eq_true] at h
        obtain ⟨leftTree, hLeft⟩ := ihLeft.mp h.1
        obtain ⟨rightTree, hRight⟩ := ihRight.mp h.2
        exact ⟨.and leftTree rightTree, by simp [Tree.toConnectives, hLeft, hRight]⟩
      · intro h
        obtain ⟨_, h⟩ := h
        rw [Tree.toConnectives] at h
        cases hLeft : left.toConnectives with
        | none => rw [hLeft] at h; simp at h
        | some leftTree =>
            cases hRight : right.toConnectives with
            | none => rw [hLeft, hRight] at h; simp at h
            | some rightTree =>
                simp [Tree.negationFree, ihLeft.mpr ⟨leftTree, hLeft⟩,
                  ihRight.mpr ⟨rightTree, hRight⟩]
  | or left right ihLeft ihRight =>
      constructor
      · intro h
        simp only [Tree.negationFree, Bool.and_eq_true] at h
        obtain ⟨leftTree, hLeft⟩ := ihLeft.mp h.1
        obtain ⟨rightTree, hRight⟩ := ihRight.mp h.2
        exact ⟨.or leftTree rightTree, by simp [Tree.toConnectives, hLeft, hRight]⟩
      · intro h
        obtain ⟨_, h⟩ := h
        rw [Tree.toConnectives] at h
        cases hLeft : left.toConnectives with
        | none => rw [hLeft] at h; simp at h
        | some leftTree =>
            cases hRight : right.toConnectives with
            | none => rw [hLeft, hRight] at h; simp at h
            | some rightTree =>
                simp [Tree.negationFree, ihLeft.mpr ⟨leftTree, hLeft⟩,
                  ihRight.mpr ⟨rightTree, hRight⟩]

/-! ### 3.1 Correctness, against the shipped interpreter -/

/-- **The load-bearing correctness theorem**, extended to the negation: however
the tree nests, the lowered branch observes what the arm the ordinary truth
table selects observes. Proved against `BranchJoin`'s own interpreter, so it is
a statement about the semantics card `IF-STATEMENT` shipped. -/
theorem the_lowering_is_the_truth_table
    (tree : Tree) (machine : Machine) (thenArm elseArm : Seq) :
    interpOp machine (lower tree thenArm elseArm)
      = interpSeq machine (if tree.value then thenArm else elseArm) := by
  induction tree generalizing machine thenArm elseArm with
  | leaf condition =>
      rw [Tree.value, the_fold_preserves_the_value]
      cases h : condition.folded.evaluate <;> simp [lower, interpOp, h]
  | not inner ih =>
      rw [lower, ih]
      cases h : inner.value <;> simp [Tree.value, h]
  | and left right ihLeft ihRight =>
      rw [lower, ihLeft]
      cases hLeft : left.value with
      | false => simp [Tree.value, hLeft]
      | true =>
          simp only [Tree.value, hLeft, Bool.true_and, ite_true,
            Connectives.only_observes_its_operation, ihRight]
  | or left right ihLeft ihRight =>
      rw [lower, ihLeft]
      cases hLeft : left.value with
      | true => simp [Tree.value, hLeft]
      | false =>
          simp [Tree.value, hLeft, Connectives.only_observes_its_operation, ihRight]

/-- The negation observes the **other** arm: the concrete both-polarities pin. -/
theorem a_negation_observes_the_other_arm (machine : Machine) (thenArm elseArm : Seq) :
    interpOp machine (lower (.not (.leaf (.lit true))) thenArm elseArm)
        = interpSeq machine elseArm ∧
    interpOp machine (lower (.not (.leaf (.lit false))) thenArm elseArm)
        = interpSeq machine thenArm := ⟨rfl, rfl⟩

/-! ### 3.2 Fail-closed through a negation

A negation is an arm swap, so a custody disagreement is *the same pair of arms*
in the other order — and the join is symmetric. Nothing can be laundered by
negating a condition, and that is proved rather than argued. -/

/-- Card `BranchJoin`'s own conditional leak, negated: still refused. -/
theorem a_conditional_leak_refuses_through_a_negation :
    validate
        (.cons (.acquire 1 true)
          (.cons (lower (.not (.leaf (.lit true))) (.cons (.drop 1) .nil) .nil)
            (.cons (.ret 0) .nil))) = .error .armsDisagree := rfl

/-- …and the shape where one arm moves what the other releases. -/
theorem moving_in_one_arm_and_releasing_in_the_other_refuses_through_a_negation :
    validate
        (.cons (.acquire 1 true)
          (.cons (lower (.not (.leaf (.lit false))) (.cons (.drop 1) .nil)
            (.cons (.transfer 1) .nil))
            (.cons (.ret 0) .nil))) = .error .armsDisagree := rfl

/-- The positive control: agreeing arms are admitted through a negation, so the
two refusals above are about the disagreement and not about the negation. -/
theorem agreeing_arms_are_admitted_through_a_negation :
    (validate
        (.cons (.acquire 1 true)
          (.cons (lower (.not (.leaf (.lit true))) (.cons (.drop 1) .nil)
            (.cons (.drop 1) .nil))
            (.cons (.ret 0) .nil)))).isOk = true := rfl

/-- A `return` inside an arm reached through a negation is admitted exactly as
one written without it: the negation is eliminated by the arm swap before any
model runs, so the model-level term is a branch with a returning arm, which
card `IF-ARMS` admits (join over survivors). Composition pin-move at
integration: this theorem stated the refusal while the arm subset excluded
`return`; the subset widened, and the same term now validates. -/
theorem a_return_in_an_arm_is_admitted_through_a_negation :
    (validate
        (.cons (lower (.not (.leaf (.lit true))) (.cons (.ret 1) .nil) .nil)
          (.cons (.ret 0) .nil))).isOk = true := rfl

/-- The refusal surface as **one list**, so a future widening has to change a
theorem rather than slip past one. Card `IF-ARMS` did exactly that at
integration: the third shape this list carried (`.ret` in an arm, then
`.returnInBranchArm`) moved to the admission above, so the list holds the two
join disagreements that remain refused. -/
theorem refused_negation_shapes_are_exactly_these :
    validate
        (.cons (.acquire 1 true)
          (.cons (lower (.not (.leaf (.lit true))) (.cons (.drop 1) .nil) .nil)
            (.cons (.ret 0) .nil))) = .error .armsDisagree ∧
    validate
        (.cons (.acquire 1 true)
          (.cons (lower (.not (.leaf (.lit false))) (.cons (.drop 1) .nil)
            (.cons (.transfer 1) .nil))
            (.cons (.ret 0) .nil))) = .error .armsDisagree :=
  ⟨rfl, rfl⟩

/-! ## 4. Precedence, and the two refusals that are the card's decision

The grammar is **two-sorted**: an expression is a condition or an integer, and
one parser reads both. That is what makes a sort error a refusal instead of a
second parse, and it is what forces the two decisions below.

```ebnf
cond  = conj , { "||" , conj } ;              (* 1, left,  as in C *)
conj  = neg  , { "&&" , neg  } ;              (* 2, left,  as in C *)
neg   = "!" , neg | cmp ;                     (* 3, right, NOT as in C *)
cmp   = int , [ op , int ] ;                  (* 4, none,  NOT as in C *)
int   = xor , { "|" , xor } ;                 (* 5, left,  as in C *)
xor   = and , { "^" , and } ;                 (* 6, left,  as in C *)
and   = atom , { "&" , atom } ;                (* 7, left,  as in C *)
atom  = "(" , cond , ")" | BOOL | INT | NAME ;
```

**The rule for declaring a precedence, stated once:** a precedence is declared
only where it agrees with C. C ranks `!` *tighter* and `|`, `^`, `&` *looser*
than the comparisons; a tier in which a comparison yields a condition and a
condition is never an integer operand must rank them the other way. Rather than
silently redefine `flags | MASK == 0`, the ungrouped spelling is **refused**. -/

/-- One token. Leaves carry their decided value, exactly as in
`QCore.Connectives`' precedence model: the leaves' internal shape is §2's. -/
inductive Tok where
  | boolTok (value : Bool)
  | intTok (value : BitVec 32)
  | nameTok (name : Nat)
  | bangTok
  | andTok
  | orTok
  | cmpTok (op : CompareOp)
  | bitTok (op : BitwiseOp)
  | lparen
  | rparen
  deriving DecidableEq, Repr

/-- A parsed node: one of the two sorts. -/
inductive Node where
  | cond (tree : Tree)
  | int (value : IntExpr)
  deriving DecidableEq, Repr

/-- A node together with whether its outermost spelling needs no grouping to be
read the way this tier reads it. This one flag is the whole mechanism behind the
card's headline decision. -/
structure Parsed where
  node : Node
  grouped : Bool
  deriving DecidableEq, Repr

/-- What a name may resolve to. `constants` is the check-time constant binding
environment (§5); `resources` is the *identity* environment, present only so the
custody refusal can be stated at the position it protects. -/
structure Env where
  constants : Nat → Option (BitVec 32)
  resources : Nat → Bool

/-- `|` is level 0 of the bitwise table, `^` level 1, `&` level 2; level 3 is
the atom. -/
def levelOperator : Nat → Option BitwiseOp
  | 0 => some .or
  | 1 => some .xor
  | 2 => some .and
  | _ => none

def asCond : Parsed → Option Tree
  | ⟨.cond tree, _⟩ => some tree
  | ⟨.int _, _⟩ => none

def asInt : Parsed → Option IntExpr
  | ⟨.int value, _⟩ => some value
  | ⟨.cond _, _⟩ => none

/-- A comparison operand: an integer **and** grouped. -/
def asGroupedInt : Parsed → Option IntExpr
  | ⟨.int value, true⟩ => some value
  | _ => none

mutual

def disjunction : Nat → Env → List Tok → Option (Parsed × List Tok)
  | 0, _, _ => none
  | fuel + 1, env, tokens => do
      let (left, rest) ← conjunction fuel env tokens
      orTail fuel env left rest

def orTail : Nat → Env → Parsed → List Tok → Option (Parsed × List Tok)
  | 0, _, _, _ => none
  | fuel + 1, env, left, tokens =>
      match tokens with
      | .orTok :: rest => do
          let leftTree ← asCond left
          let (right, after) ← conjunction fuel env rest
          let rightTree ← asCond right
          orTail fuel env ⟨.cond (.or leftTree rightTree), false⟩ after
      | _ => some (left, tokens)

def conjunction : Nat → Env → List Tok → Option (Parsed × List Tok)
  | 0, _, _ => none
  | fuel + 1, env, tokens => do
      let (left, rest) ← negation fuel env tokens
      andTail fuel env left rest

def andTail : Nat → Env → Parsed → List Tok → Option (Parsed × List Tok)
  | 0, _, _, _ => none
  | fuel + 1, env, left, tokens =>
      match tokens with
      | .andTok :: rest => do
          let leftTree ← asCond left
          let (right, after) ← negation fuel env rest
          let rightTree ← asCond right
          andTail fuel env ⟨.cond (.and leftTree rightTree), false⟩ after
      | _ => some (left, tokens)

/-- Level 3. **Refusal one:** the operand must be grouped, because C binds `!`
tighter than every comparison and this tier must bind it looser. -/
def negation : Nat → Env → List Tok → Option (Parsed × List Tok)
  | 0, _, _ => none
  | fuel + 1, env, tokens =>
      match tokens with
      | .bangTok :: rest => do
          let (operand, after) ← negation fuel env rest
          if operand.grouped then
            let tree ← asCond operand
            some (⟨.cond (.not tree), true⟩, after)
          else none
      | _ => comparison fuel env tokens

/-- Level 4, **non-associative**: at most one comparison, integer × integer →
condition. **Refusal two:** each operand must be grouped, because C ranks the
three bit operators looser than the comparisons and this tier must rank them
tighter. -/
def comparison : Nat → Env → List Tok → Option (Parsed × List Tok)
  | 0, _, _ => none
  | fuel + 1, env, tokens => do
      let (left, rest) ← bitwiseLevel fuel env 0 tokens
      match rest with
      | .cmpTok op :: after => do
          let leftValue ← asGroupedInt left
          let (right, tail) ← bitwiseLevel fuel env 0 after
          let rightValue ← asGroupedInt right
          some (⟨.cond (.leaf (.cmp leftValue op rightValue)), false⟩, tail)
      | _ => some (left, rest)

/-- Levels 5–7: `|`, then `^`, then `&`, each left-associative, in **C's own
order** — which is why this ranking is declared rather than refused. -/
def bitwiseLevel : Nat → Env → Nat → List Tok → Option (Parsed × List Tok)
  | 0, _, _, _ => none
  | fuel + 1, env, level, tokens =>
      match levelOperator level with
      | none => atom fuel env tokens
      | some op => do
          let (left, rest) ← bitwiseLevel fuel env (level + 1) tokens
          bitwiseTail fuel env level op left rest

def bitwiseTail : Nat → Env → Nat → BitwiseOp → Parsed → List Tok →
    Option (Parsed × List Tok)
  | 0, _, _, _, _, _ => none
  | fuel + 1, env, level, op, left, tokens =>
      match tokens with
      | .bitTok found :: rest =>
          if found = op then do
            let leftValue ← asInt left
            let (right, after) ← bitwiseLevel fuel env (level + 1) rest
            let rightValue ← asInt right
            bitwiseTail fuel env level op ⟨.int (.bitwise op leftValue rightValue), false⟩ after
          else some (left, tokens)
      | _ => some (left, tokens)

def atom : Nat → Env → List Tok → Option (Parsed × List Tok)
  | 0, _, _ => none
  | fuel + 1, env, tokens =>
      match tokens with
      | .lparen :: rest => do
          let (inner, after) ← disjunction fuel env rest
          match after with
          | .rparen :: tail => some (⟨inner.node, true⟩, tail)
          | _ => none
      | .boolTok value :: rest => some (⟨.cond (.leaf (.lit value)), true⟩, rest)
      | .intTok value :: rest => some (⟨.int (.lit value), true⟩, rest)
      | .nameTok name :: rest =>
          -- The custody rule, at the position it protects: a resource identity
          -- is not an integer, and a constant binding is the only name this
          -- tier reads.
          if env.resources name then none
          else
            match env.constants name with
            | some value => some (⟨.int (.lit value), true⟩, rest)
            | none => none
      | _ => none

end

/-- Parse a whole condition: every token consumed, and the result a condition.
An integer left over at the top is refused — this tier has no
integer-to-Boolean coercion, so `if (mask & 4)` is not a program. -/
def parse (env : Env) (tokens : List Tok) : Option Tree :=
  match disjunction (9 * tokens.length + 16) env tokens with
  | some (⟨.cond tree, _⟩, []) => some tree
  | _ => none

/-- The empty environment: no constant binding and no resource. -/
def Env.empty : Env := { constants := fun _ => none, resources := fun _ => false }

/-- One constant binding, `let n = value;`. -/
def Env.bind (name : Nat) (value : BitVec 32) : Env :=
  { constants := fun candidate => if candidate = name then some value else none
    resources := fun _ => false }

/-- One resource binding, `let r = qcore_acquire(…);`. -/
def Env.resource (name : Nat) : Env :=
  { constants := fun _ => none, resources := fun candidate => candidate = name }

/-! ### 4.1 The declared rankings, where they agree with C -/

/-- **`&` binds tighter than `^` binds tighter than `|`** — C's own order, which
is why it is declared rather than refused.

Pinned against the fully parenthesized intended grouping (equal) and against the
other grouping (different), so it cannot pass vacuously. The witness is chosen so
the two groupings genuinely differ: `4 | (1 & 1)` is `5` and `(4 | 1) & 1` is
`1`. -/
theorem the_bitwise_ranking_is_cs_own :
    parse Env.empty
        [.lparen, .intTok 4, .bitTok .or, .intTok 1, .bitTok .and, .intTok 1, .rparen,
          .cmpTok .eq, .intTok 5]
      = parse Env.empty
        [.lparen, .intTok 4, .bitTok .or, .lparen, .intTok 1, .bitTok .and, .intTok 1, .rparen,
          .rparen, .cmpTok .eq, .intTok 5] ∧
    parse Env.empty
        [.lparen, .intTok 4, .bitTok .or, .intTok 1, .bitTok .and, .intTok 1, .rparen,
          .cmpTok .eq, .intTok 5]
      ≠ parse Env.empty
        [.lparen, .lparen, .intTok 4, .bitTok .or, .intTok 1, .rparen, .bitTok .and,
          .intTok 1, .rparen, .cmpTok .eq, .intTok 5] := by
  refine ⟨?_, ?_⟩ <;> decide

/-- `^` sits between them, so the three-level table is not a two-level one:
`1 ^ 6 & 3` is `1 ^ 2 = 3`, and `(1 ^ 6) & 3` is `7 & 3 = 3`… so the *values*
agree there, and the pin below uses the pair that separates them, `4 | 1 ^ 5`:
`4 | (1 ^ 5)` is `4 | 4 = 4`, and `(4 | 1) ^ 5` is `5 ^ 5 = 0`. -/
theorem the_xor_level_sits_between_the_other_two :
    parse Env.empty
        [.lparen, .intTok 4, .bitTok .or, .intTok 1, .bitTok .xor, .intTok 5, .rparen,
          .cmpTok .eq, .intTok 4]
      = parse Env.empty
        [.lparen, .intTok 4, .bitTok .or, .lparen, .intTok 1, .bitTok .xor, .intTok 5, .rparen,
          .rparen, .cmpTok .eq, .intTok 4] ∧
    parse Env.empty
        [.lparen, .intTok 4, .bitTok .or, .intTok 1, .bitTok .xor, .intTok 5, .rparen,
          .cmpTok .eq, .intTok 4]
      ≠ parse Env.empty
        [.lparen, .lparen, .intTok 4, .bitTok .or, .intTok 1, .rparen, .bitTok .xor,
          .intTok 5, .rparen, .cmpTok .eq, .intTok 4] := by
  refine ⟨?_, ?_⟩ <;> decide

/-- **The associativity finding, carried over to the integers.** Each bitwise
operator is declared left-associative, and for a *homogeneous* chain that
declaration is unobservable — the two groupings parse to different trees that
carry the same value, because §1 proved each operator associative. Recorded
rather than claimed, exactly as card `OR-AND-LOWERING` recorded it for `||` and
`&&`: the declaration is load-bearing only where the three operators mix, which
is what the two theorems above pin. -/
theorem a_homogeneous_bitwise_chain_carries_the_same_value_under_either_grouping
    (a b c : BitVec 32) (op : BitwiseOp) :
    (IntExpr.bitwise op (.bitwise op (.lit a) (.lit b)) (.lit c)).evaluate
      = (IntExpr.bitwise op (.lit a) (.bitwise op (.lit b) (.lit c))).evaluate :=
  (each_operator_is_commutative_and_associative op a b c).2

/-- …and the two groupings are nonetheless **distinct trees**, so the statement
above is about the operators rather than about the parser. -/
theorem the_two_groupings_are_distinct_trees :
    IntExpr.bitwise .or (.bitwise .or (.lit 1) (.lit 2)) (.lit 4)
      ≠ IntExpr.bitwise .or (.lit 1) (.bitwise .or (.lit 2) (.lit 4)) := by decide

/-- The left-associative spelling is the one the parser builds. -/
theorem the_bitwise_operators_are_left_associative :
    parse Env.empty
        [.lparen, .intTok 1, .bitTok .or, .intTok 2, .bitTok .or, .intTok 4, .rparen,
          .cmpTok .eq, .intTok 7]
      = some (.leaf (.cmp (.bitwise .or (.bitwise .or (.lit 1) (.lit 2)) (.lit 4))
          .eq (.lit 7))) := by decide

/-- `!` binds tighter than `&&`, which binds tighter than `||` — again C's own
order. -/
theorem the_negation_binds_tighter_than_the_connectives :
    parse Env.empty [.bangTok, .boolTok true, .andTok, .boolTok false]
      = parse Env.empty
        [.lparen, .bangTok, .boolTok true, .rparen, .andTok, .boolTok false] ∧
    parse Env.empty [.bangTok, .boolTok true, .andTok, .boolTok false]
      ≠ parse Env.empty
        [.bangTok, .lparen, .boolTok true, .andTok, .boolTok false, .rparen] := by
  refine ⟨?_, ?_⟩ <;> decide

/-- Card `OR-AND-LOWERING`'s ranking is unmoved by the two new levels. -/
theorem and_still_binds_tighter_than_or :
    parse Env.empty [.boolTok true, .orTok, .boolTok false, .andTok, .boolTok true]
      = parse Env.empty
        [.boolTok true, .orTok, .lparen, .boolTok false, .andTok, .boolTok true, .rparen] ∧
    parse Env.empty [.boolTok true, .orTok, .boolTok false, .andTok, .boolTok true]
      ≠ parse Env.empty
        [.lparen, .boolTok true, .orTok, .boolTok false, .rparen, .andTok, .boolTok true] := by
  refine ⟨?_, ?_⟩ <;> decide

/-! ### 4.2 The two refusals that are the decision -/

/-- **Refusal one.** An ungrouped bitwise comparison operand is refused, and the
grouped spelling is admitted: `flags | MASK == 0` would mean one thing to a C
reader and another here, so it is not a program. -/
theorem an_ungrouped_bitwise_comparison_operand_is_refused :
    parse Env.empty [.intTok 3, .bitTok .or, .intTok 4, .cmpTok .eq, .intTok 7] = none ∧
    parse Env.empty
        [.lparen, .intTok 3, .bitTok .or, .intTok 4, .rparen, .cmpTok .eq, .intTok 7]
      = some (.leaf (.cmp (.bitwise .or (.lit 3) (.lit 4)) .eq (.lit 7))) := by
  refine ⟨?_, ?_⟩ <;> decide

/-- **Refusal two.** An ungrouped operand of `!` is refused, and the grouped
spelling is admitted: C reads `!a < b` as `(!a) < b`, which this tier cannot
mean. -/
theorem an_ungrouped_negation_operand_is_refused :
    parse Env.empty [.bangTok, .intTok 3, .cmpTok .lt, .intTok 5] = none ∧
    parse Env.empty [.bangTok, .lparen, .intTok 3, .cmpTok .lt, .intTok 5, .rparen]
      = some (.not (.leaf (.cmp (.lit 3) .lt (.lit 5)))) := by
  refine ⟨?_, ?_⟩ <;> decide

/-- The sort discipline, both directions: an integer is not a condition, and a
condition is not an integer operand. `if (mask & 4)` and `true | false` are both
refused, and neither refusal is about the operator. -/
theorem the_two_sorts_do_not_coerce :
    parse Env.empty [.intTok 4] = none ∧
    parse Env.empty [.lparen, .intTok 1, .bitTok .and, .intTok 4, .rparen] = none ∧
    parse Env.empty [.boolTok true, .bitTok .or, .boolTok false] = none := by
  refine ⟨?_, ?_, ?_⟩ <;> decide

/-- A comparison is **non-associative**: `1 < 2 < 3` is not a condition. -/
theorem a_comparison_chain_is_refused :
    parse Env.empty [.intTok 1, .cmpTok .lt, .intTok 2, .cmpTok .lt, .intTok 3] = none := by
  decide

/-- `!!A` parses, and is the double negation §3 proved to be `A`'s own
lowering. -/
theorem a_double_negation_parses :
    parse Env.empty [.bangTok, .bangTok, .boolTok true]
      = some (.not (.not (.leaf (.lit true)))) := by decide

/-- The refusal surface of the parser as **one list**, so a widening must change
a theorem rather than slip past one. -/
theorem refused_condition_shapes_are_exactly_these :
    parse Env.empty [.intTok 3, .bitTok .or, .intTok 4, .cmpTok .eq, .intTok 7] = none ∧
    parse Env.empty [.bangTok, .intTok 3, .cmpTok .lt, .intTok 5] = none ∧
    parse Env.empty [.intTok 4] = none ∧
    parse Env.empty [.boolTok true, .bitTok .or, .boolTok false] = none ∧
    parse Env.empty [.intTok 1, .cmpTok .lt, .intTok 2, .cmpTok .lt, .intTok 3] = none ∧
    parse Env.empty [.nameTok 7, .cmpTok .eq, .intTok 3] = none ∧
    parse (Env.resource 7) [.nameTok 7, .cmpTok .eq, .intTok 3] = none := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> decide

/-! ## 5. Check-time constant bindings

`let NAME = INTEGER;` is the decidable subset of follow-on
`IF-CONDITION-BINDINGS`. The binding is **substituted** at the atom level, so it
contributes no operation and the tier stays state-free — which is exactly why
card `IF-STATEMENT`'s single-observation certificate is untouched. -/

/-- A bound name parses as its value: the condition that reads a binding **is**
the condition that writes the literal out. -/
theorem a_bound_name_is_its_value :
    parse (Env.bind 7 3) [.nameTok 7, .cmpTok .eq, .intTok 3]
      = parse Env.empty [.intTok 3, .cmpTok .eq, .intTok 3] := by decide

/-- …and it is a real substitution, not a coincidence of that one example: a
bound name reaches the bitwise tier too. -/
theorem a_bound_name_is_an_integer_operand :
    parse (Env.bind 7 3) [.lparen, .nameTok 7, .bitTok .or, .intTok 4, .rparen,
        .cmpTok .eq, .intTok 7]
      = some (.leaf (.cmp (.bitwise .or (.lit 3) (.lit 4)) .eq (.lit 7))) := by decide

/-- **The custody rule.** A name bound to a resource identity is refused, and it
is refused *differently* from an unbound name only in the message: neither is an
integer. No custody-bearing value gains a position in this tier. -/
theorem a_resource_name_is_not_an_integer :
    parse (Env.resource 7) [.nameTok 7, .cmpTok .eq, .intTok 3] = none ∧
    parse Env.empty [.nameTok 7, .cmpTok .eq, .intTok 3] = none := by
  refine ⟨?_, ?_⟩ <;> decide

/-- …and the refusal is not merely because *some* environment is empty: the same
name bound as a constant is admitted, so the resource refusal is about custody
and nothing else. -/
theorem the_custody_refusal_is_about_custody :
    parse (Env.bind 7 3) [.nameTok 7, .cmpTok .eq, .intTok 3] ≠ none := by decide

/-- A binding contributes **no operation**: the program that reads one and the
program that writes its literal out are the same term, so every verdict is
shared by definition rather than by agreement. -/
theorem a_binding_contributes_no_operation (thenArm elseArm : Seq) :
    (parse (Env.bind 7 3) [.nameTok 7, .cmpTok .eq, .intTok 3]).map
        (fun tree => lower tree thenArm elseArm)
      = (parse Env.empty [.intTok 3, .cmpTok .eq, .intTok 3]).map
        (fun tree => lower tree thenArm elseArm) := by
  rw [a_bound_name_is_its_value]

end QCore.Bitwise
