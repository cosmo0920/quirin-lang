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
# T6: a literal tail is data whatever the declarations say

Card **W44-LIBC-RESIDUALS** (`LIBC-PRINTF-TAIL-ACTUALS`). `QCore.VariadicNested`
models card O04-TRACE's declared variadic data tail: the declaration admits
nothing on its own, every actual beyond the named parameters must be data at
the call site (T1-T5), and a function whose typed declarations the reader
refused admits no tail at all (`unread_declarations_never_admit_a_tail`).

That premise protects the **names** a tail may carry: without the
declarations nothing says whether a name is a scalar, an enumeration constant
or a pointer the body may bind to a tracked identity. T4 protects them too
where the declarations are read: it admits a name only if C02-P's closure says
the body never binds it, and the closure marks every root name passed to an
uncontracted call. The tokenizer delivers a string literal as one
identifier-shaped token, so a literal passed to `__assert_fail` (librdkafka's
`RD_UT_ASSERT`) made every literal of the function risky. Both premises refused
tails that carry no name at all: librdkafka's `RD_UT_*` macros expand to
`fprintf(stderr, "RDUT: …", __FILE__, __LINE__, __FUNCTION__)`, after
preprocessing two literals and a predefined identifier.

This module models T6, `variadic_nested::is_literal_or_predefined_actual`:
under a declared tail, an actual that consists only of literals and predefined
identifiers (`__func__`, `__FUNCTION__`, `__PRETTY_FUNCTION__`, keywords in
Clang) is admitted, whether or not the declarations are read and whatever the
closure says; where the declarations are unread, nothing else is.

## What is proved

* **The widening only adds**; in a function whose declarations are read, a site
  with no literal-only actual is decided exactly as before, and every other
  actual still needs T1-T5.
* **The declaration is still required.**
* **Soundness, as a statement over every binding assignment**: a literal-only
  actual carries no name and no indirection, so for every assignment of "may
  bind a tracked identity" to names — whatever the declarations or the closure
  would have said — it can carry no identity. Where the declarations are read,
  every admitted actual is either such an actual or T1-T5 data.
* **What still refuses** without read declarations: any name other than the
  predefined identifiers, any punctuation (a sign, a cast, `&`, a member
  access, a call), and an empty actual; and with them, a name the closure marks.
* **Non-vacuity**: the measured `RDUT:` tail, in an unread function and in a
  read one whose closure marked the literal, was refused and is admitted; a
  user name beside it is still refused.

## What this is not

Token extraction is trust boundary 28's premise, and so is the tokenizer's
collapse of every string or character literal to one literal token. That a
predefined identifier cannot be declared is Clang's lexing of it as a keyword,
supplied from outside.
-/

namespace QCore.LiteralTailActual

open QCore.VariadicNested

/-- One token of a tail actual, as the walker sees it. -/
inductive Tok where
  /-- A string, character or numeric literal. -/
  | literal
  /-- `__func__`, `__FUNCTION__` or `__PRETTY_FUNCTION__`. -/
  | predefined
  /-- Any other identifier, by index. -/
  | name (n : Nat)
  /-- Any punctuator: a sign, a parenthesis, `&`, `*`, `.`, `-` of `->`, `,`. -/
  | punct (p : Nat)
  deriving DecidableEq, Repr

def isLiteralOrPredefined : Tok → Bool
  | .literal => true
  | .predefined => true
  | _ => false

/-- T6 for one actual: nonempty, and every token a literal or a predefined
identifier. -/
def literalOnly (a : List Tok) : Bool :=
  !a.isEmpty && a.all isLiteralOrPredefined

/-- What an actual may carry, for one assignment `bound` of "this name may be
bound to a tracked identity" — the fact the unread declarations would have
fixed. A punctuator is counted as possibly carrying one (an address, a
dereference, a member, a call), which only makes the statement below stronger. -/
def mayCarryIdentity (bound : Nat → Bool) (a : List Tok) : Bool :=
  a.any fun t =>
    match t with
    | .name n => bound n
    | .punct _ => true
    | _ => false

