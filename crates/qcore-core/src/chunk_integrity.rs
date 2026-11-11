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

//! Card O01: the bounded chunk-file integrity observation relation.
//!
//! This module decides one relation over *recorded* executions of the pinned
//! ChunkIO reopen path under `CIO_CHECKSUM`: which class the observed on-disk
//! image belongs to, which operation (if any) was suppressed by the harness,
//! and how the observed endpoint and allocation custody is accounted for.
//!
//! Three separations are structural, not conventions:
//!
//! * **Corrupted input and injected operation failure are different axes.**
//!   A [`ContentClass`] is derived from the recorded bytes alone, by repeating
//!   the pinned layout and checksum rules here; an [`Injection`] is a statement
//!   about the harness's wrapper. Neither is inferred from the other, and a
//!   record that claims one because the other happened is refused.
//! * **A library release is never a harness reclamation.** The two are counted
//!   in separate fields, both must be present, and a case that reports a
//!   suppressed close as a library close is refused.
//! * **A suppressed call is not an OS post-state.** A suppressed `close`
//!   returns its fixed error without reaching the kernel, so the retained
//!   descriptor is a property of the wrapper. Nothing here yields a
//!   `qcore_core::os_endpoint` post-state, and a test pins that.
//!
//! Nothing this module decides is a static certificate. The only verdict it
//! can reach is [`Verdict::Observed`]; every refusal is explicit and no arm
//! promotes an observation to a guarantee about the library, the filesystem or
//! any other execution.

use core::fmt;

/// First header byte of a ChunkIO data file (`CIO_FILE_ID_00`).
pub const FILE_ID_0: u8 = 0xc1;
/// Second header byte of a ChunkIO data file (`CIO_FILE_ID_01`).
pub const FILE_ID_1: u8 = 0x00;
/// `CIO_FILE_HEADER_MIN`: the shortest image that carries a complete header.
pub const HEADER_MIN: usize = 24;
/// `CIO_FILE_CONTENT_OFFSET`: where the checksummed region starts.
pub const CONTENT_OFFSET: usize = 22;
/// `CIO_FILE_CONTENT_LENGTH_OFFSET`: big-endian u32 content length.
pub const CONTENT_LENGTH_OFFSET: usize = 10;
/// Offset of the stored big-endian crc32 of the content region.
pub const CHECKSUM_OFFSET: usize = 2;

/// `CIO_OK`.
pub const CIO_OK: i64 = 0;
/// `CIO_ERROR`.
pub const CIO_ERROR: i64 = -1;
/// `CIO_CORRUPTED`.
pub const CIO_CORRUPTED: i64 = -3;
/// `CIO_ERR_BAD_CHECKSUM`.
pub const ERR_BAD_CHECKSUM: i64 = -10;
/// `CIO_ERR_BAD_LAYOUT`.
pub const ERR_BAD_LAYOUT: i64 = -11;
/// `CIO_ERR_BAD_FILE_SIZE`: reported for both truncation classes.
pub const ERR_BAD_FILE_SIZE: i64 = -13;
/// No fine-grained error is set on the chunk.
pub const ERR_NONE: i64 = 0;

/// The class a recorded image belongs to, derived from its bytes alone.
///
/// The order of the arms is the order the pinned format check decides them, so
/// an image that fails several rules is classified by the first rule that
/// rejects it — exactly as the observed library does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContentClass {
    /// Header, declared lengths and stored checksum all agree with the bytes.
    Clean,
    /// The image is shorter than a complete header.
    TruncatedHeader,
    /// The two identification bytes are not `0xc1 0x00`.
    BadMagic,
    /// Header plus metadata plus declared content exceeds the image.
    TruncatedContent,
    /// The stored crc32 does not equal the crc32 of the content region.
    BadChecksum,
}

impl ContentClass {
    /// Whether this class is a corruption at all.
    pub const fn is_corrupt(self) -> bool {
        !matches!(self, ContentClass::Clean)
    }

    /// The `cio_error` code the pinned format check sets for this class.
    ///
    /// `Clean` sets nothing, so it maps to [`ERR_NONE`]; that is the same code
    /// an injected operation failure leaves behind, which is precisely why the
    /// code alone can never distinguish the two axes.
    pub const fn error_code(self) -> i64 {
        match self {
            ContentClass::Clean => ERR_NONE,
            ContentClass::TruncatedHeader | ContentClass::TruncatedContent => ERR_BAD_FILE_SIZE,
            ContentClass::BadMagic => ERR_BAD_LAYOUT,
            ContentClass::BadChecksum => ERR_BAD_CHECKSUM,
        }
    }

    /// The declared spelling used in a record.
    pub const fn label(self) -> &'static str {
        match self {
            ContentClass::Clean => "clean",
            ContentClass::TruncatedHeader => "truncated_header",
            ContentClass::BadMagic => "bad_magic",
            ContentClass::TruncatedContent => "truncated_content",
            ContentClass::BadChecksum => "bad_checksum",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        [
            ContentClass::Clean,
            ContentClass::TruncatedHeader,
            ContentClass::BadMagic,
            ContentClass::TruncatedContent,
            ContentClass::BadChecksum,
        ]
        .into_iter()
        .find(|class| class.label() == text)
    }
}

/// The operation the harness suppressed, if any.
///
/// A suppressed call returns a fixed error without reaching the kernel. It is
/// therefore a statement about the wrapper and about what the library does
/// next — never an observed operating-system post-state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Injection {
    /// Every selected call was forwarded.
    None,
    /// `mmap` returned `MAP_FAILED` with the recorded errno.
    Mmap,
    /// `close` returned `-1` with the recorded errno; the descriptor stays open.
    Close,
}

impl Injection {
    /// The declared spelling used in a record.
    pub const fn label(self) -> &'static str {
        match self {
            Injection::None => "none",
            Injection::Mmap => "mmap",
            Injection::Close => "close",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        [Injection::None, Injection::Mmap, Injection::Close]
            .into_iter()
            .find(|injection| injection.label() == text)
    }
}

