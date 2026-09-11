//! `source.socket.v1` — the checked model behind the `std.socket` bundled
//! module (card `QUIRIN-SOCKET-V1`).
//!
//! # What this model is, stated before anything else
//!
//! A program on this route is a flat, ordered list of statements in one entry
//! function. It may **create owned TCP and UDP sockets**, bind them to the
//! loopback interface on a kernel-chosen port, listen, connect, accept, send
//! and receive declared byte sequences, close them, and — reusing
//! `source.std-io.v1`'s statements unchanged — write rendered lines.
//!
//! Every socket is an **owned descriptor**: creation acquires, `close`
//! releases, and a leak, a double close or a use after close is refused by the
//! *unedited* [`crate::ownership`] checker, exactly as
//! [`crate::std_buffer`] inherits it for bytes. This module builds the
//! operation list and attributes the diagnostic; it decides none of those rules
//! and re-implements none of them.
//!
//! # Determinism: the peer is this program
//!
//! A socket surface is the place external nondeterminism enters a language, so
//! v1 removes the externality rather than bounding it. **Every peer of every
//! socket in an admitted program is another socket of the same program**, on
//! the loopback interface, in the same single thread. There is no remote host,
//! no name resolution, no port number in any observation, and no statement that
//! waits on an event this program did not itself cause.
//!
//! That is not a convention — it is what makes the two determinism rules below
//! *decidable*:
//!
//! 1. **A receive never blocks on bytes that were never sent.** The model
//!    carries, for every connected TCP socket, the exact byte stream that has
//!    been committed to it by an earlier `send` in this same program, and for
//!    every bound UDP socket the exact queue of datagrams committed by an
//!    earlier `sendto`. A `recv`/`recvfrom` that the committed history does not
//!    already satisfy is refused at check time by name ([`SocketError::WouldBlock`],
//!    `Q085`) rather than deadlocking at run time. The emitted program's bounded
//!    wait is therefore a **witness**, not a check — see
//!    [`crate::std_socket_c_emit`].
//! 2. **What a receive yields is model-determined.** Because the model holds
//!    the committed bytes, it can say exactly which bytes each receive takes.
//!    The certificate publishes them ([`SocketCertificate::receipts`]) and the
//!    emitted program **compares** the bytes it actually read against them,
//!    exiting non-zero on disagreement. The premise
//!    [`STREAM_BYTE_ORDER`] is checked by the run, never trusted — the
//!    discipline `STD-EVENT` established for a drained wake byte.
//!
//! # Segment boundaries are not model-determined, and are not observed
//!
//! TCP delivers a byte *sequence*, not messages: a `recv` of `n` bytes may be
//! satisfied by any number of segments. The placement of those boundaries is a
//! transport decision this model does not predict and deliberately does not
//! expose — no statement, certificate field or observation record mentions one.
//! A receive names the **count** it takes, the emitted shim reassembles to
//! exactly that count, and a short read is therefore resolved by the
//! declaration rather than absorbed. A datagram is the opposite and is modelled
//! as such: `recvfrom` takes exactly one datagram and refuses
//! ([`SocketError::PayloadOutOfRange`], `Q084`) when its declared count is not
//! that datagram's length.
//!
//! # What is refused rather than absorbed
//!
//! * **An external peer.** There is no `connect` to an address a program wrote
//!   down; `connect C L` names a *listening socket of this program*. Supplying
//!   a spelled address is the named follow-on `SOCKET-EXTERNAL-PEER`, and it is
//!   a determinism change, not a syntax change.
//! * **A receive the history does not satisfy** — `Q085`, above.
//! * **An interrupted call.** The emitted shim does not resume on `EINTR`; it
//!   fails with a distinct status. Resuming would be a silent retry loop whose
//!   iteration count is not model-determined (`SOCKET-V1-EINTR-RESUME`).
//! * **A TCP close with undrained inbound bytes.** Closing a socket that still
//!   holds unread data makes the peer's later calls fail with a reset on every
//!   platform this card targets, which is externally-timed behaviour. Refused
//!   by name (`Q085`). A **UDP** close with queued datagrams is admitted: the
//!   queue is discarded locally, no packet is emitted, and nothing observable
//!   changes. The two are decided differently because they *are* different, not
//!   for uniformity.
//! * **An unaccepted connection at listener close.** Same ground.
//!
//! # What is not claimed
//!
//! * **No addressing.** A port is chosen by the kernel (bind to port 0, read
//!   back with `getsockname`) and never reaches a source program, a certificate
//!   or an observation. This is what lets two runs of the same program, and
//!   parallel runs of the whole test suite, produce byte-identical output.
//! * **No text.** A payload is bytes. A string payload is the named follow-on
//!   `SOCKET-STRING-PAYLOAD` and belongs to whichever card owns text.
//! * **No reactor.** Registering a socket with `source.event.v1` is the named
//!   follow-on `SOCKET-EVENT-COMPOSE`. This route stands alone, as
//!   `source.buffer.v1` does.
//! * **Partial correctness only.** A primitive failure exits non-zero and
//!   produces no observation, so the claim is: *if the emitted program produces
//!   an observation at all, that observation is the reference observation.*

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use crate::os_endpoint::{
    DescriptionState, DescriptorState, Errno, Outcome, Premises, Recovery, Target, close_poststate,
};
use crate::ownership::{self, Footprint, OwnershipError, OwnershipOp, ResourceId};
use crate::std_io::{ObservedLine, Rendering, StdIoEvent, StdIoOperand, rendered_line};
use crate::typed_errors::{Effect, EffectRow};

/// The largest payload one `send` or one `sendto` may carry, in bytes.
///
/// Bounded so that a send can be shown never to block: the default loopback
/// socket send/receive buffers are at least 8 KiB on every platform this card
/// targets (Linux, macOS, Windows), and a datagram of this size is far inside
/// the loopback MTU. A larger request is refused by name rather than split —
/// splitting would introduce a boundary the model does not determine.
pub const MAX_PAYLOAD: usize = 1024;

/// The largest number of bytes one connected socket may hold committed but not
/// yet received.
///
/// This is the number the non-blocking argument for `send` actually rests on:
/// the whole in-flight total on one stream stays inside the smallest default
/// socket buffer of the three targets, so a `send` never waits for a peer to
/// drain. Exceeding it is refused ([`SocketError::WouldBlock`]) rather than
/// risking a send that blocks while this same thread is the only one that could
/// unblock it.
pub const MAX_INFLIGHT: usize = 4096;

/// The largest number of datagrams one bound UDP socket may hold undelivered.
pub const MAX_QUEUED_DATAGRAMS: usize = 8;

/// The resource-type name this route's acquisitions carry in the ownership
/// ledger. One shape, so a certificate reader can tell this route's resources
/// from any other model's.
pub const SOCKET_RESOURCE_TYPE: &str = "std.socket.Socket";

/// The ownership footprint axis a socket is counted on. A descriptor is a
/// counted kernel slot, not a quantity of bytes, so it gets its own axis rather
/// than borrowing [`crate::std_buffer::BYTE_AXIS`].
pub const DESCRIPTOR_AXIS: &str = "descriptors";

