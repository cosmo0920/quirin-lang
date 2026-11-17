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

//! The OpenSSL contract checker (card **OPENSSL-PROVIDER-PLUGIN**).
//!
//! The audit sends one request per binding (`quirin-plugin.json`): one
//! `parameter_mode_respected` obligation per bound function, bound to the
//! module's text digest. For each function the checker decides:
//!
//! 1. **The module** is one this checker was built with ([`crate::MODULES`]):
//!    its SHA-256 is the request's `source_digest`.
//! 2. **The prototype**: the pinned headers declare the function
//!    ([`crate::PINNED_PROTOTYPES`]), and the declaration's parameter and
//!    result kinds equal the contract's (a C pointer to const is
//!    `ptr<const, u8>`, any other pointer `ptr<mut, u8>`, `int` `i32`,
//!    `uint64_t` `u64`, `void` `unit`).
//! 3. **The version range**: the declaration states exactly one
//!    `resource_library_version(openssl, …)` and it admits the pinned headers'
//!    version (3.5.5).
//! 4. **The documented semantics** ([`DOCUMENTED`]): the range is the one the
//!    official manual page supports, that page was fetched
//!    ([`crate::DOC_SNAPSHOTS`]), and the custody clauses match what the page
//!    documents: a release of a reference-counted object is a reference drop
//!    (never a final free), a constructor's result is a returned reference
//!    that may be NULL, and a function that takes or returns nothing has no
//!    custody clause. Card **REFERENCE-DROP-NULL-ACCEPTING**: a release whose
//!    page states that a NULL argument does nothing is a NULL-accepting drop,
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

/// The custody shape the official documentation gives a function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Documented {
    /// Takes and returns no custody.
    Plain,
    /// Releases one reference on a reference-counted object (the parameter
    /// at this position): a reference drop.
    CountedRelease(usize),
    /// Creates a reference-counted object and returns one reference, or NULL
    /// on failure.
    CountedConstructor,
    /// Card **REFERENCE-DROP-NULL-ACCEPTING**: a counted release (the
    /// parameter at `position`) whose page states, in `sentence`, that a NULL
    /// argument does nothing: a NULL-accepting reference drop.
    NullAcceptingRelease {
        position: usize,
        sentence: &'static str,
    },
}

/// What the documentation supports for one function: the page cited, the
/// series establishing the lower bound, the range clause it supports, and the
/// custody shape.
#[derive(Clone, Copy, Debug)]
pub struct DocumentedFunction {
    pub function: &'static str,
    pub page: &'static str,
    pub series: &'static str,
    pub range: &'static str,
    pub shape: Documented,
}

