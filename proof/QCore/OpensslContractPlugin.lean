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

import QCore.SubcommandPlugins

/-!
# The OpenSSL contract plugin

Card **W48-OPENSSL-CONTRACT-PLUGIN** (standing constraint 52): the external
subcommand `quirin-audit openssl check` checks OpenSSL's own contracts against
the pinned release and answers one claim per binding through the host's
plugin firewall (`QCore.SubcommandPlugins`, rule 10h). This is the model of
`crates/quirin-audit-plugin-openssl` (`check_binding`, `BindingCheck::proposed`,
`checker::pure_node`, `checker::escapes`, `checker::no_pointer_interface`).

## What is proved

* **The proposal.** A binding's obligations are the release pin, the public
  declaration, and — for a class the checker has a rule for — the definition
  and the class rule, in that order, each evaluated only when the earlier ones
  allow (`obligations`). The plugin proposes `Safe` exactly when every one is
  discharged, and `Unknown` otherwise; it never proposes `Unsafe`. A pin
  mismatch, an undeclared or mistyped prototype, a class without a rule, a
  missing definition or a refused body each make the proposal `Unknown`.
* **The published row.** Through the host (no trusted checker accepts a
  subcommand plugin's certificate) a binding's row is never `Safe`; it is
  `Pending` exactly when every obligation is discharged and the run completed,
  and `Unknown` otherwise (a failed run included). A host verdict about the
  binding is never lowered.
* **The class rules' premises.** Every node the pure-reader whitelist admits
  has no effect (no call, write, divergence or read outside the arguments),
  so a body of admitted nodes has none; every pointer parameter a member-store
  body stores is among the escapes the checker computes, and the checker's
  escapes are exactly the stored pointer parameters; a no-pointer interface
  passes and returns nothing through which a caller's object is reached.

## What this is not

That the Rust reads Clang's JSON AST into these nodes as modelled (which
Clang node kind, operator and cast is which `Node`), that Clang's AST is the
C semantics of the definition, that the prepared tree is the upstream release
the distribution patched, and that a call binds the external definition, are
premises named in every claim's assumptions and pinned by the crate's tests
(`tests/classes.rs`, `tests/obligations.rs`, `tests/host.rs`), not proved
here.
-/

namespace QCore.OpensslContractPlugin

open QCore.VerdictSemigroup
open QCore.SubcommandPlugins

/-! ## The proposal -/

/-- What the checker established for one binding, before it forms the
obligation list. -/
structure Outcome where
  /-- The prepared tree has the pinned digest and version. -/
  pin : Bool
  /-- The public headers declare the contract's prototype. -/
  declared : Bool
  /-- The checker has a rule for the binding's class. -/
  ruled : Bool
  /-- One external definition, in the contract's file, with that prototype. -/
  defined : Bool
  /-- The class rule admits the definition. -/
  rule : Bool
  deriving DecidableEq, Repr

/-- The obligations, as `check_binding` lists them: each later one only when
the earlier ones allow it to be evaluated; an obligation not evaluated is not
discharged. -/
def obligations (o : Outcome) : List Bool :=
  if !o.pin then [false, false]
  else if !o.ruled then [true, o.declared, false]
  else if !o.defined then [true, o.declared, false]
  else [true, o.declared, true, o.rule]

/-- Every obligation discharged, and at least one listed. -/
def discharged (l : List Bool) : Bool :=
  !l.isEmpty && l.all id

/-- The plugin's proposal: `Safe` with its check record offered by reference
(a certificate no trusted checker reads), or `Unknown`. -/
def propose (o : Outcome) : Proposal :=
  if discharged (obligations o) then .safe (.offered .noChecker) else .unknown

theorem obligations_discharged_iff (o : Outcome) :
    discharged (obligations o) = true ↔
      o.pin = true ∧ o.declared = true ∧ o.ruled = true ∧ o.defined = true ∧
        o.rule = true := by
  cases o with
  | mk pin declared ruled defined rule =>
    cases pin <;> cases declared <;> cases ruled <;> cases defined <;> cases rule <;> decide

