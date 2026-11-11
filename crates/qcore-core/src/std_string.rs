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

use crate::ownership::{self, OwnershipError, OwnershipOp};
use crate::source_loop::LoopRefusal;
use crate::std_buffer::{
    BufferAccess, BufferError, BufferEvaluation, BufferEvent, BufferOperand, BufferProgram,
    BufferRun,
};
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

/// The certificate-side name of the view a **concat piece** derives, when that
/// piece is an inline slice (card `STRING-CONCAT`).
///
/// Distinct per piece as well as per event, because one `concat` may write
/// several inline slices and each is its own checked derivation with its own
/// [`SliceRecord`].
#[must_use]
pub fn concat_piece_name(event: usize, piece: usize) -> String {
    format!("qcore_concat_{event}_{piece}")
}

/// What one piece of a `concat` may be (card `STRING-CONCAT`).
///
/// **Every form is check-time-known, and that is the whole design.** The
/// composed payload is therefore a byte sequence the compiler computes, and it
/// becomes static storage of the emitted program — so a concat allocates
/// nothing, copies nothing at run time, and derives neither `Alloc` nor `Mut`,
/// exactly as amendment §7 requires of every string operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StringPiece {
    /// A quoted literal, a string this entry bound, or one inline slice — the
    /// same three forms a rendered text operand takes, resolved by the same
    /// function, so a piece cannot be admitted where the same spelling at a
    /// rendering operand would be refused.
    Text(TextOperand),
    /// `dec(NAME)` / `hex(NAME)` — the **text of a check-time-constant
    /// integer**, byte-identical to what `print dec NAME;` would write, because
    /// it is produced by [`Rendering::render`] and by nothing else.
    ///
    /// This is the one direction of scalar/text conversion v1 has, and it is
    /// the piece that makes a **computed `Content-Length`** expressible: the
    /// number is `len(content)`, which this route already decides.
    Scalar { rendering: Rendering, name: String },
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
    /// `let NAME = concat(PIECE, …);` — **text this program computes rather
    /// than writes** (card `STRING-CONCAT`).
    ///
    /// Every piece is check-time-known, so the checker computes the composed
    /// bytes and the result becomes a static payload of the emitted program.
    /// The alternative — materialising into an owned buffer at run time — was
    /// measured and rejected: it would add an allocation, a copy and a custody
    /// obligation to an operation whose result the compiler already holds, and
    /// it would make `len` of the result a run-time quantity, which is the one
    /// property this operation exists to preserve.
    Concat {
        binding: String,
        pieces: Vec<StringPiece>,
    },
    /// `let NAME = borrow_utf8(BUFFER);` — amendment §5's first operation
    /// (card `STRING-CONCAT`).
    ///
    /// Validates the buffer's bytes as UTF-8 **at check time** — v1's buffers
    /// hold only constant bytes written at constant indices, so their contents
    /// are a check-time fact — and yields a view of that buffer's storage. The
    /// view creates and discharges no custody; what it does create is a
    /// **window** in which the buffer may not be mutated or released, ending at
    /// the last statement that reads the view or anything derived from it.
    ///
    /// `range` is card `CT07-SLICE-PERMISSIONS`'s addition: `None` is the
    /// whole-extent form `borrow_utf8(BUFFER)` this route has always had, and
    /// `Some((low, high))` is `borrow_utf8(BUFFER, LOW, HIGH)`, a **sub-extent**
    /// loan. The two are one construct rather than two: a whole-extent borrow
    /// *is* the range `[0, extent)`, so the window, the UTF-8 decision and the
    /// exclusion rule are stated once, over a range, and the pre-card spelling
    /// is their degenerate case. What the range buys is that the exclusion
    /// becomes **range-wise**: a write into the borrowed bytes is refused and a
    /// write outside them is admitted, which is the first form in this language
    /// that lends part of a resource rather than all of it.
    Borrow {
        binding: String,
        buffer: String,
        range: Option<(StringOperand, StringOperand)>,
    },
    /// A `source.buffer.v1` statement, embedded unchanged (card
    /// `STRING-CONCAT`). Its bytes, bounds, extent and custody are decided by
    /// [`crate::std_buffer`]'s own published rules, so a composed module cannot
    /// admit a byte access that route would refuse.
    Buffer(BufferEvent),
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
    /// The entry row's float comparison effects (card
    /// `W48-QR-FLOAT-BUFFER-PAYLOADS`), which the embedded `source.buffer.v1`
    /// value statements are decided under. Empty for every program before
    /// the card.
    pub float_effects: crate::scalar_types::FloatEffects,
}

/// Where a view's bytes live (card `STRING-CONCAT`).
///
/// Before that card there was one answer and the field was spelled `literal`.
/// There are now three, and the distinction is exactly amendment §3.6's: the
/// first two are **static storage of this program**, which cannot end, and the
/// third is storage the program **owns and releases**, which can — so a view of
/// it carries a borrow window and a view of the others does not.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StringStorage {
    /// A `static const` payload the source spelled: an index into
    /// [`StringProgram::literals`].
    Literal(usize),
    /// A `static const` payload the **compiler composed** from check-time-known
    /// pieces: an index into [`StringCertificate::composed`]. A concat result
    /// is storage of exactly the same kind a literal is — static, immutable and
    /// unable to end — which is the whole reason the static-composition route
    /// needs no allocation, no custody and no lifetime rule.
    Composed(usize),
    /// The bytes of an owned `std.buffer` this entry holds, named by its
    /// binding (card `STRING-CONCAT`'s `borrow_utf8`). **The only storage that
    /// can end**, and therefore the only one a borrow window is about.
    Buffer(String),
}

/// One view, as the certificate publishes it: **which storage it addresses,
/// where in that storage it starts, and how long it is.**
///
/// This triple is the `{ptr, len}` of amendment §3 with the provenance made
/// explicit. `storage` is the answer to "what storage does this view address",
/// which is what makes §3.6 — a derived view cannot outlive the storage it was
/// derived from — a checkable statement rather than a promise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringViewRecord {
    /// The storage this view addresses.
    pub storage: StringStorage,
    /// Byte offset of this view's first byte within that storage.
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
    /// The derived view. `derived.storage == from.storage` and
    /// `derived.offset == from.offset + low` hold for every record — that
    /// equality *is* the zero-copy claim, and it is a field a reader can check
    /// rather than a sentence they must believe.
    pub derived: StringViewRecord,
}

/// One admitted `concat`, published so a reader sees the **length arithmetic**
/// rather than being told it happened (card `STRING-CONCAT`).
///
/// `result.len == piece_lens.iter().sum()` holds for every record, and that
/// equation is the operational content of "a computed `Content-Length` is
/// decidable": a length the compiler derived from pieces is a check-time
/// constant exactly as a literal's own length is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConcatRecord {
    pub event: usize,
    pub binding: String,
    /// The byte length each piece contributed, in written order.
    pub piece_lens: Vec<u32>,
    /// The composed view. Its storage is always
    /// [`StringStorage::Composed`]: the result is a new static payload, never a
    /// window onto a piece.
    pub result: StringViewRecord,
}

/// One admitted `borrow_utf8`, published with the **window** over which the
/// borrowed buffer may not be mutated or released (card `STRING-CONCAT`).
///
/// `through` is the index of the last event that reads this view or any view
/// derived from it. A buffer store or release at an index in `[event, through]`
/// is refused; one after it is admitted. That interval is exact rather than
/// conservative because the route is a flat statement sequence with no branch
/// and no loop — the borrow's extent *is* a range of statement indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BorrowRecord {
    pub event: usize,
    pub buffer: String,
    pub binding: String,
    pub through: usize,
    /// The **first byte** of the buffer this loan covers (card
    /// `CT07-SLICE-PERMISSIONS`). `0` for the whole-extent form.
    pub low: u32,
    /// One past the **last byte** this loan covers. The buffer's extent for the
    /// whole-extent form, which is why that form is the degenerate case of this
    /// one rather than a second rule: `low == 0 && high == extent`.
    pub high: u32,
    /// The view the borrow yields: the borrowed range, validated as UTF-8.
    /// `view.offset == low` and `view.len == high - low` hold for every record,
    /// so a reader checks the loan's footprint against the view it produced
    /// rather than being told they agree.
    pub view: StringViewRecord,
}

