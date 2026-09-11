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
//!             | "flush" ";"                                     -- std.logger
//!             | "return" ";"
//! OPERAND   ::= ["-"] INTEGER | NAME
//! EFFECT    ::= "io" | "alloc" | "mut"
//! ```
//!
//! # This card reserves no word, and that is a measurement
//!
//! Constraint 14's census was run over every `.qcore`/`.qc` fixture on disk and
//! every tracked `.rs`/`.md` file, and its result decided the surface:
//!
//! * **`len` is not reserved.** It occurs in the *pinned contract corpus* as a
//!   parameter name and, in `contract/qcore/fluent_bit/resource_memory.qcore`
//!   and `contract/qcore/librdkafka/memory.qcore`, as a **reference** inside an
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
//! checking with no `import std.string;`, and the regression test that pins it
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
//! `std.buffer`, no rendering of text and no crossing of the extern or
//! native-library boundary. Each is a named follow-on in
//! `docs/design/STRING_SLICE_V1.md` §9 with its missing capability stated.

use std::collections::BTreeMap;

use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, TypedSourceProgram,
};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use crate::source_std_io::{FLUSH_KEYWORD, LOG_KEYWORD, PRINT_KEYWORD};
use qcore_core::std_string::{
    LiteralRefusal, StringEvent, StringOperand, StringProgram, decode_string_literal,
};
use qcore_core::typed_errors::{Effect, EffectRow};

/// The byte-length query. **Contextual, not reserved** — see the module header
/// for the pinned-corpus measurement that decided it.
pub const LEN_KEYWORD: &str = "len";

/// The type name this module supplies. Admitted only in a `let` ascription,
/// where the existing reference collector already drains it, so it is reserved
/// nowhere and shadows nothing.
pub const STRING_TYPE: &str = "string";

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
/// [`lower_source_string`] and is refused at `Q065` naming the type — rather
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
}

/// Classify one statement's token window as a string construct.
///
/// The **one** place that decides whether a statement is this route's, so route
/// selection, the import gate and the lowering below cannot disagree about it.
fn string_use(window: &[&Token]) -> Option<StringUse> {
    if word(window, 0) != Some("let") {
        return None;
    }
    let (_, _, equal) = let_head(window)?;
    if !symbol(window, equal, Symbol::Equal) {
        return None;
    }
    let value = equal + 1;
    if literal_text(window, value).is_some() {
        return Some(StringUse::Literal);
    }
    if is_length_query(
        word(window, value)?,
        window.get(value + 1).map(|token| &token.kind),
    ) {
        return Some(StringUse::Length);
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

/// The span of the first statement needing `import std.string;`.
#[must_use]
pub fn string_span(program: &SourceProgram) -> Option<Span> {
    string_uses(program).values().next().copied()
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
            "Q065",
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
                            "Q065",
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
                return Err(diagnostic("Q065", span, "this `effects` row is not closed"));
            }
        }
    }
    Ok(EffectRow::new(effects))
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

