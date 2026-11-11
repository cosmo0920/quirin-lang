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

//! The `source.native-library.v1` manifest: a declared binding from a **library
//! artifact identity** to the symbols that artifact exports, so that an extern
//! declaration's premise can name *which build* it rests on.
//!
//! Design: `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` §4 (the manifest) and
//! `docs/design/UNSAFE_LINKAGE_V1.md` §4.2 (the linkage spectrum). Card
//! `UNSAFE-LINKAGE-ARTIFACT` built the contract-less end (level **L1**), where
//! the premise names a build and nothing else; card
//! `NATIVE-LIBRARY-CONTRACT-SLICE` built the contracted end (level **L3**),
//! where a symbol resolves through the manifest to a declared `.qcore` contract
//! and inherits the identity beside it. Card `NATIVE-LIBRARY-HEADER` then made
//! the entries' transcribed header declarations **checkable** against the
//! library's real exported `.h` — see [`crate::native_library_header`], which
//! owns the two optional fields [`NativeLibraryManifest::header`] carries. The
//! link step itself stays the builder's act.
//!
//! # What a manifest is
//!
//! One declarative file per native library. Its header states the artifact
//! identity (§4.3) and the four ABI premise fields (§4.4); its body lists one
//! entry per symbol, in one of **two kinds**:
//!
//! * an **explicitly marked contract-less** entry (`unchecked fn`), level L1 of
//!   the linkage spectrum, which admits no pointer in any position;
//! * a **contract slice** entry (`contract fn … from MODULE`), level L3 (card
//!   `NATIVE-LIBRARY-CONTRACT-SLICE`), which binds the symbol to a declared
//!   `.qcore` contract and is therefore where a library's **pointer** symbols
//!   belong.
//!
//! The two are told apart by their *keyword* and never by their shape: a
//! `contract fn` with no `from MODULE` tail, and an `unchecked fn` carrying one,
//! are each a named rejection rather than a lenient reading of the other. That
//! is what keeps design §4.2's requirement true — a *dropped* contract can never
//! be read as a waiver.
//!
//! ```text
//! native_library manifest v1;
//! library qcore_unchecked_demo;
//! version 0.1.0;
//! artifact libqcore_unchecked_demo.a;
//! hash sha256:0000000000000000000000000000000000000000000000000000000000000000;
//! data_model lp64;
//! unwinding abort;
//! allocator_custody closed;
//! thread_posture single_caller;
//!
//! unchecked fn k_unchecked_add(int32_t, int32_t) -> int32_t;
//! contract fn k_alloc(uintptr_t) -> uint8_t * from contracts/lib.qcore;
//! ```
//!
//! # What a contract slice entry does *not* do
//!
//! It creates no contract semantics. The manifest **selects and attributes**:
//! the contract is loaded, admitted and bridged by the unchanged
//! resource-contract v1 machinery, and the entry says only which declared
//! contract the artifact's symbol is read through and what the artifact's own
//! exported header declares for it. The binding is checked total in **both**
//! directions ([`check_slice_totality`]) against the selected environment, and
//! §4.4's closed-allocator-custody check ([`check_closed_allocator_custody`]) —
//! vacuous while entries were contract-less, because a waiver cannot acquire —
//! becomes real once one can.
//!
//! # Total or reject
//!
//! [`load_native_library_manifest`] follows the resource-contract v1 loader's
//! discipline exactly: an unknown field, an unknown field *value*, a repeated
//! or missing field, a malformed statement, a malformed entry of either kind, a
//! symbol bound twice across both kinds, a malformed contract module reference
//! or a type spelling outside the §C04.1 ABI projection **rejects the whole
//! manifest**. There is no partial admission and
//! no skipping, because a manifest that silently dropped the line a reader was
//! relying on would be a weaker table presented as a complete one.
//!
//! # What the artifact identity is, and what it is not
//!
//! The identity is a **pin, not a probe**. This module reads no file, hashes
//! nothing and touches no filesystem: it checks that the stated identity is
//! well formed and records it, and the recorded identity travels into the
//! certificate as a premise in the sense of hard invariant 2 — a *declared
//! assumption*, never a proof.
//!
//! Two reasons, both load-bearing:
//!
//! 1. **A hash of a file on this disk is not evidence about the link.** Whether
//!    the linker used the named artifact is the builder's act (design §10
//!    excludes verifying the link step). Hashing a candidate file at check time
//!    would produce a stronger-*looking* statement that is not a stronger one,
//!    and this repository does not manufacture those.
//! 2. **A premise must be rederivable from the program alone.** The driver
//!    publishes no certificate it did not independently re-derive
//!    (`recheck_extern_certificate`, `Q027`). A premise whose content depended
//!    on the contents of the checking host's filesystem could not be reproduced
//!    by a recheck on another host, so the pin is exactly what a certificate can
//!    honestly carry.
//!
//! What the identity therefore *means* on a certificate: **the program was
//! checked on the declaration that this symbol comes from this build.** A
//! substitution at link time is a premise violation — detectable, because the
//! certificate names what was obligated — rather than an invisible one. Card
//! follow-on `UNSAFE-LINKAGE-ARTIFACT-PROBE` is where a builder-side hash
//! *observation* would live; it is an observation and could never be more.
//!
//! # The generated manifest (v2)
//!
//! Card `W48-QRLIB-LINKAGE` (user decisions, 2026-09-30) makes `.qrlib` **the**
//! C linkage manifest of the FFI route, and a `native_library manifest v2`
//! file is **generated on every build, before linking**, never written by
//! hand. What changes is where the identity comes from, not what this module
//! does: the generator (`qcore_c_gateway::linkage::generate`) opens the
//! artifact, hashes it and reads its exported symbol table, and renders the
//! manifest with [`render_manifest`]; a stored manifest the regenerated one
//! does not equal byte for byte is refused as stale; the build hashes every
//! pinned file again immediately before handing exactly those paths to the
//! linker. This module still reads no file: it loads the text, total or
//! reject, and states the facts the generator derived — the linkage kind, the
//! object format and target, every file an `ld` script names, the runtime
//! library's identity as a **named premise** for a dynamic link, and for each
//! entry the exported symbol it resolved to (name, version, weak or global,
//! `ifunc`, the file that defines it). The v1 grammar keeps loading byte for
//! byte.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::extern_call::AbiType;
use crate::native_library_header::{HeaderIdentity, IncludeIdentity};

/// The data model the artifact was built under (§4.4). v1 admits `lp64` only,
/// which makes `LOADER-ABI-LP64` an explicit statement of the manifest instead
/// of an implicit assumption of a reader. Making the scalar projection genuinely
/// target-aware stays that card's own follow-on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataModel {
    /// Spelled `lp64`.
    Lp64,
    /// Spelled `llp64` (64-bit Windows, where `long` is 32 bits). Admitted by a
    /// generated (v2) manifest only, whose recorded target derives it (card
    /// `W48-QRLIB-LINKAGE`); a v1 manifest admits `lp64` alone, as before.
    Llp64,
}

impl DataModel {
    #[must_use]
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Lp64 => "lp64",
            Self::Llp64 => "llp64",
        }
    }
}

/// The unwinding posture across the boundary (§4.4). v1 admits `abort` only:
/// the library promises its entry points terminate or return, because unwinding
/// through a C ABI frame is undefined behaviour. Stated, never checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnwindingPosture {
    /// Spelled `abort`: no unwinding crosses the boundary.
    Abort,
}

/// The allocator-custody posture (§4.4). v1 admits `closed` only: every
/// allocation the library hands out is released only through the library's own
/// entry points. A contract-less entry satisfies the structural half of this
/// check vacuously, because it cannot acquire (design §4.2); a contract slice
/// entry can, so [`check_closed_allocator_custody`] is where the structural half
/// is actually decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AllocatorCustody {
    /// Spelled `closed`.
    Closed,
}

/// The thread posture (§4.4). v1 admits `single_caller` only, and the premise
/// is currently vacuous because extern calls stay refused in concurrent
/// contexts (`Q026`).
///
/// Card `UNSAFE-LINKAGE` deliberately stated *no* thread posture for a waived
/// callee, on the ground that "an uncontracted callee has no declaration in
/// which to state one" (its §12.4). A manifest **is** that declaration, so the
/// field is admitted here without disturbing that decision: a waiver with no
/// manifest still states no posture, and only a manifest-resident one carries
/// what its manifest wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreadPosture {
    /// Spelled `single_caller`.
    SingleCaller,
}

/// The stated identity of one library artifact (§4.3). Every field is declared
/// text; see the module documentation for why none of it is probed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArtifactIdentity {
    /// The library's name, as the manifest spells it.
    pub library: String,
    /// The library's version, as the manifest spells it. Not parsed as semver:
    /// the manifest's spelling is the identity, and a normalisation would make
    /// two different spellings compare equal.
    pub version: String,
    /// The artifact file's name (static archive or shared object).
    pub artifact: String,
    /// `sha256:` followed by 64 lowercase hex digits. The only admitted
    /// algorithm in v1; anything else rejects the manifest.
    pub hash: String,
}

/// Where a manifest-resident symbol lives: the artifact identity it inherits
/// and the ABI premises stated beside it. This is the whole of what a
/// contract-less entry adds to a waiver, and it is carried onto the certificate
/// verbatim (card `UNSAFE-LINKAGE-ARTIFACT`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ManifestResidence {
    pub identity: ArtifactIdentity,
    pub premises: AbiPremises,
    /// The declared contract module this symbol's manifest entry named, for a
    /// `contract fn` entry (card `NATIVE-LIBRARY-CONTRACT-SLICE`), and `None`
    /// for a contract-less `unchecked fn` entry.
    ///
    /// The invariant is maintained by construction: [`ManifestBinding`] is the
    /// only producer of a residence, and it fills this field from the entry
    /// kind it resolved. A contract-less residence therefore never names a
    /// module and a contracted one always does, which is what lets a reader of
    /// a premise tell the two linkage levels apart by inspection.
    ///
    /// The reference is the **selection key** of the contract module, matched
    /// exactly and never normalised — the same discipline constraint 11 applies
    /// to a spelled input path, for the same reason: two spellings that
    /// normalise together are two different selections.
    pub contract: Option<String>,
    /// The exported header this symbol's declaration was **checked against**
    /// (card `NATIVE-LIBRARY-HEADER`), and `None` for a manifest that names no
    /// header — which is every manifest written before that card.
    ///
    /// This field's trust status differs from every other field of this record,
    /// and a reader of a premise must be able to tell: the library, version and
    /// artifact hash are *declared* and nothing opens them, while this reference
    /// names a text that **was read** and this digest was **measured over it**.
    /// See [`crate::native_library_header`] for why the header can honestly be
    /// verified where the artifact cannot.
    pub header: Option<HeaderIdentity>,
}

/// The four ABI premise fields of §4.4, all mandatory, none checked beyond the
/// closed value vocabulary. These are premises the certificate carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AbiPremises {
    pub data_model: DataModel,
    pub unwinding: UnwindingPosture,
    pub allocator_custody: AllocatorCustody,
    pub thread_posture: ThreadPosture,
}

/// One contract-less manifest entry: a symbol the artifact exports, with the
/// exported header declaration's ABI projection and **no** contract, therefore
/// no modes and no effects.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UncheckedEntry {
    pub symbol: String,
    /// The header declaration's parameter types, in order.
    pub parameters: Vec<AbiType>,
    /// The header declaration's result type.
    pub result: AbiType,
}

/// A type in a manifest entry's declaration (card `W48-QRLIB-LINKAGE`): the
/// §C04.1 ABI projection, or — in a generated (v2) contract slice entry only —
/// an imported C struct by value or by pointer, bound to the `struct!` layout
/// of the FFI route.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntryType {
    Abi(AbiType),
    /// `struct TAG`.
    Struct(String),
    /// `struct TAG *` (`constant == false`) or `const struct TAG *`.
    StructPointer {
        tag: String,
        constant: bool,
    },
}

