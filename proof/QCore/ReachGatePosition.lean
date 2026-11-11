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

/-! # The INDIRECT half of the joint expression-position gate (card REACH-GATE)

Card C03-REACH measured that the neutral-frame recursive route's corpus reach is
gated on `POSITION + INDIRECT` **jointly**
(`docs/reports/remaining-c03-reach/README.md` §5). Card W4-EXPR-POS landed the
POSITION half, modelled in `QCore.NeutralExpressionPosition`, whose load-bearing
sentence is

> an admitted neutral expression contains no pointer-valued operand anywhere.

This module models the INDIRECT half,
`crates/qcore-c-gateway/src/neutral_position.rs`'s family **F4**: a bounded
typed access chain, admitted inside the same six expression positions. A member
access on a record pointer *is* a pointer-valued base, so that sentence cannot
survive unchanged — and it is not violated but **refined**:

> An admitted neutral expression binds, releases, escapes and addresses nothing
> the projection tracks. Every pointer it names appears only as the base or an
> intermediate link of a chain whose base this body never binds, so no place
> along that chain is a place the projection binds; and the value such a chain
> produces is either a scalar, or — only where the position discards it into a
> control-flow decision, in an expression carrying no assignment and outside
> every call argument list — one pointer that is stored nowhere, passed to
> nothing and returned to no caller.

The four theorems that carry it are

* `an_admitted_expression_reads_through_no_bound_base` — the projection's own
  tracked `->` places come from card O04-B08's aggregate store, whose base must
  be a body-acquired name and is therefore bound; a never-bound base carries no
  tracked field place at any chain depth;
* `a_returned_expression_names_no_pointer_value` — W4-EXPR-POS's sentence, kept
  verbatim in the `returned` role;
* `an_argument_of_an_admitted_call_names_no_pointer_value` — and kept verbatim
  inside every admitted call's argument list, in either role;
* `an_assignment_refuses_every_pointer_leaf` — so an admitted pointer value can
  never be stored.

Card O04-READ-BOUND's *bound*-base reads stay refused here, and
`a_bound_base_refuses_every_chain` is the theorem: that card's premise is a
flow-sensitive liveness revalidation attached to a modeled call, and an
expression position carries no modeled call to attach it to
(`REACH-GATE-BOUND-BASE`).

## What is a premise and not a theorem

That the Rust reader extracts declared types, record members, the bound-name
over-approximation and the contract environment faithfully from C source is a
premise, exactly as in `QCore.LegacyCAnalyzer`, `QCore.BorrowedActual`,
`QCore.MayAlias` and `QCore.NeutralExpressionPosition`. Nothing here is a claim
about arbitrary C memory safety.

## Modelling choices

Two bounds are carried as fuel, for the same reason as in
`QCore.NeutralExpressionPosition`: the Rust *refuses* past each bound rather
than truncating, and a `Nat`-recursive definition makes every proof finite.
`callNestingLimit` bounds nested admitted calls; `accessChainLimit` bounds the
selections of one chain.
-/

namespace QCore.ReachGatePosition

/-- A declared name: a variable, a record member or a callee. -/
abbrev Name := Nat

/-- A record tag. -/
abbrev Tag := Nat

/-- The custody modes a declared parameter can carry, projected onto the one
distinction this rule makes. -/
inductive ParameterMode
  | borrowed
  | other
  deriving DecidableEq, Repr

/-- The result modes, projected the same way. -/
inductive ResultMode
  | none
  | other
  deriving DecidableEq, Repr

/-- One callee's contract, as the rule reads it. -/
structure Contract where
  parameters : List ParameterMode
  result : ResultMode
  deriving DecidableEq, Repr

/-- `N(m)`: every formal `Borrowed`, result `None`. -/
def neutralContract (arity : Nat) : Contract :=
  { parameters := List.replicate arity ParameterMode.borrowed, result := ResultMode.none }

