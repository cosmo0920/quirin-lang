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

//! Source reading for `source.ffi.v1` (card `W48-QR-C-STRUCT-IMPORT`).
//!
//! ```text
//! module      ::= "#import std.ffi.unsafe;" (import)* (struct_import | extern-decl | function)*
//! function    ::= the entry, or a `.qr` helper (card W48-QR-USABLE-RECORDS):
//!                 "fn" NAME "(" [NAME ":" (SCALAR | TYPE) {"," …}] ")"
//!                 ["->" ("unit" | SCALAR | TYPE)] ["effects" "{" … "}"] "{" stmt* "}"
//! struct_import ::= "struct" "!" "{" "type" NAME "=" "c" STRING "from" STRING ";"
//!                   "fields" "{" field ("," field)* [","] "}" "}"
//! field       ::= NAME ":" SCALAR                      -- §G02t
//! stmt        ::= "let" NAME "=" TYPE "{" assign ("," assign)* [","] "}" ";"
//!               | "let" NAME "=" NAME "." NAME ";"
//!               | ["let" NAME "="] CALLEE "(" [arg ("," arg)*] ")" ["succeeds" outcome] ";"
//!               | <`print`, `log`, `flush` of §G02g>
//!               | NAME "." NAME "=" value ";"             -- a listed field (W48-QR-USABLE-RECORDS)
//!               | "let" NAME "=" NAME ";"                 -- a copy (W48-QR-USABLE-RECORDS)
//!               | "return" [value] ";"                    -- a value from a helper
//! assign      ::= NAME ":" value
//! value       ::= ["-"] INTEGER | "true" | "false" | NAME
//! outcome     ::= ["-"] INTEGER | "true" | "false" | TYPE "{" assign ("," assign)* [","] "}"
//! ```
//!
//! The `c"…"` spelling is the word `c` immediately followed by a string
//! literal, with no byte between them; its text is exactly `struct TAG`.
//! The layout is **not** read here: this module reads what the source wrote,
//! and the driver asks the recorded compiler for the rest.

use std::collections::BTreeMap;

use crate::frontend::{
    DeclarationKind, SemanticPass, SourceDeclaration, SourceDiagnostic, SourceProgram,
    TypedSourceProgram,
};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::ffi::{
    self, CheckedStruct, FfiCallee, FfiFunction, FfiOutcome, FfiProgram, FfiStatement, FfiType,
    FfiValue, ImportedStruct,
};
use qcore_core::module_imports::FFI_MODULE;
use qcore_core::scalar_types::{RecordedTarget, ScalarType};
use qcore_core::typed_errors::{Effect, EffectRow};

/// The at-most-32 listed fields of a record (§G07.13's bound, `Q053`).
pub const MAX_FIELDS: usize = 32;

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
        message: message.into(),
    }
}

/// The pass that owns a code this route emits (§L21): the route runs the
/// passes whose conditions it reaches, each under its own range.
#[must_use]
pub fn pass_of(code: &str) -> SemanticPass {
    match code {
        "Q022" | "Q023" | "Q024" | "Q025" | "Q026" | "Q027" => SemanticPass::ExternCall,
        "Q028" | "Q029" | "Q030" | "Q031" => SemanticPass::StandardIo,
        "Q032" | "Q033" | "Q034" | "Q035" => SemanticPass::UncheckedLinkage,
        "Q043" => SemanticPass::ComposedStdIo,
        "Q051" | "Q052" | "Q053" => SemanticPass::StandardBuffer,
        _ => SemanticPass::NativeLibrary,
    }
}

/// The span of the first `struct!` declaration, for the import gate.
#[must_use]
pub fn ffi_declaration_span(program: &SourceProgram) -> Option<Span> {
    program
        .declarations
        .iter()
        .find(|declaration| declaration.value.kind == DeclarationKind::ImportedStruct)
        .map(|declaration| declaration.span)
}

/// Whether this module is a `source.ffi.v1` module: it imports
/// `std.ffi.unsafe` or declares a `struct!`. Route selection consults this
/// before every other route, so an FFI file reaches no other route and no
/// other file reaches this one.
#[must_use]
pub fn has_source_ffi(program: &SourceProgram) -> bool {
    program
        .imports()
        .iter()
        .any(|import| import.value == FFI_MODULE)
        || ffi_declaration_span(program).is_some()
}