/// The named premise the emitted program **checks** rather than trusts: the
/// bytes a receive takes off a stream are the bytes an earlier send of this
/// program committed to it, in that order.
///
/// Stated as a premise and then verified, on the `STD-EVENT` model: if it is
/// false the run fails with a distinct status, never producing a differently
/// shaped observation.
pub const STREAM_BYTE_ORDER: &str = "SOCKET-V1-STREAM-BYTE-ORDER";

/// The named premise the payload bounds above are the argument for: a send of
/// at most [`MAX_PAYLOAD`] bytes, with at most [`MAX_INFLIGHT`] bytes
/// outstanding on the stream, completes without waiting for the peer to drain.
///
/// Unlike [`STREAM_BYTE_ORDER`] this one is **not** checkable by the run — a
/// program that blocked forever would produce no evidence at all. It is
/// converted into a *bounded* failure instead: every wait the emitted program
/// performs has a finite timeout and a distinct non-zero status.
pub const LOOPBACK_BUFFERED_SEND: &str = "SOCKET-V1-LOOPBACK-BUFFERED-SEND";

/// Which transport a socket speaks. The two are never interchangeable: no
/// statement of this route accepts either kind where the other is meant, and
/// the refusal is [`SocketError::Protocol`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SocketKind {
    /// `tcp` — a connection-oriented byte stream (`SOCK_STREAM`).
    Tcp,
    /// `udp` — a connectionless datagram socket (`SOCK_DGRAM`).
    Udp,
}

impl SocketKind {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }
}

/// What a payload byte, a count or a rendered value may be.
///
/// **This enum is half the effect discipline** (the other half is that a socket
/// is nameable only at a custody position). Neither constructor can denote a
/// socket, so a payload byte can never be one. Both forms are check-time
/// constants, exactly as [`crate::std_buffer::BufferOperand`]'s are and for the
/// same reason: this route's evidence is a byte comparison between a reference
/// execution and a native run, and a value the reference cannot compute is a
/// value it cannot predict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketOperand {
    /// An integer literal written at the site.
    Literal(i32),
    /// A name this entry bound to a check-time-constant integer — by a literal
    /// `let`, or by a one-byte receive.
    Scalar(String),
}

/// A payload that is **a string view's own bytes**, read at the transfer site
/// (card `HTTP-QUIRIN-V1`, landing the socket card's named follow-on
/// `SOCKET-STRING-PAYLOAD`).
///
/// # Why this is structural rather than a second operand form
///
/// A [`SocketOperand`] denotes one byte. A view denotes an **extent** — a
/// `{ptr, len}` pair of `source.string.v1`'s own model — and the whole point of
/// the string amendment is that the length is carried rather than recovered by
/// scanning. So the payload is the extent, read at the site, and the bytes below
/// are the view's exact decoded UTF-8 payload: no terminator, no re-encoding, no
/// escaping. That is amendment §7's observation rule applied to a wire instead
/// of to a stream, and it is why a `0` byte inside a payload is an ordinary byte
/// here exactly as it is in a rendered line.
///
/// **No custody crosses.** A string is a custody-free view of storage it does
/// not own (§3.6: "creating a view neither creates nor discharges that
/// custody"), so a text payload adds no ownership operation, no acquisition and
/// no release. The socket keeps every custody rule it had; the view contributes
/// none of its own because it has none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextPayload {
    /// The file-scope symbol a **composing** route defined for these bytes, if
    /// any.
    ///
    /// `None` is the bare `source.socket.v1` form — a quoted literal written at
    /// the transfer site, emitted as an inline array in the statement's own
    /// block. `Some(symbol)` is what a route that owns a fragment contributes:
    /// the bytes live once at file scope under that name and every transfer of
    /// them names it, which is what lets one HTTP message be sent by one end and
    /// verified by the other against **the same object**.
    pub symbol: Option<String>,
    /// The view's exact bytes.
    pub bytes: Vec<u8>,
}

impl TextPayload {
    /// A payload emitted inline at its transfer site.
    #[must_use]
    pub fn inline(bytes: Vec<u8>) -> Self {
        Self {
            symbol: None,
            bytes,
        }
    }

    /// A payload whose bytes a composing route published at file scope.
    #[must_use]
    pub fn named(symbol: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            symbol: Some(symbol.into()),
            bytes,
        }
    }
}

/// What one `send` or `sendto` puts on the wire.
///
/// Both arms are decided **before emission**, which is the invariant this route's
/// whole evidence story rests on: the checker can predict what each receive takes
/// only because nothing here is computed at run time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketPayload {
    /// `send NAME [BYTE, …];` — a bracketed list of check-time byte constants.
    Bytes(Vec<SocketOperand>),
    /// `send NAME "…";` — a string view's own bytes (card `HTTP-QUIRIN-V1`).
    Text(TextPayload),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketStatement {
    /// `let NAME = INTEGER;` — an ordinary integer binding.
    Let { binding: String, value: i32 },
    /// `let NAME = tcp;` / `let NAME = udp;` — create an owned socket. The
    /// custody obligation opens here.
    Create { name: String, kind: SocketKind },
    /// `bind NAME;` — bind to the loopback interface on a kernel-chosen port.
    ///
    /// The port is deliberately **not** spellable. A fixed port would make two
    /// concurrent runs of the same program collide, which is a correctness
    /// requirement for a test suite that runs in parallel and not a nicety.
    Bind { name: String },
    /// `listen NAME;` — accept-queue depth one, which is all a program whose
    /// every connection is accepted before the next needs.
    Listen { name: String },
    /// `connect NAME NAME;` — connect the first socket to the second, which
    /// must be a **listening socket of this same program**.
    Connect { client: String, listener: String },
    /// `let NAME = accept NAME;` — take the pending connection off a listener,
    /// acquiring a second owned socket.
    Accept { binding: String, listener: String },
    /// `send NAME [OPERAND, …];` or `send NAME "…";` — commit declared bytes to
    /// a connected TCP socket's peer.
    Send {
        name: String,
        payload: SocketPayload,
    },
    /// `recv NAME COUNT;`, `let NAME = recv NAME;` with an implied count of one,
    /// or `recv NAME "…";` — take exactly `count` bytes off the socket's
    /// committed stream.
    ///
    /// `expect` is the **declared** payload of the third form: the bytes the
    /// reader says arrive. The model already knows which bytes the receive takes
    /// (they are the bytes an earlier send of this same program committed), so
    /// the declaration is compared against that answer **at check time** and a
    /// disagreement is a refusal, never a run-time surprise. `count` is that
    /// declaration's own length.
    Recv {
        binding: Option<String>,
        name: String,
        count: u32,
        expect: Option<TextPayload>,
    },
    /// `sendto NAME NAME [OPERAND, …];` — one datagram from the first socket to
    /// the second, which must be a **bound UDP socket of this same program**.
    SendTo {
        name: String,
        target: String,
        payload: SocketPayload,
    },
    /// `recvfrom NAME COUNT;`, `let NAME = recvfrom NAME;` with an implied count
    /// of one, or `recvfrom NAME "…";`. Takes exactly one datagram, whose length
    /// must be `count`.
    RecvFrom {
        binding: Option<String>,
        name: String,
        count: u32,
        expect: Option<TextPayload>,
    },
    /// `close NAME;` — release. Deliberately the **existing** reserved word,
    /// which `std.event` and the checked mutex-guard profile already use at a
    /// statement head, rather than a new one meaning the same thing.
    Close { name: String },
    /// A `source.std-io.v1` line-producing statement, reused unchanged. The
    /// bytes of the line are decided by [`rendered_line`] and nowhere else.
    Line(StdIoEvent),
    /// The entry's return. Exactly one, last.
    Return,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketProgram {
    pub entry: String,
    /// The entry's declared `effects { … }` row, verbatim from source.
    pub declared_effects: EffectRow,
    pub statements: Vec<SocketStatement>,
}

/// One committed transfer, published so a certificate names every byte this
/// program put on the wire and where it went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketTransfer {
    pub event: usize,
    /// The socket the bytes left.
    pub from: String,
    /// The socket they were committed to.
    pub to: String,
    pub bytes: Vec<u8>,
    /// Whether this transfer is one datagram rather than a stream append.
    pub datagram: bool,
}

