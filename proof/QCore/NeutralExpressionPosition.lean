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

/-! # Neutral expression positions (card W4-EXPR-POS)

`crates/qcore-c-gateway/src/legacy_c.rs` classifies six expression positions — a
branch condition, a `while` condition, a `for`-loop condition, a `do`/`while`
condition, a `switch` expression and a `return` expression — through one shared
token predicate. Card C03-REACH measured that the six families this predicate
produces are 618 of the instructions that keep the neutral-frame recursive
route's corpus reach at zero
(`docs/reports/remaining-c03-reach/README.md` §3.2), and
`docs/design/W4_NEUTRAL_EXPRESSION_POSITIONS.md` §5 specifies the widening this
module models: `crates/qcore-c-gateway/src/neutral_position.rs`.

The rule is a conjunction of three families:

* **F1, the neutral call** — the contract in force declares every parameter
  `Borrowed` and the result `None`, the written arity equals the declared one,
  and every argument is admitted by the same rule at one greater depth;
* **F2, the typed scalar operand** — an enumeration constant, a numeric literal
  or a variable this function declares with a non-pointer, non-array arithmetic
  or enumeration type;
* **F3, the typed direct scalar-field read** — `base -> field` or `base . field`
  where the field is a scalar member of a completely read record and `base` is
  **not** a name this body may bind to a resource identity.

The load-bearing consequence, `an_admitted_expression_names_no_pointer` below,
is the one sentence the permissive direction rests on:

> An admitted neutral expression contains no pointer-valued operand anywhere.

That is why replacing the position's `Unknown` instruction with a `NoOp` skips
no modeling. A statement-position call is modeled — its `Borrowed` actuals are
checked live, and card C02-P-ALIAS attaches a may-alias obligation to each read
argument. Here there is nothing to model, because no argument can carry a
tracked identity at all.

## What is a premise and not a theorem

That the Rust reader extracts declared types, record members, the bound-name
over-approximation and the contract environment faithfully from C source is a
premise, exactly as in `QCore.LegacyCAnalyzer`, `QCore.BorrowedActual` and
`QCore.MayAlias`. Nothing here is a claim about arbitrary C memory safety.

`Env.contract` is the environment *in force at this point of the bottom-up
walk*, which is what the Rust reads: for a member of a recursive component under
`QCore.NeutralFrameRecursion` that is the seeded `N(m)`, and for every other
callee it is whatever the declared or already-validated environment holds.

## Modelling choice: depth as fuel

The Rust bounds nested admitted calls at `CALL_NESTING_LIMIT` by *refusing* a
deeper expression, never by truncating it. The model therefore carries the same
bound as a fuel argument, which also makes every definition here structurally
recursive on a `Nat` and every proof finite. `hasPointerFuel` is measured at the
*same* fuel as the admission it accompanies, so the soundness theorem relates
the two at one depth budget rather than across two.
-/

namespace QCore.NeutralExpressionPosition

/-- A declared name: a variable, a record member or a callee. -/
abbrev Name := Nat

/-- The custody modes a declared parameter can carry, projected onto the one
distinction this rule makes. `Borrowed` is the only admitted mode. -/
inductive ParameterMode
  | borrowed
  | other
  deriving DecidableEq, Repr

/-- The result modes, projected the same way. `none` is the only admitted one. -/
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

/-- One expression, projected onto the classifications the rule distinguishes.

`pointerOperand` stands for every operand the typed reader will not commit to as
a scalar: a declared pointer, an array, a `void *`, a cast, an address-of and a
name whose declaration the reader refused. They are one constructor because the
rule treats them identically — it refuses. -/
inductive Operand
  | literal
  | enumConstant
  /-- A name declared with a non-pointer arithmetic or enumeration type. -/
  | scalar (n : Name)
  /-- Anything the typed reader will not commit to as a scalar. -/
  | pointerOperand (n : Name)
  /-- A direct `base -> field` / `base . field` access. -/
  | fieldRead (base : Name) (field : Name)
  /-- A call written with these arguments. -/
  | call (callee : Name) (args : List Operand)
  deriving Repr

/-- The declaration and contract facts one classification reads. -/
structure Env where
  /-- The contract in force for a callee, if any. -/
  contract : Name → Option Contract
  /-- Whether a member is a scalar member of a completely read record reachable
  through this base's declared type. -/
  scalarField : Name → Name → Bool
  /-- The bound-name over-approximation: names this body may bind to a resource
  identity. -/
  bound : Name → Bool

