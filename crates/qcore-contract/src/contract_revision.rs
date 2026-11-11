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

//! Revision markers of resource-v1 contracts (card
//! **W45-CONTRACT-REVISION-MODE**, contract spec §C06.2).
//!
//! ```text
//! requires resource_revision(flexible);                                    // contract-level
//! ensures normal resource_revision(resource_released(p, nonnull), pinned);  // clause-level
//! ```
//!
//! **Pinned is the default.** A declaration is flexible exactly when its
//! contract-level mode is flexible and no clause is marked pinned; a contract
//! is applied as one conjunction, so one pinned clause pins all of it.
//!
//! Everything here is total-or-reject, like the rest of the resource loader:
//! a malformed marker, a second contract-level marker (duplicate or
//! conflicting), a clause-level marker that restates the contract-level mode,
//! a marker on a clause that cannot carry one (the selector, a literal
//! premise, a `requires` clause) and a marker nested in another marker (a
//! revision or a review marker) each reject the whole module.
//!
//! The mode is **not inherited** (contract spec §C15): it records how the
//! parent's text was reviewed against the parent's own source, and an
//! inheriting declaration names a different function whose source was never
//! reviewed under it. [`strip_revision_markers`] removes the markers from the
//! clauses elaboration copies, so a child is pinned unless its own extension
//! clauses say otherwise.
//!
//! [`declaration_digests`] gives every declaration's normalised text digest,
//! the "contract text" a lock entry is taken for.

use std::collections::BTreeMap;

use qcore_frontend::frontend::{SourceModuleMode, parse_source_program};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::contract_revision::{
    DeclaredRevision, RevisionMarker, RevisionMode, RevisionScope,
};

/// The clause introducer of a revision marker.
pub const REVISION_MARKER: &str = "resource_revision";

/// One revision marker, parsed.
pub(crate) enum RevisionMarkerRead {
    /// `requires resource_revision(MODE);`
    Contract(RevisionMode),
    /// `ensures normal resource_revision(CLAUSE, MODE);`, with `inner` the
    /// wrapped clause's expression text.
    Clause { inner: String, mode: RevisionMode },
}

fn mode_word(word: &str) -> Option<RevisionMode> {
    match word {
        "pinned" => Some(RevisionMode::Pinned),
        "flexible" => Some(RevisionMode::Flexible),
        _ => None,
    }
}

/// Whether an expression begins with the marker word (used to refuse a review
/// marker wrapping a revision marker).
pub(crate) fn starts_with_revision_marker(expression: &str) -> bool {
    lex(expression).is_ok_and(|tokens| {
        matches!(
            tokens.first().map(|token| &token.kind),
            Some(TokenKind::Word(word)) if word == REVISION_MARKER
        )
    })
}