/// One receive, with the exact bytes the model says it takes.
///
/// **This is the payload transport, published**: the emitted program compares
/// what it actually read against these bytes, so [`STREAM_BYTE_ORDER`] produces
/// a failed run if it is false.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketReceipt {
    pub event: usize,
    pub name: String,
    /// The socket of this same program the bytes came from.
    ///
    /// For a datagram this is load-bearing rather than informational: the
    /// emitted program reads the source address `recvfrom` reports and compares
    /// it with this socket's own bound address, so "the datagram came from where
    /// the model says" is **checked** and not assumed. A UDP socket is not
    /// connected, so without that comparison a stray datagram delivered to the
    /// same ephemeral port would be read as this one.
    pub from: String,
    pub bytes: Vec<u8>,
    pub datagram: bool,
}

/// What the OS-endpoint close relation answers for one `close` of this program,
/// on both of its branches.
///
/// Derived by [`crate::os_endpoint::close_poststate`] and not restated here.
/// The failure branch is the load-bearing half: with no target pinned and
/// `errno` unobserved the relation answers [`DescriptorState::Attempted`] — a
/// **poisoned** state carrying the conservative residual duty set — and permits
/// **no** recovery, which is exactly why the emitted shim does not retry a
/// failed close and exits instead.
///
/// `Attempted` rather than [`DescriptorState::Indeterminate`] is the honest
/// answer, and it names its own repair: `Attempted` means *this* analysis cannot
/// map the outcome, so reading `errno` and pinning the target would move some
/// arms to [`DescriptorState::Released`]. That is the named follow-on
/// `SOCKET-V1-CLOSE-ERRNO`, and it is a real improvement rather than a gap —
/// nothing on this route depends on which failure occurred, because every
/// failure exits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketCloseDisposition {
    pub name: String,
    /// The descriptor state on the `ret == 0` branch.
    pub success_descriptor: DescriptorState,
    /// The open-description state on the same branch.
    pub success_description: DescriptionState,
    /// The descriptor state on the `ret == -1`, `errno` unobserved branch.
    pub failure_descriptor: DescriptorState,
    /// Whether the relation permits a retried `close` on the failure branch.
    /// False, on every target, which the emitted shim honours.
    pub failure_retry_permitted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketCertificate {
    pub entry: String,
    /// The derived row, recomputed by the recheck and compared.
    pub effects: EffectRow,
    /// `(name, kind)` for every socket this entry acquired, in program order.
    pub acquisitions: Vec<(String, SocketKind)>,
    /// Every release, in program order.
    pub closes: Vec<String>,
    /// The maximum number of live descriptors the program ever holds, taken
    /// from the ownership state's own `live()` footprint after each prefix.
    pub peak_live_descriptors: u64,
    /// Every committed transfer, in program order.
    pub transfers: Vec<SocketTransfer>,
    /// Every receive and the bytes it takes, in program order.
    pub receipts: Vec<SocketReceipt>,
    /// What the OS-endpoint close relation answers for each close.
    pub close_disposition: Vec<SocketCloseDisposition>,
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
pub enum SocketError {
    /// `Q080`: the flat entry shape this profile models is violated.
    Shape { event: usize, reason: String },
    /// `Q081`: an operand that is not a check-time constant this entry bound.
    NotAScalar { event: usize, name: String },
    /// `Q081`: a custody position naming something this entry created no socket
    /// for.
    NotASocket { event: usize, name: String },
    /// `Q082`: an operation at a socket state that does not admit it — the
    /// transport state machine's own refusal.
    Protocol { event: usize, reason: String },
    /// `Q083`: a refusal reported by the **inherited, unedited** ownership
    /// checker — a leak, a double close, a use after close, an unbound name, or
    /// a rebound live name. This variant carries that checker's own error value
    /// rather than a re-worded copy of it.
    Custody { event: usize, error: OwnershipError },
    /// `Q084`: a payload byte outside `0..=255`.
    ByteOutOfRange { event: usize, value: i32 },
    /// `Q084`: a payload length outside `1..=MAX_PAYLOAD`, or a datagram receive
    /// whose declared count is not the queued datagram's length.
    PayloadOutOfRange {
        event: usize,
        length: usize,
        expected: Option<usize>,
    },
    /// `Q084`: a receive that **declares** the bytes it takes, where the bytes
    /// this program's own committed history delivers are different ones (card
    /// `HTTP-QUIRIN-V1`).
    ///
    /// The same code as the length disagreement above, on purpose: both are the
    /// one condition "the payload written down is not the payload", and §L21
    /// allocates a code to a condition rather than to a field. A length that
    /// disagrees and a content that disagrees are the same fault seen at two
    /// granularities, and separating them would claim a distinction this route
    /// does not make.
    PayloadDisagrees {
        event: usize,
        name: String,
        expected: Vec<u8>,
        actual: Vec<u8>,
    },
    /// `Q085`: **the determinism boundary.** A statement whose completion is not
    /// determined by this program's own committed history.
    WouldBlock { event: usize, reason: String },
    /// `Q086`: a socket at a position this route refuses to let it cross —
    /// today, a payload byte or a rendering operand.
    CrossesBoundary { event: usize, name: String },
    /// `Q087`: the declared `effects { … }` row does not admit a derived effect.
    ///
    /// Deliberately **this pass's own code and not `StandardBuffer`'s `Q057`**:
    /// §L21's allocation rule is that a code may not be reused by a second
    /// semantic pass, and this is a second pass.
    EffectNotDeclared { event: usize, effect: Effect },
}

impl SocketError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Shape { .. } => "Q080",
            Self::NotAScalar { .. } | Self::NotASocket { .. } => "Q081",
            Self::Protocol { .. } => "Q082",
            Self::Custody { .. } => "Q083",
            Self::ByteOutOfRange { .. }
            | Self::PayloadOutOfRange { .. }
            | Self::PayloadDisagrees { .. } => "Q084",
            Self::WouldBlock { .. } => "Q085",
            Self::CrossesBoundary { .. } => "Q086",
            Self::EffectNotDeclared { .. } => "Q087",
        }
    }

    #[must_use]
    pub fn event(&self) -> usize {
        match self {
            Self::Shape { event, .. }
            | Self::NotAScalar { event, .. }
            | Self::NotASocket { event, .. }
            | Self::Protocol { event, .. }
            | Self::Custody { event, .. }
            | Self::ByteOutOfRange { event, .. }
            | Self::PayloadOutOfRange { event, .. }
            | Self::PayloadDisagrees { event, .. }
            | Self::WouldBlock { event, .. }
            | Self::CrossesBoundary { event, .. }
            | Self::EffectNotDeclared { event, .. } => *event,
        }
    }
}

