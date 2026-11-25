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

/-!
# Type queries read as no access (card K2-NO-TYPED-ACCESS, rule 79)

The abstract rule implemented by `crates/qcore-c-gateway/src/type_query_positions.rs`
(`decide`, `facts`, `product_run`) and its two hooks
(`legacy_elaboration::elaborate_condition`,
`typed_field_access::is_declaration_candidate`).

Every condition is a formula of the C04.18.9 typed rendering over the K2
terms (`k2Terms`, sorts `k2Sorts`), checked by the premise-formula validator
in Rust; `formulaTable` is the Rust table, row for row. Each term is modelled
here: `Query` and `EventClass` are the facts `#k2_type_operand`,
`#k2_variably_modified`, `#k2_branch_event`, `#k2_object_read` and
`#k2_unaliased` read; `Facts` is a condition (`#k2_condition`, with
`#k2_query_of` and `#k2_event_of` its two lists); `Run` and `isDeclaration`
model `#k2_run_at`, `#k2_name_star_sizeof` and `#k2_declaration`
(`termModel`).

## (1) The region condition

`decide` is the Rust decision. The semantics: an abstract state `σ`, and an
event's effect on it. A branch or merge touches no storage; the read of an
unaliased automatic object reads one value through the object's own name and
changes nothing; every other event may change the state and may owe an
obligation (`effect`, `owes`). A path through the condition is any sublist of
its events (the arms C does not evaluate are left out). An admitted condition
reaches the state it started in on every path
(`an_admitted_condition_is_the_identity_on_every_path`) and owes nothing on
any path (`an_admitted_condition_owes_nothing`), so the lowering — no
instruction, then a branch refining neither edge — is the condition on both
edges (`the_lowering_is_the_condition`).

## (2) The product run

A declaration that names a variable puts an identifier after the declarator's
`*`; `sizeof` is a keyword, not an identifier, so a run `NAME * sizeof …`
begins no such declaration (`a_product_run_is_no_declaration`), and dropping
it from the candidates loses no declaration (`dropping_product_runs_loses_no_declaration`).

## What this is not

A model of Clang or of the lexical readers: the facts are their outputs
(trust boundary 28). The variably-modified test is conservative (any array
declarator in the spelling), a premise of the extraction.
-/

namespace QCore.TypeQueryPositions

/-! ## The facts -/

/-- One `sizeof`/`_Alignof` of the condition (`#k2_query_of`). -/
structure Query where
  /-- `#k2_type_operand`. -/
  typeOperand : Bool
  /-- `#k2_variably_modified`. -/
  variablyModified : Bool
  deriving DecidableEq, Repr

/-- One elaborated event (`#k2_event_of`), as the conditions read it. -/
inductive EventClass where
  /-- `#k2_branch_event`. -/
  | branch
  /-- `#k2_object_read`, with `#k2_unaliased`. -/
  | objectRead (unaliased : Bool)
  /-- A call, a store, a path read, a lifetime boundary. -/
  | other
  deriving DecidableEq, Repr

/-- One region condition (`#k2_condition`); the hook asks only where
`k2_region_site` holds. -/
structure Facts where
  queries : List Query
  events : List EventClass
  deriving DecidableEq, Repr

inductive Reason where
  | noTypeQuery
  | evaluatedQuery
  | event
  deriving DecidableEq, Repr

def Query.unevaluated (q : Query) : Bool := q.typeOperand && !q.variablyModified

def EventClass.inert : EventClass → Bool
  | .branch => true
  | .objectRead u => u
  | .other => false

/-- `type_query_positions::decide`. -/
def decide (f : Facts) : Option Reason :=
  if f.queries.isEmpty then some .noTypeQuery
  else if !f.queries.all Query.unevaluated then some .evaluatedQuery
  else if !f.events.all EventClass.inert then some .event
  else none

/-- Every condition holds. -/
def admits (f : Facts) : Bool :=
  !f.queries.isEmpty && f.queries.all Query.unevaluated && f.events.all EventClass.inert

theorem admitted_exactly_when (f : Facts) : decide f = none ↔ admits f = true := by
  unfold decide admits
  by_cases h1 : f.queries.isEmpty <;> by_cases h2 : f.queries.all Query.unevaluated <;>
    by_cases h3 : f.events.all EventClass.inert <;> simp [h1, h2, h3]