/// Read one clause expression as a revision marker, if it is one. `None`
/// means the clause is not a marker and is read as written.
pub(crate) fn revision_marker(
    requires: bool,
    expression: &str,
) -> Result<Option<RevisionMarkerRead>, String> {
    let lexed = lex(expression).map_err(|e| e.to_string())?;
    let tokens = lexed
        .iter()
        .filter(|token| token.kind != TokenKind::Eof)
        .collect::<Vec<_>>();
    if !matches!(
        tokens.first().map(|t| &t.kind),
        Some(TokenKind::Word(word)) if word == REVISION_MARKER
    ) {
        return Ok(None);
    }
    let malformed = || {
        format!(
            "malformed revision marker `{expression}`: write `requires {REVISION_MARKER}(pinned);`, \
             `requires {REVISION_MARKER}(flexible);` or `ensures normal \
             {REVISION_MARKER}(CLAUSE, pinned|flexible);`"
        )
    };
    if tokens.len() < 4
        || tokens[1].kind != TokenKind::Symbol(Symbol::LeftParen)
        || tokens[tokens.len() - 1].kind != TokenKind::Symbol(Symbol::RightParen)
    {
        return Err(malformed());
    }
    let arguments = &tokens[2..tokens.len() - 1];
    let last_word = |token: &&qcore_frontend::lexer::Token| match &token.kind {
        TokenKind::Word(word) => mode_word(word),
        _ => None,
    };
    if arguments.len() == 1 {
        let mode = last_word(&arguments[0]).ok_or_else(malformed)?;
        return if requires {
            Ok(Some(RevisionMarkerRead::Contract(mode)))
        } else {
            Err(format!(
                "a contract-level revision marker is a requirement: write `requires {expression};`"
            ))
        };
    }
    // Clause-level: a balanced `NAME(...)` clause, a comma, one mode word.
    let Some(TokenKind::Word(name)) = arguments.first().map(|t| &t.kind) else {
        return Err(malformed());
    };
    if name == REVISION_MARKER || name == crate::contract_loader::REVIEW_MARKER {
        return Err(format!(
            "unsupported revision marker `{expression}`: a marker cannot wrap another marker"
        ));
    }
    let mut depth = 0usize;
    let mut close = None;
    for (index, token) in arguments.iter().enumerate() {
        match token.kind {
            TokenKind::Symbol(Symbol::LeftParen) => depth += 1,
            TokenKind::Symbol(Symbol::RightParen) => {
                depth = depth.checked_sub(1).ok_or_else(malformed)?;
                if depth == 0 {
                    close = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close.ok_or_else(malformed)?;
    let rest = &arguments[close + 1..];
    let [comma, mode] = rest else {
        return Err(malformed());
    };
    if comma.kind != TokenKind::Symbol(Symbol::Comma) {
        return Err(malformed());
    }
    let mode = last_word(mode).ok_or_else(malformed)?;
    let inner = expression[arguments[0].span.start..arguments[close].span.end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if requires {
        return Err(format!(
            "unsupported revision marker `{expression}`: a clause-level marker wraps an `ensures \
             normal` clause; the resource-v1 selector and a literal-argument premise take the \
             contract's mode"
        ));
    }
    let head = inner.split('(').next().unwrap_or_default().trim();
    if head == "resource_contract_v1" || head == "resource_literal_argument" {
        return Err(format!(
            "unsupported revision marker `{expression}`: only an `ensures normal` resource-v1 \
             clause can carry its own mode"
        ));
    }
    Ok(Some(RevisionMarkerRead::Clause { inner, mode }))
}

/// Validate one declaration's markers as a whole and give its record, or
/// `None` when it carries no marker (pinned by default).
pub(crate) fn declared_revision(
    markers: Vec<RevisionMarker>,
) -> Result<Option<DeclaredRevision>, String> {
    if markers.is_empty() {
        return Ok(None);
    }
    let contract = markers
        .iter()
        .filter(|marker| marker.scope == RevisionScope::Contract)
        .map(|marker| marker.mode)
        .collect::<Vec<_>>();
    match contract.as_slice() {
        [] | [_] => {}
        [first, second, ..] if first == second => {
            return Err(format!(
                "duplicate revision marker: the contract is already `{}`",
                first.as_str()
            ));
        }
        _ => {
            return Err(
                "conflicting revision markers: a contract is either `pinned` or `flexible`".into(),
            );
        }
    }
    if let Some(mode) = contract.first()
        && let Some(clause) = markers.iter().find_map(|marker| match &marker.scope {
            RevisionScope::Clause { clause } if marker.mode == *mode => Some(clause),
            _ => None,
        })
    {
        return Err(format!(
            "duplicate revision marker on `{clause}`: the contract-level marker already makes \
             every clause `{}`",
            mode.as_str()
        ));
    }
    Ok(Some(DeclaredRevision {
        mode: DeclaredRevision::effective(&markers),
        markers,
    }))
}

/// Remove revision markers from the clauses elaboration copies from a parent:
/// a contract-level marker is dropped and a clause-level marker is unwrapped
/// to the bare clause, so the copied text reads exactly as if the parent had
/// carried no marker. The mode is not inherited (module documentation).
pub(crate) fn strip_revision_markers(clauses: &[(String, String)]) -> Vec<(String, String)> {
    clauses
        .iter()
        .filter_map(|(introducer, expression)| {
            match revision_marker(introducer == "requires", expression) {
                Ok(Some(RevisionMarkerRead::Contract(_))) => None,
                Ok(Some(RevisionMarkerRead::Clause { inner, .. })) => {
                    Some((introducer.clone(), inner))
                }
                // A malformed marker is left in place, where the loader
                // refuses it exactly as it refuses it in the parent.
                Ok(None) | Err(_) => Some((introducer.clone(), expression.clone())),
            }
        })
        .collect()
}

/// Every declaration's normalised text digest: the SHA-256 of its tokens
/// joined by single spaces (comments and layout removed), keyed by function
/// name, with the source-assumption key of its module. This is the contract
/// text a lock entry is taken for; a change that leaves the tokens unchanged
/// needs no new pin, any other change does.
///
/// # Errors
///
/// Refuses a source that does not parse as a contract module.
pub fn declaration_digests(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, (String, String)>, String> {
    let mut digests = BTreeMap::new();
    for (key, source) in sources {
        for declaration in module_declaration_digests(key, source)? {
            digests.insert(declaration.name, (key.clone(), declaration.contract_sha256));
        }
    }
    Ok(digests)
}

/// One declaration of a module with its normalised text digest (card
/// **CONTRACT-SUPERSESSION**): the digest [`declaration_digests`] gives, the
/// byte range of the declaration (from a `[| quant #c_ffi |]` marker before
/// it, when it has one) and its text as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleDeclaration {
    pub name: String,
    pub start: usize,
    pub end: usize,
    pub contract_sha256: String,
}

/// Every declaration of one module, in source order, with the digest
/// [`declaration_digests`] takes (card **CONTRACT-SUPERSESSION**). A module
/// may declare a function twice; each declaration is listed.
///
/// # Errors
///
/// Refuses a source that does not parse as a contract module.
pub fn module_declaration_digests(
    key: &str,
    source: &str,
) -> Result<Vec<ModuleDeclaration>, String> {
    let parsed = parse_source_program(source).map_err(|e| format!("{key}: {e}"))?;
    if parsed.mode != SourceModuleMode::Contract {
        return Err(format!("{key}: expected a module contract"));
    }
    // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: an alias a declaration
    // spells is part of its contract text, so a changed `alias #NAME =
    // @C_NAME;` changes the digest of every declaration spelling `#NAME`
    // (an alias never changes meaning under an unchanged pin).
    let normalise = |text: &str| -> Result<(String, Vec<TokenKind>), String> {
        let tokens = lex(text).map_err(|e| format!("{key}: {e}"))?;
        let kinds = tokens.iter().map(|token| token.kind.clone()).collect();
        Ok((
            tokens
                .iter()
                .filter(|token| token.kind != TokenKind::Eof)
                .map(|token| &text[token.span.start..token.span.end])
                .collect::<Vec<_>>()
                .join(" "),
            kinds,
        ))
    };
    let aliases = parsed
        .aliases()
        .iter()
        .map(|alias| {
            normalise(&source[alias.span.start..alias.span.end])
                .map(|(text, _)| (alias.value.name.clone(), text))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let mut declarations = Vec::new();
    for declaration in parsed.declarations {
        let text = &source[declaration.span.start..declaration.span.end];
        let (mut normalised, kinds) = normalise(text)?;
        for (name, alias) in &aliases {
            let spelled = kinds.windows(2).any(|pair| {
                pair[0] == TokenKind::Symbol(qcore_frontend::lexer::Symbol::Hash)
                    && pair[1] == TokenKind::Word(name.clone())
            });
            if spelled {
                normalised.push(' ');
                normalised.push_str(alias);
            }
        }
        let start = declaration
            .value
            .c_ffi
            .map_or(declaration.span.start, |marker| marker.start);
        declarations.push(ModuleDeclaration {
            name: declaration.value.name.clone(),
            start,
            end: declaration.span.end,
            contract_sha256: qcore_schemas::sha256_hex(normalised.as_bytes()),
        });
    }
    Ok(declarations)
}

/// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 13l): the C names each
/// declaration spells, `@NAME` directly or through a module alias `#NAME`,
/// keyed by function (a declaration spelling none is absent). `quirin-audit
/// pin` adds each name's enumeration to the declaration's reviewed identity,
/// so the value an `@` name resolves to is pinned with the declaration.
///
/// # Errors
///
/// Refuses a source that does not parse as a contract module.
pub fn declaration_c_names(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, std::collections::BTreeSet<String>>, String> {
    let mut names = BTreeMap::new();
    for (key, source) in sources {
        if !source.contains('@') {
            continue;
        }
        let parsed = parse_source_program(source).map_err(|e| format!("{key}: {e}"))?;
        let aliases = parsed
            .aliases()
            .iter()
            .map(|alias| (alias.value.name.clone(), alias.value.target.clone()))
            .collect::<BTreeMap<_, _>>();
        for declaration in parsed.declarations {
            let text = &source[declaration.span.start..declaration.span.end];
            let kinds = lex(text)
                .map_err(|e| format!("{key}: {e}"))?
                .into_iter()
                .map(|token| token.kind)
                .collect::<Vec<_>>();
            let spelled = kinds
                .windows(2)
                .filter_map(|pair| match pair {
                    [TokenKind::Symbol(Symbol::At), TokenKind::Word(name)] => Some(name.clone()),
                    [TokenKind::Symbol(Symbol::Hash), TokenKind::Word(name)] => {
                        aliases.get(name).cloned()
                    }
                    _ => None,
                })
                .collect::<std::collections::BTreeSet<_>>();
            if !spelled.is_empty() {
                names.insert(declaration.value.name.clone(), spelled);
            }
        }
    }
    Ok(names)
}

/// The function map an environment had before any pin mismatch was withheld:
/// its contracts in force plus every withheld pinned contract. The source
/// recheck (`check_source_assumptions`) compares it with a fresh load of the
/// retained texts, so the only difference a pin evaluation can make is to
/// move a pinned declaration, unchanged, out of force.
///
/// # Errors
///
/// Refuses a withheld name that is also in force, and a withheld declaration
/// whose recorded markers make it flexible.
pub fn in_force_before_pins(
    contracts: &qcore_schemas::contract::ResourceContracts,
) -> Result<BTreeMap<String, qcore_schemas::contract::ResourceFunctionContract>, String> {
    let mut functions = contracts.functions().clone();
    for (name, withheld) in contracts.pin_mismatch() {
        if contracts.revision_mode(name) != RevisionMode::Pinned {
            return Err(format!(
                "`{name}` is withheld for a pin mismatch but its contract is flexible"
            ));
        }
        // Card CALLEE-RETENTION-CLASS1: a withheld disposition or retaining
        // declaration was never in the function map.
        if withheld.relational.is_some() || withheld.retaining.is_some() {
            continue;
        }
        if functions
            .insert(name.clone(), withheld.contract.clone())
            .is_some()
        {
            return Err(format!(
                "`{name}` is both in force and withheld for a pin mismatch"
            ));
        }
    }
    Ok(functions)
}