fn window(program: &SourceProgram, span: Span) -> Vec<&Token> {
    program
        .lexical_tokens()
        .iter()
        .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
        .filter(|token| token.span.start < token.span.end)
        .collect()
}

fn word<'a>(tokens: &'a [&Token], index: usize) -> Option<&'a str> {
    match tokens.get(index).map(|token| &token.kind) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    }
}

fn symbol(tokens: &[&Token], index: usize, expected: Symbol) -> bool {
    matches!(tokens.get(index).map(|token| &token.kind), Some(TokenKind::Symbol(found)) if *found == expected)
}

fn string<'a>(tokens: &'a [&Token], index: usize) -> Option<&'a str> {
    match tokens.get(index).map(|token| &token.kind) {
        Some(TokenKind::StringLiteral(text)) => Some(text.as_str()),
        _ => None,
    }
}

/// `c"struct TAG"` at `index`: the tag, when the word `c` touches the string.
fn c_struct_at(tokens: &[&Token], index: usize) -> Result<Option<String>, &'static str> {
    if word(tokens, index) != Some("c") {
        return Ok(None);
    }
    let Some(text) = string(tokens, index + 1) else {
        return Ok(None);
    };
    if tokens[index].span.end != tokens[index + 1].span.start {
        return Err("`c\"…\"` is one spelling: no space between `c` and the quote");
    }
    ffi::parse_c_struct_spelling(text)
        .map(|tag| Some(tag.to_owned()))
        .ok_or("`c\"…\"` names exactly `struct TAG`: a union, an enum, a typedef name or any other spelling is not an importable struct")
}

