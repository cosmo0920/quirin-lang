//! The `source.native-library.v1` manifest, in the slice card
//! `UNSAFE-LINKAGE-ARTIFACT` needs: a declared binding from a **library
//! artifact identity** to the symbols that artifact exports, so that a waived
//! extern declaration's premise can name *which build* was waived.
//!
//! Design: `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` §4 (the manifest) and
//! `docs/design/UNSAFE_LINKAGE_V1.md` §4.2 (the contract-less entry, level L1
//! of the linkage spectrum). This module implements exactly the part of §4 the
//! contract-less entry needs and refuses the rest by name.
//!
//! # What a manifest is
//!
//! One declarative file per native library. Its header states the artifact
//! identity (§4.3) and the four ABI premise fields (§4.4); its body lists one
//! entry per symbol. Every v1 entry is an **explicitly marked contract-less**
//! entry (`unchecked fn`), because this slice implements the contract-less end
//! only — a contract slice member is [`ManifestRefusal::ContractedEntry`], not a
//! silently ignored line, so a *dropped* contract can never be read as a waiver
//! (design §4.2).
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
//! ```
//!
//! # Total or reject
//!
//! [`load_native_library_manifest`] follows the resource-contract v1 loader's
//! discipline exactly: an unknown field, an unknown field *value*, a repeated
//! or missing field, a malformed statement, an entry kind this slice does not
//! implement, a symbol bound twice or a type spelling outside the §C04.1 ABI
//! projection **rejects the whole manifest**. There is no partial admission and
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
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::extern_call::AbiType;

/// The data model the artifact was built under (§4.4). v1 admits `lp64` only,
/// which makes `LOADER-ABI-LP64` an explicit statement of the manifest instead
/// of an implicit assumption of a reader. Making the scalar projection genuinely
/// target-aware stays that card's own follow-on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataModel {
    /// Spelled `lp64`.
    Lp64,
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
/// check vacuously, because it cannot acquire (design §4.2).
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

