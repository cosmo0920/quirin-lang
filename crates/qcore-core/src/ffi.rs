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

//! `source.ffi.v1` — a `.qr` program importing a C struct and calling C through
//! `#c_ffi`-marked contracts (card `W48-QR-C-STRUCT-IMPORT`, card 1 of the
//! `.qr` language track).
//!
//! # What is decided here
//!
//! * [`check_layout`]: an imported struct's **listed** fields against the
//!   layout the recorded compiler reported for that struct
//!   ([`CStructFacts`], derived by `qcore_c_gateway::ffi_layout`). Every listed
//!   field must be a direct member, of a C type whose canonical type is the
//!   canonical type of the field's §G02t spelling under the same compiler, and
//!   the listed order must be the members' offset order. Nothing about the
//!   layout is declared by hand: the listing names fields and types only, and
//!   the offsets, the size and the alignment are the compiler's.
//! * [`check_ffi_program`]: the entry's statements — construction, field reads
//!   and (card `W48-QR-USABLE-RECORDS`) field writes and copies of imported
//!   records, over the listed fields only; calls through admitted callees;
//!   calls of the file's `.qr` helper functions, which take and return
//!   imported records by value and are checked at every call; and rendered
//!   lines — with the reference observation and the certificate.
//! * [`recheck_ffi_certificate`]: the certificate rederived and compared.
//!
//! # The trust mark
//!
//! A program on this route is **unsafe as a whole file** (user decision
//! 2026-09-30). That is not the `Unsafe` verdict, which is reserved for a
//! validated reachable violation: it is a named assumption,
//! [`FfiAssumption::UnsafeFfi`], that the imported C layouts are faithful and
//! that each `#c_ffi` contract function behaves as its contract states. Every
//! certificate carries it, listing every imported struct and every marked
//! function the entry calls, and [`certificate_carries_ffi_assumption`] is the
//! publication tripwire the driver runs before publishing.
//!
//! # Linkage
//!
//! Card `W48-QRLIB-LINKAGE`: every called C function carries an [`FfiLink`] —
//! the exported symbol, the artifact (or link-time stub) and its sha256, the
//! runtime premise of a dynamic link — resolved by the driver from the
//! generated `.qrlib` the file binds. A called function without one refuses
//! (`Q066`), [`certificate_carries_linkage`] is the publication tripwire
//! (`Q067`), and the premise stacks beside the unsafe-FFI assumption, never in
//! place of it: it says which code is linked, not that the code is safe.
//!
//! # Unlisted fields are opaque
//!
//! A member the `fields { … }` list does not name is **opaque bytes**: no
//! statement can read or write it. A construction zeroes the whole object
//! before storing the listed fields, so an opaque byte of a constructed record
//! is zero; a record a C function returned carries whatever the callee left
//! there, and the program cannot observe it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::scalar_types::{self as st, RecordedTarget, ScalarType};
use crate::std_io::{ObservedLine, Rendering, StdIoEvent, StdIoOperand, Stream};
use crate::typed_errors::{Effect, EffectRow};

/// The route name.
pub const PROFILE: &str = "source.ffi.v1";

/// The bundled import that enables the route (user decision 1).
pub const FFI_MODULE: &str = "std.ffi.unsafe";

/// The name of the file-wide assumption every result of this route carries.
pub const UNSAFE_FFI_ASSUMPTION: &str = "unsafe-ffi";

/// The attribute that marks a contract function as an FFI function (user
/// decision 4), in its one spelling.
pub const C_FFI_MARKER: &str = "[| quant #c_ffi |]";

/// Read the text inside `c"…"`: exactly `struct` + one space + a C
/// identifier. Every other spelling — a union, a typedef name, an enum, extra
/// whitespace — is not an imported struct tag.
#[must_use]
pub fn parse_c_struct_spelling(text: &str) -> Option<&str> {
    let tag = text.strip_prefix("struct ")?;
    let mut chars = tag.chars();
    let first = chars.next()?;
    ((first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()))
    .then_some(tag)
}

/// One `struct! { … }` declaration as the source wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedStruct {
    /// The `.qr` type name.
    pub name: String,
    /// The C tag: `c"struct TAG"`.
    pub tag: String,
    /// The header reference exactly as written after `from`.
    pub header: String,
    /// The listed fields, in listed order.
    pub fields: Vec<(String, ScalarType)>,
}

/// One direct member of a struct, as the recorded compiler reported it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CFieldFact {
    pub name: String,
    /// Byte offset, from the compiler's record layout.
    pub offset: u64,
    /// The member's type as written (`uint32_t`), from the typed AST.
    pub written_type: String,
    /// Its canonical type (`unsigned int`), from the typed AST.
    pub canonical_type: String,
    /// Every §G02t type whose C11 spelling has this canonical type under the
    /// same compiler (`int64_t` and `ptrdiff_t` are both `long` on LP64, so a
    /// `long` member is either), in table order; empty when none is — a
    /// `long long`, a `char`, a pointer, an array, an enum or a struct.
    pub scalars: Vec<ScalarType>,
}

/// What the recorded compiler reported for one `struct TAG`: the facts
/// `qcore_c_gateway::ffi_layout` derives, never a hand-written layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CStructFacts {
    pub tag: String,
    pub size: u64,
    pub align: u64,
    /// Every direct member, in declaration order.
    pub fields: Vec<CFieldFact>,
    /// The header reference as written.
    pub header: String,
    /// The path the reference resolved to, and the directory the emitted C
    /// must be compiled with on its include path.
    pub header_path: String,
    pub header_directory: String,
    /// SHA-256 of the header bytes the compiler read.
    pub header_sha256: String,
    /// The recorded compiler: the first line of its `--version`, and its
    /// `-dumpmachine` target, each equal to the recorded target's.
    pub compiler: String,
    pub triple: String,
}

/// A listed field after the layout check: its type and the recorded offset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedField {
    pub name: String,
    pub ty: ScalarType,
    pub offset: u64,
    pub canonical_type: String,
}

/// An imported struct whose listing agrees with the recorded compiler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedStruct {
    pub name: String,
    pub tag: String,
    pub header: String,
    pub listed: Vec<ListedField>,
    /// The members the listing leaves opaque, in declaration order.
    pub opaque: Vec<String>,
    pub facts: CStructFacts,
}

impl CheckedStruct {
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&ListedField> {
        self.listed.iter().find(|field| field.name == name)
    }
}

/// Why a listing disagrees with the recorded compiler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutRefusal {
    /// The facts describe another tag than the one imported.
    TagMismatch { imported: String, reported: String },
    /// A listed field is not a direct member of the struct.
    UnknownMember { field: String },
    /// A listed field's C type has no §G02t equivalent (`Q063`: a type the
    /// reader reads and the `.qr` vocabulary cannot spell).
    NoEquivalent { field: String, c_type: String },
    /// A listed field's C type is another §G02t type's.
    TypeMismatch {
        field: String,
        listed: ScalarType,
        c_type: String,
        reported: Vec<ScalarType>,
    },
    /// A listed field lies at a lower offset than the field listed before it.
    OffsetOrder {
        field: String,
        offset: u64,
        previous: String,
        previous_offset: u64,
    },
    /// A reported member does not fit its object (a malformed fact).
    OutOfBounds { field: String },
}

impl LayoutRefusal {
    /// `Q063` for a type the vocabulary cannot spell, `Q064` for every
    /// disagreement between the listing and the compiler (§L21).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoEquivalent { .. } => "Q063",
            _ => "Q064",
        }
    }
}

