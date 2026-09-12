//! Exact source lowering for `source.buffer.v1` — the `std.buffer` bundled
//! module (card `STD-BUFFER`).
//!
//! ```text
//! module    ::= import* (error-decl | entry)*
//! import    ::= "import" ("std.buffer" | "std.fmt" | "std.io" | "std.logger") ";"
//! entry     ::= "fn" NAME "(" ")" "->" "unit"
//!               ["effects" "{" EFFECT {"," EFFECT} "}"] "{" stmt* "}"
//! stmt      ::= "let" NAME "=" ["-"] INTEGER ";"
//!             | "let" NAME "=" "bytes" "(" INTEGER ")" ";"     -- acquire
//!             | "let" NAME "=" "extent" "(" NAME ")" ";"       -- bind the length
//!             | "let" NAME "=" NAME "[" OPERAND "]" ";"        -- read one byte
//!             | NAME "[" OPERAND "]" "=" OPERAND ";"           -- write one byte
//!             | "drop" NAME ";"                                -- release
//!             | "print" RENDERING OPERAND ";"                  -- std.io + std.fmt
//!             | "log" LEVEL RENDERING OPERAND ";"              -- std.logger + std.fmt
//!             | "flush" ";"                                    -- std.logger
//!             | "return" ";"
//! OPERAND   ::= ["-"] INTEGER | NAME
//! EFFECT    ::= "io" | "alloc" | "mut"
//! ```
//!
//! # Two reserved words, and one measured collision
//!
//! **`bytes` and `extent`.** Everything else reuses a word or a symbol the
//! language already has: `let`, `drop` and `return` are already reserved, and
//! `[`/`]` are already lexed and are consumed today only in a *usage-interval*
//! position (`[lo, inf]`) inside type and quantity annotations, never in a
//! statement body. So `b[0] = 65;` parses as an ordinary
//! `StatementKind::Assignment` and `let x = b[0];` as an ordinary
//! `StatementKind::Let`, with **no parser change at all**.
//!
//! `drop b;` is deliberately not a new `release` keyword: the operation *is*
//! [`qcore_core::ownership::OwnershipOp::Drop`], so a new word would have been
//! a synonym for one the language already spells.
//!
//! The constructor is spelled `bytes` because **`buffer` collides, measured**:
//! it occurs 555 times across the source-side corpus, and — unlike a parameter
//! name, which the referenced-names filter already shields because it is
//! followed by `:` — it occurs in *binding* position in shipped, pinned
//! fixtures (`let buffer = qcore_buffer_new(1)?;` and `drop buffer;` in
//! `examples/threads/shared_readers.qcore`). Reserving it would refuse those
//! files at `Q013`, so this card's syntax was renamed rather than the pinned
//! surface edited — the `work` → `effort` and `error` → `fatal` precedent. The
//! *module* is still `std.buffer`: a module name is a dotted import path, not a
//! reserved word.
//!
//! Both new words are load-bearing in the same measured sense `grow` and `rank`
//! are: name resolution collects the tokens after a `let`'s `=` as referenced
//! names, so without reservation `let b = bytes(8);` refuses at `Q013`.
//!
//! # The operand rule is half the effect discipline
//!
//! An `OPERAND` is an integer literal or a name this entry bound to a
//! check-time-constant integer. Neither form can denote a buffer, so an index
//! and a stored byte can never be one. A buffer is named only at the four
//! custody positions, and each of those must name one. That is why every access
//! is decided before emission — see [`qcore_core::std_buffer`]'s header on why
//! decidability is load-bearing for this route's evidence.
//!
//! # What this lowering does not establish
//!
//! It models no string, no encoding, no text and no literal — none exists. It
//! models no varying index, no reallocation, no sub-range borrow, and no
//! crossing of the extern or task boundary; each is a named follow-on in
//! `docs/design/STD_BUFFER_V1.md` §9 with its missing capability stated.

use std::collections::BTreeMap;

use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, TypedSourceProgram,
};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use crate::source_std_io::{FLUSH_KEYWORD, LOG_KEYWORD, PRINT_KEYWORD};
use qcore_core::std_buffer::{BufferEvent, BufferOperand, BufferProgram, MAX_EXTENT};
use qcore_core::typed_errors::{Effect, EffectRow};

