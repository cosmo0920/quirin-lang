//! Card **CG26-VARIADICS** (gateway rule 11w): the `va_list` builtins'
//! dedicated recognised form, connected to the custody gateway (route G).
//!
//! The hook is a **fallback**: it is consulted only for a statement every
//! older route refused as a call to an uncontracted `__builtin_va_start`,
//! `__builtin_va_end`, `__builtin_va_copy` or `__builtin_va_arg`, so no answer
//! an older route gives is decided by different code (hard invariant 3). Card
//! CG21's common opaque boundary for builtins is never asked for these four.
//!
//! The statement is matched to its Clang node by line and exact tokens. When
//! [`va_list_protocol::check`] establishes the function's protocol on every
//! path, the statement is lowered to its custody effect, which is none:
//!
//! * `va_start(L, P);`, `va_end(L);`, `va_copy(D, S);` and a discarded
//!   `va_arg(L, T);` are [`LegacyInstruction::NoOp`]. A list is an object of
//!   `va_list` type the projection never binds (no pointer identity can be
//!   stored in it, and none is read from it here), so initialising, reading
//!   and ending it acquires, releases, transfers and rebinds nothing;
//! * `T x = va_arg(L, T);` and `x = va_arg(L, T);` for a scalar local `x` and
//!   an arithmetic or enumeration `T` are [`LegacyInstruction::ForgetScalar`]:
//!   the value is caller data the projection does not track. A pointer `T`
//!   keeps a named refusal (the value is a caller pointer with no identity
//!   premise here; gap entry K09's pointer half stays open).
//!
//! When the protocol is not established, the statement keeps its refusal with
//! the protocol's **named** reason and the same writes: still `Unknown`, so a
//! missing `va_end`, a use after `va_end`, a second pass, or a `va_copy`
//! without its own `va_end` is never `Safe`. Every consultation is a parity row
//! (`position: "va-list-statement"`).

use serde_json::{Value, json};

use super::{LegacyInstruction, Parser, Statement, Token};
use crate::typed_access_path::record_row;
use crate::typed_c_ast::{children, kind};
use crate::va_list_protocol::{self, ARG, COPY, END, START};

const UNCONTRACTED: &str = "call to uncontracted function `";

/// The `va_*` builtin an uncontracted-call refusal names, if any.
fn refused_va_builtin(description: &str) -> Option<&str> {
    let name = description.strip_prefix(UNCONTRACTED)?.strip_suffix('`')?;
    va_list_protocol::names_va_builtin(name).then_some(name)
}

fn texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

fn strip_value(mut node: &Value) -> &Value {
    while matches!(kind(node), "ParenExpr" | "ImplicitCastExpr")
        || (kind(node) == "CStyleCastExpr" && node["castKind"].as_str() == Some("ToVoid"))
    {
        let Some(child) = children(node).first() else {
            break;
        };
        node = child;
    }
    node
}

/// Whether a `va_arg` result type is a scalar the projection never binds: an
/// arithmetic or enumeration type, never a pointer, array, record or
/// function. Read from Clang's own spelling of the canonical type.
fn scalar_type(node: &Value) -> bool {
    let text = node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
        .unwrap_or("");
    let text = text
        .trim_start_matches("const ")
        .trim_start_matches("volatile ");
    matches!(
        text,
        "char"
            | "signed char"
            | "unsigned char"
            | "short"
            | "unsigned short"
            | "int"
            | "unsigned int"
            | "long"
            | "unsigned long"
            | "long long"
            | "unsigned long long"
            | "double"
            | "long double"
            | "_Bool"
            | "__int128"
            | "unsigned __int128"
    ) || (text.starts_with("enum ") && !text.contains(['*', '[', '(']))
}

