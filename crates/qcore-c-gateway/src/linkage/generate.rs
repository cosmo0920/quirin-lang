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

//! The `.qrlib` generator: a `native_library manifest v2` derived from the
//! artifact's bytes and the build's declared inputs (card `W48-QRLIB-LINKAGE`,
//! user decision 1).
//!
//! **Derived from the artifact**: its sha256 and that of every file an `ld`
//! script names; its exported, defined, global symbol table (read by
//! [`super::artifact`], never by `nm`); the object format; for a dynamic link,
//! the runtime library's identity (`DT_SONAME`, a stub's install name, an
//! import's DLL). **Derived from the recorded target**: the triple and the data
//! model. **Declared by the build**: the library name and version, the linkage
//! kind, the three ABI premises, the header (whose digest is measured here),
//! the contract modules — every declaration of each becomes a `contract fn`
//! entry, its C signature projected from the contract — and, for a Windows
//! import, the OS build the runtime DLL was checked on.
//!
//! The result is a function of those inputs: [`generate`] renders it with
//! [`qcore_core::native_library::render_manifest`], and [`check_stale`]
//! refuses a stored manifest that is not byte-identical to the regenerated
//! one.

use std::collections::BTreeMap;
use std::fmt;

use qcore_core::extern_call::AbiType;
use qcore_core::ffi::CAbiType;
use qcore_core::native_library::{
    AbiPremises, AllocatorCustody, ArtifactIdentity, ContractedEntry, DataModel, EntryType,
    ExportBinding, ExportFacts, LinkageKind, LinkageSection, LinkedInput, NativeLibraryManifest,
    RuntimePremise, ThreadPosture, UnwindingPosture, load_native_library_manifest, render_manifest,
};
use qcore_core::native_library_header::HeaderIdentity;
use qcore_core::scalar_types::{PointerWidth, RecordedTarget, ScalarType};

use super::artifact::{ArtifactRefusal, FileKind, Format, Loader, TargetFacts, read_artifact};
use super::{Binding, SymbolRefusal, coff, resolve};

/// The build's declared inputs.
#[derive(Clone, Debug)]
pub struct GenerationInputs<'a> {
    pub library: &'a str,
    pub version: &'a str,
    pub linkage: LinkageKind,
    /// The artifact the link names, as the manifest records it; the loader
    /// resolves it.
    pub artifact: &'a str,
    /// The exported header: the reference recorded and its bytes.
    pub header: Option<(&'a str, &'a [u8])>,
    pub target: &'a RecordedTarget,
    /// The declared ABI premises, spelled as the manifest spells them; each
    /// admits exactly one value today and is never defaulted.
    pub unwinding: &'a str,
    pub allocator_custody: &'a str,
    pub thread_posture: &'a str,
    /// The contract selection: module key → text. Every declaration of every
    /// module becomes an entry, keyed `from` that module.
    pub contracts: &'a BTreeMap<String, String>,
    /// The OS build a Windows runtime DLL is checked on (dynamic COFF only).
    pub os_build: Option<&'a str>,
}

/// Why no manifest is generated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenerationRefusal {
    /// The recorded target's object format or data model is not one read here.
    Target { triple: String, reason: String },
    /// A declared input is not a single token the manifest can carry.
    Input { field: String, value: String },
    /// The artifact does not read.
    Artifact(ArtifactRefusal),
    /// The declared linkage kind is not what the artifact is.
    LinkageKind { declared: String, found: String },
    /// The contract selection does not load, or declares nothing.
    Contracts(String),
    /// A declaration's type has no manifest spelling.
    Unspellable { symbol: String, spelling: String },
    /// A bound symbol does not link from the artifact.
    Symbol {
        symbol: String,
        refusal: SymbolRefusal,
    },
    /// A static link would bind an import thunk, or a dynamic one code.
    WrongProvenance { symbol: String, reason: String },
    /// A struct position with no header to bind it to.
    StructWithoutHeader { symbol: String },
    /// A dynamic link's runtime identity cannot be stated.
    Runtime(String),
    /// A stored manifest differs from the regenerated one.
    Stale {
        line: usize,
        stored: String,
        generated: String,
    },
    /// The rendered text did not read back to the same manifest.
    RoundTrip(String),
}

