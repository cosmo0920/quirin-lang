//! `source.buffer.v1` — the checked model behind the `std.buffer` bundled
//! module (card `STD-BUFFER`).
//!
//! # What this model is, stated before anything else
//!
//! A program on this route is a flat, ordered list of events in one entry
//! function. It may **acquire an owned byte buffer of a declared length**, read
//! and write one byte at a checked index, bind that length as an ordinary
//! integer, release the buffer, and — reusing `source.std-io.v1`'s statements
//! unchanged — write rendered lines.
//!
//! # The extent, and why it is transported rather than remembered
//!
//! [`BufferEvent::Acquire`] lowers to an
//! [`crate::ownership::OwnershipOp::Acquire`] whose footprint is
//! `Footprint::single("bytes", extent)`. That is not a new mechanism: the
//! cycle-3 ownership IR has carried an exact componentwise resource quantity
//! since it was written, [`crate::ownership::OwnershipState::live`] aggregates
//! it across the ledger, and [`crate::ownership::check_ownership`] refuses a
//! normal exit that leaves any of it live. What no source program could do
//! before this card was **acquire on a byte axis**.
//!
//! So the length a program declares lives in the custody model's own ledger for
//! the resource's whole lifetime, and [`BufferEvent::Extent`] reads it back out
//! of there. There is no second table that could disagree with it, which is
//! what makes the extent *transported* rather than *remembered*.
//!
//! This is the capability `ExternAssumption::NoExtentTransport` names as
//! missing — supplied **on the source side only**. A declared C04 contract
//! still cannot say how many bytes a callee touches, and nothing here changes
//! that; see `docs/design/STD_BUFFER_V1.md` §10.
//!
//! # What is inherited, and what is new
//!
//! Every **custody** refusal on this route is the unedited ownership checker's:
//! a leak at normal exit is `NormalPathLeak`, a double release is `DoubleDrop`,
//! an access after release is `UseAfterDrop`, an unbound name is
//! `UnknownBinding`, and a rebound live name is `BindingAlreadyExists`. This
//! module builds the operation list and attributes the diagnostic; it decides
//! none of those rules and re-implements none of them (the `PIPE-CHAIN`
//! "inherit, don't reimplement" precedent).
//!
//! What **is** new is everything about *indices*, because nothing in the tree
//! had ever indexed anything: the bounds rule, the byte-range rule, and the
//! decidability rule below.
//!
//! # The decidability rule
//!
//! [`BufferOperand`] has the same two constructors [`crate::std_io::StdIoOperand`]
//! has, and **neither can denote a resource**: an index and a stored byte are an
//! integer literal or a name bound to a check-time-constant integer. A buffer is
//! named only at the four positions typed `name: String`, and every one of those
//! is a custody operation.
//!
//! Every operand is therefore decidable at check time, and that is deliberate
//! rather than incidental. This route's evidence is a **byte comparison between
//! a reference execution and a native run**; a value the reference interpreter
//! cannot compute is a value it cannot predict a line for, so a run-time-varying
//! index would leave "the" observation ill-defined. It is refused by name
//! (`Q052`), and the missing capability — a relational domain over indices — is
//! the named follow-on `STD-BUFFER-VARYING-INDEX`.
//!
//! # What is not claimed
//!
//! * **No string, no text, no encoding.** A byte buffer is not a string. There
//!   is no literal, no concatenation, and no fact anywhere about what a byte
//!   sequence *says*.
//! * **Partial correctness only.** The emitted program aborts on an allocation
//!   failure and on an out-of-range index, so the claim is: *if it produces an
//!   observation at all, that observation is the reference observation.* An
//!   aborting run produces none and so contradicts none.
//! * **The emitted bounds check is a witness, not the check.** Every index in an
//!   admitted program is a constant this checker already proved in range, so the
//!   emitted comparison is provably never taken. If it ever fired, that would be
//!   a defect in this checker.
//! * **No buffer crosses the extern boundary** (`Q055`, and the extern route's
//!   own refusal for a module that does both). A C04 pointer is an extentless
//!   `u8` projection, so passing an extent-carrying buffer through one would
//!   discard the fact this card adds and replace it with a premise.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ownership::{self, Footprint, OwnershipError, OwnershipOp, ResourceId};
use crate::std_io::{ObservedLine, Rendering, StdIoEvent, StdIoOperand, rendered_line};
use crate::typed_errors::{Effect, EffectRow};

