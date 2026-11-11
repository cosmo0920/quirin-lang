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

//! `source.http.v1` — the checked model behind the `std.http` bundled module
//! (card `HTTP-QUIRIN-V1`).
//!
//! # What this model is, stated before anything else
//!
//! A program on this route is a flat, ordered list of statements in one entry
//! function. It may **declare HTTP/1.1 messages** — a request line or a status
//! line, header fields, and a fixed literal content — **construct their exact
//! wire bytes at check time**, and move those bytes over `source.socket.v1`'s
//! own loopback transport, which it reuses unedited.
//!
//! This is **an executed HTTP/1.1 subset with the boundary named**, and never
//! "HTTP/1.1 conformance". §5 says exactly what is in and what is out.
//!
//! # The signature move: a conformance fact a compiler can decide
//!
//! Every component of a message is **check-time-known** — a quoted token, or
//! (card `HTTP-V1-COMPUTED-LENGTH`) a value this entry composed out of
//! check-time-known pieces — so its bytes, and therefore its *length*, are held
//! by the compiler before emission. That makes a whole family of RFC 9112 facts
//! **decidable statically**, and this module decides them rather than trusting
//! the author:
//!
//! * a declared `Content-Length` must equal the content's byte count, and a
//!   message with content must declare one ([`HttpError::Framing`]). **The
//!   equality is unchanged by a computed length**: both sides are resolved to
//!   bytes before [`admit_framing`] runs, so that gate never learns which
//!   spelling produced them;
//! * the version must be exactly `HTTP/1.1` ([`HTTP_VERSION`]);
//! * a request must carry exactly one `Host` field (RFC 9112 §3.2);
//! * a method, a field name and a request target must be the grammar's own
//!   productions — a token, a token, an origin-form path;
//! * **no component may contain `CR` or `LF`**, which is what makes the framing
//!   below unforgeable rather than merely intended.
//!
//! # Framing is constructed, not spelled
//!
//! No source program writes a `CRLF`. [`head_bytes`] writes them — one after the
//! start line, one after each field, one to end the head — and because every
//! component was refused if it carried `CR` or `LF`, **every `CRLF` in a
//! message's head is a framing `CRLF` this module put there**. That is the
//! property `QCore.StdHttp.a_component_carries_no_framing_byte` states and the
//! reason header injection is not expressible on this route.
//!
//! # Determinism, inherited whole
//!
//! This route creates no thread, opens no reactor and adds no waiting statement.
//! Its transport is `source.socket.v1`'s, so every peer of every socket is
//! another socket of the same program, every receive is proved satisfiable by
//! the program's own committed history, and the observation is the parent
//! thread's alone (standing constraint 15c; see [`crate::std_http_c_emit`]).
//!
//! # What the HTTP vocabulary does *not* do
//!
//! * **It derives no effect.** Declaring a message, adding a field and setting
//!   content allocate nothing, mutate nothing and write nothing: they compute a
//!   byte sequence at check time. Every effect an admitted program declares is
//!   the transport's `Io`/`Alloc` or the output module's `Io`
//!   (`no_http_statement_derives_an_effect`).
//! * **It holds no custody.** A message is a value, not a resource; the sockets
//!   it travels over keep every custody rule the unedited ownership checker
//!   already gave them.
//! * **It parses nothing.** A receive names the message it expects and the bytes
//!   are compared; reading a message out of unknown input is the named follow-on
//!   `HTTP-V1-PARSE` and needs a varying-value tier this language does not have.
//! * **It computes nothing at run time.** A `concat` result is a byte sequence
//!   *the compiler* holds, exactly as `source.string.v1`'s is (constraint 15h):
//!   no allocation, no copy, no custody, and a length that is itself a
//!   check-time constant. A composition whose pieces the compiler cannot
//!   evaluate is the sibling route's named follow-on
//!   `STRING-SLICE-CONCAT-RUNTIME`, and is what would make the framing equality
//!   undecidable here.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::std_io::{Rendering, StdIoEvent, StdIoOperand};
use crate::std_socket::{
    self, SocketCertificate, SocketError, SocketProgram, SocketStatement, TextPayload,
};
use crate::std_string::MAX_STRING_BYTES;
use crate::typed_errors::{Effect, EffectRow};

/// The only HTTP version `source.http.v1` admits, spelled exactly as RFC 9112
/// §2.3 requires it on the wire.
///
/// It is **written in the source and checked**, not supplied by the compiler:
/// a reader of an admitted program can see which protocol it speaks, and a
/// program naming `HTTP/1.0` or `HTTP/2` is refused by name rather than silently
/// upgraded to the one version this route models.
pub const HTTP_VERSION: &str = "HTTP/1.1";

/// The two bytes that end every line of a message head (RFC 9112 §2.1).
pub const CRLF: &[u8] = b"\r\n";

/// The field name whose value this route checks against the content's own byte
/// length. Compared case-insensitively, as RFC 9110 §5.1 requires of every field
/// name.
pub const CONTENT_LENGTH: &str = "Content-Length";

/// The field name a request must carry exactly one of (RFC 9112 §3.2).
pub const HOST: &str = "Host";

/// The transfer-coding field v1 refuses outright: no transfer coding is
/// modelled, so a message that declares one would frame its content by a rule
/// this route does not implement.
pub const TRANSFER_ENCODING: &str = "Transfer-Encoding";

/// The connection-option field, admitted only with the value `close`.
pub const CONNECTION: &str = "Connection";

/// The one connection option v1's transport actually honours: the emitted
/// program closes after one exchange.
pub const CONNECTION_CLOSE: &str = "close";

/// The registered method names whose lower-case spelling is refused.
///
/// RFC 9110 §9.1: "The method token is case-sensitive". A lower-case `get` is
/// therefore a *different* method from `GET`, and no origin server implements
/// it. v1 refuses the eight common spellings in the wrong case and names the
/// repair, rather than emitting a request no server will answer
/// (`HTTP-V1-METHOD-CASE`).
pub const CASE_SENSITIVE_METHODS: [&str; 8] = [
    "GET", "HEAD", "POST", "PUT", "DELETE", "CONNECT", "OPTIONS", "TRACE",
];

/// The prefix of the file-scope symbol a message's wire bytes are published
/// under in emitted C.
///
/// It cannot collide with the socket backend's `sk_` and `s_` names, which is
/// what lets one message be sent by one end of a connection and verified by the
/// other **against the same object** rather than against a second copy.
pub const MESSAGE_SYMBOL_PREFIX: &str = "qcore_http_";

/// The file-scope symbol a message's wire bytes are published under.
#[must_use]
pub fn message_symbol(binding: &str) -> String {
    format!("{MESSAGE_SYMBOL_PREFIX}{binding}")
}

