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

//! The Onigmo contract checker (card **ONIGMO-PROVIDER-PLUGIN**).
//!
//! The audit sends one request per binding (`quirin-plugin.json`): one
//! `parameter_mode_respected` obligation per bound function, bound to the
//! module's text digest. For each function the checker decides:
//!
//! 1. **The module** is one this checker was built with ([`crate::MODULES`]):
//!    its SHA-256 is the request's `source_digest`.
//! 2. **The prototype**: the pinned header declares the function
//!    ([`crate::PINNED_PROTOTYPES`], vendored `lib/onigmo/onigmo.h` at Fluent
//!    Bit `046accf6`), and each parameter's and the result's contract type is
//!    the one the header's spelling projects to ([`contract_type`]: the C
//!    gateway's ABI reading, with the typedefs the record verified).
//! 3. **The version range**: the declaration states exactly one
//!    `resource_library_version(onigmo, …)`; it is `documented` the series
//!    of the documentation section that documents the function
//!    ([`crate::DOC_SNAPSHOTS`]: Onigmo's `doc/API` names no version that
//!    added a function, so a `since` is refused); it admits the pinned header's
//!    version (6.2.0).
//! 4. **The revision**: pinned (no `resource_revision` marker), because the
//!    bodies are in the audited tree; the only other `requires` is the
//!    literal-argument premise the table names (`onig_region_free`'s
//!    `free_self == 1`), and no premise is a string.
//! 5. **The custody**: the declaration's `ensures` clauses are exactly the
//!    table's ([`DOCUMENTED`], each read off the vendored body, the module's
//!    comments citing the lines), and the documentation section carries every
//!    sentence the table names.
//!
//! Every function passing all five is answered with a `Safe` **claim** and no
//! certificate, one diagnostic per function; the audit's firewall publishes it
//! `Pending` (`plugin-claim-uncertified`). Any failure answers `Unknown`. Every
//! diagnostic, a note or a refusal, is a formula (user decision 2026-11-25:
//! every premise, condition and refusal reason is a logical-symbol formula):
//! a note is the conjunction of what was checked; a refusal is the
//! conjunction of the facts that refute the check, ending `-> #false`. The
//! checker reads only what is compiled into it; it opens no file and no
//! network connection.

use qcore_schemas::library_version::LibraryVersion;
use qcore_schemas::plugin::{
    PluginDiagnostic, PluginManifest, ProposedVerdict, VerificationRequestV2,
    VerificationResponseV2,
};
use serde_json::Value;

/// The library name the range clauses and `library-versions.json` use.
pub const LIBRARY: &str = "onigmo";

/// The crate name a request's translation unit carries.
pub const CRATE: &str = "quirin-provider-onigmo";

/// What the vendored body and the documentation support for one function.
#[derive(Clone, Copy, Debug)]
pub struct DocumentedFunction {
    pub function: &'static str,
    /// The exact `ensures` clauses, whitespace-collapsed.
    pub ensures: &'static [&'static str],
    /// The `requires` clauses beyond `resource_contract_v1` and the range.
    pub premises: &'static [&'static str],
    /// The result classes the body and the documentation give, as formulas
    /// over `#result` (recorded; no result set is a clause, C09.4).
    pub result_classes: &'static [&'static str],
    /// Sentences of the function's documentation section the reading rests
    /// on, whitespace-collapsed.
    pub evidence: &'static [&'static str],
}