/// The largest extent this route admits, in bytes.
///
/// Bounded so the model's own byte vector, the emitted `calloc` and the
/// certificate's live-byte total are all exactly representable and cannot
/// overflow the ownership footprint's `u64` axis. A larger request is refused
/// by name rather than truncated — an extent this route cannot model exactly is
/// not modelled at all, the same discipline `source.std-io.v1` applies to an
/// integer literal it cannot render exactly.
pub const MAX_EXTENT: u32 = 4096;

/// The resource-type name this route's acquisitions carry in the ownership
/// ledger. One shape, so a certificate reader can tell this route's resources
/// from any other model's.
pub const BUFFER_RESOURCE_TYPE: &str = "std.buffer.Bytes";

/// The ownership footprint axis the extent is carried on.
pub const BYTE_AXIS: &str = "bytes";

/// What an index or a stored byte may be.
///
/// **This enum is half the effect discipline** (the other half is that a buffer
/// is nameable only at a custody position). Neither constructor can denote a
/// resource, so an index can never be a buffer and a stored byte can never be
/// one either. Both forms are check-time constants — see the module header on
/// decidability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BufferOperand {
    /// An integer literal written at the site.
    Literal(i32),
    /// A name this entry bound to a check-time-constant integer — by a literal
    /// `let`, by `extent(NAME)`, or by a byte read.
    Scalar(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BufferEvent {
    /// `let NAME = INTEGER;` — an ordinary integer binding.
    Bind { binding: String, value: i32 },
    /// `let NAME = bytes(INTEGER);` — acquire an owned buffer of `extent`
    /// zero-initialised bytes. The custody obligation opens here.
    ///
    /// Spelled `bytes` and not `buffer`: `buffer` occurs 555 times across the
    /// source-side corpus **in binding position** in shipped fixtures
    /// (`let buffer = qcore_buffer_new(1)?;`), so reserving it would refuse
    /// them. The syntax was renamed rather than the pinned surface edited — the
    /// `work` → `effort` and `error` → `fatal` precedent.
    Acquire { name: String, extent: u32 },
    /// `NAME[INDEX] = VALUE;` — write one byte.
    Store {
        name: String,
        index: BufferOperand,
        value: BufferOperand,
    },
    /// `let BINDING = NAME[INDEX];` — read one byte into an integer.
    Load {
        binding: String,
        name: String,
        index: BufferOperand,
    },
    /// `let BINDING = extent(NAME);` — bind the buffer's length, read out of
    /// the ownership ledger's own footprint.
    Extent { binding: String, name: String },
    /// `drop NAME;` — release. Deliberately the **existing** reserved word: the
    /// operation *is* [`OwnershipOp::Drop`], so a new `release` keyword would
    /// have been a synonym for one the language already spells.
    Release { name: String },
    /// A `source.std-io.v1` line-producing statement, reused unchanged. The
    /// bytes of the line are decided by [`rendered_line`] and nowhere else, so
    /// "this route's lines are the standard-IO route's lines" is a statement
    /// about code rather than about two implementations that agree.
    Line(StdIoEvent),
    /// The entry's return. Exactly one, last.
    Return,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferProgram {
    pub entry: String,
    /// The entry's declared `effects { … }` row, verbatim from source.
    pub declared_effects: EffectRow,
    pub events: Vec<BufferEvent>,
}

/// One checked access, published so a certificate names every index this
/// program touched rather than only the lines it wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferAccess {
    pub event: usize,
    pub name: String,
    pub index: u32,
    /// The byte written, or the byte read.
    pub byte: u8,
    pub write: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferCertificate {
    pub entry: String,
    /// The derived row, recomputed by the recheck and compared.
    pub effects: EffectRow,
    /// `(name, extent)` for every acquisition, in program order. **This is the
    /// extent transport, published**: a reader sees how many bytes each
    /// resource was declared to be without re-deriving anything.
    pub acquisitions: Vec<(String, u32)>,
    /// Every release, in program order.
    pub releases: Vec<String>,
    /// The maximum live byte total the program ever holds, taken from the
    /// ownership state's own `live()` footprint after each event. Zero at the
    /// normal exit is not published here because it is not optional: the
    /// inherited checker refuses a program for which it is not.
    pub peak_live_bytes: u64,
    /// Every checked byte access, in program order.
    pub accesses: Vec<BufferAccess>,
    /// The final scalar environment, so a reader can see what each rendered
    /// name denoted.
    pub scalars: BTreeMap<String, i32>,
    /// Every line this program produces, in program order across both streams.
    pub lines: Vec<ObservedLine>,
    /// How many `flush` barriers the program performs.
    pub flushes: usize,
    /// The renderings actually used.
    pub renderings: BTreeSet<Rendering>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BufferError {
    /// `Q051`: the flat entry shape this profile models is violated.
    Shape { event: usize, reason: String },
    /// `Q052`: an operand that is not a check-time constant this entry bound.
    NotAScalar { event: usize, name: String },
    /// `Q052`: a custody position naming something this entry bound no buffer
    /// for.
    NotABuffer { event: usize, name: String },
    /// `Q053`: an index outside `[0, extent)`.
    OutOfBounds {
        event: usize,
        name: String,
        index: i32,
        extent: u32,
    },
    /// `Q053`: a stored byte outside `0..=255`.
    ByteOutOfRange { event: usize, value: i32 },
    /// `Q053`: a declared extent outside `1..=MAX_EXTENT`.
    ExtentOutOfRange { event: usize, extent: i64 },
    /// `Q054`: a refusal reported by the **inherited, unedited** ownership
    /// checker — a leak, a double release, an access after release, an unbound
    /// name, or a rebound live name. This variant carries that checker's own
    /// error value rather than a re-worded copy of it, so the report can show
    /// the refusal arriving from the checker it was inherited from.
    Custody {
        event: usize,
        error: OwnershipError,
    },
    /// `Q055`: a buffer at a position this route refuses to let it cross —
    /// today, a rendering operand.
    CrossesBoundary { event: usize, name: String },
    /// `Q029`'s rule on this route's own code: the declared effect row does not
    /// admit a derived effect.
    EffectNotDeclared { event: usize, effect: Effect },
}

impl BufferError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Shape { .. } => "Q051",
            Self::NotAScalar { .. } | Self::NotABuffer { .. } => "Q052",
            Self::OutOfBounds { .. }
            | Self::ByteOutOfRange { .. }
            | Self::ExtentOutOfRange { .. } => "Q053",
            Self::Custody { .. } => "Q054",
            Self::CrossesBoundary { .. } => "Q055",
            Self::EffectNotDeclared { .. } => "Q029",
        }
    }

    pub fn event(&self) -> usize {
        match self {
            Self::Shape { event, .. }
            | Self::NotAScalar { event, .. }
            | Self::NotABuffer { event, .. }
            | Self::OutOfBounds { event, .. }
            | Self::ByteOutOfRange { event, .. }
            | Self::ExtentOutOfRange { event, .. }
            | Self::Custody { event, .. }
            | Self::CrossesBoundary { event, .. }
            | Self::EffectNotDeclared { event, .. } => *event,
        }
    }
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape { reason, .. } => f.write_str(reason),
            Self::NotAScalar { name, .. } => write!(
                f,
                "`{name}` is not a check-time-constant integer this entry bound; \
                 `source.buffer.v1` indexes and stores with an integer literal or a name bound \
                 to one, so that every access is decided before emission"
            ),
            Self::NotABuffer { name, .. } => write!(
                f,
                "`{name}` is not a buffer this entry acquired; `source.buffer.v1` names a buffer \
                 only at a custody position, and every such position must name one"
            ),
            Self::OutOfBounds {
                name,
                index,
                extent,
                ..
            } => write!(
                f,
                "index {index} is outside `{name}`, whose declared extent is {extent} bytes; \
                 `source.buffer.v1` decides every access at check time, so this is a refusal and \
                 not a run-time trap"
            ),
            Self::ByteOutOfRange { value, .. } => write!(
                f,
                "{value} is not a byte; `source.buffer.v1` stores 0 through 255 and refuses \
                 anything else rather than truncating it"
            ),
            Self::ExtentOutOfRange { extent, .. } => write!(
                f,
                "{extent} is not an extent `source.buffer.v1` models; a buffer is 1 to \
                 {MAX_EXTENT} bytes, and a larger request is refused rather than truncated"
            ),
            Self::Custody { error, .. } => write!(
                f,
                "the checked ownership discipline refuses this program: {error}"
            ),
            Self::CrossesBoundary { name, .. } => write!(
                f,
                "`{name}` is an owned buffer and cannot cross here; a rendering operand is a \
                 scalar, and `source.buffer.v1` publishes no custody identity for a borrowed \
                 render. Passing a buffer to a declared extern callee is refused for a separate \
                 reason: a C04 pointer is an extentless `u8` projection, so the extent this route \
                 carries would be discarded and replaced by a premise"
            ),
            Self::EffectNotDeclared { effect, .. } => write!(
                f,
                "this entry performs `{effect:?}` but its declared `effects {{ … }}` row does not \
                 admit it"
            ),
        }
    }
}

