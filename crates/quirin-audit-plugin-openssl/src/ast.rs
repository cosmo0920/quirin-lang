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

//! Clang's JSON AST: the one front end the checker reads C through.
//!
//! Two dumps are taken. The **header probe** is one translation unit that
//! includes every header the contract set names, preprocessed as a consumer
//! sees it (`-I<release>/include` only); its top-level function declarations
//! give each binding's public prototype. A **definition dump** preprocesses
//! the defining file exactly as the pinned build compiles it (the contract
//! set's flag set, from the release root) and keeps the declarations of one
//! name (`-ast-dump-filter`); each dumped declaration starts with its file.
//! Anything Clang refuses, or any output that is not a stream of JSON
//! objects, is an error the caller turns into `Unknown`.

use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

/// A node's `kind`.
#[must_use]
pub fn kind(node: &Value) -> &str {
    node["kind"].as_str().unwrap_or("")
}

/// A node's children (`inner`), empty when it has none.
#[must_use]
pub fn children(node: &Value) -> &[Value] {
    node["inner"].as_array().map_or(&[], Vec::as_slice)
}

/// A node's spelled type.
#[must_use]
pub fn qual_type(node: &Value) -> &str {
    node["type"]["qualType"].as_str().unwrap_or("")
}

/// Whether a node's type is, or desugars to, a pointer, array or function
/// type (a type through which a caller's object can be reached).
#[must_use]
pub fn reaches_memory(node: &Value) -> bool {
    [
        node["type"]["qualType"].as_str(),
        node["type"]["desugaredQualType"].as_str(),
    ]
    .into_iter()
    .flatten()
    .any(|spelled| spelled.contains(['*', '[', '(']))
}

/// The file a declaration's location names: its expansion's for a
/// declaration a macro wrote, else its own.
#[must_use]
pub fn declaration_file(node: &Value) -> Option<&str> {
    let location = &node["loc"];
    location["expansionLoc"]["file"]
        .as_str()
        .or_else(|| location["file"].as_str())
}

/// The line a declaration's location names (expansion first).
#[must_use]
pub fn declaration_line(node: &Value) -> Option<u64> {
    let location = &node["loc"];
    location["expansionLoc"]["line"]
        .as_u64()
        .or_else(|| location["line"].as_u64())
}

/// The function's body, if this declaration is a definition.
#[must_use]
pub fn body(function: &Value) -> Option<&Value> {
    children(function)
        .iter()
        .find(|child| kind(child) == "CompoundStmt")
}

/// The function's parameters, in order.
#[must_use]
pub fn parameters(function: &Value) -> Vec<&Value> {
    children(function)
        .iter()
        .filter(|child| kind(child) == "ParmVarDecl")
        .collect()
}

/// Parse a stream of JSON objects (one per dumped declaration).
///
/// # Errors
///
/// Anything that is not a whitespace-separated stream of JSON objects.
pub fn parse_stream(bytes: &[u8]) -> Result<Vec<Value>, String> {
    let mut values = Vec::new();
    for value in serde_json::Deserializer::from_slice(bytes).into_iter::<Value>() {
        let value = value.map_err(|error| format!("Clang's JSON AST does not parse: {error}"))?;
        if !value.is_object() {
            return Err("Clang's JSON AST holds a value that is not an object".into());
        }
        values.push(value);
    }
    Ok(values)
}

fn run(
    clang: &str,
    directory: &Path,
    arguments: &[String],
    input: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let mut command = Command::new(clang);
    command
        .args(arguments)
        .current_dir(directory)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("{clang}: {error}"))?;
    let writer = input.map(|bytes| {
        let mut stdin = child.stdin.take().expect("piped stdin");
        let bytes = bytes.to_vec();
        std::thread::spawn(move || stdin.write_all(&bytes))
    });
    let output = child
        .wait_with_output()
        .map_err(|error| format!("{clang}: {error}"))?;
    if let Some(writer) = writer {
        let _ = writer.join();
    }
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{clang} failed ({}): {}",
            output.status,
            stderr.lines().take(6).collect::<Vec<_>>().join(" | ")
        ));
    }
    Ok(output.stdout)
}

