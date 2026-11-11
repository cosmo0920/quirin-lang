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

//! ELF: relocatable objects (`.symtab`) and shared objects (`.dynsym` with
//! `.gnu.version`/`.gnu.version_d`, and `DT_SONAME`).
//!
//! Both classes (32/64-bit) and both byte orders are read. Section headers are
//! required: a shared object with none (possible, since a loader needs only
//! program headers) is refused rather than read through the dynamic segment,
//! which is follow-on work (`W48-QRLIB-ELF-NO-SECTIONS`).

use super::bytes::{Bytes, Endian, ReadError, checked_extent};
use super::{Binding, Definition, SymbolRecord, SymbolTable, SymbolVersion};

const SHT_SYMTAB: u32 = 2;
const SHT_DYNAMIC: u32 = 6;
const SHT_DYNSYM: u32 = 11;
const SHT_GNU_VERDEF: u32 = 0x6fff_fffd;
const SHT_GNU_VERSYM: u32 = 0x6fff_ffff;

const ET_REL: u16 = 1;
const ET_DYN: u16 = 3;

const STB_LOCAL: u8 = 0;
const STB_GLOBAL: u8 = 1;
const STB_WEAK: u8 = 2;
const STB_GNU_UNIQUE: u8 = 10;

const STT_SECTION: u8 = 3;
const STT_FILE: u8 = 4;
const STT_GNU_IFUNC: u8 = 10;

const SHN_UNDEF: u16 = 0;

const STV_HIDDEN: u8 = 2;
const STV_INTERNAL: u8 = 1;

const DT_NULL: u64 = 0;
const DT_SONAME: u64 = 14;

const VERSYM_HIDDEN: u16 = 0x8000;

/// Whether `data` starts with the ELF magic.
#[must_use]
pub fn is_elf(data: &[u8]) -> bool {
    data.starts_with(b"\x7fELF")
}

/// The kind of ELF file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElfKind {
    Relocatable,
    Shared,
}

#[derive(Clone, Copy)]
struct Header {
    wide: bool,
    endian: Endian,
    kind: u16,
    machine: u16,
    shoff: u64,
    shentsize: u64,
    shnum: u64,
}

#[derive(Clone, Copy, Debug)]
struct Section {
    kind: u32,
    offset: u64,
    size: u64,
    link: u32,
    entsize: u64,
}

fn header(bytes: &Bytes<'_>) -> Result<Header, ReadError> {
    if !is_elf(bytes.data()) {
        return Err(ReadError::new(
            "ELF header",
            "the `\\x7fELF` magic is absent",
        ));
    }
    let wide = match bytes.u8(4)? {
        1 => false,
        2 => true,
        other => {
            return Err(ReadError::new(
                "ELF header",
                format!("class {other} is neither ELF32 nor ELF64"),
            ));
        }
    };
    let endian = match bytes.u8(5)? {
        1 => Endian::Little,
        2 => Endian::Big,
        other => {
            return Err(ReadError::new(
                "ELF header",
                format!("data encoding {other} is neither little- nor big-endian"),
            ));
        }
    };
    let kind = bytes.u16(16, endian)?;
    let machine = bytes.u16(18, endian)?;
    let (shoff, shentsize, shnum) = if wide {
        (
            bytes.u64(40, endian)?,
            u64::from(bytes.u16(58, endian)?),
            u64::from(bytes.u16(60, endian)?),
        )
    } else {
        (
            u64::from(bytes.u32(32, endian)?),
            u64::from(bytes.u16(46, endian)?),
            u64::from(bytes.u16(48, endian)?),
        )
    };
    Ok(Header {
        wide,
        endian,
        kind,
        machine,
        shoff,
        shentsize,
        shnum,
    })
}

