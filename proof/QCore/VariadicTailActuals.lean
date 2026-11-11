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

import QCore.Variadics

/-!
# A trailing actual read from Clang's node (card CG26-TAIL-RESIDUAL-ACTUALS)

Gateway rule 12g, `crates/qcore-c-gateway/src/variadic_tail_actuals.rs`. Rule
11w reads a trailing actual of a declared variadic tail from the statement's
tokens; where the lexical declaration reader refuses the enclosing function it
answers nothing, and every trailing actual but a literal stays refused. Rule
12g reads the same actual from the call's Clang argument node: one recursion
(`classify`) into data, one identity-carrying name, or a named refusal, and
`read`, which turns an identity into a `borrowed` place only under a declared
borrowed tail.

What a name carries is a premise (`Facts.carries`): the Rust adapter answers
it from the projection's integer-carried identities (an arithmetic object)
and from rule 10l's served never-bound premise (a pointer or array object);
`Facts.answered` is rule 10l's "declared exactly once" premise. Clang's node
and its type are trust boundary 28. Parentheses and implicit conversions are
transparent in the Rust reader and absent here.

Proved:

* `data_iff_nothing_carried`: over the arithmetic fragment (no cast, no other
  node, no pointer-valued operator), an actual is data exactly when no name in
  it carries an identity;
* `data_carries_nothing`: every actual read as data names no carrying name,
  whatever its shape;
* `borrowed_is_the_whole_root`: a `borrowed` reading is exactly one carrying,
  answered pointer name under a borrowed tail;
* `the_data_tail_is_never_widened`: under the data tail no actual is read
  `borrowed`, and every admitted one is data;
* `a_cast_is_never_read`, `pointer_arithmetic_is_never_read`: an actual
  containing a cast, or a pointer-valued operator, is never data or borrowed;
* `an_identity_arm_is_refused`, `an_unanswered_root_is_refused`;
* `the_reading_is_custody_neutral`: the readings, as the modeled tail of a
  call whose tail positions are all `borrowed`, change no outcome
  (`QCore.Variadics.borrowed_tail_is_the_identity`);
* `read_vectors_hold`: the Rust `READ_VECTORS`, row by row.
-/

namespace QCore.VariadicTailActuals

/-- The abstract shape of one trailing actual. -/
inductive Expr where
  /-- A literal, a predefined identifier, an enumeration constant, a
      constant-size `sizeof`. -/
  | lit
  /-- An object; `scalar` for an arithmetic or enumeration type. -/
  | name (n : Nat) (scalar : Bool)
  /-- `-`, `+`, `~`, `!`. -/
  | unary (e : Expr)
  /-- An arithmetic, bitwise, shift, comparison or logical operator;
      `pointer` when its value is a pointer. -/
  | binary (pointer : Bool) (a b : Expr)
  | cond (c a b : Expr)
  /-- An explicit cast: the card CAST-FORMALIZATION's. -/
  | cast (e : Expr)
  | other
  deriving DecidableEq, Repr

inductive Refusal where
  | cast
  | pointerArithmetic
  | identityArm
  | identityOperand
  | identityUnderDataTail
  | integerIdentity
  | unansweredRoot
  | unread
  deriving DecidableEq, Repr

inductive Class where
  | data
  | identity (n : Nat) (scalar : Bool)
  | refused (r : Refusal)
  deriving DecidableEq, Repr

inductive Reading where
  | data
  | borrowed (n : Nat)
  | refused (r : Refusal)
  deriving DecidableEq, Repr

/-- The premises the adapter supplies per name. -/
structure Facts where
  carries : Nat → Bool → Bool
  answered : Nat → Bool