/// **Do two half-open byte ranges share a byte?** (card
/// `CT07-SLICE-PERMISSIONS`.)
///
/// `[low_a, high_a)` and `[low_b, high_b)` intersect exactly when
/// `max(low_a, low_b) < min(high_a, high_b)`. This is an **equality with the
/// set-theoretic question**, not an approximation of it — every bound on this
/// route is a check-time constant, so "provably disjoint" and "disjoint" are
/// the same predicate and there is no third answer to fall closed to.
/// `QCore.SlicePermissions.overlaps_iff_a_shared_byte` is that statement.
///
/// An **empty** range shares no byte with anything, itself included: `low >= high`
/// makes the maximum at least the minimum. That is a consequence rather than a
/// special case, and it is why `borrow_utf8(b, 2, 2)` excludes no write.
///
/// The one-byte write `b[i] = v` enters here as `[i, i + 1)`, which is what
/// makes "this write is inside that loan" one question rather than two.
#[must_use]
pub fn ranges_overlap(low_a: u32, high_a: u32, low_b: u32, high_b: u32) -> bool {
    low_a.max(low_b) < high_a.min(high_b)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringCertificate {
    pub entry: String,
    /// The derived row, recomputed by the recheck and compared.
    pub effects: EffectRow,
    /// The decoded literal payloads this program stores statically.
    pub literals: Vec<Vec<u8>>,
    /// The payloads the **compiler composed**, in program order — one per
    /// admitted `concat` (card `STRING-CONCAT`). Published as bytes, so a
    /// reader checks the computed text itself rather than trusting that it was
    /// computed.
    pub composed: Vec<Vec<u8>>,
    /// Every view this program binds, in program order.
    pub views: Vec<(String, StringViewRecord)>,
    /// Every admitted slice, in program order.
    pub slices: Vec<SliceRecord>,
    /// Every admitted `concat`, with its length arithmetic.
    pub concats: Vec<ConcatRecord>,
    /// Every admitted `borrow_utf8`, with the window it excludes mutation over.
    pub borrows: Vec<BorrowRecord>,
    /// `(name, extent)` for every buffer this composed entry acquires, in
    /// program order — `source.buffer.v1`'s own extent transport, republished
    /// here because a composed module holds those resources.
    pub acquisitions: Vec<(String, u32)>,
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
    /// Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-TYPES-COMPOSED-STRING`): the
    /// final environment of every scalar of a §G02t type other than `i32` —
    /// a conversion or value statement of the composed entry. Empty for
    /// every program before the card.
    pub typed: BTreeMap<String, (crate::scalar_types::ScalarType, i128)>,
    /// The recorded target, exactly when the program names `isize` or
    /// `usize` — a named assumption of the result, as on `source.buffer.v1`.
    pub target_assumption: Option<crate::scalar_types::RecordedTarget>,
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
    /// `Q077`: the **shape or the bound** of a construct this pass's third band
    /// adds (card `STRING-CONCAT`) — a `concat` with fewer than two pieces, a
    /// composed payload longer than [`MAX_STRING_BYTES`], or an embedded
    /// `source.buffer.v1` statement whose own model refused its shape.
    Composition { event: usize, reason: String },
    /// `Q077`: a bridge position naming something this entry acquired no buffer
    /// for. Deliberately **this band's own code** and not `source.buffer.v1`'s
    /// `Q052`: that code belongs to a second semantic pass, and §L21 forbids a
    /// pass taking another's code even where the condition reads alike.
    NotABuffer { event: usize, name: String },
    /// `Q077`: the **bound** of a sub-extent borrow (card
    /// `CT07-SLICE-PERMISSIONS`) — `0 <= low <= high <= extent` is violated.
    ///
    /// This band's own shape-or-bound code, and deliberately **not** `Q071`:
    /// that condition is a bound on a *view*, stated in the view's own
    /// coordinates and decided against `len(s)`, whereas this one is a bound on
    /// an owned **buffer**, decided against the extent its acquisition
    /// declared. The two read alike from a distance and name different things,
    /// which is the `Q076`-rather-than-`Q069` precedent (`STRING-SLICE-IO`) and
    /// the `Q079`-rather-than-`Q070` precedent (`STRING-CONCAT`).
    BorrowRange {
        event: usize,
        buffer: String,
        low: i32,
        high: i32,
        extent: u32,
    },
    /// `Q078`: the **borrow discipline** — a store into, or a release of, a
    /// buffer inside a live borrow's window.
    ///
    /// Since card `CT07-SLICE-PERMISSIONS` the store arm is **range-wise**: it
    /// reports a write whose byte lies inside the loan's `[low, high)`, and a
    /// write outside every live loan of that buffer is admitted. The release
    /// arm is not range-wise and must not become one — a release ends the whole
    /// storage, so it conflicts with every live loan however narrow.
    BorrowExclusion {
        event: usize,
        buffer: String,
        view: String,
        through: usize,
        /// What the offending statement did: `"write a byte of"` or
        /// `"release"`.
        action: &'static str,
        /// The loan's footprint, and whether it covers the buffer's whole
        /// extent. A whole-extent loan reports the message this route reported
        /// before sub-extent lending existed, byte for byte.
        low: u32,
        high: u32,
        whole: bool,
    },
    /// `Q078`: a refusal reported by the **inherited, unedited** ownership
    /// checker over this composed entry's buffer operations — a leak, a double
    /// release, a use after release, or a rebound live name. It carries that
    /// checker's own error value rather than a re-worded copy.
    Custody { event: usize, error: OwnershipError },
    /// `Q097`–`Q099`: a refusal the **bounded counted loop** owns, reached from
    /// this composed route since card `LOOPS-ON-COMPOSED-ROUTES` admitted a
    /// `for` here. Carried as [`LoopRefusal`]'s own value and reported at
    /// that construct's own codes — the `BufferError::Loop` precedent, for the
    /// same §L21 reason: `source.loop.v1` is a separate semantic pass, its
    /// conditions are the loop's (a bound, a shadowing body binding), and a
    /// second pass may not reuse a code. No condition of that band is
    /// widened; the composed route reaches the same condition by the same
    /// runner (`BufferRun::repeat`).
    Loop { event: usize, refusal: LoopRefusal },
    /// `Q079`: **the static UTF-8 decision.** The bytes a `borrow_utf8` would
    /// view are not well-formed UTF-8, decided at check time from the buffer's
    /// own write history rather than trusted or deferred to a run-time scan.
    BufferNotUtf8 {
        event: usize,
        buffer: String,
        at: usize,
    },
}

impl StringError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Loop { refusal, .. } => refusal.code(),
            Self::Shape { .. } => "Q068",
            Self::NotAScalar { .. } | Self::NotAString { .. } => "Q069",
            Self::Literal { .. } => "Q070",
            Self::OutOfRange { .. } | Self::NotABoundary { .. } => "Q071",
            Self::CrossesBoundary { .. } => "Q072",
            Self::EffectNotDeclared { .. } => "Q073",
            Self::NotRenderableText { .. } => "Q076",
            Self::Composition { .. } | Self::NotABuffer { .. } | Self::BorrowRange { .. } => "Q077",
            Self::BorrowExclusion { .. } | Self::Custody { .. } => "Q078",
            Self::BufferNotUtf8 { .. } => "Q079",
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
            | Self::NotRenderableText { event, .. }
            | Self::Composition { event, .. }
            | Self::NotABuffer { event, .. }
            | Self::BorrowRange { event, .. }
            | Self::BorrowExclusion { event, .. }
            | Self::Custody { event, .. }
            | Self::Loop { event, .. }
            | Self::BufferNotUtf8 { event, .. } => *event,
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
            Self::Composition { reason, .. } => f.write_str(reason),
            Self::NotABuffer { name, .. } => write!(
                f,
                "`{name}` is not an owned byte buffer this entry acquired; `borrow_utf8(NAME)` \
                 views the bytes of a `std.buffer` acquisition, and a string view has nothing to \
                 borrow from"
            ),
            Self::BorrowRange {
                buffer,
                low,
                high,
                extent,
                ..
            } => write!(
                f,
                "`borrow_utf8({buffer}, {low}, {high})` is not a range of `{buffer}`, whose \
                 declared extent is {extent} bytes; `source.string.v1` lends a sub-extent only \
                 when `0 <= low <= high <= extent`, and decides it at check time, so this is a \
                 refusal and not a run-time trap"
            ),
            Self::BorrowExclusion {
                buffer,
                view,
                through,
                action,
                low,
                high,
                whole,
                ..
            } => {
                write!(
                    f,
                    "this statement would {action} `{buffer}` while `{view}` borrows it; a \
                     `borrow_utf8` view is immutable UTF-8 for its whole lifetime, so the buffer \
                     it addresses may not be written or released until the last statement that \
                     reads the view or anything derived from it (statement {through} of this \
                     entry)"
                )?;
                // A whole-extent loan reports exactly the sentence this route
                // reported before sub-extent lending existed. A narrower one
                // says which bytes it holds, because the useful next question
                // for its reader is *which* write would have been admitted.
                if !whole {
                    match *action {
                        "release" => write!(
                            f,
                            "; `{view}` borrows only bytes [{low}, {high}) of `{buffer}`, but a \
                             release ends the whole storage, so it conflicts with every live \
                             borrow however narrow"
                        )?,
                        _ => write!(
                            f,
                            "; `{view}` borrows bytes [{low}, {high}) of `{buffer}` and this \
                             write is inside that range — a write outside every live borrow's \
                             range is admitted"
                        )?,
                    }
                }
                Ok(())
            }
            Self::Custody { error, .. } => write!(
                f,
                "the checked ownership discipline refuses this program: {error}"
            ),
            Self::Loop { refusal, .. } => refusal.fmt(f),
            Self::BufferNotUtf8 { buffer, at, .. } => write!(
                f,
                "the bytes of `{buffer}` are not well-formed UTF-8 from byte {at}; \
                 `borrow_utf8(NAME)` yields a `string`, which is a sequence of bytes known to be \
                 well-formed UTF-8 for the whole lifetime of every view of it, and this route \
                 decides that at check time out of the buffer's own write history rather than \
                 scanning at run time"
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
            // A composed buffer statement derives `source.buffer.v1`'s own
            // effects: an acquisition and a release are `Alloc`, a store is
            // `Mut`, a read observes. Read from that route's vocabulary rather
            // than restated, so the two rows cannot drift.
            // Card `AGGREGATE-OWNED-FIELD`: the field-path release and store
            // derive what their plain-name twins derive. This route refuses
            // a record statement before emission (`AGGREGATE-STRING-COMPOSE`),
            // and the row is derived before that refusal is reported, so
            // these arms are right rather than empty — the fail-closed
            // direction the `Repeat` arm below states.
            StringEvent::Buffer(
                BufferEvent::Acquire { .. }
                | BufferEvent::Release { .. }
                | BufferEvent::FieldRelease { .. },
            ) => {
                effects.insert(Effect::Alloc);
            }
            // Card `W48-QR-USABLE-RECORDS`: a field write derives what it
            // derives on `source.buffer.v1`, for the same refusing reason.
            StringEvent::Buffer(
                BufferEvent::Store { .. }
                | BufferEvent::FieldStore { .. }
                | BufferEvent::FieldWrite { .. },
            ) => {
                effects.insert(Effect::Mut);
            }
            // A composed `for` (the `LOOP-V1` × `STRING-CONCAT` composition,
            // admitted since card `LOOPS-ON-COMPOSED-ROUTES`). It derives the
            // body's effects through `source.buffer.v1`'s own recursive walk,
            // which is the fail-closed direction: under-deriving an effect
            // would let a declared row not have to admit one the program
            // performs. A loop performs no effect of its own — iterating is
            // not an effect — and is admitted only with a positive bound, so
            // the row cannot over-report either.
            StringEvent::Buffer(repeat @ BufferEvent::Repeat { .. }) => {
                let body = BufferProgram {
                    entry: String::new(),
                    declared_effects: EffectRow::default(),
                    // A comparison effect is no effect this walk derives.
                    float_effects: Default::default(),
                    // Card `AGGREGATES-V1`: this route models no record, so the
                    // synthetic program built to derive a refused loop's
                    // effects declares none. Nothing is lost by that — a record
                    // operation derives no effect at all.
                    records: Vec::new(),
                    events: vec![repeat.clone()],
                };
                for effect in crate::std_buffer::derived_effects(&body).iter() {
                    effects.insert(effect.clone());
                }
            }
            StringEvent::Buffer(BufferEvent::Line(
                StdIoEvent::LogLevel { .. }
                | StdIoEvent::Print { .. }
                | StdIoEvent::Log { .. }
                | StdIoEvent::Flush,
            )) => {
                effects.insert(Effect::Io);
            }
            // **`concat` and `borrow_utf8` derive nothing**, and that is the
            // static-composition decision stated as an effect fact: composing
            // text is a compile-time computation whose result becomes static
            // storage, and borrowing is a read. Amendment §7's "no `Alloc` or
            // `Mut` for a string operation" therefore survives this card
            // exactly; the `Alloc` a composed module can carry is the
            // *buffer's*, never the string's.
            StringEvent::Concat { .. }
            | StringEvent::Borrow { .. }
            | StringEvent::Bind { .. }
            | StringEvent::Literal { .. }
            | StringEvent::Slice { .. }
            | StringEvent::Len { .. }
            | StringEvent::Line(StdIoEvent::Return)
            | StringEvent::Buffer(
                BufferEvent::Bind { .. }
                | BufferEvent::Load { .. }
                | BufferEvent::FieldLoad { .. }
                | BufferEvent::FieldExtent { .. }
                | BufferEvent::Extent { .. }
                // Card `SLICE-PERMISSIONS-WRITE-LEND`: a lend derives nothing
                // — authority moves, no byte does. The composed route refuses
                // the statement itself (`Q077`), but the effect row is derived
                // before that refusal is reported, so this arm must be right
                // rather than absent.
                | BufferEvent::Lend { .. }
                // Card `AGGREGATES-V1`: a record operation derives **nothing**,
                // here for exactly the reason it derives nothing on the route
                // that admits it — a v1 record is a plain value in automatic
                // storage, so constructing one allocates nothing and reading a
                // field mutates nothing. This route refuses a record statement
                // before emission, but the effect row is derived from the
                // program *before* that refusal is reported, so this arm has to
                // be right rather than absent.
                | BufferEvent::Construct { .. }
                | BufferEvent::Field { .. }
                // Card `W48-QR-SCALAR-TYPES-R3`: a conversion derives nothing,
                // and this route lowers none (`buffer_use` does not claim the
                // shape), so the arm is total rather than reachable.
                | BufferEvent::Convert { .. }
                // Card `W48-QR-SCALAR-TYPES-R5`: a value statement derives
                // nothing either; this route refuses one at any depth.
                | BufferEvent::Compute { .. }
                // Card `OWNED-FIELD-ROUTE-REACH`: a copy bind derives nothing
                // — an admitted copy binds a value that already exists. This
                // route lowers none (`buffer_use` does not claim the shape),
                // so the arm is total rather than reachable.
                | BufferEvent::CopyBind { .. }
                | BufferEvent::Line(StdIoEvent::Return)
                | BufferEvent::Return,
            )
            | StringEvent::Return => {}
        }
    }
    EffectRow::new(effects)
}

