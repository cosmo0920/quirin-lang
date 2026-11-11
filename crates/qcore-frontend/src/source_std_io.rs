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

//! Exact source lowering for `source.std-io.v1` — the `std.fmt`, `std.io` and
//! `std.logger` bundled modules (card `STD-MODULES`).
//!
//! ```text
//! module    ::= import* (error-decl | entry)*
//! import    ::= "import" ("std.fmt" | "std.io" | "std.logger") ";"
//! entry     ::= "fn" NAME "(" ")" "->" "unit"
//!               ["effects" "{" EFFECT {"," EFFECT} "}"] "{" stmt* "}"
//! stmt      ::= "let" NAME "=" ["-"] INTEGER ";"
//!             | "print" RENDERING OPERAND ";"          -- std.io  + std.fmt
//!             | "log" LEVEL RENDERING OPERAND ";"      -- std.logger + std.fmt
//!             | "log_level" LEVEL ";"                  -- std.logger
//!             | "flush" ";"                            -- std.logger
//!             | "return" ";"
//! RENDERING ::= "dec" | "hex"
//! LEVEL     ::= "trace" | "debug" | "info" | "warn" | "error" | "fatal"
//! OPERAND   ::= ["-"] INTEGER | NAME | "borrow" NAME
//! EFFECT    ::= "io" | "alloc" | "mut"
//! ```
//!
//! `error` is contextual after `log`; elsewhere the same token remains the
//! error-type declaration keyword. Logger levels are labels, not an implicit
//! filtering policy for source-authored records.
//!
//! # The operand rule is the effect discipline
//!
//! A bare `OPERAND` is an integer literal or a name this entry bound to an
//! integer literal. Neither form can denote an owned buffer, a share or a
//! handle, so a rendering of either cannot consume, release, store or retain
//! custody of anything. A name that is not one of this entry's integer
//! bindings — including every resource binding — is refused at `Q028`.
//!
//! `borrow NAME` is the one form that names a resource (card
//! `STD-FMT-BORROW`), and it renders that resource's **model identity** and
//! nothing else — no byte of the object, no address, no extent. It is admitted
//! only where a checked custody model supplied the identity *and* held it live
//! at the statement, which `source.std-io.v1` never does: this route has no
//! custody model, so every `borrow NAME` written on it refuses at `Q028`. The
//! composing route `source.std-io-extern.v1` is where the form is reachable,
//! and the liveness rule is that route's join (`Q043`), decided from the extern
//! model's own published ledger.
//!
//! `borrow` was already a reserved source word (the checked mutex-guard
//! profile's), so this form adds **no** reserved word and moves no fixture
//! surface — standing constraint 14's census was run and found no collision.
//! The marker is also what keeps the pipe out: `x |> print dec` desugars to
//! `print dec x;` with no marker, and a sink stage naming an extra selector
//! word refuses at `Q036`, so no pipeline can spell a borrowed render in v1.
//!
//! # What this lowering does not establish
//!
//! It does not model file descriptors, buffering, durability, encoding beyond
//! the two closed renderings, or any order between the two streams.
use std::collections::BTreeMap;

use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, TypedSourceProgram,
};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::std_io::{Level, Rendering, StdIoEvent, StdIoOperand, StdIoProgram};
use qcore_core::typed_errors::{Effect, EffectRow};

/// A complete lowering of one entry. The core checker still has to validate the
/// program; this type is not evidence on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredStdIoModule {
    pub entry: String,
    pub program: StdIoProgram,
    /// Source span of the event at the same index. Every event has one.
    pub event_spans: Vec<Span>,
    /// Fallback span for a refusal that names no event.
    pub scope_span: Span,
}

/// The statement keywords this route owns. A file containing any of them at a
/// statement head is a `source.std-io.v1` module.
pub const PRINT_KEYWORD: &str = "print";
pub const LOG_KEYWORD: &str = "log";
pub const LOG_LEVEL_KEYWORD: &str = "log_level";
pub const FLUSH_KEYWORD: &str = "flush";

