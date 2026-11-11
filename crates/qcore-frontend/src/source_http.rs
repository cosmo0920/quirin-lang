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

//! Exact source lowering for `source.http.v1` — the `std.http` bundled module
//! (card `HTTP-QUIRIN-V1`).
//!
//! ```text
//! module    ::= import* entry
//! import    ::= "import" ("std.http" | "std.socket" | "std.string"
//!                        | "std.fmt" | "std.io" | "std.logger") ";"
//! entry     ::= "fn" NAME "(" ")" "->" "unit"
//!               ["effects" "{" EFFECT {"," EFFECT} "}"] "{" stmt* "}"
//! stmt      ::= "let" NAME "=" "request" STRING STRING STRING ";"
//!             | "let" NAME "=" "response" STRING INTEGER STRING ";"
//!             | "header" NAME STRING VALUE ";"
//!             | "content" NAME VALUE ";"
//!             | "let" NAME "=" STRING ";"            -- a bound string view
//!             | "let" NAME "=" "concat" "(" VALUE {"," VALUE} ")" ";"
//!             | "let" NAME "=" "len" "(" NAME ")" ";"
//!             | "send" NAME NAME ";"                 -- a message or a view
//!             | "recv" NAME NAME ";"                 -- verified against one
//!             | <every `source.socket.v1` statement, unchanged>
//! VALUE     ::= STRING                               -- this module's syntax
//!             | NAME                                 -- text this entry bound
//!             | ("dec" | "hex") "(" NAME ")"         -- a bound integer's text
//! ```
//!
//! # Computed components (card `HTTP-V1-COMPUTED-LENGTH`)
//!
//! The last four productions are this card's, and they add **no reserved word**.
//! `concat` and `len` are `source.string.v1`'s **contextual** clause words, read
//! through that route's own [`is_string_clause_word`] / [`is_length_query`]
//! predicates rather than a second copy, so the two routes cannot drift about
//! what `concat(` means; `dec` and `hex` have been reserved since `STD-MODULES`.
//!
//! They also change **no route selection**: [`has_source_http`] still reads
//! exactly `request`/`response`/`header`/`content`, so a module that composes
//! text without declaring a message reaches `source.string.v1` byte for byte as
//! before, and the two constructs are admitted here only because the module is
//! already this route's for another reason.
//!
//! # Four reserved words, and the census that chose them
//!
//! `request`, `response`, `header`, `content`. Every one is a clause word at a
//! statement head or in a `let` initialiser, never a reference to a binding.
//!
//! The constraint-14 census ran over **416** `.qcore`/`.qc` fixtures and
//! **43,239** Rust string literals, at whole-word granularity with `//` comments
//! stripped from the QCore side (`docs/reports/http-quirin-v1/artifacts/`).
//!
//! | Candidate | QCore source positions | Verdict |
//! |---|---|---|
//! | `request`, `response`, `header`, `http`, `method`, `version`, `status`, `reason` | **0** | clean |
//! | `content` | **0** | **chosen** for the content statement |
//! | `body` | **8** | **rejected** — it is a *binding name* in two shipped fixtures (`examples/strings/status_line.qr`, `compiler/tests/fixtures/std_string/text.qr`), referenced by `print body;`, `body \|> print;` and `len(body)`. Reserving it would refuse programs that ship today |
//! | `message` | **2** | **rejected** — a contract `extern fn` parameter, referenced inside `ensures normal resource_borrowed(message);` |
//! | `payload` | **49** | **rejected** |
//! | `field`, `head` | 5, 6 | not needed |
//!
//! `body` is the finding worth recording: the obvious word for a message body is
//! the one word this repository already uses for one, in source position, in
//! files this card must not break. RFC 9110 §6.4 renamed that concept
//! **content** — "the content of a message" — so the word the census admits is
//! also the word the specification now uses. The same measured judgement
//! `STD-BUFFER` made about `buffer` and `QUIRIN-SOCKET-V1` made about `stream`.
//!
//! `send`, `recv`, `let`, `close` and the `source.std-io.v1` line words are
//! **reused rather than respelled**: a message goes on a wire through the socket
//! vocabulary that already exists.
//!
//! # What this lowering does not establish
//!
//! It decides no conformance question: it produces an [`HttpProgram`] and stops.
//! Every refusal a reader sees at `Q091`–`Q096` is [`qcore_core::std_http`]'s,
//! attributed to the statement whose event it named, and every one at
//! `Q095` is the *unedited* `source.socket.v1` checker's inside that.