/// One loaded manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeLibraryManifest {
    pub identity: ArtifactIdentity,
    pub premises: AbiPremises,
    /// Contract-less entries by symbol. A symbol may be bound at most once.
    pub unchecked: BTreeMap<String, UncheckedEntry>,
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
    /// An entry kind this slice does not implement. A contract slice member
    /// (`contract fn`) is the named case: implementing it is follow-on
    /// `NATIVE-LIBRARY-CONTRACT-SLICE`, and until then it must reject rather
    /// than be read as a waiver.
    ContractedEntry { statement: String },
    /// An entry is not `unchecked fn NAME(TYPES) -> TYPE`.
    MalformedEntry { statement: String },
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
    /// belong in a contract slice entry (`NATIVE-LIBRARY-CONTRACT-SLICE`), and
    /// this refusal is what keeps the two apart.
    PointerInUncheckedEntry { symbol: String, spelling: String },
    /// An entry's parameter is declared `unit`, which the ABI vocabulary admits
    /// as a result type only.
    UnitParameterInEntry { symbol: String, parameter: usize },
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
            Self::ContractedEntry { statement } => write!(
                f,
                "the entry `{statement}` carries a contract; this manifest slice implements contract-less entries only, and a contracted entry is refused rather than read as a waiver"
            ),
            Self::MalformedEntry { statement } => write!(
                f,
                "the entry `{statement}` is not `unchecked fn NAME(TYPES) -> TYPE`"
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
const HEADER_FIELDS: [&str; 8] = [
    "library",
    "version",
    "artifact",
    "hash",
    "data_model",
    "unwinding",
    "allocator_custody",
    "thread_posture",
];

/// The manifest's selector marker, which must be its first statement.
const MARKER: &str = "native_library manifest v1";

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

/// Parse one contract-less entry body, `NAME(TYPES) -> TYPE`.
fn parse_unchecked_entry(body: &str, statement: &str) -> Result<UncheckedEntry, ManifestRefusal> {
    let malformed = || ManifestRefusal::MalformedEntry {
        statement: statement.to_owned(),
    };
    let open = body.find('(').ok_or_else(malformed)?;
    let close = body.find(')').ok_or_else(malformed)?;
    if close < open {
        return Err(malformed());
    }
    let symbol = body[..open].trim();
    if !well_formed_symbol(symbol) {
        return Err(malformed());
    }
    let tail = body[close + 1..].trim();
    let result_text = tail.strip_prefix("->").ok_or_else(malformed)?.trim();
    if result_text.is_empty() {
        return Err(malformed());
    }
    let inside = body[open + 1..close].trim();
    let mut parameters = Vec::new();
    if !inside.is_empty() {
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
            if abi.is_pointer() {
                return Err(ManifestRefusal::PointerInUncheckedEntry {
                    symbol: symbol.to_owned(),
                    spelling: spelling.to_owned(),
                });
            }
            parameters.push(abi);
        }
    }
    let Some(result) = parse_c_declaration_type(result_text) else {
        return Err(ManifestRefusal::UnknownResultSpelling {
            symbol: symbol.to_owned(),
            spelling: result_text.to_owned(),
        });
    };
    if result.is_pointer() {
        return Err(ManifestRefusal::PointerInUncheckedEntry {
            symbol: symbol.to_owned(),
            spelling: result_text.to_owned(),
        });
    }
    Ok(UncheckedEntry {
        symbol: symbol.to_owned(),
        parameters,
        result,
    })
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
    match iterator.next() {
        Some(first) if first.split_whitespace().collect::<Vec<_>>().join(" ") == MARKER => {}
        _ => return Err(ManifestRefusal::MissingMarker),
    }
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    let mut unchecked: BTreeMap<String, UncheckedEntry> = BTreeMap::new();
    for statement in iterator {
        if statement.is_empty() {
            continue;
        }
        let (keyword, rest) = match statement.find(char::is_whitespace) {
            Some(position) => (&statement[..position], statement[position..].trim()),
            None => (statement.as_str(), ""),
        };
        match keyword {
            "unchecked" => {
                let Some(body) = rest.strip_prefix("fn ") else {
                    return Err(ManifestRefusal::MalformedEntry {
                        statement: statement.clone(),
                    });
                };
                let entry = parse_unchecked_entry(body.trim(), statement)?;
                if unchecked.contains_key(&entry.symbol) {
                    return Err(ManifestRefusal::SymbolBoundTwice {
                        symbol: entry.symbol,
                    });
                }
                unchecked.insert(entry.symbol.clone(), entry);
            }
            "contract" => {
                return Err(ManifestRefusal::ContractedEntry {
                    statement: statement.clone(),
                });
            }
            field if HEADER_FIELDS.contains(&field) => {
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
    for field in HEADER_FIELDS {
        if !fields.contains_key(field) {
            return Err(ManifestRefusal::MissingField { field });
        }
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
    let premises = AbiPremises {
        data_model: closed_value(&fields, "data_model", "lp64", DataModel::Lp64)?,
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
    if unchecked.is_empty() {
        return Err(ManifestRefusal::NoSymbols);
    }
    Ok(NativeLibraryManifest {
        identity: ArtifactIdentity {
            library: value("library"),
            version: value("version"),
            artifact: value("artifact"),
            hash,
        },
        premises,
        unchecked,
    })
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

/// One symbol's binding, resolved out of a selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestBinding<'a> {
    /// The manifest module this binding came from.
    pub module: &'a str,
    pub identity: &'a ArtifactIdentity,
    pub premises: &'a AbiPremises,
    pub entry: &'a UncheckedEntry,
}

impl ManifestBinding<'_> {
    /// The residence record this binding contributes to a callee.
    #[must_use]
    pub fn residence(&self) -> ManifestResidence {
        ManifestResidence {
            identity: self.identity.clone(),
            premises: *self.premises,
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
            for symbol in manifest.unchecked.keys() {
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

    /// Resolve one symbol, or `None` when no selected manifest names it.
    #[must_use]
    pub fn binding(&self, symbol: &str) -> Option<ManifestBinding<'_>> {
        let module = self.symbols.get(symbol)?;
        let manifest = self.manifests.get(module)?;
        let entry = manifest.unchecked.get(symbol)?;
        Some(ManifestBinding {
            module,
            identity: &manifest.identity,
            premises: &manifest.premises,
            entry,
        })
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
    if declared_parameters.len() != entry.parameters.len() {
        return Err(ManifestDisagreement::ParameterCount {
            declared: declared_parameters.len(),
            manifest: entry.parameters.len(),
        });
    }
    for (index, (declared, manifest)) in declared_parameters
        .iter()
        .zip(entry.parameters.iter())
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
    if declared_result != entry.result {
        return Err(ManifestDisagreement::Result {
            declared: declared_result,
            manifest: entry.result,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/native_library.rs"]
mod tests;
