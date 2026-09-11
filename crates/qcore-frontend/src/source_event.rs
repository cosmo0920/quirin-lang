//! Exact source lowering for `source.event.v1` — the `std.event` bundled module
//! (card `STD-EVENT`).
//!
//! ```text
//! module    ::= import* entry
//! import    ::= "import" ("std.event" | "std.logger") ";"
//! entry     ::= "fn" NAME "(" ")" "->" "i32"
//!               ["effects" "{" EFFECT {"," EFFECT} "}"] "{" stmt* "}"
//! stmt      ::= "event" "open" ";"
//!             | "log_level" LEVEL ";"
//!             | "event" "close" ";"
//!             | "endpoints" NAME NAME ";"
//!             | "watch" NAME NAME INTEREST ";"
//!             | "unwatch" NAME ";"
//!             | "wake" NAME ";"
//!             | "dispatch" "ready" NAME ";"
//!             | "dispatch" "idle" ";"
//!             | "close" NAME ";"
//!             | "return" ["-"] INTEGER ";"
//! INTEREST  ::= "read" | "write"
//! LEVEL     ::= "trace" | "debug" | "info" | "warn" | "error" | "fatal"
//! EFFECT    ::= "io" | "alloc" | "mut"
//! ```
//!
//! # There is no value vocabulary here, and that is the point
//!
//! The grammar has no `let`, no scalar operand and no expression. Every name it
//! resolves denotes an **endpoint** or a **registration** — a custody name — so
//! no statement of this route can compute, carry or render a value. A route
//! that needs to print one composes with `source.std-io.v1`, which is a
//! follow-on and not this card.
//!
//! # This lowering decides nothing about custody
//!
//! It produces a [`qcore_core::std_event::EventSourceProgram`] and stops. The
//! custody and lifecycle questions are answered by
//! `qcore_core::event_reactor::check_event_program` over the projection, and
//! the determinism questions by `qcore_core::std_event`'s own readiness ledger.
//! A refusal a reader sees at `Q060` is the **reactor model's**, attributed to
//! the statement whose projection it named.
//!
//! # Reserved words (standing constraint 14)
//!
//! The original eight are `event`, `endpoints`, `watch`, `unwatch`, `wake`,
//! `dispatch`, `ready`, `idle`; the quiet-emission amendment adds `log_level`.
//! Four existing ones are reused rather than
//! respelled — `open`, `close`, `read`, `write` — which is why the context pair
//! is spelled `event open;` / `event close;` rather than with two words of its
//! own. The census over every `.qcore`/`.qc` fixture and every inline source
//! string in the Rust tests found no collision; see the card report §7.

use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, TypedSourceProgram,
};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::std_event::{EventSourceProgram, EventStatement, Interest};
use qcore_core::std_io::Level;
use qcore_core::typed_errors::{Effect, EffectRow};

/// The statement keywords this route owns. A file containing any of them at a
/// statement head is a `source.event.v1` module.
///
/// `close` is deliberately **absent**: it is an existing reserved word the
/// checked mutex-guard profile already uses at a statement head, so selecting on
/// it would route a mutex module here. A `close D;` is admitted *inside* this
/// route and never selects it.
pub const SELECTING_KEYWORDS: [&str; 6] =
    ["event", "endpoints", "watch", "unwatch", "wake", "dispatch"];

/// A complete lowering of one entry. The core checker still has to validate the
/// program; this type is not evidence on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredEventModule {
    pub entry: String,
    pub program: EventSourceProgram,
    /// Source span of the statement at the same index. Every statement has one.
    pub statement_spans: Vec<Span>,
    /// Fallback span for a refusal that names no statement.
    pub scope_span: Span,
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

/// The span of the first statement in this program whose head is one of
/// [`SELECTING_KEYWORDS`], if any.
///
/// One detector serves both the import gate and route selection, so a file can
/// never be routed here without having needed `import std.event;`, or need the
/// import without being routed here — the discipline `STD-MODULES` established
/// for its own three capabilities.
///
/// Only **statement heads inside a function body** are inspected. The contract
/// corpus's six occurrences of `event` — one ghost field in an
/// `ensures normal event == registered_event;` clause, two extern-declaration
/// parameter names, three prose comments — are none of them a statement head,
/// and are therefore invisible here, exactly as `concurrency_span` keeps a ghost
/// field named `thread` out of the concurrency gate.
#[must_use]
pub fn source_event_span(program: &SourceProgram) -> Option<Span> {
    let tokens = program.lexical_tokens();

    fn scan(block: &SourceBlock, tokens: &[Token]) -> Option<Span> {
        for statement in &block.statements {
            if let Some(head) = statement_head(tokens, statement.span)
                && SELECTING_KEYWORDS.contains(&head)
            {
                return Some(statement.span);
            }
            for nested in statement.value.nested_blocks() {
                if let Some(span) = scan(nested, tokens) {
                    return Some(span);
                }
            }
        }
        None
    }

    for declaration in &program.declarations {
        if declaration.value.kind != DeclarationKind::Function {
            continue;
        }
        if let Some(body) = &declaration.value.body
            && let Some(span) = scan(body, tokens)
        {
            return Some(span);
        }
    }
    None
}