/// The operand marker of a borrowed render (card `STD-FMT-BORROW`).
///
/// Deliberately an **existing** reserved source word — the checked mutex-guard
/// profile reserved `borrow` — so this card adds none and no pinned fixture
/// surface moves. Reserving it is also what keeps the word out of name
/// resolution here: without that, `borrow b` would collect `borrow` as a
/// referenced name and refuse at `Q013`.
pub const BORROW_KEYWORD: &str = "borrow";

/// Which of this card's capabilities a statement head requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StdIoUse {
    /// `print …` — needs `std.io` and, for its rendering, `std.fmt`.
    Print,
    /// `log …` — needs `std.logger` and, for its rendering, `std.fmt`.
    Log,
    /// `log_level …` — needs `std.logger` only.
    LogLevel,
    /// `flush;` — needs `std.logger` only. It names no rendering.
    Flush,
}

fn statement_head(tokens: &[Token], span: Span) -> Option<&str> {
    tokens
        .iter()
        .find(|token| span.start <= token.span.start && token.span.end <= span.end)
        .and_then(|token| match &token.kind {
            TokenKind::Word(word) => Some(word.as_str()),
            _ => None,
        })
}

fn window(tokens: &[Token], span: Span) -> Vec<&Token> {
    tokens
        .iter()
        .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
        .collect()
}

/// Every use of this card's vocabulary in the program, with the span of the
/// first occurrence of each kind. Used by the import gate and by route
/// selection, so a file can never be routed here without having needed the
/// imports, or need them without being routed here.
#[must_use]
pub fn std_io_uses(program: &SourceProgram) -> BTreeMap<StdIoUse, Span> {
    let tokens = program.lexical_tokens();
    let mut found: BTreeMap<StdIoUse, Span> = BTreeMap::new();

    fn scan(block: &SourceBlock, tokens: &[Token], found: &mut BTreeMap<StdIoUse, Span>) {
        for statement in &block.statements {
            if let Some(head) = statement_head(tokens, statement.span) {
                let use_kind = match head {
                    PRINT_KEYWORD => Some(StdIoUse::Print),
                    LOG_KEYWORD => Some(StdIoUse::Log),
                    LOG_LEVEL_KEYWORD => Some(StdIoUse::LogLevel),
                    FLUSH_KEYWORD => Some(StdIoUse::Flush),
                    _ => None,
                };
                if let Some(kind) = use_kind {
                    found.entry(kind).or_insert(statement.span);
                }
            }
            for nested in statement.value.nested_blocks() {
                scan(nested, tokens, found);
            }
        }
    }

    for declaration in &program.declarations {
        if declaration.value.kind != DeclarationKind::Function {
            continue;
        }
        if let Some(body) = &declaration.value.body {
            scan(body, tokens, &mut found);
        }
    }
    found
}

/// Whether this module is a `source.std-io.v1` module.
#[must_use]
pub fn has_source_std_io(program: &SourceProgram) -> bool {
    std_io_uses(program)
        .keys()
        .any(|kind| *kind != StdIoUse::LogLevel)
}

/// Whether this file explicitly selects rendering of system diagnostic events.
#[must_use]
pub fn has_source_log_level(program: &SourceProgram) -> bool {
    std_io_uses(program).contains_key(&StdIoUse::LogLevel)
}

/// The validated direct `log_level` declaration of `entry`, if present.
#[must_use]
pub fn entry_log_level(program: &SourceProgram, entry: &str) -> Option<Level> {
    let declaration = program.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
    })?;
    declaration
        .value
        .body
        .as_ref()?
        .statements
        .iter()
        .find_map(
            |statement| match std_io_statement(program, statement.span) {
                Some(Ok(StdIoEvent::LogLevel { level })) => Some(level),
                _ => None,
            },
        )
}

