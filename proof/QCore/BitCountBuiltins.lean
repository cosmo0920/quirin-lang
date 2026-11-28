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
# Bit-count builtins under a nonzero guard (card K2-BUILTIN-BIT-COUNTS, rule 94)

The abstract rule implemented by `crates/qcore-c-gateway/src/bit_count_builtins.rs`
(`BUILTINS`, `proves`, `decide`, `neutral_contract`) and its one hook
(`legacy_c::build_function_cfg_with`), on the user's decision of 2026-11-28
(option (a), obligation-bearing).

## The builtins

`__builtin_c{l,t}z{,l,ll}(x)` count the leading or trailing zero bits of an
unsigned `x` of width `w`; the result is undefined when `x = 0` (`eval` is
`none` there: `undefined_at_zero`) and defined everywhere else
(`defined_off_zero`). They read no memory and write nothing: the contract a
discharged call is read with borrows its one scalar and returns no custody,
the identity on the custody state (`the_neutral_contract_is_the_identity`).

## The guard

`Cond` is a controlling expression over one object `x`: `val` is its C value
(`!`, comparisons, `&&` and `||` yield `0` or `1`), with every subexpression
that does not read `x` carrying an arbitrary value (`other`). `proves c e` is
the Rust `proves`; it is sound: when `c` evaluates to edge `e` (non-zero:
`true`), `x ≠ 0` (`proves_sound`). A read through an integer conversion is
sound for the guard because a conversion maps zero to zero
(`conversion_keeps_zero`); the operand of the call may be converted only to
an unsigned type of no lower rank, which keeps a nonzero value nonzero
(`widening_keeps_nonzero`).

## The decision

`decide` is the Rust decision over the site facts. An admitted site
(`admitted_exactly_when`) has a named, stable operand and a guard on its path
that proves `x ≠ 0` on the edge the call is reached by; with the object
stable, the value the guard tested is the value the call reads, so the
operand is nonzero (`an_admitted_site_has_a_nonzero_operand`) and the
builtin is defined there (`an_admitted_call_is_defined`). An unguarded site
(an `assert(x != 0)` under `NDEBUG` is `((void)0)`, no guard), a written
object or a narrowed operand is refused (`an_unguarded_site_is_refused`,
`an_unstable_object_is_refused`, `a_narrowed_operand_is_refused`); a
statement guard needs structured flow (`a_statement_guard_needs_structured_flow`).

## What this is not

A model of Clang: the site facts (the operand, the stability of the object,
the guards on the path, the structured-flow test) are the Rust reader's
outputs (trust boundary 28).
-/

namespace QCore.BitCountBuiltins

/-! ## The builtins -/

def builtinNames : List String := ["__builtin_clz", "__builtin_ctz", "__builtin_clzl", "__builtin_ctzl", "__builtin_clzll", "__builtin_ctzll"]

inductive Kind where
  | clz
  | ctz
  deriving DecidableEq, Repr

/-- Trailing zeros of `x`, looking at most `fuel` bits. -/
def trailing : Nat → Nat → Nat
  | 0, _ => 0
  | fuel + 1, x => if x % 2 = 1 then 0 else trailing fuel (x / 2) + 1

/-- The count on a nonzero operand of width `w`. -/
def count : Kind → Nat → Nat → Nat
  | .clz, w, x => w - 1 - Nat.log2 x
  | .ctz, w, x => trailing w x

/-- The builtin's result: undefined (`none`) at a zero operand. -/
def eval (k : Kind) (w x : Nat) : Option Nat :=
  if x = 0 then none else some (count k w x)

theorem undefined_at_zero (k : Kind) (w : Nat) : eval k w 0 = none := by
  simp [eval]

theorem defined_off_zero (k : Kind) (w x : Nat) (h : x ≠ 0) : (eval k w x).isSome = true := by
  simp [eval, h]

/-- A parameter mode of the neutral contract; `borrowed` changes no custody. -/
inductive Mode where
  | borrowed
  deriving DecidableEq, Repr

/-- The custody effect of a call read with all-`borrowed` modes and no result
custody: the identity on any custody state. -/
def neutralEffect {σ : Type} (_modes : List Mode) (s : σ) : σ := s

theorem the_neutral_contract_is_the_identity {σ : Type} (s : σ) :
    neutralEffect [Mode.borrowed] s = s := rfl

/-! ## Conversions -/

theorem conversion_keeps_zero (b : Int) : (0 : Int) % b = 0 := Int.zero_emod b

