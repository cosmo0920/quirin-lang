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

/-! The C integer domain (card CG20-NUMERIC-DOMAIN, gateway rule 11q): a model
of `crates/qcore-c-gateway/src/c_numeric.rs`.

Bound mechanically, not by transcription alone: the Rust tests in
`crates/qcore-c-gateway/tests/unit/c_numeric.rs` read `lp64`, `llp64`,
`promotionVectors`, `usualVectors`, `kernelVectors` and `conversionVectors`
from this file and compare them with the Rust kernel, and
`crates/qcore-c-gateway/tests/c_numeric_domain.rs` checks the type vectors
against the conversions Clang itself records under `x86_64-pc-linux-gnu`
(LP64) and `x86_64-pc-windows-msvc` (LLP64). That Clang's predefined macros
are its `TargetInfo` widths, and the faithful reading of its JSON AST, stay
review premises (boundary 28).

Stated:
* the standard integer types per data model, the integer promotions and the
  usual arithmetic conversions (C17 6.3.1.1, 6.3.1.8), with `long + unsigned
  int` decided differently under LP64 and LLP64;
* the kernel: a fault case is reported exactly when one is reached, unsigned
  `+ - *` never faults and wraps into range, a signed result is in range;
* the interval discharge rules the walker uses are sound: a `Discharged`
  obligation is reached by no operand pair in the ranges. -/
namespace QCore.CNumeric

/-! ## Types per target -/

/-- The widths Clang reports for a target (`__SIZEOF_*__ * __CHAR_BIT__`). -/
structure Model where
  charSigned : Bool
  short : Nat
  int : Nat
  long : Nat
  longLong : Nat
  pointer : Nat
  deriving DecidableEq, Repr

/-- `x86_64-pc-linux-gnu`, as Clang 21 reports it. -/
def lp64 : Model := ⟨true, 16, 32, 64, 64, 64⟩
/-- `x86_64-pc-windows-msvc`, as Clang 21 reports it. -/
def llp64 : Model := ⟨true, 16, 32, 32, 64, 64⟩

inductive Kind where
  | bool | char | schar | uchar | short | ushort | int | uint | long | ulong | llong | ullong
  deriving DecidableEq, Repr

def Kind.rank : Kind → Nat
  | .bool => 0
  | .char | .schar | .uchar => 1
  | .short | .ushort => 2
  | .int | .uint => 3
  | .long | .ulong => 4
  | .llong | .ullong => 5

def Kind.toUnsigned : Kind → Kind
  | .char | .schar => .uchar
  | .short => .ushort
  | .int => .uint
  | .long => .ulong
  | .llong => .ullong
  | k => k

def width (m : Model) : Kind → Nat
  | .bool => 1
  | .char | .schar | .uchar => 8
  | .short | .ushort => m.short
  | .int | .uint => m.int
  | .long | .ulong => m.long
  | .llong | .ullong => m.longLong

def signed (m : Model) : Kind → Bool
  | .char => m.charSigned
  | .schar | .short | .int | .long | .llong => true
  | _ => false

def lo (w : Nat) (s : Bool) : Int := if s then -((2 ^ (w - 1) : Nat) : Int) else 0
def hi (w : Nat) (s : Bool) : Int :=
  if s then ((2 ^ (w - 1) : Nat) : Int) - 1 else ((2 ^ w : Nat) : Int) - 1

def kindLo (m : Model) (k : Kind) : Int := lo (width m k) (signed m k)
def kindHi (m : Model) (k : Kind) : Int := hi (width m k) (signed m k)

def representsAll (m : Model) (t f : Kind) : Bool :=
  decide (kindLo m t ≤ kindLo m f) && decide (kindHi m f ≤ kindHi m t)

/-- C17 6.3.1.1p2. -/
def promote (m : Model) (k : Kind) : Kind :=
  if k.rank < Kind.int.rank then (if representsAll m .int k then .int else .uint) else k

/-- C17 6.3.1.8p1, integer operands. -/
def usual (m : Model) (a b : Kind) : Kind :=
  let a := promote m a
  let b := promote m b
  if a = b then a
  else if signed m a = signed m b then (if b.rank ≤ a.rank then a else b)
  else
    let (s, u) := if signed m a then (a, b) else (b, a)
    if s.rank ≤ u.rank then u
    else if representsAll m s u then s
    else s.toUnsigned

theorem long_is_64_under_lp64 : width lp64 .long = 64 := rfl
theorem long_is_32_under_llp64 : width llp64 .long = 32 := rfl

