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

//! Linkage: the exported symbol table of a C-ABI artifact, read from its bytes
//! on every platform, and the generated `.qrlib` manifest built from it (card
//! `W48-QRLIB-LINKAGE`, card 5 of the `.qr` track).
//!
//! # What "linkage" means here
//!
//! The user's decision (2026-09-30): linkage is the **exported symbol table**,
//! the view `nm` gives of a library — read here, never by shelling out to
//! `nm`. A library published only through its C symbol table (a Rust
//! `staticlib` exporting `extern "C"` functions is the model case) links, and
//! every symbol a manifest binds must be exported and defined in the pinned
//! artifact:
//!
//! * a missing, undefined or local-only symbol refuses;
//! * two definitions refuse (ambiguous);
//! * a **defined weak** symbol is admitted, its weakness recorded;
//! * an ELF symbol version (`ilogb@@GLIBC_2.2.5`) is recorded and must be the
//!   default one — a symbol with only hidden (`@`) versions refuses;
//! * a COFF import must be **by name**, never ordinal-only, and the runtime
//!   DLL's export must exist and must not be a forwarder;
//! * in an archive, the definition must be one the archive's symbol index
//!   lists, because the index is what a linker consults.
//!
//! # The readers
//!
//! Every reader is a function of `&[u8]`, so every platform's reader is
//! exercised on every host from bytes:
//!
//! | Format | Static | Dynamic (link-time stub) |
//! |---|---|---|
//! | ELF | [`archive`] + [`elf`] relocatable objects | [`elf`] shared objects (`.dynsym`, versions, `DT_SONAME`); [`ld_script`] |
//! | Mach-O | [`archive`] (`__.SYMDEF`) + [`macho`] objects | [`tbd`] text stubs; [`macho`] dylib export trie |
//! | COFF/PE | [`archive`] (linker members) + [`coff`] objects | [`coff`] import libraries (short import records); [`coff`] PE export directory for the runtime DLL |
//!
//! [`artifact`] recognises a file and dispatches; [`generate`] builds the
//! manifest; [`link`] verifies the pinned files immediately before a link.

pub mod archive;
pub mod artifact;
pub mod bytes;
pub mod coff;
pub mod elf;
pub mod generate;
pub mod ld_script;
pub mod link;
pub mod macho;
pub mod tbd;

use std::fmt;

/// How a defined symbol binds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Binding {
    Global,
    Weak,
}

impl Binding {
    #[must_use]
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Weak => "weak",
        }
    }
}

/// What one symbol-table record says about its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Definition {
    /// Defined and visible to the linker. `ifunc` marks an ELF
    /// `STT_GNU_IFUNC`: the address is chosen by a resolver at load time.
    Defined { binding: Binding, ifunc: bool },
    /// Referenced but not defined here.
    Undefined,
    /// Defined but not visible outside its object (a local or private
    /// external symbol).
    Local,
}

/// An ELF symbol version attached to a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolVersion {
    pub name: String,
    /// `@@` (the version a link binds to) rather than a hidden `@` one.
    pub default: bool,
}

/// A COFF import record's facts (import libraries).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportFacts {
    /// The DLL the import names.
    pub dll: String,
    /// Whether the import is by name. An ordinal-only import refuses.
    pub by_name: bool,
    /// The name the DLL is asked for (the export name), for a by-name import.
    pub export_name: Option<String>,
}

/// One record of an artifact's symbol table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolRecord {
    /// The object-level name (`_sqrt` on Mach-O, `sqrt` on ELF).
    pub name: String,
    pub definition: Definition,
    pub version: Option<SymbolVersion>,
    /// Where the record was read: `libqr.a(qr.o)`, a shared object's path, a
    /// text stub document's install name.
    pub origin: String,
    /// For an archive member: whether the archive's symbol index lists this
    /// name for this member. `None` outside archives.
    pub indexed: Option<bool>,
    pub import: Option<ImportFacts>,
}

/// Every record of one artifact (or of every file an `ld` script names).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SymbolTable {
    pub records: Vec<SymbolRecord>,
}

impl SymbolTable {
    pub fn extend(&mut self, other: SymbolTable) {
        self.records.extend(other.records);
    }
}

/// A symbol the resolver admitted: the facts a manifest entry records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedSymbol {
    pub name: String,
    pub binding: Binding,
    pub ifunc: bool,
    /// The default version, when the definition is versioned.
    pub version: Option<String>,
    pub origin: String,
    pub import: Option<ImportFacts>,
}

/// Why a symbol does not link from an artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolRefusal {
    /// No record names it: absent, or stripped.
    Missing { symbol: String },
    /// Only references to it exist.
    Undefined {
        symbol: String,
        origins: Vec<String>,
    },
    /// It is defined only as a local (or private external) symbol.
    LocalOnly {
        symbol: String,
        origins: Vec<String>,
    },
    /// Two or more linkable definitions.
    Ambiguous {
        symbol: String,
        origins: Vec<String>,
    },
    /// Defined in an archive member the archive's index does not list it for,
    /// so a linker searching the index never pulls it.
    NotIndexed { symbol: String, origin: String },
    /// Every definition carries a hidden (`@`) version, which a link never
    /// binds to by name.
    HiddenVersionOnly {
        symbol: String,
        versions: Vec<String>,
    },
    /// A COFF import by ordinal only.
    OrdinalOnly { symbol: String, dll: String },
    /// The runtime DLL exports it only as a forwarder to another DLL.
    Forwarder {
        symbol: String,
        dll: String,
        target: String,
    },
    /// The runtime DLL does not export it by name.
    NotExportedByRuntime { symbol: String, dll: String },
}

