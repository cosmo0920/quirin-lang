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
