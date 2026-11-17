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

//! The libcurl contract checker (card **LIBCURL-PROVIDER-PLUGIN**).
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
//!    `ptr<const, u8>`, any other pointer `ptr<mut, u8>`, `int` `i32`, `long`
//!    `i64` (LP64), `void` `unit`, and an enumeration the integer type the
//!    pinned record establishes for it: `CURLcode` is `unsigned int`, `u32`).
//! 3. **The version range**: the declaration states exactly one
//!    `resource_library_version(curl, …)`, it is `since` the version the
//!    fetched manual page's AVAILABILITY section names, and it admits the
//!    pinned headers' version (8.18.0).
//! 4. **The documented custody** ([`DOCUMENTED`]): the page was fetched
//!    ([`crate::DOC_SNAPSHOTS`]) and carries the sentences the shape rests
//!    on, and the custody clauses are exactly that shape: a function taking
//!    and returning no custody states none; a fresh allocation is a nullable
//!    owned result; a release accepting NULL is a nullable release of that
//!    parameter; an append that returns the (possibly new) list head is the
//!    reallocation reading. Every other parameter is borrowed, and the only
//!    `requires` clauses are the contract version, `flexible` and the range.
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

/// The library name the range clauses and `library-versions.json` use.
pub const LIBRARY: &str = "curl";

/// The custody shape the official documentation gives a function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Documented {
    /// Takes and returns no custody.
    Plain,
    /// Returns a fresh allocation the caller releases, or NULL on failure.
    OwnedResult,
    /// Releases the parameter at this position; NULL is accepted and does
    /// nothing.
    NullableRelease(usize),
    /// Appends to the list at this position (NULL creates one) and returns
    /// the list head, or NULL on failure with the list left to the caller.
    Reallocating(usize),
}

/// What the documentation supports for one function: the page cited, the
/// range clause it supports, the custody shape and the sentences of the
/// fetched page the shape rests on.
#[derive(Clone, Copy, Debug)]
pub struct DocumentedFunction {
    pub function: &'static str,
    pub page: &'static str,
    pub range: &'static str,
    pub shape: Documented,
    pub evidence: &'static [&'static str],
}

/// The documented semantics of every contracted function (user decision
/// 2026-11-17: the official manual pages; `doc-snapshots.json`).
pub const DOCUMENTED: &[DocumentedFunction] = &[
    DocumentedFunction {
        function: "curl_easy_escape",
        page: "curl_easy_escape",
        range: "resource_library_version(curl, since \"7.15.4\")",
        shape: Documented::OwnedResult,
        evidence: &[
            "returns that as a new allocated string",
            "You must curl_free the returned string",
            "NULL if it failed",
        ],
    },
    DocumentedFunction {
        function: "curl_global_init",
        page: "curl_global_init",
        range: "resource_library_version(curl, since \"7.8\")",
        shape: Documented::Plain,
        evidence: &[],
    },
    DocumentedFunction {
        function: "curl_slist_append",
        page: "curl_slist_append",
        range: "resource_library_version(curl, since \"7.1\")",
        shape: Documented::Reallocating(0),
        evidence: &[
            "Pass in NULL in the list argument to create a new list",
            "curl_slist_append copies the string",
            "A null pointer is returned if anything went wrong, otherwise the new list pointer \
             is returned",
        ],
    },
    DocumentedFunction {
        function: "curl_slist_free_all",
        page: "curl_slist_free_all",
        range: "resource_library_version(curl, since \"7.1\")",
        shape: Documented::NullableRelease(0),
        evidence: &[
            "Passing in a NULL pointer in list makes this function return immediately with no \
             action",
            "removes all traces of a previously built curl_slist linked list",
        ],
    },
    DocumentedFunction {
        function: "curl_url",
        page: "curl_url",
        range: "resource_library_version(curl, since \"7.62.0\")",
        shape: Documented::OwnedResult,
        evidence: &[
            "This function allocates a URL object",
            "The object must be destroyed with a call to curl_url_cleanup",
            "NULL if out of memory",
        ],
    },
    DocumentedFunction {
        function: "curl_url_cleanup",
        page: "curl_url_cleanup",
        range: "resource_library_version(curl, since \"7.62.0\")",
        shape: Documented::NullableRelease(0),
        evidence: &[
            "Frees all the resources associated with the given CURLU handle",
            "Passing in a NULL pointer in handle makes this function return immediately with no \
             action",
        ],
    },
];

