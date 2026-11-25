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

//! Declared cross-crate contract dependencies (card **QUIRIN-AUDIT-CONNECT**
//! Part C; user decision of 2026-11-26; contract spec C15.1).
//!
//! A resource-v1 declaration may state that its contract relies on another
//! crate's declaration, with that declaration's identity:
//!
//! ```text
//! requires resource_contract_dependency("quirin-provider-libc", libc::malloc, since "0.1.0");
//! requires resource_contract_dependency("quirin-provider-libc", libc::free, sha256 "<hex>");
//! requires resource_contract_dependency("quirin-provider-cfl", cfl::cfl_list_size,
//!     since "0.1.0", removed "0.2.0", sha256 "<hex>");
//! ```
//!
//! The crate is named by its package name (the `#include` spelling), the
//! declaration as `NS::NAME` with `NS` the crate's derived namespace, and the
//! identity as a version range over the crate's version (`since` inclusive,
//! `removed` exclusive, as `resource_library_version` reads them) and/or the
//! SHA-256 of the target declaration's normalised text, as its lock records
//! it (`contract_sha256`, C06.2). At least one identity is stated. The
//! clause is a contract-level requirement, read in place by the resource-v1
//! loader and recorded per declaration; it adds no obligation and changes no
//! other clause's reading.
//!
//! [`scan_module`] reads every such clause of a module's text with the same
//! clause reader, so the load-path manifest's `contract_dependencies` entry
//! (`providers/contracts.py sync`) is checked against the module texts by
//! `load_path_manifest` (`QI014`): a stale entry is refused, never read.
//!
//! Resolution is `qcore_schemas::connect::resolve`; the audit withholds every
//! declaration it holds (`qcore_audit::declared_dependencies`).

use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::connect::DeclaredDependency;
use qcore_schemas::library_version::LibraryVersion;

/// The clause head.
pub const CONTRACT_DEPENDENCY_CLAUSE: &str = "resource_contract_dependency";

fn malformed(expression: &str) -> String {
    format!(
        "malformed contract dependency `{expression}`: expected `{CONTRACT_DEPENDENCY_CLAUSE}(\"CRATE\", \
         NS::NAME[, since \"V\"[, removed \"W\"]][, sha256 \"HEX\"])` with at least one identity \
         (contract spec C15.1)"
    )
}

/// A dependency as written in one declaration (its dependent is the
/// declaration; its crate is the module's owner, known only on a load path).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WrittenDependency {
    pub target_crate: String,
    pub target_namespace: String,
    pub target_function: String,
    pub since: Option<LibraryVersion>,
    pub removed: Option<LibraryVersion>,
    pub sha256: Option<String>,
}

impl WrittenDependency {
    /// The dependency of `dependent` in `module` of `dependent_crate`.
    #[must_use]
    pub fn of(&self, dependent_crate: &str, dependent: &str, module: &str) -> DeclaredDependency {
        DeclaredDependency {
            dependent_crate: dependent_crate.to_owned(),
            dependent: dependent.to_owned(),
            module: module.to_owned(),
            target_crate: self.target_crate.clone(),
            target_namespace: self.target_namespace.clone(),
            target_function: self.target_function.clone(),
            since: self.since,
            removed: self.removed,
            sha256: self.sha256.clone(),
        }
    }
}

