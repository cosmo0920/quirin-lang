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
        .is_some_and(|s| s.starts_with(crate::premise_formula::GUARDED_PREFIX))
}

/// The head of a guarded symbol (C04.18.6).
pub const SYMBOL_HEAD: &str = "resource_guarded_symbol_v1";
/// The head of an embedded guarded decomposition document (C04.18.6).
pub const DOCUMENT_HEAD: &str = "resource_guarded_decomposition_v1";

fn clause_text(clause: &Clause, conjunction: &str) -> String {
    match clause {
        Clause::Enable {
            environment,
            prerequisites,
            operation,
        } => {
            let mut terms = vec![environment.as_str()];
            terms.extend(prerequisites.iter().map(String::as_str));
            format!("({}) |-> {operation}", terms.join(conjunction))
        }
        Clause::Forbid {
            condition,
            operation,
        } => format!("{condition} !-> {operation}"),
        Clause::Alternatives { operation, cases } => {
            format!("{operation} ||-> {}", cases.join(" / "))
        }
        Clause::Terminal {
            case,
            postcondition,
        } => format!("{case} ->| {postcondition}"),
    }
}

/// Card SP6-GUARDED-SYMBOL-PROSE: the readable G05-guarded document of a
/// clause list: the version header, then one clause per line, each ending in
/// `;`, the conjunction spelled `∧`. [`parse`] reads it back as the same list.
#[must_use]
pub fn readable(clauses: &[Clause]) -> String {
    let mut out = "guarded_decomposition_v1;\n".to_owned();
    for clause in clauses {
        out.push_str(&clause_text(clause, " ∧ "));
        out.push_str(";\n");
    }
    out
}

