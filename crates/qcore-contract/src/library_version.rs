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

//! The `resource_library_version` clause (card **OPENSSL-PROVIDER-PLUGIN**,
//! user decisions of 2026-11-16; contract spec §C06.2, grammar G05):
//!
//! ```text
//! requires resource_library_version(LIBRARY, since "MAJOR.MINOR[.PATCH]");
//! requires resource_library_version(LIBRARY, since "V", removed "W");
//! requires resource_library_version(LIBRARY, documented "V"[, removed "W"]);
//! ```
//!
//! Total-or-reject like the rest of the resource loader: anything that begins
//! with the clause word and is not exactly one of these spellings, an
//! `ensures` form, a removal not above the lower bound, and a second range on
//! one declaration reject the whole module. The range is **not inherited**
//! (as a revision marker is not): it cites the documentation of the parent's
//! function, and a child names a different one ([`strip_library_version`]).
//! The schema and its in-force predicate are
//! `qcore_schemas::library_version`.

use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::library_version::{LibraryVersion, LibraryVersionBound, LowerBoundSource};

/// The clause word.
pub const LIBRARY_VERSION_CLAUSE: &str = "resource_library_version";

fn malformed(expression: &str) -> String {
    format!(
        "malformed library version range `{expression}`: write `requires \
         {LIBRARY_VERSION_CLAUSE}(LIBRARY, since \"MAJOR.MINOR[.PATCH]\"[, removed \"V\"]);` or \
         `documented \"V\"` in place of `since`"
    )
}

/// Read one clause expression as a library version range, if it is one.
/// `None` means the clause is not one and is read as written.
///
/// # Errors
///
/// A clause that begins with the clause word and is malformed, written as an
/// `ensures` clause, or states a removal not above its lower bound.
pub fn library_version_clause(
    requires: bool,
    expression: &str,
) -> Result<Option<LibraryVersionBound>, String> {
    let lexed = lex(expression).map_err(|e| e.to_string())?;
    let tokens = lexed
        .iter()
        .filter(|token| token.kind != TokenKind::Eof)
        .map(|token| &token.kind)
        .collect::<Vec<_>>();
    if !matches!(tokens.first(), Some(TokenKind::Word(word)) if word == LIBRARY_VERSION_CLAUSE) {
        return Ok(None);
    }
    if !requires {
        return Err(format!(
            "a library version range is a requirement: write `requires {expression};`"
        ));
    }
    let word = |index: usize| match tokens.get(index) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    };
    let string = |index: usize| match tokens.get(index) {
        Some(TokenKind::StringLiteral(text)) => Some(text.as_str()),
        _ => None,
    };
    let symbol =
        |index: usize, wanted: Symbol| tokens.get(index) == Some(&&TokenKind::Symbol(wanted));
    let shape = symbol(1, Symbol::LeftParen)
        && symbol(3, Symbol::Comma)
        && (tokens.len() == 7 && symbol(6, Symbol::RightParen)
            || tokens.len() == 10
                && symbol(6, Symbol::Comma)
                && word(7) == Some("removed")
                && symbol(9, Symbol::RightParen));
    let (Some(library), Some(keyword), Some(lower)) = (word(2), word(4), string(5)) else {
        return Err(malformed(expression));
    };
    if !shape
        || !library
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        || !library
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(malformed(expression));
    }
    let lower_source = match keyword {
        "since" => LowerBoundSource::Since,
        "documented" => LowerBoundSource::Documented,
        _ => return Err(malformed(expression)),
    };
    let lower = LibraryVersion::parse(lower).map_err(|error| format!("{expression}: {error}"))?;
    let removed = if tokens.len() == 10 {
        let text = string(8).ok_or_else(|| malformed(expression))?;
        let removed =
            LibraryVersion::parse(text).map_err(|error| format!("{expression}: {error}"))?;
        if removed <= lower {
            return Err(format!(
                "{expression}: the removal `{removed}` must be above the lower bound `{lower}`"
            ));
        }
        Some(removed)
    } else {
        None
    };
    Ok(Some(LibraryVersionBound {
        library: library.to_owned(),
        lower,
        lower_source,
        removed,
    }))
}

/// Remove library version ranges from the clauses elaboration copies from a
/// parent: the range is not inherited (module documentation). A malformed
/// range is left in place, where the loader refuses it as it does in the
/// parent.
#[must_use]
pub fn strip_library_version(clauses: &[(String, String)]) -> Vec<(String, String)> {
    clauses
        .iter()
        .filter(|(introducer, expression)| {
            !matches!(
                library_version_clause(introducer == "requires", expression),
                Ok(Some(_))
            )
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_spellings_load_and_everything_else_refuses() {
        let read = |text: &str| library_version_clause(true, text);
        let bound = read("resource_library_version(openssl, since \"1.1.0\")")
            .unwrap()
            .unwrap();
        assert_eq!(bound.library, "openssl");
        assert_eq!(bound.lower_source, LowerBoundSource::Since);
        assert_eq!(bound.removed, None);
        let bound = read("resource_library_version(openssl, since \"1.1\", removed \"4.0.0\")")
            .unwrap()
            .unwrap();
        assert_eq!(bound.lower.to_string(), "1.1.0");
        assert_eq!(bound.removed.unwrap().to_string(), "4.0.0");
        let bound = read("resource_library_version(libcurl, documented \"7.10\")")
            .unwrap()
            .unwrap();
        assert_eq!(bound.lower_source, LowerBoundSource::Documented);
        assert!(read("resource_borrowed(p)").unwrap().is_none());
        for bad in [
            "resource_library_version(openssl)",
            "resource_library_version(openssl, since 1.1.0)",
            "resource_library_version(openssl, after \"1.1.0\")",
            "resource_library_version(OpenSSL, since \"1.1.0\")",
            "resource_library_version(openssl, since \"1.1.0\", until \"2.0\")",
            "resource_library_version(openssl, since \"1.1.0\", removed \"1.1.0\")",
            "resource_library_version(openssl, since \"1.1.0\", removed \"1.0.2\")",
            "resource_library_version(openssl, since \"x\")",
            "resource_library_version(openssl, since \"1.1.0\",)",
        ] {
            assert!(read(bad).is_err(), "{bad}");
        }
        assert!(
            library_version_clause(false, "resource_library_version(openssl, since \"1.1.0\")")
                .is_err()
        );
        let stripped = strip_library_version(&[
            (
                "requires".into(),
                "resource_library_version(openssl, since \"1.1.0\")".into(),
            ),
            ("ensures normal".into(), "resource_borrowed(p)".into()),
        ]);
        assert_eq!(stripped.len(), 1);
    }
}