/// The acquisition keyword. Spelled `bytes` and not `buffer` — see the module
/// header for the measured collision.
pub const BYTES_KEYWORD: &str = "bytes";

/// The extent keyword.
pub const EXTENT_KEYWORD: &str = "extent";

/// The release keyword. Deliberately the **existing** reserved `drop`.
pub const DROP_KEYWORD: &str = "drop";

/// A complete lowering of one entry. The core checker still has to validate the
/// program; this type is not evidence on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredBufferModule {
    pub entry: String,
    pub program: BufferProgram,
    /// Source span of the event at the same index. Every event has one.
    pub event_spans: Vec<Span>,
    /// Fallback span for a refusal that names no event.
    pub scope_span: Span,
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

fn symbol(window: &[&Token], cursor: usize, expected: Symbol) -> bool {
    matches!(
        window.get(cursor).map(|token| &token.kind),
        Some(TokenKind::Symbol(symbol)) if *symbol == expected
    )
}

fn ends_with_semicolon(window: &[&Token], cursor: usize) -> bool {
    symbol(window, cursor, Symbol::Semicolon) && window.len() == cursor + 1
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

fn operand(window: &[&Token], cursor: usize) -> Option<(BufferOperand, usize)> {
    if let Some((value, next)) = integer(window, cursor) {
        return Some((BufferOperand::Literal(value), next));
    }
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Word(name)) => Some((BufferOperand::Scalar(name.clone()), cursor + 1)),
        _ => None,
    }
}

/// Which of this card's constructs a statement uses, if any. Used by the import
/// gate and by route selection, so a file can never be routed here without
/// having needed the import, or need the import without being routed here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BufferUse {
    /// `let NAME = bytes(N);`
    Acquire,
    /// `NAME[I] = V;`
    Store,
    /// `let NAME = NAME[I];`
    Load,
    /// `let NAME = extent(NAME);`
    Extent,
}

/// Classify one statement's token window as a buffer construct.
///
/// The **one** place that decides whether a statement is this route's, so route
/// selection, the import gate and the lowering below cannot disagree about it.
/// `drop NAME;` is deliberately **not** here: `drop` is an established
/// statement of the language, and a module containing only `drop` is not a
/// buffer module.
fn buffer_use(window: &[&Token]) -> Option<BufferUse> {
    if word(window, 0) == Some("let") {
        if !symbol(window, 2, Symbol::Equal) {
            return None;
        }
        return match word(window, 3) {
            Some(BYTES_KEYWORD) => Some(BufferUse::Acquire),
            Some(EXTENT_KEYWORD) => Some(BufferUse::Extent),
            // `let NAME = NAME [ … ]`, where the bracket holds no `:`.
            Some(_)
                if symbol(window, 4, Symbol::LeftBracket) && !bracket_holds_a_colon(window, 4) =>
            {
                Some(BufferUse::Load)
            }
            _ => None,
        };
    }
    // `NAME [ … ] = …`
    if word(window, 0).is_some()
        && symbol(window, 1, Symbol::LeftBracket)
        && !bracket_holds_a_colon(window, 1)
    {
        return Some(BufferUse::Store);
    }
    None
}

/// Whether the bracket opened at `cursor` contains a `:` before its `]`.
///
/// Card `STRING-SLICE`, and the **only** change this card made to the buffer
/// route. `source.string.v1`'s slice `let t = s[0:3];` shares its head with this
/// route's byte read `let x = b[0];` — a `let`, a name, a `[` — so before the
/// colon test this classifier claimed both, and a slice written with no imports
/// at all reported a missing `std.buffer` import for a buffer it never named.
///
/// The colon decides it, in the one place this route says what its statements
/// are, so the two routes cannot disagree about which owns a statement. A byte
/// access of this route never contains a colon: its index is
/// `["-"] INTEGER | NAME` and neither form can hold one. No shipped fixture
/// contains one either, so this route's accepted set and its emitted C are
/// unchanged.
fn bracket_holds_a_colon(window: &[&Token], cursor: usize) -> bool {
    let mut at = cursor + 1;
    while at < window.len() {
        if symbol(window, at, Symbol::RightBracket) {
            return false;
        }
        if symbol(window, at, Symbol::Colon) {
            return true;
        }
        at += 1;
    }
    false
}

