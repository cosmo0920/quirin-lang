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

//! The layout of an imported C struct, read by the recorded compiler (card
//! `W48-QR-C-STRUCT-IMPORT`, `source.ffi.v1`).
//!
//! A `.qr` file's `struct! { type T = c"struct TAG" from "H"; fields { … } }`
//! never states a layout. This module asks the compiler for one:
//!
//! 1. The header reference resolves against the declaring source file's own
//!    directory — one candidate path, no search, no fallback (the
//!    `HEADER-CLI-RELATIVE` precedent) — and its bytes are digested.
//! 2. The running Clang must be the **recorded** compiler: the first line of
//!    its `--version` equal to the recorded target's `compiler`, and its
//!    `-dumpmachine` equal to the recorded `triple` (the P-RECORDED-COMPILER
//!    discipline). Otherwise nothing is read.
//! 3. A probe translation unit includes the header by its resolved path and
//!    declares one probe object of each §G02t C11 spelling. The typed AST
//!    (`-ast-dump=json`, filtered to names containing the tag) gives the
//!    record's members, their written and canonical types, and every
//!    attribute; `-fdump-record-layouts-complete` gives the offsets, the size
//!    and the alignment. The two readings must name the same members in the
//!    same order.
//!
//! Every shape this reader does not state is refused rather than
//! approximated: no complete definition of `struct TAG` (an unknown tag), a
//! union, a bit-field, a flexible array member, an anonymous member, and any
//! attribute on the record or a member (`packed`, `aligned`, `#pragma pack`'s
//! `MaxFieldAlignmentAttr`, …). A member of a type with no §G02t equivalent
//! is **read**, with an empty equivalence list: the `.qr` side may leave it
//! unlisted, as opaque bytes, and the core refuses it only if listed.
//!
//! Premises stated, not proved (trust boundary 28): P-RECORDED-COMPILER (the
//! compiler at the discovered path with the recorded version reports the
//! layout it uses when it compiles the emitted C) and that the header on disk
//! when the C is built is the one read here — the second is discharged at
//! build time by the emitted `_Static_assert`s, not here.

use std::path::Path;
use std::process::Stdio;

use qcore_core::ffi::{CFieldFact, CStructFacts};
use qcore_core::scalar_types::{self as st, ScalarType};
use serde_json::Value;

/// One struct to read.
#[derive(Clone, Debug)]
pub struct StructLayoutRequest<'a> {
    pub tag: &'a str,
    /// The header reference exactly as the source wrote it.
    pub header: &'a str,
    /// The directory of the source file that declared the import.
    pub source_directory: &'a Path,
    /// The recorded target's `compiler` and `triple`.
    pub recorded_compiler: &'a str,
    pub recorded_triple: &'a str,
}

/// Why no layout was read. Every variant is a `Q063` refusal: the header or
/// its compile context does not resolve, or the struct is outside what this
/// reader states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutReadRefusal(pub String);

impl std::fmt::Display for LayoutReadRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn refuse(message: impl Into<String>) -> LayoutReadRefusal {
    LayoutReadRefusal(message.into())
}

fn probe_name(tag: &str, ty: ScalarType) -> String {
    format!("{tag}__qcore_ffi_probe_{}", ty.name())
}

