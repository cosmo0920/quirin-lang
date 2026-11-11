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

/-! Route T's operator and literal closure (card CG01-T-OPERATOR-CLOSURE,
gateway rule 10r): a model of `crates/qcore-c-gateway/src/typed_source_nodes.rs`
and of the importer's grammar hook in `typed_source.rs`.

Bound mechanically, not by transcription alone: the Rust test
`typed_source::operators_tests::cg01_lean_vectors_match_the_rust_tables` reads
`legacyBinary`, `legacyUnary`, `nodeTable` and `kernelVectors` from this file
and compares them with `LEGACY_BINARY`, `LEGACY_UNARY`, `NODE_TABLE` and
`checked_binary`. The Clang-to-`Expr` reading itself stays a review premise.

Two properties are stated.
* Import totality and refusal: every opcode is read as legacy, as a CG01 node or
  refused; an unlisted operator is refused, and a CG01 node is admitted only if a
  valid consumer admission lists it.
* Consumer soundness: an admitted custody-neutral node leaves the custody frame
  unchanged, and the numeric kernel refuses every C fault case. -/
namespace QCore.TypedSourceOperators

inductive Fault where
  | signedOverflow | divisionByZero | quotientOverflow | shiftCount | negativeLeftShift
  deriving DecidableEq, Repr

inductive Class where
  | custodyNeutral | numericallyEvaluated | obligationBearing
  deriving DecidableEq, Repr

structure Facts where
  spelling : String
  faults : List Fault
  store : Bool
  deriving DecidableEq, Repr

/-- Binary operators route T read before CG01. -/
def legacyBinary : List String :=
  ["&", "|", "==", "!=", "<", ">", "+", "*", "/", "%", "&&", "||"]

/-- Unary operators route T read before CG01. -/
def legacyUnary : List String := ["&", "*", "-", "!"]

/-- Every CG01 node with its intrinsic facts (`NODE_TABLE`). -/
def nodeTable : List Facts := [
  ⟨"binary -", [.signedOverflow], false⟩,
  ⟨"binary <=", [], false⟩,
  ⟨"binary >=", [], false⟩,
  ⟨"binary ^", [], false⟩,
  ⟨"binary <<", [.signedOverflow, .shiftCount, .negativeLeftShift], false⟩,
  ⟨"binary >>", [.shiftCount], false⟩,
  ⟨"unary +", [], false⟩,
  ⟨"unary ~", [], false⟩,
  ⟨"compound +=", [.signedOverflow], true⟩,
  ⟨"compound -=", [.signedOverflow], true⟩,
  ⟨"compound *=", [.signedOverflow], true⟩,
  ⟨"compound /=", [.divisionByZero, .quotientOverflow], true⟩,
  ⟨"compound %=", [.divisionByZero, .quotientOverflow], true⟩,
  ⟨"compound <<=", [.signedOverflow, .shiftCount, .negativeLeftShift], true⟩,
  ⟨"compound >>=", [.shiftCount], true⟩,
  ⟨"compound &=", [], true⟩,
  ⟨"compound ^=", [], true⟩,
  ⟨"compound |=", [], true⟩,
  ⟨"literal floating", [], false⟩,
  ⟨"literal character", [], false⟩
]

/-! ## Import totality and refusal -/

/-- How the importer reads a binary or unary opcode. -/
inductive Reading where
  | legacy
  | node (spelling : String)
  | refused
  deriving DecidableEq, Repr

def isNode (spelling : String) : Bool := nodeTable.any (·.spelling == spelling)

def readBinary (op : String) : Reading :=
  if op ∈ legacyBinary then .legacy
  else if isNode ("binary " ++ op) then .node ("binary " ++ op)
  else .refused

def readUnary (op : String) : Reading :=
  if op ∈ legacyUnary then .legacy
  else if isNode ("unary " ++ op) then .node ("unary " ++ op)
  else .refused

/-- An opcode that is neither legacy nor a CG01 node is refused. -/
theorem unlisted_binary_refused (op : String) (h₁ : op ∉ legacyBinary)
    (h₂ : isNode ("binary " ++ op) = false) : readBinary op = .refused := by
  simp [readBinary, h₁, h₂]

