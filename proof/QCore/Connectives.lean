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

import QCore.BranchJoin

/-! The logical connectives `||` and `&&` (card `OR-AND-LOWERING`).

This module is about **one** claim: a connective in an `if` condition is a
*desugaring* onto the branch construct card `IF-STATEMENT` landed, and it adds
no semantics of its own. Everything here is therefore stated against
`QCore.BranchJoin`'s own `Op`, `Seq`, `validate` and interpreter, so the
theorems are about the model the branch card proved rather than about a
paraphrase of it.

The three families:

* **Identity** — `lower` produces exactly the nested branch a reader would have
  written by hand, with `handOr` / `handAnd` defined *independently* of
  `lower` so the theorem has content. This is the model counterpart of the
  card's load-bearing measurement, that `a || b` emits byte-identical C to its
  hand-lowered twin.
* **Correctness** — the arm the nested branch actually reaches is the arm the
  ordinary two-valued truth table selects, proved against `BranchJoin`'s
  interpreter by induction over the tree.
* **Neutrality and fail-closed** — a connective-free condition lowers to the
  branch it already was, and a custody disagreement the branch card refuses is
  still refused through any number of connectives.

**Short-circuit is structural here, and that is stated rather than smoothed
over.** The right operand is the condition of a branch nested *inside an arm* of
the left operand's branch, so it is reached on exactly one path
(`the_right_operand_of_an_or_lives_in_the_else_arm` and its and-twin). It is not
*observable*, because every leaf of the tier is a pure literal comparison —
the constructors of `BranchJoin.Condition` — so a strict reading would compute the
same answer. What would make the difference observable is a leaf that could do
something, which is follow-on `IF-RUNTIME-CONDITION`.

**Not proved:** that the Rust parser builds these trees from real source the way
the model says (a tested Rust boundary, per the `PipeDesugaring` / `BranchJoin`
precedent); the token retable itself, which is
`QCore.PipeDesugaring`'s §6 and §7; and anything about bitwise `|`, `&` or `^`,
which are reservations rather than operations. -/
namespace QCore.Connectives

open QCore.BranchJoin

/-! ## 1. The condition tree -/

/-- A condition after card `OR-AND-LOWERING`: a tree of the decidable leaves
card `IF-STATEMENT` shipped, joined by the two connectives.

The connectives exist **only** in this front-end tree. `BranchJoin.Condition` is
byte-unchanged: the lowering eliminates every connective before any checked
model sees one, which is why the Rust card touches no file in `qcore-core`. -/
inductive Tree where
  | leaf (condition : Condition)
  | and (left right : Tree)
  | or (left right : Tree)
  deriving DecidableEq, Repr

/-- The ordinary two-valued reading, used **only** as the oracle the lowering is
proved to agree with. It is deliberately strict: proving that the short-circuit
nesting computes it is the content of `the_lowering_is_the_truth_table`. -/
def Tree.value : Tree → Bool
  | .leaf condition => condition.evaluate
  | .and left right => left.value && right.value
  | .or left right => left.value || right.value

/-- The number of leaves — one branch node per leaf in the lowering. -/
def Tree.leaves : Tree → Nat
  | .leaf _ => 1
  | .and left right => left.leaves + right.leaves
  | .or left right => left.leaves + right.leaves

/-- A one-operation sequence: the shape a nested branch node takes as an arm,
matching the Rust, where each connective level produces a `SequenceLowering`
holding exactly one `SourceOp::Branch`. -/
def only (op : Op) : Seq := .cons op .nil

/-! ## 2. The lowering -/

/-- Turn a condition tree into the nested branch, over the two arms `thenArm`
and `elseArm`.

```text
lower (A || B) T E   =   if (A) { T } else { if (B) { T } else { E } }
lower (A && B) T E   =   if (A) { if (B) { T } else { E } } else { E }
```

The arms are the *same* two arms at every level: the Rust lowers each arm once
and duplicates its operation vectors, so a connective contributes no statement
of its own. -/
def lower : Tree → Seq → Seq → Op
  | .leaf condition, thenArm, elseArm => .branch condition thenArm elseArm
  | .and left right, thenArm, elseArm =>
      lower left (only (lower right thenArm elseArm)) elseArm
  | .or left right, thenArm, elseArm =>
      lower left thenArm (only (lower right thenArm elseArm))

/-! ## 3. Identity: the lowering is the hand-written branch

`handOr` and `handAnd` are written **without** mentioning `lower`, so the
theorems below compare two independently given terms rather than restating one
definition. -/

/-- What a reader writes by hand for `if (A || B) T else E`: an `else if`
chain, which is what `source.branch.v1` already flattens to a right-nested
branch. -/
def handOr (left right : Condition) (thenArm elseArm : Seq) : Op :=
  .branch left thenArm (.cons (.branch right thenArm elseArm) .nil)