/// Every contracted function (`onig_end` is not: see `contracts/onigmo/init.qv`).
pub const DOCUMENTED: &[DocumentedFunction] = &[
    DocumentedFunction {
        function: "onig_foreach_name",
        ensures: &[
            "ensures normal resource_owned(reg, nonnull)",
            "ensures normal resource_borrowed(func)",
            "ensures normal resource_borrowed(arg)",
            "ensures normal resource_callback_invoked(func, _, _, _, _, reg, arg)",
            "ensures normal resource_return_none()",
        ],
        premises: &[],
        result_classes: &["#result == 0", "#result != 0 -> #result == #ran(func)"],
        evidence: &[
            "Iterate function call for all names.",
            "normal return: 0 error: func's return value.",
            "func(name, name_end, <number of groups>, <group number's list>, reg, arg); if func does \
             not return 0, then iteration is stopped.",
        ],
    },
    DocumentedFunction {
        function: "onig_free",
        ensures: &[
            "ensures normal resource_released(reg, nullable)",
            "ensures normal resource_return_none()",
        ],
        premises: &[],
        result_classes: &[],
        evidence: &["Free memory used by regex object."],
    },
    DocumentedFunction {
        function: "onig_init",
        ensures: &["ensures normal resource_return_none()"],
        premises: &[],
        result_classes: &["#result == 0"],
        evidence: &["Initialize library."],
    },
    DocumentedFunction {
        function: "onig_match",
        ensures: &[
            "ensures normal resource_owned(reg, nonnull)",
            "ensures normal resource_borrowed(string)",
            "ensures normal resource_borrowed(string_end)",
            "ensures normal resource_borrowed(at)",
            "ensures normal resource_owned(region, nullable)",
            "ensures normal resource_borrowed(option)",
            "ensures normal resource_return_none()",
        ],
        premises: &[],
        result_classes: &["#result >= 0", "#result == -1", "#result <= -2"],
        evidence: &[
            "normal return: match length (>= 0) not match: ONIG_MISMATCH ( < 0)",
            "5 region: address for return group match range info (NULL is allowed)",
        ],
    },
    DocumentedFunction {
        function: "onig_name_to_backref_number",
        ensures: &[
            "ensures normal resource_owned(reg, nonnull)",
            "ensures normal resource_borrowed(name)",
            "ensures normal resource_borrowed(name_end)",
            "ensures normal resource_owned(region, nullable)",
            "ensures normal resource_return_none()",
        ],
        premises: &[],
        result_classes: &["#result >= 1", "#result <= -1"],
        evidence: &[
            "Return the group number corresponding to the named backref (\\k<name>).",
            "normal return: group number.",
            "4 region: search/match result region.",
        ],
    },
    DocumentedFunction {
        function: "onig_new",
        ensures: &[
            "ensures normal resource_out(reg, nullable)",
            "ensures normal resource_borrowed(pattern)",
            "ensures normal resource_borrowed(pattern_end)",
            "ensures normal resource_borrowed(option)",
            "ensures normal resource_borrowed(enc)",
            "ensures normal resource_borrowed(syntax)",
            "ensures normal resource_borrowed(einfo)",
            "ensures normal resource_retained_by(enc, global)",
            "ensures normal resource_retained_by(syntax, global)",
            "ensures normal resource_retained_by(pattern, einfo)",
            "ensures normal resource_retained_by(pattern_end, einfo)",
            "ensures normal resource_return_none()",
        ],
        premises: &[],
        result_classes: &["#result == 0", "#result != 0"],
        evidence: &[
            "Create a regex object.",
            "normal return: ONIG_NORMAL",
            "1 reg: return regex object's address.",
            "7 err_info: address for return optional error info.",
        ],
    },
    DocumentedFunction {
        function: "onig_region_free",
        ensures: &[
            "ensures normal resource_released(region, nullable)",
            "ensures normal resource_borrowed(free_self)",
            "ensures normal resource_return_none()",
        ],
        premises: &["requires resource_literal_argument(free_self, 1)"],
        result_classes: &[],
        evidence: &[
            "Free memory used by region.",
            "2 free_self: [1: free all, 0: free memory used in region but not self]",
        ],
    },
    DocumentedFunction {
        function: "onig_region_new",
        ensures: &["ensures normal resource_return_owned(nullable)"],
        premises: &[],
        result_classes: &[],
        evidence: &["Create a region."],
    },
    DocumentedFunction {
        function: "onig_search",
        ensures: &[
            "ensures normal resource_owned(reg, nonnull)",
            "ensures normal resource_borrowed(string)",
            "ensures normal resource_borrowed(string_end)",
            "ensures normal resource_borrowed(search_start)",
            "ensures normal resource_borrowed(search_range)",
            "ensures normal resource_owned(region, nullable)",
            "ensures normal resource_borrowed(option)",
            "ensures normal resource_return_none()",
        ],
        premises: &[],
        result_classes: &["#result >= 0", "#result == -1", "#result <= -2"],
        evidence: &[
            "Search string and return search result and matching region.",
            "normal return: match position offset (i.e. p - str >= 0) not found: ONIG_MISMATCH (< 0)",
            "7 region: address for return group match range info (NULL is allowed)",
        ],
    },
];

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

