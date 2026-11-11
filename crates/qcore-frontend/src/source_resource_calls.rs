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

//! Frontend for bounded direct calls over `std.buffer` byte storage.
//!
//! The spelling deliberately uses `bytes`, not task execution's `Buffer`:
//! `own<bytes>` carries the extent-aware `source.buffer.v1` value, while
//! `own<Buffer>` remains the task route's distinct fixed payload resource.

use std::collections::BTreeMap;

use qcore_core::resource_calls::{
    Argument, Function, Parameter, Program, ReturnValue, Statement, Type,
};
use qcore_core::std_buffer::{BufferEvent, BufferOperand};
use qcore_core::typed_errors::{Effect, EffectRow};
use qcore_core::usage::Usage;

use crate::frontend::{DeclarationKind, SemanticPass, SourceDiagnostic, SourceProgram};
use crate::lexer::{Span, Symbol, Token, TokenKind};

pub struct Lowered {
    pub program: Program,
    pub function_spans: Vec<Span>,
    pub statement_spans: Vec<Vec<Span>>,
}

fn word(token: Option<&Token>) -> Option<&str> {
    match token.map(|token| &token.kind) {
        Some(TokenKind::Word(word)) => Some(word),
        _ => None,
    }
}

fn symbol(token: Option<&Token>, expected: Symbol) -> bool {
    token.is_some_and(|token| token.kind == TokenKind::Symbol(expected))
}

fn diagnostic(
    code: &'static str,
    pass: SemanticPass,
    span: Span,
    message: impl Into<String>,
) -> SourceDiagnostic {
    SourceDiagnostic {
        code,
        pass,
        span,
        message: format!("source.resource-calls.v1: {}", message.into()),
    }
}

fn ordinary(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    diagnostic("Q020", SemanticPass::OrdinaryTypes, span, message)
}

fn buffer_shape(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    diagnostic("Q051", SemanticPass::StandardBuffer, span, message)
}

fn usage_diagnostic(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    diagnostic("Q100", SemanticPass::Usage, span, message)
}

fn tokens_for(program: &SourceProgram, span: Span) -> Vec<&Token> {
    program
        .lexical_tokens()
        .iter()
        .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
        .collect()
}

fn resource_type_tokens(tokens: &[&Token]) -> Option<Type> {
    match tokens {
        [head, less, bytes, greater]
            if matches!(&less.kind, TokenKind::Symbol(Symbol::Less))
                && matches!(&bytes.kind, TokenKind::Word(name) if name == "bytes")
                && matches!(&greater.kind, TokenKind::Symbol(Symbol::Greater)) =>
        {
            match word(Some(head)) {
                Some("own") => Some(Type::OwnedBytes),
                Some("borrow") => Some(Type::ReadBytes),
                Some("lend") => Some(Type::WriteBytes),
                _ => None,
            }
        }
        _ => None,
    }
}

fn parameter_type(tokens: &[&Token], usage: Option<Usage>) -> Option<Type> {
    match (tokens, resource_type_tokens(tokens), usage) {
        ([token], _, None) if word(Some(token)) == Some("i32") => Some(Type::I32),
        (_, Some(Type::OwnedBytes), Some(Usage::ONE)) => Some(Type::OwnedBytes),
        (_, Some(Type::ReadBytes), None) => Some(Type::ReadBytes),
        (_, Some(Type::WriteBytes), None) => Some(Type::WriteBytes),
        _ => None,
    }
}

fn result_type(tokens: &[&Token]) -> Option<Type> {
    match (tokens, resource_type_tokens(tokens)) {
        ([token], _) if word(Some(token)) == Some("unit") => Some(Type::Unit),
        ([token], _) if word(Some(token)) == Some("i32") => Some(Type::I32),
        (_, Some(Type::OwnedBytes)) => Some(Type::OwnedBytes),
        _ => None,
    }
}