/// A message's start line: RFC 9112 §3 request-line or §4 status-line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HttpStart {
    /// `request METHOD TARGET VERSION` — `method SP request-target SP
    /// HTTP-version`.
    Request {
        method: String,
        target: String,
        version: String,
    },
    /// `response VERSION STATUS REASON` — `HTTP-version SP status-code SP
    /// [reason-phrase]`.
    Response {
        version: String,
        status: i32,
        reason: String,
    },
}

impl HttpStart {
    /// Which of the two kinds this is, for a certificate reader and for the
    /// refusals that apply to one and not the other.
    #[must_use]
    pub fn is_request(&self) -> bool {
        matches!(self, Self::Request { .. })
    }

    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Request { .. } => "request",
            Self::Response { .. } => "response",
        }
    }

    /// The version literal this start line declares.
    #[must_use]
    pub fn version(&self) -> &str {
        match self {
            Self::Request { version, .. } | Self::Response { version, .. } => version,
        }
    }
}

/// One header field, exactly as it will appear on the wire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpField {
    pub name: String,
    pub value: String,
}

/// One declared message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpMessage {
    pub binding: String,
    pub start: HttpStart,
    pub fields: Vec<HttpField>,
    /// The literal content, if a `content` statement set one. `None` and
    /// `Some(vec![])` are **different**: the first is a message with no content
    /// and the second is a message whose content is zero bytes, which must
    /// declare `Content-Length: 0`.
    pub content: Option<Vec<u8>>,
}

/// A message as the certificate publishes it: its identity, its exact wire bytes
/// and the two numbers the framing rule relates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpMessageRecord {
    pub binding: String,
    /// `"request"` or `"response"`.
    pub kind: &'static str,
    /// The file-scope symbol the emitted program publishes these bytes under.
    pub symbol: String,
    /// The head: start line, fields, and the empty line that ends it.
    pub head: Vec<u8>,
    /// The content, if any.
    pub content: Option<Vec<u8>>,
    /// The `Content-Length` this message declares, if it declares one. For an
    /// admitted message this **equals** `content.map_or(0, |c| c.len())`, which
    /// is the equality this card exists to decide.
    pub declared_length: Option<u64>,
    /// The number of header fields, so a reader can check the framing count
    /// without recounting the bytes.
    pub field_count: usize,
    /// Whether some statement of this entry put these bytes on a wire.
    ///
    /// A message that is declared and never transferred is **checked exactly as
    /// hard** — its conformance is decided at the end of the program rather than
    /// at a transfer — and is deliberately **not** emitted: a file-scope array no
    /// statement names is an unused object, and the emitted C is built under
    /// `-Wall -Wextra -Werror`.
    pub transferred: bool,
    /// Whether this message's `Content-Length` was written as a value the entry
    /// **computed** rather than as a quoted token
    /// (card `HTTP-V1-COMPUTED-LENGTH`).
    ///
    /// It changes no decision: `declared_length` still equals the content's byte
    /// count for every admitted message, because both are check-time-known
    /// whichever spelling produced them. It is published so a reader of a
    /// certificate can see *that* a length was derived, which is the property
    /// this card exists to make expressible.
    pub declared_length_computed: bool,
    /// The same for the content itself.
    pub content_computed: bool,
}

impl HttpMessageRecord {
    /// The whole message: head then content (RFC 9112 §2.1).
    #[must_use]
    pub fn wire(&self) -> Vec<u8> {
        let mut bytes = self.head.clone();
        if let Some(content) = &self.content {
            bytes.extend_from_slice(content);
        }
        bytes
    }
}

/// What may stand at a **header-value** or **content** position
/// (card `HTTP-V1-COMPUTED-LENGTH`).
///
/// Every form is **check-time-known**, which is the entire reason a computed
/// component changes nothing about what the conformance gates decide. Card
/// `HTTP-QUIRIN-V1` admitted only the first; the other two are `STRING-CONCAT`'s
/// own vocabulary, resolved here into the same byte sequence a quoted token
/// would have produced, *before* [`admit_field`] and [`admit_framing`] run. The
/// gates are therefore unchanged — they see bytes, and they never learn which
/// spelling produced them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HttpPiece {
    /// A quoted token written at the position — the only form before card
    /// `HTTP-V1-COMPUTED-LENGTH`, and the one this route's own syntax owns
    /// (constraint 15f: it constructs no `string` value).
    Literal(Vec<u8>),
    /// A name this entry bound to text, with `let NAME = "…";` or
    /// `let NAME = concat(PIECE, …);`.
    Name(String),
    /// `dec(NAME)` / `hex(NAME)` — the **text of a check-time-constant
    /// integer**, byte-identical to what `print dec NAME;` writes, because it is
    /// produced by [`Rendering::render`] and by nothing else. This is the piece
    /// that makes a **computed `Content-Length`** expressible.
    Scalar { rendering: Rendering, name: String },
}