/// Every `print`/`log` statement in the entry whose operand is a **name**, with
/// that statement's span (card `STD-IO-COMPOSE`).
///
/// Published so a composing route can resolve those names against its own value
/// namespace *before* this lowering runs, and refuse a name it cannot render
/// with a message about the composition rather than one about this route's
/// literal bindings. A literal operand is absent: it names nothing to resolve.
///
/// This reads the same token window and the same operand grammar the lowering
/// below reads, so the two cannot disagree about which names an entry renders.
#[must_use]
pub fn rendered_scalar_operands(program: &SourceProgram, entry: &str) -> Vec<(Span, String)> {
    rendered_operands(program, entry)
        .into_iter()
        .filter_map(|(span, operand)| match operand {
            StdIoOperand::Scalar(name) => Some((span, name)),
            _ => None,
        })
        .collect()
}

/// Every `print`/`log` statement in the entry whose operand is `borrow NAME`,
/// with that statement's span (card `STD-FMT-BORROW`).
///
/// The custody half of [`rendered_scalar_operands`], and published for the same
/// reason: a composing route resolves these against its **custody** namespace
/// before this lowering runs, so the two namespaces are read from two lists and
/// a name can never be resolved in both.
#[must_use]
pub fn rendered_borrowed_operands(program: &SourceProgram, entry: &str) -> Vec<(Span, String)> {
    rendered_operands(program, entry)
        .into_iter()
        .filter_map(|(span, operand)| match operand {
            StdIoOperand::Borrowed(name) => Some((span, name)),
            _ => None,
        })
        .collect()
}

/// Every rendering operand of the entry that names something, with its
/// statement's span. Reads the same token window and the same operand grammar
/// the lowering below reads, so the two cannot disagree about what an entry
/// renders.
fn rendered_operands(program: &SourceProgram, entry: &str) -> Vec<(Span, StdIoOperand)> {
    let tokens = program.lexical_tokens();
    let Some(declaration) = program.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
    }) else {
        return Vec::new();
    };
    let Some(body) = &declaration.value.body else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for statement in &body.statements {
        let window = window(tokens, statement.span);
        let cursor = match word(&window, 0) {
            Some(PRINT_KEYWORD) => 2,
            Some(LOG_KEYWORD) => 3,
            _ => continue,
        };
        if let Some((rendered, next)) = operand(&window, cursor)
            && ends_with_semicolon(&window, next)
        {
            found.push((statement.span, rendered));
        }
    }
    found
}

/// The span of the first statement naming a rendering, i.e. the first place
/// this file needs `std.fmt`. `flush` names none, so a logger-only file that
/// never renders needs no `std.fmt` import — measured, and pinned by a test.
///
/// Card `STRING-SLICE-IO` narrowed this from the statement *head* to the
/// **selector word**, for the same reason `flush` was excluded when the gate was
/// written: a statement that names no rendering needs no `std.fmt`. A rendered
/// string names none — a `string` is already the bytes, so `dec` and `hex` have
/// nothing to do to one — and `print "HTTP/1.1 200 OK";` therefore needs
/// `std.io` and `std.string` and not the formatting module. The narrowing also
/// reaches a `print` that names no rendering **word** at all, which used to ask
/// for `std.fmt` and now refuses at `Q030` naming the missing `dec`/`hex`: both
/// are refusals, and the second names the actual fault.
#[must_use]
pub fn rendering_span(program: &SourceProgram) -> Option<Span> {
    let tokens = program.lexical_tokens();
    let mut found: Option<Span> = None;

    fn scan(block: &SourceBlock, tokens: &[Token], found: &mut Option<Span>) {
        for statement in &block.statements {
            let window = window(tokens, statement.span);
            // The selector position of each head, read with the same grammar
            // the lowering reads it with.
            let selector = match word(&window, 0) {
                Some(PRINT_KEYWORD) => 1,
                Some(LOG_KEYWORD) => 2,
                _ => {
                    for nested in statement.value.nested_blocks() {
                        scan(nested, tokens, found);
                    }
                    continue;
                }
            };
            if word(&window, selector).and_then(Rendering::parse).is_some()
                && found.is_none_or(|span| statement.span.start < span.start)
            {
                *found = Some(statement.span);
            }
            for nested in statement.value.nested_blocks() {
                scan(nested, tokens, found);
            }
        }
    }

    for declaration in &program.declarations {
        if declaration.value.kind != DeclarationKind::Function {
            continue;
        }
        if let Some(body) = &declaration.value.body {
            scan(body, tokens, &mut found);
        }
    }
    found
}

