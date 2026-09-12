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
//! # The aggregate this route also admits
//!
//! Card `AGGREGATES-V1` admits [`crate::source_record`]'s record type here — a
//! module-level `type NAME { FIELD: i32, … }` declaration, a construction and a
//! field read. A v1 record is a **plain copyable value**: every field is a
//! scalar, so it contributes no ownership operation, opens no obligation and
//! derives no effect, and a record name at a custody or a scalar position is
//! refused. It is admitted here rather than anywhere else because this is the
//! route that already holds a check-time-constant integer environment, which is
//! exactly what a field value is and what a field read produces.
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
use crate::source_loop::{self, BoundRefusal, LoopRefusal, MAX_ITERATIONS, MAX_UNROLLED_EVENTS};
use crate::source_record::{self, RecordRefusal, RecordType, RecordValue};
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
    /// `let NAME = lend(BUFFER, LOW, HIGH);` — lend **write authority** over
    /// the bytes `[LOW, HIGH)` of an owned buffer to a new binding (card
    /// `SLICE-PERMISSIONS-WRITE-LEND`).
    ///
    /// The second value kind this route holds, and the half of CT07 the read
    /// card could not reach: a loan a store can name. `NAME[i] = v` writes the
    /// owner's byte `LOW + i` — index rebasing into the owner's coordinates,
    /// published in the certificate and enforced again by the ownership IR —
    /// and while the loan's window is open the owner's authority over
    /// `[LOW, HIGH)` is genuinely **transferred**: an owner access inside the
    /// range refuses, one outside it is admitted, and two live lends must
    /// cover disjoint ranges. The window runs from this statement to the last
    /// statement naming the loan, exact for the same reason the read card's
    /// window is: the route is one flat statement sequence, which is also why
    /// a `for` beside a lend refuses (`SLICE-PERMISSIONS-LOOP-WINDOW`).
    ///
    /// Bounds are check-time constants of the **lowering-visible tier** — an
    /// integer literal, a literal `let`, or an `extent` binding — the same
    /// deliberately narrow tier a loop bound takes and for the same reason
    /// (`LOOP-BOUND-FROM-BYTE`): the custody projection must state the lend's
    /// footprint, so a bound only the checker's byte model can compute is
    /// refused by name rather than decided in a second place
    /// (`WRITE-LEND-BOUND-FROM-BYTE`).
    Lend {
        binding: String,
        name: String,
        low: BufferOperand,
        high: BufferOperand,
    },
    /// A `source.std-io.v1` line-producing statement, reused unchanged. The
    /// bytes of the line are decided by [`rendered_line`] and nowhere else, so
    /// "this route's lines are the standard-IO route's lines" is a statement
    /// about code rather than about two implementations that agree.
    Line(StdIoEvent),
    /// `for VARIABLE in 0..COUNT { … }` — the checked bounded counted loop
    /// (card `LOOP-V1`, `source.loop.v1`).
    ///
    /// The construct is [`crate::source_loop`]'s and its refusals carry that
    /// module's `Q097`–`Q099` codes; what this variant adds is that its body is
    /// **this route's own events**, read by this route's own reader, so "a loop
    /// body's statements are the byte-buffer route's statements" is a statement
    /// about code rather than about two implementations that agree.
    ///
    /// `COUNT` is known at check time, so the body is checked by being **run
    /// once per iteration** — exactly, with `VARIABLE` bound to the iteration
    /// number — rather than by a fixed point over a back edge. See
    /// [`crate::source_loop`]'s header for why an over-approximation is not an
    /// option on a route whose evidence is a byte comparison.
    Repeat {
        variable: String,
        count: u32,
        body: Vec<BufferEvent>,
    },
    /// `let NAME = TYPE { FIELD: OPERAND, … };` — construct a record value
    /// (card `AGGREGATES-V1`, `source.record.v1`).
    ///
    /// The construct is [`crate::source_record`]'s and its refusals are that
    /// model's, reported on this pass's own band because the aggregate
    /// allocates **no** diagnostic code; what this variant adds is that a field
    /// value is a [`BufferOperand`] — this route's own check-time-constant
    /// integer — so a record's fields are exactly the values this route already
    /// decides, and a record can never carry one it cannot.
    Construct {
        binding: String,
        type_name: String,
        fields: Vec<(String, BufferOperand)>,
    },
    /// `let BINDING = RECORD.FIELD;` — read one field into an integer.
    ///
    /// The result is an ordinary scalar of this route's environment, which is
    /// what makes a field readable, renderable and indexable with **no** new
    /// operand form: `source.std-io.v1`'s existing `Scalar` operand already
    /// denotes it, exactly as card `LOOP-V1` found for a loop index.
    Field {
        binding: String,
        record: String,
        field: String,
    },
    /// The entry's return. Exactly one, last.
    Return,
}

impl BufferEvent {
    /// The events of this event's body, if it has one.
    #[must_use]
    pub fn body(&self) -> &[BufferEvent] {
        match self {
            Self::Repeat { body, .. } => body,
            _ => &[],
        }
    }

    /// Whether `predicate` holds of this event or of any event nested in it.
    pub fn any(&self, predicate: &impl Fn(&BufferEvent) -> bool) -> bool {
        predicate(self) || self.body().iter().any(|event| event.any(predicate))
    }
}

/// Whether `predicate` holds of any event of `events`, at any nesting depth.
pub fn any_event(events: &[BufferEvent], predicate: impl Fn(&BufferEvent) -> bool) -> bool {
    events.iter().any(|event| event.any(&predicate))
}

/// How many events this sequence unrolls to — the honest cost of the checking
/// route. A `Repeat` contributes its own event plus `count` copies of its body.
#[must_use]
pub fn unrolled_event_count(events: &[BufferEvent]) -> usize {
    events
        .iter()
        .map(|event| match event {
            BufferEvent::Repeat { count, body, .. } => {
                1 + (*count as usize).saturating_mul(unrolled_event_count(body))
            }
            _ => 1,
        })
        .fold(0usize, usize::saturating_add)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferProgram {
    pub entry: String,
    /// The entry's declared `effects { … }` row, verbatim from source.
    pub declared_effects: EffectRow,
    /// The record types this **module** declares (card `AGGREGATES-V1`).
    ///
    /// Module-scoped rather than entry-scoped because a record type is a
    /// declaration and not a statement: it is `type NAME { … }` at the top
    /// level, beside the entry rather than inside it. The list is empty for
    /// every program written before that card, which is what keeps their
    /// checking, their certificates and their emitted C byte-identical.
    pub records: Vec<RecordType>,
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

/// One constructed record value, published so a certificate names every field
/// this program supplied rather than only the ones it read back (card
/// `AGGREGATES-V1`).
///
/// The field list is in **declared** order, so two certificates of one program
/// compare field for field regardless of the order the source wrote them in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordConstruction {
    pub event: usize,
    pub binding: String,
    pub type_name: String,
    pub fields: Vec<(String, i32)>,
}

/// One checked field read: which value, which field, and the integer it
/// produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordProjection {
    pub event: usize,
    pub record: String,
    pub type_name: String,
    pub field: String,
    pub value: i32,
}

/// One admitted write lend, published with the **window** over which the
/// owner's authority is transferred and the footprint the loan holds (card
/// `SLICE-PERMISSIONS-WRITE-LEND`).
///
/// `through` is the index of the last event naming the loan; the window
/// `[event, through]` is exact for the read card's reason — the route is one
/// flat statement sequence. `low`/`high` are the owner's coordinates, so a
/// reader checks the rebasing arithmetic (`loan index i` writes owner byte
/// `low + i`) against the published accesses rather than being told it
/// happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LendRecord {
    pub event: usize,
    pub buffer: String,
    pub binding: String,
    pub low: u32,
    pub high: u32,
    pub through: usize,
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
    /// Every admitted write lend, with its footprint and its window (card
    /// `SLICE-PERMISSIONS-WRITE-LEND`). Empty for every program written before
    /// that card, which is what keeps their certificates value-identical.
    pub lends: Vec<LendRecord>,
    /// The maximum live byte total the program ever holds, taken from the
    /// ownership state's own `live()` footprint after each event. Zero at the
    /// normal exit is not published here because it is not optional: the
    /// inherited checker refuses a program for which it is not.
    pub peak_live_bytes: u64,
    /// Every checked byte access, in program order.
    pub accesses: Vec<BufferAccess>,
    /// Every record value this program constructed, in program order, with the
    /// value supplied for each declared field (card `AGGREGATES-V1`).
    pub constructions: Vec<RecordConstruction>,
    /// Every field this program read, in program order. **This is the
    /// distinctness evidence, published**: a reader sees which field each read
    /// answered to and what it produced, without re-deriving anything.
    pub projections: Vec<RecordProjection>,
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

/// The refusals the write lend owns (card `SLICE-PERMISSIONS-WRITE-LEND`),
/// each landing on a condition an existing `source.buffer.v1` code already
/// names — the `Q-NAMESPACE-DECISION` ice box forbids a new allocation, and the
/// widenings are stated in spec §L21 and in the card's report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LendRefusal {
    /// `Q051`: a shape the lend's window model cannot hold — a `for` in an
    /// entry that lends, a crossed pair of windows, an acquisition inside a
    /// window, or a lend binding shadowing a live name.
    Shape(String),
    /// `Q052`: a lend position naming something that is not an owned buffer
    /// this entry acquired — a loan (re-lending is not modelled), or a `drop`
    /// of a loan, whose window closes at its last use instead.
    NotLendable { name: String, reason: String },
    /// `Q053`: `0 <= low <= high <= extent` is violated. Decided at check time
    /// against the extent the acquisition declared, exactly as the read card's
    /// `borrow_utf8` range is, so this is a refusal and not a run-time trap.
    Range {
        buffer: String,
        low: i32,
        high: i32,
        extent: u32,
    },
    /// `Q053`: a lend bound outside the lowering-visible constant tier — a
    /// value read out of a byte or a field. The custody projection must state
    /// the lend's footprint, so a bound only the checker's byte model can
    /// compute is refused by name (`WRITE-LEND-BOUND-FROM-BYTE`, the
    /// `LOOP-BOUND-FROM-BYTE` shape).
    BoundFromByte { name: String },
    /// `Q053`: an index through a loan outside the loan's own `[0, len)`.
    OutOfLend { name: String, index: i32, len: u32 },
    /// `Q055`: a loan at a position this route refuses to let it cross —
    /// a scalar position, a rendering operand, a record field value.
    CrossesBoundary { name: String },
}

