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

/-! Scalar-call model with explicit stdout observations. The frontend-to-IR and Rust/Lean correspondence
remain review premises, not extraction theorems. Arithmetic uses mathematical
integers with checked i32 bounds. Calls get fresh immutable argument frames;
strictly decreasing call ranks model the acyclic graph admission. -/
namespace QCore.ScalarCalls

def inRange (n : Int) : Prop := -2147483648 ≤ n ∧ n ≤ 2147483647
instance (n : Int) : Decidable (inRange n) := inferInstanceAs (Decidable (_ ∧ _))
def narrow (n : Int) : Option Int := if inRange n then some n else none
inductive Op where | add | sub | mul | div | rem deriving DecidableEq, Repr
/-- Quotient uses truncation toward zero, matching Rust checked_div and C11. -/
def arithmetic (op : Op) (a b : Int) : Option Int :=
  match op with
  | .add => narrow (a + b)
  | .sub => narrow (a - b)
  | .mul => narrow (a * b)
  | .div => if b = 0 ∨ (a = -2147483648 ∧ b = -1) then none else narrow (a.tdiv b)
  | .rem => if b = 0 ∨ (a = -2147483648 ∧ b = -1) then none else narrow (a.tmod b)

theorem narrow_sound (n v : Int) (h : narrow n = some v) : inRange v ∧ v = n := by
  unfold narrow at h
  split at h
  next hn => cases h; exact ⟨hn, rfl⟩
  next => contradiction

theorem arithmetic_in_range (op : Op) (a b v : Int)
    (h : arithmetic op a b = some v) : inRange v := by
  cases op <;> simp only [arithmetic] at h
  · exact (narrow_sound _ _ h).1
  · exact (narrow_sound _ _ h).1
  · exact (narrow_sound _ _ h).1
  · split at h
    · contradiction
    · exact (narrow_sound _ _ h).1
  · split at h
    · contradiction
    · exact (narrow_sound _ _ h).1

theorem add_exact (a b v : Int) (h : arithmetic .add a b = some v) : v = a + b :=
  (narrow_sound _ _ h).2

theorem mul_exact (a b v : Int) (h : arithmetic .mul a b = some v) : v = a * b :=
  (narrow_sound _ _ h).2

theorem division_zero_refuses (a : Int) : arithmetic .div a 0 = none := by simp [arithmetic]
theorem remainder_overflow_refuses : arithmetic .rem (-2147483648) (-1) = none := by decide

def bindArguments (arity : Nat) (args : List Int) : Option (List Int) :=
  if args.length = arity ∧ args.all (fun v => decide (inRange v)) then some args else none

theorem arguments_preserved (arity : Nat) (args frame : List Int)
    (h : bindArguments arity args = some frame) : frame = args ∧ frame.length = arity := by
  unfold bindArguments at h
  split at h
  next ok => cases h; exact ⟨rfl, ok.1⟩
  next => contradiction

theorem wrong_arity_refuses (arity : Nat) (args : List Int) (h : args.length ≠ arity) :
    bindArguments arity args = none := by simp [bindArguments, h]

/-- The rank is derived from graph DFS in Rust, never supplied by source. -/
def callsAdmitted (rank : Nat → Nat) (edges : List (Nat × Nat)) : Prop :=
  ∀ edge ∈ edges, rank edge.2 < rank edge.1

theorem self_call_refuses (rank : Nat → Nat) (edges : List (Nat × Nat)) (f : Nat)
    (h : (f, f) ∈ edges) : ¬ callsAdmitted rank edges := by
  intro admitted
  have hlt : rank f < rank f := admitted (f,f) h
  omega

theorem mutual_call_refuses (rank : Nat → Nat) (edges : List (Nat × Nat)) (a b : Nat)
    (hab : (a,b) ∈ edges) (hba : (b,a) ∈ edges) : ¬ callsAdmitted rank edges := by
  intro admitted
  have h₁ : rank b < rank a := admitted (a,b) hab
  have h₂ : rank a < rank b := admitted (b,a) hba
  omega

/-- Append-only local frames preserve every prior slot. -/
theorem immutable_prefix (frame : List Int) (value : Int) (slot : Nat)
    (h : slot < frame.length) : (frame ++ [value])[slot]? = frame[slot]? := by
  simp [List.getElem?_append, h]

/-- The same boundary vectors are exercised against the Rust kernel. -/
def agreement : List (Op × Int × Int × Option Int) := [
  (.add, 41, 1, some 42), (.add, 2147483647, 1, none),
  (.sub, -2147483648, 1, none), (.mul, 50000, 50000, none),
  (.div, -7, 3, some (-2)), (.rem, -7, 3, some (-1)),
  (.div, 1, 0, none), (.rem, -2147483648, -1, none)]
theorem agreement_holds : agreement.all (fun (op,a,b,v) => arithmetic op a b == v) = true := by decide
inductive Ty where | i32 | unit deriving DecidableEq
inductive Value where | int (n : Int) | unit deriving DecidableEq
def Value.type : Value → Ty | .int _ => .i32 | .unit => .unit
inductive Expr where
  | literal (n : Int) | local (slot : Nat) | neg (e : Expr)
  | binary (op : Op) (a b : Expr) | call (callee : Nat) (args : List Expr)