impl HttpPiece {
    /// The name this piece reads, if it reads one.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Literal(_) => None,
            Self::Name(name) | Self::Scalar { name, .. } => Some(name.as_str()),
        }
    }

    /// Whether this piece is a value the entry computed rather than a token it
    /// wrote. Published per message so a reader of a certificate can see that a
    /// declared length was derived rather than spelled.
    #[must_use]
    pub fn is_computed(&self) -> bool {
        !matches!(self, Self::Literal(_))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HttpStatement {
    /// `let NAME = request "GET" "/" "HTTP/1.1";` /
    /// `let NAME = response "HTTP/1.1" 200 "OK";`
    Declare { binding: String, start: HttpStart },
    /// `header NAME "Field" VALUE;` — the field name is a quoted token (a
    /// `token` is this route's own syntax) and the **value** is any
    /// [`HttpPiece`].
    Header {
        message: String,
        name: String,
        value: HttpPiece,
    },
    /// `content NAME VALUE;` — the message's content, a quoted token or a value
    /// this entry computed.
    Content { message: String, value: HttpPiece },
    /// `let NAME = concat(PIECE, …);` — **text this entry computes rather than
    /// writes** (card `HTTP-V1-COMPUTED-LENGTH`, reusing `STRING-CONCAT`'s
    /// operation unchanged in meaning).
    ///
    /// Two or more pieces: a composition of one *is* that piece, and admitting
    /// it would give the route two spellings of one value with nothing to tell
    /// them apart by — exactly `source.string.v1`'s own rule.
    Compose {
        binding: String,
        pieces: Vec<HttpPiece>,
    },
    /// `let NAME = len(NAME);` — the exact byte count of a bound text value,
    /// available without scanning because the bytes are the compiler's own.
    ///
    /// It binds a **check-time scalar**. It lowers to the transport's own
    /// `let` binding only where the transport reads the name, which is what
    /// keeps a length used solely as a header value out of the emitted C
    /// (`-Werror=unused-variable`).
    Length { binding: String, name: String },
    /// `let NAME = "…";` — a string view of statically stored literal bytes,
    /// usable as a transfer payload. `source.string.v1`'s own construction,
    /// modelled here so a payload can be named rather than only written at the
    /// site.
    Text { binding: String, bytes: Vec<u8> },
    /// `send NAME VALUE;` / `recv NAME VALUE;` where `VALUE` names a message or
    /// a text view this entry bound.
    ///
    /// Resolved to `source.socket.v1`'s own statement once the model knows what
    /// the name denotes, which is why it is one variant and not two.
    Transfer {
        socket: String,
        value: String,
        receive: bool,
    },
    /// A `source.socket.v1` statement, reused unchanged: creation, bind, listen,
    /// connect, accept, a byte-list or written-literal transfer, close, a
    /// `source.std-io.v1` line, or the return.
    Transport(SocketStatement),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpProgram {
    pub entry: String,
    /// The entry's declared `effects { … }` row, verbatim from source.
    pub declared_effects: EffectRow,
    pub statements: Vec<HttpStatement>,
}

/// One admitted `concat`, published so a reader sees the **length arithmetic**
/// rather than being told it happened (card `HTTP-V1-COMPUTED-LENGTH`).
///
/// `bytes.len() == piece_lens.iter().sum()` holds for every record, and that
/// equation is the operational content of "a computed `Content-Length` is still
/// decided": a length the compiler derived from pieces is a check-time constant
/// exactly as a literal's own length is. The shape is `STRING-CONCAT`'s
/// [`crate::std_string::ConcatRecord`], carried here with the composed bytes
/// themselves so the evidence is the text and not a claim about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpComposition {
    pub event: usize,
    pub binding: String,
    /// The byte length each piece contributed, in written order.
    pub piece_lens: Vec<u32>,
    /// The composed payload.
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpCertificate {
    pub entry: String,
    /// The derived row. Every effect in it is the transport's or the output
    /// module's; the HTTP vocabulary derives none.
    pub effects: EffectRow,
    /// Every message this entry declared, in program order, with its wire bytes.
    pub messages: Vec<HttpMessageRecord>,
    /// Every string view this entry bound, in program order — written literals
    /// and composed payloads alike, each with the exact bytes it denotes.
    pub views: Vec<(String, Vec<u8>)>,
    /// Every admitted `concat`, with its length arithmetic
    /// (card `HTTP-V1-COMPUTED-LENGTH`).
    pub compositions: Vec<HttpComposition>,
    /// The check-time scalar environment: every integer this entry bound, by
    /// name, whether with `let NAME = INTEGER;` or with `let NAME = len(NAME);`.
    pub scalars: BTreeMap<String, i32>,
    /// The transport's own certificate, derived by the **unedited**
    /// `source.socket.v1` checker over the program this route lowered to.
    pub transport: SocketCertificate,
}

/// Why one component of a message is not the production RFC 9112 names there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentRefusal {
    /// Not a `token` (RFC 9110 §5.6.2): empty, or carrying a byte outside
    /// `tchar`.
    NotAToken {
        what: &'static str,
        spelling: String,
    },
    /// A registered method spelled in the wrong case.
    MethodCase {
        spelling: String,
        registered: String,
    },
    /// Not an origin-form request target (RFC 9112 §3.2.1).
    NotAnOriginForm { spelling: String },
    /// Not the one admitted version.
    Version { spelling: String },
    /// A status code outside `100..=599`.
    Status { value: i32 },
    /// A byte that may not appear in a reason phrase or a field value.
    NotFieldContent {
        what: &'static str,
        at: usize,
        byte: u8,
    },
    /// A field value with leading or trailing whitespace. RFC 9110 §5.5 strips
    /// it on receipt, so a spelled one is not part of the value; v1 refuses
    /// rather than silently dropping bytes the author wrote.
    SurroundingWhitespace { spelling: String },
}

impl fmt::Display for ComponentRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAToken { what, spelling } => write!(
                f,
                "`{spelling}` is not a {what}; RFC 9110 §5.6.2 spells one as a non-empty `token`, \
                 and `source.http.v1` refuses anything else rather than putting bytes on the wire \
                 whose framing the grammar does not cover"
            ),
            Self::MethodCase {
                spelling,
                registered,
            } => write!(
                f,
                "`{spelling}` is not the method `{registered}`; RFC 9110 §9.1 makes the method \
                 token case-sensitive, so this request would ask for a method no origin server \
                 implements — write `{registered}`"
            ),
            Self::NotAnOriginForm { spelling } => write!(
                f,
                "`{spelling}` is not an origin-form request target; RFC 9112 §3.2.1 spells one as \
                 an absolute path beginning with `/`, optionally followed by `?query`, with no \
                 space and no fragment"
            ),
            Self::Version { spelling } => write!(
                f,
                "`{spelling}` is not `{HTTP_VERSION}`; `source.http.v1` models exactly one \
                 protocol version and refuses another rather than emitting a message whose \
                 framing rules it did not check"
            ),
            Self::Status { value } => write!(
                f,
                "{value} is not a status code; RFC 9112 §4 spells one as three digits, and every \
                 registered class lies in 100..=599"
            ),
            Self::NotFieldContent { what, at, byte } => write!(
                f,
                "byte {at} of this {what} is 0x{byte:02x}, which may not appear there. A `CR` or \
                 `LF` would forge the message framing this route constructs, and v1 also refuses \
                 the other control bytes and every non-ASCII byte (RFC 9110 §5.5's `obs-text` is \
                 opaque, so v1 declines to decide what it means — `HTTP-V1-OBS-TEXT`)"
            ),
            Self::SurroundingWhitespace { spelling } => write!(
                f,
                "`{spelling}` begins or ends with whitespace; RFC 9110 §5.5 strips leading and \
                 trailing `OWS` on receipt, so those bytes are not part of the value and \
                 `source.http.v1` refuses rather than silently dropping them"
            ),
        }
    }
}

/// Why a message's framing is not one RFC 9112 §6 admits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FramingRefusal {
    /// A declared `Content-Length` that is not the content's byte length. **The
    /// card's signature refusal.**
    LengthDisagrees { declared: u64, actual: usize },
    /// A `Content-Length` value that is not `1*DIGIT`.
    LengthNotDecimal { spelling: String },
    /// Content with no declared length.
    UnframedContent { actual: usize },
    /// A second `Content-Length` field (RFC 9112 §6.3).
    DuplicateLength,
    /// Any `Transfer-Encoding`.
    TransferCoding { spelling: String },
}