/// Read every `struct!` declaration of the program.
///
/// # Errors
///
/// Every malformed declaration: `Q051` for the shape, `Q053` for a field
/// count outside `1..=32`, `Q063` for a C spelling or header reference this
/// route cannot resolve.
pub fn imported_structs(
    program: &SourceProgram,
) -> Result<Vec<(ImportedStruct, Span)>, Vec<SourceDiagnostic>> {
    let mut structs = Vec::new();
    let mut errors = Vec::new();
    for declaration in &program.declarations {
        if declaration.value.kind != DeclarationKind::ImportedStruct {
            continue;
        }
        match imported_struct(program, declaration.span) {
            Ok(imported) => structs.push((imported, declaration.span)),
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(structs)
    } else {
        Err(errors)
    }
}

fn imported_struct(
    program: &SourceProgram,
    span: Span,
) -> Result<ImportedStruct, SourceDiagnostic> {
    let tokens = window(program, span);
    let shape = |message: &str| {
        diagnostic(
            "Q051",
            SemanticPass::StandardBuffer,
            span,
            format!(
                "{message}; an imported struct is `struct! {{ type NAME = c\"struct TAG\" from \"HEADER\"; fields {{ FIELD: TYPE, … }} }}`"
            ),
        )
    };
    // struct ! { type NAME = c "…" from "…" ; fields { … } }
    if word(&tokens, 0) != Some("struct")
        || !symbol(&tokens, 1, Symbol::Bang)
        || !symbol(&tokens, 2, Symbol::LeftBrace)
        || word(&tokens, 3) != Some("type")
    {
        return Err(shape("malformed head"));
    }
    let name = word(&tokens, 4).ok_or_else(|| shape("the `.qr` type name is missing"))?;
    if !symbol(&tokens, 5, Symbol::Equal) {
        return Err(shape("`=` follows the type name"));
    }
    let tag = match c_struct_at(&tokens, 6) {
        Ok(Some(tag)) => tag,
        Ok(None) => return Err(shape("the C type is spelled `c\"struct TAG\"`")),
        Err(message) => {
            return Err(diagnostic(
                "Q063",
                SemanticPass::NativeLibrary,
                span,
                message,
            ));
        }
    };
    if word(&tokens, 8) != Some("from") {
        return Err(shape(
            "`from \"HEADER\"` names the header the struct is declared in",
        ));
    }
    let header = string(&tokens, 9).ok_or_else(|| shape("the header is a quoted reference"))?;
    if header.is_empty()
        || header
            .chars()
            .any(|ch| ch == '\\' || ch == '"' || ch.is_control())
    {
        return Err(diagnostic(
            "Q063",
            SemanticPass::NativeLibrary,
            span,
            format!(
                "the header reference `{header}` is not a path this route can resolve: it is nonempty, with no escape, quote or control character"
            ),
        ));
    }
    if !symbol(&tokens, 10, Symbol::Semicolon)
        || word(&tokens, 11) != Some("fields")
        || !symbol(&tokens, 12, Symbol::LeftBrace)
    {
        return Err(shape("`; fields { … }` follows the header"));
    }
    let mut cursor = 13;
    let mut fields: Vec<(String, ScalarType)> = Vec::new();
    loop {
        if symbol(&tokens, cursor, Symbol::RightBrace) {
            cursor += 1;
            break;
        }
        let Some(field) = word(&tokens, cursor) else {
            return Err(shape("a field is `NAME: TYPE`"));
        };
        if !symbol(&tokens, cursor + 1, Symbol::Colon) {
            return Err(shape("a field is `NAME: TYPE`"));
        }
        // Card `W48-QR-FLOATING-POINT`: `f32`/`f64` fields, a C `float`/`double`.
        let Some(ty) = word(&tokens, cursor + 2).and_then(ScalarType::parse_admitting_floats)
        else {
            return Err(diagnostic(
                "Q051",
                SemanticPass::StandardBuffer,
                span,
                format!(
                    "field `{field}` of `{name}` is not a §G02t type (bool, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64); a C member of any other type stays unlisted, as opaque bytes"
                ),
            ));
        };
        if fields.iter().any(|(existing, _)| existing == field) {
            return Err(diagnostic(
                "Q051",
                SemanticPass::StandardBuffer,
                span,
                format!("field `{field}` of `{name}` is listed twice"),
            ));
        }
        fields.push((field.to_owned(), ty));
        cursor += 3;
        if symbol(&tokens, cursor, Symbol::Comma) {
            cursor += 1;
        } else if !symbol(&tokens, cursor, Symbol::RightBrace) {
            return Err(shape("fields are separated by `,`"));
        }
    }
    if !symbol(&tokens, cursor, Symbol::RightBrace) || cursor + 1 != tokens.len() {
        return Err(shape("the declaration ends after the field list"));
    }
    if fields.is_empty() || fields.len() > MAX_FIELDS {
        return Err(diagnostic(
            "Q053",
            SemanticPass::StandardBuffer,
            span,
            format!(
                "`{name}` lists {} field(s); an imported struct lists 1 to {MAX_FIELDS}",
                fields.len()
            ),
        ));
    }
    Ok(ImportedStruct {
        name: name.to_owned(),
        tag,
        header: header.to_owned(),
        fields,
    })
}

/// Read a `.qr` extern declaration's type against the imported struct names.
#[must_use]
pub fn ffi_type(source: &str, records: &[String]) -> Option<FfiType> {
    let tokens = crate::lexer::lex(source).ok()?;
    let kinds: Vec<&TokenKind> = tokens
        .iter()
        .map(|token| &token.kind)
        .filter(|kind| **kind != TokenKind::Eof)
        .collect();
    let word = |index: usize| match kinds.get(index) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    };
    let sym = |index: usize, expected: Symbol| matches!(kinds.get(index), Some(TokenKind::Symbol(found)) if *found == expected);
    match kinds.len() {
        1 => {
            let name = word(0)?;
            if name == "unit" {
                Some(FfiType::Unit)
            } else if let Some(scalar) = ScalarType::parse_admitting_floats(name) {
                Some(FfiType::Scalar(scalar))
            } else {
                records
                    .iter()
                    .any(|record| record == name)
                    .then(|| FfiType::Record(name.to_owned()))
            }
        }
        6 if word(0) == Some("ptr")
            && sym(1, Symbol::Less)
            && word(2) == Some("const")
            && sym(3, Symbol::Comma)
            && sym(5, Symbol::Greater) =>
        {
            let name = word(4)?;
            records
                .iter()
                .any(|record| record == name)
                .then(|| FfiType::ConstRecordPointer(name.to_owned()))
        }
        _ => None,
    }
}