/// `clang --version`'s first line.
///
/// # Errors
///
/// Clang does not run.
pub fn clang_version(clang: &str) -> Result<String, String> {
    let output = Command::new(clang)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("{clang}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{clang} --version failed ({})", output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_owned())
}

/// The header probe: the top-level function declarations of a translation
/// unit including `headers`, preprocessed with `-I<tree>/include` only.
///
/// # Errors
///
/// Clang fails or its output does not parse.
pub fn header_declarations(
    clang: &str,
    tree: &Path,
    headers: &[&str],
) -> Result<Vec<Value>, String> {
    let source: String = headers
        .iter()
        .map(|header| format!("#include <{header}>\n"))
        .collect();
    let arguments = vec![
        "-x".to_owned(),
        "c".to_owned(),
        "-fsyntax-only".to_owned(),
        "-Xclang".to_owned(),
        "-ast-dump=json".to_owned(),
        "-Iinclude".to_owned(),
        "-".to_owned(),
    ];
    let stdout = run(clang, tree, &arguments, Some(source.as_bytes()))?;
    let mut units = parse_stream(&stdout)?;
    if units.len() != 1 || kind(&units[0]) != "TranslationUnitDecl" {
        return Err("the header probe did not dump one translation unit".into());
    }
    let unit = units.remove(0);
    Ok(match unit {
        Value::Object(mut object) => match object.remove("inner") {
            Some(Value::Array(inner)) => inner
                .into_iter()
                .filter(|node| kind(node) == "FunctionDecl")
                .collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    })
}

/// A definition dump: the declarations named `name` exactly (Clang's filter
/// is a substring match; the rest are dropped) of `file`, compiled from the
/// release root with `flags`.
///
/// # Errors
///
/// Clang fails or its output does not parse.
pub fn named_declarations(
    clang: &str,
    tree: &Path,
    flags: &[String],
    file: &str,
    name: &str,
) -> Result<Vec<Value>, String> {
    let mut arguments = flags.to_vec();
    arguments.extend(
        [
            "-fsyntax-only",
            "-Xclang",
            "-ast-dump=json",
            "-Xclang",
            &format!("-ast-dump-filter={name}"),
            file,
        ]
        .map(str::to_owned),
    );
    let stdout = run(clang, tree, &arguments, None)?;
    Ok(parse_stream(&stdout)?
        .into_iter()
        .filter(|node| node["name"].as_str() == Some(name))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stream_of_objects_parses_and_anything_else_refuses() {
        let values = parse_stream(b"{\"kind\": \"A\"}\n{\"kind\": \"B\"}\n").unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(kind(&values[1]), "B");
        assert!(parse_stream(b"Dumping x:\n{}").is_err());
        assert!(parse_stream(b"[1]").is_err());
        assert!(parse_stream(b"").unwrap().is_empty());
    }

    #[test]
    fn memory_reaching_types_are_seen_through_sugar() {
        let plain: Value = serde_json::json!({"type": {"qualType": "unsigned long"}});
        let pointer: Value = serde_json::json!({"type": {"qualType": "void *"}});
        let sugar: Value = serde_json::json!({
            "type": {"qualType": "handle_t", "desugaredQualType": "struct h *"}
        });
        let callback: Value = serde_json::json!({"type": {"qualType": "int (*)(void)"}});
        assert!(!reaches_memory(&plain));
        assert!(reaches_memory(&pointer));
        assert!(reaches_memory(&sugar));
        assert!(reaches_memory(&callback));
    }

    #[test]
    fn a_macro_written_declaration_names_its_expansion_file() {
        let node: Value = serde_json::json!({"loc": {
            "spellingLoc": {"file": "include/x.h", "line": 3},
            "expansionLoc": {"file": "crypto/a.c", "line": 40}
        }});
        assert_eq!(declaration_file(&node), Some("crypto/a.c"));
        assert_eq!(declaration_line(&node), Some(40));
        let node: Value = serde_json::json!({"loc": {"file": "crypto/b.c", "line": 7}});
        assert_eq!(declaration_file(&node), Some("crypto/b.c"));
    }
}
