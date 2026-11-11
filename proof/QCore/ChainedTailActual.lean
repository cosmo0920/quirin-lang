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

import QCore.ChainedAccess
import QCore.VariadicNested

/-!
# A bounded access chain at a declared variadic data tail (NEIGHBOURS-CHAINED-TAIL)

`QCore.VariadicNested` models card O04-TRACE's tail walker, whose premise **T2**
refuses every tail actual carrying `&`, `*`, `[`, `]`, `.` or `->`. Card
NEIGHBOURS measured what that costs on the byte-verbatim upstream
`flb_input_chunk_create`: one surviving refusal, an `indirect memory access` on
a tail actual `storage->stream->name`, which is also the pinned corpus's
`flb_input_chunk.c` L19581.

This module models the clause `crates/qcore-c-gateway/src/variadic_nested.rs`
adds, **T5**: a tail actual that is *exactly* one admitted bounded typed access
chain — `QCore.ChainedAccess.admitsChain`, unchanged — is data.

## What is and is not assumed

The chain half is **the same predicate over the same declaration facts** that
`QCore.ChainedAccess` already proves decides a declared `Borrowed` actual, and
`the_chain_half_is_exactly_the_borrowed_admission_at_depth_one` re-states that
here. What the tail position contributes is the *declaration*:
`resource_variadic_data_tail()` says the callee takes no custody of, and retains
no pointer to, any argument beyond the ones it names — which for custody
purposes is at least as strong as the declared `borrowed` mode the very same
read is already admitted at. So no new assumption about a callee is introduced,
and `the_declaration_is_read_from_the_callee_never_from_the_actual` states the
fail-closed direction: no actual can buy itself a tail.

Two directions are proved, and both matter.

* **Widening, stated as such.** Everything T1-T4 admitted is still admitted, and
  the widening is strict — a witness admitted only by T5 is exhibited.
* **Fail-closed.** Every `admitsChain` refusal lifts to the whole call site: a
  refused chain, with no data reading of its own, refuses the site, the site's
  statement, and can never be `safe`. Refusal is never an accusation.

This is a model of the *decision*. That the reader extracts these declaration
facts faithfully is its own obligation, assumed here exactly as in
`QCore.TypedFieldAccess`; the recorded may-alias boundary (**C02-P-ALIAS**) is
inherited unchanged and nothing here widens it.
-/

namespace QCore.ChainedTailActual

open QCore.LegacyCAnalyzer
open QCore.TypedFieldAccess
open QCore.ChainedAccess
open QCore.VariadicNested
open QCore.ExitResiduals

/-- One actual beyond the named parameter vector, as the widened walker reads
    it: card O04-TRACE's T1-T4 reading, plus the chain the actual *is* when the
    whole actual is one. -/
structure TailArgument where
  /-- The T1-T4 reading, unchanged. -/
  base : TailActual
  /-- `some c` exactly when the whole actual is the access chain `c`. -/
  chain : Option Chain
  deriving DecidableEq

/-- T5: the actual is data when the old reading says so, **or** when it is
    exactly one admitted chain. A disjunction, so the rule can only admit more. -/
def argumentIsData (a : TailArgument) : Bool :=
  tailActualIsData a.base
    || (match a.chain with
        | some c => admitsChain c
        | none => false)

/-- One call site's excess actuals, with the callee's declaration. -/
structure Site where
  /-- Every declaration of the enclosing function was read. -/
  declarationsRead : Bool
  /-- `resource_variadic_data_tail()` was declared for this callee. -/
  declaredTail : Bool
  /-- The actuals beyond the declared parameter vector. -/
  excess : List TailArgument
  deriving DecidableEq

/-- The widened admission. Both halves are still required: the declaration,
    **and** every excess actual being data at this call site. -/
def admits (s : Site) : Bool :=
  s.declarationsRead && s.declaredTail && s.excess.all argumentIsData

/-! ## The declaration half is untouched -/

