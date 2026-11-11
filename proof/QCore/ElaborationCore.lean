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

/-! The common expression elaborator (card CG02-ELABORATION-CORE, gateway rule
10s): a model of `crates/qcore-c-gateway/src/elaboration.rs`.

Bound mechanically, not by transcription alone: the Rust test
`elaboration::tests::cg02_lean_vectors_match_the_rust_tables` reads `formTable`,
`commuteVectors` and `orderVectors` from this file. It compares the first two
with `FORM_TABLE` and `commutes`, and runs each `orderVectors` C source through
Clang and `elaborate`, comparing the admission. The Clang-JSON-to-graph reading
itself stays a review premise (trust boundary 28).

Stated here:
* **Forms.** Every form is elaborated, or refused by name with its owning
  card; an unlisted form is refused.
* **The evaluation-graph semantics and order independence.** Events run over a
  store. Two events the check calls commuting commute on every store. Hence
  every interleaving of two operands whose events pairwise commute (every
  serialisation C permits for an unsequenced pair) reaches the same store, and
  a read no unordered event writes observes the same value. A same-place
  write/write or write/read pair, and any call, never commute.
* **Value forms.** Comma sequencing, pre/post increment value separation, and
  the compound assignment's single place evaluation. -/
namespace QCore.ElaborationCore

/-! ## Forms -/

inductive Disposition where
  | admitted
  | sequenced
  | refused (card : String)
  /-- Card CG06 (rule 10z): elaborated into a branch region with exclusive
  arms (`QCore.ValueConnectives`). -/
  | branching
  deriving DecidableEq, Repr

/-- `FORM_TABLE`. -/
def formTable : List (String × Disposition) := [
  ("literal", .admitted),
  ("object read", .admitted),
  ("path read", .admitted),
  ("scalar unary", .admitted),
  ("scalar binary", .admitted),
  ("scalar cast", .admitted),
  ("assignment value", .admitted),
  ("compound assignment value", .admitted),
  ("pre increment", .admitted),
  ("post increment", .admitted),
  ("pre decrement", .admitted),
  ("post decrement", .admitted),
  ("comma", .admitted),
  ("declaration", .admitted),
  ("direct call", .sequenced),
  ("value &&", .branching),
  ("value ||", .branching),
  ("conditional", .branching),
  ("omitted-middle conditional", .branching),
  ("nested call", .refused "CG05"),
  ("indirect call", .refused "CG22"),
  ("pointer store", .refused "CG11"),
  ("pointer arithmetic", .refused "CG16"),
  ("type query operand", .refused "CG08"),
  ("generic selection", .admitted),
  ("statement expression", .admitted),
  ("compound literal", .refused "CG14"),
  ("volatile access", .refused "CG15"),
  ("atomic access", .refused "CG19"),
  -- Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): `QCore.CastRepresentation`.
  ("representation cast", .admitted),
  ("pointer null test", .admitted),
  ("pointer to integer", .admitted),
  ("integer to pointer", .admitted),
  ("pointer representation", .admitted),
  ("null pointer constant", .admitted),
  ("discarded value", .admitted),
  ("unevaluated type query", .admitted),
  ("evaluated type query", .admitted),
  -- Card CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (rule 11g): `QCore.StatementExpressions`.
  ("statement expression control", .refused "CG07")
]

def disposition (form : String) : Option Disposition :=
  (formTable.find? (·.1 == form)).map (·.2)

inductive Reading where
  | elaborated
  | refusedBy (card : String)
  | unlisted
  deriving DecidableEq, Repr

def read (form : String) : Reading :=
  match disposition form with
  | some .admitted => .elaborated
  | some .sequenced => .elaborated
  | some .branching => .elaborated
  | some (.refused card) => .refusedBy card
  | none => .unlisted

theorem forms_unique : (formTable.map (·.1)).Nodup := by decide

/-- A form the table does not list is refused. -/
theorem unlisted_refused (form : String) (h : disposition form = none) :
    read form = .unlisted := by
  simp [read, h]

/-- A listed refusal names its owning card. -/
theorem listed_refusal_named (form card : String)
    (h : disposition form = some (.refused card)) : read form = .refusedBy card := by
  simp [read, h]

