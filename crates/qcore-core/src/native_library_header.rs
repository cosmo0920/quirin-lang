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

//! The **exported-header level** of `source.native-library.v1`: reading the
//! library's real `.h` instead of believing the manifest's transcription of it.
//!
//! Design: [`docs/design/NATIVE_LIBRARY_HEADER_V1.md`](../../../docs/design/NATIVE_LIBRARY_HEADER_V1.md),
//! card `NATIVE-LIBRARY-HEADER`. The manifest levels below this one are card
//! `UNSAFE-LINKAGE-ARTIFACT` (contract-less entries) and card
//! `NATIVE-LIBRARY-CONTRACT-SLICE` (contract slice entries), both in
//! [`crate::native_library`].
//!
//! # What this level adds
//!
//! Every manifest entry carries the exported header declaration **as the
//! manifest author transcribed it**:
//!
//! ```text
//! contract fn qcore_extern_alloc(uintptr_t) -> uint8_t * from contracts/buffer_lib.qcore;
//! ```
//!
//! Until this card nothing compared that transcription against the header the
//! library actually ships, so "the manifest's spelling is the header's" was a
//! premise — and since wave 20 a premise that *pointers* rest on. A manifest may
//! now name its exported header and its digest:
//!
//! ```text
//! header include/qcore_buffer_demo.h;
//! header_hash sha256:2b0f…c41d;
//! ```
//!
//! and this module reads that header's own text, projects the declaration of
//! every symbol the manifest binds into the §C04.1 ABI vocabulary, and compares.
//!
//! # No typed AST, and no half a preprocessor
//!
//! Constraint 8's reader discipline: both sides of an ABI comparison are read
//! from text a certificate already holds, so W4-style rederivation and a
//! certificate recheck reproduce the decision. The gateway's declared-ABI reader
//! satisfies it over a *preprocessed* translation unit — Clang has already run,
//! and it never sees a `#`. This reader does see one, so it defines a **subset**
//! and refuses everything outside it **by name** ([`HeaderRefusal`]):
//!
//! * comments, line splicing and the file's include guard are read;
//! * every other conditional region is **not read**, and a bound symbol declared
//!   only inside one refuses naming that reason rather than passing as
//!   undeclared;
//! * an `#include` the manifest does not **declare** is never followed, so a
//!   type that depends on a name defined one file away refuses rather than being
//!   guessed;
//! * a directive outside `if`/`ifdef`/`ifndef`/`elif`/`else`/`endif`/`define`/
//!   `undef`/`include` refuses the whole header.
//!
//! A bounded subset with named refusals beats a half-C-preprocessor: the census
//! in the card report says exactly which real declarations the subset covers and
//! exactly which named refusal excludes the rest.
//!
//! # Total or reject, at the manifest
//!
//! A header binding that cannot be read rejects the whole manifest selection at
//! **load**, exactly as an unresolved contract slice binding does (card
//! `NATIVE-LIBRARY-CONTRACT-SLICE`, `Q047`): a partially read header binding
//! would leave the manifest attributing a build on the strength of a header this
//! reader checked only in part, and *which* part would then depend on the
//! reader's own coverage rather than on anything the manifest says.
//!
//! The totality is **one-directional**, deliberately the opposite of the
//! contract slice's rule. A contract module in a manifest's slice is authored
//! for this library, so a declaration in it that no entry binds makes the
//! binding partial; a header is the library's entire exported surface, authored
//! without reference to any manifest. Declarations the manifest does not name
//! are ignored.
//!
//! # A disagreement refuses
//!
//! Card `NATIVE-LIBRARY-CONTRACT-SLICE`'s rule decides it: *withdraw only where
//! the weaker admissible reading refuses.* Below a withdrawn header binding the
//! weaker reading is **the manifest without a header**, which is accepted — so
//! withdrawing would reward a contradiction with a premise that says less about
//! the build. That is level L1's situation (`Q041`) and it gets L1's
//! disposition.
//!
//! There is a second and sharper reason. The contract-slice withdrawal is
//! per-unit *because one of its two sides is per-unit*. Both sides of this
//! comparison are **load inputs**: the manifest text and the header text are in
//! hand before any unit is looked at, so there is no per-unit reading to
//! withdraw and no unit for which the disagreement could fail to hold.
//!
//! # The digest is verified, and the artifact hash still is not
//!
//! [`crate::native_library`]'s artifact identity is a *pin, not a probe*:
//! nothing is opened and nothing is hashed, because a hash of a file on this
//! disk is not evidence about the **link**, which stays the builder's act. The
//! header is different in the way that matters — **it is read**, and its text
//! decides admission. A certificate that did not name that text by content could
//! not be rederived at all, since a recheck whose `header` reference resolved to
//! different bytes would silently take a different decision. So the digest is
//! verified, and the premise a certificate publishes carries one **verified**
//! identity beside three declared ones.
//!
//! `qcore-core` has no dependencies at all — that is what makes the manifest
//! rederivable in the kernel — so it implements no SHA-256. The driver measures
//! the digest where the bytes are read and this module owns the *rule*, the same
//! split [`crate::native_library::CustodyShape`] uses. Recorded as
//! **`HEADER-DIGEST-DRIVER-MEASURED`**.
//!
//! # The export decoration: derived, not declared
//!
//! Card `NATIVE-LIBRARY-HEADER`'s census measured **0 of 423** Libevent
//! declarations inside the subset, for one reason: every exported declaration is
//! written `EVENT2_EXPORT_SYMBOL struct event_base *event_base_new(void);`, and
//! that macro's `#define` lives in a header the reader did not follow. The
//! obvious repair — a manifest field listing the names to erase — was declined
//! there, because it *converts a reader boundary into an unchecked premise about
//! the header's own text*, which is precisely what this level exists to remove.
//!
//! Card `NATIVE-LIBRARY-HEADER-DECORATION` takes the derived route instead, and
//! the split is exact:
//!
//! * a manifest declares **which texts** may be read — each by an `#include`
//!   spelling, a reference and a digest, identified exactly as the header is
//!   ([`IncludeIdentity`]). A manifest never says what a text contains;
//! * the reader **derives from those texts' own `#define`s** which names are
//!   erasable declaration decorations ([`derive_erasures`]), evaluating no
//!   condition: a name qualifies only when every one of its definitions is
//!   object-like and has an erasable body, it is never `#undef`ed, and it is
//!   defined on **every path** through the conditional structure preceding its
//!   first use. Whichever arm a consumer's configuration selects, the name is
//!   defined and its expansion is a decoration;
//! * the **vocabulary** of erasable bodies is the reader's own, fixed and
//!   auditable ([`ERASABLE_ATTRIBUTES`]), and admits only decorations that
//!   change neither the declared type, the parameter vector nor the linkage
//!   name. A body outside it — `__attribute__((mode(DI)))`, `alias`, `ms_abi` —
//!   is **not** erased, and the declaration keeps the refusal it already had.
//!
//! So no statement of a manifest is believed about a header's contents: the
//! manifest selects hash-identified texts, and every erasure is a reading of
//! those texts under a vocabulary this repository publishes. The one premise
//! that remains is the one card `NATIVE-LIBRARY-HEADER` already carried —
//! that the supplied bytes are the text the reference names — and it is
//! discharged the same way, by digest.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::extern_call::AbiType;
use crate::native_library::{NativeLibraries, NativeLibraryManifest};

/// The identity of one exported header: the reference the manifest spells and
/// the digest of the text that reference resolves to.
///
/// The reference is compared **exactly** and never normalised — constraint 11's
/// discipline applied to a third kind of spelled name, for the same reason: two
/// spellings that normalise together are two different selections.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeaderIdentity {
    /// The header reference, as the manifest spells it.
    pub reference: String,
    /// `sha256:` followed by 64 lowercase hex digits, as the manifest spells it.
    pub digest: String,
    /// The further texts this header's own `#include` lines are read from (card
    /// `NATIVE-LIBRARY-HEADER-DECORATION`), each identified exactly as the header
    /// is.
    ///
    /// Empty is the wave-22 world and the default: no `#include` is followed,
    /// and every name one file away stays unreadable. The set is **closed** —
    /// an `#include` this manifest does not declare is not followed either — so
    /// the texts a decision rests on are exactly the texts the certificate
    /// names.
    pub includes: Vec<IncludeIdentity>,
}

/// The identity of one text a header's `#include` is read from.
///
/// Three parts, and the separation is the point. The **spelling** is the operand
/// of the `#include` line as the header writes it, delimiters and all, because
/// that is the token the reader matches on; the **reference** is where the text
/// was resolved from, in the same namespace as [`HeaderIdentity::reference`];
/// the **digest** is measured over the bytes, like the header's and unlike the
/// artifact's.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncludeIdentity {
    /// `<event2/visibility.h>` or `"local.h"` — the `#include` operand, exactly.
    pub spelling: String,
    /// The reference the text is resolved from and supplied under.
    pub reference: String,
    /// `sha256:` followed by 64 lowercase hex digits, as the manifest spells it.
    pub digest: String,
}

/// One header text supplied to this reader, with the digest measured over the
/// bytes it was read from.
///
/// The driver resolves the reference and measures the digest; this module
/// compares and reads. See `HEADER-DIGEST-DRIVER-MEASURED` in the module
/// documentation for why the split is there and not elsewhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeaderSource<'a> {
    /// The reference this text was resolved from.
    pub reference: &'a str,
    /// The header's own text, as read.
    pub text: &'a str,
    /// `sha256:` followed by 64 lowercase hex digits, measured over `text`.
    pub measured_digest: &'a str,
}

/// What one C type position of a header declaration projects to.
///
/// A **set** rather than one spelling, exactly as the gateway's declared-ABI
/// reader projects: plain `char` admits `int8_t` and `uint8_t` because C11
/// 6.2.5p15 leaves its signedness implementation-defined, and `unsigned long`
/// admits `uint64_t` and `uintptr_t` on the LP64 model this profile's
/// `data_model lp64` premise states. The alternative would report a faithful
/// `size_t` transcription as a disagreement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderProjection {
    /// The spellings a manifest entry may write at this position.
    Spellings(BTreeSet<AbiType>),
    /// The type was **read**, and the manifest's ABI vocabulary has no spelling
    /// for it: a function pointer (`fnptr`) or a floating type (`f32`, `f64`),
    /// all three of which §C04.1 spells and
    /// [`crate::extern_call::AbiType`] does not.
    ///
    /// Kept apart from "unreadable" on purpose: collapsing the two would report
    /// a vocabulary gap as a reader gap, and the two have different follow-ons.
    Unspellable(&'static str),
}

impl HeaderProjection {
    /// Whether a manifest's spelling is one of this position's admissible
    /// readings.
    #[must_use]
    pub fn admits(&self, spelling: AbiType) -> bool {
        match self {
            Self::Spellings(spellings) => spellings.contains(&spelling),
            Self::Unspellable(_) => false,
        }
    }
}

