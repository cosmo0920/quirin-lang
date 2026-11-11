// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Card **CG19-ATOMIC-SOURCE-ROUTE** (gateway rule 11i): the atomic source
//! route of [`crate::atomic_source_route`] connected to the custody gateway
//! (route G). Extended by card **CG28-WEAK-MEMORY-EVENTS** (rule 11t) to the
//! read-modify-write, compare-exchange and fence events.
//!
//! Every hook here is a **fallback**: it is consulted only where every
//! existing route of its position refused, so no answer an older route gives
//! is decided by different code (hard invariant 3). Each consultation is
//! recorded in the `QCORE_TYPED_PATH_PARITY` stream with
//! `position: "atomic-route-…"`.
//!
//! * **Statements.** A statement the classifier refused as a call to an
//!   uncontracted function whose name is an atomic builtin
//!   ([`atomic_source_route::names_atomic_builtin`]) is matched to its Clang
//!   node by line and tokens. A routed access (the builtin alone, `T x = A;`
//!   or `x = A;`, the destination only for a value-producing load,
//!   read-modify-write or compare-exchange) is lowered to its custody effect:
//!   the dereference of a pointer operand is the typed access-path reader's
//!   [`LegacyInstruction::IndirectScalarAccess`] (its base obligations
//!   checked by `transfer` exactly as for `*p`), a write through it (a
//!   store, a read-modify-write, a compare-exchange's object and `expected`
//!   operand) carries the refused-write frame of `*p = v`, `&x` reads nothing
//!   and a write to it forgets the scalar local `x`, a fence touches no
//!   object, and a produced value's scalar destination forgets its old fact.
//!   A stored, operand or desired value must be a literal, an enumerator or a
//!   non-pointer named object (`&x` for the generic compare-exchange's
//!   desired value). Every other answer replaces the uncontracted-call
//!   refusal by the route's **named** refusal (consume, a legacy `__sync`
//!   builtin, an invalid or unevaluated order or weak flag, an unread operand
//!   shape) with the same writes: still `Unknown`, never a neutral call.
//! * **`_Atomic` statements.** A statement any route refused whose Clang node
//!   is an `_Atomic` operator form at the top
//!   ([`atomic_source_route::classify_qualified`]): a simple assignment
//!   `*p = v`, and (CG28) a compound assignment `*p op= v` or increment
//!   `(*p)++` through a pointer object, is the plain store's or
//!   read-modify-write's custody effect (the typed reader's dereference and
//!   refused-write frame, read with the value type); every other such form is
//!   the route's named refusal.
//! * **Returns.** A refused `return` whose value is one atomic builtin is
//!   lowered the same way when the function's result is a scalar: a routed
//!   load, read-modify-write or compare-exchange is its access then an exit
//!   that escapes nothing; any other atomic is the route's named refusal, then
//!   that exit.
//!
//! The custody projection is order-insensitive, so a routed access's order
//! is carried only into the parity row: route G publishes no ordering,
//! race-freedom or liveness claim (the route's single-graph assumption).
//! A memory order spelled as an enumeration constant is refused here (the
//! function's AST does not carry the enumerator's value), `memory_order_consume`
//! by its own name ([`atomic_source_route::consume_only`]).

use serde_json::{Value, json};

use super::{LegacyInstruction, Parser, RefusedWrites, Statement, Token};
use crate::atomic_source_route::{
    self, AtomicRouteRefusal, Operation, RoutedAtomic, atomic_node_name, classify_builtin,
    classify_qualified,
};
use crate::typed_access_path::{StatementIndex, StatementPlan, is_pointer, record_row, strip};
use crate::typed_c_ast::{children, kind};

const UNCONTRACTED: &str = "call to uncontracted function `";

/// The atomic builtin an uncontracted-call refusal names, if any.
fn refused_atomic_builtin(description: &str) -> Option<&str> {
    let name = description.strip_prefix(UNCONTRACTED)?.strip_suffix('`')?;
    atomic_source_route::names_atomic_builtin(name).then_some(name)
}

fn strip_parens(mut node: &Value) -> &Value {
    while kind(node) == "ParenExpr" {
        let [child] = children(node) else { break };
        node = child;
    }
    node
}