impl EntryType {
    /// The C spelling, as a manifest writes it.
    #[must_use]
    pub fn c_spelling(&self) -> String {
        match self {
            Self::Abi(abi) => abi.c_spelling().to_owned(),
            Self::Struct(tag) => format!("struct {tag}"),
            Self::StructPointer {
                tag,
                constant: false,
            } => format!("struct {tag} *"),
            Self::StructPointer {
                tag,
                constant: true,
            } => format!("const struct {tag} *"),
        }
    }

    /// The ABI projection, or `None` for a struct position.
    #[must_use]
    pub fn abi(&self) -> Option<AbiType> {
        match self {
            Self::Abi(abi) => Some(*abi),
            Self::Struct(_) | Self::StructPointer { .. } => None,
        }
    }
}

/// How a defined symbol binds in the pinned artifact (card
/// `W48-QRLIB-LINKAGE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExportBinding {
    Global,
    /// A defined weak symbol: admitted, and recorded, because a strong
    /// definition elsewhere in the link would interpose it (§L14.8).
    Weak,
}

impl ExportBinding {
    #[must_use]
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Weak => "weak",
        }
    }
}

/// What the generator read about the symbol an entry binds: the `export`
/// clause of a v2 entry (card `W48-QRLIB-LINKAGE`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExportFacts {
    /// The object-level name (`_sqrt` on Mach-O, `sqrt` on ELF).
    pub symbol: String,
    /// The ELF default version the link binds (`GLIBC_2.2.5`).
    pub version: Option<String>,
    pub binding: ExportBinding,
    /// An ELF `STT_GNU_IFUNC`: a resolver chooses the code at load time.
    pub ifunc: bool,
    /// The DLL a COFF import names.
    pub dll: Option<String>,
    /// The file (and archive member) that defines it, as the reader named it.
    pub file: String,
}

impl ExportFacts {
    /// The clause text after `export`.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = self.symbol.clone();
        if let Some(version) = &self.version {
            out.push_str(" version ");
            out.push_str(version);
        }
        out.push(' ');
        out.push_str(self.binding.spelling());
        if self.ifunc {
            out.push_str(" ifunc");
        }
        if let Some(dll) = &self.dll {
            out.push_str(" dll ");
            out.push_str(dll);
        }
        out.push_str(" in ");
        out.push_str(&self.file);
        out
    }
}

/// Static or dynamic linkage (card `W48-QRLIB-LINKAGE`, user decision 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LinkageKind {
    /// The artifact is the code, pinned fully.
    Static,
    /// The artifact is a link-time stub; the runtime library is the OS's, and
    /// its identity is a named premise.
    Dynamic,
}

impl LinkageKind {
    #[must_use]
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Dynamic => "dynamic",
        }
    }
}

/// One file an `ld` script names, pinned beside the script.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinkedInput {
    pub reference: String,
    pub digest: String,
    /// Inside `AS_NEEDED ( … )`.
    pub as_needed: bool,
}

/// The runtime library's identity for a dynamic link: a **premise**, because
/// the OS owns the file the program loads.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimePremise {
    /// ELF: the `DT_SONAME` of a pinned shared object.
    Soname { soname: String, file: String },
    /// Mach-O: the stub's install name and current version.
    InstallName {
        name: String,
        current_version: Option<String>,
    },
    /// Windows: the DLL an import names and the OS build it was checked on.
    Dll { name: String, os_build: String },
}

impl RuntimePremise {
    /// The statement text after `runtime`.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Soname { soname, file } => format!("soname {soname} in {file}"),
            Self::InstallName {
                name,
                current_version: None,
            } => format!("install_name {name}"),
            Self::InstallName {
                name,
                current_version: Some(version),
            } => format!("install_name {name} current_version {version}"),
            Self::Dll { name, os_build } => format!("dll {name} os_build {os_build}"),
        }
    }
}

/// The linkage section of a generated (v2) manifest.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinkageSection {
    pub kind: LinkageKind,
    /// `elf`, `macho` or `coff`.
    pub format: String,
    /// The recorded target triple the artifact was read for.
    pub target: String,
    /// Every file the artifact (an `ld` script) names, in script order.
    pub inputs: Vec<LinkedInput>,
    /// Empty for static linkage; at least one for dynamic.
    pub runtime: Vec<RuntimePremise>,
}

/// One contract slice entry: a symbol the artifact exports, the exported header
/// declaration's ABI projection, and the declared `.qcore` contract module that
/// states what the symbol *does* (card `NATIVE-LIBRARY-CONTRACT-SLICE`, design
/// §4.1).
///
/// This is where a library's **pointer** symbols belong. A contract-less entry
/// refuses one ([`ManifestRefusal::PointerInUncheckedEntry`]) because it has no
/// modes in which to state what a pointer may be used for; a contract slice
/// entry has exactly that, in the unchanged C04 vocabulary, so the pointer
/// spellings are admitted here and nowhere else in a manifest.
///
/// The manifest **selects and attributes**; it creates no contract semantics.
/// The contract itself is loaded, admitted and bridged by the unchanged
/// resource-contract v1 machinery, and this entry only says *which* declared
/// contract the artifact's symbol is to be read through, and what the artifact's
/// own exported header declares for it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContractedEntry {
    pub symbol: String,
    /// The header declaration's parameter types, in order. Pointers admitted;
    /// a struct only in a generated (v2) manifest.
    pub parameters: Vec<EntryType>,
    /// The header declaration's result type.
    pub result: EntryType,
    /// The declared contract module reference, as the manifest spells it. It is
    /// the contract selection's own key, compared exactly.
    pub contract_module: String,
    /// The exported symbol the generator resolved (v2), or `None` in a v1
    /// manifest, which states none.
    pub export: Option<ExportFacts>,
}

impl ContractedEntry {
    /// The ABI signature, or `None` when a position carries a struct — which
    /// only the FFI route can call.
    #[must_use]
    pub fn abi_signature(&self) -> Option<(Vec<AbiType>, AbiType)> {
        let parameters = self
            .parameters
            .iter()
            .map(EntryType::abi)
            .collect::<Option<Vec<_>>>()?;
        Some((parameters, self.result.abi()?))
    }
}

/// One loaded manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeLibraryManifest {
    pub identity: ArtifactIdentity,
    pub premises: AbiPremises,
    /// The exported header this manifest binds, and its declared digest (card
    /// `NATIVE-LIBRARY-HEADER`, `header` + `header_hash`).
    ///
    /// **Optional as a pair**: both fields or neither. `None` is the default and
    /// is exactly the wave-19/wave-20 world — every manifest written before that
    /// card loads byte-identically, admits the same symbols and publishes the
    /// same premises. Opting in is per manifest and covers every symbol it
    /// binds; there is no per-entry opt-out, because an entry that opted out
    /// would be a transcription believed beside transcriptions checked with
    /// nothing marking which was which.
    pub header: Option<HeaderIdentity>,
    /// Contract-less entries by symbol. A symbol may be bound at most once.
    pub unchecked: BTreeMap<String, UncheckedEntry>,
    /// Contract slice entries by symbol (card `NATIVE-LIBRARY-CONTRACT-SLICE`).
    /// A symbol may be bound at most once **across both maps**: an entry that
    /// is both waived and contracted has no defined premise.
    pub contracted: BTreeMap<String, ContractedEntry>,
    /// The linkage section of a generated manifest (card
    /// `W48-QRLIB-LINKAGE`), or `None` for a v1 manifest.
    pub linkage: Option<LinkageSection>,
}

impl NativeLibraryManifest {
    /// Every contract module this manifest's entries name — the manifest's
    /// **slice**, derived rather than declared separately.
    ///
    /// Design §4.1 makes the slice the unit of the both-directions totality
    /// check. It is not spelled with its own statement because a second
    /// spelling would be a second place the slice could be wrong: a module the
    /// entries never name is not part of the binding, and a module they do name
    /// is, whether or not a `slice` line agreed.
    #[must_use]
    pub fn slice(&self) -> BTreeSet<&str> {
        self.contracted
            .values()
            .map(|entry| entry.contract_module.as_str())
            .collect()
    }
}

/// Why a manifest is rejected. Each constructor names one condition; none is a
/// catch-all, so widening the manifest grammar means deleting a refusal on
/// purpose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManifestRefusal {
    /// The first statement is not `native_library manifest v1;`. The marker is
    /// the selector, exactly as `requires resource_contract_v1` is for the
    /// contract loader: a file that does not carry it is rejected here rather
    /// than interpreted under a guess.
    MissingMarker,
    /// A statement is not terminated by `;`.
    UnterminatedStatement { statement: String },
    /// A statement's keyword is not a v1 field or entry kind.
    UnknownField { field: String },
    /// A header field is stated twice. The manifest is a binding, so a repeated
    /// field has no defined meaning and is refused rather than resolved by a
    /// last-one-wins rule nobody wrote down.
    RepeatedField { field: String },
    /// A mandatory header field is absent. All eight are mandatory: an identity
    /// missing a field is not a weaker identity, it is an unusable one.
    MissingField { field: &'static str },
    /// A header field's value is empty or carries whitespace.
    MalformedValue { field: String, value: String },
    /// A header field's value is outside the closed v1 vocabulary for that
    /// field — for instance a data model other than `lp64`.
    UnknownValue { field: String, value: String },
    /// The stated hash is not `sha256:` followed by 64 lowercase hex digits.
    MalformedHash { hash: String },
    /// An entry is not `unchecked fn NAME(TYPES) -> TYPE`.
    MalformedEntry { statement: String },
    /// A contract slice entry is not
    /// `contract fn NAME(TYPES) -> TYPE from MODULE` — most often a missing
    /// `from MODULE` tail, which is the whole difference between the two entry
    /// kinds and is therefore never inferred (card
    /// `NATIVE-LIBRARY-CONTRACT-SLICE`).
    MalformedContractedEntry { statement: String },
    /// A contract slice entry's module reference is empty or carries
    /// whitespace. The reference is a selection key compared exactly, so a
    /// reference that cannot be one is refused rather than trimmed into shape.
    MalformedContractReference { symbol: String, module: String },
    /// An entry's parameter type is outside the §C04.1 ABI projection. The
    /// manifest states the *exported header declaration*, in the profile's
    /// canonical stdint spellings; reading a real `.h` is follow-on
    /// `NATIVE-LIBRARY-HEADER`.
    UnknownParameterSpelling {
        symbol: String,
        parameter: usize,
        spelling: String,
    },
    /// An entry's result type is outside the §C04.1 ABI projection.
    UnknownResultSpelling { symbol: String, spelling: String },
    /// A contract-less entry declares a pointer, in either direction. A
    /// contract-less entry has no modes to state, and design §3.1 is the
    /// argument that a pointer at that boundary has no assumption text to
    /// write — so an artifact hash cannot buy one. A library's pointer symbols
    /// belong in a contract slice entry, which card
    /// `NATIVE-LIBRARY-CONTRACT-SLICE` landed, and this refusal is what keeps
    /// the two apart: the refusal was **not** weakened when the contracted
    /// entry arrived, it acquired a destination.
    PointerInUncheckedEntry { symbol: String, spelling: String },
    /// An entry's parameter is declared `unit`, which the ABI vocabulary admits
    /// as a result type only.
    UnitParameterInEntry { symbol: String, parameter: usize },
    /// An `include` statement is not
    /// `include SPELLING from REFERENCE HASH` (card
    /// `NATIVE-LIBRARY-HEADER-DECORATION`).
    MalformedIncludeBinding { statement: String },
    /// An `include` statement's spelling is not `<…>` or `"…"` around a
    /// non-empty whitespace-free body. The spelling is matched against the
    /// header's own `#include` operand **exactly**, so a spelling that cannot be
    /// one is refused rather than repaired into shape — constraint 11's
    /// discipline, applied to a fourth kind of spelled name.
    MalformedIncludeSpelling { spelling: String },
    /// Two `include` statements declare the same spelling. The spelling is the
    /// key a `#include` line is matched on, so two texts for one spelling have
    /// no defined meaning.
    IncludeBoundTwice { spelling: String },
    /// A manifest declares an `include` text and names no `header`. The declared
    /// texts exist only to be read *into* the exported header, so one without a
    /// header names a text nothing reads — a statement with no effect, refused
    /// rather than ignored.
    IncludeWithoutHeader { spelling: String },
    /// A field of the generated (v2) grammar in a v1 manifest, or a v1-only
    /// statement (`include`) in a v2 one (card `W48-QRLIB-LINKAGE`).
    WrongVersionField {
        field: String,
        version: &'static str,
    },
    /// A contract-less `unchecked fn` entry in a generated manifest, which binds
    /// declared contracts only: a waiver is a v1 manifest's.
    UncheckedInGenerated { symbol: String },
    /// A v2 entry without its `export …` clause, or a v1 entry with one.
    MissingExport { symbol: String },
    /// An `export` clause that is not
    /// `SYMBOL [version V] (global|weak) [ifunc] [dll DLL] in FILE`.
    MalformedExport { symbol: String, clause: String },
    /// An `input` statement that is not `input REFERENCE sha256:HEX [as_needed]`.
    MalformedInput { statement: String },
    /// A `runtime` statement that is not one of the three premise forms.
    MalformedRuntime { statement: String },
    /// A static manifest states a runtime premise: the code is pinned, so there
    /// is no runtime library to name.
    RuntimeInStaticLinkage,
    /// A dynamic manifest states no runtime premise: the OS supplies the code,
    /// and a dynamic link that does not name it would drop that premise.
    MissingRuntime,
    /// Two entries bind the same symbol.
    SymbolBoundTwice { symbol: String },
    /// The manifest binds no symbol at all. An artifact identity that names
    /// nothing links nothing, and admitting it would put a premise on a
    /// certificate that no callee rests on.
    NoSymbols,
}