/// The derived effect row.
///
/// The first row in the tree that is neither `{Io}` nor empty: an acquisition
/// and a release are `Alloc`, a store is `Mut`, and a line is `Io`. A read
/// derives nothing — it observes.
pub fn derived_effects(program: &BufferProgram) -> EffectRow {
    let mut effects = BTreeSet::new();
    for event in &program.events {
        match event {
            BufferEvent::Acquire { .. } | BufferEvent::Release { .. } => {
                effects.insert(Effect::Alloc);
            }
            BufferEvent::Store { .. } => {
                effects.insert(Effect::Mut);
            }
            BufferEvent::Line(
                StdIoEvent::Print { .. } | StdIoEvent::Log { .. } | StdIoEvent::Flush,
            ) => {
                effects.insert(Effect::Io);
            }
            BufferEvent::Bind { .. }
            | BufferEvent::Load { .. }
            | BufferEvent::Extent { .. }
            | BufferEvent::Line(StdIoEvent::Return)
            | BufferEvent::Return => {}
        }
    }
    EffectRow::new(effects)
}

/// The **custody half** of the check, and the whole of what this route inherits.
///
/// Builds the cycle-3 ownership operation list from the event sequence and
/// hands it to [`ownership::check_ownership`] unchanged. Every leak, double
/// release, access after release, unbound name and rebound live name is decided
/// there — this function decides none of them.
///
/// Published separately from [`check_buffer_program`] so a test can show the
/// refusal arriving as an [`OwnershipError`] from the inherited checker rather
/// than as a re-worded copy, which is the evidence that it *is* inherited.
///
/// Only names some [`BufferEvent::Acquire`] binds contribute an operation. That
/// is deliberate and it weakens nothing: the ownership checker is asked about
/// **resources**, and a name that is not one is not a resource whose custody it
/// could decide. Such a name is refused by this route's own
/// [`BufferError::NotABuffer`] instead, which says the true thing — "this is
/// not a buffer" — rather than the ownership checker's `UnknownBinding`, which
/// would say "this is not a resource I was told about" and is a fact about the
/// operation list rather than about the program. Every refusal that *is* about
/// custody — a leak, a double release, an access after release, a rebound live
/// name — still arrives from the checker, because those all concern a name it
/// was told about.
pub fn custody_operations(program: &BufferProgram) -> Vec<OwnershipOp> {
    let acquired: BTreeSet<&str> = program
        .events
        .iter()
        .filter_map(|event| match event {
            BufferEvent::Acquire { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let names = |name: &String| acquired.contains(name.as_str());

    let mut operations = Vec::new();
    let mut next_id: ResourceId = 1;
    for event in &program.events {
        match event {
            BufferEvent::Acquire { name, extent } => {
                operations.push(OwnershipOp::Acquire {
                    id: next_id,
                    owner: name.clone(),
                    resource_type: BUFFER_RESOURCE_TYPE.to_owned(),
                    footprint: Footprint::single(BYTE_AXIS, u64::from(*extent)),
                });
                next_id += 1;
            }
            BufferEvent::Store { name, .. } if names(name) => {
                operations.push(OwnershipOp::Write(name.clone()));
            }
            BufferEvent::Load { name, .. } | BufferEvent::Extent { name, .. }
                if names(name) =>
            {
                operations.push(OwnershipOp::Read(name.clone()));
            }
            BufferEvent::Release { name } if names(name) => {
                operations.push(OwnershipOp::Drop(name.clone()));
            }
            BufferEvent::Store { .. }
            | BufferEvent::Load { .. }
            | BufferEvent::Extent { .. }
            | BufferEvent::Release { .. } => {}
            BufferEvent::Bind { .. } | BufferEvent::Line(_) | BufferEvent::Return => {}
        }
    }
    operations
}

/// The state the value half of the check carries: which names are buffers, what
/// their declared extents are, what bytes they hold, and what each scalar name
/// denotes.
struct Evaluation {
    extents: BTreeMap<String, u32>,
    contents: BTreeMap<String, Vec<u8>>,
    scalars: BTreeMap<String, i32>,
}

impl Evaluation {
    fn operand(
        &self,
        event: usize,
        operand: &BufferOperand,
    ) -> Result<i32, BufferError> {
        match operand {
            BufferOperand::Literal(value) => Ok(*value),
            BufferOperand::Scalar(name) => {
                if self.extents.contains_key(name) {
                    // A buffer at a scalar position. Reported as the boundary
                    // refusal rather than the generic one, because the reader
                    // needs to know *why* a name they bound is not usable here.
                    return Err(BufferError::CrossesBoundary {
                        event,
                        name: name.clone(),
                    });
                }
                self.scalars
                    .get(name)
                    .copied()
                    .ok_or_else(|| BufferError::NotAScalar {
                        event,
                        name: name.clone(),
                    })
            }
        }
    }

    /// Resolve `NAME[INDEX]` to a validated `(extent, index)` pair. The bounds
    /// rule lives here and nowhere else, so a read and a write cannot disagree
    /// about it.
    fn place(
        &self,
        event: usize,
        name: &str,
        index: &BufferOperand,
    ) -> Result<(u32, usize), BufferError> {
        let Some(extent) = self.extents.get(name).copied() else {
            return Err(BufferError::NotABuffer {
                event,
                name: name.to_owned(),
            });
        };
        let index = self.operand(event, index)?;
        if index < 0 || u32::try_from(index).is_ok_and(|index| index >= extent) {
            return Err(BufferError::OutOfBounds {
                event,
                name: name.to_owned(),
                index,
                extent,
            });
        }
        Ok((extent, index as usize))
    }
}

/// Check a lowered program and derive its certificate.
///
/// Two halves, in this order and deliberately:
///
/// 1. **Custody**, decided entirely by [`ownership::check_ownership`] over
///    [`custody_operations`]. Running it first is what makes the value half
///    able to assume a name it reads is live.
/// 2. **Values** — extents, indices, byte ranges, scalars and lines — which is
///    the part this card actually owns.
pub fn check_buffer_program(
    program: &BufferProgram,
) -> Result<BufferCertificate, Vec<BufferError>> {
    let mut errors = Vec::new();

    // --- Half one: the inherited custody discipline. ---
    let operations = custody_operations(program);
    if let Err(error) = ownership::check_ownership(&operations) {
        // A custody refusal is attributed to the end of the program rather than
        // guessed at a statement: `NormalPathLeak` is genuinely about the exit,
        // and the lowering carries a scope span for exactly this case.
        errors.push(BufferError::Custody {
            event: program.events.len(),
            error,
        });
    }

    // The peak live byte total, taken from the ownership state's own footprint
    // after each prefix rather than counted here. `execute_ownership` is used
    // instead of `check_ownership` because a prefix legitimately has live
    // resources; the exit rule is the call above.
    let mut peak_live_bytes = 0u64;
    for length in 0..=operations.len() {
        if let Ok(state) = ownership::execute_ownership(&operations[..length]) {
            peak_live_bytes = peak_live_bytes.max(state.live().get(BYTE_AXIS));
        }
    }

    // --- Half two: the values. ---
    let mut evaluation = Evaluation {
        extents: BTreeMap::new(),
        contents: BTreeMap::new(),
        scalars: BTreeMap::new(),
    };
    let mut acquisitions = Vec::new();
    let mut releases = Vec::new();
    let mut accesses = Vec::new();
    let mut lines = Vec::new();
    let mut flushes = 0usize;
    let mut renderings = BTreeSet::new();
    let mut returned = false;

    for (index, event) in program.events.iter().enumerate() {
        if returned {
            errors.push(BufferError::Shape {
                event: index,
                reason: "this entry continues after its return; `source.buffer.v1` models one \
                         flat sequence ending in exactly one return"
                    .to_owned(),
            });
            break;
        }
        match event {
            BufferEvent::Bind { binding, value } => {
                evaluation.scalars.insert(binding.clone(), *value);
            }
            BufferEvent::Acquire { name, extent } => {
                if *extent == 0 || *extent > MAX_EXTENT {
                    errors.push(BufferError::ExtentOutOfRange {
                        event: index,
                        extent: i64::from(*extent),
                    });
                    // Registered anyway, and deliberately: the errors are
                    // collected rather than fatal, so a refused acquisition
                    // that left its name unbound would make every later
                    // statement about it cascade into a second, misleading
                    // refusal. The program is refused either way; what this
                    // buys is that the report names the one thing that is
                    // wrong with it.
                }
                evaluation.extents.insert(name.clone(), *extent);
                // Zero-initialised, and the emitted program uses `calloc` for
                // exactly this reason: a `malloc` would leave the bytes
                // indeterminate, so a read before a write would be undefined
                // behaviour natively and unpredictable in the reference.
                evaluation
                    .contents
                    .insert(name.clone(), vec![0u8; *extent as usize]);
                acquisitions.push((name.clone(), *extent));
            }
            BufferEvent::Store { name, index: at, value } => {
                let place = evaluation.place(index, name, at);
                let value = evaluation.operand(index, value);
                match (place, value) {
                    (Ok((_, at)), Ok(value)) => {
                        let Ok(byte) = u8::try_from(value) else {
                            errors.push(BufferError::ByteOutOfRange {
                                event: index,
                                value,
                            });
                            continue;
                        };
                        if let Some(bytes) = evaluation.contents.get_mut(name) {
                            bytes[at] = byte;
                        }
                        accesses.push(BufferAccess {
                            event: index,
                            name: name.clone(),
                            index: at as u32,
                            byte,
                            write: true,
                        });
                    }
                    (place, value) => {
                        errors.extend(place.err());
                        errors.extend(value.err());
                    }
                }
            }
            BufferEvent::Load {
                binding,
                name,
                index: at,
            } => match evaluation.place(index, name, at) {
                Ok((_, at)) => {
                    let byte = evaluation
                        .contents
                        .get(name)
                        .and_then(|bytes| bytes.get(at).copied())
                        .unwrap_or(0);
                    evaluation
                        .scalars
                        .insert(binding.clone(), i32::from(byte));
                    accesses.push(BufferAccess {
                        event: index,
                        name: name.clone(),
                        index: at as u32,
                        byte,
                        write: false,
                    });
                }
                Err(error) => errors.push(error),
            },
            BufferEvent::Extent { binding, name } => {
                match evaluation.extents.get(name).copied() {
                    // The extent read back out of the acquisition. `i32` is
                    // total here because `MAX_EXTENT` is far inside it.
                    Some(extent) => {
                        evaluation
                            .scalars
                            .insert(binding.clone(), extent as i32);
                    }
                    None => errors.push(BufferError::NotABuffer {
                        event: index,
                        name: name.clone(),
                    }),
                }
            }
            BufferEvent::Release { name } => {
                if evaluation.extents.contains_key(name) {
                    releases.push(name.clone());
                } else {
                    errors.push(BufferError::NotABuffer {
                        event: index,
                        name: name.clone(),
                    });
                }
            }
            BufferEvent::Line(line) => match line {
                StdIoEvent::Print { rendering, operand }
                | StdIoEvent::Log {
                    rendering, operand, ..
                } => {
                    renderings.insert(*rendering);
                    match line_operand(&evaluation, index, operand) {
                        Ok(value) => lines.push(
                            rendered_line(line, value)
                                .expect("a print and a log each produce exactly one line"),
                        ),
                        Err(error) => errors.push(error),
                    }
                }
                StdIoEvent::Flush => flushes += 1,
                StdIoEvent::Return => errors.push(BufferError::Shape {
                    event: index,
                    reason: "a return is this route's own event, not a standard-IO one".to_owned(),
                }),
            },
            BufferEvent::Return => returned = true,
        }
    }

    if !returned {
        errors.push(BufferError::Shape {
            event: program.events.len(),
            reason: "this entry never returns; `source.buffer.v1` requires exactly one return"
                .to_owned(),
        });
    }

    let derived = derived_effects(program);
    for effect in derived.iter() {
        if !program.declared_effects.contains(effect) {
            errors.push(BufferError::EffectNotDeclared {
                event: program.events.len(),
                effect: effect.clone(),
            });
        }
    }

    if errors.is_empty() {
        Ok(BufferCertificate {
            entry: program.entry.clone(),
            effects: derived,
            acquisitions,
            releases,
            peak_live_bytes,
            accesses,
            scalars: evaluation.scalars,
            lines,
            flushes,
            renderings,
        })
    } else {
        Err(errors)
    }
}

/// Resolve a rendering operand against this route's scalar environment.
///
/// `source.std-io.v1`'s three operand forms are handled here rather than
/// re-derived: a literal is itself, a scalar name is looked up, and a
/// `borrow NAME` refuses — this route publishes no custody identity, so the
/// structural refusal `STD-FMT-BORROW` established for the uncomposed route
/// applies here for exactly the same reason.
fn line_operand(
    evaluation: &Evaluation,
    event: usize,
    operand: &StdIoOperand,
) -> Result<i32, BufferError> {
    match operand {
        StdIoOperand::Literal(value) => Ok(*value),
        StdIoOperand::Scalar(name) => {
            evaluation.operand(event, &BufferOperand::Scalar(name.clone()))
        }
        StdIoOperand::Borrowed(name) => Err(BufferError::CrossesBoundary {
            event,
            name: name.clone(),
        }),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BufferRecheckFailure {
    Entry,
    Effects,
    Acquisitions,
    Releases,
    PeakLiveBytes,
    Accesses,
    Scalars,
    Lines,
    Flushes,
    Renderings,
    Refused,
}

impl fmt::Display for BufferRecheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Self::Entry => "the entry name",
            Self::Effects => "the derived effect row",
            Self::Acquisitions => "the acquisitions and the extents they declare",
            Self::Releases => "the releases",
            Self::PeakLiveBytes => "the peak live byte total",
            Self::Accesses => "the checked byte accesses",
            Self::Scalars => "the scalar environment",
            Self::Lines => "the produced lines",
            Self::Flushes => "the barrier count",
            Self::Renderings => "the used renderings",
            Self::Refused => "the program itself, which the recheck refuses",
        };
        write!(f, "{what} does not survive an independent rederivation")
    }
}

/// Rederive the certificate from the program alone and compare it field for
/// field. A published certificate is never one the driver did not recompute.
pub fn recheck_buffer_certificate(
    program: &BufferProgram,
    certificate: &BufferCertificate,
) -> Result<(), BufferRecheckFailure> {
    let rederived = check_buffer_program(program).map_err(|_| BufferRecheckFailure::Refused)?;
    if rederived.entry != certificate.entry {
        return Err(BufferRecheckFailure::Entry);
    }
    if rederived.effects != certificate.effects {
        return Err(BufferRecheckFailure::Effects);
    }
    if rederived.acquisitions != certificate.acquisitions {
        return Err(BufferRecheckFailure::Acquisitions);
    }
    if rederived.releases != certificate.releases {
        return Err(BufferRecheckFailure::Releases);
    }
    if rederived.peak_live_bytes != certificate.peak_live_bytes {
        return Err(BufferRecheckFailure::PeakLiveBytes);
    }
    if rederived.accesses != certificate.accesses {
        return Err(BufferRecheckFailure::Accesses);
    }
    if rederived.scalars != certificate.scalars {
        return Err(BufferRecheckFailure::Scalars);
    }
    if rederived.lines != certificate.lines {
        return Err(BufferRecheckFailure::Lines);
    }
    if rederived.flushes != certificate.flushes {
        return Err(BufferRecheckFailure::Flushes);
    }
    if rederived.renderings != certificate.renderings {
        return Err(BufferRecheckFailure::Renderings);
    }
    Ok(())
}
