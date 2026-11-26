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

//! Declared cross-crate contract dependencies: the `linkage_contract_v1`
//! vocabulary (card **CONNECT-ADOPT-DECLARED-DEPENDENCIES**, user decisions
//! of 2026-11-26 "Every reliance" and "Accept all"; contract spec C15.2,
//! grammar G05l). It replaces the written form of card QUIRIN-AUDIT-CONNECT
//! Part C.
//!
//! A resource-v1 declaration states each other crate's declaration its
//! contract relies on as a **link**, beside the marker that selects the
//! vocabulary:
//!
//! ```text
//! extern fn rd_calloc(count: usize, size: usize) -> ptr<mut, u8>
//! contract {
//!     requires resource_contract_v1;
//!     requires linkage_contract_v1;
//!     quant #link libc::calloc;
//!     ...
//! };
//! ```
//!
//! or grouped, `premises { link { quant #link libc::calloc; }; ... };`, or
//! by a bare name a module-level `#include libc::calloc as quant #link;`
//! makes nameable (the inclusion elaborator writes the qualified name in
//! place, so every reader sees `NS::NAME`). `NS` is the target crate's
//! derived namespace (C15). A `contract inherit NS::NAME;` directive is
//! itself the link (D2): it elaborates to the same dependency, with form
//! [`DependencyForm::Inherit`].
//!
//! A link states **no identity**. `providers/contracts.py sync` (and
//! `quirin-audit plugin views --write`) records the target's lock digest
//! (`contract_sha256`) and crate version (`since`) in the load-path
//! manifest's `contract_dependencies` entry; [`scan_module`] reads the links
//! and inherit directives of a module's text, so `load_path_manifest`
//! refuses (`QI014`) a manifest whose entries differ from the texts. The
//! written `requires resource_contract_dependency(...)` form is retired
//! (`QL005`); it survives only as the elaborated record
//! ([`qcore_schemas::connect::DeclaredDependency::clause`]).
//!
//! Resolution is `qcore_schemas::connect::resolve`; the audit withholds every
//! declaration it holds (`qcore_audit::declared_dependencies`).

use std::fmt;

use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::connect::{DeclaredDependency, DependencyForm, LINKAGE_VOCABULARY};

/// The vocabulary marker, `requires linkage_contract_v1;`.
pub const LINKAGE_MARKER: &str = LINKAGE_VOCABULARY;

/// The retired written clause head (card QUIRIN-AUDIT-CONNECT).
pub const RETIRED_CLAUSE: &str = "resource_contract_dependency";

/// The introducer [`crate::contract_inclusion`] renders a link clause with.
pub const LINK_INTRODUCER: &str = "quant #link";

/// The linkage refusals, a loader-owned namespace on the `QI`/`QP`
/// precedent (`docs/agents/diagnostics.md`, contract spec C15.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LinkageCode {
    /// A link on a declaration without `requires linkage_contract_v1;`.
    Ql001,
    /// `requires linkage_contract_v1;` on a declaration with no link, or
    /// stated twice.
    Ql002,
    /// A link target that is not `NS::NAME`: a nested qualifier, or a bare
    /// name no module-level link import names.
    Ql003,
    /// One target linked twice by one declaration.
    Ql004,
    /// The retired written form `requires resource_contract_dependency(...)`.
    Ql005,
    /// A module-level link import that is malformed, repeated, unused, or
    /// names a function the module declares itself.
    Ql006,
    /// A link to a namespace no load-path crate derives, to a declaration
    /// its crate's lock does not record, or to its own declaration.
    Ql007,
    /// A libc/POSIX target named under a crate other than
    /// `quirin-provider-libc` (`cx_refuse_libc_target_elsewhere`).
    Ql008,
    /// A manifest entry whose recorded identity is missing or is not the
    /// target's lock digest and crate version.
    Ql009,
}

impl LinkageCode {
    /// The stable spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ql001 => "QL001",
            Self::Ql002 => "QL002",
            Self::Ql003 => "QL003",
            Self::Ql004 => "QL004",
            Self::Ql005 => "QL005",
            Self::Ql006 => "QL006",
            Self::Ql007 => "QL007",
            Self::Ql008 => "QL008",
            Self::Ql009 => "QL009",
        }
    }
}