impl fmt::Display for ManifestRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMarker => write!(
                f,
                "the first statement is not `native_library manifest v1;`; this loader interprets no file that does not carry its own marker"
            ),
            Self::UnterminatedStatement { statement } => {
                write!(f, "the statement `{statement}` is not terminated by `;`")
            }
            Self::UnknownField { field } => write!(
                f,
                "`{field}` is not a field or entry kind of a v1 native-library manifest"
            ),
            Self::RepeatedField { field } => {
                write!(f, "the field `{field}` is stated more than once")
            }
            Self::MissingField { field } => {
                write!(f, "the mandatory field `{field}` is absent")
            }
            Self::MalformedValue { field, value } => write!(
                f,
                "the value `{value}` of field `{field}` is empty or carries whitespace"
            ),
            Self::UnknownValue { field, value } => write!(
                f,
                "`{value}` is not an admitted value of field `{field}` in v1"
            ),
            Self::MalformedHash { hash } => write!(
                f,
                "the stated hash `{hash}` is not `sha256:` followed by 64 lowercase hex digits"
            ),
            Self::MalformedEntry { statement } => write!(
                f,
                "the entry `{statement}` is not `unchecked fn NAME(TYPES) -> TYPE`"
            ),
            Self::MalformedContractedEntry { statement } => write!(
                f,
                "the entry `{statement}` is not `contract fn NAME(TYPES) -> TYPE from MODULE`; a contract slice entry must name the declared contract module it binds to"
            ),
            Self::MalformedContractReference { symbol, module } => write!(
                f,
                "entry `{symbol}` names the contract module `{module}`, which is empty or carries whitespace"
            ),
            Self::UnknownParameterSpelling {
                symbol,
                parameter,
                spelling,
            } => write!(
                f,
                "entry `{symbol}` declares parameter {parameter} as `{spelling}`, which is not a C ABI type this profile projects"
            ),
            Self::UnknownResultSpelling { symbol, spelling } => write!(
                f,
                "entry `{symbol}` declares result `{spelling}`, which is not a C ABI type this profile projects"
            ),
            Self::PointerInUncheckedEntry { symbol, spelling } => write!(
                f,
                "contract-less entry `{symbol}` declares `{spelling}`; a contract-less entry has no modes in which to state what a pointer may be used for, and an artifact identity says which code, not what it does with your pointer"
            ),
            Self::UnitParameterInEntry { symbol, parameter } => write!(
                f,
                "entry `{symbol}` declares parameter {parameter} as `void`"
            ),
            Self::MalformedIncludeBinding { statement } => write!(
                f,
                "the statement `{statement}` is not `include SPELLING from REFERENCE HASH`; a declared include text names the `#include` operand it stands for, the reference it is read from, and its digest"
            ),
            Self::MalformedIncludeSpelling { spelling } => write!(
                f,
                "the include spelling `{spelling}` is not `<…>` or `\"…\"` around a non-empty reference without whitespace"
            ),
            Self::IncludeBoundTwice { spelling } => {
                write!(f, "two include statements declare `{spelling}`")
            }
            Self::IncludeWithoutHeader { spelling } => write!(
                f,
                "the manifest declares the include text `{spelling}` and names no `header`, so nothing would read it"
            ),
            Self::WrongVersionField { field, version } => write!(
                f,
                "`{field}` is not a statement of a `native_library manifest {version}`"
            ),
            Self::UncheckedInGenerated { symbol } => write!(
                f,
                "`unchecked fn {symbol}` in a generated (v2) manifest; a generated manifest binds declared contracts, and a contract-less waiver is a v1 manifest's"
            ),
            Self::MissingExport { symbol } => write!(
                f,
                "entry `{symbol}`: a generated (v2) entry carries exactly one `export …` clause naming the symbol it resolved, and a v1 entry carries none"
            ),
            Self::MalformedExport { symbol, clause } => write!(
                f,
                "entry `{symbol}`: `export {clause}` is not `export SYMBOL [version V] (global|weak) [ifunc] [dll DLL] in FILE`"
            ),
            Self::MalformedInput { statement } => write!(
                f,
                "the statement `{statement}` is not `input REFERENCE sha256:HEX [as_needed]`"
            ),
            Self::MalformedRuntime { statement } => write!(
                f,
                "the statement `{statement}` is not `runtime soname NAME in FILE`, `runtime install_name NAME [current_version V]` or `runtime dll NAME os_build BUILD`"
            ),
            Self::RuntimeInStaticLinkage => write!(
                f,
                "a static manifest pins the code itself and names no runtime library"
            ),
            Self::MissingRuntime => write!(
                f,
                "a dynamic manifest names no runtime library; the OS supplies the code, so its identity must be stated as a premise"
            ),
            Self::SymbolBoundTwice { symbol } => {
                write!(f, "the symbol `{symbol}` is bound by two entries")
            }
            Self::NoSymbols => write!(f, "the manifest binds no symbol"),
        }
    }
}

impl std::error::Error for ManifestRefusal {}

/// The v1 header fields, in the order [`ManifestRefusal::MissingField`] reports
/// them absent.
const MANDATORY_FIELDS: [&str; 8] = [
    "library",
    "version",
    "artifact",
    "hash",
    "data_model",
    "unwinding",
    "allocator_custody",
    "thread_posture",
];

/// The v1 header-binding fields (card `NATIVE-LIBRARY-HEADER`). Optional **as a
/// pair**: a `header` with no `header_hash`, or the reverse, is a
/// [`ManifestRefusal::MissingField`] on the same ground as every mandatory
/// field — an identity missing a field is not a weaker identity, it is an
/// unusable one.
const HEADER_BINDING_FIELDS: [&str; 2] = ["header", "header_hash"];

/// Read one `include SPELLING from REFERENCE HASH` statement's body (card
/// `NATIVE-LIBRARY-HEADER-DECORATION`).
///
/// The `from` tail is the contract slice entry's keyword, reused for the same
/// reason: a statement that names two references names which is which rather
/// than relying on position. The **spelling** is the operand of the header's own
/// `#include` line, delimiters and all (`<event2/visibility.h>`), because that is
/// the token a reader matches against; the **reference** is where the text is
/// resolved from, in the same namespace as `header`. Keeping them apart is what
/// stops a manifest from having to restate a library's include layout, and what
/// lets a certificate name the text by the path it was read from.
fn parse_include_binding(rest: &str, statement: &str) -> Result<IncludeIdentity, ManifestRefusal> {
    let malformed = || ManifestRefusal::MalformedIncludeBinding {
        statement: statement.to_owned(),
    };
    let (spelling, tail) = rest.split_once(char::is_whitespace).ok_or_else(malformed)?;
    let tail = tail.trim();
    let body = tail.strip_prefix("from ").ok_or_else(malformed)?;
    let (reference, digest) = body
        .trim()
        .split_once(char::is_whitespace)
        .ok_or_else(malformed)?;
    let digest = digest.trim();
    if !well_formed_include_spelling(spelling) {
        return Err(ManifestRefusal::MalformedIncludeSpelling {
            spelling: spelling.to_owned(),
        });
    }
    if !well_formed_value(reference) {
        return Err(ManifestRefusal::MalformedValue {
            field: "include".to_owned(),
            value: reference.to_owned(),
        });
    }
    if !well_formed_hash(digest) {
        return Err(ManifestRefusal::MalformedHash {
            hash: digest.to_owned(),
        });
    }
    Ok(IncludeIdentity {
        spelling: spelling.to_owned(),
        reference: reference.to_owned(),
        digest: digest.to_owned(),
    })
}

/// Whether an include spelling is `<…>` or `"…"` around a non-empty body with no
/// whitespace. The delimiters are part of the spelling because the header writes
/// them, and `<x.h>` and `"x.h"` are two different `#include` lines.
fn well_formed_include_spelling(spelling: &str) -> bool {
    let body = match (spelling.strip_prefix('<'), spelling.strip_prefix('"')) {
        (Some(rest), None) => rest.strip_suffix('>'),
        (None, Some(rest)) => rest.strip_suffix('"'),
        _ => None,
    };
    body.is_some_and(well_formed_value)
}

/// The manifest's selector marker, which must be its first statement.
const MARKER: &str = "native_library manifest v1";

/// The generated manifest's marker (card `W48-QRLIB-LINKAGE`).
pub const MARKER_V2: &str = "native_library manifest v2";

/// The fields a generated manifest adds, each mandatory and stated once.
const GENERATED_FIELDS: [&str; 3] = ["linkage", "format", "target"];

/// Split a manifest into `;`-terminated statements, dropping `#` comment lines
/// and blank lines. Returns the statement texts with surrounding whitespace
/// removed; internal whitespace is preserved because entry types carry it.
fn statements(text: &str) -> Result<Vec<String>, ManifestRefusal> {
    let mut collected = Vec::new();
    let mut pending = String::new();
    for raw in text.lines() {
        let line = match raw.find('#') {
            Some(position) => &raw[..position],
            None => raw,
        };
        if line.trim().is_empty() && pending.trim().is_empty() {
            continue;
        }
        for (index, piece) in line.split(';').enumerate() {
            if index > 0 {
                collected.push(pending.trim().to_owned());
                pending.clear();
            }
            pending.push(' ');
            pending.push_str(piece);
        }
    }
    if !pending.trim().is_empty() {
        return Err(ManifestRefusal::UnterminatedStatement {
            statement: pending.trim().to_owned(),
        });
    }
    Ok(collected)
}

/// Whether a header value is a single non-empty whitespace-free token.
fn well_formed_value(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_whitespace)
}

