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

import QCore.ChainedTailActual

/-!
# Declared-contract / C-declaration ABI agreement

Card **LOADER-ABI**. A resource-contract v1 declaration writes an opaque C ABI
projection for every parameter and for its result. Those spellings were checked
against the loader's own closed table and against **nothing else**: nothing
compared them with the C declaration of the same function in the translation
unit being analysed. They are not decoration, because the loader's admissibility
rules are keyed on them — a custody mode requires a pointer parameter, an out
mode requires a pointer to pointer, a non-`None` result requires a pointer
return type — so an unfaithful spelling is exactly what buys a custody mode the
real C type could not carry.

This is the model of the comparison
(`crates/qcore-c-gateway/src/legacy_declared_abi.rs`) and of what a
disagreement does.

## What is proved

* **Agreement applies the declared contract, unchanged.** Nothing here can
  invent, alter or widen a declaration: an applied contract is the declared one.
* **Every disagreement class withdraws**, and each class is reachable, so no
  branch of the comparison is vacuous.
* **Withdrawal is monotone in the refusing direction**: it can only turn a
  declared callee into an undeclared one.
* **The two fail-closed edges**: a retained source set that cannot be re-read
  withdraws *every* contracted name rather than none of them, and an
  environment that declares no signature compares nothing.
* **The one admission that is not exact equality**, argued rather than
  excepted: a contract naming no parameters and claiming no variadic tail
  survives a C declaration whose parameter list is empty — it asserts nothing
  about parameters, and the analyzer's own call-site arity gate refuses every
  call that passes an actual to it. Both halves are proved.
* **The projection is set-valued on purpose**: on the LP64 targets audited here
  `unsigned long` and a pointer are eight bytes, so `u64` and `usize` are two
  spellings of one reading and both are admissible; signedness and width are
  proved never to cross.

## What this is not

It is a model of a **decision**. That the reader's C projection is the type C
really declares, that the analysed token stream is the translation unit, and
C's own object model are premises supplied from outside. A withdrawn
declaration says nothing about the C body either: it says this analysis will not
apply that declaration here.
-/

namespace QCore.DeclaredAbiAgreement

/-- The loader's closed ABI vocabulary: `SCALAR_ABI_TYPES`, `POINTER_ABI_TYPES`,
`FUNCTION_POINTER_ABI_TYPE` and the result-only `unit`.

`fnptr` was added by card **CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF**. It is
deliberately not one of the three pointer spellings: the loader's custody gates
key on the leading word `ptr`, so keeping it apart is what makes every custody
mode on a function pointer a load-time rejection. `QCore.TypedefProjection`
proves that half.

`vaList` (`VA_LIST_ABI_TYPE`, spelled `va_list`) was added by card
**W44-LIBC-RESIDUALS**: the projection of the compiler's `__builtin_va_list` at
a parameter. It is kept apart from the pointer spellings for the same reason
`fnptr` is, and it is a parameter type only. `QCore.GnuBuiltinReader` proves
both halves. -/
inductive AbiType where
  | unit
  | bool
  | i8 | u8 | i16 | u16 | i32 | u32 | i64 | u64 | isize | usize
  | f32 | f64
  | ptrMut | ptrConst | ptrMutMut
  | fnptr
  | vaList
  deriving DecidableEq, BEq, Repr

/-- The C side of one type position, projected into that vocabulary.

`none` is a declaration this reader will not commit to; `some ts` is the set of
spellings the C type admits — a *set*, because one C type can have several
faithful spellings in this vocabulary (see the LP64 section). -/
abbrev Projection := Option (List AbiType)

/-- One C function declaration, as read from the analysed token stream. -/
structure CDeclaration where
  result : Projection
  parameters : List Projection
  /-- The C declaration ends in `...`. -/
  variadic : Bool
  /-- The C parameter list is empty, which specifies nothing (C11 6.7.6.3p14). -/
  unspecified : Bool
  deriving DecidableEq, Repr

/-- The declared side: the ABI spellings the contract module writes, plus
whether it carries `resource_variadic_data_tail()`. -/
structure Signature where
  parameters : List AbiType
  result : AbiType
  tail : Bool
  deriving DecidableEq, Repr

/-- Why one declared contract is withdrawn for one translation unit. -/
inductive Refusal where
  | returnType
  | parameterType
  | arity
  | variadic
  | unspecifiedParameters
  | unreadableDeclaration
  | ambiguousDeclaration
  | unreadableDeclaredSources
  deriving DecidableEq, BEq, Repr

