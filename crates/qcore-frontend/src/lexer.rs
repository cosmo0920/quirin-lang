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

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Word(String),
    Integer(String),
    Float(String),
    StringLiteral(String),
    CharLiteral(String),
    Symbol(Symbol),
    Eof,
}

/// The maps-to token `↦` of a contract state clause (card
/// `OP-CONSTRUCTOR-DECLARATIONS`, USER DECISION 2026-10-14). `->` was rejected
/// (C member access in contract text, and the `.qr` return-type arrow), `>->`
/// was rejected (Z's injective function `↣`); `|->` is the standard ASCII of
/// `↦` (Z, Isabelle, TLA+). Every reader, diagnostic and test names this
/// constant, so the spelling is one edit.
pub const MAPS_TO: &str = "|->";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symbol {
    Colon,
    Semicolon,
    Comma,
    Dot,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    /// `&`, the **bitwise-and** operator (card `OR-AND-LOWERING` reserved it,
    /// card `BITWISE-CONDITIONS` spelled it). It is not logical and — that is
    /// [`Self::AndAnd`] — and it is consumed in exactly one position, a
    /// parenthesized comparison operand inside an `if` condition; everywhere
    /// else it refuses at `Q038`.
    Ampersand,
    /// `^`, the **bitwise-xor** operator, on the same terms as
    /// [`Self::Ampersand`] and binding between it and [`Self::Pipe`]. Card
    /// `OR-AND-LOWERING` promoted it from a lexer error
    /// (`unexpected character '^'`) to a token refusing at `Q038` — the set of
    /// accepted programs unchanged, only the layer and the message moved,
    /// exactly as card `PIPE-OPERATOR` did for a bare `|` — and card
    /// `BITWISE-CONDITIONS` gave it its meaning.
    Caret,
    /// `#`, the **attribute marker** (card `LOOP-UNROLL-BUDGET`).
    ///
    /// A bare `#` was a lexer error (`unexpected character '#'`) before this
    /// card; it becomes a token on the precedent card `OR-AND-LOWERING` set for
    /// `^`, where promoting a lex error to a token moved the layer and the
    /// message without changing the set of accepted programs. The
    /// constraint-14 census found **zero** `#` characters in code position
    /// across every `.qcore`/`.qc` file in the tree, so the promotion collides
    /// with nothing.
    ///
    /// It is read in exactly one place: a `[| #unroll, where #bound < N |]`
    /// clause list immediately preceding a `for` statement. Everywhere else a
    /// `#` reaches a reader that does not model it and is refused there.
    ///
    /// **`#` is still not an identifier character.** The word rule is
    /// unchanged, so `qcore_core::source_loop::ITERATION_SCOPE_SEPARATOR`'s
    /// premise — that a scoped owner name `c#0` can never collide with a name a
    /// program wrote — survives this token exactly as it stood.
    Hash,
    /// `@`, the **C-name marker** of a contract clause (card
    /// `SUPPLIED-OWNED-RETURN-CODE-HANDOFF`, USER DECISION 2026-10-15):
    /// `@RD_KAFKA_OP_RES_PASS` names a C enumerator that Clang resolves. A
    /// bare `@` was a lexer error (`unexpected character '@'`) before the
    /// card; the constraint-14 census found **zero** `@` characters in code
    /// position across every `.qcore`/`.qc`/`.qr`/`.qv` file in the tree, so
    /// the promotion moves the layer of a refusal and accepts nothing new.
    /// It is read only by a resource v1 hand-off guard and a module's `alias`
    /// declaration; every other reader refuses it as an unexpected symbol.
    /// `@` is not an identifier character: the word rule is unchanged.
    At,
    /// `\`, **set difference** of a symbolic premise formula (card
    /// `SP1-SYMBOLIC-OPENSSL-PREMISES`, USER DECISION 2026-11-20: "Add '\'").
    /// A bare backslash was a lexer error (`unexpected character '\\'`)
    /// before the card; the census found **zero** backslashes in code
    /// position across the 781 tracked `.qcore`/`.qc`/`.qr`/`.qv`/`.qcorelib`
    /// files (a backslash inside a string or character literal is that
    /// literal's escape and never reaches this rule), so the promotion moves
    /// the layer of a refusal and accepts nothing new. One byte, no two-byte
    /// spelling, so no row of the longest-match table moves: `\|` is this
    /// token then `Pipe`, and `|->` stays one `MapsTo`. It is read only inside
    /// a `resource_premise(NAME, FORMULA)` formula
    /// (`crate::premise_formula`); every other reader refuses it as an
    /// unexpected symbol.
    Backslash,
    Question,
    Less,
    Greater,
    Equal,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    /// `[|`, the readable opener aliasing `<` on quantitative index types.
    QuantOpen,
    /// `|]`, the readable closer aliasing `>` on quantitative index types.
    QuantClose,
    /// `|>`, the pipe operator (card `PIPE-OPERATOR`). Desugared away before
    /// parsing by [`crate::pipe::desugar_pipes`]; no later phase sees it in a
    /// well-formed program.
    PipeInto,
    /// `|`, the **bitwise-or** operator (card `OR-AND-LOWERING` reserved it,
    /// card `BITWISE-CONDITIONS` spelled it). Loosest of the three bitwise
    /// levels, exactly as in C.
    ///
    /// Card `PIPE-OPERATOR` had claimed the single bar as *logical or*. The
    /// user's retable amendment inverts that: logical or is the double
    /// [`Self::PipePipe`], and the single characters `|`, `&` and `^` are freed
    /// for binary/bitwise use. A bare `|` was a lex error before
    /// `PIPE-OPERATOR` and has been a `Q038` refusal since; it stays a `Q038`
    /// refusal here, with the message redirecting to `||` instead of claiming
    /// the token.
    Pipe,
    /// `||`, the **logical-or** operator (card `OR-AND-LOWERING`). Atomic by
    /// longest match — it was two [`Self::Pipe`] tokens under card
    /// `PIPE-OPERATOR`. Admitted in an `if` condition, where it lowers onto
    /// `source.branch.v1` as a right-nested branch; anywhere else it refuses at
    /// `Q038`.
    PipePipe,
    /// `&&`, the **logical-and** operator, admitted and lowered on the same
    /// terms as [`Self::PipePipe`] and binding tighter than it. Single `&` is
    /// [`Self::Ampersand`], the bitwise-and operator of a different sort.
    AndAnd,
    Arrow,
    /// [`MAPS_TO`], the ASCII maps-to `↦` of a contract state clause (card
    /// `OP-CONSTRUCTOR-DECLARATIONS`, USER DECISION 2026-10-14: `quant live
    /// #result: TAG.FIELD |-> #absent;`). Three bytes, matched before every
    /// two-byte spelling, so `|>` stays [`Self::PipeInto`] and `||` stays
    /// [`Self::PipePipe`]. Before the card the same bytes were a [`Self::Pipe`]
    /// followed by an [`Self::Arrow`], which no reader admitted; only a
    /// contract state clause reads it, and every other reader refuses it as
    /// an unexpected symbol.
    MapsTo,
    /// Contextual premise-flow logical alternative `||->`.
    FlowOr,
    /// Contextual premise-flow forbidden condition `!->`.
    FlowExcept,
    /// Contextual premise-flow terminal condition `->|`.
    FlowTerminal,
    /// Contextual symbolic creation/introduction `|^+`.
    LifecycleCreate,
    /// Contextual symbolic destruction/retirement `|^-`.
    LifecycleRetire,
    FatArrow,
    ColonColon,
    Define,
    EqualEqual,
    NotEqual,
    LessEqual,
    GreaterEqual,
    Apostrophe,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexError {
    pub span: Span,
    pub message: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for LexError {}

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer { source, offset: 0 }.run()
}

/// Whether `tokens` are in source order: starts and ends both non-decreasing.
/// A lexed stream always is; a stream `crate::pipe::desugar_pipes` rewrote
/// may not be, because a desugared pipeline keeps each written token's own
/// span while moving it.
#[must_use]
pub fn tokens_in_source_order(tokens: &[Token]) -> bool {
    tokens.windows(2).all(|pair| {
        pair[0].span.start <= pair[1].span.start && pair[0].span.end <= pair[1].span.end
    })
}

/// An index range of `tokens` that holds every token whose span lies inside
/// `span` (`span.start <= token.span.start && token.span.end <= span.end`).
///
/// `in_source_order` must be [`tokens_in_source_order`] of the same stream.
/// When it holds, the range is found by binary search and is exactly those
/// tokens; when it does not, the range is the whole stream. Either way it is a
/// superset, so a caller keeps its own span test over the range and reads the
/// same tokens, in the same order, as a scan of the whole stream.
#[must_use]
pub fn span_candidates(
    tokens: &[Token],
    in_source_order: bool,
    span: Span,
) -> std::ops::Range<usize> {
    if !in_source_order {
        return 0..tokens.len();
    }
    let low = tokens.partition_point(|token| token.span.start < span.start);
    let high = tokens.partition_point(|token| token.span.end <= span.end);
    low..high.max(low)
}

/// A token stream together with whether it is in source order, so a reader
/// handed the stream can still narrow a span lookup ([`Self::near`]) without
/// re-deriving the order. It reads as the slice it wraps.
#[derive(Clone, Copy, Debug)]
pub struct TokenStream<'a> {
    tokens: &'a [Token],
    in_source_order: bool,
}