/// Whether a stated hash is `sha256:` followed by 64 lowercase hex digits. The
/// only admitted algorithm in v1: an unrecognised one rejects rather than being
/// carried as an opaque string, because a premise a reader cannot interpret is
/// not one this repository publishes.
fn well_formed_hash(hash: &str) -> bool {
    let Some(digest) = hash.strip_prefix("sha256:") else {
        return false;
    };
    digest.len() == 64 && digest.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}

/// Whether a symbol name is a C identifier.
fn well_formed_symbol(symbol: &str) -> bool {
    let mut characters = symbol.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    characters.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// One entry's declaration head, `NAME(TYPES) -> TYPE`, split before any type
/// spelling is interpreted. Both entry kinds share it, so the two grammars
/// cannot drift in how they read a header declaration — only in what they
/// admit inside one.
struct EntryHead<'a> {
    symbol: &'a str,
    inside: &'a str,
    result_text: &'a str,
    /// Whatever followed the result type, trimmed. Empty for a contract-less
    /// entry; the `from MODULE` tail of a contract slice entry.
    tail: &'a str,
}

/// Split `NAME(TYPES) -> TYPE …`, or refuse with the caller's own malformation.
fn split_entry_head<'a>(
    body: &'a str,
    malformed: &dyn Fn() -> ManifestRefusal,
) -> Result<EntryHead<'a>, ManifestRefusal> {
    let open = body.find('(').ok_or_else(malformed)?;
    let close = body.find(')').ok_or_else(malformed)?;
    if close < open {
        return Err(malformed());
    }
    let symbol = body[..open].trim();
    if !well_formed_symbol(symbol) {
        return Err(malformed());
    }
    let after = body[close + 1..].trim();
    let rest = after.strip_prefix("->").ok_or_else(malformed)?.trim();
    // The result type and the optional tail are separated by ` from `, which is
    // sought from the right so a multi-word result spelling (`uint8_t *`) keeps
    // its space. A module reference carries no whitespace, so the rightmost
    // separator is the only candidate.
    let (result_text, tail) = match rest.rsplit_once(" from ") {
        Some((result, tail)) => (result.trim(), tail.trim()),
        None => (rest, ""),
    };
    if result_text.is_empty() {
        return Err(malformed());
    }
    Ok(EntryHead {
        symbol,
        inside: body[open + 1..close].trim(),
        result_text,
        tail,
    })
}

/// Interpret one entry's parameter list in the §C04.1 ABI projection.
///
/// `pointers` is the one difference between the two entry kinds: a contract
/// slice entry admits the pointer spellings because its declared contract has
/// modes in which to state what they may be used for, and a contract-less entry
/// does not.
fn parse_entry_parameters(
    symbol: &str,
    inside: &str,
    pointers: bool,
) -> Result<Vec<AbiType>, ManifestRefusal> {
    let mut parameters = Vec::new();
    if inside.is_empty() {
        return Ok(parameters);
    }
    for (index, spelling) in inside.split(',').enumerate() {
        let spelling = spelling.trim();
        let Some(abi) = parse_c_declaration_type(spelling) else {
            return Err(ManifestRefusal::UnknownParameterSpelling {
                symbol: symbol.to_owned(),
                parameter: index,
                spelling: spelling.to_owned(),
            });
        };
        if abi == AbiType::Unit {
            return Err(ManifestRefusal::UnitParameterInEntry {
                symbol: symbol.to_owned(),
                parameter: index,
            });
        }
        if !pointers && abi.is_pointer() {
            return Err(ManifestRefusal::PointerInUncheckedEntry {
                symbol: symbol.to_owned(),
                spelling: spelling.to_owned(),
            });
        }
        parameters.push(abi);
    }
    Ok(parameters)
}

/// Interpret one entry's result spelling in the §C04.1 ABI projection.
fn parse_entry_result(
    symbol: &str,
    result_text: &str,
    pointers: bool,
) -> Result<AbiType, ManifestRefusal> {
    let Some(result) = parse_c_declaration_type(result_text) else {
        return Err(ManifestRefusal::UnknownResultSpelling {
            symbol: symbol.to_owned(),
            spelling: result_text.to_owned(),
        });
    };
    if !pointers && result.is_pointer() {
        return Err(ManifestRefusal::PointerInUncheckedEntry {
            symbol: symbol.to_owned(),
            spelling: result_text.to_owned(),
        });
    }
    Ok(result)
}

/// Parse one contract-less entry body, `NAME(TYPES) -> TYPE`.
fn parse_unchecked_entry(body: &str, statement: &str) -> Result<UncheckedEntry, ManifestRefusal> {
    let malformed = || ManifestRefusal::MalformedEntry {
        statement: statement.to_owned(),
    };
    let head = split_entry_head(body, &malformed)?;
    // A contract-less entry has no `from MODULE` tail. One written anyway is a
    // malformed contract-less entry rather than a silently ignored suffix: the
    // two entry kinds are told apart by their keyword, never by their tail.
    if !head.tail.is_empty() {
        return Err(malformed());
    }
    let parameters = parse_entry_parameters(head.symbol, head.inside, false)?;
    let result = parse_entry_result(head.symbol, head.result_text, false)?;
    Ok(UncheckedEntry {
        symbol: head.symbol.to_owned(),
        parameters,
        result,
    })
}

/// Parse one contract slice entry body, `NAME(TYPES) -> TYPE from MODULE` (card
/// `NATIVE-LIBRARY-CONTRACT-SLICE`).
fn parse_contracted_entry(
    body: &str,
    statement: &str,
    generated: bool,
) -> Result<ContractedEntry, ManifestRefusal> {
    let malformed = || ManifestRefusal::MalformedContractedEntry {
        statement: statement.to_owned(),
    };
    // Card `W48-QRLIB-LINKAGE`: a generated entry ends with its `export`
    // clause, split off first so the head reads exactly as a v1 entry's.
    let (body, export) = if generated {
        let Some((head, clause)) = body.rsplit_once(" export ") else {
            let symbol = body.split('(').next().unwrap_or(body).trim().to_owned();
            return Err(ManifestRefusal::MissingExport { symbol });
        };
        (head, Some(clause.trim()))
    } else {
        (body, None)
    };
    let head = split_entry_head(body, &malformed)?;
    if head.tail.is_empty() {
        return Err(malformed());
    }
    if !well_formed_value(head.tail) {
        return Err(ManifestRefusal::MalformedContractReference {
            symbol: head.symbol.to_owned(),
            module: head.tail.to_owned(),
        });
    }
    let (parameters, result) = if generated {
        let mut parameters = Vec::new();
        if !head.inside.is_empty() {
            for (index, spelling) in head.inside.split(',').enumerate() {
                let spelling = spelling.trim();
                let ty = parse_entry_type(spelling).ok_or_else(|| {
                    ManifestRefusal::UnknownParameterSpelling {
                        symbol: head.symbol.to_owned(),
                        parameter: index,
                        spelling: spelling.to_owned(),
                    }
                })?;
                if ty == EntryType::Abi(AbiType::Unit) {
                    return Err(ManifestRefusal::UnitParameterInEntry {
                        symbol: head.symbol.to_owned(),
                        parameter: index,
                    });
                }
                parameters.push(ty);
            }
        }
        let result = parse_entry_type(head.result_text).ok_or_else(|| {
            ManifestRefusal::UnknownResultSpelling {
                symbol: head.symbol.to_owned(),
                spelling: head.result_text.to_owned(),
            }
        })?;
        (parameters, result)
    } else {
        (
            parse_entry_parameters(head.symbol, head.inside, true)?
                .into_iter()
                .map(EntryType::Abi)
                .collect(),
            EntryType::Abi(parse_entry_result(head.symbol, head.result_text, true)?),
        )
    };
    let export = match export {
        None => None,
        Some(clause) => Some(parse_export(head.symbol, clause)?),
    };
    Ok(ContractedEntry {
        symbol: head.symbol.to_owned(),
        parameters,
        result,
        contract_module: head.tail.to_owned(),
        export,
    })
}

/// Parse a generated entry's type: the ABI projection, or `struct TAG`,
/// `struct TAG *`, `const struct TAG *` (card `W48-QRLIB-LINKAGE`).
#[must_use]
pub fn parse_entry_type(text: &str) -> Option<EntryType> {
    if let Some(abi) = parse_c_declaration_type(text) {
        return Some(EntryType::Abi(abi));
    }
    let normalized = text
        .replace('*', " * ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let words: Vec<&str> = normalized.split(' ').collect();
    let tag_ok = |tag: &str| well_formed_symbol(tag);
    match words.as_slice() {
        ["struct", tag] if tag_ok(tag) => Some(EntryType::Struct((*tag).to_owned())),
        ["struct", tag, "*"] if tag_ok(tag) => Some(EntryType::StructPointer {
            tag: (*tag).to_owned(),
            constant: false,
        }),
        ["const", "struct", tag, "*"] if tag_ok(tag) => Some(EntryType::StructPointer {
            tag: (*tag).to_owned(),
            constant: true,
        }),
        _ => None,
    }
}

/// Parse `SYMBOL [version V] (global|weak) [ifunc] [dll DLL] in FILE`.
fn parse_export(symbol: &str, clause: &str) -> Result<ExportFacts, ManifestRefusal> {
    let malformed = || ManifestRefusal::MalformedExport {
        symbol: symbol.to_owned(),
        clause: clause.to_owned(),
    };
    let words: Vec<&str> = clause.split_whitespace().collect();
    let mut at = 0;
    let next = |at: &mut usize| -> Option<&str> {
        let word = words.get(*at).copied();
        *at += 1;
        word
    };
    let name = next(&mut at).ok_or_else(malformed)?;
    let mut version = None;
    let mut word = next(&mut at).ok_or_else(malformed)?;
    if word == "version" {
        version = Some(next(&mut at).ok_or_else(malformed)?.to_owned());
        word = next(&mut at).ok_or_else(malformed)?;
    }
    let binding = match word {
        "global" => ExportBinding::Global,
        "weak" => ExportBinding::Weak,
        _ => return Err(malformed()),
    };
    let mut word = next(&mut at).ok_or_else(malformed)?;
    let mut ifunc = false;
    if word == "ifunc" {
        ifunc = true;
        word = next(&mut at).ok_or_else(malformed)?;
    }
    let mut dll = None;
    if word == "dll" {
        dll = Some(next(&mut at).ok_or_else(malformed)?.to_owned());
        word = next(&mut at).ok_or_else(malformed)?;
    }
    if word != "in" {
        return Err(malformed());
    }
    let file = next(&mut at).ok_or_else(malformed)?.to_owned();
    if at != words.len() || name.is_empty() {
        return Err(malformed());
    }
    Ok(ExportFacts {
        symbol: name.to_owned(),
        version,
        binding,
        ifunc,
        dll,
        file,
    })
}

/// Parse `input REFERENCE sha256:HEX [as_needed]`.
fn parse_input(rest: &str, statement: &str) -> Result<LinkedInput, ManifestRefusal> {
    let malformed = || ManifestRefusal::MalformedInput {
        statement: statement.to_owned(),
    };
    let words: Vec<&str> = rest.split_whitespace().collect();
    let (reference, digest, as_needed) = match words.as_slice() {
        [reference, digest] => (*reference, *digest, false),
        [reference, digest, "as_needed"] => (*reference, *digest, true),
        _ => return Err(malformed()),
    };
    if !well_formed_hash(digest) {
        return Err(ManifestRefusal::MalformedHash {
            hash: digest.to_owned(),
        });
    }
    Ok(LinkedInput {
        reference: reference.to_owned(),
        digest: digest.to_owned(),
        as_needed,
    })
}

/// Parse one `runtime …` premise.
fn parse_runtime(rest: &str, statement: &str) -> Result<RuntimePremise, ManifestRefusal> {
    let words: Vec<&str> = rest.split_whitespace().collect();
    match words.as_slice() {
        ["soname", soname, "in", file] => Ok(RuntimePremise::Soname {
            soname: (*soname).to_owned(),
            file: (*file).to_owned(),
        }),
        ["install_name", name] => Ok(RuntimePremise::InstallName {
            name: (*name).to_owned(),
            current_version: None,
        }),
        ["install_name", name, "current_version", version] => Ok(RuntimePremise::InstallName {
            name: (*name).to_owned(),
            current_version: Some((*version).to_owned()),
        }),
        ["dll", name, "os_build", build] => Ok(RuntimePremise::Dll {
            name: (*name).to_owned(),
            os_build: (*build).to_owned(),
        }),
        _ => Err(ManifestRefusal::MalformedRuntime {
            statement: statement.to_owned(),
        }),
    }
}

/// Parse one C declaration type as the manifest spells it: the canonical stdint
/// projection [`AbiType::c_spelling`] prints, with the space before a `*`
/// optional. Anything else is not an ABI type this profile projects, which is a
/// manifest rejection rather than a lenient reading.
#[must_use]
pub fn parse_c_declaration_type(text: &str) -> Option<AbiType> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let candidates = [
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
        // Card `W48-QRLIB-LINKAGE` (`W48-QR-FLOAT-QRLIB-SPELLING`): `float`
        // and `double`, in both manifest versions.
        AbiType::F32,
        AbiType::F64,
        AbiType::PtrMutU8,
        AbiType::PtrConstU8,
        AbiType::PtrMutPtrMutU8,
        AbiType::Unit,
    ];
    candidates.into_iter().find(|candidate| {
        let spelling = candidate.c_spelling();
        normalized == spelling || normalized == spelling.replace(" *", "*")
    })
}