/-- Every refusal, in declaration order. The vocabulary is closed. -/
def allRefusals : List Refusal :=
  [.returnType, .parameterType, .arity, .variadic, .unspecifiedParameters,
   .unreadableDeclaration, .ambiguousDeclaration, .unreadableDeclaredSources]

/-- Whether a C projection admits one declared spelling. An unreadable
projection admits nothing. -/
def admits (p : Projection) (t : AbiType) : Bool :=
  match p with
  | none => false
  | some ts => ts.any (· == t)

/-- Compare the declared parameter vector against the C one, position by
position. Lengths are equal at every call site below. -/
def compareParameters : List AbiType → List Projection → Option Refusal
  | [], [] => none
  | t :: ts, p :: ps =>
      if admits p t then compareParameters ts ps
      else if p.isNone then some .unreadableDeclaration else some .parameterType
  | _, _ => some .arity

/-- The whole comparison, in the implementation's order. -/
def compare (s : Signature) (d : CDeclaration) : Option Refusal :=
  if d.result.isNone then some .unreadableDeclaration
  else if !admits d.result s.result then some .returnType
  else if d.unspecified then
    -- The one admission that is not exact equality; see `callAdmitted` below.
    (if s.parameters.isEmpty && !s.tail then none else some .unspecifiedParameters)
  else if s.parameters.length ≠ d.parameters.length then some .arity
  else
    match compareParameters s.parameters d.parameters with
    | some r => some r
    | none => if s.tail && !d.variadic then some .variadic else none

/-- A function name. Names are opaque, so they are modelled as indices. -/
abbrev Name := Nat

/-- A declared contract. Only its identity matters here: the comparison never
reads a mode, and withdrawal never rewrites one. -/
abbrev Contract := Nat

/-- One translation unit's view: the contracts in force, the ABI signatures
rederived from the environment's own retained texts, and the C declarations
read from the analysed token stream. `poisoned` records retained texts the
loader cannot re-read. -/
structure Environment where
  contracts : Name → Option Contract
  signatures : Name → Option Signature
  declarations : Name → Option CDeclaration
  poisoned : Bool

/-- Why `n`'s declaration is withdrawn in this unit, if it is. -/
def withdrawal (e : Environment) (n : Name) : Option Refusal :=
  if e.poisoned then
    (if (e.contracts n).isSome then some .unreadableDeclaredSources else none)
  else
    match e.signatures n, e.declarations n with
    | some s, some d => compare s d
    | _, _ => none

/-- The contract actually applied in this unit. -/
def applied (e : Environment) (n : Name) : Option Contract :=
  if (withdrawal e n).isSome then none else e.contracts n

/-! ### Agreement applies the declared contract, unchanged -/

theorem agreement_applies_the_declared_contract {e : Environment} {n : Name}
    (h : withdrawal e n = none) : applied e n = e.contracts n := by
  unfold applied; simp [h]

theorem an_applied_contract_is_the_declared_one {e : Environment} {n : Name} {k : Contract}
    (h : applied e n = some k) : e.contracts n = some k := by
  unfold applied at h
  split at h
  · exact absurd h (by simp)
  · exact h

theorem an_undeclared_callee_is_never_applied {e : Environment} {n : Name}
    (h : e.contracts n = none) : applied e n = none := by
  unfold applied; split <;> simp [h]

/-! ### Every disagreement withdraws -/

theorem a_disagreement_withdraws {e : Environment} {n : Name} {r : Refusal}
    (h : withdrawal e n = some r) : applied e n = none := by
  unfold applied; simp [h]

theorem a_compared_disagreement_withdraws {e : Environment} {n : Name}
    {s : Signature} {d : CDeclaration} {r : Refusal}
    (hp : e.poisoned = false) (hs : e.signatures n = some s)
    (hd : e.declarations n = some d) (hc : compare s d = some r) :
    applied e n = none := by
  refine a_disagreement_withdraws (r := r) ?_
  unfold withdrawal
  simp [hp, hs, hd, hc]

/-! ### The two fail-closed edges -/

