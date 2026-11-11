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

//! `ar` archives: the container every static library format shares.
//!
//! Three layouts are read, each with its own symbol index:
//!
//! * **GNU/SysV** (ELF `.a`): the index member is `/` (32-bit, big-endian
//!   offsets) or `/SYM64/` (64-bit); long member names live in `//` and are
//!   referenced as `/OFFSET`.
//! * **BSD/Darwin** (Mach-O `.a`, written by `ranlib`/`libtool`): the index
//!   member is `__.SYMDEF`, `__.SYMDEF SORTED` or `__.SYMDEF_64` (possibly
//!   behind a `#1/LEN` long name), in the target's byte order.
//! * **COFF** (`.lib`): the GNU-style first linker member `/`, then an optional
//!   second linker member `/` in little-endian with sorted names; long names
//!   in `//`.
//!
//! The index is what a linker consults to pull a member, so it is read and
//! kept: a definition the index does not list is not one the linker finds.
//! Thin archives (`!<thin>`) name files outside themselves and are refused.

use std::collections::{BTreeMap, BTreeSet};

use super::bytes::{Bytes, Endian, ReadError, checked_extent};

const MAGIC: &[u8] = b"!<arch>\n";
const THIN_MAGIC: &[u8] = b"!<thin>\n";
const HEADER: u64 = 60;

/// Which index layout an archive carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexKind {
    /// `/` (GNU/SysV or COFF first linker member).
    Gnu32,
    /// `/SYM64/`.
    Gnu64,
    /// The COFF second linker member (little-endian, sorted).
    CoffSecond,
    /// `__.SYMDEF` / `__.SYMDEF SORTED`, 32-bit.
    Bsd32,
    /// `__.SYMDEF_64` / `__.SYMDEF_64 SORTED`.
    Bsd64,
}

/// One ordinary member: its name and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member<'a> {
    pub name: String,
    /// The offset of the member's 60-byte header in the archive, which is what
    /// every index layout points at.
    pub header_offset: u64,
    pub data: &'a [u8],
}

/// A read archive: its ordinary members, in order, and its index.
#[derive(Clone, Debug)]
pub struct Archive<'a> {
    pub members: Vec<Member<'a>>,
    /// Every index layout present, in archive order.
    pub index_kinds: Vec<IndexKind>,
    /// Symbol name → the header offsets of the members the index names for it.
    /// Empty when the archive has no index.
    pub index: BTreeMap<String, BTreeSet<u64>>,
}

impl Archive<'_> {
    /// Whether the index lists `symbol` in the member at `header_offset`.
    #[must_use]
    pub fn indexes(&self, symbol: &str, header_offset: u64) -> bool {
        self.index
            .get(symbol)
            .is_some_and(|offsets| offsets.contains(&header_offset))
    }
}

/// Whether `data` starts with the `ar` magic.
#[must_use]
pub fn is_archive(data: &[u8]) -> bool {
    data.starts_with(MAGIC) || data.starts_with(THIN_MAGIC)
}

fn field(bytes: &Bytes<'_>, offset: u64, length: u64) -> Result<String, ReadError> {
    let raw = bytes.slice(offset, length)?;
    std::str::from_utf8(raw)
        .map(|text| text.trim_end().to_owned())
        .map_err(|_| ReadError::new("ar member header", "a header field is not ASCII"))
}