/-- The plugin proposes `Safe` exactly when every obligation is discharged. -/
theorem a_binding_is_proposed_safe_iff_every_obligation_is_discharged (o : Outcome) :
    (∃ e, propose o = .safe e) ↔
      o.pin = true ∧ o.declared = true ∧ o.ruled = true ∧ o.defined = true ∧
        o.rule = true := by
  rw [← obligations_discharged_iff]
  unfold propose
  cases discharged (obligations o) <;> simp

/-- The plugin never proposes `Unsafe`. -/
theorem the_plugin_never_proposes_unsafe (o : Outcome) (e : Evidence) :
    propose o ≠ .violated e := by
  unfold propose
  cases discharged (obligations o) <;> simp

/-- A pin mismatch makes the binding `Unknown`, whatever else holds. -/
theorem a_pin_mismatch_is_unknown (o : Outcome) (h : o.pin = false) :
    propose o = .unknown := by
  cases o with
  | mk pin declared ruled defined rule =>
    subst h
    simp [propose, obligations, discharged]

/-- A class the checker has no rule for is `Unknown`. -/
theorem an_unruled_class_is_unknown (o : Outcome) (h : o.ruled = false) :
    propose o = .unknown := by
  cases o with
  | mk pin declared ruled defined rule =>
    subst h
    cases pin <;> simp [propose, obligations, discharged]

/-- An undeclared or mistyped prototype, a missing definition or a refused
body is `Unknown`. -/
theorem an_undischarged_obligation_is_unknown (o : Outcome)
    (h : o.declared = false ∨ o.defined = false ∨ o.rule = false) :
    propose o = .unknown := by
  unfold propose
  have : discharged (obligations o) = false := by
    cases hd : discharged (obligations o) with
    | false => rfl
    | true =>
      obtain ⟨_, d, _, f, r⟩ := (obligations_discharged_iff o).mp hd
      rcases h with h | h | h <;> simp_all
  simp [this]

/-! ## The published row -/

/-- The row the host publishes for a binding: its own host verdict (`Safe`,
the identity, when the audit report names no such artifact) joined with the
run's and the classified claim. -/
def bindingRow (host : V) (completed : Bool) (o : Outcome) : V :=
  row host (if completed then .completed [propose o] else .failure)

theorem propose_uncheckable (o : Outcome) : Uncheckable (propose o) := by
  unfold propose
  cases discharged (obligations o) <;> simp [Uncheckable, Unchecked]

/-- No binding row is `Safe`: the plugin's `Safe` is uncertified. -/
theorem a_binding_row_is_never_safe (host : V) (completed : Bool) (o : Outcome) :
    bindingRow host completed o ≠ .safe := by
  intro safe
  cases completed with
  | false =>
    have := (a_row_is_safe_only_with_accepted_certificates host .failure).mp safe
    obtain ⟨_, _, h, _⟩ := this
    cases h
  | true =>
    have := (without_a_checker_only_an_unclaimed_safe_row_is_safe host [propose o]
      (by simpa using propose_uncheckable o)).mp safe
    simp at this

/-- With no host row, a binding's row is `Pending` exactly when the run
completed and every obligation is discharged. -/
theorem a_binding_row_is_pending_iff_discharged (completed : Bool) (o : Outcome) :
    bindingRow .safe completed o = .pending ↔
      completed = true ∧ o.pin = true ∧ o.declared = true ∧ o.ruled = true ∧
        o.defined = true ∧ o.rule = true := by
  rw [← obligations_discharged_iff]
  cases completed <;> cases h : discharged (obligations o) <;>
    simp [bindingRow, propose, h] <;> decide