/-- An integer conversion into an unsigned type of `2^n = b` values, from a
source whose every value is below `b` in magnitude (no lower rank), keeps a
nonzero value nonzero (C11 6.3.1.3p2: the value modulo `b`). -/
theorem widening_keeps_nonzero (v b : Int) (hb : 0 < b) (lo : -b < v) (hi : v < b)
    (h : v ≠ 0) : v % b ≠ 0 := by
  intro hz
  have hdiv := Int.emod_def v b
  rw [hz] at hdiv
  -- `v = b * q` with `-b < v < b` and `b > 0`: `q = 0`, so `v = 0`.
  have hq0 : v / b ≠ 0 := fun hq => h (by rw [hq] at hdiv; omega)
  rcases Int.lt_or_gt_of_ne hq0 with hq | hq
  · have : b * (v / b) ≤ b * (-1) := Int.mul_le_mul_of_nonneg_left (by omega) (by omega)
    omega
  · have : b * 1 ≤ b * (v / b) := Int.mul_le_mul_of_nonneg_left (by omega) (by omega)
    omega

/-! ## The guard -/

inductive Op where
  | ne | eq | gt | lt | le | ge
  deriving DecidableEq, Repr

inductive Cond where
  | read
  | zero
  | not (c : Cond)
  | cmp (op : Op) (l r : Cond)
  | and (l r : Cond)
  | or (l r : Cond)
  /-- A subexpression that does not read `x`, with its value. -/
  | other (value : Int)
  deriving Repr

def Op.holds : Op → Int → Int → Bool
  | .ne, a, b => a != b
  | .eq, a, b => a == b
  | .gt, a, b => decide (a > b)
  | .lt, a, b => decide (a < b)
  | .le, a, b => decide (a ≤ b)
  | .ge, a, b => decide (a ≥ b)

def bit (b : Bool) : Int := if b then 1 else 0

/-- The C value of the condition when `x` has value `v`. -/
def val (v : Int) : Cond → Int
  | .read => v
  | .zero => 0
  | .not c => bit (val v c == 0)
  | .cmp op l r => bit (op.holds (val v l) (val v r))
  | .and l r => bit (val v l != 0 && val v r != 0)
  | .or l r => bit (val v l != 0 || val v r != 0)
  | .other n => n

def isRead : Cond → Bool
  | .read => true
  | _ => false

def isZero : Cond → Bool
  | .zero => true
  | _ => false

/-- The Rust `proves`. -/
def proves : Cond → Bool → Bool
  | .read, e => e
  | .not c, e => proves c (!e)
  | .cmp op l r, e =>
      let rz := isRead l && isZero r
      let zr := isZero l && isRead r
      match op with
      | .ne => e && (rz || zr)
      | .eq => !e && (rz || zr)
      | .gt => e && rz
      | .lt => e && zr
      | .le => !e && rz
      | .ge => !e && zr
  | .and l r, e => e && (proves l true || proves r true)
  | .or l r, e => !e && (proves l false || proves r false)
  | _, _ => false

/-- The edge a condition takes: non-zero is `true`. -/
def edge (v : Int) (c : Cond) : Bool := val v c != 0

theorem isRead_val {c : Cond} (h : isRead c = true) (v : Int) : val v c = v := by
  cases c <;> simp_all [isRead, val]

theorem isZero_val {c : Cond} (h : isZero c = true) (v : Int) : val v c = 0 := by
  cases c <;> simp_all [isZero, val]

theorem read_of_isRead {c : Cond} (h : isRead c = true) : c = .read := by
  cases c <;> simp_all [isRead]

theorem zero_of_isZero {c : Cond} (h : isZero c = true) : c = .zero := by
  cases c <;> simp_all [isZero]

/-- A comparison proves `x ≠ 0` only between `x` and `0`. -/
theorem cmp_shape {op : Op} {l r : Cond} {e : Bool} (hp : proves (.cmp op l r) e = true) :
    (isRead l = true ∧ isZero r = true) ∨ (isZero l = true ∧ isRead r = true) := by
  cases op <;> cases e <;> simp [proves] at hp <;>
    first | exact hp | exact Or.inl hp | exact Or.inr hp