impl fmt::Display for LayoutRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TagMismatch { imported, reported } => write!(
                f,
                "the recorded compiler's layout is of `struct {reported}`, not the imported `struct {imported}`"
            ),
            Self::UnknownMember { field } => write!(
                f,
                "`{field}` is not a member of the imported struct as the recorded compiler reads it"
            ),
            Self::NoEquivalent { field, c_type } => write!(
                f,
                "`{field}` is `{c_type}` in C, which has no §G02t equivalent; a field of that type can only stay unlisted, as opaque bytes"
            ),
            Self::TypeMismatch {
                field,
                listed,
                c_type,
                reported,
            } => write!(
                f,
                "`{field}` is listed `{listed}`, but the recorded compiler reads `{c_type}` there, which is {}",
                reported
                    .iter()
                    .map(|ty| format!("`{ty}`"))
                    .collect::<Vec<_>>()
                    .join(" or ")
            ),
            Self::OffsetOrder {
                field,
                offset,
                previous,
                previous_offset,
            } => write!(
                f,
                "`{field}` is listed after `{previous}`, but the recorded compiler places it at offset {offset}, before `{previous}` at offset {previous_offset}; the listing names fields in their layout order and states no offset of its own"
            ),
            Self::OutOfBounds { field } => write!(
                f,
                "the recorded layout places `{field}` outside its object, so the facts are not a layout"
            ),
        }
    }
}

/// Check one listing against the recorded compiler's facts.
///
/// # Errors
///
/// The first disagreement, in listed order.
pub fn check_layout(
    import: &ImportedStruct,
    facts: &CStructFacts,
    target: Option<&RecordedTarget>,
) -> Result<CheckedStruct, LayoutRefusal> {
    if facts.tag != import.tag {
        return Err(LayoutRefusal::TagMismatch {
            imported: import.tag.clone(),
            reported: facts.tag.clone(),
        });
    }
    let mut listed = Vec::new();
    let mut previous: Option<(&str, u64)> = None;
    for (name, ty) in &import.fields {
        let Some(fact) = facts.fields.iter().find(|fact| &fact.name == name) else {
            return Err(LayoutRefusal::UnknownMember {
                field: name.clone(),
            });
        };
        if fact.scalars.is_empty() {
            return Err(LayoutRefusal::NoEquivalent {
                field: name.clone(),
                c_type: fact.written_type.clone(),
            });
        }
        if !fact.scalars.contains(ty) {
            return Err(LayoutRefusal::TypeMismatch {
                field: name.clone(),
                listed: *ty,
                c_type: fact.written_type.clone(),
                reported: fact.scalars.clone(),
            });
        }
        let size = ty.size_bytes(target).unwrap_or(u64::MAX);
        if fact
            .offset
            .checked_add(size)
            .is_none_or(|end| end > facts.size)
        {
            return Err(LayoutRefusal::OutOfBounds {
                field: name.clone(),
            });
        }
        if let Some((previous_name, previous_offset)) = previous
            && fact.offset < previous_offset
        {
            return Err(LayoutRefusal::OffsetOrder {
                field: name.clone(),
                offset: fact.offset,
                previous: previous_name.to_owned(),
                previous_offset,
            });
        }
        previous = Some((name, fact.offset));
        listed.push(ListedField {
            name: name.clone(),
            ty: *ty,
            offset: fact.offset,
            canonical_type: fact.canonical_type.clone(),
        });
    }
    let opaque = facts
        .fields
        .iter()
        .filter(|fact| !import.fields.iter().any(|(name, _)| *name == fact.name))
        .map(|fact| fact.name.clone())
        .collect();
    Ok(CheckedStruct {
        name: import.name.clone(),
        tag: import.tag.clone(),
        header: import.header.clone(),
        listed,
        opaque,
        facts: facts.clone(),
    })
}

/// A type in a `#c_ffi` contract signature, in C terms.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CAbiType {
    Unit,
    Scalar(ScalarType),
    /// `c"struct TAG"`, by value.
    Struct(String),
    /// `ptr<const, c"struct TAG">`.
    ConstStructPointer(String),
}

impl fmt::Display for CAbiType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Scalar(ty) => write!(f, "{ty}"),
            Self::Struct(tag) => write!(f, "c\"struct {tag}\""),
            Self::ConstStructPointer(tag) => write!(f, "ptr<const, c\"struct {tag}\">"),
        }
    }
}

impl CAbiType {
    /// Whether this position carries an imported struct.
    #[must_use]
    pub fn carries_struct(&self) -> bool {
        matches!(self, Self::Struct(_) | Self::ConstStructPointer(_))
    }
}

/// A type in a `.qr` extern declaration on this route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FfiType {
    Unit,
    Scalar(ScalarType),
    /// An imported record, by value.
    Record(String),
    /// `ptr<const, NAME>` over an imported record.
    ConstRecordPointer(String),
}

impl fmt::Display for FfiType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Scalar(ty) => write!(f, "{ty}"),
            Self::Record(name) => write!(f, "{name}"),
            Self::ConstRecordPointer(name) => write!(f, "ptr<const, {name}>"),
        }
    }
}

/// An admitted callee: the `.qr` declaration agreed with its selected
/// contract, which is `#c_ffi`-marked whenever a position carries a struct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FfiCallee {
    pub name: String,
    /// The contract module the declaration was selected from.
    pub module: String,
    pub parameters: Vec<(String, FfiType)>,
    pub result: FfiType,
    /// Whether the contract carries `[| quant #c_ffi |]`.
    pub marked: bool,
}

impl FfiCallee {
    #[must_use]
    pub fn signature(&self) -> String {
        format!(
            "({}) -> {}",
            self.parameters
                .iter()
                .map(|(name, ty)| format!("{name}: {ty}"))
                .collect::<Vec<_>>()
                .join(", "),
            self.result
        )
    }
}

/// An operand written at a construction, an argument or a rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FfiValue {
    /// An integer literal, `-` included.
    Literal(i128),
    /// A float literal, `-` included (card `W48-QR-FLOATING-POINT`): read once
    /// at the float type of the position it stands at, never at an integer
    /// one.
    Float(crate::scalar_floats::FloatLiteral),
    /// `true` / `false`.
    Bool(bool),
    /// A name this entry bound.
    Name(String),
}

/// A declared outcome after `succeeds`: the number a scalar result carries,
/// or the listed fields of a returned record. The emitted C compares the real
/// result and exits 70 on disagreement, before anything reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FfiOutcome {
    Scalar(FfiValue),
    Record {
        record: String,
        fields: Vec<(String, FfiValue)>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FfiStatement {
    /// `let NAME = TYPE { FIELD: VALUE, … };` over every listed field.
    Construct {
        binding: String,
        record: String,
        fields: Vec<(String, FfiValue)>,
    },
    /// `let NAME = RECORD.FIELD;`
    Read {
        binding: String,
        source: String,
        field: String,
    },
    /// `[let NAME =] CALLEE(ARGS) [succeeds OUTCOME];`
    Call {
        binding: Option<String>,
        callee: String,
        arguments: Vec<FfiValue>,
        outcome: Option<FfiOutcome>,
    },
    /// `print`, `log` or `flush`, read by §G02g's own reader.
    Line(StdIoEvent),
    /// `return;` — or, in a helper function returning a value (card
    /// `W48-QR-USABLE-RECORDS`), `return VALUE;`.
    Return(Option<FfiValue>),
    /// `RECORD.FIELD = VALUE;` — **the field write** (card
    /// `W48-QR-USABLE-RECORDS`): a **listed** field only, the value of
    /// exactly the field's type. An opaque member is never written.
    Write {
        record: String,
        field: String,
        value: FfiValue,
    },
    /// `let NAME = SOURCE;` — a copy of a record or a scalar (card
    /// `W48-QR-USABLE-RECORDS`). An imported record is plain data on this
    /// route, so a copy is a second value, not a second owner.
    Copy { binding: String, source: String },
}

/// A `.qr` helper function of an FFI file (card `W48-QR-USABLE-RECORDS`,
/// `W48-QR-FFI-ROUTE-SCOPE`): parameters and a result of §G02t scalars or
/// imported records, **by value**, and a body of this route's own statements.
/// It is checked at every call from the entry, with the argument values of
/// that call, so a helper nothing calls is never checked and is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FfiFunction {
    pub name: String,
    /// Each parameter is [`FfiType::Scalar`] or [`FfiType::Record`].
    pub parameters: Vec<(String, FfiType)>,
    /// [`FfiType::Unit`], a scalar or a record.
    pub result: FfiType,
    pub statements: Vec<FfiStatement>,
    /// `None` infers the row; an explicit row must admit what the body does.
    pub declared_effects: Option<EffectRow>,
    /// The global statement index of this function's first statement, so a
    /// refusal inside it is attributed to its own source statement.
    pub base: usize,
}