/// Split at the commas outside `<…>` and `(…)`.
fn split_top(list: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let (mut depth, mut start) = (0usize, 0usize);
    for (index, character) in list.char_indices() {
        match character {
            '<' | '(' => depth += 1,
            '>' | ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                pieces.push(&list[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    pieces.push(&list[start..]);
    pieces
}

/// The declaration of `function` in a module's text.
#[must_use]
pub fn declaration(module: &str, function: &str) -> Option<Declaration> {
    let text = strip_comments(module);
    let opening = format!("extern fn {function}(");
    let start = text.find(&opening)?;
    let rest = &text[start + opening.len()..];
    let close = rest.find(')')?;
    let parameters = split_top(&rest[..close])
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

/// The contract type a header spelling projects to, as the C gateway's ABI
/// reader projects it (`crates/qcore-c-gateway/src/legacy_declared_abi.rs`):
/// a pointer to a `const` pointee is `ptr<const, u8>`, any other pointer
/// `ptr<mut, u8>`, a pointer to a pointer `ptr<mut, ptr<mut, u8>>`; a typedef
/// name that stands for a pointer is a pointer whose pointee qualifier the
/// reader does not carry through the chain, `ptr<mut, u8>`; `int` is `i32`,
/// `unsigned int` `u32`, `long` (a `ptrdiff_t`, LP64) `isize` when the typedef
/// is `OnigPosition`, `void` `unit`, a function pointer `fnptr`. `typedefs`
/// is the pinned record's verified map.
#[must_use]
pub fn contract_type(spelled: &str, typedefs: &Value) -> Option<&'static str> {
    let spelled = collapse(spelled);
    if spelled == "fnptr" {
        return Some("fnptr");
    }
    let stars = spelled.chars().filter(|c| *c == '*').count();
    let base = collapse(&spelled.replace('*', " "));
    let (is_const, core) = match base.strip_prefix("const ") {
        Some(core) => (true, core.to_owned()),
        None => (false, base.clone()),
    };
    let denoted = typedefs[core.as_str()].as_str();
    let typedef_is_pointer = denoted.is_some_and(|d| d.contains('*'));
    let depth = stars + usize::from(typedef_is_pointer);
    match depth {
        0 => {
            let scalar = match (core.as_str(), denoted) {
                ("OnigPosition", Some("long")) => return Some("isize"),
                (_, Some(d)) => d.to_owned(),
                (c, None) => c.to_owned(),
            };
            match scalar.as_str() {
                "void" => Some("unit"),
                "int" => Some("i32"),
                "unsigned int" => Some("u32"),
                _ => None,
            }
        }
        1 if typedef_is_pointer => (!is_const).then_some("ptr<mut, u8>"),
        1 => Some(if is_const {
            "ptr<const, u8>"
        } else {
            "ptr<mut, u8>"
        }),
        2 if !is_const => Some("ptr<mut, ptr<mut, u8>>"),
        _ => None,
    }
}

fn refuse(function: &str, facts: &str) -> String {
    format!("{function}: {facts} -> #false")
}

fn check_prototype(
    function: &str,
    declaration: &Declaration,
    pinned: &Value,
) -> Result<String, String> {
    let record = &pinned["prototypes"][function];
    let prototype = record["declaration"]
        .as_str()
        .ok_or_else(|| refuse(function, &format!("{function} in #pinned_prototypes")))?;
    let typedefs = &pinned["typedefs"];
    let c_result = record["result"].as_str().unwrap_or_default();
    let expected_result = contract_type(c_result, typedefs).ok_or_else(|| {
        refuse(
            function,
            &format!("#projects(\"{c_result}\") in #contract_types"),
        )
    })?;
    if declaration.result != expected_result {
        return Err(refuse(
            function,
            &format!(
                "#result_type(#contract) == {} && #result_type(#prototype) == {expected_result} && {} == {expected_result}",
                declaration.result, declaration.result
            ),
        ));
    }
    let c_parameters = record["parameters"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if c_parameters.len() != declaration.parameters.len() {
        return Err(refuse(
            function,
            &format!(
                "#arity(#contract) == {} && #arity(#prototype) == {} && {} == {}",
                declaration.parameters.len(),
                c_parameters.len(),
                declaration.parameters.len(),
                c_parameters.len()
            ),
        ));
    }
    for ((name, ty), c_type) in declaration.parameters.iter().zip(&c_parameters) {
        let expected = contract_type(c_type, typedefs).ok_or_else(|| {
            refuse(
                function,
                &format!("#projects(\"{c_type}\") in #contract_types"),
            )
        })?;
        if ty != expected {
            return Err(refuse(
                function,
                &format!(
                    "#param_type(#contract, {name}) == {ty} && #param_type(#prototype, {name}) == {expected} && {ty} == {expected}"
                ),
            ));
        }
    }
    Ok(prototype.to_owned())
}

/// `(lower, removed)` of a range clause, read with the schema's version
/// parser, and whether the lower bound is `documented` (else `since`).
#[must_use]
pub fn parse_range(range: &str) -> Option<(LibraryVersion, Option<LibraryVersion>, bool)> {
    let inner = range
        .strip_prefix("resource_library_version(")?
        .strip_suffix(')')?;
    let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();
    let quoted = |text: &str| -> Option<LibraryVersion> {
        LibraryVersion::parse(text.strip_prefix('"')?.strip_suffix('"')?).ok()
    };
    let lower = |part: &str| -> Option<(LibraryVersion, bool)> {
        if let Some(rest) = part.strip_prefix("documented ") {
            Some((quoted(rest)?, true))
        } else {
            Some((quoted(part.strip_prefix("since ")?)?, false))
        }
    };
    match parts.as_slice() {
        [LIBRARY, lower_part] => {
            let (bound, documented) = lower(lower_part)?;
            Some((bound, None, documented))
        }
        [LIBRARY, lower_part, removed] => {
            let (bound, documented) = lower(lower_part)?;
            Some((
                bound,
                Some(quoted(removed.strip_prefix("removed ")?)?),
                documented,
            ))
        }
        _ => None,
    }
}

/// Decide one function; `Ok(note)` when every check passes.
///
/// # Errors
///
/// The first check that fails, as a formula ending `-> #false`.
pub fn check_function(module_path: &str, module: &str, function: &str) -> Result<String, String> {
    let documented = DOCUMENTED
        .iter()
        .find(|entry| entry.function == function)
        .ok_or_else(|| refuse(function, &format!("{function} in #documented")))?;
    let declaration = declaration(module, function).ok_or_else(|| {
        refuse(
            function,
            &format!("{function} in #declared(\"{module_path}\")"),
        )
    })?;
    let pinned: Value = serde_json::from_str(crate::PINNED_PROTOTYPES)
        .map_err(|_| refuse(function, "#parses(#pinned_prototypes)"))?;
    let pinned_version = pinned["version"]
        .as_str()
        .and_then(|text| LibraryVersion::parse(text).ok())
        .ok_or_else(|| refuse(function, "#version(#pinned_prototypes) in #versions"))?;
    // 2. The prototype.
    let prototype = check_prototype(function, &declaration, &pinned)?;
    let header_sha256 = pinned["headers"]["onigmo.h"].as_str().unwrap_or_default();
    // 3. The range, and 4. the revision and premises.
    let requires = declaration
        .clauses
        .iter()
        .filter(|clause| clause.starts_with("requires "))
        .cloned()
        .collect::<Vec<_>>();
    let ranges = requires
        .iter()
        .filter(|clause| clause.starts_with("requires resource_library_version("))
        .collect::<Vec<_>>();
    let [range] = ranges.as_slice() else {
        return Err(refuse(
            function,
            &format!(
                "#count(#ranges(#contract)) == {} && {} == 1",
                ranges.len(),
                ranges.len()
            ),
        ));
    };
    let mut expected_requires = vec!["requires resource_contract_v1".to_owned(), (*range).clone()];
    expected_requires.extend(documented.premises.iter().map(|p| collapse(p)));
    let mut sorted_requires = requires.clone();
    sorted_requires.sort();
    expected_requires.sort();
    if sorted_requires != expected_requires {
        return Err(refuse(
            function,
            &format!(
                "#requires(#contract) == {{{}}} && #requires(#documented) == {{{}}} && #requires(#contract) == #requires(#documented)",
                sorted_requires.join("; "),
                expected_requires.join("; ")
            ),
        ));
    }
    if declaration.clauses.iter().any(|clause| {
        clause.contains('"') && !clause.starts_with("requires resource_library_version(")
    }) {
        return Err(refuse(
            function,
            "quant all #c in #clauses(#contract): #c in #formulas",
        ));
    }
    let range = range.trim_start_matches("requires ").to_owned();
    let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS)
        .map_err(|_| refuse(function, "#parses(#doc_snapshots)"))?;
    let section = snapshots["functions"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|entry| entry["function"] == function && entry["documented"] == true)
        .ok_or_else(|| refuse(function, &format!("{function} in #documented_sections")))?;
    let series = section["series"]
        .as_str()
        .ok_or_else(|| refuse(function, "#series(#section) in #versions"))?;
    let supported = format!("resource_library_version({LIBRARY}, documented \"{series}\")");
    if range != supported {
        return Err(refuse(
            function,
            &format!(
                "#range(#contract) == {range} && #range(#documentation) == {supported} && #range(#contract) == #range(#documentation)"
            ),
        ));
    }
    let (lower, removed, _) =
        parse_range(&range).ok_or_else(|| refuse(function, &format!("#parses({range})")))?;
    if !(lower <= pinned_version && removed.is_none_or(|removed| pinned_version < removed)) {
        return Err(refuse(
            function,
            &format!("{pinned_version} in #admitted({range})"),
        ));
    }
    // 5. The custody, and the documentation it rests on.
    let mut ensures = declaration
        .clauses
        .iter()
        .filter(|clause| clause.starts_with("ensures "))
        .cloned()
        .collect::<Vec<_>>();
    ensures.sort();
    let mut wanted = documented
        .ensures
        .iter()
        .map(|clause| collapse(clause))
        .collect::<Vec<_>>();
    wanted.sort();
    if ensures != wanted {
        return Err(refuse(
            function,
            &format!(
                "#ensures(#contract) == {{{}}} && #ensures(#body) == {{{}}} && #ensures(#contract) == #ensures(#body)",
                ensures.join("; "),
                wanted.join("; ")
            ),
        ));
    }
    if declaration.clauses.len() != ensures.len() + requires.len() {
        return Err(refuse(
            function,
            "quant all #c in #clauses(#contract): #c in #requires(#contract) | #ensures(#contract)",
        ));
    }
    let text = collapse(section["section"].as_str().unwrap_or_default());
    for sentence in documented.evidence {
        if !text.contains(&collapse(sentence)) {
            return Err(refuse(
                function,
                &format!("\"{}\" in #section({function})", collapse(sentence)),
            ));
        }
    }
    let classes = if documented.result_classes.is_empty() {
        "{}".to_owned()
    } else {
        documented.result_classes.join(" | ")
    };
    Ok(format!(
        "{function}: #prototype(\"{prototype}\") == #signature(#contract) && #sha256(onigmo.h) == {} && {pinned_version} in #admitted({range}) && #ensures(#contract) == #ensures(#body) && #documented(#ensures(#contract), lib/onigmo/doc/API, {series}, {}) && #result_classes == {classes}",
        &header_sha256[..header_sha256.len().min(12)],
        &snapshots["files"]["API"]["sha256"]
            .as_str()
            .unwrap_or_default()[..12.min(
            snapshots["files"]["API"]["sha256"]
                .as_str()
                .unwrap_or_default()
                .len()
        )],
    ))
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
                "{}: #sha256(#unit) == {} && #sha256(#unit) in #built_modules -> #false",
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
        "#checked(#prototype, #range, #revision, #ensures, #documentation) && #claim && (#certificate -> #false)",
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

    fn module_of(function: &str) -> (&'static str, &'static str) {
        *crate::MODULES
            .iter()
            .find(|(_, text)| text.contains(&format!("extern fn {function}(")))
            .unwrap()
    }

    #[test]
    fn every_contract_passes_its_own_checks() {
        for documented in DOCUMENTED {
            let (path, text) = module_of(documented.function);
            let note = check_function(path, text, documented.function)
                .unwrap_or_else(|failure| panic!("{failure}"));
            assert!(
                note.contains(
                    "6.2.0 in #admitted(resource_library_version(onigmo, documented \"6.1.2\"))"
                ),
                "{note}"
            );
            assert!(!note.contains("-> #false"), "{note}");
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
        assert_eq!(
            declared.into_iter().collect::<Vec<_>>(),
            [
                "onig_foreach_name",
                "onig_free",
                "onig_init",
                "onig_match",
                "onig_name_to_backref_number",
                "onig_new",
                "onig_region_free",
                "onig_region_new",
                "onig_search",
            ]
        );
        // onig_end is deliberately not declared.
        assert!(
            crate::MODULES
                .iter()
                .all(|(_, text)| !strip_comments(text).contains("extern fn onig_end("))
        );
        let names = DOCUMENTED.iter().map(|d| d.function).collect::<Vec<_>>();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "the table is sorted");
    }

    #[test]
    fn every_refusal_and_note_is_a_formula() {
        let (path, text) = module("onigmo/region.qv");
        let drifted = text.replace(
            "resource_released(region, nullable)",
            "resource_borrowed(region)",
        );
        let failure = check_function(path, &drifted, "onig_region_free").unwrap_err();
        assert!(failure.ends_with("-> #false"), "{failure}");
        assert!(failure.contains("#ensures(#contract) =="), "{failure}");
        for (path, text) in crate::MODULES {
            for documented in DOCUMENTED {
                if let Ok(note) = check_function(path, text, documented.function) {
                    assert!(note.contains(" && "), "{note}");
                }
            }
        }
    }

    #[test]
    fn a_drifted_contract_is_refused() {
        let (path, text) = module("onigmo/regex.qv");
        let refused = |text: &str, function: &str| {
            let failure = check_function(path, text, function).unwrap_err();
            assert!(failure.ends_with("-> #false"), "{failure}");
        };
        // A non-nullable release.
        refused(
            &text.replace(
                "resource_released(reg, nullable)",
                "resource_released(reg, nonnull)",
            ),
            "onig_free",
        );
        // The out slot read as non-null on every return, or as a fresh result.
        refused(
            &text.replace("resource_out(reg, nullable)", "resource_out(reg, nonnull)"),
            "onig_new",
        );
        // A retention dropped: enc borrowed and forgotten.
        refused(
            &text.replace(
                "    ensures normal resource_retained_by(enc, global);\n",
                "",
            ),
            "onig_new",
        );
        // A `since` the documentation does not give, another series, a range
        // excluding the pinned version, another library's range, no range.
        refused(
            &text.replace("documented \"6.1.2\"", "since \"6.1.2\""),
            "onig_free",
        );
        refused(
            &text.replace("documented \"6.1.2\"", "documented \"6.0.0\""),
            "onig_free",
        );
        refused(
            &text.replace(
                "documented \"6.1.2\")",
                "documented \"6.1.2\", removed \"6.2.0\")",
            ),
            "onig_free",
        );
        refused(
            &text.replace(
                "resource_library_version(onigmo,",
                "resource_library_version(curl,",
            ),
            "onig_free",
        );
        refused(
            &text.replace(
                "    requires resource_library_version(onigmo, documented \"6.1.2\");\n",
                "",
            ),
            "onig_free",
        );
        // A flexible revision: the bodies are in the audited tree.
        refused(
            &text.replace(
                "    requires resource_contract_v1;\n",
                "    requires resource_contract_v1;\n    requires resource_revision(flexible);\n",
            ),
            "onig_free",
        );
        // A signature the header does not have.
        refused(
            &text.replace("option: u32, enc", "option: i32, enc"),
            "onig_new",
        );
        refused(
            &text.replace("enc: ptr<mut, u8>", "enc: ptr<const, u8>"),
            "onig_new",
        );
        let (path, text) = module("onigmo/region.qv");
        // The free_self premise dropped (both arms read as releases), or
        // bound to the arrays-only arm.
        for drift in [
            text.replace(
                "    requires resource_literal_argument(free_self, 1);\n",
                "",
            ),
            text.replace(
                "resource_literal_argument(free_self, 1)",
                "resource_literal_argument(free_self, 0)",
            ),
        ] {
            assert!(check_function(path, &drift, "onig_region_free").is_err());
        }
        // A non-null region.
        assert!(
            check_function(
                path,
                &text.replace(
                    "resource_return_owned(nullable)",
                    "resource_return_owned(nonnull)"
                ),
                "onig_region_new"
            )
            .is_err()
        );
        let (path, text) = module("onigmo/search.qv");
        // The regex read as borrowed (no liveness requirement).
        assert!(
            check_function(
                path,
                &text.replacen(
                    "ensures normal resource_owned(reg, nonnull)",
                    "ensures normal resource_borrowed(reg)",
                    1
                ),
                "onig_search"
            )
            .is_err()
        );
        // A premise stated as a string.
        assert!(
            check_function(
                path,
                &text.replacen(
                    "    requires resource_contract_v1;\n",
                    "    requires resource_contract_v1;\n    requires resource_premise(p, \"the regex is live\");\n",
                    1
                ),
                "onig_search"
            )
            .is_err()
        );
        let (path, text) = module("onigmo/names.qv");
        // The callback's positions moved.
        assert!(
            check_function(
                path,
                &text.replace("(func, _, _, _, _, reg, arg)", "(func, _, _, _, _, _, arg)"),
                "onig_foreach_name"
            )
            .is_err()
        );
    }

    #[test]
    fn header_spellings_project_as_the_gateway_reads_them() {
        let typedefs =
            serde_json::from_str::<Value>(crate::PINNED_PROTOTYPES).unwrap()["typedefs"].clone();
        for (spelled, expected) in [
            ("OnigRegex", Some("ptr<mut, u8>")),
            ("OnigRegex *", Some("ptr<mut, ptr<mut, u8>>")),
            ("OnigEncoding", Some("ptr<mut, u8>")),
            ("const OnigUChar *", Some("ptr<const, u8>")),
            ("const OnigSyntaxType *", Some("ptr<const, u8>")),
            ("const OnigRegion *", Some("ptr<const, u8>")),
            ("OnigRegion *", Some("ptr<mut, u8>")),
            ("OnigErrorInfo *", Some("ptr<mut, u8>")),
            ("void *", Some("ptr<mut, u8>")),
            ("fnptr", Some("fnptr")),
            ("int", Some("i32")),
            ("OnigOptionType", Some("u32")),
            ("OnigPosition", Some("isize")),
            ("void", Some("unit")),
            ("OnigUChar", None),
            ("long", None),
            ("const OnigRegex", None),
        ] {
            assert_eq!(contract_type(spelled, &typedefs), expected, "{spelled}");
        }
    }
}