/// One lowered entry, with the span of each statement for attribution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredFfiModule {
    pub program: FfiProgram,
    pub statement_spans: Vec<Span>,
    pub scope_span: Span,
}

/// The declared `effects { … }` row, or `None` when the declaration writes
/// none (card `W48-QR-USABLE-RECORDS`: a helper then infers its row; the
/// entry reads `None` as the empty row it always read).
fn declared_effects(
    declaration: &SourceDeclaration,
    span: Span,
) -> Result<Option<EffectRow>, SourceDiagnostic> {
    let tokens = &declaration.tokens;
    let Some(index) = tokens
        .iter()
        .position(|token| matches!(token, TokenKind::Word(word) if word == "effects"))
    else {
        return Ok(None);
    };
    if !matches!(
        tokens.get(index + 1),
        Some(TokenKind::Symbol(Symbol::LeftBrace))
    ) {
        return Err(diagnostic(
            "Q026",
            SemanticPass::ExternCall,
            span,
            "an `effects` row requires a block",
        ));
    }
    let mut effects = Vec::new();
    let mut cursor = index + 2;
    loop {
        match tokens.get(cursor) {
            Some(TokenKind::Symbol(Symbol::RightBrace)) => break,
            Some(TokenKind::Word(name)) => {
                effects.push(match name.as_str() {
                    "io" => Effect::Io,
                    "alloc" => Effect::Alloc,
                    "mut" => Effect::Mut,
                    other => {
                        return Err(diagnostic(
                            "Q026",
                            SemanticPass::ExternCall,
                            span,
                            format!("`{other}` is not an effect `source.ffi.v1` models; use io, alloc or mut"),
                        ));
                    }
                });
                cursor += 1;
                if matches!(tokens.get(cursor), Some(TokenKind::Symbol(Symbol::Comma))) {
                    cursor += 1;
                }
            }
            _ => {
                return Err(diagnostic(
                    "Q026",
                    SemanticPass::ExternCall,
                    span,
                    "malformed `effects` row",
                ));
            }
        }
    }
    Ok(Some(EffectRow::new(effects)))
}

fn value(tokens: &[&Token], cursor: usize) -> Option<(FfiValue, usize)> {
    match tokens.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Symbol(Symbol::Minus)) => {
            match tokens.get(cursor + 1).map(|token| &token.kind) {
                Some(TokenKind::Integer(digits)) => {
                    Some((FfiValue::Literal(-digits.parse::<i128>().ok()?), cursor + 2))
                }
                // Card `W48-QR-FLOATING-POINT`: `-FLOAT`.
                Some(TokenKind::Float(text)) => Some((
                    FfiValue::Float(qcore_core::scalar_floats::FloatLiteral::read(text, true)?),
                    cursor + 2,
                )),
                _ => None,
            }
        }
        Some(TokenKind::Integer(digits)) => {
            Some((FfiValue::Literal(digits.parse().ok()?), cursor + 1))
        }
        Some(TokenKind::Float(text)) => Some((
            FfiValue::Float(qcore_core::scalar_floats::FloatLiteral::read(text, false)?),
            cursor + 1,
        )),
        Some(TokenKind::Word(word)) if word == "true" => Some((FfiValue::Bool(true), cursor + 1)),
        Some(TokenKind::Word(word)) if word == "false" => Some((FfiValue::Bool(false), cursor + 1)),
        Some(TokenKind::Word(word)) => Some((FfiValue::Name(word.clone()), cursor + 1)),
        _ => None,
    }
}