/-- Retained texts the loader cannot re-read withdraw **every** contracted
name, rather than comparing none of them. An empty signature map would apply
every declaration, which is the fail-open direction. -/
theorem poison_withdraws_every_contracted_name {e : Environment} {n : Name}
    (hp : e.poisoned = true) (hc : (e.contracts n).isSome = true) :
    applied e n = none := by
  refine a_disagreement_withdraws (r := Refusal.unreadableDeclaredSources) ?_
  unfold withdrawal
  simp [hp, hc]

/-- An environment that declares no ABI signature for a name — an inferred W4
summary, a provider proposal, a directly built table — compares nothing, and
the contract applies exactly as before. Recorded, not implied. -/
theorem an_environment_without_a_signature_compares_nothing {e : Environment} {n : Name}
    (hp : e.poisoned = false) (hs : e.signatures n = none) :
    applied e n = e.contracts n := by
  refine agreement_applies_the_declared_contract ?_
  unfold withdrawal
  simp [hp, hs]

/-- The recorded boundary `LOADER-ABI-UNDECLARED`: a name this unit does not
declare is not compared, and its contract still applies. -/
theorem an_undeclared_name_is_not_compared {e : Environment} {n : Name}
    (hp : e.poisoned = false) (hd : e.declarations n = none) :
    applied e n = e.contracts n := by
  refine agreement_applies_the_declared_contract ?_
  unfold withdrawal
  cases e.signatures n <;> simp [hp, hd]

/-! ### Withdrawal is monotone in the refusing direction -/

/-- Withdrawal removes declarations; it never adds one. -/
theorem withdrawal_only_removes {e : Environment} {n : Name}
    (h : (applied e n).isSome = true) : (e.contracts n).isSome = true := by
  cases hk : applied e n with
  | none => simp [hk] at h
  | some k => simp [an_applied_contract_is_the_declared_one hk]

/-- Reading a *first* C declaration for a name can only refuse more: where
nothing was compared, the contract applied only if it was declared, and adding a
declaration never turns an unapplied contract into an applied one. -/
theorem reading_a_first_declaration_never_admits {e : Environment} {n : Name}
    {d : CDeclaration} (hd : e.declarations n = none) (h : applied e n = none) :
    applied { e with declarations := fun m => if m = n then some d else e.declarations m } n
      = none := by
  by_cases hp : e.poisoned = true
  · unfold applied withdrawal
    simp only [hp, ite_true]
    cases e.contracts n with
    | none => simp
    | some k => simp
  · simp only [Bool.not_eq_true] at hp
    have hcontract : e.contracts n = none := by
      have hopen := an_undeclared_name_is_not_compared (e := e) (n := n) hp hd
      rw [hopen] at h
      exact h
    exact an_undeclared_callee_is_never_applied (by simpa using hcontract)

/-! ### The LP64 projection, and what it refuses to cross -/

/-- `unsigned long` on the audited LP64 targets. -/
def unsignedLong : Projection := some [.u64, .usize]

/-- `long`/`ptrdiff_t` on the same targets. -/
def signedLong : Projection := some [.i64, .isize]

/-- `int`. -/
def signedInt : Projection := some [.i32]

/-- Plain `char`, whose signedness C leaves implementation-defined. -/
def plainChar : Projection := some [.i8, .u8]

/-- A mutable object pointer, whatever its pointee. -/
def pointerMut : Projection := some [.ptrMut]

/-- A pointer to a `const`-qualified pointee. -/
def pointerConst : Projection := some [.ptrConst]

theorem an_unsigned_long_admits_both_of_its_spellings :
    admits unsignedLong .u64 = true ∧ admits unsignedLong .usize = true := by decide

theorem signedness_never_crosses :
    admits unsignedLong .i64 = false ∧ admits signedLong .u64 = false := by decide

theorem width_never_crosses :
    admits unsignedLong .u32 = false ∧ admits signedInt .i64 = false := by decide

theorem a_plain_char_admits_both_byte_spellings :
    admits plainChar .i8 = true ∧ admits plainChar .u8 = true := by decide

theorem constness_never_crosses :
    admits pointerMut .ptrConst = false ∧ admits pointerConst .ptrMut = false := by decide

theorem an_unreadable_projection_admits_nothing (t : AbiType) :
    admits none t = false := by
  cases t <;> rfl

/-! ### Each refusal class, reached -/

/-- The faithful declaration of `void k_use(void *p, int n)`. -/
def faithful : Signature := ⟨[.ptrMut, .i32], .unit, false⟩