inductive Statement where | bind (e : Expr) | discard (e : Expr) | print (e : Expr)
structure Function where
  arity : Nat
  result : Ty
  statements : List Statement
  returned : Option Expr

/-! Operational meaning, separate from finite implementation resource bounds.
The final list records stdout i32 lines. Calls create fresh frames, and traces
concatenate in left-to-right evaluation order. Only `Body.print` adds output;
return values and arithmetic failures have no implicit diagnostic output. -/
mutual
  inductive Eval (program : List Function) : List Int → Expr → Value → List Int → Prop where
    | literal (h : inRange n) : Eval program frame (.literal n) (.int n) []
    | local (h : frame[slot]? = some n) : Eval program frame (.local slot) (.int n) []
    | neg (h : Eval program frame e (.int n) out) (hn : narrow (-n) = some v) :
        Eval program frame (.neg e) (.int v) out
    | binary (ha : Eval program frame a (.int x) left) (hb : Eval program frame b (.int y) right)
        (hop : arithmetic op x y = some v) :
        Eval program frame (.binary op a b) (.int v) (left ++ right)
    | call (ha : Arguments program frame args values before) (hf : Invoke program callee values v out) :
        Eval program frame (.call callee args) v (before ++ out)
  inductive Arguments (program : List Function) : List Int → List Expr → List Int → List Int → Prop where
    | nil : Arguments program frame [] [] []
    | cons (h : Eval program frame e (.int v) out) (hs : Arguments program frame es vs rest) :
        Arguments program frame (e :: es) (v :: vs) (out ++ rest)
  inductive Body (program : List Function) : List Int → List Statement → List Int → List Int → Prop where
    | nil : Body program frame [] frame []
    | bind (h : Eval program frame e (.int v) out) (hs : Body program (frame ++ [v]) ss final rest) :
        Body program frame (.bind e :: ss) final (out ++ rest)
    | discard (h : Eval program frame e v out) (hs : Body program frame ss final rest) :
        Body program frame (.discard e :: ss) final (out ++ rest)
    | print (h : Eval program frame e (.int v) out) (hs : Body program frame ss final rest) :
        Body program frame (.print e :: ss) final (out ++ [v] ++ rest)
  inductive Return (program : List Function) : List Int → Option Expr → Value → List Int → Prop where
    | unit : Return program frame none .unit []
    | value (h : Eval program frame e v out) : Return program frame (some e) v out
  inductive Invoke (program : List Function) : Nat → List Int → Value → List Int → Prop where
    | run {id : Nat} {f : Function} (hf : program[id]? = some f) (ha : bindArguments f.arity args = some frame)
        (hb : Body program frame f.statements final before) (hr : Return program final f.returned v out)
        (ht : v.type = f.result) : Invoke program id args v (before ++ out)
end

def increment : Function := ⟨1, .i32, [], some (.binary .add (.local 0) (.literal 1))⟩
theorem increment_41_returns_42 : Invoke [increment] 0 [41] (.int 42) [] := by
  apply Invoke.run (f := increment) (frame := [41]) (final := [41]) (before := []) (out := [])
  · rfl
  · decide
  · exact Body.nil
  · apply Return.value
    apply Eval.binary (x := 41) (y := 1) (left := []) (right := [])
    · exact Eval.local rfl
    · exact Eval.literal (by decide)
    · decide
  · rfl

/-- A declared unit result cannot be used as an integer-returning call. -/
theorem unit_call_not_integer (program : List Function) (id : Nat) (args out : List Int)
    (f : Function) (v : Int) (hf : program[id]? = some f) (ht : f.result = .unit) :
    ¬ Invoke program id args (.int v) out := by
  intro call
  cases call with
  | run found _ _ _ result =>
    rw [hf] at found
    cases found
    simp [Value.type, ht] at result

/-- Printing observes the evaluated argument and does not change its local slot. -/
def showValue : Function := ⟨1, .i32, [.print (.local 0)], some (.local 0)⟩
theorem show_42_returns_and_prints : Invoke [showValue] 0 [42] (.int 42) [42] := by
  apply Invoke.run (f := showValue) (frame := [42]) (final := [42]) (before := [42]) (out := [])
  · rfl
  · decide
  · exact Body.print (out := []) (rest := []) (Eval.local rfl) Body.nil
  · exact Return.value (Eval.local rfl)
  · rfl

/-- Bottom-up effect closure over the already admitted acyclic call graph.
The source row is absent (inferred), explicitly empty, or explicitly io. The
frontend extraction and graph traversal correspondence remain review premises. -/
def requiredIo (direct : Bool) (callees : List Bool) : Bool := direct || callees.any id
def rowAdmitted (declared : Option Bool) (required : Bool) : Bool :=
  match declared with | none => true | some allowed => !required || allowed

theorem direct_print_requires_io (callees : List Bool) : requiredIo true callees = true := by
  simp [requiredIo]
theorem callee_io_requires_io (direct : Bool) (callees : List Bool) (h : true ∈ callees) :
    requiredIo direct callees = true := by
  simp [requiredIo, h]
theorem explicit_empty_refuses_io : rowAdmitted (some false) true = false := by decide
theorem inferred_or_explicit_io_admitted (required : Bool) :
    rowAdmitted none required = true ∧ rowAdmitted (some true) required = true := by
  cases required <;> decide
end QCore.ScalarCalls