use std::collections::BTreeMap;

use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, TypedSourceProgram,
};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use crate::source_socket::SocketLowering;
use crate::source_std_string::{CONCAT_KEYWORD, is_length_query, is_string_clause_word};
use qcore_core::std_http::{HttpPiece, HttpProgram, HttpStart, HttpStatement};
use qcore_core::std_io::Rendering;
use qcore_core::std_socket::SocketStatement;
use qcore_core::std_string::decode_string_literal;
use qcore_core::typed_errors::EffectRow;

/// The request-declaration keyword.
pub const REQUEST_KEYWORD: &str = "request";
/// The response-declaration keyword.
pub const RESPONSE_KEYWORD: &str = "response";
/// The header-field keyword.
pub const HEADER_KEYWORD: &str = "header";
/// The message-content keyword. RFC 9110 §6.4's own word; `body` is a measured
/// collision (module header).
pub const CONTENT_KEYWORD: &str = "content";

/// The four words this route reserves, for the constraint-14 record and for the
/// test that asserts each one is load-bearing.
pub const HTTP_KEYWORDS: [&str; 4] = [
    REQUEST_KEYWORD,
    RESPONSE_KEYWORD,
    HEADER_KEYWORD,
    CONTENT_KEYWORD,
];

/// A complete lowering of one entry. The core checker still has to validate the
/// program; this type is not evidence on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredHttpModule {
    pub entry: String,
    pub program: HttpProgram,
    /// Source span of the statement at the same index. Every statement has one.
    pub event_spans: Vec<Span>,
    /// Fallback span for a refusal that names no statement.
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

fn literal_text<'a>(window: &'a [&Token], cursor: usize) -> Option<&'a str> {
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::StringLiteral(text)) => Some(text.as_str()),
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

fn integer(window: &[&Token], cursor: usize) -> Option<i32> {
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Integer(text)) => text.parse::<i32>().ok(),
        _ => None,
    }
}

/// Which of this card's constructs a statement uses, if any.
///
/// The **one** place that decides whether a statement is this route's, so route
/// selection, the import gate and the lowering below cannot disagree about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HttpUse {
    /// `let NAME = request …;` / `let NAME = response …;`
    Declare,
    /// `header NAME "…" "…";`
    Header,
    /// `content NAME "…";`
    Content,
}

/// Whether this `let` binds a value this route **computes**: a `concat(…)`
/// composition or a `len(…)` (card `HTTP-V1-COMPUTED-LENGTH`).
///
/// Deliberately **not** part of [`http_use`]: route selection is unchanged by
/// this card. `concat` and `len` are `source.string.v1`'s vocabulary, so a
/// module that composes text without declaring a message still reaches the
/// string route exactly as it did — the two constructs are admitted *here* only
/// because the module is already this route's for another reason.
fn is_computed_binding(window: &[&Token]) -> bool {
    if word(window, 0) != Some("let") || !symbol(window, 2, Symbol::Equal) {
        return false;
    }
    let after = window.get(4).map(|token| &token.kind);
    match word(window, 3) {
        Some(CONCAT_KEYWORD) => is_string_clause_word(CONCAT_KEYWORD, after),
        Some(head) => is_length_query(head, after),
        None => false,
    }
}

fn http_use(window: &[&Token]) -> Option<HttpUse> {
    match word(window, 0) {
        Some("let") => {
            if !symbol(window, 2, Symbol::Equal) {
                return None;
            }
            match word(window, 3) {
                Some(REQUEST_KEYWORD | RESPONSE_KEYWORD) => Some(HttpUse::Declare),
                _ => None,
            }
        }
        Some(HEADER_KEYWORD) => Some(HttpUse::Header),
        Some(CONTENT_KEYWORD) => Some(HttpUse::Content),
        _ => None,
    }
}

