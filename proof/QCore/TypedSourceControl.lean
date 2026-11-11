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

/-! Route T's control statements (card CG09-TYPED-SOURCE-CONTROL, gateway
rule 11l): a model of `crates/qcore-c-gateway/src/typed_source_control.rs`
and of its hooks in `typed_source.rs`.

Bound mechanically: the Rust test
`typed_source::control_tests::cg09_lean_vectors_match_the_rust_tables` reads
`controlTable`, `legacyStatements`, `controlStatements`, `budgets` and
`switchVectors` from this file. It compares the first four with
`CONTROL_TABLE`, `LEGACY_STATEMENTS`, `CONTROL_STATEMENTS` and the importer's
four budget constants. It runs every `switchVectors` body through the importer
and compares the lowered paths with the vector's expected paths, which
`switch_vectors_hold` proves this model computes. The Clang-to-statement
reading itself stays a review premise.

Five properties are stated.
* Unlisted statements are refused, and a form is admitted only by a valid
  admission that lists it.
* A back-edge without an invariant or summary is refused, whatever the
  admission.
* A jump out of a scope ends every local of every scope it leaves, innermost
  first, and a later use of an ended local's storage is refused.
* `switch` lowering composes fallthrough: an entry runs the statements of
  every case after it up to the first `break`.
* Budget exhaustion is Unknown, and never Safe under any consumer. -/
namespace QCore.TypedSourceControl

/-! ## The control table and the admission -/

structure Form where
  spelling : String
  backEdge : Bool
  deriving DecidableEq, Repr

/-- Every control form CG09 reads (`CONTROL_TABLE`). -/
def controlTable : List Form := [
  ⟨"switch", false⟩,
  ⟨"break", false⟩,
  ⟨"continue", false⟩,
  ⟨"goto", false⟩,
  ⟨"goto backward", true⟩,
  ⟨"loop while", true⟩,
  ⟨"loop for", true⟩,
  ⟨"loop do", true⟩
]

/-- Clang statement kinds route T read before CG09 (`LEGACY_STATEMENTS`). -/
def legacyStatements : List String := [
  "CompoundStmt",
  "NullStmt",
  "ForStmt",
  "DoStmt",
  "DeclStmt",
  "BinaryOperator",
  "CompoundAssignOperator",
  "UnaryOperator",
  "IfStmt",
  "CallExpr",
  "ReturnStmt"
]

/-- The control form each statement kind is read as (`CONTROL_STATEMENTS`). -/
def controlStatements : List (String × String) := [
  ("SwitchStmt", "switch"),
  ("BreakStmt", "break"),
  ("ContinueStmt", "continue"),
  ("GotoStmt", "goto"),
  ("LabelStmt", "goto"),
  ("WhileStmt", "loop while"),
  ("ForStmt", "loop for"),
  ("DoStmt", "loop do")
]

def backEdge (spelling : String) : Bool :=
  controlTable.any (fun f => f.spelling == spelling && f.backEdge)

/-- A consumer's admission (`ControlAdmission`): the forms it decides. -/
abbrev Admission := List String

/-- A row is valid only for an acyclic form of the table: route T checks no
invariant or summary, so no back-edge form can be listed (card CG27). -/
def rowValid (spelling : String) : Bool :=
  controlTable.any (fun f => f.spelling == spelling && !f.backEdge)

def valid (a : Admission) : Bool := a.all rowValid && a.Nodup

/-- The importer admits a form only through a valid admission that lists it,
and never a back-edge form. -/
def admit (a : Admission) (spelling : String) : Bool :=
  valid a && !backEdge spelling && a.contains spelling

/-- `ControlAdmission::LEGACY`. -/
def legacy : Admission := []

theorem legacy_admits_nothing (spelling : String) : admit legacy spelling = false := by
  simp [admit, legacy]

theorem unlisted_form_refused (a : Admission) (spelling : String)
    (h : a.contains spelling = false) : admit a spelling = false := by
  unfold admit
  rw [h]
  simp

theorem invalid_admission_refuses (a : Admission) (spelling : String)
    (h : valid a = false) : admit a spelling = false := by
  simp [admit, h]

/-- A back-edge without an invariant or summary is refused, whatever the
admission lists. -/
theorem back_edge_refused (a : Admission) (spelling : String)
    (h : backEdge spelling = true) : admit a spelling = false := by
  simp [admit, h]

theorem cycles_are_back_edges :
    backEdge "loop while" = true ∧ backEdge "loop for" = true ∧
      backEdge "loop do" = true ∧ backEdge "goto backward" = true := by
  decide

theorem acyclic_forms : backEdge "switch" = false ∧ backEdge "break" = false ∧
    backEdge "continue" = false ∧ backEdge "goto" = false := by
  decide