fn diagnostic(code: &'static str, span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code,
        pass: SemanticPass::StandardIo,
        span,
        message: message.into(),
    }
}

/// Read `["-"] INTEGER` starting at `cursor`, returning the value and the new
/// cursor.
fn integer(window: &[&Token], cursor: usize) -> Option<(i32, usize)> {
    let (negative, cursor) = match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Symbol(Symbol::Minus)) => (true, cursor + 1),
        _ => (false, cursor),
    };
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Integer(text)) => {
            // Out-of-range literals are refused, never wrapped: an operand this
            // route cannot render exactly is not rendered at all.
            let value = text.parse::<i64>().ok()?;
            let value = if negative { -value } else { value };
            i32::try_from(value).ok().map(|value| (value, cursor + 1))
        }
        _ => None,
    }
}

fn operand(window: &[&Token], cursor: usize) -> Option<(StdIoOperand, usize)> {
    if let Some((value, next)) = integer(window, cursor) {
        return Some((StdIoOperand::Literal(value), next));
    }
    match window.get(cursor).map(|token| &token.kind) {
        // `borrow NAME` is tried before the bare name, and the marker is a
        // reserved word, so a bare `borrow` can never be read as a scalar name
        // and a borrowed render can never be read as a bare one.
        Some(TokenKind::Word(marker)) if marker == BORROW_KEYWORD => {
            match window.get(cursor + 1).map(|token| &token.kind) {
                Some(TokenKind::Word(name)) => {
                    Some((StdIoOperand::Borrowed(name.clone()), cursor + 2))
                }
                _ => None,
            }
        }
        Some(TokenKind::Word(name)) => Some((StdIoOperand::Scalar(name.clone()), cursor + 1)),
        _ => None,
    }
}

/// The **one** grammar of a line-producing statement: `print RENDERING OPERAND;`,
/// `log LEVEL RENDERING OPERAND;`, `log_level LEVEL;`, `flush;`.
///
/// `None` when the head is not one of the three keywords. Every route that
/// admits these statements reads them here — the lowering below, and the
/// composed task route (card `COMPOSE-CONCURRENCY`) — so a shape one route
/// accepts and another refuses is not expressible.
fn std_io_event(window: &[&Token], head: &str) -> Option<Result<StdIoEvent, &'static str>> {
    match head {
        PRINT_KEYWORD => Some({
            let Some(rendering) = word(window, 1).and_then(Rendering::parse) else {
                return Some(Err("`print` names a `std.fmt` rendering: `dec` or `hex`"));
            };
            match operand(window, 2) {
                Some((operand, next)) if ends_with_semicolon(window, next) => {
                    Ok(StdIoEvent::Print { rendering, operand })
                }
                _ => Err(
                    "`print RENDERING OPERAND;` takes one integer literal, one bound integer \
                     name, or `borrow NAME` for the identity of a live resource",
                ),
            }
        }),
        LOG_KEYWORD => Some({
            let Some(level) = word(window, 1).and_then(Level::parse) else {
                return Some(Err(
                    "`log` names a level: `trace`, `debug`, `info`, `warn`, `error` or `fatal`",
                ));
            };
            let Some(rendering) = word(window, 2).and_then(Rendering::parse) else {
                return Some(Err(
                    "`log` names a `std.fmt` rendering after its level: `dec` or `hex`",
                ));
            };
            match operand(window, 3) {
                Some((operand, next)) if ends_with_semicolon(window, next) => Ok(StdIoEvent::Log {
                    level,
                    rendering,
                    operand,
                }),
                _ => Err(
                    "`log LEVEL RENDERING OPERAND;` takes one integer literal, one bound \
                     integer name, or `borrow NAME` for the identity of a live resource",
                ),
            }
        }),
        LOG_LEVEL_KEYWORD => Some({
            let Some(level) = word(window, 1).and_then(Level::parse) else {
                return Some(Err(
                    "`log_level` names one of `trace`, `debug`, `info`, `warn`, `error` or `fatal`",
                ));
            };
            if ends_with_semicolon(window, 2) {
                Ok(StdIoEvent::LogLevel { level })
            } else {
                Err("`log_level LEVEL;` takes exactly one logger level")
            }
        }),
        FLUSH_KEYWORD => Some(if ends_with_semicolon(window, 1) {
            Ok(StdIoEvent::Flush)
        } else {
            Err(
                "`flush;` takes no operand: it hands the logger's stream to its sink and \
                 renders nothing",
            )
        }),
        _ => None,
    }
}