/-- The LP64/LLP64 split: `long + unsigned int` is `long` under LP64 and
`unsigned long` under LLP64. -/
theorem long_plus_uint_lp64 : usual lp64 .long .uint = .long := by decide
theorem long_plus_uint_llp64 : usual llp64 .long .uint = .ulong := by decide
theorem char_promotes_to_int : promote lp64 .char = .int ∧ promote llp64 .ushort = .int := by
  decide
theorem uint_plus_int_is_uint : usual lp64 .uint .int = .uint := by decide

/-- Shared vectors with Clang and `c_numeric::promote`: (model, operand, promoted). -/
def promotionVectors : List (String × Kind × Kind) := [
  ("lp64", .bool, .int),
  ("lp64", .char, .int),
  ("lp64", .uchar, .int),
  ("lp64", .short, .int),
  ("lp64", .ushort, .int),
  ("lp64", .uint, .uint),
  ("lp64", .long, .long),
  ("llp64", .ushort, .int),
  ("llp64", .ulong, .ulong)
]

/-- Shared vectors with Clang and `c_numeric::usual_arithmetic`:
(model, left, right, common type). -/
def usualVectors : List (String × Kind × Kind × Kind) := [
  ("lp64", .int, .uint, .uint),
  ("lp64", .long, .uint, .long),
  ("lp64", .long, .ulong, .ulong),
  ("lp64", .llong, .ulong, .ullong),
  ("lp64", .long, .llong, .llong),
  ("lp64", .char, .ushort, .int),
  ("lp64", .uint, .llong, .llong),
  ("llp64", .int, .uint, .uint),
  ("llp64", .long, .uint, .ulong),
  ("llp64", .long, .ulong, .ulong),
  ("llp64", .llong, .ulong, .llong),
  ("llp64", .long, .llong, .llong),
  ("llp64", .uint, .llong, .llong)
]

def modelOf : String → Model
  | "llp64" => llp64
  | _ => lp64

theorem promotion_vectors_hold :
    promotionVectors.all (fun (m, k, r) => promote (modelOf m) k == r) = true := by decide

theorem usual_vectors_hold :
    usualVectors.all (fun (m, a, b, r) => usual (modelOf m) a b == r) = true := by decide

/-! ## The kernel -/

inductive Fault where
  | signedOverflow | divisionByZero | quotientOverflow | shiftCount
  | negativeLeftShift | negativeRightShift | signedConversion
  deriving DecidableEq, Repr

inductive Op where
  | add | sub | mul | div | rem | shl | shr
  deriving DecidableEq, Repr

/-- A kernel outcome: the value, or the first fault case reached. -/
inductive Outcome where
  | ok (v : Int)
  | error (f : Fault)
  deriving DecidableEq, Repr

def inRange (w : Nat) (s : Bool) (v : Int) : Prop := lo w s ≤ v ∧ v ≤ hi w s

instance (w : Nat) (s : Bool) (v : Int) : Decidable (inRange w s v) := by
  unfold inRange; exact inferInstance

/-- The fault cases an operation can reach, in the order the kernel checks them. -/
def faults : Op → Bool → List Fault
  | .add, true | .sub, true | .mul, true => [.signedOverflow]
  | .div, true | .rem, true => [.divisionByZero, .quotientOverflow]
  | .div, false | .rem, false => [.divisionByZero]
  | .shl, true => [.shiftCount, .negativeLeftShift, .signedOverflow]
  | .shr, true => [.shiftCount, .negativeRightShift]
  | .shl, false | .shr, false => [.shiftCount]
  | _, _ => []

def countOk (w : Nat) (b : Int) : Prop := 0 ≤ b ∧ b < (w : Int)

/-- Whether one fault case is reached by one operand pair (`c_numeric::reaches`). -/
def reaches (op : Op) (w : Nat) (s : Bool) (f : Fault) (a b : Int) : Prop :=
  match f, op with
  | .signedOverflow, .add => s = true ∧ ¬ inRange w s (a + b)
  | .signedOverflow, .sub => s = true ∧ ¬ inRange w s (a - b)
  | .signedOverflow, .mul => s = true ∧ ¬ inRange w s (a * b)
  | .signedOverflow, .shl => s = true ∧ countOk w b ∧ 0 ≤ a ∧ hi w s < a * 2 ^ b.toNat
  | .divisionByZero, .div | .divisionByZero, .rem => b = 0
  | .quotientOverflow, .div | .quotientOverflow, .rem => s = true ∧ a = lo w s ∧ b = -1
  | .shiftCount, .shl | .shiftCount, .shr => ¬ countOk w b
  | .negativeLeftShift, .shl => s = true ∧ a < 0
  | .negativeRightShift, .shr => s = true ∧ a < 0
  | _, _ => False