/-- **No actual can buy itself a tail.** With no declared tail the site refuses,
    whatever chains its actuals are — the arity repair card O04-TRACE landed,
    unchanged by this widening. -/
theorem the_declaration_is_read_from_the_callee_never_from_the_actual
    (r : Bool) (excess : List TailArgument) :
    admits { declarationsRead := r, declaredTail := false, excess := excess } = false := by
  simp [admits]

theorem unread_declarations_never_admit (s : Site) (unread : s.declarationsRead = false) :
    admits s = false := by
  simp [admits, unread]

/-! ## Widening, stated as such -/

/-- Everything card O04-TRACE's tail walker admitted is still admitted: T5 is a
    disjunct, never a replacement. -/
theorem every_previously_admitted_actual_is_still_data (a : TailArgument)
    (old : tailActualIsData a.base = true) : argumentIsData a = true := by
  simp [argumentIsData, old]

/-- Lifted to the whole site, over the old model's own admission. -/
theorem every_previously_admitted_site_is_still_admitted
    (r t : Bool) (olds : List TailActual)
    (ok : admitsTail { declarationsRead := r, declaredTail := t, excess := olds } = true) :
    admits { declarationsRead := r, declaredTail := t
             excess := olds.map (fun o => { base := o, chain := none }) } = true := by
  simp only [admitsTail, Bool.and_eq_true] at ok
  obtain ⟨⟨hr, ht⟩, hall⟩ := ok
  subst hr
  subst ht
  simp only [admits, Bool.true_and, List.all_eq_true]
  intro a mem
  simp only [List.mem_map] at mem
  obtain ⟨o, memo, rfl⟩ := mem
  simp [argumentIsData, List.all_eq_true.mp hall o memo]

/-- A fully typed two-link chain whose final member is pointer-typed: the shape
    card NEIGHBOURS isolated, `storage->stream->name`. -/
def subjectChain : Chain :=
  { baseMatches := true
    baseBound := false
    declarationsRead := true
    continuesAfterLast := false
    links :=
      [ { record := ⟨TagKind.recordTag, true, true⟩
          member := MemberType.pointer, qualified := false, continuable := true }
      , { record := ⟨TagKind.recordTag, true, true⟩
          member := MemberType.pointer, qualified := false, continuable := false } ] }

/-- A T1-T4 reading that refuses: the actual carries a member access. -/
def carriesAccess : TailActual :=
  { nonempty := true, carriesIndirection := true
    carriesUnadmittedCall := false, everyNameIsData := true }

/-- **The widening is strict, and it is exactly the subject.** The actual card
    NEIGHBOURS measured is refused by T1-T4 and admitted by T5. -/
theorem the_widening_is_strict :
    tailActualIsData carriesAccess = false
    ∧ argumentIsData { base := carriesAccess, chain := some subjectChain } = true := by
  constructor <;> decide

/-- And the site it sits in is admitted, where the un-widened model refused it. -/
theorem the_subject_site_is_admitted :
    admitsTail { declarationsRead := true, declaredTail := true
                 excess := [carriesAccess] } = false
    ∧ admits { declarationsRead := true, declaredTail := true
               excess := [{ base := carriesAccess, chain := some subjectChain }] } = true := by
  constructor <;> decide

/-- **The correction card NEIGHBOURS-CHAINED-TAIL measured**, in the model: the
    widening is not about a pointer-typed final member. A depth-one chain whose
    single member is a **scalar** — squarely inside card C02's original
    whitelist — was refused in a tail position by exactly the same T2 clause, and
    is admitted by T5 by exactly the same disjunct. -/
theorem a_depth_one_scalar_chain_is_widened_the_same_way :
    argumentIsData
      { base := carriesAccess
        chain := some { baseMatches := true, baseBound := false
                        declarationsRead := true, continuesAfterLast := false
                        links := [ { record := ⟨TagKind.recordTag, true, true⟩
                                     member := MemberType.scalar, qualified := false
                                     continuable := false } ] } } = true := by
  decide

/-! ## Fail-closed: every chain refusal lifts to the site -/