/// [`std_io_event`] over one statement's own tokens, in retained order (card
/// `W48-QR-SCALAR-TYPES`). The scalar-call route reads its function bodies as
/// token streams rather than statement spans, and a desugared pipeline's tokens
/// keep their written positions, so a span would not recover the statement.
#[must_use]
pub fn std_io_statement_tokens(window: &[&Token]) -> Option<Result<StdIoEvent, &'static str>> {
    let head = word(window, 0)?;
    std_io_event(window, head)
}

/// [`std_io_event`] over one statement of a parsed program, by span.
///
/// Published for the composed task route, which reads statements out of a task
/// body rather than out of a flat entry.
#[must_use]
pub fn std_io_statement(
    program: &SourceProgram,
    span: Span,
) -> Option<Result<StdIoEvent, &'static str>> {
    let tokens = program.lexical_tokens();
    let window = window(tokens, span);
    let head = word(&window, 0)?;
    std_io_event(&window, head)
}

fn word<'a>(window: &'a [&Token], cursor: usize) -> Option<&'a str> {
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    }
}

fn ends_with_semicolon(window: &[&Token], cursor: usize) -> bool {
    matches!(
        window.get(cursor).map(|token| &token.kind),
        Some(TokenKind::Symbol(Symbol::Semicolon))
    ) && window.len() == cursor + 1
}