instance (op : Op) (w : Nat) (s : Bool) (f : Fault) (a b : Int) :
    Decidable (reaches op w s f a b) := by
  unfold reaches countOk
  split <;> exact inferInstance

/-- C's modular reduction into the type (`c_numeric::wrap`), unsigned case. -/
def wrapU (w : Nat) (v : Int) : Int := v % ((2 ^ w : Nat) : Int)

/-- The value of an operation no fault case reaches. -/
def value (op : Op) (w : Nat) (s : Bool) (a b : Int) : Int :=
  match op with
  | .add => if s then a + b else wrapU w (a + b)
  | .sub => if s then a - b else wrapU w (a - b)
  | .mul => if s then a * b else wrapU w (a * b)
  | .div => a.tdiv b
  | .rem => a.tmod b
  | .shl => if s then a * 2 ^ b.toNat else wrapU w (a * 2 ^ b.toNat)
  | .shr => a / 2 ^ b.toNat

/-- The kernel (`c_numeric::binary`): the first reached fault case, or the value. -/
def kernel (op : Op) (w : Nat) (s : Bool) (a b : Int) : Outcome :=
  match (faults op s).find? (fun f => decide (reaches op w s f a b)) with
  | some f => .error f
  | none => .ok (value op w s a b)

/-- The kernel succeeds exactly when no listed fault case is reached. -/
theorem kernel_ok_iff_unreached (op : Op) (w : Nat) (s : Bool) (a b : Int) :
    (∃ v, kernel op w s a b = .ok v) ↔ ∀ f ∈ faults op s, ¬ reaches op w s f a b := by
  unfold kernel
  constructor
  · intro ⟨v, hv⟩ f hf hr
    cases h : (faults op s).find? (fun f => decide (reaches op w s f a b)) with
    | some g => rw [h] at hv; cases hv
    | none =>
      have := List.find?_eq_none.mp h f hf
      simp [hr] at this
  · intro hall
    have : (faults op s).find? (fun f => decide (reaches op w s f a b)) = none := by
      apply List.find?_eq_none.mpr
      intro f hf
      simp [hall f hf]
    rw [this]
    exact ⟨_, rfl⟩

/-- A reported fault is one the operands reach. -/
theorem kernel_error_reached (op : Op) (w : Nat) (s : Bool) (a b : Int) (f : Fault)
    (h : kernel op w s a b = .error f) : f ∈ faults op s ∧ reaches op w s f a b := by
  unfold kernel at h
  cases hf : (faults op s).find? (fun f => decide (reaches op w s f a b)) with
  | none => rw [hf] at h; cases h
  | some g =>
    rw [hf] at h
    cases h
    exact ⟨List.mem_of_find?_eq_some hf, by simpa using List.find?_some hf⟩

/-- Unsigned `+ - *` have no fault case: C defines them modulo 2^w. -/
theorem unsigned_arithmetic_defined (op : Op) (hop : op = .add ∨ op = .sub ∨ op = .mul)
    (w : Nat) (a b : Int) : kernel op w false a b = .ok (value op w false a b) := by
  rcases hop with rfl | rfl | rfl <;> rfl

/-- The unsigned wrap lands in the type. -/
theorem wrap_in_range (w : Nat) (v : Int) : inRange w false (wrapU w v) := by
  have hpos : (0 : Int) < ((2 ^ w : Nat) : Int) := by
    have := Nat.two_pow_pos w; omega
  unfold inRange lo hi wrapU
  simp only [Bool.false_eq_true, ite_false]
  constructor
  · exact Int.emod_nonneg _ (by omega)
  · have := Int.emod_lt_of_pos v hpos; omega

/-- A signed result the kernel returns is in the type (no signed overflow). -/
theorem signed_add_in_range (w : Nat) (a b v : Int) (h : kernel .add w true a b = .ok v) :
    inRange w true v := by
  have hun := (kernel_ok_iff_unreached .add w true a b).mp ⟨v, h⟩ .signedOverflow (by decide)
  simp only [reaches, true_and, Classical.not_not] at hun
  unfold kernel at h
  simp only [faults, List.find?, reaches, true_and] at h
  split at h
  · contradiction
  · cases h; exact hun

