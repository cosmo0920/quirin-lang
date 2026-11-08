import QCore.VerdictSemigroup

/-!
# Clang-typed scalar statement forms (card SCALAR-STATEMENT-FORMS, gateway rule 41)

The abstract rule implemented by
`crates/qcore-c-gateway/src/scalar_statement_forms.rs` and its hooks in
`legacy_scalar_statement_forms.rs`, asked after every older route of a
statement or `for` clause refused it with one of five lexical refusals.

## What is proved

* **The decision.** `decide` admits a step (`X++`, `--X`), a compound
  assignment (`X op= E`), a plain assignment, a declaration (one or several
  declarators) of a scalar object or scalar path with a scalar value, and a
  type-only declaration; it refuses by name a pointer destination, a pointer
  value (pointer arithmetic included), a `volatile` object, an aggregate
  destination, an object of an inline record type, a variably modified type
  or evaluated `sizeof`, a static object, a shadowed name, a protected formal
  and an unread destination or value (`pointer_destination_refuses`,
  `pointer_value_refuses`, `evaluated_query_refuses`, ...). The Rust table
  `VECTORS` is `vectors` (`decision_vectors_hold`).
* **The forms are stores.** C11 6.5.16.2p3 and 6.5.3.1p2: a compound
  assignment is the plain store of `X op E` with `X` read once, and a prefix
  step is `X += 1` (`compound_is_assignment`, `step_is_compound`); a postfix
  step has the same side effect (`postfix_step_side_effect`).
* **The lowering is sound for the concrete store.** The concrete post-state
  of an admitted store to a named scalar is a run of `forget X`
  (`named_store_refines_forget`), and of a scalar path store a run of the
  path write with its base's obligation (`path_store_refines_write`).
  Several declarators are their declarators in order
  (`declarators_are_sequential`); a type declaration runs nothing
  (`type_declaration_is_noop`).
* **Frame preservation.** Every instruction the rule lowers to, and every
  sequence of them, leaves the pointer bindings and the owed custody set
  unchanged (`run_preserves_frame`, `runs_preserve_frame`): no identity is
  acquired, released, transferred or rebound, so an owed object stays owed
  (`an_owned_object_stays_owed`).
* **Obligations.** A path write needs its base live; a released base has no
  run, which the flow reports (`a_released_base_has_no_write`).
* **Never `Safe` by declining.** A declined statement keeps its `Unknown`
  leaf (`a_declined_statement_is_never_safe`).

## What this is not

A proof that the Rust reads C faithfully: matching a statement to its Clang
node, Clang's types and the typed reader's place reading are trust boundary
28's premises, pinned by `crates/qcore-c-gateway/tests/scalar_statement_forms.rs`
and the shared-vector test in `scalar_statement_forms.rs`.
-/

namespace QCore.ScalarStatementForms

open QCore.VerdictSemigroup (V fold)

/-! ## The decision -/

inductive Form where
  | step | compound | assignment | declaration | declarators | typeDeclaration
  deriving DecidableEq, Repr

inductive Destination where
  | scalarObject | scalarPath | noObject | pointer | volatile | aggregate | shadowed
  | static | protectedFormal | variablyModified | inlineRecordObject | unread
  deriving DecidableEq, Repr

inductive ValueClass where
  | none | scalar | pointer | evaluatedQuery | unread
  deriving DecidableEq, Repr

inductive Refusal where
  | pointerDestination | pointerValue | volatile | aggregate | inlineRecordObject
  | variablyModified | staticStorage | shadowedName | protectedFormal
  | unreadDestination | unreadValue | mismatched
  deriving DecidableEq, Repr

/-- Rust `scalar_statement_forms::decide`: `none` admits. -/
def decide (form : Form) (destination : Destination) (value : ValueClass) : Option Refusal :=
  match destination with
  | .noObject =>
      match form, value with
      | .typeDeclaration, .none => none
      | _, _ => some .mismatched
  | .pointer => some .pointerDestination
  | .volatile => some .volatile
  | .aggregate => some .aggregate
  | .inlineRecordObject => some .inlineRecordObject
  | .variablyModified => some .variablyModified
  | .static => some .staticStorage
  | .shadowed => some .shadowedName
  | .protectedFormal => some .protectedFormal
  | .unread => some .unreadDestination
  | .scalarObject | .scalarPath =>
      if form = .typeDeclaration then some .mismatched
      else if destination = .scalarPath ∧ (form = .declaration ∨ form = .declarators) then
        some .mismatched
      else
        match value with
        | .none | .scalar => none
        | .pointer => some .pointerValue
        | .evaluatedQuery => some .variablyModified
        | .unread => some .unreadValue