/-- Card CG06 (rule 10z) lifted the reservation: the value connectives and
conditionals are elaborated as branch regions. -/
theorem value_connectives_branch :
    read "value &&" = .elaborated ∧ read "value ||" = .elaborated ∧
      read "conditional" = .elaborated ∧ read "omitted-middle conditional" = .elaborated := by
  decide

theorem out_of_scope_refused :
    read "nested call" = .refusedBy "CG05" ∧ read "pointer store" = .refusedBy "CG11" ∧
      read "pointer arithmetic" = .refusedBy "CG16" := by decide

theorem value_forms_elaborated :
    read "assignment value" = .elaborated ∧ read "compound assignment value" = .elaborated ∧
      read "post increment" = .elaborated ∧ read "pre increment" = .elaborated ∧
      read "comma" = .elaborated := by decide

/-! ## Events and commutation -/

abbrev Place := Nat

/-- The abstract events the order check sees. -/
inductive Ev where
  | read (p : Place)
  | write (p : Place)
  | call
  deriving DecidableEq, Repr

/-- `commutes` in Rust, over distinct named objects (disjoint iff distinct). -/
def commutes : Ev → Ev → Bool
  | .read _, .read _ => true
  | .read p, .write q => p != q
  | .write p, .read q => p != q
  | .write p, .write q => p != q
  | _, _ => false

theorem commutes_symm (a b : Ev) : commutes a b = commutes b a := by
  cases a <;> cases b <;> simp [commutes, bne_comm]

/-- `commuteVectors`: event pairs and the decision the Rust `commutes` gives on
objects numbered by declaration. -/
def commuteVectors : List (Ev × Ev × Bool) := [
  (.read 0, .read 0, true),
  (.read 0, .read 1, true),
  (.read 0, .write 0, false),
  (.write 0, .read 0, false),
  (.write 0, .write 0, false),
  (.read 0, .write 1, true),
  (.write 0, .write 1, true),
  (.call, .read 0, false),
  (.write 0, .call, false),
  (.call, .call, false)
]

theorem commute_vectors_hold :
    commuteVectors.all (fun v => commutes v.1 v.2.1 == v.2.2) = true := by decide

theorem write_read_conflict (p : Place) :
    commutes (.write p) (.read p) = false ∧ commutes (.read p) (.write p) = false := by
  simp [commutes]

theorem write_write_conflict (p : Place) : commutes (.write p) (.write p) = false := by
  simp [commutes]

theorem call_never_commutes (e : Ev) : commutes .call e = false ∧ commutes e .call = false := by
  cases e <;> simp [commutes]

/-! ## The evaluation-graph semantics -/

abbrev Store := Place → Int

/-- A concrete event: a read, a write of one value, or an opaque call. -/
inductive CEv where
  | read (p : Place)
  | write (p : Place) (v : Int)
  | call (f : Store → Store)

def erase : CEv → Ev
  | .read p => .read p
  | .write p _ => .write p
  | .call _ => .call

def update (s : Store) (p : Place) (v : Int) : Store := fun q => if q = p then v else s q

def step : CEv → Store → Store
  | .read _, s => s
  | .write p v, s => update s p v
  | .call f, s => f s

def run : List CEv → Store → Store
  | [], s => s
  | e :: es, s => run es (step e s)

theorem run_append (a b : List CEv) (s : Store) : run (a ++ b) s = run b (run a s) := by
  induction a generalizing s with
  | nil => rfl
  | cons x a ih => exact ih (step x s)

/-- Two events the check calls commuting commute on every store. -/
theorem commutes_sound (a b : CEv) (h : commutes (erase a) (erase b) = true) (s : Store) :
    step a (step b s) = step b (step a s) := by
  cases a with
  | read p => cases b <;> simp_all [erase, commutes, step]
  | call f => cases b <;> simp_all [erase, commutes]
  | write p v =>
    cases b with
    | read q => rfl
    | call g => simp_all [erase, commutes]
    | write q w =>
      have hne : p ≠ q := by simpa [erase, commutes] using h
      funext r
      simp only [step, update]
      by_cases h₁ : r = p
      · subst h₁; simp [hne]
      · by_cases h₂ : r = q
        · subst h₂; simp [h₁]
        · simp [h₁, h₂]

