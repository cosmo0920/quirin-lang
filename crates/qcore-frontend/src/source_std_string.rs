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

//! Exact source lowering for `source.string.v1` — the `std.string` bundled
//! module (card `STRING-SLICE`, normative statement
//! `docs/design/STRING_SLICE_V1.md`).
//!
//! ```text
//! module    ::= import* (error-decl | entry)*
//! import    ::= "import" ("std.string" | "std.fmt" | "std.io" | "std.logger") ";"
//! entry     ::= "fn" NAME "(" ")" "->" "unit"
//!               ["effects" "{" EFFECT {"," EFFECT} "}"] "{" stmt* "}"
//! stmt      ::= "let" NAME [":" "string"] "=" STRING ";"        -- a literal view
//!             | "let" NAME "=" ["-"] INTEGER ";"
//!             | "let" NAME "=" "len" "(" NAME ")" ";"           -- byte length
//!             | "let" NAME [":" "string"] "=" NAME "[" OPERAND ":" OPERAND "]" ";"
//!             | "print" RENDERING OPERAND ";"                   -- std.io + std.fmt
//!             | "log" LEVEL RENDERING OPERAND ";"               -- std.logger + std.fmt
//!             | "print" TEXT ";"                                -- std.io, card STRING-SLICE-IO
//!             | "log" LEVEL TEXT ";"                            -- std.logger, same card
//!             | "flush" ";"                                     -- std.logger
//!             | "return" ";"
//! OPERAND   ::= ["-"] INTEGER | NAME
//! TEXT      ::= STRING | NAME | NAME "[" OPERAND ":" OPERAND "]"
//! EFFECT    ::= "io" | "alloc" | "mut"
//! ```
//!
//! # Rendering text (card `STRING-SLICE-IO`)
//!
//! Amendment §7 said what this composition had to be before it existed: "Output
//! modules may render a string only after their own capability and observation
//! rules are extended; this amendment does not make `std.io` or `std.logger`
//! accept text." The capability rule is unchanged — neither output module grants
//! the string and the string module opens no stream — and what a rendering
//! module names is **both**.
//!
//! The two statement families are separated by a **closed word set**, not by a
//! lookahead: `dec` and `hex` are `std.fmt`'s renderings, so a statement naming
//! one renders a scalar and a statement naming neither renders text. A string
//! therefore takes **no** selector, because there is nothing for one to do to a
//! byte sequence that is already the text, and a selector in front of a string
//! keeps refusing at `Q072` where it refused before this card.
//!
//! What reaches the stream is the view's exact UTF-8 payload: no re-encoding, no
//! escaping, no terminator, and no newline beyond the one
//! [`qcore_core::std_io::line_framing`] writes for every line. An embedded `0`
//! is an ordinary byte of the text and is written as one.
//!
//! # This card reserves no word, and that is a measurement
//!
//! Constraint 14's census was run over every `.qcore`/`.qc` fixture on disk and
//! every tracked `.rs`/`.md` file, and its result decided the surface:
//!
//! * **`len` is not reserved.** It occurs in the *pinned contract corpus* as a
//!   parameter name and, in `providers/quirin-provider-fluent-bit/contracts/fluent_bit/resource_memory.qv`
//!   and `providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv`, as a **reference** inside an
//!   `ensures normal resource_borrowed(len);` clause. Reserving it globally
//!   would have changed how those clauses resolve a name in a corpus this card
//!   is forbidden to touch. Instead `len` is a **contextual** clause word: the
//!   token is skipped by name resolution only where it is immediately followed
//!   by `(`, a shape that occurs **zero** times in QCore source anywhere in the
//!   repository (`len(` appears only as Rust's `.len()`). The corpus's
//!   `resource_borrowed(len)` has `len` followed by `)` and is untouched.
//! * **`string` is not reserved either**, and needs no rule at all. The type
//!   name is admitted only in a `let NAME : string = …;` ascription, and
//!   [`crate::frontend`]'s reference collector already drains every token up to
//!   and including a `let`'s `=`. The word occurs three times in `.qcore`
//!   fixtures, all inside comments, and never in binding position.
//! * The literal's delimiters, `[`, `]`, `:` and `-` are all already lexed.
//!
//! So the answer to "which words does this amendment reserve" is **none**,
//! exactly as amendment §2 asks, and the one contextual rule is narrower than
//! any reservation the sibling `STD-BUFFER` card needed.
//!
//! # The quoted-token hazard, and how the shape of this reader answers it
//!
//! Amendment §2 requires that recognizing quoted tokens for existing declaration
//! metadata must **not** acquire the new import requirement. The census found
//! exactly two consumers of [`TokenKind::StringLiteral`] in this crate, both
//! inside [`crate::frontend`]'s `verifier_scope` reader: a plugin `version` and
//! a plugin `profile`. Both live in a **declaration**, never in a function body.
//!
//! [`string_uses`] scans function-body statements and nothing else, so a
//! verifier-scope declaration is not merely filtered out of it — it is
//! structurally outside what this detector looks at. That is why
//! `plugin qcore.libevent version "0.1" profile linux_epoll …` keeps parsing and
//! checking with no `#import std.string;`, and the regression test that pins it
//! measures the shipped fixture rather than a reconstruction of it.
//!
//! # One collision with the sibling buffer route, measured and disambiguated
//!
//! `let t = s[0:3];` and `source.buffer.v1`'s byte read `let x = b[0];` share
//! their head: a `let`, a name, a `[`. Before this card the buffer detector
//! claimed both, so a slice with no imports at all reported a missing
//! `std.buffer` import. The disambiguation is the **colon**: a slice has one
//! inside its brackets and a byte read never does. It is applied in
//! [`crate::source_std_buffer`]'s classifier, which is the one place that route
//! decides what its statements are, so the two routes cannot disagree about it.
//! No shipped buffer fixture contains a colon in a byte access, so that route is
//! byte-identical.
//!
//! # What this lowering does not establish
//!
//! No mutation, no concatenation, no comparison, no search, no conversion from
//! `std.buffer` and no crossing of the extern or native-library boundary. Each
//! is a named follow-on in `docs/design/STRING_SLICE_V1.md` §9 with its missing
//! capability stated. Rendering text no longer belongs to that list, but its own
//! boundary does: every payload this route can render is a literal or a slice of
//! one, so a program renders text it **wrote** and never text it computed
//! (`STRING-SLICE-CONCAT`).

