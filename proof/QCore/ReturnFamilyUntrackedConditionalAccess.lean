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

import QCore.ReturnFamilyUntrackedArithAccess

/-!
# Both arms of an accessed conditional are bases (card RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS, rule 74x)

Family S of `legacy_return_family` admits any use of an untracked root, but a
root that `->`, `[` or a unary `*` accesses is a **base** of the read node (a
`->` root also carries its C02-P-ALIAS reading), so the flow check stands
behind the syntactic premise. At the base no test read a root that is an arm
of a parenthesised conditional: the direct tests need the access next to the
name, and rules 74r's, 74t's and 74w's walks stop at the `?` or `:` beside
it. So `(0 ? p : q)->n`, `(c ? p : q)[0]` and `*(c ? p : q)` made no base of
null `p` and `q`, and the functions were `Safe` (`old_misses_the_base`).

Rule 74x keeps every old reading and adds one. Each parenthesised
conditional **group** is widened outward through rule 74w's layers (a type
group, parentheses, an additive group, `classified` or not); when what it
reached is accessed (`Ctx`), both arms are read (`armRoots`). An arm is a
root, an inert value (a literal or a declared scalar), a loaded value (whose
roots the old tests read), a parenthesised or cast arm, the one pointer term
of a sum, a nested conditional, or something the reader cannot classify.

* **Both arms, never the condition.** A conditional's roots are both arms'
  (`both_arms_are_taken`), whatever the condition, constant or not
  (`the_condition_is_not_decided`).
* **Fail closed.** An arm the reader cannot classify, or an access through an
  unclassified additive group, refuses the whole expression
  (`an_unclassifiable_arm_refuses`, `an_unclassified_group_refuses`). A group
  nothing accesses reads nothing (`no_access_keeps_the_reading`).
* **Nothing admits more.** Every old base and `->` root is kept
  (`old_base_kept`, `old_arrow_kept`); a base or a `->` root only adds an
  obligation, and the one new refusal is the expression's, so a result the
  tip admits, the base admitted (`the_fix_only_refuses_more`).

## What this is not

A model of the token scan or of the flow check. Each group's layers, context
and arms are the Rust reader's output (trust boundary 28), bound by
`condVectors`, whose old fields the Rust test also checks against the
functions the base already had. Obligations are abstracted as predicates the
flow check decides.
-/

namespace QCore.ReturnFamilyUntrackedConditionalAccess

open QCore.ReturnFamilyUntrackedArithAccess (Layer Ctx Outcome classified)

/-- One arm of an accessed conditional, as `arm_roots` classifies it. -/
inductive Arm where
  /-- A name this body never binds, not a declared scalar. -/
  | root (name : String)
  /-- A numeric literal or a declared scalar: no root. -/
  | inert
  /-- `a - > f`, `a [ i ]`, `* a`: a loaded value whose roots the old tests
  read. -/
  | loaded
  /-- `( a )`. -/
  | paren (a : Arm)
  /-- `( T ) a`. -/
  | cast (a : Arm)
  /-- The one pointer term `a` of a sum; `true` when every other term can
  never be a pointer and `a` is not subtracted. -/
  | sum (classified : Bool) (a : Arm)
  /-- `c ? a : b`; `cond` is the condition's value when constant. -/
  | cond (cond : Option Bool) (a b : Arm)
  /-- A string, a unary `-`, a product, a may-bind name, anything else. -/
  | other
  deriving DecidableEq, Repr

/-- `arm_roots`: the roots an arm yields, `none` when unclassified. -/
def armRoots : Arm → Option (List String)
  | .root n => some [n]
  | .inert => some []
  | .loaded => some []
  | .paren a => armRoots a
  | .cast a => armRoots a
  | .sum c a => if c then armRoots a else none
  | .cond _ a b =>
    match armRoots a, armRoots b with
    | some x, some y => some (x ++ y)
    | _, _ => none
  | .other => none

theorem both_arms_are_taken (c : Option Bool) (a b : Arm) (x y : List String)
    (ha : armRoots a = some x) (hb : armRoots b = some y) :
    armRoots (.cond c a b) = some (x ++ y) := by
  simp [armRoots, ha, hb]

