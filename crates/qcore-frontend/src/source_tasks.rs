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

//! Exact source lowering for bounded copy-scalar and owned-buffer scoped tasks.
//!
//! Structural task records are not admitted on their own: this pass consumes
//! complete statements, checks every declaration and resolves immutable slots.
//! The returned core IR still requires independent `check_task_module` validation.
use std::collections::{BTreeMap, BTreeSet};

use crate::frontend::*;
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::composed_tasks::TaskIoSite;

// Card `W48-QR-TASK-RECORD-CAPTURES`: records on this route.
mod records;
use records::{PathTarget, RecordState, RecordValue, Records};

/// Detect source task forms in every definition, including empty/nested scopes
/// and uncalled bodies. This selects a checker, never grants execution support.
pub fn has_source_tasks(source: &SourceProgram) -> bool {
    fn has_scope(block: &SourceBlock) -> bool {
        block.statements.iter().any(|s| {
            matches!(s.value, StatementKind::ThreadScope { .. })
                || s.value.nested_blocks().iter().any(|b| has_scope(b))
        })
    }
    // The `in_library_binding` exclusion is card
    // `NATIVE-LIBRARY-SOURCE-BINDING`'s constraint-14 census outcome: this is
    // one of the two whole-token-stream route selectors, so a library named
    // `qcore_buffer_new` would otherwise select the task route from its own
    // spelling in an `extern library NAME;` declaration.
    //
    // Card `QR-LEXICAL-SELECTORS-DECLARATION-NAMES`: and never as a
    // declaration's own name or a parameter's name (an
    // `extern fn qcore_buffer_new(…)` selected this route before).
    let scope = crate::route_selection::SelectorScope::of(source);
    source.lexical_tokens().iter().any(|token| !source.in_library_binding(token.span) && scope.reads(source, token.span) && matches!(&token.kind, TokenKind::Word(word) if word == "qcore_buffer_new" || word == "qcore_buffer_read" || word == "qcore_buffer_write"))
        || !source.task_expressions().is_empty()
        || source
            .declarations
            .iter()
            .filter_map(|d| d.value.body.as_ref())
            .any(has_scope)
}

fn failure(span: Span, pass: SemanticPass, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code: match pass {
            SemanticPass::NameResolution => "Q013",
            SemanticPass::OrdinaryTypes => "Q020",
            SemanticPass::TypedErrors => "Q300",
            SemanticPass::Concurrency => "Q600",
            SemanticPass::ComposedTasks => "Q050",
            _ => "Q500",
        },
        pass,
        span,
        message: message.into(),
    }
}

/// The tokens of a byte window. Card `W48-QR-CLOSURES`: a closure's header
/// and body are skipped by a window that contains the closure without lying
/// inside it — they are the hidden worker's — so the statement that spawns a
/// closure reads `spawn SCOPE |CAPTURES|`, its hand-written twin's shape.
fn tokens_for(source: &SourceProgram, span: Span) -> Vec<Token> {
    source
        .lexical_tokens()
        .iter()
        .filter(|token| {
            token.span.start >= span.start
                && token.span.end <= span.end
                && !source.outside_closure_window(token.span, span)
        })
        .cloned()
        .collect()
}

/// The top-level comma positions of a call's argument list, which is what
/// selects between an arity this route already had and one card
/// `WRITE-LEND-TASKS-EXERCISE` adds. Nesting is tracked so a comma inside a
/// parenthesized operand cannot be mistaken for an argument separator.
fn argument_commas(tokens: &[Token]) -> Vec<usize> {
    let Some(close) = matching(tokens, 1, Symbol::LeftParen, Symbol::RightParen) else {
        return Vec::new();
    };
    let mut depth = 0usize;
    let mut commas = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(close).skip(2) {
        if symbol(Some(token), Symbol::LeftParen) {
            depth += 1;
        } else if symbol(Some(token), Symbol::RightParen) {
            depth = depth.saturating_sub(1);
        } else if depth == 0 && symbol(Some(token), Symbol::Comma) {
            commas.push(index);
        }
    }
    commas
}

fn word(token: Option<&Token>, expected: &str) -> bool {
    matches!(token.map(|t| &t.kind), Some(TokenKind::Word(w)) if w == expected)
}

fn symbol(token: Option<&Token>, expected: Symbol) -> bool {
    matches!(token.map(|t| &t.kind), Some(TokenKind::Symbol(s)) if *s == expected)
}

fn name(token: Option<&Token>) -> Option<&str> {
    match &token?.kind {
        TokenKind::Word(w) => Some(w),
        _ => None,
    }
}