/// The entry's declared `effects { … }` row. An absent suffix is the empty
/// row, which admits nothing, so the opt-in stays visible exactly as it does
/// on the extern-call route.
///
/// Optionally skips `throws<ArithmeticError>` —
/// the extern model's permission for its value statements (card
/// `W48-QR-SCALAR-TYPES-R3`), which a composed entry's one row carries and
/// which is no effect of this model. On `source.std-io.v1` itself it is never
/// skipped, so the word keeps the refusal it has always had there.
fn declared_effect_row_admitting(
    tokens: &[TokenKind],
    span: Span,
    admit_arithmetic_error: bool,
    admit_float_effects: bool,
) -> Result<EffectRow, SourceDiagnostic> {
    let Some(index) = tokens
        .iter()
        .position(|token| matches!(token, TokenKind::Word(word) if word == "effects"))
    else {
        return Ok(EffectRow::default());
    };
    if !matches!(
        tokens.get(index + 1),
        Some(TokenKind::Symbol(Symbol::LeftBrace))
    ) {
        return Err(diagnostic(
            "Q030",
            span,
            "an `effects` row requires a block",
        ));
    }
    let mut effects = Vec::new();
    let mut cursor = index + 2;
    loop {
        match tokens.get(cursor) {
            Some(TokenKind::Symbol(Symbol::RightBrace)) => break,
            Some(TokenKind::Word(name))
                if admit_arithmetic_error
                    && name == "throws"
                    && matches!(
                        (tokens.get(cursor + 1), tokens.get(cursor + 2), tokens.get(cursor + 3)),
                        (
                            Some(TokenKind::Symbol(Symbol::Less)),
                            Some(TokenKind::Word(error)),
                            Some(TokenKind::Symbol(Symbol::Greater)),
                        ) if error == "ArithmeticError"
                    ) =>
            {
                cursor += 4;
                if matches!(tokens.get(cursor), Some(TokenKind::Symbol(Symbol::Comma))) {
                    cursor += 1;
                }
            }
            // Card `W48-QR-FLOATING-POINT`: the extern model's comparison
            // effects, which a composed entry's one row carries and which are
            // no effect of this model; skipped on the composed route only, so
            // on `source.std-io.v1` itself the words keep their refusal.
            Some(TokenKind::Word(name))
                if admit_float_effects
                    && qcore_core::scalar_types::FloatEffect::parse(name).is_some() =>
            {
                cursor += 1;
                if matches!(tokens.get(cursor), Some(TokenKind::Symbol(Symbol::Comma))) {
                    cursor += 1;
                }
            }
            Some(TokenKind::Word(name)) => {
                let effect = match name.as_str() {
                    "io" => Effect::Io,
                    "alloc" => Effect::Alloc,
                    "mut" => Effect::Mut,
                    other => {
                        return Err(diagnostic(
                            "Q030",
                            span,
                            format!(
                                "`{other}` is not an effect `source.std-io.v1` models; use io, \
                                 alloc or mut"
                            ),
                        ));
                    }
                };
                effects.push(effect);
                cursor += 1;
                if matches!(tokens.get(cursor), Some(TokenKind::Symbol(Symbol::Comma))) {
                    cursor += 1;
                }
            }
            _ => {
                return Err(diagnostic("Q030", span, "this `effects` row is not closed"));
            }
        }
    }
    Ok(EffectRow::new(effects))
}