abbrev Vector := Form × Destination × ValueClass × Option Refusal

/-- The Rust table `VECTORS`, row for row. -/
def vectors : List Vector := [
  (.step, .scalarObject, .none, none),
  (.step, .scalarPath, .none, none),
  (.step, .pointer, .none, some .pointerDestination),
  (.step, .volatile, .none, some .volatile),
  (.step, .shadowed, .none, some .shadowedName),
  (.step, .static, .none, some .staticStorage),
  (.step, .protectedFormal, .none, some .protectedFormal),
  (.compound, .scalarObject, .scalar, none),
  (.compound, .scalarPath, .scalar, none),
  (.compound, .pointer, .scalar, some .pointerDestination),
  (.compound, .scalarObject, .pointer, some .pointerValue),
  (.compound, .scalarObject, .unread, some .unreadValue),
  (.assignment, .scalarObject, .scalar, none),
  (.assignment, .scalarObject, .pointer, some .pointerValue),
  (.assignment, .scalarObject, .evaluatedQuery, some .variablyModified),
  (.declaration, .scalarObject, .scalar, none),
  (.declaration, .scalarObject, .none, none),
  (.declaration, .aggregate, .scalar, some .aggregate),
  (.declaration, .scalarPath, .scalar, some .mismatched),
  (.declaration, .volatile, .scalar, some .volatile),
  (.declarators, .scalarObject, .scalar, none),
  (.declarators, .pointer, .none, some .pointerDestination),
  (.declarators, .inlineRecordObject, .none, some .inlineRecordObject),
  (.typeDeclaration, .noObject, .none, none),
  (.typeDeclaration, .variablyModified, .none, some .variablyModified),
  (.typeDeclaration, .inlineRecordObject, .none, some .inlineRecordObject),
  (.typeDeclaration, .scalarObject, .none, some .mismatched),
  (.typeDeclaration, .shadowed, .none, some .shadowedName),
  (.step, .noObject, .none, some .mismatched),
  (.step, .unread, .none, some .unreadDestination)
]

theorem decision_vectors_hold :
    vectors.all (fun v => decide v.1 v.2.1 v.2.2.1 == v.2.2.2) = true := by
  decide

/-- Admission needs a scalar destination (or no object, for a type
declaration). -/
theorem admitted_destination (form : Form) (d : Destination) (v : ValueClass)
    (h : decide form d v = none) :
    d = .scalarObject ∨ d = .scalarPath ∨ (d = .noObject ∧ form = .typeDeclaration) := by
  cases d <;> cases form <;> cases v <;> simp_all [decide]

theorem pointer_destination_refuses (form : Form) (v : ValueClass) :
    decide form .pointer v = some .pointerDestination := rfl

theorem volatile_refuses (form : Form) (v : ValueClass) :
    decide form .volatile v = some .volatile := rfl

theorem inline_record_object_refuses (form : Form) (v : ValueClass) :
    decide form .inlineRecordObject v = some .inlineRecordObject := rfl

theorem variably_modified_type_refuses (form : Form) (v : ValueClass) :
    decide form .variablyModified v = some .variablyModified := rfl

/-- Pointer arithmetic or any pointer-valued operand refuses every non-type
form. -/
theorem pointer_value_refuses (form : Form) (d : Destination)
    (hd : d = .scalarObject ∨ d = .scalarPath) (hf : form ≠ .typeDeclaration)
    (hdecl : ¬ (d = .scalarPath ∧ (form = .declaration ∨ form = .declarators))) :
    decide form d .pointer = some .pointerValue := by
  rcases hd with rfl | rfl <;> simp_all [decide]

/-- A `sizeof` Clang evaluates (a variable length array) refuses. -/
theorem evaluated_query_refuses (form : Form) (d : Destination)
    (hd : d = .scalarObject ∨ d = .scalarPath) (hf : form ≠ .typeDeclaration)
    (hdecl : ¬ (d = .scalarPath ∧ (form = .declaration ∨ form = .declarators))) :
    decide form d .evaluatedQuery = some .variablyModified := by
  rcases hd with rfl | rfl <;> simp_all [decide]