/-- The reading never depends on the condition, so a constant condition
prunes no arm. -/
theorem the_condition_is_not_decided (c c' : Option Bool) (a b : Arm) :
    armRoots (.cond c a b) = armRoots (.cond c' a b) := rfl

theorem an_unclassifiable_arm_refuses (c : Option Bool) (a b : Arm)
    (h : armRoots a = none) :
    armRoots (.cond c a b) = none ∧ armRoots (.cond c b a) = none := by
  constructor
  · simp [armRoots, h]
  · simp only [armRoots, h]
    split <;> simp_all

/-- One parenthesised conditional group: its layers outward, what accesses
what they reached, and its arms (a `cond`). -/
structure Group where
  layers : List Layer
  ctx : Ctx
  arms : Arm
  deriving DecidableEq, Repr

/-- `conditional_group_access`: rule 74w's widening without its need to cross
an additive group. -/
def access (g : Group) : Outcome :=
  match g.ctx with
  | .none => .none
  | .arrow => if classified g.layers then .arrow else .unclassified
  | .element => if classified g.layers then .element else .unclassified

/-- `conditional_roots` for one group: `none` refuses. -/
def groupRoots (g : Group) : Option (List String) :=
  match access g with
  | .none => some []
  | .unclassified => none
  | _ => armRoots g.arms

theorem no_access_keeps_the_reading (layers : List Layer) (arms : Arm) :
    groupRoots ⟨layers, .none, arms⟩ = some [] := rfl

theorem an_unclassified_group_refuses (g : Group) (h : access g = .unclassified) :
    groupRoots g = none := by
  simp [groupRoots, h]

/-- The name is a root of an accessed group. -/
def rootOf (gs : List Group) (name : String) : Bool :=
  gs.any (fun g => ((groupRoots g).getD []).contains name)

/-- The name is a root of a group `->` accesses. -/
def arrowOf (gs : List Group) (name : String) : Bool :=
  gs.any (fun g => access g == .arrow && ((groupRoots g).getD []).contains name)

/-- Some accessed group refuses. -/
def groupsRefuse (gs : List Group) : Bool := gs.any (fun g => groupRoots g == none)

/-! ## The read -/

/-- One untracked root and the base's reading of it (the direct tests and
rules 74r's, 74t's and 74w's walks). -/
structure Root where
  name : String
  oldBase : Bool
  oldArrow : Bool
  deriving DecidableEq, Repr

def base (gs : List Group) (r : Root) : Bool := r.oldBase || rootOf gs r.name

def arrowRoot (gs : List Group) (r : Root) : Bool := r.oldArrow || arrowOf gs r.name

theorem old_base_kept (gs : List Group) (r : Root) (h : r.oldBase = true) :
    base gs r = true := by
  simp [base, h]

theorem old_arrow_kept (gs : List Group) (r : Root) (h : r.oldArrow = true) :
    arrowRoot gs r = true := by
  simp [arrowRoot, h]

/-- The base's family S result: `none` refuses; else the bases and the `->`
roots. `refused` is every other refusal, which the card does not touch. -/
def oldRead (refused : Bool) (roots : List Root) : Option (List String × List String) :=
  if refused then none
  else some ((roots.filter (·.oldBase)).map (·.name), (roots.filter (·.oldArrow)).map (·.name))

/-- The tip's family S result. -/
def newRead (refused : Bool) (gs : List Group) (roots : List Root) :
    Option (List String × List String) :=
  if refused || groupsRefuse gs then none
  else some ((roots.filter (base gs)).map (·.name), (roots.filter (arrowRoot gs)).map (·.name))

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
    (gs : List Group) (roots : List Root) (h : admits ok alias (newRead refused gs roots) = true) :
    admits ok alias (oldRead refused roots) = true := by
  unfold newRead at h
  unfold oldRead
  cases hr : refused with
  | true => simp [hr, admits] at h
  | false =>
    cases hg : groupsRefuse gs with
    | true => simp [hr, hg, admits] at h
    | false =>
      simp only [hr, hg, Bool.or_false, Bool.false_eq_true, ite_false, admits,
        Bool.and_eq_true, List.all_eq_true] at h ⊢
      obtain ⟨hb, hw⟩ := h
      exact ⟨fun name hn => hb name (names_kept _ _ (old_base_kept gs) roots name hn),
        fun name hn => hw name (names_kept _ _ (old_arrow_kept gs) roots name hn)⟩

/-! ## The readings -/

/-- `( 0 ? p : q ) - > n`: no old test read it; the tip makes both arms bases
and `->` roots, the constant condition notwithstanding. -/
theorem old_misses_the_base :
    let gs : List Group := [⟨[], .arrow, .cond (some false) (.root "p") (.root "q")⟩]
    base gs ⟨"p", false, false⟩ = true ∧ arrowRoot gs ⟨"p", false, false⟩ = true ∧
      base gs ⟨"q", false, false⟩ = true ∧ groupsRefuse gs = false := by
  decide

/-- `( p ? q : r ) - > n`: the condition is a read, not an arm. -/
theorem the_condition_is_no_base :
    base [⟨[], .arrow, .cond none (.root "q") (.root "r")⟩] ⟨"p", false, false⟩ = false := by
  decide

/-! ## The token reader's vectors -/

/-- `(spelling, groups, old base, old -> root, base, -> root, refused)` for
the untracked `p`. The Rust unit test reads the spelling, the old fields
(against the base's functions) and the last three (against `scalar_read`). -/
def condVectors : List (String × List Group × Bool × Bool × Bool × Bool × Bool) := [
  ("( n - n ? p : a ) - > n", [⟨[], .arrow, .cond none (.root "p") (.root "a")⟩],
    false, false, true, true, false),
  ("( 1 / 0 ? a : p ) - > n", [⟨[], .arrow, .cond none (.root "a") (.root "p")⟩],
    false, false, true, true, false),
  ("( n ? p : a ) [ 0 ]", [⟨[], .element, .cond none (.root "p") (.root "a")⟩],
    false, false, true, false, false),
  ("* ( n ? p : a )", [⟨[], .element, .cond none (.root "p") (.root "a")⟩],
    false, false, true, false, false),
  ("( n ? ( n > 1 ? p : a ) : b ) - > m",
    [⟨[], .arrow, .cond none (.paren (.cond none (.root "p") (.root "a"))) (.root "b")⟩],
    false, false, true, true, false),
  ("( n ? a : n > 1 ? b : p ) - > m",
    [⟨[], .arrow, .cond none (.root "a") (.cond none (.root "b") (.root "p"))⟩],
    false, false, true, true, false),
  ("( ( long * ) ( n ? p : a ) ) [ 0 ]",
    [⟨[.cast, .paren], .element, .cond none (.root "p") (.root "a")⟩],
    false, false, true, false, false),
  ("( n ? ( long * ) p : a ) [ 0 ]",
    [⟨[], .element, .cond none (.cast (.root "p")) (.root "a")⟩],
    false, false, true, false, false),
  ("( n ? p + 1 : a ) - > m", [⟨[], .arrow, .cond none (.sum true (.root "p")) (.root "a")⟩],
    false, false, true, true, false),
  ("( ( n ? p : a ) + 1 ) - > m", [⟨[.sum true], .arrow, .cond none (.root "p") (.root "a")⟩],
    false, false, true, true, false),
  ("* ( long * ) ( n ? p : a )", [⟨[.cast], .element, .cond none (.root "p") (.root "a")⟩],
    false, false, true, false, false),
  ("( n ? p : 0 ) - > m", [⟨[], .arrow, .cond none (.root "p") .inert⟩],
    false, false, true, true, false),
  ("( n ? a - > next : p ) - > m", [⟨[], .arrow, .cond none .loaded (.root "p")⟩],
    false, false, true, true, false),
  ("( p ? a : b ) - > m", [⟨[], .arrow, .cond none (.root "a") (.root "b")⟩],
    false, false, false, false, false),
  ("( n ? p : - a ) - > m", [⟨[], .arrow, .cond none (.root "p") .other⟩],
    false, false, false, false, true),
  ("( n ? p : a * 2 ) [ 0 ]", [⟨[], .element, .cond none (.root "p") .other⟩],
    false, false, false, false, true),
  ("( n ? p - a : b ) [ 0 ]", [⟨[], .element, .cond none (.sum false (.root "p")) (.root "b")⟩],
    false, false, false, false, true),
  ("( ( n ? p : a ) - b ) - > m", [⟨[.sum false], .arrow, .cond none (.root "p") (.root "a")⟩],
    false, false, false, false, true),
  ("n ? p - > m : a - > m", [], true, true, true, true, false),
  ("( n ? p - > m : a - > m )", [⟨[], .none, .cond none .loaded .loaded⟩],
    true, true, true, true, false),
  ("( n ? p : a ) != 0", [⟨[], .none, .cond none (.root "p") (.root "a")⟩],
    false, false, false, false, false),
  ("( n ? p : a ) - > m + p - > m", [⟨[], .arrow, .cond none (.root "p") (.root "a")⟩],
    false, false, true, true, false),
  ("( ( p ) ) - > m", [], true, true, true, true, false)
]

def condVectorsAgree : Bool :=
  condVectors.all (fun (name, gs, oldB, oldA, b, a, refused) =>
    let r : Root := ⟨"p", oldB, oldA⟩
    let _ := name
    base gs r == b && arrowRoot gs r == a && groupsRefuse gs == refused)

theorem cond_vectors_hold : condVectorsAgree = true := by decide

end QCore.ReturnFamilyUntrackedConditionalAccess
