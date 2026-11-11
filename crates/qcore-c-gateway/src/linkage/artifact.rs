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

//! Recognise an artifact and read its whole exported symbol table.
//!
//! The file the link names may be an archive, a shared object, a text stub, a
//! dylib, an import library, or a GNU `ld` script naming several of these. The
//! reader follows a script to every file it names, hashes each, and folds all
//! their symbols into one table, because the linker sees their union.
//!
//! Files are read through a loader closure, so the whole recognition is a
//! function of bytes and testable anywhere.

use std::fmt;

use super::bytes::ReadError;
use super::{Definition, SymbolTable, archive, coff, elf, ld_script, macho, tbd};

/// The object-file family of a target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Format {
    Elf,
    MachO,
    Coff,
}

impl Format {
    #[must_use]
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Elf => "elf",
            Self::MachO => "macho",
            Self::Coff => "coff",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "elf" => Some(Self::Elf),
            "macho" => Some(Self::MachO),
            "coff" => Some(Self::Coff),
            _ => None,
        }
    }
}

/// What one file is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    /// A static archive of objects.
    StaticArchive,
    /// An ELF shared object.
    SharedObject,
    /// A GNU `ld` script.
    LdScript,
    /// An Apple text stub.
    TextStub,
    /// A Mach-O dylib (export trie).
    Dylib,
    /// A COFF import library (short import records).
    ImportLibrary,
}

impl FileKind {
    /// Whether the file is a link-time stub for a library the OS supplies at
    /// run time, rather than the code itself.
    #[must_use]
    pub fn is_dynamic(self) -> bool {
        matches!(
            self,
            Self::SharedObject | Self::TextStub | Self::Dylib | Self::ImportLibrary
        )
    }
}

/// The recorded target, as linkage reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetFacts {
    pub triple: String,
    pub format: Format,
    /// `x86_64`, `aarch64`, …: the triple's first component, normalised.
    pub arch: String,
}

impl TargetFacts {
    /// Read a recorded triple. `None` for a triple whose object format this
    /// reader does not know.
    #[must_use]
    pub fn from_triple(triple: &str) -> Option<Self> {
        let arch = match triple.split('-').next()? {
            "arm64" | "aarch64" | "arm64e" => "aarch64",
            "x86_64" | "amd64" => "x86_64",
            _ => return None,
        };
        let format = if triple.contains("-apple-") {
            Format::MachO
        } else if triple.contains("-windows") {
            Format::Coff
        } else if triple.contains("-linux") || triple.contains("-freebsd") {
            Format::Elf
        } else {
            return None;
        };
        Some(Self {
            triple: triple.to_owned(),
            format,
            arch: arch.to_owned(),
        })
    }

    /// The decoration a C symbol carries at object level on this target:
    /// Mach-O prefixes `_`; ELF and 64-bit COFF do not.
    #[must_use]
    pub fn object_name(&self, symbol: &str) -> String {
        match self.format {
            Format::MachO => format!("_{symbol}"),
            Format::Elf | Format::Coff => symbol.to_owned(),
        }
    }

    fn elf_machine(&self) -> u16 {
        match self.arch.as_str() {
            "x86_64" => 62,
            _ => 183,
        }
    }

    fn coff_machine(&self) -> u16 {
        match self.arch.as_str() {
            "x86_64" => 0x8664,
            _ => 0xaa64,
        }
    }
}

/// One file pinned by a manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinnedFile {
    /// The reference as spelled (the manifest's `artifact`, or the path an
    /// `ld` script spells).
    pub reference: String,
    /// `sha256:` + 64 lowercase hex digits over the file's bytes.
    pub digest: String,
    pub kind: FileKind,
    /// Inside a script's `AS_NEEDED ( … )`.
    pub as_needed: bool,
    /// An ELF shared object's `DT_SONAME`.
    pub soname: Option<String>,
}

/// A read artifact: every pinned file (the named one first) and the union of
/// their symbols.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadArtifact {
    pub format: Format,
    /// Whether the link binds to a stub (dynamic) or the code (static).
    pub dynamic: bool,
    pub files: Vec<PinnedFile>,
    pub table: SymbolTable,
    /// A text stub's or dylib's install name and current version.
    pub install_name: Option<(String, Option<String>)>,
}

/// Why an artifact is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactRefusal {
    /// A file could not be opened.
    Unreadable { reference: String, reason: String },
    /// A file is not a format read here.
    Unrecognised { reference: String },
    /// A file's bytes are malformed.
    Malformed { reference: String, error: ReadError },
    /// A file is for another object format or machine than the recorded target.
    WrongTarget {
        reference: String,
        found: String,
        target: String,
    },
    /// A script mixes static and dynamic files, or names a script.
    MixedScript { reference: String, reason: String },
}