/-! ## The state and the lowering -/

def upd {α : Type} (f : Nat → α) (x : Nat) (v : α) : Nat → α :=
  fun y => if y = x then v else f y

/-- What the flow reads: named scalar objects, the pointer names' bindings
to identities, the owed custody set, and each identity's object (its scalar
members), `none` once released. -/
structure State where
  scalars : Nat → Int
  bindings : Nat → Option Nat
  owed : List Nat
  heap : Nat → Option (Nat → Int)

/-- The rule's lowerings: `ForgetScalar X`, a path write through a base (the
typed reader's `IndirectScalarAccess` with its write), a read-only access,
and `NoOp`. -/
inductive Instr where
  | forget (x : Nat)
  | write (base field : Nat)
  | read (bases : List Nat)
  | noop
  deriving DecidableEq, Repr

/-- The flow's reading of one instruction: a forget havocs the scalar; a
write needs its base live and stores some value into the member; a read
needs every base live. -/
inductive Run : Instr → State → State → Prop where
  | forget (x : Nat) (σ : State) (v : Int) :
      Run (.forget x) σ { σ with scalars := upd σ.scalars x v }
  | write (b f : Nat) (σ : State) (obj : Nat → Int) (v : Int) (live : σ.heap b = some obj) :
      Run (.write b f) σ { σ with heap := upd σ.heap b (some (upd obj f v)) }
  | read (bases : List Nat) (σ : State) (live : ∀ b ∈ bases, (σ.heap b).isSome) :
      Run (.read bases) σ σ
  | noop (σ : State) : Run .noop σ σ

/-- A sequence of instructions, in order. -/
inductive Runs : List Instr → State → State → Prop where
  | nil (σ : State) : Runs [] σ σ
  | cons {i : Instr} {is : List Instr} {σ τ ρ : State} :
      Run i σ τ → Runs is τ ρ → Runs (i :: is) σ ρ

/-- Frame preservation: no instruction of the rule rebinds a pointer name or
changes the owed custody set. -/
theorem run_preserves_frame {i : Instr} {σ τ : State} (h : Run i σ τ) :
    τ.bindings = σ.bindings ∧ τ.owed = σ.owed := by
  cases h <;> exact ⟨rfl, rfl⟩

theorem runs_preserve_frame {is : List Instr} {σ τ : State} (h : Runs is σ τ) :
    τ.bindings = σ.bindings ∧ τ.owed = σ.owed := by
  induction h with
  | nil => exact ⟨rfl, rfl⟩
  | cons hi _ ih =>
      obtain ⟨b1, o1⟩ := run_preserves_frame hi
      exact ⟨ih.1.trans b1, ih.2.trans o1⟩

/-- An object owed before an admitted statement is owed after it: a leak
the flow reports stays reported. -/
theorem an_owned_object_stays_owed {is : List Instr} {σ τ : State} (h : Runs is σ τ)
    (site : Nat) (owed : site ∈ σ.owed) : site ∈ τ.owed := by
  rw [(runs_preserve_frame h).2]
  exact owed

/-- A released base has no write: the flow's dereference check fails there
and reports the statement instead of admitting it. -/
theorem a_released_base_has_no_write {b f : Nat} {σ τ : State} (released : σ.heap b = none) :
    ¬ Run (.write b f) σ τ := by
  intro h
  cases h with
  | write _ _ _ obj _ live => rw [released] at live; cases live

/-! ## The forms are stores -/

/-- A scalar value expression: literals, named scalars, members through a
base, and an arithmetic operator. -/
inductive Expr where
  | lit (n : Int)
  | var (x : Nat)
  | member (b f : Nat)
  | add (l r : Expr)
  deriving DecidableEq, Repr

def eval (σ : State) : Expr → Option Int
  | .lit n => some n
  | .var x => some (σ.scalars x)
  | .member b f => (σ.heap b).map (fun obj => obj f)
  | .add l r => do
      let a ← eval σ l
      let c ← eval σ r
      pure (a + c)

/-- The concrete plain store `X = E` to a named scalar. -/
def assign (σ : State) (x : Nat) (e : Expr) : Option State :=
  (eval σ e).map (fun v => { σ with scalars := upd σ.scalars x v })