impl LendRefusal {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Shape(_) => "Q051",
            Self::NotLendable { .. } => "Q052",
            Self::Range { .. } | Self::BoundFromByte { .. } | Self::OutOfLend { .. } => "Q053",
            Self::CrossesBoundary { .. } => "Q055",
        }
    }
}

impl fmt::Display for LendRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape(reason) => f.write_str(reason),
            Self::NotLendable { name, reason } => write!(f, "`{name}` {reason}"),
            Self::Range {
                buffer,
                low,
                high,
                extent,
            } => write!(
                f,
                "`lend({buffer}, {low}, {high})` is not a range of `{buffer}`, whose declared \
                 extent is {extent} bytes; `source.buffer.v1` lends a range only when \
                 `0 <= low <= high <= extent`, and decides it at check time, so this is a \
                 refusal and not a run-time trap"
            ),
            Self::BoundFromByte { name } => write!(
                f,
                "`{name}` is not a bound the lend's custody projection can state; a lend bound \
                 is an integer literal, a literal `let`, or an `extent` binding, so that the \
                 lend's footprint is decided in one place (`WRITE-LEND-BOUND-FROM-BYTE`)"
            ),
            Self::OutOfLend { name, index, len } => write!(
                f,
                "index {index} is outside `{name}`, a lend of {len} bytes; an access through a \
                 lend is rebased into the owner's coordinates and stays inside the lent range, \
                 decided at check time, so this is a refusal and not a run-time trap"
            ),
            Self::CrossesBoundary { name } => write!(
                f,
                "`{name}` is a lend of a buffer and cannot cross here; a scalar position takes \
                 an integer, and a lend publishes no custody identity for a borrowed render"
            ),
        }
    }
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
    Custody { event: usize, error: OwnershipError },
    /// `Q055`: a buffer at a position this route refuses to let it cross —
    /// today, a rendering operand.
    CrossesBoundary { event: usize, name: String },
    /// `Q057`: the declared `effects { … }` row does not admit a derived
    /// effect.
    ///
    /// Deliberately **this pass's own code and not `StandardIo`'s `Q029`**,
    /// even though the two rules read alike. §L21's allocation rule is that a
    /// code may not be reused by a second semantic pass, and this is a second
    /// pass; the conditions differ besides, because `Q029` reports an
    /// undeclared `Io` on a route that can derive nothing else, while this one
    /// reports an undeclared `Alloc` or `Mut` as well.
    EffectNotDeclared { event: usize, effect: Effect },
    /// `Q097`–`Q099`: a refusal the **bounded counted loop** owns (card
    /// `LOOP-V1`). Carried as [`LoopRefusal`]'s own value rather than a
    /// re-worded copy, and reported at that construct's own codes, because
    /// `source.loop.v1` is a separate semantic pass from this route — §L21
    /// forbids a second pass reusing a code, and the conditions are the loop's
    /// (a bound, an iteration's net custody effect), not the buffer's.
    Loop { event: usize, refusal: LoopRefusal },
    /// A refusal the **write lend** owns (card `SLICE-PERMISSIONS-WRITE-LEND`).
    ///
    /// Like [`Self::Record`], this variant reports at **this pass's own codes**
    /// — `Q051`, `Q052`, `Q053` and `Q055`, whose conditions the card widens —
    /// because the `Q-NAMESPACE-DECISION` ice box forbids a new allocation.
    /// The mapping lives in [`LendRefusal::code`] and nowhere else. The
    /// custody refusals of the lend — an overlap between two lends, an owner
    /// access inside a lent range, a release under a live lend — are **not**
    /// here at all: they arrive from the extended ownership IR through
    /// [`Self::Custody`] at `Q054`, which is what "the authority is genuinely
    /// transferred" means operationally.
    Lend { event: usize, refusal: LendRefusal },
    /// A refusal the **record type** owns (card `AGGREGATES-V1`). Carried as
    /// [`RecordRefusal`]'s own value rather than a re-worded copy.
    ///
    /// Unlike [`Self::Loop`], this variant reports at **this pass's own codes**
    /// — `Q051`, `Q052` and `Q053`, whose conditions the card widens — because
    /// `source.record.v1` allocates no code at all: the `Q-NAMESPACE-DECISION`
    /// ice box forbids a new allocation, so an aggregate refusal lands on the
    /// admitting band's existing shape, name and range conditions. The mapping
    /// lives in [`RecordRefusal::code`] and nowhere else.
    Record {
        event: usize,
        refusal: RecordRefusal,
    },
}

impl BufferError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Loop { refusal, .. } => refusal.code(),
            Self::Record { refusal, .. } => refusal.code(),
            Self::Lend { refusal, .. } => refusal.code(),
            Self::Shape { .. } => "Q051",
            Self::NotAScalar { .. } | Self::NotABuffer { .. } => "Q052",
            Self::OutOfBounds { .. }
            | Self::ByteOutOfRange { .. }
            | Self::ExtentOutOfRange { .. } => "Q053",
            Self::Custody { .. } => "Q054",
            Self::CrossesBoundary { .. } => "Q055",
            Self::EffectNotDeclared { .. } => "Q057",
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
            | Self::Loop { event, .. }
            | Self::Record { event, .. }
            | Self::Lend { event, .. }
            | Self::EffectNotDeclared { event, .. } => *event,
        }
    }

    /// The semantic pass that owns this refusal. Every variant but
    /// [`Self::Loop`] is `source.buffer.v1`'s own; that one is
    /// `source.loop.v1`'s, and saying so is what keeps a reader following a
    /// `Q097` back to the loop's rules rather than to this route's.
    #[must_use]
    pub fn is_loop_refusal(&self) -> bool {
        matches!(self, Self::Loop { .. })
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
            Self::Loop { refusal, .. } => refusal.fmt(f),
            Self::Record { refusal, .. } => refusal.fmt(f),
            Self::Lend { refusal, .. } => refusal.fmt(f),
        }
    }
}

/// The derived effect row.
///
/// An acquisition and a release are `Alloc`, a store is `Mut`, and a line is
/// `Io`. A read derives nothing — it observes.
///
/// `source.extern-call.v1` derives `Alloc` and `Mut` too, so this row is not the
/// first of its shape; what differs is **where the facts come from**. There they
/// follow from a *declared contract*'s projections — `Alloc` when the contract
/// says the callee acquires — and are therefore consequences of a premise. Here
/// they follow from an acquisition and a store this route checks and discharges
/// itself.
pub fn derived_effects(program: &BufferProgram) -> EffectRow {
    fn walk(events: &[BufferEvent], effects: &mut BTreeSet<Effect>) {
        for event in events {
            match event {
                BufferEvent::Acquire { .. } | BufferEvent::Release { .. } => {
                    effects.insert(Effect::Alloc);
                }
                BufferEvent::Store { .. } => {
                    effects.insert(Effect::Mut);
                }
                BufferEvent::Line(
                    StdIoEvent::LogLevel { .. }
                    | StdIoEvent::Print { .. }
                    | StdIoEvent::Log { .. }
                    | StdIoEvent::Flush,
                ) => {
                    effects.insert(Effect::Io);
                }
                // A loop derives exactly what its body derives. It performs no
                // effect of its own — iterating is not an effect — and it is
                // admitted only with a positive bound, so a body's effect is
                // always performed at least once and the row cannot over-report.
                BufferEvent::Repeat { body, .. } => walk(body, effects),
                // **A record operation derives nothing**, and that is the
                // plain-value decision stated as an effect fact (card
                // `AGGREGATES-V1`). A v1 record's fields are scalars, so
                // constructing one allocates nothing — the value lives in
                // automatic storage exactly as a `let`-bound integer does — and
                // reading a field mutates no object of the program: it
                // *observes*, exactly as a byte read does. An owned field would
                // make the first of those false, which is one more reason it is
                // a separate card (`AGGREGATE-OWNED-FIELD`).
                // **A lend derives nothing** (card
                // `SLICE-PERMISSIONS-WRITE-LEND`): authority moves, no byte
                // does. The `Mut` a lend-holding module carries is its
                // stores', which the arm above already derives whichever
                // binding they go through.
                BufferEvent::Construct { .. }
                | BufferEvent::Field { .. }
                | BufferEvent::Bind { .. }
                | BufferEvent::Lend { .. }
                | BufferEvent::Load { .. }
                | BufferEvent::Extent { .. }
                | BufferEvent::Line(StdIoEvent::Return)
                | BufferEvent::Return => {}
            }
        }
    }
    let mut effects = BTreeSet::new();
    walk(&program.events, &mut effects);
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
    custody_operations_with_gates(program).0
}