/-- A valid row names an acyclic form. -/
theorem valid_row_acyclic (spelling : String) (h : rowValid spelling = true) :
    backEdge spelling = false := by
  simp [rowValid, controlTable] at h
  rcases h with rfl | rfl | rfl | rfl <;> decide

/-! ## Reading statements -/

inductive Reading where
  | legacy
  | control (form : String)
  | refused
  deriving DecidableEq, Repr

/-- How the importer reads a statement kind. `shaped` says a `for` is a
canonical scan and a `do` has a literal zero condition: only then are they
the legacy statements. -/
def readStatement (kind : String) (shaped : Bool) : Reading :=
  if legacyStatements.contains kind && (shaped || (kind != "ForStmt" && kind != "DoStmt")) then
    .legacy
  else
    match controlStatements.lookup kind with
    | some form => .control form
    | none => .refused

/-- A statement kind that is neither legacy nor a control form is refused. -/
theorem unlisted_statement_refused (kind : String) (shaped : Bool)
    (h₁ : legacyStatements.contains kind = false)
    (h₂ : controlStatements.lookup kind = none) : readStatement kind shaped = .refused := by
  unfold readStatement
  rw [h₁, h₂]
  simp

theorem while_is_a_loop : readStatement "WhileStmt" false = .control "loop while" := by decide
theorem unscanned_for_is_a_loop : readStatement "ForStmt" false = .control "loop for" := by decide
theorem scan_for_is_legacy : readStatement "ForStmt" true = .legacy := by decide
theorem nonzero_do_is_a_loop : readStatement "DoStmt" false = .control "loop do" := by decide
theorem switch_is_read : readStatement "SwitchStmt" false = .control "switch" := by decide
theorem indirect_goto_refused : readStatement "IndirectGotoStmt" false = .refused := by decide
theorem case_outside_switch_refused : readStatement "CaseStmt" false = .refused := by decide

/-- A loop statement is read, and then refused under every admission. -/
theorem loops_refused (a : Admission) :
    admit a "loop while" = false ∧ admit a "loop for" = false ∧ admit a "loop do" = false := by
  refine ⟨?_, ?_, ?_⟩ <;> exact back_edge_refused a _ (by decide)

/-! ## Lifetimes: a jump out of a scope ends its locals -/

inductive Op where
  | call (n : Nat)
  | use (local_ : Nat)
  | endLocal (local_ : Nat)
  deriving DecidableEq, Repr

/-- What a path emits as it leaves a stack of scopes, innermost first: each
block's locals in reverse declaration order (the importer's block exit, taken
by every path that leaves, by its end, by `return` or by a jump). -/
def exitScopes : List (List Nat) → List Op
  | [] => []
  | scope :: outer => scope.reverse.map .endLocal ++ exitScopes outer

/-- Every local of every scope a jump leaves is ended. -/
theorem jump_out_ends_locals (scopes : List (List Nat)) (l : Nat)
    (h : ∃ s ∈ scopes, l ∈ s) : Op.endLocal l ∈ exitScopes scopes := by
  induction scopes with
  | nil => simp at h
  | cons s rest ih =>
    rcases h with ⟨t, ht, hl⟩
    simp only [exitScopes, List.mem_append, List.mem_map, List.mem_reverse]
    rcases List.mem_cons.mp ht with rfl | ht
    · exact Or.inl ⟨l, hl, rfl⟩
    · exact Or.inr (ih ⟨t, ht, hl⟩)

/-- The innermost scope ends first. -/
theorem inner_scope_ends_first (inner : List Nat) (outer : List (List Nat)) :
    exitScopes (inner :: outer) = inner.reverse.map .endLocal ++ exitScopes outer := rfl

/-- The importer's lifetime check (`check_local_lifetimes`). -/
def lifetimesOk : List Nat → List Op → Bool
  | _, [] => true
  | ended, .endLocal l :: rest => lifetimesOk (l :: ended) rest
  | ended, .use l :: rest => !ended.contains l && lifetimesOk ended rest
  | ended, .call _ :: rest => lifetimesOk ended rest

/-- The locals ended after a prefix. -/
def endedAfter : List Nat → List Op → List Nat
  | ended, [] => ended
  | ended, .endLocal l :: rest => endedAfter (l :: ended) rest
  | ended, _ :: rest => endedAfter ended rest