/// Every use of this card's vocabulary in the program, with the span of the
/// first occurrence of each kind.
#[must_use]
pub fn http_uses(program: &SourceProgram) -> BTreeMap<HttpUse, Span> {
    let tokens = program.lexical_tokens();
    let mut found: BTreeMap<HttpUse, Span> = BTreeMap::new();
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

fn scan(block: &SourceBlock, tokens: &[Token], found: &mut BTreeMap<HttpUse, Span>) {
    for statement in &block.statements {
        let window = window(tokens, statement.span);
        if let Some(kind) = http_use(&window) {
            found.entry(kind).or_insert(statement.span);
        }
        for nested in statement.value.nested_blocks() {
            scan(nested, tokens, found);
        }
    }
}

/// Whether this module is a `source.http.v1` module.
#[must_use]
pub fn has_source_http(program: &SourceProgram) -> bool {
    !http_uses(program).is_empty()
}

/// The span of the first statement needing `#import std.http;`.
#[must_use]
pub fn http_span(program: &SourceProgram) -> Option<Span> {
    http_uses(program).values().next().copied()
}

/// The span of the first `let NAME = "…";` in a module this route claims, which
/// is what needs `#import std.string;` **in addition to** `#import std.http;`.
///
/// A quoted token at a method, a target, a field name, a field value or a
/// content position is deliberately **not** here: those are this module's own
/// syntax, and nothing in the language can bind, slice, measure or render them.
/// A `let` binding is different — it produces a `string` value a later statement
/// names — so it takes the string module's own gate, exactly as a rendered
/// literal does.
#[must_use]
pub fn text_binding_span(program: &SourceProgram) -> Option<Span> {
    if !has_source_http(program) {
        return None;
    }
    let tokens = program.lexical_tokens();
    fn walk(block: &SourceBlock, tokens: &[Token]) -> Option<Span> {
        for statement in &block.statements {
            let window = window(tokens, statement.span);
            if word(&window, 0) == Some("let")
                && symbol(&window, 2, Symbol::Equal)
                && literal_text(&window, 3).is_some()
            {
                return Some(statement.span);
            }
            for nested in statement.value.nested_blocks() {
                if let Some(span) = walk(nested, tokens) {
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
        if let Some(span) = declaration
            .value
            .body
            .as_ref()
            .and_then(|b| walk(b, tokens))
        {
            return Some(span);
        }
    }
    None
}

fn diagnostic(code: &'static str, span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code,
        pass: SemanticPass::StandardHttp,
        span,
        message: message.into(),
    }
}

/// Decode a quoted token at `cursor`, or report why it is not a payload.
///
/// The decoder is `source.string.v1`'s own, so the escapes admitted, the UTF-8
/// well-formedness required and the length bounded are that route's rules. `\r`
/// is among those escapes — but no message component may carry one, because a
/// `CR` in a component would forge the framing this route constructs. That rule
/// is the model's ([`qcore_core::std_http::admit_field`]) and is not restated
/// here.
fn text(
    window: &[&Token],
    span: Span,
    cursor: usize,
    what: &str,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<Vec<u8>> {
    let Some(raw) = literal_text(window, cursor) else {
        diagnostics.push(diagnostic(
            "Q090",
            span,
            format!("`source.http.v1` spells a {what} as one quoted token"),
        ));
        return None;
    };
    match decode_string_literal(raw) {
        Ok(bytes) => Some(bytes),
        Err(refusal) => {
            diagnostics.push(diagnostic("Q090", span, refusal.to_string()));
            None
        }
    }
}

/// The same, as a `String`. Every component of a message is checked to be ASCII
/// by the model, so the UTF-8 the decoder guarantees is enough to build one.
fn text_string(
    window: &[&Token],
    span: Span,
    cursor: usize,
    what: &str,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<String> {
    let bytes = text(window, span, cursor, what, diagnostics)?;
    match String::from_utf8(bytes) {
        Ok(text) => Some(text),
        Err(_) => {
            diagnostics.push(diagnostic(
                "Q090",
                span,
                format!("this {what} does not decode to text"),
            ));
            None
        }
    }
}

/// Read one **component value** at `cursor`, returning it and the cursor just
/// past it (card `HTTP-V1-COMPUTED-LENGTH`).
///
/// Three forms, and every one of them is check-time-known — which is why a
/// computed component changes nothing about what the RFC 9112 gates decide:
///
/// * `"…"` — a quoted token, this module's own syntax, constructing no `string`
///   value (constraint 15f, `STRING_SLICE_V1.md` §2). The only form card
///   `HTTP-QUIRIN-V1` admitted;
/// * `NAME` — text this entry bound with `let NAME = "…";` or
///   `let NAME = concat(…);`;
/// * `dec(NAME)` / `hex(NAME)` — the text of a bound integer, the one
///   scalar-to-text direction the language has, and the piece that makes a
///   computed `Content-Length` expressible.
///
/// They are the **same three** a `source.string.v1` `concat` piece takes, minus
/// that route's inline slice: slicing beside HTTP stays the named follow-on
/// `HTTP-V1-STRING-COMPOSE`, so a spelling this route admits is a spelling that
/// route admits too.
fn piece(
    window: &[&Token],
    span: Span,
    cursor: usize,
    what: &str,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<(HttpPiece, usize)> {
    if let Some(raw) = literal_text(window, cursor) {
        return match decode_string_literal(raw) {
            Ok(bytes) => Some((HttpPiece::Literal(bytes), cursor + 1)),
            Err(refusal) => {
                diagnostics.push(diagnostic("Q090", span, refusal.to_string()));
                None
            }
        };
    }
    let Some(head) = word(window, cursor) else {
        diagnostics.push(diagnostic(
            "Q090",
            span,
            format!(
                "`source.http.v1` spells a {what} as one quoted token, a name this entry bound to \
                 text, or `dec(NAME)`/`hex(NAME)` for the text of a bound integer"
            ),
        ));
        return None;
    };
    if let Some(rendering) = Rendering::parse(head) {
        if !symbol(window, cursor + 1, Symbol::LeftParen) {
            diagnostics.push(diagnostic(
                "Q090",
                span,
                format!(
                    "`{head}` renders a bound integer as text and is spelled `{head}(NAME)`; a \
                     rendering selector is not a {what} on its own"
                ),
            ));
            return None;
        }
        let Some(name) = word(window, cursor + 2) else {
            diagnostics.push(diagnostic(
                "Q090",
                span,
                "`dec(NAME)` and `hex(NAME)` name the bound integer whose text they compose",
            ));
            return None;
        };
        if !symbol(window, cursor + 3, Symbol::RightParen) {
            diagnostics.push(diagnostic(
                "Q090",
                span,
                "this rendering piece is not closed",
            ));
            return None;
        }
        return Some((
            HttpPiece::Scalar {
                rendering,
                name: name.to_owned(),
            },
            cursor + 4,
        ));
    }
    Some((HttpPiece::Name(head.to_owned()), cursor + 1))
}

/// The entry's declared `effects { … }` row. An absent suffix is the empty row,
/// which admits nothing, so the opt-in stays visible.
fn declared_effect_row(tokens: &[TokenKind], span: Span) -> Result<EffectRow, SourceDiagnostic> {
    use qcore_core::typed_errors::Effect;
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
            "Q090",
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
                            "Q090",
                            span,
                            format!(
                                "`{other}` is not an effect `source.http.v1` models; use io, \
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
            _ => return Err(diagnostic("Q090", span, "this `effects` row is not closed")),
        }
    }
    Ok(EffectRow::new(effects))
}

/// Lower one `let` statement. This route's three shapes, then the transport's.
fn lower_let(
    window: &[&Token],
    span: Span,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<HttpStatement> {
    let binding = word(window, 1)?;
    if !symbol(window, 2, Symbol::Equal) {
        return None;
    }
    match word(window, 3) {
        // `let NAME = request "METHOD" "/target" "HTTP/1.1";`
        Some(REQUEST_KEYWORD) => {
            let method = text_string(window, span, 4, "method", diagnostics)?;
            let target = text_string(window, span, 5, "request target", diagnostics)?;
            let version = text_string(window, span, 6, "version", diagnostics)?;
            if !ends_with_semicolon(window, 7) {
                diagnostics.push(diagnostic(
                    "Q090",
                    span,
                    "`source.http.v1` declares a request as \
                     `let NAME = request \"METHOD\" \"/target\" \"HTTP/1.1\";` — the version is \
                     written and checked, never supplied",
                ));
                return None;
            }
            Some(HttpStatement::Declare {
                binding: binding.to_owned(),
                start: HttpStart::Request {
                    method,
                    target,
                    version,
                },
            })
        }
        // `let NAME = response "HTTP/1.1" 200 "OK";`
        Some(RESPONSE_KEYWORD) => {
            let version = text_string(window, span, 4, "version", diagnostics)?;
            let Some(status) = integer(window, 5) else {
                diagnostics.push(diagnostic(
                    "Q090",
                    span,
                    "`source.http.v1` spells a status code as an integer literal",
                ));
                return None;
            };
            let reason = text_string(window, span, 6, "reason phrase", diagnostics)?;
            if !ends_with_semicolon(window, 7) {
                diagnostics.push(diagnostic(
                    "Q090",
                    span,
                    "`source.http.v1` declares a response as \
                     `let NAME = response \"HTTP/1.1\" STATUS \"REASON\";`",
                ));
                return None;
            }
            Some(HttpStatement::Declare {
                binding: binding.to_owned(),
                start: HttpStart::Response {
                    version,
                    status,
                    reason,
                },
            })
        }
        // `let NAME = concat ( PIECE { "," PIECE } ) ;` — **text this entry
        // computes rather than writes** (card `HTTP-V1-COMPUTED-LENGTH`).
        //
        // The clause word is `source.string.v1`'s own and is read with that
        // route's own contextual predicate, so a spelling one route reads as a
        // composition the other cannot read as a reference.
        Some(CONCAT_KEYWORD)
            if is_string_clause_word(CONCAT_KEYWORD, window.get(4).map(|token| &token.kind)) =>
        {
            let mut pieces = Vec::new();
            let mut cursor = 5;
            loop {
                let (part, next) = piece(window, span, cursor, "`concat` piece", diagnostics)?;
                pieces.push(part);
                cursor = next;
                if symbol(window, cursor, Symbol::Comma) {
                    cursor += 1;
                    continue;
                }
                break;
            }
            if !symbol(window, cursor, Symbol::RightParen)
                || !ends_with_semicolon(window, cursor + 1)
            {
                diagnostics.push(diagnostic("Q090", span, "this `concat(…)` is not closed"));
                return None;
            }
            Some(HttpStatement::Compose {
                binding: binding.to_owned(),
                pieces,
            })
        }
        // `let NAME = len ( NAME ) ;` — the byte count of a bound text value.
        Some(head) if is_length_query(head, window.get(4).map(|token| &token.kind)) => {
            let Some(name) = word(window, 5) else {
                diagnostics.push(diagnostic(
                    "Q090",
                    span,
                    "`len(NAME)` names the text whose byte length it binds",
                ));
                return None;
            };
            if !symbol(window, 6, Symbol::RightParen) || !ends_with_semicolon(window, 7) {
                diagnostics.push(diagnostic("Q090", span, "this `len(…)` is not closed"));
                return None;
            }
            Some(HttpStatement::Length {
                binding: binding.to_owned(),
                name: name.to_owned(),
            })
        }
        // `let NAME = "…";` — a bound string view, usable as a payload.
        _ if literal_text(window, 3).is_some() => {
            let bytes = text(window, span, 3, "string literal", diagnostics)?;
            if !ends_with_semicolon(window, 4) {
                diagnostics.push(diagnostic(
                    "Q090",
                    span,
                    "a bound string view is one quoted token and nothing else",
                ));
                return None;
            }
            Some(HttpStatement::Text {
                binding: binding.to_owned(),
                bytes,
            })
        }
        _ => None,
    }
}

/// Lower one entry into an [`HttpProgram`]. The module is either entirely inside
/// the grammar above or refused with a precise diagnostic; there is no partially
/// lowered middle state.
///
/// # Errors
///
/// Every refusal the module earns, in source order.
pub fn lower_source_http(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredHttpModule, Vec<SourceDiagnostic>> {
    let tokens = parsed.lexical_tokens();
    let mut diagnostics = Vec::new();

    let Some(declaration) = parsed.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
    }) else {
        return Err(vec![diagnostic(
            "Q090",
            Span { start: 0, end: 0 },
            format!("`{entry}` is not a function declaration in this module"),
        )]);
    };
    let scope_span = declaration.span;

    // Every HTTP construct must be in the entry. A helper carrying one is
    // refused rather than silently unmodeled — the fail-closed hole every
    // composed route in this repository has had to close.
    for other in &parsed.declarations {
        if other.value.kind != DeclarationKind::Function || other.value.name == entry {
            continue;
        }
        if let Some(body) = &other.value.body {
            let mut found = BTreeMap::new();
            scan(body, tokens, &mut found);
            if let Some(span) = found.values().next() {
                diagnostics.push(diagnostic(
                    "Q090",
                    *span,
                    format!(
                        "`source.http.v1` models the entry only; `{}` is not the entry `{entry}`",
                        other.value.name
                    ),
                ));
            }
        }
    }

    if !declaration.value.parameters.is_empty() {
        diagnostics.push(diagnostic(
            "Q090",
            scope_span,
            "`source.http.v1` models an entry with no parameters",
        ));
    }

    let declared_effects = match declared_effect_row(&declaration.value.tokens, scope_span) {
        Ok(row) => row,
        Err(error) => {
            diagnostics.push(error);
            EffectRow::default()
        }
    };

    if declaration.value.body.is_none() {
        diagnostics.push(diagnostic("Q090", scope_span, "this entry has no body"));
        return Err(diagnostics);
    }

    let _ = typed;
    let body = declaration
        .value
        .body
        .as_ref()
        .expect("the absent body returned above");
    let mut statements = Vec::new();
    let mut event_spans = Vec::new();

    for statement in &body.statements {
        let span = statement.span;
        if !statement.value.nested_blocks().is_empty() {
            diagnostics.push(diagnostic(
                "Q090",
                span,
                "`source.http.v1` models one flat statement sequence; this statement opens a \
                 nested block",
            ));
            continue;
        }
        let window = window(tokens, span);
        let head = word(&window, 0).unwrap_or_default();
        let lowered = match head {
            // `header NAME "Field-Name" VALUE;`. The **field name** stays a
            // quoted token — a `token` is this route's own syntax and a
            // computed field name would be a different card — and the **value**
            // is any component value (card `HTTP-V1-COMPUTED-LENGTH`).
            HEADER_KEYWORD => {
                let message = word(&window, 1).map(str::to_owned);
                let name = text_string(&window, span, 2, "field name", &mut diagnostics);
                let value = piece(&window, span, 3, "field value", &mut diagnostics);
                match (message, name, value) {
                    (Some(message), Some(name), Some((value, next)))
                        if ends_with_semicolon(&window, next) =>
                    {
                        Some(HttpStatement::Header {
                            message,
                            name,
                            value,
                        })
                    }
                    (Some(_), Some(_), Some(_)) => {
                        diagnostics.push(diagnostic(
                            "Q090",
                            span,
                            "`source.http.v1` spells a header field as \
                             `header NAME \"Field-Name\" VALUE;`, where the value is a quoted \
                             token, a name this entry bound to text, or `dec(NAME)`/`hex(NAME)`",
                        ));
                        None
                    }
                    _ => None,
                }
            }
            CONTENT_KEYWORD => {
                let message = word(&window, 1).map(str::to_owned);
                let value = piece(&window, span, 2, "content", &mut diagnostics);
                match (message, value) {
                    (Some(message), Some((value, next))) if ends_with_semicolon(&window, next) => {
                        Some(HttpStatement::Content { message, value })
                    }
                    (Some(_), Some(_)) => {
                        diagnostics.push(diagnostic(
                            "Q090",
                            span,
                            "`source.http.v1` spells message content as `content NAME VALUE;`, \
                             where the value is a quoted token, a name this entry bound to text, \
                             or `dec(NAME)`/`hex(NAME)`",
                        ));
                        None
                    }
                    _ => None,
                }
            }
            // `send NAME NAME;` / `recv NAME NAME;` — a message or a bound view.
            // The byte-list and written-literal forms are the socket route's and
            // fall through to it below.
            "send" | "recv"
                if word(&window, 1).is_some()
                    && word(&window, 2).is_some()
                    && ends_with_semicolon(&window, 3) =>
            {
                Some(HttpStatement::Transfer {
                    socket: word(&window, 1).expect("matched above").to_owned(),
                    value: word(&window, 2).expect("matched above").to_owned(),
                    receive: head == "recv",
                })
            }
            "let"
                if http_use(&window).is_some()
                    || literal_text(&window, 3).is_some()
                    || is_computed_binding(&window) =>
            {
                let before = diagnostics.len();
                let lowered = lower_let(&window, span, &mut diagnostics);
                if lowered.is_none() && diagnostics.len() == before {
                    diagnostics.push(diagnostic(
                        "Q090",
                        span,
                        "`source.http.v1` binds a request, a response, a quoted string view, a \
                         `concat(…)` composition or a `len(…)`",
                    ));
                }
                lowered
            }
            // Not this route's vocabulary. The **transport's own reader**
            // decides it — `source.socket.v1::socket_statement`, the same
            // function that route's own driver calls — so a statement this
            // composition admits is byte for byte the statement the socket
            // route would have produced, and a shape one accepts and the other
            // refuses is not expressible. Its refusals carry their own message
            // and are re-attributed to this band, exactly as the socket route
            // re-attributes `source.std-io.v1`'s.
            _ => match crate::source_socket::socket_statement(parsed, &window, span) {
                SocketLowering::Lowered(statement) => Some(HttpStatement::Transport(*statement)),
                SocketLowering::Refused(refusals) => {
                    diagnostics.extend(
                        refusals
                            .into_iter()
                            .map(|(_, message)| diagnostic("Q090", span, message)),
                    );
                    None
                }
                SocketLowering::Unknown => {
                    diagnostics.push(diagnostic(
                        "Q090",
                        span,
                        format!(
                            "`{head}` is not a statement `source.http.v1` or its transport models"
                        ),
                    ));
                    None
                }
            },
        };
        if let Some(lowered) = lowered {
            statements.push(lowered);
            event_spans.push(span);
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(LoweredHttpModule {
        entry: entry.to_owned(),
        program: HttpProgram {
            entry: entry.to_owned(),
            declared_effects,
            statements,
        },
        event_spans,
        scope_span,
    })
}

/// Whether this statement is one the socket lowering will have produced, used to
/// keep the two cursors aligned.
#[must_use]
pub fn is_transport(statement: &HttpStatement) -> bool {
    matches!(statement, HttpStatement::Transport(_))
}

/// Every statement head this route reserves.
#[must_use]
pub fn statement_heads() -> [&'static str; 4] {
    HTTP_KEYWORDS
}

/// The transport statement a lowered HTTP statement carries, if it carries one.
#[must_use]
pub fn transport_statement(statement: &HttpStatement) -> Option<&SocketStatement> {
    match statement {
        HttpStatement::Transport(statement) => Some(statement),
        _ => None,
    }
}
