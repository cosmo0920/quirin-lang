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

//! COFF and PE: object symbol tables (static `.lib` members), short import
//! records (import-library members), and the PE export directory of a DLL.
//!
//! * An object's symbol table gives `IMAGE_SYM_CLASS_EXTERNAL` symbols with a
//!   section number as defined, section 0 with value 0 as undefined, and
//!   `IMAGE_SYM_CLASS_WEAK_EXTERNAL` as a defined weak symbol (a weak external
//!   always carries a default); `STATIC` and others as local. `/bigobj` and
//!   link-time-code-generation (anonymous) objects are refused by name
//!   (`W48-QRLIB-COFF-BIGOBJ-LTCG`).
//! * A short import record (`Sig1 = 0`, `Sig2 = 0xFFFF`) defines `__imp_NAME`
//!   and, for code, `NAME`; its name type says whether the import is by name
//!   or by ordinal only.
//! * The PE export directory lists each export by name and says whether it is
//!   a **forwarder** (its address lies inside the export directory and is the
//!   text `DLL.NAME`).

use super::bytes::{Bytes, Endian, ReadError, checked_extent};
use super::{Binding, Definition, ImportFacts, SymbolRecord, SymbolTable};

const E: Endian = Endian::Little;

const IMAGE_SYM_CLASS_EXTERNAL: u8 = 2;
const IMAGE_SYM_CLASS_WEAK_EXTERNAL: u8 = 105;
const IMAGE_SYM_UNDEFINED: i16 = 0;
const IMAGE_SYM_DEBUG: i16 = -2;

/// Machine types a COFF member may carry.
#[must_use]
pub fn machine_name(machine: u16) -> &'static str {
    match machine {
        0x8664 => "x86_64",
        0x14c => "i386",
        0xaa64 => "arm64",
        0xa641 => "arm64ec",
        0x1c4 => "armv7",
        0 => "unknown",
        _ => "other",
    }
}

/// Whether an import-library record belongs to a companion architecture that
/// may coexist with `target` in one Windows SDK archive but must not contribute
/// symbols to that target's table.
#[must_use]
pub fn is_companion_import_machine(machine: u16, target: u16) -> bool {
    target == 0xaa64 && machine == 0xa641
}

/// Whether `data` is a short import record.
#[must_use]
pub fn is_short_import(data: &[u8]) -> bool {
    data.len() >= 20 && data[0] == 0 && data[1] == 0 && data[2] == 0xff && data[3] == 0xff
}

/// Whether `data` plausibly begins a COFF object: a known machine and a sane
/// header.
#[must_use]
pub fn is_coff_object(data: &[u8]) -> bool {
    if data.len() < 20 || is_short_import(data) {
        return false;
    }
    let machine = u16::from_le_bytes([data[0], data[1]]);
    matches!(machine, 0x8664 | 0x14c | 0xaa64 | 0x1c4)
        || (machine == 0 && data[2] == 0xff && data[3] == 0xff)
}

/// The machine of a COFF object or import record.
///
/// # Errors
///
/// A truncated header.
pub fn machine(data: &[u8]) -> Result<u16, ReadError> {
    let bytes = Bytes::new(data, "COFF header");
    if is_short_import(data) {
        bytes.u16(6, E)
    } else {
        bytes.u16(0, E)
    }
}