/-- The Rust's `CALL_NESTING_LIMIT`. -/
def callNestingLimit : Nat := 4

/-- Whether a contract is the neutral one: every parameter `Borrowed`, result
`None`. -/
def isNeutralContract (c : Contract) : Bool :=
  c.result == ResultMode.none && c.parameters.all (fun m => m == ParameterMode.borrowed)

/-- One layer of the decision, with `rec` deciding the arguments.

Separating the layer from the fuel keeps the definition structurally recursive
on a `Nat` and makes every per-family theorem below a statement about this one
function. -/
def admitsStep (env : Env) (rec : Operand → Bool) : Operand → Bool
  | .literal => true
  | .enumConstant => true
  | .scalar _ => true
  | .pointerOperand _ => false
  | .fieldRead base field => !env.bound base && env.scalarField base field
  | .call callee args =>
      match env.contract callee with
      | Option.none => false
      | Option.some c =>
          isNeutralContract c && c.parameters.length == args.length && args.all rec

/-- The decision at a depth budget. At budget zero an argument is refused, which
is the Rust's `depth > CALL_NESTING_LIMIT` refusal. -/
def admitsFuel (env : Env) : Nat → Operand → Bool
  | 0 => admitsStep env (fun _ => false)
  | n + 1 => admitsStep env (admitsFuel env n)

/-- The rule as the gateway consults it. -/
def admits (env : Env) (e : Operand) : Bool :=
  admitsFuel env callNestingLimit e

/-- Whether the expression names a pointer-valued operand anywhere, measured at
the same depth budget as the admission it accompanies. -/
def hasPointerFuel : Nat → Operand → Bool
  | 0 => fun _ => false
  | n + 1 => fun e =>
      match e with
      | .pointerOperand _ => true
      | .call _ args => args.any (hasPointerFuel n)
      | _ => false

/-- Whether the expression reads through a base this body may bind, at the same
depth budget. -/
def readsBoundBaseFuel (env : Env) : Nat → Operand → Bool
  | 0 => fun _ => false
  | n + 1 => fun e =>
      match e with
      | .fieldRead base _ => env.bound base
      | .call _ args => args.any (readsBoundBaseFuel env n)
      | _ => false

/-- The instruction one classified position compiles to. `NoOp` is the whole
permissive direction of this card; `Unknown` is the existing refusal. -/
inductive Instruction
  | noOp
  | unknown
  deriving DecidableEq, Repr

/-- How `legacy_c` compiles a position: the existing lexical predicate first,
then this rule. Modelling the disjunction explicitly is the point — it is what
makes the admission additive. -/
def classify (lexical : Bool) (env : Env) (e : Operand) : Instruction :=
  if lexical || admits env e then Instruction.noOp else Instruction.unknown

/-! ## F2 and F3: operands -/

theorem a_literal_is_admitted (env : Env) (fuel : Nat) :
    admitsFuel env fuel Operand.literal = true := by
  cases fuel <;> rfl

theorem an_enumeration_constant_is_admitted (env : Env) (fuel : Nat) :
    admitsFuel env fuel Operand.enumConstant = true := by
  cases fuel <;> rfl

theorem a_declared_scalar_is_admitted (env : Env) (fuel : Nat) (n : Name) :
    admitsFuel env fuel (Operand.scalar n) = true := by
  cases fuel <;> rfl

theorem a_pointer_operand_is_never_admitted (env : Env) (fuel : Nat) (n : Name) :
    admitsFuel env fuel (Operand.pointerOperand n) = false := by
  cases fuel <;> rfl

theorem an_unbound_scalar_field_read_is_admitted
    (env : Env) (fuel : Nat) (base field : Name)
    (hb : env.bound base = false) (hf : env.scalarField base field = true) :
    admitsFuel env fuel (Operand.fieldRead base field) = true := by
  cases fuel <;> simp [admitsFuel, admitsStep, hb, hf]

theorem a_bound_base_refuses_the_read (env : Env) (fuel : Nat) (base field : Name)
    (hb : env.bound base = true) :
    admitsFuel env fuel (Operand.fieldRead base field) = false := by
  cases fuel <;> simp [admitsFuel, admitsStep, hb]

theorem a_non_scalar_field_refuses_the_read (env : Env) (fuel : Nat) (base field : Name)
    (hf : env.scalarField base field = false) :
    admitsFuel env fuel (Operand.fieldRead base field) = false := by
  cases fuel <;> simp [admitsFuel, admitsStep, hf]

/-! ## F1: calls -/