/// Allocation generations, split by who released them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllocationLedger {
    /// Generations the observer saw acquired.
    pub acquired: u64,
    /// Generations the observed library released.
    pub library_released: u64,
    /// Generations the harness released. Always zero here: the harness never
    /// frees a library allocation. The field exists so the separation is
    /// recorded rather than assumed.
    pub harness_released: u64,
    /// Generations still live when the process finished.
    pub outstanding: u64,
}

/// One endpoint kind's epochs, split by who released them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndpointLedger {
    /// Epochs the observer saw acquired.
    pub epochs: u64,
    /// Epochs released by a forwarded call the library made.
    pub library_released: u64,
    /// Epochs the harness reclaimed after the parent object was released.
    pub harness_released: u64,
    /// Whether an epoch was still live after the parent release.
    pub residual_after_parents: bool,
}

/// The outcome the constructor reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstructorOutcome {
    /// Whether a chunk was returned.
    pub chunk: bool,
    /// The `int *err` out-parameter of `cio_chunk_open`.
    pub open_error: i64,
    /// The fine-grained `cio_error` code observed on the chunk.
    pub chunk_error: i64,
}

/// One recorded fixture execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseRecord {
    /// The case identifier.
    pub case: u64,
    /// The fixture profile name.
    pub profile: String,
    /// The class the driver says it seeded.
    pub declared_class: ContentClass,
    /// The image as it was before the observed execution.
    pub image_before: Vec<u8>,
    /// The image as it was after the observed execution.
    pub image_after: Vec<u8>,
    /// The suppressed operation, if any.
    pub injection: Injection,
    /// The errno the suppressed call returned; zero when nothing was suppressed.
    pub injected_errno: i64,
    /// How many calls the suppression consumed.
    pub suppressed_calls: u64,
    /// What the constructor reported.
    pub outcome: ConstructorOutcome,
    /// Allocation accounting.
    pub allocations: AllocationLedger,
    /// Descriptor endpoint accounting.
    pub descriptors: EndpointLedger,
    /// Mapping endpoint accounting.
    pub mappings: EndpointLedger,
    /// Whether the fixture kept its device/inode identity.
    pub inode_preserved: bool,
    /// Whether the fixture kept its size.
    pub size_preserved: bool,
    /// Descriptor epochs the harness reclaimed at the end of the process.
    pub reclaimed_descriptors: u64,
    /// Mapping epochs the harness reclaimed at the end of the process.
    pub reclaimed_mappings: u64,
}

/// What a validated case establishes, and only that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseVerdict {
    /// The case identifier.
    pub case: u64,
    /// The class independently derived from the recorded bytes.
    pub derived_class: ContentClass,
    /// The suppressed operation, carried separately from the class.
    pub injection: Injection,
    /// Whether the recorded image changed across the execution.
    pub content_changed: bool,
    /// Endpoint epochs still live after the parent release.
    pub residual_endpoints: u64,
    /// Endpoint epochs the harness reclaimed, never counted as library work.
    pub harness_reclaimed: u64,
    /// Always [`Verdict::Observed`].
    pub verdict: Verdict,
}

/// The only verdict this relation can reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// A recorded execution, under the declared environment, and nothing more.
    Observed,
}

/// Why a record is refused. Every arm is fail-closed: no partial verdict is
/// published beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegrityRefusal {
    /// The record's declared class is not the class its bytes have.
    DeclaredClassIsNotInTheBytes {
        /// What the record claimed.
        declared: ContentClass,
        /// What the bytes say.
        derived: ContentClass,
    },
    /// The image after the execution has a different class from the image
    /// before it, so the recorded content is not the content that was read.
    ContentClassChanged {
        /// Class of the image before.
        before: ContentClass,
        /// Class of the image after.
        after: ContentClass,
    },
    /// The recorded constructor outcome is not the outcome the declared class
    /// and injection determine.
    OutcomeDisagreesWithClassification {
        /// What the relation expects.
        expected: ConstructorOutcome,
        /// What the record carries.
        observed: ConstructorOutcome,
    },
    /// An injection is claimed with no suppressed call, or suppressed calls are
    /// recorded with no injection.
    InjectionIsNotAccountedFor {
        /// The claimed injection.
        injection: Injection,
        /// The recorded suppression count.
        suppressed: u64,
    },
    /// A suppressed operation was recorded with an errno of zero, or a
    /// forwarded profile carries a nonzero injected errno.
    InjectedErrnoDisagrees {
        /// The claimed injection.
        injection: Injection,
        /// The recorded errno.
        errno: i64,
    },
    /// Acquired generations do not equal released plus outstanding.
    AllocationsDoNotBalance {
        /// Generations acquired.
        acquired: u64,
        /// Generations accounted for.
        accounted: u64,
    },
    /// A generation was still live when the process finished.
    OutstandingAllocations {
        /// How many.
        outstanding: u64,
    },
    /// The harness was credited with releasing a library allocation.
    HarnessClaimedAnAllocationRelease {
        /// How many.
        harness_released: u64,
    },
    /// An endpoint ledger does not balance.
    EndpointsDoNotBalance {
        /// Which endpoint kind.
        endpoint: &'static str,
        /// Epochs acquired.
        epochs: u64,
        /// Epochs accounted for.
        accounted: u64,
    },
    /// A residual is claimed without a matching harness reclamation, or a
    /// reclamation is claimed with no residual.
    ResidualAndReclamationDisagree {
        /// Which endpoint kind.
        endpoint: &'static str,
        /// Whether the record claims a residual.
        residual: bool,
        /// How many epochs the harness reclaimed.
        reclaimed: u64,
    },
    /// A descriptor residual is recorded although nothing was suppressed, or a
    /// suppressed close left no residual.
    ResidualIsNotExplainedBySuppression {
        /// The claimed injection.
        injection: Injection,
        /// Whether a descriptor residual is claimed.
        residual: bool,
    },
    /// The harness release count for an endpoint was folded into the library
    /// count, so the two are no longer separated.
    LibraryCreditedWithASuppressedCall {
        /// Which endpoint kind.
        endpoint: &'static str,
    },
    /// The fixture lost its identity or its size.
    FixtureIdentityChanged {
        /// Whether the device/inode identity survived.
        inode_preserved: bool,
        /// Whether the size survived.
        size_preserved: bool,
    },
    /// A record could not be parsed.
    Malformed {
        /// One-based line number in the record text.
        line: usize,
        /// What was wrong.
        reason: &'static str,
    },
}