/// Read one clause's expression (`resource_contract_dependency(...)`).
/// `None` when the expression is another clause.
///
/// # Errors
///
/// The head under `ensures` (a dependency is a requirement), a malformed
/// argument list, or a clause the static checks refuse
/// (`DeclaredDependency::validate`: an identity, ordered bounds, a digest,
/// a libc/POSIX target named under `quirin-provider-libc`).
pub fn contract_dependency_clause(
    requires: bool,
    expression: &str,
) -> Result<Option<WrittenDependency>, String> {
    let lexed = lex(expression).map_err(|e| e.to_string())?;
    let tokens = lexed
        .iter()
        .filter(|token| token.kind != TokenKind::Eof)
        .map(|token| &token.kind)
        .collect::<Vec<_>>();
    if !matches!(tokens.first(), Some(TokenKind::Word(word)) if word == CONTRACT_DEPENDENCY_CLAUSE)
    {
        return Ok(None);
    }
    if !requires {
        return Err(format!(
            "a contract dependency is a requirement: write `requires {expression};`"
        ));
    }
    let symbol =
        |index: usize, wanted: Symbol| tokens.get(index) == Some(&&TokenKind::Symbol(wanted));
    let word = |index: usize| match tokens.get(index) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    };
    let string = |index: usize| match tokens.get(index) {
        Some(TokenKind::StringLiteral(text)) => Some(text.clone()),
        _ => None,
    };
    // `( "CRATE" , NS :: NAME` then `, KEY "VALUE"` pairs, then `)`.
    if !symbol(1, Symbol::LeftParen)
        || !symbol(3, Symbol::Comma)
        || !symbol(5, Symbol::ColonColon)
        || !tokens
            .last()
            .is_some_and(|t| **t == TokenKind::Symbol(Symbol::RightParen))
    {
        return Err(malformed(expression));
    }
    let (Some(target_crate), Some(namespace), Some(function)) = (string(2), word(4), word(6))
    else {
        return Err(malformed(expression));
    };
    let plain = |text: &str| {
        !text.is_empty()
            && text
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    if !(target_crate.starts_with("quirin-provider-")
        || target_crate.starts_with("qcore-provider-"))
        || !plain(&target_crate)
    {
        return Err(format!(
            "{}: the crate is a `quirin-provider-NAME` or `qcore-provider-NAME` package name",
            malformed(expression)
        ));
    }
    let derived = target_crate
        .trim_start_matches("quirin-provider-")
        .trim_start_matches("qcore-provider-")
        .replace('-', "_");
    if namespace != derived {
        return Err(format!(
            "contract dependency `{expression}`: `{namespace}::` is not the namespace of \
             {target_crate} (`{derived}::`)"
        ));
    }
    let mut written = WrittenDependency {
        target_crate,
        target_namespace: namespace.to_owned(),
        target_function: function.to_owned(),
        since: None,
        removed: None,
        sha256: None,
    };
    let mut index = 7;
    let last = tokens.len() - 1;
    let mut order = 0;
    while index < last {
        if !symbol(index, Symbol::Comma) {
            return Err(malformed(expression));
        }
        let (Some(key), Some(value)) = (word(index + 1), string(index + 2)) else {
            return Err(malformed(expression));
        };
        let rank = match key {
            "since" => 1,
            "removed" => 2,
            "sha256" => 3,
            _ => return Err(malformed(expression)),
        };
        if rank <= order {
            return Err(format!(
                "contract dependency `{expression}`: `since`, `removed` and `sha256` are \
                 written once each, in that order"
            ));
        }
        order = rank;
        match key {
            "since" => {
                written.since = Some(
                    LibraryVersion::parse(&value)
                        .map_err(|error| format!("{expression}: {error}"))?,
                );
            }
            "removed" => {
                written.removed = Some(
                    LibraryVersion::parse(&value)
                        .map_err(|error| format!("{expression}: {error}"))?,
                );
            }
            _ => written.sha256 = Some(value),
        }
        index += 3;
    }
    if index != last {
        return Err(malformed(expression));
    }
    written
        .of("", "", "")
        .validate(None)
        .map_err(|error| format!("contract dependency `{expression}`: {error}"))?;
    Ok(Some(written))
}