use std::collections::BTreeMap;

use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, TypedSourceProgram,
};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use crate::source_std_io::{BORROW_KEYWORD, FLUSH_KEYWORD, LOG_KEYWORD, PRINT_KEYWORD};
use qcore_core::scalar_types::{FloatEffect, FloatEffects};
use qcore_core::std_io::{Level, Rendering};
use qcore_core::std_string::{
    LiteralRefusal, StringEvent, StringOperand, StringPiece, StringProgram, TextOperand,
    decode_string_literal,
};
use qcore_core::typed_errors::{Effect, EffectRow};

/// The byte-length query. **Contextual, not reserved** — see the module header
/// for the pinned-corpus measurement that decided it.
pub const LEN_KEYWORD: &str = "len";

/// The type name this module supplies. Admitted only in a `let` ascription,
/// where the existing reference collector already drains it, so it is reserved
/// nowhere and shadows nothing.
pub const STRING_TYPE: &str = "string";

/// The composition query (card `STRING-CONCAT`). **Contextual, not reserved**,
/// on exactly the `len` precedent: the word is skipped by name resolution only
/// where it is immediately followed by `(`.
pub const CONCAT_KEYWORD: &str = "concat";

/// The `std.buffer` → `std.string` bridge, amendment §5's first operation
/// (card `STRING-CONCAT`). **Contextual, not reserved**, on the same terms.
pub const BORROW_UTF8_KEYWORD: &str = "borrow_utf8";

/// Whether this token is one of the contextual clause words `source.string.v1`
/// supplies — `len`, `concat` or `borrow_utf8` — immediately followed by `(`.
///
/// The **one** rule name resolution consults, so the module's contextual words
/// cannot drift from the words this lowering reads. Each was censused at zero
/// occurrences of its `WORD(` shape across every `.qcore`/`.qc` fixture and
/// every tracked Rust source file, which is what makes the narrow rule collide
/// with nothing; `dec` and `hex` need no entry because they have been reserved
/// words since `STD-MODULES`.
#[must_use]
pub fn is_string_clause_word(word: &str, next: Option<&TokenKind>) -> bool {
    matches!(word, LEN_KEYWORD | CONCAT_KEYWORD | BORROW_UTF8_KEYWORD)
        && matches!(next, Some(TokenKind::Symbol(Symbol::LeftParen)))
}

/// Whether this token is the contextual `len` clause word — the word `len`
/// immediately followed by `(`.
///
/// Published so [`crate::frontend`]'s reference collector applies exactly the
/// rule this module documents, rather than a second copy of it that could drift.
#[must_use]
pub fn is_length_query(word: &str, next: Option<&TokenKind>) -> bool {
    word == LEN_KEYWORD && matches!(next, Some(TokenKind::Symbol(Symbol::LeftParen)))
}

/// A complete lowering of one entry. The core checker still has to validate the
/// program; this type is not evidence on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredStringModule {
    pub entry: String,
    pub program: StringProgram,
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

/// Read `["-"] INTEGER` starting at `cursor`. Out-of-range literals are refused,
/// never wrapped.
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

fn operand(window: &[&Token], cursor: usize) -> Option<(StringOperand, usize)> {
    if let Some((value, next)) = integer(window, cursor) {
        return Some((StringOperand::Literal(value), next));
    }
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Word(name)) => Some((StringOperand::Scalar(name.clone()), cursor + 1)),
        _ => None,
    }
}

/// The cursor just past a `let NAME` head, skipping an optional one-word type
/// ascription, positioned at what should be the `=`.
///
/// The ascription is the one place the `string` type name is spelled. It carries
/// no information the initialiser does not already give — this route's only
/// string constructors are a literal and a slice of a string — so it is
/// **optional and checked**, never required and never a way to change what a
/// binding denotes.
///
/// The ascribed word is read here but **not validated** here, deliberately. A
/// statement whose initialiser is a string construct is this route's statement
/// whatever type it ascribes, so `let s: text = "hello";` reaches
/// [`lower_source_string`] and is refused at `Q068` naming the type — rather
/// than failing this classifier, falling through to the sequential route and
/// being refused for the unrelated reason that the reduced C11 CFG cannot encode
/// a quoted token. The refusal a reader gets should name what is wrong with the
/// statement they wrote.
fn let_head(window: &[&Token]) -> Option<(String, Option<String>, usize)> {
    let binding = word(window, 1)?.to_owned();
    if symbol(window, 2, Symbol::Colon) {
        let ascribed = word(window, 3)?.to_owned();
        return Some((binding, Some(ascribed), 4));
    }
    Some((binding, Option::None, 2))
}

/// Which of this card's constructs a statement uses, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StringUse {
    /// `let NAME [: string] = "…";`
    Literal,
    /// `let NAME = len(NAME);`
    Length,
    /// `let NAME [: string] = NAME[LOW:HIGH];`
    Slice,
    /// `print "…";`, `log LEVEL "…";`, `print NAME[LOW:HIGH];` — a string
    /// **constructed at a rendering operand** (card `STRING-SLICE-IO`).
    ///
    /// Deliberately only the two *constructing* forms, and **not** the bare
    /// `print NAME;`. A bare name at a rendering operand is what a
    /// `source.std-io.v1` module writes when it misspells a rendering, and
    /// claiming it here would pull a module that never mentions a string onto
    /// this route: `print bogus;` must keep refusing at `Q030` naming the
    /// missing `dec`/`hex`, which is the refusal a reader of *that* module
    /// needs. A bare name is therefore admitted as text only inside a module
    /// this rule already selected, which is the same shape the route's other
    /// composition rules have — a text render is a capability of a string
    /// module, not a way to become one.
    Render,
    /// `let NAME [: string] = concat(PIECE, …);` — text this program computes
    /// (card `STRING-CONCAT`).
    Concat,
    /// `let NAME [: string] = borrow_utf8(NAME);` — the `std.buffer` bridge
    /// (card `STRING-CONCAT`). **This is the one construct that changes route
    /// selection**: a module holding a buffer reaches `source.buffer.v1`
    /// unless it spells this, so every program written before this card travels
    /// an identical code path.
    Borrow,
}