fn sections(bytes: &Bytes<'_>, header: &Header) -> Result<Vec<Section>, ReadError> {
    if header.shnum == 0 || header.shoff == 0 {
        return Err(ReadError::new(
            "ELF section headers",
            "the file has no section headers; reading exports through the dynamic segment alone is not done",
        ));
    }
    let minimum = if header.wide { 64 } else { 40 };
    if header.shentsize < minimum {
        return Err(ReadError::new(
            "ELF section headers",
            format!(
                "section header size {} is below {minimum}",
                header.shentsize
            ),
        ));
    }
    checked_extent("ELF section headers", header.shnum, header.shentsize)?;
    let mut sections = Vec::with_capacity(header.shnum as usize);
    for index in 0..header.shnum {
        let at = header.shoff + index * header.shentsize;
        let e = header.endian;
        let section = if header.wide {
            Section {
                kind: bytes.u32(at + 4, e)?,
                offset: bytes.u64(at + 24, e)?,
                size: bytes.u64(at + 32, e)?,
                link: bytes.u32(at + 40, e)?,
                entsize: bytes.u64(at + 56, e)?,
            }
        } else {
            Section {
                kind: bytes.u32(at + 4, e)?,
                offset: u64::from(bytes.u32(at + 16, e)?),
                size: u64::from(bytes.u32(at + 20, e)?),
                link: bytes.u32(at + 24, e)?,
                entsize: u64::from(bytes.u32(at + 36, e)?),
            }
        };
        sections.push(section);
    }
    Ok(sections)
}

/// One raw symbol.
struct RawSymbol {
    name: String,
    info: u8,
    other: u8,
    shndx: u16,
}

fn symbols(
    bytes: &Bytes<'_>,
    header: &Header,
    sections: &[Section],
    table: &Section,
) -> Result<Vec<RawSymbol>, ReadError> {
    let entsize = if header.wide { 24 } else { 16 };
    if table.entsize != 0 && table.entsize < entsize {
        return Err(ReadError::new(
            "ELF symbol table",
            format!("entry size {} is below {entsize}", table.entsize),
        ));
    }
    let step = if table.entsize == 0 {
        entsize
    } else {
        table.entsize
    };
    let strings = sections.get(table.link as usize).ok_or_else(|| {
        ReadError::new("ELF symbol table", "its string table link names no section")
    })?;
    let strings = Bytes::new(
        bytes.slice(strings.offset, strings.size)?,
        "ELF string table",
    );
    let raw = Bytes::new(bytes.slice(table.offset, table.size)?, "ELF symbol table");
    let count = table.size / step;
    let mut out = Vec::with_capacity(count as usize);
    let e = header.endian;
    for index in 0..count {
        let at = index * step;
        let (name, info, other, shndx) = if header.wide {
            (
                raw.u32(at, e)?,
                raw.u8(at + 4)?,
                raw.u8(at + 5)?,
                raw.u16(at + 6, e)?,
            )
        } else {
            (
                raw.u32(at, e)?,
                raw.u8(at + 12)?,
                raw.u8(at + 13)?,
                raw.u16(at + 14, e)?,
            )
        };
        out.push(RawSymbol {
            name: strings.c_string(u64::from(name))?.to_owned(),
            info,
            other,
            shndx,
        });
    }
    Ok(out)
}

fn classify(symbol: &RawSymbol, shared: bool) -> Option<Definition> {
    let bind = symbol.info >> 4;
    let kind = symbol.info & 0xf;
    if symbol.name.is_empty() || kind == STT_SECTION || kind == STT_FILE {
        return None;
    }
    let visibility = symbol.other & 0x3;
    if symbol.shndx == SHN_UNDEF {
        return match bind {
            STB_GLOBAL | STB_WEAK | STB_GNU_UNIQUE => Some(Definition::Undefined),
            _ => None,
        };
    }
    let binding = match bind {
        STB_GLOBAL | STB_GNU_UNIQUE => Binding::Global,
        STB_WEAK => Binding::Weak,
        STB_LOCAL => return Some(Definition::Local),
        _ => return Some(Definition::Local),
    };
    // A hidden or internal symbol of a shared object is not exported; in a
    // relocatable object it still links statically, so it stays defined.
    if shared && (visibility == STV_HIDDEN || visibility == STV_INTERNAL) {
        return Some(Definition::Local);
    }
    Some(Definition::Defined {
        binding,
        ifunc: kind == STT_GNU_IFUNC,
    })
}