/-- Whether a contract is the neutral one. -/
def isNeutralContract (c : Contract) : Bool :=
  c.result == ResultMode.none && c.parameters.all (fun m => m == ParameterMode.borrowed)

/-- What the position does with the value the expression produces. -/
inductive Role
  /-- A branch, loop, `for`, `do`/`while` or `switch` condition. -/
  | decides
  /-- A `return` expression. -/
  | returned
  deriving DecidableEq, Repr

/-- A record member's declared class, as far as the typed reader commits to it.

`recordPointer` is the only chainable one: it models a member declared *exactly*
`struct POINTEE *` in a single complete non-union definition, which is what
`TypedAccessModel::record_pointer_field` answers. `pointer` models every other
member holding exactly one pointer value; `unmodelled` models everything the reader
refuses — an aggregate, an array, a multi-star pointer, a bitfield, a member of
a `union`, an incomplete record and a `volatile` or `_Atomic` member. -/
inductive Member
  | scalar
  | recordPointer (pointee : Tag)
  | pointer
  | unmodelled
  deriving Repr

/-- One access chain: a base name and its selections, outermost first. -/
structure Chain where
  base : Name
  fields : List Name
  deriving Repr

/-- One expression, projected onto the classifications the rule distinguishes.

`pointerOperand` stands for every operand the typed reader will not commit to:
a bare pointer name, a cast, an address-of, a dereference and a subscript. They
are one constructor because the rule treats them identically — it refuses. -/
inductive Operand
  | literal
  | enumConstant
  | scalar (n : Name)
  | pointerOperand (n : Name)
  | chain (c : Chain)
  | call (callee : Name) (args : List Operand)
  deriving Repr

/-- The declaration and contract facts one classification reads. -/
structure Env where
  /-- The contract in force for a callee, if any. -/
  contract : Name → Option Contract
  /-- The record tag a base's declared type names, when the reader committed to
  it as exactly `struct TAG *` or `struct TAG`. -/
  baseTag : Name → Option Tag
  /-- A member's class in a tag's single complete non-union definition. -/
  member : Tag → Name → Member
  /-- The bound-name over-approximation: names this body may bind to a resource
  identity. -/
  bound : Name → Bool

/-- The Rust's `CALL_NESTING_LIMIT`. -/
def callNestingLimit : Nat := 4

/-- The Rust's `ACCESS_CHAIN_LIMIT`, the same bound card O04-ARMS set for a
borrowed actual. -/
def accessChainLimit : Nat := 4

/-- Walk a chain's selections, re-entering the record model at each link.

`none` for an empty selection list, for a chain longer than the fuel, and for a
link that is not a fully typed record pointer. -/
def chainLeaf (env : Env) : Nat → Tag → List Name → Option Member
  | _, _, [] => Option.none
  | 0, _, _ => Option.none
  | fuel + 1, tag, f :: rest =>
      match rest with
      | [] => Option.some (env.member tag f)
      | _ :: _ =>
          match env.member tag f with
          | Member.recordPointer pointee => chainLeaf env fuel pointee rest
          | _ => Option.none

/-- The member one whole chain selects, or `none` when the base's declared type
is not a record the reader committed to, when the selection list is empty, when
the chain is longer than [`accessChainLimit`], or when a link is not a fully
typed record pointer. -/
def chainLeafOf (env : Env) (c : Chain) : Option Member :=
  match env.baseTag c.base with
  | Option.none => Option.none
  | Option.some tag => chainLeaf env accessChainLimit tag c.fields

/-- Whether one selected member may be a chain's leaf. A scalar always may; a
member holding one pointer may only where the position discards the value; an
aggregate, an array, a multi-star pointer and everything else the reader refused
never may. -/
def leafAdmitted (pointerLeaf : Bool) : Member → Bool
  | Member.scalar => true
  | Member.recordPointer _ => pointerLeaf
  | Member.pointer => pointerLeaf
  | Member.unmodelled => false

