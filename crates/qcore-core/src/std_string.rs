//! `source.string.v1` — the checked model behind the `std.string` bundled
//! module (card `STRING-SLICE`, normative statement
//! `docs/design/STRING_SLICE_V1.md`).
//!
//! # What this model is, stated before anything else
//!
//! A program on this route is a flat, ordered list of events in one entry
//! function. It may **construct an immutable UTF-8 string view of statically
//! stored literal bytes**, read that view's exact byte length, derive another
//! view by a checked slice, bind an ordinary integer, and — reusing
//! `source.std-io.v1`'s statements unchanged — write rendered lines.
//!
//! # A view is not a resource, and that is the whole difference from the buffer
//!
//! `source.buffer.v1` acquires an owned `{identity, extent, custody}` and the
//! unedited cycle-3 ownership checker discharges it. This route builds **no
//! ownership operation list at all**, and that is not an omission: amendment §4
//! says slicing "is not ownership splitting", §3.6 says "creating a view neither
//! creates nor discharges that custody", and the only storage v1 can name is a
//! string literal, whose storage duration is static. There is therefore no
//! obligation for a leak rule to be about. A future
//! `STRING-SLICE-BUFFER-BORROW` introduces a view whose lifetime is bounded by a
//! live buffer, and *that* card owns the lifetime rule; v1 states the boundary
//! rather than pretending to have crossed it.
//!
//! What makes the claim safe today is [`StringViewRecord::literal`]: every view
//! in an admitted program — literal or derived — names the index of the literal
//! whose static bytes it addresses. A derived view cannot outlive its backing
//! storage because that storage is the program's own static data, and the
//! certificate publishes which.
//!
//! # The decidability rule, inherited for the same reason
//!
//! [`StringOperand`] has the same two constructors [`crate::std_buffer::BufferOperand`]
//! has, and **neither can denote a string**: a slice bound is an integer literal
//! or a name bound to a check-time-constant integer. A string is named only at
//! the two positions typed `name: String` (a slice's source, a `len`'s
//! argument), and a literal is the only constructor of one.
//!
//! Every bound is therefore decidable at check time. Amendment §2 permits an
//! implementation to "decide these conditions statically or emit checked failure
//! according to the enclosing effect/error profile"; v1 takes the **first** arm,
//! totally. Because every bound is a check-time constant, there is no residual
//! case for a checked failure to represent, so v1 introduces no trap, no error
//! value and no failure arm — a bound the checker cannot decide is refused by
//! name at `Q069` and the missing capability is the named follow-on
//! `STRING-SLICE-VARYING-INDEX`. That is the same discipline, and the same
//! ground, as `source.buffer.v1`'s: this route's evidence is a byte comparison
//! between a reference execution and a native run, and a value the reference
//! interpreter cannot compute is one it cannot predict a line for.
//!
//! # What is not claimed
//!
//! * **No mutation, no concatenation, no comparison, no search.** Amendment §3.7
//!   states what those must be when they arrive; none is here.
//! * **No conversion from `std.buffer`** (§5) and **no crossing of the extern or
//!   native-library boundary** (§6). The latter needs no code on this route:
//!   route selection places it *after* the extern route, so a module that does
//!   both keeps that route's existing refusal — measured, not assumed.
//! * **No rendering of text** (§7). A rendering operand is a scalar; a string at
//!   one refuses at `Q072`.
//! * **No `cap`.** The `{ptr, len, cap}` shape is reserved for the mutable
//!   byte slice of `BYTE-SLICE-CAP` and putting capacity in an immutable string
//!   would grow its value and ABI size without enabling an operation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::std_io::{
    Level, ObservedLine, Rendering, StdIoEvent, StdIoOperand, rendered_line, rendered_text_line,
};
use crate::typed_errors::{Effect, EffectRow};

/// The largest literal payload this route admits, in bytes.
///
/// Bounded so every length and offset in the model, the certificate and the
/// emitted C is exactly representable as an `i32` scalar and as a `size_t`, and
/// so a length can never overflow the scalar environment this route shares with
/// `source.std-io.v1`'s renderings. A longer literal is refused by name rather
/// than truncated — a payload this route cannot model exactly is not modelled at
/// all, the discipline `source.buffer.v1` applies to an extent and
/// `source.std-io.v1` to an integer literal.
pub const MAX_STRING_BYTES: u32 = 4096;