impl fmt::Display for SocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape { reason, .. } | Self::Protocol { reason, .. } => f.write_str(reason),
            Self::NotAScalar { name, .. } => write!(
                f,
                "`{name}` is not a check-time-constant integer this entry bound; \
                 `source.socket.v1` builds a payload from an integer literal or a name bound to \
                 one, so that every byte on the wire is decided before emission"
            ),
            Self::NotASocket { name, .. } => write!(
                f,
                "`{name}` is not a socket this entry created; `source.socket.v1` names a socket \
                 only at a custody position, and every such position must name one"
            ),
            Self::Custody { error, .. } => write!(
                f,
                "the checked ownership discipline refuses this program: {error}"
            ),
            Self::ByteOutOfRange { value, .. } => write!(
                f,
                "{value} is not a byte; `source.socket.v1` sends 0 through 255 and refuses \
                 anything else rather than truncating it"
            ),
            Self::PayloadOutOfRange {
                length,
                expected: None,
                ..
            } => write!(
                f,
                "{length} is not a payload length `source.socket.v1` models; one send carries 1 \
                 to {MAX_PAYLOAD} bytes, bounded so that it provably never waits for the peer to \
                 drain"
            ),
            Self::PayloadOutOfRange {
                length,
                expected: Some(expected),
                ..
            } => write!(
                f,
                "this receive declares {length} bytes but the datagram at the head of the queue \
                 is {expected}; a datagram is taken whole, so `source.socket.v1` refuses rather \
                 than truncating one"
            ),
            Self::PayloadDisagrees {
                name,
                expected,
                actual,
                ..
            } => write!(
                f,
                "this receive declares {} byte(s) of payload from `{name}`, but the bytes an \
                 earlier send of this program committed are different ones; \
                 `source.socket.v1` decides a declared payload at check time, so the two must \
                 agree here rather than at run time (declared {}, committed {})",
                expected.len(),
                summarize_bytes(expected),
                summarize_bytes(actual)
            ),
            Self::WouldBlock { reason, .. } => f.write_str(reason),
            Self::CrossesBoundary { name, .. } => write!(
                f,
                "`{name}` is an owned socket and cannot cross here; a payload byte and a rendering \
                 operand are scalars, and `source.socket.v1` publishes no custody identity for a \
                 borrowed render"
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
/// A creation, an accept and a close are `Alloc` — they open and close custody
/// of a kernel resource. Every transport call is `Io`. A `let` derives nothing.
///
/// `Mut` is deliberately **not** derived: this route mutates no object of the
/// program. The bytes a send commits leave the program entirely, and the bytes a
/// receive takes arrive as new values.
#[must_use]
pub fn derived_effects(program: &SocketProgram) -> EffectRow {
    let mut effects = BTreeSet::new();
    for statement in &program.statements {
        match statement {
            SocketStatement::Create { .. } | SocketStatement::Close { .. } => {
                effects.insert(Effect::Alloc);
            }
            SocketStatement::Accept { .. } => {
                effects.insert(Effect::Alloc);
                effects.insert(Effect::Io);
            }
            SocketStatement::Bind { .. }
            | SocketStatement::Listen { .. }
            | SocketStatement::Connect { .. }
            | SocketStatement::Send { .. }
            | SocketStatement::Recv { .. }
            | SocketStatement::SendTo { .. }
            | SocketStatement::RecvFrom { .. } => {
                effects.insert(Effect::Io);
            }
            SocketStatement::Line(
                StdIoEvent::LogLevel { .. }
                | StdIoEvent::Print { .. }
                | StdIoEvent::Log { .. }
                | StdIoEvent::Flush,
            ) => {
                effects.insert(Effect::Io);
            }
            SocketStatement::Let { .. }
            | SocketStatement::Line(StdIoEvent::Return)
            | SocketStatement::Return => {}
        }
    }
    EffectRow::new(effects)
}

/// The **custody half** of the check, and the whole of what this route inherits.
///
/// Builds the cycle-3 ownership operation list from the statement sequence and
/// hands it to [`ownership::check_ownership`] unchanged. Every leak, double
/// close, use after close, unbound name and rebound live name is decided there —
/// this function decides none of them.
///
/// Published separately from [`check_socket_program`] so a test can show the
/// refusal arriving as an [`OwnershipError`] from the inherited checker rather
/// than as a re-worded copy, which is the evidence that it *is* inherited.
///
/// A footprint of one descriptor per socket is what makes
/// [`SocketCertificate::peak_live_descriptors`] a number read out of the
/// ownership ledger rather than counted here.
#[must_use]
pub fn custody_operations(program: &SocketProgram) -> Vec<OwnershipOp> {
    let owned: BTreeSet<&str> = program
        .statements
        .iter()
        .filter_map(|statement| match statement {
            SocketStatement::Create { name, .. } => Some(name.as_str()),
            SocketStatement::Accept { binding, .. } => Some(binding.as_str()),
            _ => None,
        })
        .collect();
    let names = |name: &String| owned.contains(name.as_str());

    let mut operations = Vec::new();
    let mut next_id: ResourceId = 1;
    let mut acquire = |operations: &mut Vec<OwnershipOp>, name: &str| {
        operations.push(OwnershipOp::Acquire {
            id: next_id,
            owner: name.to_owned(),
            resource_type: SOCKET_RESOURCE_TYPE.to_owned(),
            footprint: Footprint::single(DESCRIPTOR_AXIS, 1),
        });
        next_id += 1;
    };

    for statement in &program.statements {
        match statement {
            SocketStatement::Create { name, .. } => acquire(&mut operations, name),
            SocketStatement::Accept { binding, listener } => {
                if names(listener) {
                    operations.push(OwnershipOp::Read(listener.clone()));
                }
                acquire(&mut operations, binding);
            }
            // A bind, a listen and a connect change the socket's own state, so
            // they are writes; a send and a sendto change no object of this
            // program and a receive observes one, so both are reads. The
            // distinction is the ownership IR's own, used for what it means.
            SocketStatement::Bind { name } | SocketStatement::Listen { name } if names(name) => {
                operations.push(OwnershipOp::Write(name.clone()));
            }
            SocketStatement::Connect { client, listener } => {
                if names(client) {
                    operations.push(OwnershipOp::Write(client.clone()));
                }
                if names(listener) {
                    operations.push(OwnershipOp::Write(listener.clone()));
                }
            }
            SocketStatement::Send { name, .. } | SocketStatement::Recv { name, .. }
                if names(name) =>
            {
                operations.push(OwnershipOp::Read(name.clone()));
            }
            SocketStatement::SendTo { name, target, .. } => {
                if names(name) {
                    operations.push(OwnershipOp::Read(name.clone()));
                }
                if names(target) {
                    operations.push(OwnershipOp::Read(target.clone()));
                }
            }
            SocketStatement::RecvFrom { name, .. } if names(name) => {
                operations.push(OwnershipOp::Read(name.clone()));
            }
            SocketStatement::Close { name } if names(name) => {
                operations.push(OwnershipOp::Drop(name.clone()));
            }
            SocketStatement::Bind { .. }
            | SocketStatement::Listen { .. }
            | SocketStatement::Send { .. }
            | SocketStatement::Recv { .. }
            | SocketStatement::RecvFrom { .. }
            | SocketStatement::Close { .. } => {}
            SocketStatement::Let { .. } | SocketStatement::Line(_) | SocketStatement::Return => {}
        }
    }
    operations
}

/// What the OS-endpoint close relation answers for one close of this route.
///
/// Every premise is justified separately and none is assumed:
///
/// * `epoch_live` — the inherited ownership checker refused the program
///   otherwise, so a close that reaches here names a live socket.
/// * `sole_custodian` — the route is single-threaded, spawns nothing, forks
///   nothing and duplicates no descriptor.
/// * `result_observed` — the emitted shim compares the result against zero.
/// * `errno_observed` — **false**. The shim does not read `errno`, because no
///   arm of this route's behaviour depends on which failure occurred: every
///   failure exits.
/// * `last_reference` — nothing in this route can create a second reference to
///   an open description.
/// * `no_record_locks` — a socket cannot carry a POSIX record lock.
///
/// The target is [`Target::Unpinned`] deliberately: the emitted program runs on
/// three platforms and pins none of them, so the relation is read at the
/// conservative join. That, together with the unobserved `errno`, is what makes
/// the failure branch poisoned with no permitted recovery, and so what forbids a
/// retry.
#[must_use]
pub fn close_disposition(name: &str) -> SocketCloseDisposition {
    let premises = Premises {
        errno_observed: false,
        ..Premises::default()
    };
    let success = close_poststate(Target::Unpinned, premises, Outcome::Zero)
        .expect("the success outcome always describes a live epoch");
    let failure = close_poststate(
        Target::Unpinned,
        premises,
        Outcome::MinusOne(Errno::Unobserved),
    )
    .expect("the failure outcome always describes a live epoch");
    SocketCloseDisposition {
        name: name.to_owned(),
        success_descriptor: success.descriptor,
        success_description: success.description,
        failure_descriptor: failure.descriptor,
        failure_retry_permitted: failure.permitted_recovery.contains(&Recovery::Close),
    }
}

/// What the model knows about one socket.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SocketState {
    kind: SocketKind,
    bound: bool,
    listening: bool,
    /// The socket this one is connected to, once a connection exists on both
    /// sides. A connector that has not yet been accepted has `None` here and a
    /// pending entry on the listener.
    peer: Option<String>,
    closed: bool,
    /// Bytes committed to this socket and not yet received (TCP).
    inbound: VecDeque<u8>,
    /// Datagrams committed to this socket and not yet received (UDP), each with
    /// the socket of this same program that sent it.
    datagrams: VecDeque<(String, Vec<u8>)>,
}