/// The file kind, the machine and whether it is 64-bit.
///
/// # Errors
///
/// A malformed header.
pub fn identify(data: &[u8]) -> Result<(ElfKind, u16, bool), ReadError> {
    let bytes = Bytes::new(data, "ELF file");
    let header = header(&bytes)?;
    let kind = match header.kind {
        ET_REL => ElfKind::Relocatable,
        ET_DYN => ElfKind::Shared,
        other => {
            return Err(ReadError::new(
                "ELF header",
                format!("file type {other} is neither a relocatable object nor a shared object"),
            ));
        }
    };
    Ok((kind, header.machine, header.wide))
}

/// The symbols of a relocatable object (`.symtab`), with `origin` recorded on
/// every record.
///
/// # Errors
///
/// A malformed file, or one that is not `ET_REL`.
pub fn object_symbols(data: &[u8], origin: &str) -> Result<SymbolTable, ReadError> {
    let bytes = Bytes::new(data, "ELF object");
    let header = header(&bytes)?;
    if header.kind != ET_REL {
        return Err(ReadError::new(
            "ELF object",
            format!("{origin} is not a relocatable object"),
        ));
    }
    let sections = sections(&bytes, &header)?;
    let mut table = SymbolTable::default();
    // An object with no `.symtab` defines nothing a link can use: stripped.
    for section in sections.iter().filter(|s| s.kind == SHT_SYMTAB) {
        for symbol in symbols(&bytes, &header, &sections, section)? {
            if let Some(definition) = classify(&symbol, false) {
                table.records.push(SymbolRecord {
                    name: symbol.name,
                    definition,
                    version: None,
                    origin: origin.to_owned(),
                    indexed: None,
                    import: None,
                });
            }
        }
    }
    Ok(table)
}

/// What a shared object exports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedObject {
    pub soname: Option<String>,
    pub table: SymbolTable,
}

/// The dynamic symbols of a shared object (`.dynsym`), each with its version
/// when `.gnu.version` gives one, and the `DT_SONAME`.
///
/// # Errors
///
/// A malformed file, one that is not `ET_DYN`, or one with no `.dynsym`.
pub fn shared_object(data: &[u8], origin: &str) -> Result<SharedObject, ReadError> {
    let bytes = Bytes::new(data, "ELF shared object");
    let header = header(&bytes)?;
    if header.kind != ET_DYN {
        return Err(ReadError::new(
            "ELF shared object",
            format!("{origin} is not a shared object"),
        ));
    }
    let sections = sections(&bytes, &header)?;
    let dynsym = sections
        .iter()
        .find(|s| s.kind == SHT_DYNSYM)
        .ok_or_else(|| {
            ReadError::new(
                "ELF shared object",
                format!("{origin} has no `.dynsym`, so it exports nothing"),
            )
        })?;
    let raw = symbols(&bytes, &header, &sections, dynsym)?;
    let versions = version_names(&bytes, &header, &sections)?;
    let versym = sections.iter().find(|s| s.kind == SHT_GNU_VERSYM);
    let e = header.endian;
    let mut table = SymbolTable::default();
    for (index, symbol) in raw.into_iter().enumerate() {
        let Some(definition) = classify(&symbol, true) else {
            continue;
        };
        let version = match (versym, &definition) {
            (Some(versym), Definition::Defined { .. }) => {
                let value =
                    Bytes::new(bytes.slice(versym.offset, versym.size)?, "ELF .gnu.version")
                        .u16(index as u64 * 2, e)?;
                let hidden = value & VERSYM_HIDDEN != 0;
                let number = value & !VERSYM_HIDDEN;
                if number <= 1 {
                    // 0 is local, 1 is the unversioned global scope.
                    if number == 0 {
                        continue;
                    }
                    None
                } else {
                    let name = versions.iter().find(|(n, _)| *n == number).map(|(_, name)| name.clone()).ok_or_else(|| {
                        ReadError::new(
                            "ELF .gnu.version",
                            format!("`{}` names version index {number}, which `.gnu.version_d` does not define", symbol.name),
                        )
                    })?;
                    Some(SymbolVersion {
                        name,
                        default: !hidden,
                    })
                }
            }
            _ => None,
        };
        table.records.push(SymbolRecord {
            name: symbol.name,
            definition,
            version,
            origin: origin.to_owned(),
            indexed: None,
            import: None,
        });
    }
    Ok(SharedObject {
        soname: soname(&bytes, &header, &sections)?,
        table,
    })
}

