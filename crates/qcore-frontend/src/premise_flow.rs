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

//! Fully parenthesized, contextual premise flow source syntax.
use crate::lexer::{Symbol, TokenKind, lex};
/// Frontend-owned syntax only; wire/schema conversion belongs to contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Atom { atom: String },
    Flow { flow: String },
    And { left: Box<Expr>, right: Box<Expr> },
    Or { left: Box<Expr>, right: Box<Expr> },
    Except { left: Box<Expr>, right: Box<Expr> },
    Terminal { left: Box<Expr>, right: Box<Expr> },
}
pub fn identifier(s: &str) -> bool {
    s.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
pub fn is_clause(source: &str) -> bool {
    source
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()
        .is_some_and(|head| head.starts_with("resource_premise_flow"))
}
struct Reader {
    tokens: Vec<TokenKind>,
    at: usize,
    nodes: usize,
}
impl Reader {
    fn need(&mut self, token: TokenKind) -> Result<(), String> {
        if self.tokens.get(self.at) != Some(&token) {
            return Err("flow: expected exact delimiter/operator".into());
        }
        self.at += 1;
        Ok(())
    }
    fn word(&mut self) -> Result<String, String> {
        let Some(TokenKind::Word(word)) = self.tokens.get(self.at) else {
            return Err("flow: expected identifier".into());
        };
        if !identifier(word) {
            return Err("flow: malformed identifier".into());
        }
        self.at += 1;
        Ok(word.clone())
    }
    fn expr(&mut self, depth: usize) -> Result<Expr, String> {
        self.nodes += 1;
        if depth > 128 || self.nodes > 4096 {
            return Err("flow: expression limit".into());
        }
        if self.tokens.get(self.at) == Some(&TokenKind::Symbol(Symbol::LeftParen)) {
            self.at += 1;
            let left = Box::new(self.expr(depth + 1)?);
            let op = self.tokens.get(self.at).cloned();
            self.at += 1;
            let right = Box::new(self.expr(depth + 1)?);
            self.need(TokenKind::Symbol(Symbol::RightParen))?;
            match op {
                Some(TokenKind::Symbol(Symbol::MapsTo)) => Ok(Expr::And { left, right }),
                Some(TokenKind::Symbol(Symbol::FlowOr)) => Ok(Expr::Or { left, right }),
                Some(TokenKind::Symbol(Symbol::FlowExcept)) => Ok(Expr::Except { left, right }),
                Some(TokenKind::Symbol(Symbol::FlowTerminal)) => Ok(Expr::Terminal { left, right }),
                _ => Err(
                    "flow: missing/unknown binary operator; parenthesize every binary expression"
                        .into(),
                ),
            }
        } else {
            let word = self.word()?;
            if word == "flow"
                && self.tokens.get(self.at) == Some(&TokenKind::Symbol(Symbol::LeftParen))
            {
                self.at += 1;
                let flow = self.word()?;
                self.need(TokenKind::Symbol(Symbol::RightParen))?;
                Ok(Expr::Flow { flow })
            } else {
                Ok(Expr::Atom { atom: word })
            }
        }
    }
}
pub fn parse_clause(source: &str) -> Result<Option<(String, Expr)>, String> {
    if !is_clause(source) {
        return Ok(None);
    }
    let tokens = lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| t.kind)
        .collect::<Vec<_>>();
    if tokens.len() > 16384 {
        return Err("flow: token limit".into());
    }
    let mut r = Reader {
        tokens,
        at: 0,
        nodes: 0,
    };
    if r.word()? != "resource_premise_flow_v1" {
        return Err("flow: unsupported source version".into());
    }
    r.need(TokenKind::Symbol(Symbol::LeftParen))?;
    let name = r.word()?;
    r.need(TokenKind::Symbol(Symbol::Comma))?;
    let expr = r.expr(0)?;
    r.need(TokenKind::Symbol(Symbol::RightParen))?;
    r.need(TokenKind::Eof)?;
    if r.at != r.tokens.len() {
        return Err("flow: trailing tokens".into());
    }
    Ok(Some((name, expr)))
}
