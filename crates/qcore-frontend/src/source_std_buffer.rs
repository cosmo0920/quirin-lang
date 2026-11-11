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

//! Exact source lowering for `source.buffer.v1` — the `std.buffer` bundled
//! module (card `STD-BUFFER`).
//!
//! ```text
//! module    ::= import* (error-decl | entry)*
//! import    ::= "import" ("std.buffer" | "std.fmt" | "std.io" | "std.logger") ";"
//! entry     ::= "fn" NAME "(" ")" "->" "unit"
//!               ["effects" "{" EFFECT {"," EFFECT} "}"] "{" stmt* "}"
//! stmt      ::= "let" NAME "=" ["-"] INTEGER ";"
//!             | "let" NAME "=" NAME ";"                        -- the copy bind
//!                                                              -- (card OWNED-FIELD-ROUTE-REACH)
//!             | "let" NAME "=" "bytes" "(" INTEGER ")" ";"     -- acquire
//!             | "let" NAME "=" "extent" "(" NAME ")" ";"       -- bind the length
//!             | "let" NAME "=" "lend" "(" NAME "," OPERAND "," OPERAND ")" ";" -- write lend
//!             | "let" NAME "=" NAME "[" OPERAND "]" ";"        -- read one byte
//!             | NAME "[" OPERAND "]" "=" OPERAND ";"           -- write one byte
//!             | "drop" NAME ";"                                -- release
//!             | "let" NAME "=" "extent" "(" NAME "." FIELD ")" ";"  -- through an owned field
//!             | "let" NAME "=" NAME "." FIELD "[" OPERAND "]" ";"   -- (card AGGREGATE-OWNED-FIELD;
//!             | NAME "." FIELD "[" OPERAND "]" "=" OPERAND ";"      --  the shapes are
//!             | "drop" NAME "." FIELD ";"                            --  source_record's)
//!             | "print" RENDERING OPERAND ";"                  -- std.io + std.fmt
//!             | "log" LEVEL RENDERING OPERAND ";"              -- std.logger + std.fmt
//!             | "flush" ";"                                    -- std.logger
//!             | [attr] "for" NAME "in" "0" ".." BOUND "{" stmt* "}"  -- source.loop.v1
//!             | "return" ";"
//! attr      ::= "[|" "#" "unroll" "," "where" "#" "bound" "<" INTEGER "|]"
//! OPERAND   ::= ["-"] INTEGER | NAME
//! BOUND     ::= INTEGER | NAME
//! EFFECT    ::= "io" | "alloc" | "mut"
//! ```
//!
//! # The one nested block this route models
//!
//! Card `LOOP-V1` admits `source.loop.v1`'s bounded counted loop here, and it is
//! the **only** nested block this route reads: every other one still refuses at
//! `Q051` with "this statement opens a nested block". The loop's own grammar,
//! its bound tier and its per-iteration custody rule are
//! [`crate::source_loop`]'s and [`qcore_core::source_loop`]'s, reported at
//! `Q097`–`Q099`; what this route supplies is the body's **statements**, read by
//! the reader below, so a shape one admits and the other refuses is not
//! expressible.
//!
//! # The copy bind, and the collision it keeps
//!
//! `let NAME = NAME;` (card `OWNED-FIELD-ROUTE-REACH`, wave 41) asks the
//! ownership IR whether a second binding may hold what a name holds. It
//! reserves nothing — four things the lexer already produces — and it is
//! **deliberately not a [`buffer_use`]**: that function decides route selection
//! and is how the composed string/buffer route reads this route's statements,
//! so claiming the shape there would move both. The card's census found the
//! identical token shape **29 times** in the tree, every occurrence
//! `source.socket.v1`'s `let listener = tcp;`, and the two constructs are told
//! apart by route selection and by nothing else — pinned, not argued
//! (`COPY-BIND-SOCKET-SHAPE-COLLISION`).
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
//! `examples/threads/shared_readers.qr`). Reserving it would refuse those
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
use qcore_core::scalar_types::{FloatEffect, FloatEffects, ScalarType};
use qcore_core::source_record::{FieldType, RecordType};
use qcore_core::std_buffer::{BufferEvent, BufferOperand, BufferProgram, FieldOperand, MAX_EXTENT};
use qcore_core::typed_errors::{Effect, EffectRow};

/// The acquisition keyword. Spelled `bytes` and not `buffer` — see the module
/// header for the measured collision.
pub const BYTES_KEYWORD: &str = "bytes";

/// The extent keyword.
pub const EXTENT_KEYWORD: &str = "extent";

/// The release keyword. Deliberately the **existing** reserved `drop`.
pub const DROP_KEYWORD: &str = "drop";

/// The write-lend clause word (card `SLICE-PERMISSIONS-WRITE-LEND`).
///
/// **Contextual, deliberately not reserved** — the `len`/`concat`/`borrow_utf8`
/// discipline: it is recognised only as `lend` immediately followed by `(` in
/// a `let`'s value position, so `let lend = 5;` is still a program and the
/// constraint-14 census stays at **zero new reserved words**. The read card's
/// census (`ct07-slice-permissions` §7) already measured `lend` free at zero
/// source-position occurrences and recorded it for this card.
pub const LEND_KEYWORD: &str = "lend";

/// Whether `word` at a clause-head position, followed by `next`, is this
/// route's contextual `lend(` — the same shape
/// `source_std_string::is_string_clause_word` decides for the string route's
/// three words, and consumed by the same reference-collection site.
#[must_use]
pub fn is_lend_clause_word(word: &str, next: Option<&TokenKind>) -> bool {
    word == LEND_KEYWORD && matches!(next, Some(TokenKind::Symbol(Symbol::LeftParen)))
}

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
    /// `let NAME = lend(NAME, LOW, HIGH);` (card `SLICE-PERMISSIONS-WRITE-LEND`)
    Lend,
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
            // Contextual: only `lend(` is this route's, so a scalar named
            // `lend` keeps every reading it had (card
            // `SLICE-PERMISSIONS-WRITE-LEND`).
            Some(LEND_KEYWORD) if symbol(window, 4, Symbol::LeftParen) => Some(BufferUse::Lend),
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