fn texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

/// A value operand's node under Clang's value conversions.
fn strip_value(mut node: &Value) -> &Value {
    while matches!(
        kind(node),
        "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr"
    ) && node["castKind"].as_str() != Some("AtomicToNonAtomic")
    {
        let [child] = children(node) else { break };
        node = child;
    }
    node
}

/// Whether a pointer object's type points at a scalar the route reads, the
/// pointee possibly `_Atomic(T)`: `is_pointer` refuses every `_Atomic`
/// spelling, including the pointee's, which is this route's own case. The
/// pointer itself is unqualified, and a `volatile` pointee stays CG15's.
fn atomic_object_pointer(node: &Value) -> bool {
    let Some(text) = node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
    else {
        return false;
    };
    let Some(pointee) = text.trim_end().strip_suffix('*') else {
        return false;
    };
    let pointee = pointee.trim_end();
    let inner = pointee
        .strip_prefix("_Atomic(")
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or(pointee);
    !inner.is_empty()
        && !inner.contains(['(', ')', '*', '['])
        && !inner.contains("union ")
        && !inner.contains("struct ")
        && !inner.contains("volatile")
        && !inner.contains("_Atomic")
}

/// `&x` for a named object `x` (a local or a formal): its name.
fn address_of_named(node: &Value) -> Option<&str> {
    if kind(node) != "UnaryOperator" || node["opcode"].as_str() != Some("&") {
        return None;
    }
    let [target] = children(node) else {
        return None;
    };
    let target = strip(target)?;
    if kind(target) != "DeclRefExpr"
        || !matches!(
            target["referencedDecl"]["kind"].as_str(),
            Some("VarDecl" | "ParmVarDecl")
        )
    {
        return None;
    }
    target["referencedDecl"]["name"].as_str()
}

/// Whether a stored value is custody-neutral data: a literal, or a read of a
/// named non-pointer object or an enumerator.
fn neutral_value(node: &Value) -> bool {
    let node = strip_value(node);
    match kind(node) {
        "IntegerLiteral" | "CharacterLiteral" => true,
        "DeclRefExpr" => match node["referencedDecl"]["kind"].as_str() {
            Some("EnumConstantDecl") => true,
            Some("VarDecl" | "ParmVarDecl") => !is_pointer(node),
            _ => false,
        },
        _ => false,
    }
}

/// The routed atomic a statement or return value node holds: the node itself,
/// or the initializer / right operand of `T x = A;` / `x = A;`, with the
/// scalar object the value is stored into.
struct Site<'a> {
    atomic: &'a Value,
    destination: Option<String>,
}

fn site(node: &Value) -> Option<Site<'_>> {
    let bare = strip_value(node);
    if atomic_node_name(bare).is_some() {
        return Some(Site {
            atomic: bare,
            destination: None,
        });
    }
    match (kind(bare), children(bare)) {
        ("DeclStmt", [declaration]) if kind(declaration) == "VarDecl" => {
            let [initializer] = children(declaration) else {
                return None;
            };
            let atomic = strip_value(initializer);
            atomic_node_name(atomic)?;
            Some(Site {
                atomic,
                destination: Some(declaration["name"].as_str()?.to_owned()),
            })
        }
        ("BinaryOperator", [left, right]) if bare["opcode"].as_str() == Some("=") => {
            let left = strip_value(left);
            let atomic = strip_value(right);
            atomic_node_name(atomic)?;
            if kind(left) != "DeclRefExpr"
                || !matches!(
                    left["referencedDecl"]["kind"].as_str(),
                    Some("VarDecl" | "ParmVarDecl")
                )
            {
                return None;
            }
            Some(Site {
                atomic,
                destination: Some(left["referencedDecl"]["name"].as_str()?.to_owned()),
            })
        }
        _ => None,
    }
}

/// Whether a `return`'s lowering is the whole-call refusal of an uncontracted
/// atomic builtin (card W43-RETURN-EXPRESSION's re-attribution).
pub(super) fn refused_atomic_return(statement: &Statement) -> bool {
    matches!(
        statement,
        Statement::Return {
            returned_evaluation: Some(evaluation),
            ..
        } if matches!(
            evaluation.as_ref(),
            LegacyInstruction::Unknown { description, .. }
                if refused_atomic_builtin(description).is_some()
        )
    )
}