theorem an_uncontracted_call_is_never_admitted
    (env : Env) (fuel : Nat) (callee : Name) (args : List Operand)
    (h : env.contract callee = Option.none) :
    admitsFuel env fuel (Operand.call callee args) = false := by
  cases fuel <;> simp [admitsFuel, admitsStep, h]

theorem a_non_none_result_is_never_admitted
    (env : Env) (fuel : Nat) (callee : Name) (args : List Operand) (c : Contract)
    (hc : env.contract callee = Option.some c) (hr : c.result = ResultMode.other) :
    admitsFuel env fuel (Operand.call callee args) = false := by
  cases fuel <;> simp [admitsFuel, admitsStep, hc, isNeutralContract, hr]

theorem a_non_borrowed_parameter_is_never_admitted
    (env : Env) (fuel : Nat) (callee : Name) (args : List Operand) (c : Contract)
    (hc : env.contract callee = Option.some c)
    (hp : ParameterMode.other ∈ c.parameters) :
    admitsFuel env fuel (Operand.call callee args) = false := by
  have hall : c.parameters.all (fun m => m == ParameterMode.borrowed) = false := by
    simp
    exact ⟨ParameterMode.other, hp, by decide⟩
  cases fuel <;> simp [admitsFuel, admitsStep, hc, isNeutralContract, hall]

theorem a_mismatched_arity_is_never_admitted
    (env : Env) (fuel : Nat) (callee : Name) (args : List Operand) (c : Contract)
    (hc : env.contract callee = Option.some c)
    (hn : c.parameters.length ≠ args.length) :
    admitsFuel env fuel (Operand.call callee args) = false := by
  cases fuel <;> simp [admitsFuel, admitsStep, hc, hn]

theorem a_neutral_call_of_admitted_arguments_is_admitted
    (env : Env) (fuel : Nat) (callee : Name) (args : List Operand)
    (hc : env.contract callee = Option.some (neutralContract args.length))
    (ha : args.all (admitsFuel env fuel) = true) :
    admitsFuel env (fuel + 1) (Operand.call callee args) = true := by
  simp [admitsFuel, admitsStep, hc, isNeutralContract, neutralContract, ha]

theorem every_argument_of_an_admitted_call_is_admitted
    (env : Env) (fuel : Nat) (callee : Name) (args : List Operand)
    (h : admitsFuel env (fuel + 1) (Operand.call callee args) = true)
    (a : Operand) (hm : a ∈ args) :
    admitsFuel env fuel a = true := by
  revert h
  simp only [admitsFuel, admitsStep]
  cases hc : env.contract callee with
  | none => simp
  | some c =>
      intro h
      have := (Bool.and_eq_true _ _).mp h
      have hall := this.2
      exact (List.all_eq_true.mp hall) a hm

theorem a_call_exhausting_the_budget_refuses_a_nonempty_argument_list
    (env : Env) (callee : Name) (args : List Operand) (h : args ≠ []) :
    admitsFuel env 0 (Operand.call callee args) = false := by
  cases args with
  | nil => exact absurd rfl h
  | cons a rest =>
      simp only [admitsFuel, admitsStep]
      cases env.contract callee <;> simp

/-! ## The soundness sentence -/

theorem an_admitted_expression_names_no_pointer :
    ∀ (env : Env) (fuel : Nat) (e : Operand),
      admitsFuel env fuel e = true → hasPointerFuel fuel e = false := by
  intro env fuel
  induction fuel with
  | zero => intro e _; cases e <;> rfl
  | succ n ih =>
      intro e h
      cases e with
      | literal => rfl
      | enumConstant => rfl
      | scalar => rfl
      | pointerOperand m =>
          exact absurd h (by simp [a_pointer_operand_is_never_admitted env (n + 1) m])
      | fieldRead => rfl
      | call callee args =>
          simp only [hasPointerFuel, List.any_eq_false]
          intro a hm
          have := every_argument_of_an_admitted_call_is_admitted env n callee args h a hm
          simpa using ih a this

theorem an_admitted_expression_reads_through_no_bound_base :
    ∀ (env : Env) (fuel : Nat) (e : Operand),
      admitsFuel env fuel e = true → readsBoundBaseFuel env fuel e = false := by
  intro env fuel
  induction fuel with
  | zero => intro e _; cases e <;> rfl
  | succ n ih =>
      intro e h
      cases e with
      | literal => rfl
      | enumConstant => rfl
      | scalar => rfl
      | pointerOperand m =>
          exact absurd h (by simp [a_pointer_operand_is_never_admitted env (n + 1) m])
      | fieldRead base field =>
          by_cases hb : env.bound base = true
          · exact absurd h (by simp [a_bound_base_refuses_the_read env (n + 1) base field hb])
          · simpa [readsBoundBaseFuel] using Bool.not_eq_true _ ▸ hb
      | call callee args =>
          simp only [readsBoundBaseFuel, List.any_eq_false]
          intro a hm
          have := every_argument_of_an_admitted_call_is_admitted env n callee args h a hm
          simpa using ih a this

