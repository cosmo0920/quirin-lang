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

/-!
# A root inside a subscript's brackets may be its pointer (card RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT, rule 74y)

C11 6.5.2.1p2 defines `E1[E2]` as `(*((E1)+(E2)))`, and 6.5.6p2/p8 let the
pointer of an additive operation stand on either side. So the object a
subscript reads is the object of whichever operand is the pointer
(`subscript_comm`): `1[p]` and `n[p]` read `p`'s object exactly as `p[1]`
does (`reversed_reads_the_inside_pointer`). Only when the subscripted operand
is itself a pointer do the brackets hold the index
(`a_pointer_outside_reads_its_own_object`).

Family S of `legacy_return_family` read a root as a subscript base only as
`p [`, so `1[p]` through a null untracked `p` made no base and the function
was `Safe` (`old_misses_the_base`); `n[*q]` with a may-bind `q` was admitted
as one dereference although the subscript dereferences the value `*q` reads.

Rule 74y's reader walks outward from the root through the layers rule 74w
reads (`cast`, `paren`, `sum classified`) until the span fills a subscript's
brackets, and classifies the subscripted operand (`Outside`): a declared
pointer or array, or anything else. The root is a base unless the outside is
a declared pointer (`subscript`); a crossed unclassified sum refuses. A
may-bind root inside brackets whose outside is not a declared pointer
refuses.

* The outcome is "index, no base" only under a pointer outside
  (`index_only_under_a_pointer_outside`), so whenever C's subscript reads the
  object of the operand inside the brackets, the reader makes a base or
  refuses (`an_inside_pointer_is_never_missed`), given a sound outside
  class (trust boundary 28: the declared-pointer fact).
* Every old base is kept and the new refusals are the expression's, so a
  result the tip admits, the base admitted (`the_fix_only_refuses_more`).

## What this is not

A model of the token scan or of the flow check. The layers, the outside
class and the may-bind position are the Rust reader's output (trust boundary
28), bound by `subscriptVectors` and `mayBindVectors`, whose old fields the
Rust test checks against the functions the base already had.
-/

namespace QCore.ReturnFamilyUntrackedReversedSubscript

/-! ## C11 6.5.2.1p2: the subscript is a dereference of a sum -/

/-- A value: a pointer into the object `base` at offset `off`, or an integer. -/
inductive Val where
  | ptr (base : String) (off : Int)
  | int (n : Int)
  deriving DecidableEq, Repr

/-- C11 6.5.6p2/p8: a pointer plus an integer, on either side, is a pointer
into the same object; two integers add; anything else is not an additive
expression C defines. -/
def add : Val → Val → Option Val
  | .ptr b o, .int n => some (.ptr b (o + n))
  | .int n, .ptr b o => some (.ptr b (o + n))
  | .int a, .int b => some (.int (a + b))
  | _, _ => none

/-- The object a unary `*` reads through a value, if it is a pointer. -/
def derefObject : Option Val → Option String
  | some (.ptr b _) => some b
  | _ => none

/-- C11 6.5.2.1p2: `E1[E2]` is `(*((E1)+(E2)))`; the object it reads. -/
def subscript (e1 e2 : Val) : Option String := derefObject (add e1 e2)

theorem subscript_comm (e1 e2 : Val) : subscript e1 e2 = subscript e2 e1 := by
  cases e1 <;> cases e2 <;> simp [subscript, add, derefObject, Int.add_comm]

/-- `1[p]`, `n[p]`: the object read is the one inside the brackets. -/
theorem reversed_reads_the_inside_pointer (n : Int) (b : String) (o : Int) :
    subscript (.int n) (.ptr b o) = some b := by
  simp [subscript, add, derefObject]

