namespace QCore.ChunkSource

/-! Executable finite-state semantics for the bounded Boolean-field source
importer. Clang typing and AST-to-IR translation remain trusted boundaries;
these theorems do not prove unrestricted C or a full ChunkIO teardown. -/

inductive Expr where
  | literal (value : Int)
  | field
  | not (value : Expr)
  | negate (value : Expr)
  | equal (left right : Expr)
  | notEqual (left right : Expr)
  deriving Repr, DecidableEq

def boolInt (value : Bool) : Int := if value then 1 else 0

def Expr.eval (field : Bool) : Expr → Int
  | .literal value => value
  | .field => boolInt field
  | .not value => boolInt (value.eval field == 0)
  | .negate value => -(value.eval field)
  | .equal left right => boolInt (left.eval field == right.eval field)
  | .notEqual left right => boolInt (left.eval field != right.eval field)

inductive Stmt where
  | empty
  | sequence (first rest : Stmt)
  | branch (condition : Expr) (yes no : Stmt)
  | assign (value : Expr)
  | ret (value : Expr)
  deriving Repr, DecidableEq

structure State where
  value : Bool
  writes : List Bool := []
  branches : List Bool := []
  deriving Repr, DecidableEq

def Stmt.eval : Stmt → State → Option (State × Option Int)
  | .empty, state => some (state, none)
  | .sequence first rest, state => do
      let (next, result) ← first.eval state
      match result with
      | some value => some (next, some value)
      | none => rest.eval next
  | .branch condition yes no, state =>
      let taken := condition.eval state.value != 0
      let next := { state with branches := state.branches ++ [taken] }
      if taken then yes.eval next else no.eval next
  | .assign expression, state =>
      let value := expression.eval state.value
      if value = 0 ∨ value = 1 then
        let next := value == 1
        let writes := state.writes ++ [next]
        some ({ state with value := next, writes := writes }, none)
      else none
  | .ret value, state => some (state, some (value.eval state.value))

structure Case where
  state : State
  result : Int
  deriving Repr, DecidableEq

def run (statement : Stmt) (initial : Bool) : Option Case := do
  let (state, result) ← statement.eval { value := initial }
  let value ← result
  pure { state, result := value }

structure Summary where
  whenFalse : Case
  whenTrue : Case
  deriving Repr, DecidableEq

def summarize (statement : Stmt) : Option Summary := do
  let whenFalse ← run statement false
  let whenTrue ← run statement true
  pure { whenFalse, whenTrue }

def Summary.select (summary : Summary) (initial : Bool) : Case :=
  if initial then summary.whenTrue else summary.whenFalse

theorem accepted_summary_covers_every_boolean_prestate
    {statement : Stmt} {summary : Summary}
    (accepted : summarize statement = some summary) (initial : Bool) :
    run statement initial = some (summary.select initial) := by
  unfold summarize at accepted
  cases hf : run statement false with
  | none => simp [hf] at accepted
  | some f =>
    cases ht : run statement true with
    | none => simp [hf, ht] at accepted
    | some t =>
      simp [hf, ht] at accepted
      subst summary
      cases initial <;> simp [Summary.select, hf, ht]

def checkContract (summary : Summary) (post : Bool → Case → Bool) : Bool :=
  post false summary.whenFalse && post true summary.whenTrue

theorem accepted_contract_holds_on_every_source_path
    {statement : Stmt} {summary : Summary} {post : Bool → Case → Bool}
    (imported : summarize statement = some summary)
    (checked : checkContract summary post = true) (initial : Bool) :
    run statement initial = some (summary.select initial) ∧
      post initial (summary.select initial) = true := by
  refine ⟨accepted_summary_covers_every_boolean_prestate imported initial, ?_⟩
  have both : post false summary.whenFalse = true ∧ post true summary.whenTrue = true := by
    simpa [checkContract] using checked
  cases initial
  · exact both.1
  · exact both.2

abbrev ChunkFields := Nat → Bool × Bool

def bindBusy (state : ChunkFields) (identity : Nat) (after : Bool) : ChunkFields :=
  fun other => if other = identity then (after, (state other).2) else state other

theorem busy_effect_preserves_chunkio_lock
    (state : ChunkFields) (identity : Nat) (after : Bool) :
    (bindBusy state identity after identity).2 = (state identity).2 := by
  simp [bindBusy]

theorem bound_effect_preserves_other_identities
    (state : ChunkFields) (identity other : Nat) (after : Bool)
    (different : other ≠ identity) :
    bindBusy state identity after other = state other := by
  simp [bindBusy, different]

def releaseBusy : Stmt :=
  .sequence (.branch (.equal .field (.literal 0)) (.ret (.literal (-1))) .empty)
    (.sequence (.assign (.literal 0)) (.ret (.literal 0)))

theorem release_busy_error_path_preserves_state :
    run releaseBusy false = some {
      state := { value := false, writes := [], branches := [true] }, result := -1 } := by
  decide

theorem release_busy_success_path_clears_busy :
    run releaseBusy true = some {
      state := { value := false, writes := [false], branches := [false] }, result := 0 } := by
  decide

end QCore.ChunkSource