/// Run the discovered Clang over `source` on standard input with `flags`,
/// from `directory`.
fn clang(directory: &Path, flags: &[&str], source: &str) -> Result<String, LayoutReadRefusal> {
    use std::io::Write;
    let mut command = crate::clang::command().map_err(refuse)?;
    command
        .current_dir(directory)
        .args(["-x", "c", "-std=c11", "-fsyntax-only"])
        .args(flags)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| refuse(format!("the recorded compiler could not be run: {error}")))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| refuse("Clang stdin unavailable"))?;
    let output = std::thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(source.as_bytes()));
        let output = child.wait_with_output();
        let written = writer
            .join()
            .map_err(|_| refuse("Clang input writer panicked"))?;
        written.map_err(|error| refuse(format!("Clang input failed: {error}")))?;
        output.map_err(|error| refuse(format!("Clang failed: {error}")))
    })?;
    if !output.status.success() {
        return Err(refuse(format!(
            "the recorded compiler rejected the header's compile context: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| refuse(format!("Clang output is not text: {error}")))
}

/// The canonical spelling of a typed-AST node's type.
fn canonical(node: &Value) -> Option<(String, String)> {
    let ty = node.get("type")?;
    let written = ty.get("qualType")?.as_str()?.to_owned();
    let canonical = ty
        .get("desugaredQualType")
        .and_then(Value::as_str)
        .map_or_else(|| written.clone(), str::to_owned);
    Some((written, canonical))
}

fn kind(node: &Value) -> &str {
    node.get("kind").and_then(Value::as_str).unwrap_or("")
}

fn inner(node: &Value) -> &[Value] {
    node.get("inner")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// One record's layout as the dump states it: `(name, offset)` of each direct
/// member in order, the size and the alignment.
type DumpedLayout = (Vec<(String, u64)>, u64, u64);

/// Direct members of `struct TAG` from Clang's record-layout dump:
/// `(name, offset)` in order, and `(size, align)`.
fn parse_layout(dump: &str, tag: &str) -> Result<DumpedLayout, LayoutReadRefusal> {
    let header = format!("struct {tag}");
    let mut found = None;
    for section in dump.split("*** Dumping AST Record Layout").skip(1) {
        let mut lines = section.lines().filter(|line| !line.trim().is_empty());
        let Some(first) = lines.next() else {
            continue;
        };
        if first.split_once('|').map(|(_, rest)| rest.trim()) != Some(header.as_str()) {
            continue;
        }
        if found.is_some() {
            return Err(refuse(format!(
                "the recorded compiler dumps two layouts of `{header}`"
            )));
        }
        let mut members = Vec::new();
        let mut size_align = None;
        for line in lines {
            let (offset, rest) = line
                .split_once('|')
                .ok_or_else(|| refuse(format!("unreadable layout line `{line}`")))?;
            if let Some(tail) = rest.trim().strip_prefix("[sizeof=") {
                let mut parts = tail.trim_end_matches(']').split(", ");
                let size = parts.next().and_then(|part| part.parse::<u64>().ok());
                let align = parts
                    .find_map(|part| part.strip_prefix("align="))
                    .and_then(|part| part.trim_end_matches(']').parse::<u64>().ok());
                size_align = size.zip(align);
                break;
            }
            let rest = rest
                .strip_prefix(' ')
                .ok_or_else(|| refuse(format!("unreadable layout line `{line}`")))?;
            // A direct member is indented by exactly two spaces.
            if !(rest.starts_with("  ") && !rest.starts_with("   ")) {
                continue;
            }
            let offset = offset.trim();
            let offset: u64 = offset.parse().map_err(|_| {
                refuse(format!(
                    "`{header}` has a member at `{offset}`, a bit position this reader does not state"
                ))
            })?;
            let name = rest
                .split_whitespace()
                .last()
                .ok_or_else(|| refuse(format!("unreadable layout line `{line}`")))?;
            members.push((name.to_owned(), offset));
        }
        let (size, align) =
            size_align.ok_or_else(|| refuse(format!("the layout of `{header}` states no size")))?;
        found = Some((members, size, align));
    }
    found.ok_or_else(|| {
        refuse(format!(
            "the recorded compiler lays out no complete `{header}`"
        ))
    })
}

/// The running Clang's `(first --version line, -dumpmachine)`: what a builder
/// records as the target's `compiler` and `triple` when this Clang is the one
/// that builds the emitted C.
///
/// # Errors
///
/// When no Clang is discoverable.
pub fn running_compiler() -> Result<(String, String), String> {
    let identity = crate::clang::ClangIdentity::capture()?;
    Ok((
        identity
            .version()
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_owned(),
        identity.target().to_owned(),
    ))
}

/// Read `struct TAG`'s layout from `H` under the recorded compiler.
///
/// # Errors
///
/// A [`LayoutReadRefusal`] for every header, compile context or struct shape
/// this reader cannot state.
pub fn read_struct_layout(
    request: &StructLayoutRequest<'_>,
) -> Result<CStructFacts, LayoutReadRefusal> {
    let tag = request.tag;
    let header = request.source_directory.join(request.header);
    let bytes = std::fs::read(&header).map_err(|error| {
        refuse(format!(
            "the header `{}` does not resolve: `{}` is unreadable ({error}); a header resolves against the importing file's own directory, with no search",
            request.header,
            header.display()
        ))
    })?;
    let header_path = header.canonicalize().map_err(|error| {
        refuse(format!(
            "the header `{}` does not resolve: {error}",
            request.header
        ))
    })?;
    let directory = header_path
        .parent()
        .ok_or_else(|| refuse("the header has no directory"))?
        .to_owned();
    let path_text = header_path
        .to_str()
        .ok_or_else(|| refuse("the header path is not UTF-8"))?
        .to_owned();
    // Header names do not interpret C string escapes. Windows Clang accepts
    // native backslashes, including the resolved verbatim path prefix. Keep
    // that spelling so the probe and retained identity refer to the same path.
    if path_text.contains(['"', '\r', '\n']) || (!cfg!(windows) && path_text.contains('\\')) {
        return Err(refuse("the header path cannot be spelled in an `#include`"));
    }
    let identity = crate::clang::ClangIdentity::capture()
        .map_err(|error| refuse(format!("the recorded compiler is unavailable: {error}")))?;
    let running = identity.version().lines().next().unwrap_or("").trim();
    if running != request.recorded_compiler.trim()
        || identity.target() != request.recorded_triple.trim()
    {
        return Err(refuse(format!(
            "the running compiler is `{running}` for `{}`, not the recorded `{}` for `{}`; a layout is read only by the recorded compiler",
            identity.target(),
            request.recorded_compiler,
            request.recorded_triple
        )));
    }
    let mut probe =
        String::from("#include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n");
    probe.push_str(&format!("#include \"{path_text}\"\n"));
    // Card `W48-QR-FLOATING-POINT`: the float types are probed too, so a
    // `float`/`double` member can be listed as `f32`/`f64`.
    for ty in st::ALL.into_iter().chain(st::FLOATS) {
        probe.push_str(&format!(
            "extern {} {};\n",
            st::c_type(ty),
            probe_name(tag, ty)
        ));
    }
    let include = directory
        .to_str()
        .ok_or_else(|| refuse("the header directory is not UTF-8"))?;
    let ast = clang(
        &directory,
        &[
            "-I",
            include,
            "-Xclang",
            "-ast-dump=json",
            "-Xclang",
            "-ast-dump-filter",
            "-Xclang",
            tag,
        ],
        &probe,
    )?;
    let dump = clang(
        &directory,
        &["-I", include, "-Xclang", "-fdump-record-layouts-complete"],
        &probe,
    )?;
    let mut nodes = Vec::new();
    let mut stream = serde_json::Deserializer::from_str(&ast).into_iter::<Value>();
    for node in &mut stream {
        nodes.push(node.map_err(|error| refuse(format!("unsupported Clang AST JSON: {error}")))?);
    }
    // The canonical type of every §G02t spelling under this compiler.
    let mut spellings = Vec::new();
    for ty in st::ALL.into_iter().chain(st::FLOATS) {
        let name = probe_name(tag, ty);
        let node = nodes
            .iter()
            .find(|node| {
                kind(node) == "VarDecl" && node.get("name").and_then(Value::as_str) == Some(&name)
            })
            .ok_or_else(|| {
                refuse(format!(
                    "the probe for `{ty}` is missing from the typed AST"
                ))
            })?;
        let (_, canonical_type) =
            canonical(node).ok_or_else(|| refuse(format!("the probe for `{ty}` has no type")))?;
        spellings.push((ty, canonical_type));
    }
    let records: Vec<&Value> = nodes
        .iter()
        .filter(|node| {
            kind(node) == "RecordDecl" && node.get("name").and_then(Value::as_str) == Some(tag)
        })
        .collect();
    let Some(record) = records
        .iter()
        .find(|node| node.get("completeDefinition").and_then(Value::as_bool) == Some(true))
    else {
        return Err(refuse(format!(
            "`struct {tag}` has no complete definition in `{}` under the recorded compiler: an unknown or incomplete tag",
            request.header
        )));
    };
    if record.get("tagUsed").and_then(Value::as_str) != Some("struct") {
        return Err(refuse(format!(
            "`{tag}` is a union, not a struct: a union has no field-by-field layout to import"
        )));
    }
    let mut members = Vec::new();
    for node in inner(record) {
        match kind(node) {
            "FieldDecl" => {
                let Some(name) = node.get("name").and_then(Value::as_str) else {
                    return Err(refuse(format!(
                        "`struct {tag}` has an anonymous member, which this reader does not state"
                    )));
                };
                if node.get("isBitfield").and_then(Value::as_bool) == Some(true) {
                    return Err(refuse(format!(
                        "`struct {tag}` has a bit-field `{name}`, whose storage this reader does not state"
                    )));
                }
                if let Some(attribute) =
                    inner(node).iter().find(|node| kind(node).ends_with("Attr"))
                {
                    return Err(refuse(format!(
                        "member `{name}` of `struct {tag}` carries `{}`, an attribute this reader does not state",
                        kind(attribute)
                    )));
                }
                let (written, canonical_type) = canonical(node).ok_or_else(|| {
                    refuse(format!("member `{name}` of `struct {tag}` has no type"))
                })?;
                if canonical_type.ends_with("[]") {
                    return Err(refuse(format!(
                        "`struct {tag}` has a flexible array member `{name}`, whose extent is not part of the layout"
                    )));
                }
                let scalars = spellings
                    .iter()
                    .filter(|(_, spelling)| *spelling == canonical_type)
                    .map(|(ty, _)| *ty)
                    .collect();
                members.push((name.to_owned(), written, canonical_type, scalars));
            }
            // A nested type definition is a type, not a member.
            "RecordDecl" | "EnumDecl" => {}
            "IndirectFieldDecl" => {
                return Err(refuse(format!(
                    "`struct {tag}` has an anonymous member, which this reader does not state"
                )));
            }
            other if other.ends_with("Attr") => {
                return Err(refuse(format!(
                    "`struct {tag}` carries `{other}`, an attribute this reader does not state"
                )));
            }
            other => {
                return Err(refuse(format!(
                    "`struct {tag}` contains `{other}`, which this reader does not state"
                )));
            }
        }
    }
    let (layout, size, align) = parse_layout(&dump, tag)?;
    if layout.len() != members.len()
        || layout
            .iter()
            .zip(&members)
            .any(|((laid, _), (member, ..))| laid != member)
    {
        return Err(refuse(format!(
            "the typed AST and the record layout of `struct {tag}` name different members"
        )));
    }
    let fields = members
        .into_iter()
        .zip(layout)
        .map(
            |((name, written_type, canonical_type, scalars), (_, offset))| CFieldFact {
                name,
                offset,
                written_type,
                canonical_type,
                scalars,
            },
        )
        .collect();
    Ok(CStructFacts {
        tag: tag.to_owned(),
        size,
        align,
        fields,
        header: request.header.to_owned(),
        header_path: path_text,
        header_directory: include.to_owned(),
        header_sha256: qcore_schemas::digest::sha256_hex(&bytes),
        compiler: running.to_owned(),
        triple: identity.target().to_owned(),
    })
}
