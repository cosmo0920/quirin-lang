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
# An untracked root accessed through pointer arithmetic is a base (card RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC, rule 74w)

Family S of `legacy_return_family` admits any use of an untracked root, but a
root that `->`, `[` or a unary `*` accesses is a **base** of the read node (a
`->` root also carries its C02-P-ALIAS reading), so the flow check stands
behind the syntactic premise. At the base the direct tests, rule 74r's
starred walk and rule 74t's member walk all stopped at a `+` or `-` between
the root and the access. So `(p + 1)->n`, `((T *)p + 1)->n`, `(p + 1)[0]`
and `*(n + p)` made no base of a null `p`, and the functions were `Safe`
(`old_misses_the_base`).

Rule 74w keeps every old reading and adds one (`arith`). The reader walks
outward from the root through **layers**: a type group before what it has
read (`cast`), a `(` and `)` right around it (`paren`), and a parenthesised
additive group around it (`sum`), which is `classified` when its other
operands are numeric literals, declared scalars or `sizeof` of a resolved
type and the root's side of it is cleanly additive. When at least one `sum`
was crossed (`crossed`) and what the walk reached is accessed (`Ctx`), the
root is a base, and a `->` root for `->`; an access through an unclassified
sum refuses the whole expression.

* Every old base and `->` root is kept (`old_base_kept`, `old_arrow_kept`).
* With no sum crossed, or nothing accessing what the walk reached, the new
  test reads nothing (`no_sum_keeps_the_reading`,
  `no_access_keeps_the_reading`): rule 74r's and 74t's readings, a value
  `p + 1` and a comparison stand as they were.
* A base or a `->` root only adds an obligation, and the one new refusal is
  the expression's, so a result the tip admits the base admitted too
  (`the_fix_only_refuses_more`). No reading admits more than at the base.

## What this is not

A model of the token scan or of the flow check. Each layer's and each
context's class is the Rust reader's output (trust boundary 28), bound by
`arithVectors`, whose old fields the Rust test also checks against the
functions the base already had. Obligations are abstracted as predicates the
flow check decides.
-/

namespace QCore.ReturnFamilyUntrackedArithAccess

/-- One widening step of the walk outward from the root, innermost first. -/
inductive Layer where
  /-- A group `type_group_open` reads as a type, before what was read. -/
  | cast
  /-- A `(` and `)` right around what was read. -/
  | paren
  /-- A parenthesised additive group around what was read; `true` when its
  other operands can never be a pointer and the root's side is additive. -/
  | sum (classified : Bool)
  deriving DecidableEq, Repr

/-- What accesses the span the walk reached. -/
inductive Ctx where
  /-- `- >` after it. -/
  | arrow
  /-- `[` after it, or a `*` that may be unary before it. -/
  | element
  /-- Nothing that accesses it. -/
  | none
  deriving DecidableEq, Repr

/-- Rule 74w's reading of one untracked, non-scalar root. -/
inductive Outcome where
  | none
  | arrow
  | element
  | unclassified
  deriving DecidableEq, Repr

def isSum : Layer → Bool
  | .sum _ => true
  | _ => false

def sumClassified : Layer → Bool
  | .sum c => c
  | _ => true

/-- At least one additive group was crossed. -/
def crossed (layers : List Layer) : Bool := layers.any isSum

/-- Every crossed additive group was classified. -/
def classified (layers : List Layer) : Bool := layers.all sumClassified

/-- `accessed_through_arithmetic`. -/
def arith (layers : List Layer) (ctx : Ctx) : Outcome :=
  if crossed layers then
    match ctx with
    | .none => .none
    | .arrow => if classified layers then .arrow else .unclassified
    | .element => if classified layers then .element else .unclassified
  else .none

theorem no_sum_keeps_the_reading (layers : List Layer) (ctx : Ctx)
    (h : crossed layers = false) : arith layers ctx = .none := by
  simp [arith, h]

theorem no_access_keeps_the_reading (layers : List Layer) :
    arith layers .none = .none := by
  unfold arith
  split <;> rfl

theorem a_classified_sum_is_accessed (layers : List Layer) (ctx : Ctx)
    (hc : crossed layers = true) (hk : classified layers = true) (ha : ctx ≠ .none) :
    arith layers ctx ≠ .none ∧ arith layers ctx ≠ .unclassified := by
  cases ctx <;> simp_all [arith]

theorem an_unclassified_access_refuses (layers : List Layer) (ctx : Ctx)
    (hc : crossed layers = true) (hk : classified layers = false) (ha : ctx ≠ .none) :
    arith layers ctx = .unclassified := by
  cases ctx <;> simp_all [arith]

/-! ## The read -/

/-- One untracked root: its name, its layers and context, and the base's
reading of it (the direct tests, rule 74r's and rule 74t's walks). -/
structure Root where
  name : String
  layers : List Layer
  ctx : Ctx
  oldBase : Bool
  oldArrow : Bool
  deriving DecidableEq, Repr

/-- Rule 74w's base: the old reading or an access through arithmetic. -/
def base (r : Root) : Bool := r.oldBase || arith r.layers r.ctx != .none

/-- Rule 74w's `->` root. -/
def arrowRoot (r : Root) : Bool := r.oldArrow || arith r.layers r.ctx == .arrow

/-- The root refuses the expression. -/
def refuses (r : Root) : Bool := arith r.layers r.ctx == .unclassified

theorem old_base_kept (r : Root) (h : r.oldBase = true) : base r = true := by
  simp [base, h]

theorem old_arrow_kept (r : Root) (h : r.oldArrow = true) : arrowRoot r = true := by
  simp [arrowRoot, h]