/// The documented semantics of every contracted function (user decision
/// 2026-11-16: the official manual pages; `doc-snapshots.json`).
pub const DOCUMENTED: &[DocumentedFunction] = &[
    DocumentedFunction {
        function: "BIO_free",
        page: "BIO_new",
        series: "1.1.1",
        range: "resource_library_version(openssl, documented \"1.1.1\")",
        shape: Documented::NullAcceptingRelease {
            position: 0,
            sentence: "If a is NULL nothing is done.",
        },
    },
    DocumentedFunction {
        function: "ENGINE_free",
        page: "ENGINE_add",
        series: "3.0",
        range: "resource_library_version(openssl, documented \"3.0\")",
        shape: Documented::NullAcceptingRelease {
            position: 0,
            sentence: "If the argument to ENGINE_free() is NULL, nothing is done.",
        },
    },
    DocumentedFunction {
        function: "ERR_clear_error",
        page: "ERR_clear_error",
        series: "1.0.2",
        range: "resource_library_version(openssl, documented \"1.0.2\")",
        shape: Documented::Plain,
    },
    DocumentedFunction {
        function: "EVP_PKEY_free",
        page: "EVP_PKEY_new",
        series: "1.1.1",
        range: "resource_library_version(openssl, documented \"1.1.1\")",
        shape: Documented::NullAcceptingRelease {
            position: 0,
            sentence: "If key is NULL, nothing is done.",
        },
    },
    DocumentedFunction {
        function: "OPENSSL_init_ssl",
        page: "OPENSSL_init_ssl",
        series: "3.5",
        range: "resource_library_version(openssl, since \"1.1.0\")",
        shape: Documented::Plain,
    },
    DocumentedFunction {
        function: "SSL_CTX_free",
        page: "SSL_CTX_free",
        series: "1.1.1",
        range: "resource_library_version(openssl, documented \"1.1.1\")",
        shape: Documented::NullAcceptingRelease {
            position: 0,
            sentence: "If ctx is NULL nothing is done.",
        },
    },
    DocumentedFunction {
        function: "SSL_CTX_new",
        page: "SSL_CTX_new",
        series: "1.0.2",
        range: "resource_library_version(openssl, documented \"1.0.2\")",
        shape: Documented::CountedConstructor,
    },
    DocumentedFunction {
        function: "SSL_free",
        page: "SSL_free",
        series: "1.1.1",
        range: "resource_library_version(openssl, documented \"1.1.1\")",
        shape: Documented::NullAcceptingRelease {
            position: 0,
            sentence: "If ssl is NULL nothing is done.",
        },
    },
    DocumentedFunction {
        function: "SSL_new",
        page: "SSL_new",
        series: "1.0.2",
        range: "resource_library_version(openssl, documented \"1.0.2\")",
        shape: Documented::CountedConstructor,
    },
    DocumentedFunction {
        function: "SSL_shutdown",
        page: "SSL_shutdown",
        series: "1.0.2",
        range: "resource_library_version(openssl, documented \"1.0.2\")",
        shape: Documented::Plain,
    },
    DocumentedFunction {
        function: "TLS_server_method",
        page: "SSL_CTX_new",
        series: "3.5",
        range: "resource_library_version(openssl, since \"1.1.0\")",
        shape: Documented::Plain,
    },
    DocumentedFunction {
        function: "X509_free",
        page: "X509_new",
        series: "1.1.1",
        range: "resource_library_version(openssl, documented \"1.1.1\")",
        shape: Documented::NullAcceptingRelease {
            position: 0,
            sentence: "If a is NULL nothing is done.",
        },
    },
];

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

/// The contract type a C type from a pinned prototype corresponds to.
#[must_use]
pub fn contract_type(c_type: &str) -> Option<&'static str> {
    let c_type = collapse(c_type);
    if c_type.ends_with('*') {
        return Some(if c_type.starts_with("const ") {
            "ptr<const, u8>"
        } else {
            "ptr<mut, u8>"
        });
    }
    match c_type.as_str() {
        "void" => Some("unit"),
        "int" => Some("i32"),
        "uint64_t" => Some("u64"),
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
    let has = |clause: &str| declaration.clauses.iter().any(|c| c == clause);
    let custody = declaration
        .clauses
        .iter()
        .filter(|c| {
            c.contains("resource_released")
                || c.contains("resource_reference_")
                || c.contains("resource_return_owned")
                || c.contains("resource_return_kind")
        })
        .count();
    match documented.shape {
        Documented::Plain => {
            if custody != 0 || !has("ensures normal resource_return_none()") {
                return Err(format!(
                    "{function}: documented to take and return no custody, but the contract \
                     states custody"
                ));
            }
        }
        Documented::CountedRelease(position) => {
            let (name, _) = &declaration.parameters[position];
            if custody != 1
                || !has(&format!(
                    "ensures normal resource_reference_dropped({name})"
                ))
            {
                return Err(format!(
                    "{function}: documented to release one reference on `{name}` (a counted \
                     drop, never a final free), and the contract does not state exactly that"
                ));
            }
        }
        Documented::NullAcceptingRelease { position, sentence } => {
            let (name, _) = &declaration.parameters[position];
            if custody != 1
                || !has(&format!(
                    "ensures normal resource_reference_dropped({name}, nullable)"
                ))
            {
                return Err(format!(
                    "{function}: documented to release one reference on `{name}` and to do \
                     nothing on NULL (a NULL-accepting counted drop), and the contract does not \
                     state exactly that"
                ));
            }
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
        Documented::CountedConstructor => {
            if custody != 2
                || !has("ensures normal resource_return_owned(nullable)")
                || !has("ensures normal resource_return_kind(reference)")
            {
                return Err(format!(
                    "{function}: documented to return one reference or NULL, and the contract \
                     does not state exactly that"
                ));
            }
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
        assert_eq!(contract_type("long"), None);
    }
}
