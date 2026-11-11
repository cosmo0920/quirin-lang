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

//! Mach-O: object symbol tables (`LC_SYMTAB` `nlist`) and a dylib's export
//! trie (`LC_DYLD_EXPORTS_TRIE`, or the export range of `LC_DYLD_INFO[_ONLY]`)
//! with its install name (`LC_ID_DYLIB`).
//!
//! Thin little-endian files only: a universal (fat) file is refused by name
//! (`W48-QRLIB-MACHO-FAT`), as is a big-endian one.

use super::bytes::{Bytes, Endian, ReadError, checked_extent};
use super::{Binding, Definition, SymbolRecord, SymbolTable};

const MH_MAGIC: u32 = 0xfeed_face;
const MH_MAGIC_64: u32 = 0xfeed_facf;
const FAT_MAGIC: u32 = 0xcafe_babe;

const MH_OBJECT: u32 = 1;
const MH_DYLIB: u32 = 6;

const LC_SYMTAB: u32 = 0x2;
const LC_ID_DYLIB: u32 = 0xd;
const LC_DYLD_INFO: u32 = 0x22;
const LC_DYLD_INFO_ONLY: u32 = 0x8000_0022;
const LC_DYLD_EXPORTS_TRIE: u32 = 0x8000_0033;

const N_STAB: u8 = 0xe0;
const N_PEXT: u8 = 0x10;
const N_TYPE: u8 = 0x0e;
const N_EXT: u8 = 0x01;
const N_UNDF: u8 = 0x0;
const N_WEAK_DEF: u16 = 0x0080;

const EXPORT_WEAK_DEFINITION: u64 = 0x04;
const EXPORT_REEXPORT: u64 = 0x08;
const EXPORT_STUB_AND_RESOLVER: u64 = 0x10;

/// Whether `data` starts with a Mach-O (thin or fat) magic.
#[must_use]
pub fn is_macho(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }
    let little = u32::from_le_bytes(data[..4].try_into().expect("four bytes"));
    let big = u32::from_be_bytes(data[..4].try_into().expect("four bytes"));
    matches!(little, MH_MAGIC | MH_MAGIC_64) || big == FAT_MAGIC
}

struct Header {
    wide: bool,
    filetype: u32,
    ncmds: u64,
    commands_at: u64,
}

fn header(bytes: &Bytes<'_>) -> Result<Header, ReadError> {
    if bytes.u32(0, Endian::Big)? == FAT_MAGIC {
        return Err(ReadError::new(
            "Mach-O header",
            "a universal (fat) file holds several architectures; only a thin file is read",
        ));
    }
    let magic = bytes.u32(0, Endian::Little)?;
    let wide = match magic {
        MH_MAGIC => false,
        MH_MAGIC_64 => true,
        _ => {
            return Err(ReadError::new(
                "Mach-O header",
                "not a thin little-endian Mach-O file",
            ));
        }
    };
    Ok(Header {
        wide,
        filetype: bytes.u32(12, Endian::Little)?,
        ncmds: u64::from(bytes.u32(16, Endian::Little)?),
        commands_at: if wide { 32 } else { 28 },
    })
}

/// Every load command as `(command, offset)`.
fn commands(bytes: &Bytes<'_>, header: &Header) -> Result<Vec<(u32, u64)>, ReadError> {
    let mut out = Vec::new();
    let mut at = header.commands_at;
    for _ in 0..header.ncmds {
        let command = bytes.u32(at, Endian::Little)?;
        let size = u64::from(bytes.u32(at + 4, Endian::Little)?);
        if size < 8 {
            return Err(ReadError::new(
                "Mach-O load command",
                format!("command {command:#x} at offset {at} has size {size}"),
            ));
        }
        bytes.slice(at, size)?;
        out.push((command, at));
        at += size;
    }
    Ok(out)
}

/// The symbols of a Mach-O object (`MH_OBJECT`), `origin` on every record.
///
/// # Errors
///
/// A malformed, fat, big-endian or non-object file.
pub fn object_symbols(data: &[u8], origin: &str) -> Result<SymbolTable, ReadError> {
    let bytes = Bytes::new(data, "Mach-O object");
    let header = header(&bytes)?;
    if header.filetype != MH_OBJECT {
        return Err(ReadError::new(
            "Mach-O object",
            format!("{origin} is not an object file (type {})", header.filetype),
        ));
    }
    let mut table = SymbolTable::default();
    for (command, at) in commands(&bytes, &header)? {
        if command != LC_SYMTAB {
            continue;
        }
        let symoff = u64::from(bytes.u32(at + 8, Endian::Little)?);
        let nsyms = u64::from(bytes.u32(at + 12, Endian::Little)?);
        let stroff = u64::from(bytes.u32(at + 16, Endian::Little)?);
        let strsize = u64::from(bytes.u32(at + 20, Endian::Little)?);
        let strings = Bytes::new(bytes.slice(stroff, strsize)?, "Mach-O string table");
        let entry = if header.wide { 16 } else { 12 };
        let symbols = Bytes::new(
            bytes.slice(symoff, checked_extent("Mach-O symbol table", nsyms, entry)?)?,
            "Mach-O symbol table",
        );
        for index in 0..nsyms {
            let base = index * entry;
            let strx = u64::from(symbols.u32(base, Endian::Little)?);
            let n_type = symbols.u8(base + 4)?;
            let n_desc = symbols.u16(base + 6, Endian::Little)?;
            if n_type & N_STAB != 0 {
                continue;
            }
            let name = strings.c_string(strx)?;
            if name.is_empty() {
                continue;
            }
            let definition = if n_type & N_EXT == 0 || n_type & N_PEXT != 0 {
                if n_type & N_TYPE == N_UNDF {
                    continue;
                }
                Definition::Local
            } else if n_type & N_TYPE == N_UNDF {
                Definition::Undefined
            } else {
                Definition::Defined {
                    binding: if n_desc & N_WEAK_DEF != 0 {
                        Binding::Weak
                    } else {
                        Binding::Global
                    },
                    ifunc: false,
                }
            };
            table.records.push(SymbolRecord {
                name: name.to_owned(),
                definition,
                version: None,
                origin: origin.to_owned(),
                indexed: None,
                import: None,
            });
        }
    }
    Ok(table)
}