/// The symbols of a COFF object, `origin` on every record.
///
/// # Errors
///
/// A malformed object, or a `/bigobj` or anonymous (LTCG) one.
pub fn object_symbols(data: &[u8], origin: &str) -> Result<SymbolTable, ReadError> {
    let bytes = Bytes::new(data, "COFF object");
    let machine = bytes.u16(0, E)?;
    if machine == 0 && bytes.u16(2, E)? == 0xffff {
        return Err(ReadError::new(
            "COFF object",
            format!(
                "{origin} is an anonymous object (`/bigobj` or link-time code generation), whose symbol table is not read here"
            ),
        ));
    }
    let pointer = u64::from(bytes.u32(8, E)?);
    let count = u64::from(bytes.u32(12, E)?);
    let mut table = SymbolTable::default();
    if pointer == 0 || count == 0 {
        return Ok(table);
    }
    let strings_at = pointer + checked_extent("COFF symbol table", count, 18)?;
    let strings_size = u64::from(bytes.u32(strings_at, E)?);
    let strings = Bytes::new(
        bytes.slice(strings_at, strings_size.max(4))?,
        "COFF string table",
    );
    let mut index = 0;
    while index < count {
        let at = pointer + index * 18;
        let short = bytes.slice(at, 8)?;
        let name = if short[..4] == [0, 0, 0, 0] {
            let offset = u64::from(u32::from_le_bytes(short[4..8].try_into().expect("four")));
            strings.c_string(offset)?.to_owned()
        } else {
            let end = short.iter().position(|b| *b == 0).unwrap_or(8);
            String::from_utf8(short[..end].to_vec())
                .map_err(|_| ReadError::new("COFF symbol table", "a short name is not UTF-8"))?
        };
        let value = bytes.u32(at + 8, E)?;
        let section = bytes.u16(at + 12, E)? as i16;
        let class = bytes.u8(at + 16)?;
        let aux = u64::from(bytes.u8(at + 17)?);
        index += 1 + aux;
        let definition = match class {
            IMAGE_SYM_CLASS_EXTERNAL => {
                if section == IMAGE_SYM_UNDEFINED {
                    if value == 0 {
                        Definition::Undefined
                    } else {
                        // A common symbol: sized storage the linker allocates.
                        Definition::Defined {
                            binding: Binding::Global,
                            ifunc: false,
                        }
                    }
                } else if section == IMAGE_SYM_DEBUG {
                    continue;
                } else {
                    Definition::Defined {
                        binding: Binding::Global,
                        ifunc: false,
                    }
                }
            }
            IMAGE_SYM_CLASS_WEAK_EXTERNAL => Definition::Defined {
                binding: Binding::Weak,
                ifunc: false,
            },
            _ => {
                // Section symbols, statics, labels and files: not linkable from
                // outside the object. Section-name symbols are dropped.
                if name.starts_with('.') || name.is_empty() {
                    continue;
                }
                Definition::Local
            }
        };
        table.records.push(SymbolRecord {
            name,
            definition,
            version: None,
            origin: origin.to_owned(),
            indexed: None,
            import: None,
        });
    }
    Ok(table)
}

/// The symbols a short import record defines (`__imp_NAME` and, for code,
/// `NAME`), each carrying the import's facts.
///
/// # Errors
///
/// A malformed record.
pub fn import_symbols(data: &[u8], origin: &str) -> Result<SymbolTable, ReadError> {
    let bytes = Bytes::new(data, "COFF import record");
    if !is_short_import(data) {
        return Err(ReadError::new(
            "COFF import record",
            format!("{origin} is not a short import record"),
        ));
    }
    let size = u64::from(bytes.u32(12, E)?);
    let kind = bytes.u16(18, E)?;
    let import_type = kind & 0x3;
    let name_type = (kind >> 2) & 0x7;
    let body = Bytes::new(bytes.slice(20, size)?, "COFF import record");
    let symbol = body.c_string(0)?.to_owned();
    let dll = body.c_string(symbol.len() as u64 + 1)?.to_owned();
    let by_name = name_type != 0;
    let export_name = match name_type {
        0 => None,
        1 => Some(symbol.clone()),
        2 => Some(symbol.trim_start_matches(['?', '@', '_']).to_owned()),
        3 => Some(
            symbol
                .trim_start_matches(['?', '@', '_'])
                .split('@')
                .next()
                .unwrap_or("")
                .to_owned(),
        ),
        4 => {
            let at = symbol.len() as u64 + 1 + dll.len() as u64 + 1;
            Some(body.c_string(at)?.to_owned())
        }
        other => {
            return Err(ReadError::new(
                "COFF import record",
                format!("`{symbol}` carries the unknown name type {other}"),
            ));
        }
    };
    let facts = ImportFacts {
        dll,
        by_name,
        export_name,
    };
    let mut table = SymbolTable::default();
    let defined = Definition::Defined {
        binding: Binding::Global,
        ifunc: false,
    };
    table.records.push(SymbolRecord {
        name: format!("__imp_{symbol}"),
        definition: defined,
        version: None,
        origin: origin.to_owned(),
        indexed: None,
        import: Some(facts.clone()),
    });
    // IMPORT_OBJECT_CODE (0) also defines the thunk under the plain name.
    if import_type == 0 {
        table.records.push(SymbolRecord {
            name: symbol,
            definition: defined,
            version: None,
            origin: origin.to_owned(),
            indexed: None,
            import: Some(facts),
        });
    }
    Ok(table)
}