impl fmt::Display for IntegrityRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntegrityRefusal::DeclaredClassIsNotInTheBytes { declared, derived } => write!(
                f,
                "declared class {} is not in the bytes, which are {}",
                declared.label(),
                derived.label()
            ),
            IntegrityRefusal::ContentClassChanged { before, after } => write!(
                f,
                "content class changed from {} to {}",
                before.label(),
                after.label()
            ),
            IntegrityRefusal::OutcomeDisagreesWithClassification { expected, observed } => write!(
                f,
                "outcome {observed:?} disagrees with the classification, which requires {expected:?}"
            ),
            IntegrityRefusal::InjectionIsNotAccountedFor {
                injection,
                suppressed,
            } => write!(
                f,
                "injection {} is not accounted for by {suppressed} suppressed calls",
                injection.label()
            ),
            IntegrityRefusal::InjectedErrnoDisagrees { injection, errno } => write!(
                f,
                "injection {} disagrees with errno {errno}",
                injection.label()
            ),
            IntegrityRefusal::AllocationsDoNotBalance {
                acquired,
                accounted,
            } => write!(f, "{acquired} generations acquired, {accounted} accounted"),
            IntegrityRefusal::OutstandingAllocations { outstanding } => {
                write!(f, "{outstanding} generations still live")
            }
            IntegrityRefusal::HarnessClaimedAnAllocationRelease { harness_released } => write!(
                f,
                "the harness claimed {harness_released} allocation releases"
            ),
            IntegrityRefusal::EndpointsDoNotBalance {
                endpoint,
                epochs,
                accounted,
            } => write!(f, "{endpoint}: {epochs} epochs, {accounted} accounted"),
            IntegrityRefusal::ResidualAndReclamationDisagree {
                endpoint,
                residual,
                reclaimed,
            } => write!(
                f,
                "{endpoint}: residual {residual} against {reclaimed} reclamations"
            ),
            IntegrityRefusal::ResidualIsNotExplainedBySuppression {
                injection,
                residual,
            } => write!(
                f,
                "descriptor residual {residual} is not explained by injection {}",
                injection.label()
            ),
            IntegrityRefusal::LibraryCreditedWithASuppressedCall { endpoint } => {
                write!(
                    f,
                    "{endpoint}: a suppressed call was credited to the library"
                )
            }
            IntegrityRefusal::FixtureIdentityChanged {
                inode_preserved,
                size_preserved,
            } => write!(
                f,
                "fixture identity changed: inode {inode_preserved}, size {size_preserved}"
            ),
            IntegrityRefusal::Malformed { line, reason } => write!(f, "line {line}: {reason}"),
        }
    }
}

/// crc32 as ChunkIO's `deps/crc32` computes it: reflected, polynomial
/// `0xedb88320`, initial value `0xffffffff`, final xor `0xffffffff`.
///
/// This is an independent statement of the algorithm, not a call into the
/// observed code, so an image the observer accepted can still be refused here.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    crc ^ 0xffff_ffff
}

/// Derive the class of an image from its bytes alone.
///
/// The decision order repeats the pinned format check: a header shorter than
/// the minimum is a size failure before anything can be read from it, the
/// identification bytes come next, then the declared lengths, then the stored
/// checksum.
pub fn classify_content(image: &[u8]) -> ContentClass {
    if image.len() < HEADER_MIN {
        return ContentClass::TruncatedHeader;
    }
    if image[0] != FILE_ID_0 || image[1] != FILE_ID_1 {
        return ContentClass::BadMagic;
    }
    let metadata = usize::from(u16::from_be_bytes([
        image[CONTENT_OFFSET],
        image[CONTENT_OFFSET + 1],
    ]));
    let content = u32::from_be_bytes([
        image[CONTENT_LENGTH_OFFSET],
        image[CONTENT_LENGTH_OFFSET + 1],
        image[CONTENT_LENGTH_OFFSET + 2],
        image[CONTENT_LENGTH_OFFSET + 3],
    ]) as usize;
    let logical = HEADER_MIN + metadata + content;
    if logical > image.len() {
        return ContentClass::TruncatedContent;
    }
    let region = &image[CONTENT_OFFSET..CONTENT_OFFSET + 2 + metadata + content];
    let stored = u32::from_be_bytes([
        image[CHECKSUM_OFFSET],
        image[CHECKSUM_OFFSET + 1],
        image[CHECKSUM_OFFSET + 2],
        image[CHECKSUM_OFFSET + 3],
    ]);
    if crc32(region) != stored {
        return ContentClass::BadChecksum;
    }
    ContentClass::Clean
}

/// The constructor outcome the declared model determines for one class and one
/// injection point.
///
/// The two axes contribute independently: the class decides whether the open is
/// refused as corrupted and which fine-grained code is left behind, and the
/// injection decides whether a clean open is refused as an ordinary error. A
/// suppressed `close` is not reached until rollback or teardown, so it does not
/// change what the constructor reports.
pub fn expected_outcome(class: ContentClass, injection: Injection) -> ConstructorOutcome {
    if class.is_corrupt() {
        return ConstructorOutcome {
            chunk: false,
            open_error: CIO_CORRUPTED,
            chunk_error: class.error_code(),
        };
    }
    match injection {
        Injection::Mmap => ConstructorOutcome {
            chunk: false,
            open_error: CIO_ERROR,
            chunk_error: ERR_NONE,
        },
        Injection::None | Injection::Close => ConstructorOutcome {
            chunk: true,
            open_error: CIO_OK,
            chunk_error: ERR_NONE,
        },
    }
}

fn check_endpoint(
    endpoint: &'static str,
    ledger: &EndpointLedger,
    reclaimed: u64,
    suppressed_here: bool,
) -> Result<(), IntegrityRefusal> {
    let accounted = ledger.library_released + ledger.harness_released;
    if accounted != ledger.epochs {
        return Err(IntegrityRefusal::EndpointsDoNotBalance {
            endpoint,
            epochs: ledger.epochs,
            accounted,
        });
    }
    if ledger.residual_after_parents != (ledger.harness_released > 0)
        || ledger.harness_released != reclaimed
    {
        return Err(IntegrityRefusal::ResidualAndReclamationDisagree {
            endpoint,
            residual: ledger.residual_after_parents,
            reclaimed,
        });
    }
    if suppressed_here && ledger.harness_released == 0 {
        return Err(IntegrityRefusal::LibraryCreditedWithASuppressedCall { endpoint });
    }
    Ok(())
}