/-- **Soundness.** A literal-only actual carries no identity under every
declaration set. -/
theorem a_literal_actual_carries_no_identity_under_any_declarations
    (a : List Tok) (h : literalOnly a = true) (bound : Nat → Bool) :
    mayCarryIdentity bound a = false := by
  unfold literalOnly at h
  simp only [Bool.and_eq_true, List.all_eq_true] at h
  unfold mayCarryIdentity
  rw [List.any_eq_false]
  intro t ht
  have := h.2 t ht
  cases t <;> simp_all [isLiteralOrPredefined]

/-- One call site's excess actuals, with the function's declaration state and
the T1-T5 reading the reader gives each actual when the declarations are read. -/
structure Site where
  declarationsRead : Bool
  declaredTail : Bool
  excess : List (List Tok)
  read : List Tok → TailActual

/-- Card O04-TRACE's admission, as `QCore.VariadicNested` states it. -/
def admitsBefore (s : Site) : Bool :=
  admitsTail { declarationsRead := s.declarationsRead, declaredTail := s.declaredTail,
               excess := s.excess.map s.read }

/-- With T6. -/
def admitsNow (s : Site) : Bool :=
  s.declaredTail &&
    (if s.declarationsRead then s.excess.all (fun a => literalOnly a || tailActualIsData (s.read a))
     else s.excess.all literalOnly)

/-- With read declarations, an actual is admitted exactly when it is literal-only
or T1-T5 data, as before. -/
theorem a_read_site_admits_literal_or_data (s : Site) (h : s.declarationsRead = true) :
    admitsNow s = (s.declaredTail &&
      s.excess.all (fun a => literalOnly a || tailActualIsData (s.read a))) := by
  simp [admitsNow, h]

theorem all_without_literal_actuals (l : List (List Tok)) (g : List Tok → Bool)
    (nl : ∀ a ∈ l, literalOnly a = false) :
    l.all (fun a => literalOnly a || g a) = l.all g := by
  induction l with
  | nil => rfl
  | cons x xs ih =>
    simp only [List.all_cons]
    rw [nl x (List.mem_cons_self ..), ih (fun a ha => nl a (List.mem_cons_of_mem _ ha))]
    simp

/-- A read site none of whose actuals is literal-only is decided exactly as
before. -/
theorem a_read_site_without_literal_actuals_is_unchanged (s : Site)
    (h : s.declarationsRead = true) (nl : ∀ a ∈ s.excess, literalOnly a = false) :
    admitsNow s = admitsBefore s := by
  simp only [admitsNow, admitsBefore, admitsTail, h, ↓reduceIte, Bool.true_and]
  rw [all_without_literal_actuals s.excess (fun a => tailActualIsData (s.read a)) nl,
    List.all_map]
  rfl

/-- The widening only adds. -/
theorem the_widening_only_adds (s : Site) (h : admitsBefore s = true) :
    admitsNow s = true := by
  cases hr : s.declarationsRead
  · simp [admitsBefore, admitsTail, hr] at h
  · simp only [admitsBefore, admitsTail, hr, Bool.true_and, Bool.and_eq_true,
      List.all_eq_true, List.mem_map] at h
    simp only [admitsNow, hr, ↓reduceIte, Bool.and_eq_true, List.all_eq_true, Bool.or_eq_true]
    exact ⟨h.1, fun a ha => Or.inr (h.2 (s.read a) ⟨a, ha, rfl⟩)⟩

/-- The declaration is still required. -/
theorem the_declaration_is_still_required (s : Site) (h : s.declaredTail = false) :
    admitsNow s = false := by
  simp [admitsNow, h]