/// `{ NAME: value, … }` from the `{` at `cursor`; the index after the `}`.
fn assignments(tokens: &[&Token], cursor: usize) -> Option<(Vec<(String, FfiValue)>, usize)> {
    if !symbol(tokens, cursor, Symbol::LeftBrace) {
        return None;
    }
    let mut cursor = cursor + 1;
    let mut fields = Vec::new();
    loop {
        if symbol(tokens, cursor, Symbol::RightBrace) {
            return Some((fields, cursor + 1));
        }
        let name = word(tokens, cursor)?;
        if !symbol(tokens, cursor + 1, Symbol::Colon) {
            return None;
        }
        let (operand, next) = value(tokens, cursor + 2)?;
        fields.push((name.to_owned(), operand));
        cursor = next;
        if symbol(tokens, cursor, Symbol::Comma) {
            cursor += 1;
        } else if !symbol(tokens, cursor, Symbol::RightBrace) {
            return None;
        }
    }
}

/// `CALLEE(args) [succeeds outcome] ;` from `cursor`.
fn call(
    tokens: &[&Token],
    cursor: usize,
    records: &[String],
) -> Option<(String, Vec<FfiValue>, Option<FfiOutcome>)> {
    let callee = word(tokens, cursor)?;
    if !symbol(tokens, cursor + 1, Symbol::LeftParen) {
        return None;
    }
    let mut at = cursor + 2;
    let mut arguments = Vec::new();
    if !symbol(tokens, at, Symbol::RightParen) {
        loop {
            let (argument, next) = value(tokens, at)?;
            arguments.push(argument);
            at = next;
            if symbol(tokens, at, Symbol::Comma) {
                at += 1;
            } else {
                break;
            }
        }
    }
    if !symbol(tokens, at, Symbol::RightParen) {
        return None;
    }
    at += 1;
    let mut outcome = None;
    if word(tokens, at) == Some("succeeds") {
        at += 1;
        if let Some(record) =
            word(tokens, at).filter(|name| records.iter().any(|record| record == name))
        {
            let (fields, next) = assignments(tokens, at + 1)?;
            outcome = Some(FfiOutcome::Record {
                record: record.to_owned(),
                fields,
            });
            at = next;
        } else {
            let (declared, next) = value(tokens, at)?;
            if matches!(declared, FfiValue::Name(_)) {
                return None;
            }
            outcome = Some(FfiOutcome::Scalar(declared));
            at = next;
        }
    }
    (symbol(tokens, at, Symbol::Semicolon) && at + 1 == tokens.len())
        .then(|| (callee.to_owned(), arguments, outcome))
}