/-- With no host row, a binding's row is `Unknown` exactly when the run failed
or an obligation is undischarged. -/
theorem a_binding_row_is_unknown_otherwise (completed : Bool) (o : Outcome) :
    bindingRow .safe completed o = .unknown ↔
      (completed = false ∨ discharged (obligations o) = false) := by
  cases completed <;> cases h : discharged (obligations o) <;>
    simp [bindingRow, propose, h] <;> decide

/-- The plugin never lowers the host's verdict about a binding. -/
theorem a_binding_row_never_lowers_the_host (host : V) (completed : Bool) (o : Outcome) :
    host.rank ≤ (bindingRow host completed o).rank :=
  the_join_never_lowers_the_host host _

/-! ## The class rules' premises -/

/-- A cast kind Clang names. -/
inductive Cast where
  | lvalueToRValue | noOp | bitCast | nullToPointer | integralCast
  | integralToBoolean | pointerToBoolean | arrayToPointerDecay
  | functionToPointerDecay | builtinFnToFnPtr | other
  deriving DecidableEq, Repr

/-- A binary operator: computing ones, and assigning ones. -/
inductive BinOp where
  | compare | logical | arithmetic | bitwise | shift | comma
  | assign | compoundAssign
  deriving DecidableEq, Repr

/-- A unary operator: computing ones, and the increments and decrements. -/
inductive UnOp where
  | not | neg | plus | complement | deref | addressOf
  | preInc | preDec | postInc | postDec
  deriving DecidableEq, Repr

/-- What a name refers to. -/
inductive Target where
  | parameter | bodyLocal | enumerator | global | function
  deriving DecidableEq, Repr

/-- A node of a definition's body, as `checker::pure_node` sees it. -/
inductive Node where
  | compound | returnStmt | ifStmt | nullStmt | declStmt | paren | member
  | subscript | integerLiteral | characterLiteral | conditional | constant
  | sizeOf
  | localDecl (isStatic : Bool)
  | cast (kind : Cast)
  | binary (op : BinOp)
  | unary (op : UnOp)
  | ref (target : Target)
  | call | loop | gotoStmt | statementExpression | stringLiteral | other
  deriving DecidableEq, Repr

/-- The whitelist of `checker::pure_node`. -/
def admitted : Node → Bool
  | .compound | .returnStmt | .ifStmt | .nullStmt | .declStmt | .paren | .member
  | .subscript | .integerLiteral | .characterLiteral | .conditional | .constant
  | .sizeOf => true
  | .localDecl isStatic => !isStatic
  | .cast .lvalueToRValue | .cast .noOp | .cast .bitCast | .cast .nullToPointer
  | .cast .integralCast | .cast .integralToBoolean | .cast .pointerToBoolean
  | .cast .arrayToPointerDecay => true
  | .cast _ => false
  | .binary .assign | .binary .compoundAssign => false
  | .binary _ => true
  | .unary .preInc | .unary .preDec | .unary .postInc | .unary .postDec => false
  | .unary _ => true
  | .ref .parameter | .ref .bodyLocal | .ref .enumerator => true
  | .ref _ => false
  | .call | .loop | .gotoStmt | .statementExpression | .stringLiteral | .other => false

/-- What a node can do beyond computing a value from the arguments: call,
write state that outlives it, fail to terminate, or read storage that is not
the arguments'. -/
inductive Effect where
  | none | calls | writes | diverges | readsOutside
  deriving DecidableEq, Repr

/-- The effect of each node, by its C meaning (an unrecognised node is taken
to call). -/
def effect : Node → Effect
  | .call | .statementExpression | .other => .calls
  | .cast .functionToPointerDecay | .cast .builtinFnToFnPtr | .cast .other => .calls
  | .ref .function => .calls
  | .binary .assign | .binary .compoundAssign => .writes
  | .unary .preInc | .unary .preDec | .unary .postInc | .unary .postDec => .writes
  | .localDecl true => .writes
  | .loop | .gotoStmt => .diverges
  | .ref .global | .stringLiteral => .readsOutside
  | _ => .none