impl fmt::Display for LinkageCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A refusal message carrying its code first, as loader errors do.
#[must_use]
pub fn refusal(code: LinkageCode, message: impl fmt::Display) -> String {
    format!("{code}: {message} (linkage_contract_v1, contract spec C15.2)")
}

/// A link as written in one declaration (its dependent is the declaration;
/// its crate is the module's owner, and the target's crate the load-path
/// crate deriving `target_namespace`, both known only on a load path).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WrittenDependency {
    pub target_namespace: String,
    pub target_function: String,
    pub form: DependencyForm,
}

impl WrittenDependency {
    /// The dependency of `dependent` in `module` of `dependent_crate`, its
    /// target crate unresolved (empty) and no identity recorded.
    #[must_use]
    pub fn of(&self, dependent_crate: &str, dependent: &str, module: &str) -> DeclaredDependency {
        self.of_target(dependent_crate, dependent, module, "")
    }

    /// The dependency with its target crate resolved.
    #[must_use]
    pub fn of_target(
        &self,
        dependent_crate: &str,
        dependent: &str,
        module: &str,
        target_crate: &str,
    ) -> DeclaredDependency {
        DeclaredDependency {
            dependent_crate: dependent_crate.to_owned(),
            dependent: dependent.to_owned(),
            module: module.to_owned(),
            target_crate: target_crate.to_owned(),
            target_namespace: self.target_namespace.clone(),
            target_function: self.target_function.clone(),
            since: None,
            removed: None,
            sha256: None,
            form: self.form,
            vocabulary: LINKAGE_VOCABULARY.to_owned(),
        }
    }
}

fn lowercase_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn c_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Read a link clause's expression (`NS::NAME`, the expression of a
/// `quant #link` clause).
///
/// # Errors
///
/// `QL003`: a bare name (the elaborator resolves imported bare names before
/// any reader runs) or any other shape.
pub fn link_target(expression: &str) -> Result<WrittenDependency, String> {
    let lexed = lex(expression).map_err(|e| e.to_string())?;
    let tokens = lexed
        .iter()
        .filter(|token| token.kind != TokenKind::Eof)
        .map(|token| &token.kind)
        .collect::<Vec<_>>();
    match tokens.as_slice() {
        [
            TokenKind::Word(namespace),
            TokenKind::Symbol(Symbol::ColonColon),
            TokenKind::Word(function),
        ] if lowercase_identifier(namespace) && c_identifier(function) => Ok(WrittenDependency {
            target_namespace: namespace.clone(),
            target_function: function.clone(),
            form: DependencyForm::Link,
        }),
        [TokenKind::Word(name)] => Err(refusal(
            LinkageCode::Ql003,
            format!(
                "`quant #link {name};` names no crate: write `quant #link NS::{name};` or \
                 import it with `#include NS::{name} as quant #link;`"
            ),
        )),
        _ => Err(refusal(
            LinkageCode::Ql003,
            format!("`quant #link {expression};` is not `quant #link NS::NAME;`"),
        )),
    }
}

/// Whether a clause is the vocabulary marker.
#[must_use]
pub fn is_marker(requires: bool, expression: &str) -> bool {
    requires && expression.trim() == LINKAGE_MARKER
}

/// Refuse the retired written form, under any introducer.
///
/// # Errors
///
/// `QL005` when `expression` is headed `resource_contract_dependency`.
pub fn refuse_retired(expression: &str) -> Result<(), String> {
    let head = expression
        .trim_start()
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .next()
        .unwrap_or_default();
    if head == RETIRED_CLAUSE {
        return Err(refusal(
            LinkageCode::Ql005,
            format!(
                "the written form `{RETIRED_CLAUSE}(...)` is retired: state `requires \
                 {LINKAGE_MARKER};` and `quant #link NS::NAME;`, whose identity \
                 `providers/contracts.py sync` records in the load-path manifest"
            ),
        ));
    }
    Ok(())
}

