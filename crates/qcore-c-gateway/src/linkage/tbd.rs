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

//! Apple text-based dylib stubs (`.tbd`), the link-time stub an SDK ships for
//! a system dylib: `libm.tbd` is `libSystem.B.tbd`, whose own document
//! re-exports `/usr/lib/system/libsystem_m.dylib`, whose document lists
//! `_sqrt`.
//!
//! Three versions are read: v3 (`--- !tapi-tbd-v3`, `archs:`), v4
//! (`--- !tapi-tbd`, `tbd-version: 4`, `targets:`) — both YAML, of which the
//! subset those files use is read here: top-level `key: value`, flow lists that
//! may span lines, and block sequences of mappings — and v5 (JSON,
//! `"tapi_tbd_version": 5`). A symbol is exported by the stub when it is listed
//! for the recorded target in the first document, or in a document of the same
//! file that document re-exports (transitively). Any other version, or a target
//! the stub does not list, is refused.
//!
//! One substitution follows Apple `ld` as measured (card `W48-QRLIB-MACOS`):
//! an `arm64-macos` link reads a stub whose first document lists `arm64e-macos`
//! but not `arm64-macos` through its `arm64e-macos` slice — the macOS 27 SDK's
//! `libSystem.tbd` lists no `arm64` slice. When `arm64-macos` is listed, only
//! that slice is read, as `ld` does. The slice is chosen once, from the first
//! document; a re-exported document that does not list it contributes nothing.

use std::collections::{BTreeMap, BTreeSet};

use super::bytes::ReadError;
use super::{Binding, Definition, SymbolRecord, SymbolTable};

/// Whether `data` looks like a text stub.
#[must_use]
pub fn is_tbd(data: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(data) else {
        return false;
    };
    let text = text.trim_start();
    text.starts_with("--- !tapi-tbd")
        || (text.starts_with('{') && text.contains("\"tapi_tbd_version\""))
}

/// One document of a stub, reduced to what linkage reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Document {
    install_name: String,
    current_version: Option<String>,
    /// Re-exported install names for the target.
    reexports: Vec<String>,
    /// `(name, weak)` for the target.
    symbols: Vec<(String, bool)>,
}

/// What a stub says its image exports for one target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextStub {
    /// The first document's install name: the runtime library's identity.
    pub install_name: String,
    pub current_version: Option<String>,
    /// The tbd target whose entries were read: the requested one, or
    /// `arm64e-macos` substituted for an unlisted `arm64-macos`.
    pub slice: String,
    /// Every symbol reachable from the first document, `origin` the install
    /// name of the document that lists it.
    pub table: SymbolTable,
}

/// The tbd target name for a recorded triple (`arm64-apple-darwin24` →
/// `arm64-macos`), or `None` for a triple that is not macOS.
#[must_use]
pub fn tbd_target(triple: &str) -> Option<String> {
    let mut parts = triple.split('-');
    let arch = match parts.next()? {
        "aarch64" | "arm64" => "arm64",
        "arm64e" => "arm64e",
        "x86_64" => "x86_64",
        _ => return None,
    };
    let rest: Vec<&str> = parts.collect();
    if rest.first() != Some(&"apple") {
        return None;
    }
    let os = rest.get(1)?;
    (os.starts_with("darwin") || os.starts_with("macos")).then(|| format!("{arch}-macos"))
}