/-- An event commuting with every event of `a` can be moved in front of `a`. -/
theorem step_through (y : CEv) (a : List CEv)
    (h : ∀ x ∈ a, commutes (erase x) (erase y) = true) (s : Store) :
    run a (step y s) = step y (run a s) := by
  induction a generalizing s with
  | nil => rfl
  | cons x a ih =>
    show run a (step x (step y s)) = step y (run a (step x s))
    rw [commutes_sound x y (h x (List.mem_cons_self ..)) s]
    exact ih (fun x' hx => h x' (List.mem_cons_of_mem _ hx)) (step x s)

/-- The serialisations C permits for two unsequenced operands: every
interleaving that keeps each operand's own order. -/
inductive Interleave {α : Type} : List α → List α → List α → Prop where
  | nil : Interleave [] [] []
  | left {a b m : List α} (x : α) : Interleave a b m → Interleave (x :: a) b (x :: m)
  | right {a b m : List α} (y : α) : Interleave a b m → Interleave a (y :: b) (y :: m)

/-- **Order independence.** When every event of one operand commutes with every
event of the other, every permitted serialisation reaches the same store. -/
theorem interleave_run (a b m : List CEv)
    (hc : ∀ x ∈ a, ∀ y ∈ b, commutes (erase x) (erase y) = true)
    (hi : Interleave a b m) (s : Store) : run m s = run (a ++ b) s := by
  induction hi generalizing s with
  | nil => rfl
  | left x _ ih =>
    exact ih (fun x' hx y hy => hc x' (List.mem_cons_of_mem _ hx) y hy) (step x s)
  | @right a b m y _ ih =>
    show run m (step y s) = run (a ++ y :: b) s
    rw [ih (fun x hx y' hy => hc x hx y' (List.mem_cons_of_mem _ hy)) (step y s),
      run_append, run_append,
      step_through y a (fun x hx => hc x hx y (List.mem_cons_self ..)) s]
    rfl

/-- Whether a concrete event may change place `p`. -/
def writesTo (p : Place) : CEv → Bool
  | .read _ => false
  | .write q _ => q == p
  | .call _ => true

/-- A read unordered only with events that do not write its place observes the
same value in every serialisation. -/
theorem read_stable (p : Place) (l : List CEv) (h : ∀ e ∈ l, writesTo p e = false)
    (s : Store) : run l s p = s p := by
  induction l generalizing s with
  | nil => rfl
  | cons e l ih =>
    show run l (step e s) p = s p
    rw [ih (fun e' he => h e' (List.mem_cons_of_mem _ he))]
    have he := h e (List.mem_cons_self ..)
    cases e with
    | read q => rfl
    | write q v =>
      have hq : q ≠ p := by simpa [writesTo] using he
      simp [step, update, Ne.symm hq]
    | call f => simp [writesTo] at he

/-- The refusal is necessary: two unordered writes of one place with different
values leave different stores in the two orders. -/
theorem write_order_matters (p : Place) (v w : Int) (hv : v ≠ w) (s : Store) :
    step (.write p v) (step (.write p w) s) p ≠ step (.write p w) (step (.write p v) s) p := by
  simp [step, update, hv]

/-! ## The elaborator over a small expression language -/