impl Parser<'_> {
    /// The typed reader's instruction for the routed access's dereference.
    fn atomic_path_instruction(&self, plan: &StatementPlan, write: bool) -> LegacyInstruction {
        LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
            alias_reads: plan
                .scalar
                .record_bases
                .iter()
                .filter_map(|base| self.read_alias(Some(base)))
                .collect(),
            bases: plan.scalar.bases.clone(),
            destination: None,
            frame: (write && plan.scalar.writes_through_pointer)
                .then(|| RefusedWrites::Sparing(self.frame_protected.clone())),
            chains: plan.chains.clone(),
            borrowed_destination: false,
            // Composition with CG12-VOID-VIEW-READBACK (rule 11j): a routed
            // atomic access is never a `void *` member readback.
            readback: None,
        }
    }

    /// Whether `name` is a scalar local the flow may forget.
    fn atomic_scalar_destination(&self, name: &str) -> bool {
        !self.protected_formals.contains(name)
            && crate::typed_field_access::is_scalar_initializer(
                name,
                &["1"],
                &self.typed_model,
                &self.typed_types,
            )
    }

    /// The custody effect of one pointer operand of a routed access: the
    /// object (or, card CG28, a compare-exchange's `expected` pointer) read
    /// and, for `write`, written through. `*p` for a pointer object `p` is the
    /// typed reader's dereference; `&x` reads nothing and a write forgets the
    /// scalar local `x`.
    fn lower_pointer_operand(
        &self,
        index: &StatementIndex,
        operand: &Value,
        value_type: &str,
        write: bool,
        never_bound: &dyn Fn(&str) -> bool,
    ) -> Option<Vec<LegacyInstruction>> {
        let object = strip(operand)?;
        match kind(object) {
            "DeclRefExpr"
                if matches!(
                    object["referencedDecl"]["kind"].as_str(),
                    Some("VarDecl" | "ParmVarDecl")
                ) && atomic_object_pointer(object) =>
            {
                let dereference = json!({
                    "kind": "UnaryOperator",
                    "opcode": "*",
                    "type": {"qualType": value_type},
                    "inner": [operand.clone()],
                });
                let mut reader = index.reader(never_bound);
                reader.path(&dereference, write, false)?;
                Some(vec![self.atomic_path_instruction(&reader.plan, write)])
            }
            "UnaryOperator" if object["opcode"].as_str() == Some("&") => {
                let name = address_of_named(object)?;
                if !write {
                    return Some(Vec::new());
                }
                if !self.atomic_scalar_destination(name) {
                    return None;
                }
                Some(vec![LegacyInstruction::ForgetScalar {
                    destination: name.to_owned(),
                }])
            }
            _ => None,
        }
    }

    /// The custody effect of one routed access. Card CG28 (rule 11t): a fence
    /// touches no object; a read-modify-write reads and writes its object; a
    /// compare-exchange reads and may write its object and its `expected`
    /// object, and reads its desired value (by pointer for the generic form).
    fn lower_routed(
        &self,
        index: &StatementIndex,
        routed: &RoutedAtomic<'_>,
        never_bound: &dyn Fn(&str) -> bool,
    ) -> Result<Vec<LegacyInstruction>, AtomicRouteRefusal> {
        let shape = || AtomicRouteRefusal::UnsupportedShape(routed.form.clone());
        if matches!(
            routed.operation,
            Operation::ThreadFence | Operation::SignalFence
        ) {
            return Ok(Vec::new());
        }
        let neutral = |value: &Value| {
            if routed.value_by_pointer {
                strip(value)
                    .filter(|value| kind(value) == "UnaryOperator")
                    .and_then(address_of_named)
                    .is_some()
            } else {
                neutral_value(value)
            }
        };
        if routed.value.is_some_and(|value| !neutral(value)) {
            return Err(shape());
        }
        let write = routed.operation != Operation::Load;
        let mut instructions = self
            .lower_pointer_operand(index, routed.object, &routed.value_type, write, never_bound)
            .ok_or_else(shape)?;
        if let Some(expected) = routed.expected {
            instructions.extend(
                self.lower_pointer_operand(index, expected, &routed.value_type, true, never_bound)
                    .ok_or_else(shape)?,
            );
        }
        Ok(instructions)
    }

    /// The custody effect of one routed `_Atomic` simple assignment, and
    /// (card CG28) compound assignment or increment: the plain store or
    /// read-modify-write `*p` it is, read by the typed reader through the
    /// lvalue with its value type (the reader refuses every `_Atomic` type
    /// spelling, which is this route's own case).
    fn lower_qualified_store(
        &self,
        index: &StatementIndex,
        routed: &RoutedAtomic<'_>,
        never_bound: &dyn Fn(&str) -> bool,
    ) -> Result<Vec<LegacyInstruction>, AtomicRouteRefusal> {
        let shape = || AtomicRouteRefusal::UnsupportedShape(routed.form.clone());
        if !matches!(
            routed.operation,
            Operation::Store | Operation::ReadModifyWrite
        ) || (routed.operation == Operation::Store && routed.value.is_none())
            || routed.value.is_some_and(|value| !neutral_value(value))
        {
            return Err(shape());
        }
        let place = strip_parens(routed.object);
        match kind(place) {
            "UnaryOperator" if place["opcode"].as_str() == Some("*") => {
                let [pointer] = children(place) else {
                    return Err(shape());
                };
                let pointer = strip(pointer).ok_or_else(shape)?;
                if kind(pointer) != "DeclRefExpr"
                    || !matches!(
                        pointer["referencedDecl"]["kind"].as_str(),
                        Some("VarDecl" | "ParmVarDecl")
                    )
                    || !atomic_object_pointer(pointer)
                {
                    return Err(shape());
                }
                let mut dereference = place.clone();
                dereference["type"] = json!({"qualType": routed.value_type});
                let mut reader = index.reader(never_bound);
                reader.path(&dereference, true, false).ok_or_else(shape)?;
                Ok(vec![self.atomic_path_instruction(&reader.plan, true)])
            }
            _ => Err(shape()),
        }
    }

    /// Route one site: the access, then the loaded value's destination.
    fn lower_site<'n>(
        &self,
        index: &StatementIndex,
        site: &Site<'n>,
        never_bound: &dyn Fn(&str) -> bool,
    ) -> Result<(RoutedAtomic<'n>, Vec<LegacyInstruction>), AtomicRouteRefusal> {
        let routed = classify_builtin(site.atomic, &atomic_source_route::consume_only)?;
        let mut instructions = self.lower_routed(index, &routed, never_bound)?;
        if let Some(destination) = &site.destination {
            // Card CG28: a read-modify-write's old value and a
            // compare-exchange's outcome are scalar results too.
            if !matches!(
                routed.operation,
                Operation::Load | Operation::ReadModifyWrite | Operation::CompareExchange
            ) || self.formal_parameters.contains(destination.as_str())
                || !self.atomic_scalar_destination(destination)
            {
                return Err(AtomicRouteRefusal::UnsupportedShape(routed.form.clone()));
            }
            instructions.push(LegacyInstruction::ForgetScalar {
                destination: destination.clone(),
            });
        }
        Ok((routed, instructions))
    }

    fn record_atomic_route(
        &self,
        position: &str,
        line: usize,
        tokens: &[Token],
        outcome: &Result<(String, Option<String>), String>,
    ) {
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts(tokens),
            "position": position,
            "old_route": "old_refusal",
            "old_admitted": false,
            "new_admitted": outcome.is_ok(),
            "routed_form": outcome.as_ref().ok().map(|(form, _)| form),
            "routed_order": outcome.as_ref().ok().and_then(|(_, order)| order.clone()),
            "new_refusal": outcome.as_ref().err(),
            "disagreement": outcome.is_ok(),
        }));
    }

    /// Hook: a statement the classifier refused as a call to an uncontracted
    /// atomic builtin. `None` keeps the refusal byte for byte (no unique Clang
    /// node for the statement).
    pub(super) fn atomic_routed_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        let LegacyInstruction::Unknown {
            description,
            writes,
        } = instruction
        else {
            return None;
        };
        let builtin = refused_atomic_builtin(description).is_some();
        let index = self.typed_positions_index().cloned()?;
        let wanted = texts(tokens);
        let node = Self::elaboration_node(&index, tokens.first()?.line, &wanted, &|node| {
            !matches!(
                kind(node),
                "ReturnStmt"
                    | "CompoundStmt"
                    | "IfStmt"
                    | "WhileStmt"
                    | "DoStmt"
                    | "ForStmt"
                    | "SwitchStmt"
                    | "CaseStmt"
                    | "DefaultStmt"
                    | "LabelStmt"
            )
        })?;
        let never_bound = |name: &str| self.joined_never_bound(name);
        let outcome = if builtin {
            let site = site(node)?;
            self.lower_site(&index, &site, &never_bound)
        } else {
            // An `_Atomic` store or read-modify-write statement another route
            // refused (any refusal): the qualified form at the statement's top.
            let bare = strip_parens(node);
            if !matches!(
                kind(bare),
                "BinaryOperator" | "CompoundAssignOperator" | "UnaryOperator"
            ) {
                return None;
            }
            let answer = classify_qualified(bare)?;
            answer.and_then(|routed| {
                let instructions = self.lower_qualified_store(&index, &routed, &never_bound)?;
                Ok((routed, instructions))
            })
        };
        self.record_atomic_route(
            "atomic-route-statement",
            line,
            tokens,
            &outcome
                .as_ref()
                .map(|(routed, _)| (routed.form.clone(), routed.order.map(|o| format!("{o:?}"))))
                .map_err(AtomicRouteRefusal::reason),
        );
        Some(match outcome {
            Ok((_, instructions)) => Statement::Block(
                instructions
                    .into_iter()
                    .map(|instruction| Statement::Instruction(instruction, line))
                    .collect(),
            ),
            Err(refusal) => Statement::Instruction(
                LegacyInstruction::Unknown {
                    description: refusal.reason(),
                    writes: writes.clone(),
                },
                line,
            ),
        })
    }

    /// Hook: a refused `return` whose value is one atomic builtin, in a
    /// function whose result is a scalar. `None` keeps the refusal.
    pub(super) fn atomic_routed_return(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<Statement> {
        if !self.return_facts.scalar_result {
            return None;
        }
        let index = self.typed_positions_index().cloned()?;
        let mut wanted = vec!["return"];
        wanted.extend(texts(expression));
        let statement =
            Self::elaboration_node(&index, expression.first()?.line, &wanted, &|node| {
                kind(node) == "ReturnStmt"
            })?;
        let [value] = children(statement) else {
            return None;
        };
        let atomic = strip_value(value);
        atomic_node_name(atomic)?;
        let site = Site {
            atomic,
            destination: None,
        };
        let never_bound = |name: &str| self.position_never_bound(name);
        let outcome =
            self.lower_site(&index, &site, &never_bound)
                .and_then(|(routed, instructions)| {
                    if matches!(
                        routed.operation,
                        Operation::Load | Operation::ReadModifyWrite | Operation::CompareExchange
                    ) {
                        Ok((routed, instructions))
                    } else {
                        Err(AtomicRouteRefusal::UnsupportedShape(routed.form.clone()))
                    }
                });
        self.record_atomic_route(
            "atomic-route-return",
            line,
            expression,
            &outcome
                .as_ref()
                .map(|(routed, _)| (routed.form.clone(), routed.order.map(|o| format!("{o:?}"))))
                .map_err(AtomicRouteRefusal::reason),
        );
        let writes = self.expression_writes(expression);
        let mut statements = match outcome {
            Ok((_, instructions)) => instructions
                .into_iter()
                .map(|instruction| Statement::Instruction(instruction, line))
                .collect::<Vec<_>>(),
            Err(refusal) => vec![Statement::Instruction(
                LegacyInstruction::Unknown {
                    description: refusal.reason(),
                    writes: writes.clone(),
                },
                line,
            )],
        };
        // The scalar value escapes nothing: the exit hands no custody back.
        statements.push(Statement::Return {
            line,
            writes,
            expression_supported: true,
            returned_place: None,
            returned_evaluation: None,
            returned_evaluation_escapes: false,
            returns_null_constant: false,
        });
        Some(Statement::Block(statements))
    }
}