/// Why a header binding does not load.
///
/// Every constructor names one condition; none is a catch-all, so widening the
/// subset means deleting a refusal on purpose. A reader boundary
/// ([`Self::UnreadableResult`], [`Self::ConditionalDeclaration`], …) and a
/// statement about the artifact ([`Self::InternalDeclaration`]) are separate
/// constructors even where both refuse, because they mean different things about
/// the library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderRefusal {
    /// The manifest names a header and no text was supplied for that reference.
    NotSupplied { manifest: String, reference: String },
    /// The digest measured over the supplied text is not the digest the manifest
    /// declares.
    DigestMismatch {
        manifest: String,
        reference: String,
        declared: String,
        measured: String,
    },
    /// The manifest declares an include text and no text was supplied for that
    /// reference (card `NATIVE-LIBRARY-HEADER-DECORATION`).
    IncludeNotSupplied {
        manifest: String,
        spelling: String,
        reference: String,
    },
    /// The digest measured over a declared include text is not the digest the
    /// manifest declares. Decided **before** the text is read, exactly as the
    /// header's own digest is: a substituted text that would have been read the
    /// same way is still refused, because the decision must be rederivable from
    /// the identities a certificate names.
    IncludeDigestMismatch {
        manifest: String,
        spelling: String,
        reference: String,
        declared: String,
        measured: String,
    },
    /// A declared include text is never reached: no `#include` of that spelling
    /// occurs in the header or in any text reached from it. A declared text
    /// nothing reads is a statement with no effect, and this level's whole
    /// content is that its statements have effects.
    IncludeNotReached { manifest: String, spelling: String },
    /// A text was supplied for a reference no selected manifest declares, while
    /// the selection binds at least one header. The texts a decision rests on
    /// are exactly the declared ones, so an undeclared text is refused rather
    /// than silently ignored — a reader who believed the transport carried only
    /// declared texts would otherwise be wrong without being told.
    SourceNotDeclared { reference: String },
    /// The header carries a preprocessing directive v1 does not admit.
    UnsupportedDirective {
        manifest: String,
        reference: String,
        directive: String,
    },
    /// The header's conditional regions do not nest to a balanced end.
    UnbalancedConditional { manifest: String, reference: String },
    /// The header declares no such symbol at all.
    UndeclaredSymbol {
        manifest: String,
        reference: String,
        symbol: String,
    },
    /// The header declares the symbol **only** inside a conditional region,
    /// which v1 does not evaluate. Distinct from
    /// [`Self::UndeclaredSymbol`]: the declaration is there, and what is missing
    /// is a reader that decides the condition.
    ConditionalDeclaration {
        manifest: String,
        reference: String,
        symbol: String,
    },
    /// The header defines the symbol as a macro rather than declaring it. A
    /// macro is not an exported symbol of an artifact.
    MacroSymbol {
        manifest: String,
        reference: String,
        symbol: String,
    },
    /// The header declares the symbol `static`, so it has internal linkage and
    /// is not an exported symbol of the artifact at all. **Not** a reader
    /// boundary: binding it to an artifact identity would be a false statement
    /// about the artifact.
    InternalDeclaration {
        manifest: String,
        reference: String,
        symbol: String,
    },
    /// The C declaration's parameter list is empty (`f()`), which specifies
    /// nothing about parameters (C11 6.7.6.3p14). A manifest entry always
    /// asserts a parameter vector — `fn f() -> T` asserts the C declaration is
    /// `f(void)` — so there is something to contradict and nothing to check it
    /// against.
    UnspecifiedParameters {
        manifest: String,
        reference: String,
        symbol: String,
    },
    /// The C declaration is variadic and a manifest entry has no tail to
    /// compare against one.
    VariadicDeclaration {
        manifest: String,
        reference: String,
        symbol: String,
    },
    /// The header declares the symbol more than once at depth zero, with
    /// readings that disagree.
    AmbiguousDeclaration {
        manifest: String,
        reference: String,
        symbol: String,
    },
    /// This reader will not commit to the declaration's result type.
    UnreadableResult {
        manifest: String,
        reference: String,
        symbol: String,
        spelling: String,
    },
    /// This reader will not commit to a declaration parameter's type.
    UnreadableParameter {
        manifest: String,
        reference: String,
        symbol: String,
        parameter: usize,
        spelling: String,
    },
    /// A generated entry declares a C struct position (card
    /// `W48-QRLIB-LINKAGE`), which this bounded reader does not compare: the
    /// FFI route reads a struct's layout with the recorded compiler instead.
    StructEntry {
        manifest: String,
        reference: String,
        symbol: String,
    },
    /// The declaration's type at this position was **read**, and the manifest's
    /// ABI vocabulary cannot spell it (`fnptr`; `f32` and `f64` until card
    /// `W48-QRLIB-LINKAGE` gave them `float` and `double`).
    UnspellableResult {
        manifest: String,
        reference: String,
        symbol: String,
        projection: &'static str,
    },
    /// The same, at a parameter position.
    UnspellableParameter {
        manifest: String,
        reference: String,
        symbol: String,
        parameter: usize,
        projection: &'static str,
    },
}

impl fmt::Display for HeaderRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSupplied {
                manifest,
                reference,
            } => write!(
                f,
                "the manifest `{manifest}` names the exported header `{reference}`, for which no text was supplied"
            ),
            Self::DigestMismatch {
                manifest,
                reference,
                declared,
                measured,
            } => write!(
                f,
                "the manifest `{manifest}` declares the exported header `{reference}` at `{declared}`, and the supplied text measures `{measured}`"
            ),
            Self::IncludeNotSupplied {
                manifest,
                spelling,
                reference,
            } => write!(
                f,
                "the manifest `{manifest}` declares the include text `{spelling}` at `{reference}`, for which no text was supplied"
            ),
            Self::IncludeDigestMismatch {
                manifest,
                spelling,
                reference,
                declared,
                measured,
            } => write!(
                f,
                "the manifest `{manifest}` declares the include text `{spelling}` at `{reference}` with digest `{declared}`, and the supplied text measures `{measured}`"
            ),
            Self::IncludeNotReached { manifest, spelling } => write!(
                f,
                "the manifest `{manifest}` declares the include text `{spelling}`, which no `#include` of the exported header or of a text reached from it names"
            ),
            Self::SourceNotDeclared { reference } => write!(
                f,
                "a text was supplied for `{reference}`, which no selected manifest declares as its exported header or as one of that header's include texts"
            ),
            Self::UnsupportedDirective {
                manifest,
                reference,
                directive,
            } => write!(
                f,
                "the exported header `{reference}` of `{manifest}` carries `#{directive}`, which this reader does not interpret; v1 reads a bounded directive subset rather than half a preprocessor"
            ),
            Self::UnbalancedConditional {
                manifest,
                reference,
            } => write!(
                f,
                "the exported header `{reference}` of `{manifest}` does not close every conditional region it opens"
            ),
            Self::UndeclaredSymbol {
                manifest,
                reference,
                symbol,
            } => write!(
                f,
                "the manifest `{manifest}` binds `{symbol}`, which its exported header `{reference}` does not declare"
            ),
            Self::ConditionalDeclaration {
                manifest,
                reference,
                symbol,
            } => write!(
                f,
                "the exported header `{reference}` of `{manifest}` declares `{symbol}` only inside a conditional region, and v1 evaluates no condition"
            ),
            Self::MacroSymbol {
                manifest,
                reference,
                symbol,
            } => write!(
                f,
                "the exported header `{reference}` of `{manifest}` defines `{symbol}` as a macro; a macro is not an exported symbol of an artifact"
            ),
            Self::InternalDeclaration {
                manifest,
                reference,
                symbol,
            } => write!(
                f,
                "the exported header `{reference}` of `{manifest}` declares `{symbol}` `static`, so it has internal linkage and is not an exported symbol of the artifact"
            ),
            Self::UnspecifiedParameters {
                manifest,
                reference,
                symbol,
            } => write!(
                f,
                "the exported header `{reference}` of `{manifest}` declares `{symbol}` with an empty parameter list, which specifies nothing about its parameters, and the manifest entry asserts a parameter vector"
            ),
            Self::VariadicDeclaration {
                manifest,
                reference,
                symbol,
            } => write!(
                f,
                "the exported header `{reference}` of `{manifest}` declares `{symbol}` variadic, and a manifest entry has no tail to compare against one"
            ),
            Self::AmbiguousDeclaration {
                manifest,
                reference,
                symbol,
            } => write!(
                f,
                "the exported header `{reference}` of `{manifest}` declares `{symbol}` more than once, with readings that disagree"
            ),
            Self::UnreadableResult {
                manifest,
                reference,
                symbol,
                spelling,
            } => write!(
                f,
                "the exported header `{reference}` of `{manifest}` declares `{symbol}` returning `{spelling}`, which this reader does not commit to"
            ),
            Self::UnreadableParameter {
                manifest,
                reference,
                symbol,
                parameter,
                spelling,
            } => write!(
                f,
                "the exported header `{reference}` of `{manifest}` declares parameter {parameter} of `{symbol}` as `{spelling}`, which this reader does not commit to"
            ),
            Self::StructEntry {
                manifest,
                reference,
                symbol,
            } => write!(
                f,
                "`{manifest}` binds `{symbol}` with a C struct position, which this bounded reader of `{reference}` does not compare; a struct's layout is read by the recorded compiler on the FFI route"
            ),
            Self::UnspellableResult {
                manifest,
                reference,
                symbol,
                projection,
            } => write!(
                f,
                "the exported header `{reference}` of `{manifest}` declares `{symbol}` returning a `{projection}`, which this reader reads and a manifest entry cannot spell"
            ),
            Self::UnspellableParameter {
                manifest,
                reference,
                symbol,
                parameter,
                projection,
            } => write!(
                f,
                "the exported header `{reference}` of `{manifest}` declares parameter {parameter} of `{symbol}` as a `{projection}`, which this reader reads and a manifest entry cannot spell"
            ),
        }
    }
}

impl std::error::Error for HeaderRefusal {}

/// How a manifest entry and the exported header's own declaration disagree.
///
/// The three constructors are
/// [`crate::native_library::ManifestDisagreement`]'s, and the type is
/// deliberately **not** that one: the header side of this comparison is
/// *set-valued* (plain `char` admits two spellings, `unsigned long` two more),
/// so reporting it as a single `AbiType` would print a reading the header does
/// not have. One vocabulary, two carriers, and the difference is the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderDisagreement {
    ParameterCount {
        header: usize,
        manifest: usize,
    },
    Parameter {
        parameter: usize,
        header: Vec<AbiType>,
        manifest: AbiType,
    },
    Result {
        header: Vec<AbiType>,
        manifest: AbiType,
    },
}

/// Render a header projection's admissible spellings for a message.
fn spellings(header: &[AbiType]) -> String {
    header
        .iter()
        .map(|abi| format!("`{}`", abi.c_spelling()))
        .collect::<Vec<_>>()
        .join(" or ")
}

impl fmt::Display for HeaderDisagreement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParameterCount { header, manifest } => write!(
                f,
                "the header declares {header} parameter(s) and the manifest declares {manifest}"
            ),
            Self::Parameter {
                parameter,
                header,
                manifest,
            } => write!(
                f,
                "parameter {parameter} is declared {} in the header and `{}` in the manifest",
                spellings(header),
                manifest.c_spelling()
            ),
            Self::Result { header, manifest } => write!(
                f,
                "the result is declared {} in the header and `{}` in the manifest",
                spellings(header),
                manifest.c_spelling()
            ),
        }
    }
}

/// Why a manifest's header binding does not hold: the header could not be read
/// ([`Self::Unreadable`]) or it contradicts the manifest
/// ([`Self::Disagreement`]).
///
/// The two are separate because they are separate diagnostics: the first says
/// this level could not be applied, and the second says it was applied and the
/// manifest lost.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderFailure {
    Unreadable(HeaderRefusal),
    Disagreement {
        manifest: String,
        reference: String,
        symbol: String,
        disagreement: HeaderDisagreement,
    },
}

impl fmt::Display for HeaderFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(refusal) => write!(f, "{refusal}"),
            Self::Disagreement {
                manifest,
                reference,
                symbol,
                disagreement,
            } => write!(
                f,
                "the manifest `{manifest}` declares `{symbol}` otherwise than its exported header `{reference}` does: {disagreement}"
            ),
        }
    }
}

impl std::error::Error for HeaderFailure {}

/// One header declaration, projected into the manifest's ABI vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderDeclaration {
    /// `None` for a type this reader will not commit to, with the spelling it
    /// declined.
    pub result: Result<HeaderProjection, String>,
    pub parameters: Vec<Result<HeaderProjection, String>>,
    pub variadic: bool,
    /// The C parameter list was empty, which specifies no parameters at all.
    pub unspecified: bool,
    /// The declaration carries `static`, so the symbol has internal linkage.
    pub internal: bool,
}

