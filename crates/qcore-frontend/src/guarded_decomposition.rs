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

//! Standalone versioned readable syntax, deliberately separate from flow-v1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Clause {
    Enable {
        environment: String,
        prerequisites: Vec<String>,
        operation: String,
    },
    Forbid {
        condition: String,
        operation: String,
    },
    Alternatives {
        operation: String,
        cases: Vec<String>,
    },
    Terminal {
        case: String,
        postcondition: String,
    },
}

fn identifier(s: &str) -> Result<String, String> {
    if s.len() > 128 || !crate::premise_flow::identifier(s) {
        Err("guarded-decomposition: malformed or oversized symbol".into())
    } else {
        Ok(s.into())
    }
}

/// Parse a whole bounded document, never a partial prefix or an ordinary source
/// expression. Atomic operator bytes cannot be split by whitespace.
pub fn parse(source: &str) -> Result<Vec<Clause>, String> {
    if source.len() > 65536 {
        return Err("guarded-decomposition: document byte limit".into());
    }
    let mut statements = source.split(';');
    if statements.next().map(str::trim) != Some("guarded_decomposition_v1") {
        return Err("guarded-decomposition: expected exact version1 header".into());
    }
    let mut clauses = Vec::new();
    let mut tail_seen = false;
    for statement in statements {
        let s = statement.trim();
        if s.is_empty() {
            if tail_seen {
                return Err("guarded-decomposition: empty statement".into());
            }
            tail_seen = true;
            continue;
        }
        if tail_seen {
            return Err("guarded-decomposition: empty statement".into());
        }
        if s.len() > 4096 {
            return Err("guarded-decomposition: statement byte limit".into());
        }
        let choices = ["|->", "!->", "||->", "->|"];
        let hits: Vec<_> = choices
            .iter()
            .filter_map(|op| {
                // Check the longest operator first, before its |-> suffix.
                s.find(op).map(|at| (*op, at))
            })
            .collect();
        let (op, at) = if let Some(at) = s.find("||->") {
            ("||->", at)
        } else if hits.len() == 1 {
            hits[0]
        } else {
            return Err("guarded-decomposition: expected one supported operator".into());
        };
        let left = s[..at].trim();
        let right = s[at + op.len()..].trim();
        let clause = match op {
            "|->" => {
                let Some(inner) = left.strip_prefix('(').and_then(|s| s.strip_suffix(')')) else {
                    return Err("guarded-decomposition: parenthesized environment and prerequisites required".into());
                };
                let terms = inner
                    .split('∧')
                    .map(str::trim)
                    .map(identifier)
                    .collect::<Result<Vec<_>, _>>()?;
                if !(2..=33).contains(&terms.len()) {
                    return Err(
                        "guarded-decomposition: one environment and 1..32 prerequisites required"
                            .into(),
                    );
                }
                Clause::Enable {
                    environment: terms[0].clone(),
                    prerequisites: terms[1..].to_vec(),
                    operation: identifier(right)?,
                }
            }
            "!->" => Clause::Forbid {
                condition: identifier(left)?,
                operation: identifier(right)?,
            },
            "||->" => {
                let cases = right
                    .split('/')
                    .map(str::trim)
                    .map(identifier)
                    .collect::<Result<Vec<_>, _>>()?;
                if !(2..=32).contains(&cases.len()) {
                    return Err(
                        "guarded-decomposition: 2..32 explicit alternatives required".into(),
                    );
                }
                Clause::Alternatives {
                    operation: identifier(left)?,
                    cases,
                }
            }
            "->|" => Clause::Terminal {
                case: identifier(left)?,
                postcondition: identifier(right)?,
            },
            _ => unreachable!(),
        };
        clauses.push(clause);
        if clauses.len() > 256 {
            return Err("guarded-decomposition: clause limit".into());
        }
    }
    if !tail_seen || clauses.is_empty() {
        return Err(
            "guarded-decomposition: nonempty document with final semicolon required".into(),
        );
    }
    Ok(clauses)
}

/// Explicit source embedding; these heads never share flow-v1's semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceClause {
    Symbol {
        id: String,
        role: String,
        proposition: String,
        dependencies: Vec<String>,
    },
    Document {
        id: String,
        readable: String,
    },
}
pub fn is_source_clause(source: &str) -> bool {
    source
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()
        .is_some_and(|s| s.starts_with("resource_guarded_"))
}
pub fn parse_source_clause(source: &str) -> Result<Option<SourceClause>, String> {
    use crate::lexer::{Symbol, TokenKind, lex};
    if !is_source_clause(source) {
        return Ok(None);
    }
    if source.len() > 131072 {
        return Err("guarded source: byte limit".into());
    }
    let tokens = lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| t.kind)
        .collect::<Vec<_>>();
    let word = |t: &TokenKind| match t {
        TokenKind::Word(s) => identifier(s),
        _ => Err("guarded source: identifier required".into()),
    };
    let string = |t: &TokenKind| -> Result<String, String> {
        match t {
            TokenKind::StringLiteral(s) if !s.contains('\\') => Ok(s.clone()),
            _ => Err("guarded source: literal unescaped string required".into()),
        }
    };
    let bad = || "guarded source: exact versioned clause required".to_string();
    if tokens.len() < 7
        || tokens[1] != TokenKind::Symbol(Symbol::LeftParen)
        || tokens[3] != TokenKind::Symbol(Symbol::Comma)
        || tokens[tokens.len() - 2] != TokenKind::Symbol(Symbol::RightParen)
        || tokens.last() != Some(&TokenKind::Eof)
    {
        return Err(bad());
    }
    let id = word(&tokens[2])?;
    match tokens.first() {
        Some(TokenKind::Word(h))
            if h == "resource_guarded_decomposition_v1" && tokens.len() == 7 =>
        {
            let readable = string(&tokens[4])?;
            parse(&readable)?;
            Ok(Some(SourceClause::Document { id, readable }))
        }
        Some(TokenKind::Word(h)) if h == "resource_guarded_symbol_v1" && tokens.len() >= 9 => {
            let role = word(&tokens[4])?;
            if tokens[5] != TokenKind::Symbol(Symbol::Comma) {
                return Err(bad());
            }
            let proposition = string(&tokens[6])?;
            let mut dependencies = Vec::new();
            let mut at = 7;
            while at < tokens.len() - 2 {
                if tokens[at] != TokenKind::Symbol(Symbol::Comma) {
                    return Err(bad());
                }
                dependencies.push(word(tokens.get(at + 1).ok_or_else(bad)?)?);
                at += 2;
            }
            if at != tokens.len() - 2 {
                return Err(bad());
            }
            Ok(Some(SourceClause::Symbol {
                id,
                role,
                proposition,
                dependencies,
            }))
        }
        _ => Err(bad()),
    }
}