impl SocketState {
    fn new(kind: SocketKind) -> Self {
        Self {
            kind,
            bound: false,
            listening: false,
            peer: None,
            closed: false,
            inbound: VecDeque::new(),
            datagrams: VecDeque::new(),
        }
    }
}

struct Evaluation {
    sockets: BTreeMap<String, SocketState>,
    /// Per listener, the connectors waiting to be accepted, in arrival order.
    pending: BTreeMap<String, VecDeque<String>>,
    scalars: BTreeMap<String, i32>,
}

impl Evaluation {
    fn operand(&self, event: usize, operand: &SocketOperand) -> Result<i32, SocketError> {
        match operand {
            SocketOperand::Literal(value) => Ok(*value),
            SocketOperand::Scalar(name) => {
                if self.sockets.contains_key(name) {
                    return Err(SocketError::CrossesBoundary {
                        event,
                        name: name.clone(),
                    });
                }
                self.scalars
                    .get(name)
                    .copied()
                    .ok_or_else(|| SocketError::NotAScalar {
                        event,
                        name: name.clone(),
                    })
            }
        }
    }

    /// The bytes one transfer commits, from either payload form.
    ///
    /// The **length rule is one rule**: a payload carries 1 to [`MAX_PAYLOAD`]
    /// bytes whether it was written as a bracketed byte list or as a string
    /// view, because the bound exists for the transport reason stated at
    /// [`LOOPBACK_BUFFERED_SEND`] and a view does not change what the kernel
    /// buffers.
    fn transfer_bytes(
        &self,
        event: usize,
        payload: &SocketPayload,
    ) -> Result<Vec<u8>, Vec<SocketError>> {
        match payload {
            SocketPayload::Bytes(operands) => self.payload(event, operands),
            SocketPayload::Text(text) => {
                if text.bytes.is_empty() || text.bytes.len() > MAX_PAYLOAD {
                    return Err(vec![SocketError::PayloadOutOfRange {
                        event,
                        length: text.bytes.len(),
                        expected: None,
                    }]);
                }
                Ok(text.bytes.clone())
            }
        }
    }

    fn payload(
        &self,
        event: usize,
        operands: &[SocketOperand],
    ) -> Result<Vec<u8>, Vec<SocketError>> {
        if operands.is_empty() || operands.len() > MAX_PAYLOAD {
            return Err(vec![SocketError::PayloadOutOfRange {
                event,
                length: operands.len(),
                expected: None,
            }]);
        }
        let mut bytes = Vec::with_capacity(operands.len());
        let mut errors = Vec::new();
        for operand in operands {
            match self.operand(event, operand) {
                Ok(value) => match u8::try_from(value) {
                    Ok(byte) => bytes.push(byte),
                    Err(_) => errors.push(SocketError::ByteOutOfRange { event, value }),
                },
                Err(error) => errors.push(error),
            }
        }
        if errors.is_empty() {
            Ok(bytes)
        } else {
            Err(errors)
        }
    }

    fn open(&self, event: usize, name: &str) -> Result<&SocketState, SocketError> {
        let Some(state) = self.sockets.get(name) else {
            return Err(SocketError::NotASocket {
                event,
                name: name.to_owned(),
            });
        };
        if state.closed {
            // The ownership checker also refuses this, and its refusal is the
            // one the report shows. This arm exists so the value half can stop
            // rather than continue against a state it knows is gone.
            return Err(SocketError::Protocol {
                event,
                reason: format!("`{name}` was already closed"),
            });
        }
        Ok(state)
    }
}

/// A byte sequence, rendered for a diagnostic: printable ASCII verbatim, `\r`,
/// `\n` and `\t` by name, everything else as `\xHH`, and a long payload
/// truncated with its full length stated.
///
/// Bounded so a refusal naming a 1024-byte payload stays readable. The length is
/// always printed, so the truncation never hides how much there was.
fn summarize_bytes(bytes: &[u8]) -> String {
    const SHOWN: usize = 64;
    let mut text = String::with_capacity(SHOWN + 16);
    text.push('"');
    for byte in bytes.iter().take(SHOWN) {
        match byte {
            b'\r' => text.push_str("\\r"),
            b'\n' => text.push_str("\\n"),
            b'\t' => text.push_str("\\t"),
            b'"' => text.push_str("\\\""),
            b'\\' => text.push_str("\\\\"),
            0x20..=0x7e => text.push(*byte as char),
            other => text.push_str(&format!("\\x{other:02x}")),
        }
    }
    text.push('"');
    if bytes.len() > SHOWN {
        text.push('…');
    }
    format!("{text} ({} bytes)", bytes.len())
}