/// The declaration-level checks: a link needs the marker (`QL001`,
/// `cx_link_needs_marker`), the marker needs a link (`QL002`,
/// `cx_marker_needs_link`) and is stated once, and no target is linked twice
/// (`QL004`).
///
/// # Errors
///
/// The first check that fails.
pub fn check_declaration(
    function: &str,
    markers: usize,
    links: &[WrittenDependency],
) -> Result<(), String> {
    if !links.is_empty() && markers == 0 {
        return Err(refusal(
            LinkageCode::Ql001,
            format!("`{function}` states a link without `requires {LINKAGE_MARKER};`"),
        ));
    }
    if markers > 0 && links.is_empty() {
        return Err(refusal(
            LinkageCode::Ql002,
            format!("`{function}` states `requires {LINKAGE_MARKER};` and no link"),
        ));
    }
    if markers > 1 {
        return Err(refusal(
            LinkageCode::Ql002,
            format!("`{function}` states `requires {LINKAGE_MARKER};` twice"),
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for link in links {
        if !seen.insert((&link.target_namespace, &link.target_function)) {
            return Err(refusal(
                LinkageCode::Ql004,
                format!(
                    "`{function}` links `{}::{}` twice",
                    link.target_namespace, link.target_function
                ),
            ));
        }
    }
    Ok(())
}

/// Drop the linkage clauses from a parent's clauses before inheritance: a
/// link and the marker are not inherited (G7), since the inherit directive is
/// itself the child's link (D2).
#[must_use]
pub fn strip_linkage(clauses: &[(String, String)]) -> Vec<(String, String)> {
    clauses
        .iter()
        .filter(|(introducer, expression)| {
            introducer != LINK_INTRODUCER && !is_marker(introducer == "requires", expression)
        })
        .cloned()
        .collect()
}

/// A declaration's tokens with every `premises {` / `link {` opener and its
/// matching `};` removed: the clause sequence a reconstruction of the
/// declaration from its parsed clauses compares against (the blocks group
/// clauses and change no reading, G6 (a)).
#[must_use]
pub fn without_linkage_blocks(tokens: &[TokenKind]) -> Vec<TokenKind> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut braces: Vec<bool> = Vec::new();
    let mut skip_semicolon = false;
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        if skip_semicolon {
            skip_semicolon = false;
            if *token == TokenKind::Symbol(Symbol::Semicolon) {
                index += 1;
                continue;
            }
        }
        let wrapper = matches!(token, TokenKind::Word(word) if word == "premises" || word == "link")
            && tokens.get(index + 1) == Some(&TokenKind::Symbol(Symbol::LeftBrace))
            && !(index >= 2
                && tokens[index - 1] == TokenKind::Symbol(Symbol::Hash)
                && tokens[index - 2] == TokenKind::Word("quant".into()));
        if wrapper {
            braces.push(true);
            index += 2;
            continue;
        }
        match token {
            TokenKind::Symbol(Symbol::LeftBrace) => braces.push(false),
            // The guard pops every closing brace; a wrapper's is dropped.
            TokenKind::Symbol(Symbol::RightBrace) if braces.pop() == Some(true) => {
                skip_semicolon = true;
                index += 1;
                continue;
            }
            _ => {}
        }
        out.push(token.clone());
        index += 1;
    }
    out
}

/// One module-level link import, `#include NS::NAME as quant #link;`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkImport {
    pub namespace: String,
    pub function: String,
}

