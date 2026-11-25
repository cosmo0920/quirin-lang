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

Every condition and refusal is a formula of the C04.18.9 typed rendering
over the K6 block (`k6Terms`, `k6Sorts`; Rust `premise_formula::RENDERED_TERMS`,
block `K6`), checked by the premise-formula validator in Rust;
`formulaTable` is the Rust table, row for row. Each term has one Lean
definition here (`k6_*` below, the model of the term), and each formula is
proved over those definitions (`formula_*`).

**Places.** `Expr` is the lvalue fragment the place reader walks. `place` is
C's designation (the root object and the steps to the storage), in which an
arrow over `&X` designates `X`'s member (the `*&` cancellation) and any other
arrow dereferences its base's value. `oldPlace` is the reader before the rule
(an arrow over `&X` refuses: `path through UnaryOperator`), `newPlace` the
reader with it, which reads an arrow over `&X` only for a member or subscript
place `X` (`arrow_through_address`). Proved: every answer of the new reader is
C's designation (`new_place_sound`), it keeps every old answer
(`the_rule_only_adds`), the rewritten spelling reads exactly as the plain one
(`arrow_through_address_is_dot`) with its root (`same_root_as_plain`), and an
arrow over the address of a whole object keeps the refusal
(`a_whole_object_address_is_refused`).

**Bindings.** An `Occurrence` is what the lexical test reads about one
`& NAME`: whether the `&` opens a group `( & NAME`, whether `NAME` is followed
by `->`, whether the group's tokens are a member/subscript selection chain,
whether `->` follows the group, and whether a unary `&` stands before the
group's opening parentheses. `oldBinds` is the address arm before the rule
(it binds `NAME`), `newBinds` after it, and `plainBinds` the arm on the plain
spelling `p SEL . f` of the same place in the same context (`& p SEL . f` when
an outer `&` stands before the group, which the plain arm binds; no `&` before
`p` otherwise). Proved: the rule only removes bindings, never binds less than
the plain spelling and agrees with it on every dereferenced group, and an
exempted `&` takes a member's address, never `NAME`'s. The Rust decision
vectors (`LEXICAL_VECTORS`) are replayed.

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

/-- An arrow's dereference of a base value, then its member. -/
def deref (f : String) (p : Place) : Place := (p.push .deref).push (.member f)

/-- C's designation. An address is a value, no place. -/
def place : Expr → Option Place
  | .var x => some ⟨x, []⟩
  | .dot e f => (place e).map (·.push (.member f))
  | .arrow (.addr x) f => (place x).map (·.push (.member f))
  | .arrow e f => (place e).map (deref f)
  | .index e => (place e).map (·.push .elem)
  | .addr _ => none

/-- A member or subscript place (`MemberExpr`, `ArraySubscriptExpr`): the
places `arrow_through_address` reads under `&`. -/
def isMemberPlace : Expr → Bool
  | .dot _ _ => true
  | .arrow _ _ => true
  | .index _ => true
  | _ => false

/-- The reader before rule 81: an arrow over an address refuses. -/
def oldPlace : Expr → Option Place
  | .var x => some ⟨x, []⟩
  | .dot e f => (oldPlace e).map (·.push (.member f))
  | .arrow (.addr _) _ => none
  | .arrow e f => (oldPlace e).map (deref f)
  | .index e => (oldPlace e).map (·.push .elem)
  | .addr _ => none

/-- The reader with rule 81 (`arrow_through_address`): an arrow over the
address of a member or subscript place reads that place's member. -/
def newPlace : Expr → Option Place
  | .var x => some ⟨x, []⟩
  | .dot e f => (newPlace e).map (·.push (.member f))
  | .arrow (.addr x) f => if isMemberPlace x then (newPlace x).map (·.push (.member f)) else none
  | .arrow e f => (newPlace e).map (deref f)
  | .index e => (newPlace e).map (·.push .elem)
  | .addr _ => none

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