/// What a slice bound may be.
///
/// **This enum is half the type discipline** (the other half is that a string is
/// nameable only at the two view positions). Neither constructor can denote a
/// string, so a slice bound can never be one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StringOperand {
    /// An integer literal written at the site.
    Literal(i32),
    /// A name this entry bound to a check-time-constant integer — by a literal
    /// `let`, or by `len(NAME)`.
    Scalar(String),
}

/// What a **rendered text** operand may be (card `STRING-SLICE-IO`).
///
/// Three forms, and every one of them denotes a string view of this program's
/// own static storage. There is deliberately no constructor for a scalar and
/// none for a borrowed custody identity: a text line renders the bytes of a
/// view, and a position that cannot hold a view cannot be written here at all.
/// The symmetry with [`StringOperand`] is exact and intentional — that enum
/// cannot denote a string and this one cannot denote anything else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextOperand {
    /// `print "…";` — a quoted literal written at the rendering site. The index
    /// is into [`StringProgram::literals`], the same pool a `let` literal view
    /// interns into.
    Literal(usize),
    /// `print s;` — a string this entry bound.
    Name(String),
    /// `print s[LOW:HIGH];` — one slice of a string this entry bound, written
    /// inline. It is the **same** checked slice `let t = s[LOW:HIGH];` is: the
    /// §4 arithmetic is recorded in the certificate's own [`SliceRecord`] list
    /// and the bounds are decided by the same rule, so an inline slice cannot be
    /// admitted where a bound one would be refused.
    Slice {
        name: String,
        low: StringOperand,
        high: StringOperand,
    },
}