/-- The condition each refusal negates. -/
def conditionOf : Reason → Facts → Bool
  | .noTypeQuery, f => !f.queries.isEmpty
  | .evaluatedQuery, f => f.queries.all Query.unevaluated
  | .event, f => f.events.all EventClass.inert

theorem a_refusal_negates_its_condition (f : Facts) (r : Reason) (h : decide f = some r) :
    conditionOf r f = false := by
  unfold decide at h
  by_cases h1 : f.queries.isEmpty <;> by_cases h2 : f.queries.all Query.unevaluated <;>
    by_cases h3 : f.events.all EventClass.inert <;> simp [h1, h2, h3] at h <;>
    subst h <;> simp [conditionOf, h1, h2, h3]

/-! ## The semantics of a condition -/

-- The abstract state is opaque: a state transformer per event.
variable {σ : Type}

/-- An event's effect: inert events change nothing; any other event may do
anything (`step`). -/
def effect (step : EventClass → σ → σ) (e : EventClass) (s : σ) : σ :=
  if e.inert then s else step e s

/-- Whether an event may owe an obligation (a dereference, a call's contract,
a store): only a non-inert one. -/
def owes (e : EventClass) : Bool := !e.inert

/-- Running a path (a list of events, in order). -/
def run (step : EventClass → σ → σ) : List EventClass → σ → σ
  | [], s => s
  | e :: es, s => run step es (effect step e s)

theorem run_of_inert (step : EventClass → σ → σ) (es : List EventClass)
    (h : es.all EventClass.inert = true) (s : σ) : run step es s = s := by
  induction es generalizing s with
  | nil => rfl
  | cons e es ih =>
    simp only [List.all_cons, Bool.and_eq_true] at h
    simp [run, effect, h.1, ih h.2]

theorem sublist_of_inert {es ps : List EventClass} (hp : ps.Sublist es)
    (h : es.all EventClass.inert = true) : ps.all EventClass.inert = true := by
  rw [List.all_eq_true] at h ⊢
  intro x hx
  exact h x (hp.subset hx)

/-- Whichever arms C evaluates (any sublist of the events), an admitted
condition leaves the abstract state as it found it. -/
theorem an_admitted_condition_is_the_identity_on_every_path (step : EventClass → σ → σ)
    (f : Facts) (h : decide f = none) (path : List EventClass) (hp : path.Sublist f.events)
    (s : σ) : run step path s = s := by
  have ha := (admitted_exactly_when f).1 h
  simp only [admits, Bool.and_eq_true] at ha
  exact run_of_inert step path (sublist_of_inert hp ha.2) s

/-- No event of any path of an admitted condition owes an obligation. -/
theorem an_admitted_condition_owes_nothing (f : Facts) (h : decide f = none)
    (path : List EventClass) (hp : path.Sublist f.events) : path.all (fun e => !owes e) = true := by
  have ha := (admitted_exactly_when f).1 h
  simp only [admits, Bool.and_eq_true] at ha
  have hi := sublist_of_inert hp ha.2
  rw [List.all_eq_true] at hi ⊢
  intro x hx
  simp [owes, hi x hx]

/-- The lowering of an admitted condition: no instruction, then a branch to
both edges (`Target::True`, `Target::False`) refining neither. -/
structure Lowering where
  instructions : List EventClass
  trueEdge : Bool
  falseEdge : Bool
  deriving DecidableEq, Repr

def lowering : Lowering := { instructions := [], trueEdge := true, falseEdge := true }

/-- The lowering reaches, on both edges, the state every path of the
condition reaches, and keeps both edges. -/
theorem the_lowering_is_the_condition (step : EventClass → σ → σ) (f : Facts)
    (h : decide f = none) (path : List EventClass) (hp : path.Sublist f.events) (s : σ) :
    run step lowering.instructions s = run step path s ∧ lowering.trueEdge = true ∧
      lowering.falseEdge = true := by
  refine ⟨?_, rfl, rfl⟩
  rw [an_admitted_condition_is_the_identity_on_every_path step f h path hp s]
  rfl