impl fmt::Display for FramingRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthDisagrees { declared, actual } => write!(
                f,
                "this message declares `Content-Length: {declared}` and carries {actual} byte(s) \
                 of content. Both numbers are known at check time — whether each component was \
                 written as a quoted token or composed from values this entry computed — so \
                 `source.http.v1` decides the equality here rather than emitting a message whose \
                 own framing contradicts it"
            ),
            Self::LengthNotDecimal { spelling } => write!(
                f,
                "`{spelling}` is not a `Content-Length`; RFC 9112 §6.2 spells one as `1*DIGIT`"
            ),
            Self::UnframedContent { actual } => write!(
                f,
                "this message carries {actual} byte(s) of content and declares no \
                 `Content-Length`. RFC 9112 §6.3 then frames the content by closing the \
                 connection, which `source.http.v1` does not model — declare the length"
            ),
            Self::DuplicateLength => f.write_str(
                "this message declares `Content-Length` twice; RFC 9112 §6.3 treats a message \
                 with conflicting lengths as unrecoverable, so `source.http.v1` refuses to build \
                 one",
            ),
            Self::TransferCoding { spelling } => write!(
                f,
                "`Transfer-Encoding: {spelling}` frames content by a transfer coding \
                 `source.http.v1` does not model; v1 frames every message by `Content-Length` \
                 (`HTTP-V1-CHUNKED`)"
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HttpError {
    /// `Q090`: the flat entry shape this profile models is violated, or a
    /// message is extended after it has already been put on a wire.
    Shape { event: usize, reason: String },
    /// `Q091`: a start-line component RFC 9112 does not admit there.
    StartLine {
        event: usize,
        refusal: ComponentRefusal,
    },
    /// `Q092`: a header field RFC 9110/9112 does not admit.
    Field { event: usize, reason: String },
    /// `Q093`: a framing rule RFC 9112 §6 states and this message breaks.
    Framing {
        event: usize,
        binding: String,
        refusal: FramingRefusal,
    },
    /// `Q094`: a name at a message or value position that is not one.
    NotAMessage { event: usize, name: String },
    /// `Q094`, **the same condition reached by a new position** (card
    /// `HTTP-V1-COMPUTED-LENGTH`): a name at a **text** or **scalar** position
    /// that this entry bound to no such value. `what` names the position, so
    /// one code still distinguishes the faults by message.
    ///
    /// No code is allocated: the Q-space note (`Q-NAMESPACE-DECISION`) puts a
    /// new allocation in the icebox, and `Q094`'s condition — "a name at a
    /// position that is not one" — is widened rather than duplicated, on the
    /// `SOCKET-EVENT-COMPOSE` precedent.
    NotAValue {
        event: usize,
        name: String,
        what: &'static str,
    },
    /// `Q095`: a refusal reported by the **inherited, unedited**
    /// `source.socket.v1` checker — custody, the transport state machine, or the
    /// determinism boundary. This variant carries that checker's own error value
    /// rather than a re-worded copy of it.
    Transport { event: usize, error: SocketError },
    /// `Q096`: the declared `effects { … }` row does not admit a derived effect.
    EffectNotDeclared { event: usize, effect: Effect },
}

impl HttpError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Shape { .. } => "Q090",
            Self::StartLine { .. } => "Q091",
            Self::Field { .. } => "Q092",
            Self::Framing { .. } => "Q093",
            Self::NotAMessage { .. } | Self::NotAValue { .. } => "Q094",
            Self::Transport { .. } => "Q095",
            Self::EffectNotDeclared { .. } => "Q096",
        }
    }

    #[must_use]
    pub fn event(&self) -> usize {
        match self {
            Self::Shape { event, .. }
            | Self::StartLine { event, .. }
            | Self::Field { event, .. }
            | Self::Framing { event, .. }
            | Self::NotAMessage { event, .. }
            | Self::NotAValue { event, .. }
            | Self::Transport { event, .. }
            | Self::EffectNotDeclared { event, .. } => *event,
        }
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape { reason, .. } | Self::Field { reason, .. } => f.write_str(reason),
            Self::StartLine { refusal, .. } => write!(f, "{refusal}"),
            Self::Framing {
                binding, refusal, ..
            } => write!(f, "`{binding}`: {refusal}"),
            Self::NotAMessage { name, .. } => write!(
                f,
                "`{name}` is not an HTTP message or a string view this entry bound; \
                 `source.http.v1` transfers a declared message or a bound view, so that every \
                 byte on the wire is decided before emission"
            ),
            Self::NotAValue { name, what, .. } => write!(
                f,
                "`{name}` is not {what} this entry bound; `source.http.v1` resolves every \
                 computed component against values whose bytes it already holds, so a name it \
                 cannot resolve is refused rather than deferred to run time"
            ),
            Self::Transport { error, .. } => write!(
                f,
                "the checked socket discipline refuses this program: {error}"
            ),
            Self::EffectNotDeclared { effect, .. } => write!(
                f,
                "this entry performs `{effect:?}` but its declared `effects {{ … }}` row does not \
                 admit it"
            ),
        }
    }
}

// --- RFC grammar predicates, each in one place. ---