/// What the reader found for one named symbol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderReading {
    /// A depth-zero declaration this reader committed to the shape of.
    Declared(HeaderDeclaration),
    /// The header declares the symbol only inside a conditional region.
    ConditionalOnly,
    /// The header defines the symbol as a macro.
    Macro,
    /// Two depth-zero declarations that project differently.
    Ambiguous,
    /// The header does not declare the symbol at all.
    Absent,
}

/// Read one header's declarations of exactly the named symbols.
///
/// The result is a **function** of the two inputs and reads nothing else, so a
/// certificate consumer repeats it rather than believing it. Symbols the caller
/// does not name are not read at all: a header is the library's whole exported
/// surface, and the manifest is the selection.
///
/// # Errors
///
/// A directive outside the v1 subset, or conditional regions this reader cannot
/// bracket, refuses the whole header — those are properties of the file, not of
/// any one symbol.
pub fn read_header(
    text: &str,
    symbols: &BTreeSet<&str>,
) -> Result<BTreeMap<String, HeaderReading>, HeaderTextRefusal> {
    read_header_with_includes(text, &BTreeMap::new(), symbols).map(|readout| readout.readings)
}

/// What one reading of a header and its declared include texts produced.
///
/// The two fields beside the readings are what card
/// `NATIVE-LIBRARY-HEADER-DECORATION` makes checkable rather than believed:
/// which declared texts were actually reached, and which names the reader
/// **derived** to be erasable declaration decorations from those texts' own
/// `#define`s.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderReadout {
    /// What the reader found for each named symbol.
    pub readings: BTreeMap<String, HeaderReading>,
    /// The include spellings actually followed, in the order they are declared.
    pub followed: BTreeSet<String>,
    /// The names derived to be erasable declaration decorations (§the derivation
    /// in the module documentation). Every one of them was *read* out of a
    /// supplied, hash-identified text; none is declared by a manifest.
    pub erasures: BTreeSet<String>,
}

/// Read one header, following `#include`s of exactly the supplied spellings.
///
/// `included` maps an `#include` operand as the header writes it
/// (`<event2/visibility.h>`, `"local.h"`) to that text. A spelling not in the map
/// is **not followed**, exactly as in card `NATIVE-LIBRARY-HEADER`, so a name one
/// file away stays unreadable rather than guessed. An empty map is that card's
/// behaviour byte for byte.
///
/// # Errors
///
/// As [`read_header`]: a directive outside the v1 subset or conditional regions
/// this reader cannot bracket, in the header **or in any text spliced into it**,
/// refuse the whole reading — the spliced texts are part of the one text the
/// reader is reading.
pub fn read_header_with_includes(
    text: &str,
    included: &BTreeMap<String, &str>,
    symbols: &BTreeSet<&str>,
) -> Result<HeaderReadout, HeaderTextRefusal> {
    let scan = scan_header(text, included)?;
    let followed = scan.followed.clone();
    let erasures = scan.erasures.clone();
    let texts = scan.tokens.iter().map(String::as_str).collect::<Vec<_>>();
    let typedefs = read_typedefs(&texts);
    let mut readings: BTreeMap<String, HeaderReading> = BTreeMap::new();
    for symbol in symbols {
        readings.insert((*symbol).to_owned(), HeaderReading::Absent);
    }
    for index in 0..texts.len() {
        if texts.get(index + 1) != Some(&"(") || !symbols.contains(texts[index]) {
            continue;
        }
        let Some(declaration) = read_declaration(&texts, index, &typedefs) else {
            continue;
        };
        let name = texts[index];
        let reading = match readings.remove(name) {
            Some(HeaderReading::Declared(existing)) => {
                if existing == declaration {
                    HeaderReading::Declared(existing)
                } else {
                    HeaderReading::Ambiguous
                }
            }
            Some(HeaderReading::Ambiguous) => HeaderReading::Ambiguous,
            _ => HeaderReading::Declared(declaration),
        };
        readings.insert(name.to_owned(), reading);
    }
    // A symbol no depth-zero declaration named may still be a macro or a
    // conditional-only declaration. Each is reported as itself rather than as
    // "absent", because the three say different things about the header.
    for symbol in symbols {
        let entry = readings
            .get_mut(*symbol)
            .expect("every named symbol has a row");
        if matches!(entry, HeaderReading::Absent) {
            if scan.macros.contains(*symbol) {
                *entry = HeaderReading::Macro;
            } else if scan.conditional_symbols.contains(*symbol) {
                *entry = HeaderReading::ConditionalOnly;
            }
        } else if scan.macros.contains(*symbol) {
            // A name both declared and `#define`d is a macro at every use site,
            // so the declaration is not what a caller reaches.
            *entry = HeaderReading::Macro;
        }
    }
    Ok(HeaderReadout {
        readings,
        followed,
        erasures,
    })
}

/// Every identifier the header text writes immediately before a `(`, at any
/// conditional depth.
///
/// **Measurement only, and no part of the gate.** The gate reads exactly the
/// symbols a manifest names; this is the candidate set the subset census uses as
/// its denominator, so that a census over a real header is a statement about
/// that header rather than about a list someone curated. A candidate is not a
/// declaration — a call inside a macro body is one too — which is why the census
/// reports [`HeaderReading::Absent`] as its own row instead of hiding it.
#[must_use]
pub fn census_candidates(text: &str) -> BTreeSet<String> {
    let normalized = normalize(text);
    let mut tokens = Vec::new();
    for line in normalized.lines() {
        if directive_of(line).is_some() {
            // A function-like macro's own `#define` line is not a declaration
            // and its name is already reported through `macros`.
            continue;
        }
        tokenize_line(line, &mut tokens);
    }
    let mut candidates = BTreeSet::new();
    for index in 0..tokens.len() {
        if tokens.get(index + 1).map(String::as_str) == Some("(")
            && is_c_identifier(&tokens[index])
            && !RESERVED_NON_SPECIFIERS.contains(&tokens[index].as_str())
            && !ARITHMETIC_WORDS.contains(&tokens[index].as_str())
        {
            candidates.insert(tokens[index].clone());
        }
    }
    candidates
}

/// Why a header's *text* cannot be read at all, independently of any symbol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderTextRefusal {
    /// A directive outside the v1 subset.
    UnsupportedDirective { directive: String },
    /// Conditional regions that do not nest to a balanced end.
    UnbalancedConditional,
}

impl fmt::Display for HeaderTextRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDirective { directive } => {
                write!(f, "the header carries `#{directive}`")
            }
            Self::UnbalancedConditional => {
                write!(f, "the header does not close every conditional region")
            }
        }
    }
}

impl std::error::Error for HeaderTextRefusal {}

/// Check every selected manifest's header binding, or refuse by name.
///
/// `sources` maps a header reference to the text read for it and the digest
/// measured over that text. A manifest that names no header is unaffected —
/// which is the whole wave-19/wave-20 world, byte for byte.
///
/// # Errors
///
/// [`HeaderFailure::Unreadable`] when the binding cannot be applied and
/// [`HeaderFailure::Disagreement`] when it was applied and the manifest's
/// transcription lost. A disagreement **refuses**; see the module documentation
/// for the direction argument.
pub fn check_header_bindings(
    libraries: &NativeLibraries,
    sources: &BTreeMap<String, HeaderSource<'_>>,
) -> Result<(), HeaderFailure> {
    // Transport totality (card `NATIVE-LIBRARY-HEADER-DECORATION`). Once the
    // selection binds a header, the texts this level reads are exactly the
    // declared ones, and a text supplied for a reference nobody declared is
    // refused rather than dropped on the floor. The check is gated on a header
    // being bound so that a selection naming no header stays byte-identical to
    // card `NATIVE-LIBRARY-HEADER`'s — where supplying an unread text is, and
    // remains, harmless.
    let mut declared: BTreeSet<&str> = BTreeSet::new();
    for (_, manifest) in libraries.manifests() {
        if let Some(header) = manifest.header.as_ref() {
            declared.insert(header.reference.as_str());
            declared.extend(
                header
                    .includes
                    .iter()
                    .map(|include| include.reference.as_str()),
            );
        }
    }
    if !declared.is_empty()
        && let Some(reference) = sources
            .keys()
            .find(|reference| !declared.contains(reference.as_str()))
    {
        return Err(HeaderFailure::Unreadable(
            HeaderRefusal::SourceNotDeclared {
                reference: reference.clone(),
            },
        ));
    }
    for (module, manifest) in libraries.manifests() {
        check_manifest_header(module, manifest, sources)?;
    }
    Ok(())
}

/// Check one manifest's header binding, or refuse by name.
///
/// # Errors
///
/// As [`check_header_bindings`].
pub fn check_manifest_header(
    module: &str,
    manifest: &NativeLibraryManifest,
    sources: &BTreeMap<String, HeaderSource<'_>>,
) -> Result<(), HeaderFailure> {
    let Some(header) = manifest.header.as_ref() else {
        return Ok(());
    };
    let unreadable = HeaderFailure::Unreadable;
    let Some(source) = sources.get(&header.reference) else {
        return Err(unreadable(HeaderRefusal::NotSupplied {
            manifest: module.to_owned(),
            reference: header.reference.clone(),
        }));
    };
    if source.measured_digest != header.digest {
        return Err(unreadable(HeaderRefusal::DigestMismatch {
            manifest: module.to_owned(),
            reference: header.reference.clone(),
            declared: header.digest.clone(),
            measured: source.measured_digest.to_owned(),
        }));
    }
    // Card `NATIVE-LIBRARY-HEADER-DECORATION`: each declared include text is
    // resolved and its digest verified **before** it is read, on the same ground
    // the header's own digest is — a decision that rests on a text must name
    // that text by content or it cannot be rederived.
    let mut included: BTreeMap<String, &str> = BTreeMap::new();
    for include in &header.includes {
        let Some(supplied) = sources.get(&include.reference) else {
            return Err(unreadable(HeaderRefusal::IncludeNotSupplied {
                manifest: module.to_owned(),
                spelling: include.spelling.clone(),
                reference: include.reference.clone(),
            }));
        };
        if supplied.measured_digest != include.digest {
            return Err(unreadable(HeaderRefusal::IncludeDigestMismatch {
                manifest: module.to_owned(),
                spelling: include.spelling.clone(),
                reference: include.reference.clone(),
                declared: include.digest.clone(),
                measured: supplied.measured_digest.to_owned(),
            }));
        }
        included.insert(include.spelling.clone(), supplied.text);
    }
    let mut symbols: BTreeSet<&str> = BTreeSet::new();
    symbols.extend(manifest.unchecked.keys().map(String::as_str));
    symbols.extend(manifest.contracted.keys().map(String::as_str));
    let readout =
        read_header_with_includes(source.text, &included, &symbols).map_err(|refusal| {
            unreadable(match refusal {
                HeaderTextRefusal::UnsupportedDirective { directive } => {
                    HeaderRefusal::UnsupportedDirective {
                        manifest: module.to_owned(),
                        reference: header.reference.clone(),
                        directive,
                    }
                }
                HeaderTextRefusal::UnbalancedConditional => HeaderRefusal::UnbalancedConditional {
                    manifest: module.to_owned(),
                    reference: header.reference.clone(),
                },
            })
        })?;
    // A declared text no `#include` reaches is a statement with no effect, and
    // this level's content is that its statements have effects. Refusing it is
    // the same totality argument the contract slice's unresolved binding rests
    // on (`Q047`), applied to the reader's own inputs.
    for include in &header.includes {
        if !readout.followed.contains(&include.spelling) {
            return Err(unreadable(HeaderRefusal::IncludeNotReached {
                manifest: module.to_owned(),
                spelling: include.spelling.clone(),
            }));
        }
    }
    let readings = readout.readings;
    // Both entry kinds are compared identically: what a manifest transcribes is
    // an exported header declaration whichever keyword introduced it, and only
    // the *disposition* of a disagreement ever depended on the kind. Here it
    // does not, because the weaker reading below a withdrawn header binding is
    // the same for both.
    for symbol in &symbols {
        let (parameters, result) = match manifest.unchecked.get(*symbol) {
            Some(entry) => (entry.parameters.clone(), entry.result),
            None => manifest
                .contracted
                .get(*symbol)
                .expect("every named symbol is bound by an entry")
                .abi_signature()
                .ok_or_else(|| {
                    unreadable(HeaderRefusal::StructEntry {
                        manifest: module.to_owned(),
                        reference: header.reference.clone(),
                        symbol: (*symbol).to_owned(),
                    })
                })?,
        };
        compare_reading(
            module,
            &header.reference,
            symbol,
            readings.get(*symbol).expect("every named symbol was read"),
            &parameters,
            result,
        )?;
    }
    Ok(())
}