/-- **Every answer of the new reader is C's designation.** -/
theorem new_place_sound : ∀ e p, newPlace e = some p → place e = some p
  | .var _, _, h => h
  | .dot e f, p, h => by
      have h' : (newPlace e).map (·.push (.member f)) = some p := h
      obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h'
      show (place e).map _ = _
      rw [new_place_sound e q hq]; rfl
  | .arrow (.addr x) f, p, h => by
      have h' : (if isMemberPlace x then (newPlace x).map (·.push (.member f)) else none)
          = some p := h
      split at h'
      · obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h'
        show (place x).map _ = _
        rw [new_place_sound x q hq]; rfl
      · cases h'
  | .arrow (.var x) f, p, h => h
  | .arrow (.dot e g) f, p, h => by
      rw [new_arrow _ _ (by intro x h; cases h)] at h
      obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h
      rw [place_arrow _ _ (by intro x h; cases h), new_place_sound (.dot e g) q hq]; rfl
  | .arrow (.arrow e g) f, p, h => by
      rw [new_arrow _ _ (by intro x h; cases h)] at h
      obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h
      rw [place_arrow _ _ (by intro x h; cases h), new_place_sound (.arrow e g) q hq]; rfl
  | .arrow (.index e) f, p, h => by
      rw [new_arrow _ _ (by intro x h; cases h)] at h
      obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h
      rw [place_arrow _ _ (by intro x h; cases h), new_place_sound (.index e) q hq]; rfl
  | .index e, p, h => by
      have h' : (newPlace e).map (·.push .elem) = some p := h
      obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h'
      show (place e).map _ = _
      rw [new_place_sound e q hq]; rfl
  | .addr _, _, h => by cases h

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
designate the same place for a member or subscript place `X`. -/
theorem arrow_through_address_is_dot (x : Expr) (f : String) (hx : isMemberPlace x = true) :
    newPlace (.arrow (.addr x) f) = newPlace (.dot x f) := by
  show (if isMemberPlace x then (newPlace x).map (·.push (.member f)) else none) = _
  simp only [hx, ↓reduceIte]; rfl

theorem same_root_as_plain (x : Expr) (f : String) (p : Place)
    (hx : isMemberPlace x = true) (h : newPlace (.arrow (.addr x) f) = some p) :
    ∃ q, newPlace x = some q ∧ p.root = q.root := by
  rw [arrow_through_address_is_dot x f hx] at h
  have h' : (newPlace x).map (·.push (.member f)) = some p := h
  obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h'
  exact ⟨q, hq, rfl⟩

/-- An arrow over the address of a whole object (`(&m)->f`) keeps the refusal. -/
theorem a_whole_object_address_is_refused (x : Expr) (f : String)
    (hx : isMemberPlace x = false) : newPlace (.arrow (.addr x) f) = none := by
  show (if isMemberPlace x then (newPlace x).map (·.push (.member f)) else none) = _
  simp only [hx, Bool.false_eq_true, ↓reduceIte]

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
  /-- The group's tokens are member selections and balanced subscripts. -/
  selectionChain : Bool
  /-- The group's `)` is followed by `->`. -/
  groupArrow : Bool
  /-- A unary `&` stands before the group's opening parentheses. -/
  outerAddress : Bool
  deriving DecidableEq, Repr

/-- The group shape the exemption reads, the outer address aside. -/
def groupShape (o : Occurrence) : Bool :=
  o.opensGroup && o.nameArrow && o.selectionChain && o.groupArrow

/-- `dereferenced_address_group`. -/
def exempt (o : Occurrence) : Bool := groupShape o && !o.outerAddress

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
def plainBinds (o : Occurrence) : Bool := if groupShape o then o.outerAddress else true

theorem the_rule_only_removes (o : Occurrence) (h : newBinds o = true) :
    oldBinds o = true := by
  simp [newBinds] at h; exact h.1

theorem at_least_the_plain_reader (o : Occurrence) (h : plainBinds o = true) :
    newBinds o = true := by
  cases o with
  | mk g n c r a =>
      cases g <;> cases n <;> cases c <;> cases r <;> cases a <;>
        simp_all [plainBinds, newBinds, oldBinds, exempt, groupShape]

