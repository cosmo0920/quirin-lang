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

//! The OpenSSL contract checker (cards **OPENSSL-PROVIDER-PLUGIN** and
//! **OPENSSL-PROVIDER-REMAINING-CALLEES**).
//!
//! The audit sends one request per binding (`quirin-plugin.json`): one
//! `parameter_mode_respected` obligation per bound function, bound to the
//! module's text digest. For each function the checker decides:
//!
//! 1. **The module** is one this checker was built with ([`crate::MODULES`]):
//!    its SHA-256 is the request's `source_digest`.
//! 2. **The prototype**: the pinned headers declare the function
//!    ([`crate::PINNED_PROTOTYPES`]), and the declaration's parameter and
//!    result kinds equal the contract's ([`contract_type`]: a C pointer to
//!    const is `ptr<const, u8>`, any other object pointer `ptr<mut, u8>`, a
//!    pointer to a pointer `ptr<mut, ptr<B, u8>>` with `B` the innermost
//!    pointee's qualifier, a pointer to a function typedef `fnptr`, `int`
//!    `i32`, `long` `i64`, `unsigned long` and `uint64_t` `u64`, `size_t`
//!    `usize`, `void` `unit`).
//! 3. **The version range**: the declaration states exactly one
//!    `resource_library_version(openssl, …)` and it admits the pinned headers'
//!    version (3.5.5).
//! 4. **The documented semantics** ([`DOCUMENTED`]): the range is the one the
//!    official manual page supports, that page was fetched
//!    ([`crate::DOC_SNAPSHOTS`]), and the declaration's clauses are exactly
//!    the set [`expected_clauses`] derives from the documented shape and
//!    premises: a release of a reference-counted object is a reference drop
//!    (never a final free), a constructor of one returns a reference that may
//!    be NULL, a getter returning what its argument owns is a borrowed result,
//!    a holder keeping a pointer is a retention, every other position is
//!    borrowed, and nothing else is stated. Card
//!    **REFERENCE-DROP-NULL-ACCEPTING**: a release whose page states that a
//!    NULL argument does nothing is a NULL-accepting drop,
//!    `resource_reference_dropped(p, nullable)`, and the cited snapshot must
//!    record that exact sentence; the range's series is then the oldest one
//!    whose page states it.
//!
//! Every function passing all four is answered with a `Safe` **claim** and no
//! certificate, one diagnostic per function naming what was checked; the
//! audit's firewall publishes it `Pending` (`plugin-claim-uncertified`). Any
//! failure answers `Unknown` with the failure as a diagnostic, which the
//! firewall publishes `Unknown`. Neither is `Safe`. The checker reads only
//! what is compiled into it; it opens no file and no network connection.

use qcore_schemas::library_version::LibraryVersion;
use qcore_schemas::plugin::{
    PluginDiagnostic, PluginManifest, ProposedVerdict, VerificationRequestV2,
    VerificationResponseV2,
};
use serde_json::Value;

pub use crate::documented::{
    DOCUMENTED, Documented, DocumentedFunction, Null, Premise, REGISTERED_CALLBACKS,
    REGISTERED_CALLBACKS_STATEMENT,
};

/// The crate name a request's translation unit carries.
pub const CRATE: &str = "quirin-provider-openssl";

/// One declaration of a module, read lexically: its parameters' types, its
/// result type and its clauses (comments removed, whitespace collapsed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    pub parameters: Vec<(String, String)>,
    pub result: String,
    pub clauses: Vec<String>,
}