def classify (f : Facts) : Expr → Class
  | .lit => .data
  | .name n s => if f.carries n s then .identity n s else .data
  | .unary e =>
    match classify f e with
    | .data => .data
    | .identity _ _ => .refused .identityOperand
    | .refused r => .refused r
  | .binary p a b =>
    match classify f a, classify f b with
    | .refused r, _ => .refused r
    | _, .refused r => .refused r
    | .data, .data => if p then .refused .pointerArithmetic else .data
    | _, _ => if p then .refused .pointerArithmetic else .refused .identityOperand
  | .cond c a b =>
    match classify f c, classify f a, classify f b with
    | .refused r, _, _ => .refused r
    | _, .refused r, _ => .refused r
    | _, _, .refused r => .refused r
    | .identity _ _, _, _ => .refused .identityOperand
    | .data, .data, .data => .data
    | _, _, _ => .refused .identityArm
  | .cast _ => .refused .cast
  | .other => .refused .unread

def read (f : Facts) (borrowedTail : Bool) (e : Expr) : Reading :=
  match classify f e with
  | .data => .data
  | .refused r => .refused r
  | .identity n s =>
    if s then .refused .integerIdentity
    else if !borrowedTail then .refused .identityUnderDataTail
    else if f.answered n then .borrowed n
    else .refused .unansweredRoot

/-- Every name of an actual, with its scalar flag. -/
def names : Expr → List (Nat × Bool)
  | .lit => []
  | .name n s => [(n, s)]
  | .unary e => names e
  | .binary _ a b => names a ++ names b
  | .cond c a b => names c ++ names a ++ names b
  | .cast e => names e
  | .other => []

/-- No name of the actual carries an identity. -/
def nothingCarried (f : Facts) (e : Expr) : Prop :=
  ∀ p ∈ names e, f.carries p.1 p.2 = false

/-- The arithmetic fragment: no cast, no other node, no pointer-valued
    operator. -/
def arithmetic : Expr → Bool
  | .lit => true
  | .name _ _ => true
  | .unary e => arithmetic e
  | .binary p a b => !p && arithmetic a && arithmetic b
  | .cond c a b => arithmetic c && arithmetic a && arithmetic b
  | .cast _ => false
  | .other => false

def hasCast : Expr → Bool
  | .lit => false
  | .name _ _ => false
  | .unary e => hasCast e
  | .binary _ a b => hasCast a || hasCast b
  | .cond c a b => hasCast c || hasCast a || hasCast b
  | .cast _ => true
  | .other => false

/-! ## Data -/

theorem classify_ne_identity_of_compound (f : Facts) (e : Expr)
    (h : ∀ n s, e ≠ .name n s) : ∀ n s, classify f e ≠ .identity n s := by
  intro n s
  cases e with
  | name m t => exact absurd rfl (h m t)
  | lit => simp [classify]
  | unary a => simp only [classify]; split <;> simp
  | binary p a b => simp only [classify]; split <;> (try split) <;> simp
  | cond c a b => simp only [classify]; split <;> simp
  | cast a => simp [classify]
  | other => simp [classify]

theorem data_carries_nothing (f : Facts) :
    ∀ e, classify f e = .data → nothingCarried f e := by
  intro e
  induction e with
  | lit => intro _ p hp; simp [names] at hp
  | name n s =>
    intro h p hp
    simp only [names, List.mem_singleton] at hp
    subst hp
    simp only [classify] at h
    cases hc : f.carries n s <;> simp_all
  | unary a ih =>
    intro h
    simp only [classify] at h
    split at h <;> simp_all [nothingCarried, names]
  | binary p a b iha ihb =>
    intro h
    simp only [classify] at h
    split at h
    · simp at h
    · simp at h
    · rename_i ha hb
      cases p <;> simp at h
      intro q hq
      simp only [names, List.mem_append] at hq
      rcases hq with hq | hq
      · exact iha ha q hq
      · exact ihb hb q hq
    · cases p <;> simp at h
  | cond c a b ihc iha ihb =>
    intro h
    simp only [classify] at h
    split at h
    · simp at h
    · simp at h
    · simp at h
    · simp at h
    · rename_i hc ha hb
      intro q hq
      simp only [names, List.mem_append] at hq
      rcases hq with (hq | hq) | hq
      · exact ihc hc q hq
      · exact iha ha q hq
      · exact ihb hb q hq
    · simp at h
  | cast a _ => intro h; simp [classify] at h
  | other => intro h; simp [classify] at h