/// What the matched statement is.
enum Shape {
    /// A whole `va_start`/`va_end`/`va_copy` call, or a discarded `va_arg`.
    Neutral(&'static str),
    /// `va_arg` stored into the named object.
    Stored { destination: String, scalar: bool },
}

fn shape(node: &Value) -> Option<Shape> {
    let bare = strip_value(node);
    match kind(bare) {
        "CallExpr" => {
            let callee = children(bare).first().map(strip_value)?;
            match callee["referencedDecl"]["name"].as_str()? {
                START => Some(Shape::Neutral(START)),
                END => Some(Shape::Neutral(END)),
                COPY => Some(Shape::Neutral(COPY)),
                _ => None,
            }
        }
        "VAArgExpr" => Some(Shape::Neutral(ARG)),
        "DeclStmt" => {
            let [declaration] = children(bare) else {
                return None;
            };
            if kind(declaration) != "VarDecl" || declaration["storageClass"].is_string() {
                return None;
            }
            let [initializer] = children(declaration) else {
                return None;
            };
            let value = strip_value(initializer);
            (kind(value) == "VAArgExpr").then(|| Shape::Stored {
                destination: declaration["name"].as_str().unwrap_or("").to_owned(),
                scalar: scalar_type(value) && scalar_type(declaration),
            })
        }
        "BinaryOperator" if bare["opcode"].as_str() == Some("=") => {
            let [left, right] = children(bare) else {
                return None;
            };
            let value = strip_value(right);
            if kind(value) != "VAArgExpr" || kind(left) != "DeclRefExpr" {
                return None;
            }
            if !matches!(
                left["referencedDecl"]["kind"].as_str(),
                Some("VarDecl" | "ParmVarDecl")
            ) {
                return None;
            }
            Some(Shape::Stored {
                destination: left["referencedDecl"]["name"].as_str()?.to_owned(),
                scalar: scalar_type(value) && scalar_type(left),
            })
        }
        _ => None,
    }
}

const TAIL_PREFIX: &str = "arguments beyond the declared parameters of `";
const TAIL_SUFFIX: &str = "` are outside the resource projection";

/// The call to `callee` a statement node is: the statement itself, a
/// one-declarator declaration's initializer, or an assignment's right operand,
/// through parentheses and implicit conversions.
fn statement_call<'n>(node: &'n Value, callee: &str) -> Option<&'n Value> {
    let bare = strip_value(node);
    let call = match kind(bare) {
        "CallExpr" => bare,
        "DeclStmt" => {
            let [declaration] = children(bare) else {
                return None;
            };
            strip_value(children(declaration).first()?)
        }
        "BinaryOperator" if bare["opcode"].as_str() == Some("=") => {
            strip_value(children(bare).get(1)?)
        }
        _ => return None,
    };
    let function = children(call).first().map(strip_value)?;
    (kind(call) == "CallExpr" && function["referencedDecl"]["name"].as_str() == Some(callee))
        .then_some(call)
}