/-- What a reader writes by hand for `if (A && B) T else E`: a branch nested in
the *then* arm, with the else arm duplicated. -/
def handAnd (left right : Condition) (thenArm elseArm : Seq) : Op :=
  .branch left (.cons (.branch right thenArm elseArm) .nil) elseArm

theorem or_lowers_to_its_hand_written_chain
    (left right : Condition) (thenArm elseArm : Seq) :
    lower (.or (.leaf left) (.leaf right)) thenArm elseArm
      = handOr left right thenArm elseArm := rfl

theorem and_lowers_to_its_hand_written_nesting
    (left right : Condition) (thenArm elseArm : Seq) :
    lower (.and (.leaf left) (.leaf right)) thenArm elseArm
      = handAnd left right thenArm elseArm := rfl

/-- The two nest the *other* way from each other: or duplicates the then arm,
and duplicates the else arm. Stated together so neither can drift into the
other. -/
theorem the_two_connectives_nest_oppositely
    (left right : Condition) (thenArm elseArm : Seq) :
    lower (.or (.leaf left) (.leaf right)) thenArm elseArm
        = .branch left thenArm (only (.branch right thenArm elseArm)) ∧
    lower (.and (.leaf left) (.leaf right)) thenArm elseArm
        = .branch left (only (.branch right thenArm elseArm)) elseArm :=
  ⟨rfl, rfl⟩

/-- **Short-circuit, structurally.** The right operand of an `||` is the
condition of a branch that sits inside the **else** arm of the left operand's
branch, so it is reached only where the left operand did not decide the answer.
-/
theorem the_right_operand_of_an_or_lives_in_the_else_arm
    (left right : Tree) (thenArm elseArm : Seq) :
    lower (.or left right) thenArm elseArm
      = lower left thenArm (only (lower right thenArm elseArm)) := rfl

/-- …and the right operand of an `&&` sits inside the **then** arm. -/
theorem the_right_operand_of_an_and_lives_in_the_then_arm
    (left right : Tree) (thenArm elseArm : Seq) :
    lower (.and left right) thenArm elseArm
      = lower left (only (lower right thenArm elseArm)) elseArm := rfl

/-! ## 4. Neutrality: a connective-free condition is unmoved -/

/-- The pre-card behaviour, exactly: a single leaf lowers to the branch card
`IF-STATEMENT` already produced. This is the model counterpart of the
byte-identical-C pin for programs that use no connective. -/
theorem a_leaf_lowers_to_the_branch_it_already_was
    (condition : Condition) (thenArm elseArm : Seq) :
    lower (.leaf condition) thenArm elseArm = .branch condition thenArm elseArm :=
  rfl

/-- Whether a tree uses a connective at all. -/
def Tree.connectiveFree : Tree → Bool
  | .leaf _ => true
  | .and _ _ => false
  | .or _ _ => false

/-- …and the claim is not vacuous: a connective-free tree is exactly a leaf, and
a tree with a connective is not one. -/
theorem a_connective_free_tree_is_a_leaf (tree : Tree) :
    tree.connectiveFree = true ↔ ∃ condition, tree = .leaf condition := by
  cases tree with
  | leaf condition => exact ⟨fun _ => ⟨condition, rfl⟩, fun _ => rfl⟩
  | and left right => exact ⟨fun h => by simp [Tree.connectiveFree] at h, fun h => by
      obtain ⟨_, h⟩ := h; exact absurd h (by simp)⟩
  | or left right => exact ⟨fun h => by simp [Tree.connectiveFree] at h, fun h => by
      obtain ⟨_, h⟩ := h; exact absurd h (by simp)⟩

/-! ## 5. Correctness: the nesting computes the truth table

Proved against `BranchJoin`'s own interpreter, so this is a statement about the
semantics the branch card shipped rather than about a fresh evaluator. -/

/-- A one-operation sequence observes what its single operation observes. -/
theorem only_observes_its_operation (machine : Machine) (op : Op) :
    interpSeq machine (only op) = interpOp machine op := by
  unfold only
  rw [interpSeq]
  cases h : interpOp machine op with
  | mk next observation =>
      cases observation with
      | none => simp [interpSeq]
      | some value => simp