/// One PE export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeExport {
    pub name: String,
    pub ordinal: u32,
    /// `Some("OTHER.Name")` for a forwarder.
    pub forwarder: Option<String>,
}

/// The export directory of a PE image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeExports {
    /// The DLL's own name, from the export directory.
    pub dll: String,
    /// Every export that has a name.
    pub named: Vec<PeExport>,
}

impl PeExports {
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&PeExport> {
        self.named.iter().find(|export| export.name == name)
    }
}

/// Read a PE image's export directory.
///
/// # Errors
///
/// A malformed image, or one with no export directory.
pub fn pe_exports(data: &[u8]) -> Result<PeExports, ReadError> {
    let bytes = Bytes::new(data, "PE image");
    if bytes.slice(0, 2)? != b"MZ" {
        return Err(ReadError::new("PE image", "the `MZ` magic is absent"));
    }
    let pe = u64::from(bytes.u32(0x3c, E)?);
    if bytes.slice(pe, 4)? != b"PE\0\0" {
        return Err(ReadError::new("PE image", "the `PE` signature is absent"));
    }
    let coff = pe + 4;
    let sections = u64::from(bytes.u16(coff + 2, E)?);
    let optional_size = u64::from(bytes.u16(coff + 16, E)?);
    let optional = coff + 20;
    let magic = bytes.u16(optional, E)?;
    let directories = match magic {
        0x10b => optional + 96,
        0x20b => optional + 112,
        other => {
            return Err(ReadError::new(
                "PE optional header",
                format!("magic {other:#x} is neither PE32 nor PE32+"),
            ));
        }
    };
    let count_at = directories - 4;
    if bytes.u32(count_at, E)? == 0 {
        return Err(ReadError::new(
            "PE image",
            "the image has no data directories",
        ));
    }
    let export_rva = u64::from(bytes.u32(directories, E)?);
    let export_size = u64::from(bytes.u32(directories + 4, E)?);
    if export_rva == 0 {
        return Err(ReadError::new(
            "PE image",
            "the image has no export directory",
        ));
    }
    let section_table = optional + optional_size;
    let mut map = Vec::new();
    for index in 0..sections {
        let at = section_table + index * 40;
        let virtual_size = u64::from(bytes.u32(at + 8, E)?);
        let virtual_address = u64::from(bytes.u32(at + 12, E)?);
        let raw_size = u64::from(bytes.u32(at + 16, E)?);
        let raw_pointer = u64::from(bytes.u32(at + 20, E)?);
        map.push((virtual_address, virtual_size.max(raw_size), raw_pointer));
    }
    let offset = |rva: u64| -> Result<u64, ReadError> {
        map.iter()
            .find(|(va, size, _)| rva >= *va && rva < va + size)
            .map(|(va, _, raw)| rva - va + raw)
            .ok_or_else(|| ReadError::new("PE image", format!("RVA {rva:#x} lies in no section")))
    };
    let directory = offset(export_rva)?;
    let name_rva = u64::from(bytes.u32(directory + 12, E)?);
    let base = bytes.u32(directory + 16, E)?;
    let functions = u64::from(bytes.u32(directory + 20, E)?);
    let names = u64::from(bytes.u32(directory + 24, E)?);
    let functions_at = offset(u64::from(bytes.u32(directory + 28, E)?))?;
    let names_at = offset(u64::from(bytes.u32(directory + 32, E)?))?;
    let ordinals_at = offset(u64::from(bytes.u32(directory + 36, E)?))?;
    checked_extent("PE export directory", functions, 4)?;
    let dll = bytes.c_string(offset(name_rva)?)?.to_owned();
    let mut named = Vec::with_capacity(names as usize);
    for index in 0..names {
        let name = bytes.c_string(offset(u64::from(bytes.u32(names_at + index * 4, E)?))?)?;
        let slot = u64::from(bytes.u16(ordinals_at + index * 2, E)?);
        if slot >= functions {
            return Err(ReadError::new(
                "PE export directory",
                format!("`{name}` names function slot {slot} of {functions}"),
            ));
        }
        let rva = u64::from(bytes.u32(functions_at + slot * 4, E)?);
        let forwarder = if rva >= export_rva && rva < export_rva + export_size {
            Some(bytes.c_string(offset(rva)?)?.to_owned())
        } else {
            None
        };
        named.push(PeExport {
            name: name.to_owned(),
            ordinal: base + slot as u32,
            forwarder,
        });
    }
    Ok(PeExports { dll, named })
}
