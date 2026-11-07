/-!
# A chained assignment is its sequential assignments (card `CHAINED-ASSIGNMENT-INNER-DESTINATION`)

`crates/qcore-c-gateway/src/legacy_classify.rs`'s statement classifier models
one write per assignment statement: the place named on the left of the
statement's first `=`. It read `d = safe = rd_malloc(n)` as `d = rd_malloc(n)`
and dropped the write of `safe` without modelling or refusing it, so
`rd_kafka_sasl_safe_string`'s `return safe;` escaped nothing and a leak was
accused (`rdkafka_sasl_scram.c:243`, `:249`); a release through the inner name
was left unmatched the same way.

The repair (gateway rule 5i, amended; `legacy_chained_assignment.rs`) is a
guard on the classifier's answer: an admission of a statement holding a write
inside an assigned value is replaced by a named refusal, and a refusal stays
itself. Rule 10s's elaborator, asked only after that refusal, lowers a chain
as the inner assignment and then the outer assignment of the inner name, each
decided by the older statement lowering as a single assignment.

This module proves, over a store model of named objects:

* **C11 6.5.16p3** — `a = b = e` stores `e`'s value, converted to `b`'s type,
  in `b`, then the value of that assignment expression (the value `b` now
  holds), converted to `a`'s type, in `a`. That is exactly the two statements
  `b = e; a = b;` (`chain_eq_sequential`), for any number of links
  (`chainN_eq_sequential`).
* **Identity** — with identity-preserving conversions (a pointer converted to
  another object pointer is the same pointer, rule 10m), both names hold the
  same identity afterwards (`both_names_hold_the_identity`), the reading the
  `Alias` instruction gives `a = b`.
* **The defect** — the old one-destination reading differs from C at the inner
  name whenever the inner name held anything else (`the_old_reading_drops_the_inner_write`).
* **The guard** — it never admits what the classifier refused, keeps every
  refusal byte for byte, refuses every admitted inner write, and changes no
  statement without one (`the_guard_never_admits`, `the_guard_keeps_refusals`,
  `the_guard_refuses_every_inner_write`, `the_guard_is_identity_elsewhere`);
  with a refused statement never certifying, the guard never promotes
  (`the_guard_never_promotes`).
* **Non-vacuity** — the scram shape: the old reading leaves `safe` unbound
  while C binds it to the allocation (`the_scram_shape_is_dropped_by_the_old_reading`).

## Premises, not theorems

That the Rust guard's token reader (`inner_write`) finds every assignment
operator inside an assigned value is a premise about the importer's lexer (one
token per punctuation character; `==`, `!=`, `<=`, `>=` are not assignments,
`<<=` and `>>=` are), checked by the module's unit tests. Faithful Clang
extraction and node matching for the elaborator's lowering are trust boundary
28, as for rule 10s. The model's objects are distinct names; the elaborator
admits a chain only when Clang shows the outer and inner names are distinct
declarations (an unsequenced pair of writes to one object is undefined
behaviour and rule 10s refuses it).
-/

namespace QCore.ChainedAssignment

/-- An object's name. -/
abbrev Name := Nat

/-- A value: a scalar, an object identity, or nothing yet. -/
inductive Val where
  | scalar (n : Nat)
  | ident (i : Nat)
  | unset
deriving DecidableEq, Repr

/-- The store: what each named object holds. -/
abbrev Store := Name → Val

/-- Store `v` in `x`. -/
def write (σ : Store) (x : Name) (v : Val) : Store :=
  fun y => if y = x then v else σ y

/-- Expressions: a literal, a read of a name, and an assignment used as a value. -/
inductive Expr where
  | lit (v : Val)
  | var (x : Name)
  | assign (x : Name) (e : Expr)

/-- The conversion C applies to a value stored in each name (its declared type). -/
abbrev Conv := Name → Val → Val

/-- Evaluation: the value and the store after the side effects.
    C11 6.5.16p3: the value of an assignment expression is the value of the
    left operand after the assignment, which is the right operand's value
    converted to the left operand's type. -/
def eval (conv : Conv) : Store → Expr → Val × Store
  | σ, .lit v => (v, σ)
  | σ, .var x => (σ x, σ)
  | σ, .assign x e =>
    let r := eval conv σ e
    let stored := conv x r.1
    (stored, write r.2 x stored)

/-- An expression statement `x = e;`. -/
def exec (conv : Conv) (σ : Store) (x : Name) (e : Expr) : Store :=
  let r := eval conv σ e
  write r.2 x (conv x r.1)

theorem write_same (σ : Store) (x : Name) (v : Val) : write σ x v x = v := by
  simp [write]

/-- **C11 6.5.16p3.** The chain `a = b = e` is the two statements `b = e; a = b;`. -/
theorem chain_eq_sequential (conv : Conv) (σ : Store) (a b : Name) (e : Expr) :
    exec conv σ a (.assign b e) = exec conv (exec conv σ b e) a (.var b) := by
  simp [exec, eval, write_same]

/-- A chain of destinations `d₁ = d₂ = … = dₙ = e`. -/
def chainExpr : List Name → Expr → Expr
  | [], e => e
  | d :: ds, e => .assign d (chainExpr ds e)

/-- The sequential reading of a chain: the innermost assignment first, then
    each outer destination assigned from the name inside it. -/
def sequential (conv : Conv) : Store → List Name → Name → Expr → Store
  | σ, [], x, e => exec conv σ x e
  | σ, d :: ds, x, e => exec conv (sequential conv σ ds d e) x (.var d)