/// One `for` loop's per-iteration custody identity obligation (card `LOOP-V1`).
///
/// `prefix` is every operation that runs before the loop, and `iteration` is the
/// operations of one pass through its body. Together they are what
/// [`source_loop::iteration_is_custody_identity`] decides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopCustodyGate {
    /// The index of the **top-level** event the refusal is attributed to.
    pub event: usize,
    pub variable: String,
    pub prefix: Vec<OwnershipOp>,
    pub iteration: Vec<OwnershipOp>,
}

/// [`custody_operations`], plus the per-loop identity obligations the unrolled
/// operation list alone would only expose for a bound of two or more.
///
/// The operations are the **unrolled** ones: a `Repeat` contributes its body's
/// operations `count` times, in order, with a fresh resource identity for each
/// acquisition. That is what makes the inherited cycle-3 checker decide a
/// loop's custody — a body that acquires without releasing presents it with two
/// acquisitions of one live name — and it is exact rather than approximate
/// because the count is a number this route holds.
#[must_use]
pub fn custody_operations_with_gates(
    program: &BufferProgram,
) -> (Vec<OwnershipOp>, Vec<LoopCustodyGate>) {
    // Card `SLICE-PERMISSIONS-WRITE-LEND`: a module with **no** lend builds the
    // operation list it always built — this branch is what makes that a
    // structural fact rather than a measurement. A module with one builds the
    // lend-structured projection below, in which a loan is a real borrow node
    // of the ownership IR carrying its footprint fraction.
    if any_event(&program.events, |event| {
        matches!(event, BufferEvent::Lend { .. })
    }) {
        return (lend_custody_operations(program), Vec::new());
    }
    fn acquired_names(events: &[BufferEvent], into: &mut BTreeSet<String>) {
        for event in events {
            match event {
                BufferEvent::Acquire { name, .. } => {
                    into.insert(name.clone());
                }
                BufferEvent::Repeat { body, .. } => acquired_names(body, into),
                _ => {}
            }
        }
    }
    let mut acquired = BTreeSet::new();
    acquired_names(&program.events, &mut acquired);

    struct Walk<'a> {
        acquired: &'a BTreeSet<String>,
        operations: Vec<OwnershipOp>,
        gates: Vec<LoopCustodyGate>,
        next_id: ResourceId,
        next_scope: u32,
    }

    impl Walk<'_> {
        fn names(&self, name: &String) -> bool {
            self.acquired.contains(name)
        }

        /// The owner name this occurrence denotes in the flat operation list.
        /// A body-local binding carries the iteration scope it belongs to; every
        /// other name is itself.
        fn owner(renames: &BTreeMap<String, String>, name: &String) -> String {
            renames.get(name).cloned().unwrap_or_else(|| name.clone())
        }

        fn walk(
            &mut self,
            events: &[BufferEvent],
            attributed: usize,
            top_level: bool,
            renames: &BTreeMap<String, String>,
        ) {
            for (index, event) in events.iter().enumerate() {
                let attributed = if top_level { index } else { attributed };
                match event {
                    BufferEvent::Acquire { name, extent } => {
                        self.operations.push(OwnershipOp::Acquire {
                            id: self.next_id,
                            owner: Self::owner(renames, name),
                            resource_type: BUFFER_RESOURCE_TYPE.to_owned(),
                            footprint: Footprint::single(BYTE_AXIS, u64::from(*extent)),
                        });
                        self.next_id += 1;
                    }
                    BufferEvent::Store { name, .. } if self.names(name) => {
                        self.operations
                            .push(OwnershipOp::Write(Self::owner(renames, name)));
                    }
                    BufferEvent::Load { name, .. } | BufferEvent::Extent { name, .. }
                        if self.names(name) =>
                    {
                        self.operations
                            .push(OwnershipOp::Read(Self::owner(renames, name)));
                    }
                    BufferEvent::Release { name } if self.names(name) => {
                        self.operations
                            .push(OwnershipOp::Drop(Self::owner(renames, name)));
                    }
                    BufferEvent::Repeat {
                        variable,
                        count,
                        body,
                    } => {
                        // Every buffer this body binds is body-local, so each
                        // iteration's is a distinct binding and carries its own
                        // scope in the name. See
                        // `source_loop::ITERATION_SCOPE_SEPARATOR` for why that
                        // is the faithful encoding rather than a relaxation.
                        let local: Vec<String> = body_bindings(body)
                            .into_iter()
                            .filter(|name| self.acquired.contains(name))
                            .collect();
                        let prefix = self.operations.clone();
                        let mut iteration = Vec::new();
                        for pass in 0..*count {
                            let mut inner = renames.clone();
                            let scope = self.next_scope;
                            self.next_scope += 1;
                            for name in &local {
                                inner.insert(
                                    name.clone(),
                                    format!(
                                        "{name}{}{scope}",
                                        source_loop::ITERATION_SCOPE_SEPARATOR
                                    ),
                                );
                            }
                            let start = self.operations.len();
                            self.walk(body, attributed, false, &inner);
                            if pass == 0 {
                                iteration = self.operations[start..].to_vec();
                            }
                        }
                        self.gates.push(LoopCustodyGate {
                            event: attributed,
                            variable: variable.clone(),
                            prefix,
                            iteration,
                        });
                    }
                    BufferEvent::Store { .. }
                    | BufferEvent::Load { .. }
                    | BufferEvent::Extent { .. }
                    | BufferEvent::Release { .. } => {}
                    // Unreachable: a program with a lend takes
                    // `lend_custody_operations` before this walk runs. Kept
                    // total with **no** operation, which cannot admit anything
                    // — a loan this walk did not open makes every later use of
                    // its binding an unknown name to the checker.
                    BufferEvent::Lend { .. } => {}
                    // **A record contributes no custody operation at all**
                    // (card `AGGREGATES-V1`). Every field of a v1 record is a
                    // scalar, so there is no resource for the cycle-3 checker
                    // to be asked about — which is exactly why a record name at
                    // a custody position is refused by this route's existing
                    // "this is not a buffer" rule rather than by anything the
                    // aggregate added.
                    BufferEvent::Construct { .. }
                    | BufferEvent::Field { .. }
                    | BufferEvent::Bind { .. }
                    | BufferEvent::Line(_)
                    | BufferEvent::Return => {}
                }
            }
        }
    }

    let mut walk = Walk {
        acquired: &acquired,
        operations: Vec::new(),
        gates: Vec::new(),
        next_id: 1,
        next_scope: 0,
    };
    walk.walk(&program.events, 0, true, &BTreeMap::new());
    (walk.operations, walk.gates)
}

/// One write lend's window and footprint, as the custody projection states it
/// (card `SLICE-PERMISSIONS-WRITE-LEND`).
///
/// `through` is the index of the **last top-level event naming the loan** — a
/// store, a load, an extent read, or a refused later spelling — and `event`
/// the lend's own, so `[event, through]` is the loan's lexical window. The
/// bounds are resolved from the lowering-visible constant tier and **clamped
/// into `[0, extent]`** when the spelling is out of range: the value half owns
/// that refusal (`Q053`), and the projection stays total so one fault produces
/// one report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LendWindow {
    pub event: usize,
    pub binding: String,
    pub buffer: String,
    pub low: u32,
    pub high: u32,
    pub through: usize,
}

/// Resolve every lend of a flat entry to its window and footprint.
///
/// The second component collects the **tier refusals**: a bound outside the
/// lowering-visible constant tier (a byte read, a field read) cannot be stated
/// by the custody projection and is refused by name
/// (`WRITE-LEND-BOUND-FROM-BYTE`). A lend whose bounds do not resolve, or whose
/// buffer this entry never acquired, still receives a window — over-approximated
/// to the buffer's whole extent, the maximally refusing direction — so the
/// projection below is total whatever the value half goes on to say.
#[must_use]
pub fn lend_windows(events: &[BufferEvent]) -> (Vec<LendWindow>, Vec<BufferError>) {
    let mut scalars: BTreeMap<String, i32> = BTreeMap::new();
    let mut extents: BTreeMap<String, u32> = BTreeMap::new();
    let mut windows = Vec::new();
    let mut errors = Vec::new();

    // The last top-level event naming each lend binding.
    let mut last_use: BTreeMap<&str, usize> = BTreeMap::new();
    let lend_bindings: BTreeSet<&str> = events
        .iter()
        .filter_map(|event| match event {
            BufferEvent::Lend { binding, .. } => Some(binding.as_str()),
            _ => None,
        })
        .collect();
    for (index, event) in events.iter().enumerate() {
        let names: Vec<&str> = match event {
            BufferEvent::Store { name, .. }
            | BufferEvent::Load { name, .. }
            | BufferEvent::Extent { name, .. }
            | BufferEvent::Release { name }
            | BufferEvent::Lend { name, .. } => vec![name.as_str()],
            _ => Vec::new(),
        };
        for name in names {
            if lend_bindings.contains(name) {
                last_use.insert(name, index);
            }
        }
    }

    for (index, event) in events.iter().enumerate() {
        match event {
            BufferEvent::Bind { binding, value } => {
                scalars.insert(binding.clone(), *value);
            }
            BufferEvent::Acquire { name, extent } => {
                extents.insert(name.clone(), *extent);
            }
            BufferEvent::Extent { binding, name } => {
                if let Some(extent) = extents.get(name).copied() {
                    scalars.insert(binding.clone(), extent as i32);
                }
            }
            BufferEvent::Lend {
                binding,
                name,
                low,
                high,
            } => {
                let extent = extents.get(name).copied().unwrap_or(0);
                let mut resolve = |operand: &BufferOperand| -> Option<i32> {
                    match operand {
                        BufferOperand::Literal(value) => Some(*value),
                        BufferOperand::Scalar(scalar) => match scalars.get(scalar) {
                            Some(value) => Some(*value),
                            Option::None => {
                                errors.push(BufferError::Lend {
                                    event: index,
                                    refusal: LendRefusal::BoundFromByte {
                                        name: scalar.clone(),
                                    },
                                });
                                Option::None
                            }
                        },
                    }
                };
                let low = resolve(low);
                let high = resolve(high);
                let (low, high) = match (low, high) {
                    // Clamped, never trusted: the value half refuses an
                    // out-of-range spelling at `Q053`, and the projection
                    // stays inside the footprint the ledger holds.
                    (Some(low), Some(high)) => {
                        let low = low.clamp(0, extent as i32) as u32;
                        let high = high.clamp(low as i32, extent as i32) as u32;
                        (low, high)
                    }
                    // A bound the projection cannot state over-approximates to
                    // the whole extent — the maximally refusing direction.
                    _ => (0, extent),
                };
                windows.push(LendWindow {
                    event: index,
                    binding: binding.clone(),
                    buffer: name.clone(),
                    low,
                    high,
                    through: last_use.get(binding.as_str()).copied().unwrap_or(index),
                });
            }
            _ => {}
        }
    }
    (windows, errors)
}

