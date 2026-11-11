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

//! The contract-inclusion directive layer (card **CONTRACT-INHERIT**).
//!
//! Two forms let a contract module take a declaration's contract from another
//! declared contract instead of restating it:
//!
//! ```text
//! module contract;
//! #include "quirin-provider-libc:libc/string.qv";
//!
//! extern fn flb_memcpy(dst: ptr<mut, u8>, src: ptr<const, u8>, n: usize) -> ptr<mut, u8>
//! contract inherit libc::memcpy;
//! ```
//!
//! This module only **reads** them: it finds every directive and every
//! `contract inherit NAME;` tail, checks their token shape, and reports their
//! byte spans. It resolves nothing. Resolution against a published load path,
//! the positional renaming and the elaboration into ordinary contract text
//! live in `qcore_contract::contract_inclusion`, which is the only consumer.
//!
//! **Why the forms are not in the shared source AST.** The structural parser
//! ([`crate::frontend::parse_source_program`]) keeps refusing both forms, with
//! a specific message, so an *unresolved* inclusion does not parse for any
//! consumer: the observation interpreter, the lifecycle adapters and the
//! route readers never see an extern declaration whose contract block is
//! merely absent. The contract loader elaborates a module into the text a
//! person would have written by hand before anything parses it.
//!
//! **No reserved word** (constraint 14). `include` is read only directly after
//! a `#` at a contract module's top level, and `inherit` only directly after
//! the word `contract` in a declaration's contract position, where `{` was the
//! only admitted token before this card. The census is in
//! `docs/reports/contract-inherit/`.

use crate::lexer::{Span, Symbol, Token, TokenKind, lex};

/// One `#include "crate:module";` directive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractIncludeDirective {
    /// From the `#` through the terminating `;`.
    pub span: Span,
    /// The crate half of the target, as written.
    pub crate_name: String,
    /// The module half of the target (a path under the crate's `contracts/`).
    pub module: String,
}

impl ContractIncludeDirective {
    /// The target as written: `crate:module`.
    #[must_use]
    pub fn target(&self) -> String {
        format!("{}:{}", self.crate_name, self.module)
    }
}

/// One `contract inherit [NAMESPACE::]NAME;` tail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractInheritSite {
    /// The span of the `contract` token.
    pub contract_span: Span,
    /// From `inherit` through `NAME` (the `;` is not included).
    pub span: Span,
    /// The crate namespace qualifying the parent (`libc` in
    /// `inherit libc::memcpy`), or `None` for an own-module parent.
    pub namespace: Option<String>,
    /// The parent declaration's name.
    pub parent: String,
}

impl ContractInheritSite {
    /// The parent as written: `NAME` or `NAMESPACE::NAME`.
    #[must_use]
    pub fn written_parent(&self) -> String {
        match &self.namespace {
            Some(namespace) => format!("{namespace}::{}", self.parent),
            None => self.parent.clone(),
        }
    }
}

/// Every inclusion form in one module, in source order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContractInclusionSyntax {
    pub includes: Vec<ContractIncludeDirective>,
    pub inherits: Vec<ContractInheritSite>,
}

impl ContractInclusionSyntax {
    /// Whether the module uses either form at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.includes.is_empty() && self.inherits.is_empty()
    }
}

/// Which refusal a malformed inclusion form is. The contract loader maps these
/// onto its `QI` codes; the front end allocates no code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractInclusionSyntaxErrorKind {
    /// A form in the wrong place or with the wrong token shape.
    Malformed,
    /// `contract inherit NAME { … }`: extension clauses, refused in this
    /// version.
    ExtensionClauses,
    /// An include operand that is not `crate:module` in the admitted charset.
    Target,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractInclusionSyntaxError {
    pub kind: ContractInclusionSyntaxErrorKind,
    pub span: Span,
    pub message: String,
}

impl std::fmt::Display for ContractInclusionSyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for ContractInclusionSyntaxError {}

fn is_symbol(tokens: &[Token], index: usize, symbol: Symbol) -> bool {
    matches!(tokens.get(index).map(|t| &t.kind), Some(TokenKind::Symbol(s)) if *s == symbol)
}

