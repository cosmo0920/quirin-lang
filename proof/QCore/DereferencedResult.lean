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

import QCore.VariadicNested

/-!
# A dereference of a neutral call's result (CONSTRUCTOR-RESIDUALS)

`QCore.VariadicNested` models card O04-TRACE's nested-call whitelist, whose
premise **N4** requires the nested call to be a *whole* argument of the
enclosing call. Card ALIAS-AGGREGATE measured what that costs on the
byte-verbatim upstream `flb_input_chunk_create`: two refusals whose reaching
state accounts for three more, all five produced by the same statement shape

```c
flb_errno_print((*__errno_location ()), "<stdin>", 2556);   /* flb_errno() */
```

whose first argument is a **dereference of a call result**.

This module models the clause `crates/qcore-c-gateway/src/variadic_nested.rs`
adds, **N4-D**, together with the new premise **N10** that pays for it.

## The refined sentence, and why refinement rather than violation

Card W4-EXPR-POS's neutral-expression families are proved under

> an admitted expression contains no pointer-valued operand,

and `__errno_location ()` is pointer-valued *by construction*, so that sentence
does not reach here and must not be stretched. `the_earlier_sentence_does_not_cover_this`
states that plainly: an admitted dereference has a pointer-valued operand.

What replaces it is a statement about what the **projection** tracks rather than
about what C calls a pointer:

> An admitted dereference-of-a-call-result binds, releases, escapes and
> addresses nothing this projection tracks.

Each conjunct is discharged by a premise already required, and
`the_refined_sentence_holds` proves the conjunction from `admits`:

* *binds nothing* — **N6** declares the nested result `none`, so no identity is
  produced and no name is bound to it, and the wrapper shapes carry no `=`;
* *releases nothing* — **N7** declares every nested parameter `borrowed`, so the
  nested call's declared effects are the identity on the abstract state;
* *escapes nothing* — **N9** requires every inner actual to be a place the
  projection denotes or data;
* *addresses nothing* — no admitted wrapper carries `&`, and **N10** requires the
  dereferenced pointee to be a **scalar**, so the value of the whole argument is
  a scalar rvalue no position of this projection can bind.

## Two directions, both audited

* **Widening, stated as such.** With the bare wrapper the rule *is*
  `admitsNested`, pointwise; and the widening is strict — the subject shape is
  exhibited, refused before and admitted after.
* **Fail-closed.** Every `admitsNested` refusal lifts unchanged, N10 refuses
  every non-scalar pointee in both spellings, and every other wrapper shape
  refuses. A refused candidate leaves card O04-B09B10's nested refusal in place.

This is a model of the *decision*. That the reader extracts the callee's
declared return type faithfully from this unit's own declarations is its own
obligation, assumed here exactly as in `QCore.TypedFieldAccess`.
-/

namespace QCore.DereferencedResult

open QCore.VariadicNested

/-- How the outer argument wraps the nested call. -/
inductive Wrapper where
  /-- `f ( … )` — card O04-TRACE's shape, unchanged. -/
  | bare
  /-- `* f ( … )`. -/
  | star
  /-- `( * f ( … ) )` — the spelling `errno` expands to. -/
  | parenStar
  /-- Everything else: a cast, `& * f ( )`, `* * f ( )`, a continuation, an
      arithmetic combination, a subscript, a further parenthesisation. -/
  | other
  deriving DecidableEq, Repr

/-- What this unit's own declarations say the nested callee returns. -/
inductive Pointee where
  /-- Exactly one `*` over a specifier list resolving to a scalar. -/
  | scalar
  /-- One `*` over a record, or two or more `*`: dereferencing yields a pointer
      or an aggregate. -/
  | notScalar
  /-- No declaration of the callee this reader will commit to. Absence, not a
      third class: it refuses. -/
  | unread
  deriving DecidableEq, Repr

/-- Whether the wrapper is one of the three admitted shapes. -/
def wrapperAdmitted : Wrapper → Bool
  | Wrapper.bare => true
  | Wrapper.star => true
  | Wrapper.parenStar => true
  | Wrapper.other => false

/-- Whether the wrapper actually dereferences the result. -/
def dereferences : Wrapper → Bool
  | Wrapper.bare => false
  | Wrapper.star => true
  | Wrapper.parenStar => true
  | Wrapper.other => false

/-- One candidate outer argument: the nested call as card O04-TRACE reads it,
    plus this card's two new facts. -/