/// Compare one symbol's manifest entry against the header's own declaration.
fn compare_reading(
    module: &str,
    reference: &str,
    symbol: &str,
    reading: &HeaderReading,
    parameters: &[AbiType],
    result: AbiType,
) -> Result<(), HeaderFailure> {
    let unreadable = HeaderFailure::Unreadable;
    let declaration = match reading {
        HeaderReading::Declared(declaration) => declaration,
        HeaderReading::Absent => {
            return Err(unreadable(HeaderRefusal::UndeclaredSymbol {
                manifest: module.to_owned(),
                reference: reference.to_owned(),
                symbol: symbol.to_owned(),
            }));
        }
        HeaderReading::ConditionalOnly => {
            return Err(unreadable(HeaderRefusal::ConditionalDeclaration {
                manifest: module.to_owned(),
                reference: reference.to_owned(),
                symbol: symbol.to_owned(),
            }));
        }
        HeaderReading::Macro => {
            return Err(unreadable(HeaderRefusal::MacroSymbol {
                manifest: module.to_owned(),
                reference: reference.to_owned(),
                symbol: symbol.to_owned(),
            }));
        }
        HeaderReading::Ambiguous => {
            return Err(unreadable(HeaderRefusal::AmbiguousDeclaration {
                manifest: module.to_owned(),
                reference: reference.to_owned(),
                symbol: symbol.to_owned(),
            }));
        }
    };
    if declaration.internal {
        return Err(unreadable(HeaderRefusal::InternalDeclaration {
            manifest: module.to_owned(),
            reference: reference.to_owned(),
            symbol: symbol.to_owned(),
        }));
    }
    if declaration.variadic {
        return Err(unreadable(HeaderRefusal::VariadicDeclaration {
            manifest: module.to_owned(),
            reference: reference.to_owned(),
            symbol: symbol.to_owned(),
        }));
    }
    if declaration.unspecified {
        return Err(unreadable(HeaderRefusal::UnspecifiedParameters {
            manifest: module.to_owned(),
            reference: reference.to_owned(),
            symbol: symbol.to_owned(),
        }));
    }
    // Arity first, then parameters in order, then the result — the order
    // `check_manifest_agreement` compares in, so the two gates report the same
    // disagreement about the same pair of declarations.
    if declaration.parameters.len() != parameters.len() {
        return Err(HeaderFailure::Disagreement {
            manifest: module.to_owned(),
            reference: reference.to_owned(),
            symbol: symbol.to_owned(),
            disagreement: HeaderDisagreement::ParameterCount {
                header: declaration.parameters.len(),
                manifest: parameters.len(),
            },
        });
    }
    for (index, (projected, spelled)) in declaration.parameters.iter().zip(parameters).enumerate() {
        match projected {
            Err(spelling) => {
                return Err(unreadable(HeaderRefusal::UnreadableParameter {
                    manifest: module.to_owned(),
                    reference: reference.to_owned(),
                    symbol: symbol.to_owned(),
                    parameter: index,
                    spelling: spelling.clone(),
                }));
            }
            Ok(HeaderProjection::Unspellable(projection)) => {
                return Err(unreadable(HeaderRefusal::UnspellableParameter {
                    manifest: module.to_owned(),
                    reference: reference.to_owned(),
                    symbol: symbol.to_owned(),
                    parameter: index,
                    projection,
                }));
            }
            Ok(HeaderProjection::Spellings(header)) => {
                if !header.contains(spelled) {
                    return Err(HeaderFailure::Disagreement {
                        manifest: module.to_owned(),
                        reference: reference.to_owned(),
                        symbol: symbol.to_owned(),
                        disagreement: HeaderDisagreement::Parameter {
                            parameter: index,
                            header: header.iter().copied().collect(),
                            manifest: *spelled,
                        },
                    });
                }
            }
        }
    }
    match &declaration.result {
        Err(spelling) => Err(unreadable(HeaderRefusal::UnreadableResult {
            manifest: module.to_owned(),
            reference: reference.to_owned(),
            symbol: symbol.to_owned(),
            spelling: spelling.clone(),
        })),
        Ok(HeaderProjection::Unspellable(projection)) => {
            Err(unreadable(HeaderRefusal::UnspellableResult {
                manifest: module.to_owned(),
                reference: reference.to_owned(),
                symbol: symbol.to_owned(),
                projection,
            }))
        }
        Ok(HeaderProjection::Spellings(header)) => {
            if header.contains(&result) {
                Ok(())
            } else {
                Err(HeaderFailure::Disagreement {
                    manifest: module.to_owned(),
                    reference: reference.to_owned(),
                    symbol: symbol.to_owned(),
                    disagreement: HeaderDisagreement::Result {
                        header: header.iter().copied().collect(),
                        manifest: result,
                    },
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The lexical and directive layer
// ---------------------------------------------------------------------------

/// What one pass over a header's text yields.
struct HeaderScan {
    /// Tokens of the depth-zero region, in order.
    tokens: Vec<String>,
    /// The include spellings actually followed.
    followed: BTreeSet<String>,
    /// The names derived to be erasable declaration decorations.
    erasures: BTreeSet<String>,
    /// Identifiers standing immediately before a `(` inside a conditional
    /// region this reader does not read. Only their *presence* is used, and
    /// only to tell a conditional-only declaration from an absent one.
    conditional_symbols: BTreeSet<String>,
    /// Every name `#define`d or `#undef`ed anywhere in the header, except the
    /// include guard's own name. A name ever touched by either directive is
    /// treated as a macro: v1 tracks no scope for one, and the conservative
    /// reading is the refusing one.
    macros: BTreeSet<String>,
}

/// The directives v1 interprets. Everything else refuses the whole header:
/// `#pragma` and `#line` change semantics this reader does not model — and
/// `#line` in particular could move attribution, which constraint 12 forbids on
/// the analysed side and this level has no reason to admit on the declared one.
const ADMITTED_DIRECTIVES: &[&str] = &[
    "if", "ifdef", "ifndef", "elif", "else", "endif", "define", "undef", "include",
];

/// Replace every comment with whitespace and perform line splicing.
///
/// Both are lexical: neither can change which declaration a token belongs to,
/// and the newlines of a block comment are preserved so a directive stays
/// line-initial.
fn normalize(text: &str) -> String {
    let spliced = text.replace("\\\n", "");
    let bytes: Vec<char> = spliced.chars().collect();
    let mut out = String::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        let current = bytes[index];
        let next = bytes.get(index + 1).copied();
        if current == '/' && next == Some('*') {
            index += 2;
            while index < bytes.len() {
                if bytes[index] == '*' && bytes.get(index + 1) == Some(&'/') {
                    index += 2;
                    break;
                }
                if bytes[index] == '\n' {
                    out.push('\n');
                }
                index += 1;
            }
            out.push(' ');
            continue;
        }
        if current == '/' && next == Some('/') {
            while index < bytes.len() && bytes[index] != '\n' {
                index += 1;
            }
            continue;
        }
        if current == '"' || current == '\'' {
            let quote = current;
            out.push(current);
            index += 1;
            while index < bytes.len() {
                let inner = bytes[index];
                out.push(inner);
                index += 1;
                if inner == '\\' {
                    if let Some(escaped) = bytes.get(index) {
                        out.push(*escaped);
                        index += 1;
                    }
                    continue;
                }
                if inner == quote {
                    break;
                }
            }
            continue;
        }
        out.push(current);
        index += 1;
    }
    out
}

/// Split one line of C into the token spellings this reader compares.
///
/// Multi-character operators are split into their characters, exactly as the
/// gateway reader's token stream presents them — a variadic tail is the three
/// tokens `.`, `.`, `.` there and here.
fn tokenize_line(line: &str, out: &mut Vec<String>) {
    let characters: Vec<char> = line.chars().collect();
    let mut index = 0usize;
    while index < characters.len() {
        let current = characters[index];
        if current.is_whitespace() {
            index += 1;
            continue;
        }
        if current.is_ascii_alphabetic() || current == '_' {
            let start = index;
            while index < characters.len()
                && (characters[index].is_ascii_alphanumeric() || characters[index] == '_')
            {
                index += 1;
            }
            out.push(characters[start..index].iter().collect());
            continue;
        }
        if current.is_ascii_digit() {
            let start = index;
            while index < characters.len()
                && (characters[index].is_ascii_alphanumeric()
                    || characters[index] == '_'
                    || characters[index] == '.')
            {
                index += 1;
            }
            out.push(characters[start..index].iter().collect());
            continue;
        }
        if current == '"' || current == '\'' {
            let start = index;
            let quote = current;
            index += 1;
            while index < characters.len() {
                let inner = characters[index];
                index += 1;
                if inner == '\\' {
                    index += 1;
                    continue;
                }
                if inner == quote {
                    break;
                }
            }
            out.push(
                characters[start..index.min(characters.len())]
                    .iter()
                    .collect(),
            );
            continue;
        }
        out.push(current.to_string());
        index += 1;
    }
}

/// One directive line, split into its keyword and the rest.
fn directive_of(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix('#')?.trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    Some((rest[..end].to_owned(), rest[end..].trim().to_owned()))
}

/// The include-guard name, when the header is **wrapped whole** in one.
///
/// A leading `#ifndef NAME` / `#define NAME` whose matching `#endif` closes the
/// last directive of the file, with no `#else`/`#elif` at that level, and with
/// no C token outside the region it brackets. This is not an interpretation of a
/// condition: a conditional that wraps the entire file is true on the reading
/// this reader performs — it reads the file once — so the guarded body *is* the
/// file.
///
/// The whole-file requirement is what keeps the rule from admitting a
/// *feature* conditional that happens to be shaped like a guard: a region with
/// C tokens outside it is a region, and its contents stay unread.
fn include_guard(lines: &[&str]) -> Option<String> {
    include_guard_span(lines).map(|(name, _, _, _)| name)
}

/// The include guard's name and the three line indices it occupies: the
/// `#ifndef`, the `#define` and the closing `#endif`.
///
/// Splicing a text into another needs the indices, not only the name: the
/// guarded body *is* the file on this reader's one-pass reading, so a spliced
/// text contributes its body at the depth of the `#include` that named it rather
/// than one level deeper — which would put every declaration of every real
/// third-party header inside an unread conditional region.
#[allow(clippy::type_complexity)]
fn include_guard_span(lines: &[&str]) -> Option<(String, usize, usize, usize)> {
    let directives: Vec<(usize, String, String)> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            directive_of(line).map(|(keyword, rest)| (index, keyword, rest))
        })
        .collect();
    let (open_line, first, name) = directives.first()?;
    if first != "ifndef" || name.is_empty() {
        return None;
    }
    let (define_line, second, defined) = directives.get(1)?;
    if second != "define"
        || !defined
            .split_whitespace()
            .next()
            .is_some_and(|word| word == name)
    {
        return None;
    }
    let mut depth = 0usize;
    let mut close_line = None;
    for (index, (line, keyword, _)) in directives.iter().enumerate() {
        match keyword.as_str() {
            "if" | "ifdef" | "ifndef" => depth += 1,
            "endif" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    if index + 1 != directives.len() {
                        return None;
                    }
                    close_line = Some(*line);
                    break;
                }
            }
            "else" | "elif" if depth == 1 => return None,
            _ => {}
        }
    }
    let close_line = close_line?;
    let outside_is_empty = lines
        .iter()
        .enumerate()
        .filter(|(index, _)| *index < *open_line || *index > close_line)
        .all(|(_, line)| line.trim().is_empty());
    outside_is_empty.then(|| (name.clone(), *open_line, *define_line, close_line))
}

// ---------------------------------------------------------------------------
// Card `NATIVE-LIBRARY-HEADER-DECORATION`: splicing declared texts, and deriving
// which names are erasable declaration decorations from what those texts say.
// ---------------------------------------------------------------------------

/// One text, normalised and with its whole-file include guard removed.
///
/// Removing the guard is not an interpretation of a condition: card
/// `NATIVE-LIBRARY-HEADER` already established that a conditional wrapping an
/// entire file is true on the reading this reader performs, because it reads the
/// file once. What is new here is only *where* the body lands — at the depth of
/// the `#include` that named the text, which is what makes a spliced text's
/// declarations as readable as the including header's own.
fn guard_stripped(text: &str) -> Vec<String> {
    let normalized = normalize(text);
    let lines: Vec<&str> = normalized.lines().collect();
    let Some((_, open, define, close)) = include_guard_span(&lines) else {
        return lines.iter().map(|line| (*line).to_owned()).collect();
    };
    lines
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != open && *index != define && *index != close)
        .map(|(_, line)| (*line).to_owned())
        .collect()
}

