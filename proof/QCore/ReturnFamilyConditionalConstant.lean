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

import QCore.ReturnFamilyUntrackedConditionalAccess

/-!
# A decided condition prunes the arm C never evaluates (card RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING, rule 74z)

Rule 74x makes every untracked root of both arms of an accessed conditional a
base and never decides the condition. So `(0 ? p : q)->n` makes a null `p` a
base, though C never evaluates `p` (C11 6.5.15p4: "the second operand is
evaluated only if the first compares unequal to 0; the third operand is
evaluated only if the first compares equal to 0").

Rule 74z decides a condition only when it is an integer constant expression
of the accepted grammar (`Expr`: integer literals, unary `+ - ! ~` and the
binary integer operators) that `eval` evaluates exactly, and then keeps only
the roots of the arm C evaluates (`prunedRoots`).

* **The evaluator.** `eval` follows C11 6.4.4.1 (literal types), 6.3.1.8 (the
  usual arithmetic conversions) and 6.5.3.3 to 6.5.14, under the readers'
  LP64 premise (rule 11y): `int` is 32 bits, `long` and `long long` are 64,
  and signed types are two's complement. Every case C leaves undefined or
  implementation-defined is undecided: division or modulo by zero
  (`div_by_zero_undecided`, `rem_by_zero_undecided`), a signed result its
  type does not represent (`signed_overflow_undecided`), a shift count out of
  range (`shift_out_of_range_undecided`) and a negative signed shift operand
  (`negative_signed_shift_undecided`). A signed operation that is decided is
  the mathematical one, never wrapped (`signed_arith_exact`); an unsigned one
  is reduced modulo `2^width` (`unsigned_arith_modular`). Anything outside the
  grammar (`Expr.other`: a name, `sizeof`, a cast, a call, a float, a
  character) is undecided, and so is every expression containing it
  (`undecided_propagates`).
* **C11 6.5.15p4.** `runs` lists the roots each execution of an arm
  evaluates. Under a decided condition only one arm runs
  (`only_one_arm_is_evaluated`); under an undecided one either may.
* **Pruning.** `prunedRoots` covers every root of every run
  (`pruned_covers_every_run`), it is a subset of rule 74x's reading
  (`pruned_subset`), it refuses exactly when rule 74x refuses
  (`pruned_refuses_iff`), and a root rule 74x has but pruning drops is in no
  run (`pruning_removes_only_unevaluated_bases`). Without a decided condition
  it is rule 74x's reading (`undecided_keeps_74x_reading`).
* **The read.** If the tip admits and every dropped base's obligations
  would discharge, rule 74x admits too (`only_dropped_bases_are_discharged`):
  the tip discharges nothing rule 74x kept except the bases of an arm C
  never evaluates.

## What this is not

A model of the token scan or of the flow check. The `Expr` of a condition and
the `Arm`s of a group are the Rust reader's output (trust boundary 28), bound
by `constVectors` and `pruneVectors`, which the Rust unit test reads. The
Rust reader reaches `Expr.other` for every token outside the grammar.
-/

namespace QCore.ReturnFamilyConditionalConstant

open QCore.ReturnFamilyUntrackedConditionalAccess
  (Arm armRoots Group access groupRoots Root base arrowRoot newRead admits groupsRefuse
    names_kept)
open QCore.ReturnFamilyUntrackedArithAccess (Outcome)

/-! ## Integer types and values under LP64 -/

/-- An integer type: its signedness and width. `long` and `long long` are
both 64 bits wide and signed, so they are the same `Kind`, and the usual
arithmetic conversions give the same value either way. -/
structure Kind where
  signed : Bool
  width : Nat
  deriving DecidableEq, Repr

def int : Kind := ⟨true, 32⟩
def uint : Kind := ⟨false, 32⟩
def long : Kind := ⟨true, 64⟩
def ulong : Kind := ⟨false, 64⟩

def Kind.modulus (k : Kind) : Int := Int.ofNat (2 ^ k.width)

def Kind.min (k : Kind) : Int := if k.signed then -Int.ofNat (2 ^ (k.width - 1)) else 0

def Kind.max (k : Kind) : Int :=
  if k.signed then Int.ofNat (2 ^ (k.width - 1)) - 1 else Int.ofNat (2 ^ k.width) - 1

def Kind.holds (k : Kind) (v : Int) : Bool := decide (k.min ≤ v) && decide (v ≤ k.max)

/-- A value: its type and its mathematical value. -/
structure Val where
  kind : Kind
  value : Int
  deriving DecidableEq, Repr

/-- `v` in `k`: unchanged for a signed `k` that represents it, modulo
`2^width` for an unsigned `k`, undecided otherwise. -/
def exact (k : Kind) (v : Int) : Option Val :=
  if k.signed then (if k.holds v then some ⟨k, v⟩ else none) else some ⟨k, v % k.modulus⟩

/-- C11 6.3.1.8: the usual arithmetic conversions of two promoted types. -/
def common (a b : Kind) : Kind :=
  if a.signed = b.signed then (if a.width ≥ b.width then a else b)
  else
    let u := if a.signed then b else a
    let s := if a.signed then a else b
    if u.width ≥ s.width then u else s

/-- C11 6.3.1.3: a conversion. -/
def convert (v : Val) (k : Kind) : Option Val :=
  if k.holds v.value then some ⟨k, v.value⟩ else if k.signed then none else exact k v.value

/-- The two's complement bit pattern of a value. -/
def bits (v : Val) : Nat := (v.value % v.kind.modulus).toNat

/-- The value of a two's complement bit pattern. -/
def fromBits (k : Kind) (p : Nat) : Val :=
  ⟨k, if k.signed && decide (Int.ofNat p > k.max) then Int.ofNat p - k.modulus else Int.ofNat p⟩

/-- The `int` C gives `!`, `&&`, `||`, the relational and equality operators. -/
def truth (b : Bool) : Val := ⟨int, if b then 1 else 0⟩

/-- C11 6.4.4.1p5: the first listed type that represents the literal's
value. `suffixL` is `0` for no `l`, `1` for `l`, `2` for `ll`. -/
def literalKind (value : Nat) (unsigned : Bool) (suffixL : Nat) (decimal : Bool) : Option Kind :=
  let candidates : List Kind :=
    match unsigned, suffixL, decimal with
    | false, 0, true => [int, long]
    | false, 0, false => [int, uint, long, ulong]
    | true, 0, _ => [uint, ulong]
    | false, _, true => [long]
    | false, _, false => [long, ulong]
    | true, _, _ => [ulong]
  candidates.find? (fun k => k.holds (Int.ofNat value))

/-! ## The accepted grammar -/

inductive Un where
  | pos | neg | lnot | compl
  deriving DecidableEq, Repr

inductive Bin where
  | mul | div | rem | add | sub | shl | shr | lt | gt | le | ge | eq | ne
  | band | bxor | bor | land | lor
  deriving DecidableEq, Repr

/-- A condition as the reader reads it. `other` is any token outside the
grammar: a name (an enumeration constant or a macro too), `sizeof`, a cast,
a call, a float, a character, a string, `?:`, a comma. -/
inductive Expr where
  | lit (value : Nat) (unsigned : Bool) (suffixL : Nat) (decimal : Bool)
  | un (op : Un) (e : Expr)
  | bin (op : Bin) (a b : Expr)
  | other
  deriving DecidableEq, Repr

/-- C11 6.5.3.3. -/
def unary : Un → Val → Option Val
  | .pos, a => some a
  | .neg, a => exact a.kind (-a.value)
  | .lnot, a => some (truth (a.value == 0))
  | .compl, a => some (fromBits a.kind ((2 ^ a.kind.width - 1) ^^^ bits a))

/-- C11 6.5.5 to 6.5.13 over operands converted to the common type `k`. -/
def arith (op : Bin) (k : Kind) (a b : Int) : Option Val :=
  match op with
  | .mul => exact k (a * b)
  | .add => exact k (a + b)
  | .sub => exact k (a - b)
  | .div => if b = 0 then none else exact k (a.tdiv b)
  | .rem => if b = 0 then none else (exact k (a.tdiv b)).bind (fun _ => exact k (a.tmod b))
  | .lt => some (truth (decide (a < b)))
  | .gt => some (truth (decide (a > b)))
  | .le => some (truth (decide (a ≤ b)))
  | .ge => some (truth (decide (a ≥ b)))
  | .eq => some (truth (decide (a = b)))
  | .ne => some (truth (decide (a ≠ b)))
  | .band => some (fromBits k (bits ⟨k, a⟩ &&& bits ⟨k, b⟩))
  | .bxor => some (fromBits k (bits ⟨k, a⟩ ^^^ bits ⟨k, b⟩))
  | .bor => some (fromBits k (bits ⟨k, a⟩ ||| bits ⟨k, b⟩))
  | _ => none

/-- C11 6.5.7: each shift operand promoted alone, the left one's type. -/
def shift (left : Bool) (a b : Val) : Option Val :=
  if b.value < 0 ∨ b.value ≥ Int.ofNat a.kind.width then none
  else if a.kind.signed = true ∧ a.value < 0 then none
  else if left then exact a.kind (a.value * Int.ofNat (2 ^ b.value.toNat))
  else some ⟨a.kind, a.value / Int.ofNat (2 ^ b.value.toNat)⟩

/-- A binary operator. `&&` and `||` need both operands decided, never more
than C's short circuit decides. -/
def binary (op : Bin) (a b : Val) : Option Val :=
  match op with
  | .shl => shift true a b
  | .shr => shift false a b
  | .land => some (truth (a.value != 0 && b.value != 0))
  | .lor => some (truth (a.value != 0 || b.value != 0))
  | _ =>
    let k := common a.kind b.kind
    match convert a k, convert b k with
    | some x, some y => arith op k x.value y.value
    | _, _ => none

/-- `constant::constant_value`. -/
def eval : Expr → Option Val
  | .lit v u l d => (literalKind v u l d).map (fun k => ⟨k, Int.ofNat v⟩)
  | .un op e => (eval e).bind (unary op)
  | .bin op a b =>
    match eval a, eval b with
    | some x, some y => binary op x y
    | _, _ => none
  | .other => none

/-- `constant::constant_condition`: C takes the middle arm when the value
compares unequal to 0 (C11 6.5.15p4). -/
def decideCond (e : Expr) : Option Bool := (eval e).map (fun v => v.value != 0)

/-! ## Exactness -/

theorem div_by_zero_undecided (k : Kind) (a : Int) : arith .div k a 0 = none := by
  simp [arith]

theorem rem_by_zero_undecided (k : Kind) (a : Int) : arith .rem k a 0 = none := by
  simp [arith]

theorem signed_overflow_undecided (k : Kind) (hk : k.signed = true) (v : Int)
    (h : k.holds v = false) : exact k v = none := by
  simp [exact, hk, h]

theorem signed_arith_exact (k : Kind) (hk : k.signed = true) (a b : Int) (r : Val) :
    (arith .add k a b = some r → r.value = a + b) ∧
    (arith .sub k a b = some r → r.value = a - b) ∧
    (arith .mul k a b = some r → r.value = a * b) ∧
    (arith .div k a b = some r → r.value = a.tdiv b) := by
  refine ⟨?_, ?_, ?_, ?_⟩ <;> intro h
  · simp only [arith, exact, hk, ite_true] at h
    split at h <;> simp_all
    exact h.symm ▸ rfl
  · simp only [arith, exact, hk, ite_true] at h
    split at h <;> simp_all
    exact h.symm ▸ rfl
  · simp only [arith, exact, hk, ite_true] at h
    split at h <;> simp_all
    exact h.symm ▸ rfl
  · simp only [arith, exact, hk, ite_true] at h
    split at h
    · simp at h
    · split at h <;> simp_all
      exact h.symm ▸ rfl

theorem unsigned_arith_modular (k : Kind) (hk : k.signed = false) (a b : Int) :
    arith .add k a b = some ⟨k, (a + b) % k.modulus⟩ ∧
    arith .sub k a b = some ⟨k, (a - b) % k.modulus⟩ ∧
    arith .mul k a b = some ⟨k, (a * b) % k.modulus⟩ := by
  simp [arith, exact, hk]

theorem shift_out_of_range_undecided (left : Bool) (a b : Val)
    (h : b.value < 0 ∨ b.value ≥ Int.ofNat a.kind.width) : shift left a b = none := by
  unfold shift
  split
  · rfl
  · contradiction

theorem negative_signed_shift_undecided (left : Bool) (a b : Val) (hs : a.kind.signed = true)
    (hn : a.value < 0) : shift left a b = none := by
  unfold shift
  split
  · rfl
  · simp [hs, hn]

/-- Anything outside the grammar is undecided, and so is every expression
containing it. -/
theorem undecided_propagates (op : Un) (bop : Bin) (e a b : Expr) :
    eval .other = none ∧
    (eval e = none → eval (.un op e) = none) ∧
    (eval a = none → eval (.bin bop a b) = none) ∧
    (eval b = none → eval (.bin bop a b) = none) := by
  refine ⟨rfl, ?_, ?_, ?_⟩
  · intro h; simp [eval, h]
  · intro h; simp [eval, h]
  · intro h; simp only [eval, h]; split <;> simp_all

theorem decided_only_when_evaluated (e : Expr) (c : Bool) (h : decideCond e = some c) :
    ∃ v, eval e = some v ∧ (v.value != 0) = c := by
  unfold decideCond at h
  cases he : eval e with
  | none => simp [he] at h
  | some v => exact ⟨v, rfl, by simpa [he] using h⟩

theorem undecided_condition (e : Expr) (h : eval e = none) : decideCond e = none := by
  simp [decideCond, h]

/-! ## C11 6.5.15p4 and pruning -/

/-- The roots each execution of an arm evaluates. A conditional whose
condition is decided runs one arm; an undecided one may run either. -/
def runs : Arm → List (List String)
  | .root n => [[n]]
  | .inert => [[]]
  | .loaded => [[]]
  | .paren a => runs a
  | .cast a => runs a
  | .sum _ a => runs a
  | .cond (some true) a _ => runs a
  | .cond (some false) _ b => runs b
  | .cond none a b => runs a ++ runs b
  | .other => [[]]

theorem only_one_arm_is_evaluated (a b : Arm) :
    runs (.cond (some true) a b) = runs a ∧ runs (.cond (some false) a b) = runs b :=
  ⟨rfl, rfl⟩

/-- `arm_roots` with rule 74z's `conditional_arm_roots`: both arms are
classified, and a decided condition keeps the evaluated arm's roots. -/
def prunedRoots : Arm → Option (List String)
  | .root n => some [n]
  | .inert => some []
  | .loaded => some []
  | .paren a => prunedRoots a
  | .cast a => prunedRoots a
  | .sum c a => if c then prunedRoots a else none
  | .cond c a b =>
    match prunedRoots a, prunedRoots b with
    | some x, some y =>
      some (match c with
        | some true => x
        | some false => y
        | none => x ++ y)
    | _, _ => none
  | .other => none

/-- No condition of the arm is decided. -/
def undecided : Arm → Bool
  | .paren a => undecided a
  | .cast a => undecided a
  | .sum _ a => undecided a
  | .cond c a b => c.isNone && undecided a && undecided b
  | _ => true

theorem undecided_keeps_74x_reading (a : Arm) (h : undecided a = true) :
    prunedRoots a = armRoots a := by
  induction a with
  | root => rfl
  | inert => rfl
  | loaded => rfl
  | paren a ih => exact ih h
  | cast a ih => exact ih h
  | sum c a ih => simp only [undecided] at h; simp [prunedRoots, armRoots, ih h]
  | cond c a b iha ihb =>
    simp only [undecided, Bool.and_eq_true] at h
    obtain ⟨⟨hc, ha⟩, hb⟩ := h
    cases c with
    | some _ => simp at hc
    | none =>
      simp only [prunedRoots, armRoots, iha ha, ihb hb]
      cases armRoots a <;> cases armRoots b <;> rfl
  | other => rfl

theorem pruned_refuses_iff (a : Arm) : prunedRoots a = none ↔ armRoots a = none := by
  induction a with
  | root => simp [prunedRoots, armRoots]
  | inert => simp [prunedRoots, armRoots]
  | loaded => simp [prunedRoots, armRoots]
  | paren a ih => exact ih
  | cast a ih => exact ih
  | sum c a ih => cases c <;> simp [prunedRoots, armRoots, ih]
  | cond c a b iha ihb =>
    simp only [prunedRoots, armRoots]
    cases hpa : prunedRoots a <;> cases hpb : prunedRoots b <;>
      cases haa : armRoots a <;> cases hab : armRoots b <;> simp_all
  | other => simp [prunedRoots, armRoots]

theorem pruned_subset (a : Arm) (x y : List String) (hp : prunedRoots a = some x)
    (ha : armRoots a = some y) : ∀ n ∈ x, n ∈ y := by
  induction a generalizing x y with
  | root => simp_all [prunedRoots, armRoots]
  | inert => simp_all [prunedRoots, armRoots]
  | loaded => simp_all [prunedRoots, armRoots]
  | paren a ih => exact ih x y hp ha
  | cast a ih => exact ih x y hp ha
  | sum c a ih =>
    cases c
    · simp [prunedRoots] at hp
    · exact ih x y hp ha
  | cond c a b iha ihb =>
    simp only [prunedRoots, armRoots] at hp ha
    cases hpa : prunedRoots a <;> cases hpb : prunedRoots b <;> simp [hpa, hpb] at hp
    cases haa : armRoots a <;> cases hab : armRoots b <;> simp [haa, hab] at ha
    rename_i pa pb aa ab
    intro n hn
    rw [← ha]
    cases c with
    | none =>
      simp at hp
      subst hp
      simp only [List.mem_append] at hn ⊢
      rcases hn with h | h
      · exact Or.inl (iha pa aa hpa haa n h)
      · exact Or.inr (ihb pb ab hpb hab n h)
    | some t =>
      cases t
      · simp at hp; subst hp; exact List.mem_append_right _ (ihb pb ab hpb hab n hn)
      · simp at hp; subst hp; exact List.mem_append_left _ (iha pa aa hpa haa n hn)
  | other => simp [prunedRoots] at hp

/-- Soundness: every root of every execution is still a root. -/
theorem pruned_covers_every_run (a : Arm) (x : List String) (hp : prunedRoots a = some x) :
    ∀ run ∈ runs a, ∀ n ∈ run, n ∈ x := by
  induction a generalizing x with
  | root => simp_all [prunedRoots, runs]
  | inert => simp [runs]
  | loaded => simp [runs]
  | paren a ih => exact ih x hp
  | cast a ih => exact ih x hp
  | sum c a ih =>
    cases c
    · simp [prunedRoots] at hp
    · exact ih x hp
  | cond c a b iha ihb =>
    simp only [prunedRoots] at hp
    cases hpa : prunedRoots a <;> cases hpb : prunedRoots b <;> simp [hpa, hpb] at hp
    rename_i pa pb
    intro run hrun n hn
    cases c with
    | none =>
      simp at hp; subst hp
      simp only [runs, List.mem_append] at hrun
      rcases hrun with h | h
      · exact List.mem_append_left _ (iha pa hpa run h n hn)
      · exact List.mem_append_right _ (ihb pb hpb run h n hn)
    | some t =>
      cases t
      · simp at hp; subst hp; exact ihb pb hpb run hrun n hn
      · simp at hp; subst hp; exact iha pa hpa run hrun n hn
  | other => simp [prunedRoots] at hp

/-- A root rule 74x reads but pruning drops is in no execution: it belongs
only to arms C never evaluates. -/
theorem pruning_removes_only_unevaluated_bases (a : Arm) (x y : List String)
    (hp : prunedRoots a = some x) (_ha : armRoots a = some y) (n : String)
    (_hy : n ∈ y) (hx : n ∉ x) : ∀ run ∈ runs a, n ∉ run := by
  intro run hrun hn
  exact hx (pruned_covers_every_run a x hp run hrun n hn)

/-! ## The group and the read -/

/-- `conditional_roots` for one group with rule 74z's pruning. -/
def prunedGroupRoots (g : Group) : Option (List String) :=
  match access g with
  | .none => some []
  | .unclassified => none
  | _ => prunedRoots g.arms

def prunedRootOf (gs : List Group) (name : String) : Bool :=
  gs.any (fun g => ((prunedGroupRoots g).getD []).contains name)

def prunedArrowOf (gs : List Group) (name : String) : Bool :=
  gs.any (fun g => access g == .arrow && ((prunedGroupRoots g).getD []).contains name)

def prunedRefuse (gs : List Group) : Bool := gs.any (fun g => prunedGroupRoots g == none)

theorem pruned_group_refuses_iff (g : Group) : prunedGroupRoots g = none ↔ groupRoots g = none := by
  unfold prunedGroupRoots groupRoots
  split <;> simp_all [pruned_refuses_iff]

/-- Pruning adds no refusal and removes none. -/
theorem pruning_keeps_every_refusal (gs : List Group) : prunedRefuse gs = groupsRefuse gs := by
  unfold prunedRefuse groupsRefuse
  congr 1
  funext g
  have := pruned_group_refuses_iff g
  cases h1 : prunedGroupRoots g <;> cases h2 : groupRoots g <;> simp_all

def prunedBase (gs : List Group) (r : Root) : Bool := r.oldBase || prunedRootOf gs r.name

def prunedArrow (gs : List Group) (r : Root) : Bool := r.oldArrow || prunedArrowOf gs r.name

/-- The tip's family S result. -/
def prunedRead (refused : Bool) (gs : List Group) (roots : List Root) :
    Option (List String × List String) :=
  if refused || prunedRefuse gs then none
  else some ((roots.filter (prunedBase gs)).map (·.name),
    (roots.filter (prunedArrow gs)).map (·.name))

theorem old_base_kept (gs : List Group) (r : Root) (h : r.oldBase = true) :
    prunedBase gs r = true := by
  simp [prunedBase, h]

/-- The fail-closed argument. If the tip admits, and every base and `->` root
rule 74x has but the tip dropped would discharge too, then rule 74x admits.
So the tip discharges nothing rule 74x kept except dropped bases, and a
dropped base is a root only of arms C never evaluates
(`pruning_removes_only_unevaluated_bases`). -/
theorem only_dropped_bases_are_discharged (ok alias : String → Bool) (refused : Bool)
    (gs : List Group) (roots : List Root)
    (hdrop : ∀ r ∈ roots, base gs r = true → prunedBase gs r = false → ok r.name = true)
    (hdropA : ∀ r ∈ roots, arrowRoot gs r = true → prunedArrow gs r = false →
      alias r.name = true)
    (h : admits ok alias (prunedRead refused gs roots) = true) :
    admits ok alias (newRead refused gs roots) = true := by
  unfold prunedRead at h
  unfold newRead
  rw [pruning_keeps_every_refusal] at h
  cases hr : (refused || groupsRefuse gs) with
  | true => simp [hr, admits] at h
  | false =>
    simp only [hr, Bool.false_eq_true, ite_false, admits, Bool.and_eq_true,
      List.all_eq_true, List.mem_map, List.mem_filter] at h ⊢
    obtain ⟨hb, hw⟩ := h
    refine ⟨?_, ?_⟩
    · rintro name ⟨r, ⟨hm, hbase⟩, rfl⟩
      cases hp : prunedBase gs r with
      | true => exact hb r.name ⟨r, ⟨hm, hp⟩, rfl⟩
      | false => exact hdrop r hm hbase hp
    · rintro name ⟨r, ⟨hm, harrow⟩, rfl⟩
      cases hp : prunedArrow gs r with
      | true => exact hw r.name ⟨r, ⟨hm, hp⟩, rfl⟩
      | false => exact hdropA r hm harrow hp

/-! ## The readings -/

/-- `( 0 ? p : q ) - > n`: `p` is no base, `q` is. -/
theorem constant_false_prunes_the_middle :
    let gs : List Group := [⟨[], .arrow, .cond (decideCond (.lit 0 false 0 true))
      (.root "p") (.root "q")⟩]
    prunedBase gs ⟨"p", false, false⟩ = false ∧ prunedBase gs ⟨"q", false, false⟩ = true ∧
      base gs ⟨"p", false, false⟩ = true := by
  decide

/-! ## The token reader's vectors -/

/-- `(condition spelling, expression, value)`: the value `constant_value`
gives, `none` when undecided. A spelling is the gateway's token stream, which
spells every punctuator as one byte (`< <`, `= =`, `& &`). The Rust unit test
reads the spelling and the value. -/
def constVectors : List (String × Expr × Option Int) := [
  ("0", .lit 0 false 0 true, some 0),
  ("1", .lit 1 false 0 true, some 1),
  ("( 2 - 2 )", .bin .sub (.lit 2 false 0 true) (.lit 2 false 0 true), some 0),
  ("0x10UL", .lit 16 true 1 false, some 16),
  ("017", .lit 15 false 0 false, some 15),
  ("- 1 < 0u", .bin .lt (.un .neg (.lit 1 false 0 true)) (.lit 0 true 0 true), some 0),
  ("- 1 < 0", .bin .lt (.un .neg (.lit 1 false 0 true)) (.lit 0 false 0 true), some 1),
  ("- 1L < 0u", .bin .lt (.un .neg (.lit 1 false 1 true)) (.lit 0 true 0 true), some 1),
  ("0u - 1", .bin .sub (.lit 0 true 0 true) (.lit 1 false 0 true), some 4294967295),
  ("~ 0", .un .compl (.lit 0 false 0 true), some (-1)),
  ("~ 0u", .un .compl (.lit 0 true 0 true), some 4294967295),
  ("! 5", .un .lnot (.lit 5 false 0 true), some 0),
  ("1 < < 4", .bin .shl (.lit 1 false 0 true) (.lit 4 false 0 true), some 16),
  ("256 > > 8", .bin .shr (.lit 256 false 0 true) (.lit 8 false 0 true), some 1),
  ("- 7 / 2", .bin .div (.un .neg (.lit 7 false 0 true)) (.lit 2 false 0 true), some (-3)),
  ("- 7 % 2", .bin .rem (.un .neg (.lit 7 false 0 true)) (.lit 2 false 0 true), some (-1)),
  ("6 & 3 | 8 ^ 1", .bin .bor (.bin .band (.lit 6 false 0 true) (.lit 3 false 0 true))
      (.bin .bxor (.lit 8 false 0 true) (.lit 1 false 0 true)), some 11),
  ("- 1 & 255", .bin .band (.un .neg (.lit 1 false 0 true)) (.lit 255 false 0 true), some 255),
  ("1 + 2 * 3 = = 7", .bin .eq (.bin .add (.lit 1 false 0 true)
      (.bin .mul (.lit 2 false 0 true) (.lit 3 false 0 true))) (.lit 7 false 0 true), some 1),
  ("1 ! = 1 | | 2 > = 3 & & 1", .bin .lor (.bin .ne (.lit 1 false 0 true) (.lit 1 false 0 true))
      (.bin .land (.bin .ge (.lit 2 false 0 true) (.lit 3 false 0 true)) (.lit 1 false 0 true)),
    some 0),
  ("2147483648", .lit 2147483648 false 0 true, some 2147483648),
  ("0x80000000 > 0", .bin .gt (.lit 2147483648 false 0 false) (.lit 0 false 0 true), some 1),
  ("1 / 0", .bin .div (.lit 1 false 0 true) (.lit 0 false 0 true), none),
  ("1 % 0", .bin .rem (.lit 1 false 0 true) (.lit 0 false 0 true), none),
  ("2147483647 + 1", .bin .add (.lit 2147483647 false 0 true) (.lit 1 false 0 true), none),
  ("( - 2147483647 - 1 ) / - 1", .bin .div (.bin .sub (.un .neg (.lit 2147483647 false 0 true))
      (.lit 1 false 0 true)) (.un .neg (.lit 1 false 0 true)), none),
  ("1 < < 32", .bin .shl (.lit 1 false 0 true) (.lit 32 false 0 true), none),
  ("1 < < - 1", .bin .shl (.lit 1 false 0 true) (.un .neg (.lit 1 false 0 true)), none),
  ("1 < < 31", .bin .shl (.lit 1 false 0 true) (.lit 31 false 0 true), none),
  ("- 1 > > 1", .bin .shr (.un .neg (.lit 1 false 0 true)) (.lit 1 false 0 true), none),
  ("18446744073709551616", .lit 18446744073709551616 false 0 true, none),
  ("k", .other, none),
  ("N", .other, none),
  ("sizeof ( int ) - 4", .bin .sub .other (.lit 4 false 0 true), none),
  ("( int ) 0", .other, none),
  ("x - x", .bin .sub .other .other, none),
  ("1.0", .other, none),
  ("'a'", .other, none),
  ("08", .other, none),
  ("1e5", .other, none),
  ("0 ? 1 : 0", .other, none)
]

def constVectorsAgree : Bool :=
  constVectors.all (fun (_, e, v) => (eval e).map (·.value) == v)

theorem const_vectors_hold : constVectorsAgree = true := by decide

/-- `(spelling, groups, p base, refused)` for the untracked `p` (no old
test reads it). The Rust unit test reads the spelling and the last two
fields against `scalar_read`. -/
def pruneVectors : List (String × List Group × Bool × Bool × Bool) := [
  ("( 0 ? p : a ) - > n", [⟨[], .arrow, .cond (some false) (.root "p") (.root "a")⟩], false, false, false),
  ("( 1 ? a : p ) - > n", [⟨[], .arrow, .cond (some true) (.root "a") (.root "p")⟩], false, false, false),
  ("( 1 ? p : a ) - > n", [⟨[], .arrow, .cond (some true) (.root "p") (.root "a")⟩], false, true, false),
  ("( ( 2 - 2 ) ? p : a ) [ 0 ]",
    [⟨[], .element, .cond (some false) (.root "p") (.root "a")⟩], false, false, false),
  ("* ( 0x0 ? p : a )", [⟨[], .element, .cond (some false) (.root "p") (.root "a")⟩],
    false, false, false),
  ("( n ? p : a ) - > m", [⟨[], .arrow, .cond none (.root "p") (.root "a")⟩], false, true, false),
  ("( N ? p : a ) - > m", [⟨[], .arrow, .cond none (.root "p") (.root "a")⟩], false, true, false),
  ("( sizeof ( int ) - 4 ? p : a ) - > m",
    [⟨[], .arrow, .cond none (.root "p") (.root "a")⟩], false, true, false),
  ("( 1 / 0 ? p : a ) - > m", [⟨[], .arrow, .cond none (.root "p") (.root "a")⟩], false, true, false),
  ("( n - n ? p : a ) - > m", [⟨[], .arrow, .cond none (.root "p") (.root "a")⟩], false, true, false),
  ("( 0 ? ( n ? p : b ) : a ) - > m",
    [⟨[], .arrow, .cond (some false) (.paren (.cond none (.root "p") (.root "b"))) (.root "a")⟩],
    false, false, false),
  ("( 1 ? ( n ? p : b ) : a ) - > m",
    [⟨[], .arrow, .cond (some true) (.paren (.cond none (.root "p") (.root "b"))) (.root "a")⟩],
    false, true, false),
  ("( n ? ( 0 ? p : b ) : a ) - > m",
    [⟨[], .arrow, .cond none (.paren (.cond (some false) (.root "p") (.root "b"))) (.root "a")⟩],
    false, false, false),
  ("( n ? ( 1 ? p : b ) : a ) - > m",
    [⟨[], .arrow, .cond none (.paren (.cond (some true) (.root "p") (.root "b"))) (.root "a")⟩],
    false, true, false),
  ("( 0 ? b : 1 ? p : a ) - > m",
    [⟨[], .arrow, .cond (some false) (.root "b") (.cond (some true) (.root "p") (.root "a"))⟩],
    false, true, false),
  ("( ( long * ) ( 0 ? p : a ) ) [ 0 ]",
    [⟨[.cast, .paren], .element, .cond (some false) (.root "p") (.root "a")⟩], false, false, false),
  ("( 0 ? ( long * ) p : a ) [ 0 ]",
    [⟨[], .element, .cond (some false) (.cast (.root "p")) (.root "a")⟩], false, false, false),
  ("( 0 ? p + 1 : a ) - > m",
    [⟨[], .arrow, .cond (some false) (.sum true (.root "p")) (.root "a")⟩], false, false, false),
  ("( ( 0 ? p : a ) + 1 ) - > m",
    [⟨[.sum true], .arrow, .cond (some false) (.root "p") (.root "a")⟩], false, false, false),
  ("( 0 ? p : - a ) - > m", [⟨[], .arrow, .cond (some false) (.root "p") .other⟩], false, false, true),
  ("( 1 ? p : a * 2 ) [ 0 ]", [⟨[], .element, .cond (some true) (.root "p") .other⟩],
    false, false, true),
  ("( 0 ? p - > next : a ) - > m", [⟨[], .arrow, .cond (some false) .loaded (.root "a")⟩],
    true, true, false)
]

def pruneVectorsAgree : Bool :=
  pruneVectors.all (fun (_, gs, oldB, b, refused) =>
    prunedBase gs ⟨"p", oldB, false⟩ == b && prunedRefuse gs == refused)

theorem prune_vectors_hold : pruneVectorsAgree = true := by decide

end QCore.ReturnFamilyConditionalConstant