/// Every link and inherit directive of a module's text, keyed by the
/// declaring function, in text order: each `quant #link NS::NAME;` (a bare
/// `NAME` resolved through the module's `#include NS::NAME as quant #link;`
/// imports) and each `contract inherit NS::NAME;` (form
/// [`DependencyForm::Inherit`]; an unqualified parent is the module's own
/// and no dependency). The caller filters a qualified parent of the
/// module's own crate.
///
/// # Errors
///
/// A text that does not lex, a malformed link or import (`QL003`, `QL006`).
pub fn scan_module(text: &str) -> Result<Vec<(String, WrittenDependency)>, String> {
    let tokens = lex(text).map_err(|e| e.to_string())?;
    let kinds: Vec<&TokenKind> = tokens.iter().map(|token| &token.kind).collect();
    let word = |index: usize| match kinds.get(index) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    };
    let symbol =
        |index: usize, wanted: Symbol| kinds.get(index) == Some(&&TokenKind::Symbol(wanted));
    let imports = link_imports_of(&kinds)?;
    let declared: std::collections::BTreeSet<&str> = (0..kinds.len())
        .filter(|&i| word(i) == Some("extern") && word(i + 1) == Some("fn"))
        .filter_map(|i| word(i + 2))
        .collect();
    for import in &imports {
        if declared.contains(import.function.as_str()) {
            return Err(refusal(
                LinkageCode::Ql006,
                format!(
                    "`#include {}::{} as quant #link;` names a function this module declares",
                    import.namespace, import.function
                ),
            ));
        }
    }
    let mut used = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    let mut index = 0;
    while index < kinds.len() {
        if symbol(index, Symbol::Hash) && word(index + 1) == Some("include") {
            // A directive (an include or a link import): read above.
            while index < kinds.len() && !symbol(index, Symbol::Semicolon) {
                index += 1;
            }
        } else if word(index) == Some("extern")
            && word(index + 1) == Some("fn")
            && let Some(name) = word(index + 2)
        {
            current = Some(name.to_owned());
        } else if word(index) == Some("quant")
            && symbol(index + 1, Symbol::Hash)
            && word(index + 2) == Some("link")
            && !symbol(index + 3, Symbol::Equal)
            && word(index + 3) != Some("in")
        {
            let Some(name) = &current else {
                return Err(refusal(
                    LinkageCode::Ql003,
                    "`quant #link` outside an `extern fn` declaration",
                ));
            };
            let (written, next) = if symbol(index + 4, Symbol::ColonColon) {
                let (Some(namespace), Some(function)) = (word(index + 3), word(index + 5)) else {
                    return Err(refusal(LinkageCode::Ql003, "malformed `quant #link`"));
                };
                (link_target(&format!("{namespace}::{function}"))?, index + 6)
            } else {
                let Some(bare) = word(index + 3) else {
                    return Err(refusal(LinkageCode::Ql003, "malformed `quant #link`"));
                };
                let Some(import) = imports.iter().find(|import| import.function == bare) else {
                    return Err(link_target(bare).expect_err("a bare name is refused"));
                };
                used.insert(bare.to_owned());
                (
                    WrittenDependency {
                        target_namespace: import.namespace.clone(),
                        target_function: import.function.clone(),
                        form: DependencyForm::Link,
                    },
                    index + 4,
                )
            };
            if !symbol(next, Symbol::Semicolon) {
                return Err(refusal(
                    LinkageCode::Ql003,
                    "a link names one declaration, `quant #link NS::NAME;`",
                ));
            }
            out.push((name.clone(), written));
            index = next;
        } else if word(index) == Some("contract")
            && word(index + 1) == Some("inherit")
            && symbol(index + 3, Symbol::ColonColon)
            && let (Some(name), Some(namespace), Some(function)) =
                (&current, word(index + 2), word(index + 4))
        {
            out.push((
                name.clone(),
                WrittenDependency {
                    target_namespace: namespace.to_owned(),
                    target_function: function.to_owned(),
                    form: DependencyForm::Inherit,
                },
            ));
            index += 4;
        }
        index += 1;
    }
    if let Some(unused) = imports
        .iter()
        .find(|import| !used.contains(&import.function))
    {
        return Err(refusal(
            LinkageCode::Ql006,
            format!(
                "`#include {}::{} as quant #link;` is used by no link of this module",
                unused.namespace, unused.function
            ),
        ));
    }
    Ok(out)
}

/// The `contract_dependencies` entries of one crate's modules as a load-path
/// manifest records them, each with the target crate's version as `since`
/// and no digest: the record of a crate whose targets no lock reviews (a
/// test fixture, an unreviewed crate), which therefore resolves `Pending`.
/// `owner` maps a namespace to its load-path crate and that crate's version;
/// a qualified inherit directive of the crate's own namespace is no
/// dependency. `providers/contracts.py sync` writes the same entries with
/// the reviewed digest.
///
/// # Errors
///
/// A module [`scan_module`] refuses, a namespace `owner` does not know
/// (`QL007`), or a version that does not parse.
pub fn unreviewed_manifest_entries(
    crate_name: &str,
    namespace: &str,
    modules: &[(&str, &str)],
    owner: &dyn Fn(&str) -> Option<(String, String)>,
) -> Result<Vec<DeclaredDependency>, String> {
    let mut sorted = modules.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    let mut out = Vec::new();
    for (module, text) in sorted {
        for (function, written) in scan_module(text)? {
            if written.form == DependencyForm::Inherit && written.target_namespace == namespace {
                continue;
            }
            let (target, version) = owner(&written.target_namespace).ok_or_else(|| {
                refusal(
                    LinkageCode::Ql007,
                    format!(
                        "`{}::{}`: no load-path crate derives the namespace",
                        written.target_namespace, written.target_function
                    ),
                )
            })?;
            let mut dependency = written.of_target(crate_name, &function, module, &target);
            dependency.since = Some(
                qcore_schemas::library_version::LibraryVersion::parse(&version)
                    .map_err(|error| format!("{target}: {error}"))?,
            );
            out.push(dependency);
        }
    }
    Ok(out)
}