/// The first crossed pair of lend windows, if any: `a` opens first, `b` opens
/// inside `a`'s window and outlives it. The ownership IR's borrow nodes are
/// lexical, so a crossed pair cannot be encoded as nesting; it is refused by
/// name rather than approximated — the `SOCKET-EVENT-CROSSED-SPANS` precedent,
/// and residual `SLICE-PERMISSIONS-CROSSED-WINDOWS`.
#[must_use]
pub fn crossed_lend_windows(windows: &[LendWindow]) -> Option<(&LendWindow, &LendWindow)> {
    for (position, a) in windows.iter().enumerate() {
        for b in &windows[position + 1..] {
            if a.event < b.event && b.event <= a.through && b.through > a.through {
                return Some((a, b));
            }
        }
    }
    None
}

/// The custody projection of an entry that lends (card
/// `SLICE-PERMISSIONS-WRITE-LEND`): every lend is an
/// [`OwnershipOp::LendRange`] node whose body is the ops of its window, every
/// resolved access states its byte footprint, and an access the
/// lowering-visible tier cannot resolve stays rangeless — which under a live
/// loan conflicts with everything, the fail-closed direction
/// (`WRITE-LEND-COMPUTED-INDEX`).
fn lend_custody_operations(program: &BufferProgram) -> Vec<OwnershipOp> {
    let acquired: BTreeSet<String> = program
        .events
        .iter()
        .filter_map(|event| match event {
            BufferEvent::Acquire { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    let (windows, _) = lend_windows(&program.events);
    let by_event: BTreeMap<usize, &LendWindow> = windows
        .iter()
        .map(|window| (window.event, window))
        .collect();

    struct Build<'a> {
        acquired: &'a BTreeSet<String>,
        by_event: &'a BTreeMap<usize, &'a LendWindow>,
        scalars: BTreeMap<String, i32>,
        extents: BTreeMap<String, u32>,
        /// binding -> (buffer, low, high) for every loan whose window this
        /// segment is inside.
        loans: BTreeMap<String, (String, u64, u64)>,
        next_id: ResourceId,
    }

    impl Build<'_> {
        fn constant(&self, operand: &BufferOperand) -> Option<u64> {
            let value = match operand {
                BufferOperand::Literal(value) => *value,
                BufferOperand::Scalar(name) => *self.scalars.get(name)?,
            };
            u64::try_from(value).ok()
        }

        fn segment(
            &mut self,
            events: &[BufferEvent],
            start: usize,
            end: usize,
        ) -> Vec<OwnershipOp> {
            let mut operations = Vec::new();
            let mut index = start;
            while index < end {
                let event = &events[index];
                match event {
                    BufferEvent::Bind { binding, value } => {
                        self.scalars.insert(binding.clone(), *value);
                    }
                    BufferEvent::Acquire { name, extent } => {
                        self.extents.insert(name.clone(), *extent);
                        operations.push(OwnershipOp::Acquire {
                            id: self.next_id,
                            owner: name.clone(),
                            resource_type: BUFFER_RESOURCE_TYPE.to_owned(),
                            footprint: Footprint::single(BYTE_AXIS, u64::from(*extent)),
                        });
                        self.next_id += 1;
                    }
                    BufferEvent::Lend { binding, .. } => {
                        if let Some(window) = self.by_event.get(&index).copied()
                            && self.acquired.contains(&window.buffer)
                        {
                            // The window is `[event, through]`, so the body
                            // is the ops of `(event, through]`, clamped to
                            // this segment — the gate refuses a crossed
                            // pair before this projection is consulted, so
                            // the clamp is reachable only through a direct
                            // model-level caller, and then it shortens the
                            // loan, which only *adds* refusals.
                            let body_end = (window.through + 1).min(end);
                            let (low, high) = (u64::from(window.low), u64::from(window.high));
                            self.loans
                                .insert(binding.clone(), (window.buffer.clone(), low, high));
                            let body = self.segment(events, index + 1, body_end);
                            self.loans.remove(binding);
                            operations.push(OwnershipOp::LendRange {
                                owner: window.buffer.clone(),
                                borrower: binding.clone(),
                                axis: BYTE_AXIS.to_owned(),
                                low,
                                high,
                                body,
                            });
                            index = body_end;
                            continue;
                        }
                        // A lend of nothing this entry acquired contributes no
                        // node; the value half refuses the statement, and every
                        // later use of the binding is an unknown name there.
                    }
                    BufferEvent::Store {
                        name, index: at, ..
                    } => {
                        if let Some((_, lend_low, _)) = self.loans.get(name).cloned() {
                            match self.constant(at) {
                                // Rebased into the owner's coordinates; the IR
                                // checks the result stays inside the lend, so
                                // an access provably past the loan is stated
                                // as written and refused by containment.
                                Some(at) => operations.push(OwnershipOp::WriteRange {
                                    place: name.clone(),
                                    axis: BYTE_AXIS.to_owned(),
                                    low: lend_low.saturating_add(at),
                                    high: lend_low.saturating_add(at).saturating_add(1),
                                }),
                                Option::None => {
                                    // Not resolvable by this tier: a rangeless
                                    // write through a range lend refuses in
                                    // the IR — fail-closed
                                    // (`WRITE-LEND-COMPUTED-INDEX`).
                                    operations.push(OwnershipOp::Write(name.clone()));
                                }
                            }
                        } else if self.acquired.contains(name) {
                            match self.constant(at) {
                                Some(at) => operations.push(OwnershipOp::WriteRange {
                                    place: name.clone(),
                                    axis: BYTE_AXIS.to_owned(),
                                    low: at,
                                    high: at.saturating_add(1),
                                }),
                                // An owner index this tier cannot resolve is
                                // not provably disjoint from any live lend, so
                                // it stays rangeless and conflicts with every
                                // one of them — the read card's own rule,
                                // reached from the write side.
                                Option::None => {
                                    operations.push(OwnershipOp::Write(name.clone()));
                                }
                            }
                        }
                    }
                    BufferEvent::Load {
                        name, index: at, ..
                    } => {
                        if let Some((_, lend_low, _)) = self.loans.get(name).cloned() {
                            match self.constant(at) {
                                Some(at) => operations.push(OwnershipOp::ReadRange {
                                    place: name.clone(),
                                    axis: BYTE_AXIS.to_owned(),
                                    low: lend_low.saturating_add(at),
                                    high: lend_low.saturating_add(at).saturating_add(1),
                                }),
                                Option::None => {
                                    operations.push(OwnershipOp::Read(name.clone()));
                                }
                            }
                        } else if self.acquired.contains(name) {
                            match self.constant(at) {
                                Some(at) => operations.push(OwnershipOp::ReadRange {
                                    place: name.clone(),
                                    axis: BYTE_AXIS.to_owned(),
                                    low: at,
                                    high: at.saturating_add(1),
                                }),
                                Option::None => {
                                    operations.push(OwnershipOp::Read(name.clone()));
                                }
                            }
                        }
                    }
                    BufferEvent::Extent { binding, name } => {
                        // An extent read touches no byte, so its footprint is
                        // the empty interval — admitted beside any loan while
                        // the binding's liveness is still checked.
                        if let Some((_, lend_low, _)) = self.loans.get(name).cloned() {
                            operations.push(OwnershipOp::ReadRange {
                                place: name.clone(),
                                axis: BYTE_AXIS.to_owned(),
                                low: lend_low,
                                high: lend_low,
                            });
                        } else if self.acquired.contains(name) {
                            operations.push(OwnershipOp::ReadRange {
                                place: name.clone(),
                                axis: BYTE_AXIS.to_owned(),
                                low: 0,
                                high: 0,
                            });
                        }
                        if let Some(extent) = self.extents.get(name).copied() {
                            self.scalars.insert(binding.clone(), extent as i32);
                        }
                    }
                    BufferEvent::Release { name } => {
                        if self.acquired.contains(name) {
                            operations.push(OwnershipOp::Drop(name.clone()));
                        }
                    }
                    // Unreachable from the route: `check_buffer_program`
                    // refuses a `for` in an entry that lends before this
                    // projection is consulted (`SLICE-PERMISSIONS-LOOP-WINDOW`).
                    // A direct model-level caller gets no operation for it,
                    // and the refusal it already received stands.
                    BufferEvent::Repeat { .. }
                    | BufferEvent::Construct { .. }
                    | BufferEvent::Field { .. }
                    | BufferEvent::Line(_)
                    | BufferEvent::Return => {}
                }
                index += 1;
            }
            operations
        }
    }

    let mut build = Build {
        acquired: &acquired,
        by_event: &by_event,
        scalars: BTreeMap::new(),
        extents: BTreeMap::new(),
        loans: BTreeMap::new(),
        next_id: 1,
    };
    let end = program.events.len();
    build.segment(&program.events, 0, end)
}

/// The state the value half of the check carries: which names are buffers, what
/// their declared extents are, what bytes they hold, and what each scalar name
/// denotes.
///
/// **Published rather than private since card `STRING-CONCAT`**, which needed
/// this route's byte semantics inside a module that also holds string views. It
/// is published as the *state plus one stepping function* rather than as three
/// maps, so a second route reaches the extent rule, the bounds rule, the
/// byte-range rule and the decidability rule by **running them** — there is one
/// implementation of each and a composed module cannot admit a byte access this
/// route would refuse, or refuse one it would admit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BufferEvaluation {
    /// Declared extent per acquired buffer name.
    pub extents: BTreeMap<String, u32>,
    /// The modelled bytes of each buffer, zero at acquisition.
    pub contents: BTreeMap<String, Vec<u8>>,
    /// The check-time-constant integer environment. A composed route shares
    /// **this** map rather than keeping a second one, so a name bound by a byte
    /// read and a name bound by a string's `len` live in one environment.
    pub scalars: BTreeMap<String, i32>,
    /// The record types the module declared, by name (card `AGGREGATES-V1`).
    ///
    /// Installed **before** the run and never written by it, because a record
    /// type is a declaration: a statement can construct a value of one and read
    /// a field of one, and neither can change what a type is.
    pub record_types: BTreeMap<String, RecordType>,
    /// The record values this entry constructed, by binding name. A record is a
    /// plain value, so this map is the whole of what holding one costs — there
    /// is no ledger entry beside it.
    pub records: BTreeMap<String, RecordValue>,
    /// The live write lends, by binding name (card
    /// `SLICE-PERMISSIONS-WRITE-LEND`). A loan never leaves this map, and that
    /// is not a leak: the loan's window runs to the last statement naming it,
    /// so by construction there is no later use for an expired entry to
    /// mislead — the same self-consistency the read card's `through` rests on.
    pub loans: BTreeMap<String, LoanShape>,
}