theorem division_by_zero_reached (op : Op) (hop : op = .div ∨ op = .rem) (w : Nat) (s : Bool)
    (a : Int) : reaches op w s .divisionByZero a 0 := by
  rcases hop with rfl | rfl <;> simp [reaches]

theorem quotient_overflow_reached (op : Op) (hop : op = .div ∨ op = .rem) (w : Nat) :
    reaches op w true .quotientOverflow (lo w true) (-1) := by
  rcases hop with rfl | rfl <;> simp [reaches]

theorem shift_count_reached (op : Op) (hop : op = .shl ∨ op = .shr) (w : Nat) (s : Bool)
    (a b : Int) (hb : b < 0 ∨ (w : Int) ≤ b) : reaches op w s .shiftCount a b := by
  rcases hop with rfl | rfl <;> simp only [reaches, countOk] <;> omega

theorem negative_left_shift_reached (w : Nat) (a b : Int) (ha : a < 0) :
    reaches .shl w true .negativeLeftShift a b := by
  simp [reaches, ha]

/-- The card's three named obligations, decided. -/
theorem int_div_zero : kernel .div 32 true 7 0 = .error .divisionByZero := by decide
theorem int_min_div_neg_one :
    kernel .div 32 true (-2147483648) (-1) = .error .quotientOverflow := by decide
theorem one_shl_32 : kernel .shl 32 true 1 32 = .error .shiftCount := by decide
theorem uint_max_plus_one_wraps : kernel .add 32 false 4294967295 1 = .ok 0 := by decide

/-- Shared vectors with `c_numeric::binary`: (op, width, signed, a, b, outcome). -/
def kernelVectors : List (Op × Nat × Bool × Int × Int × Outcome) := [
  (.add, 32, true, 2147483647, 1, .error .signedOverflow),
  (.add, 32, false, 4294967295, 1, .ok 0),
  (.add, 64, true, 9223372036854775807, 1, .error .signedOverflow),
  (.add, 64, false, 18446744073709551615, 2, .ok 1),
  (.sub, 32, false, 0, 1, .ok 4294967295),
  (.sub, 32, true, -2147483648, 1, .error .signedOverflow),
  (.mul, 32, true, 65536, 65536, .error .signedOverflow),
  (.mul, 32, false, 65536, 65536, .ok 0),
  (.mul, 64, true, 4294967296, 2147483648, .error .signedOverflow),
  (.mul, 64, true, 4294967296, 2147483647, .ok 9223372032559808512),
  (.div, 32, true, 7, 0, .error .divisionByZero),
  (.div, 32, true, -2147483648, -1, .error .quotientOverflow),
  (.rem, 32, true, -2147483648, -1, .error .quotientOverflow),
  (.rem, 32, false, 7, 0, .error .divisionByZero),
  (.div, 32, true, -7, 2, .ok (-3)),
  (.rem, 32, true, -7, 2, .ok (-1)),
  (.shl, 32, true, 1, 32, .error .shiftCount),
  (.shl, 32, true, 1, -1, .error .shiftCount),
  (.shl, 32, true, -1, 1, .error .negativeLeftShift),
  (.shl, 32, true, 1, 31, .error .signedOverflow),
  (.shl, 32, true, 1, 30, .ok 1073741824),
  (.shl, 32, false, 1, 31, .ok 2147483648),
  (.shl, 32, false, 3, 31, .ok 2147483648),
  (.shl, 64, false, 1, 64, .error .shiftCount),
  (.shr, 32, true, -8, 1, .error .negativeRightShift),
  (.shr, 32, true, 8, 1, .ok 4),
  (.shr, 32, false, 8, 32, .error .shiftCount)
]

theorem kernel_vectors_hold :
    kernelVectors.all (fun (op, w, s, a, b, r) => decide (kernel op w s a b = r)) = true := by
  decide

/-! ## Conversions -/

/-- An integer conversion (`c_numeric::convert`): exact when the value fits,
modular for an unsigned target, the implementation-defined case otherwise. -/
def convert (w : Nat) (s : Bool) (v : Int) : Outcome :=
  if inRange w s v then .ok v
  else if s then .error .signedConversion
  else .ok (wrapU w v)

theorem convert_unsigned_defined (w : Nat) (v : Int) : ∃ r, convert w false v = .ok r ∧
    inRange w false r := by
  unfold convert
  split
  · exact ⟨v, rfl, by assumption⟩
  · exact ⟨wrapU w v, rfl, wrap_in_range w v⟩