fn matching(tokens: &[Token], start: usize, open: Symbol, close: Symbol) -> Option<usize> {
    if !symbol(tokens.get(start), open) {
        return None;
    }
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if symbol(Some(token), open) {
            depth += 1;
        }
        if symbol(Some(token), close) {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

#[derive(Clone, Debug, Default)]
struct ParsedEffects {
    alloc: bool,
    spawn: bool,
    join: bool,
    may_block: bool,
    /// Card `COMPOSE-CONCURRENCY`. Admitted **only** when this module is on the
    /// composed route; on the task route alone `io` is still the unsupported
    /// effect it has always been, refused at `Q020`, so a module that spawns and
    /// prints without composing keeps its exact refusal.
    io: bool,
    /// Card `W48-QR-TASK-RECORD-CAPTURES`: `mut`, which a record field write
    /// derives (card `W48-QR-USABLE-RECORDS`). Admitted **only** in a module
    /// that declares a record type; in every other module it is still the
    /// unsupported effect it has always been, refused at `Q020`.
    mutates: bool,
    errors: BTreeSet<String>,
    /// Card T06. The declared subset of `errors` whose variants carry exactly
    /// one `own<Buffer>`. It is derived from the error type declaration, never
    /// from a use site, so one tag cannot carry a payload in one function and
    /// not in another.
    error_payloads: BTreeSet<String>,
    /// Card `W48-QR-FLOATING-POINT`: `partial_eq`/`partial_ord`, the
    /// permission a float comparison of this function's own body needs.
    comparisons: qcore_core::scalar_types::FloatEffects,
}

/// A declared nominal error type: its complete variant set and the subset that
/// carries one owned Buffer.
#[derive(Clone, Debug, Default)]
struct ErrorType {
    variants: BTreeSet<String>,
    payloads: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scalar {
    /// `own<Buffer>` — or, since card `W48-QR-SCALAR-TYPES-R6`,
    /// `own<Buffer<T>>` for any §G02t type `T`: the payload type is part of
    /// the Buffer's type, and `own<Buffer>` is `own<Buffer<i32>>`.
    OwnedBuffer(FixedType),
    /// `borrow<Buffer>`: a scoped read loan. Only a worker parameter can have
    /// this type, and only a `read … share FULL` capture can produce one.
    ReadLoan(FixedType),
    /// `lend<Buffer>`: a scoped range write lease (card `WRITE-LEND-TASKS`).
    /// Only a worker parameter can have this type, and only a
    /// `lend PLACE LOW..HIGH` capture can produce one. `lend` stays
    /// contextual: it is recognised only immediately before `<Buffer>` in a
    /// task parameter type, a spelling that refused before this card.
    WriteLease(FixedType),
    I32,
    /// Every other type of the §G02t fixed-width table (card
    /// `W48-QR-SCALAR-TYPES-R4`); `i32` keeps its own variant.
    Fixed(FixedType),
    Unit,
}

use qcore_core::scalar_types::{RecordedTarget, ScalarType as FixedType};

/// A §G02t type name as a task scalar: `i32` is [`Scalar::I32`], every other
/// name of the table [`Scalar::Fixed`] — `f32`/`f64` included (card
/// `W48-QR-FLOATING-POINT`: parameters, results, `copy` captures and record
/// fields). A Buffer's payload is read by [`buffer_payload`], which keeps the
/// integer table.
fn table_scalar(name: &str) -> Option<Scalar> {
    FixedType::parse_admitting_floats(name).map(|ty| {
        if ty == FixedType::I32 {
            Scalar::I32
        } else {
            Scalar::Fixed(ty)
        }
    })
}

/// The payload of a Buffer type's argument: `Buffer` is the `i32` payload
/// every Buffer had before card `W48-QR-SCALAR-TYPES-R6`, and `Buffer<T>`
/// names any §G02t type `T` (`Buffer<i32>` is the same type as `Buffer`) —
/// `f32`/`f64` included since card `W48-QR-FLOAT-BUFFER-PAYLOADS`, whose byte
/// extents are their sizes, 4 and 8.
fn buffer_payload(tokens: &[Token]) -> Option<FixedType> {
    match tokens {
        [buffer] if word(Some(buffer), "Buffer") => Some(FixedType::I32),
        [buffer, lt, ty, gt]
            if word(Some(buffer), "Buffer")
                && symbol(Some(lt), Symbol::Less)
                && symbol(Some(gt), Symbol::Greater) =>
        {
            name(Some(ty)).and_then(FixedType::parse_admitting_floats)
        }
        _ => None,
    }
}

fn scalar(tokens: &[Token]) -> Option<Scalar> {
    match tokens {
        [kind, lt, inner @ .., gt]
            if symbol(Some(lt), Symbol::Less) && symbol(Some(gt), Symbol::Greater) =>
        {
            let payload = buffer_payload(inner)?;
            if word(Some(kind), "own") {
                Some(Scalar::OwnedBuffer(payload))
            } else if word(Some(kind), "borrow") {
                Some(Scalar::ReadLoan(payload))
            } else if word(Some(kind), "lend") {
                Some(Scalar::WriteLease(payload))
            } else {
                None
            }
        }
        [t] if word(Some(t), "unit") => Some(Scalar::Unit),
        [t] => name(Some(t)).and_then(table_scalar),
        _ => None,
    }
}

impl Scalar {
    /// The `i32`-payload Buffer, `own<Buffer>`.
    const OWNED_BUFFER: Self = Self::OwnedBuffer(FixedType::I32);
    /// The payload type of a Buffer, a loan or a lease.
    fn payload(self) -> Option<FixedType> {
        match self {
            Self::OwnedBuffer(ty) | Self::ReadLoan(ty) | Self::WriteLease(ty) => Some(ty),
            _ => None,
        }
    }
    /// The copy scalar holding one value of a §G02t type.
    fn of(ty: FixedType) -> Self {
        if ty == FixedType::I32 {
            Self::I32
        } else {
            Self::Fixed(ty)
        }
    }
}

/// The type of a task parameter or result: one of the route's scalars, or a
/// record type this module declares (card `W48-QR-TASK-RECORD-CAPTURES`),
/// which the core sees as its leaves — and, as a result, whole.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Shape {
    Scalar(Scalar),
    Record(String),
}

impl Shape {
    fn core(&self, records: &Records) -> ScalarType {
        match self {
            Shape::Scalar(scalar) => scalar.core(),
            Shape::Record(name) => ScalarType::Record(records[name].key),
        }
    }
}

#[derive(Clone, Debug)]
struct Signature {
    id: SymbolId,
    name: String,
    parameters: Vec<(SymbolId, String, Shape)>,
    result: Shape,
    effects: ParsedEffects,
    /// Card `W48-QR-CLOSURES`: a closure's hidden worker. Its parameters are
    /// the capture binders, whose types the spawn site decides; until then
    /// `parameters` is empty and the worker is not lowered.
    closure: Option<ClosureParameters>,
}

/// A closure's parameters before and after its spawn site decides their types.
#[derive(Clone, Debug)]
struct ClosureParameters {
    binders: Vec<(SymbolId, String)>,
    decided: bool,
}

fn error_declarations(
    source: &SourceProgram,
) -> Result<BTreeMap<String, ErrorType>, SourceDiagnostic> {
    let builtin = |tag: &str| ErrorType {
        variants: BTreeSet::from([tag.to_owned()]),
        // Both are raised by the runtime after a failure that acquired
        // nothing, so neither can ever hold a payload.
        payloads: BTreeSet::new(),
    };
    let mut errors = BTreeMap::from([
        ("AllocError".to_owned(), builtin("AllocError")),
        ("SpawnError".to_owned(), builtin("SpawnError")),
        // Card `W48-QR-SCALAR-TYPES-R4`: the §G02t builtin, payload-free, one
        // tag per variant — what a value statement raises on a fault.
        (
            qcore_core::scalar_types::ArithmeticError::NAME.to_owned(),
            ErrorType {
                variants: qcore_core::scalar_types::ArithmeticError::ALL
                    .iter()
                    .map(|error| error.to_string())
                    .collect(),
                payloads: BTreeSet::new(),
            },
        ),
    ]);
    for declaration in &source.declarations {
        if declaration.value.kind != DeclarationKind::ErrorType {
            continue;
        }
        let tokens = tokens_for(source, declaration.span);
        let bad = || {
            failure(
                declaration.span,
                SemanticPass::TypedErrors,
                "task execution supports nominal error variants that are payload-free or carry exactly one own<Buffer>",
            )
        };
        if tokens.len() < 6
            || !word(tokens.first(), "error")
            || !symbol(tokens.get(2), Symbol::LeftBrace)
            || !symbol(tokens.last(), Symbol::RightBrace)
            || declaration.value.name == "SpawnError"
        {
            return Err(bad());
        }
        let mut declared = ErrorType::default();
        let inner = &tokens[3..tokens.len() - 1];
        let mut cursor = 0;
        while cursor < inner.len() {
            let Some(variant) = name(inner.get(cursor)) else {
                return Err(bad());
            };
            let tag = format!("{}::{variant}", declaration.value.name);
            // `V;` is payload-free; `V(own<Buffer>);` carries exactly one owned
            // Buffer. Nothing else is admitted, so a scalar, a borrow, several
            // fields and an empty list each reject rather than degrade.
            // Card `W48-QR-SCALAR-TYPES-R6`: a typed payload in an error
            // variant (`V(own<Buffer<T>>);`) is refused by name — the error
            // payload stays the `i32` Buffer (`W48-QR-TYPES-ERROR-PAYLOAD`).
            if symbol(inner.get(cursor + 1), Symbol::LeftParen)
                && inner.len() > cursor + 10
                && matches!(
                    scalar(&inner[cursor + 2..cursor + 9]),
                    Some(Scalar::OwnedBuffer(ty)) if ty != FixedType::I32
                )
                && symbol(inner.get(cursor + 9), Symbol::RightParen)
            {
                return Err(failure(
                    declaration.span,
                    SemanticPass::TypedErrors,
                    "an error variant's payload is `own<Buffer>`, the i32 payload; a typed \
                     payload `own<Buffer<T>>` in an error is not admitted \
                     (W48-QR-TYPES-ERROR-PAYLOAD)",
                ));
            }
            let payload = if symbol(inner.get(cursor + 1), Symbol::Semicolon) {
                cursor += 2;
                false
            } else if symbol(inner.get(cursor + 1), Symbol::LeftParen)
                && inner.len() > cursor + 7
                && scalar(&inner[cursor + 2..cursor + 6]) == Some(Scalar::OWNED_BUFFER)
                && symbol(inner.get(cursor + 6), Symbol::RightParen)
                && symbol(inner.get(cursor + 7), Symbol::Semicolon)
            {
                cursor += 8;
                true
            } else {
                return Err(bad());
            };
            if !declared.variants.insert(tag.clone()) {
                return Err(bad());
            }
            if payload {
                declared.payloads.insert(tag);
            }
        }
        if declared.variants.is_empty()
            || errors
                .insert(declaration.value.name.clone(), declared)
                .is_some()
        {
            return Err(bad());
        }
    }
    Ok(errors)
}

fn signature(
    source: &SourceProgram,
    declaration: &Located<SourceDeclaration>,
    typed: &TypedSourceDeclaration,
    errors: &BTreeMap<String, ErrorType>,
    composing: bool,
    records: &Records,
) -> Result<Signature, SourceDiagnostic> {
    let bad = |message| failure(declaration.span, SemanticPass::OrdinaryTypes, message);
    let body = declaration
        .value
        .body
        .as_ref()
        .ok_or_else(|| bad("task functions require source bodies"))?;
    if declaration.value.function_contract.is_some() {
        return Err(bad(
            "source contracts require an implemented consumer for task execution",
        ));
    }
    let header_span = Span {
        start: declaration.span.start,
        end: body.span.start,
    };
    let tokens = tokens_for(source, header_span);
    if !word(tokens.first(), "fn")
        || name(tokens.get(1)) != Some(declaration.value.name.as_str())
        || !symbol(tokens.get(2), Symbol::LeftParen)
    {
        return Err(bad(
            "generic/unsupported function headers are outside task execution",
        ));
    }
    let close = matching(&tokens, 2, Symbol::LeftParen, Symbol::RightParen)
        .ok_or_else(|| bad("invalid task parameter list"))?;
    let groups = if close == 3 {
        Vec::new()
    } else {
        tokens[3..close]
            .split(|token| symbol(Some(token), Symbol::Comma))
            .collect::<Vec<_>>()
    };
    if groups.len() != typed.parameters.len() {
        return Err(bad("task parameter metadata disagrees with the source"));
    }
    let mut parameters = Vec::new();
    for (group, parameter) in groups.iter().zip(&typed.parameters) {
        if name(group.first()) != Some(parameter.name.as_str())
            || !symbol(group.get(1), Symbol::Colon)
        {
            return Err(bad("task parameters require explicit types"));
        }
        // Card `W48-QR-TASK-RECORD-CAPTURES`: a record parameter — ungraded for
        // a record of scalars, `use one` for a record that owns a Buffer, on
        // `own<Buffer> use one`'s footing.
        if let Some(record) = name(group.get(2)).and_then(|word| records.get(word)) {
            let owns = record.owns();
            let graded = group.len() == 5
                && word(group.get(3), "use")
                && word(group.get(4), "one")
                && parameter
                    .usage
                    .as_ref()
                    .is_some_and(|usage| usage.value == qcore_core::usage::Usage::ONE);
            let ungraded = group.len() == 3 && parameter.usage.is_none();
            if (owns && graded) || (!owns && ungraded) {
                parameters.push((
                    parameter.symbol,
                    parameter.name.clone(),
                    Shape::Record(record.name.clone()),
                ));
                continue;
            }
            return Err(bad(if owns {
                "a record parameter that owns a Buffer is spelled `NAME: RECORD use one`"
            } else {
                "a record parameter of scalars is spelled `NAME: RECORD`, ungraded"
            }));
        }
        let kind = if group.len() == 3
            && let Some(kind) = name(group.get(2)).and_then(table_scalar)
            && parameter.usage.is_none()
        {
            // Card `W48-QR-SCALAR-TYPES-R4`: every §G02t type, `i32` as before.
            kind
        } else if group.len() >= 8
            && let Some(Scalar::OwnedBuffer(payload)) = scalar(&group[2..group.len() - 2])
            && word(group.get(group.len() - 2), "use")
            && word(group.last(), "one")
            && parameter
                .usage
                .as_ref()
                .is_some_and(|usage| usage.value == qcore_core::usage::Usage::ONE)
        {
            // Card `W48-QR-SCALAR-TYPES-R6`: `own<Buffer>` or
            // `own<Buffer<T>>`, the payload type part of the parameter's type.
            Scalar::OwnedBuffer(payload)
        } else if group.len() >= 6
            && let Some(Scalar::ReadLoan(payload)) = scalar(&group[2..])
            && parameter.usage.is_none()
        {
            // A read loan carries no usage grade in this profile: the borrower
            // may read it any number of times and may do nothing else with it.
            // Grading loan reads is deliberately left open.
            Scalar::ReadLoan(payload)
        } else if group.len() >= 6
            && let Some(Scalar::WriteLease(payload)) = scalar(&group[2..])
            && parameter.usage.is_none()
        {
            // A write lease carries no usage grade either: in this first
            // slice the holder performs no operation on it at all — the
            // authority is held and enforced against the owner, and the
            // worker-side range write is the named residual
            // `WRITE-LEND-TASKS-EXERCISE`.
            Scalar::WriteLease(payload)
        } else {
            return Err(bad(
                "task parameters require i32, own<Buffer> use one, an ungraded borrow<Buffer>, \
                 or an ungraded lend<Buffer>",
            ));
        };
        parameters.push((
            parameter.symbol,
            parameter.name.clone(),
            Shape::Scalar(kind),
        ));
    }
    if !symbol(tokens.get(close + 1), Symbol::Arrow) {
        return Err(bad("task functions require an explicit result type"));
    }
    let (result, effects) =
        result_and_effects(declaration, &tokens, close + 1, errors, composing, records)?;
    Ok(Signature {
        id: typed.symbol,
        name: typed.name.clone(),
        parameters,
        result,
        effects,
        closure: None,
    })
}

/// A closure's hidden worker's signature (card `W48-QR-CLOSURES`): its result
/// and effect row read exactly as a named worker's are, after its capture list;
/// its parameters are the capture binders, typed at the spawn site.
fn closure_signature(
    source: &SourceProgram,
    declaration: &Located<SourceDeclaration>,
    typed: &TypedSourceDeclaration,
    errors: &BTreeMap<String, ErrorType>,
    composing: bool,
    records: &Records,
) -> Result<Signature, SourceDiagnostic> {
    let bad = |message| failure(declaration.span, SemanticPass::OrdinaryTypes, message);
    let body = declaration
        .value
        .body
        .as_ref()
        .ok_or_else(|| bad("task functions require source bodies"))?;
    let tokens = tokens_for(
        source,
        Span {
            start: declaration.span.start,
            end: body.span.start,
        },
    );
    // The capture list holds no `->`, so the first is the result's.
    let arrow = tokens
        .iter()
        .position(|token| symbol(Some(token), Symbol::Arrow))
        .ok_or_else(|| bad("task functions require an explicit result type"))?;
    let (result, effects) =
        result_and_effects(declaration, &tokens, arrow, errors, composing, records)?;
    Ok(Signature {
        id: typed.symbol,
        name: typed.name.clone(),
        parameters: Vec::new(),
        result,
        effects,
        closure: Some(ClosureParameters {
            binders: typed
                .parameters
                .iter()
                .map(|parameter| (parameter.symbol, parameter.name.clone()))
                .collect(),
            decided: false,
        }),
    })
}

/// A task function's result type and effect row, read from its header's
/// tokens after the `->` at `arrow` (split out of [`signature`] by card
/// `W48-QR-CLOSURES`, byte for byte, so a closure's header is read by the same
/// code).
fn result_and_effects(
    declaration: &Located<SourceDeclaration>,
    tokens: &[Token],
    arrow: usize,
    errors: &BTreeMap<String, ErrorType>,
    composing: bool,
    records: &Records,
) -> Result<(Shape, ParsedEffects), SourceDiagnostic> {
    let bad = |message| failure(declaration.span, SemanticPass::OrdinaryTypes, message);
    let close = arrow - 1;
    let result_end = tokens
        .iter()
        .enumerate()
        .skip(close + 2)
        .find_map(|(i, t)| word(Some(t), "effects").then_some(i))
        .unwrap_or(tokens.len());
    let result = match &tokens[close + 2..result_end] {
        // Card `W48-QR-TASK-RECORD-CAPTURES`: a record result.
        [token] if name(Some(token)).is_some_and(|word| records.contains_key(word)) => {
            Shape::Record(name(Some(token)).unwrap_or_default().to_owned())
        }
        spelled => Shape::Scalar(
            scalar(spelled)
                .filter(|kind| !matches!(kind, Scalar::ReadLoan(_) | Scalar::WriteLease(_)))
                .ok_or_else(|| bad("task functions support i32, unit and own<Buffer> results"))?,
        ),
    };
    let mut cursor = result_end;
    let mut effects = ParsedEffects::default();
    if cursor < tokens.len() {
        if !word(tokens.get(cursor), "effects")
            || !symbol(tokens.get(cursor + 1), Symbol::LeftBrace)
        {
            return Err(bad("unsupported function annotation in task execution"));
        }
        let last = matching(tokens, cursor + 1, Symbol::LeftBrace, Symbol::RightBrace)
            .ok_or_else(|| bad("invalid task effect row"))?;
        if last + 1 != tokens.len() {
            return Err(bad(
                "trailing function annotations are not supported by task execution",
            ));
        }
        cursor += 2;
        let mut seen = BTreeSet::new();
        while cursor < last {
            let effect = name(tokens.get(cursor)).ok_or_else(|| bad("invalid task effect"))?;
            let key;
            match effect {
                "alloc" => {
                    effects.alloc = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                "spawn" => {
                    effects.spawn = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                "join" => {
                    effects.join = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                "may_block" => {
                    effects.may_block = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                // The one widening card `COMPOSE-CONCURRENCY` makes to this
                // grammar, and it is gated: off the composed route the arm is
                // not taken and `io` falls to the refusal below, byte for byte.
                "io" if composing => {
                    effects.io = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                // Card `W48-QR-TASK-RECORD-CAPTURES`, gated the same way: only
                // in a module that declares a record, which a field write needs.
                "mut" if !records.is_empty() => {
                    effects.mutates = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                // Card `W48-QR-FLOATING-POINT`: the comparison effects.
                "partial_eq" | "partial_ord" => {
                    effects.comparisons.insert(
                        qcore_core::scalar_types::FloatEffect::parse(effect)
                            .expect("matched above"),
                    );
                    key = effect.to_owned();
                    cursor += 1;
                }
                "throws" => {
                    if !symbol(tokens.get(cursor + 1), Symbol::Less)
                        || !symbol(tokens.get(cursor + 3), Symbol::Greater)
                    {
                        return Err(bad("throws requires one declared nominal error type"));
                    }
                    let error = name(tokens.get(cursor + 2))
                        .ok_or_else(|| bad("throws requires a nominal error type"))?;
                    let declared = errors
                        .get(error)
                        .ok_or_else(|| bad("throws references an unknown error type"))?;
                    effects.errors.extend(declared.variants.iter().cloned());
                    effects
                        .error_payloads
                        .extend(declared.payloads.iter().cloned());
                    key = format!("throws<{error}>");
                    cursor += 4;
                }
                _ => return Err(bad("unsupported effect in task execution")),
            }
            if !seen.insert(key) {
                return Err(bad("duplicate task effect"));
            }
            if cursor == last {
                break;
            }
            if !symbol(tokens.get(cursor), Symbol::Comma) {
                return Err(bad("task effects must be separated by commas"));
            }
            cursor += 1;
            if cursor == last {
                return Err(bad("missing effect after comma"));
            }
        }
    }
    // Card T06's bound on the *declared row*, not on a path. When some declared
    // variant carries a payload, every one of them must, so a joined error
    // always holds exactly one Buffer and the caller's take is total. A mixed
    // row would make the payload's presence depend on which variant occurred,
    // which this profile does not represent. It is a refusal, not a silent
    // widening: the core would still reject the take on the payload-free path.
    if !effects.error_payloads.is_empty() && effects.error_payloads != effects.errors {
        return Err(failure(
            declaration.span,
            SemanticPass::TypedErrors,
            "a declared error row must carry an owned payload on every variant or on none",
        ));
    }
    Ok((result, effects))
}

use qcore_core::prelude::concurrency::task_execution::{
    ErrorOperand, FULL_READ_AUTHORITY, ScalarOperand, ScalarType, TaskBlock, TaskEffects,
    TaskFunction, TaskInstruction, TaskInstructionKind, TaskModule, TaskSpan, TaskTerminator,
    TaskTerminatorKind,
};

impl Scalar {
    fn core(self) -> ScalarType {
        match self {
            Self::OwnedBuffer(payload) => ScalarType::OwnedBuffer(payload),
            Self::ReadLoan(payload) => ScalarType::ReadLoan(payload),
            Self::WriteLease(payload) => ScalarType::WriteLease(payload),
            Self::I32 => ScalarType::I32,
            Self::Fixed(ty) => ScalarType::Fixed(ty),
            Self::Unit => ScalarType::Unit,
        }
    }
}

/// Evaluate a source read share into the ledger's authority unit.
///
/// `FULL` is the whole read authority; a decimal literal and a sum of admitted
/// shares are the fractions of it. Every other form is a represented refusal:
/// a named share needs a declared share constant, which this profile does not
/// have. A share of zero grants nothing and a share above the full authority is
/// not a fraction of it, so both are refused here rather than clamped — the
/// ledger repeats both checks, so this is the first of two gates, not the only
/// one.
fn read_share(share: &SourceShareExpression, span: Span) -> Result<u32, SourceDiagnostic> {
    let value = match share {
        SourceShareExpression::Full => FULL_READ_AUTHORITY,
        SourceShareExpression::Parenthesized(inner) => read_share(&inner.value, inner.span)?,
        SourceShareExpression::Add(left, right) => read_share(&left.value, left.span)?
            .checked_add(read_share(&right.value, right.span)?)
            .ok_or_else(|| {
                failure(
                    span,
                    SemanticPass::Concurrency,
                    "read share sum is outside the representable authority range",
                )
            })?,
        SourceShareExpression::Integer(digits) => digits.parse::<u32>().map_err(|_| {
            failure(
                span,
                SemanticPass::Concurrency,
                "a read share literal must be a decimal fraction of the full authority",
            )
        })?,
        SourceShareExpression::Name(_) => {
            return Err(failure(
                span,
                SemanticPass::Concurrency,
                "named read shares require a declared share constant, which is outside this profile",
            ));
        }
    };
    if value == 0 || value > FULL_READ_AUTHORITY {
        return Err(failure(
            span,
            SemanticPass::Concurrency,
            format!(
                "a read share must be positive and at most FULL ({}); this capture requests {value}",
                FULL_READ_AUTHORITY
            ),
        ));
    }
    Ok(value)
}

fn core_span(span: Span) -> TaskSpan {
    TaskSpan {
        start: span.start,
        end: span.end,
    }
}

/// Evaluate one bound of a `lend PLACE LOW..HIGH` capture (card
/// `WRITE-LEND-TASKS`). The bound tier is deliberately the narrowest of the
/// family: an integer literal only, because the crossing's custody projection
/// must *state* the lease's footprint and the disjointness question must be
/// decided at check time. A name is refused here as a **varying bound** — by
/// its own spelling, at the boundary, not as a parse failure — and the
/// widening to a constant-binding tier is the named residual
/// `WRITE-LEND-TASKS-VARYING-BOUND`.
fn lease_bound_value(bound: &Located<SourceLeaseBound>) -> Result<u64, SourceDiagnostic> {
    match &bound.value {
        SourceLeaseBound::Integer(digits) => digits.parse::<u64>().map_err(|_| {
            failure(
                bound.span,
                SemanticPass::Concurrency,
                "a write lend bound must be a decimal byte offset",
            )
        }),
        SourceLeaseBound::Name(name) => Err(failure(
            bound.span,
            SemanticPass::Concurrency,
            format!(
                "`{name}` is not a bound this crossing can decide: a write lend's range is \
                 decided at check time, from integer literals, so the disjointness of two \
                 workers' ranges is a decision rather than a premise; a varying bound is \
                 the named residual `WRITE-LEND-TASKS-VARYING-BOUND`"
            ),
        )),
    }
}

// ---------------------------------------------------------------------------
// Card CT46-SENDSYNC: the transferability judgment, consulted redundantly.
//
// `qcore_core::transfer` derives which kinds may cross a task-creation
// boundary from the kind itself. It is consulted here *beside* the existing
// enumeration in `spawn_inputs`, and the two must agree; see that function.
// Neither side is preferred, so this cannot widen admission — a disagreement
// in either direction is a refusal.
// ---------------------------------------------------------------------------

/// The elaborated scalar grammar, as a transfer kind.
///
/// This is total over `Scalar`, so a new executable scalar spelling must be
/// classified before it can reach a boundary.
fn transfer_kind(scalar: Scalar) -> qcore_core::transfer::TransferKind {
    use qcore_core::transfer::TransferKind;
    match scalar {
        Scalar::I32 => TransferKind::I32,
        Scalar::Fixed(_) => TransferKind::FixedScalar,
        Scalar::Unit => TransferKind::Unit,
        // Card `W48-QR-SCALAR-TYPES-R6`: the payload type does not change how
        // a Buffer crosses — custody, a counted share or a disjoint lease —
        // so every payload is the same transfer kind.
        Scalar::OwnedBuffer(_) => TransferKind::OwnedBuffer,
        Scalar::ReadLoan(_) => TransferKind::ReadLoan,
        Scalar::WriteLease(_) => TransferKind::WriteLease,
    }
}

/// The lowering's binding grammar, as a transfer kind. Total over
/// [`ValueType`].
///
/// Only the scalar cases can reach the capture match, because a non-scalar
/// place is already refused by the place/parameter type check. The other arms
/// exist so the guard in `spawn_inputs` can state — and a test can check —
/// that no non-scalar binding is ever sendable.
fn binding_transfer_kind(kind: &ValueType) -> qcore_core::transfer::TransferKind {
    use qcore_core::transfer::TransferKind;
    match kind {
        ValueType::Scalar(scalar) => transfer_kind(*scalar),
        ValueType::Handle(_) => TransferKind::TaskHandle,
        ValueType::Result { .. } => TransferKind::TaskResult,
        ValueType::Scope => TransferKind::ThreadScope,
        ValueType::KnownError(_) => TransferKind::KnownErrorTag,
        ValueType::Error { .. } => TransferKind::ErrorValue,
        // Card `W48-QR-TASK-RECORD-CAPTURES`: a record is no flat kind; its
        // crossing is the aggregate judgment's, over its leaves, decided in
        // `record_capture` before this guard is reached.
        ValueType::Record(_) => TransferKind::Unmodeled,
    }
}

/// The capture keywords, as transfer modes. Total over
/// [`SourceCaptureMode`], `Write` included: the keyword parses and has never
/// had a lowering, and the judgment refuses it by name rather than by
/// falling off the end of a match.
fn transfer_mode(mode: &SourceCaptureMode) -> qcore_core::transfer::TransferMode {
    use qcore_core::transfer::TransferMode;
    match mode {
        SourceCaptureMode::Copy => TransferMode::Copy,
        SourceCaptureMode::Move => TransferMode::Move,
        SourceCaptureMode::Read { .. } => TransferMode::Lend,
        SourceCaptureMode::Write => TransferMode::Write,
        SourceCaptureMode::LendWrite { .. } => TransferMode::LendWrite,
    }
}

// ---------------------------------------------------------------------------
// Card CT47-COPY-CAPTURE: the judgment's refusal, spelled at the boundary.
//
// CT46-SENDSYNC wired the judgment in beside the enumeration and deliberately
// moved nothing: a capture both sides refused still fell through to the
// enumeration's executable-subset catch-all in `spawn_inputs`, attributed to
// `CEmission` (`Q500`) — a statement about what the backend can encode, not
// about why the value may not cross. This card moves that attribution: where
// the judgment and the enumeration agree the capture is refused, the refusal
// is now the judgment's own, attributed to the crossing construct at
// `Concurrency` (`Q600`; a condition widening stated in spec §L21), with a
// message naming the missing transfer/share contract and the contracts the
// kind does carry. The grammar gate (`Q000`, a capture with no declared mode)
// and the catch-all itself are untouched: the catch-all stays as the
// fail-closed backstop for anything the judgment does not decide first.
// ---------------------------------------------------------------------------

/// The source spelling of an executable scalar, for the judgment's refusal
/// messages. Total over `Scalar`.
fn scalar_spelling(scalar: Scalar) -> String {
    // Card `W48-QR-SCALAR-TYPES-R6`: an `i32` payload keeps its historical
    // spelling, so every message written before the card is unchanged.
    let buffer = |kind: &str, payload: FixedType| {
        if payload == FixedType::I32 {
            format!("`{kind}<Buffer>`")
        } else {
            format!("`{kind}<Buffer<{payload}>>`")
        }
    };
    match scalar {
        Scalar::I32 => "`i32`".to_owned(),
        Scalar::Fixed(ty) => fixed_spelling(ty).to_owned(),
        Scalar::Unit => "`unit`".to_owned(),
        Scalar::OwnedBuffer(payload) => buffer("own", payload),
        Scalar::ReadLoan(payload) => buffer("borrow", payload),
        Scalar::WriteLease(payload) => buffer("lend", payload),
    }
}

/// The source spelling of a fixed-width type, for the judgment's refusal
/// messages.
fn fixed_spelling(ty: FixedType) -> &'static str {
    match ty {
        FixedType::Bool => "`bool`",
        FixedType::I8 => "`i8`",
        FixedType::I16 => "`i16`",
        FixedType::I32 => "`i32`",
        FixedType::I64 => "`i64`",
        FixedType::Isize => "`isize`",
        FixedType::U8 => "`u8`",
        FixedType::U16 => "`u16`",
        FixedType::U32 => "`u32`",
        FixedType::U64 => "`u64`",
        FixedType::Usize => "`usize`",
        FixedType::F32 => "`f32`",
        FixedType::F64 => "`f64`",
    }
}

/// The source spelling of a capture mode, for the judgment's refusal
/// messages. Total over `SourceCaptureMode`.
fn mode_spelling(mode: &SourceCaptureMode) -> &'static str {
    match mode {
        SourceCaptureMode::Copy => "`copy`",
        SourceCaptureMode::Move => "`move`",
        SourceCaptureMode::Read { .. } => "`read ... share ...`",
        SourceCaptureMode::Write => "`write`",
        SourceCaptureMode::LendWrite { .. } => "`lend ... LOW..HIGH`",
    }
}

/// The transfer/share contracts a kind does carry, read from the judgment's
/// own classification so a refusal names what would cross rather than only
/// what does not.
fn admitted_contracts(scalar: Scalar) -> &'static str {
    use qcore_core::transfer::{ShareProtocol, TransferClass};
    match qcore_core::transfer::transferable(transfer_kind(scalar)) {
        TransferClass::Duplicated => "`copy` (an independent duplicate; no custody crosses)",
        TransferClass::MovedOnly {
            shareable_under: Some(ShareProtocol::CountedReadShares),
            mutably_shareable_under: Some(ShareProtocol::DisjointWriteLeases),
        } => {
            "`move` (custody transferred to the worker), `read PLACE share N` \
             (a counted read share), or `lend PLACE LOW..HIGH` (a disjoint \
             range write lease)"
        }
        TransferClass::MovedOnly {
            shareable_under: Some(ShareProtocol::CountedReadShares),
            mutably_shareable_under: _,
        } => {
            "`move` (custody transferred to the worker) or `read PLACE share N` \
             (a counted read share)"
        }
        TransferClass::MovedOnly { .. } => "`move` (custody transferred to the worker)",
        TransferClass::Never(_) => "no transfer or share contract at all",
    }
}

/// Card CT47-COPY-CAPTURE: one refusal message per [`TransferRefusal`]
/// variant. Each names the judgment, the capture it refuses, and the missing
/// transfer/share contract — which is what makes the boundary's refusal
/// attributable to the crossing construct rather than to the backend.
fn transfer_refusal_message(
    place: Scalar,
    mode: &SourceCaptureMode,
    parameter: Scalar,
    refusal: qcore_core::transfer::TransferRefusal,
) -> String {
    use qcore_core::transfer::{TransferKind, TransferRefusal};
    let kind = scalar_spelling(place);
    let asked = mode_spelling(mode);
    let carries = admitted_contracts(place);
    match refusal {
        TransferRefusal::ModeNotAdmittedForKind => format!(
            "the transferability judgment refuses this capture: no transfer or share \
             contract admits {asked} of {kind} across a spawn boundary; {kind} crosses \
             only under {carries}"
        ),
        TransferRefusal::WriteCaptureUnimplemented => format!(
            "the transferability judgment refuses this capture: a {asked} capture names \
             no transfer or share contract — no kind crosses a spawn boundary under it; \
             {kind} crosses only under {carries}"
        ),
        TransferRefusal::ReceivedKindMismatch => {
            let declared = scalar_spelling(parameter);
            // A mismatch presupposes an admitted transfer; re-derive what it
            // delivers. The `Err` arm is unreachable and refuses generically
            // rather than panicking.
            let delivered =
                match qcore_core::transfer::admits(transfer_kind(place), transfer_mode(mode)) {
                    Ok(admitted) => {
                        match qcore_core::transfer::received_kind(transfer_kind(place), admitted) {
                            TransferKind::ReadLoan => "`borrow<Buffer>`".to_owned(),
                            TransferKind::WriteLease => "`lend<Buffer>`".to_owned(),
                            _ => kind.clone(),
                        }
                    }
                    Err(_) => kind.clone(),
                };
            format!(
                "the transferability judgment refuses this capture: {asked} of {kind} is \
                 an admitted transfer, but it delivers {delivered} and the worker \
                 declares it receives {declared}"
            )
        }
        TransferRefusal::KindNotTransferable(_) => format!(
            "the transferability judgment refuses this capture: {kind} carries no \
             transfer or share contract, so it does not cross a spawn boundary in any \
             capture mode"
        ),
        TransferRefusal::ProtocolUnavailable(_) => format!(
            "the transferability judgment refuses this capture: sharing {kind} needs a \
             protocol no capture mode of this route can request"
        ),
    }
}

/// Which of the three admitted capture forms the existing enumeration picked,
/// as a value rather than as an early return, so that the derived judgment can
/// be compared against a refusal as well as against an admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnumeratedCapture {
    Copy,
    Move,
    Lend,
    /// A range write lend (card `WRITE-LEND-TASKS`).
    LendWrite,
}

impl EnumeratedCapture {
    /// The derived transfer this enumerated form claims to be, for the
    /// agreement check.
    fn as_admitted(self) -> qcore_core::transfer::AdmittedTransfer {
        use qcore_core::transfer::{AdmittedTransfer, ShareProtocol};
        match self {
            EnumeratedCapture::Copy => AdmittedTransfer::Duplicate,
            EnumeratedCapture::Move => AdmittedTransfer::MoveCustody,
            EnumeratedCapture::Lend => {
                AdmittedTransfer::LendShare(ShareProtocol::CountedReadShares)
            }
            EnumeratedCapture::LendWrite => {
                AdmittedTransfer::LendWriteRange(ShareProtocol::DisjointWriteLeases)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ValueType {
    Scalar(Scalar),
    Handle(SymbolId),
    Result {
        /// A scalar, or — card `W48-QR-TASK-RECORD-CAPTURES` — a record
        /// result, unpacked into leaf slots where it is taken.
        value: Shape,
        errors: BTreeSet<String>,
        /// True when some variant this Result can carry declares an owned
        /// Buffer payload. Such a Result cannot be propagated with `?`: its
        /// payload has to be taken on an explicit error arm.
        error_payload: bool,
    },
    Scope,
    KnownError(String),
    Error {
        errors: BTreeSet<String>,
        /// True until `qcore_error_payload` takes the Buffer out. While it is
        /// true the binding cannot be re-raised or silently dropped.
        payload: bool,
    },
    /// A record binding: a slot per leaf (card `W48-QR-TASK-RECORD-CAPTURES`).
    Record(RecordValue),
}

/// Lower complete source syntax into task execution IR. This checks the source
/// import/type boundary; the resulting IR requires `check_task_module` before
/// emission or a source-execution claim.
pub fn lower_source_tasks(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<TaskModule, Vec<SourceDiagnostic>> {
    lower_source_tasks_for_target(source, typed, entry, None)
}

/// [`lower_source_tasks`] under the builder's recorded target (card
/// `W48-QR-SCALAR-TYPES-R4`), which alone gives `isize`/`usize` a width.
pub fn lower_source_tasks_for_target(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
    target: Option<&RecordedTarget>,
) -> Result<TaskModule, Vec<SourceDiagnostic>> {
    lower(source, typed, entry, false, target)
        .map(|lowered| lowered.tasks)
        .map_err(|error| vec![error])
}

/// A task module and the `source.std-io.v1` events placed in its entry
/// (card `COMPOSE-CONCURRENCY`).
#[derive(Clone, Debug)]
pub struct LoweredComposedTaskModule {
    pub tasks: TaskModule,
    pub sites: Vec<TaskIoSite>,
    /// Whether the entry declared `io`.
    pub declares_io: bool,
    /// The non-entry functions that declared `io` in their own row (card
    /// `OBS-REORDERING`). A worker that writes must say so itself.
    pub worker_io: std::collections::BTreeSet<usize>,
}

/// [`lower_source_tasks`] with the line-producing statements of `std.io`,
/// `std.logger` and `std.fmt` admitted in the **parent's** body.
///
/// `lower_source_tasks` is this function with `composing = false`, so a module
/// on the task route alone travels the identical code path: `io` is still an
/// unsupported effect there, a `print` is still an unsupported expression
/// statement there, and the lowered module is byte-identical. There is no second
/// lowering to drift.
pub fn lower_source_composed_tasks(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredComposedTaskModule, Vec<SourceDiagnostic>> {
    lower_source_composed_tasks_for_target(source, typed, entry, None)
}

/// [`lower_source_composed_tasks`] under the builder's recorded target.
pub fn lower_source_composed_tasks_for_target(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
    target: Option<&RecordedTarget>,
) -> Result<LoweredComposedTaskModule, Vec<SourceDiagnostic>> {
    lower(source, typed, entry, true, target).map_err(|error| vec![error])
}

fn lower(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
    composing: bool,
    target: Option<&RecordedTarget>,
) -> Result<LoweredComposedTaskModule, SourceDiagnostic> {
    let span = Span {
        start: 0,
        end: source.source().len(),
    };
    if source.mode != SourceModuleMode::Executable {
        return Err(failure(
            span,
            SemanticPass::CEmission,
            "entry-free task execution checking is not implemented",
        ));
    }
    let reparsed = parse_source_program(source.source())
        .map_err(|error| failure(error.span, SemanticPass::NameResolution, error.message))?;
    if reparsed != *source {
        return Err(failure(
            span,
            SemanticPass::NameResolution,
            "task source metadata differs from independent parsing",
        ));
    }
    let retyped = elaborate_source(source).map_err(|errors| {
        errors
            .into_iter()
            .next()
            .expect("elaboration rejection has diagnostics")
    })?;
    if retyped != *typed {
        return Err(failure(
            span,
            SemanticPass::NameResolution,
            "task symbol metadata differs from independent elaboration",
        ));
    }
    if source
        .declarations
        .iter()
        .filter(|d| d.value.kind == DeclarationKind::ConcurrencyProfile)
        .count()
        != 1
    {
        return Err(failure(
            span,
            SemanticPass::Concurrency,
            "task execution requires exactly one `concurrency checked;` marker",
        ));
    }
    for declaration in &source.declarations {
        if matches!(
            declaration.value.name.as_str(),
            "qcore_buffer_new"
                | "qcore_buffer_read"
                | "qcore_buffer_write"
                | "qcore_error_payload"
                | "AllocError"
                // Card `W48-QR-SCALAR-TYPES-R4`: the builtin §G02t error type.
                | "ArithmeticError"
        ) {
            return Err(failure(
                declaration.span,
                SemanticPass::NameResolution,
                "reserved owned-buffer intrinsic or error name",
            ));
        }
        // Card `W48-QR-TASK-RECORD-CAPTURES` lifts card `OWNED-FIELD-ROUTE-REACH`'s
        // named refusal of a record type here (`AGGREGATE-TASK-CAPTURE`): a
        // record type is read by `records::task_records` below, and a record
        // crosses a spawn as its leaves under the aggregate judgment.
        if !matches!(
            declaration.value.kind,
            DeclarationKind::ConcurrencyProfile
                | DeclarationKind::Function
                | DeclarationKind::ErrorType
                | DeclarationKind::Record
        ) {
            return Err(failure(
                declaration.span,
                SemanticPass::CEmission,
                "declaration is outside the copy-scalar task execution profile",
            ));
        }
    }
    let errors = error_declarations(source)?;
    let records = records::task_records(source)?;
    let mut signatures: BTreeMap<SymbolId, Signature> = BTreeMap::new();
    for declaration in source
        .declarations
        .iter()
        .filter(|d| d.value.kind == DeclarationKind::Function)
    {
        let resolved = typed
            .declarations
            .iter()
            .find(|d| d.name == declaration.value.name)
            .expect("independent elaboration preserved functions");
        // Card `W48-QR-CLOSURES`: a closure's hidden worker reads its header
        // after its capture list and waits for its spawn site's types.
        let signature = if source.closure(&declaration.value.name).is_some() {
            closure_signature(source, declaration, resolved, &errors, composing, &records)?
        } else {
            signature(source, declaration, resolved, &errors, composing, &records)?
        };
        signatures.insert(signature.id, signature);
    }
    let entry = typed
        .globals
        .get(entry)
        .copied()
        .filter(|symbol| signatures.contains_key(symbol))
        .ok_or_else(|| {
            failure(
                span,
                SemanticPass::NameResolution,
                "task entry must name a source function",
            )
        })?;
    if !signatures[&entry].parameters.is_empty() {
        return Err(failure(
            span,
            SemanticPass::OrdinaryTypes,
            "task execution entry must have no parameters",
        ));
    }
    // Card `W48-QR-SCALAR-TYPES-R4`: the entry's result is published, never a
    // value a caller receives, so it stays `i32`/`unit` (card 0's rule); a
    // fixed-width value is a worker's result, collected by `join`.
    if let Shape::Scalar(Scalar::Fixed(ty)) = signatures[&entry].result {
        return Err(failure(
            span,
            SemanticPass::OrdinaryTypes,
            format!(
                "a task entry returns i32 or unit; `{ty}` is a worker's result, collected by join"
            ),
        ));
    }
    // One function's lowering. With `discovering`, a spawn of a closure whose
    // parameter types are not yet decided decides them from the spawn site
    // and records them there (card `W48-QR-CLOSURES`); without it, every
    // worker's signature is complete.
    let lower_function = |declaration: &Located<SourceDeclaration>,
                          signatures: &BTreeMap<SymbolId, Signature>,
                          discovering: Option<&Discovered>|
     -> Result<(usize, TaskFunction, Vec<TaskIoSite>), SourceDiagnostic> {
        let resolved = typed
            .declarations
            .iter()
            .find(|d| d.name == declaration.value.name)
            .unwrap();
        let signature = &signatures[&resolved.symbol];
        let body = declaration.value.body.as_ref().unwrap();
        let mut builder = Builder::new(
            source,
            signatures,
            signature,
            &errors,
            &records,
            next_temporary(resolved)?,
        )?;
        builder.composing = composing;
        builder.function = resolved.symbol.0 as usize;
        builder.target = target;
        builder.discovering = discovering;
        if !builder.block(body, &resolved.statements)? {
            return Err(failure(
                body.span,
                SemanticPass::OrdinaryTypes,
                "task functions require an explicit return or raise on every path",
            ));
        }
        Ok((
            resolved.symbol.0 as usize,
            TaskFunction {
                name: signature.name.clone(),
                // Card `W48-QR-TASK-RECORD-CAPTURES`: a record parameter is
                // its leaves, one core parameter each, in leaf order.
                parameters: builder.parameters.clone(),
                result: signature.result.core(&records),
                errors: signature.effects.errors.clone(),
                error_payloads: signature.effects.error_payloads.clone(),
                declared_effects: TaskEffects {
                    alloc: signature.effects.alloc,
                    spawn: signature.effects.spawn,
                    join: signature.effects.join,
                    may_block: signature.effects.may_block,
                    comparisons: signature.effects.comparisons,
                },
                entry: 0,
                blocks: builder.blocks,
                span: core_span(declaration.span),
            },
            builder.sites,
        ))
    };
    let function_declarations = || {
        source
            .declarations
            .iter()
            .filter(|d| d.value.kind == DeclarationKind::Function)
    };
    // Card `W48-QR-CLOSURES`: decide every closure's parameter types before
    // any function is lowered for the module. A capture list names modes, not
    // types; each parameter is the captured place's type at the spawn site
    // (the loan or lease type for `read`/`lend` of an owned Buffer), which
    // only the enclosing function's lowering knows. So each function that
    // spawns an undecided closure is lowered once to find out, and its outcome
    // discarded — unless it refused before every closure it spawns was
    // decided, which is then the module's refusal. With every signature
    // complete, the loop below lowers every function in declaration order,
    // exactly as it lowers the hand-written twin.
    let undecided = |signatures: &BTreeMap<SymbolId, Signature>, id: &SymbolId| {
        signatures[id]
            .closure
            .as_ref()
            .is_some_and(|closure| !closure.decided)
    };
    if signatures.keys().any(|id| undecided(&signatures, id)) {
        let discovered = Discovered::default();
        let mut probed = BTreeSet::new();
        loop {
            let mut progress = false;
            for declaration in function_declarations() {
                let id = typed.globals[&declaration.value.name];
                if probed.contains(&id) || undecided(&signatures, &id) {
                    continue;
                }
                probed.insert(id);
                let spawned: Vec<SymbolId> = source
                    .closures()
                    .iter()
                    .filter(|closure| closure.enclosing == declaration.value.name)
                    .map(|closure| typed.globals[&closure.hidden])
                    .filter(|closure| undecided(&signatures, closure))
                    .collect();
                if spawned.is_empty() {
                    continue;
                }
                progress = true;
                let outcome = lower_function(declaration, &signatures, Some(&discovered));
                let decided = std::mem::take(&mut *discovered.borrow_mut());
                for (worker, shapes) in decided {
                    let signature = signatures
                        .get_mut(&worker)
                        .expect("a discovered closure has a signature");
                    if let Some(closure) = &mut signature.closure {
                        signature.parameters = closure
                            .binders
                            .iter()
                            .zip(shapes)
                            .map(|((symbol, name), shape)| (*symbol, name.clone(), shape))
                            .collect();
                        closure.decided = true;
                    }
                }
                if let Err(error) = outcome
                    && spawned.iter().any(|id| undecided(&signatures, id))
                {
                    return Err(error);
                }
            }
            if !progress {
                break;
            }
        }
        if let Some(closure) = source
            .closures()
            .iter()
            .find(|closure| undecided(&signatures, &typed.globals[&closure.hidden]))
        {
            return Err(failure(
                closure.span,
                SemanticPass::Concurrency,
                "this closure's spawn site was never lowered, so no parameter type was decided \
                 for its captures; a closure is lowered from the spawn that names it",
            ));
        }
    }
    let mut functions = BTreeMap::new();
    let mut sites = Vec::new();
    for declaration in function_declarations() {
        let (id, function, function_sites) = lower_function(declaration, &signatures, None)?;
        functions.insert(id, function);
        sites.extend(function_sites);
    }
    // A worker cannot recursively or transitively create tasks in this first
    // executable profile. All source functions were still lowered above.
    let workers = functions
        .values()
        .flat_map(|function| {
            function
                .blocks
                .values()
                .filter_map(|block| match block.terminator.kind {
                    TaskTerminatorKind::TrySpawn { worker, .. } => Some(worker),
                    _ => None,
                })
        })
        .collect::<BTreeSet<_>>();
    for worker in &workers {
        if functions[worker]
            .blocks
            .values()
            .any(|block| matches!(block.terminator.kind, TaskTerminatorKind::TrySpawn { .. }))
        {
            let span = functions[worker].span;
            return Err(failure(
                Span {
                    start: span.start,
                    end: span.end,
                },
                SemanticPass::Concurrency,
                "spawned workers must be first-order leaf functions in this task profile",
            ));
        }
    }
    // Card `OBS-REORDERING`. A line stands in the entry or in a function the
    // entry spawns; a line anywhere else has no thread to attribute it to, so
    // there is no emitter and no window and the comparison would have nothing to
    // compare. The spawn set is only known here, after every terminator exists,
    // which is why this is a pass rather than a guard inside the builder.
    let admissible: BTreeSet<usize> = std::iter::once(entry.0 as usize)
        .chain(workers.iter().copied())
        .collect();
    if let Some(site) = sites
        .iter()
        .find(|site| !admissible.contains(&site.function))
    {
        return Err(failure(
            Span {
                start: site.span.start,
                end: site.span.end,
            },
            SemanticPass::ComposedTasks,
            "`source.std-io-tasks.v1` writes a line from the parent's own body or from the body \
             of a worker this entry spawns, and from nowhere else: a line in a function this \
             program never runs has no thread to attribute it to and no window to place it in",
        ));
    }
    let module = TaskModule {
        entry: entry.0 as usize,
        functions,
        source_len: source.source().len(),
        target: target.cloned(),
        // Card `W48-QR-TASK-RECORD-CAPTURES`: every declared record type, by
        // the key a record result carries.
        records: records
            .values()
            .map(|record| (record.key, record.core()))
            .collect(),
    };
    // Card `W48-QR-SCALAR-TYPES-R4`: `isize`/`usize` take their width from the
    // recorded target, and a function naming either with none is refused —
    // the width is never guessed.
    if target.is_none() {
        // Card `W48-QR-TASK-RECORD-CAPTURES`: a record's leaves name their
        // types wherever the record is held, so a target-width leaf needs the
        // record too.
        for record in records.values() {
            if let Some(ty) = record.leaves.iter().find_map(|leaf| {
                leaf.kind
                    .payload()
                    .or(match leaf.kind {
                        Scalar::Fixed(ty) => Some(ty),
                        _ => None,
                    })
                    .filter(|ty| ty.is_target_width())
            }) {
                let declaration = source
                    .declarations
                    .iter()
                    .find(|declaration| declaration.value.name == record.name)
                    .map_or(span, |declaration| declaration.span);
                return Err(failure(
                    declaration,
                    SemanticPass::OrdinaryTypes,
                    qcore_core::scalar_values::ValueRefusal::NoRecordedTarget { ty }.to_string(),
                ));
            }
        }
        for function in module.functions.values() {
            if let Some(ty) = qcore_core::task_execution::function_fixed_types(function)
                .into_iter()
                .find(|ty| ty.is_target_width())
            {
                return Err(failure(
                    Span {
                        start: function.span.start,
                        end: function.span.end,
                    },
                    SemanticPass::OrdinaryTypes,
                    qcore_core::scalar_values::ValueRefusal::NoRecordedTarget { ty }.to_string(),
                ));
            }
        }
    }
    Ok(LoweredComposedTaskModule {
        declares_io: signatures[&entry].effects.io,
        worker_io: signatures
            .values()
            .filter(|signature| signature.id != entry && signature.effects.io)
            .map(|signature| signature.id.0 as usize)
            .collect(),
        tasks: module,
        sites,
    })
}

fn next_temporary(function: &TypedSourceDeclaration) -> Result<usize, SourceDiagnostic> {
    fn statement_max(statements: &[TypedStatement]) -> usize {
        statements
            .iter()
            .map(|s| {
                s.binding
                    .into_iter()
                    .chain(s.body_binding)
                    .chain(
                        s.result_match
                            .iter()
                            .flat_map(|m| m.arms.iter().filter_map(|a| a.binding)),
                    )
                    .map(|id| id.0 as usize)
                    .chain(std::iter::once(statement_max(&s.nested)))
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0)
    }
    function
        .parameters
        .iter()
        .map(|p| p.symbol.0 as usize)
        .chain(std::iter::once(statement_max(&function.statements)))
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| {
            failure(
                function.span,
                SemanticPass::CEmission,
                "task slot identity space exhausted",
            )
        })
}

/// The parameter types a closure's spawn site decides, by hidden worker (card
/// `W48-QR-CLOSURES`).
type Discovered = std::cell::RefCell<BTreeMap<SymbolId, Vec<Shape>>>;

struct Builder<'a> {
    /// Card `W48-QR-CLOSURES`: set while an enclosing function is lowered to
    /// decide its closures' parameter types; `None` for every real lowering.
    discovering: Option<&'a Discovered>,
    source: &'a SourceProgram,
    signatures: &'a BTreeMap<SymbolId, Signature>,
    signature: &'a Signature,
    errors: &'a BTreeMap<String, ErrorType>,
    environment: Vec<BTreeMap<String, (SymbolId, ValueType)>>,
    scopes: Vec<u64>,
    blocks: BTreeMap<usize, TaskBlock>,
    current: usize,
    next_block: usize,
    next_temporary: usize,
    instructions: Vec<TaskInstruction>,
    /// Card `COMPOSE-CONCURRENCY`; `false` on the task route alone.
    composing: bool,
    /// The function this builder is lowering, which is the function a line it
    /// records stands in (card `OBS-REORDERING`). Before that card only the
    /// entry could carry a line, and the flag was a Boolean.
    function: usize,
    sites: Vec<TaskIoSite>,
    /// The builder's recorded target (card `W48-QR-SCALAR-TYPES-R4`).
    target: Option<&'a RecordedTarget>,
    /// The number of every binding known at check time (card
    /// `W48-QR-SCALAR-TYPES-R4`): a literal, a copy of a known binding, a value
    /// statement over known operands. A binding identity is unique in its
    /// function, so this map needs no scoping. It decides, with the shared
    /// decision, whether a value statement folds or raises — the core decides
    /// the same thing again, independently.
    known: BTreeMap<SymbolId, i128>,
    /// This module's record types (card `W48-QR-TASK-RECORD-CAPTURES`).
    records: &'a Records,
    /// This function's core parameters: a scalar parameter's own slot, and a
    /// record parameter's leaf slots in leaf order.
    parameters: Vec<(usize, ScalarType)>,
}

impl<'a> Builder<'a> {
    fn new(
        source: &'a SourceProgram,
        signatures: &'a BTreeMap<SymbolId, Signature>,
        signature: &'a Signature,
        errors: &'a BTreeMap<String, ErrorType>,
        records: &'a Records,
        next_temporary: usize,
    ) -> Result<Self, SourceDiagnostic> {
        let mut builder = Self {
            discovering: None,
            source,
            signatures,
            signature,
            errors,
            environment: vec![BTreeMap::new()],
            scopes: Vec::new(),
            blocks: BTreeMap::new(),
            current: 0,
            next_block: 1,
            next_temporary,
            instructions: Vec::new(),
            composing: false,
            function: 0,
            sites: Vec::new(),
            target: None,
            known: BTreeMap::new(),
            records,
            parameters: Vec::new(),
        };
        // A scalar parameter is its own slot; a record parameter binds a fresh
        // slot per leaf (card `W48-QR-TASK-RECORD-CAPTURES`).
        let span = Span { start: 0, end: 0 };
        for (symbol, name, shape) in &signature.parameters {
            let kind = match shape {
                Shape::Scalar(scalar) => {
                    builder.parameters.push((symbol.0 as usize, scalar.core()));
                    ValueType::Scalar(*scalar)
                }
                Shape::Record(ty) => {
                    let leaves = builder.fresh_leaves(ty, span)?;
                    builder
                        .parameters
                        .extend(leaves.iter().map(|leaf| (leaf.slot, leaf.kind.core())));
                    ValueType::Record(RecordValue {
                        ty: ty.clone(),
                        leaves,
                        state: RecordState::Live,
                    })
                }
            };
            builder.environment[0].insert(name.clone(), (*symbol, kind));
        }
        Ok(builder)
    }

    /// Whether this function's row permits raising `ArithmeticError`.
    fn raises_arithmetic(&self) -> bool {
        qcore_core::scalar_types::ArithmeticError::ALL
            .iter()
            .all(|error| self.signature.effects.errors.contains(&error.to_string()))
    }

    /// A literal written where a fixed-width type `ty` is expected — a typed
    /// `let` or a worker's `return` (card `W48-QR-SCALAR-TYPES-R4`): `INTEGER`,
    /// `- INTEGER`, `true` or `false`, range-checked against `ty`. `None` when
    /// the tokens are not a literal.
    fn fixed_literal(
        &self,
        tokens: &[Token],
        ty: FixedType,
        span: Span,
    ) -> Option<Result<ScalarOperand, SourceDiagnostic>> {
        // Card `W48-QR-FLOATING-POINT`: `["-"] FLOAT`, read once at a float
        // `ty`; at any other type a float literal is refused, and an integer
        // literal at a float type is refused by the literal check below.
        if let [.., last] = tokens
            && let TokenKind::Float(text) = &last.kind
            && (tokens.len() == 1 || (tokens.len() == 2 && symbol(tokens.first(), Symbol::Minus)))
        {
            let negative = tokens.len() == 2;
            return Some(if !ty.is_float() {
                Err(failure(
                    span,
                    SemanticPass::OrdinaryTypes,
                    qcore_core::scalar_values::ValueRefusal::LiteralKind { ty, float: true }
                        .to_string(),
                ))
            } else {
                match qcore_core::scalar_floats::FloatLiteral::read(text, negative)
                    .map(|literal| literal.at(ty))
                {
                    Some(Ok(value)) => Ok(ScalarOperand::Fixed { ty, value }),
                    Some(Err(error)) => Err(failure(
                        span,
                        SemanticPass::OrdinaryTypes,
                        qcore_core::scalar_values::ValueRefusal::FloatLiteral { ty, error }
                            .to_string(),
                    )),
                    None => Err(failure(
                        span,
                        SemanticPass::OrdinaryTypes,
                        format!("`{text}` is not a float literal"),
                    )),
                }
            });
        }
        let value = match tokens {
            [
                Token {
                    kind: TokenKind::Integer(text),
                    ..
                },
            ] => text.parse::<i128>().ok()?,
            [
                Token {
                    kind: TokenKind::Symbol(Symbol::Minus),
                    ..
                },
                Token {
                    kind: TokenKind::Integer(text),
                    ..
                },
            ] => -text.parse::<i128>().ok()?,
            [t] if word(Some(t), "true") || word(Some(t), "false") => {
                let value = i128::from(word(Some(t), "true"));
                return Some(if ty == FixedType::Bool {
                    Ok(ScalarOperand::Fixed { ty, value })
                } else {
                    Err(failure(
                        span,
                        SemanticPass::OrdinaryTypes,
                        format!("`{ty}` is not `bool`; there is no conversion from bool"),
                    ))
                });
            }
            _ => return None,
        };
        let refused = |refusal: qcore_core::scalar_values::ValueRefusal| {
            failure(span, SemanticPass::OrdinaryTypes, refusal.to_string())
        };
        Some(
            qcore_core::scalar_values::Decider::default()
                .literal_at(value, ty, self.target)
                .map(|value| ScalarOperand::Fixed { ty, value })
                .map_err(refused),
        )
    }

    /// `let NAME = VALUE_EXPRESSION;` (card `W48-QR-SCALAR-TYPES-R4`): one
    /// §G02t operation, decided by the shared decision. A statement that can
    /// raise at run time lowers to the checked terminator whose error edge is
    /// this route's ordinary error exit; any other to one instruction.
    fn value_statement(
        &mut self,
        syntax: crate::source_value_statement::ValueExpressionSyntax<(SymbolId, Scalar)>,
        span: Span,
        binding: SymbolId,
    ) -> Result<ValueType, SourceDiagnostic> {
        use crate::source_value_statement::{
            ValueExpressionSyntax as Syntax, ValueOperandSyntax as Operand,
        };
        use qcore_core::prelude::concurrency::task_execution::{
            TaskValueOperand, TaskValueOperation,
        };
        use qcore_core::scalar_values::{self, Decider, ValueRefusal};
        let shared = |operand: Operand<(SymbolId, Scalar)>, known: &BTreeMap<SymbolId, i128>| {
            match operand {
                Operand::Value((symbol, scalar)) => scalar_values::Operand::Typed {
                    ty: match scalar {
                        Scalar::Fixed(ty) => ty,
                        _ => FixedType::I32,
                    },
                    known: known.get(&symbol).copied(),
                },
                Operand::Integer(literal) => scalar_values::Operand::Integer(literal),
                Operand::Bool(value) => scalar_values::Operand::Bool(value),
                Operand::Float(literal) => scalar_values::Operand::Float(literal),
            }
        };
        let operation = match syntax {
            Syntax::Binary(op, a, b) => {
                scalar_values::Operation::Binary(op, shared(a, &self.known), shared(b, &self.known))
            }
            Syntax::Compare(cmp, a, b) => scalar_values::Operation::Compare(
                cmp,
                shared(a, &self.known),
                shared(b, &self.known),
            ),
            Syntax::Negate(a) => scalar_values::Operation::Negate(shared(a, &self.known)),
            Syntax::Convert(a, to) => scalar_values::Operation::Convert(shared(a, &self.known), to),
        };
        let decision = Decider {
            permitted: self.signature.effects.comparisons,
            ..Decider::default()
        }
        .decide(operation, self.raises_arithmetic(), self.target)
        .map_err(|refusal| match refusal {
            ValueRefusal::NeedsPermission { ty } => failure(
                span,
                SemanticPass::TypedErrors,
                format!(
                    "this `{ty}` operation reads a value known only at run time and can \
                         fault; the function's row must declare `throws<ArithmeticError>` for \
                         the fault to be raised as a typed error, and without it the operation \
                         is refused"
                ),
            ),
            other => failure(span, SemanticPass::OrdinaryTypes, other.to_string()),
        })?;
        // A literal takes the operation's own type, as the decision typed it.
        let lower_operand = |operand: Operand<(SymbolId, Scalar)>| match operand {
            Operand::Value((symbol, _)) => TaskValueOperand::Slot(symbol.0 as usize),
            Operand::Integer(value) => TaskValueOperand::Literal {
                ty: decision.operand_ty,
                value,
            },
            Operand::Bool(value) => TaskValueOperand::Literal {
                ty: FixedType::Bool,
                value: i128::from(value),
            },
            // Card `W48-QR-FLOATING-POINT`: the literal at the type the
            // decision gave it, which it has already read there.
            Operand::Float(literal) => TaskValueOperand::Literal {
                ty: decision.operand_ty,
                value: literal.at(decision.operand_ty).unwrap_or_default(),
            },
        };
        let operation = match syntax {
            Syntax::Binary(op, a, b) => {
                TaskValueOperation::Binary(op, lower_operand(a), lower_operand(b))
            }
            Syntax::Compare(cmp, a, b) => {
                TaskValueOperation::Compare(cmp, lower_operand(a), lower_operand(b))
            }
            Syntax::Negate(a) => TaskValueOperation::Negate(lower_operand(a)),
            Syntax::Convert(a, to) => TaskValueOperation::Convert(lower_operand(a), to),
        };
        let slot = binding.0 as usize;
        if decision.raises {
            let success = self.block_id(span)?;
            let error_slot = self.temporary(span)?;
            let error = self.error_block(span, ErrorOperand::Slot(error_slot))?;
            self.finish(
                span,
                TaskTerminatorKind::TryCompute {
                    slot,
                    operation,
                    error_slot,
                    success,
                    error,
                },
            );
            self.current = success;
        } else {
            self.instruction(span, TaskInstructionKind::Compute { slot, operation });
        }
        if let Some(known) = decision.known {
            self.known.insert(binding, known);
        }
        Ok(ValueType::Scalar(if decision.ty == FixedType::I32 {
            Scalar::I32
        } else {
            Scalar::Fixed(decision.ty)
        }))
    }

    /// Record one `source.std-io.v1` event at the position the emitted program
    /// will perform it: immediately before the next instruction of the block
    /// being built.
    ///
    /// The operand is resolved here, in the **task** model's own environment, so
    /// a rendering names a task slot or nothing at all. Whether that slot may be
    /// *read* there is not decided here: `check_composed_task_program` asks the
    /// task checker, by copying the scalar at this exact position.
    fn source_line(
        &mut self,
        span: Span,
        event: qcore_core::std_io::StdIoEvent,
    ) -> Result<(), SourceDiagnostic> {
        use qcore_core::std_io::{StdIoEvent, StdIoOperand};
        // Which function may carry a line is decided once, after every function
        // is lowered, by `admissible_line_sites` below: it is the entry and the
        // functions the entry spawns, and the spawn set is not known until the
        // terminators exist. Card `OBS-REORDERING`.
        let operand = match &event {
            StdIoEvent::LogLevel { .. } | StdIoEvent::Flush | StdIoEvent::Return => None,
            StdIoEvent::Print { operand, .. } | StdIoEvent::Log { operand, .. } => match operand {
                StdIoOperand::Literal(_) => None,
                StdIoOperand::Borrowed(name) => {
                    return Err(failure(
                        span,
                        SemanticPass::ComposedTasks,
                        format!(
                            "`borrow {name}` renders the model identity a checked custody model \
                             published for a resource, and `source.std-io-tasks.v1` publishes \
                             none: the task ledger's parent/child custody edges, read shares and \
                             join discipline are a different question from the extern route's \
                             straight-line ledger, and a borrowed render across a task boundary \
                             needs its own custody argument"
                        ),
                    ));
                }
                StdIoOperand::Scalar(name) => {
                    let (id, kind) = self.lookup(name, span)?;
                    // Card `W48-QR-SCALAR-TYPES-R4`: a value of every §G02t
                    // type renders at its type; `bool` has no `hex`.
                    if let ValueType::Scalar(Scalar::Fixed(ty)) = kind {
                        let hex = matches!(
                            &event,
                            StdIoEvent::Print {
                                rendering: qcore_core::std_io::Rendering::Hex,
                                ..
                            } | StdIoEvent::Log {
                                rendering: qcore_core::std_io::Rendering::Hex,
                                ..
                            }
                        );
                        if hex && ty == FixedType::Bool {
                            return Err(failure(
                                span,
                                SemanticPass::ComposedTasks,
                                format!(
                                    "`{name}` is a `bool`, which renders `true`/`false` under \
                                     `dec` and has no `hex` rendering"
                                ),
                            ));
                        }
                    } else if kind != ValueType::Scalar(Scalar::I32) {
                        return Err(failure(
                            span,
                            SemanticPass::ComposedTasks,
                            format!(
                                "`{name}` is not an `i32` this entry bound; \
                                 `source.std-io-tasks.v1` renders integer literals and the `i32` \
                                 values the checked task model computes — a copied parameter, a \
                                 `let`-bound literal, a `qcore_buffer_read` of a live buffer, or \
                                 a collected join outcome"
                            ),
                        ));
                    }
                    Some(id.0 as usize)
                }
            },
        };
        self.sites.push(TaskIoSite {
            function: self.function,
            block: self.current,
            index: self.instructions.len(),
            event,
            operand,
            span: core_span(span),
        });
        Ok(())
    }

    fn bad(&self, span: Span, message: impl Into<String>) -> SourceDiagnostic {
        failure(span, SemanticPass::CEmission, message)
    }
    fn temporary(&mut self, span: Span) -> Result<usize, SourceDiagnostic> {
        let slot = self.next_temporary;
        self.next_temporary = slot
            .checked_add(1)
            .ok_or_else(|| self.bad(span, "task slot identity space exhausted"))?;
        Ok(slot)
    }
    fn block_id(&mut self, span: Span) -> Result<usize, SourceDiagnostic> {
        let block = self.next_block;
        self.next_block = block
            .checked_add(1)
            .ok_or_else(|| self.bad(span, "task block identity space exhausted"))?;
        Ok(block)
    }
    fn instruction(&mut self, span: Span, kind: TaskInstructionKind) {
        self.instructions.push(TaskInstruction {
            span: core_span(span),
            kind,
        });
    }
    fn finish(&mut self, span: Span, kind: TaskTerminatorKind) {
        self.blocks.insert(
            self.current,
            TaskBlock {
                instructions: std::mem::take(&mut self.instructions),
                terminator: TaskTerminator {
                    span: core_span(span),
                    kind,
                },
            },
        );
    }
    fn exit_instructions(&self, span: Span) -> Vec<TaskInstruction> {
        self.scopes
            .iter()
            .rev()
            .map(|scope| TaskInstruction {
                span: core_span(span),
                kind: TaskInstructionKind::ExitScope { scope: *scope },
            })
            .collect()
    }
    fn error_block(&mut self, span: Span, error: ErrorOperand) -> Result<usize, SourceDiagnostic> {
        let block = self.block_id(span)?;
        self.blocks.insert(
            block,
            TaskBlock {
                instructions: self.exit_instructions(span),
                terminator: TaskTerminator {
                    span: core_span(span),
                    // Every synthesized raise is payload-free: the two builtin
                    // failures hold nothing, and `?` is refused for a Result
                    // whose error can carry a payload.
                    kind: TaskTerminatorKind::Raise {
                        error,
                        payload: None,
                    },
                },
            },
        );
        Ok(block)
    }
    fn lookup(&self, name: &str, span: Span) -> Result<(SymbolId, ValueType), SourceDiagnostic> {
        self.environment
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .cloned()
            .ok_or_else(|| {
                failure(
                    span,
                    SemanticPass::NameResolution,
                    "value is not a live local or parameter in this task function",
                )
            })
    }
    fn lookup_place(
        &self,
        place: &ResolvedTaskPlace,
    ) -> Result<(SymbolId, ValueType), SourceDiagnostic> {
        // Card `W48-QR-TASK-RECORD-CAPTURES`: a field of a record binding is a
        // place — its leaf's slot, or a sub-record's leaves.
        if let SourcePlace::Field { .. } = &place.syntax.value
            && let Some((root, path)) = records::place_path(&place.syntax.value)
            && matches!(
                self.lookup(&root.value, root.span),
                Ok((_, ValueType::Record(_)))
            )
        {
            let (found, _, target) = self.resolve_path(&root.value, &path, place.syntax.span)?;
            if found != place.root_symbol {
                return Err(self.bad(
                    root.span,
                    "task place identity disagrees with the lexical value binding",
                ));
            }
            return Ok(match target {
                PathTarget::Leaf(leaf) => {
                    (SymbolId(leaf.slot as u32), ValueType::Scalar(leaf.kind))
                }
                PathTarget::Record { ty, leaves } => (
                    found,
                    ValueType::Record(RecordValue {
                        ty,
                        leaves,
                        state: RecordState::Live,
                    }),
                ),
            });
        }
        let SourcePlace::Name(name) = &place.syntax.value else {
            return Err(self.bad(
                place.syntax.span,
                "task execution supports direct local/parameter places only",
            ));
        };
        let found = self.lookup(&name.value, name.span)?;
        if found.0 != place.root_symbol {
            return Err(self.bad(
                name.span,
                "task place identity disagrees with the lexical value binding",
            ));
        }
        Ok(found)
    }

    /// The initializer of `qcore_buffer_new(VALUE)` and the payload type it
    /// names (card `W48-QR-SCALAR-TYPES-R6`): the initializer's own type — an
    /// `i32` literal or binding as before, or a binding of any other §G02t
    /// type. The payload type is named at acquisition, by the value.
    fn allocation_input(
        &self,
        tokens: &[Token],
        span: Span,
    ) -> Result<Option<(ScalarOperand, FixedType)>, SourceDiagnostic> {
        if !word(tokens.first(), "qcore_buffer_new") {
            return Ok(None);
        }
        if !self.signature.effects.alloc {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                "buffer allocation requires alloc effect",
            ));
        }
        if !symbol(tokens.get(1), Symbol::LeftParen)
            || matching(tokens, 1, Symbol::LeftParen, Symbol::RightParen)
                != Some(tokens.len().saturating_sub(1))
        {
            return Err(self.bad(
                span,
                "buffer constructor requires one complete i32 argument",
            ));
        }
        let (initial, kind) = self.scalar_operand(&tokens[2..tokens.len() - 1], span)?;
        let payload = match kind {
            Scalar::I32 => FixedType::I32,
            Scalar::Fixed(ty) => ty,
            _ => return Err(self.bad(span, "buffer constructor requires i32")),
        };
        Ok(Some((initial, payload)))
    }

    fn buffer_initializer(
        &mut self,
        tokens: &[Token],
        span: Span,
        slot: usize,
    ) -> Result<Option<ValueType>, SourceDiagnostic> {
        if word(tokens.first(), "qcore_buffer_new") {
            if !symbol(tokens.last(), Symbol::Question) {
                return Err(self.bad(
                    span,
                    "buffer allocation requires ? or explicit Result match",
                ));
            }
            let (initial, payload) = self
                .allocation_input(&tokens[..tokens.len() - 1], span)?
                .unwrap();
            if !self.signature.effects.errors.contains("AllocError") {
                return Err(failure(
                    span,
                    SemanticPass::TypedErrors,
                    "allocation propagation requires throws<AllocError>",
                ));
            }
            let success = self.block_id(span)?;
            let allocation_error =
                self.error_block(span, ErrorOperand::Tag("AllocError".into()))?;
            self.finish(
                span,
                TaskTerminatorKind::TryAllocateBuffer {
                    initial,
                    value_slot: slot,
                    success,
                    allocation_error,
                },
            );
            self.current = success;
            return Ok(Some(ValueType::Scalar(Scalar::OwnedBuffer(payload))));
        }
        // Card `W48-QR-TASK-RECORD-CAPTURES`: a whole read or write of an owned
        // field, `qcore_buffer_read(RECORD.PATH)` and
        // `qcore_buffer_write(RECORD.PATH, VALUE)`, on the field's leaf.
        if let Some(kind) = self.field_buffer_access(tokens, span, slot)? {
            return Ok(Some(kind));
        }
        if word(tokens.first(), "qcore_buffer_write") {
            // `qcore_buffer_write(BUFFER, VALUE)` evaluates to `unit`. Only an
            // owner can name the buffer: a read loan is not an `own<Buffer>`
            // binding, so a borrower cannot reach this form at all, and the
            // ledger separately refuses the write while any share is out.
            //
            // Card `WRITE-LEND-TASKS-EXERCISE` adds the **three**-argument
            // spelling `qcore_buffer_write(LEASE, INDEX, VALUE)`, the
            // `borrow_utf8` arity precedent: one byte of the payload, through
            // a range write lease, at an index relative to that lease's own
            // low. The arity is what selects the form, so no word is
            // reserved and the two-argument owner write is untouched.
            let commas = argument_commas(tokens);
            if commas.len() == 2 {
                return self.lease_byte_write(tokens, span, slot, &commas);
            }
            let comma = tokens
                .iter()
                .position(|token| symbol(Some(token), Symbol::Comma))
                .ok_or_else(|| {
                    self.bad(
                        span,
                        "buffer write requires one owned-buffer name and one i32 value",
                    )
                })?;
            if comma < 3
                || !symbol(tokens.get(1), Symbol::LeftParen)
                || matching(tokens, 1, Symbol::LeftParen, Symbol::RightParen)
                    != Some(tokens.len().saturating_sub(1))
            {
                return Err(self.bad(
                    span,
                    "buffer write requires one owned-buffer name and one i32 value",
                ));
            }
            let target = name(tokens.get(2))
                .filter(|_| comma == 3)
                .ok_or_else(|| self.bad(span, "buffer write requires an exact buffer name"))?;
            let (id, kind) = self.lookup(target, span)?;
            let ValueType::Scalar(Scalar::OwnedBuffer(payload)) = kind else {
                return Err(self.bad(
                    span,
                    "buffer write requires own<Buffer>; a read loan grants no write authority",
                ));
            };
            let value_tokens = &tokens[comma + 1..tokens.len() - 1];
            // Card `W48-QR-SCALAR-TYPES-R6`: the value is exactly the payload
            // type — a literal takes it (range-checked), a binding must have
            // it; an `i32` payload is unchanged.
            let (value, value_kind) = if payload != FixedType::I32
                && let Some(literal) = self.fixed_literal(value_tokens, payload, span)
            {
                (literal?, Scalar::Fixed(payload))
            } else {
                self.scalar_operand(value_tokens, span)?
            };
            if value_kind != Scalar::of(payload) {
                return Err(self.bad(
                    span,
                    if payload == FixedType::I32 {
                        "buffer write requires an i32 value".to_owned()
                    } else {
                        format!(
                            "a write into an `own<Buffer<{payload}>>` requires a `{payload}` \
                             value: the payload type is part of the Buffer's type and nothing \
                             converts implicitly"
                        )
                    },
                ));
            }
            self.instruction(
                span,
                TaskInstructionKind::WriteBuffer {
                    buffer: id.0 as usize,
                    value,
                },
            );
            self.instruction(
                span,
                TaskInstructionKind::BindScalar {
                    slot,
                    value: ScalarOperand::Unit,
                },
            );
            return Ok(Some(ValueType::Scalar(Scalar::Unit)));
        }
        if word(tokens.first(), "qcore_buffer_read") {
            // The two-argument spelling `qcore_buffer_read(NAME, INDEX)` reads
            // one byte (card `WRITE-LEND-TASKS-EXERCISE`) — through a lease,
            // relative to its low, or through an owner, where the base is 0
            // and the whole-payload read's own authority gate applies.
            let commas = argument_commas(tokens);
            if commas.len() == 1 {
                return self.byte_read(tokens, span, slot, commas[0]);
            }
            if tokens.len() != 4
                || !symbol(tokens.get(1), Symbol::LeftParen)
                || !symbol(tokens.get(3), Symbol::RightParen)
            {
                return Err(self.bad(span, "buffer read requires one exact owned-buffer name"));
            }
            let buffer =
                name(tokens.get(2)).ok_or_else(|| self.bad(span, "buffer read requires a name"))?;
            let (id, kind) = self.lookup(buffer, span)?;
            // A read is the one operation a loan admits, and the one operation
            // an owner keeps while its buffer is lent out.
            let ValueType::Scalar(Scalar::OwnedBuffer(payload) | Scalar::ReadLoan(payload)) = kind
            else {
                return Err(self.bad(span, "buffer read requires own<Buffer> or borrow<Buffer>"));
            };
            self.instruction(
                span,
                TaskInstructionKind::ReadBuffer {
                    buffer: id.0 as usize,
                    value_slot: slot,
                },
            );
            // Card `W48-QR-SCALAR-TYPES-R6`: a value of the payload type.
            return Ok(Some(ValueType::Scalar(Scalar::of(payload))));
        }
        // Card T06. `qcore_error_payload(e)` moves the owned Buffer out of a
        // bound typed error and into a new owned binding. It is the only way a
        // payload leaves an error, it works at most once per binding, and the
        // error keeps its tag afterwards so it can still be inspected.
        if word(tokens.first(), "qcore_error_payload") {
            if tokens.len() != 4
                || !symbol(tokens.get(1), Symbol::LeftParen)
                || !symbol(tokens.get(3), Symbol::RightParen)
            {
                return Err(self.bad(
                    span,
                    "taking an error payload requires one exact bound error name",
                ));
            }
            let binding = name(tokens.get(2))
                .ok_or_else(|| self.bad(span, "taking an error payload requires a name"))?;
            let (id, kind) = self.lookup(binding, span)?;
            let ValueType::Error {
                errors,
                payload: true,
            } = kind
            else {
                return Err(self.bad(
                    span,
                    "qcore_error_payload requires a bound error that still holds an owned payload",
                ));
            };
            self.instruction(
                span,
                TaskInstructionKind::TakeErrorPayload {
                    error_slot: id.0 as usize,
                    buffer_slot: slot,
                },
            );
            // The binding now names a payload-free error. Rebinding it here is
            // what makes a second take, and a silent re-raise, impossible.
            self.rebind(
                binding,
                ValueType::Error {
                    errors,
                    payload: false,
                },
            );
            return Ok(Some(ValueType::Scalar(Scalar::OWNED_BUFFER)));
        }
        Ok(None)
    }

    /// `let NAME = …;` of a record (card `W48-QR-TASK-RECORD-CAPTURES`): a
    /// construction `TYPE { … }`, a field read `RECORD.PATH`, or a copy of a
    /// record of scalars `RECORD`. `None` when the initializer is none of them.
    fn record_initializer(
        &mut self,
        rhs: &[Token],
        span: Span,
        binding: SymbolId,
    ) -> Result<Option<ValueType>, SourceDiagnostic> {
        if let Some(ty) = name(rhs.first())
            && symbol(rhs.get(1), Symbol::LeftBrace)
            && self.records.contains_key(ty)
        {
            let ty = ty.to_owned();
            return Ok(Some(ValueType::Record(
                self.construct_record(rhs, span, &ty)?,
            )));
        }
        let Some((root, path, after)) = records::token_path(rhs, 0) else {
            return Ok(None);
        };
        if after != rhs.len() || !matches!(self.lookup(&root, span), Ok((_, ValueType::Record(_))))
        {
            return Ok(None);
        }
        if path.is_empty() {
            return self.record_copy(&root, span).map(Some);
        }
        self.field_read(&root, &path, span, binding).map(Some)
    }

    /// `qcore_buffer_read(RECORD.PATH)` and `qcore_buffer_write(RECORD.PATH,
    /// VALUE)` on an owned field (card `W48-QR-TASK-RECORD-CAPTURES`): the
    /// whole-payload read and write of the field's leaf, with the owner's
    /// authority gates the core applies to any owned Buffer.
    fn field_buffer_access(
        &mut self,
        tokens: &[Token],
        span: Span,
        slot: usize,
    ) -> Result<Option<ValueType>, SourceDiagnostic> {
        let reading = word(tokens.first(), "qcore_buffer_read");
        if !(reading || word(tokens.first(), "qcore_buffer_write"))
            || !symbol(tokens.get(1), Symbol::LeftParen)
        {
            return Ok(None);
        }
        let Some((root, path, after)) = records::token_path(tokens, 2) else {
            return Ok(None);
        };
        if path.is_empty() || !matches!(self.lookup(&root, span), Ok((_, ValueType::Record(_)))) {
            return Ok(None);
        }
        let spelled = format!("{root}.{}", path.join("."));
        let (_, _, target) = self.resolve_path(&root, &path, span)?;
        let PathTarget::Leaf(leaf) = target else {
            return Err(self.bad(
                span,
                format!("`{spelled}` is a sub-record, not an owned field"),
            ));
        };
        let Scalar::OwnedBuffer(payload) = leaf.kind else {
            return Err(self.bad(
                span,
                format!("`{spelled}` is a scalar field; read it with `let NAME = {spelled};`"),
            ));
        };
        if reading {
            if after + 1 != tokens.len() || !symbol(tokens.get(after), Symbol::RightParen) {
                return Err(self.bad(span, "buffer read requires one exact owned-buffer name"));
            }
            self.instruction(
                span,
                TaskInstructionKind::ReadBuffer {
                    buffer: leaf.slot,
                    value_slot: slot,
                },
            );
            return Ok(Some(ValueType::Scalar(Scalar::of(payload))));
        }
        if !symbol(tokens.get(after), Symbol::Comma)
            || !symbol(tokens.last(), Symbol::RightParen)
            || after + 2 >= tokens.len()
        {
            return Err(self.bad(
                span,
                "buffer write requires one owned-buffer name and one value",
            ));
        }
        let value_tokens = &tokens[after + 1..tokens.len() - 1];
        let (value, value_kind) = if payload != FixedType::I32
            && let Some(literal) = self.fixed_literal(value_tokens, payload, span)
        {
            (literal?, Scalar::Fixed(payload))
        } else {
            self.scalar_operand(value_tokens, span)?
        };
        if value_kind != Scalar::of(payload) {
            return Err(self.bad(
                span,
                format!(
                    "a write into `{spelled}` requires {} value: the payload type is part of the \
                     Buffer's type and nothing converts implicitly",
                    fixed_spelling(payload)
                ),
            ));
        }
        self.instruction(
            span,
            TaskInstructionKind::WriteBuffer {
                buffer: leaf.slot,
                value,
            },
        );
        self.instruction(
            span,
            TaskInstructionKind::BindScalar {
                slot,
                value: ScalarOperand::Unit,
            },
        );
        Ok(Some(ValueType::Scalar(Scalar::Unit)))
    }

    /// Replace the innermost binding of `name` in place. Only the payload
    /// state of a typed error changes this way, and only from held to taken.
    fn rebind(&mut self, name: &str, kind: ValueType) {
        for scope in self.environment.iter_mut().rev() {
            if let Some(entry) = scope.get_mut(name) {
                entry.1 = kind;
                return;
            }
        }
        unreachable!("rebinding follows a successful lookup of the same name")
    }

    /// `qcore_buffer_write(LEASE, INDEX, VALUE)` — one byte of the payload,
    /// through a range write lease. The lease is the only authority this form
    /// admits: the owner's whole-payload write remains its only write
    /// footprint, so wave 31's "the owner holds no whole-payload authority
    /// while a lease is out" rule is inherited unchanged rather than restated
    /// against a second owner-side footprint.
    fn lease_byte_write(
        &mut self,
        tokens: &[Token],
        span: Span,
        slot: usize,
        commas: &[usize],
    ) -> Result<Option<ValueType>, SourceDiagnostic> {
        let close = matching(tokens, 1, Symbol::LeftParen, Symbol::RightParen)
            .filter(|close| *close == tokens.len().saturating_sub(1))
            .ok_or_else(|| {
                self.bad(
                    span,
                    "a byte write requires one lend<Buffer> name, one byte index and one \
                     i32 value",
                )
            })?;
        let target = name(tokens.get(2))
            .filter(|_| commas[0] == 3)
            .ok_or_else(|| self.bad(span, "a byte write requires an exact lease name"))?;
        let (id, kind) = self.lookup(target, span)?;
        if !matches!(kind, ValueType::Scalar(Scalar::WriteLease(_))) {
            return Err(self.bad(
                span,
                "a byte write requires lend<Buffer>: the owner's write footprint is the \
                 whole payload, and a read share grants no write authority",
            ));
        }
        let (index, index_kind) = self.scalar_operand(&tokens[commas[0] + 1..commas[1]], span)?;
        if index_kind != Scalar::I32 {
            return Err(self.bad(span, "a byte index must be an i32"));
        }
        let (value, value_kind) = self.scalar_operand(&tokens[commas[1] + 1..close], span)?;
        if value_kind != Scalar::I32 {
            return Err(self.bad(span, "a byte write requires an i32 value"));
        }
        self.instruction(
            span,
            TaskInstructionKind::WriteBufferByte {
                authority: id.0 as usize,
                index,
                value,
            },
        );
        self.instruction(
            span,
            TaskInstructionKind::BindScalar {
                slot,
                value: ScalarOperand::Unit,
            },
        );
        Ok(Some(ValueType::Scalar(Scalar::Unit)))
    }

    /// `qcore_buffer_read(NAME, INDEX)` — one byte, through a lease or through
    /// the owner. A read share is refused by name: giving a loan a byte view
    /// is the named residual `WRITE-LEND-EXERCISE-LOAN-BYTE-READ`.
    fn byte_read(
        &mut self,
        tokens: &[Token],
        span: Span,
        slot: usize,
        comma: usize,
    ) -> Result<Option<ValueType>, SourceDiagnostic> {
        let close = matching(tokens, 1, Symbol::LeftParen, Symbol::RightParen)
            .filter(|close| *close == tokens.len().saturating_sub(1))
            .ok_or_else(|| {
                self.bad(
                    span,
                    "a byte read requires one exact name and one byte index",
                )
            })?;
        let target = name(tokens.get(2))
            .filter(|_| comma == 3)
            .ok_or_else(|| self.bad(span, "a byte read requires an exact name"))?;
        let (id, kind) = self.lookup(target, span)?;
        if !matches!(
            kind,
            ValueType::Scalar(Scalar::OwnedBuffer(_) | Scalar::WriteLease(_))
        ) {
            return Err(self.bad(
                span,
                "a byte read requires own<Buffer> or lend<Buffer>; a read share admits the \
                 whole-payload read only",
            ));
        }
        let (index, index_kind) = self.scalar_operand(&tokens[comma + 1..close], span)?;
        if index_kind != Scalar::I32 {
            return Err(self.bad(span, "a byte index must be an i32"));
        }
        self.instruction(
            span,
            TaskInstructionKind::ReadBufferByte {
                authority: id.0 as usize,
                index,
                value_slot: slot,
            },
        );
        Ok(Some(ValueType::Scalar(Scalar::I32)))
    }

    fn scalar_operand(
        &self,
        tokens: &[Token],
        span: Span,
    ) -> Result<(ScalarOperand, Scalar), SourceDiagnostic> {
        match tokens {
            [
                Token {
                    kind: TokenKind::Integer(value),
                    ..
                },
            ] => value
                .parse::<i32>()
                .map(|value| (ScalarOperand::I32(value), Scalar::I32))
                .map_err(|_| self.bad(span, "integer literal is outside i32")),
            [
                Token {
                    kind: TokenKind::Symbol(Symbol::Minus),
                    ..
                },
                Token {
                    kind: TokenKind::Integer(value),
                    ..
                },
            ] => {
                let magnitude = value
                    .parse::<u64>()
                    .ok()
                    .filter(|n| *n <= 2147483648)
                    .ok_or_else(|| self.bad(span, "negative integer literal is outside i32"))?;
                Ok((
                    ScalarOperand::I32((-(magnitude as i64)) as i32),
                    Scalar::I32,
                ))
            }
            // Card `W48-QR-SCALAR-TYPES-R4`: `true`/`false` are `bool` literals.
            [token] if word(Some(token), "true") || word(Some(token), "false") => Ok((
                ScalarOperand::Fixed {
                    ty: FixedType::Bool,
                    value: i128::from(word(Some(token), "true")),
                },
                Scalar::Fixed(FixedType::Bool),
            )),
            [
                Token {
                    kind: TokenKind::Word(name),
                    ..
                },
            ] => {
                let (symbol, kind) = self.lookup(name, span)?;
                let ValueType::Scalar(scalar) = kind else {
                    return Err(self.bad(
                        span,
                        "a scalar expression cannot copy a handle, result, or scope",
                    ));
                };
                if matches!(scalar, Scalar::OwnedBuffer(_)) {
                    return Err(self.bad(
                        span,
                        "owned buffers require explicit move or a checked buffer read",
                    ));
                }
                if matches!(scalar, Scalar::ReadLoan(_)) {
                    return Err(self.bad(
                        span,
                        "a read loan cannot be copied; it admits only a checked buffer read",
                    ));
                }
                Ok((ScalarOperand::Slot(symbol.0 as usize), scalar))
            }
            _ => Err(self.bad(
                span,
                "task scalar expressions require an i32 literal or immutable scalar binding",
            )),
        }
    }

    fn propagate(
        &mut self,
        span: Span,
        result_slot: usize,
        value_slot: usize,
        value: Shape,
        errors: &BTreeSet<String>,
        error_payload: bool,
    ) -> Result<ValueType, SourceDiagnostic> {
        if !errors.is_subset(&self.signature.effects.errors) {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                "propagated task errors are absent from the function's throws effects",
            ));
        }
        // `?` has no place to name the payload, so it could only discard or
        // silently forward it. Both are refused: the payload has to be taken
        // on an explicit error arm.
        if error_payload {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                "an error that carries an owned payload requires an explicit Result match, not `?`",
            ));
        }
        if errors.is_empty() {
            self.instruction(
                span,
                TaskInstructionKind::UnwrapInfallible {
                    result_slot,
                    value_slot,
                },
            );
            return self.collected(value, value_slot, span);
        }
        let success = self.block_id(span)?;
        let error_slot = self.temporary(span)?;
        let error = self.error_block(span, ErrorOperand::Slot(error_slot))?;
        self.finish(
            span,
            TaskTerminatorKind::TryOutcome {
                result_slot,
                value_slot,
                error_slot,
                success,
                error,
            },
        );
        self.current = success;
        self.collected(value, value_slot, span)
    }

    /// The value a collection bound at `value_slot`: a scalar as before, or a
    /// record unpacked into fresh leaf slots right here (card
    /// `W48-QR-TASK-RECORD-CAPTURES`).
    fn collected(
        &mut self,
        value: Shape,
        value_slot: usize,
        span: Span,
    ) -> Result<ValueType, SourceDiagnostic> {
        match value {
            Shape::Scalar(scalar) => Ok(ValueType::Scalar(scalar)),
            Shape::Record(ty) => Ok(ValueType::Record(
                self.unpack_record(&ty, value_slot, span)?,
            )),
        }
    }

    fn spawn_inputs(
        &self,
        span: Span,
        scope: SymbolId,
        worker: SymbolId,
        captures: &[ResolvedTaskCapture],
    ) -> Result<(u64, Vec<ScalarOperand>, Vec<String>), SourceDiagnostic> {
        if !self.signature.effects.spawn {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                "spawn requires its declared effect",
            ));
        }
        let scope_id = u64::from(scope.0);
        if self.scopes.last() != Some(&scope_id) {
            return Err(self.bad(span, "spawn requires the innermost active thread scope"));
        }
        let signature = self
            .signatures
            .get(&worker)
            .ok_or_else(|| self.bad(span, "unknown source worker"))?;
        // Card `W48-QR-CLOSURES`: a closure's parameter types are decided
        // here, at its spawn site, from the places it captures, while its
        // enclosing function is lowered to find them. Every other check below
        // is the named worker's, unchanged.
        let decided;
        let signature = match &signature.closure {
            Some(closure) if !closure.decided => {
                let Some(discovering) = self.discovering else {
                    return Err(self.bad(
                        span,
                        "a closure's parameter types are decided at its spawn site before it is \
                         lowered",
                    ));
                };
                if captures.len() != closure.binders.len() {
                    return Err(self.bad(span, "worker capture arity mismatch"));
                }
                let shapes = captures
                    .iter()
                    .zip(&closure.binders)
                    .map(|(capture, (_, binder))| self.closure_parameter(capture, binder))
                    .collect::<Result<Vec<_>, _>>()?;
                discovering.borrow_mut().insert(worker, shapes.clone());
                decided = Signature {
                    parameters: closure
                        .binders
                        .iter()
                        .zip(shapes)
                        .map(|((symbol, name), shape)| (*symbol, name.clone(), shape))
                        .collect(),
                    closure: Some(ClosureParameters {
                        binders: closure.binders.clone(),
                        decided: true,
                    }),
                    ..signature.clone()
                };
                &decided
            }
            _ => signature,
        };
        if captures.len() != signature.parameters.len() {
            return Err(self.bad(span, "worker capture arity mismatch"));
        }
        let mut arguments = Vec::new();
        let mut loans = 0usize;
        let mut leases = 0usize;
        // Card `W48-QR-TASK-RECORD-CAPTURES`: records a successful creation
        // moves whole, which its caller marks moved on that path.
        let mut moved_records = Vec::new();
        for (capture, (_, _, parameter_shape)) in captures.iter().zip(&signature.parameters) {
            let parameter_type = match parameter_shape {
                // A record parameter takes a whole record: the aggregate
                // judgment over its leaves, beside the enumeration.
                Shape::Record(parameter) => {
                    let (root, path) = records::place_path(&capture.place.syntax.value)
                        .ok_or_else(|| {
                            self.bad(
                                capture.span,
                                "task execution supports direct local/parameter places only",
                            )
                        })?;
                    let (found, kind) = self.lookup(&root.value, root.span)?;
                    if found != capture.place.root_symbol {
                        return Err(self.bad(
                            root.span,
                            "task place identity disagrees with the lexical value binding",
                        ));
                    }
                    if !matches!(kind, ValueType::Record(_)) {
                        return Err(failure(
                            capture.span,
                            SemanticPass::OrdinaryTypes,
                            "capture type does not match worker parameter",
                        ));
                    }
                    let (operands, moved) =
                        self.record_capture(capture, &root.value, &path, parameter)?;
                    arguments.extend(operands);
                    moved_records.extend(moved);
                    continue;
                }
                Shape::Scalar(scalar) => *scalar,
            };
            let parameter_type = &parameter_type;
            let (symbol, kind) = self.lookup_place(&capture.place)?;
            // A read loan is the one capture whose place type and parameter
            // type differ on purpose: the lender keeps `own<Buffer>` and the
            // borrower receives `borrow<Buffer>`. A write lease (card
            // `WRITE-LEND-TASKS`) is the second: the lessor keeps
            // `own<Buffer>` and the worker receives `lend<Buffer>`.
            // Card `W48-QR-SCALAR-TYPES-R6`: the lent or leased Buffer has the
            // payload type the worker declares — no conversion at the boundary.
            let place_type = match *parameter_type {
                Scalar::ReadLoan(payload) | Scalar::WriteLease(payload) => {
                    Scalar::OwnedBuffer(payload)
                }
                other => other,
            };
            // Card CT46-SENDSYNC. A non-scalar binding has no transferable
            // kind, and the check just below already refuses it. This guard
            // states that agreement rather than assuming it: if the judgment
            // ever called a handle, scope, Result or error value sendable,
            // this refuses instead of letting the type check be the only
            // thing standing between it and a boundary.
            if !matches!(kind, ValueType::Scalar(_))
                && qcore_core::transfer::transferable(binding_transfer_kind(&kind)).sendable()
            {
                return Err(self.bad(
                    capture.span,
                    "the transferability judgment admits a binding that is not a value",
                ));
            }
            if kind != ValueType::Scalar(place_type) {
                return Err(failure(
                    capture.span,
                    SemanticPass::OrdinaryTypes,
                    "capture type does not match worker parameter",
                ));
            }
            // Card CT46-SENDSYNC: the derived judgment, over what the parent
            // holds (`place_type`), the mode asked for, and what the worker
            // declared it receives (`parameter_type`).
            let derived = qcore_core::transfer::admits_capture(
                transfer_kind(place_type),
                transfer_mode(&capture.mode),
                transfer_kind(*parameter_type),
            );
            // The existing enumeration, as a decision rather than an early
            // return, so that the two can be compared on refusals too.
            let enumerated = match (&capture.mode, parameter_type) {
                (SourceCaptureMode::Copy, Scalar::I32 | Scalar::Fixed(_)) => {
                    Some(EnumeratedCapture::Copy)
                }
                (SourceCaptureMode::Move, Scalar::OwnedBuffer(_)) => Some(EnumeratedCapture::Move),
                (SourceCaptureMode::Read { .. }, Scalar::ReadLoan(_)) => {
                    Some(EnumeratedCapture::Lend)
                }
                (SourceCaptureMode::LendWrite { .. }, Scalar::WriteLease(_)) => {
                    Some(EnumeratedCapture::LendWrite)
                }
                _ => None,
            };
            // Both must agree, on the verdict and on which transfer it is.
            // A disagreement in either direction refuses: the judgment cannot
            // admit what the enumeration refuses, and it cannot silently
            // withdraw what the enumeration admits either.
            let agrees = match (&derived, enumerated) {
                (Ok(admitted), Some(form)) => *admitted == form.as_admitted(),
                (Err(_), None) => true,
                _ => false,
            };
            if !agrees {
                return Err(self.bad(
                    capture.span,
                    "the derived transferability judgment and the capture enumeration disagree \
                     about this capture",
                ));
            }
            // Card CT47-COPY-CAPTURE. Where both sides agree the capture is
            // refused, the refusal is the judgment's own, attributed to this
            // capture at the crossing construct with the missing contract
            // named (`Q600`; condition widening stated in spec §L21). Before
            // this card the same shapes fell through to the executable-subset
            // catch-all below — a `CEmission` (`Q500`) statement about the
            // backend rather than about the boundary. The catch-all is kept,
            // unedited, as the fail-closed backstop.
            if let Err(refusal) = derived {
                return Err(failure(
                    capture.span,
                    SemanticPass::Concurrency,
                    transfer_refusal_message(place_type, &capture.mode, *parameter_type, refusal),
                ));
            }
            let operand = match enumerated {
                Some(EnumeratedCapture::Copy) => ScalarOperand::Slot(symbol.0 as usize),
                Some(EnumeratedCapture::Move) => ScalarOperand::Move(symbol.0 as usize),
                Some(EnumeratedCapture::Lend) => {
                    // The share the *borrower* receives. Whether the resource
                    // has that much authority left to give is the ledger's
                    // decision, on the path that reaches this creation.
                    let SourceCaptureMode::Read { share } = &capture.mode else {
                        // Unreachable: only a `read` capture enumerates as a
                        // lend. Refused rather than panicked on.
                        return Err(self.bad(capture.span, "a lend capture without a share"));
                    };
                    loans += 1;
                    ScalarOperand::Loan {
                        slot: symbol.0 as usize,
                        share: read_share(&share.value, share.span)?,
                    }
                }
                Some(EnumeratedCapture::LendWrite) => {
                    // The range the worker receives. Whether it is disjoint
                    // from every outstanding lease is the ledger's decision,
                    // on the path that reaches this creation, by the wave-29
                    // overlap rule; the two decisions made *here* are the
                    // check-time ones — the bounds are literals, the range is
                    // nonempty, and it names bytes the payload has.
                    let SourceCaptureMode::LendWrite { low, high } = &capture.mode else {
                        // Unreachable: only a `lend` capture enumerates as a
                        // write lend. Refused rather than panicked on.
                        return Err(self.bad(capture.span, "a write lend capture without bounds"));
                    };
                    let low_value = lease_bound_value(low)?;
                    let high_value = lease_bound_value(high)?;
                    if low_value >= high_value {
                        return Err(failure(
                            capture.span,
                            SemanticPass::Concurrency,
                            format!(
                                "a write lend must cover at least one byte: \
                                 [{low_value}, {high_value}) is empty"
                            ),
                        ));
                    }
                    // Card `W48-QR-SCALAR-TYPES-R6`: the extent is the payload
                    // type's size — never an assumed four. An `i32` payload
                    // keeps its message byte for byte.
                    let payload = place_type.payload().unwrap_or(FixedType::I32);
                    let extent = qcore_core::task_execution::payload_extent(payload, self.target)
                        .ok_or_else(|| {
                        failure(
                            capture.span,
                            SemanticPass::OrdinaryTypes,
                            qcore_core::scalar_values::ValueRefusal::NoRecordedTarget {
                                ty: payload,
                            }
                            .to_string(),
                        )
                    })?;
                    if high_value > extent {
                        return Err(failure(
                            capture.span,
                            SemanticPass::Concurrency,
                            if payload == FixedType::I32 {
                                format!(
                                    "a write lend must stay inside the payload's {} bytes: \
                                     [{low_value}, {high_value}) ends past it",
                                    qcore_core::task_execution::BUFFER_PAYLOAD_BYTES
                                )
                            } else {
                                format!(
                                    "a write lend must stay inside the `{payload}` payload's \
                                     {extent} byte(s): [{low_value}, {high_value}) ends past it"
                                )
                            },
                        ));
                    }
                    leases += 1;
                    ScalarOperand::Lease {
                        slot: symbol.0 as usize,
                        low: low_value,
                        high: high_value,
                    }
                }
                None => {
                    return Err(self.bad(
                        capture.span,
                        "only copy i32, move own<Buffer>, read own<Buffer> share FULL and lend own<Buffer> LOW..HIGH captures are executable",
                    ));
                }
            };
            arguments.push(operand);
        }
        if loans > 1 {
            return Err(self.bad(
                span,
                "this profile admits at most one read loan per task creation",
            ));
        }
        if leases > 1 {
            return Err(self.bad(
                span,
                "this profile admits at most one write lease per task creation",
            ));
        }
        if loans != 0 && leases != 0 {
            return Err(self.bad(
                span,
                "one creation cannot both lend a read share and lease a write range",
            ));
        }
        Ok((scope_id, arguments, moved_records))
    }

    /// The parameter type a closure's capture binds (card `W48-QR-CLOSURES`):
    /// the captured place's own type — a record's record type, a scalar's
    /// scalar type, an owned Buffer's owned Buffer — except that `read … share`
    /// and `lend … LOW..HIGH` of an owned Buffer deliver its loan and its
    /// lease, exactly the types a hand-written worker declares for them. So
    /// the closure's worker receives what a twin receives, and the
    /// transferability judgment decides the capture against it unchanged: a
    /// mode it refuses (`copy` of an owned Buffer, `move` of a scalar, `write`)
    /// is refused with the judgment's own reason. A place that is not a value
    /// — a handle, a Result, a scope, an error, `unit` — is a type no worker
    /// parameter can declare, and refuses here, naming the capture.
    fn closure_parameter(
        &self,
        capture: &ResolvedTaskCapture,
        binder: &str,
    ) -> Result<Shape, SourceDiagnostic> {
        let (_, kind) = self.lookup_place(&capture.place)?;
        let what = match kind {
            ValueType::Record(record) => return Ok(Shape::Record(record.ty)),
            ValueType::Scalar(Scalar::Unit) => "a `unit` value",
            ValueType::Scalar(scalar) => {
                return Ok(Shape::Scalar(match (&capture.mode, scalar) {
                    (SourceCaptureMode::Read { .. }, Scalar::OwnedBuffer(payload)) => {
                        Scalar::ReadLoan(payload)
                    }
                    (SourceCaptureMode::LendWrite { .. }, Scalar::OwnedBuffer(payload)) => {
                        Scalar::WriteLease(payload)
                    }
                    _ => scalar,
                }));
            }
            ValueType::Handle(_) => "a task handle",
            ValueType::Result { .. } => "an undiscriminated Result",
            ValueType::Scope => "a thread scope",
            ValueType::KnownError(_) | ValueType::Error { .. } => "an error value",
        };
        Err(failure(
            capture.span,
            SemanticPass::Concurrency,
            format!(
                "the closure's capture `{binder}` names {what}, which no worker parameter can \
                 receive: a closure captures values — a scalar, an owned Buffer (moved, lent or \
                 leased) or a record"
            ),
        ))
    }

    /// Mark each record a successful creation moved whole (card
    /// `W48-QR-TASK-RECORD-CAPTURES`).
    fn mark_moved(&mut self, moved: &[String]) {
        for name in moved {
            if let Ok((_, ValueType::Record(mut record))) =
                self.lookup(name, Span { start: 0, end: 0 })
            {
                record.state = RecordState::Moved;
                self.rebind(name, ValueType::Record(record));
            }
        }
    }

    /// After a `match`, a record visible outside it takes the state every
    /// continuing arm left it in, and is unsettled where they disagree (card
    /// `W48-QR-TASK-RECORD-CAPTURES`, `W48-QR-TASK-RECORD-ARM-MERGE`).
    fn settle_records(&mut self, continued: &[Vec<BTreeMap<String, (SymbolId, ValueType)>>]) {
        if continued.is_empty() {
            return;
        }
        for depth in 0..self.environment.len() {
            let names: Vec<String> = self.environment[depth]
                .iter()
                .filter(|(_, (_, kind))| matches!(kind, ValueType::Record(_)))
                .map(|(name, _)| name.clone())
                .collect();
            for name in names {
                let states: Vec<Option<&(SymbolId, ValueType)>> = continued
                    .iter()
                    .map(|environment| environment.get(depth).and_then(|scope| scope.get(&name)))
                    .collect();
                let first = states[0].cloned();
                let settled = if states.iter().all(|state| state.cloned() == first) {
                    first
                } else {
                    None
                };
                let entry = self.environment[depth]
                    .get_mut(&name)
                    .expect("the name was listed from this scope");
                match settled {
                    Some(value) => *entry = value,
                    None => {
                        if let ValueType::Record(record) = &mut entry.1 {
                            record.state = RecordState::Unsettled;
                        }
                    }
                }
            }
        }
    }

    fn result_match(
        &mut self,
        statement: &TypedStatement,
        tokens: &[Token],
    ) -> Result<bool, SourceDiagnostic> {
        let span = statement.span;
        let resolved = statement
            .result_match
            .as_ref()
            .ok_or_else(|| self.bad(span, "missing resolved Result match metadata"))?;
        let syntax = &resolved.syntax;
        if syntax.span != span
            || resolved.arms.len() != 2
            || syntax.arms.len() != 2
            || !word(tokens.first(), "match")
            || tokens.get(1).map(|t| t.span.start) != Some(syntax.scrutinee_span.start)
        {
            return Err(self.bad(
                span,
                "Result match requires one complete scrutinee and exactly two canonical arms",
            ));
        }
        let open = tokens
            .iter()
            .position(|t| t.span.start >= syntax.scrutinee_span.end)
            .ok_or_else(|| self.bad(span, "missing Result match body"))?;
        if open < 2
            || tokens[open - 1].span.end != self.source.visible_end(span, syntax.scrutinee_span.end)
            || matching(tokens, open, Symbol::LeftBrace, Symbol::RightBrace)
                != Some(tokens.len() - 1)
        {
            return Err(self.bad(
                span,
                "Result match must consume its complete expression and outer body",
            ));
        }
        let mut cursor = open + 1;
        let mut ok = None;
        let mut err = None;
        for (index, arm) in resolved.arms.iter().enumerate() {
            if arm.syntax != syntax.arms[index] {
                return Err(self.bad(span, "Result arm metadata disagrees with source"));
            }
            let pattern = arm.syntax.pattern.as_ref().ok_or_else(|| self.bad(arm.syntax.pattern_span, "only exact Result::Ok { value: name } and Result::Err { error: name } patterns are supported"))?;
            let binder = arm.binding.ok_or_else(|| {
                self.bad(
                    arm.syntax.pattern_span,
                    "missing resolved Result arm binder",
                )
            })?;
            let end = tokens
                .iter()
                .position(|t| t.span.end == arm.syntax.pattern_span.end)
                .ok_or_else(|| self.bad(span, "Result pattern boundary is unavailable"))?;
            if tokens.get(cursor).map(|t| t.span.start) != Some(arm.syntax.pattern_span.start)
                || end < cursor
                || !symbol(tokens.get(end + 1), Symbol::FatArrow)
                || tokens.get(end + 2).map(|t| t.span.start) != Some(arm.syntax.body.span.start)
            {
                return Err(self.bad(span, "unsupported tokens around Result match arm"));
            }
            let body_end = matching(tokens, end + 2, Symbol::LeftBrace, Symbol::RightBrace)
                .ok_or_else(|| self.bad(span, "invalid Result arm body"))?;
            if tokens[body_end].span.end != arm.syntax.body.span.end {
                return Err(self.bad(span, "Result arm body boundary mismatch"));
            }
            cursor = body_end + 1;
            let slot = match pattern.kind {
                ResultPatternKind::Ok => &mut ok,
                ResultPatternKind::Err => &mut err,
            };
            if slot.replace((index, binder)).is_some() {
                return Err(self.bad(arm.syntax.pattern_span, "duplicate Result match variant"));
            }
        }
        if cursor != tokens.len() - 1 {
            return Err(self.bad(span, "unsupported trailing Result match tokens"));
        }
        let (ok_index, ok_binder) =
            ok.ok_or_else(|| self.bad(span, "Result match lacks Ok arm"))?;
        let (err_index, err_binder) =
            err.ok_or_else(|| self.bad(span, "Result match lacks Err arm"))?;
        let success = self.block_id(span)?;
        let error = self.block_id(span)?;
        // Card `W48-QR-TASK-RECORD-CAPTURES`: the records a successful creation
        // moves (moved on the `Ok` arm only), and the record type the `Ok`
        // arm's binder unpacks, when the value is a record.
        let mut ok_moves = Vec::new();
        let mut ok_record: Option<String> = None;
        let (ok_type, err_type) = match resolved.scrutinee.as_ref() {
            Some(ResolvedResultMatchScrutinee::Task(task)) => {
                if task.syntax.propagates_error || task.syntax.span != syntax.scrutinee_span {
                    return Err(self.bad(
                        span,
                        "Result match requires the whole unpropagated task expression",
                    ));
                }
                match &task.kind {
                    ResolvedTaskExpressionKind::Spawn {
                        scope,
                        worker,
                        captures,
                    } => {
                        let (scope, arguments, moved) =
                            self.spawn_inputs(span, *scope, *worker, captures)?;
                        ok_moves = moved;
                        self.finish(
                            span,
                            TaskTerminatorKind::TrySpawn {
                                scope,
                                worker: worker.0 as usize,
                                arguments,
                                handle_slot: ok_binder.0 as usize,
                                success,
                                spawn_error: error,
                            },
                        );
                        (
                            ValueType::Handle(*worker),
                            ValueType::KnownError("SpawnError".into()),
                        )
                    }
                    ResolvedTaskExpressionKind::Join { .. } => {
                        let result_slot = self.temporary(span)?;
                        let mut scrutinee_statement = statement.clone();
                        scrutinee_statement.task_expressions = vec![(**task).clone()];
                        let kind = self
                            .task_expression(
                                &scrutinee_statement,
                                &tokens[1..open],
                                Some(result_slot),
                            )?
                            .ok_or_else(|| self.bad(span, "join match requires a Result value"))?;
                        let ValueType::Result {
                            value,
                            errors,
                            error_payload,
                        } = kind
                        else {
                            return Err(self.bad(span, "join match scrutinee is not a Result"));
                        };
                        self.finish(
                            span,
                            TaskTerminatorKind::TryOutcome {
                                result_slot,
                                value_slot: ok_binder.0 as usize,
                                error_slot: err_binder.0 as usize,
                                success,
                                error,
                            },
                        );
                        (
                            self.arm_value(value, &mut ok_record),
                            ValueType::Error {
                                errors,
                                payload: error_payload,
                            },
                        )
                    }
                }
            }
            Some(ResolvedResultMatchScrutinee::Value(symbol_id)) => {
                let [token] = &tokens[1..open] else {
                    return Err(
                        self.bad(span, "Result match requires one immutable Result binding")
                    );
                };
                let spelling =
                    name(Some(token)).ok_or_else(|| self.bad(span, "invalid Result binding"))?;
                let (found, kind) = self.lookup(spelling, syntax.scrutinee_span)?;
                if found != *symbol_id {
                    return Err(self.bad(span, "Result binding identity mismatch"));
                }
                let ValueType::Result {
                    value,
                    errors,
                    error_payload,
                } = kind
                else {
                    return Err(self.bad(span, "match requires an initialized task Result"));
                };
                self.finish(
                    span,
                    TaskTerminatorKind::TryOutcome {
                        result_slot: found.0 as usize,
                        value_slot: ok_binder.0 as usize,
                        error_slot: err_binder.0 as usize,
                        success,
                        error,
                    },
                );
                (
                    self.arm_value(value, &mut ok_record),
                    ValueType::Error {
                        errors,
                        payload: error_payload,
                    },
                )
            }
            None => {
                let scrutinee = tokens_for(self.source, syntax.scrutinee_span);
                let (initial, payload) = self
                    .allocation_input(&scrutinee, span)?
                    .ok_or_else(|| self.bad(span, "unsupported Result match scrutinee"))?;
                self.finish(
                    span,
                    TaskTerminatorKind::TryAllocateBuffer {
                        initial,
                        value_slot: ok_binder.0 as usize,
                        success,
                        allocation_error: error,
                    },
                );
                (
                    ValueType::Scalar(Scalar::OwnedBuffer(payload)),
                    ValueType::KnownError("AllocError".into()),
                )
            }
        };
        let outer_environment = self.environment.clone();
        let outer_scopes = self.scopes.clone();
        let mut continuation = None;
        let mut continued = Vec::new();
        for (index, arm) in resolved.arms.iter().enumerate() {
            let (target, binder, mut kind) = if index == ok_index {
                (success, ok_binder, ok_type.clone())
            } else {
                debug_assert_eq!(index, err_index);
                (error, err_binder, err_type.clone())
            };
            self.current = target;
            self.environment = outer_environment.clone();
            self.scopes = outer_scopes.clone();
            if index == ok_index {
                // Card `W48-QR-TASK-RECORD-CAPTURES`: on the arm on which the
                // creation succeeded the records it moved are moved, and a
                // collected record is unpacked before the arm's first statement.
                self.mark_moved(&ok_moves);
                if let Some(ty) = &ok_record {
                    kind = ValueType::Record(self.unpack_record(ty, binder.0 as usize, span)?);
                }
            }
            self.environment.push(BTreeMap::from([(
                arm.syntax.pattern.as_ref().unwrap().binding.value.clone(),
                (binder, kind),
            )]));
            let arm_statements = statement
                .nested
                .get(arm.statement_range.clone())
                .ok_or_else(|| self.bad(span, "Result arm statement range is invalid"))?;
            let terminated = self.block(&arm.syntax.body, arm_statements)?;
            self.environment.pop();
            if !terminated {
                continued.push(self.environment.clone());
                let join = match continuation {
                    Some(id) => id,
                    None => {
                        let id = self.block_id(span)?;
                        continuation = Some(id);
                        id
                    }
                };
                self.finish(span, TaskTerminatorKind::Goto { target: join });
            }
        }
        self.environment = outer_environment;
        self.scopes = outer_scopes;
        self.settle_records(&continued);
        if let Some(block) = continuation {
            self.current = block;
        }
        Ok(continuation.is_none())
    }

    /// The `Ok` arm binder's type for a collected value: a scalar as before,
    /// or — card `W48-QR-TASK-RECORD-CAPTURES` — a record, noted in
    /// `ok_record` so the arm unpacks it; its placeholder type is never read.
    fn arm_value(&self, value: Shape, ok_record: &mut Option<String>) -> ValueType {
        match value {
            Shape::Scalar(scalar) => ValueType::Scalar(scalar),
            Shape::Record(ty) => {
                *ok_record = Some(ty);
                ValueType::Scalar(Scalar::Unit)
            }
        }
    }

    fn task_expression(
        &mut self,
        statement: &TypedStatement,
        rhs: &[Token],
        slot: Option<usize>,
    ) -> Result<Option<ValueType>, SourceDiagnostic> {
        let span = statement.span;
        if statement.task_expressions.len() != 1 {
            return Err(self.bad(
                span,
                "task execution requires exactly one complete task expression",
            ));
        }
        let task = &statement.task_expressions[0];
        if rhs.first().map(|t| t.span.start) != Some(task.syntax.span.start)
            || rhs.last().map(|t| t.span.end)
                != Some(self.source.visible_end(span, task.syntax.span.end))
        {
            return Err(self.bad(span, "task expression must consume its complete statement or initializer; suffixes and surrounding operations are unsupported"));
        }
        match &task.kind {
            ResolvedTaskExpressionKind::Spawn {
                scope,
                worker,
                captures,
            } => {
                let Some(slot) = slot else {
                    return Err(self.bad(span, "spawn requires a fresh handle binding"));
                };
                if !task.syntax.propagates_error {
                    return Err(self.bad(
                        span,
                        "spawn requires immediate error discrimination with `?` in this profile",
                    ));
                }
                if !self.signature.effects.spawn
                    || !self.signature.effects.errors.contains("SpawnError")
                {
                    return Err(failure(
                        span,
                        SemanticPass::TypedErrors,
                        "spawn requires spawn and throws<SpawnError> effects",
                    ));
                }
                let (scope_id, arguments, moved) =
                    self.spawn_inputs(span, *scope, *worker, captures)?;
                let success = self.block_id(span)?;
                let spawn_error = self.error_block(span, ErrorOperand::Tag("SpawnError".into()))?;
                self.finish(
                    span,
                    TaskTerminatorKind::TrySpawn {
                        scope: scope_id,
                        worker: worker.0 as usize,
                        arguments,
                        handle_slot: slot,
                        success,
                        spawn_error,
                    },
                );
                self.current = success;
                // The failure edge exits, so a record moved whole is moved on
                // every path that continues (card `W48-QR-TASK-RECORD-CAPTURES`).
                self.mark_moved(&moved);
                Ok(Some(ValueType::Handle(*worker)))
            }
            ResolvedTaskExpressionKind::Join { handle } => {
                if !self.signature.effects.join || !self.signature.effects.may_block {
                    return Err(failure(
                        span,
                        SemanticPass::TypedErrors,
                        "join requires join and may_block effects",
                    ));
                }
                let (handle, kind) = self.lookup_place(handle)?;
                let ValueType::Handle(worker) = kind else {
                    return Err(failure(
                        span,
                        SemanticPass::OrdinaryTypes,
                        "join requires a task handle, not a scalar or undiscriminated result",
                    ));
                };
                let worker = &self.signatures[&worker];
                let value = worker.result.clone();
                let errors = worker.effects.errors.clone();
                // The worker's own declared payload row, not the caller's: a
                // caller cannot decide that a joined error holds nothing.
                let error_payload = !worker.effects.error_payloads.is_empty();
                if task.syntax.propagates_error {
                    let result_slot = self.temporary(span)?;
                    let value_slot = match slot {
                        Some(slot) => slot,
                        None => self.temporary(span)?,
                    };
                    self.instruction(
                        span,
                        TaskInstructionKind::Join {
                            handle: handle.0 as usize,
                            outcome_slot: Some(result_slot),
                        },
                    );
                    let kind = self.propagate(
                        span,
                        result_slot,
                        value_slot,
                        value.clone(),
                        &errors,
                        error_payload,
                    )?;
                    if slot.is_none() && value != Shape::Scalar(Scalar::Unit) {
                        return Err(self.bad(span, "discarded join result must be unit"));
                    }
                    Ok(slot.map(|_| kind))
                } else {
                    if slot.is_none()
                        && (value != Shape::Scalar(Scalar::Unit) || !errors.is_empty())
                    {
                        return Err(self.bad(span, "discarded join result must be infallible unit"));
                    }
                    self.instruction(
                        span,
                        TaskInstructionKind::Join {
                            handle: handle.0 as usize,
                            outcome_slot: slot,
                        },
                    );
                    Ok(slot.map(|_| ValueType::Result {
                        value,
                        errors,
                        error_payload,
                    }))
                }
            }
        }
    }

    fn block(
        &mut self,
        block: &SourceBlock,
        typed: &[TypedStatement],
    ) -> Result<bool, SourceDiagnostic> {
        if block.statements.len() != typed.len() {
            return Err(self.bad(block.span, "task block metadata mismatch"));
        }
        let mut terminated = false;
        for (statement, typed) in block.statements.iter().zip(typed) {
            if terminated {
                return Err(self.bad(
                    statement.span,
                    "unreachable statements after return/raise are not admitted by task execution",
                ));
            }
            if statement.span != typed.span {
                return Err(self.bad(statement.span, "task statement metadata mismatch"));
            }
            let tokens = tokens_for(self.source, statement.span);
            match &statement.value {
                StatementKind::Match { .. } => {
                    terminated = self.result_match(typed, &tokens)?;
                }
                StatementKind::ThreadScope { name, body } => {
                    if tokens.len() < 5
                        || !word(tokens.first(), "thread")
                        || !word(tokens.get(1), "scope")
                        || name != super_name(tokens.get(2)).unwrap_or("")
                        || !symbol(tokens.get(3), Symbol::LeftBrace)
                        || tokens[3].span.start != body.span.start
                        || matching(&tokens, 3, Symbol::LeftBrace, Symbol::RightBrace)
                            != Some(tokens.len() - 1)
                        || tokens.last().map(|token| token.span.end) != Some(body.span.end)
                        || !typed.task_expressions.is_empty()
                    {
                        return Err(self.bad(
                            statement.span,
                            "thread scope header must be consumed exactly",
                        ));
                    }
                    let scope = typed.body_binding.ok_or_else(|| {
                        self.bad(statement.span, "missing typed thread-scope binder")
                    })?;
                    self.instruction(
                        statement.span,
                        TaskInstructionKind::EnterScope {
                            scope: u64::from(scope.0),
                        },
                    );
                    self.scopes.push(u64::from(scope.0));
                    self.environment
                        .push(BTreeMap::from([(name.clone(), (scope, ValueType::Scope))]));
                    terminated = self.block(body, &typed.nested)?;
                    self.environment.pop();
                    self.scopes.pop();
                    if !terminated {
                        self.instruction(
                            statement.span,
                            TaskInstructionKind::ExitScope {
                                scope: u64::from(scope.0),
                            },
                        );
                    }
                }
                StatementKind::Let {
                    binding: Some(binding),
                } => {
                    if !word(tokens.first(), "let")
                        || name(tokens.get(1)) != Some(binding.as_str())
                        || !symbol(tokens.last(), Symbol::Semicolon)
                    {
                        return Err(self.bad(
                            statement.span,
                            "only a single immutable name binding is supported",
                        ));
                    }
                    let equal = tokens
                        .iter()
                        .position(|token| symbol(Some(token), Symbol::Equal))
                        .ok_or_else(|| {
                            self.bad(statement.span, "task locals require initialized values")
                        })?;
                    let annotation = if equal == 2 {
                        None
                    } else if equal > 3 && symbol(tokens.get(2), Symbol::Colon) {
                        Some(scalar(&tokens[3..equal]).ok_or_else(|| {
                            self.bad(statement.span, "unsupported task local type")
                        })?)
                    } else {
                        return Err(self.bad(
                            statement.span,
                            "unsupported binding pattern/type annotation",
                        ));
                    };
                    let rhs = &tokens[equal + 1..tokens.len() - 1];
                    let id = typed
                        .binding
                        .ok_or_else(|| self.bad(statement.span, "missing typed local binder"))?;
                    let kind = if !typed.task_expressions.is_empty() {
                        self.task_expression(typed, rhs, Some(id.0 as usize))?
                            .unwrap()
                    } else if let Some(kind) =
                        self.buffer_initializer(rhs, statement.span, id.0 as usize)?
                    {
                        kind
                    } else if let Some(kind) = self.record_initializer(rhs, statement.span, id)? {
                        // Card `W48-QR-TASK-RECORD-CAPTURES`: a construction, a
                        // field read, or a copy of a record of scalars.
                        kind
                    } else if rhs.len() == 2 && symbol(rhs.get(1), Symbol::Question) {
                        let source_name = name(rhs.first()).ok_or_else(|| {
                            self.bad(
                                statement.span,
                                "result propagation requires an immutable result binding",
                            )
                        })?;
                        let (result, kind) = self.lookup(source_name, statement.span)?;
                        let ValueType::Result {
                            value,
                            errors,
                            error_payload,
                        } = kind
                        else {
                            return Err(
                                self.bad(statement.span, "`?` requires a collected task result")
                            );
                        };
                        self.propagate(
                            statement.span,
                            result.0 as usize,
                            id.0 as usize,
                            value,
                            &errors,
                            error_payload,
                        )?
                    } else if let Some(Scalar::Fixed(ty)) = annotation.or_else(|| {
                        // Card `W48-QR-FLOATING-POINT`: an unannotated float
                        // literal is an `f64`, as on every route.
                        matches!(
                            rhs.last().map(|token| &token.kind),
                            Some(TokenKind::Float(_))
                        )
                        .then_some(Scalar::Fixed(FixedType::F64))
                    }) && let Some(literal) = self.fixed_literal(rhs, ty, statement.span)
                    {
                        // Card `W48-QR-SCALAR-TYPES-R4`: a literal takes the
                        // annotation's type, range-checked against it.
                        let value = literal?;
                        if let ScalarOperand::Fixed { value, .. } = value {
                            self.known.insert(id, value);
                        }
                        self.instruction(
                            statement.span,
                            TaskInstructionKind::BindScalar {
                                slot: id.0 as usize,
                                value,
                            },
                        );
                        ValueType::Scalar(Scalar::Fixed(ty))
                    } else if let Some(parsed) = {
                        let environment = &self.environment;
                        // Card `W48-QR-FLOATING-POINT`: this route admits floats.
                        crate::source_value_statement::parse_value_expression_admitting_floats(
                            &tokens[equal + 1..],
                            |name| {
                                environment
                                    .iter()
                                    .rev()
                                    .find_map(|scope| scope.get(name))
                                    .and_then(|(symbol, kind)| match kind {
                                        ValueType::Scalar(
                                            scalar @ (Scalar::I32 | Scalar::Fixed(_)),
                                        ) => Some((*symbol, *scalar)),
                                        _ => None,
                                    })
                            },
                        )
                    } {
                        // Card `W48-QR-SCALAR-TYPES-R4`: a value statement. A
                        // shape it cannot read is this route's shape refusal.
                        let syntax = parsed.map_err(|message| self.bad(statement.span, message))?;
                        self.value_statement(syntax, statement.span, id)?
                    } else {
                        let (value, scalar) = self.scalar_operand(rhs, statement.span)?;
                        // A copy of a known binding is known.
                        let known = match &value {
                            ScalarOperand::I32(value) => Some(i128::from(*value)),
                            ScalarOperand::Fixed { value, .. } => Some(*value),
                            ScalarOperand::Slot(slot) => self
                                .known
                                .iter()
                                .find(|(symbol, _)| symbol.0 as usize == *slot)
                                .map(|(_, value)| *value),
                            _ => None,
                        };
                        if let Some(known) = known {
                            self.known.insert(id, known);
                        }
                        self.instruction(
                            statement.span,
                            TaskInstructionKind::BindScalar {
                                slot: id.0 as usize,
                                value,
                            },
                        );
                        ValueType::Scalar(scalar)
                    };
                    if annotation.is_some_and(|annotation| kind != ValueType::Scalar(annotation)) {
                        return Err(failure(
                            statement.span,
                            SemanticPass::OrdinaryTypes,
                            "local result does not match its declared scalar type",
                        ));
                    }
                    self.environment
                        .last_mut()
                        .unwrap()
                        .insert(binding.clone(), (id, kind));
                }
                StatementKind::Drop { .. }
                    if records::token_path(&tokens, 1).is_some_and(|(root, _, _)| {
                        matches!(
                            self.lookup(&root, statement.span),
                            Ok((_, ValueType::Record(_)))
                        )
                    }) =>
                {
                    // Card `W48-QR-TASK-RECORD-CAPTURES`: `drop RECORD;` or
                    // `drop RECORD.PATH;`.
                    let (root, path, after) =
                        records::token_path(&tokens, 1).expect("the guard read it");
                    if !word(tokens.first(), "drop")
                        || after + 1 != tokens.len()
                        || !symbol(tokens.get(after), Symbol::Semicolon)
                    {
                        return Err(self.bad(statement.span, "buffer drop requires one exact name"));
                    }
                    self.drop_record_path(&root, &path, statement.span)?;
                }
                StatementKind::Drop { place: Some(place) } => {
                    if tokens.len() != 3
                        || !word(tokens.first(), "drop")
                        || name(tokens.get(1)) != Some(place.as_str())
                        || !symbol(tokens.get(2), Symbol::Semicolon)
                    {
                        return Err(self.bad(statement.span, "buffer drop requires one exact name"));
                    }
                    let (id, kind) = self.lookup(place, statement.span)?;
                    if !matches!(kind, ValueType::Scalar(Scalar::OwnedBuffer(_))) {
                        return Err(self.bad(statement.span, "drop requires an owned buffer"));
                    }
                    self.instruction(
                        statement.span,
                        TaskInstructionKind::DropBuffer {
                            buffer: id.0 as usize,
                        },
                    );
                }
                StatementKind::Assignment { .. }
                    if records::token_path(&tokens, 0).is_some_and(|(root, path, after)| {
                        !path.is_empty()
                            && symbol(tokens.get(after), Symbol::Equal)
                            && matches!(
                                self.lookup(&root, statement.span),
                                Ok((_, ValueType::Record(_)))
                            )
                    }) =>
                {
                    // Card `W48-QR-TASK-RECORD-CAPTURES`: `RECORD.PATH = VALUE;`.
                    let (root, path, after) =
                        records::token_path(&tokens, 0).expect("the guard read it");
                    if tokens.len() < after + 3 || !symbol(tokens.last(), Symbol::Semicolon) {
                        return Err(
                            self.bad(statement.span, "a field write is `RECORD.FIELD = VALUE;`")
                        );
                    }
                    let value = tokens[after + 1..tokens.len() - 1].to_vec();
                    self.field_write(&root, &path, &value, statement.span)?;
                }
                StatementKind::Expression => {
                    // Card `W48-QR-TASK-RECORD-CAPTURES`: a line rendering a
                    // scalar field of a record, on the composed route only.
                    if self.composing
                        && let Some(line) = self.record_line(&tokens, statement.span)
                    {
                        line?;
                        continue;
                    }
                    // Card `COMPOSE-CONCURRENCY`. Tried first and only when
                    // composing, so off the composed route this statement is the
                    // unsupported expression statement it has always been.
                    if self.composing
                        && let Some(parsed) =
                            crate::source_std_io::std_io_statement(self.source, statement.span)
                    {
                        match parsed {
                            Ok(event) => {
                                self.source_line(statement.span, event)?;
                                continue;
                            }
                            Err(message) => {
                                // The shape refusal is `source.std-io.v1`'s own,
                                // under that route's own code: the composition
                                // states no grammar of its own.
                                return Err(SourceDiagnostic {
                                    code: "Q030",
                                    pass: SemanticPass::StandardIo,
                                    span: statement.span,
                                    message: message.to_owned(),
                                });
                            }
                        }
                    }
                    if !symbol(tokens.last(), Symbol::Semicolon)
                        || typed.task_expressions.is_empty()
                    {
                        return Err(self.bad(
                            statement.span,
                            "unsupported expression statement in task execution",
                        ));
                    }
                    self.task_expression(typed, &tokens[..tokens.len() - 1], None)?;
                }
                StatementKind::Return => {
                    if !symbol(tokens.last(), Symbol::Semicolon)
                        || !word(tokens.first(), "return")
                        || !typed.task_expressions.is_empty()
                    {
                        return Err(self.bad(statement.span, "unsupported task return expression"));
                    }
                    // Card `W48-QR-TASK-RECORD-CAPTURES`: a record result.
                    if let Shape::Record(ty) = &self.signature.result {
                        let value = self.record_return(&tokens, statement.span, ty)?;
                        self.instructions
                            .extend(self.exit_instructions(statement.span));
                        self.finish(statement.span, TaskTerminatorKind::Return { value });
                        terminated = true;
                        continue;
                    }
                    let (value, kind) = if tokens.len() == 4 && word(tokens.get(1), "move") {
                        let buffer = name(tokens.get(2)).ok_or_else(|| {
                            self.bad(statement.span, "owned return requires a name")
                        })?;
                        let (id, kind) = self.lookup(buffer, statement.span)?;
                        let ValueType::Scalar(returned @ Scalar::OwnedBuffer(_)) = kind else {
                            return Err(
                                self.bad(statement.span, "move return requires own<Buffer>")
                            );
                        };
                        (ScalarOperand::Move(id.0 as usize), returned)
                    } else if tokens.len() == 2 {
                        (ScalarOperand::Unit, Scalar::Unit)
                    } else if let Shape::Scalar(Scalar::Fixed(ty)) = self.signature.result
                        && let Some(literal) =
                            self.fixed_literal(&tokens[1..tokens.len() - 1], ty, statement.span)
                    {
                        // Card `W48-QR-SCALAR-TYPES-R4`: a returned literal
                        // takes the declared result's type.
                        (literal?, Scalar::Fixed(ty))
                    } else {
                        self.scalar_operand(&tokens[1..tokens.len() - 1], statement.span)?
                    };
                    if Shape::Scalar(kind) != self.signature.result {
                        return Err(failure(
                            statement.span,
                            SemanticPass::OrdinaryTypes,
                            "return value does not match the declared task function result",
                        ));
                    }
                    self.instructions
                        .extend(self.exit_instructions(statement.span));
                    self.finish(statement.span, TaskTerminatorKind::Return { value });
                    terminated = true;
                }
                StatementKind::Raise => {
                    let (error, payload) = if tokens.len() == 3
                        && word(tokens.first(), "raise")
                        && symbol(tokens.get(2), Symbol::Semicolon)
                    {
                        let binding = name(tokens.get(1)).ok_or_else(|| {
                            self.bad(statement.span, "raise requires a bound typed error")
                        })?;
                        let (id, kind) = self.lookup(binding, statement.span)?;
                        let error = match kind {
                            ValueType::KnownError(tag) => {
                                if !self.signature.effects.errors.contains(&tag) {
                                    return Err(failure(
                                        statement.span,
                                        SemanticPass::TypedErrors,
                                        "bound error is absent from throws effects",
                                    ));
                                }
                                ErrorOperand::Tag(tag)
                            }
                            ValueType::Error { errors, payload } => {
                                if !errors.is_subset(&self.signature.effects.errors) {
                                    return Err(failure(
                                        statement.span,
                                        SemanticPass::TypedErrors,
                                        "bound error variants are absent from throws effects",
                                    ));
                                }
                                // Re-raising a bound error keeps its tag, and a
                                // tag alone cannot carry a Buffer onward: take
                                // the payload first, then decide what to do with
                                // it and which tag to raise.
                                if payload {
                                    return Err(failure(
                                        statement.span,
                                        SemanticPass::TypedErrors,
                                        "take the owned payload with qcore_error_payload before re-raising this error",
                                    ));
                                }
                                ErrorOperand::Slot(id.0 as usize)
                            }
                            _ => {
                                return Err(self.bad(
                                    statement.span,
                                    "raise requires a typed error arm binding",
                                ));
                            }
                        };
                        (error, None)
                    } else {
                        // `raise E::V;` is payload-free; `raise E::V(b);` moves
                        // exactly one owned Buffer into the raised error. The
                        // declaration decides which spelling is required.
                        let carried = tokens.len() == 8
                            && symbol(tokens.get(4), Symbol::LeftParen)
                            && symbol(tokens.get(6), Symbol::RightParen);
                        if (tokens.len() != 5 && !carried)
                            || !word(tokens.first(), "raise")
                            || !symbol(tokens.get(2), Symbol::ColonColon)
                            || !symbol(tokens.last(), Symbol::Semicolon)
                        {
                            return Err(self.bad(
                                statement.span,
                                "task raise requires a bound error, Error::Variant, or Error::Variant(buffer)",
                            ));
                        }
                        let error_type = name(tokens.get(1))
                            .ok_or_else(|| self.bad(statement.span, "invalid error type"))?;
                        let variant = name(tokens.get(3))
                            .ok_or_else(|| self.bad(statement.span, "invalid error variant"))?;
                        let tag = format!("{error_type}::{variant}");
                        if !self
                            .errors
                            .get(error_type)
                            .is_some_and(|declared| declared.variants.contains(&tag))
                            || !self.signature.effects.errors.contains(&tag)
                        {
                            return Err(failure(
                                statement.span,
                                SemanticPass::TypedErrors,
                                "raised variant must be declared and covered by throws effects",
                            ));
                        }
                        if self.signature.effects.error_payloads.contains(&tag) != carried {
                            return Err(failure(
                                statement.span,
                                SemanticPass::TypedErrors,
                                "raised payload disagrees with the declared error variant",
                            ));
                        }
                        let payload = if carried {
                            let buffer = name(tokens.get(5)).ok_or_else(|| {
                                self.bad(statement.span, "an error payload requires one name")
                            })?;
                            let (id, kind) = self.lookup(buffer, statement.span)?;
                            // A loan, a scalar and a Result each fail here: the
                            // payload has to be an owned Buffer this frame may
                            // give away, and the raise consumes it.
                            // Card `W48-QR-SCALAR-TYPES-R6`: exactly the
                            // `i32` Buffer a payload variant declares.
                            if kind != ValueType::Scalar(Scalar::OWNED_BUFFER) {
                                return Err(self
                                    .bad(statement.span, "an error payload requires own<Buffer>"));
                            }
                            Some(ScalarOperand::Move(id.0 as usize))
                        } else {
                            None
                        };
                        (ErrorOperand::Tag(tag), payload)
                    };
                    self.instructions
                        .extend(self.exit_instructions(statement.span));
                    self.finish(statement.span, TaskTerminatorKind::Raise { error, payload });
                    terminated = true;
                }
                _ => {
                    return Err(self.bad(
                        statement.span,
                        "statement is outside the copy-scalar task execution profile",
                    ));
                }
            }
        }
        Ok(terminated)
    }
}

// Avoid a pattern binding named `name` obscuring the token helper in scope arms.
fn super_name(token: Option<&Token>) -> Option<&str> {
    name(token)
}