theorem a_call_or_store_is_refused (f : Facts) (h : EventClass.other ∈ f.events) :
    decide f ≠ none := by
  intro hd
  have ha := (admitted_exactly_when f).1 hd
  simp only [admits, Bool.and_eq_true, List.all_eq_true] at ha
  exact absurd (ha.2 _ h) (by simp [EventClass.inert])

theorem an_aliased_read_is_refused (f : Facts) (h : EventClass.objectRead false ∈ f.events) :
    decide f ≠ none := by
  intro hd
  have ha := (admitted_exactly_when f).1 hd
  simp only [admits, Bool.and_eq_true, List.all_eq_true] at ha
  exact absurd (ha.2 _ h) (by simp [EventClass.inert])

theorem an_evaluated_query_is_refused (f : Facts) (q : Query) (hq : q ∈ f.queries)
    (h : q.unevaluated = false) : decide f ≠ none := by
  intro hd
  have ha := (admitted_exactly_when f).1 hd
  simp only [admits, Bool.and_eq_true, List.all_eq_true] at ha
  exact absurd (ha.1.2 q hq) (by simp [h])

theorem a_variably_modified_query_is_refused (f : Facts) (q : Query) (hq : q ∈ f.queries)
    (h : q.variablyModified = true) : decide f ≠ none :=
  an_evaluated_query_is_refused f q hq (by simp [Query.unevaluated, h])

theorem a_condition_without_a_query_is_refused (f : Facts) (h : f.queries = []) :
    decide f = some .noTypeQuery := by
  simp [decide, h]

/-! ## (2) The product run -/

/-- A token. -/
inductive Tok where
  | ident (name : String)
  | keyword (name : String)
  | punct (text : String)
  deriving DecidableEq, Repr

/-- A run of tokens (`#k2_run_at`). -/
abbrev Run := List Tok

/-- An identifier token. -/
def isIdent : Tok → Bool
  | .ident _ => true
  | _ => false

/-- `#k2_name_star_sizeof`. -/
def nameStarSizeof : Run → Bool
  | t1 :: t2 :: t3 :: _ => isIdent t1 && t2 == .punct "*" && t3 == .keyword "sizeof"
  | _ => false

/-- A declarator identifier (C11 6.7.6): an identifier, never a keyword. -/
def declaratorIdent : Tok → Bool := isIdent

/-- `#k2_declaration` for the runs the rule meets: a run `SPEC * D …` begins a
pointer declaration naming a variable only when `D` is a declarator
identifier (a further `*` or a qualifier before one is the reader's own next
candidate). -/
def isDeclaration : Run → Bool
  | t1 :: t2 :: d :: _ => isIdent t1 && t2 == .punct "*" && declaratorIdent d
  | _ => false

theorem sizeof_is_no_declarator : declaratorIdent (.keyword "sizeof") = false := rfl

theorem a_product_run_is_no_declaration (r : Run) (h : nameStarSizeof r = true) :
    isDeclaration r = false := by
  match r with
  | t1 :: t2 :: t3 :: rest =>
    simp only [nameStarSizeof, Bool.and_eq_true, beq_iff_eq] at h
    obtain ⟨_, h3⟩ := h
    subst h3
    simp [isDeclaration, declaratorIdent, isIdent]
  | [] => simp [nameStarSizeof] at h
  | [_] => simp [nameStarSizeof] at h
  | [_, _] => simp [nameStarSizeof] at h

/-- The candidates with product runs dropped. -/
def candidates (runs : List Run) : List Run := runs.filter (fun r => !nameStarSizeof r)

theorem dropping_product_runs_loses_no_declaration (runs : List Run) (r : Run) (hr : r ∈ runs)
    (hd : isDeclaration r = true) : r ∈ candidates runs := by
  unfold candidates
  rw [List.mem_filter]
  refine ⟨hr, ?_⟩
  cases h : nameStarSizeof r
  · rfl
  · exact absurd hd (by rw [a_product_run_is_no_declaration r h]; simp)

/-! ## The tables shared with Rust -/