/// The span of the first statement needing `#import std.buffer;`.
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
///
/// Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: the row may also name `partial_eq` and
/// `partial_ord`, the float comparison effects, returned beside the row as
/// the extern route carries them; they are what a float comparison needs and
/// are never inferred.
fn declared_effect_row(
    tokens: &[TokenKind],
    span: Span,
) -> Result<(EffectRow, FloatEffects), SourceDiagnostic> {
    let Some(index) = tokens
        .iter()
        .position(|token| matches!(token, TokenKind::Word(word) if word == "effects"))
    else {
        return Ok((EffectRow::default(), FloatEffects::default()));
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
    let mut comparisons = FloatEffects::default();
    let mut cursor = index + 2;
    loop {
        match tokens.get(cursor) {
            Some(TokenKind::Symbol(Symbol::RightBrace)) => break,
            Some(TokenKind::Word(name)) if FloatEffect::parse(name).is_some() => {
                comparisons.insert(FloatEffect::parse(name).expect("parsed above"));
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
    Ok((EffectRow::new(effects), comparisons))
}

/// Lower one `let` statement. Four shapes share the head, so they are
/// distinguished here and in one place.
fn lower_let(
    window: &[&Token],
    span: Span,
    constants: &LoweringConstants,
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
    // Card `AGGREGATES-V1`: the two aggregate forms, tried **first**.
    //
    // Both are told apart from every other `let` by the token at index 4 — a
    // `{` or a `.` — and no other form of this route has either there: `bytes`
    // and `extent` take a `(`, a byte read takes a `[`, and a literal bind takes
    // a digit or a `-`. Trying them first is therefore free of ambiguity, and it
    // is what makes `type extent { … }` constructible rather than swallowed by
    // the `extent(NAME)` branch below.
    if let Some(event) = lower_record(window, span, constants, diagnostics) {
        return event;
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
        // Card `AGGREGATE-OWNED-FIELD`: `extent(RECORD.FIELD)`, the extent
        // of the buffer an owned field holds — since card
        // `W48-QR-USABLE-RECORDS`, at a path.
        if symbol(window, 4, Symbol::LeftParen) && symbol(window, 6, Symbol::Dot) {
            let Some((path, after)) = crate::source_record::read_path(window, 6) else {
                diagnostics.push(diagnostic(
                    "Q051",
                    span,
                    "`extent(RECORD.FIELD)` names the owned field whose length it binds",
                ));
                return None;
            };
            if !symbol(window, after, Symbol::RightParen) || !ends_with_semicolon(window, after + 1)
            {
                diagnostics.push(diagnostic("Q051", span, "this `extent(…)` is not closed"));
                return None;
            }
            return Some(BufferEvent::FieldExtent {
                binding: binding.to_owned(),
                record: name.to_owned(),
                path,
            });
        }
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
    // `let NAME = lend ( NAME , OPERAND , OPERAND ) ;` — the write lend (card
    // `SLICE-PERMISSIONS-WRITE-LEND`). Contextual: only the `lend(` shape is
    // this route's, exactly as `buffer_use` decides it.
    if word(window, 3) == Some(LEND_KEYWORD) && symbol(window, 4, Symbol::LeftParen) {
        let Some(name) = word(window, 5) else {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "`lend(NAME, LOW, HIGH)` names the owned buffer whose bytes it lends",
            ));
            return None;
        };
        if !symbol(window, 6, Symbol::Comma) {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "`lend(NAME, LOW, HIGH)` lends a range and needs both of its ends; a lend of a \
                 whole buffer is `lend(NAME, 0, extent)` with the extent bound by `extent(NAME)`",
            ));
            return None;
        }
        let Some((low, next)) = operand(window, 7) else {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "`lend(NAME, LOW, HIGH)` bounds the lent range with an integer literal or a \
                 name bound to one",
            ));
            return None;
        };
        if !symbol(window, next, Symbol::Comma) {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "`lend(NAME, LOW, HIGH)` lends a range and needs both of its ends",
            ));
            return None;
        }
        let Some((high, next)) = operand(window, next + 1) else {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "`lend(NAME, LOW, HIGH)` bounds the lent range with an integer literal or a \
                 name bound to one",
            ));
            return None;
        };
        if !symbol(window, next, Symbol::RightParen) || !ends_with_semicolon(window, next + 1) {
            diagnostics.push(diagnostic("Q051", span, "this `lend(…)` is not closed"));
            return None;
        }
        return Some(BufferEvent::Lend {
            binding: binding.to_owned(),
            name: name.to_owned(),
            low,
            high,
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
    // `let NAME = OPERAND as TYPE ;` — **the conversion** (card
    // `W48-QR-SCALAR-TYPES-R3`), §G02t's one spelling. Told apart by the
    // already-reserved `as` after one operand and a scalar type name after it;
    // name resolution already skips a type name after `as`, and the census of
    // `w48-qr-scalar-types` found the shape in no tracked source. Like the copy
    // bind it is **not** a `buffer_use`, so it selects no route.
    if let Some((operand, next)) = operand(window, 3)
        && word(window, next) == Some("as")
    {
        // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: `f32`/`f64` are conversion
        // targets here too; the checker applies §G02t's float rules.
        let Some(to) = word(window, next + 1).and_then(ScalarType::parse_admitting_floats) else {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "`as` converts to one fixed-width scalar type (bool, i8, i16, i32, i64, isize, \
                 u8, u16, u32, u64, usize)",
            ));
            return None;
        };
        if !ends_with_semicolon(window, next + 2) {
            diagnostics.push(diagnostic(
                "Q051",
                span,
                "`source.buffer.v1` converts one operand per statement, \
                 `let NAME = OPERAND as TYPE;`, and carries no arithmetic",
            ));
            return None;
        }
        return Some(BufferEvent::Convert {
            binding: binding.to_owned(),
            operand,
            to,
        });
    }
    // `let NAME = VALUE_EXPRESSION ;` — **the value statement** (card
    // `W48-QR-SCALAR-TYPES-R5`, `W48-QR-BUFFER-ROUTE-ARITHMETIC`): one
    // arithmetic operation, comparison or negation, read by the reader the
    // extern and task routes share (`source_value_statement`), so the three
    // routes read one grammar. Its `as` form is this route's conversion,
    // already read above; a shape the reader does not claim (a bare name, a
    // literal) falls through to the forms below, unchanged. Like the
    // conversion it is **not** a `buffer_use`, so it selects no route.
    {
        let rest: Vec<Token> = window[3..].iter().map(|token| (*token).clone()).collect();
        // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: the route reads float
        // spellings — `["-"] FLOAT` operands and `as f32`/`as f64` — with the
        // reader the extern and task routes admit floats through, so the
        // three routes read one grammar.
        match crate::source_value_statement::parse_value_expression_admitting_floats(
            &rest,
            |name: &str| Some(name.to_owned()),
        ) {
            Some(Ok(syntax)) => {
                use crate::source_value_statement::{
                    ValueExpressionSyntax as Syntax, ValueOperandSyntax as Operand,
                };
                use qcore_core::std_buffer::{ValueOperand, ValueOperation};
                let operand = |operand: Operand<String>| match operand {
                    Operand::Value(name) => ValueOperand::Name(name),
                    Operand::Integer(value) => ValueOperand::Integer(value),
                    Operand::Bool(value) => ValueOperand::Bool(value),
                    Operand::Float(literal) => ValueOperand::Float(literal),
                };
                let operation = match syntax {
                    Syntax::Binary(op, a, b) => ValueOperation::Binary(op, operand(a), operand(b)),
                    Syntax::Compare(cmp, a, b) => {
                        ValueOperation::Compare(cmp, operand(a), operand(b))
                    }
                    Syntax::Negate(a) => ValueOperation::Negate(operand(a)),
                    // A float literal's conversion — the one `as` whose
                    // operand the conversion form above does not read.
                    Syntax::Convert(a @ Operand::Float(_), to) => {
                        ValueOperation::Convert(operand(a), to)
                    }
                    Syntax::Convert(..) => {
                        diagnostics.push(diagnostic(
                            "Q051",
                            span,
                            "`source.buffer.v1` converts one operand per statement, `let NAME = \
                             OPERAND as TYPE;`, where OPERAND is an `i32` literal or a bound name",
                        ));
                        return None;
                    }
                };
                return Some(BufferEvent::Compute {
                    binding: binding.to_owned(),
                    operation,
                });
            }
            Some(Err(message)) => {
                diagnostics.push(diagnostic("Q051", span, message));
                return None;
            }
            None => {}
        }
    }
    // `let NAME = NAME ;` — **the copy bind** (card `OWNED-FIELD-ROUTE-REACH`,
    // wave 41).
    //
    // Tried after every other form, so it claims nothing another shape wants:
    // `bytes`, `extent` and `lend` take a `(`, a byte read takes a `[`, the
    // two record forms take a `{` or a `.`, and a literal bind takes a digit
    // or a `-`. What is left at index 3 is a bare word followed by the
    // semicolon, which before this card was the `Q051` shape refusal below.
    //
    // The statement is **not** added to `buffer_use`, deliberately: that
    // function decides route selection and is what the composed string route
    // reads this route's statements through, so claiming a shape there that is
    // a plain value bind on every route would move both. `lower_let` is
    // reached only from this route's own entry walk.
    if let Some(source) = word(window, 3)
        && symbol(window, 4, Symbol::Semicolon)
        && window.len() == 5
    {
        return Some(BufferEvent::CopyBind {
            binding: binding.to_owned(),
            source: source.to_owned(),
        });
    }
    // `let NAME = ["-"] FLOAT ;` — the float literal binding (card
    // `W48-QR-FLOAT-BUFFER-PAYLOADS`), an `f64` as on the extern route: the
    // literal read once at `f64`, which is the value statement `FLOAT as f64`.
    // A digit or a `-` at index 3, like the integer binding below, so it
    // claims no other form's shape.
    {
        let (negative, at) = if symbol(window, 3, Symbol::Minus) {
            (true, 4)
        } else {
            (false, 3)
        };
        if let Some(TokenKind::Float(text)) = window.get(at).map(|token| &token.kind)
            && ends_with_semicolon(window, at + 1)
        {
            let Some(literal) = qcore_core::scalar_floats::FloatLiteral::read(text, negative)
            else {
                diagnostics.push(diagnostic(
                    "Q051",
                    span,
                    format!("`{text}` is not a float literal"),
                ));
                return None;
            };
            return Some(BufferEvent::Compute {
                binding: binding.to_owned(),
                operation: qcore_core::std_buffer::ValueOperation::Convert(
                    qcore_core::std_buffer::ValueOperand::Float(literal),
                    ScalarType::F64,
                ),
            });
        }
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
                "`source.buffer.v1` binds an integer literal, a name this entry bound, \
                 `bytes(INTEGER)`, `extent(NAME)`, `lend(NAME, LOW, HIGH)` or one byte read; \
                 nothing else",
            ));
            None
        }
    }
}