/-- Whether a selected member is a scalar, so the value the chain produces is
not a pointer at all. -/
def leafIsScalar : Member → Bool
  | Member.scalar => true
  | Member.recordPointer _ => false
  | Member.pointer => false
  | Member.unmodelled => false

/-- Whether one chain is admitted. `pointerLeaf` says whether the position
discards the value the chain produces. -/
def admitsChain (env : Env) (pointerLeaf : Bool) (c : Chain) : Bool :=
  !env.bound c.base &&
    (match chainLeafOf env c with
     | Option.none => false
     | Option.some m => leafAdmitted pointerLeaf m)

/-- Whether a chain's leaf is a scalar. -/
def chainLeafIsScalar (env : Env) (c : Chain) : Bool :=
  match chainLeafOf env c with
  | Option.none => false
  | Option.some m => leafIsScalar m

/-- One layer of the decision, with `rec` deciding the arguments. -/
def admitsStep (env : Env) (pointerLeaf : Bool) (rec : Operand → Bool) : Operand → Bool
  | .literal => true
  | .enumConstant => true
  | .scalar _ => true
  | .pointerOperand _ => false
  | .chain c => admitsChain env pointerLeaf c
  | .call callee args =>
      match env.contract callee with
      | Option.none => false
      | Option.some c =>
          isNeutralContract c && c.parameters.length == args.length && args.all rec

/-- The decision at a depth budget. An argument is always decided **without**
the pointer leaf, in either role, so card W4-EXPR-POS's argument sentence
survives this card word for word. -/
def admitsFuel (env : Env) (pointerLeaf : Bool) : Nat → Operand → Bool
  | 0 => admitsStep env pointerLeaf (fun _ => false)
  | n + 1 => admitsStep env pointerLeaf (admitsFuel env false n)

/-- Whether a role lets a chain produce a pointer value. -/
def roleAllowsPointerLeaf : Role → Bool
  | .decides => true
  | .returned => false

/-- The F4 walker as the gateway consults it: the whole run refuses when a
pointer leaf is in play and the expression carries an assignment. -/
def admitsExpression (env : Env) (role : Role) (assigns : Bool) (e : Operand) : Bool :=
  !(roleAllowsPointerLeaf role && assigns) &&
    admitsFuel env (roleAllowsPointerLeaf role) callNestingLimit e

/-- Whether the expression reads through a base this body may bind, at the same
depth budget as the admission it accompanies. -/
def readsBoundBaseFuel (env : Env) : Nat → Operand → Bool
  | 0 => fun _ => false
  | n + 1 => fun e =>
      match e with
      | .chain c => env.bound c.base
      | .call _ args => args.any (readsBoundBaseFuel env n)
      | _ => false

/-- Whether the expression names a bare pointer operand — a cast, an address-of,
a dereference, a subscript or a pointer-typed name — anywhere. -/
def namesBarePointerFuel : Nat → Operand → Bool
  | 0 => fun _ => false
  | n + 1 => fun e =>
      match e with
      | .pointerOperand _ => true
      | .call _ args => args.any (namesBarePointerFuel n)
      | _ => false

/-- Whether the expression produces a pointer **value** anywhere: a bare pointer
operand, or a chain whose leaf is not a scalar. This is the measure card
W4-EXPR-POS's sentence was about; the theorems below keep it false in the
`returned` role and inside every call argument list. -/
def namesPointerValueFuel (env : Env) : Nat → Operand → Bool
  | 0 => fun _ => false
  | n + 1 => fun e =>
      match e with
      | .pointerOperand _ => true
      | .chain c => !chainLeafIsScalar env c
      | .call _ args => args.any (namesPointerValueFuel env n)
      | _ => false

/-- The instruction one classified position compiles to. -/
inductive Instruction
  | noOp
  | unknown
  deriving DecidableEq, Repr