fn declared_effects(tokens: &[&Token], span: Span) -> Result<Option<EffectRow>, SourceDiagnostic> {
    if tokens.is_empty() {
        return Ok(None);
    }
    if word(tokens.first().copied()) != Some("effects") {
        return Err(ordinary(
            span,
            "unsupported tokens follow the function result",
        ));
    }
    if !matches!(
        tokens.get(1).map(|token| &token.kind),
        Some(TokenKind::Symbol(Symbol::LeftBrace))
    ) {
        return Err(ordinary(span, "an effects row requires `{ ... }`"));
    }
    let mut effects = Vec::new();
    let mut cursor = 2;
    let mut need_effect = false;
    loop {
        match tokens.get(cursor).map(|token| &token.kind) {
            Some(TokenKind::Symbol(Symbol::RightBrace)) if !need_effect => {
                cursor += 1;
                break;
            }
            Some(TokenKind::Word(name)) => {
                effects.push(match name.as_str() {
                    "alloc" => Effect::Alloc,
                    "mut" => Effect::Mut,
                    _ => {
                        return Err(ordinary(
                            span,
                            format!("effect `{name}` is outside resource-calls.v1"),
                        ));
                    }
                });
                cursor += 1;
                if matches!(
                    tokens.get(cursor).map(|token| &token.kind),
                    Some(TokenKind::Symbol(Symbol::Comma))
                ) {
                    cursor += 1;
                    need_effect = true;
                } else if matches!(
                    tokens.get(cursor).map(|token| &token.kind),
                    Some(TokenKind::Symbol(Symbol::RightBrace))
                ) {
                    need_effect = false;
                } else {
                    return Err(ordinary(span, "effects are separated by commas"));
                }
            }
            _ => return Err(ordinary(span, "this effects row is not closed")),
        }
    }
    if cursor != tokens.len() {
        return Err(ordinary(span, "unsupported tokens follow the effects row"));
    }
    Ok(Some(EffectRow::new(effects)))
}

fn operand(tokens: &[&Token], at: usize) -> Option<(BufferOperand, usize)> {
    match tokens.get(at).map(|token| &token.kind) {
        Some(TokenKind::Integer(value)) => {
            Some((BufferOperand::Literal(value.parse().ok()?), at + 1))
        }
        Some(TokenKind::Word(name)) => Some((BufferOperand::Scalar(name.clone()), at + 1)),
        _ => None,
    }
}

fn parse_arguments(
    tokens: &[&Token],
    mut at: usize,
    span: Span,
) -> Result<(Vec<Argument>, usize), SourceDiagnostic> {
    let mut arguments = Vec::new();
    if symbol(tokens.get(at).copied(), Symbol::RightParen) {
        return Ok((arguments, at + 1));
    }
    loop {
        let argument = match word(tokens.get(at).copied()) {
            Some("move") => {
                let name = word(tokens.get(at + 1).copied())
                    .ok_or_else(|| ordinary(span, "`move` requires an owned bytes name"))?;
                at += 2;
                Argument::Move(name.to_owned())
            }
            Some("borrow") => {
                let name = word(tokens.get(at + 1).copied())
                    .ok_or_else(|| ordinary(span, "`borrow` requires an owned bytes name"))?;
                at += 2;
                Argument::Borrow(name.to_owned())
            }
            Some("lend") => {
                if !symbol(tokens.get(at + 1).copied(), Symbol::LeftParen) {
                    return Err(ordinary(span, "a write-loan argument is `lend(NAME)`"));
                }
                let name = word(tokens.get(at + 2).copied())
                    .ok_or_else(|| ordinary(span, "`lend(NAME)` requires an owned bytes name"))?;
                if !symbol(tokens.get(at + 3).copied(), Symbol::RightParen) {
                    return Err(ordinary(span, "`lend(NAME)` is not closed"));
                }
                at += 4;
                Argument::Lend(name.to_owned())
            }
            _ => {
                let (value, next) = operand(tokens, at)
                    .ok_or_else(|| ordinary(span, "unsupported helper argument"))?;
                at = next;
                Argument::Scalar(value)
            }
        };
        arguments.push(argument);
        if symbol(tokens.get(at).copied(), Symbol::Comma) {
            at += 1;
            continue;
        }
        if !symbol(tokens.get(at).copied(), Symbol::RightParen) {
            return Err(ordinary(span, "helper argument list is not closed"));
        }
        return Ok((arguments, at + 1));
    }
}