impl FfiFunction {
    #[must_use]
    pub fn signature(&self) -> String {
        format!(
            "fn {}({}) -> {}",
            self.name,
            self.parameters
                .iter()
                .map(|(name, ty)| format!("{name}: {ty}"))
                .collect::<Vec<_>>()
                .join(", "),
            self.result
        )
    }
}

/// The deepest chain of helper calls this route checks (card
/// `W48-QR-USABLE-RECORDS`). Calls are acyclic, so the bound is the only
/// thing between a long chain and the checker's own stack.
pub const MAX_CALL_DEPTH: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FfiProgram {
    pub entry: String,
    pub structs: Vec<CheckedStruct>,
    pub callees: BTreeMap<String, FfiCallee>,
    pub statements: Vec<FfiStatement>,
    pub declared_effects: EffectRow,
    /// Required: the recorded compiler is the one whose layout was read.
    pub target: RecordedTarget,
    /// The `.qr` helper functions (card `W48-QR-USABLE-RECORDS`). Empty for
    /// every program written before it.
    pub functions: Vec<FfiFunction>,
    /// How each admitted callee is linked, from the generated manifest its
    /// file binds (card `W48-QRLIB-LINKAGE`). A called function with no link
    /// refuses (`Q066`).
    pub links: BTreeMap<String, FfiLink>,
}

/// How one callee is linked (card `W48-QRLIB-LINKAGE`): symbol X from artifact
/// Y@sha256 (static), or stub Y@sha256 plus runtime premise R (dynamic), under
/// ABI premises Z. This says **which code** is linked, never that it is safe.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FfiLink {
    pub callee: String,
    /// The `extern library NAME;` spelling that bound it.
    pub named: String,
    /// The manifest's selection key.
    pub manifest: String,
    pub library: String,
    pub version: String,
    pub kind: crate::native_library::LinkageKind,
    pub target: String,
    /// The file the link names, and its sha256.
    pub artifact: String,
    pub hash: String,
    /// Every file an `ld` script artifact names, with its sha256.
    pub inputs: Vec<(String, String)>,
    pub export: crate::native_library::ExportFacts,
    /// The runtime library's identity, for a dynamic link.
    pub runtime: Vec<crate::native_library::RuntimePremise>,
    /// The ABI premises, as the manifest states them.
    pub data_model: crate::native_library::DataModel,
}