/-- How `legacy_c` compiles a position. `earlier` is the disjunction of the
lexical predicate and card W4-EXPR-POS's F2/F3, both of which are consulted
first and unchanged; modelling it explicitly is what makes this card's
admission additive. -/
def classify (earlier : Bool) (env : Env) (role : Role) (assigns : Bool)
    (e : Operand) : Instruction :=
  if earlier || admitsExpression env role assigns e then Instruction.noOp
  else Instruction.unknown

/-! ## F4: the bounded typed access chain -/

theorem a_scalar_leaf_is_admitted_in_either_role
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (c : Chain)
    (hb : env.bound c.base = false)
    (hl : chainLeafOf env c = Option.some Member.scalar) :
    admitsFuel env pointerLeaf fuel (Operand.chain c) = true := by
  cases fuel <;> simp [admitsFuel, admitsStep, admitsChain, leafAdmitted, hb, hl]

theorem a_pointer_leaf_is_admitted_exactly_where_the_value_is_discarded
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (c : Chain)
    (hb : env.bound c.base = false)
    (hl : chainLeafOf env c = Option.some Member.pointer) :
    admitsFuel env pointerLeaf fuel (Operand.chain c) = pointerLeaf := by
  cases fuel <;> simp [admitsFuel, admitsStep, admitsChain, leafAdmitted, hb, hl]

theorem a_record_pointer_leaf_is_admitted_exactly_where_the_value_is_discarded
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (c : Chain) (pointee : Tag)
    (hb : env.bound c.base = false)
    (hl : chainLeafOf env c = Option.some (Member.recordPointer pointee)) :
    admitsFuel env pointerLeaf fuel (Operand.chain c) = pointerLeaf := by
  cases fuel <;> simp [admitsFuel, admitsStep, admitsChain, leafAdmitted, hb, hl]

theorem a_bound_base_refuses_every_chain
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (c : Chain)
    (hb : env.bound c.base = true) :
    admitsFuel env pointerLeaf fuel (Operand.chain c) = false := by
  cases fuel <;> simp [admitsFuel, admitsStep, admitsChain, hb]

theorem a_base_without_a_declared_record_type_refuses
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (c : Chain)
    (ht : env.baseTag c.base = Option.none) :
    admitsFuel env pointerLeaf fuel (Operand.chain c) = false := by
  cases fuel <;> simp [admitsFuel, admitsStep, admitsChain, chainLeafOf, ht]

theorem an_opaque_leaf_refuses_in_either_role
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (c : Chain)
    (hl : chainLeafOf env c = Option.some Member.unmodelled) :
    admitsFuel env pointerLeaf fuel (Operand.chain c) = false := by
  cases fuel <;> simp [admitsFuel, admitsStep, admitsChain, leafAdmitted, hl]

theorem an_empty_selection_list_refuses
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (base : Name) :
    admitsFuel env pointerLeaf fuel (Operand.chain { base := base, fields := [] }) = false := by
  have h : chainLeafOf env { base := base, fields := [] } = Option.none := by
    unfold chainLeafOf
    cases env.baseTag base <;> simp [chainLeaf]
  cases fuel <;> simp [admitsFuel, admitsStep, admitsChain, h]

theorem a_chain_through_a_non_record_pointer_link_refuses
    (env : Env) (fuel : Nat) (tag : Tag) (f : Name) (rest : List Name)
    (hrest : rest ≠ []) (hm : env.member tag f = Member.scalar) :
    chainLeaf env (fuel + 1) tag (f :: rest) = Option.none := by
  cases rest with
  | nil => exact absurd rfl hrest
  | cons _ _ => simp [chainLeaf, hm]

theorem a_chain_at_the_bound_is_walked
    (env : Env) (tag t1 t2 t3 : Tag) (a b c d : Name)
    (h1 : env.member tag a = Member.recordPointer t1)
    (h2 : env.member t1 b = Member.recordPointer t2)
    (h3 : env.member t2 c = Member.recordPointer t3) :
    chainLeaf env accessChainLimit tag [a, b, c, d] = Option.some (env.member t3 d) := by
  simp [chainLeaf, accessChainLimit, h1, h2, h3]