/// Lower the entry `entry` of an FFI module into its program.
///
/// Card `W48-QR-USABLE-RECORDS`: every other function of the module is a
/// `.qr` **helper** — parameters and a result of §G02t scalars or imported
/// records by value, an optional effect row, and a body of this route's own
/// statements — lowered here and checked at every call by the model.
///
/// # Errors
///
/// Every statement this route does not model (`Q026`), and every entry shape
/// it does not model: parameters, a result other than `unit`, or a
/// declaration the route does not read.
pub fn lower_source_ffi(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
    structs: Vec<CheckedStruct>,
    callees: BTreeMap<String, FfiCallee>,
    target: &RecordedTarget,
) -> Result<LoweredFfiModule, Vec<SourceDiagnostic>> {
    let _ = typed;
    let context =
        |span: Span, message: String| diagnostic("Q026", SemanticPass::ExternCall, span, message);
    let mut errors = Vec::new();
    for declaration in &parsed.declarations {
        let admitted = matches!(
            declaration.value.kind,
            DeclarationKind::ImportedStruct
                | DeclarationKind::ExternFunction
                | DeclarationKind::Function
        );
        if !admitted {
            errors.push(context(
                declaration.span,
                format!(
                    "`{}` is a declaration `source.ffi.v1` does not model; an FFI file declares imported structs, extern functions, `.qr` functions and one entry",
                    declaration.value.name
                ),
            ));
        }
    }
    let Some(declaration) = parsed.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
    }) else {
        return Err(vec![context(
            Span { start: 0, end: 0 },
            format!("`{entry}` is not a function declaration in this module"),
        )]);
    };
    let scope_span = declaration.span;
    if !declaration.value.parameters.is_empty()
        || declaration
            .value
            .result_type
            .as_ref()
            .is_none_or(|result| result.source.trim() != "unit")
    {
        errors.push(context(
            scope_span,
            format!("the entry `{entry}` takes no parameter and returns `unit` on `source.ffi.v1`"),
        ));
    }
    let entry_effects = match declared_effects(&declaration.value, scope_span) {
        Ok(row) => row.unwrap_or_default(),
        Err(error) => {
            errors.push(error);
            EffectRow::default()
        }
    };
    let records: Vec<String> = structs.iter().map(|record| record.name.clone()).collect();
    let mut statement_spans = Vec::new();
    let statements = lower_body(
        parsed,
        declaration,
        &records,
        &mut statement_spans,
        &mut errors,
    );
    // Card `W48-QR-USABLE-RECORDS`: the helpers, in declaration order, each
    // numbered after the statements lowered before it.
    let mut functions = Vec::new();
    for helper in &parsed.declarations {
        if helper.value.kind != DeclarationKind::Function || helper.value.name == entry {
            continue;
        }
        let name = helper.value.name.clone();
        let mut parameters = Vec::new();
        for parameter in &helper.value.parameters {
            match ffi_type(&parameter.type_source, &records) {
                Some(ty @ (FfiType::Scalar(_) | FfiType::Record(_))) => {
                    parameters.push((parameter.name.value.clone(), ty));
                }
                _ => errors.push(context(
                    helper.span,
                    format!(
                        "parameter `{}` of `{name}` is `{}`; a `.qr` function on `source.ffi.v1` takes §G02t scalars and imported records by value",
                        parameter.name.value,
                        parameter.type_source.trim()
                    ),
                )),
            }
        }
        let result = match helper
            .value
            .result_type
            .as_ref()
            .map_or(Some(FfiType::Unit), |result| {
                ffi_type(&result.source, &records)
            }) {
            Some(ty @ (FfiType::Unit | FfiType::Scalar(_) | FfiType::Record(_))) => ty,
            _ => {
                errors.push(context(
                    helper.span,
                    format!(
                        "`{name}` returns a type `source.ffi.v1` does not model; a `.qr` function returns `unit`, a §G02t scalar or an imported record by value"
                    ),
                ));
                FfiType::Unit
            }
        };
        let declared = match declared_effects(&helper.value, helper.span) {
            Ok(row) => row,
            Err(error) => {
                errors.push(error);
                None
            }
        };
        let base = statement_spans.len();
        let statements = lower_body(parsed, helper, &records, &mut statement_spans, &mut errors);
        if statements.is_empty() {
            statement_spans.push(helper.span);
        }
        functions.push(FfiFunction {
            name,
            parameters,
            result,
            statements,
            declared_effects: declared,
            base,
        });
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(LoweredFfiModule {
        program: FfiProgram {
            entry: entry.to_owned(),
            structs,
            callees,
            statements,
            declared_effects: entry_effects,
            target: target.clone(),
            functions,
            // Card `W48-QRLIB-LINKAGE`: the driver resolves each callee's
            // generated manifest and fills this in before the check.
            links: std::collections::BTreeMap::new(),
        },
        statement_spans,
        scope_span,
    })
}

/// Lower one function body's statements, pushing each one's span.
fn lower_body(
    parsed: &SourceProgram,
    declaration: &crate::frontend::Located<SourceDeclaration>,
    records: &[String],
    statement_spans: &mut Vec<Span>,
    errors: &mut Vec<SourceDiagnostic>,
) -> Vec<FfiStatement> {
    let context =
        |span: Span, message: String| diagnostic("Q026", SemanticPass::ExternCall, span, message);
    let mut statements = Vec::new();
    let body = declaration
        .value
        .body
        .as_ref()
        .map(|body| body.statements.as_slice())
        .unwrap_or(&[]);
    for statement in body {
        let tokens = window(parsed, statement.span);
        match lower_statement(&tokens, records) {
            Ok(lowered) => {
                statements.push(lowered);
                statement_spans.push(statement.span);
            }
            Err(message) => errors.push(context(
                statement.span,
                format!(
                    "{message}; `source.ffi.v1` admits constructions, field reads and writes, copies, calls, `print`/`log`/`flush` and `return`"
                ),
            )),
        }
    }
    statements
}