theorem convert_fits_exact (w : Nat) (s : Bool) (v : Int) (h : inRange w s v) :
    convert w s v = .ok v := by
  simp [convert, h]

/-- Shared vectors with `c_numeric::convert`: (width, signed, value, outcome). -/
def conversionVectors : List (Nat × Bool × Int × Outcome) := [
  (8, false, 300, .ok 44),
  (8, false, -1, .ok 255),
  (32, false, -1, .ok 4294967295),
  (8, true, 127, .ok 127),
  (8, true, 128, .error .signedConversion),
  (16, true, -129, .ok (-129)),
  (32, true, 4294967295, .error .signedConversion),
  (64, true, -1, .ok (-1)),
  (64, false, -1, .ok 18446744073709551615)
]

theorem conversion_vectors_hold :
    conversionVectors.all (fun (w, s, v, r) => decide (convert w s v = r)) = true := by decide

/-! ## Interval discharge is sound -/

/-- A closed range. -/
structure Interval where
  lo : Int
  hi : Int

def Interval.mem (i : Interval) (v : Int) : Prop := i.lo ≤ v ∧ v ≤ i.hi

theorem add_discharge_sound (w : Nat) (A B : Interval)
    (hfit : lo w true ≤ A.lo + B.lo ∧ A.hi + B.hi ≤ hi w true) (a b : Int)
    (ha : A.mem a) (hb : B.mem b) : ¬ reaches .add w true .signedOverflow a b := by
  simp only [reaches, inRange, true_and, Classical.not_not]
  unfold Interval.mem at ha hb
  omega

theorem sub_discharge_sound (w : Nat) (A B : Interval)
    (hfit : lo w true ≤ A.lo - B.hi ∧ A.hi - B.lo ≤ hi w true) (a b : Int)
    (ha : A.mem a) (hb : B.mem b) : ¬ reaches .sub w true .signedOverflow a b := by
  simp only [reaches, inRange, true_and, Classical.not_not]
  unfold Interval.mem at ha hb
  omega

/-- The product bound the walker uses: `|a| ≤ ma`, `|b| ≤ mb` and
`ma * mb ≤ hi` exclude signed overflow (`lo = -hi - 1`). -/
theorem mul_discharge_sound (w : Nat) (ma mb : Nat) (hfit : ((ma * mb : Nat) : Int) ≤ hi w true)
    (a b : Int) (ha : a.natAbs ≤ ma) (hb : b.natAbs ≤ mb) :
    ¬ reaches .mul w true .signedOverflow a b := by
  simp only [reaches, inRange, true_and, Classical.not_not]
  have hprod : (a * b).natAbs ≤ ma * mb := by
    rw [Int.natAbs_mul]; exact Nat.mul_le_mul ha hb
  have hlohi : lo w true = - hi w true - 1 := by simp [lo, hi]; omega
  have h1 : ((a * b).natAbs : Int) ≤ ((ma * mb : Nat) : Int) := by exact_mod_cast hprod
  omega

theorem division_discharge_sound (op : Op) (w : Nat) (s : Bool) (B : Interval)
    (hzero : ¬ B.mem 0) (a b : Int) (hb : B.mem b) : ¬ reaches op w s .divisionByZero a b := by
  intro h
  cases op <;> simp only [reaches] at h <;> subst h <;> exact hzero hb

theorem quotient_discharge_sound (op : Op) (w : Nat) (A B : Interval)
    (hex : ¬ A.mem (lo w true) ∨ ¬ B.mem (-1)) (a b : Int) (ha : A.mem a) (hb : B.mem b) :
    ¬ reaches op w true .quotientOverflow a b := by
  intro h
  cases op <;> simp only [reaches] at h <;> (try exact h.elim) <;>
    obtain ⟨-, rfl, rfl⟩ := h <;> rcases hex with hex | hex <;> contradiction

theorem shift_count_discharge_sound (op : Op) (w : Nat) (s : Bool) (B : Interval)
    (hfit : 0 ≤ B.lo ∧ B.hi < (w : Int)) (a b : Int) (hb : B.mem b) :
    ¬ reaches op w s .shiftCount a b := by
  unfold Interval.mem at hb
  cases op <;> simp only [reaches, countOk, Classical.not_not, not_false_eq_true] <;> omega

theorem negative_shift_discharge_sound (w : Nat) (A : Interval) (hfit : 0 ≤ A.lo) (a b : Int)
    (ha : A.mem a) : ¬ reaches .shl w true .negativeLeftShift a b ∧
      ¬ reaches .shr w true .negativeRightShift a b := by
  unfold Interval.mem at ha
  simp only [reaches, true_and, Int.not_lt]
  omega