/-- Its C declaration, as read. -/
def declaration : CDeclaration := ⟨some [.unit], [pointerMut, signedInt], false, false⟩

theorem the_faithful_pair_agrees : compare faithful declaration = none := by decide

theorem a_wrong_result_type_is_refused :
    compare { faithful with result := .ptrMut } declaration = some .returnType := by decide

theorem a_wrong_parameter_type_is_refused :
    compare { faithful with parameters := [.ptrMut, .usize] } declaration
      = some .parameterType := by decide

theorem a_swapped_parameter_vector_is_refused :
    compare { faithful with parameters := [.i32, .ptrMut] } declaration
      = some .parameterType := by decide

theorem a_wrong_arity_is_refused :
    compare { faithful with parameters := [.ptrMut] } declaration = some .arity := by decide

theorem a_tail_claimed_of_a_non_variadic_callee_is_refused :
    compare { faithful with tail := true } declaration = some .variadic := by decide

/-- The other direction is not a disagreement: the clause only ever widens what
the arity gate admits, so its absence is the refusing direction. -/
theorem a_variadic_callee_without_a_declared_tail_agrees :
    compare faithful { declaration with variadic := true } = none := by decide

theorem an_unreadable_parameter_type_is_refused :
    compare faithful { declaration with parameters := [pointerMut, none] }
      = some .unreadableDeclaration := by decide

theorem an_unreadable_result_type_is_refused :
    compare faithful { declaration with result := none }
      = some .unreadableDeclaration := by decide

theorem an_unspecified_list_refuses_a_declared_vector :
    compare faithful { declaration with parameters := [], unspecified := true }
      = some .unspecifiedParameters := by decide

/-! ### The zero-arity admission, and the gate that makes it safe -/

/-- A contract naming no parameters and claiming no tail. The measured instance
is Fluent Bit's `double flb_time_now();`. -/
def nullary : Signature := ⟨[], .f64, false⟩

/-- Its C declaration, whose empty parameter list specifies nothing. -/
def nullaryUnspecified : CDeclaration := ⟨some [.f64], [], false, true⟩

theorem a_nullary_contract_survives_an_unspecified_list :
    compare nullary nullaryUnspecified = none := by decide

/-- A tail claim is still refused there: it would admit actuals C never typed. -/
theorem a_tail_claim_never_survives_an_unspecified_list :
    compare { nullary with tail := true } nullaryUnspecified
      = some .unspecifiedParameters := by decide

/-- The analyzer's existing call-site arity gate: a call is modelled only when
it passes no more actuals than the declaration names, or the declaration claims
a tail. -/
def callAdmitted (s : Signature) (actuals : Nat) : Bool :=
  decide (actuals ≤ s.parameters.length) || s.tail

/-- The sentence the admission rests on: under a nullary tail-free contract the
gate admits **only** the call that passes nothing, so an unspecified C list has
nothing left to contradict. -/
theorem a_nullary_contract_admits_only_the_empty_call (actuals : Nat) :
    callAdmitted nullary actuals = true ↔ actuals = 0 := by
  unfold callAdmitted nullary
  simp

theorem a_nullary_contract_refuses_any_actual :
    callAdmitted nullary 1 = false := by decide

/-! ### The vocabulary is closed, and every class is distinct -/

theorem the_refusal_vocabulary_has_eight_members : allRefusals.length = 8 := by decide

theorem the_refusal_vocabulary_has_no_duplicates : allRefusals.Nodup := by decide

theorem every_refusal_is_in_the_vocabulary (r : Refusal) : r ∈ allRefusals := by
  cases r <;> simp [allRefusals]

/-! ### Composition with card NEIGHBOURS-CHAINED-TAIL's clause T5

The composed tree runs this comparison *and* T5, and they meet on one question:
T5 admits a bounded typed access chain at a position **the callee's declaration
covers with `resource_variadic_data_tail()`**, and this card decides whether
that declaration is applied here at all. The two are not independent, and the
sentence that ties them has to be stated, because the implementation withdraws
a declaration by rewriting its call sites rather than by clearing the tail set
rederived from the retained texts: the retained text of a withdrawn declaration
still *mentions* its tail. What makes that safe is that no tail T5 may act on
survives the withdrawal, which is exactly the theorem below.