/// The certificate-side name of the view an **inline** text slice derives.
///
/// An inline slice binds no source name, but the evidence a reader checks is
/// per-view, so it gets one here rather than being left out of the provenance
/// record. The spelling cannot collide with a source binding's emitted name: a
/// source binding is emitted as `v_NAME` or `s_NAME` and this is neither.
#[must_use]
pub fn text_slice_name(event: usize) -> String {
    format!("qcore_text_{event}")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StringEvent {
    /// `let NAME = INTEGER;` — an ordinary integer binding.
    Bind { binding: String, value: i32 },
    /// `let NAME = "…";` — a view of statically stored literal bytes. The
    /// payload is the **decoded** byte sequence; a C implementation terminator
    /// is not part of it (amendment §3).
    Literal { binding: String, literal: usize },
    /// `let NAME = NAME[LOW:HIGH];` — derive a view by checked pointer offset
    /// and length subtraction. No allocation, no copy, no custody.
    Slice {
        binding: String,
        name: String,
        low: StringOperand,
        high: StringOperand,
    },
    /// `let NAME = len(NAME);` — the exact byte count, available without
    /// scanning. Amendment §3.1, and the reason `strlen` appears nowhere.
    Len { binding: String, name: String },
    /// A `source.std-io.v1` line-producing statement, reused unchanged. The
    /// bytes of the line are decided by [`rendered_line`] and nowhere else.
    Line(StdIoEvent),
    /// `print TEXT;` / `log LEVEL TEXT;` — render a view's **exact UTF-8 bytes**
    /// to a stream (card `STRING-SLICE-IO`).
    ///
    /// `level` is `None` for `print` (standard output) and `Some(level)` for
    /// `log LEVEL` (the logger's stream), exactly as
    /// [`crate::std_io::line_framing`] reads it — so a text line and a rendered
    /// scalar line are framed by one function and cannot drift apart.
    ///
    /// There is **no rendering selector**: `dec` and `hex` are `std.fmt`'s two
    /// total functions from `i32` to digits, and there is nothing for either to
    /// do to a byte sequence that is already the text. A selector written in
    /// front of a string therefore keeps refusing at `Q072`, where it refused
    /// before this card.
    Text {
        level: Option<Level>,
        operand: TextOperand,
    },
    /// The entry's return. Exactly one, last.
    Return,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringProgram {
    pub entry: String,
    /// The entry's declared `effects { … }` row, verbatim from source.
    pub declared_effects: EffectRow,
    /// The decoded literal payloads, in the order the lowering met them. A
    /// literal's index is its static-storage identity for the whole program.
    pub literals: Vec<Vec<u8>>,
    pub events: Vec<StringEvent>,
}

/// One view, as the certificate publishes it: **which static storage it
/// addresses, where in that storage it starts, and how long it is.**
///
/// This triple is the `{ptr, len}` of amendment §3 with the provenance made
/// explicit. `literal` is the answer to "what storage does this view address",
/// which is what makes §3.6 — a derived view cannot outlive the storage it was
/// derived from — a checkable statement rather than a promise: in v1 that
/// storage is always a static literal of this program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StringViewRecord {
    /// Index into [`StringProgram::literals`].
    pub literal: usize,
    /// Byte offset of this view's first byte within that literal.
    pub offset: u32,
    /// Exact byte length. Never recovered by scanning.
    pub len: u32,
}

/// One admitted slice, published so a reader sees the §4 arithmetic rather than
/// being told it happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SliceRecord {
    pub event: usize,
    pub source: String,
    pub binding: String,
    /// The source view, verbatim.
    pub from: StringViewRecord,
    pub low: u32,
    pub high: u32,
    /// The derived view. `derived.literal == from.literal` and
    /// `derived.offset == from.offset + low` hold for every record — that
    /// equality *is* the zero-copy claim, and it is a field a reader can check
    /// rather than a sentence they must believe.
    pub derived: StringViewRecord,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringCertificate {
    pub entry: String,
    /// The derived row, recomputed by the recheck and compared.
    pub effects: EffectRow,
    /// The decoded literal payloads this program stores statically.
    pub literals: Vec<Vec<u8>>,
    /// Every view this program binds, in program order.
    pub views: Vec<(String, StringViewRecord)>,
    /// Every admitted slice, in program order.
    pub slices: Vec<SliceRecord>,
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

/// Why a quoted token is not a string value this route can construct.
///
/// Separate from the other refusals because decoding happens **before** any view
/// exists: a literal that does not decode has no payload, no length and no
/// boundaries, so none of the later rules can be stated about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiteralRefusal {
    /// An escape this route does not define. Refused rather than passed
    /// through: a byte sequence whose meaning the compiler guesses is not a
    /// decoded payload.
    UnknownEscape { at: usize, spelling: String },
    /// `\x` without two hexadecimal digits after it.
    TruncatedHexEscape { at: usize },
    /// A backslash at the very end of the token.
    TrailingBackslash,
    /// The decoded bytes are not well-formed UTF-8. Amendment §3.4 requires
    /// well-formedness for the whole lifetime of the view, so an ill-formed
    /// payload is refused at construction and never produced.
    NotUtf8 { at: usize },
    /// A payload longer than [`MAX_STRING_BYTES`].
    TooLong { bytes: usize },
}

impl fmt::Display for LiteralRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEscape { at, spelling } => write!(
                f,
                "`\\{spelling}` at byte {at} is not an escape `source.string.v1` defines; v1 \
                 defines `\\\\`, `\\\"`, `\\n`, `\\t`, `\\r`, `\\0` and `\\xHH`, and refuses \
                 anything else rather than guessing what its bytes were meant to be"
            ),
            Self::TruncatedHexEscape { at } => write!(
                f,
                "the `\\x` escape at byte {at} needs exactly two hexadecimal digits"
            ),
            Self::TrailingBackslash => {
                f.write_str("this literal ends in a backslash that escapes nothing")
            }
            Self::NotUtf8 { at } => write!(
                f,
                "the decoded bytes are not well-formed UTF-8 from byte {at}; a `string` is a \
                 sequence of bytes known to be well-formed UTF-8 for the whole lifetime of every \
                 view of it, so an ill-formed payload is refused at construction and never \
                 produced"
            ),
            Self::TooLong { bytes } => write!(
                f,
                "{bytes} bytes is longer than the {MAX_STRING_BYTES}-byte literal \
                 `source.string.v1` models; a longer payload is refused rather than truncated"
            ),
        }
    }
}