impl fmt::Display for GenerationRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Target { triple, reason } => {
                write!(
                    f,
                    "the recorded target `{triple}` is not one linkage reads: {reason}"
                )
            }
            Self::Input { field, value } => write!(
                f,
                "the declared {field} `{value}` is empty or carries whitespace, `#` or `;`, so a manifest cannot state it"
            ),
            Self::Artifact(refusal) => write!(f, "{refusal}"),
            Self::LinkageKind { declared, found } => write!(
                f,
                "the build declares {declared} linkage, but the artifact is {found}"
            ),
            Self::Contracts(reason) => write!(f, "the contract selection: {reason}"),
            Self::Unspellable { symbol, spelling } => write!(
                f,
                "`{symbol}` has the type `{spelling}`, which no manifest entry spells"
            ),
            Self::Symbol { symbol, refusal } => {
                write!(
                    f,
                    "`{symbol}` does not link from the pinned artifact: {refusal}"
                )
            }
            Self::WrongProvenance { symbol, reason } => write!(f, "`{symbol}`: {reason}"),
            Self::StructWithoutHeader { symbol } => write!(
                f,
                "`{symbol}` takes or returns a C struct, and no header is declared; the manifest's header digest is what a `struct!` read is compared with"
            ),
            Self::Runtime(reason) => write!(f, "the runtime library: {reason}"),
            Self::Stale {
                line,
                stored,
                generated,
            } => write!(
                f,
                "the stored manifest is stale: line {line} is `{stored}` and the regenerated manifest says `{generated}`; a manifest is generated on every build and never edited"
            ),
            Self::RoundTrip(reason) => write!(
                f,
                "the rendered manifest does not read back to itself: {reason}"
            ),
        }
    }
}

impl std::error::Error for GenerationRefusal {}

/// A generated manifest and its canonical text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedManifest {
    pub manifest: NativeLibraryManifest,
    pub text: String,
}

fn token(field: &str, value: &str) -> Result<String, GenerationRefusal> {
    if value.is_empty()
        || value
            .chars()
            .any(|ch| ch.is_whitespace() || ch == '#' || ch == ';')
    {
        return Err(GenerationRefusal::Input {
            field: field.to_owned(),
            value: value.to_owned(),
        });
    }
    Ok(value.to_owned())
}

fn abi_of_scalar(scalar: ScalarType) -> AbiType {
    [
        AbiType::Bool,
        AbiType::I8,
        AbiType::I16,
        AbiType::I32,
        AbiType::I64,
        AbiType::Isize,
        AbiType::U8,
        AbiType::U16,
        AbiType::U32,
        AbiType::U64,
        AbiType::Usize,
        AbiType::F32,
        AbiType::F64,
    ]
    .into_iter()
    .find(|abi| abi.scalar() == Some(scalar))
    .expect("every scalar type has an ABI projection")
}

fn entry_type(ty: &CAbiType) -> EntryType {
    match ty {
        CAbiType::Unit => EntryType::Abi(AbiType::Unit),
        CAbiType::Scalar(scalar) => EntryType::Abi(abi_of_scalar(*scalar)),
        CAbiType::Struct(tag) => EntryType::Struct(tag.clone()),
        CAbiType::ConstStructPointer(tag) => EntryType::StructPointer {
            tag: tag.clone(),
            constant: true,
        },
    }
}

/// One declaration of the contract selection, projected to C.
struct Declared {
    symbol: String,
    module: String,
    parameters: Vec<EntryType>,
    result: EntryType,
}