fn strip_comments(text: &str) -> String {
    text.lines()
        .map(|line| line.split("//").next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The declaration of `function` in a module's text.
#[must_use]
pub fn declaration(module: &str, function: &str) -> Option<Declaration> {
    let text = strip_comments(module);
    let start = text.find(&format!("extern fn {function}("))?;
    let rest = &text[start + format!("extern fn {function}(").len()..];
    let close = rest.find(')')?;
    // Split at the commas outside `<…>` (`ptr<mut, u8>` holds one).
    let mut pieces = Vec::new();
    let (mut depth, mut start) = (0usize, 0usize);
    let list = &rest[..close];
    for (index, character) in list.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                pieces.push(&list[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    pieces.push(&list[start..]);
    let parameters = pieces
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (name, ty) = p.split_once(':')?;
            Some((name.trim().to_owned(), collapse(ty)))
        })
        .collect::<Option<Vec<_>>>()?;
    let after = &rest[close + 1..];
    let result = collapse(after.split_once("->")?.1.split("contract").next()?);
    let body_start = after.find('{')?;
    let body_end = after.find("};")?;
    let clauses = after[body_start + 1..body_end]
        .split(';')
        .map(collapse)
        .filter(|clause| !clause.is_empty())
        .collect();
    Some(Declaration {
        parameters,
        result,
        clauses,
    })
}

/// The pinned headers' typedefs of function types, whose pointers are
/// `fnptr`.
pub const FUNCTION_TYPEDEFS: &[&str] = &["pem_password_cb"];

/// The contract type a C type from a pinned prototype corresponds to.
#[must_use]
pub fn contract_type(c_type: &str) -> Option<&'static str> {
    let c_type = collapse(c_type);
    let stars = c_type.matches('*').count();
    let base = c_type.trim_end_matches(['*', ' ']);
    if FUNCTION_TYPEDEFS.contains(&base) {
        // A pointer to a function type is `fnptr`; a pointer to that pointer
        // has no spelling.
        return (stars == 1).then_some("fnptr");
    }
    match stars {
        0 => {}
        1 if c_type.ends_with('*') => {
            return Some(if c_type.starts_with("const ") {
                "ptr<const, u8>"
            } else {
                "ptr<mut, u8>"
            });
        }
        // `T **` / `const T **`: the slot is unqualified, the innermost
        // pointee carries the qualifier (C04.1, rule 12v).
        2 if c_type.ends_with("**") && !c_type[..c_type.len() - 2].contains('*') => {
            return Some(if c_type.starts_with("const ") {
                "ptr<mut, ptr<const, u8>>"
            } else {
                "ptr<mut, ptr<mut, u8>>"
            });
        }
        _ => return None,
    }
    match c_type.as_str() {
        "void" => Some("unit"),
        "int" => Some("i32"),
        "long" => Some("i64"),
        "unsigned long" | "uint64_t" => Some("u64"),
        "size_t" => Some("usize"),
        _ => None,
    }
}

/// A pinned declaration's (parameter types, result type), as C types.
#[must_use]
pub fn prototype_types(declaration: &str, function: &str) -> Option<(Vec<String>, String)> {
    let open = declaration.find(&format!("{function}("))?;
    let result = declaration[..open].trim().to_owned();
    let inner = declaration[open + function.len() + 1..].strip_suffix(')')?;
    let parameters = if inner.trim() == "void" {
        Vec::new()
    } else {
        inner
            .split(',')
            .map(|parameter| {
                // Drop a trailing parameter name, if any: the identifier
                // after the last space or `*`, when a type remains before it.
                let parameter = parameter.trim();
                let cut = parameter.rfind([' ', '*']).map_or(0, |index| index + 1);
                let (base, name) = parameter.split_at(cut);
                if !base.trim().is_empty()
                    && !name.is_empty()
                    && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                {
                    collapse(base)
                } else {
                    collapse(parameter)
                }
            })
            .collect()
    };
    Some((parameters, result))
}

/// Decide one function; `Ok(note)` when every check passes.
///
/// # Errors
///
/// The first check that fails, as the diagnostic text.
pub fn check_function(module_path: &str, module: &str, function: &str) -> Result<String, String> {
    let documented = DOCUMENTED
        .iter()
        .find(|entry| entry.function == function)
        .ok_or_else(|| format!("{function}: no documented semantics in this checker"))?;
    let declaration = declaration(module, function)
        .ok_or_else(|| format!("{function}: `{module_path}` does not declare it"))?;
    let pinned: Value = serde_json::from_str(crate::PINNED_PROTOTYPES)
        .map_err(|error| format!("pinned prototypes: {error}"))?;
    let pinned_version = pinned["version"]
        .as_str()
        .and_then(|text| LibraryVersion::parse(text).ok())
        .ok_or("pinned prototypes carry no version")?;
    let prototype = pinned["prototypes"][function]["declaration"]
        .as_str()
        .ok_or_else(|| format!("{function}: the pinned headers do not declare it"))?;
    let header = pinned["prototypes"][function]["header"]
        .as_str()
        .unwrap_or_default();
    let header_sha256 = pinned["headers"][header].as_str().unwrap_or_default();
    // 2. The prototype.
    let (c_parameters, c_result) = prototype_types(prototype, function)
        .ok_or_else(|| format!("{function}: unreadable pinned prototype `{prototype}`"))?;
    let expected_result = contract_type(&c_result)
        .ok_or_else(|| format!("{function}: unmapped C result type `{c_result}`"))?;
    if declaration.result != expected_result {
        return Err(format!(
            "{function}: the contract returns `{}` but the pinned prototype `{prototype}` returns \
             `{expected_result}`",
            declaration.result
        ));
    }
    if c_parameters.len() != declaration.parameters.len() {
        return Err(format!(
            "{function}: the contract has {} parameter(s), the pinned prototype `{prototype}` {}",
            declaration.parameters.len(),
            c_parameters.len()
        ));
    }
    for ((name, ty), c_type) in declaration.parameters.iter().zip(&c_parameters) {
        let expected = contract_type(c_type)
            .ok_or_else(|| format!("{function}: unmapped C parameter type `{c_type}`"))?;
        if ty != expected {
            return Err(format!(
                "{function}: parameter `{name}` is `{ty}` but the pinned prototype `{prototype}` \
                 makes it `{expected}`"
            ));
        }
    }
    // 3. The version range.
    let ranges = declaration
        .clauses
        .iter()
        .filter(|clause| clause.starts_with("requires resource_library_version("))
        .collect::<Vec<_>>();
    let [range] = ranges.as_slice() else {
        return Err(format!(
            "{function}: the declaration must state exactly one library version range"
        ));
    };
    let range = range.trim_start_matches("requires ").to_owned();
    if range != documented.range {
        return Err(format!(
            "{function}: the declaration states `{range}` but the documentation supports `{}`",
            documented.range
        ));
    }
    let bound = parse_range(&range).ok_or_else(|| format!("{function}: unreadable `{range}`"))?;
    if !(bound.0 <= pinned_version && bound.1.is_none_or(|removed| pinned_version < removed)) {
        return Err(format!(
            "{function}: `{range}` does not admit the pinned OpenSSL {pinned_version}"
        ));
    }
    // 4. The documented semantics.
    let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS)
        .map_err(|error| format!("doc snapshots: {error}"))?;
    let page = snapshots["pages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|page| {
            page["page"] == documented.page
                && page["series"] == documented.series
                && page["fetched"] == true
        })
        .ok_or_else(|| {
            format!(
                "{function}: no fetched snapshot of {} at series {}",
                documented.page, documented.series
            )
        })?;
    let names = declaration
        .parameters
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let expected = expected_clauses(documented, &names).ok_or_else(|| {
        format!("{function}: the documented shape names a parameter the declaration lacks")
    })?;
    let mut stated = declaration.clauses.clone();
    stated.sort();
    if stated != expected {
        let missing = expected
            .iter()
            .filter(|clause| !stated.contains(clause))
            .cloned()
            .collect::<Vec<_>>();
        let extra = stated
            .iter()
            .filter(|clause| !expected.contains(clause))
            .cloned()
            .collect::<Vec<_>>();
        return Err(format!(
            "{function}: the contract does not state exactly the documented {} \
             (missing: {missing:?}; not documented: {extra:?})",
            describe(documented.shape)
        ));
    }
    // Card REFERENCE-DROP-NULL-ACCEPTING: a NULL-accepting drop stands only
    // on a snapshot that records the page's exact NULL sentence.
    if let Documented::NullAcceptingRelease { sentence, .. } = documented.shape {
        let stated = page["null_sentences"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|stated| stated.as_str() == Some(sentence));
        if !stated {
            return Err(format!(
                "{function}: the snapshot of {} at series {} does not record \"{sentence}\"",
                documented.page, documented.series
            ));
        }
    }
    Ok(format!(
        "{function}: prototype `{prototype}` ({header}, sha256 {}) matches the contract; `{range}` \
         admits the pinned OpenSSL {pinned_version}; custody as documented at {} (sha256 {} as \
         fetched)",
        &header_sha256[..header_sha256.len().min(12)],
        page["url"].as_str().unwrap_or_default(),
        &page["sha256"].as_str().unwrap_or_default()[..12],
    ))
}