theorem agrees_with_the_plain_spelling (o : Occurrence) (h : groupShape o = true) :
    newBinds o = plainBinds o := by
  cases o with
  | mk g n c r a =>
      cases g <;> cases n <;> cases c <;> cases r <;> cases a <;>
        simp_all [plainBinds, newBinds, oldBinds, exempt, groupShape]

theorem an_exempted_address_is_a_member_address (o : Occurrence) (h : exempt o = true) :
    ampOperand o = .member := by
  cases o with
  | mk g n c r a => cases n <;> simp_all [exempt, groupShape, ampOperand]

/-- A whole-object address (`NAME` not followed by `->`) is always bound. -/
theorem a_whole_address_is_bound (o : Occurrence) (h : ampOperand o = .whole) :
    newBinds o = true := by
  cases o with
  | mk g n c r a => cases n <;> simp_all [exempt, groupShape, ampOperand, newBinds, oldBinds]

/-- An outer address keeps the binding (`&((&p->q)->f)` is `&p->q.f`). -/
theorem an_outer_address_keeps_the_binding (o : Occurrence) (h : o.outerAddress = true) :
    newBinds o = true := by
  simp [newBinds, oldBinds, exempt, h]

/-- The Rust `LEXICAL_VECTORS`, as occurrences (opens group, name arrow,
selection chain, group arrow, outer address), with the expected answer of
`dereferenced_address_group`. -/
def lexicalVectors : List (Occurrence × Bool) :=
  [ (⟨true, true, true, true, false⟩, true),     -- `( & p - > q ) - > f`
    (⟨true, true, true, true, false⟩, true),     -- `( ( & p - > q ) - > f )`
    (⟨true, true, true, true, false⟩, true),     -- `( & p - > a [ i ] . h ) - > len`
    (⟨true, true, true, true, false⟩, true),     -- `( & p - > q . r ) - > f`
    (⟨true, true, true, true, true⟩, false),     -- `& ( ( & p - > q ) - > f )`
    (⟨true, true, true, true, true⟩, false),     -- `& ( & p - > q ) - > f`
    (⟨true, false, false, true, false⟩, false),  -- `( & p ) - > f`
    (⟨true, true, true, false, false⟩, false),   -- `( & p - > q )`
    (⟨true, true, true, false, false⟩, false),   -- `( & p - > q ) . f`
    (⟨true, true, false, true, false⟩, false),   -- `( & p - > q + 1 ) - > f`
    (⟨true, false, false, true, false⟩, false),  -- `( & p . q ) - > f`
    (⟨false, true, true, false, false⟩, false),  -- `& p - > q`
    (⟨true, true, false, true, false⟩, false) ]  -- `( & p - > a [ i ) - > f`

theorem lexical_vectors_hold :
    lexicalVectors.all (fun (o, r) => exempt o == r) = true := by
  decide

/-! ## The K6 block: one definition per term, and the formulas -/

/-- The K6 block of the typed rendering (`premise_formula::RENDERED_TERMS`). -/
def k6Terms : List String := ["k6_binds", "k6_old_binds", "k6_plain_binds", "k6_exempt", "k6_opens_group", "k6_name_arrow", "k6_selection_chain", "k6_group_arrow", "k6_outer_address", "k6_place", "k6_old_place", "k6_plain_place", "k6_root", "k6_refused", "k6_over_address", "k6_over_member_address"]

/-- The K6 block's value sorts (`premise_formula::RENDERED_VALUE_SORTS`):
`k6_amp` is `Occurrence`, `k6_member` is `Expr`, `k6_place` is `Option Place`. -/
def k6Sorts : List String := ["k6_amp", "k6_member", "k6_place"]

def k6_binds : Occurrence → Bool := newBinds
def k6_old_binds : Occurrence → Bool := oldBinds
def k6_plain_binds : Occurrence → Bool := plainBinds
def k6_exempt : Occurrence → Bool := exempt
def k6_opens_group (o : Occurrence) : Bool := o.opensGroup
def k6_name_arrow (o : Occurrence) : Bool := o.nameArrow
def k6_selection_chain (o : Occurrence) : Bool := o.selectionChain
def k6_group_arrow (o : Occurrence) : Bool := o.groupArrow
def k6_outer_address (o : Occurrence) : Bool := o.outerAddress
def k6_place : Expr → Option Place := newPlace
def k6_old_place : Expr → Option Place := oldPlace