/-- **Soundness where the declarations are read.** Every admitted actual is
either literal-only — carrying no identity under every binding assignment — or
T1-T5 data, as card O04-TRACE's admission already required. -/
theorem every_actual_admitted_read_is_literal_or_data (s : Site)
    (ok : admitsNow s = true) (read : s.declarationsRead = true)
    (a : List Tok) (ha : a ∈ s.excess) :
    (∀ bound, mayCarryIdentity bound a = false) ∨ tailActualIsData (s.read a) = true := by
  simp only [admitsNow, read, ↓reduceIte, Bool.and_eq_true, List.all_eq_true,
    Bool.or_eq_true] at ok
  rcases ok.2 a ha with hl | hd
  · exact Or.inl (a_literal_actual_carries_no_identity_under_any_declarations a hl)
  · exact Or.inr hd

/-- Without read declarations, only literal-only actuals are admitted. -/
theorem an_unread_site_admits_only_literal_actuals (s : Site)
    (ok : admitsNow s = true) (unread : s.declarationsRead = false)
    (a : List Tok) (ha : a ∈ s.excess) : literalOnly a = true := by
  simp only [admitsNow, unread, Bool.false_eq_true, ↓reduceIte, Bool.and_eq_true,
    List.all_eq_true] at ok
  exact ok.2 a ha

/-- **The site-level soundness statement.** Every actual admitted without read
declarations carries no identity under every assignment those declarations
could have made. -/
theorem every_actual_admitted_unread_carries_no_identity (s : Site)
    (ok : admitsNow s = true) (unread : s.declarationsRead = false)
    (a : List Tok) (ha : a ∈ s.excess) (bound : Nat → Bool) :
    mayCarryIdentity bound a = false :=
  a_literal_actual_carries_no_identity_under_any_declarations a
    (an_unread_site_admits_only_literal_actuals s ok unread a ha) bound

/-- A name still refuses without read declarations, whatever it is. -/
theorem an_unread_name_still_refuses (n : Nat) : literalOnly [.name n] = false := by
  rfl

/-- So does any punctuator: a negated literal, a cast, an address. -/
theorem an_unread_punctuator_still_refuses (p : Nat) (rest : List Tok) :
    literalOnly (.punct p :: rest) = false := by
  simp [literalOnly, isLiteralOrPredefined]

theorem an_empty_actual_still_refuses : literalOnly [] = false := by rfl

/-! ## Non-vacuity, with the measured `RDUT:` shape -/

/-- Any T1-T5 reading; the unread site never consults it. -/
def someReading (_ : List Tok) : TailActual :=
  { nonempty := true, carriesIndirection := false, carriesUnadmittedCall := false,
    everyNameIsData := true }

/-- `fprintf(stderr, "RDUT: …", __FILE__, __LINE__, __FUNCTION__)` after
preprocessing, in a function whose declarations the reader refused. -/
def rdutSite : Site :=
  { declarationsRead := false, declaredTail := true,
    excess := [[.literal], [.literal], [.predefined]], read := someReading }

theorem the_rdut_tail_was_refused_and_is_admitted :
    admitsBefore rdutSite = false ∧ admitsNow rdutSite = true := by
  decide

/-- The closure half: a read function whose closure marked the literal token
risky (it was passed to `__assert_fail`), so T4 read the literal as a name that
may be bound. -/
def riskyReading (_ : List Tok) : TailActual :=
  { nonempty := true, carriesIndirection := false, carriesUnadmittedCall := false,
    everyNameIsData := false }

def assertingSite : Site :=
  { declarationsRead := true, declaredTail := true,
    excess := [[.literal], [.literal], [.predefined]], read := riskyReading }

theorem the_closure_risky_literal_tail_was_refused_and_is_admitted :
    admitsBefore assertingSite = false ∧ admitsNow assertingSite = true := by
  decide

/-- A name the same closure marked risky keeps its refusal. -/
theorem a_risky_name_beside_it_is_still_refused :
    admitsNow { assertingSite with excess := [[.literal], [.name 0]] } = false := by
  decide

/-- The user's own `fprintf(stderr, fmt, x)` next to it keeps its refusal. -/
theorem a_user_tail_beside_it_is_still_refused :
    admitsNow { rdutSite with excess := [[.literal], [.name 0]] } = false := by
  decide

end QCore.LiteralTailActual