/// Validate one recorded case.
///
/// Every check is a refusal, never a downgrade: there is no partially accepted
/// record and no arm that returns a verdict stronger than [`Verdict::Observed`].
pub fn validate_case(record: &CaseRecord) -> Result<CaseVerdict, IntegrityRefusal> {
    let derived = classify_content(&record.image_before);
    if derived != record.declared_class {
        return Err(IntegrityRefusal::DeclaredClassIsNotInTheBytes {
            declared: record.declared_class,
            derived,
        });
    }
    let after = classify_content(&record.image_after);
    if after != derived {
        return Err(IntegrityRefusal::ContentClassChanged {
            before: derived,
            after,
        });
    }
    if !record.inode_preserved || !record.size_preserved {
        return Err(IntegrityRefusal::FixtureIdentityChanged {
            inode_preserved: record.inode_preserved,
            size_preserved: record.size_preserved,
        });
    }

    let suppressed_expected = record.injection != Injection::None;
    if suppressed_expected != (record.suppressed_calls > 0) {
        return Err(IntegrityRefusal::InjectionIsNotAccountedFor {
            injection: record.injection,
            suppressed: record.suppressed_calls,
        });
    }
    if suppressed_expected != (record.injected_errno != 0) {
        return Err(IntegrityRefusal::InjectedErrnoDisagrees {
            injection: record.injection,
            errno: record.injected_errno,
        });
    }

    let expected = expected_outcome(derived, record.injection);
    if expected != record.outcome {
        return Err(IntegrityRefusal::OutcomeDisagreesWithClassification {
            expected,
            observed: record.outcome,
        });
    }

    let accounted = record.allocations.library_released
        + record.allocations.harness_released
        + record.allocations.outstanding;
    if accounted != record.allocations.acquired {
        return Err(IntegrityRefusal::AllocationsDoNotBalance {
            acquired: record.allocations.acquired,
            accounted,
        });
    }
    if record.allocations.outstanding != 0 {
        return Err(IntegrityRefusal::OutstandingAllocations {
            outstanding: record.allocations.outstanding,
        });
    }
    if record.allocations.harness_released != 0 {
        return Err(IntegrityRefusal::HarnessClaimedAnAllocationRelease {
            harness_released: record.allocations.harness_released,
        });
    }

    let closed = record.injection == Injection::Close;
    check_endpoint(
        "descriptor",
        &record.descriptors,
        record.reclaimed_descriptors,
        closed,
    )?;
    check_endpoint(
        "mapping",
        &record.mappings,
        record.reclaimed_mappings,
        false,
    )?;
    if record.descriptors.residual_after_parents != closed {
        return Err(IntegrityRefusal::ResidualIsNotExplainedBySuppression {
            injection: record.injection,
            residual: record.descriptors.residual_after_parents,
        });
    }

    Ok(CaseVerdict {
        case: record.case,
        derived_class: derived,
        injection: record.injection,
        content_changed: record.image_before != record.image_after,
        residual_endpoints: u64::from(record.descriptors.residual_after_parents)
            + u64::from(record.mappings.residual_after_parents),
        harness_reclaimed: record.reclaimed_descriptors + record.reclaimed_mappings,
        verdict: Verdict::Observed,
    })
}

/// Validate a whole recorded family, in the order it was recorded.
pub fn validate_family(records: &[CaseRecord]) -> Result<Vec<CaseVerdict>, IntegrityRefusal> {
    records.iter().map(validate_case).collect()
}

// --------------------------------------------------------------------------
// Record parsing
// --------------------------------------------------------------------------

struct Fields<'a> {
    line: usize,
    parts: Vec<(&'a str, &'a str)>,
}

impl<'a> Fields<'a> {
    fn get(&self, key: &str) -> Result<&'a str, IntegrityRefusal> {
        self.parts
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| *value)
            .ok_or(IntegrityRefusal::Malformed {
                line: self.line,
                reason: "missing field",
            })
    }

    fn number(&self, key: &str) -> Result<u64, IntegrityRefusal> {
        self.get(key)?
            .parse()
            .map_err(|_| IntegrityRefusal::Malformed {
                line: self.line,
                reason: "field is not a natural number",
            })
    }

    fn signed(&self, key: &str) -> Result<i64, IntegrityRefusal> {
        self.get(key)?
            .parse()
            .map_err(|_| IntegrityRefusal::Malformed {
                line: self.line,
                reason: "field is not an integer",
            })
    }

    fn flag(&self, key: &str) -> Result<bool, IntegrityRefusal> {
        match self.get(key)? {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(IntegrityRefusal::Malformed {
                line: self.line,
                reason: "field is not a flag",
            }),
        }
    }

    fn hex(&self, key: &str) -> Result<Vec<u8>, IntegrityRefusal> {
        let text = self.get(key)?;
        if text.is_empty() || text.len() % 2 != 0 {
            return Err(IntegrityRefusal::Malformed {
                line: self.line,
                reason: "field is not a byte image",
            });
        }
        let mut bytes = Vec::with_capacity(text.len() / 2);
        let raw = text.as_bytes();
        for pair in raw.chunks(2) {
            let text = core::str::from_utf8(pair).map_err(|_| IntegrityRefusal::Malformed {
                line: self.line,
                reason: "field is not a byte image",
            })?;
            bytes.push(
                u8::from_str_radix(text, 16).map_err(|_| IntegrityRefusal::Malformed {
                    line: self.line,
                    reason: "field is not a byte image",
                })?,
            );
        }
        Ok(bytes)
    }
}

fn split<'a>(line: usize, text: &'a str, keyword: &str) -> Result<Fields<'a>, IntegrityRefusal> {
    split_positional(line, text, keyword, 0)
}