/// The guard names of a text, so a guard's own `#define` is not read as a macro
/// definition. Card `NATIVE-LIBRARY-HEADER` excluded the one guard; a spliced
/// reading has one per text.
fn guard_name(text: &str) -> Option<String> {
    let normalized = normalize(text);
    let lines: Vec<&str> = normalized.lines().collect();
    include_guard(&lines)
}

/// How deep a chain of declared `#include`s may nest. Each declared text is
/// spliced **at most once** — which is what an include guard does, and what
/// keeps a cycle finite — so this limit is a second, redundant bound.
const INCLUDE_DEPTH_LIMIT: usize = 16;

/// Splice the declared `#include`s of `text`, and of the texts they reach, into
/// one line stream.
///
/// A spelling in `included` is followed the **first** time it is named and is a
/// no-op afterwards, which is exactly what the include guard of a real header
/// does. A spelling the manifest does not declare is **not followed** — the card
/// `NATIVE-LIBRARY-HEADER` behaviour, unchanged — so every name it would have
/// supplied stays unreadable.
fn splice(
    text: &str,
    included: &BTreeMap<String, &str>,
    followed: &mut BTreeSet<String>,
    guards: &mut BTreeSet<String>,
    depth: usize,
) -> Vec<String> {
    if let Some(name) = guard_name(text) {
        guards.insert(name);
    }
    let mut out = Vec::new();
    for line in guard_stripped(text) {
        let spliced = directive_of(&line).and_then(|(keyword, rest)| {
            (keyword == "include" && depth < INCLUDE_DEPTH_LIMIT)
                .then(|| included.get_key_value(rest.trim()))
                .flatten()
        });
        match spliced {
            Some((spelling, inner)) => {
                let first_time = followed.insert(spelling.clone());
                if first_time {
                    out.extend(splice(inner, included, followed, guards, depth + 1));
                }
            }
            None => out.push(line),
        }
    }
    out
}

/// Attribute names this reader will erase, with the one property that admits
/// them: **none of them changes the declared type, the parameter vector or the
/// linkage name** of the declaration it decorates.
///
/// This list is the reader's own — auditable, fixed, and in this repository —
/// and it is the *only* thing a manifest does not supply. What a manifest
/// supplies is which hash-identified texts to read; which names those texts bind
/// to this vocabulary is **derived from their own `#define`s**. That split is the
/// whole argument of card `NATIVE-LIBRARY-HEADER-DECORATION`: an erasure list in
/// a manifest would be an author's claim about a header's text, and this is a
/// reading of it.
///
/// Everything else refuses by staying in the token stream, where it makes the
/// declaration unreadable under card `NATIVE-LIBRARY-HEADER`'s existing refusal.
/// The names deliberately **absent** are the ones that would make erasure a lie:
/// `mode`, `aligned`, `packed`, `vector_size` and `transparent_union` change the
/// type; `ms_abi`, `sysv_abi`, `regparm`, `stdcall`, `cdecl` and `fastcall`
/// change the calling convention; `alias`, `weak`, `weakref`, `constructor` and
/// `destructor` change what the artifact exports under this name.
const ERASABLE_ATTRIBUTES: &[&str] = &[
    "visibility",
    "dllexport",
    "dllimport",
    "deprecated",
    "unused",
    "used",
    "warn_unused_result",
    "malloc",
    "nonnull",
    "format",
    "always_inline",
    "noinline",
    "noreturn",
    "nothrow",
    "hot",
    "cold",
];

/// The introducers whose parenthesised payload is an attribute list, with the
/// nesting depth at which that list's **names** stand.
///
/// `__attribute__` is written `((…))` and `__declspec` `(…)`, so the names sit
/// one level deeper in the first. Anything nested deeper than the name level is
/// an *argument* of an attribute this reader has already committed to
/// (`format(printf, 2, 3)`), not a second attribute name.
const ATTRIBUTE_INTRODUCERS: &[(&str, usize)] =
    &[("__attribute__", 2), ("__attribute", 2), ("__declspec", 1)];

/// A bare decoration keyword that carries no payload and no type.
const ERASABLE_KEYWORDS: &[&str] = &["__global"];

/// If an erasable decoration starts at `index`, the index just past it.
///
/// Returns `None` for anything this reader does not commit to — including an
/// attribute list carrying a name outside [`ERASABLE_ATTRIBUTES`], which is how
/// a type-changing attribute refuses instead of being erased.
fn erasable_decoration_end(tokens: &[&str], index: usize) -> Option<usize> {
    let head = *tokens.get(index)?;
    if ERASABLE_KEYWORDS.contains(&head) {
        return Some(index + 1);
    }
    let (_, name_depth) = ATTRIBUTE_INTRODUCERS
        .iter()
        .find(|(introducer, _)| *introducer == head)?;
    if tokens.get(index + 1) != Some(&"(") {
        return None;
    }
    let close = matching_close(tokens, index + 1)?;
    let mut depth = 0usize;
    for position in index + 1..close {
        match tokens[position] {
            "(" => depth += 1,
            ")" => depth = depth.saturating_sub(1),
            token
                if depth == *name_depth
                    && is_c_identifier(token)
                    && matches!(tokens[position - 1], "(" | ",")
                    && !ERASABLE_ATTRIBUTES.contains(&token) =>
            {
                return None;
            }
            _ => {}
        }
    }
    Some(close + 1)
}

/// Whether a `#define` body is a token sequence this reader erases.
///
/// `established` carries the names already derived erasable, so a definition
/// whose body is another such name (`#define A B`) is erasable too. The empty
/// body is the common case and the base case at once.
fn erasable_body(tokens: &[String], established: &BTreeSet<String>) -> bool {
    let borrowed = tokens.iter().map(String::as_str).collect::<Vec<_>>();
    let mut index = 0usize;
    while index < borrowed.len() {
        if established.contains(borrowed[index]) {
            index += 1;
            continue;
        }
        match erasable_decoration_end(&borrowed, index) {
            Some(next) => index = next,
            None => return false,
        }
    }
    true
}

/// One `#define` of one name, as the derivation needs it.
struct MacroDefinition {
    line: usize,
    /// A function-like macro is never erased: its expansion depends on the
    /// arguments at each use site, and this reader evaluates no expansion.
    function_like: bool,
    body: Vec<String>,
}

/// Every `#define` and `#undef` of every name in a spliced line stream, plus the
/// first line on which each name is *used* outside a directive.
#[derive(Default)]
struct MacroFacts {
    defined: BTreeMap<String, Vec<MacroDefinition>>,
    undefined: BTreeSet<String>,
    first_use: BTreeMap<String, usize>,
}

fn macro_facts(lines: &[String], guards: &BTreeSet<String>) -> MacroFacts {
    let mut facts = MacroFacts::default();
    for (index, line) in lines.iter().enumerate() {
        if let Some((keyword, rest)) = directive_of(line) {
            match keyword.as_str() {
                "define" => {
                    let mut tokens = Vec::new();
                    tokenize_line(&rest, &mut tokens);
                    let Some(name) = tokens.first().cloned() else {
                        continue;
                    };
                    if guards.contains(&name) || !is_c_identifier(&name) {
                        continue;
                    }
                    // `#define NAME(x)` is function-like only when the `(`
                    // abuts the name; `#define NAME (x)` defines an object-like
                    // macro whose body is a parenthesised expression.
                    let function_like = rest[name.len()..].starts_with('(');
                    facts
                        .defined
                        .entry(name)
                        .or_default()
                        .push(MacroDefinition {
                            line: index,
                            function_like,
                            body: tokens[1..].to_vec(),
                        });
                }
                "undef" => {
                    let mut tokens = Vec::new();
                    tokenize_line(&rest, &mut tokens);
                    if let Some(name) = tokens.first() {
                        facts.undefined.insert(name.clone());
                    }
                }
                _ => {}
            }
            continue;
        }
        let mut tokens = Vec::new();
        tokenize_line(line, &mut tokens);
        for token in tokens {
            facts.first_use.entry(token).or_insert(index);
        }
    }
    facts
}

/// Whether `name` is defined on **every path** through `lines[..end]`.
///
/// No condition is evaluated and none is guessed. The predicate is structural:
/// a conditional group establishes definedness only when it has an `#else` — so
/// that no implicit empty arm escapes — and every one of its arms establishes
/// it. That is what makes the derivation a reading of the text rather than an
/// assumption about the build that will consume it: whichever arm a consumer's
/// configuration selects, the name is defined and its definition is erasable.
fn defined_on_every_path(lines: &[String], name: &str, end: usize) -> bool {
    fn region(lines: &[String], name: &str, mut index: usize, end: usize) -> (bool, usize) {
        let mut defines = false;
        while index < end {
            let Some((keyword, rest)) = directive_of(&lines[index]) else {
                index += 1;
                continue;
            };
            match keyword.as_str() {
                "if" | "ifdef" | "ifndef" => {
                    let (group_defines, next) = group(lines, name, index, end);
                    defines = defines || group_defines;
                    index = next;
                }
                "else" | "elif" | "endif" => break,
                "define" => {
                    if rest
                        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .find(|word| !word.is_empty())
                        == Some(name)
                    {
                        defines = true;
                    }
                    index += 1;
                }
                _ => index += 1,
            }
        }
        (defines, index)
    }

    fn group(lines: &[String], name: &str, open: usize, end: usize) -> (bool, usize) {
        let mut index = open + 1;
        let mut every_arm = true;
        let mut has_else = false;
        let mut closed = false;
        loop {
            let (arm_defines, next) = region(lines, name, index, end);
            every_arm = every_arm && arm_defines;
            index = next;
            if index >= end {
                break;
            }
            match directive_of(&lines[index])
                .map(|(keyword, _)| keyword)
                .unwrap_or_default()
                .as_str()
            {
                "elif" => index += 1,
                "else" => {
                    has_else = true;
                    index += 1;
                }
                "endif" => {
                    closed = true;
                    index += 1;
                    break;
                }
                _ => break,
            }
        }
        (closed && has_else && every_arm, index)
    }

    region(lines, name, 0, end).0
}