/// Lower one record construction or field read, if this statement is one (card
/// `AGGREGATES-V1`).
///
/// The outer `Option` says whether this statement is a record form at all; the
/// inner one is the ordinary "lowered, or refused with a diagnostic already
/// pushed" answer. The *grammar* is [`crate::source_record`]'s, read by that
/// module's own reader, and the *field values* are this route's operands, read
/// by the reader above — so neither half is written twice.
fn lower_record(
    window: &[&Token],
    span: Span,
    constants: &LoweringConstants,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<Option<BufferEvent>> {
    let read = crate::source_record::read_record_statement(window)?;
    let statement = match read {
        Ok(statement) => statement,
        Err(message) => {
            // `Q051` is this pass's **shape** condition, widened by this card to
            // cover a malformed record declaration, construction or field read.
            // The card allocates no code; see `qcore_core::source_record`.
            diagnostics.push(diagnostic("Q051", span, message));
            return Some(None);
        }
    };
    match statement {
        crate::source_record::RecordStatement::Field {
            binding,
            record,
            path,
        } => Some(Some(BufferEvent::Field {
            binding,
            record,
            path,
        })),
        // Card `AGGREGATE-OWNED-FIELD`: the index is this route's operand,
        // read by the reader above out of the span the record reader returned.
        crate::source_record::RecordStatement::FieldLoad {
            binding,
            record,
            path,
            index: (start, end),
        } => {
            let Some((index, next)) = operand(window, start) else {
                diagnostics.push(diagnostic(
                    "Q051",
                    span,
                    "a byte read through a field indexes with one integer literal or one bound \
                     integer name",
                ));
                return Some(None);
            };
            if next != end {
                diagnostics.push(diagnostic(
                    "Q051",
                    span,
                    "a byte read through a field indexes with exactly one operand",
                ));
                return Some(None);
            }
            Some(Some(BufferEvent::FieldLoad {
                binding,
                record,
                path,
                index,
            }))
        }
        crate::source_record::RecordStatement::Construct {
            binding,
            type_name,
            fields,
        } => {
            let mut lowered = Vec::new();
            for (field, (start, end)) in fields {
                // Card `W48-QR-SCALAR-TYPES-R3`: a field declared a scalar type
                // other than `i32` reads its value at that type — a literal of
                // any magnitude (the checker range-checks it), `true`/`false`,
                // or a name. Every other field (an `i32`, a `bytes`, one the
                // type does not declare) is read exactly as before the card.
                if let Some(ty) = constants
                    .field_type(&type_name, &field)
                    .and_then(|declared| declared.scalar())
                    .filter(|ty| *ty != ScalarType::I32)
                {
                    match typed_field_operand(window, start, end) {
                        Ok(value) => {
                            lowered.push((field, value));
                            continue;
                        }
                        Err(message) => {
                            diagnostics.push(diagnostic(
                                "Q051",
                                span,
                                format!("`{type_name}.{field}` is declared `{ty}`: {message}"),
                            ));
                            return Some(None);
                        }
                    }
                }
                let Some((value, next)) = operand(window, start) else {
                    diagnostics.push(diagnostic(
                        "Q051",
                        span,
                        format!(
                            "`{type_name}.{field}` is supplied a value that is not one \
                             `source.buffer.v1` decides; a field takes one integer literal or one \
                             bound integer name, so that every field of a constructed record is \
                             known before emission"
                        ),
                    ));
                    return Some(None);
                };
                if next != end {
                    diagnostics.push(diagnostic(
                        "Q051",
                        span,
                        format!(
                            "`{type_name}.{field}` is supplied more than one operand; this \
                             revision carries no arithmetic, so a field value is exactly one \
                             literal or one bound name"
                        ),
                    ));
                    return Some(None);
                }
                lowered.push((field, FieldOperand::from(value)));
            }
            Some(Some(BufferEvent::Construct {
                binding,
                type_name,
                fields: lowered,
            }))
        }
    }
}

/// Read the value supplied at a field of a scalar type other than `i32` (card
/// `W48-QR-SCALAR-TYPES-R3`): `["-"] INTEGER` read as an exact integer — the
/// checker decides whether it is a value of the field's type — `true`,
/// `false`, or one name. The window range is `[start, end)`.
fn typed_field_operand(
    window: &[&Token],
    start: usize,
    end: usize,
) -> Result<FieldOperand, String> {
    let exactly = |width: usize, value: FieldOperand| {
        if start + width == end {
            Ok(value)
        } else {
            Err(
                "a field value is exactly one literal or one bound name; this revision \
                 carries no arithmetic"
                    .to_owned(),
            )
        }
    };
    let (negative, at) = match window.get(start).map(|token| &token.kind) {
        Some(TokenKind::Symbol(Symbol::Minus)) => (true, start + 1),
        _ => (false, start),
    };
    match window.get(at).map(|token| &token.kind) {
        // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: `["-"] FLOAT`, read once at the
        // field's type by the checker (which refuses it at a field that is
        // not a float).
        Some(TokenKind::Float(text)) => {
            match qcore_core::scalar_floats::FloatLiteral::read(text, negative) {
                Some(literal) => exactly(at + 1 - start, FieldOperand::Float(literal)),
                None => Err(format!("`{text}` is not a float literal")),
            }
        }
        Some(TokenKind::Integer(text)) => {
            let Ok(magnitude) = text.parse::<i128>() else {
                return Err(format!(
                    "`{text}` is not an integer any fixed-width type holds"
                ));
            };
            let value = if negative { -magnitude } else { magnitude };
            exactly(at + 1 - start, FieldOperand::Literal(value))
        }
        Some(TokenKind::Word(word)) if !negative && word == "true" => {
            exactly(1, FieldOperand::Bool(true))
        }
        Some(TokenKind::Word(word)) if !negative && word == "false" => {
            exactly(1, FieldOperand::Bool(false))
        }
        Some(TokenKind::Word(name)) if !negative => exactly(1, FieldOperand::Name(name.clone())),
        _ => Err(
            "a field takes one integer literal, `true` or `false`, or one bound name, so that \
             every field of a constructed record is known before emission"
                .to_owned(),
        ),
    }
}

/// Lower one store through a field or one field release (card
/// `AGGREGATE-OWNED-FIELD`), if this statement is one — the [`lower_record`]
/// contract, for the two field-path forms that are not a `let`.
fn lower_field_path(
    window: &[&Token],
    span: Span,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<Option<BufferEvent>> {
    let read = crate::source_record::read_field_path_statement(window)?;
    let statement = match read {
        Ok(statement) => statement,
        Err(message) => {
            diagnostics.push(diagnostic("Q051", span, message));
            return Some(None);
        }
    };
    match statement {
        crate::source_record::FieldPathStatement::Release { record, path } => {
            Some(Some(BufferEvent::FieldRelease { record, path }))
        }
        // Card `W48-QR-USABLE-RECORDS`: the field write. The value is read at
        // its written form — a literal of any magnitude, `true`/`false` or one
        // name — and the checker types it against the field it names, so an
        // `i32`, a narrower type and an owned field (whose value names a
        // buffer) are one reader.
        crate::source_record::FieldPathStatement::Write {
            record,
            path,
            value: (start, end),
        } => match typed_field_operand(window, start, end) {
            Ok(value) => Some(Some(BufferEvent::FieldWrite {
                record,
                path,
                value,
            })),
            Err(message) => {
                diagnostics.push(diagnostic(
                    "Q051",
                    span,
                    format!("a field write stores one value: {message}"),
                ));
                Some(None)
            }
        },
        crate::source_record::FieldPathStatement::Store {
            record,
            path,
            index: (index_start, index_end),
            value: (value_start, value_end),
        } => {
            let Some((index, next)) = operand(window, index_start) else {
                diagnostics.push(diagnostic(
                    "Q051",
                    span,
                    "a byte write through a field indexes with one integer literal or one bound \
                     integer name",
                ));
                return Some(None);
            };
            if next != index_end {
                diagnostics.push(diagnostic(
                    "Q051",
                    span,
                    "a byte write through a field indexes with exactly one operand",
                ));
                return Some(None);
            }
            let Some((value, next)) = operand(window, value_start) else {
                diagnostics.push(diagnostic(
                    "Q051",
                    span,
                    "a byte write through a field stores one integer literal or one bound \
                     integer name",
                ));
                return Some(None);
            };
            if next != value_end {
                diagnostics.push(diagnostic(
                    "Q051",
                    span,
                    "a byte write through a field stores exactly one operand",
                ));
                return Some(None);
            }
            Some(Some(BufferEvent::FieldStore {
                record,
                path,
                index,
                value,
            }))
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
/// A conversion or a value statement embedded in a composing route's entry
/// (card `W48-QR-SCALAR-TYPES-R6`, `W48-QR-TYPES-COMPOSED-STRING`): `let NAME =
/// OPERAND as TYPE;` or one arithmetic operation, comparison or negation,
/// read by **this** route's `lower_let` — so the composed string/buffer route
/// reads the statement exactly as this route and the extern and task routes
/// read it. It claims only those two shapes: a bare literal, a name, and
/// every other `let` form fall through (`None`) to the composing route's own
/// reader, and like the conversion here it is not a `buffer_use`, so it
/// selects no route.
pub fn value_statement(parsed: &SourceProgram, span: Span) -> Option<Result<BufferEvent, String>> {
    let tokens = parsed.lexical_tokens();
    let window = window(tokens, span);
    if word(&window, 0) != Some("let") || !symbol(&window, 2, Symbol::Equal) {
        return None;
    }
    let conversion = operand(&window, 3).is_some_and(|(_, next)| word(&window, next) == Some("as"));
    let rest: Vec<Token> = window[3..].iter().map(|token| (*token).clone()).collect();
    // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: float spellings too, read as this
    // route reads them — and the float literal binding `let NAME = FLOAT;`,
    // which is `FLOAT as f64` and which no composing route's own reader spells.
    let expression = crate::source_value_statement::parse_value_expression_admitting_floats(
        &rest,
        |name: &str| Some(name.to_owned()),
    )
    .is_some();
    let float_binding = {
        let at = if symbol(&window, 3, Symbol::Minus) {
            4
        } else {
            3
        };
        matches!(
            window.get(at).map(|token| &token.kind),
            Some(TokenKind::Float(_))
        ) && ends_with_semicolon(&window, at + 1)
    };
    if !conversion && !expression && !float_binding {
        return None;
    }
    let mut diagnostics = Vec::new();
    match lower_let(
        &window,
        span,
        &LoweringConstants::default(),
        &mut diagnostics,
    ) {
        Some(event @ (BufferEvent::Convert { .. } | BufferEvent::Compute { .. })) => {
            Some(Ok(event))
        }
        Some(_) => None,
        None => Some(Err(diagnostics.into_iter().next().map_or_else(
            || "this is not a statement `source.buffer.v1` models".to_owned(),
            |diagnostic| diagnostic.message,
        ))),
    }
}

pub fn buffer_statement(parsed: &SourceProgram, span: Span) -> Option<Result<BufferEvent, String>> {
    let tokens = parsed.lexical_tokens();
    let window = window(tokens, span);
    let mut diagnostics = Vec::new();
    let event = match word(&window, 0) {
        Some("let") => {
            buffer_use(&window)?;
            lower_let(
                &window,
                span,
                &LoweringConstants::default(),
                &mut diagnostics,
            )
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
        None => {
            let message = diagnostics.into_iter().next().map_or_else(
                || "this is not a statement `source.buffer.v1` models".to_owned(),
                |diagnostic| diagnostic.message,
            );
            Some(Err(message))
        }
    }
}

/// Lower one entry into a [`BufferProgram`]. The module is either entirely
/// inside the grammar above or refused with a precise diagnostic; there is no
/// partially lowered middle state.
pub fn lower_source_buffer(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredBufferModule, Vec<SourceDiagnostic>> {
    lower_source_buffer_for_target(parsed, typed, entry, None)
}

/// [`lower_source_buffer`] for the builder's recorded target (card
/// `W48-QR-SCALAR-TYPES-R5`): the lowering's constant tier folds conversions
/// and value statements to `isize`/`usize` only with a width, so a loop bound
/// of either type needs the target here as well as in the checker.
pub fn lower_source_buffer_for_target(
    parsed: &SourceProgram,
    _typed: &TypedSourceProgram,
    entry: &str,
    target: Option<&qcore_core::scalar_types::RecordedTarget>,
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
            // Card `LOOP-V1`: a `for` outside the entry is refused for the same
            // reason and at the loop's own code. Without this it would be
            // silently unmodelled — this route lowers the entry only, and a
            // construct nothing checks is exactly what invariant 1 forbids.
            // Card `AGGREGATES-V1`: a record construct outside the entry is
            // refused for exactly the reason a `for` outside it is — this route
            // lowers the entry only, and a construct nothing checks is what
            // invariant 1 forbids.
            if let Some(span) = crate::source_record::block_record_span(body, tokens) {
                diagnostics.push(diagnostic(
                    "Q051",
                    span,
                    format!(
                        "`source.record.v1` models the entry only on this route; `{}` is not the \
                         entry `{entry}`, and a record nothing checks is not one this revision \
                         admits",
                        other.value.name
                    ),
                ));
            }
            if let Some(span) = crate::source_loop::block_loop_span(body) {
                diagnostics.push(loop_shape_diagnostic(
                    span,
                    format!(
                        "`source.loop.v1` is admitted in the entry `{entry}` only on this route; \
                         `{}` is not it, and a loop nothing checks is not one this revision \
                         admits",
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

    // Card `AGGREGATES-V1`: the module's record type declarations, read before
    // any statement. A malformed declaration is refused here; whether a
    // well-formed one is a type the *model* admits is
    // `qcore_core::source_record::admit_type`'s and is decided in the checker,
    // so the shape rule and the formation rule each live in one place.
    // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: `f32`/`f64` are field types here.
    let records = match crate::source_record::record_declarations_admitting_floats(parsed) {
        Ok(records) => records,
        Err((span, message)) => {
            diagnostics.push(diagnostic("Q051", span, message));
            return Err(diagnostics);
        }
    };

    let (declared_effects, float_effects) =
        match declared_effect_row(&declaration.value.tokens, scope_span) {
            Ok(row) => row,
            Err(error) => {
                diagnostics.push(error);
                (EffectRow::default(), FloatEffects::default())
            }
        };

    let Some(body) = &declaration.value.body else {
        diagnostics.push(diagnostic("Q051", scope_span, "this entry has no body"));
        return Err(diagnostics);
    };

    let mut events = Vec::new();
    let mut event_spans = Vec::new();
    let mut constants = LoweringConstants::default();
    // Card `W48-QR-SCALAR-TYPES-R3`: the declared field types, so a
    // construction reads each field's value at its declared type.
    constants.declare_records(&records);
    // Card `W48-QR-SCALAR-TYPES-R5`: the width `isize`/`usize` take in the
    // constant tier.
    constants.target = target.cloned();

    lower_statements(
        parsed,
        tokens,
        &body.statements,
        &mut constants,
        &mut events,
        Some(&mut event_spans),
        &mut diagnostics,
    );

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(LoweredBufferModule {
        entry: entry.to_owned(),
        program: BufferProgram {
            entry: entry.to_owned(),
            declared_effects,
            records,
            events,
            float_effects,
        },
        event_spans,
        scope_span,
    })
}

/// The check-time constants the lowering itself can see.
///
/// The loop's **bound** is decided here, before any byte exists, so its
/// decidable tier is deliberately narrower than the index tier
/// `source.buffer.v1` uses: a literal, a name bound by a literal `let`, or a
/// name bound by `extent(NAME)`. A bound read out of a *byte* is refused by
/// name — the value is known to the checker but not to the lowering, and
/// widening the tier would mean deciding the count in a second place
/// (`LOOP-BOUND-FROM-BYTE`).
///
/// Published since card `LOOPS-ON-COMPOSED-ROUTES` so the composed
/// string/buffer route, which now embeds a `for`, resolves its bound through
/// the same environment and the same reader this route does.
#[derive(Clone, Debug, Default)]
pub struct LoweringConstants {
    scalars: BTreeMap<String, i32>,
    extents: BTreeMap<String, u32>,
    /// The module's declared record types (card `W48-QR-SCALAR-TYPES-R3`):
    /// read-only, so a construction reads each field value at its declared
    /// type. Contributes no constant.
    records: BTreeMap<String, RecordType>,
    /// Card `W48-QR-SCALAR-TYPES-R5`: the tier's values of every §G02t type
    /// other than `i32`, each with its number — a conversion over a value of
    /// the tier, folded by the shared decision (`scalar_values::Decider`),
    /// exactly as the checker folds it. A name is in at most one of `scalars`
    /// and this map.
    typed: BTreeMap<String, (ScalarType, i128)>,
    /// Derived names of `isize`/`usize` the tier could not fold for want of a
    /// recorded target (card `W48-QR-SCALAR-TYPES-R5`), so a bound spelled
    /// through one is refused for the width rather than as unknown.
    widthless: BTreeMap<String, ScalarType>,
    /// The builder's recorded target, which alone gives `isize`/`usize` a
    /// width in the tier (card `W48-QR-SCALAR-TYPES-R5`).
    pub target: Option<qcore_core::scalar_types::RecordedTarget>,
}

impl LoweringConstants {
    /// The type and number of a name of the tier, if it has one (card
    /// `W48-QR-SCALAR-TYPES-R5`).
    #[must_use]
    pub fn known(&self, name: &str) -> Option<(ScalarType, i128)> {
        self.scalars
            .get(name)
            .map(|value| (ScalarType::I32, i128::from(*value)))
            .or_else(|| self.typed.get(name).copied())
    }

    /// What the tier knows of a bound name (card `W48-QR-SCALAR-TYPES-R5`).
    #[must_use]
    pub fn bound_value(&self, name: &str) -> Option<crate::source_loop::BoundValue> {
        use crate::source_loop::BoundValue;
        if let Some(value) = self.scalars.get(name) {
            return Some(BoundValue::I32(*value));
        }
        if let Some((ty, value)) = self.typed.get(name) {
            return Some(BoundValue::Derived {
                ty: *ty,
                value: *value,
            });
        }
        self.widthless
            .get(name)
            .map(|ty| BoundValue::Widthless { ty: *ty })
    }

    /// Bind `binding` to a derived value of `ty` (card
    /// `W48-QR-SCALAR-TYPES-R5`). An `i32` too goes to the derived map, so a
    /// loop over it names its bound; `scalars` keeps exactly the tier the
    /// card found.
    fn bind_known(&mut self, binding: &str, ty: ScalarType, value: i128) {
        self.typed.insert(binding.to_owned(), (ty, value));
    }

    /// Record a derived name the tier could not fold for want of a width.
    fn bind_widthless(&mut self, binding: &str, ty: ScalarType) {
        if ty.is_target_width() && self.target.is_none() {
            self.widthless.insert(binding.to_owned(), ty);
        }
    }

    /// Fold one §G02t operation over the tier with the shared decision (card
    /// `W48-QR-SCALAR-TYPES-R5`): the result's type and number when every
    /// operand is known and the operation neither faults nor is refused.
    /// Anything else contributes no bound — the checker decides (and, for a
    /// fault, refuses) the statement itself, so the tier never holds a value
    /// the checker would not.
    fn fold(&self, operation: qcore_core::scalar_values::Operation) -> Option<(ScalarType, i128)> {
        let decision = qcore_core::scalar_values::Decider::default()
            .decide(operation, false, self.target.as_ref())
            .ok()?;
        Some((decision.ty, decision.known?))
    }

    /// A known operand for the shared decision.
    fn shared_operand(
        &self,
        operand: &BufferOperand,
    ) -> Option<qcore_core::scalar_values::Operand> {
        Some(match operand {
            BufferOperand::Literal(value) => qcore_core::scalar_values::Operand::Typed {
                ty: ScalarType::I32,
                known: Some(i128::from(*value)),
            },
            BufferOperand::Scalar(name) => {
                let (ty, value) = self.known(name)?;
                qcore_core::scalar_values::Operand::Typed {
                    ty,
                    known: Some(value),
                }
            }
        })
    }

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Install the module's record declarations (card `W48-QR-SCALAR-TYPES-R3`).
    pub fn declare_records(&mut self, records: &[RecordType]) {
        self.records = records
            .iter()
            .map(|record| (record.name.clone(), record.clone()))
            .collect();
    }

    /// The declared type of `type_name.field`, if the module declares one.
    #[must_use]
    pub fn field_type(&self, type_name: &str, field: &str) -> Option<FieldType> {
        self.records.get(type_name)?.field(field)
    }

    /// Record a constant a **composing route's own** `let` bound — the
    /// composed string route's `let n = 4;` is that route's statement, not
    /// one of this route's events, and a loop bound may name it.
    pub fn observe_scalar(&mut self, binding: &str, value: i32) {
        self.scalars.insert(binding.to_owned(), value);
    }

    /// Record what one lowered event binds.
    pub fn observe(&mut self, event: &BufferEvent) {
        match event {
            BufferEvent::Bind { binding, value } => {
                self.scalars.insert(binding.clone(), *value);
            }
            BufferEvent::Acquire { name, extent } => {
                self.extents.insert(name.clone(), *extent);
            }
            BufferEvent::Extent { binding, name } => {
                if let Some(extent) = self.extents.get(name).copied() {
                    self.scalars.insert(binding.clone(), extent as i32);
                }
            }
            // A byte read binds a value the *checker* computes; the lowering
            // does not, so it contributes no bound. Card `AGGREGATES-V1`: a
            // **field read** is in exactly that position and for exactly that
            // reason — the value is a field of a record the checker constructs,
            // so a loop bound read out of a field is refused by name
            // (`AGGREGATE-BOUND-FROM-FIELD`, the `LOOP-BOUND-FROM-BYTE` shape)
            // rather than decided in a second place.
            BufferEvent::Load { .. }
            | BufferEvent::Construct { .. }
            | BufferEvent::Field { .. }
            | BufferEvent::Store { .. }
            | BufferEvent::Release { .. }
            // Card `AGGREGATE-OWNED-FIELD`: a load or an extent read through
            // a field binds a value the checker computes out of a moved-in
            // buffer, which this lowering does not follow — so, like a byte
            // read, it contributes no bound (`AGGREGATE-BOUND-FROM-FIELD`'s
            // shape at a field path).
            | BufferEvent::FieldStore { .. }
            | BufferEvent::FieldLoad { .. }
            | BufferEvent::FieldExtent { .. }
            | BufferEvent::FieldRelease { .. }
            // Card `W48-QR-USABLE-RECORDS`: a field write binds nothing, and
            // a later read of the field it wrote is a field read — outside
            // this tier for the reason above.
            | BufferEvent::FieldWrite { .. }
            // Card `SLICE-PERMISSIONS-WRITE-LEND`: a lend binds a loan, not a
            // constant, so it contributes no bound.
            | BufferEvent::Lend { .. }
            | BufferEvent::Line(_)
            | BufferEvent::Repeat { .. }
            | BufferEvent::Return => {}
            // Card `OWNED-FIELD-ROUTE-REACH`: **a copy bind contributes no
            // bound**, and that is a decision rather than an omission. The
            // lowering's tier is a literal, a literal `let` and an `extent`
            // binding, and it is deliberately narrower than the checker's, so
            // that a loop bound and a lend footprint are decided in one place
            // (`LOOP-BOUND-FROM-BYTE`, `WRITE-LEND-BOUND-FROM-BYTE`).
            // Following a copy here would widen that tier by a second rule
            // rather than by that card's own; a bound spelled through a copy
            // is refused by name, in the same direction as a bound read out of
            // a byte (`AGGREGATE-BOUND-FROM-FIELD`'s shape at a copy).
            | BufferEvent::CopyBind { .. } => {}
            // Card `W48-QR-SCALAR-TYPES-R3` left a conversion out of this tier
            // (`W48-QR-TYPES-LOOP-BOUNDS`). Card `W48-QR-SCALAR-TYPES-R5`
            // admits it, and **not** by a second rule: the conversion of a
            // value of the tier is folded by the shared decision the checker
            // decides the same statement with (`scalar_values::Decider`), so
            // the tier gains exactly the values of every other §G02t type
            // that a literal, a literal `let` or an `extent` determine. A
            // conversion of a byte, a field or a copy is still outside it —
            // its operand is — and a bound spelled through one is refused by
            // name, as before.
            // Card `W48-QR-SCALAR-TYPES-R5`: a value statement over values of
            // the tier is folded by the shared decision, as a conversion is.
            BufferEvent::Compute { binding, operation } => {
                use qcore_core::std_buffer::{ValueOperand, ValueOperation};
                use qcore_core::scalar_values::{Operand, Operation};
                let operand = |operand: &ValueOperand| -> Option<Operand> {
                    Some(match operand {
                        ValueOperand::Name(name) => {
                            let (ty, value) = self.known(name)?;
                            Operand::Typed {
                                ty,
                                known: Some(value),
                            }
                        }
                        ValueOperand::Integer(value) => Operand::Integer(*value),
                        ValueOperand::Bool(value) => Operand::Bool(*value),
                        ValueOperand::Float(literal) => Operand::Float(*literal),
                    })
                };
                let shared = match operation {
                    ValueOperation::Binary(op, a, b) => operand(a)
                        .zip(operand(b))
                        .map(|(a, b)| Operation::Binary(*op, a, b)),
                    ValueOperation::Compare(cmp, a, b) => operand(a)
                        .zip(operand(b))
                        .map(|(a, b)| Operation::Compare(*cmp, a, b)),
                    ValueOperation::Negate(a) => operand(a).map(Operation::Negate),
                    ValueOperation::Convert(a, to) => {
                        operand(a).map(|a| Operation::Convert(a, *to))
                    }
                };
                match shared.and_then(|shared| self.fold(shared)) {
                    Some((ty, value)) => self.bind_known(binding, ty, value),
                    None => {
                        let widthless = operation
                            .names()
                            .into_iter()
                            .find_map(|name| self.widthless.get(name).copied());
                        if let Some(ty) = widthless {
                            self.bind_widthless(binding, ty);
                        }
                    }
                }
            }
            BufferEvent::Convert {
                binding,
                operand,
                to,
            } => {
                match self.shared_operand(operand).and_then(|operand| {
                    self.fold(qcore_core::scalar_values::Operation::Convert(operand, *to))
                }) {
                    Some((ty, value)) => self.bind_known(binding, ty, value),
                    None => self.bind_widthless(binding, *to),
                }
            }
        }
    }
}

/// Lower one statement sequence — an entry body, or a loop body.
///
/// `event_spans` is `Some` only for the entry's own sequence: a loop body's
/// events have no top-level index, and their refusals are attributed to the
/// `for` statement (`LOOP-BODY-ATTRIBUTION`).
#[allow(clippy::too_many_arguments)]
fn lower_statements(
    parsed: &SourceProgram,
    tokens: &[Token],
    statements: &[crate::frontend::Located<crate::frontend::StatementKind>],
    constants: &mut LoweringConstants,
    events: &mut Vec<BufferEvent>,
    mut event_spans: Option<&mut Vec<Span>>,
    diagnostics: &mut Vec<SourceDiagnostic>,
) {
    // Card `LOOP-UNROLL-BUDGET`: the unrolling attribute is **statement
    // adjacent** — it licenses the `for` immediately after it, and nothing
    // else. The pairing is carried here, in the reader that walks the
    // statements in order, so "immediately preceding" is a property of this
    // walk rather than of a span comparison.
    let mut pending: Option<Span> = None;
    let mut budget = qcore_core::source_loop::LoopBudget::Default;
    for statement in statements {
        let span = statement.span;
        if matches!(
            statement.value,
            crate::frontend::StatementKind::LoopAttribute
        ) {
            if pending.is_some() {
                diagnostics.push(loop_shape_diagnostic(
                    span,
                    "an unrolling attribute licenses the one `for` statement that follows it, and \
                     this one follows another attribute; `source.loop.v1` refuses a second clause \
                     list rather than deciding which of two conditions governs the loop",
                ));
                continue;
            }
            let window = crate::source_loop::attribute_window(tokens, span);
            match crate::source_loop::read_loop_attribute(&window, span) {
                Ok(read) => budget = qcore_core::source_loop::LoopBudget::Licensed(read),
                Err(refusal) => {
                    // A clause list this reader cannot read licenses nothing —
                    // and holding the loop to the *default* budget on top of
                    // that would print two refusals for one mistake, which is
                    // card `LOOP-V1` §9.9's rule applied to the attribute. The
                    // budget decision is suppressed; every other rule still
                    // applies to the loop.
                    diagnostics.push(refusal);
                    budget = qcore_core::source_loop::LoopBudget::Unreadable;
                }
            }
            pending = Some(span);
            continue;
        }
        // Card `LOOP-V1`: the one nested block this route models.
        if let crate::frontend::StatementKind::For { binding, body } = &statement.value {
            let licensed = std::mem::take(&mut budget);
            pending = None;
            if let Some(event) = lower_loop(
                parsed,
                tokens,
                binding,
                body,
                span,
                constants,
                licensed,
                diagnostics,
            ) {
                events.push(event);
                if let Some(spans) = event_spans.as_deref_mut() {
                    spans.push(span);
                }
            }
            continue;
        }
        if let Some(attribute) = pending.take() {
            budget = qcore_core::source_loop::LoopBudget::Default;
            diagnostics.push(loop_shape_diagnostic(
                attribute,
                "an unrolling attribute licenses a `for` statement and this one precedes a \
                 statement that is not one; `source.loop.v1` refuses an attribute that licenses \
                 nothing rather than letting it apply to whatever comes next",
            ));
        }
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
            "let" => lower_let(&window, span, constants, diagnostics),
            DROP_KEYWORD => {
                let Some(name) = word(&window, 1) else {
                    diagnostics.push(diagnostic("Q051", span, "`drop NAME;` names one buffer"));
                    continue;
                };
                // Card `AGGREGATE-OWNED-FIELD`: `drop RECORD.FIELD;`, read by
                // the record reader.
                if let Some(event) = lower_field_path(&window, span, diagnostics) {
                    if let Some(event) = event {
                        constants.observe(&event);
                        events.push(event);
                        if let Some(spans) = event_spans.as_deref_mut() {
                            spans.push(span);
                        }
                    }
                    continue;
                }
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
                // Card `LOOP-V1`: a `return` inside a loop body is the loop's
                // refusal, not this route's. An early exit needs a lowering with
                // more than one exit position — the same missing capability
                // `IF-ARM-RETURN` names — and saying so at `Q097` points the
                // reader at the construct they have to change.
                if event_spans.is_none() {
                    diagnostics.push(loop_shape_diagnostic(
                        span,
                        "a `return` inside a `for` body is an early exit, and `source.loop.v1` \
                         models none: a counted loop runs its body a known number of times, and a \
                         lowering with one exit position has nowhere to put a second \
                         (`LOOP-BODY-RETURN`)",
                    ));
                    None
                } else if ends_with_semicolon(&window, 1) {
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
            // Card `LOOP-V1`: both are already reserved words with their own
            // `StatementKind`, so they reach this reader as an unmodelled head.
            // They are refused at the **loop's** code, by name, because that is
            // the construct whose absence they name.
            "break" | "continue" => {
                diagnostics.push(loop_shape_diagnostic(
                    span,
                    format!(
                        "`{head}` is an early exit from a loop and `source.loop.v1` models none; \
                         the per-exit cleanup discharge it needs is the capability \
                         `IF-ARM-RETURN` names and no lowering on this route has (`LOOP-BREAK`)"
                    ),
                ));
                None
            }
            other => {
                // A byte write has a *name* at its head, so it is recognised by
                // shape rather than by keyword — which is exactly why it needs
                // no reserved word. Card `AGGREGATE-OWNED-FIELD`: so does a
                // write through a field, told by the `.` after the name.
                if symbol(&window, 1, Symbol::LeftBracket) {
                    lower_store(&window, span, diagnostics)
                } else if let Some(event) = lower_field_path(&window, span, diagnostics) {
                    event
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
            constants.observe(&event);
            events.push(event);
            if let Some(spans) = event_spans.as_deref_mut() {
                spans.push(span);
            }
        }
    }
    // A clause list at the end of a sequence licenses nothing at all.
    if let Some(attribute) = pending {
        diagnostics.push(loop_shape_diagnostic(
            attribute,
            "an unrolling attribute licenses the `for` statement that follows it and nothing \
             follows this one; `source.loop.v1` refuses a dangling clause list rather than \
             discarding it",
        ));
    }
}

/// Lower one `for NAME in 0..BOUND { … }` statement for a route that embeds
/// this route's statements (card `LOOPS-ON-COMPOSED-ROUTES`: the composed
/// string/buffer route), through the **same** reader the entry's own
/// sequence uses — so a loop body's statements on that route are this
/// route's statements, read by this route's reader, bounded by this route's
/// tier, exactly as `LOOP-V1` stated it for the uncomposed route.
///
/// `None` for a statement that is not a `for`. The diagnostics it pushes carry
/// this route's `Q051` for a body statement outside the buffer grammar and
/// the loop's own `Q097`–`Q099` for the loop's own refusals; a composing route
/// re-attributes the former to its own band (§L21 forbids a second pass
/// emitting `Q051`) and keeps the latter, which are the loop pass's.
pub fn lower_for_statement(
    parsed: &SourceProgram,
    statement: &crate::frontend::Located<crate::frontend::StatementKind>,
    constants: &mut LoweringConstants,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<BufferEvent> {
    let crate::frontend::StatementKind::For { binding, body } = &statement.value else {
        return None;
    };
    lower_loop(
        parsed,
        parsed.lexical_tokens(),
        binding,
        body,
        statement.span,
        constants,
        qcore_core::source_loop::LoopBudget::Default,
        diagnostics,
    )
}

fn loop_shape_diagnostic(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code: "Q097",
        pass: SemanticPass::Loop,
        span,
        message: message.into(),
    }
}

/// Lower one `for NAME in 0..BOUND { … }` into a [`BufferEvent::Repeat`].
///
/// The header is [`crate::source_loop`]'s and the body is **this route's own**
/// statements, read by the reader above — so "a loop body's statements are the
/// byte-buffer route's statements" is a statement about code rather than about
/// two implementations that agree.
#[allow(clippy::too_many_arguments)]
fn lower_loop(
    parsed: &SourceProgram,
    tokens: &[Token],
    binding: &str,
    body: &SourceBlock,
    span: Span,
    constants: &mut LoweringConstants,
    budget: qcore_core::source_loop::LoopBudget,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<BufferEvent> {
    let header_window = crate::source_loop::header_window(tokens, span);
    let header = match crate::source_loop::read_loop_header(
        &header_window,
        span,
        |name| constants.bound_value(name),
        constants.target.as_ref(),
        budget,
    ) {
        Ok(header) => header,
        Err(refusal) => {
            diagnostics.push(refusal);
            return None;
        }
    };
    if header.variable != binding {
        // The parser and this reader disagree about which name the header
        // binds, which no well-formed header can produce. Refused rather than
        // resolved in favour of either.
        diagnostics.push(loop_shape_diagnostic(
            span,
            "this `for` header does not bind the name the parser read from it",
        ));
        return None;
    }

    // The body is lowered in a **scoped copy** of the constant environment: the
    // index is deliberately *not* in it, so a nested loop's bound can never be
    // a varying value, and names the body binds do not escape it.
    let mut inner = constants.clone();
    let mut body_events = Vec::new();
    lower_statements(
        parsed,
        tokens,
        &body.statements,
        &mut inner,
        &mut body_events,
        None,
        diagnostics,
    );

    Some(BufferEvent::Repeat {
        variable: header.variable,
        count: header.count,
        index: header.index,
        body: body_events,
    })
}