impl fmt::Display for ArtifactRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { reference, reason } => {
                write!(f, "`{reference}` cannot be read: {reason}")
            }
            Self::Unrecognised { reference } => write!(
                f,
                "`{reference}` is not an archive, ELF shared object, `ld` script, text stub, dylib or import library"
            ),
            Self::Malformed { reference, error } => write!(f, "`{reference}`: {error}"),
            Self::WrongTarget {
                reference,
                found,
                target,
            } => write!(
                f,
                "`{reference}` holds {found}, not code for the recorded target `{target}`"
            ),
            Self::MixedScript { reference, reason } => write!(f, "`{reference}`: {reason}"),
        }
    }
}

impl std::error::Error for ArtifactRefusal {}

/// A loader: the bytes of a reference, or why they cannot be had.
pub type Loader<'a> = dyn Fn(&str) -> Result<Vec<u8>, String> + 'a;

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", qcore_schemas::digest::sha256_hex(bytes))
}

/// Read the artifact the link names, following an `ld` script.
///
/// # Errors
///
/// An [`ArtifactRefusal`].
pub fn read_artifact(
    reference: &str,
    loader: &Loader<'_>,
    target: &TargetFacts,
) -> Result<ReadArtifact, ArtifactRefusal> {
    let data = loader(reference).map_err(|reason| ArtifactRefusal::Unreadable {
        reference: reference.to_owned(),
        reason,
    })?;
    if ld_script::is_ld_script(&data) {
        let inputs = ld_script::read(&data).map_err(|error| ArtifactRefusal::Malformed {
            reference: reference.to_owned(),
            error,
        })?;
        let mut files = vec![PinnedFile {
            reference: reference.to_owned(),
            digest: digest(&data),
            kind: FileKind::LdScript,
            as_needed: false,
            soname: None,
        }];
        let mut table = SymbolTable::default();
        let mut dynamic = None;
        for input in inputs {
            let bytes = loader(&input.path).map_err(|reason| ArtifactRefusal::Unreadable {
                reference: input.path.clone(),
                reason,
            })?;
            if ld_script::is_ld_script(&bytes) {
                return Err(ArtifactRefusal::MixedScript {
                    reference: reference.to_owned(),
                    reason: format!(
                        "it names another script, `{}`; a library script is followed one level",
                        input.path
                    ),
                });
            }
            let file = read_file(&input.path, &bytes, target)?;
            let is_dynamic = file.kind.is_dynamic();
            if *dynamic.get_or_insert(is_dynamic) != is_dynamic {
                return Err(ArtifactRefusal::MixedScript {
                    reference: reference.to_owned(),
                    reason: "it names both static archives and shared objects; one linkage kind per manifest".into(),
                });
            }
            files.push(PinnedFile {
                reference: input.path.clone(),
                digest: digest(&bytes),
                kind: file.kind,
                as_needed: input.as_needed,
                soname: file.soname,
            });
            table.extend(file.table);
        }
        return Ok(ReadArtifact {
            format: target.format,
            dynamic: dynamic.unwrap_or(false),
            files,
            table,
            install_name: None,
        });
    }
    let file = read_file(reference, &data, target)?;
    Ok(ReadArtifact {
        format: target.format,
        dynamic: file.kind.is_dynamic(),
        files: vec![PinnedFile {
            reference: reference.to_owned(),
            digest: digest(&data),
            kind: file.kind,
            as_needed: false,
            soname: file.soname,
        }],
        table: file.table,
        install_name: file.install_name,
    })
}

struct ReadFile {
    kind: FileKind,
    table: SymbolTable,
    soname: Option<String>,
    install_name: Option<(String, Option<String>)>,
}

fn malformed(reference: &str) -> impl Fn(ReadError) -> ArtifactRefusal + '_ {
    move |error| ArtifactRefusal::Malformed {
        reference: reference.to_owned(),
        error,
    }
}

fn wrong(reference: &str, found: impl Into<String>, target: &TargetFacts) -> ArtifactRefusal {
    ArtifactRefusal::WrongTarget {
        reference: reference.to_owned(),
        found: found.into(),
        target: target.triple.clone(),
    }
}