/// RFC 9110 §5.6.2 `tchar`.
#[must_use]
pub fn is_tchar(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

/// RFC 9110 §5.6.2 `token`: `1*tchar`.
#[must_use]
pub fn is_token(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(is_tchar)
}

/// A byte admitted inside a reason phrase or a field value: `HTAB`, `SP`, or a
/// visible ASCII character.
///
/// `obs-text` (`%x80-FF`) is deliberately **excluded**, which is stricter than
/// RFC 9110 §5.5 and stated as such: obs-text is opaque data with no defined
/// character encoding, and a route whose whole evidence is a byte comparison
/// declines to put bytes on a wire whose meaning it cannot state
/// (`HTTP-V1-OBS-TEXT`).
#[must_use]
pub fn is_field_byte(byte: u8) -> bool {
    byte == b'\t' || (0x20..=0x7e).contains(&byte)
}

/// The first byte of `text` that may not appear in a field value or reason
/// phrase.
fn first_bad_field_byte(text: &str) -> Option<(usize, u8)> {
    text.bytes()
        .enumerate()
        .find(|(_, byte)| !is_field_byte(*byte))
}

/// RFC 9112 §3.2.1 origin-form, narrowed to what v1 models: an absolute path
/// beginning with `/`, every byte a visible ASCII character, and no `#`.
#[must_use]
pub fn is_origin_form(target: &str) -> bool {
    target.starts_with('/')
        && target
            .bytes()
            .all(|byte| (0x21..=0x7e).contains(&byte) && byte != b'#')
}

/// Check one start line, returning the first component RFC 9112 does not admit
/// there.
///
/// # Errors
///
/// [`ComponentRefusal`] naming the component and why.
pub fn admit_start(start: &HttpStart) -> Result<(), ComponentRefusal> {
    if start.version() != HTTP_VERSION {
        return Err(ComponentRefusal::Version {
            spelling: start.version().to_owned(),
        });
    }
    match start {
        HttpStart::Request { method, target, .. } => {
            if !is_token(method) {
                return Err(ComponentRefusal::NotAToken {
                    what: "method",
                    spelling: method.clone(),
                });
            }
            if let Some(registered) = CASE_SENSITIVE_METHODS.iter().find(|registered| {
                **registered != method && registered.eq_ignore_ascii_case(method)
            }) {
                return Err(ComponentRefusal::MethodCase {
                    spelling: method.clone(),
                    registered: (*registered).to_owned(),
                });
            }
            if !is_origin_form(target) {
                return Err(ComponentRefusal::NotAnOriginForm {
                    spelling: target.clone(),
                });
            }
            Ok(())
        }
        HttpStart::Response { status, reason, .. } => {
            if !(100..=599).contains(status) {
                return Err(ComponentRefusal::Status { value: *status });
            }
            match first_bad_field_byte(reason) {
                Some((at, byte)) => Err(ComponentRefusal::NotFieldContent {
                    what: "reason phrase",
                    at,
                    byte,
                }),
                None => Ok(()),
            }
        }
    }
}

/// Check one header field, returning why it is not one this route admits.
///
/// # Errors
///
/// [`ComponentRefusal`] naming the component and why.
pub fn admit_field(field: &HttpField) -> Result<(), ComponentRefusal> {
    if !is_token(&field.name) {
        return Err(ComponentRefusal::NotAToken {
            what: "field name",
            spelling: field.name.clone(),
        });
    }
    if let Some((at, byte)) = first_bad_field_byte(&field.value) {
        return Err(ComponentRefusal::NotFieldContent {
            what: "field value",
            at,
            byte,
        });
    }
    let trimmed = field.value.trim_matches([' ', '\t']);
    if trimmed != field.value {
        return Err(ComponentRefusal::SurroundingWhitespace {
            spelling: field.value.clone(),
        });
    }
    Ok(())
}

/// The message head: start line, each field, and the empty line that ends it
/// (RFC 9112 §2.1).
///
/// **The framing is written here and nowhere else.** Every `CRLF` in the result
/// is one of this function's, because [`admit_start`] and [`admit_field`] refuse
/// a component carrying `CR` or `LF`.
#[must_use]
pub fn head_bytes(message: &HttpMessage) -> Vec<u8> {
    let mut head = match &message.start {
        HttpStart::Request {
            method,
            target,
            version,
        } => format!("{method} {target} {version}"),
        HttpStart::Response {
            version,
            status,
            reason,
        } => format!("{version} {status} {reason}"),
    }
    .into_bytes();
    head.extend_from_slice(CRLF);
    for field in &message.fields {
        head.extend_from_slice(field.name.as_bytes());
        head.extend_from_slice(b": ");
        head.extend_from_slice(field.value.as_bytes());
        head.extend_from_slice(CRLF);
    }
    head.extend_from_slice(CRLF);
    head
}

/// The whole message: head then content.
#[must_use]
pub fn wire_bytes(message: &HttpMessage) -> Vec<u8> {
    let mut bytes = head_bytes(message);
    if let Some(content) = &message.content {
        bytes.extend_from_slice(content);
    }
    bytes
}

/// The `Content-Length` values a message declares, in order, with the parse of
/// each.
fn declared_lengths(message: &HttpMessage) -> Vec<Result<u64, String>> {
    message
        .fields
        .iter()
        .filter(|field| field.name.eq_ignore_ascii_case(CONTENT_LENGTH))
        .map(|field| {
            let spelling = field.value.clone();
            if !spelling.is_empty() && spelling.bytes().all(|byte| byte.is_ascii_digit()) {
                spelling.parse::<u64>().map_err(|_| spelling)
            } else {
                Err(spelling)
            }
        })
        .collect()
}

/// The framing rule of RFC 9112 §6, decided over a message whose components are
/// all literals.
///
/// # Errors
///
/// [`FramingRefusal`] naming the rule this message breaks.
pub fn admit_framing(message: &HttpMessage) -> Result<Option<u64>, FramingRefusal> {
    if let Some(field) = message
        .fields
        .iter()
        .find(|field| field.name.eq_ignore_ascii_case(TRANSFER_ENCODING))
    {
        return Err(FramingRefusal::TransferCoding {
            spelling: field.value.clone(),
        });
    }
    let declared = declared_lengths(message);
    if declared.len() > 1 {
        return Err(FramingRefusal::DuplicateLength);
    }
    let actual = message.content.as_ref().map_or(0usize, Vec::len);
    match declared.first() {
        Some(Err(spelling)) => Err(FramingRefusal::LengthNotDecimal {
            spelling: spelling.clone(),
        }),
        Some(Ok(declared)) => {
            if *declared == actual as u64 {
                Ok(Some(*declared))
            } else {
                Err(FramingRefusal::LengthDisagrees {
                    declared: *declared,
                    actual,
                })
            }
        }
        // A message with no content and no declared length is framed as having
        // none, which RFC 9112 §6.3 rule 6 says exactly. A message *with*
        // content and no declared length is not.
        None if message.content.is_none() => Ok(None),
        None => Err(FramingRefusal::UnframedContent { actual }),
    }
}

/// The whole-message admission: start line, every field, the required `Host` of
/// a request, and the framing.
///
/// # Errors
///
/// Every refusal the message earns, in the order the rules are stated.
pub fn admit_message(event: usize, message: &HttpMessage) -> Result<Option<u64>, Vec<HttpError>> {
    let mut errors = Vec::new();
    if let Err(refusal) = admit_start(&message.start) {
        errors.push(HttpError::StartLine { event, refusal });
    }
    for field in &message.fields {
        if let Err(refusal) = admit_field(field) {
            errors.push(HttpError::Field {
                event,
                reason: refusal.to_string(),
            });
        }
        if field.name.eq_ignore_ascii_case(CONNECTION)
            && !field.value.eq_ignore_ascii_case(CONNECTION_CLOSE)
        {
            errors.push(HttpError::Field {
                event,
                reason: format!(
                    "`{CONNECTION}: {}` declares a connection option `source.http.v1` does not \
                     honour; v1's transport closes after one exchange, so the only value it can \
                     truthfully carry is `{CONNECTION_CLOSE}` (`HTTP-V1-PERSISTENT`)",
                    field.value
                ),
            });
        }
    }
    // RFC 9112 §3.2: "A client MUST send a Host header field in all HTTP/1.1
    // request messages." Exactly one: a second is a request this route refuses
    // to build rather than one a server must reject.
    if message.start.is_request() {
        let hosts = message
            .fields
            .iter()
            .filter(|field| field.name.eq_ignore_ascii_case(HOST))
            .count();
        if hosts != 1 {
            errors.push(HttpError::Field {
                event,
                reason: format!(
                    "this request carries {hosts} `{HOST}` field(s); RFC 9112 §3.2 requires \
                     exactly one in every HTTP/1.1 request, and a server that receives a request \
                     without one must respond 400"
                ),
            });
        }
    }
    let framing = match admit_framing(message) {
        Ok(declared) => declared,
        Err(refusal) => {
            errors.push(HttpError::Framing {
                event,
                binding: message.binding.clone(),
                refusal,
            });
            None
        }
    };
    if errors.is_empty() {
        Ok(framing)
    } else {
        Err(errors)
    }
}

/// The transport program this route lowers to, with the message records it
/// published and the index map back to the HTTP statements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpTransport {
    /// The `source.socket.v1` program, checked by that route's **unedited**
    /// checker.
    pub program: SocketProgram,
    /// `attribution[i]` is the HTTP statement index the socket statement at `i`
    /// came from, so an inherited refusal names the source statement a reader
    /// wrote.
    pub attribution: Vec<usize>,
    pub messages: Vec<HttpMessageRecord>,
    pub views: Vec<(String, Vec<u8>)>,
    pub compositions: Vec<HttpComposition>,
    pub scalars: BTreeMap<String, i32>,
}