/// Whether this module is a `source.event.v1` module.
#[must_use]
pub fn has_source_event(program: &SourceProgram) -> bool {
    source_event_span(program).is_some()
}

/// The first event-profile diagnostic logger declaration in this file.
///
/// Kept separate from [`source_event_span`] because `log_level` selects the
/// event route **and** consumes the independently imported `std.logger`
/// capability.
#[must_use]
pub fn event_log_level_span(program: &SourceProgram) -> Option<Span> {
    let tokens = program.lexical_tokens();

    fn scan(block: &SourceBlock, tokens: &[Token]) -> Option<Span> {
        for statement in &block.statements {
            if statement_head(tokens, statement.span) == Some("log_level") {
                return Some(statement.span);
            }
            for nested in statement.value.nested_blocks() {
                if let Some(span) = scan(nested, tokens) {
                    return Some(span);
                }
            }
        }
        None
    }

    program
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::Function)
        .filter_map(|declaration| declaration.value.body.as_ref())
        .find_map(|body| scan(body, tokens))
}

fn diagnostic(code: &'static str, span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code,
        pass: SemanticPass::Event,
        span,
        message: message.into(),
    }
}

/// Read `["-"] INTEGER` starting at `cursor`, returning the value and the new
/// cursor. Out-of-range literals are refused, never wrapped.
fn integer(window: &[&Token], cursor: usize) -> Option<(i32, usize)> {
    let (negative, cursor) = match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Symbol(Symbol::Minus)) => (true, cursor + 1),
        _ => (false, cursor),
    };
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Integer(text)) => {
            let value = text.parse::<i64>().ok()?;
            let value = if negative { -value } else { value };
            i32::try_from(value).ok().map(|value| (value, cursor + 1))
        }
        _ => None,
    }
}

/// The **one** grammar of a `source.event.v1` statement.
///
/// `None` when the head is not one this route owns; `Some(Err(message))` when
/// the head is one but the rest of the statement is not the form it introduces.
fn event_statement(window: &[&Token], head: &str) -> Option<Result<EventStatement, String>> {
    match head {
        "log_level" => Some(match word(window, 1).and_then(Level::parse) {
            Some(level) if ends_with_semicolon(window, 2) => Ok(EventStatement::LogLevel { level }),
            _ => Err(
                "`log_level LEVEL;` selects one of trace, debug, info, warn, error, or fatal"
                    .to_owned(),
            ),
        }),
        "event" => Some(match word(window, 1) {
            Some("open") if ends_with_semicolon(window, 2) => Ok(EventStatement::ContextOpen),
            Some("close") if ends_with_semicolon(window, 2) => Ok(EventStatement::ContextClose),
            _ => Err(
                "`source.event.v1` opens and closes its one event context with `event open;` and \
                 `event close;`"
                    .to_owned(),
            ),
        }),
        "endpoints" => Some(
            match (
                word(window, 1),
                word(window, 2),
                ends_with_semicolon(window, 3),
            ) {
                (Some(first), Some(second), true) => Ok(EventStatement::Endpoints {
                    first: first.to_owned(),
                    second: second.to_owned(),
                }),
                _ => Err(
                    "`endpoints A B;` binds the two owned endpoints of one pair; \
                     `source.event.v1` models the success branch only"
                        .to_owned(),
                ),
            },
        ),
        "watch" => Some({
            let registration = word(window, 1);
            let endpoint = word(window, 2);
            let interest = word(window, 3).and_then(Interest::parse);
            match (registration, endpoint, interest) {
                (Some(registration), Some(endpoint), Some(interest))
                    if ends_with_semicolon(window, 4) =>
                {
                    Ok(EventStatement::Watch {
                        registration: registration.to_owned(),
                        endpoint: endpoint.to_owned(),
                        interest,
                    })
                }
                _ => Err(
                    "`watch E D INTEREST;` registers interest on a live endpoint, where INTEREST \
                     is `read` or `write`"
                        .to_owned(),
                ),
            }
        }),
        "unwatch" => Some(match word(window, 1) {
            Some(registration) if ends_with_semicolon(window, 2) => Ok(EventStatement::Unwatch {
                registration: registration.to_owned(),
            }),
            _ => Err("`unwatch E;` deregisters and frees one registration".to_owned()),
        }),
        "wake" => Some(match word(window, 1) {
            Some(endpoint) if ends_with_semicolon(window, 2) => Ok(EventStatement::Wake {
                endpoint: endpoint.to_owned(),
            }),
            _ => Err(
                "`wake D;` writes one byte into the peer of a live endpoint pair, so that a read \
                 registration on `D` becomes ready"
                    .to_owned(),
            ),
        }),
        "dispatch" => Some(match word(window, 1) {
            Some("idle") if ends_with_semicolon(window, 2) => Ok(EventStatement::DispatchIdle),
            Some("ready") => match word(window, 2) {
                Some(registration) if ends_with_semicolon(window, 3) => {
                    Ok(EventStatement::DispatchReady {
                        registration: registration.to_owned(),
                    })
                }
                _ => Err(
                    "`dispatch ready E;` names the one registration this turn reports".to_owned(),
                ),
            },
            _ => Err(
                "a dispatch turn declares what it reports: `dispatch ready E;` for exactly one \
                 registration, or `dispatch idle;` for none. `source.event.v1` has no undeclared \
                 turn, because a certificate compares one observation and an undeclared turn has \
                 none"
                    .to_owned(),
            ),
        }),
        "close" => Some(match word(window, 1) {
            Some(endpoint) if ends_with_semicolon(window, 2) => Ok(EventStatement::Close {
                endpoint: endpoint.to_owned(),
            }),
            _ => Err(
                "`close D;` releases one live endpoint; `source.event.v1` models the \
                      success branch only"
                    .to_owned(),
            ),
        }),
        "return" => Some(match integer(window, 1) {
            Some((status, next)) if ends_with_semicolon(window, next) => {
                Ok(EventStatement::Return { status })
            }
            _ => Err("`source.event.v1` models `return INTEGER;` from an `i32` entry".to_owned()),
        }),
        _ => None,
    }
}