/-- `0 ≤ a ≤ ahi`, `0 ≤ b ≤ bhi` and `ahi * 2^bhi ≤ hi` exclude a signed
left-shift overflow. -/
theorem shl_overflow_discharge_sound (w : Nat) (ahi bhi : Nat)
    (hfit : ((ahi * 2 ^ bhi : Nat) : Int) ≤ hi w true) (a b : Int)
    (ha : 0 ≤ a ∧ a ≤ ahi) (hb : 0 ≤ b ∧ b ≤ bhi) :
    ¬ reaches .shl w true .signedOverflow a b := by
  simp only [reaches, true_and, not_and, Int.not_lt]
  intro _ _
  have hat : a.toNat ≤ ahi := by omega
  have hbt : b.toNat ≤ bhi := by omega
  have hnat : a.toNat * 2 ^ b.toNat ≤ ahi * 2 ^ bhi :=
    Nat.mul_le_mul hat (Nat.pow_le_pow_right (by decide) hbt)
  have hcast : a * 2 ^ b.toNat = ((a.toNat * 2 ^ b.toNat : Nat) : Int) := by
    rw [Int.natCast_mul, Int.natCast_pow, Int.toNat_of_nonneg ha.1]; rfl
  rw [hcast]
  exact Int.le_trans (by exact_mod_cast hnat) hfit

theorem conversion_discharge_sound (w : Nat) (A : Interval)
    (hfit : lo w true ≤ A.lo ∧ A.hi ≤ hi w true) (a : Int) (ha : A.mem a) :
    convert w true a = .ok a := by
  unfold Interval.mem at ha
  apply convert_fits_exact
  unfold inRange
  omega

/-! ## Width-independent constants (`c_numeric::portable_constant`)

The cycle-18 evaluator (absorbed `CG01-CYCLE18-EVALUATOR-WIDTH`) accepts a value
only inside its type's portable range, the minimum C17 5.2.4.2.1 guarantees on
every target. Such a value is a value of the type under every data model whose
widths meet those minimums, so it needs no target query. -/

/-- The portable width of a standard type; plain `char` has none (its
signedness is the target's). -/
def portableWidth : Kind → Option Nat
  | .char => none
  | .bool => some 1
  | .schar | .uchar => some 8
  | .short | .ushort | .int | .uint => some 16
  | .long | .ulong => some 32
  | .llong | .ullong => some 64

/-- A model meeting C's minimum widths. -/
def meetsMinimums (m : Model) : Prop :=
  16 ≤ m.short ∧ m.short ≤ m.int ∧ 32 ≤ m.long ∧ 64 ≤ m.longLong

theorem range_mono (w w' : Nat) (s : Bool) (v : Int) (hw : w ≤ w') (h1 : 1 ≤ w)
    (h : inRange w s v) : inRange w' s v := by
  unfold inRange lo hi at *
  have hp : 2 ^ (w - 1) ≤ 2 ^ (w' - 1) := Nat.pow_le_pow_right (by decide) (by omega)
  have hq : 2 ^ w ≤ 2 ^ w' := Nat.pow_le_pow_right (by decide) hw
  have hp' : ((2 ^ (w - 1) : Nat) : Int) ≤ ((2 ^ (w' - 1) : Nat) : Int) := by exact_mod_cast hp
  have hq' : ((2 ^ w : Nat) : Int) ≤ ((2 ^ w' : Nat) : Int) := by exact_mod_cast hq
  cases s <;> simp only [Bool.false_eq_true, ite_false, ite_true] at h ⊢ <;> omega

theorem portable_in_every_model (m : Model) (hm : meetsMinimums m) (k : Kind) (w : Nat)
    (hw : portableWidth k = some w) (v : Int) (h : inRange w (signed lp64 k) v) :
    inRange (width m k) (signed m k) v := by
  unfold meetsMinimums at hm
  cases k <;> simp only [portableWidth, Option.some.injEq, reduceCtorEq] at hw <;> subst hw <;>
    simp only [width, signed] at h ⊢ <;>
    first
    | exact h
    | exact range_mono _ _ _ _ (by omega) (by decide) h

theorem portable_long_holds_on_llp64 (v : Int) (h : inRange 32 true v) :
    inRange (width llp64 .long) (signed llp64 .long) v :=
  portable_in_every_model llp64 (by unfold meetsMinimums llp64; decide) .long 32 rfl v h

end QCore.CNumeric