fn declarations(contracts: &BTreeMap<String, String>) -> Result<Vec<Declared>, GenerationRefusal> {
    use qcore_contract::contract_loader::{ffi_declarations, load_resource_contract_sources};
    use qcore_contract::extern_bridge::declared_signatures;

    if contracts.is_empty() {
        return Err(GenerationRefusal::Contracts(
            "no contract module is declared, so the manifest would bind nothing".into(),
        ));
    }
    let loaded = load_resource_contract_sources(contracts).map_err(GenerationRefusal::Contracts)?;
    let marked = ffi_declarations(contracts).map_err(GenerationRefusal::Contracts)?;
    let signatures = declared_signatures(&loaded).map_err(GenerationRefusal::Contracts)?;
    let mut out = Vec::new();
    for (name, signature) in &signatures {
        if let Some(ffi) = marked.get(name) {
            out.push(Declared {
                symbol: name.clone(),
                module: ffi.module.clone(),
                parameters: ffi
                    .parameters
                    .iter()
                    .map(|(_, ty)| entry_type(ty))
                    .collect(),
                result: entry_type(&ffi.result),
            });
            continue;
        }
        let parse = |spelling: &str| {
            AbiType::parse(spelling).map(EntryType::Abi).ok_or_else(|| {
                GenerationRefusal::Unspellable {
                    symbol: name.clone(),
                    spelling: spelling.to_owned(),
                }
            })
        };
        let parameters = signature
            .parameter_types
            .iter()
            .map(|ty| parse(ty))
            .collect::<Result<Vec<_>, _>>()?;
        let result = if signature.result_type.is_empty() {
            EntryType::Abi(AbiType::Unit)
        } else {
            parse(&signature.result_type)?
        };
        out.push(Declared {
            symbol: name.clone(),
            module: signature.module.clone(),
            parameters,
            result,
        });
    }
    for module in contracts.keys() {
        if !out.iter().any(|declared| &declared.module == module) {
            return Err(GenerationRefusal::Contracts(format!(
                "`{module}` declares nothing, so it binds no symbol"
            )));
        }
    }
    Ok(out)
}

fn data_model(
    target: &RecordedTarget,
    facts: &TargetFacts,
) -> Result<DataModel, GenerationRefusal> {
    match (target.pointer_width, facts.format) {
        (PointerWidth::W64, Format::Coff) => Ok(DataModel::Llp64),
        (PointerWidth::W64, Format::Elf | Format::MachO) => Ok(DataModel::Lp64),
        _ => Err(GenerationRefusal::Target {
            triple: target.triple.clone(),
            reason: "a 32-bit data model is not stated by a v2 manifest".into(),
        }),
    }
}