/// Decode one quoted token's raw content into the payload the language means by
/// it.
///
/// The **semantic byte sequence** of amendment §3: the decoded UTF-8 payload,
/// excluding any C implementation terminator. `\0` is an ordinary escape here
/// and produces U+0000, which is well-formed UTF-8 — that is why an embedded NUL
/// is a value this route can hold, and why no operation on it may call `strlen`.
pub fn decode_string_literal(raw: &str) -> Result<Vec<u8>, LiteralRefusal> {
    let source = raw.as_bytes();
    let mut bytes = Vec::with_capacity(source.len());
    let mut cursor = 0usize;
    while cursor < source.len() {
        if source[cursor] != b'\\' {
            bytes.push(source[cursor]);
            cursor += 1;
            continue;
        }
        let at = cursor;
        let Some(&escape) = source.get(cursor + 1) else {
            return Err(LiteralRefusal::TrailingBackslash);
        };
        cursor += 2;
        match escape {
            b'\\' => bytes.push(b'\\'),
            b'"' => bytes.push(b'"'),
            b'n' => bytes.push(b'\n'),
            b't' => bytes.push(b'\t'),
            b'r' => bytes.push(b'\r'),
            b'0' => bytes.push(0),
            b'x' => {
                let digits = source.get(cursor..cursor + 2);
                let Some(digits) = digits.and_then(|digits| std::str::from_utf8(digits).ok())
                else {
                    return Err(LiteralRefusal::TruncatedHexEscape { at });
                };
                let Ok(value) = u8::from_str_radix(digits, 16) else {
                    return Err(LiteralRefusal::TruncatedHexEscape { at });
                };
                bytes.push(value);
                cursor += 2;
            }
            other => {
                return Err(LiteralRefusal::UnknownEscape {
                    at,
                    spelling: String::from_utf8_lossy(&[other]).into_owned(),
                });
            }
        }
    }
    if bytes.len() > MAX_STRING_BYTES as usize {
        return Err(LiteralRefusal::TooLong { bytes: bytes.len() });
    }
    // Amendment §3.4, enforced at the only point where bytes enter the model.
    if let Err(error) = std::str::from_utf8(&bytes) {
        return Err(LiteralRefusal::NotUtf8 {
            at: error.valid_up_to(),
        });
    }
    Ok(bytes)
}

/// Whether byte offset `at` is a UTF-8 code-point boundary of `bytes`.
///
/// The whole boundary rule, in one place, so a low bound and a high bound cannot
/// be decided differently. `at == bytes.len()` is a boundary — the empty slice
/// at `n:n` is valid (§4) — and a continuation byte (`0b10xxxxxx`) never is.
#[must_use]
pub fn is_code_point_boundary(bytes: &[u8], at: usize) -> bool {
    if at == bytes.len() {
        return true;
    }
    match bytes.get(at) {
        Some(byte) => (byte & 0b1100_0000) != 0b1000_0000,
        Option::None => false,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StringError {
    /// `Q068`: the flat entry shape this profile models is violated.
    Shape { event: usize, reason: String },
    /// `Q069`: a bound that is not a check-time constant this entry bound, or a
    /// view position naming something this entry bound no string for.
    NotAScalar { event: usize, name: String },
    /// `Q069`: a view position naming something this entry bound no string for.
    NotAString { event: usize, name: String },
    /// `Q070`: a quoted token this route cannot decode into a payload.
    Literal {
        event: usize,
        refusal: LiteralRefusal,
    },
    /// `Q071`: `0 <= low <= high <= len(s)` is violated.
    OutOfRange {
        event: usize,
        name: String,
        low: i32,
        high: i32,
        len: u32,
    },
    /// `Q071`: a bound that is in range but is not a UTF-8 code-point boundary.
    /// Refused, **never rounded** — amendment §2 forbids silently moving an
    /// index — and never allowed to produce ill-formed text.
    NotABoundary {
        event: usize,
        name: String,
        at: u32,
        len: u32,
    },
    /// `Q072`: a string at a position this route refuses to let it cross —
    /// today, a rendering operand.
    CrossesBoundary { event: usize, name: String },
    /// `Q073`: the declared `effects { … }` row does not admit a derived effect.
    EffectNotDeclared { event: usize, effect: Effect },
    /// `Q076`: a text-line operand naming something this entry bound no string
    /// for (card `STRING-SLICE-IO`).
    ///
    /// Deliberately **this composition's own code** rather than the base route's
    /// `Q069`. The two conditions read the same from a distance and are not the
    /// same: `Q069` is a *view position* inside the string vocabulary — a
    /// slice's source, a `len`'s argument — whereas this is the output
    /// vocabulary's operand position, where a reader who wrote `print x;` for an
    /// integer `x` wants to be told to name a rendering rather than to be told
    /// that `x` is not a string.
    NotRenderableText { event: usize, name: String },
}

impl StringError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Shape { .. } => "Q068",
            Self::NotAScalar { .. } | Self::NotAString { .. } => "Q069",
            Self::Literal { .. } => "Q070",
            Self::OutOfRange { .. } | Self::NotABoundary { .. } => "Q071",
            Self::CrossesBoundary { .. } => "Q072",
            Self::EffectNotDeclared { .. } => "Q073",
            Self::NotRenderableText { .. } => "Q076",
        }
    }

    pub fn event(&self) -> usize {
        match self {
            Self::Shape { event, .. }
            | Self::NotAScalar { event, .. }
            | Self::NotAString { event, .. }
            | Self::Literal { event, .. }
            | Self::OutOfRange { event, .. }
            | Self::NotABoundary { event, .. }
            | Self::CrossesBoundary { event, .. }
            | Self::EffectNotDeclared { event, .. }
            | Self::NotRenderableText { event, .. } => *event,
        }
    }
}

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape { reason, .. } => f.write_str(reason),
            Self::NotAScalar { name, .. } => write!(
                f,
                "`{name}` is not a check-time-constant integer this entry bound; \
                 `source.string.v1` bounds a slice with an integer literal or a name bound to \
                 one, so that every slice is decided before emission and v1 needs no run-time \
                 failure arm"
            ),
            Self::NotAString { name, .. } => write!(
                f,
                "`{name}` is not a string this entry bound; `source.string.v1` names a string \
                 only at a slice's source and a `len`'s argument, and every such position must \
                 name one"
            ),
            Self::Literal { refusal, .. } => write!(f, "{refusal}"),
            Self::OutOfRange {
                name,
                low,
                high,
                len,
                ..
            } => write!(
                f,
                "`{name}[{low}:{high}]` is not a slice of a {len}-byte string; \
                 `source.string.v1` admits a slice only when `0 <= low <= high <= len(s)`, and \
                 decides it at check time, so this is a refusal and not a run-time trap"
            ),
            Self::NotABoundary { name, at, len, .. } => write!(
                f,
                "byte {at} is inside a UTF-8 code point of `{name}` ({len} bytes), not at a code-\
                 point boundary; `source.string.v1` refuses such a bound rather than rounding it, \
                 because rounding would silently slice a different string and not rounding would \
                 produce ill-formed text"
            ),
            Self::CrossesBoundary { name, .. } => write!(
                f,
                "`{name}` is a string and cannot cross here; a rendering operand is a scalar, and \
                 this amendment does not make `std.io` or `std.logger` accept text \
                 (`STRING-SLICE-IO`). Passing a string to a declared extern callee is refused for \
                 a separate reason: no C04 pointer contract transports this aggregate or its \
                 extent, so the crossing would discard the length and provenance the view carries \
                 (`STRING-SLICE-ABI`)"
            ),
            Self::EffectNotDeclared { effect, .. } => write!(
                f,
                "this entry performs `{effect:?}` but its declared `effects {{ … }}` row does not \
                 admit it"
            ),
            Self::NotRenderableText { name, .. } => write!(
                f,
                "`{name}` is not a string this entry bound, and a bare rendering operand renders a \
                 string's own bytes; name a `std.fmt` rendering (`print dec {name};`) to render a \
                 scalar, or bind `{name}` to a string literal or a slice"
            ),
        }
    }
}