/// Load one manifest, total or reject.
///
/// Every rejection is a named [`ManifestRefusal`]; nothing is skipped and
/// nothing is admitted partially. The result is a *function* of the text, so a
/// certificate consumer can repeat the load and reproduce the binding — which
/// is what makes the artifact identity rederivable.
pub fn load_native_library_manifest(text: &str) -> Result<NativeLibraryManifest, ManifestRefusal> {
    let statements = statements(text)?;
    let mut iterator = statements.iter();
    // Card `W48-QRLIB-LINKAGE`: the marker selects the grammar. `v2` is the
    // generated manifest; `v1` loads exactly as before.
    let generated = match iterator.next() {
        Some(first) => match first
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .as_str()
        {
            MARKER => false,
            MARKER_V2 => true,
            _ => return Err(ManifestRefusal::MissingMarker),
        },
        None => return Err(ManifestRefusal::MissingMarker),
    };
    let version = if generated { "v2" } else { "v1" };
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    let mut unchecked: BTreeMap<String, UncheckedEntry> = BTreeMap::new();
    let mut contracted: BTreeMap<String, ContractedEntry> = BTreeMap::new();
    let mut includes: BTreeMap<String, IncludeIdentity> = BTreeMap::new();
    let mut inputs: Vec<LinkedInput> = Vec::new();
    let mut runtime: Vec<RuntimePremise> = Vec::new();
    for statement in iterator {
        if statement.is_empty() {
            continue;
        }
        let (keyword, rest) = match statement.find(char::is_whitespace) {
            Some(position) => (&statement[..position], statement[position..].trim()),
            None => (statement.as_str(), ""),
        };
        match keyword {
            "input" | "runtime" | "linkage" | "format" | "target" if !generated => {
                return Err(ManifestRefusal::WrongVersionField {
                    field: keyword.to_owned(),
                    version,
                });
            }
            "include" if generated => {
                return Err(ManifestRefusal::WrongVersionField {
                    field: keyword.to_owned(),
                    version,
                });
            }
            "input" => {
                let input = parse_input(rest, statement)?;
                if inputs
                    .iter()
                    .any(|known| known.reference == input.reference)
                {
                    return Err(ManifestRefusal::RepeatedField {
                        field: format!("input {}", input.reference),
                    });
                }
                inputs.push(input);
            }
            "runtime" => {
                let premise = parse_runtime(rest, statement)?;
                if runtime.contains(&premise) {
                    return Err(ManifestRefusal::RepeatedField {
                        field: format!("runtime {}", premise.render()),
                    });
                }
                runtime.push(premise);
            }
            "unchecked" => {
                let Some(body) = rest.strip_prefix("fn ") else {
                    return Err(ManifestRefusal::MalformedEntry {
                        statement: statement.clone(),
                    });
                };
                if generated {
                    let symbol = body.split('(').next().unwrap_or(body).trim().to_owned();
                    return Err(ManifestRefusal::UncheckedInGenerated { symbol });
                }
                let entry = parse_unchecked_entry(body.trim(), statement)?;
                if unchecked.contains_key(&entry.symbol) || contracted.contains_key(&entry.symbol) {
                    return Err(ManifestRefusal::SymbolBoundTwice {
                        symbol: entry.symbol,
                    });
                }
                unchecked.insert(entry.symbol.clone(), entry);
            }
            // Card `NATIVE-LIBRARY-CONTRACT-SLICE` implements what card
            // `UNSAFE-LINKAGE-ARTIFACT` refused by name here. The refusal was a
            // seam, not a shortcut, and filling it is the whole of this card's
            // manifest grammar delta.
            "contract" => {
                let Some(body) = rest.strip_prefix("fn ") else {
                    return Err(ManifestRefusal::MalformedContractedEntry {
                        statement: statement.clone(),
                    });
                };
                let entry = parse_contracted_entry(body.trim(), statement, generated)?;
                // A symbol may not be bound twice **across both kinds**: an
                // entry that is at once waived and contracted has no defined
                // premise, and choosing one by entry order would make the
                // premise depend on the order the author wrote its lines.
                if unchecked.contains_key(&entry.symbol) || contracted.contains_key(&entry.symbol) {
                    return Err(ManifestRefusal::SymbolBoundTwice {
                        symbol: entry.symbol,
                    });
                }
                contracted.insert(entry.symbol.clone(), entry);
            }
            // Card `NATIVE-LIBRARY-HEADER-DECORATION`: the texts the exported
            // header's own `#include` lines may be read from. Each is
            // hash-identified exactly as the header is, and the set is closed —
            // an `#include` this manifest does not declare is not followed, and
            // stays exactly as unreadable as it was before this card.
            "include" => {
                let binding = parse_include_binding(rest, statement)?;
                if includes.contains_key(&binding.spelling) {
                    return Err(ManifestRefusal::IncludeBoundTwice {
                        spelling: binding.spelling,
                    });
                }
                includes.insert(binding.spelling.clone(), binding);
            }
            field
                if MANDATORY_FIELDS.contains(&field)
                    || HEADER_BINDING_FIELDS.contains(&field)
                    || (generated && GENERATED_FIELDS.contains(&field)) =>
            {
                if fields.contains_key(field) {
                    return Err(ManifestRefusal::RepeatedField {
                        field: field.to_owned(),
                    });
                }
                if !well_formed_value(rest) {
                    return Err(ManifestRefusal::MalformedValue {
                        field: field.to_owned(),
                        value: rest.to_owned(),
                    });
                }
                fields.insert(field.to_owned(), rest.to_owned());
            }
            other => {
                return Err(ManifestRefusal::UnknownField {
                    field: other.to_owned(),
                });
            }
        }
    }
    for field in MANDATORY_FIELDS {
        if !fields.contains_key(field) {
            return Err(ManifestRefusal::MissingField { field });
        }
    }
    if generated {
        for field in GENERATED_FIELDS {
            if !fields.contains_key(field) {
                return Err(ManifestRefusal::MissingField { field });
            }
        }
    }
    // Card `NATIVE-LIBRARY-HEADER`: the two header-binding fields are optional
    // together and mandatory together. One without the other names a header
    // whose identity is unstated, or an identity for a header the manifest does
    // not name — neither is a weaker binding, both are unusable ones.
    let header = match (fields.get("header"), fields.get("header_hash")) {
        (None, None) => None,
        (Some(reference), Some(digest)) => {
            if !well_formed_hash(digest) {
                return Err(ManifestRefusal::MalformedHash {
                    hash: digest.clone(),
                });
            }
            Some(HeaderIdentity {
                reference: reference.clone(),
                digest: digest.clone(),
                includes: includes.values().cloned().collect(),
            })
        }
        (Some(_), None) => {
            return Err(ManifestRefusal::MissingField {
                field: "header_hash",
            });
        }
        (None, Some(_)) => return Err(ManifestRefusal::MissingField { field: "header" }),
    };
    // A declared include text is read *into* the exported header. Declaring one
    // without a header would name a text nothing opens, and a manifest statement
    // with no effect is a statement a reader would be entitled to believe.
    if header.is_none()
        && let Some((spelling, _)) = includes.iter().next()
    {
        return Err(ManifestRefusal::IncludeWithoutHeader {
            spelling: spelling.clone(),
        });
    }
    let value = |field: &str| fields.get(field).expect("every field is present").clone();
    let hash = value("hash");
    if !well_formed_hash(&hash) {
        return Err(ManifestRefusal::MalformedHash { hash });
    }
    fn closed_value<T>(
        fields: &BTreeMap<String, String>,
        field: &str,
        admitted: &str,
        parsed: T,
    ) -> Result<T, ManifestRefusal> {
        let stated = fields.get(field).expect("every field is present").clone();
        if stated == admitted {
            Ok(parsed)
        } else {
            Err(ManifestRefusal::UnknownValue {
                field: field.to_owned(),
                value: stated,
            })
        }
    }
    let data_model = if generated && fields.get("data_model").map(String::as_str) == Some("llp64") {
        DataModel::Llp64
    } else {
        closed_value(&fields, "data_model", "lp64", DataModel::Lp64)?
    };
    let premises = AbiPremises {
        data_model,
        unwinding: closed_value(&fields, "unwinding", "abort", UnwindingPosture::Abort)?,
        allocator_custody: closed_value(
            &fields,
            "allocator_custody",
            "closed",
            AllocatorCustody::Closed,
        )?,
        thread_posture: closed_value(
            &fields,
            "thread_posture",
            "single_caller",
            ThreadPosture::SingleCaller,
        )?,
    };
    if unchecked.is_empty() && contracted.is_empty() {
        return Err(ManifestRefusal::NoSymbols);
    }
    let linkage = if generated {
        let kind = match value("linkage").as_str() {
            "static" => LinkageKind::Static,
            "dynamic" => LinkageKind::Dynamic,
            other => {
                return Err(ManifestRefusal::UnknownValue {
                    field: "linkage".to_owned(),
                    value: other.to_owned(),
                });
            }
        };
        let format = value("format");
        if !matches!(format.as_str(), "elf" | "macho" | "coff") {
            return Err(ManifestRefusal::UnknownValue {
                field: "format".to_owned(),
                value: format,
            });
        }
        match (kind, runtime.is_empty()) {
            (LinkageKind::Static, false) => return Err(ManifestRefusal::RuntimeInStaticLinkage),
            (LinkageKind::Dynamic, true) => return Err(ManifestRefusal::MissingRuntime),
            _ => {}
        }
        Some(LinkageSection {
            kind,
            format,
            target: value("target"),
            inputs,
            runtime,
        })
    } else {
        None
    };
    Ok(NativeLibraryManifest {
        identity: ArtifactIdentity {
            library: value("library"),
            version: value("version"),
            artifact: value("artifact"),
            hash,
        },
        premises,
        header,
        unchecked,
        contracted,
        linkage,
    })
}