/// Whether a text operand at `cursor` **constructs** a string, which is what
/// selects this route (see [`StringUse::Render`]).
///
/// A quoted token and an inline slice both do; a bare name does not, and an
/// integer, a `borrow` marker or anything else is not a text operand at all.
fn constructs_a_rendered_string(window: &[&Token], cursor: usize) -> bool {
    if literal_text(window, cursor).is_some() {
        return true;
    }
    word(window, cursor).is_some()
        && symbol(window, cursor + 1, Symbol::LeftBracket)
        && bracket_holds_a_colon(window, cursor + 1)
}

/// Classify one statement's token window as a string construct.
///
/// The **one** place that decides whether a statement is this route's, so route
/// selection, the import gate and the lowering below cannot disagree about it.
fn string_use(window: &[&Token]) -> Option<StringUse> {
    match word(window, 0) {
        // `print TEXT;` — the operand position is 1, and a `dec`/`hex` there is
        // the scalar rendering this card does not touch.
        Some(PRINT_KEYWORD) => {
            return (word(window, 1).and_then(Rendering::parse).is_none()
                && constructs_a_rendered_string(window, 1))
            .then_some(StringUse::Render);
        }
        // `log LEVEL TEXT;` — the level is `source.std-io.v1`'s and is read by
        // its vocabulary, so a `log` whose second word is not a level is that
        // route's refusal and not this one's.
        Some(LOG_KEYWORD) => {
            return (word(window, 1).and_then(Level::parse).is_some()
                && word(window, 2).and_then(Rendering::parse).is_none()
                && constructs_a_rendered_string(window, 2))
            .then_some(StringUse::Render);
        }
        Some("let") => {}
        _ => return None,
    }
    let (_, _, equal) = let_head(window)?;
    if !symbol(window, equal, Symbol::Equal) {
        return None;
    }
    let value = equal + 1;
    if literal_text(window, value).is_some() {
        return Some(StringUse::Literal);
    }
    let head = word(window, value)?;
    let after = window.get(value + 1).map(|token| &token.kind);
    if is_length_query(head, after) {
        return Some(StringUse::Length);
    }
    if head == CONCAT_KEYWORD && is_string_clause_word(head, after) {
        return Some(StringUse::Concat);
    }
    if head == BORROW_UTF8_KEYWORD && is_string_clause_word(head, after) {
        return Some(StringUse::Borrow);
    }
    // `let NAME = NAME [ … : … ]`. The colon is what separates a slice from
    // `source.buffer.v1`'s byte read; see the module header.
    if word(window, value).is_some()
        && symbol(window, value + 1, Symbol::LeftBracket)
        && bracket_holds_a_colon(window, value + 1)
    {
        return Some(StringUse::Slice);
    }
    None
}

/// Whether the bracket opened at `cursor` contains a `:` before its `]`.
///
/// One statement, one bracket depth: this route's grammar has no nested
/// brackets, so a depth counter would model a shape neither route admits.
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
///
/// **Function bodies only**, which is what keeps a `verifier scope`
/// declaration's quoted plugin version and profile outside this detector
/// entirely — see the module header.
#[must_use]
pub fn string_uses(program: &SourceProgram) -> BTreeMap<StringUse, Span> {
    let tokens = program.lexical_tokens();
    let mut found: BTreeMap<StringUse, Span> = BTreeMap::new();

    fn scan(block: &SourceBlock, tokens: &[Token], found: &mut BTreeMap<StringUse, Span>) {
        for statement in &block.statements {
            let window = window(tokens, statement.span);
            if let Some(kind) = string_use(&window) {
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

/// Whether this module is a `source.string.v1` module.
#[must_use]
pub fn has_source_string(program: &SourceProgram) -> bool {
    !string_uses(program).is_empty()
}

/// The span of the first statement needing `#import std.string;`.
#[must_use]
pub fn string_span(program: &SourceProgram) -> Option<Span> {
    string_uses(program).values().next().copied()
}

/// Whether this module spells the `std.buffer` → `std.string` bridge
/// (card `STRING-CONCAT`).
///
/// **The whole of the route-selection change this card makes.** A module that
/// holds a buffer reaches `source.buffer.v1` exactly as it did before, unless
/// it writes `borrow_utf8(…)` — the one construct whose meaning needs both
/// models at once. A module that only composes text holds no buffer and never
/// competed with that route in the first place.
#[must_use]
pub fn has_source_string_bridge(program: &SourceProgram) -> bool {
    string_uses(program).contains_key(&StringUse::Borrow)
}

fn diagnostic(code: &'static str, span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code,
        pass: SemanticPass::StandardString,
        span,
        message: message.into(),
    }
}

/// The entry's declared `effects { … }` row. An absent suffix is the empty row,
/// which admits nothing, so the opt-in stays visible.
///
/// Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: the row may also name `partial_eq` and
/// `partial_ord`, returned beside it — what a float comparison of the embedded
/// `source.buffer.v1` value statements needs, never inferred.
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
            "Q068",
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
                            "Q068",
                            span,
                            format!(
                                "`{other}` is not an effect `source.string.v1` models; use io, \
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
                return Err(diagnostic("Q068", span, "this `effects` row is not closed"));
            }
        }
    }
    Ok((EffectRow::new(effects), comparisons))
}

/// The literal pool of one lowering: decoded payloads, deduplicated by payload.
///
/// Pooling identical literals is amendment §3's explicit permission — "identical
/// literals may be pooled only when observable identity is absent" — and this
/// profile exposes no literal address, so the premise holds. Pooling is
/// therefore an emission choice with no semantic consequence, and the check is
/// written so that it stays one: a view's identity in the certificate is
/// `(literal, offset, len)`, and two views of the same pooled payload are the
/// same view of the same static storage, which is exactly what the source said.
struct Literals {
    payloads: Vec<Vec<u8>>,
}