/// Lower one statement's tokens.
fn lower_statement(tokens: &[&Token], records: &[String]) -> Result<FfiStatement, String> {
    if let Some(event) = crate::source_std_io::std_io_statement_tokens(tokens) {
        return event.map(FfiStatement::Line).map_err(str::to_owned);
    }
    if word(tokens, 0) == Some("return") {
        if symbol(tokens, 1, Symbol::Semicolon) && tokens.len() == 2 {
            return Ok(FfiStatement::Return(None));
        }
        // Card `W48-QR-USABLE-RECORDS`: a helper's `return VALUE;`.
        return match value(tokens, 1) {
            Some((returned, next))
                if symbol(tokens, next, Symbol::Semicolon) && next + 1 == tokens.len() =>
            {
                Ok(FfiStatement::Return(Some(returned)))
            }
            _ => Err("`return;` or `return VALUE;` with one literal or name".into()),
        };
    }
    if word(tokens, 0) == Some("let") {
        let binding = word(tokens, 1).ok_or("`let` binds a name")?.to_owned();
        if !symbol(tokens, 2, Symbol::Equal) {
            return Err("`let NAME = …;`".into());
        }
        // A construction.
        if let Some(record) =
            word(tokens, 3).filter(|name| records.iter().any(|record| record == name))
            && symbol(tokens, 4, Symbol::LeftBrace)
        {
            let (fields, next) =
                assignments(tokens, 4).ok_or("a construction is `TYPE { FIELD: VALUE, … }`")?;
            if !(symbol(tokens, next, Symbol::Semicolon) && next + 1 == tokens.len()) {
                return Err("a construction ends with `;`".into());
            }
            return Ok(FfiStatement::Construct {
                binding,
                record: record.to_owned(),
                fields,
            });
        }
        // A field read.
        if symbol(tokens, 4, Symbol::Dot) {
            let source = word(tokens, 3).ok_or("a field read is `RECORD.FIELD`")?;
            let field = word(tokens, 5).ok_or("a field read is `RECORD.FIELD`")?;
            if !(symbol(tokens, 6, Symbol::Semicolon) && tokens.len() == 7) {
                return Err("a field read is `let NAME = RECORD.FIELD;`".into());
            }
            return Ok(FfiStatement::Read {
                binding,
                source: source.to_owned(),
                field: field.to_owned(),
            });
        }
        // Card `W48-QR-USABLE-RECORDS`: a copy, `let NAME = SOURCE;`.
        if let Some(source) = word(tokens, 3)
            && symbol(tokens, 4, Symbol::Semicolon)
            && tokens.len() == 5
            && !matches!(source, "true" | "false")
        {
            return Ok(FfiStatement::Copy {
                binding,
                source: source.to_owned(),
            });
        }
        let (callee, arguments, outcome) = call(tokens, 3, records).ok_or(
            "a binding on `source.ffi.v1` is a construction, a field read, a copy or a call",
        )?;
        return Ok(FfiStatement::Call {
            binding: Some(binding),
            callee,
            arguments,
            outcome,
        });
    }
    // Card `W48-QR-USABLE-RECORDS`: the field write, `RECORD.FIELD = VALUE;`.
    if let Some(record) = word(tokens, 0)
        && symbol(tokens, 1, Symbol::Dot)
        && symbol(tokens, 3, Symbol::Equal)
    {
        let field = word(tokens, 2).ok_or("a field write is `RECORD.FIELD = VALUE;`")?;
        return match value(tokens, 4) {
            Some((written, next))
                if symbol(tokens, next, Symbol::Semicolon) && next + 1 == tokens.len() =>
            {
                Ok(FfiStatement::Write {
                    record: record.to_owned(),
                    field: field.to_owned(),
                    value: written,
                })
            }
            _ => Err("a field write is `RECORD.FIELD = VALUE;` with one literal or name".into()),
        };
    }
    let (callee, arguments, outcome) =
        call(tokens, 0, records).ok_or("a statement `source.ffi.v1` does not model")?;
    Ok(FfiStatement::Call {
        binding: None,
        callee,
        arguments,
        outcome,
    })
}