impl<'a> TokenStream<'a> {
    /// `in_source_order` must be [`tokens_in_source_order`] of `tokens`.
    #[must_use]
    pub fn new(tokens: &'a [Token], in_source_order: bool) -> Self {
        Self {
            tokens,
            in_source_order,
        }
    }

    /// The whole stream.
    #[must_use]
    pub fn all(self) -> &'a [Token] {
        self.tokens
    }

    /// A slice holding every token whose span lies inside `span`
    /// ([`span_candidates`]); a caller keeps its own span test.
    #[must_use]
    pub fn near(self, span: Span) -> &'a [Token] {
        &self.tokens[span_candidates(self.tokens, self.in_source_order, span)]
    }

    pub fn iter(self) -> std::slice::Iter<'a, Token> {
        self.tokens.iter()
    }

    #[must_use]
    pub fn get(self, index: usize) -> Option<&'a Token> {
        self.tokens.get(index)
    }
}

impl std::ops::Deref for TokenStream<'_> {
    type Target = [Token];

    fn deref(&self) -> &[Token] {
        self.tokens
    }
}

struct Lexer<'a> {
    source: &'a str,
    offset: usize,
}

impl Lexer<'_> {
    fn run(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        while self.offset < self.source.len() {
            self.skip_trivia()?;
            if self.offset == self.source.len() {
                break;
            }
            tokens.push(self.next_token()?);
        }
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span {
                start: self.offset,
                end: self.offset,
            },
        });
        Ok(tokens)
    }

    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.bump();
            }
            if self.rest().starts_with("//") {
                while self.peek().is_some_and(|ch| ch != '\n') {
                    self.bump();
                }
                continue;
            }
            if self.rest().starts_with("/*") {
                let start = self.offset;
                self.offset += 2;
                while self.offset < self.source.len() && !self.rest().starts_with("*/") {
                    self.bump();
                }
                if self.offset == self.source.len() {
                    return Err(self.error(start, "unterminated block comment"));
                }
                self.offset += 2;
                continue;
            }
            return Ok(());
        }
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        let start = self.offset;
        let ch = self.bump().expect("caller checks end of input");
        let kind = if ch == '_' || ch.is_ascii_alphabetic() {
            while self
                .peek()
                .is_some_and(|next| next == '_' || next.is_ascii_alphanumeric())
            {
                self.bump();
            }
            TokenKind::Word(self.source[start..self.offset].to_owned())
        } else if ch.is_ascii_digit() {
            while self.peek().is_some_and(|next| next.is_ascii_digit()) {
                self.bump();
            }
            let mut float = false;
            if self.peek() == Some('.')
                && self.peek_second().is_some_and(|next| next.is_ascii_digit())
            {
                self.bump();
                while self.peek().is_some_and(|next| next.is_ascii_digit()) {
                    self.bump();
                }
                float = true;
            }
            // Card `W48-QR-FLOATING-POINT`: an exponent `(e|E) [+|-] DIGITS`
            // written directly after the digits is part of the literal
            // (`1e5`, `1.5e-3`). It is taken only when a digit follows the
            // `e` and its optional sign, so `1.5e`, `1e+` and `1 e5` lex as
            // before; the census found no tracked file spelling one.
            if matches!(self.peek(), Some('e' | 'E')) {
                let rest = &self.source[self.offset + 1..];
                let sign = usize::from(rest.starts_with(['+', '-']));
                if rest[sign..].starts_with(|c: char| c.is_ascii_digit()) {
                    self.offset += 1 + sign;
                    while self.peek().is_some_and(|next| next.is_ascii_digit()) {
                        self.bump();
                    }
                    float = true;
                }
            }
            if float {
                TokenKind::Float(self.source[start..self.offset].to_owned())
            } else {
                TokenKind::Integer(self.source[start..self.offset].to_owned())
            }
        } else if ch == '"' {
            self.quoted(start, '"', TokenKind::StringLiteral)?
        } else if ch == '\'' && self.starts_char_literal() {
            self.quoted(start, '\'', TokenKind::CharLiteral)?
        } else {
            TokenKind::Symbol(self.symbol(start, ch)?)
        };
        Ok(Token {
            kind,
            span: Span {
                start,
                end: self.offset,
            },
        })
    }

    fn quoted(
        &mut self,
        start: usize,
        delimiter: char,
        constructor: fn(String) -> TokenKind,
    ) -> Result<TokenKind, LexError> {
        let content_start = self.offset;
        let mut escaped = false;
        while let Some(ch) = self.bump() {
            if !escaped && ch == delimiter {
                let content_end = self.offset - ch.len_utf8();
                return Ok(constructor(
                    self.source[content_start..content_end].to_owned(),
                ));
            }
            escaped = !escaped && ch == '\\';
        }
        Err(self.error(start, "unterminated literal"))
    }

    /// Two-byte spellings are tried before every one-byte spelling, so the
    /// tokenization is longest-match at a fixed width of two. The four members
    /// of the vertical-bar family are pairwise distinct byte pairs, so their
    /// order inside this table is immaterial and the resolution is decided by
    /// width alone:
    ///
    /// | Input | Lexes as | Note |
    /// |---|---|---|
    /// | `[\|` | `QuantOpen` | matched at `[`, so `[\|>` is `QuantOpen` then `Greater` |
    /// | `\|]` | `QuantClose` | |
    /// | `\|>` | `PipeInto` | the pipe wins over `Pipe` then `Greater` by width |
    /// | `\|\|` | `PipePipe` | **logical or** — atomic, one token (card `OR-AND-LOWERING`) |
    /// | `\|` | `Pipe` | **bitwise or** — admitted as a grouped comparison operand in a condition; `Q038` elsewhere |
    /// | `&&` | `AndAnd` | **logical and** — binds tighter than `\|\|` |
    /// | `&` | `Ampersand` | **bitwise and** — same position, tightest of the three |
    /// | `^` | `Caret` | **bitwise xor** — same position, between `&` and `\|` |
    ///
    /// The five members of the vertical-bar family are pairwise distinct byte
    /// pairs (or a single byte), so the width-two rule decides every one of
    /// them and the order inside the table below is immaterial.
    ///
    /// The one genuine ambiguity in the family is `a |> b` against a
    /// hypothetical `a | > b`: `|>` wins by longest match, so a bitwise or
    /// followed by a `>` comparison has to be spaced. Recorded by card
    /// `PIPE-OPERATOR`, inherited by `BITWISE-CONDITIONS` and measured there:
    /// the adjacency can never produce a wrong *acceptance*, only a refusal,
    /// because `>` has no left operand inside an integer expression. `(1 | 2)
    /// > 0` is the spaced spelling and it parses; `1 |> 2` is a pipeline in a
    /// condition and refuses at `Q045`.
    fn symbol(&mut self, start: usize, first: char) -> Result<Symbol, LexError> {
        // Longest-match additions. Existing MapsTo keeps its exact identity.
        for (spelling, symbol) in [
            ("||->", Symbol::FlowOr),
            ("|^+", Symbol::LifecycleCreate),
            ("|^-", Symbol::LifecycleRetire),
            ("!->", Symbol::FlowExcept),
            ("->|", Symbol::FlowTerminal),
        ] {
            if self
                .source
                .get(start..)
                .is_some_and(|rest| rest.starts_with(spelling))
            {
                for _ in 1..spelling.len() {
                    self.bump();
                }
                return Ok(symbol);
            }
        }
        // Card OP-CONSTRUCTOR-DECLARATIONS: the one three-byte spelling,
        // tried first, so the width-two rule below is unchanged for `|>`,
        // `||` and `|]`.
        if self
            .source
            .get(start..)
            .is_some_and(|rest| rest.starts_with(MAPS_TO))
        {
            for _ in 1..MAPS_TO.len() {
                self.bump();
            }
            return Ok(Symbol::MapsTo);
        }
        let pair = self
            .source
            .get(start..self.offset + self.peek().map_or(0, char::len_utf8));
        let two = match pair {
            Some("->") => Some(Symbol::Arrow),
            Some("=>") => Some(Symbol::FatArrow),
            Some("::") => Some(Symbol::ColonColon),
            Some(":=") => Some(Symbol::Define),
            Some("==") => Some(Symbol::EqualEqual),
            Some("!=") => Some(Symbol::NotEqual),
            Some("<=") => Some(Symbol::LessEqual),
            Some(">=") => Some(Symbol::GreaterEqual),
            Some("[|") => Some(Symbol::QuantOpen),
            Some("|]") => Some(Symbol::QuantClose),
            Some("|>") => Some(Symbol::PipeInto),
            Some("||") => Some(Symbol::PipePipe),
            Some("&&") => Some(Symbol::AndAnd),
            _ => None,
        };
        if let Some(symbol) = two {
            self.bump();
            return Ok(symbol);
        }
        match first {
            ':' => Ok(Symbol::Colon),
            ';' => Ok(Symbol::Semicolon),
            ',' => Ok(Symbol::Comma),
            '.' => Ok(Symbol::Dot),
            '+' => Ok(Symbol::Plus),
            '-' => Ok(Symbol::Minus),
            '*' => Ok(Symbol::Star),
            '/' => Ok(Symbol::Slash),
            '%' => Ok(Symbol::Percent),
            '!' => Ok(Symbol::Bang),
            // The three bitwise operators: reserved by card
            // `OR-AND-LOWERING`, spelled by card `BITWISE-CONDITIONS`. Each is
            // consumed only by `source.branch.v1`'s integer tier, inside an
            // `if` condition; the operator gate refuses every other position.
            '&' => Ok(Symbol::Ampersand),
            '|' => Ok(Symbol::Pipe),
            '^' => Ok(Symbol::Caret),
            // Card `LOOP-UNROLL-BUDGET`: the attribute marker. One byte, no
            // two-byte spelling, so no row of the longest-match table above
            // moves — `[|`, `|]`, `|>`, `||`, `&&` and every one-byte member
            // lex exactly as they did.
            '#' => Ok(Symbol::Hash),
            // Card `SUPPLIED-OWNED-RETURN-CODE-HANDOFF`: the C-name marker.
            // One byte, no two-byte spelling: no row above moves.
            '@' => Ok(Symbol::At),
            // Card `SP1-SYMBOLIC-OPENSSL-PREMISES`: set difference in a
            // premise formula. One byte, no two-byte spelling.
            '\\' => Ok(Symbol::Backslash),
            '?' => Ok(Symbol::Question),
            '<' => Ok(Symbol::Less),
            '>' => Ok(Symbol::Greater),
            '=' => Ok(Symbol::Equal),
            '(' => Ok(Symbol::LeftParen),
            ')' => Ok(Symbol::RightParen),
            '{' => Ok(Symbol::LeftBrace),
            '}' => Ok(Symbol::RightBrace),
            '[' => Ok(Symbol::LeftBracket),
            ']' => Ok(Symbol::RightBracket),
            '\'' => Ok(Symbol::Apostrophe),
            _ => Err(self.error(start, format!("unexpected character {first:?}"))),
        }
    }

    fn rest(&self) -> &str {
        &self.source[self.offset..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_second(&self) -> Option<char> {
        self.rest().chars().nth(1)
    }

    fn starts_char_literal(&self) -> bool {
        let mut chars = self.rest().chars();
        match chars.next() {
            Some('\\') => chars.next().is_some() && chars.next() == Some('\''),
            Some('\n' | '\r' | '\'') | None => false,
            Some(_) => chars.next() == Some('\''),
        }
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }

    fn error(&self, start: usize, message: impl Into<String>) -> LexError {
        LexError {
            span: Span {
                start,
                end: self.offset,
            },
            message: message.into(),
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/lexer.rs"]
mod tests;