impl Literals {
    fn intern(&mut self, payload: Vec<u8>) -> usize {
        if let Some(index) = self
            .payloads
            .iter()
            .position(|existing| *existing == payload)
        {
            return index;
        }
        self.payloads.push(payload);
        self.payloads.len() - 1
    }
}

/// What a line-producing statement turned out to be (card `STRING-SLICE-IO`).
enum LineForm {
    /// A rendered-text line, lowered.
    Text(StringEvent),
    /// Not a text line: `source.std-io.v1`'s own reader decides it, including
    /// every refusal it already had. `flush s;` lands here and keeps that
    /// route's "takes no operand" refusal rather than acquiring one of this
    /// composition's.
    Scalar,
    /// A text line this route refuses. The diagnostic is already recorded.
    Refused,
}

/// Read `print TEXT;` / `log LEVEL TEXT;` — the composition of `std.string`
/// with the output modules (card `STRING-SLICE-IO`).
///
/// **The selector is the discriminator, and it is a closed word set.** `dec` and
/// `hex` are `std.fmt`'s two renderings; a statement naming one is the scalar
/// line this card does not touch, and a statement naming neither renders text.
/// There is no rendering for a string because there is nothing for one to do: a
/// `string` is already the bytes, so the observation is the view's exact UTF-8
/// payload — no re-encoding, no escaping, no terminator, and no newline beyond
/// the one the sink's own framing writes for every line.
fn text_line(
    window: &[&Token],
    span: Span,
    literals: &mut Literals,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> LineForm {
    let (level, cursor) = match word(window, 0) {
        Some(PRINT_KEYWORD) => {
            if word(window, 1).and_then(Rendering::parse).is_some() {
                return LineForm::Scalar;
            }
            (Option::None, 1)
        }
        Some(LOG_KEYWORD) => {
            let Some(level) = word(window, 1).and_then(Level::parse) else {
                return LineForm::Scalar;
            };
            if word(window, 2).and_then(Rendering::parse).is_some() {
                return LineForm::Scalar;
            }
            (Some(level), 2)
        }
        _ => return LineForm::Scalar,
    };

    // `print "…";` — a literal rendered where it is written. It interns into the
    // same pool a `let` literal view does, so two spellings of one payload are
    // one static array and one view of it.
    if let Some(raw) = literal_text(window, cursor) {
        if !ends_with_semicolon(window, cursor + 1) {
            diagnostics.push(diagnostic(
                "Q075",
                span,
                "a rendered literal is one quoted token and nothing else",
            ));
            return LineForm::Refused;
        }
        let payload = match decode_string_literal(raw) {
            Ok(payload) => payload,
            Err(refusal) => {
                // Decoding is the base route's rule and keeps the base route's
                // code: a token that does not decode has no payload wherever it
                // was written.
                diagnostics.push(diagnostic("Q070", span, refusal.to_string()));
                return LineForm::Refused;
            }
        };
        return LineForm::Text(StringEvent::Text {
            level,
            operand: TextOperand::Literal(literals.intern(payload)),
        });
    }

    let Some(name) = word(window, cursor) else {
        diagnostics.push(diagnostic(
            "Q075",
            span,
            "a rendering operand is a `std.fmt` rendering and a scalar (`print dec n;`), or a \
             string this entry bound, a quoted literal, or one slice of a string",
        ));
        return LineForm::Refused;
    };
    if name == BORROW_KEYWORD {
        diagnostics.push(diagnostic(
            "Q075",
            span,
            "`borrow NAME` renders the model identity of a live resource, and a string view is not \
             a resource: it owns nothing, it publishes no identity, and creating one neither \
             creates nor discharges custody. Render the view itself (`print NAME;`)",
        ));
        return LineForm::Refused;
    }

    // `print NAME;` — the bare form. It renders a string and only a string; a
    // scalar at this position is refused by the model at `Q076`, naming the
    // rendering that would have rendered it.
    if ends_with_semicolon(window, cursor + 1) {
        return LineForm::Text(StringEvent::Text {
            level,
            operand: TextOperand::Name(name.to_owned()),
        });
    }

    // `print NAME[LOW:HIGH];` — the slice written where it is rendered. It is
    // the same checked slice a `let` binds; the bounds are decided by the same
    // rule and the arithmetic is recorded in the same certificate list.
    if symbol(window, cursor + 1, Symbol::LeftBracket) {
        let Some((low, next)) = operand(window, cursor + 2) else {
            diagnostics.push(diagnostic(
                "Q075",
                span,
                "a slice bounds with one integer literal or one bound integer name",
            ));
            return LineForm::Refused;
        };
        if !symbol(window, next, Symbol::Colon) {
            diagnostics.push(diagnostic(
                "Q075",
                span,
                "`source.string.v1` slices with `NAME[LOW:HIGH]`; a single-index access into a \
                 string is not a construct this route models, because a byte of UTF-8 text is not \
                 a character",
            ));
            return LineForm::Refused;
        }
        let Some((high, next)) = operand(window, next + 1) else {
            diagnostics.push(diagnostic(
                "Q075",
                span,
                "a slice bounds with one integer literal or one bound integer name",
            ));
            return LineForm::Refused;
        };
        if !symbol(window, next, Symbol::RightBracket) || !ends_with_semicolon(window, next + 1) {
            diagnostics.push(diagnostic("Q075", span, "this slice is not closed"));
            return LineForm::Refused;
        }
        return LineForm::Text(StringEvent::Text {
            level,
            operand: TextOperand::Slice {
                name: name.to_owned(),
                low,
                high,
            },
        });
    }

    diagnostics.push(diagnostic(
        "Q075",
        span,
        "a rendered string is one operand and nothing else: a string this entry bound, a quoted \
         literal, or one slice of a string",
    ));
    LineForm::Refused
}

/// Read one `concat` piece, returning it and the cursor just past it
/// (card `STRING-CONCAT`).
///
/// Four forms, and every one of them is check-time-known — which is the whole
/// reason the result can be composed by the compiler rather than built at run
/// time. The first three are the **same three** a rendered text operand takes,
/// spelled the same way; the fourth is the one scalar-to-text direction this
/// route has.
fn concat_piece(
    window: &[&Token],
    cursor: usize,
    literals: &mut Literals,
    span: Span,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<(StringPiece, usize)> {
    // `"…"` — a quoted literal, interned into the same pool a bound literal
    // view interns into.
    if let Some(raw) = literal_text(window, cursor) {
        let payload = match decode_string_literal(raw) {
            Ok(payload) => payload,
            Err(refusal) => {
                // Decoding is the base route's rule and keeps the base route's
                // code, wherever the token is written.
                diagnostics.push(diagnostic("Q070", span, refusal.to_string()));
                return None;
            }
        };
        return Some((
            StringPiece::Text(TextOperand::Literal(literals.intern(payload))),
            cursor + 1,
        ));
    }
    let Some(head) = word(window, cursor) else {
        diagnostics.push(diagnostic(
            "Q077",
            span,
            "a `concat` piece is a quoted literal, a string this entry bound, one slice of one, or \
             `dec(NAME)`/`hex(NAME)` for the text of a bound integer",
        ));
        return None;
    };
    // `dec ( NAME )` / `hex ( NAME )` — the digits of a check-time constant,
    // produced by the **same** `Rendering` a `print dec n;` line renders with,
    // so the two spellings of a number cannot disagree.
    if let Some(rendering) = Rendering::parse(head)
        && symbol(window, cursor + 1, Symbol::LeftParen)
    {
        let Some(name) = word(window, cursor + 2) else {
            diagnostics.push(diagnostic(
                "Q077",
                span,
                "`dec(NAME)` and `hex(NAME)` name the bound integer whose text they compose",
            ));
            return None;
        };
        if !symbol(window, cursor + 3, Symbol::RightParen) {
            diagnostics.push(diagnostic(
                "Q077",
                span,
                "this rendering piece is not closed",
            ));
            return None;
        }
        return Some((
            StringPiece::Scalar {
                rendering,
                name: name.to_owned(),
            },
            cursor + 4,
        ));
    }
    // `NAME [ OPERAND : OPERAND ]` — an inline slice, decided by the same
    // predicate a bound slice is decided by.
    if symbol(window, cursor + 1, Symbol::LeftBracket) {
        let Some((low, next)) = operand(window, cursor + 2) else {
            diagnostics.push(diagnostic(
                "Q077",
                span,
                "a slice bounds with one integer literal or one bound integer name",
            ));
            return None;
        };
        if !symbol(window, next, Symbol::Colon) {
            diagnostics.push(diagnostic(
                "Q077",
                span,
                "`source.string.v1` slices with `NAME[LOW:HIGH]`",
            ));
            return None;
        }
        let Some((high, next)) = operand(window, next + 1) else {
            diagnostics.push(diagnostic(
                "Q077",
                span,
                "a slice bounds with one integer literal or one bound integer name",
            ));
            return None;
        };
        if !symbol(window, next, Symbol::RightBracket) {
            diagnostics.push(diagnostic("Q077", span, "this slice is not closed"));
            return None;
        }
        return Some((
            StringPiece::Text(TextOperand::Slice {
                name: head.to_owned(),
                low,
                high,
            }),
            next + 1,
        ));
    }
    // `NAME` — a string this entry bound.
    Some((
        StringPiece::Text(TextOperand::Name(head.to_owned())),
        cursor + 1,
    ))
}

/// Lower one `let` statement.
fn lower_let(
    window: &[&Token],
    span: Span,
    literals: &mut Literals,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<StringEvent> {
    let Some((binding, ascribed, equal)) = let_head(window) else {
        diagnostics.push(diagnostic("Q068", span, "this `let` binds no name"));
        return None;
    };
    // The one place `string` is spelled, and the one place the spelling is
    // checked. The type name is *supplied by the module*: it is reserved
    // nowhere, shadows nothing, and is admitted only here.
    if let Some(ascribed) = ascribed
        && ascribed != STRING_TYPE
    {
        diagnostics.push(diagnostic(
            "Q068",
            span,
            format!(
                "`{ascribed}` is not a type `source.string.v1` models; the only type name \
                 `std.string` supplies is `{STRING_TYPE}`"
            ),
        ));
        return None;
    }
    if !symbol(window, equal, Symbol::Equal) {
        diagnostics.push(diagnostic(
            "Q068",
            span,
            "`source.string.v1` binds `let NAME = …;` only",
        ));
        return None;
    }
    let value = equal + 1;

    // `let NAME [: string] = STRING ;`
    if let Some(raw) = literal_text(window, value) {
        if !ends_with_semicolon(window, value + 1) {
            diagnostics.push(diagnostic(
                "Q068",
                span,
                "a literal view binds one quoted token and nothing else",
            ));
            return None;
        }
        // Decoding is where bytes enter the model, and the only place. A token
        // that does not decode has no payload, so it has no length and no
        // boundaries either and none of the later rules could be stated of it.
        let payload = match decode_string_literal(raw) {
            Ok(payload) => payload,
            Err(refusal) => {
                diagnostics.push(diagnostic("Q070", span, refusal.to_string()));
                // A `TooLong` payload would still be usable as bytes, but this
                // route refuses rather than truncating it (`LiteralRefusal`'s
                // own rule), so nothing is registered for it.
                let _: LiteralRefusal = refusal;
                return None;
            }
        };
        return Some(StringEvent::Literal {
            binding,
            literal: literals.intern(payload),
        });
    }

    // `let NAME [: string] = borrow_utf8 ( NAME ) ;` (card `STRING-CONCAT`)
    if let Some(head) = word(window, value)
        && head == BORROW_UTF8_KEYWORD
        && is_string_clause_word(head, window.get(value + 1).map(|token| &token.kind))
    {
        let Some(buffer) = word(window, value + 2) else {
            diagnostics.push(diagnostic(
                "Q077",
                span,
                "`borrow_utf8(NAME)` names the owned byte buffer whose bytes it views",
            ));
            return None;
        };
        // Card `CT07-SLICE-PERMISSIONS`: `borrow_utf8(NAME, LOW, HIGH)` lends a
        // **sub-extent**. The arity is exactly one or three and nothing between:
        // a two-argument spelling has no reading — a range needs both ends —
        // and it keeps refusing by shape where it always did.
        let range = if symbol(window, value + 3, Symbol::Comma) {
            let Some((low, after_low)) = operand(window, value + 4) else {
                diagnostics.push(diagnostic(
                    "Q077",
                    span,
                    "`borrow_utf8(NAME, LOW, HIGH)` bounds the lent range with an integer literal \
                     or a name this entry bound to one",
                ));
                return None;
            };
            if !symbol(window, after_low, Symbol::Comma) {
                diagnostics.push(diagnostic(
                    "Q077",
                    span,
                    "`borrow_utf8(NAME, LOW, HIGH)` lends a range and needs both of its ends",
                ));
                return None;
            }
            let Some((high, after_high)) = operand(window, after_low + 1) else {
                diagnostics.push(diagnostic(
                    "Q077",
                    span,
                    "`borrow_utf8(NAME, LOW, HIGH)` bounds the lent range with an integer literal \
                     or a name this entry bound to one",
                ));
                return None;
            };
            if !symbol(window, after_high, Symbol::RightParen)
                || !ends_with_semicolon(window, after_high + 1)
            {
                diagnostics.push(diagnostic(
                    "Q077",
                    span,
                    "this `borrow_utf8(…)` is not closed",
                ));
                return None;
            }
            Some((low, high))
        } else {
            if !symbol(window, value + 3, Symbol::RightParen)
                || !ends_with_semicolon(window, value + 4)
            {
                diagnostics.push(diagnostic(
                    "Q077",
                    span,
                    "this `borrow_utf8(…)` is not closed",
                ));
                return None;
            }
            None
        };
        return Some(StringEvent::Borrow {
            binding,
            buffer: buffer.to_owned(),
            range,
        });
    }

    // `let NAME [: string] = concat ( PIECE { "," PIECE } ) ;`
    // (card `STRING-CONCAT`)
    if let Some(head) = word(window, value)
        && head == CONCAT_KEYWORD
        && is_string_clause_word(head, window.get(value + 1).map(|token| &token.kind))
    {
        let mut pieces = Vec::new();
        let mut cursor = value + 2;
        loop {
            let (piece, next) = concat_piece(window, cursor, literals, span, diagnostics)?;
            pieces.push(piece);
            cursor = next;
            if symbol(window, cursor, Symbol::Comma) {
                cursor += 1;
                continue;
            }
            break;
        }
        if !symbol(window, cursor, Symbol::RightParen) || !ends_with_semicolon(window, cursor + 1) {
            diagnostics.push(diagnostic("Q077", span, "this `concat(…)` is not closed"));
            return None;
        }
        return Some(StringEvent::Concat { binding, pieces });
    }

    // `let NAME = len ( NAME ) ;`
    if let Some(head) = word(window, value)
        && is_length_query(head, window.get(value + 1).map(|token| &token.kind))
    {
        let Some(name) = word(window, value + 2) else {
            diagnostics.push(diagnostic(
                "Q068",
                span,
                "`len(NAME)` names the string whose byte length it binds",
            ));
            return None;
        };
        if !symbol(window, value + 3, Symbol::RightParen) || !ends_with_semicolon(window, value + 4)
        {
            diagnostics.push(diagnostic("Q068", span, "this `len(…)` is not closed"));
            return None;
        }
        return Some(StringEvent::Len {
            binding,
            name: name.to_owned(),
        });
    }

    // `let NAME [: string] = NAME [ OPERAND : OPERAND ] ;`
    if let Some(name) = word(window, value)
        && symbol(window, value + 1, Symbol::LeftBracket)
    {
        let Some((low, next)) = operand(window, value + 2) else {
            diagnostics.push(diagnostic(
                "Q068",
                span,
                "a slice bounds with one integer literal or one bound integer name",
            ));
            return None;
        };
        if !symbol(window, next, Symbol::Colon) {
            diagnostics.push(diagnostic(
                "Q068",
                span,
                "`source.string.v1` slices with `NAME[LOW:HIGH]`; a single-index access into a \
                 string is not a construct this route models, because a byte of UTF-8 text is not \
                 a character",
            ));
            return None;
        }
        let Some((high, next)) = operand(window, next + 1) else {
            diagnostics.push(diagnostic(
                "Q068",
                span,
                "a slice bounds with one integer literal or one bound integer name",
            ));
            return None;
        };
        if !symbol(window, next, Symbol::RightBracket) || !ends_with_semicolon(window, next + 1) {
            diagnostics.push(diagnostic("Q068", span, "this slice is not closed"));
            return None;
        }
        return Some(StringEvent::Slice {
            binding,
            name: name.to_owned(),
            low,
            high,
        });
    }

    // `let NAME = ["-"] INTEGER ;`
    match integer(window, value) {
        Some((literal, next)) if ends_with_semicolon(window, next) => Some(StringEvent::Bind {
            binding,
            value: literal,
        }),
        _ => {
            diagnostics.push(diagnostic(
                "Q068",
                span,
                "`source.string.v1` binds an integer literal, a quoted literal, `len(NAME)` or one \
                 slice; nothing else",
            ));
            None
        }
    }
}

/// Lower one entry into a [`StringProgram`]. The module is either entirely
/// inside the grammar above or refused with a precise diagnostic; there is no
/// partially lowered middle state.
pub fn lower_source_string(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredStringModule, Vec<SourceDiagnostic>> {
    lower_source_string_for_target(parsed, typed, entry, None)
}

/// [`lower_source_string`] under the builder's recorded target (card
/// `W48-QR-SCALAR-TYPES-R6`), which the lowering's constant tier reads to fold
/// an `isize`/`usize` loop bound of a composed module.
pub fn lower_source_string_for_target(
    parsed: &SourceProgram,
    _typed: &TypedSourceProgram,
    entry: &str,
    target: Option<&qcore_core::scalar_types::RecordedTarget>,
) -> Result<LoweredStringModule, Vec<SourceDiagnostic>> {
    let tokens = parsed.lexical_tokens();
    let mut diagnostics = Vec::new();

    let Some(declaration) = parsed.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
    }) else {
        return Err(vec![diagnostic(
            "Q068",
            Span { start: 0, end: 0 },
            format!("`{entry}` is not a function declaration in this module"),
        )]);
    };
    let scope_span = declaration.span;

    // Every string construct must be in the entry. A helper function carrying
    // one is refused rather than silently unmodeled.
    for other in &parsed.declarations {
        if other.value.kind != DeclarationKind::Function || other.value.name == entry {
            continue;
        }
        if let Some(body) = &other.value.body {
            let mut found = BTreeMap::new();
            fn scan(block: &SourceBlock, tokens: &[Token], found: &mut BTreeMap<StringUse, Span>) {
                for statement in &block.statements {
                    let window = window(tokens, statement.span);
                    if let Some(kind) = string_use(&window) {
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
                    "Q068",
                    *span,
                    format!(
                        "`source.string.v1` models the entry only; `{}` is not the entry `{entry}`",
                        other.value.name
                    ),
                ));
            }
            // Card `STRING-CONCAT`: a composed module models buffer statements
            // too, so a buffer statement in a helper must be refused here
            // rather than being neither emitted nor refused. That is the same
            // fail-closed hole `STRING-SLICE-IO` closed for a rendered text
            // line, at this band's own code.
            let outside = body.statements.iter().find(|statement| {
                crate::source_std_buffer::buffer_statement(parsed, statement.span).is_some()
            });
            if let Some(statement) = outside {
                diagnostics.push(diagnostic(
                    "Q077",
                    statement.span,
                    format!(
                        "`source.string.v1` models the entry only; the owned byte buffer `{}` \
                         holds is outside the entry `{entry}`",
                        other.value.name
                    ),
                ));
            }
        }
    }

    if !declaration.value.parameters.is_empty() {
        diagnostics.push(diagnostic(
            "Q068",
            scope_span,
            "`source.string.v1` models an entry with no parameters",
        ));
    }

    let (declared_effects, float_effects) =
        match declared_effect_row(&declaration.value.tokens, scope_span) {
            Ok(row) => row,
            Err(error) => {
                diagnostics.push(error);
                (EffectRow::default(), FloatEffects::default())
            }
        };

    let Some(body) = &declaration.value.body else {
        diagnostics.push(diagnostic("Q068", scope_span, "this entry has no body"));
        return Err(diagnostics);
    };

    let mut literals = Literals {
        payloads: Vec::new(),
    };
    let mut events = Vec::new();
    let mut event_spans = Vec::new();

    // Card `STRING-CONCAT`: **a composed module is one that acquires a
    // buffer.** Deciding that up front, from the acquisition and nothing else,
    // is what keeps every program written before this card byte-identical: a
    // module with no `bytes(N)` in its entry embeds no buffer statement, so
    // `drop s;` on a string keeps refusing at `Q068` where it refused before,
    // rather than acquiring a "that is not a buffer" answer to a question it
    // never asked.
    let holds_a_buffer = body.statements.iter().any(|statement| {
        matches!(
            crate::source_std_buffer::buffer_statement(parsed, statement.span),
            Some(Ok(qcore_core::std_buffer::BufferEvent::Acquire { .. }))
        )
    });

    // Card `LOOPS-ON-COMPOSED-ROUTES`: the lowering-visible constants a loop
    // bound may name, kept by `source.buffer.v1`'s own environment so the
    // bound is resolved in that route's tier and no other.
    let mut constants = crate::source_std_buffer::LoweringConstants::new();
    constants.target = target.cloned();

    for statement in &body.statements {
        let span = statement.span;
        // Card `LOOPS-ON-COMPOSED-ROUTES`: **a `for` is admitted in a composed
        // module**, lowered by `source.buffer.v1`'s own loop reader, so the
        // loop and its body are that route's statements read by that route's
        // reader — `LOOP-V1`'s own statement, surviving the composition. The
        // window question the old refusal named is decided in the checker
        // (`check_string_program`'s `Repeat` arm, over `QCore.LoopWindows`).
        // Every *other* nested block keeps the flat-sequence refusal below.
        if holds_a_buffer && matches!(statement.value, crate::frontend::StatementKind::For { .. }) {
            // A body is `source.buffer.v1`'s vocabulary. A string construct
            // inside it is refused **here, by name**, before the buffer reader
            // sees the statement — a `borrow_utf8` in a body would be a
            // per-iteration loan (`LOOP-PER-ITERATION-LOAN`), and a literal,
            // a `len`, a `concat` or a text line is a view construct this
            // composition's loop body does not model
            // (`LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY`).
            fn body_string_use(block: &SourceBlock, tokens: &[Token]) -> Option<(StringUse, Span)> {
                for statement in &block.statements {
                    let window = window(tokens, statement.span);
                    if let Some(kind) = string_use(&window) {
                        return Some((kind, statement.span));
                    }
                    for nested in statement.value.nested_blocks() {
                        if let Some(found) = body_string_use(nested, tokens) {
                            return Some(found);
                        }
                    }
                }
                None
            }
            let mut refused = false;
            for block in statement.value.nested_blocks() {
                if let Some((kind, inner)) = body_string_use(block, tokens) {
                    let message = match kind {
                        StringUse::Borrow => {
                            "this `for` body borrows a buffer, which is a per-iteration loan: \
                             its window opens and closes inside one iteration and is not an \
                             interval of statement indices, so the composed string/buffer entry \
                             refuses it by name rather than approximating it; a view a loop \
                             body needs is borrowed before the loop (`LOOP-PER-ITERATION-LOAN`)"
                        }
                        StringUse::Literal
                        | StringUse::Length
                        | StringUse::Concat
                        | StringUse::Render
                        | StringUse::Slice => {
                            "a `for` body on the composed string/buffer route is \
                             `source.buffer.v1`'s statement vocabulary, read by that route's \
                             reader; a string view, a `len`, a `concat` or a text line inside \
                             one is not modelled and is refused by name \
                             (`LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY`)"
                        }
                    };
                    diagnostics.push(diagnostic("Q077", inner, message));
                    refused = true;
                    break;
                }
            }
            if refused {
                continue;
            }
            let mut loop_diagnostics = Vec::new();
            let event = crate::source_std_buffer::lower_for_statement(
                parsed,
                statement,
                &mut constants,
                &mut loop_diagnostics,
            );
            for mut refusal in loop_diagnostics {
                // The buffer reader's own message under **this** pass's code:
                // §L21 forbids a second pass taking `Q051`, and the loop's
                // `Q097`–`Q099` are the loop pass's and stay as they are.
                if refusal.code == "Q051" {
                    refusal.code = "Q077";
                    refusal.pass = SemanticPass::StandardString;
                }
                diagnostics.push(refusal);
            }
            if let Some(event) = event {
                events.push(StringEvent::Buffer(event));
                event_spans.push(span);
            }
            continue;
        }
        if !statement.value.nested_blocks().is_empty() {
            diagnostics.push(diagnostic(
                "Q068",
                span,
                "`source.string.v1` models one flat statement sequence; this statement opens a \
                 nested block",
            ));
            continue;
        }
        let window = window(tokens, span);
        let head = word(&window, 0).unwrap_or_default();
        // Card `STRING-CONCAT`: a `source.buffer.v1` statement embedded in a
        // composed module is read by **that route's own reader**, so a byte
        // write this route admits is one that route admits, spelled and bounded
        // identically. Its refusal message is that route's; only the code is
        // this pass's, because §L21 gives each pass its own range.
        //
        // The attempt runs before this route's own `let` reader and cannot
        // steal a statement from it: `buffer_use` claims a `let` only for
        // `bytes(…)`, `extent(…)` and a **colon-free** bracket, and this
        // route's own `let` forms are a quoted literal, `len(…)`, `concat(…)`,
        // `borrow_utf8(…)`, an integer and a colon-bearing bracket.
        // Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-TYPES-COMPOSED-STRING`): a
        // conversion or a value statement of a composed module is
        // `source.buffer.v1`'s statement, read by that route's reader — the one
        // the extern and task routes share — so the composed entry reads the
        // fixed-width tier exactly as the buffer route does. It claims only
        // those two shapes, and none of this route's own `let` forms (a quoted
        // literal, `len`, `concat`, `borrow_utf8`, an integer, a slice) is
        // one, so it steals nothing; before the card the shape refused here
        // at `Q068`.
        if holds_a_buffer
            && head == "let"
            && string_use(&window).is_none()
            && let Some(read) = crate::source_std_buffer::value_statement(parsed, span)
        {
            match read {
                Ok(buffer_event) => {
                    constants.observe(&buffer_event);
                    events.push(StringEvent::Buffer(buffer_event));
                    event_spans.push(span);
                }
                Err(message) => diagnostics.push(diagnostic("Q077", span, message)),
            }
            continue;
        }
        let buffer_read = if !holds_a_buffer
            || head.is_empty()
            || matches!(head, PRINT_KEYWORD | LOG_KEYWORD | FLUSH_KEYWORD | "return")
            || string_use(&window).is_some()
        {
            Option::None
        } else {
            crate::source_std_buffer::buffer_statement(parsed, span)
        };
        if let Some(read) = buffer_read {
            match read {
                Ok(buffer_event) => {
                    constants.observe(&buffer_event);
                    events.push(StringEvent::Buffer(buffer_event));
                    event_spans.push(span);
                }
                Err(message) => diagnostics.push(diagnostic("Q077", span, message)),
            }
            continue;
        }
        let event = match head {
            "let" => lower_let(&window, span, &mut literals, &mut diagnostics),
            // `log_level LEVEL;` is deliberately **absent** from this list, so
            // it reaches the `other` arm below and refuses at `Q068` — the same
            // disposition `source.buffer.v1` gives it, and fail-closed rather
            // than accept-and-ignore. The quiet-emission amendment's
            // declaration installs the shared system-trace adapter at
            // emission time through `emit_c11_with_log_level`, which this
            // route's backend does not call; accepting the statement here
            // would let a module declare a threshold that silently did
            // nothing. Source-authored `print` and `log` are unaffected and
            // are this route's whole observation, because they are the
            // formatted-I/O owners rather than compiler observations.
            PRINT_KEYWORD | LOG_KEYWORD | FLUSH_KEYWORD => {
                // Card `STRING-SLICE-IO` first: a line whose operand renders
                // **text** is this composition's, and a line naming a `std.fmt`
                // rendering is `source.std-io.v1`'s, unchanged. The two are
                // separated by a closed word set, so no statement is both.
                match text_line(&window, span, &mut literals, &mut diagnostics) {
                    LineForm::Text(event) => {
                        events.push(event);
                        event_spans.push(span);
                        continue;
                    }
                    LineForm::Refused => continue,
                    LineForm::Scalar => {}
                }
                // The scalar line grammar is `source.std-io.v1`'s, read by its
                // own reader, so a shape one route accepts and another refuses
                // is not expressible.
                match crate::source_std_io::std_io_statement(parsed, span) {
                    Some(Ok(event)) => Some(StringEvent::Line(event)),
                    Some(Err(message)) => {
                        diagnostics.push(diagnostic("Q068", span, message));
                        None
                    }
                    None => {
                        diagnostics.push(diagnostic(
                            "Q068",
                            span,
                            "this line-producing statement is not one `source.std-io.v1` models",
                        ));
                        None
                    }
                }
            }
            "return" => {
                if ends_with_semicolon(&window, 1) {
                    Some(StringEvent::Return)
                } else {
                    diagnostics.push(diagnostic(
                        "Q068",
                        span,
                        "`source.string.v1` models `return;` from a `unit` entry",
                    ));
                    None
                }
            }
            other => {
                diagnostics.push(diagnostic(
                    "Q068",
                    span,
                    format!("`{other}` is not a statement `source.string.v1` models"),
                ));
                None
            }
        };
        if let Some(event) = event {
            if let StringEvent::Bind { binding, value } = &event {
                constants.observe_scalar(binding, *value);
            }
            events.push(event);
            event_spans.push(span);
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(LoweredStringModule {
        entry: entry.to_owned(),
        program: StringProgram {
            entry: entry.to_owned(),
            declared_effects,
            literals: literals.payloads,
            events,
            float_effects,
        },
        event_spans,
        scope_span,
    })
}