/-- The concrete compound assignment `X += E` (C11 6.5.16.2p3: `X` read
once). -/
def compound (σ : State) (x : Nat) (e : Expr) : Option State :=
  (eval σ e).map (fun v => { σ with scalars := upd σ.scalars x (σ.scalars x + v) })

/-- The concrete prefix step `++X` (C11 6.5.3.1p2). -/
def step (σ : State) (x : Nat) : State :=
  { σ with scalars := upd σ.scalars x (σ.scalars x + 1) }

theorem compound_is_assignment (σ : State) (x : Nat) (e : Expr) :
    compound σ x e = assign σ x (.add (.var x) e) := by
  unfold compound assign
  cases h : eval σ e <;> simp [eval, h]

theorem step_is_compound (σ : State) (x : Nat) :
    some (step σ x) = compound σ x (.lit 1) := rfl

/-- The postfix step yields the old value and has the prefix step's side
effect (C11 6.5.2.4p2). -/
def postfixStep (σ : State) (x : Nat) : Int × State := (σ.scalars x, step σ x)

theorem postfix_step_side_effect (σ : State) (x : Nat) :
    (postfixStep σ x).2 = step σ x ∧ (postfixStep σ x).1 = σ.scalars x := ⟨rfl, rfl⟩

/-- The concrete post-state of an admitted store to a named scalar is a run
of its lowering `forget X`. -/
theorem named_store_refines_forget {σ τ : State} {x : Nat} {e : Expr}
    (h : assign σ x e = some τ) : Run (.forget x) σ τ := by
  unfold assign at h
  cases he : eval σ e with
  | none => simp [he] at h
  | some v =>
      simp [he] at h
      subst h
      exact Run.forget x σ v

theorem compound_refines_forget {σ τ : State} {x : Nat} {e : Expr}
    (h : compound σ x e = some τ) : Run (.forget x) σ τ := by
  rw [compound_is_assignment] at h
  exact named_store_refines_forget h

theorem step_refines_forget (σ : State) (x : Nat) : Run (.forget x) σ (step σ x) :=
  compound_refines_forget (step_is_compound σ x).symm

/-- The concrete store `B->F = V` through a live base is a run of the path
write. -/
def pathStore (σ : State) (b f : Nat) (v : Int) : Option State :=
  (σ.heap b).map (fun obj => { σ with heap := upd σ.heap b (some (upd obj f v)) })

theorem path_store_refines_write {σ τ : State} {b f : Nat} {v : Int}
    (h : pathStore σ b f v = some τ) : Run (.write b f) σ τ := by
  unfold pathStore at h
  cases hb : σ.heap b with
  | none => simp [hb] at h
  | some obj =>
      simp [hb] at h
      subst h
      exact Run.write b f σ obj v hb

/-- Several declarators: C11 6.7.6p3 puts a sequence point after each full
declarator, so the declaration is its declarators' stores in order. -/
def declarators (σ : State) : List (Nat × Expr) → Option State
  | [] => some σ
  | (x, e) :: rest => (assign σ x e).bind (fun τ => declarators τ rest)

theorem declarators_are_sequential {σ τ : State} :
    ∀ {ds : List (Nat × Expr)}, declarators σ ds = some τ →
      Runs (ds.map (fun d => Instr.forget d.1)) σ τ
  | [], h => by
      simp [declarators] at h
      subst h
      exact Runs.nil σ
  | (x, e) :: rest, h => by
      simp only [declarators] at h
      cases ha : assign σ x e with
      | none => simp [ha] at h
      | some ρ =>
          simp [ha] at h
          exact Runs.cons (named_store_refines_forget ha) (declarators_are_sequential h)

/-- A type declaration runs nothing (C11 6.7p5: no object, no storage). -/
theorem type_declaration_is_noop {σ τ : State} (h : Run .noop σ τ) : τ = σ := by
  cases h
  rfl

/-! ## Never `Safe` by declining -/

/-- A declined statement keeps its refusal, an `Unknown` leaf. -/
theorem a_declined_statement_is_never_safe (xs : List V) : fold (.unknown :: xs) ≠ .safe := by
  intro h
  have := (QCore.VerdictSemigroup.fold_eq_safe_iff (.unknown :: xs)).mp h .unknown (by simp)
  exact absurd this (by decide)

end QCore.ScalarStatementForms
