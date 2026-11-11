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

namespace QCore.Frontend

/-! A reduced Cycle 10 model of the source-front-end boundary.  It deliberately
models the invariants shared by every later semantic pass: source locations are
valid and preserved, global declarations are unique, every recorded reference
resolves, and elaboration assigns a duplicate-free finite symbol domain. -/

structure Span where
  start : Nat
  stop : Nat
  deriving DecidableEq, Repr

def Span.Valid (span : Span) : Prop := span.start ≤ span.stop

inductive DeclarationKind where
  | dimension
  | axis
  | typeAlias
  | resourceType
  | errorType
  | constant
  | externFunction
  | function
  deriving DecidableEq, Repr

structure Declaration where
  kind : DeclarationKind
  name : String
  span : Span
  references : List String
  deriving DecidableEq, Repr

structure Program where
  declarations : List Declaration
  deriving DecidableEq, Repr

def Program.names (program : Program) : List String :=
  program.declarations.map Declaration.name

def Declaration.ValidIn (declaration : Declaration) (names : List String) : Prop :=
  declaration.span.Valid ∧
  declaration.references.all (fun reference => decide (reference ∈ names)) = true

instance (declaration : Declaration) (names : List String) :
    Decidable (declaration.ValidIn names) := by
  unfold Declaration.ValidIn Span.Valid
  infer_instance

def Program.WellFormed (program : Program) : Prop :=
  program.names.Nodup ∧
  program.declarations.all
    (fun declaration => decide (declaration.ValidIn program.names)) = true

instance (program : Program) : Decidable program.WellFormed := by
  unfold Program.WellFormed
  infer_instance

structure TypedProgram where
  source : Program
  symbols : List Nat
  deriving DecidableEq, Repr

def elaborate (program : Program) : Option TypedProgram :=
  if program.WellFormed then
    some {
      source := program
      symbols := List.range program.declarations.length
    }
  else
    none

theorem elaborate_implies_wellFormed {program : Program} {typed : TypedProgram}
    (result : elaborate program = some typed) : program.WellFormed := by
  unfold elaborate at result
  split at result
  · assumption
  · simp at result

theorem elaborate_preserves_source {program : Program} {typed : TypedProgram}
    (result : elaborate program = some typed) : typed.source = program := by
  unfold elaborate at result
  split at result
  · cases result
    rfl
  · simp at result

theorem elaborate_preserves_declaration_count
    {program : Program} {typed : TypedProgram}
    (result : elaborate program = some typed) :
    typed.source.declarations.length = program.declarations.length := by
  rw [elaborate_preserves_source result]

theorem elaborate_preserves_spans {program : Program} {typed : TypedProgram}
    (result : elaborate program = some typed) :
    typed.source.declarations.map Declaration.span =
      program.declarations.map Declaration.span := by
  rw [elaborate_preserves_source result]

theorem elaborate_has_unique_declarations
    {program : Program} {typed : TypedProgram}
    (result : elaborate program = some typed) : program.names.Nodup :=
  (elaborate_implies_wellFormed result).1

theorem elaborate_resolves_references
    {program : Program} {typed : TypedProgram}
    (result : elaborate program = some typed)
    {declaration : Declaration} (member : declaration ∈ program.declarations)
    {reference : String} (used : reference ∈ declaration.references) :
    reference ∈ program.names := by
  have validDecisions := List.all_eq_true.mp (elaborate_implies_wellFormed result).2
  have validDecision := validDecisions declaration member
  have valid : declaration.ValidIn program.names := of_decide_eq_true validDecision
  have referenceDecisions := List.all_eq_true.mp valid.2
  exact of_decide_eq_true (referenceDecisions reference used)

theorem elaborate_produces_valid_spans
    {program : Program} {typed : TypedProgram}
    (result : elaborate program = some typed)
    {declaration : Declaration} (member : declaration ∈ program.declarations) :
    declaration.span.Valid := by
  have validDecisions := List.all_eq_true.mp (elaborate_implies_wellFormed result).2
  exact (of_decide_eq_true (validDecisions declaration member) :
    declaration.ValidIn program.names).1

theorem elaborate_symbol_count
    {program : Program} {typed : TypedProgram}
    (result : elaborate program = some typed) :
    typed.symbols.length = program.declarations.length := by
  unfold elaborate at result
  split at result
  · cases result
    simp
  · simp at result

theorem elaborate_symbols_unique
    {program : Program} {typed : TypedProgram}
    (result : elaborate program = some typed) : typed.symbols.Nodup := by
  unfold elaborate at result
  split at result
  · cases result
    exact List.nodup_range
  · simp at result

theorem invalid_program_is_rejected (program : Program)
    (invalid : ¬program.WellFormed) : elaborate program = none := by
  simp [elaborate, invalid]

end QCore.Frontend

namespace QCore.Frontend.BodyScope

/-- Reduced lexical scope stack for guard/thread header binding. The body scope
is pushed once, used while elaborating that body, and popped on return. -/
abbrev Scope := List (String × Nat)
abbrev Scopes := List Scope

def enter (name : String) (symbol : Nat) (outer : Scopes) : Scopes :=
  [(name, symbol)] :: outer

def leave (scopes : Scopes) : Scopes := scopes.tail

def lookup (name : String) : Scopes → Option Nat
  | [] => none
  | scope :: outer =>
      match scope.find? (fun binding => binding.1 == name) with
      | some binding => some binding.2
      | none => lookup name outer

theorem binder_resolves_in_body (name : String) (symbol : Nat) (outer : Scopes) :
    lookup name (enter name symbol outer) = some symbol := by simp [lookup, enter]

theorem leaving_body_restores_outer (name : String) (symbol : Nat) (outer : Scopes) :
    leave (enter name symbol outer) = outer := rfl

theorem binder_cannot_escape (name queried : String) (symbol : Nat) (outer : Scopes) :
    lookup queried (leave (enter name symbol outer)) = lookup queried outer := rfl

end QCore.Frontend.BodyScope