/// Render a generated (v2) manifest in its one canonical text (card
/// `W48-QRLIB-LINKAGE`). The generator writes exactly this, and a stale check
/// compares a stored manifest with it byte for byte, so the rendering is a
/// function of the manifest alone: fixed statement order, entries by symbol,
/// no timestamp and no host detail. [`load_native_library_manifest`] reads it
/// back to an equal manifest.
///
/// # Panics
///
/// Never for a manifest with a linkage section; `None` is returned for a v1
/// manifest, which is hand-written and has no canonical rendering.
#[must_use]
pub fn render_manifest(manifest: &NativeLibraryManifest) -> Option<String> {
    use std::fmt::Write as _;
    let linkage = manifest.linkage.as_ref()?;
    let mut out = String::new();
    let _ = writeln!(out, "{MARKER_V2};");
    let _ = writeln!(
        out,
        "# Generated on every build from the artifact's bytes and the build's inputs; never edited by hand."
    );
    let _ = writeln!(out, "library {};", manifest.identity.library);
    let _ = writeln!(out, "version {};", manifest.identity.version);
    let _ = writeln!(out, "linkage {};", linkage.kind.spelling());
    let _ = writeln!(out, "format {};", linkage.format);
    let _ = writeln!(out, "target {};", linkage.target);
    let _ = writeln!(out, "artifact {};", manifest.identity.artifact);
    let _ = writeln!(out, "hash {};", manifest.identity.hash);
    for input in &linkage.inputs {
        let _ = writeln!(
            out,
            "input {} {}{};",
            input.reference,
            input.digest,
            if input.as_needed { " as_needed" } else { "" }
        );
    }
    for premise in &linkage.runtime {
        let _ = writeln!(out, "runtime {};", premise.render());
    }
    let _ = writeln!(
        out,
        "data_model {};",
        manifest.premises.data_model.spelling()
    );
    let _ = writeln!(out, "unwinding abort;");
    let _ = writeln!(out, "allocator_custody closed;");
    let _ = writeln!(out, "thread_posture single_caller;");
    if let Some(header) = &manifest.header {
        let _ = writeln!(out, "header {};", header.reference);
        let _ = writeln!(out, "header_hash {};", header.digest);
    }
    for entry in manifest.contracted.values() {
        let _ = writeln!(
            out,
            "contract fn {}({}) -> {} from {} export {};",
            entry.symbol,
            entry
                .parameters
                .iter()
                .map(EntryType::c_spelling)
                .collect::<Vec<_>>()
                .join(", "),
            entry.result.c_spelling(),
            entry.contract_module,
            entry
                .export
                .as_ref()
                .map(ExportFacts::render)
                .unwrap_or_default()
        );
    }
    Some(out)
}

/// Why a *selection* of manifests is rejected, as opposed to one manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionRefusal {
    /// One selected manifest does not load.
    Manifest {
        module: String,
        refusal: ManifestRefusal,
    },
    /// Two selected manifests bind the same symbol. The binding must be a
    /// function: a symbol resolving to two artifacts has no defined premise,
    /// and picking one by selection order would make the premise depend on the
    /// order the builder listed its manifests.
    SymbolBoundTwice {
        symbol: String,
        first: String,
        second: String,
    },
}

impl fmt::Display for SelectionRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest { module, refusal } => {
                write!(f, "the manifest `{module}` does not load: {refusal}")
            }
            Self::SymbolBoundTwice {
                symbol,
                first,
                second,
            } => write!(
                f,
                "the symbol `{symbol}` is bound by both `{first}` and `{second}`; a manifest binding must be a function"
            ),
        }
    }
}

impl std::error::Error for SelectionRefusal {}

/// One contract-less symbol's binding, resolved out of a selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestBinding<'a> {
    /// The manifest module this binding came from.
    pub module: &'a str,
    pub identity: &'a ArtifactIdentity,
    pub premises: &'a AbiPremises,
    /// The exported header the manifest bound, if any (card
    /// `NATIVE-LIBRARY-HEADER`).
    pub header: Option<&'a HeaderIdentity>,
    pub entry: &'a UncheckedEntry,
}

impl ManifestBinding<'_> {
    /// The residence record this binding contributes to a callee. A
    /// contract-less binding names **no** contract module, which is what keeps
    /// a level-L1 premise exactly what card `UNSAFE-LINKAGE-ARTIFACT`
    /// published.
    #[must_use]
    pub fn residence(&self) -> ManifestResidence {
        ManifestResidence {
            identity: self.identity.clone(),
            premises: *self.premises,
            contract: None,
            header: self.header.cloned(),
        }
    }
}

/// One contract slice symbol's binding, resolved out of a selection (card
/// `NATIVE-LIBRARY-CONTRACT-SLICE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContractedBinding<'a> {
    /// The manifest module this binding came from.
    pub module: &'a str,
    pub identity: &'a ArtifactIdentity,
    pub premises: &'a AbiPremises,
    /// The exported header the manifest bound, if any (card
    /// `NATIVE-LIBRARY-HEADER`).
    pub header: Option<&'a HeaderIdentity>,
    pub entry: &'a ContractedEntry,
    /// The generated manifest's linkage section (card `W48-QRLIB-LINKAGE`),
    /// or `None` for a v1 manifest.
    pub linkage: Option<&'a LinkageSection>,
}

impl ContractedBinding<'_> {
    /// The residence record this binding contributes to a **contracted**
    /// callee. It names the contract module, so the premise a certificate
    /// publishes reads *contracted, per this contract, from this build*.
    #[must_use]
    pub fn residence(&self) -> ManifestResidence {
        ManifestResidence {
            identity: self.identity.clone(),
            premises: *self.premises,
            contract: Some(self.entry.contract_module.clone()),
            header: self.header.cloned(),
        }
    }
}

/// An explicit, immutable selection of native-library manifests.
///
/// The empty selection resolves nothing, which is exactly the wave-18 world: a
/// waived declaration no manifest names keeps the bare
/// `UncheckedLinkage { callee, signature }` premise, byte for byte.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeLibraries {
    manifests: BTreeMap<String, NativeLibraryManifest>,
    symbols: BTreeMap<String, String>,
}

impl NativeLibraries {
    /// Load a selection, total or reject. A single manifest that does not load
    /// rejects the selection: a partially loaded selection would silently
    /// downgrade a manifest-resident premise to a bare one, which is a *weaker*
    /// published statement obtained by a mistake.
    pub fn load(sources: &BTreeMap<String, String>) -> Result<Self, SelectionRefusal> {
        let mut manifests = BTreeMap::new();
        let mut symbols: BTreeMap<String, String> = BTreeMap::new();
        for (module, text) in sources {
            let manifest = load_native_library_manifest(text).map_err(|refusal| {
                SelectionRefusal::Manifest {
                    module: module.clone(),
                    refusal,
                }
            })?;
            for symbol in manifest.unchecked.keys().chain(manifest.contracted.keys()) {
                if let Some(first) = symbols.get(symbol) {
                    return Err(SelectionRefusal::SymbolBoundTwice {
                        symbol: symbol.clone(),
                        first: first.clone(),
                        second: module.clone(),
                    });
                }
                symbols.insert(symbol.clone(), module.clone());
            }
            manifests.insert(module.clone(), manifest);
        }
        Ok(Self { manifests, symbols })
    }

    /// Resolve one symbol as a **contract-less** entry, or `None` when no
    /// selected manifest names it that way.
    ///
    /// Named `unchecked_binding` since card `NATIVE-LIBRARY-CONTRACT-SLICE`:
    /// with two entry kinds in the grammar, a single `binding` accessor would
    /// have made "the manifest names this symbol" and "the manifest waives this
    /// symbol" the same question, and they decide different gates.
    #[must_use]
    pub fn unchecked_binding(&self, symbol: &str) -> Option<ManifestBinding<'_>> {
        let module = self.symbols.get(symbol)?;
        let manifest = self.manifests.get(module)?;
        let entry = manifest.unchecked.get(symbol)?;
        Some(ManifestBinding {
            module,
            identity: &manifest.identity,
            premises: &manifest.premises,
            header: manifest.header.as_ref(),
            entry,
        })
    }

    /// Resolve one symbol as a **contract slice** entry, or `None` when no
    /// selected manifest names it that way (card
    /// `NATIVE-LIBRARY-CONTRACT-SLICE`).
    #[must_use]
    pub fn contracted_binding(&self, symbol: &str) -> Option<ContractedBinding<'_>> {
        let module = self.symbols.get(symbol)?;
        let manifest = self.manifests.get(module)?;
        let entry = manifest.contracted.get(symbol)?;
        Some(ContractedBinding {
            module,
            identity: &manifest.identity,
            premises: &manifest.premises,
            header: manifest.header.as_ref(),
            entry,
            linkage: manifest.linkage.as_ref(),
        })
    }

    /// The selection key of the manifest that binds `symbol`, of either entry
    /// kind, or `None` when no selected manifest binds it.
    ///
    /// Card `NATIVE-LIBRARY-SOURCE-BINDING` reads this to tell a symbol *no
    /// manifest claims* — which stays unresident, exactly as before — from one a
    /// manifest this file did not bind claims, which refuses. Absence of
    /// evidence is not evidence, and the two are different absences.
    #[must_use]
    pub fn binding_module(&self, symbol: &str) -> Option<&str> {
        self.symbols.get(symbol).map(String::as_str)
    }

    /// Every library name the selection declares, in sorted order and without
    /// repeats. This is the vocabulary an `extern library NAME;` declaration
    /// draws from, and it is what a refusal names so the repair is visible.
    #[must_use]
    pub fn library_names(&self) -> BTreeSet<&str> {
        self.manifests
            .values()
            .map(|manifest| manifest.identity.library.as_str())
            .collect()
    }

    /// Resolve one source file's `extern library` declarations against this
    /// selection, total or reject (card `NATIVE-LIBRARY-SOURCE-BINDING`).
    ///
    /// `named` is the file's binding list **in the order it declared them**,
    /// spelled exactly as written. The result is a function of the two inputs
    /// and reads nothing else, so a certificate consumer repeats it rather than
    /// believing it — the same property that makes the manifest load and the
    /// slice totality check rederivable.
    ///
    /// An empty list yields the **unbound** view, which resolves selection-wide:
    /// this card is opt-in, and a file that says nothing is unchanged.
    pub fn bind_source(&self, named: &[String]) -> Result<BoundSelection<'_>, BindingRefusal> {
        if named.is_empty() {
            return Ok(BoundSelection::unbound(self));
        }
        let mut bindings = Vec::with_capacity(named.len());
        let mut modules = BTreeSet::new();
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for name in named {
            if !seen.insert(name.as_str()) {
                return Err(BindingRefusal::RepeatedBinding {
                    named: name.clone(),
                });
            }
            let matching = self
                .manifests
                .iter()
                .filter(|(_, manifest)| manifest.identity.library == *name)
                .collect::<Vec<_>>();
            match matching.as_slice() {
                [] => {
                    return Err(BindingRefusal::UnknownLibrary {
                        named: name.clone(),
                        available: self
                            .library_names()
                            .into_iter()
                            .map(ToOwned::to_owned)
                            .collect(),
                    });
                }
                [(module, manifest)] => {
                    bindings.push(SourceLibraryBinding {
                        named: name.clone(),
                        manifest: (*module).clone(),
                        identity: manifest.identity.clone(),
                        premises: manifest.premises,
                        header: manifest.header.clone(),
                    });
                    modules.insert((*module).clone());
                }
                _ => {
                    return Err(BindingRefusal::AmbiguousLibrary {
                        named: name.clone(),
                        manifests: matching
                            .iter()
                            .map(|(module, _)| (*module).clone())
                            .collect(),
                    });
                }
            }
        }
        Ok(BoundSelection {
            libraries: self,
            modules: Some(modules),
            bindings,
        })
    }

    /// Every selected manifest, as `(module, manifest)`, in module order.
    ///
    /// The header level (card `NATIVE-LIBRARY-HEADER`) checks a binding **per
    /// manifest**, because a header is one artifact's exported surface and a
    /// header binding is one manifest's statement about it.
    pub fn manifests(&self) -> impl Iterator<Item = (&str, &NativeLibraryManifest)> {
        self.manifests
            .iter()
            .map(|(module, manifest)| (module.as_str(), manifest))
    }

    /// Every contract slice entry of the selection, as
    /// `(manifest module, entry)`, in manifest-module then symbol order.
    ///
    /// This is the **demand** side of design §4.1's totality: what the
    /// selection asserts about declared contracts, computed from the manifests
    /// alone so the consumer compares it against a loaded environment rather
    /// than believing it.
    #[must_use]
    pub fn contracted_entries(&self) -> Vec<(&str, &ContractedEntry)> {
        self.manifests
            .iter()
            .flat_map(|(module, manifest)| {
                manifest
                    .contracted
                    .values()
                    .map(move |entry| (module.as_str(), entry))
            })
            .collect()
    }

    /// Whether any selected manifest binds a contract slice entry.
    #[must_use]
    pub fn binds_contracts(&self) -> bool {
        self.manifests
            .values()
            .any(|manifest| !manifest.contracted.is_empty())
    }

    /// The contract modules the selection's entries name — the union of every
    /// selected manifest's slice.
    #[must_use]
    pub fn slice(&self) -> BTreeSet<&str> {
        self.manifests
            .values()
            .flat_map(NativeLibraryManifest::slice)
            .collect()
    }

    /// Every symbol this selection binds.
    #[must_use]
    pub fn symbols(&self) -> BTreeSet<&str> {
        self.symbols.keys().map(String::as_str).collect()
    }

    /// Whether the selection binds nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
}