/// The **custody half** of a composed module's check, and the whole of what it
/// inherits (card `STRING-CONCAT`).
///
/// Builds the cycle-3 ownership operation list from the embedded buffer events
/// and hands it to [`ownership::check_ownership`] unchanged, exactly as
/// [`crate::std_buffer::custody_operations`] does for the uncomposed route. The
/// one addition is that **a borrow is a read** of the buffer it views, so
/// borrowing a released buffer is `UseAfterDrop` from the inherited checker
/// rather than a rule this card wrote.
#[must_use]
pub fn string_custody_operations(program: &StringProgram) -> Vec<OwnershipOp> {
    let acquired: BTreeSet<&str> = program
        .events
        .iter()
        .filter_map(|event| match event {
            StringEvent::Buffer(BufferEvent::Acquire { name, .. }) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let names = |name: &String| acquired.contains(name.as_str());

    /// The operations of one `for` body, **unrolled** `count` times (card
    /// `LOOPS-ON-COMPOSED-ROUTES`) — `LOOP-V1`'s own reading, so a body store
    /// after a release is the inherited checker's `UseAfterDrop` exactly as a
    /// top-level one is. A body contributes accesses only: an acquisition, a
    /// release or a lend inside a body is refused by name before this list is
    /// decided (`LOOP-WINDOW-BODY-CUSTODY`, `LOOP-PER-ITERATION-LOAN`), and a
    /// direct model-level caller handing one in gets no operation for it,
    /// which cannot admit anything — a release the checker never saw leaves
    /// the owner live at the normal exit.
    fn unrolled(
        body: &[BufferEvent],
        count: u32,
        names: &dyn Fn(&String) -> bool,
        operations: &mut Vec<OwnershipOp>,
    ) {
        for _ in 0..count {
            for event in body {
                match event {
                    BufferEvent::Store { name, .. } if names(name) => {
                        operations.push(OwnershipOp::Write(name.clone()));
                    }
                    BufferEvent::Load { name, .. } | BufferEvent::Extent { name, .. }
                        if names(name) =>
                    {
                        operations.push(OwnershipOp::Read(name.clone()));
                    }
                    BufferEvent::Repeat { count, body, .. } => {
                        unrolled(body, *count, names, operations);
                    }
                    _ => {}
                }
            }
        }
    }

    let mut operations = Vec::new();
    let mut next_id: ownership::ResourceId = 1;
    for event in &program.events {
        match event {
            StringEvent::Buffer(BufferEvent::Acquire { name, extent }) => {
                operations.push(OwnershipOp::Acquire {
                    id: next_id,
                    owner: name.clone(),
                    resource_type: crate::std_buffer::BUFFER_RESOURCE_TYPE.to_owned(),
                    footprint: ownership::Footprint::single(
                        crate::std_buffer::BYTE_AXIS,
                        u64::from(*extent),
                    ),
                });
                next_id += 1;
            }
            StringEvent::Buffer(BufferEvent::Store { name, .. }) if names(name) => {
                operations.push(OwnershipOp::Write(name.clone()));
            }
            StringEvent::Buffer(
                BufferEvent::Load { name, .. } | BufferEvent::Extent { name, .. },
            ) if names(name) => {
                operations.push(OwnershipOp::Read(name.clone()));
            }
            StringEvent::Borrow { buffer, .. } if names(buffer) => {
                operations.push(OwnershipOp::Read(buffer.clone()));
            }
            StringEvent::Buffer(BufferEvent::Release { name }) if names(name) => {
                operations.push(OwnershipOp::Drop(name.clone()));
            }
            StringEvent::Buffer(BufferEvent::Repeat { count, body, .. }) => {
                unrolled(body, *count, &names, &mut operations);
            }
            _ => {}
        }
    }
    operations
}

/// Every **view name** one event reads.
///
/// The borrow window rests on this and nothing else, so it is written once: a
/// widening of the grammar that forgets to extend this function would shorten a
/// borrow's window, and the exhaustive `match` is what makes that a compile
/// error rather than an unsoundness.
fn view_uses(event: &StringEvent) -> Vec<&str> {
    fn operand_use(operand: &TextOperand) -> Option<&str> {
        match operand {
            TextOperand::Literal(_) => None,
            TextOperand::Name(name) => Some(name.as_str()),
            TextOperand::Slice { name, .. } => Some(name.as_str()),
        }
    }
    match event {
        StringEvent::Slice { name, .. } | StringEvent::Len { name, .. } => vec![name.as_str()],
        StringEvent::Text { operand, .. } => operand_use(operand).into_iter().collect(),
        StringEvent::Concat { pieces, .. } => pieces
            .iter()
            .filter_map(|piece| match piece {
                StringPiece::Text(operand) => operand_use(operand),
                StringPiece::Scalar { .. } => None,
            })
            .collect(),
        StringEvent::Bind { .. }
        | StringEvent::Literal { .. }
        | StringEvent::Borrow { .. }
        | StringEvent::Buffer(_)
        | StringEvent::Line(_)
        | StringEvent::Return => Vec::new(),
    }
}

/// Every **view name** one event reads **at any depth** (card
/// `LOOPS-ON-COMPOSED-ROUTES`) — the window function's reader.
///
/// Under the runs-many-times reading an event inside a `for` body occurs at
/// every iteration, and the loop is one top-level statement, so a view a body
/// read would be a view the **loop's** index reads.
///
/// **The depth contribution is empty for every input, and card
/// `COMPOSED-WINDOW-REACH` (wave 41) measured what that does and does not
/// mean.** `buffer_view_uses` below returns the empty vector for every
/// [`BufferEvent`], so this function is extensionally [`view_uses`] — pinned
/// by `the_depth_contribution_is_empty_for_every_event`, over every
/// constructor. What is *not* true is the reading
/// `COMPOSED-VIEW-WINDOW-INERT` recorded, that this function returns the empty
/// vector for every input: it returns a name at every position a view is read
/// from — a slice, a `len`, a text render, a `concat` piece — and those are
/// the positions the composed route's window is actually computed from. The
/// inert part is the recursion, and its consequence is precise: **a `for`
/// contributes nothing to any view's window.**
///
/// That is sound only because **no admitted composed loop body can read a
/// view**, and that premise now names its own three mechanisms instead of one,
/// each measured rather than argued (`composed_window_reach.rs` §2):
/// 1. a body is lowered by `source.buffer.v1`'s own reader, so it holds
///    [`BufferEvent`]s only, and no `BufferEvent` constructor names a view —
///    the exhaustive `match` below is what makes a widening that forgot this a
///    compile error;
/// 2. a body statement that *constructs* a view — a quoted literal, a `len`, a
///    `concat`, an inline-slice render, a `borrow_utf8` — is refused by name at
///    `Q077` before the buffer reader sees it
///    (`LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY`, `LOOP-PER-ITERATION-LOAN`);
/// 3. a **bare name** at a rendering operand — `print s;` — is *not* claimed
///    by that refusal, because a bare name deliberately does not select this
///    route (see `StringUse::Render`). It is refused instead by the buffer
///    route's own line reader, which demands a `dec`/`hex` selector. That
///    third mechanism is **incidental rather than a window rule**, and it is
///    the one a later card could remove without noticing: it is recorded as
///    `COMPOSED-BODY-BARE-RENDER-INCIDENTAL` and pinned by
///    `a_bare_render_of_a_view_inside_a_composed_body_is_refused`.
///
/// The recursion is kept, so that a later widening of the body grammar that
/// lets a view be read inside a loop lengthens the window here rather than
/// closing it before the loop opens. It is **defence in depth and not the
/// deciding rule**; the deciding rule is the `Repeat` arm of
/// [`check_string_program`], which judges every body store against every loan
/// live at the loop's own index before the body runs.
fn view_uses_at_any_depth(event: &StringEvent) -> Vec<&str> {
    /// The view names a buffer event reads, at any depth: a loop's are its
    /// body's, and every other buffer event's are none.
    fn buffer_view_uses(event: &BufferEvent) -> Vec<&str> {
        match event {
            BufferEvent::Repeat { body, .. } => body.iter().flat_map(buffer_view_uses).collect(),
            // No buffer event names a view; see above.
            BufferEvent::Bind { .. }
            | BufferEvent::Acquire { .. }
            | BufferEvent::Store { .. }
            | BufferEvent::Load { .. }
            | BufferEvent::Extent { .. }
            | BufferEvent::Release { .. }
            | BufferEvent::Lend { .. }
            | BufferEvent::Line(_)
            | BufferEvent::Construct { .. }
            | BufferEvent::Field { .. }
            | BufferEvent::FieldStore { .. }
            | BufferEvent::FieldLoad { .. }
            | BufferEvent::FieldExtent { .. }
            | BufferEvent::FieldRelease { .. }
            // Card `W48-QR-USABLE-RECORDS`: a field write names no view; this
            // route refuses every record statement before emission.
            | BufferEvent::FieldWrite { .. }
            // Card `OWNED-FIELD-ROUTE-REACH`: a copy bind names no view, and
            // on this route it cannot exist at all — `buffer_use` does not
            // claim the shape, so a composed module never lowers one and
            // `let t = s;` keeps the `Q068` it always had. Listed rather than
            // omitted so the exhaustive match stays the tripwire it is.
            | BufferEvent::CopyBind { .. }
            // Card `W48-QR-SCALAR-TYPES-R3`: a conversion names no view, and
            // this route lowers none; listed for the tripwire.
            | BufferEvent::Convert { .. }
            // Card `W48-QR-SCALAR-TYPES-R5`: nor does a value statement.
            | BufferEvent::Compute { .. }
            | BufferEvent::Return => Vec::new(),
        }
    }
    let mut names = view_uses(event);
    if let StringEvent::Buffer(buffer_event) = event {
        names.extend(buffer_view_uses(buffer_event));
    }
    names
}

/// The composed route's window reader, split into its two halves (card
/// `COMPOSED-WINDOW-REACH`, wave 41): the view names the event reads at its
/// **own** level, and the ones its **depth recursion** contributes.
///
/// Published so that the inertness [`view_uses_at_any_depth`]'s documentation
/// describes is a *tested equation* over every constructor rather than a
/// sentence a reader must take on trust. Neither half is used by the checker;
/// the checker calls [`view_uses_at_any_depth`], whose result is the
/// concatenation of the two.
#[must_use]
pub fn view_window_names(event: &StringEvent) -> (Vec<&str>, Vec<&str>) {
    let shallow = view_uses(event);
    let deep = view_uses_at_any_depth(event);
    let contributed = deep[shallow.len()..].to_vec();
    (shallow, contributed)
}

/// The last **top-level** event index at which each view name is read, at any
/// depth — the composed route's window function (card
/// `LOOPS-ON-COMPOSED-ROUTES`).
///
/// Purely syntactic. Exact over a flat sequence — there is no branch to make a
/// later index unreachable — and, where a `for` sits inside the window, sound
/// relative to `LOOP-V1`'s unrolling of it: every iteration of a loop at
/// top-level index `k` is judged against the loans whose window reaches `k`,
/// which over-approximates the unrolled program's exact windows in the
/// refusing direction (`QCore.LoopWindows.every_use_lies_inside_the_window`).
/// The statement that made the composed route refuse a `for` — "a store
/// inside a body could precede, in index order, a read that happens after it
/// in time" — is answered by *where the body is judged*, not by this map: the
/// `Repeat` arm of [`check_string_program`] judges every body store, at every
/// iteration, against every loan live at the loop's own index, before the
/// body runs.
fn last_view_uses(program: &StringProgram) -> BTreeMap<String, usize> {
    let mut last: BTreeMap<String, usize> = BTreeMap::new();
    for (index, event) in program.events.iter().enumerate() {
        for name in view_uses_at_any_depth(event) {
            last.insert(name.to_owned(), index);
        }
    }
    last
}

/// The state the check carries: which names are views of which storage, and what
/// each scalar name denotes.
struct Evaluation<'a> {
    literals: &'a [Vec<u8>],
    /// Payloads the compiler composed, in program order (card `STRING-CONCAT`).
    composed: Vec<Vec<u8>>,
    views: BTreeMap<String, StringViewRecord>,
    /// The embedded `source.buffer.v1` state — extents, buffer contents **and
    /// the scalar environment**. There is deliberately no second scalar map: a
    /// name bound by `len(s)` and a name bound by `b[0]` live in one
    /// environment, so neither route can see a value the other cannot.
    buffer: BufferEvaluation,
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
                if self.buffer.extents.contains_key(name) {
                    // A buffer at a scalar position, refused for the same
                    // reason and at the same code a string is: neither is a
                    // number, and the reader needs to be told which of the two
                    // they wrote.
                    return Err(StringError::CrossesBoundary {
                        event,
                        name: name.clone(),
                    });
                }
                // Card `W48-QR-SCALAR-TYPES-R6`: a name of another §G02t type
                // at a position of this route that takes an `i32` (a slice
                // bound, a composed piece) is refused naming its type — there
                // is no implicit conversion; `as` makes it an `i32`.
                if let Some((ty, _)) = self.buffer.typed.get(name) {
                    return Err(StringError::Composition {
                        event,
                        reason: format!(
                            "`{name}` is a `{ty}` and this position takes an `i32`; convert it \
                             with `as` (nothing converts implicitly)"
                        ),
                    });
                }
                self.buffer
                    .scalars
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
    /// because every record was built from a decoded literal, a composed
    /// payload, a validated buffer, or a checked slice of one of those.
    fn bytes(&self, view: &StringViewRecord) -> &[u8] {
        let payload: &[u8] = match &view.storage {
            StringStorage::Literal(index) => &self.literals[*index],
            StringStorage::Composed(index) => &self.composed[*index],
            StringStorage::Buffer(name) => self
                .buffer
                .contents
                .get(name)
                .map_or(&[][..], |bytes| bytes.as_slice()),
        };
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
    let Some(source) = evaluation.views.get(name).cloned() else {
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
    let bytes = evaluation.bytes(&source);
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
    // §4, verbatim: `{ ptr = p + low, len = high - low }`. The storage is
    // carried through unchanged, which is the statement that the derived view
    // addresses the *same* storage — and, since card `STRING-CONCAT`, the
    // statement that a slice of a **borrowed** view is itself a borrow of the
    // same buffer and inherits its exclusion window.
    let derived = StringViewRecord {
        storage: source.storage.clone(),
        offset: source.offset + low,
        len: high - low,
    };
    Ok((source, derived, low, high))
}

/// The byte index a `source.buffer.v1` operand denotes, **if this route can
/// decide it** (card `CT07-SLICE-PERMISSIONS`).
///
/// `None` means "not a non-negative check-time constant of this entry", and
/// every caller must read it as *not provably disjoint from anything* rather
/// than as an absence of conflict. It is deliberately a separate reader from
/// [`Evaluation::operand`]: this one answers a question about a **write's
/// footprint** and may not report a refusal, because the statement it is asked
/// about has not been judged yet — `source.buffer.v1`'s own model judges it a
/// few lines later and reports the operand refusal in its own words.
fn buffer_constant(evaluation: &Evaluation<'_>, operand: &BufferOperand) -> Option<u32> {
    let value = match operand {
        BufferOperand::Literal(value) => *value,
        BufferOperand::Scalar(name) => *evaluation.buffer.scalars.get(name)?,
    };
    u32::try_from(value).ok()
}

/// Where a [`TextOperand`] was written, which decides only **which code a name
/// with no view refuses at** (card `STRING-CONCAT`).
///
/// Nothing else differs: the literal, the name and the slice are resolved by
/// the same rules in both positions, so a piece cannot be admitted where the
/// same spelling at a rendering operand would be refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextPosition {
    /// A rendering operand: `print TEXT;`.
    Render,
    /// A `concat` piece.
    Piece,
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
    label: &str,
    position: TextPosition,
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
                storage: StringStorage::Literal(*literal),
                offset: 0,
                // `as` is exact: the payload was bounded at decoding.
                len: payload.len() as u32,
            };
            views.push((label.to_owned(), record.clone()));
            Ok(record)
        }
        TextOperand::Name(name) => match evaluation.views.get(name).cloned() {
            Some(view) => Ok(view),
            Option::None => Err(match position {
                // The reader wrote a bare *rendering* operand, so the refusal is
                // `STRING-SLICE-IO`'s own and names the two repairs.
                TextPosition::Render => StringError::NotRenderableText {
                    event,
                    name: name.clone(),
                },
                // A `concat` piece is a view position **inside the string
                // vocabulary**, exactly like a slice's source and a `len`'s
                // argument, so it keeps the base route's own `Q069` rather than
                // telling the reader to name a rendering they did not write.
                TextPosition::Piece => StringError::NotAString {
                    event,
                    name: name.clone(),
                },
            }),
        },
        TextOperand::Slice { name, low, high } => {
            match admit_slice(evaluation, event, name, low, high) {
                Ok((from, derived, low, high)) => {
                    let binding = label.to_owned();
                    views.push((binding.clone(), derived.clone()));
                    slices.push(SliceRecord {
                        event,
                        source: name.clone(),
                        binding,
                        from,
                        low,
                        high,
                        derived: derived.clone(),
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

/// Bind a refused `concat` or `borrow_utf8` name to an empty view, so that a
/// later statement reading it does not cascade into a second, misleading
/// refusal (card `STRING-CONCAT`).
///
/// This is `STD-BUFFER`'s own lesson, applied to a string binding: errors on
/// this route are collected rather than fatal, and a refused binding that left
/// its name unbound would make `print s;` report that `s` is not a string when
/// the one thing wrong with the program is the statement above it. The program
/// is refused either way; what this buys is that the report names the fault.
///
/// The empty payload it addresses is never published: a program reaching here
/// has at least one error, so no certificate is built from this evaluation.
fn register_refused_view(evaluation: &mut Evaluation<'_>, binding: &str) {
    let storage = StringStorage::Composed(evaluation.composed.len());
    evaluation.composed.push(Vec::new());
    evaluation.views.insert(
        binding.to_owned(),
        StringViewRecord {
            storage,
            offset: 0,
            len: 0,
        },
    );
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
    check_string_program_for_target(program, None)
}

/// Whether the composed entry holds an `f32`/`f64` value (card
/// `W48-QR-FLOAT-BUFFER-PAYLOADS`): `source.buffer.v1`'s own test over its
/// embedded statements. Exactly such a result carries the IEEE premise.
#[must_use]
pub fn uses_floats(program: &StringProgram) -> bool {
    crate::std_buffer::uses_floats(&crate::std_buffer::BufferProgram {
        entry: program.entry.clone(),
        declared_effects: program.declared_effects.clone(),
        float_effects: program.float_effects,
        records: Vec::new(),
        events: program
            .events
            .iter()
            .filter_map(|event| match event {
                StringEvent::Buffer(event) => Some(event.clone()),
                _ => None,
            })
            .collect(),
    })
}

/// Whether the program names `isize` or `usize` anywhere — a conversion's
/// target, a value statement's result or a loop's index (card
/// `W48-QR-SCALAR-TYPES-R6`). A program naming neither never reads a target.
pub fn names_target_width(program: &StringProgram) -> bool {
    let buffer = crate::std_buffer::BufferProgram {
        entry: program.entry.clone(),
        declared_effects: program.declared_effects.clone(),
        float_effects: program.float_effects,
        records: Vec::new(),
        events: program
            .events
            .iter()
            .filter_map(|event| match event {
                StringEvent::Buffer(event) => Some(event.clone()),
                _ => None,
            })
            .collect(),
    };
    crate::std_buffer::names_target_width(&buffer)
}

/// [`check_string_program`] for the builder's recorded target (card
/// `W48-QR-SCALAR-TYPES-R6`), which alone gives `isize`/`usize` a width on
/// the composed string/buffer route; with none such a program is refused.
pub fn check_string_program_for_target(
    program: &StringProgram,
    target: Option<&crate::scalar_types::RecordedTarget>,
) -> Result<StringCertificate, Vec<StringError>> {
    let mut errors = Vec::new();
    let target_assumption = if names_target_width(program) {
        target.cloned()
    } else {
        None
    };

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

    // The **inherited custody discipline** (card `STRING-CONCAT`), run first
    // and for the same reason `source.buffer.v1` runs it first: it is what lets
    // the value half assume a name it reads is live. A module holding no buffer
    // produces an empty operation list, which the checker accepts — so an
    // uncomposed string program reaches exactly the code it reached before.
    let operations = string_custody_operations(program);
    if let Err(error) = ownership::check_ownership(&operations) {
        errors.push(StringError::Custody {
            event: program.events.len(),
            error,
        });
    }

    let last_use = last_view_uses(program);
    let mut evaluation = Evaluation {
        literals: &program.literals,
        composed: Vec::new(),
        views: BTreeMap::new(),
        buffer: BufferEvaluation {
            target: target_assumption.clone(),
            // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: the entry row's comparison
            // effects, which the embedded value statements are decided under.
            float_effects: program.float_effects,
            ..BufferEvaluation::new()
        },
    };
    let mut views = Vec::new();
    let mut slices = Vec::new();
    let mut concats = Vec::new();
    let mut borrows: Vec<BorrowRecord> = Vec::new();
    let mut acquisitions = Vec::new();
    let mut accesses = Vec::new();
    let mut lines = Vec::new();
    let mut flushes = 0usize;
    let mut renderings = BTreeSet::new();
    let mut returned = false;
    /// One loan whose window has not closed: which buffer it holds, which view
    /// names it, when the window ends — and, since card
    /// `CT07-SLICE-PERMISSIONS`, **which bytes it covers**.
    ///
    /// The range is read out of the view's own `{offset, len}` rather than
    /// carried beside it, which is what makes a slice of a loan a loan of
    /// exactly the bytes that slice addresses: `admit_slice` already computes
    /// `offset + low` and `high - low`, so there is no second arithmetic to
    /// disagree with the first.
    struct LiveBorrow {
        buffer: String,
        view: String,
        through: usize,
        low: u32,
        high: u32,
        /// Whether this loan covers the buffer's whole declared extent. Carried
        /// so a whole-extent loan's refusal reads exactly as it read before
        /// sub-extent lending existed.
        whole: bool,
    }

    /// The first conflict between a `for` body and the loans live at the
    /// loop, under the runs-many-times reading (card
    /// `LOOPS-ON-COMPOSED-ROUTES`): the body is walked once per iteration
    /// with the index bound, nested loops likewise, and every store is judged
    /// against every live loan of its buffer exactly as a top-level store is
    /// — `ranges_overlap` over `[i, i + 1)`, and a byte the lowering-visible
    /// tier cannot resolve **not provably disjoint** from anything. That tier
    /// is the loop index, a literal `let`, an `extent` read and the entry's
    /// own constants; a value read out of a byte or a field is deliberately
    /// outside it, so a store indexed by one conflicts with every live loan
    /// of that buffer, the fail-closed direction (`WRITE-LEND-COMPUTED-INDEX`'s
    /// shape, reached from the read side). A release inside a body conflicts
    /// with any live loan of its buffer; it is also refused by name before
    /// this runs, and the arm is kept so the decision is total on its own
    /// terms.
    fn loop_body_conflict<'a>(
        live: &'a [LiveBorrow],
        scalars: &BTreeMap<String, i32>,
        extents: &BTreeMap<String, u32>,
        variable: &str,
        count: u32,
        body: &[BufferEvent],
    ) -> Option<(&'a LiveBorrow, &'static str)> {
        fn resolve(env: &BTreeMap<String, i32>, operand: &BufferOperand) -> Option<u32> {
            let value = match operand {
                BufferOperand::Literal(value) => *value,
                BufferOperand::Scalar(name) => *env.get(name)?,
            };
            u32::try_from(value).ok()
        }
        let mut env = scalars.clone();
        for iteration in 0..count {
            env.insert(
                variable.to_owned(),
                i32::try_from(iteration).expect("MAX_ITERATIONS is far inside i32"),
            );
            for event in body {
                match event {
                    BufferEvent::Store {
                        name, index: at, ..
                    } => {
                        let written = resolve(&env, at);
                        if let Some(loan) = live.iter().find(|loan| {
                            loan.buffer == *name
                                && written.is_none_or(|byte| {
                                    ranges_overlap(loan.low, loan.high, byte, byte + 1)
                                })
                        }) {
                            return Some((loan, "write a byte of"));
                        }
                    }
                    BufferEvent::Release { name } => {
                        if let Some(loan) = live.iter().find(|loan| loan.buffer == *name) {
                            return Some((loan, "release"));
                        }
                    }
                    BufferEvent::Bind { binding, value } => {
                        env.insert(binding.clone(), *value);
                    }
                    BufferEvent::Extent { binding, name } => match extents.get(name) {
                        Some(extent) => {
                            env.insert(binding.clone(), *extent as i32);
                        }
                        Option::None => {
                            env.remove(binding);
                        }
                    },
                    // A byte or a field value is outside the tier: the binding
                    // becomes unresolvable, so a later index through it is
                    // judged as not provably disjoint.
                    BufferEvent::Load { binding, .. } | BufferEvent::Field { binding, .. } => {
                        env.remove(binding);
                    }
                    BufferEvent::Repeat {
                        variable,
                        count,
                        body,
                        ..
                    } => {
                        if let Some(conflict) =
                            loop_body_conflict(live, &env, extents, variable, *count, body)
                        {
                            return Some(conflict);
                        }
                    }
                    // Card `AGGREGATE-OWNED-FIELD`'s field-path loads bind a
                    // name outside the tier; every record statement is refused
                    // on this route before the body runs, so these arms are
                    // total rather than reachable.
                    BufferEvent::FieldLoad { binding, .. }
                    | BufferEvent::FieldExtent { binding, .. } => {
                        env.remove(binding);
                    }
                    // Card `OWNED-FIELD-ROUTE-REACH`: a copy bind's binding
                    // leaves the constant tier, the `Load` rule — this route
                    // lowers none, so the arm is total rather than reachable,
                    // and it is written in the refusing direction (an
                    // unresolvable index is not provably disjoint).
                    BufferEvent::CopyBind { binding, .. } => {
                        env.remove(binding);
                    }
                    // Card `W48-QR-SCALAR-TYPES-R3`: a conversion's binding
                    // leaves the tier too, in the refusing direction; this
                    // route lowers none.
                    BufferEvent::Convert { binding, .. }
                    // Card `W48-QR-SCALAR-TYPES-R5`: a value statement's too.
                    | BufferEvent::Compute { binding, .. } => {
                        env.remove(binding);
                    }
                    BufferEvent::Acquire { .. }
                    | BufferEvent::Lend { .. }
                    | BufferEvent::Construct { .. }
                    | BufferEvent::FieldStore { .. }
                    | BufferEvent::FieldRelease { .. }
                    | BufferEvent::FieldWrite { .. }
                    | BufferEvent::Line(_)
                    | BufferEvent::Return => {}
                }
            }
        }
        Option::None
    }
    let mut live: Vec<LiveBorrow> = Vec::new();

    if !errors.is_empty() {
        return Err(errors);
    }

    for (index, event) in program.events.iter().enumerate() {
        // A borrow's window closes after the last statement that reads it, so
        // the expiry happens *before* this statement is judged. That is what
        // makes `let s = borrow_utf8(b); print s; b[0] = 65; drop b;` admitted
        // and `let s = borrow_utf8(b); b[0] = 65; print s;` refused — the same
        // two statements, in the order that decides whether the view the
        // program renders is the view the checker validated.
        live.retain(|loan| loan.through >= index);
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
                evaluation.buffer.scalars.insert(binding.clone(), *value);
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
                    storage: StringStorage::Literal(*literal),
                    offset: 0,
                    // `as` is exact: the payload was bounded above.
                    len: payload.len() as u32,
                };
                evaluation.views.insert(binding.clone(), record.clone());
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
                // A slice of a borrowed view is itself a borrow of the same
                // buffer: its storage is that buffer's, so it is registered as
                // a live borrow with its **own** last use. That is how a borrow
                // window extends through every view derived from it, rather
                // than by a separate reachability pass that could disagree.
                if let StringStorage::Buffer(buffer) = &derived.storage {
                    // Card `CT07-SLICE-PERMISSIONS`: the derived loan covers
                    // exactly the bytes the derived view addresses, which are a
                    // **subset** of its source's — so slicing a loan narrows a
                    // permission the entry already holds rather than taking a
                    // second one, and a write the source loan excluded stays
                    // excluded only while the source loan is itself live.
                    let extent = evaluation.buffer.extents.get(buffer).copied();
                    live.push(LiveBorrow {
                        buffer: buffer.clone(),
                        view: binding.clone(),
                        through: last_use.get(binding).copied().unwrap_or(index),
                        low: derived.offset,
                        high: derived.offset + derived.len,
                        whole: derived.offset == 0 && Some(derived.len) == extent,
                    });
                }
                evaluation.views.insert(binding.clone(), derived.clone());
                views.push((binding.clone(), derived.clone()));
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
            // Card `STRING-CONCAT`: amendment §5's borrow, with its two
            // obligations decided here — the bytes are validated as UTF-8 out
            // of the buffer's own write history, and the window in which the
            // buffer may not change opens.
            StringEvent::Borrow {
                binding,
                buffer,
                range,
            } => {
                let Some(bytes) = evaluation.buffer.contents.get(buffer).cloned() else {
                    errors.push(StringError::NotABuffer {
                        event: index,
                        name: buffer.clone(),
                    });
                    register_refused_view(&mut evaluation, binding);
                    continue;
                };
                // `as` is exact: an extent is at most `MAX_EXTENT`, and
                // `contents` is exactly that many bytes.
                let extent = bytes.len() as u32;
                // Card `CT07-SLICE-PERMISSIONS`. The whole-extent form is
                // `[0, extent)` — written here once, so that everything below
                // this point is stated about a range and the pre-card spelling
                // is that statement's degenerate case rather than a second path
                // through the checker.
                let (low, high) = match range {
                    Option::None => (0u32, extent),
                    Some((low, high)) => {
                        // The bounds are resolved by the **same** operand rule a
                        // slice bound is: an integer literal, or a name this
                        // entry bound to one. A buffer or a string at a bound
                        // position is `Q072`, exactly where it is at every other
                        // scalar position on this route.
                        let low = evaluation.operand(index, low);
                        let high = evaluation.operand(index, high);
                        let (low, high) = match (low, high) {
                            (Ok(low), Ok(high)) => (low, high),
                            (low, high) => {
                                errors.extend(low.err());
                                errors.extend(high.err());
                                register_refused_view(&mut evaluation, binding);
                                continue;
                            }
                        };
                        let length =
                            i32::try_from(extent).expect("an extent is bounded by MAX_EXTENT");
                        if low < 0 || high < low || high > length {
                            errors.push(StringError::BorrowRange {
                                event: index,
                                buffer: buffer.clone(),
                                low,
                                high,
                                extent,
                            });
                            register_refused_view(&mut evaluation, binding);
                            continue;
                        }
                        // Both bounds were checked before either was used as an
                        // offset, so neither conversion below can be out of
                        // range.
                        (low as u32, high as u32)
                    }
                };
                // The UTF-8 decision is made on **the bytes this loan views**
                // and no others, which is what makes a range whose ends split a
                // code point refuse: the bytes it would view are then
                // ill-formed, and `Q079`'s condition is reached without being
                // widened. For the whole-extent form this is the pre-card
                // decision over the pre-card bytes, `at` included.
                if let Err(error) = std::str::from_utf8(&bytes[low as usize..high as usize]) {
                    errors.push(StringError::BufferNotUtf8 {
                        event: index,
                        buffer: buffer.clone(),
                        at: error.valid_up_to(),
                    });
                    register_refused_view(&mut evaluation, binding);
                    continue;
                }
                let through = last_use.get(binding).copied().unwrap_or(index);
                let record = StringViewRecord {
                    storage: StringStorage::Buffer(buffer.clone()),
                    offset: low,
                    len: high - low,
                };
                live.push(LiveBorrow {
                    buffer: buffer.clone(),
                    view: binding.clone(),
                    through,
                    low,
                    high,
                    whole: low == 0 && high == extent,
                });
                evaluation.views.insert(binding.clone(), record.clone());
                views.push((binding.clone(), record.clone()));
                borrows.push(BorrowRecord {
                    event: index,
                    buffer: buffer.clone(),
                    binding: binding.clone(),
                    through,
                    low,
                    high,
                    view: record,
                });
            }
            // Card `STRING-CONCAT`: text this program **computes**. Every piece
            // is check-time-known, so the bytes are composed here and become a
            // static payload of the emitted program — no allocation, no copy at
            // run time, and a result whose length is a check-time constant.
            StringEvent::Concat { binding, pieces } => {
                if pieces.len() < 2 {
                    errors.push(StringError::Composition {
                        event: index,
                        reason: "`concat` composes two or more pieces; a composition of one is \
                                 that piece, and `source.string.v1` refuses a spelling whose \
                                 result is a view it already has"
                            .to_owned(),
                    });
                    register_refused_view(&mut evaluation, binding);
                    continue;
                }
                let mut payload: Vec<u8> = Vec::new();
                let mut piece_lens = Vec::new();
                let mut refused = false;
                for (position, piece) in pieces.iter().enumerate() {
                    let bytes = match piece {
                        StringPiece::Text(operand) => {
                            let label = concat_piece_name(index, position);
                            match text_view(
                                &evaluation,
                                index,
                                operand,
                                &label,
                                TextPosition::Piece,
                                &mut slices,
                                &mut views,
                            ) {
                                Ok(view) => evaluation.bytes(&view).to_vec(),
                                Err(error) => {
                                    errors.push(error);
                                    refused = true;
                                    continue;
                                }
                            }
                        }
                        // The digits are `Rendering::render`'s and nothing
                        // else's, so `concat("n=", dec(n))` and `print dec n;`
                        // cannot disagree about what the number looks like.
                        StringPiece::Scalar { rendering, name } => {
                            match evaluation.operand(index, &StringOperand::Scalar(name.clone())) {
                                Ok(value) => rendering.render(value).into_bytes(),
                                Err(error) => {
                                    errors.push(error);
                                    refused = true;
                                    continue;
                                }
                            }
                        }
                    };
                    piece_lens.push(bytes.len() as u32);
                    payload.extend_from_slice(&bytes);
                }
                if refused {
                    register_refused_view(&mut evaluation, binding);
                    continue;
                }
                if payload.len() > MAX_STRING_BYTES as usize {
                    errors.push(StringError::Composition {
                        event: index,
                        reason: format!(
                            "this composition is {} bytes, longer than the {MAX_STRING_BYTES}-byte \
                             payload `source.string.v1` models; a longer result is refused rather \
                             than truncated",
                            payload.len()
                        ),
                    });
                    register_refused_view(&mut evaluation, binding);
                    continue;
                }
                // Well-formedness is a *consequence* here rather than a check:
                // every piece is well-formed UTF-8 (a literal is validated at
                // decoding, a slice only at code-point boundaries, a borrow at
                // its own gate, and rendered digits are ASCII), and the
                // concatenation of well-formed UTF-8 sequences is well-formed.
                // It is checked anyway, because the model must not be able to
                // hold an ill-formed payload however it was constructed.
                if let Err(error) = std::str::from_utf8(&payload) {
                    errors.push(StringError::Composition {
                        event: index,
                        reason: format!(
                            "this composition is not well-formed UTF-8 from byte {}, which is a \
                             defect in this checker rather than in the program: every piece it \
                             admits is well-formed and concatenation preserves that",
                            error.valid_up_to()
                        ),
                    });
                    register_refused_view(&mut evaluation, binding);
                    continue;
                }
                let storage = StringStorage::Composed(evaluation.composed.len());
                let record = StringViewRecord {
                    storage,
                    offset: 0,
                    len: payload.len() as u32,
                };
                evaluation.composed.push(payload);
                evaluation.views.insert(binding.clone(), record.clone());
                views.push((binding.clone(), record.clone()));
                concats.push(ConcatRecord {
                    event: index,
                    binding: binding.clone(),
                    piece_lens,
                    result: record,
                });
            }
            // Card `STRING-CONCAT`: an embedded `source.buffer.v1` statement.
            // The borrow exclusion is decided here, **before** the buffer
            // model runs, because a statement refused for writing through a
            // live borrow should say that rather than report a byte it never
            // wrote.
            StringEvent::Buffer(buffer_event) => {
                // Card `LOOPS-ON-COMPOSED-ROUTES`: a `for` is **admitted** on
                // this composed route, and the statement that used to refuse
                // it here — "a store inside a body could precede, in index
                // order, a read that happens after it in time" — is answered
                // by where the body is judged rather than by a window over
                // body positions. The loop is one top-level statement at
                // `index`; the loans live at `index` are exactly the loans
                // still read after the loop (a body cannot read a view, see
                // `view_uses_at_any_depth`); and under the runs-many-times
                // reading every body store, at every iteration, is judged
                // against every one of them **before the body runs**. That is
                // `QCore.LoopWindows`' iteration window, which covers the
                // unrolled program's exact one in the refusing direction.
                //
                // Two body shapes are refused by name first, before any
                // decision below: a lend inside the body is a per-iteration
                // loan (`LOOP-PER-ITERATION-LOAN`), and a body that acquires
                // or releases would put a custody step inside an iteration
                // where neither this route's custody list nor the buffer
                // route's identity gate decides it
                // (`LOOP-WINDOW-BODY-CUSTODY`).
                if let BufferEvent::Repeat {
                    variable,
                    count,
                    body,
                    ..
                } = buffer_event
                {
                    // Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-TYPES-COMPOSED-STRING`)
                    // discharges card R5's refusal of the typed tier here: a
                    // conversion, a value statement and a loop of any index
                    // type run through `source.buffer.v1`'s own runner below,
                    // over its typed environment, under the recorded target,
                    // and the emitter carries that route's typed environment
                    // and helpers across the whole entry. A typed statement
                    // stores no byte and releases nothing, so it adds no
                    // conflict to the window question decided next.
                    if buffer_event.any(&|inner| matches!(inner, BufferEvent::Lend { .. })) {
                        errors.push(StringError::Composition {
                            event: index,
                            reason: "this `for` body opens a lend, which is a per-iteration loan: \
                                     its window opens and closes inside one iteration and is not \
                                     an interval of statement indices, so the composed \
                                     string/buffer entry refuses it by name rather than \
                                     approximating it (`LOOP-PER-ITERATION-LOAN`)"
                                .to_owned(),
                        });
                        continue;
                    }
                    if buffer_event.any(&|inner| {
                        matches!(
                            inner,
                            BufferEvent::Acquire { .. } | BufferEvent::Release { .. }
                        )
                    }) {
                        errors.push(StringError::Composition {
                            event: index,
                            reason: "this `for` body acquires or releases a buffer; on the \
                                     composed string/buffer route a loop body neither acquires \
                                     nor releases, because a custody step inside an iteration is \
                                     decided by neither this route's custody list nor the buffer \
                                     route's per-iteration identity gate \
                                     (`LOOP-WINDOW-BODY-CUSTODY`)"
                                .to_owned(),
                        });
                        continue;
                    }
                    if let Some((loan, action)) = loop_body_conflict(
                        &live,
                        &evaluation.buffer.scalars,
                        &evaluation.buffer.extents,
                        variable,
                        *count,
                        body,
                    ) {
                        errors.push(StringError::BorrowExclusion {
                            event: index,
                            buffer: loan.buffer.clone(),
                            view: loan.view.clone(),
                            through: loan.through,
                            action,
                            low: loan.low,
                            high: loan.high,
                            whole: loan.whole,
                        });
                        continue;
                    }
                    // The body runs through `source.buffer.v1`'s **own**
                    // runner — the one that owns the index binding, the
                    // body-local scoping and the shadowing refusal — rather
                    // than through a second iteration rule written here.
                    let mut run = BufferRun::new(std::mem::take(&mut evaluation.buffer));
                    run.repeat(variable, *count, body, index);
                    evaluation.buffer = run.evaluation;
                    for error in run.errors {
                        match error {
                            BufferError::Loop { refusal, .. } => {
                                errors.push(StringError::Loop {
                                    event: index,
                                    refusal,
                                });
                            }
                            other => errors.push(StringError::Composition {
                                event: index,
                                reason: other.to_string(),
                            }),
                        }
                    }
                    if run.returned {
                        errors.push(StringError::Shape {
                            event: index,
                            reason: "a `return` inside a `for` body is an early exit, and this \
                                     composed entry models one flat sequence ending in exactly \
                                     one return"
                                .to_owned(),
                        });
                    }
                    acquisitions.extend(run.acquisitions);
                    accesses.extend(run.accesses);
                    lines.extend(run.lines);
                    flushes += run.flushes;
                    renderings.extend(run.renderings);
                    continue;
                }
                // Card `CT07-SLICE-PERMISSIONS`: the decision is **range-wise**
                // for a write and **whole-buffer** for a release, and the two
                // are different because the operations are.
                //
                // A write touches one byte, so it conflicts with a loan exactly
                // when that byte is inside the loan — `ranges_overlap` over
                // `[i, i + 1)`. A write outside every live loan is admitted,
                // and that admission is this card: two disjoint ranges of one
                // buffer can now be held by two parties at once, one reading
                // and one writing, with the disjointness *decided* rather than
                // assumed.
                //
                // A release ends the storage every loan addresses, so it stays
                // refused for any live loan however narrow. Making that arm
                // range-wise would let a program free the bytes a live view
                // reads, which is the one thing the window exists to prevent.
                let conflict = match buffer_event {
                    BufferEvent::Store {
                        name, index: at, ..
                    } => {
                        // A byte this route cannot resolve to a constant is
                        // **not provably disjoint** from anything, so it
                        // conflicts with every live loan of the buffer. That is
                        // the fail-closed direction, and it is reachable only
                        // through a program `source.buffer.v1` refuses anyway —
                        // kept because the exclusion runs first and a decision
                        // that fell through here would be an admission.
                        let written = buffer_constant(&evaluation, at);
                        live.iter()
                            .find(|loan| {
                                loan.buffer == *name
                                    && written.is_none_or(|byte| {
                                        ranges_overlap(loan.low, loan.high, byte, byte + 1)
                                    })
                            })
                            .map(|loan| (loan, "write a byte of"))
                    }
                    BufferEvent::Release { name } => live
                        .iter()
                        .find(|loan| loan.buffer == *name)
                        .map(|loan| (loan, "release")),
                    _ => Option::None,
                };
                if let Some((loan, action)) = conflict {
                    errors.push(StringError::BorrowExclusion {
                        event: index,
                        buffer: loan.buffer.clone(),
                        view: loan.view.clone(),
                        through: loan.through,
                        action,
                        low: loan.low,
                        high: loan.high,
                        whole: loan.whole,
                    });
                    continue;
                }
                if matches!(buffer_event, BufferEvent::Line(_) | BufferEvent::Return) {
                    errors.push(StringError::Shape {
                        event: index,
                        reason: "a line and a return are this route's own events, not embedded \
                                 buffer ones"
                            .to_owned(),
                    });
                    continue;
                }
                // A `for` never reaches this point: card
                // `LOOPS-ON-COMPOSED-ROUTES` admits it in the arm above,
                // discharging `STRING-CONCAT-LOOP-WINDOW`, whose refusal used
                // to stand here.
                //
                // **A write lend is refused here, and refusing is fail-closed**
                // (card `SLICE-PERMISSIONS-WRITE-LEND`). This composed entry
                // already holds `borrow_utf8`'s immutable read loans, and a
                // lend's exclusive write authority over a range of the same
                // buffer would need the cross-model overlap rule — a write
                // lend overlapping a live read loan refused, a disjoint one
                // admitted — which no route decides yet. Until it exists,
                // every lend beside a read loan refuses, which trivially
                // refuses every overlapping one; the range-wise admission is
                // the named follow-on `SLICE-PERMISSIONS-COMPOSED-LEND`. The
                // uncomposed `source.buffer.v1` route models the lend itself.
                if matches!(buffer_event, BufferEvent::Lend { .. }) {
                    errors.push(StringError::Composition {
                        event: index,
                        reason: "`lend(…)` is not a statement this composed string/buffer entry \
                                 models; a write lend's exclusive authority beside \
                                 `borrow_utf8`'s immutable read loans needs the cross-model \
                                 overlap rule, and until that exists the composition refuses \
                                 rather than admitting an unchecked pair \
                                 (`SLICE-PERMISSIONS-COMPOSED-LEND`)"
                            .to_owned(),
                    });
                    continue;
                }
                let step = evaluation.buffer.apply(index, buffer_event);
                for error in step.errors {
                    // `source.buffer.v1`'s message, verbatim, under **this**
                    // pass's code. §L21 forbids a second pass taking `Q052`
                    // through `Q055`; what a reader needs is the buffer model's
                    // own account of what is wrong plus the code that says
                    // which route asked, and that is what this is.
                    errors.push(StringError::Composition {
                        event: index,
                        reason: error.to_string(),
                    });
                }
                acquisitions.extend(step.acquisition);
                accesses.extend(step.access);
            }
            StringEvent::Len { binding, name } => match evaluation.views.get(name).cloned() {
                // §3.1: the exact byte count, available without scanning. It is
                // read out of the view record itself, which is the model's
                // `{ptr, len}` pair, and there is no second place it could come
                // from.
                Some(view) => {
                    evaluation
                        .buffer
                        .scalars
                        .insert(binding.clone(), view.len as i32);
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
                    // Card `W48-QR-SCALAR-TYPES-R6`: a name of the typed
                    // environment renders at its own type — `source.buffer.v1`'s
                    // rendering, framed by the one framing every route shares;
                    // every other operand takes the path it always took.
                    if let StdIoOperand::Scalar(name) = operand
                        && let Some((ty, value)) = evaluation.buffer.typed.get(name).copied()
                        && !evaluation.views.contains_key(name)
                    {
                        let level = match line {
                            StdIoEvent::Log { level, .. } => Some(*level),
                            _ => None,
                        };
                        match crate::scalar_types::render(
                            *rendering == Rendering::Hex,
                            ty,
                            value,
                            evaluation.buffer.target.as_ref(),
                        ) {
                            Some(body) => lines.push(rendered_text_line(level, &body)),
                            None => errors.push(StringError::Composition {
                                event: index,
                                reason: format!(
                                    "`{name}` is a `{ty}`, which has no `hex` rendering"
                                ),
                            }),
                        }
                        continue;
                    }
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
                let label = text_slice_name(index);
                match text_view(
                    &evaluation,
                    index,
                    operand,
                    &label,
                    TextPosition::Render,
                    &mut slices,
                    &mut views,
                ) {
                    Ok(view) => {
                        let bytes = evaluation.bytes(&view);
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
            composed: evaluation.composed,
            views,
            slices,
            concats,
            borrows,
            acquisitions,
            accesses,
            scalars: evaluation.buffer.scalars,
            lines,
            flushes,
            renderings,
            typed: evaluation.buffer.typed,
            target_assumption,
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
    Composed,
    Views,
    Slices,
    Concats,
    Borrows,
    Acquisitions,
    Accesses,
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
            Self::Composed => "the composed payloads",
            Self::Views => "the views and the storage they address",
            Self::Slices => "the admitted slices and their arithmetic",
            Self::Concats => "the admitted compositions and their length arithmetic",
            Self::Borrows => "the admitted borrows and their exclusion windows",
            Self::Acquisitions => "the buffers acquired and the extents they declare",
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
pub fn recheck_string_certificate(
    program: &StringProgram,
    certificate: &StringCertificate,
) -> Result<(), StringRecheckFailure> {
    let rederived =
        check_string_program_for_target(program, certificate.target_assumption.as_ref())
            .map_err(|_| StringRecheckFailure::Refused)?;
    if rederived.entry != certificate.entry {
        return Err(StringRecheckFailure::Entry);
    }
    if rederived.effects != certificate.effects {
        return Err(StringRecheckFailure::Effects);
    }
    if rederived.literals != certificate.literals {
        return Err(StringRecheckFailure::Literals);
    }
    if rederived.composed != certificate.composed {
        return Err(StringRecheckFailure::Composed);
    }
    if rederived.views != certificate.views {
        return Err(StringRecheckFailure::Views);
    }
    if rederived.slices != certificate.slices {
        return Err(StringRecheckFailure::Slices);
    }
    if rederived.concats != certificate.concats {
        return Err(StringRecheckFailure::Concats);
    }
    if rederived.borrows != certificate.borrows {
        return Err(StringRecheckFailure::Borrows);
    }
    if rederived.acquisitions != certificate.acquisitions {
        return Err(StringRecheckFailure::Acquisitions);
    }
    if rederived.accesses != certificate.accesses {
        return Err(StringRecheckFailure::Accesses);
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
    if rederived.typed != certificate.typed
        || rederived.target_assumption != certificate.target_assumption
    {
        return Err(StringRecheckFailure::Scalars);
    }
    Ok(())
}