/// Derive the erasable declaration decorations of a spliced line stream.
///
/// A name qualifies when **all four** hold, and every one of them is read out of
/// the supplied texts:
///
/// 1. every `#define` of it is **object-like** — a function-like macro's
///    expansion depends on its arguments, which this reader does not evaluate;
/// 2. every one of those definitions' bodies is in the erasure vocabulary
///    ([`erasable_body`]);
/// 3. it is never `#undef`ed, anywhere in the stream;
/// 4. it is defined on **every path** through the text preceding its first use
///    ([`defined_on_every_path`]), so no configuration reaches that use with the
///    name unexpanded.
///
/// Under those four, deleting the name from the token stream is *equal to*
/// expanding it and then deleting the decoration the expansion produced — which
/// is what [`erase_decorations`] does to a decoration written out in full, and
/// what the Lean model states.
///
/// The rule is a least fixed point so that `#define A B` may stand on a `B`
/// already derived; the iteration is bounded by the number of names.
fn derive_erasures(lines: &[String], guards: &BTreeSet<String>) -> BTreeSet<String> {
    let facts = macro_facts(lines, guards);
    let mut erasures: BTreeSet<String> = BTreeSet::new();
    loop {
        let mut grown = false;
        for (name, definitions) in &facts.defined {
            if erasures.contains(name) || facts.undefined.contains(name) {
                continue;
            }
            if definitions
                .iter()
                .any(|definition| definition.function_like)
            {
                continue;
            }
            if !definitions
                .iter()
                .all(|definition| erasable_body(&definition.body, &erasures))
            {
                continue;
            }
            let first_use = facts.first_use.get(name).copied().unwrap_or(lines.len());
            // A definition after the first use cannot reach it, so the
            // definedness question is about the prefix only.
            if definitions
                .iter()
                .all(|definition| definition.line >= first_use)
                || !defined_on_every_path(lines, name, first_use)
            {
                continue;
            }
            erasures.insert(name.clone());
            grown = true;
        }
        if !grown {
            return erasures;
        }
    }
}

/// Delete every erasable decoration from a token stream, whether it arrived as a
/// derived macro name or was written out in full.
///
/// Both directions are deleted by the **one** vocabulary, which is what makes
/// the derived erasure a claim this reader can be held to: a header writing
/// `__attribute__((visibility("default"))) int f(void);` and a header writing
/// `EXPORT int f(void);` with an `EXPORT` derived from a declared text read to
/// exactly the same declaration.
fn erase_decorations(tokens: &mut Vec<String>, erasures: &BTreeSet<String>) {
    // One left-to-right pass over one borrowed view. A decoration's extent is a
    // function of the tokens from its introducer onwards, so removing earlier
    // ones cannot change where a later one ends — which is what lets the whole
    // stream be scanned once rather than re-borrowed at every index.
    let borrowed = tokens.iter().map(String::as_str).collect::<Vec<_>>();
    let mut kept: Vec<String> = Vec::with_capacity(borrowed.len());
    let mut index = 0usize;
    while index < borrowed.len() {
        if erasures.contains(borrowed[index]) {
            index += 1;
            continue;
        }
        match erasable_decoration_end(&borrowed, index) {
            Some(end) => index = end,
            None => {
                kept.push(borrowed[index].to_owned());
                index += 1;
            }
        }
    }
    *tokens = kept;
}

/// Read one header's text into the depth-zero token stream plus the two sets
/// that let a symbol's absence be attributed.
fn scan_header(
    text: &str,
    included: &BTreeMap<String, &str>,
) -> Result<HeaderScan, HeaderTextRefusal> {
    let mut followed: BTreeSet<String> = BTreeSet::new();
    let mut guards: BTreeSet<String> = BTreeSet::new();
    // The whole-file include guard of every text — the header's and each spliced
    // one's — is removed rather than counted as a region, so the readable depth
    // is zero throughout. Card `NATIVE-LIBRARY-HEADER` obtained the same reading
    // for the one text by admitting depth one; the argument is that card's,
    // unchanged, and only the arithmetic moved.
    let lines = splice(text, included, &mut followed, &mut guards, 0);
    let erasures = derive_erasures(&lines, &guards);
    let mut depth = 0usize;
    let mut tokens: Vec<String> = Vec::new();
    let mut conditional: Vec<String> = Vec::new();
    let mut macros: BTreeSet<String> = BTreeSet::new();
    for line in &lines {
        if let Some((keyword, rest)) = directive_of(line) {
            if !ADMITTED_DIRECTIVES.contains(&keyword.as_str()) {
                return Err(HeaderTextRefusal::UnsupportedDirective { directive: keyword });
            }
            match keyword.as_str() {
                "if" | "ifdef" | "ifndef" => depth += 1,
                "endif" => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or(HeaderTextRefusal::UnbalancedConditional)?;
                }
                "define" | "undef" => {
                    if let Some(name) = rest
                        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .find(|w| !w.is_empty())
                        && !guards.contains(name)
                    {
                        macros.insert(name.to_owned());
                    }
                }
                _ => {}
            }
            continue;
        }
        if depth == 0 {
            tokenize_line(line, &mut tokens);
        } else {
            tokenize_line(line, &mut conditional);
        }
    }
    if depth != 0 {
        return Err(HeaderTextRefusal::UnbalancedConditional);
    }
    erase_decorations(&mut tokens, &erasures);
    erase_decorations(&mut conditional, &erasures);
    let mut conditional_symbols = BTreeSet::new();
    for index in 0..conditional.len() {
        if conditional.get(index + 1).map(String::as_str) == Some("(")
            && is_c_identifier(&conditional[index])
        {
            conditional_symbols.insert(conditional[index].clone());
        }
    }
    Ok(HeaderScan {
        tokens,
        followed,
        erasures,
        conditional_symbols,
        macros,
    })
}

// ---------------------------------------------------------------------------
// The declaration reader
//
// From here down the algorithm is the gateway declared-ABI reader's, ported
// rather than re-invented: the same specifier tables, the same typedef targets
// (cards CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF and ABI-TYPEDEF-ADDRESS-OF-MEMBER),
// the same array/K&R/nested-declarator refusals, and the same set-valued
// arithmetic projection. The divergences are deliberate and censused in the card
// report: this reader projects into the *manifest's* vocabulary rather than the
// contract loader's, and it decides `static` and the unspecified parameter list
// differently because it is reading an artifact's exported surface rather than a
// translation unit. The crate layering forbids sharing the code itself —
// `qcore-core` has no dependencies and the gateway's reader sits above the
// contract loader — and retiring either reader for the other needs invariant 3
// parity first (follow-on `NATIVE-LIBRARY-HEADER-ONE-READER`).
// ---------------------------------------------------------------------------

fn is_c_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// Storage-class, function and inline specifiers that carry no type. `static`
/// is **not** here: this reader decides on it (internal linkage), so it must
/// reach the declaration record rather than being discarded.
const DISCARDED_SPECIFIERS: &[&str] = &[
    "extern",
    "inline",
    "__inline",
    "__inline__",
    "register",
    "auto",
    "_Noreturn",
    "typedef",
    "static",
];

/// Qualifiers that do not change the ABI projection of the type they qualify.
const QUALIFIERS: &[&str] = &[
    "const",
    "volatile",
    "restrict",
    "__restrict",
    "__restrict__",
];

/// Declaration suffixes this reader steps over: they carry no type.
///
/// `__attribute__` and `__attribute` **left this list** in card
/// `NATIVE-LIBRARY-HEADER-DECORATION`, and that is a strengthening rather than a
/// loss: they are now erased by the one whitelisted decoration vocabulary
/// ([`ERASABLE_ATTRIBUTES`]) in *either* position, so a suffix
/// `__attribute__((mode(DI)))` — which changes the declared type — no longer
/// steps silently over. What stays here is the asm-label form, which is stepped
/// over unread and whose residual is recorded as `HEADER-ASM-LABEL-UNREAD`: an
/// asm label renames the exported symbol, so a manifest binding the source name
/// would be naming a symbol the artifact does not export. Tightening it is
/// follow-on `NATIVE-LIBRARY-HEADER-ASM-LABEL`.
const SUFFIX_INTRODUCERS: &[&str] = &["__asm__", "asm", "__asm"];

/// Identifiers that are reserved words and can never be a type specifier or a
/// typedef name, so meeting one ends the specifier prefix.
const RESERVED_NON_SPECIFIERS: &[&str] = &[
    "return",
    "if",
    "else",
    "while",
    "for",
    "do",
    "switch",
    "case",
    "default",
    "break",
    "continue",
    "goto",
    "sizeof",
    "_Alignof",
    "alignof",
    "typeof",
    "__typeof",
    "__typeof__",
];

const ARITHMETIC_WORDS: &[&str] = &[
    "void", "char", "short", "int", "long", "float", "double", "signed", "unsigned", "_Bool",
    "bool", "_Complex",
];

/// How deep a typedef chain may be followed before the reader gives up.
const TYPEDEF_DEPTH_LIMIT: usize = 8;

/// The **`HEADER-STDINT-PRELUDE`**: the typedef names C11 itself fixes, with the
/// readings the standard fixes them to.
///
/// This is the one place where the reader commits to a name it did not see
/// defined, and it is a reading of the *standard* rather than a guess about an
/// implementation. `<stdint.h>` is a system header the manifest's own canonical
/// spellings come from — an exported header writing `uint8_t *` is writing the
/// §C04.1 vocabulary — so without this prelude every real declaration would be
/// unreadable for want of a file this reader deliberately does not follow, and
/// the level would decide nothing at all.
///
/// Two classes, and the difference is stated rather than smoothed over:
///
/// * **Exact-width names** (C11 7.20.1.1): `intN_t`/`uintN_t` are specified to
///   be exactly `N` bits, two's complement, no padding. Their reading is the
///   standard's and rests on nothing else.
/// * **Pointer- and size-carrying names** (7.20.1.4, 7.19): `intptr_t`,
///   `uintptr_t`, `size_t` and `ptrdiff_t` are *some* integer type of
///   sufficient width, which the standard does not fix. Their reading here is
///   the LP64 one — and LP64 is not assumed, it is the manifest's own declared
///   `data_model lp64` premise (§4.4), which is exactly where a target
///   assumption belongs.
///
/// Names the standard leaves open in width — `intmax_t`, `int_least*_t`,
/// `int_fast*_t`, `wchar_t` — and names that are not C11's at all (`ssize_t`,
/// `off_t`) are **absent**, and therefore unreadable. The prelude is consulted
/// **only** for a name the header itself does not typedef: a header that
/// redefines one of these names is read on its own terms, and if that reading is
/// unresolvable the declaration refuses rather than falling back here.
///
/// The readings are the *sets* [`project_arithmetic`] yields for the
/// corresponding arithmetic types, so this reader and the gateway's declared-ABI
/// reader compare at one granularity: an opaque C ABI projection, in which
/// `int64_t` and `intptr_t` are two spellings of one eight-byte signed reading
/// on the audited targets. Narrowing that here would make the manifest gate
/// stricter than the gateway gate on the same pair of texts, which is a
/// divergence with no argument behind it.
const STANDARD_TYPEDEFS: &[(&str, &[AbiType])] = &[
    ("int8_t", &[AbiType::I8]),
    ("int16_t", &[AbiType::I16]),
    ("int32_t", &[AbiType::I32]),
    ("int64_t", &[AbiType::I64, AbiType::Isize]),
    ("uint8_t", &[AbiType::U8]),
    ("uint16_t", &[AbiType::U16]),
    ("uint32_t", &[AbiType::U32]),
    ("uint64_t", &[AbiType::U64, AbiType::Usize]),
    ("intptr_t", &[AbiType::I64, AbiType::Isize]),
    ("uintptr_t", &[AbiType::U64, AbiType::Usize]),
    ("ptrdiff_t", &[AbiType::I64, AbiType::Isize]),
    ("size_t", &[AbiType::U64, AbiType::Usize]),
];