/-- The plain spelling's place: `(&X)->f` read as `X.f`; any other expression
is its own plain spelling. -/
def k6_plain_place : Expr → Option Place
  | .arrow (.addr x) f => newPlace (.dot x f)
  | e => newPlace e

def k6_root : Option Place → Option String := Option.map Place.root
def k6_refused : Option Place := none

def k6_over_address : Expr → Bool
  | .arrow (.addr _) _ => true
  | _ => false

def k6_over_member_address : Expr → Bool
  | .arrow (.addr x) _ => isMemberPlace x
  | _ => false

/-- The Rust formula table (`address_arrow_place::FORMULAS`): the admission
conditions, then the named refusals, each `(id, formula)`. -/
def formulaTable : List (String × String) := [
  ("k6_exempt_is_the_shape", "forall (#a: k6_amp) { (#k6_exempt(#a) => ((((#k6_opens_group(#a) && #k6_name_arrow(#a)) && #k6_selection_chain(#a)) && #k6_group_arrow(#a)) && !#k6_outer_address(#a))) }"),
  ("k6_the_shape_is_exempt", "forall (#a: k6_amp) { (((((#k6_opens_group(#a) && #k6_name_arrow(#a)) && #k6_selection_chain(#a)) && #k6_group_arrow(#a)) && !#k6_outer_address(#a)) => #k6_exempt(#a)) }"),
  ("k6_exempt_binds_as_the_plain_spelling", "forall (#a: k6_amp) { (#k6_exempt(#a) => (!#k6_binds(#a) && !#k6_plain_binds(#a))) }"),
  ("k6_at_least_the_plain_reader", "forall (#a: k6_amp) { (#k6_plain_binds(#a) => #k6_binds(#a)) }"),
  ("k6_only_removes_bindings", "forall (#a: k6_amp) { (#k6_binds(#a) => #k6_old_binds(#a)) }"),
  ("k6_place_is_the_plain_place", "forall (#e: k6_member) { (#k6_over_member_address(#e) => (#k6_place(#e) == #k6_plain_place(#e) && #k6_root(#k6_place(#e)) == #k6_root(#k6_plain_place(#e)))) }"),
  ("k6_place_only_adds", "forall (#e: k6_member) { (!#k6_old_place(#e) == #k6_refused => #k6_place(#e) == #k6_old_place(#e)) }"),
  ("k6_refuse_outer_address", "forall (#a: k6_amp) { (#k6_outer_address(#a) => #k6_binds(#a)) }"),
  ("k6_refuse_whole_address", "forall (#a: k6_amp) { (!#k6_name_arrow(#a) => #k6_binds(#a)) }"),
  ("k6_refuse_operator_in_group", "forall (#a: k6_amp) { (!#k6_selection_chain(#a) => #k6_binds(#a)) }"),
  ("k6_refuse_undereferenced_group", "forall (#a: k6_amp) { (!#k6_group_arrow(#a) => #k6_binds(#a)) }"),
  ("k6_refuse_no_group", "forall (#a: k6_amp) { (!#k6_opens_group(#a) => #k6_binds(#a)) }"),
  ("k6_refuse_whole_object_address", "forall (#e: k6_member) { ((#k6_over_address(#e) && !#k6_over_member_address(#e)) => #k6_place(#e) == #k6_refused) }")
]

theorem formula_k6_exempt_is_the_shape (a : Occurrence) (h : k6_exempt a = true) :
    k6_opens_group a = true ∧ k6_name_arrow a = true ∧ k6_selection_chain a = true ∧
      k6_group_arrow a = true ∧ k6_outer_address a = false := by
  cases a with
  | mk g n c r o =>
      cases g <;> cases n <;> cases c <;> cases r <;> cases o <;>
        simp_all [k6_exempt, exempt, groupShape, k6_opens_group, k6_name_arrow,
          k6_selection_chain, k6_group_arrow, k6_outer_address]