theorem lifetimesOk_append (xs ys : List Op) : ∀ ended : List Nat,
    lifetimesOk ended (xs ++ ys) = (lifetimesOk ended xs && lifetimesOk (endedAfter ended xs) ys) := by
  induction xs with
  | nil => intro ended; simp [lifetimesOk, endedAfter]
  | cons op rest ih =>
    intro ended
    cases op with
    | call n => simp only [List.cons_append, lifetimesOk, endedAfter]; exact ih ended
    | endLocal m => simp only [List.cons_append, lifetimesOk, endedAfter]; exact ih (m :: ended)
    | use m =>
      simp only [List.cons_append, lifetimesOk, endedAfter]
      rw [ih ended, Bool.and_assoc]

theorem endedAfter_keeps (xs : List Op) : ∀ (ended : List Nat) (l : Nat),
    l ∈ ended → l ∈ endedAfter ended xs := by
  induction xs with
  | nil => intro ended l h; exact h
  | cons op rest ih =>
    intro ended l h
    cases op with
    | call n => exact ih ended l h
    | use m => exact ih ended l h
    | endLocal m => exact ih (m :: ended) l (List.mem_cons_of_mem m h)

theorem endedAfter_ends (xs : List Op) : ∀ (ended : List Nat) (l : Nat),
    Op.endLocal l ∈ xs → l ∈ endedAfter ended xs := by
  induction xs with
  | nil => intro ended l h; cases h
  | cons op rest ih =>
    intro ended l h
    rcases List.mem_cons.mp h with hhead | htail
    · cases hhead
      exact endedAfter_keeps rest (l :: ended) l List.mem_cons_self
    · cases op with
      | call n => exact ih ended l htail
      | use m => exact ih ended l htail
      | endLocal m => exact ih (m :: ended) l htail

theorem ended_use_refused (ops : List Op) : ∀ (ended : List Nat) (l : Nat),
    l ∈ ended → Op.use l ∈ ops → lifetimesOk ended ops = false := by
  induction ops with
  | nil => intro ended l _ hu; cases hu
  | cons op rest ih =>
    intro ended l hl hu
    rcases List.mem_cons.mp hu with h | h
    · cases h
      simp [lifetimesOk, hl]
    · cases op with
      | call n => exact ih ended l hl h
      | endLocal m => exact ih (m :: ended) l (List.mem_cons_of_mem m hl) h
      | use m => simp [lifetimesOk, ih ended l hl h]

/-- A path that jumps out of a scope holding `l` and later uses `l`'s storage
is refused. -/
theorem jump_then_use_refused (pre post : List Op) (scopes : List (List Nat)) (l : Nat)
    (hs : ∃ s ∈ scopes, l ∈ s) (hu : Op.use l ∈ post) :
    lifetimesOk [] (pre ++ exitScopes scopes ++ post) = false := by
  rw [lifetimesOk_append]
  have hend : l ∈ endedAfter [] (pre ++ exitScopes scopes) :=
    endedAfter_ends _ [] l (List.mem_append_right pre (jump_out_ends_locals scopes l hs))
  simp [ended_use_refused post _ l hend hu]

/-! ## `switch`: fallthrough composition -/

inductive Item where
  | case (constant : Nat)
  | dflt
  | stmt (id : Nat)
  | brk
  deriving DecidableEq, Repr

/-- The statements that run from an entry to the first top-level `break`;
later labels are passed through (fallthrough). -/
def runFrom : List Item → List Nat
  | [] => []
  | .stmt s :: rest => s :: runFrom rest
  | .brk :: _ => []
  | .case _ :: rest => runFrom rest
  | .dflt :: rest => runFrom rest

def constants : List Item → List Nat
  | [] => []
  | .case c :: rest => c :: constants rest
  | _ :: rest => constants rest

/-- A lowered path: its guards `(constant, taken)` on `selector == constant`,
and the statements it runs. -/
abbrev Lowered := List (Nat × Bool) × List Nat

def entries (cs : List Nat) : List Item → List Lowered
  | [] => []
  | .case c :: rest => ([(c, true)], runFrom rest) :: entries cs rest
  | .dflt :: rest => (cs.map (·, false), runFrom rest) :: entries cs rest
  | _ :: rest => entries cs rest

/-- The importer's lowering: one path per label, in source order; with no
`default`, one more path with every case guard not taken, running nothing. -/
def lowerSwitch (items : List Item) : List Lowered :=
  entries (constants items) items ++
    (if items.contains .dflt then [] else [((constants items).map (·, false), [])])

/-- Fallthrough composes: before the first `break`, running from an entry
runs the statements of each later case in order. -/
theorem fallthrough_composes (xs ys : List Item) (h : Item.brk ∉ xs) :
    runFrom (xs ++ ys) = runFrom xs ++ runFrom ys := by
  induction xs with
  | nil => rfl
  | cons x rest ih =>
    have hx : x ≠ .brk := fun e => h (e ▸ List.mem_cons_self)
    have hrest : Item.brk ∉ rest := fun e => h (List.mem_cons_of_mem x e)
    cases x with
    | brk => exact absurd rfl hx
    | stmt s => simp [runFrom, ih hrest]
    | case c => simp [runFrom, ih hrest]
    | dflt => simp [runFrom, ih hrest]

