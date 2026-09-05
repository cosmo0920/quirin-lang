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
            '&' => Ok(Symbol::Ampersand),
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
mod tests {
    use super::*;

    #[test]
    fn tokenizes_literals_comments_and_compound_symbols() {
        let tokens = lex("foo 12 3.5 \"x\\\"y\" 'z' // c\n := :: -> => == != <= >=").unwrap();
        let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Word("foo".into()),
                TokenKind::Integer("12".into()),
                TokenKind::Float("3.5".into()),
                TokenKind::StringLiteral("x\\\"y".into()),
                TokenKind::CharLiteral("z".into()),
                TokenKind::Symbol(Symbol::Define),
                TokenKind::Symbol(Symbol::ColonColon),
                TokenKind::Symbol(Symbol::Arrow),
                TokenKind::Symbol(Symbol::FatArrow),
                TokenKind::Symbol(Symbol::EqualEqual),
                TokenKind::Symbol(Symbol::NotEqual),
                TokenKind::Symbol(Symbol::LessEqual),
                TokenKind::Symbol(Symbol::GreaterEqual),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn rejects_unterminated_comment() {
        let error = lex("/* no end").unwrap_err();
        assert_eq!(error.message, "unterminated block comment");
    }

    #[test]
    fn distinguishes_lifetimes_from_character_literals() {
        let tokens = lex("'scope 'x' 'static").unwrap();
        let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Symbol(Symbol::Apostrophe),
                TokenKind::Word("scope".into()),
                TokenKind::CharLiteral("x".into()),
                TokenKind::Symbol(Symbol::Apostrophe),
                TokenKind::Word("static".into()),
                TokenKind::Eof,
            ]
        );
    }
}