/// What one live write lend holds: which buffer, and which of its bytes (card
/// `SLICE-PERMISSIONS-WRITE-LEND`). The window is not here — it is a
/// whole-program fact ([`lend_windows`]) and the certificate's [`LendRecord`]
/// carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoanShape {
    pub buffer: String,
    pub low: u32,
    pub high: u32,
}

impl LoanShape {
    /// The loan's own extent: how many bytes `[low, high)` covers.
    #[must_use]
    pub fn len(&self) -> u32 {
        self.high - self.low
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.high == self.low
    }
}

/// What applying one value-bearing [`BufferEvent`] produced.
///
/// Every field is something the certificate publishes, so a composed route
/// records exactly what this route records without knowing how it was decided.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BufferStep {
    pub acquisition: Option<(String, u32)>,
    pub release: Option<String>,
    pub access: Option<BufferAccess>,
    /// The record value this event constructed (card `AGGREGATES-V1`).
    pub construction: Option<RecordConstruction>,
    /// The field this event read.
    pub projection: Option<RecordProjection>,
    /// The write lend this event opened, with its **resolved** footprint in the
    /// owner's coordinates (card `SLICE-PERMISSIONS-WRITE-LEND`). The window is
    /// the caller's to pair in, because `through` is a whole-program fact.
    pub lend: Option<(String, LoanShape)>,
    pub errors: Vec<BufferError>,
}

