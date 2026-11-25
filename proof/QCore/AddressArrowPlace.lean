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
# `(&X)->f` is `X.f` in two more readers (card K6-DIRECT-POSITION-ACCESS, rule 81)

Rule 10n (1) reads `(&X)->f` as `X.f` in the typed access-path reader. Rule
81 applies the same reading to the elaborator's place reader
(`elaboration::path_place`) and to the lexical `may_bind` reader
(`typed_field_access::bound_names`), whose `&` arm read the `& p` of
`( & p -> q ) -> f` as the address of `p`.

C decides both: `E1->E2` is `(*(E1)).E2` (C11 6.5.2.3p4), `*&E` is `E`
(6.5.3.2p3), and a parenthesised expression is the expression it encloses
(6.5.1p5; the readers strip parentheses first, so `Expr` has none).

**Places.** `Expr` is the lvalue fragment the place reader walks. `place` is
C's designation (the root object and the steps to the storage), in which an
arrow over `&X` designates `X`'s member (the `*&` cancellation) and any other
arrow dereferences its base's value. `oldPlace` is the reader before the rule
(an arrow over `&X` refuses: `path through UnaryOperator`), `newPlace` the
reader with it. Proved: the new reader is C's designation
(`new_place_is_c_place`), it keeps every old answer (`the_rule_only_adds`),
the rewritten spelling reads exactly as the plain one
(`arrow_through_address_is_dot`) and keeps its root (`same_root_as_plain`),
and an old refusal it lifts is exactly an arrow over an address
(`a_lifted_refusal_is_an_arrow_over_an_address`).

**Bindings.** An `Occurrence` is what the lexical test reads about one
`& NAME`: whether the `&` opens a group `( & NAME`, whether `NAME` is followed
by `->`, whether the group's tokens are a member/subscript selection chain
closed by `)` with `->` after it (`dereferenced`), and whether a unary `&`
stands before the group's opening parentheses (`outerAddress`). `oldBinds` is
the address arm before the rule (it binds `NAME`), `newBinds` after it, and
`plainBinds` the arm on the plain spelling `p SEL . f` of the same place in
the same context (`& p SEL . f` when an outer `&` stands before the group,
which the plain arm binds; no `&` before `p` otherwise). Proved: the rule only
removes bindings (`the_rule_only_removes`), never binds less than the plain
spelling (`at_least_the_plain_reader`) and agrees with it on every
dereferenced group (`agrees_with_the_plain_spelling`), and an exempted `&`
takes a member's address, never `NAME`'s (`an_exempted_address_is_a_member_address`).
The Rust decision vectors (`LEXICAL_VECTORS`) are replayed.

## What this is not