/-- **The load-bearing correctness theorem.** However the connectives nest, the
lowered branch observes what the arm the ordinary truth table selects observes.
Short-circuit evaluation and strict evaluation agree — which is exactly the
claim a desugaring has to make. -/
theorem the_lowering_is_the_truth_table
    (tree : Tree) (machine : Machine) (thenArm elseArm : Seq) :
    interpOp machine (lower tree thenArm elseArm)
      = interpSeq machine (if tree.value then thenArm else elseArm) := by
  induction tree generalizing machine thenArm elseArm with
  | leaf condition =>
      cases h : condition.evaluate <;> simp [lower, interpOp, Tree.value, h]
  | and left right ihLeft ihRight =>
      rw [lower, ihLeft]
      cases hLeft : left.value with
      | false => simp [Tree.value, hLeft]
      | true =>
          simp only [Tree.value, hLeft, Bool.true_and, ite_true, only_observes_its_operation,
            ihRight]
  | or left right ihLeft ihRight =>
      rw [lower, ihLeft]
      cases hLeft : left.value with
      | true => simp [Tree.value, hLeft]
      | false => simp [Tree.value, hLeft, only_observes_its_operation, ihRight]

theorem an_or_is_disjunction (left right : Tree) :
    (Tree.or left right).value = (left.value || right.value) := rfl

theorem an_and_is_conjunction (left right : Tree) :
    (Tree.and left right).value = (left.value && right.value) := rfl

/-! ## 6. Precedence, and the associativity finding -/

/-- One token of a condition. A leaf carries its decided value, which is all the
precedence model needs — the leaves' internal shape is `BranchJoin`'s. -/
inductive Tok where
  | leaf (condition : Condition)
  | orTok
  | andTok
  | lparen
  | rparen
  deriving DecidableEq, Repr

/-! The declared grammar, as recursive descent with an explicit fuel so the
functions are total:

```ebnf
cond     = disjunct, { "||", disjunct } ;      (* left-associative *)
disjunct = primary,  { "&&", primary  } ;      (* binds tighter    *)
primary  = "(", cond, ")" | leaf ;
```
-/
mutual

def disjunction : Nat → List Tok → Option (Tree × List Tok)
  | 0, _ => none
  | fuel + 1, tokens => do
      let (left, rest) ← conjunction fuel tokens
      orTail fuel left rest

def orTail : Nat → Tree → List Tok → Option (Tree × List Tok)
  | 0, _, _ => none
  | fuel + 1, left, tokens =>
      match tokens with
      | .orTok :: rest => do
          let (right, after) ← conjunction fuel rest
          orTail fuel (.or left right) after
      | _ => some (left, tokens)

def conjunction : Nat → List Tok → Option (Tree × List Tok)
  | 0, _ => none
  | fuel + 1, tokens => do
      let (left, rest) ← primary fuel tokens
      andTail fuel left rest

def andTail : Nat → Tree → List Tok → Option (Tree × List Tok)
  | 0, _, _ => none
  | fuel + 1, left, tokens =>
      match tokens with
      | .andTok :: rest => do
          let (right, after) ← primary fuel rest
          andTail fuel (.and left right) after
      | _ => some (left, tokens)

def primary : Nat → List Tok → Option (Tree × List Tok)
  | 0, _ => none
  | fuel + 1, tokens =>
      match tokens with
      | .lparen :: rest => do
          let (inner, after) ← disjunction fuel rest
          match after with
          | .rparen :: tail => some (inner, tail)
          | _ => none
      | .leaf condition :: rest => some (.leaf condition, rest)
      | _ => none

end

/-- Parse a whole condition: every token must be consumed. -/
def parse (tokens : List Tok) : Option Tree :=
  match disjunction (tokens.length + 2) tokens with
  | some (tree, []) => some tree
  | _ => none

private def a : Condition := .lit true
private def b : Condition := .lit false
private def c : Condition := .cmp 3 .lt 5

/-- **The precedence decision, pinned:** `&&` binds tighter than `||`, so
`A || B && C` is `A || (B && C)` and never `(A || B) && C`. -/
theorem and_binds_tighter_than_or :
    parse [.leaf a, .orTok, .leaf b, .andTok, .leaf c]
      = some (.or (.leaf a) (.and (.leaf b) (.leaf c))) := by
  decide

/-- …and the other grouping is a different tree, so the pin is not vacuous. -/
theorem the_two_groupings_are_different_trees :
    (Tree.or (.leaf a) (.and (.leaf b) (.leaf c)))
      ≠ (Tree.and (.or (.leaf a) (.leaf b)) (.leaf c)) := by
  intro h
  exact Tree.noConfusion h

/-- Parentheses override the precedence. -/
theorem parentheses_group :
    parse [.lparen, .leaf a, .orTok, .leaf b, .rparen, .andTok, .leaf c]
      = some (.and (.or (.leaf a) (.leaf b)) (.leaf c)) := by
  decide

