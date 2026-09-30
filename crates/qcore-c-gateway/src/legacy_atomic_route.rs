//! Card **CG19-ATOMIC-SOURCE-ROUTE** (gateway rule 11i): the atomic source
//! route of [`crate::atomic_source_route`] connected to the custody gateway
//! (route G).
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
//!   node by line and tokens. A routed plain load or store (the builtin alone,
//!   `T x = LOAD;` or `x = LOAD;`) is lowered to its custody effect, which is
//!   the plain access's: the dereference of the pointer operand is the typed
//!   access-path reader's [`LegacyInstruction::IndirectScalarAccess`] (its
//!   base obligations checked by `transfer` exactly as for `*p`), a store
//!   through it carries the refused-write frame of `*p = v`, and a loaded
//!   value's scalar destination forgets its old fact. Every other answer
//!   replaces the uncontracted-call refusal by the route's **named** refusal
//!   (read-modify-write, compare-exchange, fence, consume, an invalid or
//!   unevaluated order, an unread operand shape) with the same writes: still
//!   `Unknown`, never a neutral call.
//! * **`_Atomic` statements.** A statement any route refused whose Clang node
//!   is an `_Atomic` operator form at the top
//!   ([`atomic_source_route::classify_qualified`]): a simple assignment
//!   `*p = v` through a pointer object is the plain store's custody effect
//!   (the typed reader's dereference and refused-write frame, read with the
//!   value type); every other such store, and every compound assignment or
//!   increment (a read-modify-write), is the route's named refusal.
//! * **Returns.** A refused `return` whose value is one atomic builtin is
//!   lowered the same way when the function's result is a scalar: a routed
//!   load is its access then an exit that escapes nothing; any other atomic
//!   is the route's named refusal, then that exit.
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

    /// The custody effect of one routed plain access.
    fn lower_routed(
        &self,
        index: &StatementIndex,
        routed: &RoutedAtomic<'_>,
        never_bound: &dyn Fn(&str) -> bool,
    ) -> Result<Vec<LegacyInstruction>, AtomicRouteRefusal> {
        let shape = || AtomicRouteRefusal::UnsupportedShape(routed.form.clone());
        if routed.value.is_some_and(|value| !neutral_value(value)) {
            return Err(shape());
        }
        let write = routed.operation != Operation::Load;
        let object = strip(routed.object).ok_or_else(shape)?;
        match kind(object) {
            // `*p` for a pointer object `p`: the typed reader's dereference.
            "DeclRefExpr"
                if matches!(
                    object["referencedDecl"]["kind"].as_str(),
                    Some("VarDecl" | "ParmVarDecl")
                ) && atomic_object_pointer(object) =>
            {
                let dereference = json!({
                    "kind": "UnaryOperator",
                    "opcode": "*",
                    "type": {"qualType": routed.value_type},
                    "inner": [routed.object.clone()],
                });
                let mut reader = index.reader(never_bound);
                reader.path(&dereference, write, false).ok_or_else(shape)?;
                Ok(vec![self.atomic_path_instruction(&reader.plan, write)])
            }
            // `&x`: the named object itself.
            "UnaryOperator" if object["opcode"].as_str() == Some("&") => {
                let [target] = children(object) else {
                    return Err(shape());
                };
                let target = strip(target).ok_or_else(shape)?;
                if kind(target) != "DeclRefExpr"
                    || !matches!(
                        target["referencedDecl"]["kind"].as_str(),
                        Some("VarDecl" | "ParmVarDecl")
                    )
                {
                    return Err(shape());
                }
                let name = target["referencedDecl"]["name"]
                    .as_str()
                    .ok_or_else(shape)?;
                if !write {
                    return Ok(Vec::new());
                }
                if !self.atomic_scalar_destination(name) {
                    return Err(shape());
                }
                Ok(vec![LegacyInstruction::ForgetScalar {
                    destination: name.to_owned(),
                }])
            }
            _ => Err(shape()),
        }
    }

    /// The custody effect of one routed `_Atomic` simple assignment: the
    /// plain store `*p = v` or `x = v` it is, read by the typed reader through
    /// the lvalue with its value type (the reader refuses every `_Atomic`
    /// type spelling, which is this route's own case).
    fn lower_qualified_store(
        &self,
        index: &StatementIndex,
        routed: &RoutedAtomic<'_>,
        never_bound: &dyn Fn(&str) -> bool,
    ) -> Result<Vec<LegacyInstruction>, AtomicRouteRefusal> {
        let shape = || AtomicRouteRefusal::UnsupportedShape(routed.form.clone());
        if routed.operation != Operation::Store
            || routed.value.is_none_or(|value| !neutral_value(value))
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
            if routed.operation != Operation::Load
                || self.formal_parameters.contains(destination.as_str())
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
                    if routed.operation == Operation::Load {
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