/// What a shape is, for a diagnostic.
#[must_use]
pub fn describe(shape: Documented) -> &'static str {
    match shape {
        Documented::Plain => "plain call (no custody taken or returned)",
        Documented::CountedRelease(_) => {
            "counted release (one reference dropped, never a final free)"
        }
        Documented::NullAcceptingRelease { .. } => {
            "NULL-accepting counted release (one reference dropped, nothing done on NULL)"
        }
        Documented::CountedConstructor => "returned reference (one reference, or NULL)",
        Documented::Constructor => "fresh object (owned, or NULL)",
        Documented::Release(..) => "release of an object that is not reference counted",
        Documented::BorrowedResult(..) => "borrowed result (owned by the argument)",
        Documented::Retains { .. } => "retention (the holder keeps the pointer)",
    }
}

/// The exact clause set, sorted, of a declaration stating `documented` over
/// parameters named `parameters` (in order); `None` when the shape or a
/// premise names a position the declaration lacks.
#[must_use]
pub fn expected_clauses(
    documented: &DocumentedFunction,
    parameters: &[String],
) -> Option<Vec<String>> {
    let name = |position: usize| parameters.get(position).cloned();
    let mut clauses = vec![
        "requires resource_contract_v1".to_owned(),
        "requires resource_revision(flexible)".to_owned(),
        format!("requires {}", documented.range),
    ];
    for premise in documented.premises {
        clauses.push(match premise {
            Premise::NullArgument(position) => {
                format!("requires resource_literal_argument({}, 0)", name(*position)?)
            }
            Premise::RegisteredCallbacks => format!(
                "requires resource_premise({REGISTERED_CALLBACKS}, \"{REGISTERED_CALLBACKS_STATEMENT}\")"
            ),
            Premise::CallbackInvoked {
                callback,
                arguments,
            }
            | Premise::CallbackInvokedOrDefault {
                callback,
                arguments,
            } => {
                let mut actuals = vec![name(*callback)?];
                for argument in *arguments {
                    actuals.push(match argument {
                        Some(position) => name(*position)?,
                        None => "_".to_owned(),
                    });
                }
                let keyword = if matches!(premise, Premise::CallbackInvokedOrDefault { .. }) {
                    "resource_callback_invoked_or_default"
                } else {
                    "resource_callback_invoked"
                };
                format!("ensures normal {keyword}({})", actuals.join(", "))
            }
        });
    }
    for (position, parameter) in parameters.iter().enumerate() {
        clauses.push(match documented.shape {
            Documented::Release(released, null) if released == position => {
                format!(
                    "ensures normal resource_released({parameter}, {})",
                    null.word()
                )
            }
            _ => format!("ensures normal resource_borrowed({parameter})"),
        });
    }
    match documented.shape {
        Documented::Plain => clauses.push("ensures normal resource_return_none()".to_owned()),
        Documented::CountedRelease(position) => {
            clauses.push(format!(
                "ensures normal resource_reference_dropped({})",
                name(position)?
            ));
            clauses.push("ensures normal resource_return_none()".to_owned());
        }
        Documented::NullAcceptingRelease { position, .. } => {
            clauses.push(format!(
                "ensures normal resource_reference_dropped({}, nullable)",
                name(position)?
            ));
            clauses.push("ensures normal resource_return_none()".to_owned());
        }
        Documented::CountedConstructor => {
            clauses.push("ensures normal resource_return_owned(nullable)".to_owned());
            clauses.push("ensures normal resource_return_kind(reference)".to_owned());
        }
        Documented::Constructor => {
            clauses.push("ensures normal resource_return_owned(nullable)".to_owned());
        }
        Documented::Release(position, _) => {
            name(position)?;
            clauses.push("ensures normal resource_return_none()".to_owned());
        }
        Documented::BorrowedResult(position, null) => clauses.push(format!(
            "ensures normal resource_return_borrowed_from({}, {})",
            name(position)?,
            null.word()
        )),
        Documented::Retains { item, holder } => {
            clauses.push(format!(
                "ensures normal resource_retained_by({}, {})",
                name(item)?,
                name(holder)?
            ));
            clauses.push("ensures normal resource_return_none()".to_owned());
        }
    }
    clauses.sort();
    Some(clauses)
}