/-- `a[i]` with a pointer `a`: the object read is `a`'s; the brackets hold
the index. -/
theorem a_pointer_outside_reads_its_own_object (b b' : String) (o : Int) (e2 : Val)
    (h : subscript (.ptr b o) e2 = some b') : b' = b := by
  cases e2 <;> simp_all [subscript, add, derefObject]

/-- With an integer outside, whatever object the subscript reads is the
inside operand's. -/
theorem an_integer_outside_puts_the_pointer_inside (n : Int) (e2 : Val) (b : String)
    (h : subscript (.int n) e2 = some b) : ∃ o, e2 = .ptr b o := by
  cases e2 with
  | ptr b' o => simp_all [subscript, add, derefObject]
  | int m => simp [subscript, add, derefObject] at h

/-! ## The reader -/

/-- One widening step of the walk outward from the root (rule 74w's). -/
inductive Layer where
  | cast
  | paren
  | sum (classified : Bool)
  deriving DecidableEq, Repr

/-- The reader's class of the subscripted operand. -/
inductive Outside where
  /-- A name the declaration reader declares as a pointer or array. -/
  | pointer
  /-- A numeric literal or a declared scalar. -/
  | integer
  /-- Anything else (a member, an undeclared name, a group). -/
  | undecided
  deriving DecidableEq, Repr

/-- Rule 74y's reading of one untracked, non-scalar root. -/
inductive Outcome where
  | none
  | base
  | unclassified
  deriving DecidableEq, Repr

def sumClassified : Layer → Bool
  | .sum c => c
  | _ => true

/-- `subscript_index`: `none` for no bracket reached. -/
def subscriptIndex (layers : List Layer) : Option Outside → Outcome
  | none => .none
  | some .pointer => .none
  | some _ => if layers.all sumClassified then .base else .unclassified

theorem no_bracket_keeps_the_reading (layers : List Layer) :
    subscriptIndex layers none = .none := rfl

theorem index_only_under_a_pointer_outside (layers : List Layer) (c : Outside)
    (h : subscriptIndex layers (some c) = .none) : c = .pointer := by
  cases c with
  | pointer => rfl
  | integer => simp [subscriptIndex] at h; split at h <;> simp_all
  | undecided => simp [subscriptIndex] at h; split at h <;> simp_all

/-- The class is sound for the subscripted operand's value. -/
def Sound : Outside → Val → Prop
  | .pointer, v => ∃ b o, v = .ptr b o
  | .integer, v => ∃ n, v = .int n
  | .undecided, _ => True

/-- Whenever C's subscript reads the object of the operand inside the
brackets, the reader makes the root a base or refuses. -/
theorem an_inside_pointer_is_never_missed (layers : List Layer) (c : Outside)
    (e1 : Val) (b : String) (o : Int) (hs : Sound c e1)
    (hr : subscript e1 (.ptr b o) = some b) : subscriptIndex layers (some c) ≠ .none := by
  intro h
  have hc := index_only_under_a_pointer_outside layers c h
  subst hc
  obtain ⟨b', o', he⟩ := hs
  subst he
  simp [subscript, add, derefObject] at hr

/-! ## The read -/

/-- One untracked root: its name, the reader's outcome, and the base's
reading (the direct tests and rules 74r's, 74t's and 74w's walks). -/
structure Root where
  name : String
  outcome : Outcome
  oldBase : Bool
  deriving DecidableEq, Repr

def base (r : Root) : Bool := r.oldBase || r.outcome == .base

def refuses (r : Root) : Bool := r.outcome == .unclassified

theorem old_base_kept (r : Root) (h : r.oldBase = true) : base r = true := by
  simp [base, h]

/-- One may-bind root: the base's admission and whether a `[` enclosing it
subscripts an operand that is not a declared pointer. -/
structure MayBind where
  name : String
  oldAdmitted : Bool
  insideUndecided : Bool
  deriving DecidableEq, Repr

def admitted (m : MayBind) : Bool := m.oldAdmitted && !m.insideUndecided

theorem admitted_was_admitted (m : MayBind) (h : admitted m = true) :
    m.oldAdmitted = true := by
  simp only [admitted, Bool.and_eq_true] at h
  exact h.1

/-- `n [ * q ]`: refused. -/
theorem a_may_bind_root_inside_undecided_brackets_refuses (m : MayBind)
    (h : m.insideUndecided = true) : admitted m = false := by
  simp [admitted, h]

/-- The base's family S result: `none` refuses; else the bases. `refused` is
every other refusal, which the card does not touch. -/
def oldRead (refused : Bool) (roots : List Root) (binds : List MayBind) :
    Option (List String) :=
  if refused || binds.any (fun m => !m.oldAdmitted) then none
  else some ((roots.filter (·.oldBase)).map (·.name) ++ binds.map (·.name))

/-- The tip's family S result. -/
def newRead (refused : Bool) (roots : List Root) (binds : List MayBind) :
    Option (List String) :=
  if refused || roots.any refuses || binds.any (fun m => !admitted m) then none
  else some ((roots.filter base).map (·.name) ++ binds.map (·.name))

/-- The flow check admits only when every base's obligation `ok` discharges. -/
def admits (ok : String → Bool) : Option (List String) → Bool
  | none => false
  | some bases => bases.all ok

theorem names_kept (roots : List Root) (name : String)
    (hn : name ∈ (roots.filter (·.oldBase)).map (·.name)) :
    name ∈ (roots.filter base).map (·.name) := by
  simp only [List.mem_map, List.mem_filter] at hn ⊢
  obtain ⟨r, ⟨hm, ho⟩, he⟩ := hn
  exact ⟨r, ⟨hm, old_base_kept r ho⟩, he⟩

/-- Bases only grow and the new refusals are the expression's, so a result
the tip admits, the base admitted. -/
theorem the_fix_only_refuses_more (ok : String → Bool) (refused : Bool)
    (roots : List Root) (binds : List MayBind)
    (h : admits ok (newRead refused roots binds) = true) :
    admits ok (oldRead refused roots binds) = true := by
  unfold newRead at h
  unfold oldRead
  by_cases hn : (refused || roots.any refuses || binds.any (fun m => !admitted m)) = true
  · simp [hn, admits] at h
  · have hr : refused = false := by
      cases hr : refused <;> simp_all
    have hq : roots.any refuses = false := by
      cases hq : roots.any refuses <;> simp_all
    have hb : binds.any (fun m => !admitted m) = false := by
      cases hb : binds.any (fun m => !admitted m) <;> simp_all
    have ho : binds.any (fun m => !m.oldAdmitted) = false := by
      simp only [List.any_eq_false, Bool.not_eq_true'] at hb ⊢
      intro m hm
      have := hb m hm
      cases hm' : m.oldAdmitted
      · simp [admitted, hm'] at this
      · simp
    simp only [hr, hq, hb, ho, Bool.or_false, Bool.false_eq_true, ite_false, admits,
      List.all_eq_true, List.mem_append] at h ⊢
    intro name hm
    cases hm with
    | inl hl => exact h name (Or.inl (names_kept roots name hl))
    | inr hr' => exact h name (Or.inr hr')

/-! ## The readings -/

/-- `1 [ p ]`: no old test read it; the tip makes `p` a base. -/
theorem old_misses_the_base :
    let r : Root := ⟨"p", subscriptIndex [] (some .integer), false⟩
    base r = true ∧ refuses r = false := by
  decide

/-- `a [ p ]` with a declared array `a`: the root is the index. -/
theorem a_declared_pointer_keeps_the_index :
    subscriptIndex [] (some .pointer) = .none := rfl

/-- `a [ b ]`, neither declared: both are bases (`a` already was). -/
theorem an_undecided_outside_is_a_base :
    subscriptIndex [] (some .undecided) = .base := rfl

/-- `n [ p + a ]`: an unclassified sum in the brackets refuses. -/
theorem an_unclassified_sum_refuses :
    subscriptIndex [.sum false] (some .integer) = .unclassified := rfl

/-! ## The token reader's vectors -/

/-- `(spelling, layers, outside, old base, base, refused)` for the untracked
`p`. The Rust unit test reads the spelling, the old base (against the base's
functions) and the last two (against `scalar_read`). -/
def subscriptVectors : List (String × List Layer × Option Outside × Bool × Bool × Bool) := [
  ("1 [ p ]", [], some .integer, false, true, false),
  ("n [ p ]", [], some .integer, false, true, false),
  ("n [ ( long * ) p ]", [.cast], some .integer, false, true, false),
  ("n [ ( p ) ]", [.paren], some .integer, false, true, false),
  ("n [ p + 1 ]", [.sum true], some .integer, false, true, false),
  ("0 [ ( long * ) p + n ]", [.cast, .sum true], some .integer, false, true, false),
  ("n [ ( p + 1 ) ]", [.sum true], some .integer, false, true, false),
  ("1 [ p ] [ 0 ]", [], some .integer, false, true, false),
  ("n [ p ] . m", [], some .integer, false, true, false),
  ("( n + 1 ) [ p ]", [], some .undecided, false, true, false),
  ("s - > a [ p ]", [], some .undecided, false, true, false),
  ("b [ p ]", [], some .undecided, false, true, false),
  ("a [ p ]", [], some .pointer, false, false, false),
  ("a [ p + 1 ]", [.sum true], some .pointer, false, false, false),
  ("n [ p + b ]", [.sum false], some .integer, false, false, true),
  ("n [ b - p ]", [.sum false], some .integer, false, false, true),
  ("p [ 1 ]", [], none, true, true, false),
  ("p + n", [], none, false, false, false),
  ("n [ * p ]", [], none, true, true, false),
  ("n * p [ 0 ]", [], none, true, true, false)
]

def subscriptVectorsAgree : Bool :=
  subscriptVectors.all (fun (name, layers, outside, oldB, b, refused) =>
    let r : Root := ⟨name, subscriptIndex layers outside, oldB⟩
    base r == b && refuses r == refused)

theorem subscript_vectors_hold : subscriptVectorsAgree = true := by decide

/-- `(spelling, old admitted, inside undecided, admitted)` for the may-bind
`q`. -/
def mayBindVectors : List (String × Bool × Bool × Bool) := [
  ("n [ * q ]", true, true, false),
  ("n [ * q + 1 ]", true, true, false),
  ("1 [ q - > m ]", true, true, false),
  ("n [ q [ 0 ] ]", true, true, false),
  ("b [ * q ]", true, true, false),
  ("a [ * q ]", true, false, true),
  ("* q", true, false, true),
  ("q [ n ]", true, false, true),
  ("n [ q ]", false, true, false)
]

def mayBindVectorsAgree : Bool :=
  mayBindVectors.all (fun (name, oldA, inside, a) =>
    admitted ⟨name, oldA, inside⟩ == a)

theorem may_bind_vectors_hold : mayBindVectorsAgree = true := by decide

end QCore.ReturnFamilyUntrackedReversedSubscript