/// Split a record line whose keyword is followed by `positional` bare words
/// before its `key=value` fields.
fn split_positional<'a>(
    line: usize,
    text: &'a str,
    keyword: &str,
    positional: usize,
) -> Result<Fields<'a>, IntegrityRefusal> {
    let mut words = text.split_whitespace();
    if words.next() != Some(keyword) {
        return Err(IntegrityRefusal::Malformed {
            line,
            reason: "unexpected record keyword",
        });
    }
    for _ in 0..positional {
        if words.next().is_none() {
            return Err(IntegrityRefusal::Malformed {
                line,
                reason: "record line is missing a positional word",
            });
        }
    }
    let mut parts = Vec::new();
    for word in words {
        let (name, value) = word.split_once('=').ok_or(IntegrityRefusal::Malformed {
            line,
            reason: "record field is not key=value",
        })?;
        parts.push((name, value));
    }
    Ok(Fields { line, parts })
}

/// Parse the canonical record text the native observer writes.
///
/// The grammar is fixed and total-or-reject: a missing line, an unexpected
/// keyword, an unknown class or injection name, or a malformed field refuses
/// the whole text.
pub fn parse_records(text: &str) -> Result<Vec<CaseRecord>, IntegrityRefusal> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let mut index = 0usize;
    let mut records = Vec::new();
    if lines.first().copied() != Some("schema chunk_integrity_observation_v1") {
        return Err(IntegrityRefusal::Malformed {
            line: 1,
            reason: "unknown record schema",
        });
    }
    index += 1;
    split(
        index + 1,
        lines.get(index).copied().unwrap_or(""),
        "identity",
    )?;
    index += 1;

    while index < lines.len() {
        let start = index;
        let head = split_positional(start + 1, lines[start], "case", 1)?;
        let case = lines[start]
            .split_whitespace()
            .nth(1)
            .and_then(|word| word.parse::<u64>().ok())
            .ok_or(IntegrityRefusal::Malformed {
                line: start + 1,
                reason: "case identifier is missing",
            })?;
        let profile = head.get("profile")?.to_string();
        let need = |offset: usize, keyword: &str| -> Result<Fields<'_>, IntegrityRefusal> {
            let line = lines
                .get(start + offset)
                .copied()
                .ok_or(IntegrityRefusal::Malformed {
                    line: start + offset + 1,
                    reason: "record block is truncated",
                })?;
            split(start + offset + 1, line, keyword)
        };
        let input = need(1, "input")?;
        let injection = need(2, "injection")?;
        let outcome = need(3, "outcome")?;
        let allocations = need(4, "allocations")?;
        let descriptors = need(5, "descriptor")?;
        let mappings = need(6, "mapping")?;
        let fixture = need(7, "fixture")?;
        let reclaimed = need(8, "reclaimed")?;
        let end = lines
            .get(start + 9)
            .copied()
            .ok_or(IntegrityRefusal::Malformed {
                line: start + 10,
                reason: "record block is truncated",
            })?;
        if end != format!("end case {case}") {
            return Err(IntegrityRefusal::Malformed {
                line: start + 10,
                reason: "record block does not close on its own case",
            });
        }
        records.push(CaseRecord {
            case,
            profile,
            declared_class: ContentClass::parse(input.get("kind")?).ok_or(
                IntegrityRefusal::Malformed {
                    line: start + 2,
                    reason: "unknown content class",
                },
            )?,
            image_before: input.hex("pre")?,
            image_after: fixture.hex("post")?,
            injection: Injection::parse(injection.get("op")?).ok_or(
                IntegrityRefusal::Malformed {
                    line: start + 3,
                    reason: "unknown injected operation",
                },
            )?,
            injected_errno: injection.signed("errno")?,
            suppressed_calls: injection.number("suppressed")?,
            outcome: ConstructorOutcome {
                chunk: outcome.flag("chunk")?,
                open_error: outcome.signed("open_error")?,
                chunk_error: outcome.signed("chunk_error")?,
            },
            allocations: AllocationLedger {
                acquired: allocations.number("acquired")?,
                library_released: allocations.number("library_released")?,
                harness_released: allocations.number("harness_released")?,
                outstanding: allocations.number("outstanding")?,
            },
            descriptors: EndpointLedger {
                epochs: descriptors.number("epochs")?,
                library_released: descriptors.number("library_closed")?,
                harness_released: descriptors.number("harness_closed")?,
                residual_after_parents: descriptors.flag("residual_after_parents")?,
            },
            mappings: EndpointLedger {
                epochs: mappings.number("epochs")?,
                library_released: mappings.number("library_unmapped")?,
                harness_released: mappings.number("harness_unmapped")?,
                residual_after_parents: mappings.flag("residual_after_parents")?,
            },
            inode_preserved: fixture.flag("inode_preserved")?,
            size_preserved: fixture.flag("size_preserved")?,
            reclaimed_descriptors: reclaimed.number("descriptors")?,
            reclaimed_mappings: reclaimed.number("mappings")?,
        });
        index = start + 10;
    }
    if records.is_empty() {
        return Err(IntegrityRefusal::Malformed {
            line: lines.len(),
            reason: "no recorded case",
        });
    }
    Ok(records)
}