fn is_word(tokens: &[Token], index: usize, word: &str) -> bool {
    matches!(tokens.get(index).map(|t| &t.kind), Some(TokenKind::Word(w)) if w == word)
}

/// Whether `source` contains either inclusion form anywhere, by token shape
/// alone: a `#` directly followed by the word `include`, or the word
/// `contract` directly followed by the word `inherit`.
///
/// A source the lexer rejects contains neither (it is refused by the parser
/// exactly as before). This is the guard the contract loader applies at every
/// entry point that has no load path, so it is deliberately broader than the
/// admitted shapes: a malformed use still counts.
#[must_use]
pub fn contains_contract_inclusion_syntax(source: &str) -> bool {
    let Ok(tokens) = lex(source) else {
        return false;
    };
    (0..tokens.len()).any(|index| {
        (is_symbol(&tokens, index, Symbol::Hash) && is_word(&tokens, index + 1, "include"))
            || (is_word(&tokens, index, "contract") && is_word(&tokens, index + 1, "inherit"))
    })
}

fn charset(value: &str, extra: &[char]) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || extra.contains(&c))
}

/// Whether `name` is an admitted contract crate name: a lowercase Cargo
/// package name (`[a-z0-9][a-z0-9_-]*`).
#[must_use]
pub fn valid_contract_crate_name(name: &str) -> bool {
    charset(name, &['_', '-'])
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// Whether `module` is an admitted logical module path: `providers/contracts.py`'s
/// asset rule (`[a-z0-9_/-]+\.qcore`) with every `/`-separated segment
/// nonempty. Absolute paths, `.`, `..`, backslashes and `:` are
/// unrepresentable.
#[must_use]
pub fn valid_contract_module_path(module: &str) -> bool {
    crate::source_extension::contract_module_stem(module).is_some_and(|stem| {
        charset(stem, &['_', '-', '/']) && stem.split('/').all(|segment| !segment.is_empty())
    })
}

/// Split and validate an include operand `crate:module`.
fn include_target(text: &str) -> Option<(String, String)> {
    let (crate_name, module) = text.split_once(':')?;
    (valid_contract_crate_name(crate_name) && valid_contract_module_path(module))
        .then(|| (crate_name.to_owned(), module.to_owned()))
}

/// Read every inclusion form in `source`, checking its shape.
///
/// Admitted shapes:
///
/// * `# include "crate:module" ;` — only in a module that begins with
///   `module contract ;`, and only between that header and the first
///   declaration;
/// * `contract inherit NAME ;` — at brace, bracket and parenthesis depth 0.
///
/// Everything else that uses either form is refused. A source with neither
/// form yields an empty result, including a source the lexer rejects.
pub fn scan_contract_inclusion(
    source: &str,
) -> Result<ContractInclusionSyntax, ContractInclusionSyntaxError> {
    let mut result = ContractInclusionSyntax::default();
    if !contains_contract_inclusion_syntax(source) {
        return Ok(result);
    }
    let mut tokens = lex(source).map_err(|error| ContractInclusionSyntaxError {
        kind: ContractInclusionSyntaxErrorKind::Malformed,
        span: error.span,
        message: error.message,
    })?;
    tokens.pop();
    let malformed = |span: Span, message: &str| ContractInclusionSyntaxError {
        kind: ContractInclusionSyntaxErrorKind::Malformed,
        span,
        message: message.to_owned(),
    };
    let span_at = |index: usize| {
        tokens.get(index).map_or(
            Span {
                start: source.len(),
                end: source.len(),
            },
            |token| token.span,
        )
    };
    if !(is_word(&tokens, 0, "module")
        && is_word(&tokens, 1, "contract")
        && is_symbol(&tokens, 2, Symbol::Semicolon))
    {
        return Err(malformed(
            span_at(0),
            "contract inclusion is admitted only in a `module contract;` module",
        ));
    }
    let mut cursor = 3;
    while is_symbol(&tokens, cursor, Symbol::Hash) && is_word(&tokens, cursor + 1, "include") {
        let Some(TokenKind::StringLiteral(text)) = tokens.get(cursor + 2).map(|t| &t.kind) else {
            return Err(malformed(
                span_at(cursor + 2),
                "`#include` requires one string operand",
            ));
        };
        if !is_symbol(&tokens, cursor + 3, Symbol::Semicolon) {
            return Err(malformed(
                span_at(cursor + 3),
                "expected `;` after the `#include` operand",
            ));
        }
        let Some((crate_name, module)) = include_target(text) else {
            return Err(ContractInclusionSyntaxError {
                kind: ContractInclusionSyntaxErrorKind::Target,
                span: span_at(cursor + 2),
                message: format!(
                    "include target `{text}` is not `crate:module` (a lowercase crate name, \
                     then a relative `[a-z0-9_/-]+` module path ending in `.qv` or `.qcore`)"
                ),
            });
        };
        result.includes.push(ContractIncludeDirective {
            span: Span {
                start: tokens[cursor].span.start,
                end: tokens[cursor + 3].span.end,
            },
            crate_name,
            module,
        });
        cursor += 4;
    }
    let mut depth = 0_i64;
    for index in cursor..tokens.len() {
        match &tokens[index].kind {
            TokenKind::Symbol(Symbol::LeftBrace | Symbol::LeftParen | Symbol::LeftBracket) => {
                depth += 1;
            }
            TokenKind::Symbol(Symbol::RightBrace | Symbol::RightParen | Symbol::RightBracket) => {
                depth -= 1;
            }
            TokenKind::Symbol(Symbol::Hash) if is_word(&tokens, index + 1, "include") => {
                return Err(malformed(
                    tokens[index].span,
                    "an `#include` directive must follow `module contract;` and precede every declaration",
                ));
            }
            TokenKind::Word(word)
                if word == "contract" && is_word(&tokens, index + 1, "inherit") =>
            {
                if depth != 0 {
                    return Err(malformed(
                        tokens[index].span,
                        "`contract inherit` is admitted only as a declaration's contract",
                    ));
                }
                let Some(TokenKind::Word(first)) = tokens.get(index + 2).map(|t| &t.kind) else {
                    return Err(malformed(
                        span_at(index + 2),
                        "`contract inherit` requires one parent declaration name",
                    ));
                };
                // `[NAMESPACE ::] NAME`: exactly one qualifier level.
                let (namespace, parent, last) = if is_symbol(&tokens, index + 3, Symbol::ColonColon)
                {
                    let Some(TokenKind::Word(name)) = tokens.get(index + 4).map(|t| &t.kind) else {
                        return Err(malformed(
                            span_at(index + 4),
                            "expected a declaration name after `NAMESPACE::`",
                        ));
                    };
                    if is_symbol(&tokens, index + 5, Symbol::ColonColon) {
                        return Err(malformed(
                            span_at(index + 5),
                            "a parent is `NAME` or `NAMESPACE::NAME`; one qualifier level only",
                        ));
                    }
                    (Some(first.clone()), name.clone(), index + 4)
                } else {
                    (None, first.clone(), index + 2)
                };
                if is_symbol(&tokens, last + 1, Symbol::LeftBrace) {
                    return Err(ContractInclusionSyntaxError {
                        kind: ContractInclusionSyntaxErrorKind::ExtensionClauses,
                        span: span_at(last + 1),
                        message: "clauses after `contract inherit PARENT` are not admitted in \
                                  this version; write the whole contract or inherit it unchanged"
                            .to_owned(),
                    });
                }
                if !is_symbol(&tokens, last + 1, Symbol::Semicolon) {
                    return Err(malformed(
                        span_at(last + 1),
                        "expected `;` after `contract inherit PARENT`",
                    ));
                }
                result.inherits.push(ContractInheritSite {
                    contract_span: tokens[index].span,
                    span: Span {
                        start: tokens[index + 1].span.start,
                        end: tokens[last].span.end,
                    },
                    namespace,
                    parent,
                });
            }
            _ => {}
        }
    }
    Ok(result)
}

#[cfg(test)]
#[path = "../tests/unit/contract_inclusion.rs"]
mod tests;