/// Every declared dependency of a module's text, keyed by the declaring
/// function, in text order: each `extern fn NAME ... contract { ... }`
/// block's `requires resource_contract_dependency(...);` clauses, read with
/// [`contract_dependency_clause`].
///
/// # Errors
///
/// A text that does not lex, or a clause [`contract_dependency_clause`]
/// refuses.
pub fn scan_module(text: &str) -> Result<Vec<(String, WrittenDependency)>, String> {
    let tokens = lex(text).map_err(|e| e.to_string())?;
    let kinds: Vec<&TokenKind> = tokens.iter().map(|token| &token.kind).collect();
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    let mut index = 0;
    while index < kinds.len() {
        match kinds[index] {
            TokenKind::Word(word) if word == "extern" => {
                if let (Some(TokenKind::Word(fn_word)), Some(TokenKind::Word(name))) =
                    (kinds.get(index + 1), kinds.get(index + 2))
                    && fn_word == "fn"
                {
                    current = Some(name.clone());
                }
            }
            TokenKind::Word(word) if word == "requires" => {
                if matches!(kinds.get(index + 1), Some(TokenKind::Word(head)) if head == CONTRACT_DEPENDENCY_CLAUSE)
                {
                    // The clause runs to its balanced `)`.
                    let start = index + 1;
                    let mut depth = 0_i32;
                    let mut end = start;
                    for (offset, kind) in kinds[start..].iter().enumerate() {
                        match kind {
                            TokenKind::Symbol(Symbol::LeftParen) => depth += 1,
                            TokenKind::Symbol(Symbol::RightParen) => {
                                depth -= 1;
                                if depth == 0 {
                                    end = start + offset;
                                    break;
                                }
                            }
                            TokenKind::Eof => break,
                            _ => {}
                        }
                    }
                    if end == start {
                        return Err(malformed(CONTRACT_DEPENDENCY_CLAUSE));
                    }
                    let first = &tokens[start].span;
                    let close = &tokens[end].span;
                    let expression = &text[first.start..close.end];
                    let Some(name) = &current else {
                        return Err(format!(
                            "contract dependency `{expression}` outside an `extern fn` declaration"
                        ));
                    };
                    let written = contract_dependency_clause(true, expression)?
                        .expect("the head was matched");
                    out.push((name.clone(), written));
                    index = end;
                }
            }
            _ => {}
        }
        index += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: &str = "647e223d864f5dfacfa57bb573f270fc7ae4dfd1c0af5f7fb022a0d00b534636";

    #[test]
    fn the_clause_reads_every_identity_form() {
        let read = |text: &str| contract_dependency_clause(true, text);
        let range = read(
            "resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, since \"0.1.0\")",
        )
        .unwrap()
        .unwrap();
        assert_eq!(range.target_function, "malloc");
        assert_eq!(range.since.unwrap().to_string(), "0.1.0");
        let digest = read(&format!(
            "resource_contract_dependency(\"quirin-provider-libc\", libc::free, sha256 \"{HEX}\")"
        ))
        .unwrap()
        .unwrap();
        assert_eq!(digest.sha256.as_deref(), Some(HEX));
        let both = read(&format!(
            "resource_contract_dependency(\"quirin-provider-cfl\", cfl::cfl_list_size, since \
             \"0.1.0\", removed \"0.2.0\", sha256 \"{HEX}\")"
        ))
        .unwrap()
        .unwrap();
        assert_eq!(both.removed.unwrap().to_string(), "0.2.0");
        assert!(read("resource_on_return(x)").unwrap().is_none());
    }

    #[test]
    fn malformed_clauses_reject() {
        for text in [
            // No identity.
            "resource_contract_dependency(\"quirin-provider-libc\", libc::malloc)",
            // Wrong namespace for the crate.
            "resource_contract_dependency(\"quirin-provider-libc\", cfl::malloc, since \"0.1\")",
            // A libc target named under another crate.
            "resource_contract_dependency(\"quirin-provider-onigmo\", libc::malloc, since \"0.1\")",
            // Not a provider crate.
            "resource_contract_dependency(\"serde\", serde::x, since \"0.1\")",
            // Out of order, repeated, unknown key, bad digest.
            "resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, removed \"0.2\", since \"0.1\")",
            "resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, since \"0.1\", since \"0.2\")",
            "resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, after \"0.1\")",
            "resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, sha256 \"abc\")",
            // Bounds not ordered.
            "resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, since \"0.2\", removed \"0.1\")",
        ] {
            assert!(
                contract_dependency_clause(true, text).is_err(),
                "admitted: {text}"
            );
        }
        assert!(
            contract_dependency_clause(
                false,
                "resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, since \"0.1\")"
            )
            .is_err(),
            "under ensures"
        );
    }

    #[test]
    fn a_module_scan_keys_clauses_by_declaration() {
        let text = format!(
            "module contract;\n\nextern fn onig_new(reg: ptr<mut, u8>) -> i32\ncontract {{\n    \
             requires resource_contract_v1;\n    requires resource_contract_dependency(\
             \"quirin-provider-libc\", libc::malloc, since \"0.1.0\");\n    requires \
             resource_contract_dependency(\"quirin-provider-libc\", libc::free, sha256 \
             \"{HEX}\");\n}};\n\nextern fn onig_free(reg: ptr<mut, u8>)\ncontract {{\n    \
             requires resource_contract_v1;\n}};\n"
        );
        let scanned = scan_module(&text).unwrap();
        assert_eq!(scanned.len(), 2);
        assert!(scanned.iter().all(|(name, _)| name == "onig_new"));
        assert_eq!(scanned[1].1.target_function, "free");
        assert!(scan_module("module contract;\n").unwrap().is_empty());
    }
}