/// The values a validated case contributes to the declared observation policy.
///
/// Only integers cross this boundary, and `static_certificate` and
/// `durability_proved` are constant zero: no arm of this module can set them.
pub fn policy_values(record: &CaseRecord, verdict: &CaseVerdict) -> Vec<(&'static str, i128)> {
    vec![
        ("exclusive_instrumented_fixture", 1),
        ("compiled_source_and_build_recorded", 1),
        (
            "corrupted_input",
            i128::from(verdict.derived_class.is_corrupt()),
        ),
        (
            "bad_checksum",
            i128::from(verdict.derived_class == ContentClass::BadChecksum),
        ),
        (
            "bad_magic",
            i128::from(verdict.derived_class == ContentClass::BadMagic),
        ),
        (
            "truncated",
            i128::from(
                verdict.derived_class == ContentClass::TruncatedHeader
                    || verdict.derived_class == ContentClass::TruncatedContent,
            ),
        ),
        (
            "injected_operation_failure",
            i128::from(verdict.injection != Injection::None),
        ),
        (
            "injected_map_failure",
            i128::from(verdict.injection == Injection::Mmap),
        ),
        (
            "injected_close_failure",
            i128::from(verdict.injection == Injection::Close),
        ),
        ("suppressed_calls", i128::from(record.suppressed_calls)),
        ("chunk_returned", i128::from(record.outcome.chunk)),
        ("allocations", i128::from(record.allocations.acquired)),
        (
            "library_allocation_releases",
            i128::from(record.allocations.library_released),
        ),
        (
            "harness_allocation_releases",
            i128::from(record.allocations.harness_released),
        ),
        ("descriptor_epochs", i128::from(record.descriptors.epochs)),
        (
            "library_descriptor_releases",
            i128::from(record.descriptors.library_released),
        ),
        (
            "harness_descriptor_releases",
            i128::from(record.descriptors.harness_released),
        ),
        ("mapping_epochs", i128::from(record.mappings.epochs)),
        (
            "library_mapping_releases",
            i128::from(record.mappings.library_released),
        ),
        (
            "harness_mapping_releases",
            i128::from(record.mappings.harness_released),
        ),
        (
            "parent_descriptor_residuals",
            i128::from(record.descriptors.residual_after_parents),
        ),
        ("content_changed", i128::from(verdict.content_changed)),
        (
            "original_inode_retained",
            i128::from(record.inode_preserved),
        ),
        ("original_size_retained", i128::from(record.size_preserved)),
        ("static_certificate", 0),
        ("durability_proved", 0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The recorded family, as it was archived under
    /// `docs/reports/remaining-o01/logs/records.txt`.
    const RECORDED: &str = include_str!("../tests/data/o01_records.txt");

    fn recorded() -> Vec<CaseRecord> {
        parse_records(RECORDED).expect("the archived family parses")
    }

    fn clean_image() -> Vec<u8> {
        let mut image = vec![0u8; 4096];
        image[0] = FILE_ID_0;
        image[1] = FILE_ID_1;
        image[CONTENT_LENGTH_OFFSET..CONTENT_LENGTH_OFFSET + 4]
            .copy_from_slice(&4u32.to_be_bytes());
        image[24..28].copy_from_slice(b"seed");
        let checksum = crc32(&image[CONTENT_OFFSET..CONTENT_OFFSET + 6]);
        image[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4].copy_from_slice(&checksum.to_be_bytes());
        image
    }

    // ---- the checksum itself -------------------------------------------

    #[test]
    fn the_crc32_agrees_with_its_published_check_values() {
        // The standard CRC-32 check value: "123456789" hashes to 0xcbf43926.
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(crc32(b""), 0);
    }

    // ---- classification from bytes alone --------------------------------

    #[test]
    fn every_content_class_is_reachable_from_an_image() {
        let mut images = Vec::new();
        images.push((ContentClass::Clean, clean_image()));
        let mut magic = clean_image();
        magic[0] = 0;
        images.push((ContentClass::BadMagic, magic));
        images.push((ContentClass::TruncatedHeader, clean_image()[..8].to_vec()));
        let mut declared = clean_image();
        declared[CONTENT_LENGTH_OFFSET..CONTENT_LENGTH_OFFSET + 4]
            .copy_from_slice(&4096u32.to_be_bytes());
        images.push((ContentClass::TruncatedContent, declared));
        let mut payload = clean_image();
        payload[24] = b'S';
        images.push((ContentClass::BadChecksum, payload));
        for (class, image) in images {
            assert_eq!(classify_content(&image), class, "{class:?}");
        }
    }

    #[test]
    fn a_flipped_payload_byte_is_a_checksum_failure_not_a_layout_failure() {
        let mut image = clean_image();
        image[25] = b'E';
        assert_eq!(classify_content(&image), ContentClass::BadChecksum);
        assert_eq!(ContentClass::BadChecksum.error_code(), ERR_BAD_CHECKSUM);
        assert_ne!(
            ContentClass::BadChecksum.error_code(),
            ContentClass::BadMagic.error_code()
        );
    }

    #[test]
    fn a_stale_stored_checksum_is_detected_even_when_the_payload_is_valid() {
        let mut image = clean_image();
        image[CHECKSUM_OFFSET] ^= 0xff;
        assert_eq!(classify_content(&image), ContentClass::BadChecksum);
    }

    #[test]
    fn the_decision_order_matches_the_observed_format_check() {
        // An image that is both magic-broken and checksum-broken is classified
        // by the earlier rule, as the library decides it.
        let mut image = clean_image();
        image[0] = 0;
        image[24] = b'S';
        assert_eq!(classify_content(&image), ContentClass::BadMagic);
        // A short image is a size failure before its magic can be read.
        let mut short = clean_image()[..8].to_vec();
        short[0] = 0;
        assert_eq!(classify_content(&short), ContentClass::TruncatedHeader);
    }

    // ---- the two axes stay separate --------------------------------------

    #[test]
    fn a_corrupted_input_and_an_injected_failure_are_distinct_classes() {
        let corrupt = expected_outcome(ContentClass::BadChecksum, Injection::None);
        let injected = expected_outcome(ContentClass::Clean, Injection::Mmap);
        assert_ne!(corrupt, injected);
        assert_eq!(corrupt.open_error, CIO_CORRUPTED);
        assert_eq!(injected.open_error, CIO_ERROR);
        // The fine-grained code cannot tell them apart on its own, which is
        // exactly why the class is derived from the bytes.
        assert_eq!(injected.chunk_error, ERR_NONE);
        assert_eq!(
            expected_outcome(ContentClass::Clean, Injection::None).chunk_error,
            injected.chunk_error
        );
    }

    #[test]
    fn an_injection_never_changes_the_derived_content_class() {
        for injection in [Injection::None, Injection::Mmap, Injection::Close] {
            let record = recorded()
                .into_iter()
                .find(|record| record.injection == injection)
                .expect("the family records every injection");
            assert_eq!(
                classify_content(&record.image_before),
                record.declared_class,
                "{injection:?}"
            );
        }
    }

    #[test]
    fn a_suppressed_call_yields_no_operating_system_post_state() {
        // The verdict names the injection and the residual; it carries no
        // `os_endpoint` post-state, and a suppressed close never reached the
        // kernel, so none may be derived from it.
        let record = recorded()
            .into_iter()
            .find(|record| record.injection == Injection::Close)
            .expect("the family records a suppressed close");
        let verdict = validate_case(&record).expect("the recorded case validates");
        assert_eq!(verdict.verdict, Verdict::Observed);
        assert_eq!(verdict.residual_endpoints, 1);
        assert_eq!(verdict.harness_reclaimed, 1);
    }

    // ---- the archived family ---------------------------------------------

    #[test]
    fn the_archived_family_validates_and_covers_both_axes() {
        let records = recorded();
        assert_eq!(records.len(), 8);
        let verdicts = validate_family(&records).expect("the archived family validates");
        assert!(verdicts.iter().all(|v| v.verdict == Verdict::Observed));
        let corrupted = verdicts
            .iter()
            .filter(|v| v.derived_class.is_corrupt())
            .count();
        let injected = verdicts
            .iter()
            .filter(|v| v.injection != Injection::None)
            .count();
        let both = verdicts
            .iter()
            .filter(|v| v.derived_class.is_corrupt() && v.injection != Injection::None)
            .count();
        // Five corrupted images, three suppressed operations, and exactly one
        // case that carries both axes at once without merging them.
        assert_eq!((corrupted, injected, both), (5, 3, 1));
        for class in [
            ContentClass::Clean,
            ContentClass::BadChecksum,
            ContentClass::BadMagic,
            ContentClass::TruncatedHeader,
            ContentClass::TruncatedContent,
        ] {
            assert!(
                verdicts.iter().any(|v| v.derived_class == class),
                "{class:?} is unrepresented"
            );
        }
    }

    #[test]
    fn no_recorded_execution_changed_its_fixture_content() {
        for verdict in validate_family(&recorded()).expect("the archived family validates") {
            assert!(!verdict.content_changed, "case {}", verdict.case);
        }
    }

    #[test]
    fn every_recorded_allocation_generation_is_released_by_the_library() {
        for record in recorded() {
            assert_eq!(record.allocations.acquired, 10, "case {}", record.case);
            assert_eq!(
                record.allocations.library_released, record.allocations.acquired,
                "case {}",
                record.case
            );
            assert_eq!(record.allocations.harness_released, 0);
            assert_eq!(record.allocations.outstanding, 0);
        }
    }

    #[test]
    fn only_a_suppressed_close_leaves_a_descriptor_for_the_harness() {
        for record in recorded() {
            let suppressed = record.injection == Injection::Close;
            assert_eq!(
                record.descriptors.residual_after_parents, suppressed,
                "case {}",
                record.case
            );
            assert_eq!(
                record.reclaimed_descriptors,
                u64::from(suppressed),
                "case {}",
                record.case
            );
            assert_eq!(record.reclaimed_mappings, 0, "case {}", record.case);
        }
    }

    #[test]
    fn a_suppressed_map_acquires_no_mapping_epoch() {
        let record = recorded()
            .into_iter()
            .find(|record| record.injection == Injection::Mmap)
            .expect("the family records a suppressed map");
        assert_eq!(record.mappings.epochs, 0);
        assert_eq!(record.descriptors.epochs, 1);
        assert_eq!(record.descriptors.library_released, 1);
    }

    #[test]
    fn the_policy_values_never_publish_a_certificate() {
        for record in recorded() {
            let verdict = validate_case(&record).expect("validates");
            let values = policy_values(&record, &verdict);
            assert_eq!(
                values
                    .iter()
                    .find(|(key, _)| *key == "static_certificate")
                    .map(|(_, value)| *value),
                Some(0)
            );
            assert_eq!(
                values
                    .iter()
                    .find(|(key, _)| *key == "durability_proved")
                    .map(|(_, value)| *value),
                Some(0)
            );
        }
    }

    // ---- the mutation matrix ---------------------------------------------

    fn first() -> CaseRecord {
        recorded().remove(0)
    }

    fn corrupted() -> CaseRecord {
        recorded()
            .into_iter()
            .find(|record| record.declared_class == ContentClass::BadChecksum)
            .expect("the family records a checksum failure")
    }

    #[test]
    fn m01_a_claimed_corruption_that_is_not_in_the_bytes_is_refused() {
        let mut record = first();
        record.declared_class = ContentClass::BadChecksum;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::DeclaredClassIsNotInTheBytes { .. })
        ));
    }

    #[test]
    fn m02_a_repaired_stored_checksum_no_longer_matches_the_claim() {
        let mut record = corrupted();
        let metadata = 0usize;
        let content = 4usize;
        let checksum =
            crc32(&record.image_before[CONTENT_OFFSET..CONTENT_OFFSET + 2 + metadata + content]);
        record.image_before[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4]
            .copy_from_slice(&checksum.to_be_bytes());
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::DeclaredClassIsNotInTheBytes {
                derived: ContentClass::Clean,
                ..
            })
        ));
    }

    #[test]
    fn m03_an_image_that_changed_class_across_the_execution_is_refused() {
        let mut record = first();
        record.image_after[0] = 0;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::ContentClassChanged { .. })
        ));
    }

    #[test]
    fn m04_a_corrupted_input_may_not_borrow_an_injection_outcome() {
        let mut record = corrupted();
        record.outcome.open_error = CIO_ERROR;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::OutcomeDisagreesWithClassification { .. })
        ));
    }

    #[test]
    fn m05_an_injected_failure_may_not_borrow_a_corruption_code() {
        let mut record = recorded()
            .into_iter()
            .find(|record| record.injection == Injection::Mmap)
            .expect("a suppressed map");
        record.outcome.chunk_error = ERR_BAD_CHECKSUM;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::OutcomeDisagreesWithClassification { .. })
        ));
    }

    #[test]
    fn m06_a_refused_open_may_not_report_a_returned_chunk() {
        let mut record = corrupted();
        record.outcome.chunk = true;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::OutcomeDisagreesWithClassification { .. })
        ));
    }

    #[test]
    fn m07_a_clean_open_may_not_report_a_refusal() {
        let mut record = first();
        record.outcome.chunk = false;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::OutcomeDisagreesWithClassification { .. })
        ));
    }

    #[test]
    fn m08_an_injection_without_a_suppressed_call_is_refused() {
        let mut record = first();
        record.injection = Injection::Close;
        record.injected_errno = 5;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::InjectionIsNotAccountedFor { .. })
        ));
    }

    #[test]
    fn m09_a_suppressed_call_without_an_injection_is_refused() {
        let mut record = first();
        record.suppressed_calls = 1;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::InjectionIsNotAccountedFor { .. })
        ));
    }

    #[test]
    fn m10_a_suppressed_call_with_no_errno_is_refused() {
        let mut record = recorded()
            .into_iter()
            .find(|record| record.injection == Injection::Close)
            .expect("a suppressed close");
        record.injected_errno = 0;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::InjectedErrnoDisagrees { .. })
        ));
    }

    #[test]
    fn m11_an_unbalanced_allocation_ledger_is_refused() {
        let mut record = first();
        record.allocations.library_released -= 1;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::AllocationsDoNotBalance { .. })
        ));
    }

    #[test]
    fn m12_an_outstanding_generation_is_refused() {
        let mut record = first();
        record.allocations.library_released -= 1;
        record.allocations.outstanding += 1;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::OutstandingAllocations { .. })
        ));
    }

    #[test]
    fn m13_the_harness_may_not_claim_a_library_allocation_release() {
        let mut record = first();
        record.allocations.library_released -= 1;
        record.allocations.harness_released += 1;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::HarnessClaimedAnAllocationRelease { .. })
        ));
    }

    #[test]
    fn m14_a_suppressed_close_credited_to_the_library_is_refused() {
        let mut record = recorded()
            .into_iter()
            .find(|record| record.injection == Injection::Close)
            .expect("a suppressed close");
        record.descriptors.library_released += 1;
        record.descriptors.harness_released -= 1;
        record.reclaimed_descriptors -= 1;
        record.descriptors.residual_after_parents = false;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::LibraryCreditedWithASuppressedCall { .. })
        ));
    }

    #[test]
    fn m15_an_unbalanced_descriptor_ledger_is_refused() {
        let mut record = first();
        record.descriptors.epochs += 1;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::EndpointsDoNotBalance {
                endpoint: "descriptor",
                ..
            })
        ));
    }

    #[test]
    fn m16_an_unbalanced_mapping_ledger_is_refused() {
        let mut record = first();
        record.mappings.library_released += 1;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::EndpointsDoNotBalance {
                endpoint: "mapping",
                ..
            })
        ));
    }

    #[test]
    fn m17_a_residual_with_no_reclamation_is_refused() {
        let mut record = recorded()
            .into_iter()
            .find(|record| record.injection == Injection::Close)
            .expect("a suppressed close");
        record.reclaimed_descriptors = 0;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::ResidualAndReclamationDisagree { .. })
        ));
    }

    #[test]
    fn m18_a_reclamation_with_no_residual_is_refused() {
        let mut record = first();
        record.reclaimed_descriptors = 1;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::ResidualAndReclamationDisagree { .. })
        ));
    }

    #[test]
    fn m19_a_residual_that_no_suppression_explains_is_refused() {
        let mut record = first();
        record.descriptors.harness_released = 1;
        record.descriptors.library_released = 0;
        record.descriptors.residual_after_parents = true;
        record.reclaimed_descriptors = 1;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::ResidualIsNotExplainedBySuppression { .. })
        ));
    }

    #[test]
    fn m20_a_replaced_fixture_inode_is_refused() {
        let mut record = first();
        record.inode_preserved = false;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::FixtureIdentityChanged { .. })
        ));
    }

    #[test]
    fn m21_a_resized_fixture_is_refused() {
        let mut record = first();
        record.size_preserved = false;
        assert!(matches!(
            validate_case(&record),
            Err(IntegrityRefusal::FixtureIdentityChanged { .. })
        ));
    }

    #[test]
    fn m22_an_unknown_schema_is_refused() {
        let text = RECORDED.replacen("_v1", "_v2", 1);
        assert!(matches!(
            parse_records(&text),
            Err(IntegrityRefusal::Malformed { line: 1, .. })
        ));
    }

    #[test]
    fn m23_a_missing_identity_line_is_refused() {
        let text: String = RECORDED
            .lines()
            .filter(|line| !line.starts_with("identity "))
            .map(|line| format!("{line}\n"))
            .collect();
        assert!(matches!(
            parse_records(&text),
            Err(IntegrityRefusal::Malformed { .. })
        ));
    }

    #[test]
    fn m24_a_truncated_block_is_refused() {
        let text: String = RECORDED
            .lines()
            .filter(|line| !line.starts_with("mapping "))
            .map(|line| format!("{line}\n"))
            .collect();
        assert!(matches!(
            parse_records(&text),
            Err(IntegrityRefusal::Malformed { .. })
        ));
    }

    #[test]
    fn m25_a_block_that_closes_on_another_case_is_refused() {
        let text = RECORDED.replacen("end case 400", "end case 401", 1);
        assert!(matches!(
            parse_records(&text),
            Err(IntegrityRefusal::Malformed { .. })
        ));
    }

    #[test]
    fn m26_an_unknown_content_class_is_refused() {
        let text = RECORDED.replacen("kind=clean ", "kind=repaired ", 1);
        assert!(matches!(
            parse_records(&text),
            Err(IntegrityRefusal::Malformed { .. })
        ));
    }

    #[test]
    fn m27_an_unknown_injected_operation_is_refused() {
        let text = RECORDED.replacen("op=none", "op=truncate", 1);
        assert!(matches!(
            parse_records(&text),
            Err(IntegrityRefusal::Malformed { .. })
        ));
    }

    #[test]
    fn m28_an_odd_length_image_is_refused() {
        let text = RECORDED.replacen("pre=c1", "pre=c", 1);
        assert!(matches!(
            parse_records(&text),
            Err(IntegrityRefusal::Malformed { .. })
        ));
    }

    #[test]
    fn m29_an_empty_record_text_is_refused() {
        assert!(matches!(
            parse_records(""),
            Err(IntegrityRefusal::Malformed { .. })
        ));
    }

    #[test]
    fn m30_a_family_refuses_wholesale_when_one_case_is_forged() {
        let mut records = recorded();
        records[3].inode_preserved = false;
        assert!(validate_family(&records).is_err());
    }
}