/// Lower one entry into a [`StdIoProgram`]. The module is either entirely
/// inside the grammar above or refused with a precise diagnostic; there is no
/// partially lowered middle state.
pub fn lower_source_std_io(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredStdIoModule, Vec<SourceDiagnostic>> {
    lower_source_std_io_skipping(
        parsed,
        typed,
        entry,
        &[],
        &BTreeMap::new(),
        &BTreeMap::new(),
        &ComposedValues::default(),
    )
}

/// What a composing route supplies beyond `i32` scalars and identities (card
/// `W48-QR-SCALAR-TYPES-R3`): the values of the other §G02t types it
/// determined, the recorded target, and whether the one row may carry the
/// extern model's `throws<ArithmeticError>`. Empty on `source.std-io.v1`.
#[derive(Clone, Debug, Default)]
pub struct ComposedValues {
    pub typed: BTreeMap<String, (qcore_core::scalar_types::ScalarType, i128)>,
    pub target: Option<qcore_core::scalar_types::RecordedTarget>,
    pub admit_arithmetic_error: bool,
    /// Card `W48-QR-FLOATING-POINT`: whether the one row may carry the extern
    /// model's `partial_eq`/`partial_ord`.
    pub admit_float_effects: bool,
}

/// [`lower_source_std_io`] with a set of entry-body statement spans left to
/// another model, and a scalar environment that model bound (card
/// `STD-IO-COMPOSE`).
///
/// `lower_source_std_io` is this function with an empty skip set and an empty
/// environment, so a module on the standard-IO route alone travels the
/// identical code path: its lowering, its certificate and its emitted C are
/// byte-identical by construction rather than by comparison.
///
/// `provided_scalars` is what makes a *composed* rendering possible at all.
/// This route's own `let` binds an integer literal; on the composed route every
/// `let` belongs to the extern model, which owns the value namespace, and the
/// composing lowering passes back exactly the values whose numbers that model
/// determined. A value whose number is not determined is deliberately absent,
/// so it refuses here as an unbound operand rather than being rendered as a
/// number no model computed.
///
/// `provided_identities` is the same mechanism for a **borrowed** render (card
/// `STD-FMT-BORROW`): a custody model supplies the identity of each name it
/// binds to a resource, and a name it does not supply refuses here at `Q028`.
/// This route passes an empty map, which is why every `borrow NAME` written on
/// `source.std-io.v1` alone refuses — structurally, for want of a custody
/// model, rather than by a rule this lowering states. Whether the identity is
/// *live* at the statement is not decided here either: that is the composing
/// route's join, which reads the extern model's own ledger.
pub fn lower_source_std_io_skipping(
    parsed: &SourceProgram,
    _typed: &TypedSourceProgram,
    entry: &str,
    skip: &[Span],
    provided_scalars: &BTreeMap<String, i32>,
    provided_identities: &BTreeMap<String, i32>,
    composed: &ComposedValues,
) -> Result<LoweredStdIoModule, Vec<SourceDiagnostic>> {
    let tokens = parsed.lexical_tokens();
    let mut diagnostics = Vec::new();

    let Some(declaration) = parsed.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
    }) else {
        return Err(vec![diagnostic(
            "Q030",
            Span { start: 0, end: 0 },
            format!("`{entry}` is not a function declaration in this module"),
        )]);
    };
    let scope_span = declaration.span;

    // Every `print`/`log`/`flush` must be in the entry. A helper function
    // carrying one is refused rather than silently unmodeled.
    for other in &parsed.declarations {
        if other.value.kind != DeclarationKind::Function || other.value.name == entry {
            continue;
        }
        if let Some(body) = &other.value.body {
            let mut found = BTreeMap::new();
            fn scan(block: &SourceBlock, tokens: &[Token], found: &mut BTreeMap<StdIoUse, Span>) {
                for statement in &block.statements {
                    if let Some(head) = statement_head(tokens, statement.span) {
                        match head {
                            PRINT_KEYWORD => {
                                found.entry(StdIoUse::Print).or_insert(statement.span);
                            }
                            LOG_KEYWORD => {
                                found.entry(StdIoUse::Log).or_insert(statement.span);
                            }
                            LOG_LEVEL_KEYWORD => {
                                found.entry(StdIoUse::LogLevel).or_insert(statement.span);
                            }
                            FLUSH_KEYWORD => {
                                found.entry(StdIoUse::Flush).or_insert(statement.span);
                            }
                            _ => {}
                        }
                    }
                    for nested in statement.value.nested_blocks() {
                        scan(nested, tokens, found);
                    }
                }
            }
            scan(body, tokens, &mut found);
            if let Some(span) = found.values().next() {
                diagnostics.push(diagnostic(
                    "Q030",
                    *span,
                    format!(
                        "`source.std-io.v1` models the entry only; `{}` is not the entry `{entry}`",
                        other.value.name
                    ),
                ));
            }
        }
    }

    if !declaration.value.parameters.is_empty() {
        diagnostics.push(diagnostic(
            "Q030",
            scope_span,
            "`source.std-io.v1` models an entry with no parameters",
        ));
    }

    let declared_effects = match declared_effect_row_admitting(
        &declaration.value.tokens,
        scope_span,
        composed.admit_arithmetic_error,
        composed.admit_float_effects,
    ) {
        Ok(row) => row,
        Err(error) => {
            diagnostics.push(error);
            EffectRow::default()
        }
    };

    let Some(body) = &declaration.value.body else {
        diagnostics.push(diagnostic("Q030", scope_span, "this entry has no body"));
        return Err(diagnostics);
    };

    let mut scalars: BTreeMap<String, i32> = provided_scalars.clone();
    let identities: BTreeMap<String, i32> = provided_identities.clone();
    let mut events = Vec::new();
    let mut event_spans = Vec::new();

    for statement in &body.statements {
        let span = statement.span;
        if skip.contains(&span) {
            continue;
        }
        if !statement.value.nested_blocks().is_empty() {
            diagnostics.push(diagnostic(
                "Q030",
                span,
                "`source.std-io.v1` models one flat statement sequence; this statement opens a \
                 nested block",
            ));
            continue;
        }
        let window = window(tokens, span);
        let head = word(&window, 0).unwrap_or_default();
        match head {
            "let" => {
                // `let NAME = ["-"] INTEGER ;`
                let Some(name) = word(&window, 1) else {
                    diagnostics.push(diagnostic("Q030", span, "this `let` binds no name"));
                    continue;
                };
                if !matches!(
                    window.get(2).map(|token| &token.kind),
                    Some(TokenKind::Symbol(Symbol::Equal))
                ) {
                    diagnostics.push(diagnostic(
                        "Q030",
                        span,
                        "`source.std-io.v1` binds `let NAME = INTEGER;` only",
                    ));
                    continue;
                }
                match integer(&window, 3) {
                    Some((value, next)) if ends_with_semicolon(&window, next) => {
                        scalars.insert(name.to_owned(), value);
                    }
                    _ => diagnostics.push(diagnostic(
                        "Q030",
                        span,
                        "`source.std-io.v1` binds an integer literal only; it has no operand form \
                         that could denote a resource",
                    )),
                }
            }
            PRINT_KEYWORD | LOG_KEYWORD | LOG_LEVEL_KEYWORD | FLUSH_KEYWORD => {
                match std_io_event(&window, head)
                    .expect("the three line-producing heads are matched above")
                {
                    Ok(event) => {
                        events.push(event);
                        event_spans.push(span);
                    }
                    Err(message) => diagnostics.push(diagnostic("Q030", span, message)),
                }
            }
            "return" => {
                if ends_with_semicolon(&window, 1) {
                    events.push(StdIoEvent::Return);
                    event_spans.push(span);
                } else {
                    diagnostics.push(diagnostic(
                        "Q030",
                        span,
                        "`source.std-io.v1` models `return;` from a `unit` entry",
                    ));
                }
            }
            other => diagnostics.push(diagnostic(
                "Q030",
                span,
                format!("`{other}` is not a statement `source.std-io.v1` models"),
            )),
        }
    }

    // Every operand name must be one of this entry's integer bindings. A
    // resource binding lands here, which is the whole point: the route has no
    // way to render one.
    for (index, event) in events.iter().enumerate() {
        let operand = match event {
            StdIoEvent::Print { operand, .. } | StdIoEvent::Log { operand, .. } => operand,
            StdIoEvent::LogLevel { .. } | StdIoEvent::Flush | StdIoEvent::Return => continue,
        };
        let span = event_spans.get(index).copied().unwrap_or(scope_span);
        match operand {
            StdIoOperand::Scalar(name)
                if !scalars.contains_key(name) && !composed.typed.contains_key(name) =>
            {
                diagnostics.push(diagnostic(
                    "Q028",
                    span,
                    format!(
                        "`{name}` is not a scalar this entry bound; `source.std-io.v1` renders \
                         integer literals and `let`-bound integers only, and its bare operand \
                         form has none that could denote a resource"
                    ),
                ));
            }
            // Card `STD-FMT-BORROW`. On this route `identities` is empty, so
            // every borrowed render lands here: the refusal is the absence of a
            // custody model, not a rule about this statement.
            StdIoOperand::Borrowed(name) if !identities.contains_key(name) => {
                diagnostics.push(diagnostic(
                    "Q028",
                    span,
                    format!(
                        "`borrow {name}` renders the model identity of a live resource, and no \
                         checked custody model published one here; `source.std-io.v1` has no \
                         custody model of its own, so a borrowed render needs a route that \
                         composes this one with a model that holds the identity live at that \
                         statement"
                    ),
                ));
            }
            _ => {}
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(LoweredStdIoModule {
        entry: entry.to_owned(),
        program: StdIoProgram {
            entry: entry.to_owned(),
            scalars,
            identities,
            typed_scalars: composed.typed.clone(),
            target: composed.target.clone(),
            declared_effects,
            events,
        },
        event_spans,
        scope_span,
    })
}