/// `(index, name)` of every version `.gnu.version_d` defines.
fn version_names(
    bytes: &Bytes<'_>,
    header: &Header,
    sections: &[Section],
) -> Result<Vec<(u16, String)>, ReadError> {
    let Some(verdef) = sections.iter().find(|s| s.kind == SHT_GNU_VERDEF) else {
        return Ok(Vec::new());
    };
    let strings = sections.get(verdef.link as usize).ok_or_else(|| {
        ReadError::new(
            "ELF .gnu.version_d",
            "its string table link names no section",
        )
    })?;
    let strings = Bytes::new(
        bytes.slice(strings.offset, strings.size)?,
        "ELF string table",
    );
    let raw = Bytes::new(
        bytes.slice(verdef.offset, verdef.size)?,
        "ELF .gnu.version_d",
    );
    let e = header.endian;
    let mut out = Vec::new();
    let mut at = 0u64;
    // At most one entry per 20 bytes: a bound on a malformed `vd_next` chain.
    for _ in 0..=(verdef.size / 20) {
        let index = raw.u16(at + 4, e)?;
        let aux = u64::from(raw.u32(at + 12, e)?);
        let next = u64::from(raw.u32(at + 16, e)?);
        let name = raw.u32(at + aux, e)?;
        out.push((index, strings.c_string(u64::from(name))?.to_owned()));
        if next == 0 {
            return Ok(out);
        }
        at += next;
    }
    Err(ReadError::new(
        "ELF .gnu.version_d",
        "the version definition chain does not terminate",
    ))
}

fn soname(
    bytes: &Bytes<'_>,
    header: &Header,
    sections: &[Section],
) -> Result<Option<String>, ReadError> {
    let Some(dynamic) = sections.iter().find(|s| s.kind == SHT_DYNAMIC) else {
        return Ok(None);
    };
    let strings = sections
        .get(dynamic.link as usize)
        .ok_or_else(|| ReadError::new("ELF .dynamic", "its string table link names no section"))?;
    let strings = Bytes::new(
        bytes.slice(strings.offset, strings.size)?,
        "ELF string table",
    );
    let raw = Bytes::new(bytes.slice(dynamic.offset, dynamic.size)?, "ELF .dynamic");
    let e = header.endian;
    let step = if header.wide { 16 } else { 8 };
    let mut at = 0u64;
    while at + step <= dynamic.size {
        let (tag, value) = if header.wide {
            (raw.u64(at, e)?, raw.u64(at + 8, e)?)
        } else {
            (u64::from(raw.u32(at, e)?), u64::from(raw.u32(at + 4, e)?))
        };
        if tag == DT_NULL {
            break;
        }
        if tag == DT_SONAME {
            return Ok(Some(strings.c_string(value)?.to_owned()));
        }
        at += step;
    }
    Ok(None)
}