theorem unlisted_unary_refused (op : String) (h₁ : op ∉ legacyUnary)
    (h₂ : isNode ("unary " ++ op) = false) : readUnary op = .refused := by
  simp [readUnary, h₁, h₂]

/-- A read CG01 node is one of the table's nodes. -/
theorem read_binary_node (op spelling : String) (h : readBinary op = .node spelling) :
    isNode spelling = true := by
  unfold readBinary at h
  split at h
  · contradiction
  · split at h
    · cases h; assumption
    · contradiction

theorem comma_refused : readBinary "," = .refused := by decide
theorem assignment_refused : readBinary "=" = .refused := by decide
theorem increment_refused : readUnary "++" = .refused := by decide
theorem xor_is_a_node : readBinary "^" = .node "binary ^" := by decide

/-- A consumer's classification rows (`NodeAdmission`). -/
abbrev Admission := List (String × Class)

def allowed (f : Facts) : Class → Bool
  | .custodyNeutral => f.faults.isEmpty && !f.store
  | .numericallyEvaluated => !f.store
  | .obligationBearing => true

def rowValid (row : String × Class) : Bool :=
  match nodeTable.find? (·.spelling == row.1) with
  | some f => allowed f row.2
  | none => false

def valid (a : Admission) : Bool := a.all rowValid && (a.map (·.1)).Nodup

/-- The importer admits a CG01 node only through a valid admission that lists it. -/
def admit (a : Admission) (spelling : String) : Option Class :=
  if valid a then (a.find? (·.1 == spelling)).map (·.2) else none

theorem legacy_admits_nothing (spelling : String) : admit [] spelling = none := by
  simp [admit, valid]

theorem unlisted_node_refused (a : Admission) (spelling : String)
    (h : ∀ row ∈ a, row.1 ≠ spelling) : admit a spelling = none := by
  unfold admit
  split
  · rw [List.find?_eq_none.mpr]
    · rfl
    · intro row hrow
      simpa using h row hrow
  · rfl

theorem invalid_admission_refuses (a : Admission) (spelling : String) (h : valid a = false) :
    admit a spelling = none := by
  simp [admit, h]

/-- Admission soundness: an admitted class is one the node's facts allow. -/
theorem admit_sound (a : Admission) (spelling : String) (c : Class)
    (h : admit a spelling = some c) :
    ∃ f ∈ nodeTable, f.spelling = spelling ∧ allowed f c = true := by
  unfold admit at h
  split at h
  next hv =>
    cases hfind : a.find? (·.1 == spelling) with
    | none => simp [hfind] at h
    | some row =>
      simp [hfind] at h
      have hrow := List.mem_of_find?_eq_some hfind
      have hname : row.1 = spelling := by simpa using List.find?_some hfind
      have hall : a.all rowValid = true := by
        simp only [valid, Bool.and_eq_true] at hv
        exact hv.1
      have hvalid : rowValid row = true := List.all_eq_true.mp hall row hrow
      unfold rowValid at hvalid
      cases hf : nodeTable.find? (·.spelling == row.1) with
      | none => simp [hf] at hvalid
      | some f =>
        simp [hf] at hvalid
        refine ⟨f, List.mem_of_find?_eq_some hf, ?_, ?_⟩
        · have := List.find?_some hf
          simp at this
          rw [this, hname]
        · rw [← h]; exact hvalid
  next => contradiction

/-! ## Consumer soundness: the custody frame -/

/-- The place accesses one node adds to a path: a compound assignment reads its
evaluated place once and writes it once; every other node accesses nothing. -/
inductive Access where
  | read (place : Nat)
  | write (place : Nat)
  deriving DecidableEq, Repr

def accesses (f : Facts) (place : Nat) : List Access :=
  if f.store then [.read place, .write place] else []

/-- Run accesses over an abstract custody frame; only a write can change it. -/
def run {σ : Type} (update : Nat → σ → σ) (frame : σ) : List Access → σ
  | [] => frame
  | .read _ :: rest => run update frame rest
  | .write place :: rest => run update (update place frame) rest