/-- The workhorse. An excess actual that neither reads as data nor carries an
    admitted chain refuses the whole site. -/
theorem a_refused_actual_refuses_the_site (s : Site) (a : TailArgument)
    (present : a ∈ s.excess) (refused : argumentIsData a = false) : admits s = false := by
  have : s.excess.all argumentIsData = false :=
    List.all_eq_false.mpr ⟨a, present, by simp [refused]⟩
  simp [admits, this]

/-- A chained actual whose chain the whitelist refuses, and whose T1-T4 reading
    refuses too, refuses the whole site. Every `QCore.ChainedAccess` refusal
    below is an instance of this. -/
theorem a_refused_chain_refuses_the_site (s : Site) (a : TailArgument) (c : Chain)
    (present : a ∈ s.excess) (isChain : a.chain = some c)
    (notData : tailActualIsData a.base = false) (refused : admitsChain c = false) :
    admits s = false :=
  a_refused_actual_refuses_the_site s a present (by simp [argumentIsData, isChain, notData, refused])

/-- **Premise (4), the load-bearing one.** A base this body may bind to a tracked
    resource identity refuses the chain, and therefore the site — at any depth.
    This is what refuses a released base, an acquired aggregate and every
    protected formal, and it is inherited from card C02-P unchanged. -/
theorem a_bound_base_refuses_the_site (s : Site) (a : TailArgument) (c : Chain)
    (present : a ∈ s.excess) (isChain : a.chain = some c)
    (notData : tailActualIsData a.base = false) (bound : c.baseBound = true) :
    admits s = false :=
  a_refused_chain_refuses_the_site s a c present isChain notData
    (a_bound_root_never_admits_a_chain c bound)

theorem a_continued_access_refuses_the_site (s : Site) (a : TailArgument) (c : Chain)
    (present : a ∈ s.excess) (isChain : a.chain = some c)
    (notData : tailActualIsData a.base = false) (continues : c.continuesAfterLast = true) :
    admits s = false :=
  a_refused_chain_refuses_the_site s a c present isChain notData
    (a_continued_access_never_admits c continues)

/-- The depth bound is card O04-ARMS' own, inherited: a chain past it is refused,
    never truncated to an admitted prefix. -/
theorem a_chain_beyond_the_bound_refuses_the_site (s : Site) (a : TailArgument) (c : Chain)
    (present : a ∈ s.excess) (isChain : a.chain = some c)
    (notData : tailActualIsData a.base = false) (long : c.links.length > limit) :
    admits s = false :=
  a_refused_chain_refuses_the_site s a c present isChain notData
    (a_chain_beyond_the_bound_never_admits c long)

theorem a_union_link_refuses_the_site (s : Site) (a : TailArgument) (c : Chain) (l : Link)
    (present : a ∈ s.excess) (isChain : a.chain = some c)
    (notData : tailActualIsData a.base = false) (mem : l ∈ c.links)
    (union : l.record.kind = TagKind.unionTag) : admits s = false :=
  a_refused_chain_refuses_the_site s a c present isChain notData
    (a_union_link_never_admits c l mem union)

theorem an_incomplete_link_refuses_the_site (s : Site) (a : TailArgument) (c : Chain) (l : Link)
    (present : a ∈ s.excess) (isChain : a.chain = some c)
    (notData : tailActualIsData a.base = false) (mem : l ∈ c.links)
    (incomplete : l.record.complete = false) : admits s = false :=
  a_refused_chain_refuses_the_site s a c present isChain notData
    (an_incomplete_link_never_admits c l mem incomplete)

theorem an_ambiguous_link_refuses_the_site (s : Site) (a : TailArgument) (c : Chain) (l : Link)
    (present : a ∈ s.excess) (isChain : a.chain = some c)
    (notData : tailActualIsData a.base = false) (mem : l ∈ c.links)
    (ambiguous : l.record.unique = false) : admits s = false :=
  a_refused_chain_refuses_the_site s a c present isChain notData
    (an_ambiguous_link_never_admits c l mem ambiguous)