/// Read a stub for `target` (a tbd target name such as `x86_64-macos`).
///
/// # Errors
///
/// A [`ReadError`] for an unknown version, malformed text, no document, or a
/// first document that lists neither the target nor its substitute.
pub fn read(data: &[u8], target: &str) -> Result<TextStub, ReadError> {
    let text =
        std::str::from_utf8(data).map_err(|_| ReadError::new("tbd", "the stub is not UTF-8"))?;
    let parse = |slice: &str| {
        if text.trim_start().starts_with('{') {
            read_v5(text, slice)
        } else {
            read_yaml(text, slice)
        }
    };
    let (slice, documents) = match parse(target)? {
        Ok(documents) => (target, documents),
        Err(install_name) => {
            let substitute = (target == "arm64-macos").then_some("arm64e-macos");
            match substitute.map(parse).transpose()? {
                Some(Ok(documents)) => ("arm64e-macos", documents),
                _ => {
                    return Err(ReadError::new(
                        "tbd",
                        format!("`{install_name}` does not list the target `{target}`"),
                    ));
                }
            }
        }
    };
    let first = documents
        .first()
        .ok_or_else(|| ReadError::new("tbd", "the stub holds no document"))?;
    let by_name: BTreeMap<&str, &Document> = documents
        .iter()
        .map(|document| (document.install_name.as_str(), document))
        .collect();
    let mut table = SymbolTable::default();
    let mut seen = BTreeSet::new();
    let mut pending = vec![first];
    while let Some(document) = pending.pop() {
        if !seen.insert(document.install_name.clone()) {
            continue;
        }
        for (name, weak) in &document.symbols {
            table.records.push(SymbolRecord {
                name: name.clone(),
                definition: Definition::Defined {
                    binding: if *weak {
                        Binding::Weak
                    } else {
                        Binding::Global
                    },
                    ifunc: false,
                },
                version: None,
                origin: document.install_name.clone(),
                indexed: None,
                import: None,
            });
        }
        // A re-exported library another file supplies is not read: its
        // symbols are simply not in this table, which refuses them as missing.
        for reexport in &document.reexports {
            if let Some(next) = by_name.get(reexport.as_str()) {
                pending.push(next);
            }
        }
    }
    Ok(TextStub {
        install_name: first.install_name.clone(),
        current_version: first.current_version.clone(),
        slice: slice.to_owned(),
        table,
    })
}

// ---------------------------------------------------------------------------
// YAML (v3, v4)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Scalar(String),
    List(Vec<String>),
    Items(Vec<Vec<(String, Value)>>),
}

fn unquote(text: &str) -> String {
    let text = text.trim();
    for quote in ['\'', '"'] {
        if let Some(inner) = text.strip_prefix(quote).and_then(|t| t.strip_suffix(quote)) {
            return inner.to_owned();
        }
    }
    text.to_owned()
}

/// Read a value starting with `first`; a flow list may continue on the
/// following lines until its `]`.
fn value<'a>(
    first: &str,
    lines: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
) -> Result<Value, ReadError> {
    let first = first.trim();
    if !first.starts_with('[') {
        return Ok(Value::Scalar(unquote(first)));
    }
    let mut text = first.to_owned();
    while !text.contains(']') {
        let next = lines
            .next()
            .ok_or_else(|| ReadError::new("tbd", "an unterminated `[` list"))?;
        text.push(' ');
        text.push_str(next.trim());
    }
    let inner = text
        .trim()
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .ok_or_else(|| ReadError::new("tbd", format!("`{text}` is not one flow list")))?;
    Ok(Value::List(
        inner
            .split(',')
            .map(unquote)
            .filter(|item| !item.is_empty())
            .collect(),
    ))
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn split_key(text: &str) -> Option<(&str, &str)> {
    let (key, rest) = text.split_once(':')?;
    let key = key.trim();
    (!key.is_empty() && !key.contains(' ')).then_some((key, rest))
}

fn parse_document(text: &str) -> Result<Vec<(String, Value)>, ReadError> {
    let mut lines = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .peekable();
    let mut out = Vec::new();
    while let Some(line) = lines.next() {
        if indent(line) != 0 {
            return Err(ReadError::new(
                "tbd",
                format!(
                    "`{}` is indented where a top-level key belongs",
                    line.trim()
                ),
            ));
        }
        let (key, rest) = split_key(line)
            .ok_or_else(|| ReadError::new("tbd", format!("`{line}` is not `key: value`")))?;
        if !rest.trim().is_empty() {
            out.push((key.to_owned(), value(rest, &mut lines)?));
            continue;
        }
        // A block sequence of mappings.
        let mut items: Vec<Vec<(String, Value)>> = Vec::new();
        while let Some(next) = lines.peek() {
            if indent(next) == 0 {
                break;
            }
            let next = lines.next().expect("peeked");
            let trimmed = next.trim_start();
            let (entry, fresh) = match trimmed.strip_prefix("- ") {
                Some(entry) => (entry, true),
                None => (trimmed, false),
            };
            if fresh {
                items.push(Vec::new());
            }
            let item = items.last_mut().ok_or_else(|| {
                ReadError::new(
                    "tbd",
                    format!("`{trimmed}` precedes any `- ` item of `{key}`"),
                )
            })?;
            let (sub, rest) = split_key(entry)
                .ok_or_else(|| ReadError::new("tbd", format!("`{entry}` is not `key: value`")))?;
            item.push((sub.to_owned(), value(rest, &mut lines)?));
        }
        out.push((key.to_owned(), Value::Items(items)));
    }
    Ok(out)
}

fn scalar<'a>(document: &'a [(String, Value)], key: &str) -> Option<&'a str> {
    document.iter().find_map(|(k, v)| match v {
        Value::Scalar(text) if k == key => Some(text.as_str()),
        _ => None,
    })
}