theorem compound_reads_once_writes_once (f : Facts) (place : Nat) (h : f.store = true) :
    accesses f place = [.read place, .write place] := by
  simp [accesses, h]

/-- A node a consumer admits as custody-neutral leaves every custody frame unchanged. -/
theorem neutral_leaves_frame {σ : Type} (a : Admission) (spelling : String)
    (h : admit a spelling = some .custodyNeutral) :
    ∃ f ∈ nodeTable, f.spelling = spelling ∧ f.faults = [] ∧
      ∀ (update : Nat → σ → σ) (frame : σ) (place : Nat),
        run update frame (accesses f place) = frame := by
  obtain ⟨f, hf, hs, hallowed⟩ := admit_sound a spelling .custodyNeutral h
  simp only [allowed, Bool.and_eq_true, Bool.not_eq_eq_eq_not, Bool.not_true,
    List.isEmpty_iff] at hallowed
  refine ⟨f, hf, hs, hallowed.1, ?_⟩
  intro update frame place
  simp [accesses, hallowed.2, run]

/-- A fault-bearing node can never be admitted as custody-neutral. -/
theorem faulty_not_neutral (a : Admission) (spelling : String) (f : Facts)
    (hf : f ∈ nodeTable) (hs : f.spelling = spelling) (hfault : f.faults ≠ [])
    (hunique : ∀ g ∈ nodeTable, g.spelling = spelling → g = f) :
    admit a spelling ≠ some .custodyNeutral := by
  intro h
  obtain ⟨g, hg, hgs, hallowed⟩ := admit_sound a spelling .custodyNeutral h
  have := hunique g hg hgs
  subst this
  simp [allowed, List.isEmpty_iff, hfault] at hallowed

theorem spellings_unique : (nodeTable.map (·.spelling)).Nodup := by decide

/-! ## Consumer soundness: the numeric kernel (`checked_binary`) -/

inductive BinOp where
  | add | sub | mul | div | rem | shl | shr
  deriving DecidableEq, Repr

def lo (w : Nat) (s : Bool) : Int := if s then -((2 ^ (w - 1) : Nat) : Int) else 0
def hi (w : Nat) (s : Bool) : Int :=
  if s then ((2 ^ (w - 1) : Nat) : Int) - 1 else ((2 ^ w : Nat) : Int) - 1
def inRange (w : Nat) (s : Bool) (v : Int) : Bool := decide (lo w s ≤ v) && decide (v ≤ hi w s)

/-- The exact result, or `none` on a fault the operands alone decide. -/
def exact (op : BinOp) (w : Nat) (s : Bool) (a b : Int) : Option Int :=
  match op with
  | .add => some (a + b)
  | .sub => some (a - b)
  | .mul => some (a * b)
  | .div => if b = 0 ∨ (s = true ∧ a = lo w s ∧ b = -1) then none else some (a.tdiv b)
  | .rem => if b = 0 ∨ (s = true ∧ a = lo w s ∧ b = -1) then none else some (a.tmod b)
  | .shl => if b < 0 ∨ (w : Int) ≤ b ∨ a < 0 then none else some (a * 2 ^ b.toNat)
  | .shr => if b < 0 ∨ (w : Int) ≤ b ∨ a < 0 then none else some (a / 2 ^ b.toNat)

def isShift : BinOp → Bool
  | .shl | .shr => true
  | _ => false

/-- Signed results outside the type refuse; unsigned results wrap, as C defines. -/
def checkedBinary (op : BinOp) (w : Nat) (s : Bool) (a b : Int) : Option Int :=
  if w = 0 ∨ 64 < w then none
  else if inRange w s a = false then none
  else if isShift op = false ∧ inRange w s b = false then none
  else match exact op w s a b with
    | none => none
    | some v => if s then (if inRange w s v then some v else none) else some (v % ((2 ^ w : Nat) : Int))