/// One resolved `extern library NAME;` declaration: what a source file bound,
/// and which build that turned out to be (card `NATIVE-LIBRARY-SOURCE-BINDING`).
///
/// This record is what makes the binding say something a certificate can carry.
/// The source file spells a *name*; the selection supplies the *identity*, and
/// publishing both together is the point — a reader of the premise sees the
/// name the author wrote and the `library@hash` it resolved to on the run that
/// produced the certificate, so a later selection that resolves the same name to
/// a different build is a visible difference rather than an invisible one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceLibraryBinding {
    /// The library name as the source file spelled it, compared to a manifest's
    /// own `library` field **exactly** and never normalised (constraint 11's
    /// rule: two spellings that normalise together are two different
    /// selections).
    pub named: String,
    /// The selection key of the manifest that declared that name.
    pub manifest: String,
    /// The artifact identity the binding resolved to. Still a stated pin: this
    /// card resolves a name to a declaration, it opens no file.
    pub identity: ArtifactIdentity,
    /// The ABI premises stated beside that identity.
    pub premises: AbiPremises,
    /// The exported header that manifest bound, if any.
    pub header: Option<HeaderIdentity>,
}

/// Why a source file's `extern library` declarations do not resolve against the
/// selection (card `NATIVE-LIBRARY-SOURCE-BINDING`).
///
/// Every arm refuses; nothing resolves implicitly and nothing falls back. The
/// direction is the one card `UNSAFE-LINKAGE-ARTIFACT` fixed for `Q041`: where
/// the only weaker admissible reading would be *silence about the build*, the
/// disagreement refuses outright rather than buying that silence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingRefusal {
    /// The file binds the same library name twice. A second binding of one name
    /// adds nothing and means nothing; resolving it by taking the first would
    /// make a repeated line silently ignorable.
    RepeatedBinding { named: String },
    /// No selected manifest declares that library name. The refusal names what
    /// *is* selected, because the actionable repair is either the spelling or
    /// the selection and a reader cannot tell which without seeing both.
    UnknownLibrary {
        named: String,
        available: Vec<String>,
    },
    /// Two or more selected manifests declare the same library name, so the
    /// binding would not be a function. Refused on exactly the ground
    /// [`SelectionRefusal::SymbolBoundTwice`] refuses its own ambiguity:
    /// choosing one by selection order would make the premise depend on the
    /// order the builder listed its manifests.
    AmbiguousLibrary {
        named: String,
        manifests: Vec<String>,
    },
    /// This file declares an extern symbol that a selected manifest binds — but
    /// not one **this file** bound.
    ///
    /// A refusal, not a fallback to the bare premise. Falling back would mean a
    /// file could stop its certificate naming a build simply by binding a
    /// different library, which is the `Q041` shape: a contradiction rewarded
    /// with a premise that says less. The repair is to bind the library that
    /// declares the symbol, or to declare neither.
    ForeignSymbol {
        symbol: String,
        manifest: String,
        bound: Vec<String>,
    },
}

impl fmt::Display for BindingRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RepeatedBinding { named } => write!(
                f,
                "this file binds the library `{named}` more than once; a binding is a statement, not a repeatable request"
            ),
            Self::UnknownLibrary { named, available } => {
                if available.is_empty() {
                    write!(
                        f,
                        "this file binds the library `{named}`, but no native-library manifest is selected at all"
                    )
                } else {
                    write!(
                        f,
                        "this file binds the library `{named}`, which no selected manifest declares; the selection declares {}",
                        available
                            .iter()
                            .map(|name| format!("`{name}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            Self::AmbiguousLibrary { named, manifests } => write!(
                f,
                "this file binds the library `{named}`, which {} both declare; a source binding must be a function",
                manifests
                    .iter()
                    .map(|module| format!("`{module}`"))
                    .collect::<Vec<_>>()
                    .join(" and ")
            ),
            Self::ForeignSymbol {
                symbol,
                manifest,
                bound,
            } => write!(
                f,
                "`{symbol}` is declared here and bound by the manifest `{manifest}`, which this file did not bind; this file bound {}. Bind that library or declare neither — a foreign symbol is not silently unresident",
                if bound.is_empty() {
                    "no library".to_owned()
                } else {
                    bound
                        .iter()
                        .map(|name| format!("`{name}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            ),
        }
    }
}

impl std::error::Error for BindingRefusal {}

/// One source file's view of a manifest selection (card
/// `NATIVE-LIBRARY-SOURCE-BINDING`).
///
/// Two states, and the difference between them is the whole card:
///
/// * **unbound** — the file declares no `extern library`. Resolution is
///   selection-wide, exactly as it was before this card: every accepted program
///   written against waves 19–22 travels this state and its certificate is
///   byte-identical.
/// * **bound** — the file declared one or more `extern library NAME;`. Only the
///   named manifests resolve for it, and a symbol some *other* selected manifest
///   binds is a [`BindingRefusal::ForeignSymbol`] rather than a symbol that
///   quietly stops being manifest-resident.
///
/// The view is per file by construction: it is built from one file's own
/// binding list and holds no cross-file state, which is the constraint-13
/// discipline (importing another compilation input grants this file nothing)
/// applied to a binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundSelection<'a> {
    libraries: &'a NativeLibraries,
    /// The manifest modules this file bound, or `None` for an unbound file.
    /// `None` and `Some(empty)` are deliberately different: the first is "this
    /// file made no statement", the second cannot arise, because a file that
    /// declared a binding at all resolved at least one.
    modules: Option<BTreeSet<String>>,
    bindings: Vec<SourceLibraryBinding>,
}

impl<'a> BoundSelection<'a> {
    /// The unbound view: resolution is selection-wide, byte for byte as before
    /// this card.
    #[must_use]
    pub fn unbound(libraries: &'a NativeLibraries) -> Self {
        Self {
            libraries,
            modules: None,
            bindings: Vec::new(),
        }
    }

    /// Whether this file declared a binding at all.
    #[must_use]
    pub fn is_bound(&self) -> bool {
        self.modules.is_some()
    }

    /// The resolved bindings, in the order the file declared them. Empty for an
    /// unbound file.
    #[must_use]
    pub fn bindings(&self) -> &[SourceLibraryBinding] {
        &self.bindings
    }

    /// Whether `module` is one this file resolves against.
    fn visible(&self, module: &str) -> bool {
        match &self.modules {
            None => true,
            Some(modules) => modules.contains(module),
        }
    }

    /// Resolve one symbol as a contract-less entry **within this file's
    /// binding**, or `None`.
    #[must_use]
    pub fn unchecked_binding(&self, symbol: &str) -> Option<ManifestBinding<'a>> {
        let binding = self.libraries.unchecked_binding(symbol)?;
        self.visible(binding.module).then_some(binding)
    }

    /// Resolve one symbol as a contract slice entry **within this file's
    /// binding**, or `None`.
    #[must_use]
    pub fn contracted_binding(&self, symbol: &str) -> Option<ContractedBinding<'a>> {
        let binding = self.libraries.contracted_binding(symbol)?;
        self.visible(binding.module).then_some(binding)
    }

    /// The refusal owed for a symbol some selected manifest binds and this file
    /// did not bind, or `None` when there is nothing foreign about it.
    ///
    /// An unbound file is never owed one: it made no statement to contradict,
    /// and that is what keeps every pre-card program unmoved.
    #[must_use]
    pub fn foreign_symbol(&self, symbol: &str) -> Option<BindingRefusal> {
        let modules = self.modules.as_ref()?;
        let module = self.libraries.binding_module(symbol)?;
        if modules.contains(module) {
            return None;
        }
        Some(BindingRefusal::ForeignSymbol {
            symbol: symbol.to_owned(),
            manifest: module.to_owned(),
            bound: self
                .bindings
                .iter()
                .map(|binding| binding.named.clone())
                .collect(),
        })
    }
}

/// How a declaration and the manifest entry that names it disagree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManifestDisagreement {
    ParameterCount {
        declared: usize,
        manifest: usize,
    },
    Parameter {
        parameter: usize,
        declared: AbiType,
        manifest: AbiType,
    },
    Result {
        declared: AbiType,
        manifest: AbiType,
    },
    /// The entry carries a struct (card `W48-QRLIB-LINKAGE`), which only the
    /// FFI route calls.
    StructEntry {
        symbol: String,
    },
}

impl fmt::Display for ManifestDisagreement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParameterCount { declared, manifest } => write!(
                f,
                "the declaration takes {declared} parameter(s) and the manifest declares {manifest}"
            ),
            Self::Parameter {
                parameter,
                declared,
                manifest,
            } => write!(
                f,
                "parameter {parameter} is declared `{}` here and `{}` in the manifest",
                declared.c_spelling(),
                manifest.c_spelling()
            ),
            Self::Result { declared, manifest } => write!(
                f,
                "the result is declared `{}` here and `{}` in the manifest",
                declared.c_spelling(),
                manifest.c_spelling()
            ),
            Self::StructEntry { symbol } => write!(
                f,
                "the manifest declares `{symbol}` with a C struct, which only `source.ffi.v1` calls"
            ),
        }
    }
}

impl std::error::Error for ManifestDisagreement {}

/// The type gate of design §4.2, at the setting a contract-less entry admits:
/// the waived declaration's ABI projection must equal the manifest's exported
/// header declaration, or the symbol is refused.
///
/// **Refusal, not withdrawal.** The contracted route withdraws a disagreeing
/// declaration for the unit and lets the call sites fall through to the
/// uncontracted refusal (`LOADER-ABI`, constraint 8), because there a weaker
/// admissible reading exists and it refuses. Here the only weaker reading is the
/// *bare waiver*, and falling back to it would reward a contradiction with a
/// premise that says less — a disagreement would buy silence about the artifact.
/// So the disagreement refuses outright, which is the monotone direction.
pub fn check_manifest_agreement(
    entry: &UncheckedEntry,
    declared_parameters: &[AbiType],
    declared_result: AbiType,
) -> Result<(), ManifestDisagreement> {
    check_abi_agreement(
        &entry.parameters,
        entry.result,
        declared_parameters,
        declared_result,
    )
}

