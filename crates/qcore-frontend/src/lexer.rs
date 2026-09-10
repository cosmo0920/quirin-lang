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
    Ampersand,
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
    /// `|`, the logical-or operator. Card `PIPE-OPERATOR` claims the token so a
    /// later wave inherits it without a collision; the short-circuit lowering
    /// is named follow-on `OR-LOWERING` and every occurrence refuses at `Q034`
    /// until it lands. A bare `|` was a lex error before that card; it is still
    /// refused, one layer later and with an actionable message.
    Pipe,
    /// `&&`, the logical-and operator, claimed on the same terms as [`Self::Pipe`]
    /// (follow-on `AND-LOWERING`, refusing at `Q034`). Single `&` stays
    /// [`Self::Ampersand`] and is deliberately left for a future binary/bitwise
    /// operation; this card gives it no new meaning.
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
            if self.peek() == Some('.')
                && self.peek_second().is_some_and(|next| next.is_ascii_digit())
            {
                self.bump();
                while self.peek().is_some_and(|next| next.is_ascii_digit()) {
                    self.bump();
                }
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
    /// | `\|` | `Pipe` | logical or; refused at `Q034` until `OR-LOWERING` lands |
    /// | `\|\|` | `Pipe` `Pipe` | refused at `Q034` naming the single-`\|` spelling |
    /// | `&&` | `AndAnd` | logical and; refused at `Q034` until `AND-LOWERING` |
    /// | `&` | `Ampersand` | reserved for a future binary/bitwise operation |
    ///
    /// The one genuine future ambiguity is `a |> b` against a hypothetical
    /// `a | > b`: `|>` wins by longest match, so an or-operator followed by a
    /// `>` comparison must be spaced or parenthesized. That constraint is
    /// recorded now rather than discovered by the wave that implements `|`.
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
            // Unchanged by card `PIPE-OPERATOR`: nothing in the workspace
            // consumes `Ampersand`, and it is deliberately left that way so a
            // future binary/bitwise wave inherits a clean single `&`.
            '&' => Ok(Symbol::Ampersand),
            '|' => Ok(Symbol::Pipe),
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