/// The crate name a request's translation unit carries.
pub const CRATE: &str = "quirin-provider-libcurl";

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
    let opening = format!("extern fn {function}(");
    let start = text.find(&opening)?;
    let rest = &text[start + opening.len()..];
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

/// The contract type a C type from a pinned prototype corresponds to, given
/// the integer types the pinned record establishes for enumerations.
#[must_use]
pub fn contract_type(c_type: &str, enums: &Value) -> Option<&'static str> {
    let c_type = collapse(c_type);
    if c_type.ends_with('*') {
        return Some(if c_type.starts_with("const ") {
            "ptr<const, u8>"
        } else {
            "ptr<mut, u8>"
        });
    }
    let integer = enums[c_type.as_str()].as_str().unwrap_or(c_type.as_str());
    match integer {
        "void" => Some("unit"),
        "int" => Some("i32"),
        "unsigned int" => Some("u32"),
        "long" => Some("i64"),
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

fn check_prototype(
    function: &str,
    declaration: &Declaration,
    pinned: &Value,
) -> Result<String, String> {
    let prototype = pinned["prototypes"][function]["declaration"]
        .as_str()
        .ok_or_else(|| format!("{function}: the pinned headers do not declare it"))?;
    let enums = &pinned["enums"];
    let (c_parameters, c_result) = prototype_types(prototype, function)
        .ok_or_else(|| format!("{function}: unreadable pinned prototype `{prototype}`"))?;
    let expected_result = contract_type(&c_result, enums)
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
        let expected = contract_type(c_type, enums)
            .ok_or_else(|| format!("{function}: unmapped C parameter type `{c_type}`"))?;
        if ty != expected {
            return Err(format!(
                "{function}: parameter `{name}` is `{ty}` but the pinned prototype `{prototype}` \
                 makes it `{expected}`"
            ));
        }
    }
    Ok(prototype.to_owned())
}