/-- The base's family S result: `none` refuses; else the bases and the `->`
roots. `refused` is every other refusal, which the card does not touch. -/
def oldRead (refused : Bool) (roots : List Root) : Option (List String × List String) :=
  if refused then none
  else some ((roots.filter (·.oldBase)).map (·.name), (roots.filter (·.oldArrow)).map (·.name))

/-- The tip's family S result. -/
def newRead (refused : Bool) (roots : List Root) : Option (List String × List String) :=
  if refused || roots.any refuses then none
  else some ((roots.filter base).map (·.name), (roots.filter arrowRoot).map (·.name))

/-- The flow check admits only when every base's obligation `ok` and every
`->` root's C02-P-ALIAS obligation `alias` discharge. -/
def admits (ok alias : String → Bool) : Option (List String × List String) → Bool
  | none => false
  | some (bases, arrows) => bases.all ok && arrows.all alias

theorem names_kept (old new : Root → Bool) (h : ∀ r, old r = true → new r = true)
    (roots : List Root) (name : String)
    (hn : name ∈ (roots.filter old).map (·.name)) : name ∈ (roots.filter new).map (·.name) := by
  simp only [List.mem_map, List.mem_filter] at hn ⊢
  obtain ⟨r, ⟨hm, ho⟩, he⟩ := hn
  exact ⟨r, ⟨hm, h r ho⟩, he⟩

/-- Bases and `->` roots only grow and the only new refusal is the
expression's, so a result the tip admits, the base admitted. -/
theorem the_fix_only_refuses_more (ok alias : String → Bool) (refused : Bool)
    (roots : List Root) (h : admits ok alias (newRead refused roots) = true) :
    admits ok alias (oldRead refused roots) = true := by
  unfold newRead at h
  unfold oldRead
  cases hr : refused with
  | true => simp [hr, admits] at h
  | false =>
    cases ha : roots.any refuses with
    | true => simp [hr, ha, admits] at h
    | false =>
      simp only [hr, ha, Bool.or_false, Bool.false_eq_true, ite_false, admits,
        Bool.and_eq_true, List.all_eq_true] at h ⊢
      obtain ⟨hb, hw⟩ := h
      exact ⟨fun name hn => hb name (names_kept _ _ old_base_kept roots name hn),
        fun name hn => hw name (names_kept _ _ old_arrow_kept roots name hn)⟩

/-! ## The readings -/

/-- `( p + 1 ) - > n`: no old test read it; the tip makes `p` a base and a
`->` root. -/
theorem old_misses_the_base :
    let r : Root := ⟨"p", [.sum true], .arrow, false, false⟩
    base r = true ∧ arrowRoot r = true ∧ refuses r = false := by
  decide

/-- `( p - q ) - > n`: an access through a sum with another pointer refuses. -/
theorem another_pointer_refuses :
    refuses ⟨"p", [.sum false], .arrow, false, false⟩ = true := by
  decide

/-! ## The token reader's vectors -/

/-- `(spelling, layers, context, old base, old -> root, base, -> root,
refused)` for the untracked `p`. The Rust unit test reads the spelling, the
old fields (against the base's functions) and the last three (against
`scalar_read`). -/
def arithVectors : List (String × List Layer × Ctx × Bool × Bool × Bool × Bool × Bool) := [
  ("( p + 1 ) - > n", [.sum true], .arrow, false, false, true, true, false),
  ("( n + p ) - > m", [.sum true], .arrow, false, false, true, true, false),
  ("( p - n ) - > m", [.sum true], .arrow, false, false, true, true, false),
  ("( ( long * ) p + n ) - > m", [.cast, .sum true], .arrow, false, false, true, true, false),
  ("( ( long * ) ( p + n ) ) - > m", [.sum true, .cast, .paren], .arrow,
    false, false, true, true, false),
  ("( p + n ) [ 0 ]", [.sum true], .element, false, false, true, false, false),
  ("( n + ( p ) ) - > m", [.paren, .sum true], .arrow, false, false, true, true, false),
  ("( ( p + 1 ) + n ) [ 0 ]", [.sum true, .sum true], .element, false, false, true, false, false),
  ("( p + n * 2 ) - > m", [.sum true], .arrow, false, false, true, true, false),
  ("( p + sizeof ( long ) ) - > m", [.sum true], .arrow, false, false, true, true, false),
  ("* ( n + p )", [.sum true], .element, false, false, true, false, false),
  ("* ( p + 1 )", [.sum true], .element, true, false, true, false, false),
  ("( p + 1 ) - > n + ( p - 1 ) [ 0 ]", [.sum true], .arrow, false, false, true, true, false),
  ("( p - a ) - > m", [.sum false], .arrow, false, false, true, false, true),
  ("( a + p ) [ 0 ]", [.sum false], .element, false, false, true, false, true),
  ("( n % p ) - > m", [.sum false], .arrow, false, false, true, false, true),
  ("( - p + 1 ) - > m", [], .none, false, false, false, false, false),
  ("( p + 1 ) == a", [.sum true], .none, false, false, false, false, false),
  ("p + 1", [], .none, false, false, false, false, false),
  ("( p + 1 )", [.sum true], .none, false, false, false, false, false),
  ("( ( long * ) p ) - > n", [.cast, .paren], .arrow, true, true, true, true, false),
  ("p - > n + 1", [], .none, true, true, true, true, false),
  ("n * ( p + 1 )", [.sum true], .none, false, false, false, false, false)
]

def arithVectorsAgree : Bool :=
  arithVectors.all (fun (name, layers, ctx, oldB, oldA, b, a, refused) =>
    let r : Root := ⟨name, layers, ctx, oldB, oldA⟩
    base r == b && arrowRoot r == a && refuses r == refused)

theorem arith_vectors_hold : arithVectorsAgree = true := by decide

end QCore.ReturnFamilyUntrackedArithAccess