/// The prelude reading of one name, when the header itself does not typedef it.
fn standard_typedef(
    name: &str,
    typedefs: &BTreeMap<String, TypedefTarget>,
) -> Option<&'static [AbiType]> {
    if typedefs.contains_key(name) {
        return None;
    }
    STANDARD_TYPEDEFS
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, spellings)| *spellings)
}

/// What one `typedef` name stands for, as far as this reader will commit.
#[derive(Clone, Debug, PartialEq, Eq)]
enum TypedefTarget {
    Declarator {
        specifiers: Vec<String>,
        stars: usize,
    },
    Aggregate {
        stars: usize,
    },
    FunctionPointer,
    Unresolved,
}

fn read_typedefs(texts: &[&str]) -> BTreeMap<String, TypedefTarget> {
    let mut typedefs: BTreeMap<String, TypedefTarget> = BTreeMap::new();
    for index in 0..texts.len() {
        if texts[index] != "typedef" {
            continue;
        }
        let Some(end) = typedef_end(texts, index + 1) else {
            continue;
        };
        let slice = &texts[index + 1..end];
        let Some((name, resolved)) = read_typedef_declarator(slice) else {
            continue;
        };
        match typedefs.get(name) {
            Some(existing) if *existing == resolved => {}
            Some(_) => {
                typedefs.insert(name.to_owned(), TypedefTarget::Unresolved);
            }
            None => {
                typedefs.insert(name.to_owned(), resolved);
            }
        }
    }
    typedefs
}

fn typedef_end(texts: &[&str], start: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(start) {
        match *text {
            "{" | "(" | "[" => depth += 1,
            "}" | ")" | "]" => depth = depth.checked_sub(1)?,
            ";" if depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn read_typedef_declarator<'a>(slice: &[&'a str]) -> Option<(&'a str, TypedefTarget)> {
    if let Some(read) = read_function_pointer_typedef(slice) {
        return Some(read);
    }
    if let Some(read) = read_aggregate_typedef(slice) {
        return Some(read);
    }
    let last = slice.last().copied().filter(|last| is_c_identifier(last))?;
    let body = &slice[..slice.len() - 1];
    let resolved = if body
        .iter()
        .any(|text| matches!(*text, "(" | ")" | "[" | "]" | "," | "{" | "}"))
    {
        TypedefTarget::Unresolved
    } else {
        let stars = body.iter().rev().take_while(|text| **text == "*").count();
        let specifiers = &body[..body.len() - stars];
        if specifiers.contains(&"*") {
            TypedefTarget::Unresolved
        } else {
            TypedefTarget::Declarator {
                specifiers: specifiers.iter().map(|text| (*text).to_owned()).collect(),
                stars,
            }
        }
    };
    Some((last, resolved))
}

fn read_function_pointer_typedef<'a>(slice: &[&'a str]) -> Option<(&'a str, TypedefTarget)> {
    if slice.last() != Some(&")") {
        return None;
    }
    let parameters_open = matching_open(slice, slice.len() - 1)?;
    let declarator_close = parameters_open.checked_sub(1)?;
    if slice[declarator_close] != ")" {
        return None;
    }
    let declarator_open = matching_open(slice, declarator_close)?;
    let ["*", name] = &slice[declarator_open + 1..declarator_close] else {
        return None;
    };
    if !is_c_identifier(name) {
        return None;
    }
    let head = &slice[..declarator_open];
    if head.is_empty()
        || head
            .iter()
            .any(|text| matches!(*text, "(" | ")" | "[" | "]" | "," | "{" | "}"))
    {
        return None;
    }
    Some((name, TypedefTarget::FunctionPointer))
}

fn read_aggregate_typedef<'a>(slice: &[&'a str]) -> Option<(&'a str, TypedefTarget)> {
    let kind = *slice.first()?;
    if !matches!(kind, "struct" | "union" | "enum") {
        return None;
    }
    let (tag, brace) = match (slice.get(1), slice.get(2)) {
        (Some(&"{"), _) => (None, 1),
        (Some(tag), Some(&"{")) if is_c_identifier(tag) => (Some(*tag), 2),
        _ => return None,
    };
    let close = matching_close(slice, brace)?;
    let rest = &slice[close + 1..];
    let stars = rest.iter().take_while(|text| **text == "*").count();
    let [name] = &rest[stars..] else {
        return None;
    };
    if !is_c_identifier(name) {
        return None;
    }
    let target = match tag {
        Some(tag) => TypedefTarget::Declarator {
            specifiers: vec![kind.to_owned(), tag.to_owned()],
            stars,
        },
        None => TypedefTarget::Aggregate { stars },
    };
    Some((name, target))
}

/// Read the declaration whose declarator name stands at `index`, if the tokens
/// there really are a function declaration or definition.
fn read_declaration(
    texts: &[&str],
    index: usize,
    typedefs: &BTreeMap<String, TypedefTarget>,
) -> Option<HeaderDeclaration> {
    let close = matching_close(texts, index + 1)?;
    let mut after = close + 1;
    while let Some(introducer) = texts.get(after) {
        if !SUFFIX_INTRODUCERS.contains(introducer) || texts.get(after + 1) != Some(&"(") {
            break;
        }
        after = matching_close(texts, after + 1)? + 1;
    }
    if !matches!(texts.get(after), Some(&";") | Some(&"{")) {
        return None;
    }
    let mut start = index;
    while start > 0 {
        let previous = texts[start - 1];
        if previous == "*"
            || QUALIFIERS.contains(&previous)
            || DISCARDED_SPECIFIERS.contains(&previous)
            || matches!(previous, "struct" | "union" | "enum")
            || (is_c_identifier(previous) && !RESERVED_NON_SPECIFIERS.contains(&previous))
        {
            start -= 1;
            continue;
        }
        break;
    }
    if start == index {
        return None;
    }
    let prefix = &texts[start..index];
    if prefix.contains(&"typedef") {
        return None;
    }
    let internal = prefix.contains(&"static");
    // A decoration this reader does not erase stands **immediately before** the
    // specifier prefix and ends in `)`, which is where the backward walk above
    // stops. Card `NATIVE-LIBRARY-HEADER-DECORATION` found that it stopped there
    // *silently*, so `__attribute__((mode(__DI__))) uint8_t *f(…)` was read as
    // `uint8_t *` — an attribute that changes the declared type, ignored. The
    // erasable vocabulary is deleted from the token stream before this point, so
    // anything still standing here is a decoration this reader does not commit
    // to, and it makes the **result** unreadable by name.
    //
    // The result rather than the whole record, deliberately: a fabricated
    // parameter list would be compared for arity first — the order
    // [`compare_reading`] uses, which is `check_manifest_agreement`'s — and would
    // report a *disagreement*, a claim that the header contradicts the manifest,
    // where the truth is that this reader could not read the declaration.
    let result = match undecided_decoration(texts, start) {
        Some(decoration) => Err(decoration),
        None => project_declarator(prefix, typedefs, true).ok_or_else(|| prefix.join(" ")),
    };
    let parameters = &texts[index + 2..close];
    if parameters.is_empty() {
        return Some(HeaderDeclaration {
            result,
            parameters: Vec::new(),
            variadic: false,
            unspecified: texts.get(after) != Some(&"{"),
            internal,
        });
    }
    if parameters == ["void"] {
        return Some(HeaderDeclaration {
            result,
            parameters: Vec::new(),
            variadic: false,
            unspecified: false,
            internal,
        });
    }
    let mut groups: Vec<Vec<&str>> = Vec::new();
    let mut depth = 0usize;
    let mut current: Vec<&str> = Vec::new();
    for text in parameters {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                groups.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(text);
    }
    groups.push(current);
    let mut projected = Vec::new();
    let mut variadic = false;
    for group in &groups {
        // A tail in any position marks the declaration variadic, which refuses
        // by name. Position is not checked here: `f(int, ..., int)` is not C,
        // and reading it as "not a declaration of this name" would leave the
        // symbol reported *absent* — an attribution this reader will not make
        // for a declaration it can plainly see.
        if group.as_slice() == [".", ".", "."] {
            variadic = true;
            continue;
        }
        projected.push(project_parameter(group, typedefs).ok_or_else(|| group.join(" ")));
    }
    Some(HeaderDeclaration {
        result,
        parameters: projected,
        variadic,
        unspecified: false,
        internal,
    })
}

/// A decoration ending just before `start` that this reader does not erase.
///
/// Returns its spelling so the refusal names the token, exactly as an unreadable
/// type does. Only the attribute introducers are looked for: a bare identifier
/// this reader does not know is already walked into the specifier prefix, where
/// it refuses as an unresolvable typedef name.
fn undecided_decoration(texts: &[&str], start: usize) -> Option<String> {
    let close = start.checked_sub(1)?;
    if *texts.get(close)? != ")" {
        return None;
    }
    let open = matching_open(texts, close)?;
    let introducer = *texts.get(open.checked_sub(1)?)?;
    ATTRIBUTE_INTRODUCERS
        .iter()
        .any(|(candidate, _)| *candidate == introducer)
        .then(|| texts[open - 1..=close].join(" "))
}

fn project_parameter(
    group: &[&str],
    typedefs: &BTreeMap<String, TypedefTarget>,
) -> Option<HeaderProjection> {
    if group.iter().any(|text| *text == "(" || *text == ")") {
        return None;
    }
    let mut rest = group.to_vec();
    let mut array_levels = 0usize;
    while rest.last() == Some(&"]") {
        let open = rest.iter().rposition(|text| *text == "[")?;
        let bounds = &rest[open + 1..rest.len() - 1];
        if bounds.len() > 1
            || bounds
                .iter()
                .any(|text| !text.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return None;
        }
        rest.truncate(open);
        array_levels += 1;
    }
    if array_levels > 1 {
        return None;
    }
    if let Some(last) = rest.last().copied()
        && rest.len() > 1
        && is_c_identifier(last)
        && !ARITHMETIC_WORDS.contains(&last)
        && !QUALIFIERS.contains(&last)
        && !typedefs.contains_key(last)
        && standard_typedef(last, typedefs).is_none()
        && !matches!(rest[rest.len() - 2], "struct" | "union" | "enum")
    {
        rest.pop();
    }
    if array_levels == 1 {
        rest.push("*");
    }
    project_declarator(&rest, typedefs, false)
}

/// The pointer projection of `stars` levels over a pointee that is or is not
/// `const`-qualified.
fn pointer_projection(stars: usize, const_pointee: bool) -> Option<HeaderProjection> {
    let spelling = match (stars, const_pointee) {
        (1, false) => AbiType::PtrMutU8,
        (1, true) => AbiType::PtrConstU8,
        (2, false) => AbiType::PtrMutPtrMutU8,
        _ => return None,
    };
    Some(HeaderProjection::Spellings(BTreeSet::from([spelling])))
}