impl BufferEvaluation {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply one **value-bearing** event: `Bind`, `Acquire`, `Store`, `Load`,
    /// `Extent` or `Release`.
    ///
    /// [`BufferEvent::Line`] and [`BufferEvent::Return`] carry no buffer value
    /// and return [`BufferStep::default`]; a caller decides those itself,
    /// because a line's bytes are `source.std-io.v1`'s and a return is the
    /// caller's own shape rule.
    ///
    /// [`BufferEvent::Repeat`] does the same, and for a sharper reason (the
    /// `LOOP-V1` × `STRING-CONCAT` composition). A loop is not a value-bearing
    /// event: it decides **how often** its body's events run, and running them
    /// is [`ValueRun::repeat`]'s, which owns the index binding, the body-local
    /// scoping and the shadowing refusal. Deciding it here would put a second
    /// iteration rule beside that one. **A caller that cannot recurse must
    /// therefore refuse a `Repeat` rather than pass it here** — silently
    /// stepping over a loop would drop its body's writes, which is the one
    /// fail-open shape this composition could have had.
    #[must_use]
    pub fn apply(&mut self, index: usize, event: &BufferEvent) -> BufferStep {
        let mut step = BufferStep::default();
        match event {
            BufferEvent::Bind { binding, value } => {
                self.scalars.insert(binding.clone(), *value);
            }
            BufferEvent::Acquire { name, extent } => {
                if *extent == 0 || *extent > MAX_EXTENT {
                    step.errors.push(BufferError::ExtentOutOfRange {
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
                self.extents.insert(name.clone(), *extent);
                // Zero-initialised, and the emitted program uses `calloc` for
                // exactly this reason: a `malloc` would leave the bytes
                // indeterminate, so a read before a write would be undefined
                // behaviour natively and unpredictable in the reference.
                self.contents
                    .insert(name.clone(), vec![0u8; *extent as usize]);
                step.acquisition = Some((name.clone(), *extent));
            }
            BufferEvent::Store {
                name,
                index: at,
                value,
            } => {
                // Card `SLICE-PERMISSIONS-WRITE-LEND`: a store through a loan
                // is decided against the loan's **own** extent and rebased
                // into the owner's coordinates — `NAME[i]` is the owner's byte
                // `low + i`, the one arithmetic the certificate's `LendRecord`
                // publishes. The owner's exclusion from the lent range is
                // *not* decided here: it is the extended ownership IR's, so it
                // arrives through the inherited `Q054` and nothing here can
                // disagree with it.
                let place = match self.loans.get(name).cloned() {
                    Some(loan) => self
                        .lend_place(index, name, &loan, at)
                        .map(|at| (loan.buffer.clone(), loan.low as usize + at)),
                    Option::None => self
                        .place(index, name, at)
                        .map(|(_, at)| (name.clone(), at)),
                };
                let value = self.operand(index, value);
                match (place, value) {
                    (Ok((owner, at)), Ok(value)) => {
                        let Ok(byte) = u8::try_from(value) else {
                            step.errors.push(BufferError::ByteOutOfRange {
                                event: index,
                                value,
                            });
                            return step;
                        };
                        if let Some(bytes) = self.contents.get_mut(&owner) {
                            bytes[at] = byte;
                        }
                        step.access = Some(BufferAccess {
                            event: index,
                            name: name.clone(),
                            // Published in the coordinates the source wrote —
                            // the owner's for an owner store, the loan's own
                            // for a lend store, with the `LendRecord` carrying
                            // the displacement a reader adds to check them.
                            index: if self.loans.contains_key(name) {
                                (at - self.loans[name].low as usize) as u32
                            } else {
                                at as u32
                            },
                            byte,
                            write: true,
                        });
                    }
                    (place, value) => {
                        step.errors.extend(place.err());
                        step.errors.extend(value.err());
                    }
                }
            }
            BufferEvent::Load {
                binding,
                name,
                index: at,
            } => {
                let place = match self.loans.get(name).cloned() {
                    Some(loan) => self
                        .lend_place(index, name, &loan, at)
                        .map(|at| (loan.buffer.clone(), loan.low as usize + at, at as u32)),
                    Option::None => self
                        .place(index, name, at)
                        .map(|(_, at)| (name.clone(), at, at as u32)),
                };
                match place {
                    Ok((owner, at, published)) => {
                        let byte = self
                            .contents
                            .get(&owner)
                            .and_then(|bytes| bytes.get(at).copied())
                            .unwrap_or(0);
                        self.scalars.insert(binding.clone(), i32::from(byte));
                        step.access = Some(BufferAccess {
                            event: index,
                            name: name.clone(),
                            index: published,
                            byte,
                            write: false,
                        });
                    }
                    Err(error) => step.errors.push(error),
                }
            }
            BufferEvent::Extent { binding, name } => match self.extents.get(name).copied() {
                // The extent read back out of the acquisition. `i32` is
                // total here because `MAX_EXTENT` is far inside it.
                Some(extent) => {
                    self.scalars.insert(binding.clone(), extent as i32);
                }
                // Card `SLICE-PERMISSIONS-WRITE-LEND`: the extent of a loan is
                // its own length, a check-time constant exactly as the owner's
                // is — read out of the loan's footprint, which the ownership
                // ledger's own extent bounded at the lend.
                None => match self.loans.get(name) {
                    Some(loan) => {
                        self.scalars.insert(binding.clone(), loan.len() as i32);
                    }
                    Option::None => step.errors.push(BufferError::NotABuffer {
                        event: index,
                        name: name.clone(),
                    }),
                },
            },
            BufferEvent::Release { name } => {
                if self.extents.contains_key(name) {
                    step.release = Some(name.clone());
                } else if self.loans.contains_key(name) {
                    // Card `SLICE-PERMISSIONS-WRITE-LEND`: a loan is not
                    // released — its window closes at its last use, and the
                    // storage it addresses is the owner's to release.
                    step.errors.push(BufferError::Lend {
                        event: index,
                        refusal: LendRefusal::NotLendable {
                            name: name.clone(),
                            reason: "is a lend, and a lend is not released; its window closes \
                                     at its last use, and the storage it addresses is the \
                                     owner's to release"
                                .to_owned(),
                        },
                    });
                } else {
                    step.errors.push(BufferError::NotABuffer {
                        event: index,
                        name: name.clone(),
                    });
                }
            }
            // Card `SLICE-PERMISSIONS-WRITE-LEND`: open one write lend. The
            // bound rule lives here; the exclusivity rule — two live lends
            // must cover disjoint ranges — deliberately does **not**: it is
            // the extended ownership IR's own admission, inherited at `Q054`.
            BufferEvent::Lend {
                binding,
                name,
                low,
                high,
            } => {
                if self.scalars.contains_key(binding)
                    || self.extents.contains_key(binding)
                    || self.loans.contains_key(binding)
                    || self.records.contains_key(binding)
                {
                    step.errors.push(BufferError::Lend {
                        event: index,
                        refusal: LendRefusal::Shape(format!(
                            "`{binding}` is already bound in this entry; a lend binds a fresh \
                             name, and `source.buffer.v1` refuses a shadowing binding rather \
                             than deciding which of two values survives"
                        )),
                    });
                    return step;
                }
                if self.loans.contains_key(name) {
                    step.errors.push(BufferError::Lend {
                        event: index,
                        refusal: LendRefusal::NotLendable {
                            name: name.clone(),
                            reason: "is itself a lend; re-lending is not modelled in this \
                                     revision (`WRITE-LEND-RELEND`)"
                                .to_owned(),
                        },
                    });
                    return step;
                }
                let Some(extent) = self.extents.get(name).copied() else {
                    step.errors.push(BufferError::NotABuffer {
                        event: index,
                        name: name.clone(),
                    });
                    return step;
                };
                let low = self.operand(index, low);
                let high = self.operand(index, high);
                let (low, high) = match (low, high) {
                    (Ok(low), Ok(high)) => (low, high),
                    (low, high) => {
                        step.errors.extend(low.err());
                        step.errors.extend(high.err());
                        return step;
                    }
                };
                let length = i32::try_from(extent).expect("an extent is bounded by MAX_EXTENT");
                if low < 0 || high < low || high > length {
                    step.errors.push(BufferError::Lend {
                        event: index,
                        refusal: LendRefusal::Range {
                            buffer: name.clone(),
                            low,
                            high,
                            extent,
                        },
                    });
                    // Registered anyway with the bounds clamped into the
                    // extent — the refused-acquisition precedent: the errors
                    // are collected rather than fatal, and a refused lend that
                    // left its name unbound would make every later statement
                    // about it cascade into a second, misleading refusal. The
                    // program is refused either way; the custody projection
                    // clamps identically, so the two layers still agree.
                    let low = low.clamp(0, length) as u32;
                    self.loans.insert(
                        binding.clone(),
                        LoanShape {
                            buffer: name.clone(),
                            low,
                            high: high.clamp(low as i32, length) as u32,
                        },
                    );
                    return step;
                }
                let shape = LoanShape {
                    buffer: name.clone(),
                    low: low as u32,
                    high: high as u32,
                };
                self.loans.insert(binding.clone(), shape.clone());
                step.lend = Some((binding.clone(), shape));
            }
            // --- The aggregate (card `AGGREGATES-V1`). ---
            //
            // Both arms delegate every rule to `crate::source_record`: this
            // function decides *which* operand values a construction supplies
            // and *which* value a field read produces, and that module decides
            // whether the construction is total and whether the field exists.
            // There is one implementation of each rule and it is the one the
            // Lean model mirrors.
            BufferEvent::Construct {
                binding,
                type_name,
                fields,
            } => {
                let Some(declared) = self.record_types.get(type_name).cloned() else {
                    step.errors.push(BufferError::Record {
                        event: index,
                        refusal: RecordRefusal::UnknownType {
                            type_name: type_name.clone(),
                        },
                    });
                    return step;
                };
                let mut supplied = Vec::new();
                for (field, operand) in fields {
                    match self.operand(index, operand) {
                        Ok(value) => supplied.push((field.clone(), value)),
                        Err(error) => {
                            step.errors.push(error);
                            return step;
                        }
                    }
                }
                match source_record::construct(&declared, &supplied) {
                    Ok(value) => {
                        // Published in **declared** order, so two certificates
                        // of one program compare field for field however the
                        // source wrote them.
                        let published = declared
                            .fields
                            .iter()
                            .filter_map(|field| {
                                value
                                    .project(&field.name)
                                    .map(|value| (field.name.clone(), value))
                            })
                            .collect();
                        self.records.insert(binding.clone(), value);
                        step.construction = Some(RecordConstruction {
                            event: index,
                            binding: binding.clone(),
                            type_name: type_name.clone(),
                            fields: published,
                        });
                    }
                    Err(refusal) => step.errors.push(BufferError::Record {
                        event: index,
                        refusal,
                    }),
                }
            }
            BufferEvent::Field {
                binding,
                record,
                field,
            } => {
                let Some(value) = self.records.get(record) else {
                    step.errors.push(BufferError::Record {
                        event: index,
                        refusal: RecordRefusal::NotARecord {
                            name: record.clone(),
                        },
                    });
                    return step;
                };
                match value.project(field) {
                    Some(read) => {
                        step.projection = Some(RecordProjection {
                            event: index,
                            record: record.clone(),
                            type_name: value.type_name.clone(),
                            field: field.clone(),
                            value: read,
                        });
                        self.scalars.insert(binding.clone(), read);
                    }
                    None => step.errors.push(BufferError::Record {
                        event: index,
                        refusal: RecordRefusal::NoSuchField {
                            name: record.clone(),
                            type_name: value.type_name.clone(),
                            field: field.clone(),
                        },
                    }),
                }
            }
            BufferEvent::Line(_) | BufferEvent::Return | BufferEvent::Repeat { .. } => {}
        }
        step
    }

    /// Resolve `LOAN[INDEX]` to a validated index **in the loan's own
    /// coordinates** (card `SLICE-PERMISSIONS-WRITE-LEND`). The bounds rule is
    /// the loan's own `[0, len)`; the caller rebases by adding `loan.low`,
    /// which is the one arithmetic the certificate publishes.
    fn lend_place(
        &self,
        event: usize,
        name: &str,
        loan: &LoanShape,
        index: &BufferOperand,
    ) -> Result<usize, BufferError> {
        let index = self.operand(event, index)?;
        if index < 0 || u32::try_from(index).is_ok_and(|index| index >= loan.len()) {
            return Err(BufferError::Lend {
                event,
                refusal: LendRefusal::OutOfLend {
                    name: name.to_owned(),
                    index,
                    len: loan.len(),
                },
            });
        }
        Ok(index as usize)
    }

    pub fn operand(&self, event: usize, operand: &BufferOperand) -> Result<i32, BufferError> {
        match operand {
            BufferOperand::Literal(value) => Ok(*value),
            BufferOperand::Scalar(name) => {
                if self.loans.contains_key(name) {
                    // A lend at a scalar position (card
                    // `SLICE-PERMISSIONS-WRITE-LEND`), refused for the reason
                    // a buffer is and at the same code: neither is a number,
                    // and the reader needs to be told which they wrote.
                    return Err(BufferError::Lend {
                        event,
                        refusal: LendRefusal::CrossesBoundary { name: name.clone() },
                    });
                }
                if self.extents.contains_key(name) {
                    // A buffer at a scalar position. Reported as the boundary
                    // refusal rather than the generic one, because the reader
                    // needs to know *why* a name they bound is not usable here.
                    return Err(BufferError::CrossesBoundary {
                        event,
                        name: name.clone(),
                    });
                }
                if self.records.contains_key(name) {
                    // A record at a position that takes an integer. Reported as
                    // the aggregate's own refusal rather than the generic one,
                    // for the reason the buffer arm above states: the reader
                    // needs to know *why* a name they bound is not usable here,
                    // and the answer — read the field you mean first — is one
                    // this revision can actually give them.
                    return Err(BufferError::Record {
                        event,
                        refusal: RecordRefusal::RecordAtScalarPosition { name: name.clone() },
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
    pub fn place(
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

    // --- Half minus one: the module's record types, total or reject. ---
    //
    // Built before anything runs, and a module carrying one declaration
    // `admit_type` refuses gets **no table at all** rather than a partial one
    // (card `AGGREGATES-V1`). That is what makes it impossible for a later rule
    // to read a type this model did not admit — the `contract_loader`
    // total-or-reject discipline, applied to a declaration the source wrote.
    //
    // Two record types of one module cannot share a name: name resolution
    // already refuses a duplicate declaration at `Q010`, so the table below
    // cannot silently lose one to the other.
    let record_types = match source_record::admit_types(&program.records) {
        Ok(types) => types,
        Err(refusal) => {
            return Err(vec![BufferError::Record {
                // Attributed to the entry's scope: a declaration is not one of
                // this entry's events and has no event index of its own. The
                // lowering carries a scope span for exactly this case.
                event: program.events.len(),
                refusal,
            }]);
        }
    };

    // --- Half zero: the checking route's own cost, bounded before it is paid. ---
    //
    // The bounded counted loop is checked by running its body once per
    // iteration, so an entry's *unrolled* event total is the work this function
    // does. It is bounded by name rather than paid: a program above the budget
    // is refused, never checked slowly and never approximated (card `LOOP-V1`).
    let unrolled = unrolled_event_count(&program.events);
    if unrolled > MAX_UNROLLED_EVENTS {
        return Err(vec![BufferError::Loop {
            event: program.events.len(),
            refusal: LoopRefusal::Bound(BoundRefusal::Budget { unrolled }),
        }]);
    }

    // --- Half zero-and-a-half: the lend's window premises (card
    // `SLICE-PERMISSIONS-WRITE-LEND`), decided before the custody projection
    // is consulted because the projection's borrow nodes rest on them. ---
    let mut lend_window_by_event: BTreeMap<usize, usize> = BTreeMap::new();
    if any_event(&program.events, |event| {
        matches!(event, BufferEvent::Lend { .. })
    }) {
        // A lend's window is an interval of statement indices, exact because
        // the route is one flat sequence; a `for` makes one index run many
        // times and invalidates that premise — the same wall the read card
        // re-measured (`STRING-CONCAT-LOOP-WINDOW`), reached from the write
        // side (`SLICE-PERMISSIONS-LOOP-WINDOW`).
        if let Some(position) = program
            .events
            .iter()
            .position(|event| matches!(event, BufferEvent::Repeat { .. }))
        {
            return Err(vec![BufferError::Lend {
                event: position,
                refusal: LendRefusal::Shape(
                    "`source.loop.v1`'s `for` is not a statement an entry that lends models; a \
                     lend's window is an interval of statement indices and is exact only \
                     because the route is one flat sequence, so a loop body writing under a \
                     live lend would need a window over iterations rather than over indices \
                     (`SLICE-PERMISSIONS-LOOP-WINDOW`)"
                        .to_owned(),
                ),
            }]);
        }
        let (windows, tier_errors) = lend_windows(&program.events);
        if !tier_errors.is_empty() {
            return Err(tier_errors);
        }
        if let Some((first, second)) = crossed_lend_windows(&windows) {
            return Err(vec![BufferError::Lend {
                event: second.event,
                refusal: LendRefusal::Shape(format!(
                    "the windows of `{}` (statements {} through {}) and `{}` (statements {} \
                     through {}) cross; the ownership IR's borrow nodes are lexical, so two \
                     lends either nest or are disjoint, and a crossed pair is refused by name \
                     rather than approximated (`SLICE-PERMISSIONS-CROSSED-WINDOWS`)",
                    first.binding,
                    first.event,
                    first.through,
                    second.binding,
                    second.event,
                    second.through
                )),
            }]);
        }
        // An acquisition inside a lend's window would put a footprint step
        // inside a borrow node's body, where the one-pass peak walker cannot
        // see it; refused rather than under-measured
        // (`WRITE-LEND-NESTED-ACQUIRE`).
        for window in &windows {
            for (index, event) in program.events.iter().enumerate() {
                if index > window.event
                    && index <= window.through
                    && matches!(event, BufferEvent::Acquire { .. })
                {
                    return Err(vec![BufferError::Lend {
                        event: index,
                        refusal: LendRefusal::Shape(format!(
                            "this acquisition sits inside the window of `{}` (statements {} \
                             through {}); an entry that lends acquires before its first lend \
                             opens, so the published peak-live measurement stays exact \
                             (`WRITE-LEND-NESTED-ACQUIRE`)",
                            window.binding, window.event, window.through
                        )),
                    }]);
                }
            }
        }
        lend_window_by_event = windows
            .iter()
            .map(|window| (window.event, window.through))
            .collect();
    }

    // --- Half one: the inherited custody discipline. ---
    let (operations, gates) = custody_operations_with_gates(program);
    // The per-iteration identity obligation. Most of this discipline is the
    // inherited checker's — the call below sees the body's operations `count`
    // times and refuses a second acquisition of one live name — but that
    // exposure needs a bound of two or more. This gate makes the rule hold at a
    // bound of one too, so it is the rule it names rather than an artifact of
    // how many times the body happens to run.
    for gate in &gates {
        match source_loop::iteration_is_custody_identity(&gate.prefix, &gate.iteration) {
            Ok(Ok(())) => {}
            Ok(Err(detail)) => errors.push(BufferError::Loop {
                event: gate.event,
                refusal: LoopRefusal::NotIdentity {
                    variable: gate.variable.clone(),
                    detail,
                },
            }),
            // The ownership model refused the operations outright. That is the
            // inherited checker's refusal and it is reported at its own code by
            // the call below; this gate deliberately says nothing about it
            // rather than reporting the same condition twice.
            Err(_) => {}
        }
    }
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
    // as the operations run rather than counted here.
    //
    // Card `LOOP-V1` narrowed a scan of every prefix to the prefixes that can
    // *be* the maximum — the empty one and each one ending in an acquisition,
    // because the live footprint grows at an `Acquire` and nowhere else
    // (`peak_candidate_prefixes`). That is linear in the number of
    // acquisitions, but each of those prefixes was still **re-executed from the
    // start**, so a loop body acquiring once per iteration paid a quadratic
    // number of ownership steps: measured at a bound of 1024, 56.4 s. Card
    // `LOOP-UNROLL-BUDGET` replaces the rescan with `ownership::peak_live_axis`,
    // one pass taking the maximum of the values the run passes through — the
    // **same number** by definition, pinned by
    // `the_one_pass_peak_is_the_number_a_full_prefix_scan_gives`, and 0.39 s at
    // the same bound. It is what makes the absolute ceiling defensible at 4096.
    let peak_live_bytes = ownership::peak_live_axis(&operations, BYTE_AXIS);

    // --- Half two: the values. ---
    let mut run = ValueRun {
        evaluation: BufferEvaluation {
            record_types,
            ..BufferEvaluation::new()
        },
        acquisitions: Vec::new(),
        releases: Vec::new(),
        lend_windows: lend_window_by_event,
        lends: Vec::new(),
        accesses: Vec::new(),
        constructions: Vec::new(),
        projections: Vec::new(),
        lines: Vec::new(),
        flushes: 0,
        renderings: BTreeSet::new(),
        returned: false,
        errors: Vec::new(),
    };
    run.walk(&program.events, 0, true);
    let ValueRun {
        evaluation,
        acquisitions,
        releases,
        lends,
        accesses,
        constructions,
        projections,
        lines,
        flushes,
        renderings,
        returned,
        errors: value_errors,
        ..
    } = run;
    errors.extend(value_errors);

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
            lends,
            peak_live_bytes,
            accesses,
            constructions,
            projections,
            scalars: evaluation.scalars,
            lines,
            flushes,
            renderings,
        })
    } else {
        Err(errors)
    }
}

/// The value half's running state.
///
/// Carried in a struct rather than in locals because the bounded counted loop
/// makes the walk **recursive**: a `Repeat` runs its body `count` times, and
/// every one of those passes writes into the same bytes, the same scalar
/// environment and the same published line sequence as the surrounding entry
/// does.
struct ValueRun {
    evaluation: BufferEvaluation,
    acquisitions: Vec<(String, u32)>,
    releases: Vec<String>,
    /// Each lend's window end by its event index, precomputed by
    /// [`lend_windows`] — `through` is a whole-program fact the stepper cannot
    /// know (card `SLICE-PERMISSIONS-WRITE-LEND`).
    lend_windows: BTreeMap<usize, usize>,
    lends: Vec<LendRecord>,
    accesses: Vec<BufferAccess>,
    constructions: Vec<RecordConstruction>,
    projections: Vec<RecordProjection>,
    lines: Vec<ObservedLine>,
    flushes: usize,
    renderings: BTreeSet<Rendering>,
    returned: bool,
    errors: Vec<BufferError>,
}

impl ValueRun {
    /// Run `events` in order.
    ///
    /// `attributed` is the index of the **top-level** event every refusal and
    /// every published access of this walk is attributed to, and `top_level`
    /// says whether this call is the entry's own sequence. A refusal inside a
    /// loop body is therefore reported at the `for` statement: the body has no
    /// top-level index of its own, the statement is what the reader has to
    /// change, and `LOOP-BODY-ATTRIBUTION` records the finer span as a
    /// follow-on rather than guessing one.
    fn walk(&mut self, events: &[BufferEvent], attributed: usize, top_level: bool) {
        for (position, event) in events.iter().enumerate() {
            let index = if top_level { position } else { attributed };
            if self.returned {
                self.errors.push(BufferError::Shape {
                    event: index,
                    reason: "this entry continues after its return; `source.buffer.v1` models one \
                             flat sequence ending in exactly one return"
                        .to_owned(),
                });
                break;
            }
            self.event(event, index);
        }
    }

    fn event(&mut self, event: &BufferEvent, index: usize) {
        // The loop is handled before the borrow below, because running its body
        // needs the whole state rather than its fields.
        if let BufferEvent::Repeat {
            variable,
            count,
            body,
        } = event
        {
            self.repeat(variable, *count, body, index);
            return;
        }
        let Self {
            evaluation,
            acquisitions,
            releases,
            lend_windows,
            lends,
            accesses,
            constructions,
            projections,
            lines,
            flushes,
            renderings,
            returned,
            errors,
        } = self;
        match event {
            // Handled above, before the fields were borrowed.
            BufferEvent::Repeat { .. } => {}
            // **The composition of the two restructures** (cards `LOOP-V1` and
            // `STRING-CONCAT`). The recursive walk above is `LOOP-V1`'s and is
            // unchanged: a `Repeat` runs its body once per iteration through
            // `Self::repeat`, and every one of those passes arrives here. What
            // arrives here is then decided by [`BufferEvaluation::apply`] and
            // nowhere else, which is `STRING-CONCAT`'s published stepper.
            //
            // Composing them this way is what makes **both** claims survive at
            // once: the extent rule, the bounds rule, the byte-range rule and
            // the decidability rule have one implementation, and it is the one
            // a loop body's events run through as well as the one the composed
            // string/buffer route runs. Neither card's semantics is a special
            // case of the other's — the loop decides *how often* an event runs
            // and `apply` decides *what it means*.
            BufferEvent::Bind { .. }
            | BufferEvent::Acquire { .. }
            | BufferEvent::Store { .. }
            | BufferEvent::Load { .. }
            | BufferEvent::Extent { .. }
            | BufferEvent::Release { .. }
            // Card `SLICE-PERMISSIONS-WRITE-LEND`: a lend is value-bearing —
            // it binds the loan the later stores name — and it reaches the
            // same stepper, whose bound rule is the one implementation.
            | BufferEvent::Lend { .. }
            // Card `AGGREGATES-V1`: the two aggregate events are value-bearing
            // exactly as the six above are, and they reach the **same** stepper,
            // so a record constructed inside a `for` body is decided by the one
            // implementation a top-level one is.
            | BufferEvent::Construct { .. }
            | BufferEvent::Field { .. } => {
                let step = evaluation.apply(index, event);
                errors.extend(step.errors);
                acquisitions.extend(step.acquisition);
                releases.extend(step.release);
                accesses.extend(step.access);
                constructions.extend(step.construction);
                projections.extend(step.projection);
                if let Some((binding, shape)) = step.lend {
                    lends.push(LendRecord {
                        event: index,
                        buffer: shape.buffer,
                        binding,
                        low: shape.low,
                        high: shape.high,
                        through: lend_windows.get(&index).copied().unwrap_or(index),
                    });
                }
            }
            BufferEvent::Line(line) => match line {
                StdIoEvent::LogLevel { .. } => {}
                StdIoEvent::Print { rendering, operand }
                | StdIoEvent::Log {
                    rendering, operand, ..
                } => {
                    renderings.insert(*rendering);
                    match evaluation.line_operand(index, operand) {
                        Ok(value) => lines.push(
                            rendered_line(line, value)
                                .expect("a print and a log each produce exactly one line"),
                        ),
                        Err(error) => errors.push(error),
                    }
                }
                StdIoEvent::Flush => *flushes += 1,
                StdIoEvent::Return => errors.push(BufferError::Shape {
                    event: index,
                    reason: "a return is this route's own event, not a standard-IO one".to_owned(),
                }),
            },
            BufferEvent::Return => *returned = true,
        }
    }

    /// Run one `for VARIABLE in 0..COUNT { … }`.
    ///
    /// Three rules live here and nowhere else:
    ///
    /// 1. **The body is run, exactly, once per iteration**, with `VARIABLE`
    ///    bound to the iteration number. Nothing is widened and nothing is
    ///    approximated — `COUNT` is a number this route holds.
    /// 2. **The body's bindings are body-local**, the `IF-STATEMENT` precedent
    ///    for arm bindings: a name a body binds — including `VARIABLE` — is not
    ///    visible after the loop, so naming one later refuses at this route's
    ///    own `Q052`. This is what makes the emitted C's block scoping and this
    ///    model agree: a C declaration inside the loop's braces does not outlive
    ///    them either.
    /// 3. **Shadowing is refused, not resolved** (`Q097`). A body binding whose
    ///    name the surrounding entry already bound would leave the model and the
    ///    emitted C to agree by coincidence about which of two values survives
    ///    the loop, and a refusal is the fail-closed answer.
    fn repeat(&mut self, variable: &str, count: u32, body: &[BufferEvent], index: usize) {
        let outer_scalars: BTreeSet<String> = self.evaluation.scalars.keys().cloned().collect();
        let outer_buffers: BTreeSet<String> = self.evaluation.extents.keys().cloned().collect();
        // Card `AGGREGATES-V1`: a record binding is body-local exactly as a
        // scalar and a buffer binding are, and it shadows exactly as they do.
        // Listed here rather than given a rule of its own, so the loop's scoping
        // rule stays one rule over every kind of binding this route holds.
        let outer_records: BTreeSet<String> = self.evaluation.records.keys().cloned().collect();

        if outer_scalars.contains(variable)
            || outer_buffers.contains(variable)
            || outer_records.contains(variable)
        {
            self.errors.push(BufferError::Loop {
                event: index,
                refusal: LoopRefusal::Shape(format!(
                    "`{variable}` is already bound in this entry; `source.loop.v1` scopes its \
                     index to the body and refuses a shadowing binding rather than deciding \
                     which of two values survives the loop"
                )),
            });
            return;
        }
        if count == 0 || count > MAX_ITERATIONS {
            // Unreachable for a lowered program — the frontend decides the
            // bound — but the model refuses it rather than iterating on a
            // number it does not model.
            self.errors.push(BufferError::Loop {
                event: index,
                refusal: LoopRefusal::Bound(if count == 0 {
                    BoundRefusal::Zero
                } else {
                    BoundRefusal::TooLarge {
                        value: i64::from(count),
                    }
                }),
            });
            return;
        }

        // Shadowing is decided **before** the body runs, because afterwards the
        // inner binding has already overwritten the outer value and the two are
        // no longer distinguishable.
        for name in body_bindings(body) {
            if outer_scalars.contains(&name)
                || outer_buffers.contains(&name)
                || outer_records.contains(&name)
            {
                self.errors.push(BufferError::Loop {
                    event: index,
                    refusal: LoopRefusal::Shape(format!(
                        "`{name}` is bound in this loop body and already bound outside it; \
                         `source.loop.v1` scopes a body's bindings to the body and refuses a \
                         shadowing binding rather than deciding which of two values survives \
                         the loop"
                    )),
                });
                return;
            }
        }

        for iteration in 0..count {
            self.evaluation.scalars.insert(
                variable.to_owned(),
                i32::try_from(iteration).expect("MAX_ITERATIONS is far inside i32"),
            );
            let before = self.errors.len();
            self.walk(body, index, false);
            // **One refusal per loop, not one per failing iteration.** The
            // program is refused either way — this is about what the report
            // says, not about what it admits. `for i in 0..100 { b[i] = 1; }`
            // on a four-byte buffer is one mistake, and printing it 96 times
            // at the same span buries it. Every iteration before the failing
            // one still ran and was checked, so the refusal still names the
            // first iteration that is actually wrong.
            if self.errors.len() > before {
                break;
            }
        }

        // Scope the body's bindings out, including the index. A later statement
        // naming one refuses at `Q052`, which says the true thing: that name is
        // not a check-time constant *this entry* bound.
        self.evaluation
            .scalars
            .retain(|name, _| outer_scalars.contains(name));
        self.evaluation
            .extents
            .retain(|name, _| outer_buffers.contains(name));
        self.evaluation
            .contents
            .retain(|name, _| outer_buffers.contains(name));
        self.evaluation
            .records
            .retain(|name, _| outer_records.contains(name));
    }
}

/// Every name a loop body binds, at any depth — including a nested loop's own
/// index, which is a binding of that body exactly as a `let` is.
fn body_bindings(events: &[BufferEvent]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    fn walk(events: &[BufferEvent], names: &mut BTreeSet<String>) {
        for event in events {
            match event {
                BufferEvent::Bind { binding, .. }
                | BufferEvent::Load { binding, .. }
                | BufferEvent::Extent { binding, .. }
                // Card `AGGREGATES-V1`: both aggregate events bind a name, and
                // both names are body-local.
                | BufferEvent::Construct { binding, .. }
                | BufferEvent::Field { binding, .. } => {
                    names.insert(binding.clone());
                }
                BufferEvent::Acquire { name, .. } => {
                    names.insert(name.clone());
                }
                // Card `SLICE-PERMISSIONS-WRITE-LEND`: a lend binds a name,
                // and it is body-local exactly as every other binding is —
                // though an entry that lends refuses a `for` before this can
                // matter (`SLICE-PERMISSIONS-LOOP-WINDOW`).
                BufferEvent::Lend { binding, .. } => {
                    names.insert(binding.clone());
                }
                BufferEvent::Repeat { variable, body, .. } => {
                    names.insert(variable.clone());
                    walk(body, names);
                }
                BufferEvent::Store { .. }
                | BufferEvent::Release { .. }
                | BufferEvent::Line(_)
                | BufferEvent::Return => {}
            }
        }
    }
    walk(events, &mut names);
    names
}

impl BufferEvaluation {
    /// Resolve a rendering operand against this route's scalar environment.
    ///
    /// `source.std-io.v1`'s three operand forms are handled here rather than
    /// re-derived: a literal is itself, a scalar name is looked up, and a
    /// `borrow NAME` refuses — this route publishes no custody identity, so the
    /// structural refusal `STD-FMT-BORROW` established for the uncomposed route
    /// applies here for exactly the same reason.
    pub fn line_operand(&self, event: usize, operand: &StdIoOperand) -> Result<i32, BufferError> {
        match operand {
            StdIoOperand::Literal(value) => Ok(*value),
            StdIoOperand::Scalar(name) => self.operand(event, &BufferOperand::Scalar(name.clone())),
            StdIoOperand::Borrowed(name) => Err(BufferError::CrossesBoundary {
                event,
                name: name.clone(),
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BufferRecheckFailure {
    Entry,
    Effects,
    Acquisitions,
    Releases,
    Lends,
    PeakLiveBytes,
    Accesses,
    Constructions,
    Projections,
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
            Self::Lends => "the admitted write lends, their footprints and their windows",
            Self::PeakLiveBytes => "the peak live byte total",
            Self::Accesses => "the checked byte accesses",
            Self::Constructions => "the constructed record values and the fields they supply",
            Self::Projections => "the checked field reads",
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
    if rederived.lends != certificate.lends {
        return Err(BufferRecheckFailure::Lends);
    }
    if rederived.peak_live_bytes != certificate.peak_live_bytes {
        return Err(BufferRecheckFailure::PeakLiveBytes);
    }
    if rederived.accesses != certificate.accesses {
        return Err(BufferRecheckFailure::Accesses);
    }
    if rederived.constructions != certificate.constructions {
        return Err(BufferRecheckFailure::Constructions);
    }
    if rederived.projections != certificate.projections {
        return Err(BufferRecheckFailure::Projections);
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