/// Generate a manifest.
///
/// `runtime` loads a Windows runtime DLL by the name an import names (it is
/// read to check the export exists by name and is not a forwarder; it is not
/// pinned, because the OS owns it).
///
/// # Errors
///
/// A [`GenerationRefusal`] naming the first input or symbol that does not
/// admit a manifest.
pub fn generate(
    inputs: &GenerationInputs<'_>,
    loader: &Loader<'_>,
    runtime: Option<&Loader<'_>>,
) -> Result<GeneratedManifest, GenerationRefusal> {
    let library = token("library", inputs.library)?;
    let version = token("version", inputs.version)?;
    let artifact_ref = token("artifact", inputs.artifact)?;
    let facts = TargetFacts::from_triple(&inputs.target.triple).ok_or_else(|| {
        GenerationRefusal::Target {
            triple: inputs.target.triple.clone(),
            reason: "neither an ELF, Mach-O nor COFF target of x86_64 or aarch64".into(),
        }
    })?;
    let data_model = data_model(inputs.target, &facts)?;
    let closed = |field: &str, value: &str, admitted: &str| {
        if value == admitted {
            Ok(())
        } else {
            Err(GenerationRefusal::Input {
                field: field.to_owned(),
                value: value.to_owned(),
            })
        }
    };
    closed("unwinding", inputs.unwinding, "abort")?;
    closed("allocator_custody", inputs.allocator_custody, "closed")?;
    closed("thread_posture", inputs.thread_posture, "single_caller")?;
    let artifact =
        read_artifact(&artifact_ref, loader, &facts).map_err(GenerationRefusal::Artifact)?;
    let declared_kind = inputs.linkage;
    match (declared_kind, artifact.dynamic) {
        (LinkageKind::Static, true) | (LinkageKind::Dynamic, false) => {
            return Err(GenerationRefusal::LinkageKind {
                declared: declared_kind.spelling().to_owned(),
                found: if artifact.dynamic {
                    "a link-time stub for a dynamic library".to_owned()
                } else {
                    "a static archive".to_owned()
                },
            });
        }
        _ => {}
    }
    for file in &artifact.files {
        token("artifact file", &file.reference)?;
    }
    let declared = declarations(inputs.contracts)?;
    let mut contracted = BTreeMap::new();
    let mut dlls: Vec<String> = Vec::new();
    let runtime_exports = |dll: &str| -> Result<coff::PeExports, GenerationRefusal> {
        let loader = runtime.ok_or_else(|| {
            GenerationRefusal::Runtime(format!(
                "`{dll}` must be read to check its exports, and no runtime image was supplied"
            ))
        })?;
        let bytes = loader(dll).map_err(|reason| {
            GenerationRefusal::Runtime(format!("`{dll}` cannot be read: {reason}"))
        })?;
        coff::pe_exports(&bytes)
            .map_err(|error| GenerationRefusal::Runtime(format!("`{dll}`: {error}")))
    };
    for item in declared {
        let object = facts.object_name(&item.symbol);
        let resolved =
            resolve(&artifact.table, &object).map_err(|refusal| GenerationRefusal::Symbol {
                symbol: item.symbol.clone(),
                refusal,
            })?;
        match (declared_kind, &resolved.import) {
            (LinkageKind::Static, Some(import)) => {
                return Err(GenerationRefusal::WrongProvenance {
                    symbol: item.symbol,
                    reason: format!(
                        "a static link resolved it to an import from `{}`; the code is not in the archive",
                        import.dll
                    ),
                });
            }
            (LinkageKind::Dynamic, None) if facts.format == Format::Coff => {
                return Err(GenerationRefusal::WrongProvenance {
                    symbol: item.symbol,
                    reason:
                        "a dynamic COFF link resolved it to code in the library, not to an import"
                            .into(),
                });
            }
            (LinkageKind::Dynamic, Some(import)) => {
                let exports = runtime_exports(&import.dll)?;
                let export_name = import.export_name.clone().unwrap_or_default();
                match exports.find(&export_name) {
                    None => {
                        return Err(GenerationRefusal::Symbol {
                            symbol: item.symbol,
                            refusal: SymbolRefusal::NotExportedByRuntime {
                                symbol: export_name,
                                dll: import.dll.clone(),
                            },
                        });
                    }
                    Some(export) => {
                        if let Some(target) = &export.forwarder {
                            return Err(GenerationRefusal::Symbol {
                                symbol: item.symbol,
                                refusal: SymbolRefusal::Forwarder {
                                    symbol: export_name,
                                    dll: import.dll.clone(),
                                    target: target.clone(),
                                },
                            });
                        }
                    }
                }
                if !dlls.contains(&import.dll) {
                    dlls.push(import.dll.clone());
                }
            }
            _ => {}
        }
        let file = token("defining file", &resolved.origin)?;
        let carries_struct = item
            .parameters
            .iter()
            .chain([&item.result])
            .any(|ty| ty.abi().is_none());
        if carries_struct && inputs.header.is_none() {
            return Err(GenerationRefusal::StructWithoutHeader {
                symbol: item.symbol,
            });
        }
        let export = ExportFacts {
            symbol: object,
            version: resolved.version.clone(),
            binding: match resolved.binding {
                Binding::Global => ExportBinding::Global,
                Binding::Weak => ExportBinding::Weak,
            },
            ifunc: resolved.ifunc,
            dll: resolved.import.as_ref().map(|import| import.dll.clone()),
            file,
        };
        contracted.insert(
            item.symbol.clone(),
            ContractedEntry {
                symbol: item.symbol,
                parameters: item.parameters,
                result: item.result,
                contract_module: token("contract module", &item.module)?,
                export: Some(export),
            },
        );
    }
    let mut runtime_premises = Vec::new();
    if declared_kind == LinkageKind::Dynamic {
        match facts.format {
            Format::Elf => {
                for file in artifact
                    .files
                    .iter()
                    .filter(|file| file.kind == FileKind::SharedObject)
                {
                    let soname = file.soname.clone().ok_or_else(|| {
                        GenerationRefusal::Runtime(format!(
                            "`{}` carries no `DT_SONAME`, so the runtime library it stands for cannot be named",
                            file.reference
                        ))
                    })?;
                    runtime_premises.push(RuntimePremise::Soname {
                        soname: token("soname", &soname)?,
                        file: file.reference.clone(),
                    });
                }
            }
            Format::MachO => {
                let (name, current_version) = artifact.install_name.clone().ok_or_else(|| {
                    GenerationRefusal::Runtime("the stub states no install name".into())
                })?;
                runtime_premises.push(RuntimePremise::InstallName {
                    name: token("install name", &name)?,
                    current_version: current_version
                        .map(|version| token("current version", &version))
                        .transpose()?,
                });
            }
            Format::Coff => {
                let build = inputs.os_build.ok_or_else(|| {
                    GenerationRefusal::Runtime(
                        "a Windows runtime DLL is named with the OS build it was checked on, and none was declared".into(),
                    )
                })?;
                for dll in dlls {
                    runtime_premises.push(RuntimePremise::Dll {
                        name: token("DLL", &dll)?,
                        os_build: token("OS build", build)?,
                    });
                }
            }
        }
        if runtime_premises.is_empty() {
            return Err(GenerationRefusal::Runtime(
                "no runtime library could be named".into(),
            ));
        }
    }
    let header = match inputs.header {
        None => None,
        Some((reference, bytes)) => Some(HeaderIdentity {
            reference: token("header", reference)?,
            digest: format!("sha256:{}", qcore_schemas::digest::sha256_hex(bytes)),
            includes: Vec::new(),
        }),
    };
    let manifest = NativeLibraryManifest {
        identity: ArtifactIdentity {
            library,
            version,
            artifact: artifact_ref,
            hash: artifact.files[0].digest.clone(),
        },
        premises: AbiPremises {
            data_model,
            unwinding: UnwindingPosture::Abort,
            allocator_custody: AllocatorCustody::Closed,
            thread_posture: ThreadPosture::SingleCaller,
        },
        header,
        unchecked: BTreeMap::new(),
        contracted,
        linkage: Some(LinkageSection {
            kind: declared_kind,
            format: facts.format.spelling().to_owned(),
            target: token("target", &inputs.target.triple)?,
            inputs: artifact.files[1..]
                .iter()
                .map(|file| LinkedInput {
                    reference: file.reference.clone(),
                    digest: file.digest.clone(),
                    as_needed: file.as_needed,
                })
                .collect(),
            runtime: runtime_premises,
        }),
    };
    let text = render_manifest(&manifest).expect("a generated manifest has a linkage section");
    match load_native_library_manifest(&text) {
        Ok(read) if read == manifest => {}
        Ok(_) => {
            return Err(GenerationRefusal::RoundTrip(
                "the text reads back to a different manifest".into(),
            ));
        }
        Err(refusal) => return Err(GenerationRefusal::RoundTrip(refusal.to_string())),
    }
    Ok(GeneratedManifest { manifest, text })
}