/// The module-level link imports at the head of a module, after `module
/// contract;` among the `#include` directives.
fn link_imports_of(kinds: &[&TokenKind]) -> Result<Vec<LinkImport>, String> {
    let word = |index: usize| match kinds.get(index) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    };
    let symbol =
        |index: usize, wanted: Symbol| kinds.get(index) == Some(&&TokenKind::Symbol(wanted));
    let mut imports: Vec<LinkImport> = Vec::new();
    for index in 0..kinds.len() {
        if !(symbol(index, Symbol::Hash) && word(index + 1) == Some("include")) {
            continue;
        }
        if matches!(kinds.get(index + 2), Some(TokenKind::StringLiteral(_))) {
            continue;
        }
        let shape = word(index + 2).is_some_and(lowercase_identifier)
            && symbol(index + 3, Symbol::ColonColon)
            && word(index + 4).is_some_and(c_identifier)
            && word(index + 5) == Some("as")
            && word(index + 6) == Some("quant")
            && symbol(index + 7, Symbol::Hash)
            && word(index + 8) == Some("link")
            && symbol(index + 9, Symbol::Semicolon);
        if !shape {
            return Err(refusal(
                LinkageCode::Ql006,
                "a module-level link import is `#include NS::NAME as quant #link;`",
            ));
        }
        let import = LinkImport {
            namespace: word(index + 2).unwrap_or_default().to_owned(),
            function: word(index + 4).unwrap_or_default().to_owned(),
        };
        if imports
            .iter()
            .any(|known| known.function == import.function)
        {
            return Err(refusal(
                LinkageCode::Ql006,
                format!("`{}` is imported for linking twice", import.function),
            ));
        }
        imports.push(import);
    }
    Ok(imports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_names_one_qualified_declaration() {
        let link = link_target("libc::malloc").unwrap();
        assert_eq!(link.target_namespace, "libc");
        assert_eq!(link.target_function, "malloc");
        assert_eq!(link.form, DependencyForm::Link);
        for bad in ["malloc", "libc::a::b", "Libc::malloc", "libc :: malloc ,"] {
            let error = link_target(bad).unwrap_err();
            assert!(error.starts_with("QL003"), "{bad}: {error}");
        }
    }

    #[test]
    fn the_marker_and_the_links_agree() {
        let link = link_target("libc::free").unwrap();
        assert!(check_declaration("f", 1, std::slice::from_ref(&link)).is_ok());
        assert!(
            check_declaration("f", 0, std::slice::from_ref(&link))
                .unwrap_err()
                .starts_with("QL001")
        );
        assert!(
            check_declaration("f", 1, &[])
                .unwrap_err()
                .starts_with("QL002")
        );
        assert!(
            check_declaration("f", 2, std::slice::from_ref(&link))
                .unwrap_err()
                .starts_with("QL002")
        );
        assert!(
            check_declaration("f", 1, &[link.clone(), link])
                .unwrap_err()
                .starts_with("QL004")
        );
        assert!(check_declaration("f", 0, &[]).is_ok());
    }

    #[test]
    fn the_written_form_is_retired() {
        assert!(
            refuse_retired("resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, since \"0.1.0\")")
                .unwrap_err()
                .starts_with("QL005")
        );
        assert!(refuse_retired("resource_borrowed(x)").is_ok());
    }

    #[test]
    fn a_block_wrapper_is_removed_with_its_terminator() {
        let kinds = |text: &str| {
            lex(text)
                .unwrap()
                .into_iter()
                .map(|t| t.kind)
                .filter(|k| *k != TokenKind::Eof)
                .collect::<Vec<_>>()
        };
        let grouped = kinds(
            "contract { requires a; premises { link { quant #link libc::free; }; requires b; };              ensures normal c; };",
        );
        let flat = kinds(
            "contract { requires a; quant #link libc::free; requires b; ensures normal c; };",
        );
        assert_eq!(without_linkage_blocks(&grouped), flat);
        assert_eq!(without_linkage_blocks(&flat), flat);
    }

    #[test]
    fn linkage_is_not_inherited() {
        let clauses = vec![
            ("requires".to_owned(), "resource_contract_v1".to_owned()),
            ("requires".to_owned(), LINKAGE_MARKER.to_owned()),
            (LINK_INTRODUCER.to_owned(), "libc::free".to_owned()),
            (
                "ensures normal".to_owned(),
                "resource_borrowed(x)".to_owned(),
            ),
        ];
        let kept = strip_linkage(&clauses);
        assert_eq!(kept.len(), 2);
        assert!(
            kept.iter()
                .all(|(i, e)| i != LINK_INTRODUCER && e != LINKAGE_MARKER)
        );
    }

    #[test]
    fn a_module_scan_reads_every_form() {
        let text = "module contract;\n#include \"quirin-provider-libc:libc/memory.qv\";\n\
            #include libc::free as quant #link;\n\n\
            extern fn acme_new(size: usize) -> ptr<mut, u8>\ncontract {\n    \
            requires resource_contract_v1;\n    requires linkage_contract_v1;\n    \
            premises {\n        link {\n            quant #link libc::malloc;\n            \
            quant #link free;\n        };\n    };\n    ensures normal resource_borrowed(size);\n};\n\n\
            extern fn acme_alloc(size: usize) -> ptr<mut, u8> contract inherit libc::malloc;\n\
            extern fn acme_own(size: usize) -> ptr<mut, u8> contract inherit acme_new;\n";
        let scanned = scan_module(text).unwrap();
        let read: Vec<(&str, &str, &str, DependencyForm)> = scanned
            .iter()
            .map(|(f, w)| {
                (
                    f.as_str(),
                    w.target_namespace.as_str(),
                    w.target_function.as_str(),
                    w.form,
                )
            })
            .collect();
        assert_eq!(
            read,
            vec![
                ("acme_new", "libc", "malloc", DependencyForm::Link),
                ("acme_new", "libc", "free", DependencyForm::Link),
                ("acme_alloc", "libc", "malloc", DependencyForm::Inherit),
            ]
        );
        assert!(scan_module("module contract;\n").unwrap().is_empty());
    }

    #[test]
    fn module_level_imports_are_exact() {
        let decl = "extern fn f(x: usize) -> unit\ncontract {\n    requires resource_contract_v1;\n    \
            requires linkage_contract_v1;\n    quant #link free;\n    ensures normal resource_borrowed(x);\n    \
            ensures normal resource_return_none();\n};\n";
        // Unused import.
        let unused = format!(
            "module contract;\n#include libc::free as quant #link;\n#include libc::malloc as quant #link;\n{decl}"
        );
        assert!(scan_module(&unused).unwrap_err().starts_with("QL006"));
        // Repeated import.
        let repeated = format!(
            "module contract;\n#include libc::free as quant #link;\n#include libc::free as quant #link;\n{decl}"
        );
        assert!(scan_module(&repeated).unwrap_err().starts_with("QL006"));
        // A bare link with no import.
        let bare = format!("module contract;\n{decl}");
        assert!(scan_module(&bare).unwrap_err().starts_with("QL003"));
        // An import of a name the module declares.
        let own = format!(
            "module contract;\n#include libc::f as quant #link;\n{}",
            decl.replace("quant #link free;", "quant #link f;")
        );
        assert!(scan_module(&own).unwrap_err().starts_with("QL006"));
        // Malformed.
        let malformed = format!("module contract;\n#include libc::free as #link;\n{decl}");
        assert!(scan_module(&malformed).unwrap_err().starts_with("QL006"));
    }
}