theorem a_qualified_link_refuses_the_site (s : Site) (a : TailArgument) (c : Chain) (l : Link)
    (present : a ∈ s.excess) (isChain : a.chain = some c)
    (notData : tailActualIsData a.base = false) (mem : l ∈ c.links)
    (qualified : l.qualified = true) : admits s = false :=
  a_refused_chain_refuses_the_site s a c present isChain notData
    (a_qualified_link_never_admits c l mem qualified)

theorem an_aggregate_link_refuses_the_site (s : Site) (a : TailArgument) (c : Chain) (l : Link)
    (present : a ∈ s.excess) (isChain : a.chain = some c)
    (notData : tailActualIsData a.base = false) (mem : l ∈ c.links)
    (aggregate : l.member = MemberType.aggregate) : admits s = false :=
  a_refused_chain_refuses_the_site s a c present isChain notData
    (an_aggregate_link_never_admits c l mem aggregate)

/-- An intermediate link the reader did not type as exactly `struct POINTEE *` —
    a `union` pointer, a double pointer, a typedef it will not resolve — refuses
    the site. -/
theorem a_non_continuable_intermediate_refuses_the_site (s : Site) (a : TailArgument)
    (c : Chain) (l : Link) (rest : List Link)
    (present : a ∈ s.excess) (isChain : a.chain = some c)
    (notData : tailActualIsData a.base = false)
    (shape : c.links = l :: rest) (nonEmpty : rest ≠ []) (notTyped : l.continuable = false) :
    admits s = false :=
  a_refused_chain_refuses_the_site s a c present isChain notData
    (a_non_continuable_intermediate_never_admits c l rest shape nonEmpty notTyped)

/-- An actual that is **not** a whole chain gets no chain half at all, so T2
    governs it unchanged: an address, a dereference, a subscript, a partial chain
    and a chain mixed with arithmetic all still refuse. -/
theorem a_partial_chain_keeps_its_refusal (s : Site) (a : TailArgument)
    (present : a ∈ s.excess) (notChain : a.chain = none)
    (indirect : a.base.carriesIndirection = true) : admits s = false :=
  a_refused_actual_refuses_the_site s a present
    (by simp [argumentIsData, notChain, tailActualIsData, indirect])

/-- The declaration covers what it covers: a tail actual naming something this
    body may bind to a tracked identity is refused **under** the declaration,
    exactly as it was without one, whenever it is not a whole admitted chain. -/
theorem a_risky_name_still_refuses_under_the_declaration (s : Site) (a : TailArgument)
    (present : a ∈ s.excess) (notChain : a.chain = none)
    (risky : a.base.everyNameIsData = false) : admits s = false :=
  a_refused_actual_refuses_the_site s a present
    (by simp [argumentIsData, notChain, tailActualIsData, risky])

/-- The contrapositive: every actual an admitted site carries is data. -/
theorem every_admitted_actual_is_data (s : Site) (ok : admits s = true)
    (a : TailArgument) (present : a ∈ s.excess) : argumentIsData a = true := by
  cases refused : argumentIsData a with
  | false =>
    have : s.excess.all argumentIsData = false :=
      List.all_eq_false.mpr ⟨a, present, by simp [refused]⟩
    simp [admits, this] at ok
  | true => rfl

/-- **The soundness sentence, in the model.** Every actual an admitted site
    carries that is a chain rather than plain data names a base this body can
    **never** bind to a tracked resource identity, and does not continue past its
    last selection. So the value it reads is denoted by no place this projection
    tracks. -/
theorem an_admitted_chain_names_no_bound_base (s : Site) (ok : admits s = true)
    (a : TailArgument) (c : Chain) (present : a ∈ s.excess) (isChain : a.chain = some c)
    (notData : tailActualIsData a.base = false) :
    c.baseBound = false ∧ c.continuesAfterLast = false := by
  have data := every_admitted_actual_is_data s ok a present
  rw [argumentIsData, isChain, notData] at data
  simp only [Bool.false_or] at data
  refine ⟨?_, ?_⟩
  · cases bound : c.baseBound with
    | false => rfl
    | true =>
      rw [a_bound_root_never_admits_a_chain c bound] at data
      exact Bool.noConfusion data
  · cases continues : c.continuesAfterLast with
    | false => rfl
    | true =>
      rw [a_continued_access_never_admits c continues] at data
      exact Bool.noConfusion data