impl fmt::Display for FfiLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::native_library::{ExportBinding, LinkageKind, RuntimePremise};
        let export = &self.export;
        let mut facts = vec![match export.binding {
            ExportBinding::Global => "global".to_owned(),
            ExportBinding::Weak => "weak".to_owned(),
        }];
        if let Some(version) = &export.version {
            facts.push(format!("version {version}"));
        }
        if export.ifunc {
            facts.push("ifunc".to_owned());
        }
        write!(
            f,
            "linked: `{}` is the symbol `{}` ({}) defined in `{}`",
            self.callee,
            export.symbol,
            facts.join(", "),
            export.file
        )?;
        match self.kind {
            LinkageKind::Static => write!(
                f,
                ", from the static artifact `{}` at {}",
                self.artifact, self.hash
            )?,
            LinkageKind::Dynamic => write!(
                f,
                ", through the link-time stub `{}` at {}",
                self.artifact, self.hash
            )?,
        }
        if !self.inputs.is_empty() {
            write!(
                f,
                " (an `ld` script naming {})",
                self.inputs
                    .iter()
                    .map(|(reference, digest)| format!("`{reference}` at {digest}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }
        write!(
            f,
            ", of library `{} {}` for `{}`, bound by `extern library {}` through the generated manifest `{}`",
            self.library, self.version, self.target, self.named, self.manifest
        )?;
        if !self.runtime.is_empty() {
            write!(
                f,
                "; runtime premise: the OS supplies {}",
                self.runtime
                    .iter()
                    .map(|premise| match premise {
                        RuntimePremise::Soname { soname, file } => {
                            format!("the library whose soname is `{soname}` (read from `{file}`)")
                        }
                        RuntimePremise::InstallName {
                            name,
                            current_version,
                        } => match current_version {
                            Some(version) => format!("`{name}` (current version {version})"),
                            None => format!("`{name}`"),
                        },
                        RuntimePremise::Dll { name, os_build } => {
                            format!("`{name}` as on OS build {os_build}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" and ")
            )?;
        }
        write!(
            f,
            "; under ABI premises (data model {}, no unwinding across the boundary, closed allocator custody, single-threaded caller); the build hashes every pinned file before it links exactly that artifact — which code is linked, not that the code is safe",
            self.data_model.spelling()
        )
    }
}

impl FfiProgram {
    #[must_use]
    pub fn record(&self, name: &str) -> Option<&CheckedStruct> {
        self.structs.iter().find(|record| record.name == name)
    }

    /// A helper function by name.
    #[must_use]
    pub fn function(&self, name: &str) -> Option<&FfiFunction> {
        self.functions.iter().find(|function| function.name == name)
    }
}

/// A premise every result of this route holds relative to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FfiAssumption {
    /// The file-wide trust mark: every imported layout is faithful, and every
    /// `#c_ffi` contract function behaves as its contract states. Lists every
    /// imported struct (`Header = struct qr_header from "qr_header.h"`) and
    /// every marked function the entry calls.
    UnsafeFfi {
        structs: Vec<String>,
        functions: Vec<String>,
    },
    /// The layouts were read by this compiler for this target, and the
    /// emitted C is built by it.
    RecordedCompiler { compiler: String, triple: String },
    /// An admitted callee behaves as its selected contract states.
    DeclaredContract(String),
    /// The emitted C is compiled with the header's directory on its include
    /// path. Card `W48-QRLIB-LINKAGE` split off what this premise used to add —
    /// "and the library supplying each callee is linked by the builder" — and
    /// replaced it by one [`FfiAssumption::Linked`] per called function.
    BuilderIncludes { directories: Vec<String> },
    /// A called function's linkage, from a generated manifest (card
    /// `W48-QRLIB-LINKAGE`).
    Linked(Box<FfiLink>),
}

impl fmt::Display for FfiAssumption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsafeFfi { structs, functions } => write!(
                f,
                "unsafe FFI ({UNSAFE_FFI_ASSUMPTION}): this whole file imports C through `#import {FFI_MODULE};`, so every result holds only if the imported C layouts are faithful and each `#c_ffi` contract function behaves as its contract states; imported structs: {}; `#c_ffi` functions called: {}",
                if structs.is_empty() {
                    "none".to_owned()
                } else {
                    structs.join(", ")
                },
                if functions.is_empty() {
                    "none".to_owned()
                } else {
                    functions.join(", ")
                }
            ),
            Self::RecordedCompiler { compiler, triple } => write!(
                f,
                "recorded compiler: the imported layouts are the ones `{compiler}` reports for `{triple}`, and the emitted C asserts each of them when it is compiled"
            ),
            Self::DeclaredContract(callee) => {
                write!(
                    f,
                    "declared contract: `{callee}` behaves as its selected contract states"
                )
            }
            Self::BuilderIncludes { directories } => write!(
                f,
                "builder: the emitted C is compiled with {} on its include path",
                directories
                    .iter()
                    .map(|directory| format!("`{directory}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Linked(link) => write!(f, "{link}"),
        }
    }
}

/// One function the entry called, as the certificate lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FfiFunctionUse {
    pub name: String,
    pub module: String,
    pub signature: String,
    pub marked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FfiCertificate {
    pub entry: String,
    pub profile: &'static str,
    pub structs: Vec<CheckedStruct>,
    pub functions: Vec<FfiFunctionUse>,
    /// Every line, in program order across both streams.
    pub lines: Vec<ObservedLine>,
    pub flushes: usize,
    /// `(callee, outcome)` of every declared outcome, in program order.
    pub declared_outcomes: Vec<(String, String)>,
    pub effects: EffectRow,
    pub assumptions: Vec<FfiAssumption>,
    /// The signature of every `.qr` helper function, each checked at every
    /// call (card `W48-QR-USABLE-RECORDS`). Empty for every program written
    /// before it.
    pub helpers: Vec<String>,
    /// Every field write, in execution order (card `W48-QR-USABLE-RECORDS`).
    pub writes: Vec<FfiWrite>,
}

/// One checked field write (card `W48-QR-USABLE-RECORDS`): the statement, the
/// record, the listed field and the number it now holds, when determined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FfiWrite {
    pub statement: usize,
    pub record: String,
    pub field: String,
    pub value: Option<i128>,
}

/// One refusal of the checker, at a statement index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FfiError {
    pub statement: usize,
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for FfiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    /// A scalar, with the number when the source determined it.
    Scalar(ScalarType, Option<i128>),
    /// A record, with each listed field's number when determined.
    Record(String, BTreeMap<String, Option<i128>>),
}

fn error(statement: usize, code: &'static str, message: impl Into<String>) -> FfiError {
    FfiError {
        statement,
        code,
        message: message.into(),
    }
}

/// The number a literal or bool operand denotes at `ty`, range-checked.
fn literal_at(value: &FfiValue, ty: ScalarType, target: &RecordedTarget) -> Option<i128> {
    match (value, ty) {
        (FfiValue::Bool(flag), ScalarType::Bool) => Some(i128::from(*flag)),
        (FfiValue::Literal(number), ty) if ty.is_integer() => {
            ty.contains(*number, Some(target)).then_some(*number)
        }
        (FfiValue::Float(literal), ty) if ty.is_float() => literal.at(ty).ok(),
        _ => None,
    }
}

/// An operand at a scalar position of type `ty`: its number, if determined.
fn scalar_operand(
    index: usize,
    (code_type, code_range): (&'static str, &'static str),
    value: &FfiValue,
    ty: ScalarType,
    what: &str,
    env: &BTreeMap<String, Value>,
    target: &RecordedTarget,
) -> Result<Option<i128>, FfiError> {
    match value {
        FfiValue::Name(name) => match env.get(name) {
            Some(Value::Scalar(bound, number)) if *bound == ty => Ok(*number),
            Some(Value::Scalar(bound, _)) => Err(error(
                index,
                code_type,
                format!(
                    "`{name}` is a `{bound}` and {what} takes a `{ty}`; no conversion is implicit"
                ),
            )),
            Some(Value::Record(record, _)) => Err(error(
                index,
                code_type,
                format!("`{name}` is a `{record}` record and {what} takes a `{ty}`"),
            )),
            None => Err(error(
                index,
                code_type,
                format!("`{name}` is not a value this entry bound"),
            )),
        },
        other => literal_at(other, ty, target).map(Some).ok_or_else(|| {
            error(
                index,
                code_range,
                format!("{} is not a `{ty}` value, which {what} takes", show(other)),
            )
        }),
    }
}

fn show(value: &FfiValue) -> String {
    match value {
        FfiValue::Literal(number) => format!("`{number}`"),
        // The literal as its `f64` reading renders (card `W48-QR-FLOATING-POINT`).
        FfiValue::Float(literal) => match literal.f64 {
            Ok(bits) => format!(
                "the float literal `{}`",
                st::render(false, ScalarType::F64, bits, None).unwrap_or_default()
            ),
            Err(error) => format!("a float literal that {error}"),
        },
        FfiValue::Bool(flag) => format!("`{flag}`"),
        FfiValue::Name(name) => format!("`{name}`"),
    }
}

/// The fields of a construction or a record outcome: every listed field
/// exactly once, each value of exactly its type.
fn record_fields(
    index: usize,
    record: &CheckedStruct,
    fields: &[(String, FfiValue)],
    env: &BTreeMap<String, Value>,
    target: &RecordedTarget,
    literal_only: bool,
) -> Result<BTreeMap<String, Option<i128>>, FfiError> {
    let mut values = BTreeMap::new();
    for (name, value) in fields {
        let Some(field) = record.field(name) else {
            let reason = if record.opaque.contains(name) {
                format!(
                    "`{name}` is a member of `struct {}` the listing leaves opaque; an unlisted field is opaque bytes and cannot be read or written",
                    record.tag
                )
            } else {
                format!("`{name}` is not a field of `{}`", record.name)
            };
            return Err(error(index, "Q051", reason));
        };
        if literal_only && matches!(value, FfiValue::Name(_)) {
            return Err(error(
                index,
                "Q025",
                format!(
                    "a declared outcome states literal field values, and `{name}` names a binding"
                ),
            ));
        }
        let number = scalar_operand(
            index,
            ("Q052", "Q053"),
            value,
            field.ty,
            &format!("field `{name}`"),
            env,
            target,
        )?;
        if values.insert(name.clone(), number).is_some() {
            return Err(error(
                index,
                "Q051",
                format!("field `{name}` is supplied twice"),
            ));
        }
    }
    if let Some(missing) = record
        .listed
        .iter()
        .find(|field| !values.contains_key(&field.name))
    {
        return Err(error(
            index,
            "Q051",
            format!(
                "`{}` supplies no value for field `{}`; a construction supplies every listed field",
                record.name, missing.name
            ),
        ));
    }
    Ok(values)
}

fn outcome_text(outcome: &FfiOutcome) -> String {
    match outcome {
        FfiOutcome::Scalar(value) => show(value).trim_matches('`').to_owned(),
        FfiOutcome::Record { record, fields } => format!(
            "{record} {{ {} }}",
            fields
                .iter()
                .map(|(name, value)| format!("{name}: {}", show(value).trim_matches('`')))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Check the program and derive its certificate.
///
/// Card `W48-QR-USABLE-RECORDS`: the entry's statements are run in order, and
/// a call to a `.qr` helper function runs that function's statements with
/// the call's argument values bound to its parameters — records **copied**,
/// since an imported record is plain data here — so a helper is checked at
/// every call, with the values it is actually given, by the one statement
/// rule the entry uses. The call graph is acyclic and at most
/// [`MAX_CALL_DEPTH`] deep, and every helper must be reached from the entry:
/// a helper nothing calls would be checked by nothing.
///
/// # Errors
///
/// Every refusal found, in statement order; the certificate is produced only
/// when there is none.
pub fn check_ffi_program(program: &FfiProgram) -> Result<FfiCertificate, Vec<FfiError>> {
    let mut checker = Checker {
        program,
        errors: Vec::new(),
        lines: Vec::new(),
        flushes: 0,
        outcomes: Vec::new(),
        called: BTreeSet::new(),
        writes: Vec::new(),
    };
    // The entry ends with exactly one `return;`, and a helper with one
    // `return` of its result.
    let last = program.statements.len().saturating_sub(1);
    if !matches!(program.statements.last(), Some(FfiStatement::Return(None))) {
        checker.errors.push(error(
            last,
            "Q026",
            "the entry ends with exactly one `return;` on `source.ffi.v1`",
        ));
    }
    let order = match helper_order(program) {
        Ok(order) => order,
        Err(refusal) => return Err(vec![refusal]),
    };
    let derived = derived_effects(program, &order);
    let mut env = BTreeMap::new();
    checker.run(&program.statements, 0, &mut env, None, 0);
    for effect in derived.entry.iter() {
        if !program.declared_effects.contains(effect) {
            checker.errors.push(error(
                0,
                "Q029",
                format!(
                    "this entry performs `{effect:?}` but its declared `effects {{ … }}` row does not admit it"
                ),
            ));
        }
    }
    for function in &program.functions {
        if let Some(declared) = &function.declared_effects {
            for effect in derived.helpers[&function.name].iter() {
                if !declared.contains(effect) {
                    checker.errors.push(error(
                        function.base,
                        "Q029",
                        format!(
                            "`{}` performs `{effect:?}` but its declared `effects {{ … }}` row does not admit it",
                            function.name
                        ),
                    ));
                }
            }
        }
    }
    let mut errors = std::mem::take(&mut checker.errors);
    // A helper checked at two calls reports one fault once.
    let mut seen = BTreeSet::new();
    errors
        .retain(|refusal| seen.insert((refusal.statement, refusal.code, refusal.message.clone())));
    if !errors.is_empty() {
        errors.sort_by_key(|refusal| refusal.statement);
        return Err(errors);
    }
    let functions: Vec<FfiFunctionUse> = checker
        .called
        .iter()
        .map(|name| {
            let callee = &program.callees[name];
            FfiFunctionUse {
                name: name.clone(),
                module: callee.module.clone(),
                signature: callee.signature(),
                marked: callee.marked,
            }
        })
        .collect();
    let assumptions = derive_assumptions(program, &functions);
    Ok(FfiCertificate {
        entry: program.entry.clone(),
        profile: PROFILE,
        structs: program.structs.clone(),
        functions,
        lines: checker.lines,
        flushes: checker.flushes,
        declared_outcomes: checker.outcomes,
        effects: derived.entry,
        assumptions,
        helpers: program
            .functions
            .iter()
            .map(FfiFunction::signature)
            .collect(),
        writes: checker.writes,
    })
}

/// The helper functions in callee-first order, or the refusal: a call cycle,
/// a chain deeper than [`MAX_CALL_DEPTH`], or a helper the entry never
/// reaches (card `W48-QR-USABLE-RECORDS`).
fn helper_order(program: &FfiProgram) -> Result<Vec<String>, FfiError> {
    fn calls(statements: &[FfiStatement], program: &FfiProgram) -> BTreeSet<String> {
        statements
            .iter()
            .filter_map(|statement| match statement {
                FfiStatement::Call { callee, .. } if program.function(callee).is_some() => {
                    Some(callee.clone())
                }
                _ => None,
            })
            .collect()
    }
    fn visit(
        program: &FfiProgram,
        name: &str,
        active: &mut Vec<String>,
        done: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), FfiError> {
        let function = program.function(name).expect("a called helper exists");
        if let Some(at) = active.iter().position(|entry| entry == name) {
            let mut cycle = active[at..].to_vec();
            cycle.push(name.to_owned());
            return Err(error(
                function.base,
                "Q026",
                format!(
                    "`{name}` calls itself ({}); `.qr` helper calls on `source.ffi.v1` are acyclic, so every call is checked by running it",
                    cycle.join(" -> ")
                ),
            ));
        }
        if done.contains(name) {
            return Ok(());
        }
        if active.len() >= MAX_CALL_DEPTH {
            return Err(error(
                function.base,
                "Q026",
                format!(
                    "`{name}` is reached through more than {MAX_CALL_DEPTH} nested helper calls, the depth `source.ffi.v1` checks"
                ),
            ));
        }
        active.push(name.to_owned());
        for callee in calls(&function.statements, program) {
            visit(program, &callee, active, done, order)?;
        }
        active.pop();
        done.insert(name.to_owned());
        order.push(name.to_owned());
        Ok(())
    }
    let mut done = BTreeSet::new();
    let mut order = Vec::new();
    for callee in calls(&program.statements, program) {
        visit(program, &callee, &mut Vec::new(), &mut done, &mut order)?;
    }
    if let Some(unreached) = program
        .functions
        .iter()
        .find(|function| !done.contains(&function.name))
    {
        return Err(error(
            unreached.base,
            "Q026",
            format!(
                "`{}` is never called from the entry; a helper on `source.ffi.v1` is checked at every call, so one nothing calls would be admitted unchecked",
                unreached.name
            ),
        ));
    }
    Ok(order)
}

/// The derived effect rows: the entry's and each helper's, a helper's
/// including its callees' (card `W48-QR-USABLE-RECORDS`).
struct DerivedEffects {
    entry: EffectRow,
    helpers: BTreeMap<String, EffectRow>,
}

fn derived_effects(program: &FfiProgram, order: &[String]) -> DerivedEffects {
    let own = |statements: &[FfiStatement], helpers: &BTreeMap<String, EffectRow>| {
        let mut effects: BTreeSet<Effect> = BTreeSet::new();
        for statement in statements {
            match statement {
                FfiStatement::Call { callee, .. } => match helpers.get(callee) {
                    Some(row) => effects.extend(row.iter().cloned()),
                    None => {
                        effects.insert(Effect::Io);
                    }
                },
                FfiStatement::Line(StdIoEvent::LogLevel { .. } | StdIoEvent::Return) => {}
                FfiStatement::Line(_) => {
                    effects.insert(Effect::Io);
                }
                // A field write mutates an object of the program, as it does
                // on `source.buffer.v1`.
                FfiStatement::Write { .. } => {
                    effects.insert(Effect::Mut);
                }
                FfiStatement::Construct { .. }
                | FfiStatement::Read { .. }
                | FfiStatement::Copy { .. }
                | FfiStatement::Return(_) => {}
            }
        }
        EffectRow::new(effects)
    };
    let mut helpers = BTreeMap::new();
    for name in order {
        let function = program.function(name).expect("an ordered helper exists");
        let row = own(&function.statements, &helpers);
        helpers.insert(name.clone(), row);
    }
    DerivedEffects {
        entry: own(&program.statements, &helpers),
        helpers,
    }
}

/// The running state of one check.
struct Checker<'a> {
    program: &'a FfiProgram,
    errors: Vec<FfiError>,
    lines: Vec<ObservedLine>,
    flushes: usize,
    outcomes: Vec<(String, String)>,
    called: BTreeSet<String>,
    writes: Vec<FfiWrite>,
}

impl Checker<'_> {
    /// Run `statements`, whose first has global index `base`, in `env`.
    /// `function` is the helper being run (`None` for the entry). Returns the
    /// value a helper's `return VALUE;` produced.
    fn run(
        &mut self,
        statements: &[FfiStatement],
        base: usize,
        env: &mut BTreeMap<String, Value>,
        function: Option<&FfiFunction>,
        depth: usize,
    ) -> Option<Value> {
        let last = statements.len().saturating_sub(1);
        let mut returned = None;
        for (offset, statement) in statements.iter().enumerate() {
            let index = base + offset;
            match self.statement(index, offset == last, statement, env, function, depth) {
                Ok(Some(value)) => returned = Some(value),
                Ok(None) => {}
                Err(refusal) => self.errors.push(refusal),
            }
        }
        returned
    }

    #[allow(clippy::too_many_lines)]
    fn statement(
        &mut self,
        index: usize,
        is_last: bool,
        statement: &FfiStatement,
        env: &mut BTreeMap<String, Value>,
        function: Option<&FfiFunction>,
        depth: usize,
    ) -> Result<Option<Value>, FfiError> {
        let program = self.program;
        let target = &program.target;
        let bind = |env: &mut BTreeMap<String, Value>, name: &str, value: Value| {
            if env.insert(name.to_owned(), value).is_some() {
                Err(error(
                    index,
                    "Q026",
                    format!("`{name}` is bound twice in this function"),
                ))
            } else {
                Ok(())
            }
        };
        match statement {
            FfiStatement::Construct {
                binding,
                record,
                fields,
            } => {
                let Some(checked) = program.record(record) else {
                    return Err(error(
                        index,
                        "Q052",
                        format!("`{record}` is not an imported struct of this file"),
                    ));
                };
                let values = record_fields(index, checked, fields, env, target, false)?;
                bind(env, binding, Value::Record(record.clone(), values))?;
                Ok(None)
            }
            FfiStatement::Read {
                binding,
                source,
                field,
            } => {
                let Some(Value::Record(record, values)) = env.get(source) else {
                    return Err(error(
                        index,
                        "Q052",
                        format!("`{source}` is not a record this function bound"),
                    ));
                };
                let checked = program.record(record).expect("a bound record is imported");
                let listed = listed_field(index, checked, source, field)?;
                let value = Value::Scalar(listed.ty, values.get(field).copied().flatten());
                bind(env, binding, value)?;
                Ok(None)
            }
            // Card `W48-QR-USABLE-RECORDS`: the field write — a listed field
            // only, the value of exactly its type, replacing that field's
            // number and nothing else.
            FfiStatement::Write {
                record: source,
                field,
                value,
            } => {
                let Some(Value::Record(record, _)) = env.get(source) else {
                    return Err(error(
                        index,
                        "Q052",
                        format!("`{source}` is not a record this function bound"),
                    ));
                };
                let checked = program.record(record).expect("a bound record is imported");
                let listed = listed_field(index, checked, source, field)?;
                let number = scalar_operand(
                    index,
                    ("Q052", "Q053"),
                    value,
                    listed.ty,
                    &format!("field `{field}`"),
                    env,
                    target,
                )?;
                if let Some(Value::Record(_, values)) = env.get_mut(source) {
                    values.insert(field.clone(), number);
                }
                self.writes.push(FfiWrite {
                    statement: index,
                    record: source.clone(),
                    field: field.clone(),
                    value: number,
                });
                Ok(None)
            }
            FfiStatement::Copy { binding, source } => {
                let Some(value) = env.get(source).cloned() else {
                    return Err(error(
                        index,
                        "Q052",
                        format!("`{source}` is not a value this function bound"),
                    ));
                };
                bind(env, binding, value)?;
                Ok(None)
            }
            FfiStatement::Call {
                binding,
                callee,
                arguments,
                outcome,
            } => {
                if let Some(helper) = program.function(callee) {
                    let value = self.call_helper(index, helper, arguments, outcome, env, depth)?;
                    match (&helper.result, binding, value) {
                        (FfiType::Unit, None, _) => {}
                        (FfiType::Unit, Some(_), _) => {
                            return Err(error(
                                index,
                                "Q025",
                                format!("`{callee}` returns `unit`: a call of it binds nothing"),
                            ));
                        }
                        (result, None, _) => {
                            return Err(error(
                                index,
                                "Q025",
                                format!(
                                    "`{callee}` returns `{result}`: bind it with `let NAME = {callee}(…);`"
                                ),
                            ));
                        }
                        (_, Some(name), Some(value)) => bind(env, name, value)?,
                        // The helper refused before it returned; its refusal
                        // is the one reported.
                        (_, Some(_), None) => {}
                    }
                    return Ok(None);
                }
                let Some(admitted) = program.callees.get(callee) else {
                    return Err(error(
                        index,
                        "Q023",
                        format!("`{callee}` is not an admitted callee of this file"),
                    ));
                };
                self.called.insert(callee.clone());
                // Card `W48-QRLIB-LINKAGE`: a called C function is linked from
                // a pinned artifact a generated manifest names, never supplied
                // by the builder unnamed.
                if !program.links.contains_key(callee) {
                    return Err(error(
                        index,
                        "Q066",
                        format!(
                            "`{callee}` is called and resolves through no generated `.qrlib` this file binds with `extern library NAME;`; a C function is linked from a pinned artifact"
                        ),
                    ));
                }
                if arguments.len() != admitted.parameters.len() {
                    return Err(error(
                        index,
                        "Q024",
                        format!(
                            "`{callee}` takes {} argument(s) and is given {}",
                            admitted.parameters.len(),
                            arguments.len()
                        ),
                    ));
                }
                for ((parameter, ty), argument) in admitted.parameters.iter().zip(arguments) {
                    match ty {
                        FfiType::Scalar(scalar) => {
                            scalar_operand(
                                index,
                                ("Q024", "Q024"),
                                argument,
                                *scalar,
                                &format!("parameter `{parameter}` of `{callee}`"),
                                env,
                                target,
                            )?;
                        }
                        FfiType::Record(record) | FfiType::ConstRecordPointer(record) => {
                            let ok = matches!(argument, FfiValue::Name(name)
                                if matches!(env.get(name), Some(Value::Record(bound, _)) if bound == record));
                            if !ok {
                                return Err(error(
                                    index,
                                    "Q024",
                                    format!(
                                        "parameter `{parameter}` of `{callee}` takes a `{record}` record{}, and {} is not one this function bound",
                                        if matches!(ty, FfiType::ConstRecordPointer(_)) {
                                            " by `const` pointer"
                                        } else {
                                            " by value"
                                        },
                                        show(argument)
                                    ),
                                ));
                            }
                        }
                        FfiType::Unit => {
                            return Err(error(
                                index,
                                "Q024",
                                format!("parameter `{parameter}` of `{callee}` is `unit`"),
                            ));
                        }
                    }
                }
                let value = match (&admitted.result, binding, outcome) {
                    (FfiType::Unit, None, None) => None,
                    (FfiType::Unit, _, _) => {
                        return Err(error(
                            index,
                            "Q025",
                            format!(
                                "`{callee}` returns `unit`: a call of it binds nothing and declares no outcome"
                            ),
                        ));
                    }
                    (_, None, _) => {
                        return Err(error(
                            index,
                            "Q025",
                            format!(
                                "`{callee}` returns `{}`: bind it with `let NAME = {callee}(…);`",
                                admitted.result
                            ),
                        ));
                    }
                    (FfiType::ConstRecordPointer(record), Some(_), _) => {
                        return Err(error(
                            index,
                            "Q024",
                            format!(
                                "`{callee}` returns a pointer to `{record}`; a result is `unit`, a scalar or a record by value — a returned address is a lifetime this route does not model"
                            ),
                        ));
                    }
                    (FfiType::Scalar(ty), Some(_), None) => Some(Value::Scalar(*ty, None)),
                    (FfiType::Scalar(ty), Some(_), Some(FfiOutcome::Scalar(declared))) => {
                        let number = literal_at(declared, *ty, target).ok_or_else(|| {
                            error(
                                index,
                                "Q025",
                                format!(
                                    "the declared outcome {} is not a `{ty}` literal, which `{callee}` returns",
                                    show(declared)
                                ),
                            )
                        })?;
                        self.outcomes.push((
                            callee.clone(),
                            outcome_text(outcome.as_ref().expect("matched")),
                        ));
                        Some(Value::Scalar(*ty, Some(number)))
                    }
                    (FfiType::Record(record), Some(_), None) => {
                        let checked = program
                            .record(record)
                            .expect("an admitted type is imported");
                        Some(Value::Record(
                            record.clone(),
                            checked
                                .listed
                                .iter()
                                .map(|field| (field.name.clone(), None))
                                .collect(),
                        ))
                    }
                    (
                        FfiType::Record(record),
                        Some(_),
                        Some(FfiOutcome::Record {
                            record: declared,
                            fields,
                        }),
                    ) if declared == record => {
                        let checked = program
                            .record(record)
                            .expect("an admitted type is imported");
                        let values = record_fields(index, checked, fields, env, target, true)?;
                        self.outcomes.push((
                            callee.clone(),
                            outcome_text(outcome.as_ref().expect("matched")),
                        ));
                        Some(Value::Record(record.clone(), values))
                    }
                    (result, Some(_), Some(declared)) => {
                        return Err(error(
                            index,
                            "Q025",
                            format!(
                                "the declared outcome `{}` is not a value of `{result}`, which `{callee}` returns",
                                outcome_text(declared)
                            ),
                        ));
                    }
                };
                if let (Some(name), Some(value)) = (binding, value) {
                    bind(env, name, value)?;
                }
                Ok(None)
            }
            FfiStatement::Line(event) => {
                match event {
                    StdIoEvent::Print { rendering, operand }
                    | StdIoEvent::Log {
                        rendering, operand, ..
                    } => {
                        let level = match event {
                            StdIoEvent::Log { level, .. } => Some(*level),
                            _ => None,
                        };
                        let (ty, number) = match operand {
                            StdIoOperand::Literal(number) => (ScalarType::I32, i128::from(*number)),
                            StdIoOperand::Borrowed(name) => {
                                return Err(error(
                                    index,
                                    "Q028",
                                    format!(
                                        "`borrow {name}` renders a custody identity, and `source.ffi.v1` has no custody model"
                                    ),
                                ));
                            }
                            StdIoOperand::Scalar(name) => match env.get(name) {
                                Some(Value::Scalar(ty, Some(number))) => (*ty, *number),
                                Some(Value::Scalar(ty, None)) => {
                                    return Err(error(
                                        index,
                                        "Q043",
                                        format!(
                                            "`{name}` is a `{ty}` this module models no number for — a C result with no declared outcome, or a field of one — so rendering it would publish a line no model computed; declare the call's outcome with `succeeds`"
                                        ),
                                    ));
                                }
                                Some(Value::Record(record, _)) => {
                                    return Err(error(
                                        index,
                                        "Q028",
                                        format!(
                                            "`{name}` is a `{record}` record; a line renders one scalar, so read a field first"
                                        ),
                                    ));
                                }
                                None => {
                                    return Err(error(
                                        index,
                                        "Q028",
                                        format!("`{name}` is not a value this function bound"),
                                    ));
                                }
                            },
                        };
                        let Some(body) =
                            st::render(*rendering == Rendering::Hex, ty, number, Some(target))
                        else {
                            return Err(error(
                                index,
                                "Q043",
                                format!("`hex` renders an integer, and this operand is a `{ty}`"),
                            ));
                        };
                        self.lines
                            .push(crate::std_io::rendered_text_line(level, &body));
                    }
                    StdIoEvent::Flush => self.flushes += 1,
                    StdIoEvent::LogLevel { .. } => {
                        return Err(error(
                            index,
                            "Q030",
                            "`log_level` installs a system-event adapter, and `source.ffi.v1` publishes no system event",
                        ));
                    }
                    StdIoEvent::Return => {}
                }
                Ok(None)
            }
            FfiStatement::Return(value) => {
                if !is_last {
                    return Err(error(
                        index,
                        "Q026",
                        "`return` is its function's last statement",
                    ));
                }
                match (function, value) {
                    (None, None) => Ok(None),
                    (None, Some(_)) => Err(error(
                        index,
                        "Q026",
                        "`return;` takes no value in the entry",
                    )),
                    (Some(helper), value) => self.returned(index, helper, value.as_ref(), env),
                }
            }
        }
    }

    /// The value a helper's `return` produces, of exactly its result type.
    fn returned(
        &self,
        index: usize,
        helper: &FfiFunction,
        value: Option<&FfiValue>,
        env: &BTreeMap<String, Value>,
    ) -> Result<Option<Value>, FfiError> {
        let name = &helper.name;
        match (&helper.result, value) {
            (FfiType::Unit, None) => Ok(Some(Value::Scalar(ScalarType::Bool, None))),
            (FfiType::Scalar(ty), Some(value)) => {
                let number = scalar_operand(
                    index,
                    ("Q025", "Q025"),
                    value,
                    *ty,
                    &format!("the result of `{name}`"),
                    env,
                    &self.program.target,
                )?;
                Ok(Some(Value::Scalar(*ty, number)))
            }
            (FfiType::Record(record), Some(FfiValue::Name(bound))) => match env.get(bound) {
                Some(Value::Record(found, values)) if found == record => {
                    Ok(Some(Value::Record(found.clone(), values.clone())))
                }
                _ => Err(error(
                    index,
                    "Q025",
                    format!("`{name}` returns a `{record}` record, and `{bound}` is not one"),
                )),
            },
            (result, _) => Err(error(
                index,
                "Q025",
                format!("`{name}` returns `{result}`, and this `return` does not return one"),
            )),
        }
    }

    /// Run one helper call: its arguments checked against its parameters and
    /// bound — a record **copied** — then its body.
    fn call_helper(
        &mut self,
        index: usize,
        helper: &FfiFunction,
        arguments: &[FfiValue],
        outcome: &Option<FfiOutcome>,
        env: &BTreeMap<String, Value>,
        depth: usize,
    ) -> Result<Option<Value>, FfiError> {
        let name = &helper.name;
        if outcome.is_some() {
            return Err(error(
                index,
                "Q025",
                format!(
                    "`{name}` is a `.qr` function this file checks, so its result is computed rather than declared; `succeeds` states the outcome of a C call"
                ),
            ));
        }
        if arguments.len() != helper.parameters.len() {
            return Err(error(
                index,
                "Q024",
                format!(
                    "`{name}` takes {} argument(s) and is given {}",
                    helper.parameters.len(),
                    arguments.len()
                ),
            ));
        }
        if depth >= MAX_CALL_DEPTH {
            return Err(error(
                index,
                "Q026",
                format!("`{name}` is called more than {MAX_CALL_DEPTH} helper calls deep"),
            ));
        }
        let mut inner = BTreeMap::new();
        for ((parameter, ty), argument) in helper.parameters.iter().zip(arguments) {
            let value = match ty {
                FfiType::Scalar(scalar) => Value::Scalar(
                    *scalar,
                    scalar_operand(
                        index,
                        ("Q024", "Q024"),
                        argument,
                        *scalar,
                        &format!("parameter `{parameter}` of `{name}`"),
                        env,
                        &self.program.target,
                    )?,
                ),
                FfiType::Record(record) => match argument {
                    FfiValue::Name(bound) => match env.get(bound) {
                        Some(Value::Record(found, values)) if found == record => {
                            Value::Record(found.clone(), values.clone())
                        }
                        _ => {
                            return Err(error(
                                index,
                                "Q024",
                                format!(
                                    "parameter `{parameter}` of `{name}` takes a `{record}` record by value, and `{bound}` is not one this function bound"
                                ),
                            ));
                        }
                    },
                    other => {
                        return Err(error(
                            index,
                            "Q024",
                            format!(
                                "parameter `{parameter}` of `{name}` takes a `{record}` record by value, and {} is not one",
                                show(other)
                            ),
                        ));
                    }
                },
                FfiType::Unit | FfiType::ConstRecordPointer(_) => {
                    return Err(error(
                        index,
                        "Q024",
                        format!(
                            "parameter `{parameter}` of `{name}` is `{ty}`; a `.qr` function takes scalars and imported records by value"
                        ),
                    ));
                }
            };
            inner.insert(parameter.clone(), value);
        }
        let before = self.errors.len();
        let value = self.run(
            &helper.statements,
            helper.base,
            &mut inner,
            Some(helper),
            depth + 1,
        );
        if self.errors.len() > before {
            return Ok(None);
        }
        if !matches!(helper.statements.last(), Some(FfiStatement::Return(_))) {
            return Err(error(
                helper.base + helper.statements.len().saturating_sub(1),
                "Q026",
                format!("`{name}` ends with exactly one `return`"),
            ));
        }
        Ok(value)
    }
}

/// A listed field of `checked`, or the refusal: an opaque member by name,
/// or not a member at all (`Q051`).
fn listed_field<'a>(
    index: usize,
    checked: &'a CheckedStruct,
    source: &str,
    field: &str,
) -> Result<&'a ListedField, FfiError> {
    checked.field(field).ok_or_else(|| {
        let reason = if checked.opaque.iter().any(|opaque| opaque == field) {
            format!(
                "`{source}.{field}` names a member of `struct {}` the listing leaves opaque; an unlisted field is opaque bytes and cannot be read or written",
                checked.tag
            )
        } else {
            format!("`{field}` is not a field of `{}`", checked.name)
        };
        error(index, "Q051", reason)
    })
}

fn derive_assumptions(program: &FfiProgram, functions: &[FfiFunctionUse]) -> Vec<FfiAssumption> {
    let mut assumptions = vec![FfiAssumption::UnsafeFfi {
        structs: program
            .structs
            .iter()
            .map(|record| {
                format!(
                    "`{} = struct {}` from \"{}\" (sha256 {})",
                    record.name, record.tag, record.header, record.facts.header_sha256
                )
            })
            .collect(),
        functions: functions
            .iter()
            .filter(|function| function.marked)
            .map(|function| format!("`{}` ({})", function.name, function.module))
            .collect(),
    }];
    assumptions.push(FfiAssumption::RecordedCompiler {
        compiler: program.target.compiler.clone(),
        triple: program.target.triple.clone(),
    });
    for function in functions {
        assumptions.push(FfiAssumption::DeclaredContract(function.name.clone()));
    }
    let mut directories: Vec<String> = program
        .structs
        .iter()
        .map(|record| record.facts.header_directory.clone())
        .collect();
    directories.sort();
    directories.dedup();
    if !directories.is_empty() {
        assumptions.push(FfiAssumption::BuilderIncludes { directories });
    }
    // Card `W48-QRLIB-LINKAGE`: one linkage premise per called function, in
    // the order the certificate lists them.
    for function in functions {
        if let Some(link) = program.links.get(&function.name) {
            assumptions.push(FfiAssumption::Linked(Box::new(link.clone())));
        }
    }
    assumptions
}

/// Whether a certificate carries a linkage premise for every function it
/// lists as called (card `W48-QRLIB-LINKAGE`). The driver refuses to publish
/// one that does not (`Q067`): a called C function whose certificate stopped
/// naming the pinned artifact would be supplied by the builder unnamed again.
#[must_use]
pub fn certificate_carries_linkage(certificate: &FfiCertificate) -> bool {
    certificate.functions.iter().all(|function| {
        certificate.assumptions.iter().any(|assumption| {
            matches!(assumption, FfiAssumption::Linked(link) if link.callee == function.name)
        })
    })
}

/// Whether a certificate carries the file-wide trust mark, naming every
/// imported struct and every marked function it lists. The driver refuses to
/// publish one that does not (`Q035`): an FFI result that stopped saying what
/// it rests on would be a plain `Safe`.
#[must_use]
pub fn certificate_carries_ffi_assumption(certificate: &FfiCertificate) -> bool {
    certificate.assumptions.iter().any(|assumption| {
        let FfiAssumption::UnsafeFfi { structs, functions } = assumption else {
            return false;
        };
        structs.len() == certificate.structs.len()
            && certificate.structs.iter().all(|record| {
                structs.iter().any(|entry| {
                    entry.starts_with(&format!("`{} = struct {}`", record.name, record.tag))
                })
            })
            && certificate
                .functions
                .iter()
                .filter(|function| function.marked)
                .all(|function| {
                    functions
                        .iter()
                        .any(|entry| entry.starts_with(&format!("`{}`", function.name)))
                })
    })
}

/// Why a certificate failed its recheck.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FfiRecheckFailure {
    /// The program no longer checks.
    Refused(Vec<FfiError>),
    /// A listed field's published offset or type is not the recorded one.
    LayoutDiffers { record: String },
    /// Any other field of the certificate differs from its rederivation.
    Differs,
}

impl fmt::Display for FfiRecheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(errors) => write!(
                f,
                "the program no longer checks ({} refusal(s))",
                errors.len()
            ),
            Self::LayoutDiffers { record } => write!(
                f,
                "the certificate publishes a layout of `{record}` the recorded compiler's facts do not give"
            ),
            Self::Differs => write!(f, "the certificate differs from its rederivation"),
        }
    }
}