/// The entry's declared `effects { … }` row. An absent suffix is the empty row,
/// which admits nothing, so the opt-in stays visible exactly as it does on the
/// standard-IO and extern-call routes.
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
            "Q058",
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
                            "Q058",
                            span,
                            format!(
                                "`{other}` is not an effect `source.event.v1` models; use io, \
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
                return Err(diagnostic("Q058", span, "this `effects` row is not closed"));
            }
        }
    }
    Ok(EffectRow::new(effects))
}

/// Lower one entry into an [`EventSourceProgram`]. The module is either entirely
/// inside the grammar above or refused with a precise diagnostic; there is no
/// partially lowered middle state.
///
/// # Errors
///
/// One [`SourceDiagnostic`] per refusal, all of them, at `Q058`.
pub fn lower_source_event(
    parsed: &SourceProgram,
    _typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredEventModule, Vec<SourceDiagnostic>> {
    let tokens = parsed.lexical_tokens();
    let mut diagnostics = Vec::new();

    let Some(declaration) = parsed.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
    }) else {
        return Err(vec![diagnostic(
            "Q058",
            Span { start: 0, end: 0 },
            format!("`{entry}` is not a function declaration in this module"),
        )]);
    };
    let scope_span = declaration.span;

    // Every event statement must be in the entry. A helper function carrying one
    // is refused rather than silently unmodeled.
    for other in &parsed.declarations {
        if other.value.kind != DeclarationKind::Function || other.value.name == entry {
            continue;
        }
        if let Some(body) = &other.value.body {
            fn scan(block: &SourceBlock, tokens: &[Token]) -> Option<Span> {
                for statement in &block.statements {
                    if let Some(head) = statement_head(tokens, statement.span)
                        && SELECTING_KEYWORDS.contains(&head)
                    {
                        return Some(statement.span);
                    }
                    for nested in statement.value.nested_blocks() {
                        if let Some(span) = scan(nested, tokens) {
                            return Some(span);
                        }
                    }
                }
                None
            }
            if let Some(span) = scan(body, tokens) {
                diagnostics.push(diagnostic(
                    "Q058",
                    span,
                    format!(
                        "`source.event.v1` models the entry only; `{}` is not the entry `{entry}`",
                        other.value.name
                    ),
                ));
            }
        }
    }

    if !declaration.value.parameters.is_empty() {
        diagnostics.push(diagnostic(
            "Q058",
            scope_span,
            "`source.event.v1` models an entry with no parameters",
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
        diagnostics.push(diagnostic("Q058", scope_span, "this entry has no body"));
        return Err(diagnostics);
    };

    let mut statements = Vec::new();
    let mut statement_spans = Vec::new();

    for statement in &body.statements {
        let span = statement.span;
        if !statement.value.nested_blocks().is_empty() {
            diagnostics.push(diagnostic(
                "Q058",
                span,
                "`source.event.v1` models one flat statement sequence; this statement opens a \
                 nested block",
            ));
            continue;
        }
        let window = window(tokens, span);
        let head = word(&window, 0).unwrap_or_default();
        match event_statement(&window, head) {
            Some(Ok(lowered)) => {
                statements.push(lowered);
                statement_spans.push(span);
            }
            Some(Err(message)) => diagnostics.push(diagnostic("Q058", span, message)),
            None => diagnostics.push(diagnostic(
                "Q058",
                span,
                format!("`{head}` is not a statement `source.event.v1` models"),
            )),
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(LoweredEventModule {
        entry: entry.to_owned(),
        program: EventSourceProgram {
            entry: entry.to_owned(),
            declared_effects,
            statements,
        },
        statement_spans,
        scope_span,
    })
}