fn read_file(
    reference: &str,
    data: &[u8],
    target: &TargetFacts,
) -> Result<ReadFile, ArtifactRefusal> {
    if archive::is_archive(data) {
        return read_archive(reference, data, target);
    }
    if elf::is_elf(data) {
        if target.format != Format::Elf {
            return Err(wrong(reference, "an ELF file", target));
        }
        let (kind, machine, _) = elf::identify(data).map_err(malformed(reference))?;
        if machine != target.elf_machine() {
            return Err(wrong(reference, format!("ELF machine {machine}"), target));
        }
        if kind != elf::ElfKind::Shared {
            return Err(ArtifactRefusal::Unrecognised {
                reference: reference.to_owned(),
            });
        }
        let shared = elf::shared_object(data, reference).map_err(malformed(reference))?;
        return Ok(ReadFile {
            kind: FileKind::SharedObject,
            table: shared.table,
            soname: shared.soname,
            install_name: None,
        });
    }
    if tbd::is_tbd(data) {
        if target.format != Format::MachO {
            return Err(wrong(reference, "an Apple text stub", target));
        }
        let name = tbd::tbd_target(&target.triple)
            .ok_or_else(|| wrong(reference, "an Apple text stub", target))?;
        let stub = tbd::read(data, &name).map_err(malformed(reference))?;
        return Ok(ReadFile {
            kind: FileKind::TextStub,
            table: stub.table,
            soname: None,
            install_name: Some((stub.install_name, stub.current_version)),
        });
    }
    if macho::is_macho(data) {
        if target.format != Format::MachO {
            return Err(wrong(reference, "a Mach-O file", target));
        }
        let dylib = macho::dylib(data, reference).map_err(malformed(reference))?;
        return Ok(ReadFile {
            kind: FileKind::Dylib,
            install_name: dylib.install_name.clone().map(|name| (name, None)),
            table: dylib.table,
            soname: None,
        });
    }
    Err(ArtifactRefusal::Unrecognised {
        reference: reference.to_owned(),
    })
}

fn read_archive(
    reference: &str,
    data: &[u8],
    target: &TargetFacts,
) -> Result<ReadFile, ArtifactRefusal> {
    let read = archive::read(data).map_err(malformed(reference))?;
    let mut table = SymbolTable::default();
    let mut imports = false;
    let mut objects = false;
    for member in &read.members {
        let origin = format!("{reference}({})", member.name);
        let mut member_table = if elf::is_elf(member.data) {
            if target.format != Format::Elf {
                return Err(wrong(&origin, "an ELF object", target));
            }
            let (_, machine, _) = elf::identify(member.data).map_err(malformed(&origin))?;
            if machine != target.elf_machine() {
                return Err(wrong(&origin, format!("ELF machine {machine}"), target));
            }
            objects = true;
            elf::object_symbols(member.data, &origin).map_err(malformed(&origin))?
        } else if macho::is_macho(member.data) {
            if target.format != Format::MachO {
                return Err(wrong(&origin, "a Mach-O object", target));
            }
            objects = true;
            macho::object_symbols(member.data, &origin).map_err(malformed(&origin))?
        } else if coff::is_short_import(member.data) {
            if target.format != Format::Coff {
                return Err(wrong(&origin, "a COFF import record", target));
            }
            let machine = coff::machine(member.data).map_err(malformed(&origin))?;
            if coff::is_companion_import_machine(machine, target.coff_machine()) {
                // Windows ARM64 SDK libraries can be hybrid archives carrying
                // ARM64EC alongside native ARM64 import records. Only the
                // exact known companion machine is omitted; every other
                // mismatch remains a refusal below.
                continue;
            }
            if machine != target.coff_machine() {
                return Err(wrong(
                    &origin,
                    format!("a {} import", coff::machine_name(machine)),
                    target,
                ));
            }
            imports = true;
            coff::import_symbols(member.data, &origin).map_err(malformed(&origin))?
        } else if coff::is_coff_object(member.data) {
            if target.format != Format::Coff {
                return Err(wrong(&origin, "a COFF object", target));
            }
            let machine = coff::machine(member.data).map_err(malformed(&origin))?;
            if machine != target.coff_machine() && machine != 0 {
                return Err(wrong(
                    &origin,
                    format!("a {} object", coff::machine_name(machine)),
                    target,
                ));
            }
            objects = true;
            coff::object_symbols(member.data, &origin).map_err(malformed(&origin))?
        } else if target.format == Format::Coff
            && matches!(member.name.as_str(), "/<XFGHASHMAP>" | "/<ECSYMBOLS>")
        {
            // Windows 11 SDK and WDK archives may carry these exact,
            // undocumented linker-metadata members. They contain no object
            // symbol definitions. Keep every other unrecognised member
            // fail-closed; LLVM's archive reader recognizes the same names.
            continue;
        } else {
            return Err(ArtifactRefusal::Malformed {
                reference: origin,
                error: ReadError::new(
                    "archive member",
                    "not an ELF, Mach-O or COFF object nor a COFF import record; an archive is read only when every member is",
                ),
            });
        };
        for record in &mut member_table.records {
            if matches!(record.definition, Definition::Defined { .. }) {
                record.indexed = Some(read.indexes(&record.name, member.header_offset));
            }
        }
        table.extend(member_table);
    }
    // An import library also carries ordinary objects (the import
    // descriptors); what makes it one is its import records.
    let _ = objects;
    Ok(ReadFile {
        kind: if imports {
            FileKind::ImportLibrary
        } else {
            FileKind::StaticArchive
        },
        table,
        soname: None,
        install_name: None,
    })
}