/-- **Every chain is its sequential assignments.** -/
theorem chainN_eq_sequential (conv : Conv) (ds : List Name) :
    ∀ (σ : Store) (x : Name) (e : Expr),
      exec conv σ x (chainExpr ds e) = sequential conv σ ds x e := by
  induction ds with
  | nil => intro σ x e; rfl
  | cons d ds ih =>
    intro σ x e
    simp only [chainExpr, sequential]
    rw [← ih σ d e]
    simp [exec, eval, write_same]

/-- A conversion that preserves identity (rule 10m: an object pointer converted
    to another object pointer is the same pointer). -/
def PreservesIdentity (conv : Conv) : Prop := ∀ x i, conv x (.ident i) = .ident i

/-- **Identity.** After `a = b = e` with `e` an identity, both names hold it:
    the `Alias` reading of the outer assignment. -/
theorem both_names_hold_the_identity (conv : Conv) (h : PreservesIdentity conv)
    (σ : Store) (a b : Name) (i : Nat) (hab : a ≠ b) :
    exec conv σ a (.assign b (.lit (.ident i))) a = .ident i ∧
      exec conv σ a (.assign b (.lit (.ident i))) b = .ident i := by
  constructor
  · simp [exec, eval, write, h b i, h a i]
  · simp [exec, eval, write, h b i, Ne.symm hab]

/-- The old one-destination reading: `a = b = e` read as `a = e`. -/
def oldReading (conv : Conv) (σ : Store) (a _b : Name) (e : Expr) : Store :=
  exec conv σ a e

/-- **The defect.** Whenever the inner name held something other than the
    value C stores in it, the old reading differs from C at the inner name. -/
theorem the_old_reading_drops_the_inner_write (conv : Conv) (σ : Store)
    (a b : Name) (v : Val) (hab : a ≠ b) (hdiff : σ b ≠ conv b v) :
    oldReading conv σ a b (.lit v) b ≠ exec conv σ a (.assign b (.lit v)) b := by
  simp [oldReading, exec, eval, write, Ne.symm hab]
  exact hdiff

/-! ## The guard -/

/-- The classifier's answer, abstracted: an admission carrying its lowering, or
    a refusal carrying its description. -/
inductive Answer (L D : Type) where
  | admitted (lowering : L)
  | refused (description : D)
deriving DecidableEq

/-- `legacy_chained_assignment.rs`'s guard: `inner` says the statement holds a
    write inside an assigned value; `reason` is the named refusal. -/
def guard {L D : Type} (inner : Bool) (reason : D) : Answer L D → Answer L D
  | .refused d => .refused d
  | .admitted l => if inner then .refused reason else .admitted l

theorem the_guard_never_admits {L D : Type} (inner : Bool) (reason : D)
    (a : Answer L D) (l : L) (h : guard inner reason a = .admitted l) :
    a = .admitted l ∧ inner = false := by
  cases a with
  | refused d => simp [guard] at h
  | admitted l' =>
    cases inner <;> simp_all [guard]

theorem the_guard_keeps_refusals {L D : Type} (inner : Bool) (reason d : D) :
    guard (L := L) inner reason (.refused d) = .refused d := rfl

theorem the_guard_refuses_every_inner_write {L D : Type} (reason : D) (l : L) :
    guard true reason (.admitted l) = .refused reason := rfl

theorem the_guard_is_identity_elsewhere {L D : Type} (reason : D) (a : Answer L D) :
    guard false reason a = a := by
  cases a <;> rfl

/-- The verdict a statement's answer can contribute: a refusal never certifies
    (`QCore.RefusedWrites.a_refusal_never_certifies`). -/
inductive Verdict where
  | safe
  | unknown
deriving DecidableEq

def verdictOf {L D : Type} (certifies : L → Bool) : Answer L D → Verdict
  | .refused _ => .unknown
  | .admitted l => if certifies l then .safe else .unknown

/-- **The guard never promotes.** A statement the guarded classifier lets
    certify was admitted, unchanged, before. -/
theorem the_guard_never_promotes {L D : Type} (certifies : L → Bool) (inner : Bool)
    (reason : D) (a : Answer L D)
    (h : verdictOf certifies (guard inner reason a) = .safe) :
    verdictOf certifies a = .safe := by
  cases a with
  | refused d => simp [guard, verdictOf] at h
  | admitted l =>
    cases inner <;> simp_all [guard, verdictOf]

/-! ## Non-vacuity: the scram shape -/

/-- `d` and `safe`, with nothing held yet. -/
def d : Name := 0
def safe : Name := 1
def empty : Store := fun _ => .unset

/-- Pointer conversions preserve identity; scalars are unchanged here. -/
def idConv : Conv := fun _ v => v

/-- `d = safe = rd_malloc(len + 1)`, the allocation's identity being `7`. -/
def scram : Expr := .assign safe (.lit (.ident 7))

theorem the_scram_shape_is_dropped_by_the_old_reading :
    oldReading idConv empty d safe (.lit (.ident 7)) safe = .unset ∧
      exec idConv empty d scram safe = .ident 7 ∧
      exec idConv empty d scram d = .ident 7 := by
  decide

/-- The sequential reading binds `safe` too, so `return safe;` escapes the
    allocation. -/
theorem the_scram_shape_is_its_sequential_assignments :
    exec idConv empty d scram =
      exec idConv (exec idConv empty safe (.lit (.ident 7))) d (.var safe) :=
  chain_eq_sequential idConv empty d safe _

/-- A scalar chain is its sequential assignments too (it was already refused
    by the older routes and lowered by rule 10s; nothing moves). -/
theorem a_scalar_chain_is_sequential :
    exec idConv empty d (.assign safe (.lit (.scalar 3))) d = .scalar 3 ∧
      exec idConv empty d (.assign safe (.lit (.scalar 3))) safe = .scalar 3 := by
  decide

end QCore.ChainedAssignment