The direction that matters is one-way. An agreeing declaration supplies its own
tail unchanged — this card adds nothing to what T5 may admit — while a withdrawn
one supplies none, so T5's admission is refused for it by T5's *own* premise
(`ChainedTailActual.the_declaration_is_read_from_the_callee_never_from_the_actual`),
not by a rule invented here. -/

/-- The declared tail a consumer may act on: the one carried by a declaration
that is actually applied in this unit. -/
def appliedTail (e : Environment) (n : Name) : Bool :=
  match applied e n, e.signatures n with
  | some _, some s => s.tail
  | _, _ => false

theorem a_withdrawn_declaration_supplies_no_tail {e : Environment} {n : Name} {r : Refusal}
    (h : withdrawal e n = some r) : appliedTail e n = false := by
  unfold appliedTail
  rw [a_disagreement_withdraws h]

theorem an_agreeing_declaration_supplies_its_own_tail {e : Environment} {n : Name}
    {s : Signature} {k : Contract} (hw : withdrawal e n = none)
    (hs : e.signatures n = some s) (hc : e.contracts n = some k) :
    appliedTail e n = s.tail := by
  unfold appliedTail
  rw [agreement_applies_the_declared_contract hw, hc, hs]

/-- The composed sentence: a declaration this unit's own C declaration
contradicts admits **no** tail actual, chain or not. -/
theorem a_withdrawn_declaration_admits_no_tail_actual {e : Environment} {n : Name}
    {r : Refusal} (h : withdrawal e n = some r) (read : Bool)
    (excess : List QCore.ChainedTailActual.TailArgument) :
    QCore.ChainedTailActual.admits
        { declarationsRead := read, declaredTail := appliedTail e n, excess := excess }
      = false := by
  rw [a_withdrawn_declaration_supplies_no_tail h]
  exact QCore.ChainedTailActual.the_declaration_is_read_from_the_callee_never_from_the_actual
    read excess

/-- And the other direction is not lost: where the comparison agrees, T5 decides
exactly as it does without this card. -/
theorem an_agreeing_declaration_leaves_t5_alone {e : Environment} {n : Name}
    {s : Signature} {k : Contract} (hw : withdrawal e n = none)
    (hs : e.signatures n = some s) (hc : e.contracts n = some k) (read : Bool)
    (excess : List QCore.ChainedTailActual.TailArgument) :
    QCore.ChainedTailActual.admits
        { declarationsRead := read, declaredTail := appliedTail e n, excess := excess }
      = QCore.ChainedTailActual.admits
        { declarationsRead := read, declaredTail := s.tail, excess := excess } := by
  rw [an_agreeing_declaration_supplies_its_own_tail hw hs hc]

/-! ### Composing two declarations of one name -/

/-- Compose the readings of one name in one unit: an unspecified list composes
with a specified one to the specified signature (C11 6.2.7p3), two specified
readings must agree, and disagreeing results are ambiguous either way. -/
def compose (l r : CDeclaration) : Option CDeclaration :=
  if l.result ≠ r.result then none
  else if l.unspecified then some r
  else if r.unspecified then some l
  else if l.parameters = r.parameters ∧ l.variadic = r.variadic then some l
  else none

/-- The refusal a failed composition carries. -/
def compositionRefusal : Refusal := .ambiguousDeclaration

theorem a_prototype_and_its_definition_compose :
    compose ⟨some [.unit], [], false, true⟩ declaration = some declaration := by decide

theorem two_agreeing_declarations_are_one :
    compose declaration declaration = some declaration := by decide

theorem two_disagreeing_declarations_are_ambiguous :
    compose declaration { declaration with parameters := [pointerMut, unsignedLong] }
      = none := by decide

theorem disagreeing_results_are_ambiguous :
    compose declaration { declaration with result := some [.ptrMut] }
      = none := by decide

/-- Composition never invents a third reading: its result is one of its inputs.
-/
theorem composition_returns_one_of_its_inputs {l r : CDeclaration} {d : CDeclaration}
    (h : compose l r = some d) : d = l ∨ d = r := by
  unfold compose at h
  split at h
  · exact absurd h (by simp)
  · split at h
    · exact Or.inr (by simpa using h.symm)
    · split at h
      · exact Or.inl (by simpa using h.symm)
      · split at h
        · exact Or.inl (by simpa using h.symm)
        · exact absurd h (by simp)

end QCore.DeclaredAbiAgreement