/// Card SP6-GUARDED-SYMBOL-PROSE: the source spelling of a clause list, the
/// arguments after the entry ID in `resource_guarded_decomposition_v1(ENTRY,
/// ...)`: the clauses separated by `, `, the conjunction spelled `&&` (so
/// the shared lexer is unchanged). [`parse_source_clause`] renders it back
/// as [`readable`].
#[must_use]
pub fn source_clauses(clauses: &[Clause]) -> String {
    clauses
        .iter()
        .map(|c| clause_text(c, " && "))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The arguments of a clause, split at the commas outside parentheses and
/// braces. An empty argument or unbalanced nesting rejects.
fn arguments(
    tokens: &[crate::lexer::TokenKind],
) -> Result<Vec<&[crate::lexer::TokenKind]>, String> {
    use crate::lexer::{Symbol, TokenKind};
    let bad = || "guarded source: exact versioned clause required".to_string();
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (at, token) in tokens.iter().enumerate() {
        match token {
            TokenKind::Symbol(Symbol::LeftParen | Symbol::LeftBrace) => depth += 1,
            TokenKind::Symbol(Symbol::RightParen | Symbol::RightBrace) => {
                depth = depth.checked_sub(1).ok_or_else(bad)?;
            }
            TokenKind::Symbol(Symbol::Comma) if depth == 0 => {
                out.push(&tokens[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    out.push(&tokens[start..]);
    if depth != 0 || out.iter().any(|a| a.is_empty()) {
        return Err(bad());
    }
    Ok(out)
}

/// One document clause in its source spelling.
fn document_clause(tokens: &[crate::lexer::TokenKind]) -> Result<Clause, String> {
    use crate::lexer::{Symbol, TokenKind as T};
    let bad = || "guarded source: malformed document clause".to_string();
    let word = |t: &T| match t {
        T::Word(s) => identifier(s),
        _ => Err(bad()),
    };
    match tokens {
        [condition, T::Symbol(Symbol::FlowExcept), operation] => Ok(Clause::Forbid {
            condition: word(condition)?,
            operation: word(operation)?,
        }),
        [case, T::Symbol(Symbol::FlowTerminal), postcondition] => Ok(Clause::Terminal {
            case: word(case)?,
            postcondition: word(postcondition)?,
        }),
        [operation, T::Symbol(Symbol::FlowOr), rest @ ..] => {
            let mut cases = vec![word(rest.first().ok_or_else(bad)?)?];
            let mut at = 1;
            while at < rest.len() {
                if rest[at] != T::Symbol(Symbol::Slash) {
                    return Err(bad());
                }
                cases.push(word(rest.get(at + 1).ok_or_else(bad)?)?);
                at += 2;
            }
            Ok(Clause::Alternatives {
                operation: word(operation)?,
                cases,
            })
        }
        [
            T::Symbol(Symbol::LeftParen),
            inner @ ..,
            T::Symbol(Symbol::RightParen),
            T::Symbol(Symbol::MapsTo),
            operation,
        ] => {
            let mut terms = vec![word(inner.first().ok_or_else(bad)?)?];
            let mut at = 1;
            while at < inner.len() {
                if inner[at] != T::Symbol(Symbol::AndAnd) {
                    return Err(bad());
                }
                terms.push(word(inner.get(at + 1).ok_or_else(bad)?)?);
                at += 2;
            }
            if terms.len() < 2 {
                return Err(bad());
            }
            Ok(Clause::Enable {
                environment: terms[0].clone(),
                prerequisites: terms[1..].to_vec(),
                operation: word(operation)?,
            })
        }
        _ => Err(bad()),
    }
}

/// Card SP6-GUARDED-SYMBOL-PROSE (C04.18.6): a guarded source clause, read
/// from its expression (no introducer, no `;`). `Ok(None)` when the clause is
/// not headed `resource_guarded_`.
///
/// * `resource_guarded_symbol_v1(ID, ROLE, FORMULA, DEPENDENCY, ...)`: the
///   proposition is an unquoted C04.18.9 typed-rendering formula
///   (`premise_formula::guarded_formula`), kept as its canonical printing.
/// * `resource_guarded_decomposition_v1(ENTRY, CLAUSE, ...)`: the G05-guarded
///   clauses unquoted, separated by `,`, the conjunction spelled `&&`; the
///   readable document is rendered from them ([`readable`]) and checked by
///   [`parse`].
///
/// A string anywhere in either clause is refused by name
/// (`premise_formula::PROSE_GUARDED`, `QF003`); every other malformation,
/// unknown head or version rejects.
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
        .filter(|t| *t != TokenKind::Eof)
        .collect::<Vec<_>>();
    let bad = || "guarded source: exact versioned clause required".to_string();
    let [
        TokenKind::Word(head),
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Word(id),
        TokenKind::Symbol(Symbol::Comma),
        rest @ ..,
        TokenKind::Symbol(Symbol::RightParen),
    ] = tokens.as_slice()
    else {
        return Err(bad());
    };
    if head != SYMBOL_HEAD && head != DOCUMENT_HEAD {
        return Err(bad());
    }
    let id = identifier(id)?;
    if tokens
        .iter()
        .any(|t| matches!(t, TokenKind::StringLiteral(_)))
    {
        return Err(crate::premise_formula::prose_guarded(head, &id));
    }
    let args = arguments(rest)?;
    if head == DOCUMENT_HEAD {
        let clauses = args
            .iter()
            .map(|a| document_clause(a))
            .collect::<Result<Vec<_>, _>>()?;
        let readable = readable(&clauses);
        if parse(&readable)? != clauses {
            return Err(bad());
        }
        return Ok(Some(SourceClause::Document { id, readable }));
    }
    let [role, formula, deps @ ..] = args.as_slice() else {
        return Err(bad());
    };
    let role = match role {
        [TokenKind::Word(role)] => identifier(role)?,
        _ => return Err(bad()),
    };
    let (proposition, _) = crate::premise_formula::guarded_formula(&id, formula)?;
    let dependencies = deps
        .iter()
        .map(|d| match d {
            [TokenKind::Word(d)] => identifier(d),
            _ => Err(bad()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(SourceClause::Symbol {
        id,
        role,
        proposition,
        dependencies,
    }))
}