fn project_declarator(
    words: &[&str],
    typedefs: &BTreeMap<String, TypedefTarget>,
    result: bool,
) -> Option<HeaderProjection> {
    let stars = words.iter().rev().take_while(|word| **word == "*").count();
    let head = &words[..words.len() - stars];
    if stars > 0 && head.contains(&"*") {
        return None;
    }
    let specifiers = head
        .iter()
        .copied()
        .filter(|word| !DISCARDED_SPECIFIERS.contains(word))
        .collect::<Vec<_>>();
    let const_pointee = specifiers.contains(&"const");
    let core = specifiers
        .iter()
        .copied()
        .filter(|word| !QUALIFIERS.contains(word))
        .collect::<Vec<_>>();
    if stars > 0 {
        let pointee_readable = match core.as_slice() {
            [] => false,
            ["struct", tag] | ["union", tag] | ["enum", tag] => is_c_identifier(tag),
            ["struct"] | ["union"] | ["enum"] => false,
            // A prelude name is a readable pointee: `uint8_t *` is the
            // manifest's own canonical pointer spelling, and a header that
            // writes it is writing the §C04.1 vocabulary.
            [name] if standard_typedef(name, typedefs).is_some() => true,
            [name] if !ARITHMETIC_WORDS.contains(name) => {
                match resolve_typedef(name, typedefs, 0) {
                    Some(TypedefTarget::Declarator {
                        specifiers,
                        stars: target_stars,
                    }) => {
                        let target = specifiers.iter().map(String::as_str).collect::<Vec<_>>();
                        return project_declarator_resolved(
                            &target,
                            target_stars + stars,
                            const_pointee,
                            typedefs,
                            result,
                        );
                    }
                    Some(TypedefTarget::Aggregate {
                        stars: target_stars,
                    }) => return pointer_projection(target_stars + stars, const_pointee),
                    // `ABI-FNPTR-DEPTH1`: a pointer *to* a function pointer is
                    // an object pointer, and projecting it would put a function
                    // pointer's storage into a custody-bearing spelling.
                    Some(TypedefTarget::FunctionPointer) => return None,
                    Some(TypedefTarget::Unresolved) | None => false,
                }
            }
            rest => rest.iter().all(|word| ARITHMETIC_WORDS.contains(word)),
        };
        if !pointee_readable {
            return None;
        }
        return pointer_projection(stars, const_pointee);
    }
    project_scalar(&core, typedefs, result, 0)
}

fn project_declarator_resolved(
    target: &[&str],
    stars: usize,
    const_pointee: bool,
    typedefs: &BTreeMap<String, TypedefTarget>,
    result: bool,
) -> Option<HeaderProjection> {
    if stars == 0 {
        let core = target
            .iter()
            .copied()
            .filter(|word| !QUALIFIERS.contains(word) && !DISCARDED_SPECIFIERS.contains(word))
            .collect::<Vec<_>>();
        return project_scalar(&core, typedefs, result, 0);
    }
    let const_pointee = const_pointee || target.contains(&"const");
    let core = target
        .iter()
        .copied()
        .filter(|word| !QUALIFIERS.contains(word) && !DISCARDED_SPECIFIERS.contains(word))
        .collect::<Vec<_>>();
    let readable = match core.as_slice() {
        [] => false,
        ["struct", tag] | ["union", tag] | ["enum", tag] => is_c_identifier(tag),
        ["struct"] | ["union"] | ["enum"] => false,
        [name] if standard_typedef(name, typedefs).is_some() => true,
        [name] if !ARITHMETIC_WORDS.contains(name) => !matches!(
            resolve_typedef(name, typedefs, 0),
            None | Some(TypedefTarget::Unresolved)
        ),
        rest => rest.iter().all(|word| ARITHMETIC_WORDS.contains(word)),
    };
    if !readable {
        return None;
    }
    pointer_projection(stars, const_pointee)
}

fn resolve_typedef(
    name: &str,
    typedefs: &BTreeMap<String, TypedefTarget>,
    depth: usize,
) -> Option<TypedefTarget> {
    if depth >= TYPEDEF_DEPTH_LIMIT {
        return None;
    }
    let (specifiers, stars) = match typedefs.get(name)? {
        TypedefTarget::Unresolved => return None,
        TypedefTarget::FunctionPointer => return Some(TypedefTarget::FunctionPointer),
        TypedefTarget::Aggregate { stars } => {
            return Some(TypedefTarget::Aggregate { stars: *stars });
        }
        TypedefTarget::Declarator { specifiers, stars } => (specifiers, *stars),
    };
    let core = specifiers
        .iter()
        .map(String::as_str)
        .filter(|word| !QUALIFIERS.contains(word) && !DISCARDED_SPECIFIERS.contains(word))
        .collect::<Vec<_>>();
    if let [inner] = core.as_slice()
        && !ARITHMETIC_WORDS.contains(inner)
        && !matches!(*inner, "struct" | "union" | "enum")
    {
        match resolve_typedef(inner, typedefs, depth + 1) {
            Some(TypedefTarget::Declarator {
                specifiers: deeper,
                stars: deeper_stars,
            }) => {
                return Some(TypedefTarget::Declarator {
                    specifiers: deeper,
                    stars: deeper_stars + stars,
                });
            }
            Some(TypedefTarget::Aggregate {
                stars: deeper_stars,
            }) => {
                return Some(TypedefTarget::Aggregate {
                    stars: deeper_stars + stars,
                });
            }
            Some(TypedefTarget::FunctionPointer) if stars == 0 => {
                return Some(TypedefTarget::FunctionPointer);
            }
            Some(TypedefTarget::FunctionPointer) => return None,
            Some(TypedefTarget::Unresolved) | None => {}
        }
    }
    Some(TypedefTarget::Declarator {
        specifiers: specifiers.clone(),
        stars,
    })
}

/// Project a multiset of arithmetic type specifiers.
///
/// C11 6.7.2p2 lists the admissible specifier sets *as multisets*, so counting
/// rather than sequence-matching is the faithful reading — which is what stops a
/// perfectly faithful `long unsigned int` declaration being reported unreadable.
/// `long double` is deliberately absent: this vocabulary has no spelling for it,
/// so it stays unreadable rather than being folded onto `f64`.
fn project_arithmetic(core: &[&str], result: bool) -> Option<HeaderProjection> {
    let mut signed = 0usize;
    let mut unsigned = 0usize;
    let mut char_words = 0usize;
    let mut short_words = 0usize;
    let mut long_words = 0usize;
    let mut int_words = 0usize;
    let mut float_words = 0usize;
    let mut double_words = 0usize;
    let mut bool_words = 0usize;
    let mut void_words = 0usize;
    for word in core {
        match *word {
            "signed" => signed += 1,
            "unsigned" => unsigned += 1,
            "char" => char_words += 1,
            "short" => short_words += 1,
            "long" => long_words += 1,
            "int" => int_words += 1,
            "float" => float_words += 1,
            "double" => double_words += 1,
            "_Bool" | "bool" => bool_words += 1,
            "void" => void_words += 1,
            _ => return None,
        }
    }
    let set = |entries: &[AbiType]| {
        Some(HeaderProjection::Spellings(
            entries.iter().copied().collect::<BTreeSet<_>>(),
        ))
    };
    let integer = |signed_spelling: &[AbiType], unsigned_spelling: &[AbiType]| {
        if unsigned == 1 && signed == 0 {
            set(unsigned_spelling)
        } else if unsigned == 0 && signed <= 1 {
            set(signed_spelling)
        } else {
            None
        }
    };
    match (
        void_words,
        bool_words,
        float_words,
        double_words,
        char_words,
        short_words,
        long_words,
        int_words,
    ) {
        (1, 0, 0, 0, 0, 0, 0, 0) if result && signed == 0 && unsigned == 0 => set(&[AbiType::Unit]),
        (0, 1, 0, 0, 0, 0, 0, 0) if signed == 0 && unsigned == 0 => set(&[AbiType::Bool]),
        // `float` and `double`: card `W48-QRLIB-LINKAGE` gave the manifest's
        // ABI vocabulary their spellings (`W48-QR-FLOAT-QRLIB-SPELLING`), so a
        // read float is now compared rather than reported unspellable.
        (0, 0, 1, 0, 0, 0, 0, 0) if signed == 0 && unsigned == 0 => set(&[AbiType::F32]),
        (0, 0, 0, 1, 0, 0, 0, 0) if signed == 0 && unsigned == 0 => set(&[AbiType::F64]),
        (0, 0, 0, 0, 1, 0, 0, 0) => {
            if signed == 1 && unsigned == 0 {
                set(&[AbiType::I8])
            } else if unsigned == 1 && signed == 0 {
                set(&[AbiType::U8])
            } else if signed == 0 && unsigned == 0 {
                set(&[AbiType::I8, AbiType::U8])
            } else {
                None
            }
        }
        (0, 0, 0, 0, 0, 1, 0, 0 | 1) => integer(&[AbiType::I16], &[AbiType::U16]),
        (0, 0, 0, 0, 0, 0, 1 | 2, 0 | 1) => integer(
            &[AbiType::I64, AbiType::Isize],
            &[AbiType::U64, AbiType::Usize],
        ),
        (0, 0, 0, 0, 0, 0, 0, 1) => integer(&[AbiType::I32], &[AbiType::U32]),
        (0, 0, 0, 0, 0, 0, 0, 0) if signed + unsigned == 1 => {
            integer(&[AbiType::I32], &[AbiType::U32])
        }
        _ => None,
    }
}

fn project_scalar(
    core: &[&str],
    typedefs: &BTreeMap<String, TypedefTarget>,
    result: bool,
    depth: usize,
) -> Option<HeaderProjection> {
    if depth >= TYPEDEF_DEPTH_LIMIT {
        return None;
    }
    if let Some(projection) = project_arithmetic(core, result) {
        return Some(projection);
    }
    if let [name] = core
        && let Some(spellings) = standard_typedef(name, typedefs)
    {
        return Some(HeaderProjection::Spellings(
            spellings.iter().copied().collect(),
        ));
    }
    match core {
        [name]
            if !ARITHMETIC_WORDS.contains(name)
                && !matches!(*name, "struct" | "union" | "enum") =>
        {
            match resolve_typedef(name, typedefs, 0)? {
                TypedefTarget::Declarator {
                    specifiers,
                    stars: 0,
                } => {
                    let target = specifiers
                        .iter()
                        .map(String::as_str)
                        .filter(|word| {
                            !QUALIFIERS.contains(word) && !DISCARDED_SPECIFIERS.contains(word)
                        })
                        .collect::<Vec<_>>();
                    project_scalar(&target, typedefs, result, depth + 1)
                }
                TypedefTarget::Declarator { specifiers, stars } => {
                    let target = specifiers.iter().map(String::as_str).collect::<Vec<_>>();
                    project_declarator_resolved(&target, stars, false, typedefs, result)
                }
                // An aggregate *value* has no spelling in this vocabulary, for
                // the same reason a `struct tag` value has none.
                TypedefTarget::Aggregate { stars: 0 } => None,
                TypedefTarget::Aggregate { stars } => pointer_projection(stars, false),
                // Read, and unspellable in a manifest entry.
                TypedefTarget::FunctionPointer => Some(HeaderProjection::Unspellable("fnptr")),
                TypedefTarget::Unresolved => None,
            }
        }
        // An enumeration's compatible integer type is implementation-defined
        // (C11 6.7.2.2p4), so no spelling of this vocabulary is established.
        _ => None,
    }
}

fn matching_close(texts: &[&str], open: usize) -> Option<usize> {
    let (opener, closer) = match *texts.get(open)? {
        "(" => ("(", ")"),
        "[" => ("[", "]"),
        "{" => ("{", "}"),
        _ => return None,
    };
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        if *text == opener {
            depth += 1;
        } else if *text == closer {
            if depth == 0 {
                return None;
            }
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn matching_open(texts: &[&str], close: usize) -> Option<usize> {
    let (opener, closer) = match *texts.get(close)? {
        ")" => ("(", ")"),
        "]" => ("[", "]"),
        "}" => ("{", "}"),
        _ => return None,
    };
    let mut depth = 0usize;
    for index in (0..=close).rev() {
        let text = texts[index];
        if text == closer {
            depth += 1;
        } else if text == opener {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "../tests/unit/native_library_header.rs"]
mod tests;