/// The derived effect row.
///
/// **Literal construction, `len` and a successful slice derive nothing** — that
/// is amendment §7 stated as code, and it is the operational content of
/// "zero-allocation, zero-copy": a route that could allocate would derive
/// `Alloc`, and this one has no event that does. Only a line derives `Io`, and
/// that is `source.std-io.v1`'s effect, not this route's.
pub fn derived_effects(program: &StringProgram) -> EffectRow {
    let mut effects = BTreeSet::new();
    for event in &program.events {
        match event {
            StringEvent::Line(
                StdIoEvent::LogLevel { .. }
                | StdIoEvent::Print { .. }
                | StdIoEvent::Log { .. }
                | StdIoEvent::Flush,
            )
            // A rendered text line is a line: it derives `Io` and nothing
            // else, exactly as a rendered scalar line does. Rendering a view
            // reads static bytes and writes them; it allocates nothing and
            // mutates nothing, so amendment §7's "no `Alloc` or `Mut`" survives
            // the composition unchanged.
            | StringEvent::Text { .. } => {
                effects.insert(Effect::Io);
            }
            StringEvent::Bind { .. }
            | StringEvent::Literal { .. }
            | StringEvent::Slice { .. }
            | StringEvent::Len { .. }
            | StringEvent::Line(StdIoEvent::Return)
            | StringEvent::Return => {}
        }
    }
    EffectRow::new(effects)
}

/// The state the check carries: which names are views of which storage, and what
/// each scalar name denotes.
struct Evaluation<'a> {
    literals: &'a [Vec<u8>],
    views: BTreeMap<String, StringViewRecord>,
    scalars: BTreeMap<String, i32>,
}