/-! ## Extension, not replacement: the chain half is the borrowed half -/

/-- **The chain half is exactly the declared-`Borrowed` admission.** At depth one
    the tail rule's chain half decides precisely what card C02-P's
    borrowed-actual whitelist decides at a declared `borrowed` position, so the
    read premises are identical and only the *position* differs — and the
    position is decided by the callee's declaration. -/
theorem the_chain_half_is_exactly_the_borrowed_admission_at_depth_one
    (c : Chain) (l : Link) (single : c.links = [l]) :
    admitsChain c = admitsActual (chainedActual c l) :=
  a_depth_one_chain_is_the_direct_admission c l single

/-! ## Bridging into the statement classification -/

/-- A refused site refuses its statement: the walker claims no span, so the
    lexical indirect-access gate keeps every token of every unadmitted actual in
    view. -/
def classifySite (s : Site) (otherIndirectAccess : Bool) : Classification :=
  if otherIndirectAccess || !admits s then .refused else .modeledCall

theorem a_refused_site_refuses_the_statement (s : Site) (other : Bool)
    (refused : admits s = false) : classifySite s other = .refused := by
  simp [classifySite, refused]

/-- Fail-closed, the direction that matters: a refused site is a reason to say
    nothing, never a reason to accuse. Its statement contributes
    `Instruction.unsupported`, which can never be classified `safe`. -/
theorem a_refused_site_is_never_safe (s : Site) (state : AbstractState)
    (_refused : admits s = false) :
    classify (transfer state .unsupported) ≠ .safe :=
  unsupported_is_never_safe state

/-- **The admission removes only the spans it justified.** Any other indirect
    access in the same statement still refuses it, however many tail chains were
    admitted — which is why exempting an admitted tail span is not admitting the
    call. -/
theorem an_admitted_chain_does_not_rescue_the_statement (s : Site) :
    classifySite s true = .refused := by
  simp [classifySite]

/-- And one refused sibling refuses the whole site, so an admitted chain beside a
    tail actual that may hold a tracked identity buys nothing. -/
theorem an_admitted_chain_beside_a_risky_sibling_still_refuses
    (good : TailArgument) (bad : TailActual) (risky : bad.everyNameIsData = false) :
    admits { declarationsRead := true, declaredTail := true
             excess := [good, { base := bad, chain := none }] } = false :=
  a_risky_name_still_refuses_under_the_declaration _ { base := bad, chain := none }
    (by simp) rfl risky

/-! ## Non-vacuity, in both directions -/

/-- The rule separates: the subject site is admitted, and the same site with the
    base marked bindable, with the chain continued, or with one `union` link is
    refused. Each premise is flipped on the same witness. -/
theorem each_premise_is_load_bearing_on_the_subject :
    admits { declarationsRead := true, declaredTail := true
             excess := [{ base := carriesAccess, chain := some subjectChain }] } = true
    ∧ admits { declarationsRead := true, declaredTail := true
               excess := [{ base := carriesAccess
                            chain := some { subjectChain with baseBound := true } }] } = false
    ∧ admits { declarationsRead := true, declaredTail := true
               excess := [{ base := carriesAccess
                            chain := some { subjectChain with
                                            continuesAfterLast := true } }] } = false
    ∧ admits { declarationsRead := true, declaredTail := false
               excess := [{ base := carriesAccess, chain := some subjectChain }] } = false
    ∧ admits { declarationsRead := false, declaredTail := true
               excess := [{ base := carriesAccess, chain := some subjectChain }] } = false := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩ <;> decide

end QCore.ChainedTailActual