/// The check-time environment a computed component resolves against.
///
/// Two maps, and they never merge: a text name is never a scalar and a scalar
/// name is never text. That is [`crate::std_io::StdIoProgram`]'s own discipline
/// for `scalars`/`identities`, applied here for the same reason — a name
/// resolved in the wrong namespace would be a silently different byte sequence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ValueEnvironment {
    text: BTreeMap<String, Vec<u8>>,
    scalars: BTreeMap<String, i32>,
}

impl ValueEnvironment {
    /// The exact bytes one piece denotes, or the position that did not resolve.
    ///
    /// **Total on a resolvable environment, and never deferred.** A literal is
    /// already bytes, a name is bytes this entry bound, and a rendering is the
    /// digits [`Rendering::render`] writes — so every arm is a byte sequence the
    /// compiler holds, which is exactly the premise the framing gate needs.
    fn resolve(&self, piece: &HttpPiece) -> Result<Vec<u8>, (String, &'static str)> {
        match piece {
            HttpPiece::Literal(bytes) => Ok(bytes.clone()),
            HttpPiece::Name(name) => self
                .text
                .get(name)
                .cloned()
                .ok_or_else(|| (name.clone(), "text")),
            HttpPiece::Scalar { rendering, name } => self
                .scalars
                .get(name)
                .map(|value| rendering.render(*value).into_bytes())
                .ok_or_else(|| (name.clone(), "an integer")),
        }
    }
}

/// The scalar names the **transport half** of this program reads.
///
/// A `len` binding is a check-time constant: it lowers to the transport's own
/// integer binding only where a `print`/`log` line renders it, so a length used
/// solely as a header value contributes no C object. Without this the emitted
/// program would carry an unread `int32_t` and fail `-Werror=unused-variable` —
/// a C compile failure rather than a QCore refusal, which is the shape of
/// hygiene finding `STRING-CONCAT` §8 recorded three of.
fn transport_scalar_reads(program: &HttpProgram) -> BTreeSet<String> {
    let mut read = BTreeSet::new();
    let mut note = |operand: &StdIoOperand| {
        if let StdIoOperand::Scalar(name) | StdIoOperand::Borrowed(name) = operand {
            read.insert(name.clone());
        }
    };
    for statement in &program.statements {
        if let HttpStatement::Transport(SocketStatement::Line(event)) = statement {
            match event {
                StdIoEvent::Print { operand, .. } | StdIoEvent::Log { operand, .. } => {
                    note(operand)
                }
                StdIoEvent::LogLevel { .. } | StdIoEvent::Flush | StdIoEvent::Return => {}
            }
        }
    }
    read
}

/// Resolve one piece against the environment, reporting the position that did
/// not resolve at `Q094`.
fn resolve(
    environment: &ValueEnvironment,
    piece: &HttpPiece,
    event: usize,
    errors: &mut Vec<HttpError>,
) -> Option<Vec<u8>> {
    match environment.resolve(piece) {
        Ok(bytes) => Some(bytes),
        Err((name, what)) => {
            errors.push(HttpError::NotAValue { event, name, what });
            None
        }
    }
}

/// Register a value binding, refusing a name this entry already bound.
///
/// One namespace for the check: a name is text **or** a scalar and never both,
/// so `dec(s)` on a bound view and `concat(n)` on a bound integer are each a
/// `Q094` rather than a silently different byte sequence.
fn bind_value(
    environment: &mut ValueEnvironment,
    binding: &str,
    event: usize,
    errors: &mut Vec<HttpError>,
) -> bool {
    if environment.text.contains_key(binding) || environment.scalars.contains_key(binding) {
        errors.push(HttpError::Shape {
            event,
            reason: format!(
                "`{binding}` already names a value this entry bound; `source.http.v1` binds each \
                 name once, so that the bytes a component names are never ambiguous"
            ),
        });
        return false;
    }
    true
}

