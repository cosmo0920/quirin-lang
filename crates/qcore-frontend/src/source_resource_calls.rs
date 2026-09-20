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

fn diagnostic(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code: "Q051",
        pass: SemanticPass::StandardBuffer,
        span,
        message: format!("source.resource-calls.v1: {}", message.into()),
    }
}

fn tokens_for(program: &SourceProgram, span: Span) -> Vec<&Token> {
    program
        .lexical_tokens()
        .iter()
        .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
        .collect()
}

fn parameter_type(source: &str, usage: Option<Usage>) -> Option<Type> {
    let compact = source
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    match (compact.as_str(), usage) {
        ("i32", None) => Some(Type::I32),
        ("own<bytes>", Some(Usage::ONE)) => Some(Type::OwnedBytes),
        ("borrow<bytes>", None) => Some(Type::ReadBytes),
        ("lend<bytes>", None) => Some(Type::WriteBytes),
        _ => None,
    }
}

fn result_type(source: &str) -> Option<Type> {
    let compact = source
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    match compact.as_str() {
        "unit" => Some(Type::Unit),
        "i32" => Some(Type::I32),
        "own<bytes>" => Some(Type::OwnedBytes),
        _ => None,
    }
}

fn declared_effects(tokens: &[&Token], span: Span) -> Result<Option<EffectRow>, SourceDiagnostic> {
    let Some(index) = tokens
        .iter()
        .position(|token| matches!(&token.kind, TokenKind::Word(word) if word == "effects"))
    else {
        return Ok(None);
    };
    if !matches!(
        tokens.get(index + 1).map(|token| &token.kind),
        Some(TokenKind::Symbol(Symbol::LeftBrace))
    ) {
        return Err(diagnostic(span, "an effects row requires `{ ... }`"));
    }
    let mut effects = Vec::new();
    let mut cursor = index + 2;
    loop {
        match tokens.get(cursor).map(|token| &token.kind) {
            Some(TokenKind::Symbol(Symbol::RightBrace)) => break,
            Some(TokenKind::Word(name)) => {
                effects.push(match name.as_str() {
                    "alloc" => Effect::Alloc,
                    "mut" => Effect::Mut,
                    _ => {
                        return Err(diagnostic(
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
                }
            }
            _ => return Err(diagnostic(span, "this effects row is not closed")),
        }
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
                    .ok_or_else(|| diagnostic(span, "`move` requires an owned bytes name"))?;
                at += 2;
                Argument::Move(name.to_owned())
            }
            Some("borrow") => {
                let name = word(tokens.get(at + 1).copied())
                    .ok_or_else(|| diagnostic(span, "`borrow` requires an owned bytes name"))?;
                at += 2;
                Argument::Borrow(name.to_owned())
            }
            Some("lend") => {
                if !symbol(tokens.get(at + 1).copied(), Symbol::LeftParen) {
                    return Err(diagnostic(span, "a write-loan argument is `lend(NAME)`"));
                }
                let name = word(tokens.get(at + 2).copied())
                    .ok_or_else(|| diagnostic(span, "`lend(NAME)` requires an owned bytes name"))?;
                if !symbol(tokens.get(at + 3).copied(), Symbol::RightParen) {
                    return Err(diagnostic(span, "`lend(NAME)` is not closed"));
                }
                at += 4;
                Argument::Lend(name.to_owned())
            }
            _ => {
                let (value, next) = operand(tokens, at)
                    .ok_or_else(|| diagnostic(span, "unsupported helper argument"))?;
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
            return Err(diagnostic(span, "helper argument list is not closed"));
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
        return Err(diagnostic(
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
        return Err(diagnostic(span, "direct helper call is not closed"));
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
        return Err(diagnostic(
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
                parameter
                    .type_source
                    .chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect::<String>()
                    .ends_with("<bytes>")
            }) || declaration
                .value
                .result_type
                .as_ref()
                .is_some_and(|result| {
                    result
                        .source
                        .chars()
                        .filter(|ch| !ch.is_whitespace())
                        .eq("own<bytes>".chars())
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
        return Err(vec![diagnostic(
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
        if declaration
            .value
            .function_contract
            .as_ref()
            .is_some_and(|contract| !contract.clauses.is_empty())
        {
            diagnostics.push(diagnostic(
                scope,
                "source contracts have no resource-call consumer",
            ));
            continue;
        }
        let mut parameters = Vec::new();
        for parameter in &declaration.value.parameters {
            let usage = parameter.usage.as_ref().map(|usage| usage.value);
            let Some(ty) = parameter_type(&parameter.type_source, usage) else {
                diagnostics.push(diagnostic(
                    parameter.type_span,
                    "parameters require i32, own<bytes> use one, borrow<bytes>, or lend<bytes>; task Buffer is a distinct resource",
                ));
                continue;
            };
            parameters.push(Parameter {
                name: parameter.name.value.clone(),
                ty,
            });
        }
        let Some(result) = declaration
            .value
            .result_type
            .as_ref()
            .and_then(|result| result_type(&result.source))
        else {
            diagnostics.push(diagnostic(
                scope,
                "result requires unit, i32, or own<bytes>",
            ));
            continue;
        };
        let Some(body) = &declaration.value.body else {
            diagnostics.push(diagnostic(scope, "resource helper requires a body"));
            continue;
        };
        let header = tokens_for(
            program,
            Span {
                start: declaration.span.start,
                end: body.span.start,
            },
        );
        let effects = match declared_effects(&header, scope) {
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
                diagnostics.push(diagnostic(
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
                        Some(Err(message)) => Err(diagnostic(source_statement.span, message)),
                        None => Err(diagnostic(
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