/-- The value forms, with places named by number (a place's address is pure). -/
inductive Expr where
  | lit (n : Int)
  | var (p : Place)
  | assign (p : Place) (e : Expr)
  /-- `P = e` where computing `P`'s address evaluates `addr` (`a[i] = e`). -/
  | store (addr : Expr) (p : Place) (e : Expr)
  | compound (op : Int → Int → Int) (p : Place) (e : Expr)
  | preInc (p : Place)
  | postInc (p : Place)
  | comma (a b : Expr)
  | bin (op : Int → Int → Int) (a b : Expr)
  /-- A call with two actuals. -/
  | call2 (a b : Expr)

/-- The events of an elaborated expression and its side effects (the events a
consumer of its value is *not* sequenced after). -/
structure Part where
  all : List Ev
  sides : List Ev

def cross (a b : List Ev) : Bool := a.all fun x => b.all fun y => commutes x y

/-- The elaborator: its events and whether the order check admits them. -/
def elaborate : Expr → Part × Bool
  | .lit _ => (⟨[], []⟩, true)
  | .var p => (⟨[.read p], []⟩, true)
  | .assign p e =>
    let (pe, ok) := elaborate e
    (⟨pe.all ++ [.write p], pe.sides ++ [.write p]⟩, ok && pe.sides.all (commutes (.write p)))
  | .store addr p e =>
    let (pa, oka) := elaborate addr
    let (pe, oke) := elaborate e
    (⟨pa.all ++ pe.all ++ [.write p], pa.sides ++ pe.sides ++ [.write p]⟩,
      oka && oke && cross pa.all pe.all && (pa.sides ++ pe.sides).all (commutes (.write p)))
  | .compound _ p e =>
    let (pe, ok) := elaborate e
    (⟨.read p :: pe.all ++ [.write p], pe.sides ++ [.write p]⟩,
      ok && cross [.read p] pe.all && pe.sides.all (commutes (.write p)))
  | .preInc p => (⟨[.read p, .write p], [.write p]⟩, true)
  | .postInc p => (⟨[.read p, .write p], [.write p]⟩, true)
  | .comma a b =>
    let (pa, oka) := elaborate a
    let (pb, okb) := elaborate b
    (⟨pa.all ++ pb.all, pb.sides⟩, oka && okb)
  | .bin _ a b =>
    let (pa, oka) := elaborate a
    let (pb, okb) := elaborate b
    (⟨pa.all ++ pb.all, pa.sides ++ pb.sides⟩, oka && okb && cross pa.all pb.all)
  | .call2 a b =>
    let (pa, oka) := elaborate a
    let (pb, okb) := elaborate b
    (⟨pa.all ++ pb.all ++ [.call], []⟩, oka && okb && cross pa.all pb.all)

def admit (e : Expr) : Bool := (elaborate e).2

/-- `orderVectors`: a C source, its model, and the admission. The Rust test runs
each source through Clang and `elaborate`. -/
def orderVectors : List (String × Expr × Bool) := [
  ("int subject(int i) { i = i++; return i; }", .assign 0 (.postInc 0), false),
  ("int subject(int i, int *a) { a[i] = i++; return i; }", .store (.var 0) 1 (.postInc 0), false),
  ("int f(int, int); int subject(int x) { return f(x++, x); }", .call2 (.postInc 0) (.var 0), false),
  ("int subject(int x) { x += x++; return x; }", .compound (· + ·) 0 (.postInc 0), false),
  ("int f(int); int g(int); int subject(int a) { return f(a) + g(a); }",
    .bin (· + ·) (.call2 (.var 0) (.lit 0)) (.call2 (.var 0) (.lit 0)), false),
  ("int subject(int i) { int j = 0; j = i++; return j; }", .assign 1 (.postInc 0), true),
  ("int subject(int a) { int x; int y; x = (y = a); return x; }", .assign 0 (.assign 1 (.var 2)), true),
  ("int subject(int i) { int j = 0; j = (i++, i); return j; }", .assign 1 (.comma (.postInc 0) (.var 0)), true),
  ("int subject(int x, int a) { x *= a; return x; }", .compound (· * ·) 0 (.var 1), true),
  ("int f(int); int subject(int x) { return f(x++); }", .call2 (.postInc 0) (.lit 0), true)
]

theorem order_vectors_hold :
    orderVectors.all (fun v => admit v.2.1 == v.2.2) = true := by decide

/-- The three unsequenced refusals the card must pin. -/
theorem unsequenced_refusals :
    admit (.assign 0 (.postInc 0)) = false ∧
      admit (.store (.var 0) 1 (.postInc 0)) = false ∧
      admit (.call2 (.postInc 0) (.var 0)) = false := by decide

/-- An admitted unsequenced pair: any concrete events the two operands perform
(abstracting to the elaborated ones) reach one store in every permitted order. -/
theorem bin_admitted_order_independent (op : Int → Int → Int) (a b : Expr)
    (h : admit (.bin op a b) = true) (ca cb m : List CEv)
    (ha : ca.map erase = (elaborate a).1.all) (hb : cb.map erase = (elaborate b).1.all)
    (hi : Interleave ca cb m) (s : Store) : run m s = run (ca ++ cb) s := by
  have hcross : cross (elaborate a).1.all (elaborate b).1.all = true := by
    simp only [admit, elaborate, Bool.and_eq_true] at h
    exact h.2
  apply interleave_run ca cb m _ hi s
  intro x hx y hy
  have hx' : erase x ∈ (elaborate a).1.all := ha ▸ List.mem_map_of_mem hx
  have hy' : erase y ∈ (elaborate b).1.all := hb ▸ List.mem_map_of_mem hy
  simp only [cross, List.all_eq_true] at hcross
  exact hcross _ hx' _ hy'

/-! ## Value semantics: one permitted serialisation -/

def eval : Expr → Store → Int × Store
  | .lit n, s => (n, s)
  | .var p, s => (s p, s)
  | .assign p e, s =>
    let (v, s₁) := eval e s
    (v, update s₁ p v)
  | .store addr p e, s =>
    let (_, s₁) := eval addr s
    let (v, s₂) := eval e s₁
    (v, update s₂ p v)
  | .compound op p e, s =>
    let r := s p
    let (v, s₁) := eval e s
    (op r v, update s₁ p (op r v))
  | .preInc p, s => (s p + 1, update s p (s p + 1))
  | .postInc p, s => (s p, update s p (s p + 1))
  | .comma a b, s => eval b (eval a s).2
  | .bin op a b, s =>
    let (va, s₁) := eval a s
    let (vb, s₂) := eval b s₁
    (op va vb, s₂)
  | .call2 a b, s => (0, (eval b (eval a s).2).2)

/-- Comma sequencing: the left operand completes, then the right operand runs
and gives the value. -/
theorem comma_sequencing (a b : Expr) (s : Store) :
    eval (.comma a b) s = eval b (eval a s).2 := rfl

/-- Every event of the left operand precedes the right operand's, and only the
right operand's side effects remain unsequenced with a consumer. -/
theorem comma_events (a b : Expr) :
    (elaborate (.comma a b)).1.all = (elaborate a).1.all ++ (elaborate b).1.all ∧
      (elaborate (.comma a b)).1.sides = (elaborate b).1.sides := ⟨rfl, rfl⟩

theorem post_value_old (p : Place) (s : Store) : (eval (.postInc p) s).1 = s p := rfl

theorem pre_value_new (p : Place) (s : Store) : (eval (.preInc p) s).1 = s p + 1 := rfl

/-- Pre and post increment store the same value and yield different ones. -/
theorem pre_post_separated (p : Place) (s : Store) :
    (eval (.preInc p) s).2 = (eval (.postInc p) s).2 ∧
      (eval (.preInc p) s).1 ≠ (eval (.postInc p) s).1 := by
  refine ⟨rfl, ?_⟩
  show s p + 1 ≠ s p
  omega

/-- `p++` is not `p = p + 1` as an expression: the values differ. -/
theorem post_increment_not_assignment (p : Place) (s : Store) :
    (eval (.postInc p) s).1 ≠ (eval (.assign p (.bin (· + ·) (.var p) (.lit 1))) s).1 := by
  show s p ≠ s p + 1
  omega

/-- The compound assignment evaluates its place once: one read before the right
operand's events, one write after them. -/
theorem compound_single_evaluation (op : Int → Int → Int) (p : Place) (e : Expr) :
    (elaborate (.compound op p e)).1.all = .read p :: (elaborate e).1.all ++ [.write p] := rfl

/-- With a pure place, `p op= e` computes what `p = p op e` computes. -/
theorem compound_desugars (op : Int → Int → Int) (p : Place) (e : Expr) (s : Store) :
    eval (.compound op p e) s = eval (.assign p (.bin op (.var p) e)) s := rfl

/-- Assignment as a value: the value is the stored one. -/
theorem assignment_value (p : Place) (e : Expr) (s : Store) :
    (eval (.assign p e) s).2 p = (eval (.assign p e) s).1 := by
  simp [eval, update]

end QCore.ElaborationCore