/// `(lower, removed)` of a range clause, read with the schema's version
/// parser.
#[must_use]
pub fn parse_range(range: &str) -> Option<(LibraryVersion, Option<LibraryVersion>)> {
    let inner = range
        .strip_prefix("resource_library_version(")?
        .strip_suffix(')')?;
    let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();
    let quoted = |text: &str| -> Option<LibraryVersion> {
        LibraryVersion::parse(text.strip_prefix('"')?.strip_suffix('"')?).ok()
    };
    let lower = |part: &str| {
        part.strip_prefix("since ")
            .or_else(|| part.strip_prefix("documented "))
            .and_then(quoted)
    };
    match parts.as_slice() {
        ["openssl", lower_part] => Some((lower(lower_part)?, None)),
        ["openssl", lower_part, removed] => Some((
            lower(lower_part)?,
            Some(quoted(removed.strip_prefix("removed ")?)?),
        )),
        _ => None,
    }
}

/// The checker's answer to one request: `Safe` (a claim) when every bound
/// function passes, `Unknown` otherwise, with one diagnostic per function.
#[must_use]
pub fn answer(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    let functions = quirin_provider_plugin::checker::requested_functions(request);
    if functions.is_empty() {
        return None;
    }
    let module = crate::MODULES.iter().find(|(path, text)| {
        request.unit.translation_unit == format!("{CRATE}:{path}")
            && quirin_provider_plugin::sha256_hex(text.as_bytes()) == request.unit.source_digest
    });
    let mut notes = Vec::new();
    let mut verdict = ProposedVerdict::Safe;
    match module {
        None => {
            verdict = ProposedVerdict::Unknown;
            notes.push(format!(
                "`{}` (sha256 {}) is not a module text this checker was built with",
                request.unit.translation_unit, request.unit.source_digest
            ));
        }
        Some((path, text)) => {
            for function in functions {
                match check_function(path, text, function) {
                    Ok(note) => notes.push(note),
                    Err(failure) => {
                        verdict = ProposedVerdict::Unknown;
                        notes.push(failure);
                    }
                }
            }
        }
    }
    let mut response = quirin_provider_plugin::checker::respond(
        request,
        manifest,
        verdict,
        "OpenSSL contract check (prototype, version range, documented semantics); a claim, not a \
         certificate",
        None,
    );
    response
        .diagnostics
        .extend(notes.into_iter().map(|message| PluginDiagnostic {
            code: format!("{}.binding", manifest.plugin_id),
            message,
            fact_id: None,
        }));
    Some(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_contract_passes_its_own_checks() {
        for documented in DOCUMENTED {
            let (path, text) = crate::MODULES
                .iter()
                .find(|(_, text)| text.contains(&format!("extern fn {}(", documented.function)))
                .unwrap();
            let note = check_function(path, text, documented.function)
                .unwrap_or_else(|failure| panic!("{failure}"));
            assert!(note.contains("admits the pinned OpenSSL 3.5.5"), "{note}");
        }
    }

    #[test]
    fn every_declared_function_is_documented_and_bound() {
        let manifest: Value = serde_json::from_str(include_str!("../quirin-plugin.json")).unwrap();
        let bound = manifest["checkers"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|binding| binding["functions"].as_array().unwrap().clone())
            .map(|function| function.as_str().unwrap().to_owned())
            .collect::<std::collections::BTreeSet<_>>();
        let mut declared = std::collections::BTreeSet::new();
        for (_, text) in crate::MODULES {
            for line in strip_comments(text).lines() {
                if let Some(rest) = line.trim().strip_prefix("extern fn ") {
                    declared.insert(rest.split('(').next().unwrap().to_owned());
                }
            }
        }
        let documented = DOCUMENTED
            .iter()
            .map(|entry| entry.function.to_owned())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            declared, bound,
            "every declaration is behind a checker binding"
        );
        assert_eq!(declared, documented);
    }

    #[test]
    fn a_drifted_contract_is_refused() {
        let (path, text) = crate::MODULES
            .iter()
            .find(|(path, _)| *path == "openssl/x509.qv")
            .unwrap();
        // A final free in place of a counted drop.
        let freed = text.replace(
            "resource_reference_dropped(a, nullable)",
            "resource_released(a, nullable)",
        );
        assert!(check_function(path, &freed, "X509_free").is_err());
        // Card REFERENCE-DROP-NULL-ACCEPTING: a drop that does not state the
        // documented NULL behaviour, and the NULL-accepting form from a series
        // whose page does not state it.
        let strict = text.replace(
            "resource_reference_dropped(a, nullable)",
            "resource_reference_dropped(a)",
        );
        assert!(check_function(path, &strict, "X509_free").is_err());
        let older = text.replace("documented \"1.1.1\"", "documented \"1.0.2\"");
        assert!(check_function(path, &older, "X509_free").is_err());
        // A range the documentation does not support.
        let ranged = text.replace("documented \"1.1.1\"", "since \"0.9.8\"");
        assert!(check_function(path, &ranged, "X509_free").is_err());
        // A range that excludes the pinned version.
        let removed = text.replace(
            "documented \"1.1.1\")",
            "documented \"1.1.1\", removed \"3.0.0\")",
        );
        assert!(check_function(path, &removed, "X509_free").is_err());
        // A signature the pinned prototype does not have.
        let typed = text.replace("X509_free(a: ptr<mut, u8>)", "X509_free(a: ptr<const, u8>)");
        assert!(check_function(path, &typed, "X509_free").is_err());
        // No range at all.
        let unranged = text.replace(
            "    requires resource_library_version(openssl, documented \"1.1.1\");\n",
            "",
        );
        assert!(check_function(path, &unranged, "X509_free").is_err());
    }

    /// Card REFERENCE-DROP-NULL-ACCEPTING: every NULL-accepting drop cites a
    /// fetched page at its range's series that records the NULL sentence,
    /// and no older series checked records it (the bound is the oldest).
    #[test]
    fn every_null_accepting_drop_cites_the_oldest_series_stating_it() {
        let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS).unwrap();
        let pages = snapshots["pages"].as_array().unwrap();
        let mut seen = 0;
        for documented in DOCUMENTED {
            let Documented::NullAcceptingRelease { sentence, .. } = documented.shape else {
                continue;
            };
            seen += 1;
            let bound = LibraryVersion::parse(documented.series).unwrap();
            for page in pages
                .iter()
                .filter(|page| page["page"] == documented.page && page["fetched"] == true)
            {
                let series = LibraryVersion::parse(page["series"].as_str().unwrap()).unwrap();
                let states = page["null_sentences"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|stated| stated.as_str() == Some(sentence));
                if series < bound {
                    assert!(!states, "{}: {} states it", documented.function, series);
                }
                if series == bound {
                    assert!(
                        states,
                        "{}: {} does not state it",
                        documented.function, series
                    );
                }
            }
        }
        assert_eq!(seen, 6);
    }

    #[test]
    fn prototypes_map_to_contract_types() {
        assert_eq!(
            prototype_types(
                "SSL_CTX *SSL_CTX_new(const SSL_METHOD *meth)",
                "SSL_CTX_new"
            ),
            Some((vec!["const SSL_METHOD *".into()], "SSL_CTX *".into()))
        );
        assert_eq!(
            prototype_types("void SSL_CTX_free(SSL_CTX *)", "SSL_CTX_free"),
            Some((vec!["SSL_CTX *".into()], "void".into()))
        );
        assert_eq!(
            prototype_types(
                "int OPENSSL_init_ssl(uint64_t opts, const OPENSSL_INIT_SETTINGS *settings)",
                "OPENSSL_init_ssl"
            ),
            Some((
                vec!["uint64_t".into(), "const OPENSSL_INIT_SETTINGS *".into()],
                "int".into()
            ))
        );
        assert_eq!(
            prototype_types("void ERR_clear_error(void)", "ERR_clear_error"),
            Some((vec![], "void".into()))
        );
        assert_eq!(contract_type("const SSL_METHOD *"), Some("ptr<const, u8>"));
        assert_eq!(contract_type("SSL *"), Some("ptr<mut, u8>"));
        // Card OPENSSL-PROVIDER-REMAINING-CALLEES: the shapes its prototypes
        // add, each exactly one spelling, and no reading of anything else.
        assert_eq!(contract_type("long"), Some("i64"));
        assert_eq!(contract_type("unsigned long"), Some("u64"));
        assert_eq!(contract_type("size_t"), Some("usize"));
        assert_eq!(contract_type("X509 **"), Some("ptr<mut, ptr<mut, u8>>"));
        assert_eq!(
            contract_type("const char **"),
            Some("ptr<mut, ptr<const, u8>>")
        );
        assert_eq!(contract_type("pem_password_cb *"), Some("fnptr"));
        assert_eq!(contract_type("char *const *"), None);
        assert_eq!(contract_type("X509 ***"), None);
        assert_eq!(contract_type("pem_password_cb **"), None);
        assert_eq!(contract_type("unsigned int"), None);
        assert_eq!(
            prototype_types(
                "X509 *PEM_read_bio_X509(BIO *out, X509 **x, pem_password_cb *cb, void *u)",
                "PEM_read_bio_X509"
            ),
            Some((
                vec![
                    "BIO *".into(),
                    "X509 **".into(),
                    "pem_password_cb *".into(),
                    "void *".into()
                ],
                "X509 *".into()
            ))
        );
        assert_eq!(
            prototype_types(
                "void *OPENSSL_sk_value(const OPENSSL_STACK *, int)",
                "OPENSSL_sk_value"
            ),
            Some((
                vec!["const OPENSSL_STACK *".into(), "int".into()],
                "void *".into()
            ))
        );
    }

    fn module_of(function: &str) -> (&'static str, &'static str) {
        let (path, text) = crate::MODULES
            .iter()
            .find(|(_, text)| text.contains(&format!("extern fn {function}(")))
            .unwrap();
        (path, text)
    }

    #[test]
    fn a_drifted_shape_of_the_new_readings_is_refused() {
        // A borrowed result read as an owned one: a getter whose result the
        // page says the caller must not free.
        let (path, text) = module_of("SSL_get_certificate");
        let owned = text.replacen(
            "ensures normal resource_return_borrowed_from(ssl, nullable);",
            "ensures normal resource_return_owned(nullable);",
            1,
        );
        assert!(check_function(path, &owned, "SSL_get_certificate").is_err());
        // A retention dropped: the borrowed-and-forgotten reading C04.14
        // forbids.
        let (path, text) = module_of("OPENSSL_sk_push");
        let forgotten = text.replace("    ensures normal resource_retained_by(data, st);\n", "");
        assert!(check_function(path, &forgotten, "OPENSSL_sk_push").is_err());
        // A returned reference read as a fresh heap object (a final free
        // would then discharge it).
        let (path, text) = module_of("X509_new");
        let heap = text.replacen(
            "    ensures normal resource_return_kind(reference);\n",
            "",
            1,
        );
        assert!(check_function(path, &heap, "X509_new").is_err());
        // The callbacks premise removed from an I/O call.
        let (path, text) = module_of("SSL_write");
        let premise = format!(
            "    requires resource_premise({REGISTERED_CALLBACKS}, \"{REGISTERED_CALLBACKS_STATEMENT}\");\n"
        );
        assert!(text.contains(&premise));
        let unpremised = text.replacen(&premise, "", 1);
        assert!(check_function(path, &unpremised, "SSL_write").is_err());
        // The null-argument premise removed: the reuse arm would read as the
        // fresh-object arm.
        let (path, text) = module_of("PEM_read_bio_X509");
        let widened = text.replacen("    requires resource_literal_argument(x, 0);\n", "", 1);
        assert!(check_function(path, &widened, "PEM_read_bio_X509").is_err());
        // Card OPENSSL-PEM-CALLBACK-ARM: the invocation removed (a caller's
        // password callback would read as a benign borrowed pointer), its
        // `u` actual dropped to `_`, the null default dropped (the plain
        // invocation), and the retired null-callback premise restored are
        // each refused.
        let invoked = "    ensures normal resource_callback_invoked_or_default(cb, _, _, _, u);\n";
        assert!(text.contains(invoked));
        assert!(!text.contains("resource_literal_argument(cb, 0)"));
        let benign = text.replacen(invoked, "", 1);
        assert!(check_function(path, &benign, "PEM_read_bio_X509").is_err());
        let unnamed = text.replacen(
            invoked,
            "    ensures normal resource_callback_invoked_or_default(cb, _, _, _, _);\n",
            1,
        );
        assert!(check_function(path, &unnamed, "PEM_read_bio_X509").is_err());
        let plain = text.replacen(
            invoked,
            "    ensures normal resource_callback_invoked(cb, _, _, _, u);\n",
            1,
        );
        assert!(check_function(path, &plain, "PEM_read_bio_X509").is_err());
        let narrowed = text.replacen(
            "    requires resource_literal_argument(x, 0);\n",
            "    requires resource_literal_argument(x, 0);\n    requires resource_literal_argument(cb, 0);\n",
            1,
        );
        assert!(check_function(path, &narrowed, "PEM_read_bio_X509").is_err());
        // A release of a non-counted object read as a counted drop, and a
        // counted drop read as a final free.
        let (path, text) = module_of("PKCS12_free");
        let counted = text.replacen(
            "ensures normal resource_released(a, nullable);",
            "ensures normal resource_borrowed(a);\n    ensures normal resource_reference_dropped(a);",
            1,
        );
        assert!(check_function(path, &counted, "PKCS12_free").is_err());
        let (path, text) = module_of("X509_STORE_free");
        let freed = text.replacen(
            "    ensures normal resource_borrowed(xs);\n    ensures normal resource_reference_dropped(xs);\n",
            "    ensures normal resource_released(xs, nullable);\n",
            1,
        );
        assert!(check_function(path, &freed, "X509_STORE_free").is_err());
        // A since bound the HISTORY section does not give.
        let (path, text) = module_of("SSL_CTX_set_options");
        let older = text.replacen("since \"3.0.0\"", "documented \"1.0.2\"", 1);
        assert!(check_function(path, &older, "SSL_CTX_set_options").is_err());
    }

    #[test]
    fn every_documented_page_was_fetched() {
        let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS).unwrap();
        for documented in DOCUMENTED {
            assert!(
                snapshots["pages"].as_array().unwrap().iter().any(|page| {
                    page["page"] == documented.page
                        && page["series"] == documented.series
                        && page["fetched"] == true
                }),
                "{}: {}@{}",
                documented.function,
                documented.page,
                documented.series
            );
        }
    }
}