structure Candidate where
  nested : Nested
  wrapper : Wrapper
  /-- **N10** — the nested callee's declared return pointee, read from this
      unit's declarations. Consulted only when the wrapper dereferences. -/
  pointee : Pointee
  deriving DecidableEq, Repr

/-- **N10**: a dereferenced result must be a pointer to a scalar. A bare
    argument does not read the pointee at all, which is what keeps the base
    family byte-identical. -/
def pointeeAdmitted (d : Candidate) : Bool :=
  !dereferences d.wrapper || d.pointee == Pointee.scalar

/-- The widened admission: every O04-TRACE premise, **and** an admitted wrapper,
    **and** N10. A conjunction, so failing any one refuses. -/
def admits (d : Candidate) : Bool :=
  admitsNested d.nested && wrapperAdmitted d.wrapper && pointeeAdmitted d

/-! ## The base family is unchanged -/

/-- With the bare wrapper the widened rule **is** card O04-TRACE's rule,
    pointwise and for every pointee reading. Nothing about the base family
    moved. -/
theorem the_bare_wrapper_is_exactly_the_base_admission (n : Nested) (p : Pointee) :
    admits { nested := n, wrapper := Wrapper.bare, pointee := p } = admitsNested n := by
  cases h : admitsNested n <;> simp [admits, pointeeAdmitted, dereferences, wrapperAdmitted, h]

/-- Everything card O04-TRACE admitted is still admitted. -/
theorem every_previously_admitted_nested_call_is_still_admitted (n : Nested) (p : Pointee)
    (old : admitsNested n = true) :
    admits { nested := n, wrapper := Wrapper.bare, pointee := p } = true := by
  simp [the_bare_wrapper_is_exactly_the_base_admission, old]

/-! ## Fail-closed: every base refusal lifts -/

theorem a_refused_nested_call_refuses_the_candidate (d : Candidate)
    (refused : admitsNested d.nested = false) : admits d = false := by
  simp [admits, refused]

theorem unread_declarations_never_admit (d : Candidate)
    (unread : d.nested.declarationsRead = false) : admits d = false :=
  a_refused_nested_call_refuses_the_candidate d
    (unread_declarations_never_admit_a_nested_call d.nested unread)

theorem a_two_level_nested_call_is_never_admitted (d : Candidate)
    (deeper : d.nested.oneLevel = false) : admits d = false :=
  a_refused_nested_call_refuses_the_candidate d
    (VariadicNested.a_two_level_nested_call_is_never_admitted d.nested deeper)

theorem a_call_inside_the_nested_arguments_is_never_admitted (d : Candidate)
    (inner : d.nested.innerArgumentsCallFree = false) : admits d = false :=
  a_refused_nested_call_refuses_the_candidate d
    (VariadicNested.a_call_inside_the_nested_arguments_is_never_admitted d.nested inner)

theorem an_uncontracted_nested_callee_is_never_admitted (d : Candidate)
    (uncontracted : d.nested.calleeContracted = false) : admits d = false :=
  a_refused_nested_call_refuses_the_candidate d
    (VariadicNested.an_uncontracted_nested_callee_is_never_admitted d.nested uncontracted)

theorem an_uncontracted_outer_callee_never_admits (d : Candidate)
    (uncontracted : d.nested.outerContracted = false) : admits d = false :=
  a_refused_nested_call_refuses_the_candidate d
    (VariadicNested.an_uncontracted_outer_callee_never_admits_a_nested_call d.nested uncontracted)

theorem a_nested_arity_mismatch_is_never_admitted (d : Candidate)
    (arity : d.nested.innerArityExact = false) : admits d = false :=
  a_refused_nested_call_refuses_the_candidate d
    (VariadicNested.a_nested_arity_mismatch_is_never_admitted d.nested arity)

/-- **N9**, the fail-open card O04-B09B10 closed, inherited here unchanged: an
    inner actual the projection can neither denote nor call data refuses, so no
    tracked identity can escape through the dereferenced call's arguments. -/
theorem an_undenotable_inner_actual_is_never_admitted (d : Candidate)
    (undenotable : d.nested.innerActualsDenotable = false) : admits d = false :=
  a_refused_nested_call_refuses_the_candidate d
    (VariadicNested.an_undenotable_inner_actual_is_never_admitted d.nested undenotable)