/-- The bound is a refusal, never a truncation: a chain with more selections
than the fuel yields no leaf at all. -/
theorem a_chain_longer_than_the_bound_refuses :
    ∀ (env : Env) (fuel : Nat) (tag : Tag) (fields : List Name),
      fuel < fields.length → chainLeaf env fuel tag fields = Option.none := by
  intro env fuel
  induction fuel with
  | zero =>
      intro tag fields h
      cases fields with
      | nil => simp at h
      | cons _ _ => rfl
  | succ n ih =>
      intro tag fields h
      cases fields with
      | nil => rfl
      | cons f rest =>
          cases rest with
          | nil => simp at h
          | cons g rest' =>
              have hlen : n < (g :: rest').length := by
                simpa using Nat.lt_of_succ_lt_succ h
              simp only [chainLeaf]
              cases hm : env.member tag f with
              | scalar => rfl
              | pointer => rfl
              | unmodelled => rfl
              | recordPointer p => exact ih p (g :: rest') hlen

theorem a_five_selection_chain_refuses_at_the_bound
    (env : Env) (tag : Tag) (a b c d e : Name) :
    chainLeaf env accessChainLimit tag [a, b, c, d, e] = Option.none :=
  a_chain_longer_than_the_bound_refuses env accessChainLimit tag [a, b, c, d, e]
    (by simp [accessChainLimit])

/-- A chain admitted with the pointer leaf off has a scalar leaf. -/
theorem a_chain_admitted_without_the_pointer_leaf_has_a_scalar_leaf
    (env : Env) (c : Chain) (h : admitsChain env false c = true) :
    chainLeafIsScalar env c = true := by
  unfold admitsChain at h
  unfold chainLeafIsScalar
  revert h
  cases chainLeafOf env c with
  | none => simp
  | some m => cases m <;> simp [leafAdmitted, leafIsScalar]

/-- Switching the pointer leaf on can only add admissions, never withdraw one:
a chain admitted without it is admitted with it. -/
theorem a_chain_admitted_without_the_pointer_leaf_is_admitted_with_it
    (env : Env) (c : Chain) (h : admitsChain env false c = true) :
    admitsChain env true c = true := by
  unfold admitsChain at h ⊢
  revert h
  cases chainLeafOf env c with
  | none => simp
  | some m => cases m <;> simp [leafAdmitted]

/-! ## F1: calls, restated for this card's step -/

theorem an_uncontracted_call_is_never_admitted
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (callee : Name) (args : List Operand)
    (h : env.contract callee = Option.none) :
    admitsFuel env pointerLeaf fuel (Operand.call callee args) = false := by
  cases fuel <;> simp [admitsFuel, admitsStep, h]

theorem a_non_none_result_is_never_admitted
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (callee : Name) (args : List Operand)
    (c : Contract) (hc : env.contract callee = Option.some c)
    (hr : c.result = ResultMode.other) :
    admitsFuel env pointerLeaf fuel (Operand.call callee args) = false := by
  cases fuel <;> simp [admitsFuel, admitsStep, hc, isNeutralContract, hr]

theorem a_non_borrowed_parameter_is_never_admitted
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (callee : Name) (args : List Operand)
    (c : Contract) (hc : env.contract callee = Option.some c)
    (hp : ParameterMode.other ∈ c.parameters) :
    admitsFuel env pointerLeaf fuel (Operand.call callee args) = false := by
  have hall : c.parameters.all (fun m => m == ParameterMode.borrowed) = false := by
    simp
    exact ⟨ParameterMode.other, hp, by decide⟩
  cases fuel <;> simp [admitsFuel, admitsStep, hc, isNeutralContract, hall]

theorem a_mismatched_arity_is_never_admitted
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (callee : Name) (args : List Operand)
    (c : Contract) (hc : env.contract callee = Option.some c)
    (hn : c.parameters.length ≠ args.length) :
    admitsFuel env pointerLeaf fuel (Operand.call callee args) = false := by
  cases fuel <;> simp [admitsFuel, admitsStep, hc, hn]

theorem a_neutral_call_of_admitted_arguments_is_admitted
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (callee : Name) (args : List Operand)
    (hc : env.contract callee = Option.some (neutralContract args.length))
    (ha : args.all (admitsFuel env false fuel) = true) :
    admitsFuel env pointerLeaf (fuel + 1) (Operand.call callee args) = true := by
  simp [admitsFuel, admitsStep, hc, isNeutralContract, neutralContract, ha]

theorem every_argument_of_an_admitted_call_is_admitted_without_the_pointer_leaf
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (callee : Name) (args : List Operand)
    (h : admitsFuel env pointerLeaf (fuel + 1) (Operand.call callee args) = true)
    (a : Operand) (hm : a ∈ args) :
    admitsFuel env false fuel a = true := by
  revert h
  simp only [admitsFuel, admitsStep]
  cases hc : env.contract callee with
  | none => simp
  | some c =>
      intro h
      have := (Bool.and_eq_true _ _).mp h
      exact (List.all_eq_true.mp this.2) a hm

theorem a_call_exhausting_the_budget_refuses_a_nonempty_argument_list
    (env : Env) (pointerLeaf : Bool) (callee : Name) (args : List Operand)
    (h : args ≠ []) :
    admitsFuel env pointerLeaf 0 (Operand.call callee args) = false := by
  cases args with
  | nil => exact absurd rfl h
  | cons a rest =>
      simp only [admitsFuel, admitsStep]
      cases env.contract callee <;> simp

/-! ## The refined soundness sentence -/

theorem an_admitted_expression_reads_through_no_bound_base :
    ∀ (env : Env) (fuel : Nat) (pointerLeaf : Bool) (e : Operand),
      admitsFuel env pointerLeaf fuel e = true → readsBoundBaseFuel env fuel e = false := by
  intro env fuel
  induction fuel with
  | zero => intro _ e _; cases e <;> rfl
  | succ n ih =>
      intro pointerLeaf e h
      cases e with
      | literal => rfl
      | enumConstant => rfl
      | scalar => rfl
      | pointerOperand m =>
          exact absurd h (by simp [admitsFuel, admitsStep])
      | chain c =>
          by_cases hb : env.bound c.base = true
          · exact absurd h (by
              simp [a_bound_base_refuses_every_chain env (n + 1) pointerLeaf c hb])
          · simpa [readsBoundBaseFuel] using Bool.not_eq_true _ ▸ hb
      | call callee args =>
          simp only [readsBoundBaseFuel, List.any_eq_false]
          intro a hm
          have :=
            every_argument_of_an_admitted_call_is_admitted_without_the_pointer_leaf
              env n pointerLeaf callee args h a hm
          simpa using ih false a this

theorem an_admitted_expression_names_no_bare_pointer :
    ∀ (env : Env) (fuel : Nat) (pointerLeaf : Bool) (e : Operand),
      admitsFuel env pointerLeaf fuel e = true → namesBarePointerFuel fuel e = false := by
  intro env fuel
  induction fuel with
  | zero => intro _ e _; cases e <;> rfl
  | succ n ih =>
      intro pointerLeaf e h
      cases e with
      | literal => rfl
      | enumConstant => rfl
      | scalar => rfl
      | pointerOperand m => exact absurd h (by simp [admitsFuel, admitsStep])
      | chain => rfl
      | call callee args =>
          simp only [namesBarePointerFuel, List.any_eq_false]
          intro a hm
          have :=
            every_argument_of_an_admitted_call_is_admitted_without_the_pointer_leaf
              env n pointerLeaf callee args h a hm
          simpa using ih false a this

/-- Card W4-EXPR-POS's sentence, kept verbatim wherever the pointer leaf is off:
an expression admitted without it produces no pointer value anywhere. -/
theorem an_expression_admitted_without_the_pointer_leaf_names_no_pointer_value :
    ∀ (env : Env) (fuel : Nat) (e : Operand),
      admitsFuel env false fuel e = true → namesPointerValueFuel env fuel e = false := by
  intro env fuel
  induction fuel with
  | zero => intro e _; cases e <;> rfl
  | succ n ih =>
      intro e h
      cases e with
      | literal => rfl
      | enumConstant => rfl
      | scalar => rfl
      | pointerOperand m => exact absurd h (by simp [admitsFuel, admitsStep])
      | chain c =>
          have hc : admitsChain env false c = true := by
            cases n <;> simpa [admitsFuel, admitsStep] using h
          have hs := a_chain_admitted_without_the_pointer_leaf_has_a_scalar_leaf env c hc
          simp [namesPointerValueFuel, hs]
      | call callee args =>
          simp only [namesPointerValueFuel, List.any_eq_false]
          intro a hm
          have :=
            every_argument_of_an_admitted_call_is_admitted_without_the_pointer_leaf
              env n false callee args h a hm
          simpa using ih a this

/-- A `return` expression therefore keeps W4-EXPR-POS's sentence exactly. -/
theorem a_returned_expression_names_no_pointer_value
    (env : Env) (assigns : Bool) (e : Operand)
    (h : admitsExpression env Role.returned assigns e = true) :
    namesPointerValueFuel env callNestingLimit e = false := by
  have : admitsFuel env false callNestingLimit e = true := by
    simpa [admitsExpression, roleAllowsPointerLeaf] using h
  exact an_expression_admitted_without_the_pointer_leaf_names_no_pointer_value
    env callNestingLimit e this

/-- And so does every argument of an admitted call, in either role. -/
theorem an_argument_of_an_admitted_call_names_no_pointer_value
    (env : Env) (fuel : Nat) (pointerLeaf : Bool) (callee : Name) (args : List Operand)
    (h : admitsFuel env pointerLeaf (fuel + 1) (Operand.call callee args) = true)
    (a : Operand) (hm : a ∈ args) :
    namesPointerValueFuel env fuel a = false := by
  have :=
    every_argument_of_an_admitted_call_is_admitted_without_the_pointer_leaf
      env fuel pointerLeaf callee args h a hm
  exact an_expression_admitted_without_the_pointer_leaf_names_no_pointer_value env fuel a this

/-- An assignment refuses the whole run wherever the pointer leaf is in play, so
an admitted pointer value can never be stored. -/
theorem an_assignment_refuses_every_pointer_leaf_position
    (env : Env) (e : Operand) :
    admitsExpression env Role.decides true e = false := by
  simp [admitsExpression, roleAllowsPointerLeaf]

/-! ## Additivity, and the refusing direction -/

theorem the_earlier_families_still_decide_every_position_they_accepted
    (env : Env) (role : Role) (assigns : Bool) (e : Operand) :
    classify true env role assigns e = Instruction.noOp := by
  simp [classify]

theorem a_refused_expression_keeps_its_unknown
    (env : Env) (role : Role) (assigns : Bool) (e : Operand)
    (h : admitsExpression env role assigns e = false) :
    classify false env role assigns e = Instruction.unknown := by
  simp [classify, h]

theorem an_admitted_expression_becomes_a_no_op
    (env : Env) (role : Role) (assigns : Bool) (e : Operand)
    (h : admitsExpression env role assigns e = true) :
    classify false env role assigns e = Instruction.noOp := by
  simp [classify, h]

/-- Switching the pointer leaf off is the strictly more refusing direction: it
withdraws admissions and never adds one. -/
theorem refusing_the_pointer_leaf_only_refuses :
    ∀ (env : Env) (fuel : Nat) (e : Operand),
      admitsFuel env false fuel e = true → admitsFuel env true fuel e = true := by
  intro env fuel e h
  cases e with
  | literal => cases fuel <;> rfl
  | enumConstant => cases fuel <;> rfl
  | scalar => cases fuel <;> rfl
  | pointerOperand => cases fuel <;> simp [admitsFuel, admitsStep] at h
  | chain c =>
      have hc : admitsChain env false c = true := by
        cases fuel <;> simpa [admitsFuel, admitsStep] using h
      have hd := a_chain_admitted_without_the_pointer_leaf_is_admitted_with_it env c hc
      cases fuel <;> simpa [admitsFuel, admitsStep] using hd
  | call callee args => cases fuel <;> simpa [admitsFuel, admitsStep] using h

/-! ## Non-vacuity: the measured shapes, in both directions

The `z_*` fixture family of
`crates/qcore-c-gateway/tests/reach_gate_positions.rs`, projected: base `0` is
`struct z_view *view`, tag `1` is `z_view`, tag `2` is `z_node`, tag `3` is
`z_leaf`; members `10 = head`, `11 = next`, `12 = leaf`, `13 = size`,
`14 = opaque`. -/
def viewEnv : Env :=
  { contract := fun n => if n == 20 then Option.some (neutralContract 1) else Option.none
    baseTag := fun n => if n == 0 then Option.some 1 else Option.none
    member := fun tag field =>
      if tag == 1 && field == 10 then Member.recordPointer 2
      else if tag == 2 && field == 11 then Member.recordPointer 2
      else if tag == 2 && field == 12 then Member.recordPointer 3
      else if tag == 2 && field == 13 then Member.scalar
      else if tag == 3 && field == 13 then Member.scalar
      else if tag == 3 && field == 14 then Member.unmodelled
      else Member.unmodelled
    bound := fun n => n == 5 }

/-- `view->head->leaf->size` — the card's measured admission. -/
def scalarChain : Operand := Operand.chain { base := 0, fields := [10, 12, 13] }

/-- `view->head->next` — the same shape with a pointer leaf. -/
def pointerChain : Operand := Operand.chain { base := 0, fields := [10, 11] }

theorem the_measured_scalar_chain_is_admitted_in_either_role :
    admitsExpression viewEnv Role.decides false scalarChain = true
      ∧ admitsExpression viewEnv Role.returned false scalarChain = true := by
  constructor <;> decide

theorem the_measured_pointer_chain_splits_on_the_role :
    admitsExpression viewEnv Role.decides false pointerChain = true
      ∧ admitsExpression viewEnv Role.returned false pointerChain = false := by
  constructor <;> decide

theorem the_measured_chain_refuses_on_a_bound_base :
    admitsExpression viewEnv Role.decides false
        (Operand.chain { base := 5, fields := [10, 12, 13] }) = false := by
  decide

theorem a_five_selection_chain_refuses :
    admitsExpression viewEnv Role.decides false
        (Operand.chain { base := 0, fields := [10, 11, 11, 11, 13] }) = false := by
  decide

theorem a_four_selection_chain_is_admitted :
    admitsExpression viewEnv Role.decides false
        (Operand.chain { base := 0, fields := [10, 11, 11, 13] }) = true := by
  decide

theorem an_opaque_leaf_refuses_the_measured_chain :
    admitsExpression viewEnv Role.decides false
        (Operand.chain { base := 0, fields := [10, 12, 14] }) = false := by
  decide

theorem a_scalar_chain_is_admitted_inside_a_call_argument :
    admitsExpression viewEnv Role.decides false (Operand.call 20 [scalarChain]) = true := by
  decide

theorem a_pointer_chain_refuses_inside_a_call_argument :
    admitsExpression viewEnv Role.decides false (Operand.call 20 [pointerChain]) = false := by
  decide

theorem an_assignment_refuses_the_measured_pointer_chain :
    admitsExpression viewEnv Role.decides true pointerChain = false := by
  decide

theorem a_bare_pointer_operand_refuses_beside_an_admitted_chain :
    admitsExpression viewEnv Role.decides false (Operand.call 20 [Operand.pointerOperand 9])
      = false := by
  decide

end QCore.ReachGatePosition