/// Read an archive.
///
/// # Errors
///
/// A [`ReadError`] for a thin archive, a truncated member, a malformed header
/// or an index whose offsets or counts do not fit.
pub fn read(data: &[u8]) -> Result<Archive<'_>, ReadError> {
    if data.starts_with(THIN_MAGIC) {
        return Err(ReadError::new(
            "ar archive",
            "a thin archive names member files outside itself; only a self-contained archive is read",
        ));
    }
    if !data.starts_with(MAGIC) {
        return Err(ReadError::new(
            "ar archive",
            "the `!<arch>` magic is absent",
        ));
    }
    let bytes = Bytes::new(data, "ar archive");
    let mut offset = MAGIC.len() as u64;
    let mut long_names: Option<&[u8]> = None;
    let mut raw_members = Vec::new();
    let mut indexes: Vec<(IndexKind, &[u8])> = Vec::new();
    let mut first_slash_seen = false;
    while offset < bytes.len() as u64 {
        // A single `\n` pads a member to an even offset; tolerate it at the end.
        if bytes.len() as u64 - offset < HEADER {
            if bytes
                .slice(offset, bytes.len() as u64 - offset)?
                .iter()
                .all(|b| *b == b'\n')
            {
                break;
            }
            return Err(ReadError::new("ar archive", "a truncated member header"));
        }
        let header_offset = offset;
        let name = field(&bytes, offset, 16)?;
        let size_text = field(&bytes, offset + 48, 10)?;
        if bytes.slice(offset + 58, 2)? != b"`\n" {
            return Err(ReadError::new(
                "ar member header",
                format!("the header at offset {offset} does not end with the `\\`\\n` marker"),
            ));
        }
        let size: u64 = size_text.parse().map_err(|_| {
            ReadError::new(
                "ar member header",
                format!("the size `{size_text}` at offset {offset} is not a decimal number"),
            )
        })?;
        let mut body = bytes.slice(offset + HEADER, size)?;
        let mut member_name = name.clone();
        // BSD long name: `#1/LEN`, the name is the first LEN bytes of the body.
        if let Some(length) = name.strip_prefix("#1/") {
            let length: usize = length.parse().map_err(|_| {
                ReadError::new("ar member header", format!("`{name}` is not `#1/LEN`"))
            })?;
            if length > body.len() {
                return Err(ReadError::new(
                    "ar member header",
                    format!("`{name}` names more bytes than the member holds"),
                ));
            }
            member_name = String::from_utf8(body[..length].to_vec())
                .map_err(|_| ReadError::new("ar member header", "a BSD long name is not UTF-8"))?
                .trim_end_matches('\0')
                .to_owned();
            body = &body[length..];
        }
        match member_name.as_str() {
            "/" => {
                // The first `/` is the GNU/COFF first linker member; a second
                // `/` right after it is the COFF second linker member.
                if first_slash_seen {
                    indexes.push((IndexKind::CoffSecond, body));
                } else {
                    first_slash_seen = true;
                    indexes.push((IndexKind::Gnu32, body));
                }
            }
            "/SYM64/" => indexes.push((IndexKind::Gnu64, body)),
            "//" => long_names = Some(body),
            "__.SYMDEF" | "__.SYMDEF SORTED" => indexes.push((IndexKind::Bsd32, body)),
            "__.SYMDEF_64" | "__.SYMDEF_64 SORTED" => indexes.push((IndexKind::Bsd64, body)),
            _ => raw_members.push((header_offset, member_name, body)),
        }
        offset += HEADER + size;
        if offset % 2 == 1 {
            offset += 1;
        }
    }
    let mut members = Vec::with_capacity(raw_members.len());
    for (header_offset, name, body) in raw_members {
        let name = resolve_name(&name, long_names)?;
        members.push(Member {
            name,
            header_offset,
            data: body,
        });
    }
    let mut index: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    let mut index_kinds = Vec::new();
    for (kind, body) in indexes {
        index_kinds.push(kind);
        for (symbol, target) in read_index(kind, body, &members)? {
            index.entry(symbol).or_default().insert(target);
        }
    }
    Ok(Archive {
        members,
        index_kinds,
        index,
    })
}

/// A GNU/COFF member name: `name/`, `/OFFSET` into `//`, or a plain name.
fn resolve_name(name: &str, long_names: Option<&[u8]>) -> Result<String, ReadError> {
    if let Some(reference) = name.strip_prefix('/')
        && !reference.is_empty()
        && reference.bytes().all(|b| b.is_ascii_digit())
    {
        let offset: usize = reference.parse().map_err(|_| {
            ReadError::new(
                "ar member header",
                format!("`{name}` is not a long-name offset"),
            )
        })?;
        let table = long_names.ok_or_else(|| {
            ReadError::new(
                "ar member header",
                format!("`{name}` refers to a long-name table the archive does not have"),
            )
        })?;
        let rest = table.get(offset..).ok_or_else(|| {
            ReadError::new(
                "ar long-name table",
                format!("offset {offset} lies outside the table"),
            )
        })?;
        let end = rest
            .iter()
            .position(|b| *b == b'\n' || *b == 0)
            .unwrap_or(rest.len());
        let text = std::str::from_utf8(&rest[..end])
            .map_err(|_| ReadError::new("ar long-name table", "a name is not UTF-8"))?;
        return Ok(text.trim_end_matches('/').to_owned());
    }
    Ok(name.strip_suffix('/').unwrap_or(name).to_owned())
}