def vectorFacts (differs : String) : Facts :=
  let q : Query := { typeOperand := true, variablyModified := false }
  let evs : List EventClass := [.objectRead true, .branch, .objectRead true, .branch]
  if differs = "no type query" then { queries := [], events := evs }
  else if differs = "an expression operand" then
    { queries := [{ typeOperand := false, variablyModified := true }], events := evs }
  else if differs = "a variably modified operand" then
    { queries := [{ typeOperand := true, variablyModified := true }], events := evs }
  else if differs = "a call event" then
    { queries := [q], events := [.objectRead true, .branch, .other, .branch] }
  else if differs = "an address-taken object read" then
    { queries := [q], events := [.objectRead true, .branch, .objectRead false, .branch] }
  else { queries := [q], events := evs }

def code : Option Reason → String
  | none => "admit"
  | some .noTypeQuery => "k2_refuse_no_type_query"
  | some .evaluatedQuery => "k2_refuse_evaluated_query"
  | some .event => "k2_refuse_event"

def decisionVectors : List (String × String × String) := [("admitted", "none", "admit"), ("no_query", "no type query", "k2_refuse_no_type_query"), ("expression_operand", "an expression operand", "k2_refuse_evaluated_query"), ("vla_operand", "a variably modified operand", "k2_refuse_evaluated_query"), ("call_event", "a call event", "k2_refuse_event"), ("aliased_read", "an address-taken object read", "k2_refuse_event")]

theorem decision_vectors_hold :
    decisionVectors.all (fun (_, d, e) => code (decide (vectorFacts d)) == e) = true := by
  decide

def k2Sorts : List String := ["k2_site", "k2_event", "k2_query", "k2_run"]

def k2Terms : List String := ["k2_condition", "k2_older_refused", "k2_region", "k2_scalar_result", "k2_event_of", "k2_branch_event", "k2_object_read", "k2_unaliased", "k2_query_of", "k2_type_operand", "k2_variably_modified", "k2_run_at", "k2_name_star_sizeof", "k2_declaration"]

/-- The Lean definition each term is modelled by. -/
def termModel : List (String × String) := [
  ("k2_condition", "Facts"),
  ("k2_older_refused", "Facts (the hook's precondition)"),
  ("k2_region", "Facts (the hook's precondition)"),
  ("k2_scalar_result", "Facts (the hook's precondition)"),
  ("k2_event_of", "Facts.events"),
  ("k2_branch_event", "EventClass.branch"),
  ("k2_object_read", "EventClass.objectRead"),
  ("k2_unaliased", "EventClass.objectRead true"),
  ("k2_query_of", "Facts.queries"),
  ("k2_type_operand", "Query.typeOperand"),
  ("k2_variably_modified", "Query.variablyModified"),
  ("k2_run_at", "Run"),
  ("k2_name_star_sizeof", "nameStarSizeof"),
  ("k2_declaration", "isDeclaration")]

def formulaTable : List (String × String) := [
  ("k2_region_site", "((#k2_older_refused(#k2_condition) && #k2_region(#k2_condition)) && #k2_scalar_result(#k2_condition))"),
  ("k2_type_query_present", "exists (#q: k2_query) { #k2_query_of(#q,#k2_condition) }"),
  ("k2_queries_unevaluated", "forall (#q: k2_query) { (#k2_query_of(#q,#k2_condition) => (#k2_type_operand(#q) && !#k2_variably_modified(#q))) }"),
  ("k2_events_inert", "forall (#e: k2_event) { (#k2_event_of(#e,#k2_condition) => (#k2_branch_event(#e) || (#k2_object_read(#e) && #k2_unaliased(#e)))) }"),
  ("k2_product_run_is_no_declaration", "(#k2_name_star_sizeof(#k2_run_at) => !#k2_declaration(#k2_run_at))"),
  ("k2_refuse_no_type_query", "forall (#q: k2_query) { !#k2_query_of(#q,#k2_condition) }"),
  ("k2_refuse_evaluated_query", "exists (#q: k2_query) { (#k2_query_of(#q,#k2_condition) && (!#k2_type_operand(#q) || #k2_variably_modified(#q))) }"),
  ("k2_refuse_event", "exists (#e: k2_event) { (#k2_event_of(#e,#k2_condition) && (!#k2_branch_event(#e) && (!#k2_object_read(#e) || !#k2_unaliased(#e)))) }")
]

theorem the_tables_are_counted :
    formulaTable.length = 8 ∧ k2Terms.length = 14 ∧ k2Sorts.length = 4 ∧
      termModel.map Prod.fst = k2Terms := by
  decide

end QCore.TypeQueryPositions