/// Lower one `let` statement.
fn lower_let(
    window: &[&Token],
    span: Span,
    literals: &mut Literals,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<StringEvent> {
    let Some((binding, ascribed, equal)) = let_head(window) else {
        diagnostics.push(diagnostic("Q065", span, "this `let` binds no name"));
        return None;
    };
    // The one place `string` is spelled, and the one place the spelling is
    // checked. The type name is *supplied by the module*: it is reserved
    // nowhere, shadows nothing, and is admitted only here.
    if let Some(ascribed) = ascribed
        && ascribed != STRING_TYPE
    {
        diagnostics.push(diagnostic(
            "Q065",
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
            "Q065",
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
                "Q065",
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
                diagnostics.push(diagnostic("Q067", span, refusal.to_string()));
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

    // `let NAME = len ( NAME ) ;`
    if let Some(head) = word(window, value)
        && is_length_query(head, window.get(value + 1).map(|token| &token.kind))
    {
        let Some(name) = word(window, value + 2) else {
            diagnostics.push(diagnostic(
                "Q065",
                span,
                "`len(NAME)` names the string whose byte length it binds",
            ));
            return None;
        };
        if !symbol(window, value + 3, Symbol::RightParen) || !ends_with_semicolon(window, value + 4)
        {
            diagnostics.push(diagnostic("Q065", span, "this `len(…)` is not closed"));
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
                "Q065",
                span,
                "a slice bounds with one integer literal or one bound integer name",
            ));
            return None;
        };
        if !symbol(window, next, Symbol::Colon) {
            diagnostics.push(diagnostic(
                "Q065",
                span,
                "`source.string.v1` slices with `NAME[LOW:HIGH]`; a single-index access into a \
                 string is not a construct this route models, because a byte of UTF-8 text is not \
                 a character",
            ));
            return None;
        }
        let Some((high, next)) = operand(window, next + 1) else {
            diagnostics.push(diagnostic(
                "Q065",
                span,
                "a slice bounds with one integer literal or one bound integer name",
            ));
            return None;
        };
        if !symbol(window, next, Symbol::RightBracket) || !ends_with_semicolon(window, next + 1) {
            diagnostics.push(diagnostic("Q065", span, "this slice is not closed"));
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
                "Q065",
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
    _typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredStringModule, Vec<SourceDiagnostic>> {
    let tokens = parsed.lexical_tokens();
    let mut diagnostics = Vec::new();

    let Some(declaration) = parsed.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
    }) else {
        return Err(vec![diagnostic(
            "Q065",
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
                    "Q065",
                    *span,
                    format!(
                        "`source.string.v1` models the entry only; `{}` is not the entry `{entry}`",
                        other.value.name
                    ),
                ));
            }
        }
    }

    if !declaration.value.parameters.is_empty() {
        diagnostics.push(diagnostic(
            "Q065",
            scope_span,
            "`source.string.v1` models an entry with no parameters",
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
        diagnostics.push(diagnostic("Q065", scope_span, "this entry has no body"));
        return Err(diagnostics);
    };

    let mut literals = Literals {
        payloads: Vec::new(),
    };
    let mut events = Vec::new();
    let mut event_spans = Vec::new();

    for statement in &body.statements {
        let span = statement.span;
        if !statement.value.nested_blocks().is_empty() {
            diagnostics.push(diagnostic(
                "Q065",
                span,
                "`source.string.v1` models one flat statement sequence; this statement opens a \
                 nested block",
            ));
            continue;
        }
        let window = window(tokens, span);
        let head = word(&window, 0).unwrap_or_default();
        let event = match head {
            "let" => lower_let(&window, span, &mut literals, &mut diagnostics),
            // `log_level LEVEL;` is deliberately **absent** from this list, so
            // it reaches the `other` arm below and refuses at `Q065` — the same
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
                // The line grammar is `source.std-io.v1`'s, read by its own
                // reader, so a shape one route accepts and another refuses is
                // not expressible.
                match crate::source_std_io::std_io_statement(parsed, span) {
                    Some(Ok(event)) => Some(StringEvent::Line(event)),
                    Some(Err(message)) => {
                        diagnostics.push(diagnostic("Q065", span, message));
                        None
                    }
                    None => {
                        diagnostics.push(diagnostic(
                            "Q065",
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
                        "Q065",
                        span,
                        "`source.string.v1` models `return;` from a `unit` entry",
                    ));
                    None
                }
            }
            other => {
                diagnostics.push(diagnostic(
                    "Q065",
                    span,
                    format!("`{other}` is not a statement `source.string.v1` models"),
                ));
                None
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

    Ok(LoweredStringModule {
        entry: entry.to_owned(),
        program: StringProgram {
            entry: entry.to_owned(),
            declared_effects,
            literals: literals.payloads,
            events,
        },
        event_spans,
        scope_span,
    })
}