fn parse_call(
    tokens: &[&Token],
    span: Span,
    functions: &BTreeMap<String, usize>,
    shadowed: &BTreeMap<String, ()>,
) -> Result<Option<Statement>, SourceDiagnostic> {
    let (binding, name_at) = if word(tokens.first().copied()) == Some("let") {
        let Some(binding) = word(tokens.get(1).copied()) else {
            return Ok(None);
        };
        if !symbol(tokens.get(2).copied(), Symbol::Equal) {
            return Ok(None);
        }
        (Some(binding.to_owned()), 3)
    } else {
        (None, 0)
    };
    let Some(name) = word(tokens.get(name_at).copied()) else {
        return Ok(None);
    };
    if shadowed.contains_key(name) && functions.contains_key(name) {
        return Err(ordinary(
            span,
            "a local or parameter shadows this helper; indirect calls are unsupported",
        ));
    }
    let Some(&callee) = functions.get(name) else {
        return Ok(None);
    };
    if !symbol(tokens.get(name_at + 1).copied(), Symbol::LeftParen) {
        return Ok(None);
    }
    let (arguments, next) = parse_arguments(tokens, name_at + 2, span)?;
    if !symbol(tokens.get(next).copied(), Symbol::Semicolon) || next + 1 != tokens.len() {
        return Err(ordinary(span, "direct helper call is not closed"));
    }
    Ok(Some(Statement::Call {
        binding,
        callee,
        arguments,
    }))
}

fn parse_return(tokens: &[&Token], span: Span) -> Result<Option<Statement>, SourceDiagnostic> {
    if word(tokens.first().copied()) != Some("return") {
        return Ok(None);
    }
    let value = if symbol(tokens.get(1).copied(), Symbol::Semicolon) && tokens.len() == 2 {
        ReturnValue::Unit
    } else if word(tokens.get(1).copied()) == Some("move")
        && word(tokens.get(2).copied()).is_some()
        && symbol(tokens.get(3).copied(), Symbol::Semicolon)
        && tokens.len() == 4
    {
        ReturnValue::Move(word(tokens.get(2).copied()).unwrap().to_owned())
    } else if let Some((operand, next)) = operand(tokens, 1)
        && symbol(tokens.get(next).copied(), Symbol::Semicolon)
        && next + 1 == tokens.len()
    {
        ReturnValue::Scalar(operand)
    } else {
        return Err(ordinary(
            span,
            "return is bare, scalar, or `return move NAME;`",
        ));
    };
    Ok(Some(Statement::Return(value)))
}

pub fn has_resource_calls(program: &SourceProgram) -> bool {
    program.declarations.iter().any(|declaration| {
        declaration.value.kind == DeclarationKind::Function
            && (declaration.value.parameters.iter().any(|parameter| {
                resource_type_tokens(&tokens_for(program, parameter.type_span)).is_some()
            }) || declaration
                .value
                .result_type
                .as_ref()
                .is_some_and(|result| {
                    resource_type_tokens(&tokens_for(program, result.span))
                        == Some(Type::OwnedBytes)
                }))
    })
}