/-- **N6**: a dereference does not make a discarded owned result disappear. -/
theorem an_owned_nested_result_is_never_admitted (d : Candidate)
    (owned : d.nested.contract.result = ResultMode.owned) : admits d = false :=
  a_refused_nested_call_refuses_the_candidate d
    (VariadicNested.an_owned_nested_result_is_never_admitted d.nested owned)

/-- **N7**: nor does it make a nested release vanish into the exemption. -/
theorem a_custody_bearing_nested_parameter_is_never_admitted (d : Candidate)
    (m : ParamMode) (present : m ∈ d.nested.contract.modes) (custody : m ≠ ParamMode.borrowed) :
    admits d = false :=
  a_refused_nested_call_refuses_the_candidate d
    (VariadicNested.a_custody_bearing_nested_parameter_is_never_admitted d.nested m present custody)

/-- **N8**: the position is decided by the enclosing declaration and is not
    weakened by the wrapper. -/
theorem a_custody_slot_never_admits (d : Candidate)
    (slot : d.nested.slot = Slot.custodyParameter) : admits d = false :=
  a_refused_nested_call_refuses_the_candidate d
    (VariadicNested.a_custody_slot_never_admits_a_nested_call d.nested slot)

theorem a_candidate_beyond_the_named_vector_needs_a_declared_tail (d : Candidate)
    (slot : d.nested.slot = Slot.beyondNamedVector) (noTail : d.nested.outerDeclaredTail = false) :
    admits d = false :=
  a_refused_nested_call_refuses_the_candidate d
    (VariadicNested.a_nested_call_beyond_the_named_vector_needs_a_declared_tail
      d.nested slot noTail)

/-! ## The wrapper half -/

/-- Every wrapper shape outside the three admitted ones refuses, whatever the
    nested call and whatever the pointee. -/
theorem every_other_wrapper_shape_refuses (n : Nested) (p : Pointee) :
    admits { nested := n, wrapper := Wrapper.other, pointee := p } = false := by
  simp [admits, wrapperAdmitted]

/-- **Both spellings answer alike.** `errno` is written with the enclosing
    parentheses and a hand-written dereference usually is not; the rule must not
    depend on parenthesisation. -/
theorem both_spellings_answer_alike (n : Nested) (p : Pointee) :
    admits { nested := n, wrapper := Wrapper.star, pointee := p }
      = admits { nested := n, wrapper := Wrapper.parenStar, pointee := p } := by
  simp [admits, pointeeAdmitted, dereferences, wrapperAdmitted]

/-! ## The N10 half -/

/-- A dereferenced result the reader did not type as a pointer to a scalar is
    refused in both spellings. This is what keeps `* f ( )` from denoting, say,
    a `struct chunk *` and reaching a position as an identity nobody named. -/
theorem a_non_scalar_pointee_refuses (n : Nested) (w : Wrapper) (p : Pointee)
    (deref : dereferences w = true) (notScalar : p ≠ Pointee.scalar) :
    admits { nested := n, wrapper := w, pointee := p } = false := by
  have : (p == Pointee.scalar) = false := by
    simpa using notScalar
  simp [admits, pointeeAdmitted, deref, this]

/-- An unread declaration is absence, not a third class: it refuses. -/
theorem an_unread_pointee_refuses (n : Nested) (w : Wrapper) (deref : dereferences w = true) :
    admits { nested := n, wrapper := w, pointee := Pointee.unread } = false :=
  a_non_scalar_pointee_refuses n w Pointee.unread deref (by decide)

/-- And N10 is consulted **only** under a dereference, which is why the base
    family cannot move: a bare argument answers the same for every pointee. -/
theorem the_pointee_is_read_only_under_a_dereference (n : Nested) (p q : Pointee) :
    admits { nested := n, wrapper := Wrapper.bare, pointee := p }
      = admits { nested := n, wrapper := Wrapper.bare, pointee := q } := by
  simp [the_bare_wrapper_is_exactly_the_base_admission]

/-! ## The refined soundness sentence -/

/-- The four conjuncts, over the projection's own vocabulary. -/
structure Refined (d : Candidate) : Prop where
  /-- No identity is produced, so no name binds one. -/
  bindsNothing : d.nested.contract.result = ResultMode.none
  /-- Every declared nested parameter is `borrowed`. -/
  releasesNothing : ∀ m ∈ d.nested.contract.modes, m = ParamMode.borrowed
  /-- Every inner actual is a place the projection denotes, or data. -/
  escapesNothing : d.nested.innerActualsDenotable = true
  /-- No admitted wrapper carries `&`, and a dereferenced value is a scalar. -/
  addressesNothing : pointeeAdmitted d = true