theorem checked_some_exact (op : BinOp) (w : Nat) (s : Bool) (a b v : Int)
    (h : checkedBinary op w s a b = some v) : ∃ e, exact op w s a b = some e := by
  unfold checkedBinary at h
  split at h
  · contradiction
  · split at h
    · contradiction
    · split at h
      · contradiction
      · cases he : exact op w s a b with
        | none => simp [he] at h
        | some e => exact ⟨e, rfl⟩

theorem division_by_zero_refused (op : BinOp) (hop : op = .div ∨ op = .rem) (w : Nat)
    (s : Bool) (a : Int) : checkedBinary op w s a 0 = none := by
  cases h : checkedBinary op w s a 0 with
  | none => rfl
  | some v =>
    obtain ⟨e, he⟩ := checked_some_exact op w s a 0 v h
    rcases hop with rfl | rfl <;> simp [exact] at he

theorem quotient_overflow_refused (op : BinOp) (hop : op = .div ∨ op = .rem) (w : Nat)
    (b : Int) (hb : b = -1) : checkedBinary op w true (lo w true) b = none := by
  cases h : checkedBinary op w true (lo w true) b with
  | none => rfl
  | some v =>
    obtain ⟨e, he⟩ := checked_some_exact op w true (lo w true) b v h
    subst hb
    rcases hop with rfl | rfl <;> simp [exact] at he

theorem shift_count_refused (op : BinOp) (hop : op = .shl ∨ op = .shr) (w : Nat) (s : Bool)
    (a b : Int) (hb : b < 0 ∨ (w : Int) ≤ b) : checkedBinary op w s a b = none := by
  cases h : checkedBinary op w s a b with
  | none => rfl
  | some v =>
    obtain ⟨e, he⟩ := checked_some_exact op w s a b v h
    rcases hop with rfl | rfl <;> rcases hb with hb | hb <;> simp [exact, hb] at he

theorem negative_left_shift_refused (w : Nat) (s : Bool) (a b : Int) (ha : a < 0) :
    checkedBinary .shl w s a b = none := by
  cases h : checkedBinary .shl w s a b with
  | none => rfl
  | some v =>
    obtain ⟨e, he⟩ := checked_some_exact .shl w s a b v h
    simp [exact, ha] at he

/-- No signed overflow: a signed result the kernel returns is in the type's range. -/
theorem signed_result_in_range (op : BinOp) (w : Nat) (a b v : Int)
    (h : checkedBinary op w true a b = some v) : inRange w true v = true := by
  unfold checkedBinary at h
  split at h
  · contradiction
  · split at h
    · contradiction
    · split at h
      · contradiction
      · cases he : exact op w true a b with
        | none => simp [he] at h
        | some e =>
          simp only [he, ite_true] at h
          split at h
          next hr => cases h; exact hr
          next => contradiction

/-- Shared vectors with `checked_binary`: (op, width, signed, a, b, result). -/
def kernelVectors : List (BinOp × Nat × Bool × Int × Int × Option Int) := [
  (.sub, 32, true, -2147483648, 1, none),
  (.sub, 32, false, 0, 1, some 4294967295),
  (.div, 32, true, -2147483648, -1, none),
  (.rem, 32, true, -2147483648, -1, none),
  (.rem, 32, true, 7, 0, none),
  (.div, 32, true, -7, 2, some (-3)),
  (.rem, 32, true, -7, 2, some (-1)),
  (.shl, 32, true, 1, 31, none),
  (.shl, 32, true, 1, 30, some 1073741824),
  (.shl, 32, true, -1, 1, none),
  (.shl, 32, true, 1, -1, none),
  (.shl, 32, false, 1, 31, some 2147483648),
  (.shl, 64, false, 1, 64, none),
  (.shr, 32, true, -8, 1, none),
  (.shr, 32, true, 8, 1, some 4),
  (.shr, 32, true, 8, 32, none),
  (.mul, 64, true, 4294967296, 2147483648, none),
  (.add, 64, false, 18446744073709551615, 1, some 0),
  (.add, 32, true, 2147483647, 1, none)
]

theorem kernel_vectors_hold :
    kernelVectors.all (fun (op, w, s, a, b, r) => checkedBinary op w s a b == r) = true := by
  decide

end QCore.TypedSourceOperators