theorem proves_sound (c : Cond) :
    ∀ (e : Bool) (v : Int), proves c e = true → edge v c = e → v ≠ 0 := by
  induction c with
  | read => intro e v hp he; simp_all [proves, edge, val]
  | zero => intro e v hp; simp [proves] at hp
  | other n => intro e v hp; simp [proves] at hp
  | not c ih =>
      intro e v hp he
      apply ih (!e) v hp
      cases e <;> simp_all [edge, val, bit] <;> split at he <;> simp_all
  | cmp op l r _ _ =>
      intro e v hp he
      rcases cmp_shape hp with ⟨hl, hr⟩ | ⟨hl, hr⟩
      · rw [read_of_isRead hl, zero_of_isZero hr] at hp he
        cases op <;> cases e <;> simp [proves, isRead, isZero] at hp <;>
          simp [edge, val, bit, Op.holds] at he <;> (try omega) <;> (intro hz; subst hz; simp at he)
      · rw [zero_of_isZero hl, read_of_isRead hr] at hp he
        cases op <;> cases e <;> simp [proves, isRead, isZero] at hp <;>
          simp [edge, val, bit, Op.holds] at he <;> (try omega) <;> (intro hz; subst hz; simp at he)
  | and l r ihl ihr =>
      intro e v hp he
      simp only [proves, Bool.and_eq_true, Bool.or_eq_true] at hp
      obtain ⟨rfl, hp⟩ := hp
      simp only [edge, val, bit] at he
      split at he
      · rename_i hboth
        simp only [Bool.and_eq_true, bne_iff_ne, ne_eq] at hboth
        rcases hp with hl | hr
        · exact ihl true v hl (by simp [edge, hboth.1])
        · exact ihr true v hr (by simp [edge, hboth.2])
      · simp at he
  | or l r ihl ihr =>
      intro e v hp he
      simp only [proves, Bool.and_eq_true, Bool.or_eq_true, Bool.not_eq_true'] at hp
      obtain ⟨rfl, hp⟩ := hp
      simp only [edge, val, bit] at he
      split at he
      · simp at he
      · rename_i hnone
        simp only [Bool.or_eq_true, bne_iff_ne, ne_eq, not_or, Decidable.not_not] at hnone
        rcases hp with hl | hr
        · exact ihl false v hl (by simp [edge, hnone.1])
        · exact ihr false v hr (by simp [edge, hnone.2])

/-! ## The decision -/

inductive Guard where
  | expression (c : Cond) (e : Bool)
  | branch (c : Cond) (e : Bool)
  | earlyExit (c : Cond)
  deriving Repr

structure SiteFacts where
  namedOperand : Bool
  stable : Bool
  structured : Bool
  guards : List Guard
  deriving Repr

inductive Reason where
  | operand | unstable | unguarded
  deriving DecidableEq, Repr

def Guard.discharges (structured : Bool) : Guard → Bool
  | .expression c e => proves c e
  | .branch c e => structured && proves c e
  | .earlyExit c => structured && proves c false

/-- The Rust `decide`. -/
def decide (f : SiteFacts) : Except Reason Unit :=
  if !f.namedOperand then .error .operand
  else if !f.stable then .error .unstable
  else if f.guards.any (Guard.discharges f.structured) then .ok ()
  else .error .unguarded

def code : Except Reason Unit → String
  | .ok () => "admit"
  | .error .operand => "bit-count-operand-not-a-named-object"
  | .error .unstable => "bit-count-operand-object-not-stable"
  | .error .unguarded => "bit-count-operand-not-proved-nonzero"

theorem admitted_exactly_when (f : SiteFacts) :
    decide f = .ok () ↔
      (f.namedOperand = true ∧ f.stable = true ∧
        ∃ g ∈ f.guards, g.discharges f.structured = true) := by
  unfold decide
  cases f.namedOperand <;> cases f.stable <;> simp <;>
    split <;> rename_i h <;> simp_all [List.any_eq_true]

/-- The guard held on the way to the call: its condition, with `x = v`, took
the edge the call is reached by. -/
def Guard.held (v : Int) : Guard → Prop
  | .expression c e => edge v c = e
  | .branch c e => edge v c = e
  | .earlyExit c => edge v c = false

def Guard.isExpression : Guard → Bool
  | .expression _ _ => true
  | _ => false

/-- Every guard on the path held at the value the call reads: an expression
guard always (same full expression, the object stable); a statement guard
when the flow is structured (no label lets the call be reached around it). -/
def Reached (f : SiteFacts) (v : Int) : Prop :=
  ∀ g ∈ f.guards, (g.isExpression = true ∨ f.structured = true) → g.held v

theorem an_admitted_site_has_a_nonzero_operand (f : SiteFacts) (v : Int)
    (h : decide f = .ok ()) (reach : Reached f v) : v ≠ 0 := by
  obtain ⟨_, _, g, hg, hd⟩ := (admitted_exactly_when f).1 h
  cases g with
  | expression c e =>
      exact proves_sound c e v hd (reach _ hg (Or.inl rfl))
  | branch c e =>
      simp only [Guard.discharges, Bool.and_eq_true] at hd
      exact proves_sound c e v hd.2 (reach _ hg (Or.inr hd.1))
  | earlyExit c =>
      simp only [Guard.discharges, Bool.and_eq_true] at hd
      exact proves_sound c false v hd.2 (reach _ hg (Or.inr hd.1))

theorem an_admitted_call_is_defined (f : SiteFacts) (k : Kind) (w : Nat) (x : Nat)
    (h : decide f = .ok ()) (reach : Reached f x) : (eval k w x).isSome = true :=
  defined_off_zero k w x (fun hx => an_admitted_site_has_a_nonzero_operand f x h reach
    (by simp [hx]))

theorem an_unguarded_site_is_refused (f : SiteFacts) (h : f.guards = []) :
    decide f ≠ .ok () := by
  intro hd
  obtain ⟨_, _, g, hg, _⟩ := (admitted_exactly_when f).1 hd
  simp [h] at hg

theorem an_unstable_object_is_refused (f : SiteFacts) (h : f.stable = false) :
    decide f ≠ .ok () := by
  intro hd
  have := ((admitted_exactly_when f).1 hd).2.1
  simp_all

theorem a_narrowed_operand_is_refused (f : SiteFacts) (h : f.namedOperand = false) :
    decide f ≠ .ok () := by
  intro hd
  have := ((admitted_exactly_when f).1 hd).1
  simp_all

theorem a_statement_guard_needs_structured_flow (c : Cond) (e : Bool) :
    (Guard.branch c e).discharges false = false ∧
      (Guard.earlyExit c).discharges false = false := by
  simp [Guard.discharges]

/-! ## Vectors shared with the Rust tests -/

def vectorCond : String → Cond
  | "x" => .read
  | "x != 0" => .cmp .ne .read .zero
  | "0 != x" => .cmp .ne .zero .read
  | "x == 0" => .cmp .eq .read .zero
  | "x > 0" => .cmp .gt .read .zero
  | "0 < x" => .cmp .lt .zero .read
  | "x <= 0" => .cmp .le .read .zero
  | "x >= 0" => .cmp .ge .read .zero
  | "!x" => .not .read
  | "x && y" => .and .read (.other 0)
  | "y && x" => .and (.other 0) .read
  | "x == 0 || y" => .or (.cmp .eq .read .zero) (.other 0)
  | _ => .other 0

def proofVectors : List (String × Bool × Bool) := [("x", true, true), ("x", false, false), ("x != 0", true, true), ("0 != x", true, true), ("x != 0", false, false), ("x == 0", false, true), ("x == 0", true, false), ("x > 0", true, true), ("0 < x", true, true), ("x <= 0", false, true), ("x >= 0", true, false), ("!x", false, true), ("!x", true, false), ("x && y", true, true), ("y && x", true, true), ("x && y", false, false), ("x == 0 || y", false, true), ("y", true, false)]

theorem proof_vectors_hold :
    proofVectors.all (fun (s, e, x) => proves (vectorCond s) e == x) = true := by
  decide

def vectorFacts : String → SiteFacts
  | "ternary_guard" => ⟨true, true, true, [.expression (vectorCond "x == 0") false]⟩
  | "branch_guard" => ⟨true, true, true, [.branch (vectorCond "x != 0") true]⟩
  | "early_exit" => ⟨true, true, true, [.earlyExit (vectorCond "x == 0")]⟩
  | "wrong_edge" => ⟨true, true, true, [.expression (vectorCond "x == 0") true]⟩
  | "branch_unstructured" => ⟨true, true, false, [.branch (vectorCond "x != 0") true]⟩
  | "written_object" => ⟨true, false, true, [.branch (vectorCond "x != 0") true]⟩
  | "narrowed_operand" => ⟨false, true, true, [.branch (vectorCond "x != 0") true]⟩
  | "ternary_unstructured" => ⟨true, true, false, [.expression (vectorCond "x != 0") true]⟩
  | _ => ⟨true, true, true, []⟩

def decisionVectors : List (String × String) := [("ternary_guard", "admit"), ("branch_guard", "admit"), ("early_exit", "admit"), ("no_guard", "bit-count-operand-not-proved-nonzero"), ("wrong_edge", "bit-count-operand-not-proved-nonzero"), ("branch_unstructured", "bit-count-operand-not-proved-nonzero"), ("written_object", "bit-count-operand-object-not-stable"), ("narrowed_operand", "bit-count-operand-not-a-named-object"), ("ternary_unstructured", "admit")]

theorem decision_vectors_hold :
    decisionVectors.all (fun (s, a) => code (decide (vectorFacts s)) == a) = true := by
  decide

theorem the_tables_are_counted :
    builtinNames.length = 6 ∧ proofVectors.length = 18 ∧ decisionVectors.length = 9 := by
  decide

end QCore.BitCountBuiltins