/-- Both connectives are left-associative. -/
theorem the_connectives_are_left_associative :
    parse [.leaf a, .orTok, .leaf b, .orTok, .leaf c]
      = some (.or (.or (.leaf a) (.leaf b)) (.leaf c)) ∧
    parse [.leaf a, .andTok, .leaf b, .andTok, .leaf c]
      = some (.and (.and (.leaf a) (.leaf b)) (.leaf c)) := by
  constructor <;> decide

/-- **A finding, measured in the Rust and explained here.** For a *homogeneous*
chain the associativity declaration is unobservable: the lowering makes each
connective associative on the nose, so the two groupings are the same nested
branch. The declaration is therefore load-bearing only where the two connectives
mix, which `and_binds_tighter_than_or` is what pins. -/
theorem or_lowers_associatively (left middle right : Tree) (thenArm elseArm : Seq) :
    lower (.or (.or left middle) right) thenArm elseArm
      = lower (.or left (.or middle right)) thenArm elseArm := rfl

theorem and_lowers_associatively (left middle right : Tree) (thenArm elseArm : Seq) :
    lower (.and (.and left middle) right) thenArm elseArm
      = lower (.and left (.and middle right)) thenArm elseArm := rfl

/-- The trees themselves are *not* equal, so the theorems above are statements
about the lowering rather than about the parser. -/
theorem the_groupings_are_distinct_trees_that_lower_alike :
    (Tree.or (.or (.leaf a) (.leaf b)) (.leaf c))
      ≠ (Tree.or (.leaf a) (.or (.leaf b) (.leaf c))) := by
  intro h
  exact Tree.noConfusion h (fun h _ => Tree.noConfusion h)

/-! ## 7. Fail-closed: a connective cannot launder a custody disagreement

Every level of the nesting is an ordinary `BranchJoin` branch, so every level
runs that card's four-model join. The programs below are card `IF-STATEMENT`'s
own refused shapes with a connective in the condition. -/

/-- `let a = acquire; if (true || false) { drop a; } return;` — the conditional
leak, through a connective. -/
def orConditionalLeak : Seq :=
  .cons (.acquire 1 true)
    (.cons (lower (.or (.leaf (.lit true)) (.leaf (.lit false)))
              (.cons (.drop 1) .nil) .nil)
      (.cons (.ret 0) .nil))

/-- The same with both arms releasing: the correct spelling. -/
def orBothArmsRelease : Seq :=
  .cons (.acquire 1 true)
    (.cons (lower (.or (.leaf (.lit true)) (.leaf (.lit false)))
              (.cons (.drop 1) .nil) (.cons (.drop 1) .nil))
      (.cons (.ret 0) .nil))

/-- The and-connective's own conditional leak. -/
def andConditionalLeak : Seq :=
  .cons (.acquire 1 true)
    (.cons (lower (.and (.leaf (.lit true)) (.leaf (.lit false)))
              (.cons (.drop 1) .nil) .nil)
      (.cons (.ret 0) .nil))

/-- One arm moves what the other releases, through a connective: the live lists
agree and the states do not, so comparing live resources alone would admit it. -/
def orMoveVersusRelease : Seq :=
  .cons (.acquire 1 true)
    (.cons (lower (.or (.leaf (.lit true)) (.leaf (.lit false)))
              (.cons (.drop 1) .nil) (.cons (.transfer 1) .nil))
      (.cons (.ret 0) .nil))

theorem an_or_conditional_leak_refuses :
    validate orConditionalLeak = .error .armsDisagree := rfl

theorem an_and_conditional_leak_refuses :
    validate andConditionalLeak = .error .armsDisagree := rfl

theorem moving_in_one_arm_and_releasing_in_the_other_refuses_through_a_connective :
    validate orMoveVersusRelease = .error .armsDisagree := rfl

/-- The positive control: agreeing arms are admitted through a connective, so
the refusals above are about the disagreement rather than about the
connective. -/
theorem agreeing_arms_are_admitted_through_a_connective :
    Admits orBothArmsRelease := by decide

/-- Collected as one list, so a future widening must change a theorem rather
than slip past one. -/
theorem refused_connective_shapes_are_exactly_these :
    [validate orConditionalLeak, validate andConditionalLeak,
     validate orMoveVersusRelease]
      = [.error .armsDisagree, .error .armsDisagree, .error .armsDisagree] := rfl

/-- …and the whole point of the desugaring, stated once: the connective program
and the branch a reader would have hand-written are **the same term**, so every
verdict the validator gives one it gives the other by definition rather than by
agreement. -/
theorem the_connective_program_is_its_hand_written_twin :
    orConditionalLeak
      = .cons (.acquire 1 true)
          (.cons (handOr (.lit true) (.lit false) (.cons (.drop 1) .nil) .nil)
            (.cons (.ret 0) .nil)) := rfl

end QCore.Connectives