/-- Every node the whitelist admits has no effect. -/
theorem an_admitted_node_has_no_effect (n : Node) (h : admitted n = true) :
    effect n = .none := by
  cases n with
  | localDecl s => cases s <;> simp_all [admitted, effect]
  | cast k => cases k <;> simp_all [admitted, effect]
  | binary op => cases op <;> simp_all [admitted, effect]
  | unary op => cases op <;> simp_all [admitted, effect]
  | ref t => cases t <;> simp_all [admitted, effect]
  | _ => simp_all [admitted, effect]

/-- A body every node of which the whitelist admits has no effect anywhere: it
calls nothing, writes no state, terminates and reads only its arguments'
memory and constants. -/
theorem a_pure_reader_body_has_no_effect (body : List Node)
    (h : ∀ n ∈ body, admitted n = true) : ∀ n ∈ body, effect n = .none :=
  fun n mem => an_admitted_node_has_no_effect n (h n mem)

/-- A node with an effect is refused. -/
theorem an_effectful_node_is_refused (n : Node) (h : effect n ≠ .none) :
    admitted n = false := by
  cases hn : admitted n with
  | false => rfl
  | true => exact absurd (an_admitted_node_has_no_effect n hn) h

/-- One store of a member-store body: the parameters its value reads. -/
structure Store where
  base : Nat
  reads : List Nat
  deriving DecidableEq, Repr

/-- The escapes `checker::escapes` computes: the pointer parameters some store
reads. -/
def escapes (isPointer : Nat → Bool) (stores : List Store) : List Nat :=
  (stores.flatMap Store.reads).filter isPointer

/-- The computed escapes are exactly the stored pointer parameters. -/
theorem escapes_are_the_stored_pointers (isPointer : Nat → Bool) (stores : List Store)
    (i : Nat) :
    i ∈ escapes isPointer stores ↔ isPointer i = true ∧ ∃ s ∈ stores, i ∈ s.reads := by
  simp [escapes, List.mem_filter, List.mem_flatMap, and_comm]

/-- When the checker's escapes equal the contract's (the rule's last
condition), every pointer parameter a store reads is an escape the contract
states: the contract omits no escape. -/
theorem a_stored_pointer_is_a_stated_escape (isPointer : Nat → Bool) (stores : List Store)
    (stated : List Nat) (agree : escapes isPointer stores = stated)
    (s : Store) (mem : s ∈ stores) (i : Nat) (reads : i ∈ s.reads)
    (pointer : isPointer i = true) : i ∈ stated := by
  rw [← agree]
  exact (escapes_are_the_stored_pointers isPointer stores i).mpr ⟨pointer, s, mem, reads⟩

/-- A type at a function's interface. -/
inductive Ty where
  | void | builtinInteger | pointer | other
  deriving DecidableEq, Repr

/-- Whether a caller's object can be reached through a value of the type (an
unrecognised type is taken to reach one). -/
def reaches : Ty → Bool
  | .void | .builtinInteger => false
  | _ => true

/-- `checker::no_pointer_interface`: no parameter, and a void or builtin
integer result. -/
def noPointerInterface (parameters : List Ty) (result : Ty) : Bool :=
  parameters.isEmpty && (result == .void || result == .builtinInteger)

/-- A no-pointer interface passes in and returns nothing through which a
caller's object is reached. -/
theorem a_no_pointer_interface_reaches_nothing (parameters : List Ty) (result : Ty)
    (h : noPointerInterface parameters result = true) :
    (∀ t ∈ parameters, reaches t = false) ∧ reaches result = false := by
  cases parameters with
  | cons t ts => simp [noPointerInterface] at h
  | nil =>
    refine ⟨by simp, ?_⟩
    cases result <;> simp_all [noPointerInterface, reaches]

end QCore.OpensslContractPlugin