/// Every use of this card's vocabulary in the program, with the span of the
/// first occurrence of each kind.
#[must_use]
pub fn buffer_uses(program: &SourceProgram) -> BTreeMap<BufferUse, Span> {
    let tokens = program.lexical_tokens();
    let mut found: BTreeMap<BufferUse, Span> = BTreeMap::new();

    fn scan(block: &SourceBlock, tokens: &[Token], found: &mut BTreeMap<BufferUse, Span>) {
        for statement in &block.statements {
            let window = window(tokens, statement.span);
            if let Some(kind) = buffer_use(&window) {
                found.entry(kind).or_insert(statement.span);
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

/// Whether this module is a `source.buffer.v1` module.
#[must_use]
pub fn has_source_buffer(program: &SourceProgram) -> bool {
    !buffer_uses(program).is_empty()
}

/// The span of the first statement needing `import std.buffer;`.
#[must_use]
pub fn buffer_span(program: &SourceProgram) -> Option<Span> {
    buffer_uses(program).values().next().copied()
}

fn diagnostic(code: &'static str, span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code,
        pass: SemanticPass::StandardBuffer,
        span,
        message: message.into(),
    }
}

/// The entry's declared `effects { … }` row. An absent suffix is the empty row,
/// which admits nothing, so the opt-in stays visible.
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
            "Q051",
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
                            "Q051",
                            span,
                            format!(
                                "`{other}` is not an effect `source.buffer.v1` models; use io, \
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
                return Err(diagnostic("Q051", span, "this `effects` row is not closed"));
            }
        }
    }
    Ok(EffectRow::new(effects))
}

/// Lower one `let` statement. Four shapes share the head, so they are
/// distinguished here and in one place.
fn lower_let(
    window: &[&Token],
    span: Span,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<BufferEvent> {
    let Some(binding) = word(window, 1) else {
        diagnostics.push(diagnostic("Q051", span, "this `let` binds no name"));
        return None;
    };
    if !symbol(window, 2, Symbol::Equal) {
        diagnostics.push(diagnostic(
            "Q051",
            span,
            "`source.buffer.v1` binds `let NAME = …;` only",
        ));
        return None;
    }
    // `let NAME = bytes ( INTEGER ) ;`
    if word(window, 3) == Some(BYTES_KEYWORD) {
        if !symbol(window, 4, Symbol::LeftParen) {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "`bytes(INTEGER)` takes its extent in parentheses",
            ));
            return None;
        }
        let Some((extent, next)) = integer(window, 5) else {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "`bytes(INTEGER)` takes one integer literal extent; a computed extent is not a \
                 check-time constant and `source.buffer.v1` decides every access before emission",
            ));
            return None;
        };
        if !symbol(window, next, Symbol::RightParen) || !ends_with_semicolon(window, next + 1) {
            diagnostics.push(diagnostic("Q051", span, "this `bytes(…)` is not closed"));
            return None;
        }
        // The range refusal itself is the core checker's (`Q053`); this is the
        // narrower question of whether the literal is representable as an
        // extent at all, which has to be answered before the event exists.
        let Ok(extent) = u32::try_from(extent) else {
            diagnostics.push(diagnostic(
                "Q053",
                span,
                format!(
                    "{extent} is not an extent `source.buffer.v1` models; a buffer is 1 to \
                     {MAX_EXTENT} bytes"
                ),
            ));
            return None;
        };
        return Some(BufferEvent::Acquire {
            name: binding.to_owned(),
            extent,
        });
    }
    // `let NAME = extent ( NAME ) ;`
    if word(window, 3) == Some(EXTENT_KEYWORD) {
        let Some(name) = word(window, 5) else {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "`extent(NAME)` names the buffer whose length it binds",
            ));
            return None;
        };
        if !symbol(window, 4, Symbol::LeftParen)
            || !symbol(window, 6, Symbol::RightParen)
            || !ends_with_semicolon(window, 7)
        {
            diagnostics.push(diagnostic("Q051", span, "this `extent(…)` is not closed"));
            return None;
        }
        return Some(BufferEvent::Extent {
            binding: binding.to_owned(),
            name: name.to_owned(),
        });
    }
    // `let NAME = NAME [ OPERAND ] ;`
    if let Some(name) = word(window, 3)
        && symbol(window, 4, Symbol::LeftBracket)
    {
        let Some((index, next)) = operand(window, 5) else {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "a byte read indexes with one integer literal or one bound integer name",
            ));
            return None;
        };
        if !symbol(window, next, Symbol::RightBracket) || !ends_with_semicolon(window, next + 1) {
            diagnostics.push(diagnostic("Q051", span, "this byte read is not closed"));
            return None;
        }
        return Some(BufferEvent::Load {
            binding: binding.to_owned(),
            name: name.to_owned(),
            index,
        });
    }
    // `let NAME = ["-"] INTEGER ;`
    match integer(window, 3) {
        Some((value, next)) if ends_with_semicolon(window, next) => Some(BufferEvent::Bind {
            binding: binding.to_owned(),
            value,
        }),
        _ => {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "`source.buffer.v1` binds an integer literal, `bytes(INTEGER)`, `extent(NAME)` or \
                 one byte read; nothing else",
            ));
            None
        }
    }
}