/// What a dylib exports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dylib {
    pub install_name: Option<String>,
    /// Every exported name. A per-symbol re-export (to another dylib) is
    /// recorded as [`Definition::Local`]: the code is not in this image.
    pub table: SymbolTable,
}

/// The export trie and install name of a dylib (`MH_DYLIB`).
///
/// # Errors
///
/// A malformed, fat or non-dylib file, or a trie that does not terminate.
pub fn dylib(data: &[u8], origin: &str) -> Result<Dylib, ReadError> {
    let bytes = Bytes::new(data, "Mach-O dylib");
    let header = header(&bytes)?;
    if header.filetype != MH_DYLIB {
        return Err(ReadError::new(
            "Mach-O dylib",
            format!(
                "{origin} is not a dynamic library (type {})",
                header.filetype
            ),
        ));
    }
    let mut install_name = None;
    let mut trie: Option<(u64, u64)> = None;
    for (command, at) in commands(&bytes, &header)? {
        match command {
            LC_ID_DYLIB => {
                let offset = u64::from(bytes.u32(at + 8, Endian::Little)?);
                install_name = Some(bytes.c_string(at + offset)?.to_owned());
            }
            LC_DYLD_EXPORTS_TRIE => {
                trie = Some((
                    u64::from(bytes.u32(at + 8, Endian::Little)?),
                    u64::from(bytes.u32(at + 12, Endian::Little)?),
                ));
            }
            LC_DYLD_INFO | LC_DYLD_INFO_ONLY if trie.is_none() => {
                trie = Some((
                    u64::from(bytes.u32(at + 40, Endian::Little)?),
                    u64::from(bytes.u32(at + 44, Endian::Little)?),
                ));
            }
            _ => {}
        }
    }
    let (offset, size) = trie.ok_or_else(|| {
        ReadError::new("Mach-O dylib", format!("{origin} carries no export trie"))
    })?;
    let trie = Bytes::new(bytes.slice(offset, size)?, "Mach-O export trie");
    let mut table = SymbolTable::default();
    if size > 0 {
        walk_trie(&trie, 0, String::new(), origin, &mut table, 0)?;
    }
    Ok(Dylib {
        install_name,
        table,
    })
}

fn walk_trie(
    trie: &Bytes<'_>,
    node: u64,
    prefix: String,
    origin: &str,
    table: &mut SymbolTable,
    depth: usize,
) -> Result<(), ReadError> {
    if depth > 4096 {
        return Err(ReadError::new(
            "Mach-O export trie",
            "the trie is deeper than 4096 edges",
        ));
    }
    let mut at = node;
    let terminal = trie.uleb128(&mut at)?;
    let children_at = at + terminal;
    if terminal > 0 {
        let flags = trie.uleb128(&mut at)?;
        let definition = if flags & EXPORT_REEXPORT != 0 {
            Definition::Local
        } else {
            Definition::Defined {
                binding: if flags & EXPORT_WEAK_DEFINITION != 0 {
                    Binding::Weak
                } else {
                    Binding::Global
                },
                ifunc: flags & EXPORT_STUB_AND_RESOLVER != 0,
            }
        };
        table.records.push(SymbolRecord {
            name: prefix.clone(),
            definition,
            version: None,
            origin: origin.to_owned(),
            indexed: None,
            import: None,
        });
    }
    let mut at = children_at;
    let count = trie.u8(at)?;
    at += 1;
    for _ in 0..count {
        let edge = trie.c_string(at)?;
        at += edge.len() as u64 + 1;
        let child = trie.uleb128(&mut at)?;
        if child == 0 || child >= trie.len() as u64 {
            return Err(ReadError::new(
                "Mach-O export trie",
                format!("an edge `{edge}` points at offset {child}, outside the trie"),
            ));
        }
        walk_trie(
            trie,
            child,
            format!("{prefix}{edge}"),
            origin,
            table,
            depth + 1,
        )?;
    }
    Ok(())
}