/// Build the transport program: validate every message, compute its wire bytes,
/// and rewrite each statement into `source.socket.v1`'s own.
///
/// This is the **whole** of what the HTTP vocabulary does. Everything after it —
/// custody, the transport state machine, the determinism boundary, the effect
/// row — is the socket checker's, run unedited over the result.
///
/// # Errors
///
/// Every refusal the HTTP half of the program earns, in program order.
pub fn lower_transport(program: &HttpProgram) -> Result<HttpTransport, Vec<HttpError>> {
    let mut errors = Vec::new();
    let mut messages: BTreeMap<String, HttpMessage> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut sent: BTreeSet<String> = BTreeSet::new();
    let mut computed: BTreeMap<String, (bool, bool)> = BTreeMap::new();
    let mut environment = ValueEnvironment::default();
    let mut view_order: Vec<String> = Vec::new();
    let mut compositions: Vec<HttpComposition> = Vec::new();
    let mut published: Vec<HttpMessageRecord> = Vec::new();
    let mut statements = Vec::new();
    let mut attribution = Vec::new();
    let rendered_scalars = transport_scalar_reads(program);

    for (index, statement) in program.statements.iter().enumerate() {
        match statement {
            HttpStatement::Declare { binding, start } => {
                if messages.contains_key(binding) {
                    errors.push(HttpError::Shape {
                        event: index,
                        reason: format!(
                            "`{binding}` already names a message this entry declared; \
                             `source.http.v1` builds each message once, so that the bytes a \
                             transfer names are never ambiguous"
                        ),
                    });
                    continue;
                }
                messages.insert(
                    binding.clone(),
                    HttpMessage {
                        binding: binding.clone(),
                        start: start.clone(),
                        fields: Vec::new(),
                        content: None,
                    },
                );
                order.push(binding.clone());
            }
            HttpStatement::Header {
                message: name,
                name: field_name,
                value,
            } => {
                if sent.contains(name) {
                    errors.push(HttpError::Shape {
                        event: index,
                        reason: format!(
                            "`{name}` was already put on a wire, so its bytes are decided; \
                             `source.http.v1` completes a message before it transfers one"
                        ),
                    });
                    continue;
                }
                // **The value is resolved here, before any gate runs.** A
                // computed component becomes the same byte sequence a quoted
                // token would have produced, so `admit_field`'s `CR`/`LF`
                // exclusion and `admit_framing`'s length equality are the
                // *unchanged* functions of card `HTTP-QUIRIN-V1` — they see
                // bytes and never learn which spelling produced them.
                let Some(bytes) = resolve(&environment, value, index, &mut errors) else {
                    continue;
                };
                let Ok(text) = String::from_utf8(bytes) else {
                    // Unreachable from source: every piece is a decoded literal
                    // (well-formed UTF-8 by `decode_string_literal`) or ASCII
                    // digits, and a concatenation of well-formed UTF-8 is
                    // well-formed. Kept as a fail-closed guard on the model
                    // rather than an `expect`, on the `EXTERN-F1-STORE-REACH`
                    // precedent (`HTTP-COMPUTED-UTF8-REACH`).
                    errors.push(HttpError::Field {
                        event: index,
                        reason: format!(
                            "the bytes this `{field_name}` value denotes are not text; a field \
                             value is ASCII field-content, so `source.http.v1` refuses rather \
                             than putting bytes on the wire whose meaning it cannot state"
                        ),
                    });
                    continue;
                };
                if value.is_computed() && field_name.eq_ignore_ascii_case(CONTENT_LENGTH) {
                    computed.entry(name.clone()).or_default().0 = true;
                }
                match messages.get_mut(name) {
                    Some(message) => message.fields.push(HttpField {
                        name: field_name.clone(),
                        value: text,
                    }),
                    None => errors.push(HttpError::NotAMessage {
                        event: index,
                        name: name.clone(),
                    }),
                }
            }
            HttpStatement::Content {
                message: name,
                value,
            } => {
                if sent.contains(name) {
                    errors.push(HttpError::Shape {
                        event: index,
                        reason: format!(
                            "`{name}` was already put on a wire, so its bytes are decided; \
                             `source.http.v1` completes a message before it transfers one"
                        ),
                    });
                    continue;
                }
                let Some(bytes) = resolve(&environment, value, index, &mut errors) else {
                    continue;
                };
                if value.is_computed() {
                    computed.entry(name.clone()).or_default().1 = true;
                }
                match messages.get_mut(name) {
                    Some(message) if message.content.is_some() => {
                        errors.push(HttpError::Shape {
                            event: index,
                            reason: format!(
                                "`{name}` already carries content; `source.http.v1` sets it once, \
                                 so that the byte length a `{CONTENT_LENGTH}` is checked against \
                                 is never a moving target"
                            ),
                        });
                    }
                    Some(message) => message.content = Some(bytes),
                    None => errors.push(HttpError::NotAMessage {
                        event: index,
                        name: name.clone(),
                    }),
                }
            }
            HttpStatement::Compose { binding, pieces } => {
                // A composition of one *is* that piece. `source.string.v1`
                // refuses it for the reason this route inherits: two spellings
                // of one value with nothing to tell them apart by.
                if pieces.len() < 2 {
                    errors.push(HttpError::Shape {
                        event: index,
                        reason: format!(
                            "`concat(…)` composes two or more pieces; a composition of \
                             {} *is* that piece, and `source.http.v1` refuses the second \
                             spelling of one value",
                            pieces.len()
                        ),
                    });
                    continue;
                }
                let mut bytes = Vec::new();
                let mut piece_lens = Vec::new();
                let mut resolved = true;
                for piece in pieces {
                    match resolve(&environment, piece, index, &mut errors) {
                        Some(piece_bytes) => {
                            piece_lens.push(u32::try_from(piece_bytes.len()).unwrap_or(u32::MAX));
                            bytes.extend_from_slice(&piece_bytes);
                        }
                        None => resolved = false,
                    }
                }
                if !resolved {
                    continue;
                }
                // The same bound `source.string.v1` puts on a literal, applied
                // to a composition for the same reason: a payload this route
                // cannot model exactly is not modelled at all.
                if bytes.len() > MAX_STRING_BYTES as usize {
                    errors.push(HttpError::Shape {
                        event: index,
                        reason: format!(
                            "this composition is {} bytes, longer than the {MAX_STRING_BYTES}-byte \
                             payload `source.http.v1` models; a longer one is refused rather than \
                             truncated",
                            bytes.len()
                        ),
                    });
                    continue;
                }
                if !bind_value(&mut environment, binding, index, &mut errors) {
                    continue;
                }
                compositions.push(HttpComposition {
                    event: index,
                    binding: binding.clone(),
                    piece_lens,
                    bytes: bytes.clone(),
                });
                environment.text.insert(binding.clone(), bytes);
                view_order.push(binding.clone());
            }
            HttpStatement::Length { binding, name } => {
                let Some(bytes) = environment.text.get(name) else {
                    errors.push(HttpError::NotAValue {
                        event: index,
                        name: name.clone(),
                        what: "text",
                    });
                    continue;
                };
                let length = i32::try_from(bytes.len()).unwrap_or(i32::MAX);
                if !bind_value(&mut environment, binding, index, &mut errors) {
                    continue;
                }
                environment.scalars.insert(binding.clone(), length);
                // The transport binds the same name only where the emitted
                // program reads it; see `transport_scalar_reads`.
                if rendered_scalars.contains(binding) {
                    statements.push(SocketStatement::Let {
                        binding: binding.clone(),
                        value: length,
                    });
                    attribution.push(index);
                }
            }
            HttpStatement::Text { binding, bytes } => {
                if !bind_value(&mut environment, binding, index, &mut errors) {
                    continue;
                }
                environment.text.insert(binding.clone(), bytes.clone());
                view_order.push(binding.clone());
            }
            HttpStatement::Transfer {
                socket,
                value,
                receive,
            } => {
                // A message is admitted at the point it is transferred, not at
                // the point it is declared: that is when it is complete, and the
                // framing rule is a statement about a complete message.
                let payload = if let Some(message) = messages.get(value) {
                    // Admitted **once**, at the first transfer: a message is
                    // complete from that point on (a later `header` or `content`
                    // is refused above), so re-deciding it at every transfer
                    // would report one fault as several.
                    if sent.insert(value.clone()) {
                        let flags = computed.get(value).copied().unwrap_or_default();
                        match admit_message(index, message) {
                            Ok(declared) => published.push(HttpMessageRecord {
                                binding: message.binding.clone(),
                                kind: message.start.label(),
                                symbol: message_symbol(&message.binding),
                                head: head_bytes(message),
                                content: message.content.clone(),
                                declared_length: declared,
                                field_count: message.fields.len(),
                                transferred: true,
                                declared_length_computed: flags.0,
                                content_computed: flags.1,
                            }),
                            Err(refusals) => errors.extend(refusals),
                        }
                    }
                    let Some(record) = published.iter().find(|record| record.binding == *value)
                    else {
                        // The message was refused above; a transfer of it says
                        // nothing further.
                        continue;
                    };
                    TextPayload::named(record.symbol.clone(), record.wire())
                } else if let Some(bytes) = environment.text.get(value) {
                    TextPayload::inline(bytes.clone())
                } else {
                    errors.push(HttpError::NotAMessage {
                        event: index,
                        name: value.clone(),
                    });
                    continue;
                };
                statements.push(transfer_statement(socket, *receive, payload));
                attribution.push(index);
            }
            HttpStatement::Transport(statement) => {
                // The transport's own integer binding is the **one** place a
                // scalar enters this route: `let n = 200;` is read by
                // `source.socket.v1`'s reader and checked by its checker, and
                // this route merely observes the value it already carries. A
                // second scalar reader here could disagree with that one.
                if let SocketStatement::Let { binding, value } = statement {
                    environment.scalars.insert(binding.clone(), *value);
                }
                statements.push(statement.clone());
                attribution.push(index);
            }
        }
    }

    // A message this entry declared and never transferred is checked **exactly
    // as hard** as one it sent, at the end of the program rather than at a
    // transfer. Skipping it would make "declare it and never send it" a way to
    // hold an unconformant message, which is the kind of hole invariant 1
    // exists to close.
    for binding in &order {
        if sent.contains(binding) {
            continue;
        }
        let message = messages.get(binding).expect("declared above");
        let flags = computed.get(binding).copied().unwrap_or_default();
        match admit_message(program.statements.len(), message) {
            Ok(declared) => published.push(HttpMessageRecord {
                binding: message.binding.clone(),
                kind: message.start.label(),
                symbol: message_symbol(&message.binding),
                head: head_bytes(message),
                content: message.content.clone(),
                declared_length: declared,
                field_count: message.fields.len(),
                transferred: false,
                declared_length_computed: flags.0,
                content_computed: flags.1,
            }),
            Err(refusals) => errors.extend(refusals),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(HttpTransport {
        program: SocketProgram {
            entry: program.entry.clone(),
            declared_effects: program.declared_effects.clone(),
            statements,
        },
        attribution,
        messages: published,
        views: view_order
            .into_iter()
            .map(|name| {
                let bytes = environment.text.get(&name).cloned().unwrap_or_default();
                (name, bytes)
            })
            .collect(),
        compositions,
        scalars: environment.scalars,
    })
}

/// One transfer, as `source.socket.v1` spells it.
fn transfer_statement(socket: &str, receive: bool, payload: TextPayload) -> SocketStatement {
    if receive {
        let count = u32::try_from(payload.bytes.len()).unwrap_or(u32::MAX);
        SocketStatement::Recv {
            binding: None,
            name: socket.to_owned(),
            count,
            expect: Some(payload),
        }
    } else {
        SocketStatement::Send {
            name: socket.to_owned(),
            payload: std_socket::SocketPayload::Text(payload),
        }
    }
}

/// Check a lowered program and derive its certificate.
///
/// Two halves, in this order and deliberately:
///
/// 1. **The HTTP vocabulary** — [`lower_transport`]: every message's components
///    against RFC 9112's grammar, its framing against its own content, and its
///    wire bytes constructed.
/// 2. **Everything else**, decided by the *unedited* `source.socket.v1` checker
///    over the program half one produced. Custody, the transport state machine,
///    the determinism boundary and the declared effect row are all that checker's
///    answers, re-attributed to the HTTP statement a reader wrote.
///
/// # Errors
///
/// Every refusal the program earns, in program order.
pub fn check_http_program(program: &HttpProgram) -> Result<HttpCertificate, Vec<HttpError>> {
    let transport = lower_transport(program)?;
    let certificate = std_socket::check_socket_program(&transport.program).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                let event = transport
                    .attribution
                    .get(error.event())
                    .copied()
                    .unwrap_or(program.statements.len());
                match error {
                    // The one socket refusal that is about a row a reader of
                    // *this* route wrote, so it keeps its own code here.
                    SocketError::EffectNotDeclared { effect, .. } => {
                        HttpError::EffectNotDeclared { event, effect }
                    }
                    error => HttpError::Transport { event, error },
                }
            })
            .collect::<Vec<_>>()
    })?;
    Ok(HttpCertificate {
        entry: program.entry.clone(),
        effects: certificate.effects.clone(),
        messages: transport.messages,
        views: transport.views,
        compositions: transport.compositions,
        scalars: transport.scalars,
        transport: certificate,
    })
}