impl Parser<'_> {
    /// Card CG26-VARIADICS (rule 11w), the classifier's half of the tail
    /// widening: whether every trailing actual of `function` is T1-T6 data, an
    /// admitted nested call, or a T7/T8 shape. Asked only while
    /// `variadic_tail_widening` is set.
    pub(super) fn tail_widened(
        &self,
        function: &str,
        arguments: &[&[&str]],
        named: usize,
        nested: &[crate::variadic_nested::NestedCallActual],
    ) -> bool {
        let view = self.tail_view(function, arguments);
        view.iter().enumerate().skip(named).all(|(index, argument)| {
            nested.iter().any(|nested| nested.argument == index)
                || crate::variadic_nested::tail_arguments_are_data_with_nested(
                    &[argument],
                    0,
                    &[],
                    &self.typed_model,
                    &self.typed_types,
                    &self.typed_bases,
                )
                || crate::variadic_tail::widened_actual(
                    argument,
                    &self.typed_types,
                    &|name| self.descriptor_names.contains(name),
                    &self.protected_formals,
                    self.variadic_borrowed_tails.contains(function),
                )
                .is_some()
        })
    }

    /// Hook: a call statement the classifier refused only for its trailing
    /// actuals, to a callee whose declaration carries the data tail. The
    /// statement is re-classified with every trailing position read as
    /// `Borrowed` (T7, T8), after Clang's promoted type of every trailing actual
    /// is checked; a T7 scalar's fact is forgotten after the call. `None`
    /// keeps the refusal byte for byte.
    pub(super) fn variadic_tail_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        let LegacyInstruction::Unknown { description, .. } = instruction else {
            return None;
        };
        let callee = description
            .strip_prefix(TAIL_PREFIX)?
            .strip_suffix(TAIL_SUFFIX)?
            .to_owned();
        if !self.variadic_tails.contains(&callee) {
            return None;
        }
        let named = self.contracts.contract(&callee)?.parameters.len();
        let index = self.typed_positions_index().cloned()?;
        let wanted = texts(tokens);
        let node = Self::elaboration_node(&index, tokens.first()?.line, &wanted, &|node| {
            matches!(kind(node), "CallExpr" | "DeclStmt" | "BinaryOperator")
        })?;
        let call = statement_call(node, &callee)?;
        let promoted = children(call)
            .get(1 + named..)
            .is_some_and(|tail| tail.iter().all(crate::variadic_tail::promoted_scalar_or_pointer));
        let widened = if promoted {
            self.variadic_tail_widening.set(true);
            let widened = self.classify_simple(tokens);
            self.variadic_tail_widening.set(false);
            Some(widened)
        } else {
            None
        };
        let admitted = matches!(widened, Some(LegacyInstruction::ContractCall { .. }));
        // The T7 scalars, read from the statement's own tokens.
        let addresses = if admitted {
            super::find_call(&wanted)
                .and_then(|(call_index, _)| super::call_argument_spans(&wanted, call_index))
                .map(|spans| {
                    spans
                        .iter()
                        .skip(named)
                        .filter_map(|&(start, end)| {
                            match crate::variadic_tail::widened_actual(
                                &wanted[start..end],
                                &self.typed_types,
                                &|name| self.descriptor_names.contains(name),
                                &self.protected_formals,
                                self.variadic_borrowed_tails.contains(&callee),
                            ) {
                                Some(crate::variadic_tail::WidenedTail::ScalarAddress(name)) => {
                                    Some(name)
                                }
                                _ => None,
                            }
                        })
                        .collect::<Vec<_>>()
                })
        } else {
            None
        };
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": wanted,
            "position": "variadic-tail-statement",
            "callee": callee,
            "old_route": "old_refusal",
            "old_admitted": false,
            "promoted_scalar_or_pointer": promoted,
            "new_admitted": admitted && addresses.is_some(),
            "scalar_addresses": addresses,
            "disagreement": admitted && addresses.is_some(),
        }));
        let (Some(call), Some(addresses)) = (widened, addresses) else {
            return None;
        };
        let mut block = vec![Statement::Instruction(call, line)];
        block.extend(addresses.into_iter().map(|destination| {
            Statement::Instruction(LegacyInstruction::ForgetScalar { destination }, line)
        }));
        Some(Statement::Block(block))
    }

    /// Hook: a statement the classifier refused as a call to an uncontracted
    /// `va_*` builtin. `None` keeps the refusal byte for byte (no unique Clang
    /// node, or a shape this rule does not read).
    pub(super) fn va_list_statement(
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
        refused_va_builtin(description)?;
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
        let shape = shape(node)?;
        let form = match &shape {
            Shape::Neutral(builtin) => *builtin,
            Shape::Stored { .. } => ARG,
        };
        let outcome = va_list_protocol::check(&index.function)
            .map_err(|refusal| refusal.description())
            .and_then(|protocol| match &shape {
                Shape::Neutral(_) => Ok((protocol, LegacyInstruction::NoOp)),
                Shape::Stored {
                    destination,
                    scalar,
                } => {
                    if *scalar
                        && !self.protected_formals.contains(destination.as_str())
                        && !self.formal_parameters.contains(destination.as_str())
                        && crate::typed_field_access::is_scalar_initializer(
                            destination,
                            &["1"],
                            &self.typed_model,
                            &self.typed_types,
                        )
                    {
                        Ok((
                            protocol,
                            LegacyInstruction::ForgetScalar {
                                destination: destination.clone(),
                            },
                        ))
                    } else {
                        Err(format!(
                            "va_list operation refused: the `va_arg` value stored into \
                             `{destination}` is not an arithmetic value into a scalar local"
                        ))
                    }
                }
            });
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": wanted,
            "position": "va-list-statement",
            "form": form,
            "old_route": "old_refusal",
            "old_admitted": false,
            "new_admitted": outcome.is_ok(),
            "lists": outcome.as_ref().ok().map(|(protocol, _)| protocol
                .lists
                .iter()
                .map(|list| list.name.clone())
                .collect::<Vec<_>>()),
            "new_refusal": outcome.as_ref().err(),
            "disagreement": outcome.is_ok(),
        }));
        Some(match outcome {
            Ok((_, instruction)) => Statement::Instruction(instruction, line),
            Err(reason) => Statement::Instruction(
                LegacyInstruction::Unknown {
                    description: reason,
                    writes: writes.clone(),
                },
                line,
            ),
        })
    }
}