/// Lower one `NAME [ OPERAND ] = OPERAND ;` statement.
fn lower_store(
    window: &[&Token],
    span: Span,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<BufferEvent> {
    let name = word(window, 0)?.to_owned();
    let Some((index, next)) = operand(window, 2) else {
        diagnostics.push(diagnostic(
            "Q051",
            span,
            "a byte write indexes with one integer literal or one bound integer name",
        ));
        return None;
    };
    if !symbol(window, next, Symbol::RightBracket) || !symbol(window, next + 1, Symbol::Equal) {
        diagnostics.push(diagnostic("Q051", span, "this byte write is not closed"));
        return None;
    }
    let Some((value, next)) = operand(window, next + 2) else {
        diagnostics.push(diagnostic(
            "Q051",
            span,
            "a byte write stores one integer literal or one bound integer name",
        ));
        return None;
    };
    if !ends_with_semicolon(window, next) {
        diagnostics.push(diagnostic("Q051", span, "this byte write is not closed"));
        return None;
    }
    Some(BufferEvent::Store { name, index, value })
}

/// Read one statement as a `source.buffer.v1` event, reporting a refusal as a
/// **message** the caller attributes to its own diagnostic code.
///
/// Published for card `STRING-CONCAT`'s composed string/buffer route, on the
/// `source_std_io::std_io_statement` shape and for its reason: a composed route
/// must read this route's statements with **this route's own reader**, so a
/// shape one route accepts and the other refuses is not expressible. The code
/// is the caller's because §L21 gives each semantic pass its own range; the
/// message is this route's, because the fault is this route's to describe.
///
/// `None` means the statement is not one this route spells at all.
#[must_use]
pub fn buffer_statement(
    parsed: &SourceProgram,
    span: Span,
) -> Option<Result<BufferEvent, String>> {
    let tokens = parsed.lexical_tokens();
    let window = window(&tokens, span);
    let mut diagnostics = Vec::new();
    let event = match word(&window, 0) {
        Some("let") => {
            buffer_use(&window)?;
            lower_let(&window, span, &mut diagnostics)
        }
        Some(DROP_KEYWORD) if ends_with_semicolon(&window, 2) => {
            let name = word(&window, 1)?;
            Some(BufferEvent::Release {
                name: name.to_owned(),
            })
        }
        _ => {
            buffer_use(&window)?;
            lower_store(&window, span, &mut diagnostics)
        }
    };
    match event {
        Some(event) => Some(Ok(event)),
        None => Some(Err(diagnostics
            .into_iter()
            .next()
            .map_or_else(|| "this is not a statement `source.buffer.v1` models".to_owned(), |diagnostic| diagnostic.message))),
    }
}

/// Lower one entry into a [`BufferProgram`]. The module is either entirely
/// inside the grammar above or refused with a precise diagnostic; there is no
/// partially lowered middle state.
pub fn lower_source_buffer(
    parsed: &SourceProgram,
    _typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredBufferModule, Vec<SourceDiagnostic>> {
    let tokens = parsed.lexical_tokens();
    let mut diagnostics = Vec::new();

    let Some(declaration) = parsed.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
    }) else {
        return Err(vec![diagnostic(
            "Q051",
            Span { start: 0, end: 0 },
            format!("`{entry}` is not a function declaration in this module"),
        )]);
    };
    let scope_span = declaration.span;

    // Every buffer construct must be in the entry. A helper function carrying
    // one is refused rather than silently unmodeled.
    for other in &parsed.declarations {
        if other.value.kind != DeclarationKind::Function || other.value.name == entry {
            continue;
        }
        if let Some(body) = &other.value.body {
            let mut found = BTreeMap::new();
            fn scan(block: &SourceBlock, tokens: &[Token], found: &mut BTreeMap<BufferUse, Span>) {
                for statement in &block.statements {
                    let window = window(tokens, statement.span);
                    if let Some(kind) = buffer_use(&window) {
                        found.entry(kind).or_insert(statement.span);
                    }
                    for nested in statement.value.nested_blocks() {
                        scan(nested, tokens, found);
                    }
                }
            }
            scan(body, tokens, &mut found);
            if let Some(span) = found.values().next() {
                diagnostics.push(diagnostic(
                    "Q051",
                    *span,
                    format!(
                        "`source.buffer.v1` models the entry only; `{}` is not the entry `{entry}`",
                        other.value.name
                    ),
                ));
            }
        }
    }

    if !declaration.value.parameters.is_empty() {
        diagnostics.push(diagnostic(
            "Q051",
            scope_span,
            "`source.buffer.v1` models an entry with no parameters",
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
        diagnostics.push(diagnostic("Q051", scope_span, "this entry has no body"));
        return Err(diagnostics);
    };

    let mut events = Vec::new();
    let mut event_spans = Vec::new();

    for statement in &body.statements {
        let span = statement.span;
        if !statement.value.nested_blocks().is_empty() {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "`source.buffer.v1` models one flat statement sequence; this statement opens a \
                 nested block",
            ));
            continue;
        }
        let window = window(tokens, span);
        let head = word(&window, 0).unwrap_or_default();
        let event = match head {
            "let" => lower_let(&window, span, &mut diagnostics),
            DROP_KEYWORD => {
                let Some(name) = word(&window, 1) else {
                    diagnostics.push(diagnostic("Q051", span, "`drop NAME;` names one buffer"));
                    continue;
                };
                if ends_with_semicolon(&window, 2) {
                    Some(BufferEvent::Release {
                        name: name.to_owned(),
                    })
                } else {
                    diagnostics.push(diagnostic(
                        "Q051",
                        span,
                        "`source.buffer.v1` releases with `drop NAME;`",
                    ));
                    None
                }
            }
            PRINT_KEYWORD | LOG_KEYWORD | FLUSH_KEYWORD => {
                // The line grammar is `source.std-io.v1`'s, read by its own
                // reader, so a shape one route accepts and another refuses is
                // not expressible.
                match crate::source_std_io::std_io_statement(parsed, span) {
                    Some(Ok(event)) => Some(BufferEvent::Line(event)),
                    Some(Err(message)) => {
                        diagnostics.push(diagnostic("Q051", span, message));
                        None
                    }
                    None => {
                        diagnostics.push(diagnostic(
                            "Q051",
                            span,
                            "this line-producing statement is not one `source.std-io.v1` models",
                        ));
                        None
                    }
                }
            }
            "return" => {
                if ends_with_semicolon(&window, 1) {
                    Some(BufferEvent::Return)
                } else {
                    diagnostics.push(diagnostic(
                        "Q051",
                        span,
                        "`source.buffer.v1` models `return;` from a `unit` entry",
                    ));
                    None
                }
            }
            other => {
                // A byte write has a *name* at its head, so it is recognised by
                // shape rather than by keyword — which is exactly why it needs
                // no reserved word.
                if symbol(&window, 1, Symbol::LeftBracket) {
                    lower_store(&window, span, &mut diagnostics)
                } else {
                    diagnostics.push(diagnostic(
                        "Q051",
                        span,
                        format!("`{other}` is not a statement `source.buffer.v1` models"),
                    ));
                    None
                }
            }
        };
        if let Some(event) = event {
            events.push(event);
            event_spans.push(span);
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(LoweredBufferModule {
        entry: entry.to_owned(),
        program: BufferProgram {
            entry: entry.to_owned(),
            declared_effects,
            events,
        },
        event_spans,
        scope_span,
    })
}