fn list<'a>(item: &'a [(String, Value)], key: &str) -> &'a [String] {
    item.iter()
        .find_map(|(k, v)| match v {
            Value::List(items) if k == key => Some(items.as_slice()),
            _ => None,
        })
        .unwrap_or(&[])
}

fn items<'a>(document: &'a [(String, Value)], key: &str) -> &'a [Vec<(String, Value)>] {
    document
        .iter()
        .find_map(|(k, v)| match v {
            Value::Items(items) if k == key => Some(items.as_slice()),
            _ => None,
        })
        .unwrap_or(&[])
}

/// The documents read for `target`, or `Err(install_name)` when the first
/// document does not list it.
type Documents = Result<Vec<Document>, String>;

fn read_yaml(text: &str, target: &str) -> Result<Documents, ReadError> {
    let mut documents = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in text.lines() {
        if let Some(tag) = line.strip_prefix("---") {
            if let Some((tag, body)) = current.take() {
                documents.push((tag, body));
            }
            current = Some((tag.trim().to_owned(), String::new()));
            continue;
        }
        if line.trim() == "..." {
            if let Some((tag, body)) = current.take() {
                documents.push((tag, body));
            }
            continue;
        }
        if let Some((_, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(done) = current.take() {
        documents.push(done);
    }
    let (arch, _) = target
        .split_once('-')
        .ok_or_else(|| ReadError::new("tbd", format!("`{target}` is not ARCH-PLATFORM")))?;
    let mut out = Vec::new();
    for (index, (tag, body)) in documents.iter().enumerate() {
        let parsed = parse_document(body)?;
        let v3 = match tag.as_str() {
            "!tapi-tbd-v3" => true,
            "!tapi-tbd" => {
                if scalar(&parsed, "tbd-version") != Some("4") {
                    return Err(ReadError::new(
                        "tbd",
                        format!(
                            "document {index} is `!tapi-tbd` with `tbd-version: {}`; versions 3, 4 and 5 are read",
                            scalar(&parsed, "tbd-version").unwrap_or("?")
                        ),
                    ));
                }
                false
            }
            other => {
                return Err(ReadError::new(
                    "tbd",
                    format!(
                        "document {index} is tagged `{other}`, not a text stub version read here"
                    ),
                ));
            }
        };
        let install_name = scalar(&parsed, "install-name")
            .ok_or_else(|| {
                ReadError::new("tbd", format!("document {index} names no `install-name`"))
            })?
            .to_owned();
        // v3 scopes an entry by architecture, v4 by `ARCH-PLATFORM` target.
        let scope_key = if v3 { "archs" } else { "targets" };
        let matches = |listed: &[String]| {
            listed
                .iter()
                .any(|entry| if v3 { entry == arch } else { entry == target })
        };
        let own = if v3 {
            list(&parsed, "archs")
        } else {
            list(&parsed, "targets")
        };
        if !matches(own) {
            if index == 0 {
                return Ok(Err(install_name));
            }
            continue;
        }
        let mut document = Document {
            install_name,
            current_version: scalar(&parsed, "current-version").map(ToOwned::to_owned),
            ..Document::default()
        };
        if !v3 {
            for item in items(&parsed, "reexported-libraries") {
                if matches(list(item, scope_key)) {
                    document
                        .reexports
                        .extend(list(item, "libraries").iter().cloned());
                }
            }
        }
        for item in items(&parsed, "exports") {
            if !matches(list(item, scope_key)) {
                continue;
            }
            for name in list(item, "symbols") {
                document.symbols.push((name.clone(), false));
            }
            let weak_key = if v3 {
                "weak-def-symbols"
            } else {
                "weak-symbols"
            };
            for name in list(item, weak_key) {
                document.symbols.push((name.clone(), true));
            }
            if v3 {
                document
                    .reexports
                    .extend(list(item, "re-exports").iter().cloned());
            }
        }
        out.push(document);
    }
    Ok(Ok(out))
}

// ---------------------------------------------------------------------------
// JSON (v5)
// ---------------------------------------------------------------------------

fn read_v5(text: &str, target: &str) -> Result<Documents, ReadError> {
    let root: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| ReadError::new("tbd", format!("the v5 stub is not JSON: {error}")))?;
    if root
        .get("tapi_tbd_version")
        .and_then(serde_json::Value::as_u64)
        != Some(5)
    {
        return Err(ReadError::new(
            "tbd",
            "a JSON stub whose `tapi_tbd_version` is not 5",
        ));
    }
    let mut libraries = Vec::new();
    if let Some(main) = root.get("main_library") {
        libraries.push(main);
    } else {
        return Err(ReadError::new("tbd", "a v5 stub with no `main_library`"));
    }
    if let Some(serde_json::Value::Array(more)) = root.get("libraries") {
        libraries.extend(more.iter());
    }
    let strings = |value: Option<&serde_json::Value>| -> Vec<String> {
        value
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    // An entry applies to the target when it names no `targets` (all of the
    // library's) or names this one.
    let applies = |entry: &serde_json::Value| {
        entry
            .get("targets")
            .and_then(serde_json::Value::as_array)
            .is_none_or(|targets| targets.iter().any(|t| t.as_str() == Some(target)))
    };
    let mut out = Vec::new();
    for (index, library) in libraries.iter().enumerate() {
        let targets: Vec<String> = library
            .get("target_info")
            .and_then(serde_json::Value::as_array)
            .map(|infos| {
                infos
                    .iter()
                    .filter_map(|info| {
                        info.get("target")
                            .and_then(|t| t.as_str())
                            .map(ToOwned::to_owned)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let install_name = library
            .get("install_names")
            .and_then(serde_json::Value::as_array)
            .and_then(|names| names.iter().find(|entry| applies(entry)))
            .and_then(|entry| entry.get("name"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ReadError::new("tbd", format!("library {index} names no install name")))?
            .to_owned();
        if !targets.iter().any(|t| t == target) {
            if index == 0 {
                return Ok(Err(install_name));
            }
            continue;
        }
        let mut document = Document {
            install_name,
            current_version: library
                .get("current_versions")
                .and_then(serde_json::Value::as_array)
                .and_then(|versions| versions.first())
                .and_then(|entry| entry.get("version"))
                .and_then(|v| {
                    v.as_str()
                        .map(ToOwned::to_owned)
                        .or_else(|| Some(v.to_string()))
                }),
            ..Document::default()
        };
        if let Some(serde_json::Value::Array(entries)) = library.get("reexported_libraries") {
            for entry in entries.iter().filter(|entry| applies(entry)) {
                document.reexports.extend(strings(entry.get("names")));
            }
        }
        if let Some(serde_json::Value::Array(entries)) = library.get("exported_symbols") {
            for entry in entries.iter().filter(|entry| applies(entry)) {
                for section in ["text", "data"] {
                    if let Some(section) = entry.get(section) {
                        for name in strings(section.get("global")) {
                            document.symbols.push((name, false));
                        }
                        for name in strings(section.get("weak")) {
                            document.symbols.push((name, true));
                        }
                    }
                }
            }
        }
        out.push(document);
    }
    Ok(Ok(out))
}