theorem formula_k6_the_shape_is_exempt (a : Occurrence)
    (h : k6_opens_group a = true ∧ k6_name_arrow a = true ∧ k6_selection_chain a = true ∧
      k6_group_arrow a = true ∧ k6_outer_address a = false) :
    k6_exempt a = true := by
  obtain ⟨h1, h2, h3, h4, h5⟩ := h
  cases a with
  | mk g n c r o =>
      simp_all [k6_exempt, exempt, groupShape, k6_opens_group, k6_name_arrow,
        k6_selection_chain, k6_group_arrow, k6_outer_address]

theorem formula_k6_exempt_binds_as_the_plain_spelling (a : Occurrence)
    (h : k6_exempt a = true) : k6_binds a = false ∧ k6_plain_binds a = false := by
  cases a with
  | mk g n c r o =>
      cases g <;> cases n <;> cases c <;> cases r <;> cases o <;>
        simp_all [k6_exempt, k6_binds, k6_plain_binds, exempt, groupShape, newBinds,
          oldBinds, plainBinds]

theorem formula_k6_at_least_the_plain_reader (a : Occurrence)
    (h : k6_plain_binds a = true) : k6_binds a = true :=
  at_least_the_plain_reader a h

theorem formula_k6_only_removes_bindings (a : Occurrence) (h : k6_binds a = true) :
    k6_old_binds a = true :=
  the_rule_only_removes a h

theorem formula_k6_place_is_the_plain_place (e : Expr) (h : k6_over_member_address e = true) :
    k6_place e = k6_plain_place e ∧ k6_root (k6_place e) = k6_root (k6_plain_place e) := by
  cases e with
  | arrow b f =>
      cases b with
      | addr x =>
          have hx : isMemberPlace x = true := h
          have heq : k6_place (.arrow (.addr x) f) = k6_plain_place (.arrow (.addr x) f) :=
            arrow_through_address_is_dot x f hx
          exact ⟨heq, by rw [heq]⟩
      | _ => cases h
  | _ => cases h

theorem formula_k6_place_only_adds (e : Expr) (h : k6_old_place e ≠ k6_refused) :
    k6_place e = k6_old_place e := by
  cases hp : oldPlace e with
  | none => exact absurd hp h
  | some p => exact (the_rule_only_adds e p hp).trans hp.symm

theorem formula_k6_refuse_outer_address (a : Occurrence) (h : k6_outer_address a = true) :
    k6_binds a = true :=
  an_outer_address_keeps_the_binding a h

theorem formula_k6_refuse_whole_address (a : Occurrence) (h : k6_name_arrow a = false) :
    k6_binds a = true := by
  simp [k6_binds, newBinds, oldBinds, exempt, groupShape, show a.nameArrow = false from h]

theorem formula_k6_refuse_operator_in_group (a : Occurrence)
    (h : k6_selection_chain a = false) : k6_binds a = true := by
  simp [k6_binds, newBinds, oldBinds, exempt, groupShape, show a.selectionChain = false from h]

theorem formula_k6_refuse_undereferenced_group (a : Occurrence)
    (h : k6_group_arrow a = false) : k6_binds a = true := by
  simp [k6_binds, newBinds, oldBinds, exempt, groupShape, show a.groupArrow = false from h]

theorem formula_k6_refuse_no_group (a : Occurrence) (h : k6_opens_group a = false) :
    k6_binds a = true := by
  simp [k6_binds, newBinds, oldBinds, exempt, groupShape, show a.opensGroup = false from h]

theorem formula_k6_refuse_whole_object_address (e : Expr)
    (h : k6_over_address e = true ∧ k6_over_member_address e = false) :
    k6_place e = k6_refused := by
  cases e with
  | arrow b f =>
      cases b with
      | addr x => exact a_whole_object_address_is_refused x f h.2
      | _ => cases h.1
  | _ => cases h.1

end QCore.AddressArrowPlace