/-- A `break` ends the entry's statements. -/
theorem break_stops (xs ys : List Item) (h : Item.brk ∉ xs) :
    runFrom (xs ++ .brk :: ys) = runFrom xs := by
  rw [fallthrough_composes xs _ h]
  simp [runFrom]

/-- Every `case` label is an entry that runs its suffix. -/
theorem case_entry_lowered (pre post : List Item) (c : Nat) (cs : List Nat) :
    ([(c, true)], runFrom post) ∈ entries cs (pre ++ .case c :: post) := by
  induction pre with
  | nil => simp [entries]
  | cons x rest ih =>
    cases x with
    | case d => exact List.mem_cons_of_mem _ ih
    | dflt => exact List.mem_cons_of_mem _ ih
    | stmt s => exact ih
    | brk => exact ih

/-- With no `default`, the last path takes no case and runs nothing. -/
theorem no_default_path (items : List Item) (h : items.contains .dflt = false) :
    ((constants items).map (·, false), []) ∈ lowerSwitch items := by
  unfold lowerSwitch
  rw [h]
  simp

/-- Shared vectors, executed through the importer by the Rust binding. -/
def switchVectors : List (List Item × List Lowered) := [
  ([.case 1, .stmt 1, .case 2, .stmt 2, .brk, .dflt, .stmt 3],
   [([(1, true)], [1, 2]), ([(2, true)], [2]), ([(1, false), (2, false)], [3])]),
  ([.case 1, .stmt 1, .brk, .case 2, .stmt 2],
   [([(1, true)], [1]), ([(2, true)], [2]), ([(1, false), (2, false)], [])]),
  ([.case 1, .case 2, .stmt 1, .brk, .dflt, .brk],
   [([(1, true)], [1]), ([(2, true)], [1]), ([(1, false), (2, false)], [])]),
  ([.case 1, .stmt 1, .dflt, .stmt 2, .case 3, .stmt 3],
   [([(1, true)], [1, 2, 3]), ([(1, false), (3, false)], [2, 3]), ([(3, true)], [3])]),
  ([.case 1, .stmt 7, .case 2, .stmt 7, .brk],
   [([(1, true)], [7, 7]), ([(2, true)], [7]), ([(1, false), (2, false)], [])]),
  ([],
   [([], [])])
]

theorem switch_vectors_hold :
    switchVectors.all (fun v => lowerSwitch v.1 == v.2) = true := by decide

/-! ## Budgets: exhaustion is Unknown, never Safe -/

structure Usage where
  paths : Nat
  operations : Nat
  depth : Nat
  nodes : Nat
  deriving DecidableEq, Repr

/-- `MAX_PATHS`, `MAX_OPERATIONS`, `MAX_DEPTH`, `MAX_NODES`. -/
def budgets : Usage := ⟨64, 512, 64, 2048⟩

def within (u : Usage) : Bool :=
  u.paths ≤ budgets.paths && u.operations ≤ budgets.operations &&
    u.depth ≤ budgets.depth && u.nodes ≤ budgets.nodes

inductive Outcome where
  | summary
  | unknown
  deriving DecidableEq, Repr

def importOutcome (u : Usage) : Outcome := if within u then .summary else .unknown

inductive Verdict where
  | safe
  | pending
  | unknown
  | «unsafe»
  deriving DecidableEq, Repr

def rank : Verdict → Nat
  | .safe => 0
  | .pending => 1
  | .unknown => 2
  | .«unsafe» => 3

/-- The join of `Safe < Pending < Unknown < Unsafe`. -/
def join (a b : Verdict) : Verdict := if rank b ≤ rank a then a else b

/-- A consumer's verdict: an Unknown import joins Unknown into whatever the
consumer would otherwise conclude. -/
def consumerVerdict (o : Outcome) (v : Verdict) : Verdict :=
  match o with
  | .summary => v
  | .unknown => join .unknown v

theorem exhausted_unknown (u : Usage) (h : within u = false) : importOutcome u = .unknown := by
  simp [importOutcome, h]

/-- Budget exhaustion is never Safe, under any consumer. -/
theorem exhausted_never_safe (u : Usage) (v : Verdict) (h : within u = false) :
    consumerVerdict (importOutcome u) v ≠ .safe := by
  rw [exhausted_unknown u h]
  cases v <;> decide

theorem one_path_too_many : within ⟨65, 0, 0, 0⟩ = false := by decide

end QCore.TypedSourceControl