pub fn lower(program: &SourceProgram, entry: &str) -> Result<Lowered, Vec<SourceDiagnostic>> {
    let declarations = program
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::Function)
        .collect::<Vec<_>>();
    let functions = declarations
        .iter()
        .enumerate()
        .map(|(index, declaration)| (declaration.value.name.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let Some(&entry) = functions.get(entry) else {
        return Err(vec![ordinary(
            Span { start: 0, end: 0 },
            "selected entry is not a function",
        )]);
    };
    let mut lowered = Vec::new();
    let mut function_spans = Vec::new();
    let mut statement_spans = Vec::new();
    let mut diagnostics = Vec::new();
    for declaration in declarations {
        let scope = declaration.span;
        if let Some(contract) = declaration.value.function_contract.as_ref() {
            diagnostics.push(if contract.clauses.is_empty() {
                ordinary(scope, "contract blocks are outside resource-calls.v1")
            } else {
                diagnostic(
                    "Q021",
                    SemanticPass::OrdinaryTypes,
                    scope,
                    "source contracts have no resource-call consumer",
                )
            });
            continue;
        }
        let mut parameters = Vec::new();
        for parameter in &declaration.value.parameters {
            let usage = parameter.usage.as_ref().map(|usage| usage.value);
            let type_tokens = tokens_for(program, parameter.type_span);
            let Some(ty) = parameter_type(&type_tokens, usage) else {
                let grade_error = (type_tokens.len() == 1
                    && word(type_tokens.first().copied()) == Some("i32"))
                    || resource_type_tokens(&type_tokens).is_some();
                diagnostics.push(if grade_error {
                    usage_diagnostic(
                        parameter.type_span,
                        "own<bytes> requires grade one; i32 and byte loans are ungraded",
                    )
                } else {
                    ordinary(
                        parameter.type_span,
                        "parameters require i32, own<bytes> use one, borrow<bytes>, or lend<bytes>; task Buffer is a distinct resource",
                    )
                });
                continue;
            };
            parameters.push(Parameter {
                name: parameter.name.value.clone(),
                ty,
            });
        }
        let result_source = declaration.value.result_type.as_ref();
        let Some(result) = result_source
            .map(|result| tokens_for(program, result.span))
            .as_deref()
            .and_then(result_type)
        else {
            diagnostics.push(ordinary(scope, "result requires unit, i32, or own<bytes>"));
            continue;
        };
        let Some(body) = &declaration.value.body else {
            diagnostics.push(ordinary(scope, "resource helper requires a body"));
            continue;
        };
        let header = tokens_for(
            program,
            Span {
                start: declaration.span.start,
                end: body.span.start,
            },
        );
        let result_end = result_source.unwrap().span.end;
        let suffix = header
            .into_iter()
            .filter(|token| token.span.start >= result_end)
            .collect::<Vec<_>>();
        let effects = match declared_effects(&suffix, scope) {
            Ok(effects) => effects,
            Err(error) => {
                diagnostics.push(error);
                continue;
            }
        };
        let mut body_ir = Vec::new();
        let mut spans = Vec::new();
        let mut shadowed = parameters
            .iter()
            .map(|parameter| (parameter.name.clone(), ()))
            .collect::<BTreeMap<_, _>>();
        for source_statement in &body.statements {
            if !source_statement.value.nested_blocks().is_empty() {
                diagnostics.push(ordinary(
                    source_statement.span,
                    "resource-calls.v1 has no branches or loops",
                ));
                continue;
            }
            let tokens = tokens_for(program, source_statement.span);
            let parsed = match parse_return(&tokens, source_statement.span) {
                Ok(Some(statement)) => Ok(statement),
                Ok(None) => match parse_call(&tokens, source_statement.span, &functions, &shadowed)
                {
                    Ok(Some(statement)) => Ok(statement),
                    Ok(None) => match crate::source_std_buffer::buffer_statement(
                        program,
                        source_statement.span,
                    ) {
                        Some(Ok(event)) => Ok(Statement::Buffer(event)),
                        Some(Err(message)) => Err(buffer_shape(source_statement.span, message)),
                        None => Err(ordinary(
                            source_statement.span,
                            "unsupported resource helper statement",
                        )),
                    },
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            };
            match parsed {
                Ok(statement) => {
                    match &statement {
                        Statement::Buffer(BufferEvent::Bind { binding, .. })
                        | Statement::Buffer(BufferEvent::Acquire { name: binding, .. })
                        | Statement::Buffer(BufferEvent::Load { binding, .. })
                        | Statement::Buffer(BufferEvent::Extent { binding, .. }) => {
                            shadowed.insert(binding.clone(), ());
                        }
                        Statement::Call {
                            binding: Some(binding),
                            ..
                        } => {
                            shadowed.insert(binding.clone(), ());
                        }
                        _ => {}
                    }
                    body_ir.push(statement);
                    spans.push(source_statement.span);
                }
                Err(error) => diagnostics.push(error),
            }
        }
        lowered.push(Function {
            name: declaration.value.name.clone(),
            parameters,
            result,
            declared_effects: effects,
            statements: body_ir,
        });
        function_spans.push(scope);
        statement_spans.push(spans);
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    Ok(Lowered {
        program: Program {
            functions: lowered,
            entry,
        },
        function_spans,
        statement_spans,
    })
}