fn protocol(event: usize, reason: impl Into<String>) -> SocketError {
    SocketError::Protocol {
        event,
        reason: reason.into(),
    }
}

fn would_block(event: usize, reason: impl Into<String>) -> SocketError {
    SocketError::WouldBlock {
        event,
        reason: reason.into(),
    }
}

/// Check a lowered program and derive its certificate.
///
/// Two halves, in this order and deliberately:
///
/// 1. **Custody**, decided entirely by [`ownership::check_ownership`] over
///    [`custody_operations`]. Running it first is what makes the second half
///    able to assume a name it reads is live.
/// 2. **The transport state machine and the committed history** — the part this
///    card actually owns, and where both determinism rules live.
///
/// # Errors
///
/// Every refusal the program earns, in program order.
pub fn check_socket_program(
    program: &SocketProgram,
) -> Result<SocketCertificate, Vec<SocketError>> {
    let mut errors = Vec::new();

    // --- Half one: the inherited custody discipline. ---
    let operations = custody_operations(program);
    if let Err(error) = ownership::check_ownership(&operations) {
        errors.push(SocketError::Custody {
            event: program.statements.len(),
            error,
        });
    }
    let mut peak_live_descriptors = 0u64;
    for length in 0..=operations.len() {
        if let Ok(state) = ownership::execute_ownership(&operations[..length]) {
            peak_live_descriptors = peak_live_descriptors.max(state.live().get(DESCRIPTOR_AXIS));
        }
    }

    // --- Half two: the transport. ---
    let mut evaluation = Evaluation {
        sockets: BTreeMap::new(),
        pending: BTreeMap::new(),
        scalars: BTreeMap::new(),
    };
    let mut acquisitions = Vec::new();
    let mut closes = Vec::new();
    let mut transfers = Vec::new();
    let mut receipts = Vec::new();
    let mut close_dispositions = Vec::new();
    let mut lines = Vec::new();
    let mut flushes = 0usize;
    let mut renderings = BTreeSet::new();
    let mut returned = false;

    for (index, statement) in program.statements.iter().enumerate() {
        if returned {
            errors.push(SocketError::Shape {
                event: index,
                reason: "this entry continues after its return; `source.socket.v1` models one \
                         flat sequence ending in exactly one return"
                    .to_owned(),
            });
            break;
        }
        match statement {
            SocketStatement::Let { binding, value } => {
                evaluation.scalars.insert(binding.clone(), *value);
            }
            SocketStatement::Create { name, kind } => {
                evaluation
                    .sockets
                    .insert(name.clone(), SocketState::new(*kind));
                acquisitions.push((name.clone(), *kind));
            }
            SocketStatement::Bind { name } => match evaluation.open(index, name) {
                Ok(state) => {
                    if state.bound {
                        errors.push(protocol(index, format!("`{name}` is already bound")));
                    } else if state.peer.is_some() {
                        errors.push(protocol(
                            index,
                            format!("`{name}` is already connected and cannot be bound"),
                        ));
                    } else {
                        evaluation
                            .sockets
                            .get_mut(name)
                            .expect("resolved above")
                            .bound = true;
                    }
                }
                Err(error) => errors.push(error),
            },
            SocketStatement::Listen { name } => match evaluation.open(index, name) {
                Ok(state) => {
                    if state.kind != SocketKind::Tcp {
                        errors.push(protocol(
                            index,
                            format!(
                                "`{name}` is a udp socket; `listen` is a connection-oriented \
                                 operation and `source.socket.v1` models no datagram listener"
                            ),
                        ));
                    } else if !state.bound {
                        errors.push(protocol(
                            index,
                            format!("`{name}` must be bound before it can listen"),
                        ));
                    } else if state.listening {
                        errors.push(protocol(index, format!("`{name}` is already listening")));
                    } else {
                        evaluation
                            .sockets
                            .get_mut(name)
                            .expect("resolved above")
                            .listening = true;
                    }
                }
                Err(error) => errors.push(error),
            },
            SocketStatement::Connect { client, listener } => {
                let client_state = evaluation.open(index, client);
                let listener_state = evaluation.open(index, listener);
                match (client_state, listener_state) {
                    (Ok(client_state), Ok(listener_state)) => {
                        let queued = evaluation
                            .pending
                            .get(listener)
                            .map_or(0, std::collections::VecDeque::len);
                        if client_state.kind != SocketKind::Tcp
                            || listener_state.kind != SocketKind::Tcp
                        {
                            errors.push(protocol(
                                index,
                                "`connect` joins two tcp sockets; `source.socket.v1` models no \
                                 connected datagram socket (`SOCKET-UDP-CONNECT`)"
                                    .to_owned(),
                            ));
                        } else if client == listener {
                            errors.push(protocol(
                                index,
                                "a socket cannot connect to itself".to_owned(),
                            ));
                        } else if !listener_state.listening {
                            errors.push(protocol(
                                index,
                                format!(
                                    "`{listener}` is not listening; `source.socket.v1` connects \
                                     only to a listening socket of this same program, which is \
                                     what makes the connection's completion decidable here"
                                ),
                            ));
                        } else if client_state.peer.is_some() || client_state.listening {
                            errors.push(protocol(
                                index,
                                format!("`{client}` is not a fresh socket and cannot connect"),
                            ));
                        } else if queued >= 1 {
                            // Accept-queue depth one. A second unaccepted
                            // connect is refused rather than modelled, because
                            // whether a backlogged connect completes before its
                            // accept is a kernel decision this model does not
                            // make.
                            errors.push(would_block(
                                index,
                                format!(
                                    "`{listener}` already holds a connection no `accept` has \
                                     taken; `source.socket.v1` models an accept queue of depth \
                                     one, so whether a second connect completes before its accept \
                                     is a decision this model does not make \
                                     (`SOCKET-BACKLOG-DEPTH`)"
                                ),
                            ));
                        } else {
                            evaluation
                                .pending
                                .entry(listener.clone())
                                .or_default()
                                .push_back(client.clone());
                        }
                    }
                    (client_state, listener_state) => {
                        errors.extend(client_state.err());
                        errors.extend(listener_state.err());
                    }
                }
            }
            SocketStatement::Accept { binding, listener } => {
                match evaluation.open(index, listener) {
                    Ok(state) => {
                        if state.kind != SocketKind::Tcp || !state.listening {
                            errors.push(protocol(
                                index,
                                format!("`{listener}` is not a listening tcp socket"),
                            ));
                        } else {
                            match evaluation
                                .pending
                                .get_mut(listener)
                                .and_then(std::collections::VecDeque::pop_front)
                            {
                                Some(client) => {
                                    let mut accepted = SocketState::new(SocketKind::Tcp);
                                    accepted.peer = Some(client.clone());
                                    evaluation.sockets.insert(binding.clone(), accepted);
                                    if let Some(client_state) = evaluation.sockets.get_mut(&client)
                                    {
                                        client_state.peer = Some(binding.clone());
                                    }
                                    acquisitions.push((binding.clone(), SocketKind::Tcp));
                                }
                                None => {
                                    errors.push(would_block(
                                        index,
                                        format!(
                                            "no connection is waiting on `{listener}`; an accept \
                                             with nothing queued would block on an event this \
                                             program never causes, so `source.socket.v1` refuses \
                                             it here rather than deadlocking at run time"
                                        ),
                                    ));
                                    // The binding still has to exist, or every
                                    // later statement about it cascades into a
                                    // second, misleading refusal. The program is
                                    // refused either way.
                                    evaluation
                                        .sockets
                                        .insert(binding.clone(), SocketState::new(SocketKind::Tcp));
                                    acquisitions.push((binding.clone(), SocketKind::Tcp));
                                }
                            }
                        }
                    }
                    Err(error) => errors.push(error),
                }
            }
            SocketStatement::Send { name, payload } => {
                let bytes = match evaluation.transfer_bytes(index, payload) {
                    Ok(bytes) => bytes,
                    Err(mut found) => {
                        errors.append(&mut found);
                        continue;
                    }
                };
                match evaluation.open(index, name) {
                    Ok(state) => {
                        if state.kind != SocketKind::Tcp {
                            errors.push(protocol(
                                index,
                                format!("`{name}` is a udp socket; use `sendto`"),
                            ));
                            continue;
                        }
                        let Some(peer) = state.peer.clone() else {
                            errors.push(protocol(
                                index,
                                format!(
                                    "`{name}` is not connected; a connector becomes usable only \
                                     once its listener has accepted it"
                                ),
                            ));
                            continue;
                        };
                        let Some(peer_state) = evaluation.sockets.get_mut(&peer) else {
                            errors.push(SocketError::NotASocket {
                                event: index,
                                name: peer,
                            });
                            continue;
                        };
                        if peer_state.closed {
                            errors.push(protocol(
                                index,
                                format!(
                                    "the peer of `{name}` is closed; this send has no receiver"
                                ),
                            ));
                            continue;
                        }
                        if peer_state.inbound.len() + bytes.len() > MAX_INFLIGHT {
                            errors.push(would_block(
                                index,
                                format!(
                                    "this send would leave more than {MAX_INFLIGHT} bytes \
                                     committed and unreceived on one stream; beyond that bound \
                                     the send may wait for a peer only this same thread could \
                                     drain, so `source.socket.v1` refuses it \
                                     (`{LOOPBACK_BUFFERED_SEND}`)"
                                ),
                            ));
                            continue;
                        }
                        peer_state.inbound.extend(bytes.iter().copied());
                        transfers.push(SocketTransfer {
                            event: index,
                            from: name.clone(),
                            to: peer,
                            bytes,
                            datagram: false,
                        });
                    }
                    Err(error) => errors.push(error),
                }
            }
            SocketStatement::Recv {
                binding,
                name,
                count,
                expect,
            } => {
                let count = *count as usize;
                match evaluation.open(index, name) {
                    Ok(state) => {
                        if state.kind != SocketKind::Tcp {
                            errors.push(protocol(
                                index,
                                format!("`{name}` is a udp socket; use `recvfrom`"),
                            ));
                            continue;
                        }
                        if state.peer.is_none() {
                            errors.push(protocol(index, format!("`{name}` is not connected")));
                            continue;
                        }
                        if count == 0 || count > MAX_PAYLOAD {
                            errors.push(SocketError::PayloadOutOfRange {
                                event: index,
                                length: count,
                                expected: None,
                            });
                            continue;
                        }
                        if state.inbound.len() < count {
                            errors.push(would_block(
                                index,
                                format!(
                                    "this receive takes {count} bytes from `{name}`, but only {} \
                                     have been committed to it by an earlier send of this \
                                     program; `source.socket.v1` refuses a receive its own \
                                     history does not satisfy rather than blocking on bytes that \
                                     may never arrive",
                                    state.inbound.len()
                                ),
                            ));
                            continue;
                        }
                        // A **declared** payload is decided here, against the
                        // committed history, and not at run time. The bytes are
                        // compared before they are drained, so a refused receive
                        // leaves the model's history exactly as it found it. The
                        // emitted program still compares what it actually read
                        // against the certificate's receipt, so the declaration
                        // is checked twice by two different means and neither is
                        // trusted.
                        if let Some(expect) = expect {
                            let taking: Vec<u8> =
                                state.inbound.iter().take(count).copied().collect();
                            if expect.bytes != taking {
                                errors.push(SocketError::PayloadDisagrees {
                                    event: index,
                                    name: name.clone(),
                                    expected: expect.bytes.clone(),
                                    actual: taking,
                                });
                                continue;
                            }
                        }
                        let peer = state.peer.clone().expect("checked just above");
                        let state = evaluation.sockets.get_mut(name).expect("resolved above");
                        let bytes: Vec<u8> = state.inbound.drain(..count).collect();
                        if let Some(binding) = binding {
                            evaluation
                                .scalars
                                .insert(binding.clone(), i32::from(bytes[0]));
                        }
                        receipts.push(SocketReceipt {
                            event: index,
                            name: name.clone(),
                            from: peer,
                            bytes,
                            datagram: false,
                        });
                    }
                    Err(error) => errors.push(error),
                }
            }
            SocketStatement::SendTo {
                name,
                target,
                payload,
            } => {
                let bytes = match evaluation.transfer_bytes(index, payload) {
                    Ok(bytes) => bytes,
                    Err(mut found) => {
                        errors.append(&mut found);
                        continue;
                    }
                };
                let sender = evaluation.open(index, name);
                let receiver = evaluation.open(index, target);
                match (sender, receiver) {
                    (Ok(sender), Ok(receiver)) => {
                        if sender.kind != SocketKind::Udp || receiver.kind != SocketKind::Udp {
                            errors.push(protocol(
                                index,
                                "`sendto` carries one datagram between two udp sockets".to_owned(),
                            ));
                            continue;
                        }
                        if !receiver.bound {
                            errors.push(protocol(
                                index,
                                format!(
                                    "`{target}` is not bound and so has no address to receive at"
                                ),
                            ));
                            continue;
                        }
                        if receiver.datagrams.len() >= MAX_QUEUED_DATAGRAMS {
                            errors.push(would_block(
                                index,
                                format!(
                                    "`{target}` already holds {MAX_QUEUED_DATAGRAMS} undelivered \
                                     datagrams; beyond that bound a datagram may be dropped, \
                                     which `source.socket.v1` refuses rather than modelling as a \
                                     loss it cannot predict"
                                ),
                            ));
                            continue;
                        }
                        let receiver = evaluation.sockets.get_mut(target).expect("resolved above");
                        receiver.datagrams.push_back((name.clone(), bytes.clone()));
                        transfers.push(SocketTransfer {
                            event: index,
                            from: name.clone(),
                            to: target.clone(),
                            bytes,
                            datagram: true,
                        });
                    }
                    (sender, receiver) => {
                        errors.extend(sender.err());
                        errors.extend(receiver.err());
                    }
                }
            }
            SocketStatement::RecvFrom {
                binding,
                name,
                count,
                expect,
            } => {
                let count = *count as usize;
                match evaluation.open(index, name) {
                    Ok(state) => {
                        if state.kind != SocketKind::Udp {
                            errors.push(protocol(
                                index,
                                format!("`{name}` is a tcp socket; use `recv`"),
                            ));
                            continue;
                        }
                        if !state.bound {
                            errors.push(protocol(
                                index,
                                format!("`{name}` is not bound and so receives nothing"),
                            ));
                            continue;
                        }
                        let Some(head) = state.datagrams.front() else {
                            errors.push(would_block(
                                index,
                                format!(
                                    "no datagram has been committed to `{name}` by an earlier \
                                     `sendto` of this program; `source.socket.v1` refuses a \
                                     receive its own history does not satisfy rather than \
                                     blocking on a datagram that may never arrive"
                                ),
                            ));
                            continue;
                        };
                        if head.1.len() != count {
                            errors.push(SocketError::PayloadOutOfRange {
                                event: index,
                                length: count,
                                expected: Some(head.1.len()),
                            });
                            continue;
                        }
                        if let Some(expect) = expect
                            && expect.bytes != head.1
                        {
                            errors.push(SocketError::PayloadDisagrees {
                                event: index,
                                name: name.clone(),
                                expected: expect.bytes.clone(),
                                actual: head.1.clone(),
                            });
                            continue;
                        }
                        let state = evaluation.sockets.get_mut(name).expect("resolved above");
                        let (from, bytes) = state
                            .datagrams
                            .pop_front()
                            .expect("the head was inspected above");
                        if let Some(binding) = binding {
                            evaluation
                                .scalars
                                .insert(binding.clone(), i32::from(bytes[0]));
                        }
                        receipts.push(SocketReceipt {
                            event: index,
                            name: name.clone(),
                            from,
                            bytes,
                            datagram: true,
                        });
                    }
                    Err(error) => errors.push(error),
                }
            }
            SocketStatement::Close { name } => match evaluation.open(index, name) {
                Ok(state) => {
                    // A stream close with unread inbound bytes sends a reset on
                    // every platform this card targets, and a reset makes the
                    // peer's later calls fail at a moment nothing here decides.
                    // A datagram close discards its queue locally and emits
                    // nothing, so it is admitted — the two are decided
                    // differently because they are different.
                    if state.kind == SocketKind::Tcp && !state.inbound.is_empty() {
                        errors.push(would_block(
                            index,
                            format!(
                                "`{name}` still holds {} committed byte(s) no receive took; \
                                 closing a stream socket with unread data resets the connection, \
                                 which would make the peer's later calls fail at a moment this \
                                 program does not decide",
                                state.inbound.len()
                            ),
                        ));
                    }
                    if evaluation
                        .pending
                        .get(name)
                        .is_some_and(|queue| !queue.is_empty())
                    {
                        errors.push(would_block(
                            index,
                            format!(
                                "`{name}` still holds a connection no `accept` took; closing a \
                                 listener with a queued connection resets it, which the connector \
                                 observes at a moment this program does not decide"
                            ),
                        ));
                    }
                    let state = evaluation.sockets.get_mut(name).expect("resolved above");
                    state.closed = true;
                    closes.push(name.clone());
                    close_dispositions.push(close_disposition(name));
                }
                Err(error) => errors.push(error),
            },
            SocketStatement::Line(line) => match line {
                StdIoEvent::LogLevel { .. } => {}
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
                StdIoEvent::Return => errors.push(SocketError::Shape {
                    event: index,
                    reason: "a return is this route's own statement, not a standard-IO one"
                        .to_owned(),
                }),
            },
            SocketStatement::Return => returned = true,
        }
    }

    if !returned {
        errors.push(SocketError::Shape {
            event: program.statements.len(),
            reason: "this entry never returns; `source.socket.v1` requires exactly one return"
                .to_owned(),
        });
    }

    let derived = derived_effects(program);
    for effect in derived.iter() {
        if !program.declared_effects.contains(effect) {
            errors.push(SocketError::EffectNotDeclared {
                event: program.statements.len(),
                effect: effect.clone(),
            });
        }
    }

    if errors.is_empty() {
        Ok(SocketCertificate {
            entry: program.entry.clone(),
            effects: derived,
            acquisitions,
            closes,
            peak_live_descriptors,
            transfers,
            receipts,
            close_disposition: close_dispositions,
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
/// `borrow NAME` refuses — this route publishes no custody identity a render
/// could name, so the structural refusal `STD-FMT-BORROW` established for the
/// uncomposed route applies here for exactly the same reason.
fn line_operand(
    evaluation: &Evaluation,
    event: usize,
    operand: &StdIoOperand,
) -> Result<i32, SocketError> {
    match operand {
        StdIoOperand::Literal(value) => Ok(*value),
        StdIoOperand::Scalar(name) => {
            evaluation.operand(event, &SocketOperand::Scalar(name.clone()))
        }
        StdIoOperand::Borrowed(name) => Err(SocketError::CrossesBoundary {
            event,
            name: name.clone(),
        }),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketRecheckFailure {
    Entry,
    Effects,
    Acquisitions,
    Closes,
    PeakLiveDescriptors,
    Transfers,
    Receipts,
    CloseDisposition,
    Scalars,
    Lines,
    Flushes,
    Renderings,
    Refused,
}

impl fmt::Display for SocketRecheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Self::Entry => "the entry name",
            Self::Effects => "the derived effect row",
            Self::Acquisitions => "the sockets this entry acquired",
            Self::Closes => "the closes",
            Self::PeakLiveDescriptors => "the peak live descriptor total",
            Self::Transfers => "the committed transfers",
            Self::Receipts => "the received byte sequences",
            Self::CloseDisposition => "the close relation's answers",
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
///
/// # Errors
///
/// [`SocketRecheckFailure`] naming the first field that does not reproduce.
pub fn recheck_socket_certificate(
    program: &SocketProgram,
    certificate: &SocketCertificate,
) -> Result<(), SocketRecheckFailure> {
    let rederived = check_socket_program(program).map_err(|_| SocketRecheckFailure::Refused)?;
    if rederived.entry != certificate.entry {
        return Err(SocketRecheckFailure::Entry);
    }
    if rederived.effects != certificate.effects {
        return Err(SocketRecheckFailure::Effects);
    }
    if rederived.acquisitions != certificate.acquisitions {
        return Err(SocketRecheckFailure::Acquisitions);
    }
    if rederived.closes != certificate.closes {
        return Err(SocketRecheckFailure::Closes);
    }
    if rederived.peak_live_descriptors != certificate.peak_live_descriptors {
        return Err(SocketRecheckFailure::PeakLiveDescriptors);
    }
    if rederived.transfers != certificate.transfers {
        return Err(SocketRecheckFailure::Transfers);
    }
    if rederived.receipts != certificate.receipts {
        return Err(SocketRecheckFailure::Receipts);
    }
    if rederived.close_disposition != certificate.close_disposition {
        return Err(SocketRecheckFailure::CloseDisposition);
    }
    if rederived.scalars != certificate.scalars {
        return Err(SocketRecheckFailure::Scalars);
    }
    if rederived.lines != certificate.lines {
        return Err(SocketRecheckFailure::Lines);
    }
    if rederived.flushes != certificate.flushes {
        return Err(SocketRecheckFailure::Flushes);
    }
    if rederived.renderings != certificate.renderings {
        return Err(SocketRecheckFailure::Renderings);
    }
    Ok(())
}