impl Evaluation<'_> {
    fn operand(&self, event: usize, operand: &StringOperand) -> Result<i32, StringError> {
        match operand {
            StringOperand::Literal(value) => Ok(*value),
            StringOperand::Scalar(name) => {
                if self.views.contains_key(name) {
                    // A string at a scalar position. Reported as the boundary
                    // refusal rather than the generic one, because the reader
                    // needs to know *why* a name they bound is not usable here.
                    return Err(StringError::CrossesBoundary {
                        event,
                        name: name.clone(),
                    });
                }
                self.scalars
                    .get(name)
                    .copied()
                    .ok_or_else(|| StringError::NotAScalar {
                        event,
                        name: name.clone(),
                    })
            }
        }
    }

    /// The bytes a view addresses. Total for a view this evaluation holds,
    /// because every record was built from a decoded literal and a checked
    /// slice of one.
    fn bytes(&self, view: StringViewRecord) -> &[u8] {
        let payload = &self.literals[view.literal];
        &payload[view.offset as usize..(view.offset + view.len) as usize]
    }
}

/// §2's admission condition for one slice, whole and in order, returning the
/// source view, the derived view and the two decided bounds.
///
/// **The one place a slice is decided**, so a slice written inline at a
/// rendering operand (card `STRING-SLICE-IO`) cannot be admitted where the same
/// slice written as a `let` binding would be refused, or refused where it would
/// be admitted. Every refusal it can return is one of the base route's, at the
/// base route's own code: composing the route added no new way for a slice to
/// fail.
fn admit_slice(
    evaluation: &Evaluation<'_>,
    event: usize,
    name: &str,
    low: &StringOperand,
    high: &StringOperand,
) -> Result<(StringViewRecord, StringViewRecord, u32, u32), Vec<StringError>> {
    let Some(source) = evaluation.views.get(name).copied() else {
        return Err(vec![StringError::NotAString {
            event,
            name: name.to_owned(),
        }]);
    };
    let low = evaluation.operand(event, low);
    let high = evaluation.operand(event, high);
    let (low, high) = match (low, high) {
        (Ok(low), Ok(high)) => (low, high),
        (low, high) => {
            let mut refusals = Vec::new();
            refusals.extend(low.err());
            refusals.extend(high.err());
            return Err(refusals);
        }
    };
    // Both halves are checked before either is used as an offset, so no
    // arithmetic below can be out of range. `source.len` is at most
    // `MAX_STRING_BYTES`, so the conversion is exact and the comparison is the
    // one §2 writes.
    let length = i32::try_from(source.len).expect("a length is bounded by 4096");
    if low < 0 || high < low || high > length {
        return Err(vec![StringError::OutOfRange {
            event,
            name: name.to_owned(),
            low,
            high,
            len: source.len,
        }]);
    }
    let bytes = evaluation.bytes(source);
    let low = low as u32;
    let high = high as u32;
    let mut refusals = Vec::new();
    for at in [low, high] {
        if !is_code_point_boundary(bytes, at as usize) {
            refusals.push(StringError::NotABoundary {
                event,
                name: name.to_owned(),
                at,
                len: source.len,
            });
        }
    }
    if !refusals.is_empty() {
        return Err(refusals);
    }
    // §4, verbatim: `{ ptr = p + low, len = high - low }`. The literal index is
    // carried through unchanged, which is the statement that the derived view
    // addresses the *same* storage.
    let derived = StringViewRecord {
        literal: source.literal,
        offset: source.offset + low,
        len: high - low,
    };
    Ok((source, derived, low, high))
}

/// Resolve a text-line operand to the view whose bytes the line writes (card
/// `STRING-SLICE-IO`).
///
/// An inline slice records its arithmetic in `slices` and its provenance in
/// `views`, exactly as a bound one does, so the certificate's zero-copy evidence
/// covers a rendering operand as completely as it covers a binding. It binds no
/// name in the evaluation: the view exists for the duration of the line and a
/// later statement cannot name it.
fn text_view(
    evaluation: &Evaluation<'_>,
    event: usize,
    operand: &TextOperand,
    slices: &mut Vec<SliceRecord>,
    views: &mut Vec<(String, StringViewRecord)>,
) -> Result<StringViewRecord, StringError> {
    match operand {
        TextOperand::Literal(literal) => {
            let Some(payload) = evaluation.literals.get(*literal) else {
                return Err(StringError::Shape {
                    event,
                    reason: "this rendered literal names no stored payload".to_owned(),
                });
            };
            let record = StringViewRecord {
                literal: *literal,
                offset: 0,
                // `as` is exact: the payload was bounded at decoding.
                len: payload.len() as u32,
            };
            views.push((text_slice_name(event), record));
            Ok(record)
        }
        TextOperand::Name(name) => match evaluation.views.get(name).copied() {
            Some(view) => Ok(view),
            // The reader wrote a bare operand, which is the *text* form, so the
            // refusal is this composition's own and names the two repairs.
            Option::None => Err(StringError::NotRenderableText {
                event,
                name: name.clone(),
            }),
        },
        TextOperand::Slice { name, low, high } => {
            match admit_slice(evaluation, event, name, low, high) {
                Ok((from, derived, low, high)) => {
                    let binding = text_slice_name(event);
                    views.push((binding.clone(), derived));
                    slices.push(SliceRecord {
                        event,
                        source: name.clone(),
                        binding,
                        from,
                        low,
                        high,
                        derived,
                    });
                    Ok(derived)
                }
                // `admit_slice` reports every refusal of a slice; a text line
                // adds none, so the first is the one to raise. The others are
                // the same statement's and would name the same span.
                Err(refusals) => Err(refusals
                    .into_iter()
                    .next()
                    .expect("a refused slice reports at least one refusal")),
            }
        }
    }
}

