/-! Pure scalar-call model. The frontend-to-IR and Rust/Lean correspondence
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
inductive Statement where | bind (e : Expr) | discard (e : Expr)
structure Function where
  arity : Nat
  result : Ty
  statements : List Statement
  returned : Option Expr

/-! Operational meaning, separate from finite implementation resource bounds.
Each call creates a fresh frame. There is no assignment, resource or effect
constructor. Arithmetic admission is the same `arithmetic` function above. -/
mutual
  inductive Eval (program : List Function) : List Int → Expr → Value → Prop where
    | literal (h : inRange n) : Eval program frame (.literal n) (.int n)
    | local (h : frame[slot]? = some n) : Eval program frame (.local slot) (.int n)
    | neg (h : Eval program frame e (.int n)) (hn : narrow (-n) = some v) :
        Eval program frame (.neg e) (.int v)
    | binary (ha : Eval program frame a (.int x)) (hb : Eval program frame b (.int y))
        (hop : arithmetic op x y = some v) : Eval program frame (.binary op a b) (.int v)
    | call (ha : Arguments program frame args values) (hf : Invoke program callee values v) :
        Eval program frame (.call callee args) v
  inductive Arguments (program : List Function) : List Int → List Expr → List Int → Prop where
    | nil : Arguments program frame [] []
    | cons (h : Eval program frame e (.int v)) (hs : Arguments program frame es vs) :
        Arguments program frame (e :: es) (v :: vs)
  inductive Body (program : List Function) : List Int → List Statement → List Int → Prop where
    | nil : Body program frame [] frame
    | bind (h : Eval program frame e (.int v)) (hs : Body program (frame ++ [v]) ss final) :
        Body program frame (.bind e :: ss) final
    | discard (h : Eval program frame e v) (hs : Body program frame ss final) :
        Body program frame (.discard e :: ss) final
  inductive Return (program : List Function) : List Int → Option Expr → Value → Prop where
    | unit : Return program frame none .unit
    | value (h : Eval program frame e v) : Return program frame (some e) v
  inductive Invoke (program : List Function) : Nat → List Int → Value → Prop where
    | run {id : Nat} {f : Function} (hf : program[id]? = some f) (ha : bindArguments f.arity args = some frame)
        (hb : Body program frame f.statements final) (hr : Return program final f.returned v)
        (ht : v.type = f.result) : Invoke program id args v
end

def increment : Function := ⟨1, .i32, [], some (.binary .add (.local 0) (.literal 1))⟩
theorem increment_41_returns_42 : Invoke [increment] 0 [41] (.int 42) := by
  apply Invoke.run (f := increment) (frame := [41]) (final := [41])
  · rfl
  · decide
  · exact Body.nil
  · apply Return.value
    apply Eval.binary (x := 41) (y := 1)
    · exact Eval.local rfl
    · exact Eval.literal (by decide)
    · decide
  · rfl

/-- A declared unit result cannot be used as an integer-returning call. -/
theorem unit_call_not_integer (program : List Function) (id : Nat) (args : List Int)
    (f : Function) (v : Int) (hf : program[id]? = some f) (ht : f.result = .unit) :
    ¬ Invoke program id args (.int v) := by
  intro call
  cases call with
  | run found _ _ _ result =>
    rw [hf] at found
    cases found
    simp [Value.type, ht] at result
end QCore.ScalarCalls