/-! ## The admission is additive, and never promotes on its own -/

theorem the_lexical_predicate_still_decides_every_position_it_accepted
    (env : Env) (e : Operand) :
    classify true env e = Instruction.noOp := by
  simp [classify]

theorem a_refused_expression_keeps_its_unknown (env : Env) (e : Operand)
    (h : admits env e = false) :
    classify false env e = Instruction.unknown := by
  simp [classify, h]

theorem an_admitted_expression_becomes_a_no_op (env : Env) (e : Operand)
    (h : admits env e = true) :
    classify false env e = Instruction.noOp := by
  simp [classify, h]

/-! ## Non-vacuity: the measured shapes -/

/-- The environment `digits10`'s own component seeds: `N(1)` for `digits10`,
no record member and no bound name. -/
def digits10Env : Env :=
  { contract := fun n => if n == 0 then Option.some (neutralContract 1) else Option.none
    scalarField := fun _ _ => false
    bound := fun _ => false }

/-- `return 12 + digits10(v / 1000000000000UL);` — the design note's §6 minimal
validation case, projected: one admitted neutral call of one scalar operand,
beside two literals. -/
def digits10Return : Operand :=
  Operand.call 0 [Operand.scalar 1]

theorem the_digits10_return_expression_is_admitted :
    admits digits10Env digits10Return = true := by
  decide

theorem the_digits10_return_expression_refuses_without_its_contract :
    admits { digits10Env with contract := fun _ => Option.none } digits10Return = false := by
  decide

/-- The same call with a pointer actual: refused, which is the C00 hazard this
rule must not reopen. -/
theorem the_same_call_with_a_pointer_actual_refuses :
    admits digits10Env (Operand.call 0 [Operand.pointerOperand 1]) = false := by
  decide

/-- A releasing callee at the same arity: refused. -/
theorem a_releasing_callee_refuses :
    admits
        { digits10Env with
          contract := fun n =>
            if n == 0 then
              Option.some { parameters := [ParameterMode.other], result := ResultMode.none }
            else Option.none }
        digits10Return = false := by
  decide

/-- A callee handing back an owned result at the same arity: refused. -/
theorem an_owned_result_refuses :
    admits
        { digits10Env with
          contract := fun n =>
            if n == 0 then
              Option.some { parameters := [ParameterMode.borrowed], result := ResultMode.other }
            else Option.none }
        digits10Return = false := by
  decide

/-- Nesting: four admitted calls fit the budget and five do not, so the bound is
a refusal rather than a truncation. -/
theorem four_nested_neutral_calls_are_admitted :
    admits digits10Env
        (Operand.call 0 [Operand.call 0 [Operand.call 0 [Operand.call 0 [Operand.scalar 1]]]])
      = true := by
  decide

theorem five_nested_neutral_calls_refuse :
    admits digits10Env
        (Operand.call 0
          [Operand.call 0 [Operand.call 0 [Operand.call 0 [Operand.call 0 [Operand.scalar 1]]]]])
      = false := by
  decide

/-- The field-read family is not vacuous either. -/
def recordEnv : Env :=
  { contract := fun _ => Option.none
    scalarField := fun base field => base == 7 && field == 9
    bound := fun n => n == 8 }

theorem an_unbound_base_reads_its_scalar_member :
    admits recordEnv (Operand.fieldRead 7 9) = true := by
  decide

theorem a_bound_base_never_reads_even_a_scalar_member :
    admits { recordEnv with scalarField := fun _ _ => true } (Operand.fieldRead 8 9) = false := by
  decide

theorem an_unmodelled_member_refuses :
    admits recordEnv (Operand.fieldRead 7 10) = false := by
  decide

/-- One unadmitted call refuses the whole expression even beside an admitted
one: the all-or-nothing rule, which is why no unmodelled call can hide. -/
theorem one_unadmitted_argument_refuses_the_whole_call :
    admits digits10Env (Operand.call 0 [Operand.call 5 [Operand.scalar 1]]) = false := by
  decide

end QCore.NeutralExpressionPosition
