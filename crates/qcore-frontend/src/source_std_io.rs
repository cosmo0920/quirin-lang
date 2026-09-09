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
//!             | "flush" ";"                            -- std.logger
//!             | "return" ";"
//! RENDERING ::= "dec" | "hex"
//! LEVEL     ::= "info" | "warn" | "fatal"
//! OPERAND   ::= ["-"] INTEGER | NAME
//! EFFECT    ::= "io" | "alloc" | "mut"
//! ```
//!
//! `fatal` is spelled that way because `error` is already the reserved
//! error-type declaration keyword; renaming this card's syntax rather than the
//! pinned declaration surface follows card T08-EPOCH-SRC's `work` → `effort`
//! resolution.
//!
//! # The operand rule is the effect discipline
//!
//! `OPERAND` is an integer literal or a name this entry bound to an integer
//! literal. There is no operand form that can denote an owned buffer, a borrow,
//! a share or a handle, so a rendering on this route cannot consume, release,
//! store or retain custody of anything. A name that is not one of this entry's
//! integer bindings — including every resource binding — is refused at `Q028`.
//! Admitting a *borrowed* render of a live resource is out of v1 and named
//! `STD-FMT-BORROW`.
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
pub const FLUSH_KEYWORD: &str = "flush";

/// Which of this card's capabilities a statement head requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StdIoUse {
    /// `print …` — needs `std.io` and, for its rendering, `std.fmt`.
    Print,
    /// `log …` — needs `std.logger` and, for its rendering, `std.fmt`.
    Log,
    /// `flush;` — needs `std.logger` only. It names no rendering.
    Flush,
}

fn statement_head<'a>(tokens: &'a [Token], span: Span) -> Option<&'a str> {
    tokens
        .iter()
        .find(|token| span.start <= token.span.start && token.span.end <= span.end)
        .and_then(|token| match &token.kind {
            TokenKind::Word(word) => Some(word.as_str()),
            _ => None,
        })
}

fn window<'a>(tokens: &'a [Token], span: Span) -> Vec<&'a Token> {
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
    !std_io_uses(program).is_empty()
}

/// The span of the first statement naming a rendering, i.e. the first place
/// this file needs `std.fmt`. `flush` names none, so a logger-only file that
/// never renders needs no `std.fmt` import — measured, and pinned by a test.
#[must_use]
pub fn rendering_span(program: &SourceProgram) -> Option<Span> {
    let uses = std_io_uses(program);
    uses.get(&StdIoUse::Print)
        .or_else(|| uses.get(&StdIoUse::Log))
        .copied()
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
        Some(TokenKind::Word(name)) => Some((StdIoOperand::Scalar(name.clone()), cursor + 1)),
        _ => None,
    }
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
fn declared_effect_row(tokens: &[TokenKind], span: Span) -> Result<EffectRow, SourceDiagnostic> {
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
    _typed: &TypedSourceProgram,
    entry: &str,
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

    let declared_effects = match declared_effect_row(&declaration.value.tokens, scope_span) {
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

    let mut scalars: BTreeMap<String, i32> = BTreeMap::new();
    let mut events = Vec::new();
    let mut event_spans = Vec::new();

    for statement in &body.statements {
        let span = statement.span;
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
            PRINT_KEYWORD => {
                // `print RENDERING OPERAND ;`
                let Some(rendering) = word(&window, 1).and_then(Rendering::parse) else {
                    diagnostics.push(diagnostic(
                        "Q030",
                        span,
                        "`print` names a `std.fmt` rendering: `dec` or `hex`",
                    ));
                    continue;
                };
                match operand(&window, 2) {
                    Some((operand, next)) if ends_with_semicolon(&window, next) => {
                        events.push(StdIoEvent::Print { rendering, operand });
                        event_spans.push(span);
                    }
                    _ => diagnostics.push(diagnostic(
                        "Q030",
                        span,
                        "`print RENDERING OPERAND;` takes one integer literal or one bound \
                         integer name",
                    )),
                }
            }
            LOG_KEYWORD => {
                // `log LEVEL RENDERING OPERAND ;`
                let Some(level) = word(&window, 1).and_then(Level::parse) else {
                    diagnostics.push(diagnostic(
                        "Q030",
                        span,
                        "`log` names a level: `info`, `warn` or `fatal` (`error` is the reserved \
                         error-type declaration keyword)",
                    ));
                    continue;
                };
                let Some(rendering) = word(&window, 2).and_then(Rendering::parse) else {
                    diagnostics.push(diagnostic(
                        "Q030",
                        span,
                        "`log` names a `std.fmt` rendering after its level: `dec` or `hex`",
                    ));
                    continue;
                };
                match operand(&window, 3) {
                    Some((operand, next)) if ends_with_semicolon(&window, next) => {
                        events.push(StdIoEvent::Log {
                            level,
                            rendering,
                            operand,
                        });
                        event_spans.push(span);
                    }
                    _ => diagnostics.push(diagnostic(
                        "Q030",
                        span,
                        "`log LEVEL RENDERING OPERAND;` takes one integer literal or one bound \
                         integer name",
                    )),
                }
            }
            FLUSH_KEYWORD => {
                if ends_with_semicolon(&window, 1) {
                    events.push(StdIoEvent::Flush);
                    event_spans.push(span);
                } else {
                    diagnostics.push(diagnostic(
                        "Q030",
                        span,
                        "`flush;` takes no operand: it hands the logger's stream to its sink and \
                         renders nothing",
                    ));
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
            StdIoEvent::Flush | StdIoEvent::Return => continue,
        };
        if let StdIoOperand::Scalar(name) = operand
            && !scalars.contains_key(name)
        {
            diagnostics.push(diagnostic(
                "Q028",
                event_spans.get(index).copied().unwrap_or(scope_span),
                format!(
                    "`{name}` is not a scalar this entry bound; `source.std-io.v1` renders \
                     integer literals and `let`-bound integers only, and has no operand form \
                     that could denote a resource"
                ),
            ));
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
            declared_effects,
            events,
        },
        event_spans,
        scope_span,
    })
}