/// The effects the **HTTP vocabulary itself** derives: none, on any program.
///
/// Stated as a function rather than as a sentence so that widening the
/// vocabulary changes a theorem rather than slipping past one. Declaring a
/// message, adding a field and setting content compute a byte sequence at check
/// time: they allocate nothing, mutate nothing and write nothing. Every effect an
/// admitted program declares is the transport's or the output module's.
#[must_use]
pub fn http_derived_effects(program: &HttpProgram) -> EffectRow {
    let _ = program;
    EffectRow::default()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HttpRecheckFailure {
    Entry,
    Effects,
    Messages,
    Views,
    Compositions,
    Scalars,
    Transport,
    Refused,
}

impl fmt::Display for HttpRecheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Self::Entry => "the entry name",
            Self::Effects => "the derived effect row",
            Self::Messages => "the declared messages and their wire bytes",
            Self::Views => "the bound string views",
            Self::Compositions => "the composed payloads and their length arithmetic",
            Self::Scalars => "the check-time scalar environment",
            Self::Transport => "the transport certificate",
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
/// [`HttpRecheckFailure`] naming the first field that does not reproduce.
pub fn recheck_http_certificate(
    program: &HttpProgram,
    certificate: &HttpCertificate,
) -> Result<(), HttpRecheckFailure> {
    let rederived = check_http_program(program).map_err(|_| HttpRecheckFailure::Refused)?;
    if rederived.entry != certificate.entry {
        return Err(HttpRecheckFailure::Entry);
    }
    if rederived.effects != certificate.effects {
        return Err(HttpRecheckFailure::Effects);
    }
    if rederived.messages != certificate.messages {
        return Err(HttpRecheckFailure::Messages);
    }
    if rederived.views != certificate.views {
        return Err(HttpRecheckFailure::Views);
    }
    if rederived.compositions != certificate.compositions {
        return Err(HttpRecheckFailure::Compositions);
    }
    if rederived.scalars != certificate.scalars {
        return Err(HttpRecheckFailure::Scalars);
    }
    if rederived.transport != certificate.transport {
        return Err(HttpRecheckFailure::Transport);
    }
    Ok(())
}
