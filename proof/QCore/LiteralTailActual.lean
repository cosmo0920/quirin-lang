import QCore.VariadicNested

/-!
# T6: a literal tail is data whatever the declarations say

Card **W44-LIBC-RESIDUALS** (`LIBC-PRINTF-TAIL-ACTUALS`). `QCore.VariadicNested`
models card O04-TRACE's declared variadic data tail: the declaration admits
nothing on its own, every actual beyond the named parameters must be data at
the call site (T1-T5), and a function whose typed declarations the reader
refused admits no tail at all (`unread_declarations_never_admit_a_tail`).

That last premise protects the **names** a tail may carry: without the
declarations nothing says whether a name is a scalar, an enumeration constant
or a pointer the body may bind to a tracked identity. It also refused tails that
carry no name. librdkafka's `RD_UT_*` unit-test macros expand to
`fprintf(stderr, "RDUT: …", __FILE__, __LINE__, __FUNCTION__)`: after
preprocessing, two literals and a predefined identifier. Up to 1,091 of the
1,200 tail refusals at the card's base are of that kind.

This module models the one readability-independent admission,
`variadic_nested::is_literal_or_predefined_actual`: under a declared tail, when
the declarations are unread, a tail whose every actual consists only of
literals and predefined identifiers (`__func__`, `__FUNCTION__`,
`__PRETTY_FUNCTION__`, keywords in Clang) is admitted.

## What is proved

* **The widening only adds**, and a function whose declarations are read is
  decided exactly as before.
* **The declaration is still required.**
* **Soundness, as a statement over every declaration set**: an actual admitted
  without read declarations carries no name and no indirection, so for every
  assignment of "may bind a tracked identity" to names — whatever the unread
  declarations would have said — it can carry no identity.
* **What still refuses** without read declarations: any name other than the
  predefined identifiers, any punctuation (a sign, a cast, `&`, a member
  access, a call), and an empty actual.
* **Non-vacuity**: the measured `RDUT:` tail was refused and is admitted; a
  tail with a user name next to it is still refused.

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
    (if s.declarationsRead then (s.excess.map s.read).all tailActualIsData
     else s.excess.all literalOnly)

/-- A function whose declarations are read is decided exactly as before. -/
theorem a_read_site_is_unchanged (s : Site) (h : s.declarationsRead = true) :
    admitsNow s = admitsBefore s := by
  simp [admitsNow, admitsBefore, admitsTail, h]

/-- The widening only adds. -/
theorem the_widening_only_adds (s : Site) (h : admitsBefore s = true) :
    admitsNow s = true := by
  cases hr : s.declarationsRead
  · simp [admitsBefore, admitsTail, hr] at h
  · rw [a_read_site_is_unchanged s hr]; exact h

/-- The declaration is still required. -/
theorem the_declaration_is_still_required (s : Site) (h : s.declaredTail = false) :
    admitsNow s = false := by
  simp [admitsNow, h]

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

/-- The user's own `fprintf(stderr, fmt, x)` next to it keeps its refusal. -/
theorem a_user_tail_beside_it_is_still_refused :
    admitsNow { rdutSite with excess := [[.literal], [.name 0]] } = false := by
  decide

end QCore.LiteralTailActual