impl fmt::Display for SymbolRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let list = |items: &[String]| {
            items
                .iter()
                .map(|item| format!("`{item}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        match self {
            Self::Missing { symbol } => write!(
                f,
                "`{symbol}` is not in the artifact's symbol table (absent or stripped)"
            ),
            Self::Undefined { symbol, origins } => write!(
                f,
                "`{symbol}` is only referenced, never defined, in {}",
                list(origins)
            ),
            Self::LocalOnly { symbol, origins } => write!(
                f,
                "`{symbol}` is defined only as a local symbol in {}, which no other object can link",
                list(origins)
            ),
            Self::Ambiguous { symbol, origins } => write!(
                f,
                "`{symbol}` has more than one linkable definition ({}); which one a link takes is not a function of the manifest",
                list(origins)
            ),
            Self::NotIndexed { symbol, origin } => write!(
                f,
                "`{symbol}` is defined in {origin}, but the archive's symbol index does not list it there, so a linker searching the index never pulls it"
            ),
            Self::HiddenVersionOnly { symbol, versions } => write!(
                f,
                "`{symbol}` is defined only under hidden versions ({}); a link binds a name only to its default `@@` version",
                list(versions)
            ),
            Self::OrdinalOnly { symbol, dll } => write!(
                f,
                "`{symbol}` is imported from `{dll}` by ordinal only; an import must be by name"
            ),
            Self::Forwarder {
                symbol,
                dll,
                target,
            } => write!(
                f,
                "`{dll}` exports `{symbol}` as a forwarder to `{target}`; the code is not in the DLL the manifest names"
            ),
            Self::NotExportedByRuntime { symbol, dll } => {
                write!(f, "`{dll}` does not export `{symbol}` by name")
            }
        }
    }
}

impl std::error::Error for SymbolRefusal {}

/// Resolve `symbol` in `table`: the one linkable definition, or the refusal.
///
/// # Errors
///
/// A [`SymbolRefusal`] naming why the symbol does not link.
pub fn resolve(table: &SymbolTable, symbol: &str) -> Result<ResolvedSymbol, SymbolRefusal> {
    let records: Vec<&SymbolRecord> = table
        .records
        .iter()
        .filter(|record| record.name == symbol)
        .collect();
    let origins = |filter: &dyn Fn(&SymbolRecord) -> bool| -> Vec<String> {
        let mut origins: Vec<String> = records
            .iter()
            .filter(|record| filter(record))
            .map(|record| record.origin.clone())
            .collect();
        origins.sort();
        origins.dedup();
        origins
    };
    let defined: Vec<&SymbolRecord> = records
        .iter()
        .copied()
        .filter(|record| matches!(record.definition, Definition::Defined { .. }))
        .collect();
    if defined.is_empty() {
        let locals = origins(&|record| record.definition == Definition::Local);
        if !locals.is_empty() {
            return Err(SymbolRefusal::LocalOnly {
                symbol: symbol.to_owned(),
                origins: locals,
            });
        }
        let undefined = origins(&|record| record.definition == Definition::Undefined);
        if !undefined.is_empty() {
            return Err(SymbolRefusal::Undefined {
                symbol: symbol.to_owned(),
                origins: undefined,
            });
        }
        return Err(SymbolRefusal::Missing {
            symbol: symbol.to_owned(),
        });
    }
    // A versioned shared object may define one name under several versions;
    // a link by name binds only the default one.
    let linkable: Vec<&SymbolRecord> = defined
        .iter()
        .copied()
        .filter(|record| {
            record
                .version
                .as_ref()
                .is_none_or(|version| version.default)
        })
        .collect();
    if linkable.is_empty() {
        let mut versions: Vec<String> = defined
            .iter()
            .filter_map(|record| record.version.as_ref().map(|v| v.name.clone()))
            .collect();
        versions.sort();
        versions.dedup();
        return Err(SymbolRefusal::HiddenVersionOnly {
            symbol: symbol.to_owned(),
            versions,
        });
    }
    if linkable.len() > 1 {
        let mut origins: Vec<String> = linkable
            .iter()
            .map(|record| match &record.version {
                Some(version) => format!("{}@@{}", record.origin, version.name),
                None => record.origin.clone(),
            })
            .collect();
        origins.sort();
        return Err(SymbolRefusal::Ambiguous {
            symbol: symbol.to_owned(),
            origins,
        });
    }
    let record = linkable[0];
    if record.indexed == Some(false) {
        return Err(SymbolRefusal::NotIndexed {
            symbol: symbol.to_owned(),
            origin: record.origin.clone(),
        });
    }
    if let Some(import) = &record.import
        && !import.by_name
    {
        return Err(SymbolRefusal::OrdinalOnly {
            symbol: symbol.to_owned(),
            dll: import.dll.clone(),
        });
    }
    let Definition::Defined { binding, ifunc } = record.definition else {
        unreachable!("filtered to definitions above");
    };
    Ok(ResolvedSymbol {
        name: symbol.to_owned(),
        binding,
        ifunc,
        version: record.version.as_ref().map(|version| version.name.clone()),
        origin: record.origin.clone(),
        import: record.import.clone(),
    })
}

#[cfg(test)]
#[path = "../../tests/unit/linkage.rs"]
mod tests;