theorem nothing_carried_data (f : Facts) :
    ∀ e, arithmetic e = true → nothingCarried f e → classify f e = .data := by
  intro e
  induction e with
  | lit => intro _ _; rfl
  | name n s =>
    intro _ h
    have := h (n, s) (by simp [names])
    simp [classify, this]
  | unary a ih =>
    intro ha h
    simp only [arithmetic] at ha
    simp [classify, ih ha (fun q hq => h q (by simpa [names] using hq))]
  | binary p a b iha ihb =>
    intro hab h
    simp only [arithmetic, Bool.and_eq_true, Bool.not_eq_true'] at hab
    obtain ⟨⟨hp, ha⟩, hb⟩ := hab
    subst hp
    have ra := iha ha (fun q hq => h q (by simp [names, hq]))
    have rb := ihb hb (fun q hq => h q (by simp [names, hq]))
    simp [classify, ra, rb]
  | cond c a b ihc iha ihb =>
    intro habc h
    simp only [arithmetic, Bool.and_eq_true] at habc
    obtain ⟨⟨hc, ha⟩, hb⟩ := habc
    have rc := ihc hc (fun q hq => h q (by simp [names, hq]))
    have ra := iha ha (fun q hq => h q (by simp [names, hq]))
    have rb := ihb hb (fun q hq => h q (by simp [names, hq]))
    simp [classify, rc, ra, rb]
  | cast _ _ => intro h; simp [arithmetic] at h
  | other => intro h; simp [arithmetic] at h

/-- Over the arithmetic fragment, an actual is data exactly when no name in it
    carries an identity. -/
theorem data_iff_nothing_carried (f : Facts) (e : Expr) (h : arithmetic e = true) :
    classify f e = .data ↔ nothingCarried f e :=
  ⟨data_carries_nothing f e, nothing_carried_data f e h⟩

theorem read_data (f : Facts) (bt : Bool) (e : Expr) :
    read f bt e = .data ↔ classify f e = .data := by
  unfold read
  split <;> simp_all
  split <;> (try split) <;> (try split) <;> simp

/-! ## The borrowed place -/

theorem classify_identity_is_a_name (f : Facts) :
    ∀ e n s, classify f e = .identity n s → e = .name n s ∧ f.carries n s = true := by
  intro e n s h
  cases e with
  | name m t =>
    simp only [classify] at h
    cases hc : f.carries m t <;> simp_all
  | _ =>
    exact absurd h (classify_ne_identity_of_compound f _ (by intro m t; simp) n s)

/-- A `borrowed` reading is exactly one carrying, answered pointer name under
    a borrowed tail. -/
theorem borrowed_is_the_whole_root (f : Facts) (bt : Bool) (e : Expr) (n : Nat)
    (h : read f bt e = .borrowed n) :
    e = .name n false ∧ bt = true ∧ f.carries n false = true ∧ f.answered n = true := by
  unfold read at h
  split at h
  · simp at h
  · simp at h
  · rename_i m s hc
    obtain ⟨he, hcar⟩ := classify_identity_is_a_name f e m s hc
    cases s <;> cases bt <;> simp at h
    cases ha : f.answered m <;> simp [ha] at h
    subst h
    exact ⟨he, rfl, hcar, ha⟩

/-- Nothing widens the data tail: under it no actual is borrowed, and every
    admitted actual carries nothing. -/
theorem the_data_tail_is_never_widened (f : Facts) (e : Expr) :
    (∀ n, read f false e ≠ .borrowed n) ∧
      (read f false e = .data → nothingCarried f e) := by
  refine ⟨fun n h => ?_, fun h => data_carries_nothing f e ((read_data f false e).1 h)⟩
  have := (borrowed_is_the_whole_root f false e n h).2.1
  simp at this

/-- Every actual read as data carries nothing, under either tail. -/
theorem every_data_reading_carries_nothing (f : Facts) (bt : Bool) (e : Expr)
    (h : read f bt e = .data) : nothingCarried f e :=
  data_carries_nothing f e ((read_data f bt e).1 h)

/-! ## Refusals -/

theorem cast_not_data (f : Facts) : ∀ e, hasCast e = true → classify f e ≠ .data := by
  intro e
  induction e with
  | lit => simp [hasCast]
  | name _ _ => simp [hasCast]
  | unary a ih =>
    intro h
    simp only [hasCast] at h
    simp only [classify]
    split <;> simp_all
  | binary p a b iha ihb =>
    intro h
    simp only [hasCast, Bool.or_eq_true] at h
    simp only [classify]
    split
    · simp
    · simp
    · rename_i ha hb
      rcases h with h | h
      · exact absurd ha (iha h)
      · exact absurd hb (ihb h)
    · split <;> simp
  | cond c a b ihc iha ihb =>
    intro h
    simp only [hasCast, Bool.or_eq_true] at h
    simp only [classify]
    split
    · simp
    · simp
    · simp
    · simp
    · rename_i hc ha hb
      rcases h with (h | h) | h
      · exact absurd hc (ihc h)
      · exact absurd ha (iha h)
      · exact absurd hb (ihb h)
    · simp
  | cast _ _ => intro _; simp [classify]
  | other => simp [hasCast]

/-- An actual containing an explicit cast is never read: neither data nor a
    borrowed place (the card CAST-FORMALIZATION reads casts). -/
theorem a_cast_is_never_read (f : Facts) (bt : Bool) (e : Expr) (h : hasCast e = true) :
    read f bt e ≠ .data ∧ ∀ n, read f bt e ≠ .borrowed n := by
  refine ⟨fun hd => cast_not_data f e h ((read_data f bt e).1 hd), fun n hb => ?_⟩
  have := (borrowed_is_the_whole_root f bt e n hb).1
  subst this
  simp [hasCast] at h

theorem a_cast_is_refused_by_name (f : Facts) (bt : Bool) (e : Expr) :
    read f bt (.cast e) = .refused .cast := rfl

/-- A pointer-valued operator is never data or borrowed. -/
theorem pointer_arithmetic_is_never_read (f : Facts) (bt : Bool) (a b : Expr) :
    read f bt (.binary true a b) ≠ .data ∧ ∀ n, read f bt (.binary true a b) ≠ .borrowed n := by
  refine ⟨fun hd => ?_, fun n hb => ?_⟩
  · have h := (read_data f bt _).1 hd
    simp only [classify] at h
    split at h <;> simp_all
  · have := (borrowed_is_the_whole_root f bt _ n hb).1
    simp at this

/-- A conditional with a data condition, one carrying arm and a data arm is
    refused by name. -/
theorem an_identity_arm_is_refused (f : Facts) (bt : Bool) (c b : Expr) (n : Nat)
    (hc : classify f c = .data) (hb : classify f b = .data) (hn : f.carries n false = true) :
    read f bt (.cond c (.name n false) b) = .refused .identityArm := by
  simp [read, classify, hc, hb, hn]

/-- An identity-carrying condition is refused by name. -/
theorem an_identity_condition_is_refused (f : Facts) (bt : Bool) (a b : Expr) (n : Nat)
    (ha : classify f a = .data) (hb : classify f b = .data) (hn : f.carries n false = true) :
    read f bt (.cond (.name n false) a b) = .refused .identityOperand := by
  simp [read, classify, ha, hb, hn]

/-- A carrying pointer name rule 10l does not answer stays refused. -/
theorem an_unanswered_root_is_refused (f : Facts) (n : Nat)
    (hn : f.carries n false = true) (ha : f.answered n = false) :
    read f true (.name n false) = .refused .unansweredRoot := by
  simp [read, classify, hn, ha]

/-- An integer-carried identity is never borrowed. -/
theorem an_integer_identity_is_refused (f : Facts) (bt : Bool) (n : Nat)
    (hn : f.carries n true = true) :
    read f bt (.name n true) = .refused .integerIdentity := by
  simp [read, classify, hn]

/-! ## Custody neutrality -/

open QCore.VariadicNested

/-- What a reading hands the modeled call: nothing for data, the name's
    identity for a borrowed place. -/
def denote : Reading → Option Nat
  | .borrowed n => some n
  | _ => none

/-- The readings, as the modeled tail of a call whose tail positions are all
    `borrowed`, change no outcome. -/
theorem the_reading_is_custody_neutral (c : Contract) (args : List (Option Nat))
    (readings : List Reading) (o : Outcome) (arity : args.length = c.modes.length) :
    applyCall { c with modes := c.modes ++ List.replicate readings.length ParamMode.borrowed }
        (args ++ readings.map denote) o
      = applyCall c args o := by
  have := QCore.Variadics.borrowed_tail_is_the_identity c args (readings.map denote) o arity
  simpa using this

/-! ## Shared vectors (`variadic_tail_actuals::READ_VECTORS`) -/

def factsOf (carrying answered : List Nat) : Facts :=
  ⟨fun n _ => carrying.contains n, fun n => answered.contains n⟩

def readVectors : List (Expr × List Nat × List Nat × Bool × Reading) := [
  (.lit, [], [], false, .data),
  ((.name 1 true), [], [], false, .data),
  ((.name 1 true), [1], [1], true, (.refused .integerIdentity)),
  ((.name 1 false), [], [], false, .data),
  ((.name 1 false), [1], [1], true, (.borrowed 1)),
  ((.name 1 false), [1], [1], false, (.refused .identityUnderDataTail)),
  ((.name 1 false), [1], [], true, (.refused .unansweredRoot)),
  ((.binary false (.name 1 true) .lit), [], [], true, .data),
  ((.binary false (.name 1 false) .lit), [1], [1], true, (.refused .identityOperand)),
  ((.binary true (.name 1 false) .lit), [], [1], true, (.refused .pointerArithmetic)),
  ((.binary true (.name 1 false) .lit), [1], [1], true, (.refused .pointerArithmetic)),
  ((.binary true (.cast (.name 1 false)) .lit), [1], [1], true, (.refused .cast)),
  ((.unary (.name 1 true)), [], [], false, .data),
  ((.unary (.name 1 false)), [1], [1], true, (.refused .identityOperand)),
  ((.cond (.name 1 true) .lit .lit), [], [], false, .data),
  ((.cond (.name 1 false) (.name 1 false) .lit), [], [1], true, .data),
  ((.cond (.name 1 false) (.name 1 false) .lit), [1], [1], true, (.refused .identityOperand)),
  ((.cond .lit (.name 1 false) .lit), [1], [1], true, (.refused .identityArm)),
  ((.cond .lit .lit (.name 2 false)), [2], [2], true, (.refused .identityArm)),
  ((.cast (.name 1 true)), [], [], true, (.refused .cast)),
  ((.binary false (.cast (.name 1 true)) .lit), [], [], true, (.refused .cast)),
  ((.cast (.name 1 false)), [1], [1], true, (.refused .cast)),
  (.other, [], [], true, (.refused .unread)),
  ((.binary false .other (.cast .lit)), [], [], true, (.refused .unread))
]

theorem read_vectors_hold :
    readVectors.all (fun (e, c, a, bt, r) => read (factsOf c a) bt e == r) = true := by
  decide

end QCore.VariadicTailActuals