/// Check a lowered program and derive its certificate.
///
/// Unlike [`crate::std_buffer::check_buffer_program`] there is no custody half:
/// a view is not a resource (see the module header), so there is no ownership
/// operation list, no leak rule and nothing for the cycle-3 checker to decide.
/// What is here instead is the **storage-provenance** half: every view records
/// the static literal it addresses, and a slice is admitted only against that
/// literal's own bytes.
pub fn check_string_program(
    program: &StringProgram,
) -> Result<StringCertificate, Vec<StringError>> {
    let mut errors = Vec::new();

    for (index, payload) in program.literals.iter().enumerate() {
        if payload.len() > MAX_STRING_BYTES as usize {
            errors.push(StringError::Literal {
                event: program.events.len(),
                refusal: LiteralRefusal::TooLong {
                    bytes: payload.len(),
                },
            });
        }
        if let Err(error) = std::str::from_utf8(payload) {
            // The lowering decodes and validates, so reaching this is a defect
            // in the lowering rather than in the program. It is checked again
            // here because the model must not be able to hold an ill-formed
            // payload however it was constructed.
            errors.push(StringError::Literal {
                event: index,
                refusal: LiteralRefusal::NotUtf8 {
                    at: error.valid_up_to(),
                },
            });
        }
    }

    let mut evaluation = Evaluation {
        literals: &program.literals,
        views: BTreeMap::new(),
        scalars: BTreeMap::new(),
    };
    let mut views = Vec::new();
    let mut slices = Vec::new();
    let mut lines = Vec::new();
    let mut flushes = 0usize;
    let mut renderings = BTreeSet::new();
    let mut returned = false;

    if !errors.is_empty() {
        return Err(errors);
    }

    for (index, event) in program.events.iter().enumerate() {
        if returned {
            errors.push(StringError::Shape {
                event: index,
                reason: "this entry continues after its return; `source.string.v1` models one \
                         flat sequence ending in exactly one return"
                    .to_owned(),
            });
            break;
        }
        match event {
            StringEvent::Bind { binding, value } => {
                evaluation.scalars.insert(binding.clone(), *value);
            }
            StringEvent::Literal { binding, literal } => {
                let Some(payload) = program.literals.get(*literal) else {
                    errors.push(StringError::Shape {
                        event: index,
                        reason: "this literal view names no stored payload".to_owned(),
                    });
                    continue;
                };
                let record = StringViewRecord {
                    literal: *literal,
                    offset: 0,
                    // `as` is exact: the payload was bounded above.
                    len: payload.len() as u32,
                };
                evaluation.views.insert(binding.clone(), record);
                views.push((binding.clone(), record));
            }
            StringEvent::Slice {
                binding,
                name,
                low,
                high,
            } => {
                let (source, derived, low, high) =
                    match admit_slice(&evaluation, index, name, low, high) {
                        Ok(admitted) => admitted,
                        Err(refusals) => {
                            errors.extend(refusals);
                            continue;
                        }
                    };
                evaluation.views.insert(binding.clone(), derived);
                views.push((binding.clone(), derived));
                slices.push(SliceRecord {
                    event: index,
                    source: name.clone(),
                    binding: binding.clone(),
                    from: source,
                    low,
                    high,
                    derived,
                });
            }
            StringEvent::Len { binding, name } => match evaluation.views.get(name).copied() {
                // §3.1: the exact byte count, available without scanning. It is
                // read out of the view record itself, which is the model's
                // `{ptr, len}` pair, and there is no second place it could come
                // from.
                Some(view) => {
                    evaluation.scalars.insert(binding.clone(), view.len as i32);
                }
                Option::None => errors.push(StringError::NotAString {
                    event: index,
                    name: name.clone(),
                }),
            },
            StringEvent::Line(line) => match line {
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
                StdIoEvent::Return => errors.push(StringError::Shape {
                    event: index,
                    reason: "a return is this route's own event, not a standard-IO one".to_owned(),
                }),
            },
            // Card `STRING-SLICE-IO`. The view is resolved by the same rules
            // every other view position uses, the §4 arithmetic of an inline
            // slice is recorded in the same list a bound slice's is, and the
            // line's bytes are decided by `source.std-io.v1`'s own framing.
            StringEvent::Text { level, operand } => {
                match text_view(&evaluation, index, operand, &mut slices, &mut views) {
                    Ok(view) => {
                        let bytes = evaluation.bytes(view);
                        match std::str::from_utf8(bytes) {
                            Ok(text) => lines.push(rendered_text_line(*level, text)),
                            // Unreachable for a view this evaluation holds: a
                            // payload is validated at decoding and every
                            // admitted bound is a code-point boundary. Checked
                            // rather than asserted, because the alternative is
                            // publishing an observation whose bytes no rule
                            // covered.
                            Err(error) => errors.push(StringError::Literal {
                                event: index,
                                refusal: LiteralRefusal::NotUtf8 {
                                    at: error.valid_up_to(),
                                },
                            }),
                        }
                    }
                    Err(error) => errors.push(error),
                }
            }
            StringEvent::Return => returned = true,
        }
    }

    if !returned {
        errors.push(StringError::Shape {
            event: program.events.len(),
            reason: "this entry never returns; `source.string.v1` requires exactly one return"
                .to_owned(),
        });
    }

    let derived = derived_effects(program);
    for effect in derived.iter() {
        if !program.declared_effects.contains(effect) {
            errors.push(StringError::EffectNotDeclared {
                event: program.events.len(),
                effect: effect.clone(),
            });
        }
    }

    if errors.is_empty() {
        Ok(StringCertificate {
            entry: program.entry.clone(),
            effects: derived,
            literals: program.literals.clone(),
            views,
            slices,
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
/// A `borrow NAME` rendering refuses for the structural reason `STD-FMT-BORROW`
/// established: this route publishes no custody identity — it has none to
/// publish, because a view is not a resource.
fn line_operand(
    evaluation: &Evaluation<'_>,
    event: usize,
    operand: &StdIoOperand,
) -> Result<i32, StringError> {
    match operand {
        StdIoOperand::Literal(value) => Ok(*value),
        StdIoOperand::Scalar(name) => {
            evaluation.operand(event, &StringOperand::Scalar(name.clone()))
        }
        StdIoOperand::Borrowed(name) => Err(StringError::CrossesBoundary {
            event,
            name: name.clone(),
        }),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StringRecheckFailure {
    Entry,
    Effects,
    Literals,
    Views,
    Slices,
    Scalars,
    Lines,
    Flushes,
    Renderings,
    Refused,
}

impl fmt::Display for StringRecheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Self::Entry => "the entry name",
            Self::Effects => "the derived effect row",
            Self::Literals => "the stored literal payloads",
            Self::Views => "the views and the storage they address",
            Self::Slices => "the admitted slices and their arithmetic",
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
pub fn recheck_string_certificate(
    program: &StringProgram,
    certificate: &StringCertificate,
) -> Result<(), StringRecheckFailure> {
    let rederived = check_string_program(program).map_err(|_| StringRecheckFailure::Refused)?;
    if rederived.entry != certificate.entry {
        return Err(StringRecheckFailure::Entry);
    }
    if rederived.effects != certificate.effects {
        return Err(StringRecheckFailure::Effects);
    }
    if rederived.literals != certificate.literals {
        return Err(StringRecheckFailure::Literals);
    }
    if rederived.views != certificate.views {
        return Err(StringRecheckFailure::Views);
    }
    if rederived.slices != certificate.slices {
        return Err(StringRecheckFailure::Slices);
    }
    if rederived.scalars != certificate.scalars {
        return Err(StringRecheckFailure::Scalars);
    }
    if rederived.lines != certificate.lines {
        return Err(StringRecheckFailure::Lines);
    }
    if rederived.flushes != certificate.flushes {
        return Err(StringRecheckFailure::Flushes);
    }
    if rederived.renderings != certificate.renderings {
        return Err(StringRecheckFailure::Renderings);
    }
    Ok(())
}