fn check_custody(
    function: &str,
    declaration: &Declaration,
    shape: Documented,
) -> Result<(), String> {
    let has = |clause: &str| declaration.clauses.iter().any(|c| c == clause);
    let borrowed = |name: &str| format!("ensures normal resource_borrowed({name})");
    // Every `ensures` clause other than a borrow and `return_none` states
    // custody; the shape fixes exactly which.
    let custody = declaration
        .clauses
        .iter()
        .filter(|clause| clause.starts_with("ensures "))
        .filter(|clause| {
            !clause.starts_with("ensures normal resource_borrowed(")
                && *clause != "ensures normal resource_return_none()"
        })
        .cloned()
        .collect::<Vec<_>>();
    let parameter = |position: usize| -> Result<&str, String> {
        declaration
            .parameters
            .get(position)
            .map(|(name, _)| name.as_str())
            .ok_or_else(|| format!("{function}: no parameter at position {position}"))
    };
    let (expected, held): (Vec<String>, Option<&str>) = match shape {
        Documented::Plain => (vec![], None),
        Documented::OwnedResult => (
            vec!["ensures normal resource_return_owned(nullable)".into()],
            None,
        ),
        Documented::NullableRelease(position) => {
            let name = parameter(position)?;
            (
                vec![format!("ensures normal resource_released({name}, nullable)")],
                Some(name),
            )
        }
        Documented::Reallocating(position) => {
            let name = parameter(position)?;
            (
                vec![
                    format!("ensures normal resource_owned({name}, nullable)"),
                    format!("ensures normal resource_return_reallocated({name}, nullable)"),
                ],
                Some(name),
            )
        }
    };
    let mut sorted = custody.clone();
    sorted.sort();
    let mut wanted = expected.clone();
    wanted.sort();
    if sorted != wanted {
        return Err(format!(
            "{function}: the documented custody is {wanted:?}, but the contract states \
             {sorted:?}"
        ));
    }
    let returns_none = has("ensures normal resource_return_none()");
    let returns_custody = matches!(shape, Documented::OwnedResult | Documented::Reallocating(_));
    if returns_none == returns_custody {
        return Err(format!(
            "{function}: `resource_return_none()` must be stated exactly when the result carries \
             no custody"
        ));
    }
    for (name, _) in &declaration.parameters {
        if Some(name.as_str()) != held && !has(&borrowed(name)) {
            return Err(format!(
                "{function}: parameter `{name}` is documented to be only read, and the contract \
                 does not borrow it"
            ));
        }
        if Some(name.as_str()) == held && has(&borrowed(name)) {
            return Err(format!(
                "{function}: parameter `{name}` carries the documented custody and cannot also \
                 be borrowed"
            ));
        }
    }
    Ok(())
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
    // 2. The prototype.
    let prototype = check_prototype(function, &declaration, &pinned)?;
    let header = pinned["prototypes"][function]["header"]
        .as_str()
        .unwrap_or_default();
    let header_sha256 = pinned["headers"][header].as_str().unwrap_or_default();
    // 3. The version range, and the page it comes from.
    let requires = declaration
        .clauses
        .iter()
        .filter(|clause| clause.starts_with("requires "))
        .collect::<Vec<_>>();
    let ranges = requires
        .iter()
        .filter(|clause| clause.starts_with("requires resource_library_version("))
        .collect::<Vec<_>>();
    let [range] = ranges.as_slice() else {
        return Err(format!(
            "{function}: the declaration must state exactly one library version range"
        ));
    };
    if requires.len() != 3
        || !requires.contains(&&"requires resource_contract_v1".to_owned())
        || !requires.contains(&&"requires resource_revision(flexible)".to_owned())
    {
        return Err(format!(
            "{function}: the only requirements may be `resource_contract_v1`, \
             `resource_revision(flexible)` and the range"
        ));
    }
    let range = range.trim_start_matches("requires ").to_owned();
    let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS)
        .map_err(|error| format!("doc snapshots: {error}"))?;
    let page = snapshots["pages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|page| page["page"] == documented.page && page["fetched"] == true)
        .ok_or_else(|| format!("{function}: no fetched snapshot of {}", documented.page))?;
    let supported = match page["added_in"].as_str() {
        Some(added) => format!("resource_library_version({LIBRARY}, since \"{added}\")"),
        None => {
            return Err(format!(
                "{function}: the fetched page names no version in its AVAILABILITY section"
            ));
        }
    };
    if documented.range != supported || range != supported {
        return Err(format!(
            "{function}: the declaration states `{range}` but the documentation supports \
             `{supported}`"
        ));
    }
    let bound = parse_range(&range).ok_or_else(|| format!("{function}: unreadable `{range}`"))?;
    if !(bound.0 <= pinned_version && bound.1.is_none_or(|removed| pinned_version < removed)) {
        return Err(format!(
            "{function}: `{range}` does not admit the pinned libcurl {pinned_version}"
        ));
    }
    // 4. The documented custody.
    let text = [
        page["availability"].clone(),
        page["return_value"].clone(),
        page["null_sentences"].clone(),
        page["custody_sentences"].clone(),
    ]
    .iter()
    .flat_map(|value| match value {
        Value::String(text) => vec![text.clone()],
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_owned))
            .collect(),
        _ => vec![],
    })
    .collect::<Vec<_>>()
    .join(" ");
    let text = collapse(&text);
    for sentence in documented.evidence {
        if !text.contains(&collapse(sentence)) {
            return Err(format!(
                "{function}: the fetched page does not say \"{sentence}\", which the documented \
                 custody rests on"
            ));
        }
    }
    check_custody(function, &declaration, documented.shape)?;
    Ok(format!(
        "{function}: prototype `{prototype}` ({header}, sha256 {}) matches the contract; `{range}` \
         admits the pinned libcurl {pinned_version}; custody as documented at {} (sha256 {} as \
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
        [LIBRARY, lower_part] => Some((lower(lower_part)?, None)),
        [LIBRARY, lower_part, removed] => Some((
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
        "libcurl contract check (prototype, version range, documented custody); a claim, not a \
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

    fn module(path: &str) -> (&'static str, &'static str) {
        *crate::MODULES.iter().find(|(p, _)| *p == path).unwrap()
    }

    #[test]
    fn every_contract_passes_its_own_checks() {
        for documented in DOCUMENTED {
            let (path, text) = crate::MODULES
                .iter()
                .find(|(_, text)| text.contains(&format!("extern fn {}(", documented.function)))
                .unwrap();
            let note = check_function(path, text, documented.function)
                .unwrap_or_else(|failure| panic!("{failure}"));
            assert!(note.contains("admits the pinned libcurl 8.18.0"), "{note}");
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
        // The W48 plugin's six established contracts, no more.
        assert_eq!(
            declared.into_iter().collect::<Vec<_>>(),
            [
                "curl_easy_escape",
                "curl_global_init",
                "curl_slist_append",
                "curl_slist_free_all",
                "curl_url",
                "curl_url_cleanup"
            ]
        );
    }

    #[test]
    fn a_drifted_contract_is_refused() {
        let (path, text) = module("libcurl/url.qv");
        // A release that does not accept NULL.
        let strict = text.replace(
            "resource_released(handle, nullable)",
            "resource_released(handle)",
        );
        assert!(check_function(path, &strict, "curl_url_cleanup").is_err());
        // A release read as a borrow.
        let borrowed = text.replace(
            "resource_released(handle, nullable)",
            "resource_borrowed(handle)",
        );
        assert!(check_function(path, &borrowed, "curl_url_cleanup").is_err());
        // A non-null owned result.
        let nonnull = text.replace(
            "resource_return_owned(nullable)",
            "resource_return_owned()",
        );
        assert!(check_function(path, &nonnull, "curl_url").is_err());
        // A range the documentation does not support.
        let ranged = text.replace("since \"7.62.0\"", "since \"7.61.0\"");
        assert!(check_function(path, &ranged, "curl_url").is_err());
        let documented = text.replace("since \"7.62.0\"", "documented \"7.62.0\"");
        assert!(check_function(path, &documented, "curl_url").is_err());
        // A range that excludes the pinned version.
        let removed = text.replace("since \"7.62.0\")", "since \"7.62.0\", removed \"8.0.0\")");
        assert!(check_function(path, &removed, "curl_url").is_err());
        // Another library's range.
        let other = text.replace("resource_library_version(curl,", "resource_library_version(openssl,");
        assert!(check_function(path, &other, "curl_url").is_err());
        // No range at all.
        let unranged = text.replace(
            "    requires resource_library_version(curl, since \"7.62.0\");\n",
            "",
        );
        assert!(check_function(path, &unranged, "curl_url").is_err());
        // A signature the pinned prototype does not have.
        let typed = text.replace(
            "curl_url_cleanup(handle: ptr<mut, u8>)",
            "curl_url_cleanup(handle: ptr<const, u8>)",
        );
        assert!(check_function(path, &typed, "curl_url_cleanup").is_err());
        let (path, text) = module("libcurl/global.qv");
        let signed = text.replace("-> u32", "-> i32");
        assert!(check_function(path, &signed, "curl_global_init").is_err());
        let custody = text.replace(
            "ensures normal resource_return_none();",
            "ensures normal resource_return_owned(nullable);",
        );
        assert!(check_function(path, &custody, "curl_global_init").is_err());
        let (path, text) = module("libcurl/slist.qv");
        // The append read as a plain return in place of the reallocation.
        let plain = text.replace(
            "resource_return_reallocated(list, nullable)",
            "resource_return_owned(nullable)",
        );
        assert!(check_function(path, &plain, "curl_slist_append").is_err());
        // The copied string read as retained.
        let unborrowed = text.replace("    ensures normal resource_borrowed(data);\n", "");
        assert!(check_function(path, &unborrowed, "curl_slist_append").is_err());
    }

    #[test]
    fn prototypes_map_to_contract_types() {
        let enums = serde_json::json!({"CURLcode": "unsigned int"});
        assert_eq!(
            prototype_types(
                "char *curl_easy_escape(CURL *handle, const char *string, int length)",
                "curl_easy_escape"
            ),
            Some((
                vec!["CURL *".into(), "const char *".into(), "int".into()],
                "char *".into()
            ))
        );
        assert_eq!(
            prototype_types("CURLU *curl_url(void)", "curl_url"),
            Some((vec![], "CURLU *".into()))
        );
        assert_eq!(contract_type("CURLcode", &enums), Some("u32"));
        assert_eq!(contract_type("long", &enums), Some("i64"));
        assert_eq!(contract_type("struct curl_slist *", &enums), Some("ptr<mut, u8>"));
        assert_eq!(contract_type("const char *", &enums), Some("ptr<const, u8>"));
        assert_eq!(contract_type("CURLUcode", &enums), None);
        assert_eq!(contract_type("size_t", &enums), None);
    }
}
