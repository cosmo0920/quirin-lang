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

use crate::lexer::{LexError, Span, Symbol, Token, TokenKind, lex};
use qcore_core::ast::{AxisDecl, AxisMode, Declaration, DimensionDecl, DimensionRef, Program};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub span: Span,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(error: LexError) -> Self {
        Self {
            span: error.span,
            message: error.message,
        }
    }
}

pub fn parse_program(source: &str) -> Result<Program, ParseError> {
    Parser {
        tokens: lex(source)?,
        cursor: 0,
    }
    .program()
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn program(mut self) -> Result<Program, ParseError> {
        let mut declarations = Vec::new();
        while !matches!(self.current().kind, TokenKind::Eof) {
            declarations.push(self.declaration()?);
        }
        Ok(Program { declarations })
    }

    fn declaration(&mut self) -> Result<Declaration, ParseError> {
        if self.at_word("dimension") {
            self.advance();
            return self.dimension().map(Declaration::Dimension);
        }
        if self.at_word("quant") {
            self.advance();
            self.expect_word("axis")?;
            return self.axis().map(Declaration::Axis);
        }
        Err(self.error("expected `dimension` or `quant axis` declaration"))
    }

    fn dimension(&mut self) -> Result<DimensionDecl, ParseError> {
        let name = self.identifier()?;
        self.expect_symbol(Symbol::Colon)?;
        self.expect_word("Idx")?;
        self.expect_symbol(Symbol::Plus)?;
        self.expect_word("Ordered")?;
        self.expect_symbol(Symbol::Semicolon)?;
        Ok(DimensionDecl { name })
    }

    fn axis(&mut self) -> Result<AxisDecl, ParseError> {
        let name = self.identifier()?;
        self.expect_symbol(Symbol::Colon)?;
        let mode = self.axis_mode()?;
        self.expect_symbol(Symbol::Less)?;
        let dimension = self.dimension_ref()?;
        self.expect_symbol(Symbol::Greater)?;
        self.expect_symbol(Symbol::Semicolon)?;
        Ok(AxisDecl {
            name,
            mode,
            dimension,
        })
    }

    fn axis_mode(&mut self) -> Result<AxisMode, ParseError> {
        if self.at_word("credit") {
            self.advance();
            Ok(AxisMode::Credit)
        } else if self.at_word("live") {
            self.advance();
            Ok(AxisMode::Live)
        } else if self.at_word("cost") {
            self.advance();
            self.expect_symbol(Symbol::Dot)?;
            if self.at_word("total") {
                self.advance();
                Ok(AxisMode::CostTotal)
            } else if self.at_word("peak") {
                self.advance();
                Ok(AxisMode::CostPeak)
            } else {
                Err(self.error("expected `total` or `peak` after `cost.`"))
            }
        } else {
            Err(self.error("expected axis mode"))
        }
    }

    fn dimension_ref(&mut self) -> Result<DimensionRef, ParseError> {
        if self.at_word("Count") {
            self.advance();
            self.expect_symbol(Symbol::Less)?;
            let resource = self.identifier()?;
            self.expect_symbol(Symbol::Greater)?;
            Ok(DimensionRef::Count(resource))
        } else {
            self.identifier().map(DimensionRef::Named)
        }
    }

    fn identifier(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::Word(word) => {
                let word = word.clone();
                self.advance();
                Ok(word)
            }
            _ => Err(self.error("expected identifier")),
        }
    }

    fn expect_word(&mut self, expected: &str) -> Result<(), ParseError> {
        if self.at_word(expected) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("expected `{expected}`")))
        }
    }

    fn expect_symbol(&mut self, expected: Symbol) -> Result<(), ParseError> {
        if self.current().kind == TokenKind::Symbol(expected) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("expected {expected:?}")))
        }
    }

    fn at_word(&self, expected: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Word(word) if word == expected)
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn advance(&mut self) {
        if self.cursor + 1 < self.tokens.len() {
            self.cursor += 1;
        }
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            span: self.current().span,
            message: message.into(),
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/parser.rs"]
mod tests;