/-- **The refined sentence, proved from the admission.** -/
theorem the_refined_sentence_holds (d : Candidate) (ok : admits d = true) : Refined d := by
  have base : admitsNested d.nested = true := by
    simp only [admits, Bool.and_eq_true] at ok
    exact ok.1.1
  have pointee : pointeeAdmitted d = true := by
    simp only [admits, Bool.and_eq_true] at ok
    exact ok.2
  have neutral : nestedContractNeutral d.nested.contract = true := by
    simp only [admitsNested, Bool.and_eq_true] at base
    exact base.1.2
  simp only [nestedContractNeutral, Bool.and_eq_true, beq_iff_eq, List.all_eq_true] at neutral
  refine ⟨neutral.1, ?_, ?_, pointee⟩
  · intro m mem
    simpa using neutral.2 m mem
  · simp only [admitsNested, Bool.and_eq_true] at base
    exact base.1.1.2

/-- **Card W4-EXPR-POS's sentence genuinely does not cover this**, stated rather
    than stretched: an admitted dereference of a call result reads through a
    pointer-valued operand. The earlier proof is therefore not weakened here —
    it is a different family with a different obligation. -/
theorem the_earlier_sentence_does_not_cover_this (d : Candidate)
    (ok : admits d = true) (deref : dereferences d.wrapper = true) :
    d.pointee = Pointee.scalar ∧ dereferences d.wrapper = true := by
  refine ⟨?_, deref⟩
  have pointee : pointeeAdmitted d = true := by
    simp only [admits, Bool.and_eq_true] at ok
    exact ok.2
  simp only [pointeeAdmitted, deref, Bool.not_true, Bool.false_or, beq_iff_eq] at pointee
  exact pointee

/-- The composition argument the wrapper inherits unchanged: an admitted nested
    call's declared effects are the identity, so the unspecified evaluation
    order of the enclosing call's arguments (C11 6.5.2.2p10) is unobservable. -/
theorem an_admitted_candidate_is_neutral (d : Candidate) (args : List (Option Nat))
    (ok : admits d = true) : isNeutral d.nested.contract args := by
  have base : admitsNested d.nested = true := by
    simp only [admits, Bool.and_eq_true] at ok
    exact ok.1.1
  exact an_admitted_nested_call_is_neutral d.nested.contract args
    (an_admitted_nested_contract_is_neutral d.nested base)

/-! ## The subject, and non-vacuity -/

/-- The production shape: `flb_errno_print ( ( * __errno_location ( ) ) , … )`.
    `__errno_location` takes no parameter, returns `int *`, and its declared
    contract is `resource_return_none()`; the outer position is a declared
    `borrowed` parameter. -/
def errnoNested : Nested :=
  { declarationsRead := true
    oneLevel := true
    wholeArgument := true
    innerArgumentsCallFree := true
    calleeContracted := true
    outerContracted := true
    contract := { modes := [], result := ResultMode.none, declaredTail := false }
    slot := Slot.borrowedParameter
    outerDeclaredTail := false
    innerArityExact := true
    innerActualsDenotable := true }

def errnoCandidate : Candidate :=
  { nested := errnoNested, wrapper := Wrapper.parenStar, pointee := Pointee.scalar }

/-- **The widening is strict, and it is exactly the subject.** The same nested
    call is refused under the wrapper the base family cannot read (`other`) and
    admitted under this card's, and the base family's own bare form is admitted
    as it always was. -/
theorem the_widening_is_strict :
    admits { errnoCandidate with wrapper := Wrapper.other } = false
    ∧ admits errnoCandidate = true := by
  constructor <;> decide