/// The same type gate at the **contract slice** setting (card
/// `NATIVE-LIBRARY-CONTRACT-SLICE`): the declared contract's own ABI
/// projection, which the consuming unit's `extern fn` declaration has already
/// been compared against (`Q027`), must equal the manifest's exported header
/// declaration.
///
/// **Withdrawal, not refusal — and the direction is the argument.** Both cards
/// apply one rule: *withdraw only when the weaker admissible reading refuses.*
/// At the contract-less setting the weaker reading below a withdrawn binding is
/// the bare waiver, which is **accepted**, so withdrawing there would reward a
/// contradiction with a premise that says less and card
/// `UNSAFE-LINKAGE-ARTIFACT` refuses outright. Here the weaker reading is the
/// **uncontracted callee**, which *refuses* (`Q023`), so withdrawal is already
/// the monotone direction and `LOADER-ABI`'s per-unit withdrawal (constraint 8)
/// is exactly the right shape: the contradicted declaration is withdrawn for the
/// analysed unit, every call site becomes an uncontracted callee, and nothing is
/// silently reinterpreted.
///
/// Deciding *which* is a property of this function's callers, not of this
/// comparison, which is why the comparison itself is shared.
pub fn check_contracted_agreement(
    entry: &ContractedEntry,
    declared_parameters: &[AbiType],
    declared_result: AbiType,
) -> Result<(), ManifestDisagreement> {
    // Card `W48-QRLIB-LINKAGE`: an entry carrying a struct is the FFI route's;
    // an ABI-only declaration never agrees with it.
    let Some((parameters, result)) = entry.abi_signature() else {
        return Err(ManifestDisagreement::StructEntry {
            symbol: entry.symbol.clone(),
        });
    };
    check_abi_agreement(&parameters, result, declared_parameters, declared_result)
}

/// The FFI route's type gate (card `W48-QRLIB-LINKAGE`): the callee's C
/// signature, struct positions included, must equal the generated entry's.
///
/// # Errors
///
/// The first disagreement.
pub fn check_ffi_agreement(
    entry: &ContractedEntry,
    declared_parameters: &[EntryType],
    declared_result: &EntryType,
) -> Result<(), FfiEntryDisagreement> {
    if declared_parameters.len() != entry.parameters.len() {
        return Err(FfiEntryDisagreement {
            position: "the parameter count".to_owned(),
            declared: declared_parameters.len().to_string(),
            manifest: entry.parameters.len().to_string(),
        });
    }
    for (index, (declared, manifest)) in declared_parameters
        .iter()
        .zip(&entry.parameters)
        .enumerate()
    {
        if declared != manifest {
            return Err(FfiEntryDisagreement {
                position: format!("parameter {index}"),
                declared: declared.c_spelling(),
                manifest: manifest.c_spelling(),
            });
        }
    }
    if *declared_result != entry.result {
        return Err(FfiEntryDisagreement {
            position: "the result".to_owned(),
            declared: declared_result.c_spelling(),
            manifest: entry.result.c_spelling(),
        });
    }
    Ok(())
}

/// How an FFI callee and its generated entry disagree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FfiEntryDisagreement {
    pub position: String,
    pub declared: String,
    pub manifest: String,
}

impl fmt::Display for FfiEntryDisagreement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} is `{}` in the contract and `{}` in the manifest",
            self.position, self.declared, self.manifest
        )
    }
}

impl std::error::Error for FfiEntryDisagreement {}

/// The one ABI comparison both entry kinds use, so the two gates cannot drift
/// in what "agree" means — only in what a disagreement does.
fn check_abi_agreement(
    manifest_parameters: &[AbiType],
    manifest_result: AbiType,
    declared_parameters: &[AbiType],
    declared_result: AbiType,
) -> Result<(), ManifestDisagreement> {
    if declared_parameters.len() != manifest_parameters.len() {
        return Err(ManifestDisagreement::ParameterCount {
            declared: declared_parameters.len(),
            manifest: manifest_parameters.len(),
        });
    }
    for (index, (declared, manifest)) in declared_parameters
        .iter()
        .zip(manifest_parameters.iter())
        .enumerate()
    {
        if declared != manifest {
            return Err(ManifestDisagreement::Parameter {
                parameter: index,
                declared: *declared,
                manifest: *manifest,
            });
        }
    }
    if declared_result != manifest_result {
        return Err(ManifestDisagreement::Result {
            declared: declared_result,
            manifest: manifest_result,
        });
    }
    Ok(())
}

/// Why a manifest selection's **contract slice binding** does not resolve
/// against the selected contract environment (design §4.1, card
/// `NATIVE-LIBRARY-CONTRACT-SLICE`).
///
/// This is a *load-time* rejection of the whole selection, not a per-call-site
/// decision: an entry whose contract does not resolve leaves the manifest
/// stating something about a build that the environment does not corroborate,
/// and a partially resolved slice would silently narrow what the manifest was
/// selected to attribute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SliceRefusal {
    /// The selection binds contracts but no contract environment is selected at
    /// all, so no entry can resolve.
    NoContractEnvironment { manifest: String, symbol: String },
    /// The environment declares no contract for the entry's symbol.
    UnresolvedContract {
        manifest: String,
        symbol: String,
        module: String,
    },
    /// The environment declares the symbol, but in a different module than the
    /// entry names. The reference is compared exactly: a contract in another
    /// module is another contract.
    ContractInAnotherModule {
        manifest: String,
        symbol: String,
        entry_module: String,
        declared_module: String,
    },
    /// §4.4's **closed allocator custody**, structurally: a manifest binds a
    /// contract that establishes fresh custody and binds none that discharges
    /// it. Such a library could only ever leak or lie — an allocation it hands
    /// out has no declared release entry point, and the premise
    /// `allocator_custody closed` would be stating something its own slice
    /// contradicts.
    ///
    /// The check was **vacuous** while every entry was contract-less (a waiver
    /// cannot acquire), and card `NATIVE-LIBRARY-CONTRACT-SLICE` is where it
    /// becomes real. Design §4.4 says "for the same resource shape"; §C04
    /// projects every resource to one shape (an extentless `u8` buffer), so the
    /// shape match is vacuous in v1 and this compares the slice as a whole —
    /// recorded rather than silently narrowed.
    OpenAllocatorCustody { manifest: String, acquiring: String },
    /// A contract declared in a module the slice names is bound by **no** entry
    /// — design §4.1's other direction. The slice is the library's surface, so
    /// a declaration inside it that names no exported symbol makes the binding
    /// partial, and a partial binding is not one.
    UnboundContract { module: String, symbol: String },
}

impl fmt::Display for SliceRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoContractEnvironment { manifest, symbol } => write!(
                f,
                "the manifest `{manifest}` binds `{symbol}` to a declared contract, but no extern contract environment is selected"
            ),
            Self::UnresolvedContract {
                manifest,
                symbol,
                module,
            } => write!(
                f,
                "the manifest `{manifest}` binds `{symbol}` to a contract in `{module}`, which the selected environment does not declare"
            ),
            Self::ContractInAnotherModule {
                manifest,
                symbol,
                entry_module,
                declared_module,
            } => write!(
                f,
                "the manifest `{manifest}` binds `{symbol}` to a contract in `{entry_module}`, but the selected environment declares it in `{declared_module}`"
            ),
            Self::OpenAllocatorCustody {
                manifest,
                acquiring,
            } => write!(
                f,
                "the manifest `{manifest}` declares `allocator_custody closed` and binds `{acquiring}`, which establishes fresh custody, but binds no contract that consumes or releases it; such a slice could only leak or lie"
            ),
            Self::UnboundContract { module, symbol } => write!(
                f,
                "the contract module `{module}` is in a manifest's slice but its declaration of `{symbol}` is bound by no manifest entry; the binding must be total in both directions"
            ),
        }
    }
}

impl std::error::Error for SliceRefusal {}

/// What one bound contract does to custody, projected to the two facts §4.4's
/// closed-allocator-custody check reads.
///
/// A projection rather than the contract itself, because this crate has no
/// dependencies at all: the *reading* of a C04 declaration belongs to the
/// contract machinery, and the *decision* belongs here, where it is a function
/// of its inputs and repeatable at a certificate recheck.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CustodyShape {
    /// The contract establishes fresh custody — an owned or reallocated result,
    /// or a declared out slot.
    pub acquires: bool,
    /// The contract moves the obligation off the caller's ledger for good — a
    /// consumption or a declared release. A `resource_stored` parameter does
    /// **not** count: it relocates custody beneath a declared owner and
    /// discharges nothing, which is exactly the premise
    /// `StoredCustodyUnderDeclaredOwner` records as undischarged.
    pub discharges: bool,
}

/// Check design §4.4's **closed allocator custody**, structurally: a manifest
/// whose slice can establish custody must also declare somewhere to discharge
/// it.
///
/// `shapes` maps every bound symbol to what its declared contract does. A symbol
/// absent from the map contributes nothing, which is fail-closed in the right
/// direction here: an unknown shape cannot supply the discharge a manifest is
/// missing.
pub fn check_closed_allocator_custody(
    libraries: &NativeLibraries,
    shapes: &BTreeMap<String, CustodyShape>,
) -> Result<(), SliceRefusal> {
    let mut per_manifest: BTreeMap<&str, Vec<&ContractedEntry>> = BTreeMap::new();
    for (manifest, entry) in libraries.contracted_entries() {
        per_manifest.entry(manifest).or_default().push(entry);
    }
    for (manifest, entries) in per_manifest {
        let discharges = entries
            .iter()
            .any(|entry| shapes.get(&entry.symbol).is_some_and(|s| s.discharges));
        if discharges {
            continue;
        }
        if let Some(acquiring) = entries
            .iter()
            .find(|entry| shapes.get(&entry.symbol).is_some_and(|s| s.acquires))
        {
            return Err(SliceRefusal::OpenAllocatorCustody {
                manifest: manifest.to_owned(),
                acquiring: acquiring.symbol.clone(),
            });
        }
    }
    Ok(())
}

/// Check design §4.1's binding totality, in **both** directions, between a
/// manifest selection's contract slice entries and a selected contract
/// environment.
///
/// `declared` maps every contract the environment declares to the module that
/// declares it. Passing `None` means no environment is selected at all.
///
/// The check is a function of its two inputs and reads nothing else, so a
/// certificate consumer repeats it rather than believing it.
pub fn check_slice_totality(
    libraries: &NativeLibraries,
    declared: Option<&BTreeMap<String, String>>,
) -> Result<(), SliceRefusal> {
    let entries = libraries.contracted_entries();
    if entries.is_empty() {
        return Ok(());
    }
    let Some(declared) = declared else {
        let (manifest, entry) = entries[0];
        return Err(SliceRefusal::NoContractEnvironment {
            manifest: manifest.to_owned(),
            symbol: entry.symbol.clone(),
        });
    };
    // Direction one: every entry resolves to a contract, in the module it names.
    for (manifest, entry) in &entries {
        let Some(module) = declared.get(&entry.symbol) else {
            return Err(SliceRefusal::UnresolvedContract {
                manifest: (*manifest).to_owned(),
                symbol: entry.symbol.clone(),
                module: entry.contract_module.clone(),
            });
        };
        if *module != entry.contract_module {
            return Err(SliceRefusal::ContractInAnotherModule {
                manifest: (*manifest).to_owned(),
                symbol: entry.symbol.clone(),
                entry_module: entry.contract_module.clone(),
                declared_module: module.clone(),
            });
        }
    }
    // Direction two: every contract of every module in the slice is bound.
    let slice = libraries.slice();
    let bound: BTreeSet<&str> = entries
        .iter()
        .map(|(_, entry)| entry.symbol.as_str())
        .collect();
    for (symbol, module) in declared {
        if slice.contains(module.as_str()) && !bound.contains(symbol.as_str()) {
            return Err(SliceRefusal::UnboundContract {
                module: module.clone(),
                symbol: symbol.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/native_library.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/unit/native_library_v2.rs"]
mod v2_tests;