#[cfg(test)]
#[path = "../../tests/unit/linkage_generate.rs"]
mod tests;

/// Regenerate and compare with a stored manifest, byte for byte.
///
/// # Errors
///
/// Every [`generate`] refusal, or [`GenerationRefusal::Stale`] naming the first
/// line that differs.
pub fn check_stale(
    stored: &str,
    inputs: &GenerationInputs<'_>,
    loader: &Loader<'_>,
    runtime: Option<&Loader<'_>>,
) -> Result<GeneratedManifest, GenerationRefusal> {
    let generated = generate(inputs, loader, runtime)?;
    if generated.text == stored {
        return Ok(generated);
    }
    let stored_lines: Vec<&str> = stored.lines().collect();
    let generated_lines: Vec<&str> = generated.text.lines().collect();
    let index = stored_lines
        .iter()
        .zip(&generated_lines)
        .position(|(a, b)| a != b)
        .unwrap_or(stored_lines.len().min(generated_lines.len()));
    Err(GenerationRefusal::Stale {
        line: index + 1,
        stored: stored_lines
            .get(index)
            .copied()
            .unwrap_or("<end of file>")
            .to_owned(),
        generated: generated_lines
            .get(index)
            .copied()
            .unwrap_or("<end of file>")
            .to_owned(),
    })
}