/-- Non-vacuity, with each premise separately flipped on the subject. -/
theorem each_premise_is_load_bearing_on_the_subject :
    admits errnoCandidate = true
    ∧ admits { errnoCandidate with pointee := Pointee.notScalar } = false
    ∧ admits { errnoCandidate with pointee := Pointee.unread } = false
    ∧ admits { errnoCandidate with wrapper := Wrapper.other } = false
    ∧ admits { errnoCandidate with
               nested := { errnoNested with declarationsRead := false } } = false
    ∧ admits { errnoCandidate with
               nested := { errnoNested with oneLevel := false } } = false
    ∧ admits { errnoCandidate with
               nested := { errnoNested with calleeContracted := false } } = false
    ∧ admits { errnoCandidate with
               nested := { errnoNested with outerContracted := false } } = false
    ∧ admits { errnoCandidate with
               nested := { errnoNested with innerArityExact := false } } = false
    ∧ admits { errnoCandidate with
               nested := { errnoNested with innerActualsDenotable := false } } = false
    ∧ admits { errnoCandidate with
               nested := { errnoNested with slot := Slot.custodyParameter } } = false
    ∧ admits { errnoCandidate with
               nested := { errnoNested with
                           contract := { modes := [], result := ResultMode.owned
                                         declaredTail := false } } } = false := by
  refine ⟨by decide, by decide, by decide, by decide, by decide, by decide, by decide,
          by decide, by decide, by decide, by decide, by decide⟩

/-- The bare spelling of the same nested call is admitted for every pointee,
    including the one N10 refuses under a dereference: the widening added a
    premise, it did not move one. -/
theorem the_bare_subject_is_unaffected_by_the_new_premise :
    admits { errnoCandidate with wrapper := Wrapper.bare } = true
    ∧ admits { nested := errnoNested, wrapper := Wrapper.bare
               pointee := Pointee.notScalar } = true
    ∧ admits { nested := errnoNested, wrapper := Wrapper.bare
               pointee := Pointee.unread } = true := by
  refine ⟨by decide, by decide, by decide⟩

/-! ## The borrow-liveness repair

Card BORROW-LIVENESS refuses a `borrowed` actual the projection's own bindings
show is a definitely released identity. Its pre-pass read the **enclosing**
call's actuals only, so a released identity handed to an *admitted nested*
call's borrowed actual was `Safe` with no diagnostic — measured at the base tree
and repaired by this card. The repair is modeled here as a set inclusion, which
is the whole of what it changes.
-/

/-- One call site's modeled reads: the enclosing call's actuals, and the
    admitted nested call's, when there is one. -/
structure Site where
  outer : List Nat
  nested : Option (List Nat)
  /-- The identities the projection records `released` in the settled
      pre-state. Both calls are read against the same state, which is sound in
      the ordering sense because an admitted nested call changes nothing
      (`an_admitted_candidate_is_neutral`). -/
  released : List Nat
  deriving DecidableEq, Repr

/-- What card BORROW-LIVENESS refused before this card. -/
def refusedBefore (s : Site) : List Nat :=
  s.outer.filter (fun i => s.released.contains i)

/-- What it refuses after. -/
def refusedAfter (s : Site) : List Nat :=
  (s.outer ++ s.nested.getD []).filter (fun i => s.released.contains i)

/-- **Strictly more refusing**: every actual refused before is refused after. -/
theorem the_repair_only_adds_refusals (s : Site) (i : Nat) (before : i ∈ refusedBefore s) :
    i ∈ refusedAfter s := by
  simp only [refusedBefore, List.mem_filter] at before
  simp only [refusedAfter, List.mem_filter, List.mem_append]
  exact ⟨Or.inl before.1, before.2⟩

/-- **Nothing moves where there is no nested call**, so no existing site's
    answer can change: the repair is invisible to every statement card
    O04-TRACE did not already admit a nested call in. -/
theorem a_site_with_no_nested_call_is_unchanged (s : Site) (none : s.nested = Option.none) :
    refusedAfter s = refusedBefore s := by
  simp [refusedAfter, refusedBefore, none]

/-- **The measured shape**, in the model: the outer call borrows nothing the
    state released, the nested call borrows exactly what it released, and only
    the repaired reading refuses it. -/
theorem the_measured_fail_open_is_closed :
    refusedBefore { outer := [], nested := some [0], released := [0] } = []
    ∧ refusedAfter { outer := [], nested := some [0], released := [0] } = [0] := by
  constructor <;> decide

/-- And the control the base tree already refused answers the same both ways, so
    the repair did not move it. -/
theorem the_control_is_unmoved :
    refusedBefore { outer := [0], nested := Option.none, released := [0] } = [0]
    ∧ refusedAfter { outer := [0], nested := Option.none, released := [0] } = [0] := by
  constructor <;> decide

end QCore.DereferencedResult