/// Rederive the certificate from the program and compare. Each published
/// struct's listed offsets and types are checked against its own facts first,
/// so a certificate whose layout was edited is named as such.
///
/// # Errors
///
/// The first disagreement.
pub fn recheck_ffi_certificate(
    program: &FfiProgram,
    certificate: &FfiCertificate,
) -> Result<(), FfiRecheckFailure> {
    for record in &certificate.structs {
        let import = ImportedStruct {
            name: record.name.clone(),
            tag: record.tag.clone(),
            header: record.header.clone(),
            fields: record
                .listed
                .iter()
                .map(|field| (field.name.clone(), field.ty))
                .collect(),
        };
        match check_layout(&import, &record.facts, Some(&program.target)) {
            Ok(rederived) if rederived == *record => {}
            _ => {
                return Err(FfiRecheckFailure::LayoutDiffers {
                    record: record.name.clone(),
                });
            }
        }
    }
    let rederived = check_ffi_program(program).map_err(FfiRecheckFailure::Refused)?;
    if rederived == *certificate {
        Ok(())
    } else {
        Err(FfiRecheckFailure::Differs)
    }
}

/// The reference observation as `"<stream>: <line>"` records, in program
/// order — the form every source route's native comparison reads.
#[must_use]
pub fn reference_observation(certificate: &FfiCertificate) -> Vec<String> {
    certificate
        .lines
        .iter()
        .map(|line| format!("{}: {}", line.stream.label(), line.text))
        .collect()
}

/// The stdout projection of the reference observation, one line each.
#[must_use]
pub fn reference_stream(certificate: &FfiCertificate, stream: Stream) -> String {
    certificate
        .lines
        .iter()
        .filter(|line| line.stream == stream)
        .map(|line| format!("{}\n", line.text))
        .collect()
}