A model of tokenisation, of the rest of `bound_names` (writes, increments,
call actuals, rule 10o's parenthesised writes), or of Clang's AST: that the
importer splits `->` into `-` `>`, that the group's `)` is the matching one,
and that Clang's `MemberExpr`/`UnaryOperator`/`ParenExpr` nodes are what
`Expr` names are premises (trust boundary 28).
-/

namespace QCore.AddressArrowPlace

/-- The lvalue fragment the place reader walks (parentheses stripped). -/
inductive Expr where
  | var (x : String)
  | dot (e : Expr) (f : String)
  | arrow (e : Expr) (f : String)
  | index (e : Expr)
  | addr (e : Expr)
  deriving DecidableEq, Repr

/-- A step from the root object to the designated storage. -/
inductive Step where
  | member (f : String)
  | deref
  | elem
  deriving DecidableEq, Repr

structure Place where
  root : String
  steps : List Step
  deriving DecidableEq, Repr

def Place.push (p : Place) (s : Step) : Place := ⟨p.root, p.steps ++ [s]⟩

/-- C's designation. An address is a value, no place. -/
def place : Expr → Option Place
  | .var x => some ⟨x, []⟩
  | .dot e f => (place e).map (·.push (.member f))
  | .arrow (.addr x) f => (place x).map (·.push (.member f))
  | .arrow e f => (place e).map (fun p => (p.push .deref).push (.member f))
  | .index e => (place e).map (·.push .elem)
  | .addr _ => none

/-- The reader before rule 81: an arrow over an address refuses. -/
def oldPlace : Expr → Option Place
  | .var x => some ⟨x, []⟩
  | .dot e f => (oldPlace e).map (·.push (.member f))
  | .arrow (.addr _) _ => none
  | .arrow e f => (oldPlace e).map (fun p => (p.push .deref).push (.member f))
  | .index e => (oldPlace e).map (·.push .elem)
  | .addr _ => none

/-- The reader with rule 81 (`arrow_through_address`). -/
def newPlace : Expr → Option Place
  | .var x => some ⟨x, []⟩
  | .dot e f => (newPlace e).map (·.push (.member f))
  | .arrow (.addr x) f => (newPlace x).map (·.push (.member f))
  | .arrow e f => (newPlace e).map (fun p => (p.push .deref).push (.member f))
  | .index e => (newPlace e).map (·.push .elem)
  | .addr _ => none

/-- The arrow over a base that is no address, in each reader. -/
def deref (f : String) (p : Place) : Place := (p.push .deref).push (.member f)

theorem place_arrow (b : Expr) (f : String) (hb : ∀ x, b ≠ .addr x) :
    place (.arrow b f) = (place b).map (deref f) := by
  cases b with
  | addr x => exact absurd rfl (hb x)
  | _ => rfl

theorem new_arrow (b : Expr) (f : String) (hb : ∀ x, b ≠ .addr x) :
    newPlace (.arrow b f) = (newPlace b).map (deref f) := by
  cases b with
  | addr x => exact absurd rfl (hb x)
  | _ => rfl

theorem old_arrow (b : Expr) (f : String) (hb : ∀ x, b ≠ .addr x) :
    oldPlace (.arrow b f) = (oldPlace b).map (deref f) := by
  cases b with
  | addr x => exact absurd rfl (hb x)
  | _ => rfl

/-- **The new reader is C's designation.** -/
theorem new_place_is_c_place : ∀ e, newPlace e = place e
  | .var _ => rfl
  | .dot e f => by
      show (newPlace e).map _ = (place e).map _
      rw [new_place_is_c_place e]
  | .arrow (.addr x) f => by
      show (newPlace x).map _ = (place x).map _
      rw [new_place_is_c_place x]
  | .arrow (.var x) f => rfl
  | .arrow (.dot e g) f => by
      rw [new_arrow _ _ (by intro x h; cases h), place_arrow _ _ (by intro x h; cases h),
        new_place_is_c_place (.dot e g)]
  | .arrow (.arrow e g) f => by
      rw [new_arrow _ _ (by intro x h; cases h), place_arrow _ _ (by intro x h; cases h),
        new_place_is_c_place (.arrow e g)]
  | .arrow (.index e) f => by
      rw [new_arrow _ _ (by intro x h; cases h), place_arrow _ _ (by intro x h; cases h),
        new_place_is_c_place (.index e)]
  | .index e => by
      show (newPlace e).map _ = (place e).map _
      rw [new_place_is_c_place e]
  | .addr _ => rfl

/-- **The rule only adds**: every place the old reader answered is answered
the same. -/
theorem the_rule_only_adds : ∀ e p, oldPlace e = some p → newPlace e = some p
  | .var _, _, h => h
  | .dot e f, p, h => by
      have h' : (oldPlace e).map (·.push (.member f)) = some p := h
      obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h'
      show (newPlace e).map _ = _
      rw [the_rule_only_adds e q hq]; rfl
  | .arrow (.addr _) _, _, h => by cases h
  | .arrow (.var x) f, p, h => h
  | .arrow (.dot e g) f, p, h => by
      rw [old_arrow _ _ (by intro x h; cases h)] at h
      obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h
      rw [new_arrow _ _ (by intro x h; cases h), the_rule_only_adds (.dot e g) q hq]; rfl
  | .arrow (.arrow e g) f, p, h => by
      rw [old_arrow _ _ (by intro x h; cases h)] at h
      obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h
      rw [new_arrow _ _ (by intro x h; cases h), the_rule_only_adds (.arrow e g) q hq]; rfl
  | .arrow (.index e) f, p, h => by
      rw [old_arrow _ _ (by intro x h; cases h)] at h
      obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h
      rw [new_arrow _ _ (by intro x h; cases h), the_rule_only_adds (.index e) q hq]; rfl
  | .index e, p, h => by
      have h' : (oldPlace e).map (·.push .elem) = some p := h
      obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h'
      show (newPlace e).map _ = _
      rw [the_rule_only_adds e q hq]; rfl
  | .addr _, _, h => by cases h

/-- **The rewritten spelling reads as the plain one**: `(&X)->f` and `X.f`
designate the same place. -/
theorem arrow_through_address_is_dot (x : Expr) (f : String) :
    newPlace (.arrow (.addr x) f) = newPlace (.dot x f) := rfl

theorem same_root_as_plain (x : Expr) (f : String) (p : Place)
    (h : newPlace (.arrow (.addr x) f) = some p) :
    ∃ q, newPlace x = some q ∧ p.root = q.root := by
  have h' : (newPlace x).map (·.push (.member f)) = some p := h
  obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h'
  exact ⟨q, hq, rfl⟩

/-- A refusal the rule lifts at the top is an arrow over an address. -/
theorem a_lifted_refusal_is_an_arrow_over_an_address (x : Expr) (f : String) :
    oldPlace (.arrow (.addr x) f) = none := rfl

/-! ## The lexical `may_bind` arm -/

/-- What the lexical test reads about one `& NAME` (a single `&`). -/
structure Occurrence where
  /-- The `&` directly follows `(`. -/
  opensGroup : Bool
  /-- `NAME` is followed by `->`. -/
  nameArrow : Bool
  /-- The group is a selection chain closed by `)` and followed by `->`. -/
  dereferenced : Bool
  /-- A unary `&` stands before the group's opening parentheses. -/
  outerAddress : Bool
  deriving DecidableEq, Repr

/-- `dereferenced_address_group`. -/
def exempt (o : Occurrence) : Bool :=
  o.opensGroup && o.nameArrow && o.dereferenced && !o.outerAddress

/-- The address arm before the rule: a single `&` before `NAME` binds it. -/
def oldBinds (_ : Occurrence) : Bool := true

/-- The address arm after the rule. -/
def newBinds (o : Occurrence) : Bool := oldBinds o && !exempt o

/-- The operand C gives the `&`: `NAME` itself only when no postfix operator
follows `NAME` (6.5.2 binds tighter than 6.5.3). -/
inductive Operand where
  | whole
  | member
  deriving DecidableEq, Repr

def ampOperand (o : Occurrence) : Operand := if o.nameArrow then .member else .whole

/-- The address arm on the plain spelling of a dereferenced group
(`p SEL . f`): it binds `p` only when an outer `&` makes it `& p SEL . f`. On
any other occurrence the plain spelling is the occurrence itself. -/
def plainBinds (o : Occurrence) : Bool :=
  if o.opensGroup && o.nameArrow && o.dereferenced then o.outerAddress else true

theorem the_rule_only_removes (o : Occurrence) (h : newBinds o = true) :
    oldBinds o = true := by
  simp [newBinds] at h; exact h.1

theorem at_least_the_plain_reader (o : Occurrence) (h : plainBinds o = true) :
    newBinds o = true := by
  cases o with
  | mk g n d a => cases g <;> cases n <;> cases d <;> cases a <;> simp_all [plainBinds, newBinds, oldBinds, exempt]

theorem agrees_with_the_plain_spelling (o : Occurrence)
    (h : (o.opensGroup && o.nameArrow && o.dereferenced) = true) :
    newBinds o = plainBinds o := by
  cases o with
  | mk g n d a => cases g <;> cases n <;> cases d <;> cases a <;> simp_all [plainBinds, newBinds, oldBinds, exempt]

theorem an_exempted_address_is_a_member_address (o : Occurrence) (h : exempt o = true) :
    ampOperand o = .member := by
  cases o with
  | mk g n d a => cases n <;> simp_all [exempt, ampOperand]

/-- A whole-object address (`NAME` not followed by `->`) is always bound. -/
theorem a_whole_address_is_bound (o : Occurrence) (h : ampOperand o = .whole) :
    newBinds o = true := by
  cases o with
  | mk g n d a => cases n <;> simp_all [exempt, ampOperand, newBinds, oldBinds]

/-- An outer address keeps the binding (`&((&p->q)->f)` is `&p->q.f`). -/
theorem an_outer_address_keeps_the_binding (o : Occurrence) (h : o.outerAddress = true) :
    newBinds o = true := by
  simp [newBinds, oldBinds, exempt, h]

/-- The Rust `LEXICAL_VECTORS`, as occurrences, with the expected answer of
`dereferenced_address_group`. -/
def lexicalVectors : List (Occurrence × Bool) :=
  [ (⟨true, true, true, false⟩, true),    -- `( & p - > q ) - > f`
    (⟨true, true, true, false⟩, true),    -- `( ( & p - > q ) - > f )`
    (⟨true, true, true, false⟩, true),    -- `( & p - > a [ i ] . h ) - > len`
    (⟨true, true, true, false⟩, true),    -- `( & p - > q . r ) - > f`
    (⟨true, true, true, true⟩, false),    -- `& ( ( & p - > q ) - > f )`
    (⟨true, true, true, true⟩, false),    -- `& ( & p - > q ) - > f`
    (⟨true, false, false, false⟩, false), -- `( & p ) - > f`
    (⟨true, true, false, false⟩, false),  -- `( & p - > q )`
    (⟨true, true, false, false⟩, false),  -- `( & p - > q ) . f`
    (⟨true, true, false, false⟩, false),  -- `( & p - > q + 1 ) - > f`
    (⟨true, false, false, false⟩, false), -- `( & p . q ) - > f`
    (⟨false, true, false, false⟩, false), -- `& p - > q`
    (⟨true, true, false, false⟩, false) ] -- `( & p - > a [ i ) - > f`

theorem lexical_vectors_hold :
    lexicalVectors.all (fun (o, r) => exempt o == r) = true := by
  decide

end QCore.AddressArrowPlace