fn read_index(
    kind: IndexKind,
    body: &[u8],
    members: &[Member<'_>],
) -> Result<Vec<(String, u64)>, ReadError> {
    let what = "ar symbol index";
    let bytes = Bytes::new(body, what);
    let mut entries = Vec::new();
    match kind {
        IndexKind::Gnu32 | IndexKind::Gnu64 => {
            let width: u64 = if kind == IndexKind::Gnu32 { 4 } else { 8 };
            let count = if width == 4 {
                u64::from(bytes.u32(0, Endian::Big)?)
            } else {
                bytes.u64(0, Endian::Big)?
            };
            let mut names_at = width + checked_extent(what, count, width)?;
            for index in 0..count {
                let at = width + index * width;
                let target = if width == 4 {
                    u64::from(bytes.u32(at, Endian::Big)?)
                } else {
                    bytes.u64(at, Endian::Big)?
                };
                let name = bytes.c_string(names_at)?;
                names_at += name.len() as u64 + 1;
                entries.push((name.to_owned(), target));
            }
        }
        IndexKind::CoffSecond => {
            let member_count = u64::from(bytes.u32(0, Endian::Little)?);
            let offsets_at = 4;
            let symbols_at = offsets_at + checked_extent(what, member_count, 4)?;
            let symbol_count = u64::from(bytes.u32(symbols_at, Endian::Little)?);
            let indices_at = symbols_at + 4;
            let mut names_at = indices_at + checked_extent(what, symbol_count, 2)?;
            for index in 0..symbol_count {
                let member = u64::from(bytes.u16(indices_at + index * 2, Endian::Little)?);
                if member == 0 || member > member_count {
                    return Err(ReadError::new(
                        what,
                        format!("a second-linker-member index {member} names no member"),
                    ));
                }
                let target = u64::from(bytes.u32(offsets_at + (member - 1) * 4, Endian::Little)?);
                let name = bytes.c_string(names_at)?;
                names_at += name.len() as u64 + 1;
                entries.push((name.to_owned(), target));
            }
        }
        IndexKind::Bsd32 | IndexKind::Bsd64 => {
            // The byte order is the target's; every Mach-O target this reader
            // admits is little-endian, and a big-endian index fails the
            // member-offset check below rather than being guessed.
            let endian = Endian::Little;
            let width: u64 = if kind == IndexKind::Bsd32 { 4 } else { 8 };
            let read = |offset: u64| -> Result<u64, ReadError> {
                if width == 4 {
                    Ok(u64::from(bytes.u32(offset, endian)?))
                } else {
                    bytes.u64(offset, endian)
                }
            };
            let ranlib_bytes = read(0)?;
            let entry = 2 * width;
            if ranlib_bytes % entry != 0 {
                return Err(ReadError::new(
                    what,
                    "the `__.SYMDEF` table size is not a whole number of entries",
                ));
            }
            let strings_size_at = width + ranlib_bytes;
            let strings_size = read(strings_size_at)?;
            let strings_at = strings_size_at + width;
            let strings = Bytes::new(bytes.slice(strings_at, strings_size)?, what);
            for index in 0..ranlib_bytes / entry {
                let at = width + index * entry;
                let name_offset = read(at)?;
                let target = read(at + width)?;
                entries.push((strings.c_string(name_offset)?.to_owned(), target));
            }
        }
    }
    let known: BTreeSet<u64> = members.iter().map(|member| member.header_offset).collect();
    for (name, target) in &entries {
        if !known.contains(target) {
            return Err(ReadError::new(
                what,
                format!(
                    "the index entry for `{name}` points at offset {target}, where no member starts"
                ),
            ));
        }
    }
    Ok(entries)
}
