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
//! field read. A record of scalar fields is a **plain copyable value**: it
//! contributes no ownership operation, opens no obligation and derives no
//! effect, and a record name at a custody or a scalar position is refused. It
//! is admitted here rather than anywhere else because this is the route that
//! already holds a check-time-constant integer environment, which is exactly
//! what a field value is and what a field read produces.
//!
//! Card `AGGREGATE-OWNED-FIELD` (wave 39) admits a field declared `bytes`: a
//! construction supplying an acquired buffer at such a field **moves** it in,
//! and the record is then a custody shape decided entirely by the
//! path-extended ownership IR — `p.FIELD` stands at every custody position a
//! buffer name does (a store, a load, an extent read, a release), `let c =
//! p.FIELD;` is the partial move, and `drop p;` releases every live owned
//! field in declaration order. This module builds the operation list
//! ([`OwnershipOp::Compose`], [`OwnershipOp::ReadField`] and their siblings)
//! and decides none of the four rules; a record declaring no `bytes` field
//! contributes exactly the operations it always did — none.
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

use crate::ownership::{self, FieldPath, Footprint, OwnershipError, OwnershipOp, ResourceId};
use crate::scalar_types::{self as st, RecordedTarget, ScalarType};
use crate::source_loop::{self, BoundRefusal, LoopRefusal, MAX_ITERATIONS, MAX_UNROLLED_EVENTS};
use crate::source_record::{self, FieldType, FieldValue, RecordRefusal, RecordType, RecordValue};
use crate::std_io::{
    ObservedLine, Rendering, StdIoEvent, StdIoOperand, rendered_line, rendered_text_line,
};
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

/// What a record construction supplies at one field (card
/// `W48-QR-SCALAR-TYPES-R3`).
///
/// A field value is either a literal, read at the field's own type, or a name
/// this entry bound — a scalar of some §G02t type, or (at a field declared
/// `bytes`) an acquired buffer moved in. Which of those the name denotes, and
/// whether its type is the field's, is the checker's to decide: there is no
/// implicit conversion, so a name of another type is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldOperand {
    /// An integer literal. At an `i32` field the lowering reads it with this
    /// route's own `i32` reader, exactly as before the card; at a field of
    /// another type it is read at that type and range-checked by the checker.
    Literal(i128),
    /// `true` or `false`, written at a field declared `bool`.
    Bool(bool),
    /// A name this entry bound.
    Name(String),
    /// `["-"] FLOAT` (card `W48-QR-FLOAT-BUFFER-PAYLOADS`), written at a
    /// field declared `f32` or `f64`: read once at the field's type. At a
    /// field of any other type it is refused — a float is never implicitly
    /// an integer.
    Float(crate::scalar_floats::FloatLiteral),
}

impl From<BufferOperand> for FieldOperand {
    fn from(operand: BufferOperand) -> Self {
        match operand {
            BufferOperand::Literal(value) => Self::Literal(i128::from(value)),
            BufferOperand::Scalar(name) => Self::Name(name),
        }
    }
}

/// One operand of a value statement on this route (card
/// `W48-QR-SCALAR-TYPES-R5`, `W48-QR-BUFFER-ROUTE-ARITHMETIC`): a name this
/// entry bound to a scalar of any §G02t type, an integer literal typed by the
/// other operand (`i32` when both are literals), or `true`/`false`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueOperand {
    Name(String),
    Integer(i128),
    Bool(bool),
    /// `["-"] FLOAT` (card `W48-QR-FLOAT-BUFFER-PAYLOADS`), typed by the other
    /// operand (`f64` when there is none) and read once at that type — the
    /// shared decision's `Operand::Float`.
    Float(crate::scalar_floats::FloatLiteral),
}

/// The one §G02t operation a value statement performs on this route (card
/// `W48-QR-SCALAR-TYPES-R5`). The fourth, `as`, is [`BufferEvent::Convert`],
/// the statement card `W48-QR-SCALAR-TYPES-R3` gave the route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueOperation {
    /// `a OP b`, checked at the operands' one type.
    Binary(st::Op, ValueOperand, ValueOperand),
    /// `a CMP b`, producing a `bool`.
    Compare(st::Cmp, ValueOperand, ValueOperand),
    /// `-a`, checked.
    Negate(ValueOperand),
    /// `LITERAL as TYPE` of a float literal, and the literal binding
    /// `let NAME = FLOAT;`, which is `FLOAT as f64` (card
    /// `W48-QR-FLOAT-BUFFER-PAYLOADS`). A literal converted to a float type is
    /// read once at that type; one converted to an integer type is the `f64`
    /// literal truncated, `OutOfRange` when it does not fit. Every other
    /// conversion — of a name or an `i32` literal — stays
    /// [`BufferEvent::Convert`].
    Convert(ValueOperand, ScalarType),
}

impl ValueOperation {
    /// The operands, in order.
    #[must_use]
    pub fn operands(&self) -> Vec<&ValueOperand> {
        match self {
            Self::Binary(_, a, b) | Self::Compare(_, a, b) => vec![a, b],
            Self::Negate(a) | Self::Convert(a, _) => vec![a],
        }
    }

    /// The names the statement reads.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.operands()
            .into_iter()
            .filter_map(|operand| match operand {
                ValueOperand::Name(name) => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }
}

/// The index type of one `for` (card `W48-QR-SCALAR-TYPES-R5`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoopIndex {
    /// An `i32` index: a literal bound, or a name of the lowering's `i32`
    /// tier — every loop written before the card.
    I32,
    /// An index of another integer type, whose bound is the name `bound` of
    /// that type. A literal bound is an `i32`, so a bound of another type is
    /// always a name, and the checker compares the name's value against the
    /// count the lowering decided (a disagreement refuses), while the emitted
    /// C compares the index against the name's own C object, so a native run
    /// checks the lowering's fold as well.
    Typed { ty: ScalarType, bound: String },
}

impl LoopIndex {
    /// The index's §G02t type.
    #[must_use]
    pub fn ty(&self) -> ScalarType {
        match self {
            Self::I32 => ScalarType::I32,
            Self::Typed { ty, .. } => *ty,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BufferEvent {
    /// `let NAME = INTEGER;` — an ordinary integer binding.
    Bind { binding: String, value: i32 },
    /// `let BINDING = OPERAND as TYPE;` — the one explicit conversion of
    /// §G02t (card `W48-QR-SCALAR-TYPES-R3`). The operand is an `i32` literal
    /// or a name of any §G02t type; the result is the same number at `to`, or
    /// the program is refused. Every value on this route is a check-time
    /// constant, so a conversion that does not fit is decided — and refused —
    /// at check time, never at run time. It is **not** a `buffer_use`, so it
    /// selects no route: it is read by `lower_let` alone.
    Convert {
        binding: String,
        operand: BufferOperand,
        to: ScalarType,
    },
    /// `let BINDING = VALUE_EXPRESSION;` — §G02t's value statement (card
    /// `W48-QR-SCALAR-TYPES-R5`, `W48-QR-BUFFER-ROUTE-ARITHMETIC`): one
    /// arithmetic operation, comparison or negation over bound scalars and
    /// literals, read by the shared reader (`source_value_statement`) and
    /// decided by the shared decision (`scalar_values::Decider`). Every value
    /// on this route is a check-time constant — a loop index is one per
    /// iteration — so every statement is **folded**: a fault is refused at
    /// check time (`Q053`), naming the variant the typed path would raise, and
    /// never happens at run time. Not a `buffer_use`: it selects no route.
    Compute {
        binding: String,
        operation: ValueOperation,
    },
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
    /// top-level statement naming the loan **at any depth** — since card
    /// `LOOPS-ON-COMPOSED-ROUTES` a `for` whose body names the loan is that
    /// statement, and the loan is live across the whole loop
    /// ([`last_uses`], proved sound relative to `LOOP-V1`'s unrolling in
    /// `QCore.LoopWindows`). A lend written *inside* a body is a
    /// per-iteration loan and refuses by name (`LOOP-PER-ITERATION-LOAN`).
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
    ///
    /// Card `W48-QR-SCALAR-TYPES-R5`: `index` is the bound's §G02t type, and
    /// so the index's — the low end `0` is a literal and takes it. An `i32`
    /// loop ([`LoopIndex::I32`]) is exactly the loop before the card; a loop
    /// of any other integer type names its bound ([`LoopIndex::Typed`]) and
    /// binds the index at that type, each value `0 … COUNT-1` a value of it
    /// ([`crate::source_loop::typed_bound`]).
    Repeat {
        variable: String,
        count: u32,
        index: LoopIndex,
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
        fields: Vec<(String, FieldOperand)>,
    },
    /// `let BINDING = RECORD.FIELD;` — read one field into an integer, or —
    /// since card `AGGREGATE-OWNED-FIELD`, when the field is declared `bytes`
    /// — **move the owned buffer out** of the record into a plain binding.
    ///
    /// For a scalar field the result is an ordinary scalar of this route's
    /// environment, which is what makes a field readable, renderable and
    /// indexable with **no** new operand form: `source.std-io.v1`'s existing
    /// `Scalar` operand already denotes it, exactly as card `LOOP-V1` found for
    /// a loop index. For an owned field the statement is the partial move
    /// ([`OwnershipOp::MoveOut`]): the binding is a buffer this entry now owns
    /// and the field is a hole a later `drop RECORD;` tolerates. Which of the
    /// two a statement is, is decided by the declared type and nowhere else.
    ///
    /// Card `W48-QR-USABLE-RECORDS`: the field is a **path** (`o.inner.a`,
    /// never empty), each component but the last a field whose type is a
    /// record; a one-element path is the field read it always was. A path
    /// ending at a record field of a record of scalars copies that sub-record.
    /// This is true of every field-path event below.
    Field {
        binding: String,
        record: String,
        path: Vec<String>,
    },
    /// `RECORD.FIELD[INDEX] = VALUE;` — write one byte through an owned field
    /// (card `AGGREGATE-OWNED-FIELD`). [`Self::Store`] at a field path.
    FieldStore {
        record: String,
        path: Vec<String>,
        index: BufferOperand,
        value: BufferOperand,
    },
    /// `let BINDING = RECORD.FIELD[INDEX];` — [`Self::Load`] at a field path.
    FieldLoad {
        binding: String,
        record: String,
        path: Vec<String>,
        index: BufferOperand,
    },
    /// `let BINDING = extent(RECORD.FIELD);` — [`Self::Extent`] at a field path.
    FieldExtent {
        binding: String,
        record: String,
        path: Vec<String>,
    },
    /// `drop RECORD.FIELD;` — release one owned field, leaving a hole
    /// ([`OwnershipOp::DropField`]). `drop RECORD;` is [`Self::Release`] naming
    /// the record, and releases every live owned field.
    FieldRelease { record: String, path: Vec<String> },
    /// `RECORD.FIELD = VALUE;` — **the field write** (card
    /// `W48-QR-USABLE-RECORDS`, which lifted `AGGREGATE-FIELD-WRITE`).
    ///
    /// At a scalar field the value is checked against the field's declared
    /// type by the construction's own typing ([`BufferEvaluation::field_value`]:
    /// a literal range-checked, `true`/`false` at a `bool`, or a name of
    /// exactly that type) and replaces that field's value and nothing else
    /// ([`RecordValue::write_path`]). At an owned field the value names an
    /// owned buffer and the statement is **the refill** —
    /// [`OwnershipOp::RefillField`], admitted only into a hole. On a record
    /// with an owned field anywhere, a scalar write asks the IR that the
    /// record is live ([`OwnershipOp::AccessRecord`]), so a write through a
    /// released record is a use after drop. A path ending at a record field
    /// is refused by name (`W48-QR-RECORD-FIELD-WHOLE-WRITE`).
    FieldWrite {
        record: String,
        path: Vec<String>,
        value: FieldOperand,
    },
    /// `let BINDING = SOURCE;` — **the copy bind** (card
    /// `OWNED-FIELD-ROUTE-REACH`, wave 41).
    ///
    /// One statement asking one question: *may a second binding hold what this
    /// name holds?* The answer is the ownership IR's
    /// ([`OwnershipOp::Copy`]) for every name the ledger holds a binding for,
    /// and it is always **no** — an owned buffer, a record with an owned field
    /// (hollow or not), a moved binding, a dropped one. For a name the ledger
    /// holds nothing for it is **yes**, and that is not a second rule: a place
    /// with no binding owns no resource, so the copy moves no duty
    /// (`QCore.OwnedFieldCopyReach.copy_refuses_every_bound_place` and its
    /// neutrality sibling).
    ///
    /// Why the statement exists: wave 39's non-copyability rule was reachable
    /// from **no source program** — `OwnershipOp::Copy` was constructed at four
    /// places in the tree, all unit tests (`OWNERSHIP-COPY-OP-UNPRODUCED`,
    /// measured by `STABILITY-REEVAL-4`). A rule no program can reach is
    /// decided by hand-built operation lists, and constraint 15k's "a v1 record
    /// is a plain copyable value" was an assertion nothing could check in
    /// either direction. This statement makes both halves real.
    ///
    /// **Deliberately not a `buffer_use`.** Route selection and the composed
    /// string route both read [`crate::std_buffer`] statements through
    /// `source_std_buffer::buffer_use`, and adding this shape there would make
    /// a module select this route for a statement that is a plain value bind
    /// on every route. It is read by `lower_let` alone, so `let t = s;` on the
    /// composed string route keeps the `Q068` it always had.
    CopyBind { binding: String, source: String },
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
    /// The entry row's float comparison effects, `partial_eq` and
    /// `partial_ord` (card `W48-QR-FLOAT-BUFFER-PAYLOADS`): what the row
    /// permits a value statement to compare. Empty for every program before
    /// the card, so no float comparison is admitted without them.
    pub float_effects: st::FloatEffects,
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
    /// The `i32` fields, in declared order.
    pub fields: Vec<(String, i32)>,
    /// The scalar fields of every other §G02t type, in declared order, each
    /// with its type (card `W48-QR-SCALAR-TYPES-R3`). Empty for every record
    /// declared before it.
    pub typed: Vec<(String, ScalarType, i128)>,
    /// The owned fields, in declared order, each with the name of the buffer
    /// binding the construction moved into it (card `AGGREGATE-OWNED-FIELD`).
    /// Empty for every record of scalars.
    pub owned: Vec<(String, String)>,
    /// The record fields, in declared order, each with the record binding the
    /// construction copied — or, for a record with an owned field, moved —
    /// into it (card `W48-QR-USABLE-RECORDS`). That binding's own construction
    /// publishes its fields. Empty for every record declaring no record field.
    pub records: Vec<(String, String)>,
}

/// One checked field write (card `W48-QR-USABLE-RECORDS`): which record,
/// which field path, and what it now holds — a scalar's type and number, or
/// the buffer a refill moved in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordWrite {
    pub event: usize,
    pub record: String,
    /// The path below the record, spelled `inner.a`.
    pub field: String,
    pub value: WrittenValue,
}

/// What a field write stored (card `W48-QR-USABLE-RECORDS`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WrittenValue {
    /// A scalar of the field's declared type.
    Scalar { ty: ScalarType, value: i128 },
    /// The buffer a refill moved into an owned field, with its extent.
    Refill { buffer: String, extent: u32 },
}

/// One partial move: an owned field's buffer leaving its record for a plain
/// binding (card `AGGREGATE-OWNED-FIELD`), published with the extent the
/// binding now owns so a reader checks it against the acquisition it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldMove {
    pub event: usize,
    pub record: String,
    pub field: String,
    pub binding: String,
    pub extent: u32,
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

/// One checked read of a field of a §G02t type other than `i32` (card
/// `W48-QR-SCALAR-TYPES-R3`): the [`RecordProjection`] of a typed field, with
/// the type the binding now has. Published beside the `i32` reads rather than
/// folded into them, so every certificate written before the card is
/// value-identical.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedProjection {
    pub event: usize,
    pub record: String,
    pub type_name: String,
    pub field: String,
    pub ty: ScalarType,
    pub value: i128,
}

/// One admitted write lend, published with the **window** over which the
/// owner's authority is transferred and the footprint the loan holds (card
/// `SLICE-PERMISSIONS-WRITE-LEND`).
///
/// `through` is the index of the last top-level event naming the loan at any
/// depth; the window `[event, through]` is exact over a flat sequence and,
/// where a `for` is the last use, covers every iteration of it (card
/// `LOOPS-ON-COMPOSED-ROUTES`). `low`/`high` are the owner's coordinates, so a
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
    /// Every release, in program order. A release through an owned field is
    /// published by its `RECORD.FIELD` path, and a whole-record `drop` publishes
    /// one entry per live owned field it releases, in declaration order (card
    /// `AGGREGATE-OWNED-FIELD`).
    pub releases: Vec<String>,
    /// Every partial move out of an owned field, in program order (card
    /// `AGGREGATE-OWNED-FIELD`). Empty for every program written before it.
    pub moves: Vec<FieldMove>,
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
    /// Every read of a field of a type other than `i32`, in program order
    /// (card `W48-QR-SCALAR-TYPES-R3`).
    pub typed_projections: Vec<TypedProjection>,
    /// Every field write, in program order (card `W48-QR-USABLE-RECORDS`).
    /// Empty for every program written before it.
    pub writes: Vec<RecordWrite>,
    /// The final scalar environment, so a reader can see what each rendered
    /// name denoted.
    pub scalars: BTreeMap<String, i32>,
    /// The final environment of every name of a §G02t type other than `i32`,
    /// with its type (card `W48-QR-SCALAR-TYPES-R3`). A name is in exactly one
    /// of the two maps: the typing function of this route is
    /// `scalars ↦ i32, typed_scalars ↦ its type`.
    pub typed_scalars: BTreeMap<String, (ScalarType, i128)>,
    /// The recorded target this certificate depends on: present exactly when
    /// the program names `isize` or `usize`, whose width only the builder's
    /// record supplies (card `W48-QR-SCALAR-TYPES-R3`). A named assumption of
    /// the result, never a guess.
    pub target_assumption: Option<RecordedTarget>,
    /// Every line this program produces, in program order across both streams.
    pub lines: Vec<ObservedLine>,
    /// How many `flush` barriers the program performs.
    pub flushes: usize,
    /// The renderings actually used.
    pub renderings: BTreeSet<Rendering>,
    /// The float comparison effects the program's value statements used
    /// (card `W48-QR-FLOAT-BUFFER-PAYLOADS`), each declared by the entry's
    /// row — the decision refuses an undeclared one. Empty for every program
    /// without a float comparison.
    pub float_comparisons: st::FloatEffects,
}

/// The refusals the write lend owns (card `SLICE-PERMISSIONS-WRITE-LEND`),
/// each landing on a condition an existing `source.buffer.v1` code already
/// names — the `Q-NAMESPACE-DECISION` ice box forbids a new allocation, and the
/// widenings are stated in spec §L21 and in the card's report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LendRefusal {
    /// `Q051`: a shape the lend's window model cannot hold — a crossed pair of
    /// windows, an acquisition inside a window, a lend binding shadowing a
    /// live name, and (card `LOOPS-ON-COMPOSED-ROUTES`, which lifted the
    /// blanket `for` refusal) a lend inside a `for` body or a `for` body that
    /// acquires or releases in an entry that lends.
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
    /// A refusal of the §G02t typing rules on this route (card
    /// `W48-QR-SCALAR-TYPES-R3`), reported at this pass's own codes through
    /// [`TypedRefusal::code`] — no code is allocated.
    Typed { event: usize, refusal: TypedRefusal },
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
            Self::Typed { refusal, .. } => refusal.code(),
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
            | Self::Typed { event, .. }
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
            Self::Typed { refusal, .. } => refusal.fmt(f),
        }
    }
}

/// The §G02t typing refusals of this route (card `W48-QR-SCALAR-TYPES-R3`).
///
/// Every value of this route is a check-time constant, so each rule is decided
/// at check time: a position that takes an `i32` refuses a name of another
/// type, a conversion that does not fit refuses, and a target-width type with
/// no recorded target refuses. None allocates a code; each widens a condition
/// of the band this route already owns (`Q051` shape, `Q052` "a name that is
/// not what this position takes", `Q053` range).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedRefusal {
    /// `Q052`: a name of a type other than `i32` at a position that takes an
    /// `i32` — an index, a stored byte, a lend bound.
    NotAnI32 { name: String, found: ScalarType },
    /// `Q052`: a conversion to or from `bool`, which §G02t does not define.
    NoConversion { from: ScalarType, to: ScalarType },
    /// `Q053`: a conversion whose value is not a value of its target type.
    ConversionOutOfRange {
        from: ScalarType,
        to: ScalarType,
        value: i128,
    },
    /// `Q051`: `isize` or `usize` named in a module checked with no recorded
    /// target; the width is never guessed.
    NoRecordedTarget { ty: ScalarType },
    /// `Q052`: `hex` of a `bool`, which has no bit pattern to render.
    NoHexRendering { name: String },
    /// A value statement the shared decision refuses (card
    /// `W48-QR-SCALAR-TYPES-R5`), at the condition its fault is: `Q053` a
    /// fault decided now or a literal outside its type (the range condition),
    /// `Q051` a target width with no record, `Q052` every typing refusal —
    /// mixed operand types, `bool` arithmetic, negation or ordering.
    Value(crate::scalar_values::ValueRefusal),
}

impl TypedRefusal {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoRecordedTarget { .. } => "Q051",
            Self::NotAnI32 { .. } | Self::NoConversion { .. } | Self::NoHexRendering { .. } => {
                "Q052"
            }
            Self::ConversionOutOfRange { .. } => "Q053",
            Self::Value(refusal) => {
                use crate::scalar_values::ValueRefusal;
                match refusal {
                    ValueRefusal::NoRecordedTarget { .. } => "Q051",
                    // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: a float literal
                    // outside its type is the same range condition.
                    ValueRefusal::StaticFault { .. }
                    | ValueRefusal::LiteralOutOfRange { .. }
                    | ValueRefusal::FloatLiteral { .. } => "Q053",
                    _ => "Q052",
                }
            }
        }
    }
}

impl fmt::Display for TypedRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAnI32 { name, found } => write!(
                f,
                "`{name}` is a `{found}` and this position takes an `i32`; there is no implicit \
                 conversion, so bind `let v = {name} as i32;` first where the number is meant as \
                 an `i32`"
            ),
            Self::NoConversion { from, to } => write!(
                f,
                "`{from} as {to}` is not a conversion: `bool` is not an integer, so nothing \
                 converts to or from it"
            ),
            // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: a float source is named by
            // its `dec` rendering, never its bit pattern, and its conversion
            // truncates toward zero before the range is asked.
            Self::ConversionOutOfRange { from, to, value } if from.is_float() => write!(
                f,
                "`{}` (a `{from}`) truncated toward zero is not a value of `{to}`, and NaN and \
                 the infinities are no integer's; `as` raises `ArithmeticError::OutOfRange` \
                 here, and on `source.buffer.v1` every value is a check-time constant, so the \
                 conversion is refused rather than run",
                st::render(false, *from, *value, None).unwrap_or_else(|| value.to_string())
            ),
            Self::ConversionOutOfRange { from, to, value } => write!(
                f,
                "`{value}` (a `{from}`) is not a value of `{to}`; `as` is the same number at the \
                 new type or no value at all, and on `source.buffer.v1` every value is a \
                 check-time constant, so a conversion that does not fit is refused here rather \
                 than truncated"
            ),
            Self::NoRecordedTarget { ty } => write!(
                f,
                "`{ty}` takes its width from a recorded target, none of which was supplied; the \
                 width is never guessed (supply one with `--recorded-target FILE`)"
            ),
            Self::NoHexRendering { name } => write!(
                f,
                "`{name}` is a `bool`, which renders `true`/`false` under `dec` and has no `hex` \
                 rendering"
            ),
            Self::Value(refusal) => refusal.fmt(f),
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
                // Card `AGGREGATE-OWNED-FIELD`: a release through a field, and
                // a whole-record release, free storage exactly as `drop NAME;`
                // does; a store through a field mutates exactly as a store
                // does. A construction *moves* a buffer in and a field move
                // moves one out, and a move allocates nothing.
                BufferEvent::Acquire { .. }
                | BufferEvent::Release { .. }
                | BufferEvent::FieldRelease { .. } => {
                    effects.insert(Effect::Alloc);
                }
                // Card `W48-QR-USABLE-RECORDS`: a field write mutates an object
                // of the program — the record — exactly as a byte store mutates
                // a buffer, so it derives `Mut`; a read of the same field
                // observes and derives nothing. A refill moves a buffer in and
                // allocates nothing.
                BufferEvent::Store { .. }
                | BufferEvent::FieldStore { .. }
                | BufferEvent::FieldWrite { .. } => {
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
                // **A copy bind derives nothing** (card
                // `OWNED-FIELD-ROUTE-REACH`): an admitted copy binds a value
                // that already exists — a scalar, or a record of scalars in
                // automatic storage — so it allocates nothing and mutates no
                // object of the program, exactly as `let n = 5;` does. A copy
                // of anything the ledger holds is refused before this row is
                // read, so no arm of this walk can be reached by one.
                // **A conversion derives nothing** (card
                // `W48-QR-SCALAR-TYPES-R3`): it binds a number that already
                // exists at another type, exactly as `let n = 5;` binds one.
                BufferEvent::Construct { .. }
                | BufferEvent::Field { .. }
                | BufferEvent::FieldLoad { .. }
                | BufferEvent::FieldExtent { .. }
                | BufferEvent::Bind { .. }
                | BufferEvent::Convert { .. }
                // Card `W48-QR-SCALAR-TYPES-R5`: a value statement derives
                // nothing, for the conversion's reason.
                | BufferEvent::Compute { .. }
                | BufferEvent::CopyBind { .. }
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
    let types: BTreeMap<&str, &RecordType> = program
        .records
        .iter()
        .map(|record| (record.name.as_str(), record))
        .collect();

    struct Walk<'a> {
        acquired: &'a BTreeSet<String>,
        /// The module's record types by name, so a construction knows which
        /// of its fields are owned (card `AGGREGATE-OWNED-FIELD`).
        types: &'a BTreeMap<&'a str, &'a RecordType>,
        /// Record binding → declared type, for every record this walk
        /// composed **with at least one owned field**. A record of scalars is
        /// never entered here, because it contributes no operation.
        composed: BTreeMap<String, String>,
        /// Plain bindings that own a buffer moved **out** of a field: custody
        /// names exactly as an acquisition's are, for every later statement.
        moved_out: BTreeSet<String>,
        operations: Vec<OwnershipOp>,
        gates: Vec<LoopCustodyGate>,
        next_id: ResourceId,
        next_scope: u32,
    }

    impl Walk<'_> {
        fn names(&self, name: &String) -> bool {
            self.acquired.contains(name) || self.moved_out.contains(name)
        }

        /// The declared type at `record.path` for a record this walk composed
        /// — `None` for a record it did not compose (a record of scalars
        /// contributes no operation) or a path its type does not declare (the
        /// value half refuses it by name).
        fn composed_field(&self, record: &str, path: &[String]) -> Option<FieldType> {
            let type_name = self.composed.get(record)?;
            source_record::field_type_at(
                &|name: &str| self.types.get(name).copied(),
                record,
                type_name,
                path,
            )
            .ok()
        }

        /// Whether `record.path` is an owned leaf of a record this walk
        /// composed — the one question every field-path custody arm below
        /// asks. A path that is not one contributes no operation, and the
        /// value half refuses the statement by name.
        fn owned_field(&self, record: &str, path: &[String]) -> bool {
            self.composed_field(record, path) == Some(FieldType::Bytes)
        }

        /// Whether a value of `type_name` owns anything at any depth (card
        /// `W48-QR-USABLE-RECORDS`).
        fn owns(&self, type_name: &str) -> bool {
            source_record::owns(&|name: &str| self.types.get(name).copied(), type_name)
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
                    // --- Card `AGGREGATE-OWNED-FIELD`: the aggregate arms. ---
                    //
                    // Each builds one operation of the path-extended IR and
                    // decides nothing: which fields are owned is the declared
                    // type's, and every custody rule — the move in, the hole
                    // a move out leaves, the declaration-order drop-through,
                    // the leak that names a path — is `ownership::check_ownership`'s.
                    //
                    // A construction of a type declaring an owned field is a
                    // `Compose` of exactly its owned fields in declared order,
                    // each moving the named buffer in. A field supplied
                    // something that is not a custody name — a literal, a
                    // scalar, a record — contributes no move, on this
                    // function's standing rule (the checker is asked about
                    // **resources**, and the value half refuses such a field
                    // by name at `Q052`); and a record of scalars contributes
                    // **no operation at all**, which is what keeps every
                    // program written before the card byte-identical.
                    //
                    // Card `W48-QR-USABLE-RECORDS`: a type holding a record
                    // field whose type owns is a custody shape too, and is a
                    // `ComposeNested` whose record part moves the named
                    // aggregate — every leaf of it — under the field. A type
                    // declaring no record field builds exactly the `Compose`
                    // it always built, so every program written before the
                    // card presents the IR with the operation list it always
                    // did.
                    BufferEvent::Construct {
                        binding,
                        type_name,
                        fields,
                    } => {
                        if let Some(declared) = self.types.get(type_name.as_str()).copied()
                            && self.owns(type_name)
                        {
                            let supplied = |field: &str| {
                                fields.iter().find(|(name, _)| name == field).and_then(
                                    |(_, operand)| match operand {
                                        FieldOperand::Name(from) => Some(from),
                                        FieldOperand::Literal(_)
                                        | FieldOperand::Bool(_)
                                        | FieldOperand::Float(_) => None,
                                    },
                                )
                            };
                            let nests = declared
                                .fields
                                .iter()
                                .any(|field| matches!(field.field_type, FieldType::Record(_)));
                            if nests {
                                let mut parts = Vec::new();
                                for field in &declared.fields {
                                    match &field.field_type {
                                        FieldType::Bytes => {
                                            if let Some(from) = supplied(&field.name)
                                                && self.names(from)
                                            {
                                                parts.push(ownership::ComposePart::Buffer {
                                                    field: field.name.clone(),
                                                    from: Self::owner(renames, from),
                                                });
                                            }
                                        }
                                        FieldType::Record(inner) if self.owns(inner) => {
                                            if let Some(from) = supplied(&field.name)
                                                && self.composed.contains_key(from)
                                            {
                                                parts.push(ownership::ComposePart::Record {
                                                    field: field.name.clone(),
                                                    from: Self::owner(renames, from),
                                                });
                                            }
                                        }
                                        FieldType::Record(_) | FieldType::Scalar(_) => {}
                                    }
                                }
                                self.operations.push(OwnershipOp::ComposeNested {
                                    owner: binding.clone(),
                                    parts,
                                });
                            } else {
                                let moved: Vec<(String, String)> = declared
                                    .owned_field_names()
                                    .filter_map(|owned| {
                                        supplied(owned).filter(|from| self.names(from)).map(
                                            |from| (owned.to_owned(), Self::owner(renames, from)),
                                        )
                                    })
                                    .collect();
                                self.operations.push(OwnershipOp::Compose {
                                    owner: binding.clone(),
                                    fields: moved,
                                });
                            }
                            self.composed.insert(binding.clone(), type_name.clone());
                        }
                    }
                    // The partial move: a read of an owned field is the
                    // `MoveOut`, and its binding is a custody name from here on.
                    // Card `W48-QR-USABLE-RECORDS`: a read of a scalar field
                    // — or a copy of a sub-record of scalars — of a composed
                    // record asks that the record is live.
                    BufferEvent::Field {
                        binding,
                        record,
                        path,
                    } => {
                        if self.owned_field(record, path) {
                            self.operations.push(OwnershipOp::MoveOut {
                                from: FieldPath::nested(record.clone(), path.clone()),
                                to: binding.clone(),
                            });
                            self.moved_out.insert(binding.clone());
                        } else if self.composed_field(record, path).is_some() {
                            self.operations
                                .push(OwnershipOp::AccessRecord(record.clone()));
                        }
                    }
                    // Card `W48-QR-USABLE-RECORDS`: the field write. At an
                    // owned leaf it is the refill, moving the named buffer in;
                    // at a scalar field of a composed record it asks that the
                    // record is live. A write to a record of scalars
                    // contributes nothing, like every other operation on one.
                    BufferEvent::FieldWrite {
                        record,
                        path,
                        value,
                    } => {
                        if self.owned_field(record, path) {
                            if let FieldOperand::Name(from) = value
                                && self.names(from)
                            {
                                self.operations.push(OwnershipOp::RefillField {
                                    from: Self::owner(renames, from),
                                    into: FieldPath::nested(record.clone(), path.clone()),
                                });
                            }
                        } else if self.composed_field(record, path).is_some() {
                            self.operations
                                .push(OwnershipOp::AccessRecord(record.clone()));
                        }
                    }
                    BufferEvent::FieldStore { record, path, .. }
                        if self.owned_field(record, path) =>
                    {
                        self.operations
                            .push(OwnershipOp::WriteField(FieldPath::nested(
                                record.clone(),
                                path.clone(),
                            )));
                    }
                    BufferEvent::FieldLoad { record, path, .. }
                    | BufferEvent::FieldExtent { record, path, .. }
                        if self.owned_field(record, path) =>
                    {
                        self.operations
                            .push(OwnershipOp::ReadField(FieldPath::nested(
                                record.clone(),
                                path.clone(),
                            )));
                    }
                    BufferEvent::FieldRelease { record, path }
                        if self.owned_field(record, path) =>
                    {
                        self.operations
                            .push(OwnershipOp::DropField(FieldPath::nested(
                                record.clone(),
                                path.clone(),
                            )));
                    }
                    // `drop p;` of a composed record is the plain `Drop` — the
                    // IR sees an aggregate binding and walks its fields.
                    BufferEvent::Release { name } if self.composed.contains_key(name) => {
                        self.operations.push(OwnershipOp::Drop(name.clone()));
                    }
                    // --- Card `OWNED-FIELD-ROUTE-REACH`: the copy bind. ---
                    //
                    // `let g = f;` asks the IR whether a second binding may
                    // hold what `f` holds, and it asks **only for a name the
                    // ledger holds a binding for**: an acquired buffer, a
                    // binding a partial move gave a buffer to, or a record
                    // this entry composed with an owned field. For any other
                    // name — a scalar, a record of scalars, a loan — no
                    // operation is built, because `copy_owned` is a question
                    // about a *place in the ledger* and a place that is not in
                    // it owns nothing; those are the value half's, at its own
                    // codes.
                    //
                    // This is the one arm that makes `OwnershipOp::Copy`
                    // producible from source at all. Every answer it can
                    // receive is a refusal: `CopyOfOwned` for an owner and for
                    // an aggregate hollow or not, `UseAfterMove` for a moved
                    // binding, `UseAfterDrop` for a dropped one. That is the
                    // rule, not a special case of it.
                    BufferEvent::CopyBind { source, .. }
                        if self.names(source) || self.composed.contains_key(source) =>
                    {
                        self.operations
                            .push(OwnershipOp::Copy(Self::owner(renames, source)));
                    }
                    BufferEvent::Repeat {
                        variable,
                        count,
                        body,
                        ..
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
                    // A custody position naming nothing the checker was told
                    // about — a name that is not a buffer, a field that is not
                    // owned — contributes no operation; the value half refuses
                    // the statement by name (`NotABuffer`, `FieldIsNotABuffer`).
                    BufferEvent::Store { .. }
                    | BufferEvent::Load { .. }
                    | BufferEvent::Extent { .. }
                    | BufferEvent::Release { .. }
                    | BufferEvent::FieldStore { .. }
                    | BufferEvent::FieldLoad { .. }
                    | BufferEvent::FieldExtent { .. }
                    | BufferEvent::FieldRelease { .. } => {}
                    // Unreachable: a program with a lend takes
                    // `lend_custody_operations` before this walk runs. Kept
                    // total with **no** operation, which cannot admit anything
                    // — a loan this walk did not open makes every later use of
                    // its binding an unknown name to the checker.
                    BufferEvent::Lend { .. } => {}
                    // A record of scalars contributes no custody operation at
                    // all (card `AGGREGATES-V1`): the `Construct` and `Field`
                    // arms above push one only for a declared owned field.
                    //
                    // Card `OWNED-FIELD-ROUTE-REACH`: a copy bind whose source
                    // the ledger holds nothing for is in exactly that class —
                    // the guarded arm above did not claim it, so it contributes
                    // no operation and the value half decides what it binds.
                    //
                    // Card `W48-QR-SCALAR-TYPES-R3`: a conversion names a
                    // scalar and binds one, so it is in that class too.
                    BufferEvent::Bind { .. }
                    | BufferEvent::Convert { .. }
                    | BufferEvent::Compute { .. }
                    | BufferEvent::CopyBind { .. }
                    | BufferEvent::Line(_)
                    | BufferEvent::Return => {}
                }
            }
        }
    }

    let mut walk = Walk {
        acquired: &acquired,
        types: &types,
        composed: BTreeMap::new(),
        moved_out: BTreeSet::new(),
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
/// `through` is the index of the **last top-level event naming the loan at any
/// depth** — a store, a load, an extent read, or a refused later spelling,
/// whether written at the top level or inside a `for` body that top-level
/// event contains (card `LOOPS-ON-COMPOSED-ROUTES`, [`last_uses`]) — and
/// `event` the lend's own, so `[event, through]` is the loan's lexical window.
/// The bounds are resolved from the lowering-visible constant tier and
/// **clamped into `[0, extent]`** when the spelling is out of range: the value
/// half owns that refusal (`Q053`), and the projection stays total so one fault
/// produces one report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LendWindow {
    pub event: usize,
    pub binding: String,
    pub buffer: String,
    pub low: u32,
    pub high: u32,
    pub through: usize,
}

/// A name the window function met at a custody or access position, classified
/// by whether the window's **key space** can express it (card
/// `COMPOSED-WINDOW-REACH`, wave 41).
///
/// The window map is keyed by plain binding names, because that is the only
/// shape a lend binding has: `let NAME = lend(BUFFER, LOW, HIGH);` binds one
/// identifier. A buffer addressed through a record's field path is a name of a
/// second kind, and the classification exists so the difference is a value the
/// callers must decide about rather than an arm that silently contributes
/// nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WindowName<'a> {
    /// A plain binding — an owner this entry acquired, or a loan it opened.
    Binding(&'a str),
    /// A buffer addressed as `RECORD.FIELD` (card `AGGREGATE-OWNED-FIELD`'s
    /// four field-path events). **The window's key space cannot express this
    /// name**, so a window computed over a program containing one would be a
    /// window that forgot a use — the shortening, and therefore admitting,
    /// direction. It is refused by [`window_unnameable_events`] rather than
    /// dropped.
    FieldPath { record: &'a str, path: &'a [String] },
}

/// Every buffer or loan name one event names at a custody or access position,
/// **at any depth** (card `LOOPS-ON-COMPOSED-ROUTES`).
///
/// A `Repeat` names nothing itself and contributes every name its body names,
/// however deeply nested: under the runs-many-times reading a body event
/// occurs at every iteration of every enclosing loop, and the loop is one
/// top-level statement, so a name a body mentions is a name the **loop's**
/// top-level index mentions. This is the one function the window computation
/// reads names through, so a widening of the grammar that forgets to extend it
/// would shorten a window — and the exhaustive `match` makes that a compile
/// error rather than an unsoundness.
///
/// Card `COMPOSED-WINDOW-REACH` (wave 41) changed what the four **field-path**
/// arms do. They used to sit in the same empty arm as `Bind` and `Line`,
/// justified in-comment by a refusal that runs earlier
/// (`OWNED-FIELD-LEND-WINDOW`) — so the function was correct only because a
/// construct was unreachable, and the day that refusal is lifted it would have
/// silently forgotten every field-path name and produced a **shorter** window
/// (`WINDOW-FIELD-ARM-UNREACHED`). They now yield [`WindowName::FieldPath`],
/// which [`window_unnameable_events`] turns into a refusal by name. Two arms
/// the same residual counted with them — `Construct` and `Field` — are
/// **measured reachable today** (a record whose fields are all `i32` beside a
/// lend, and one inside a loop body of a lending entry, are both admitted) and
/// are correctly empty: a scalar construction and a scalar field read name no
/// buffer and no loan at all.
fn custody_names_at_any_depth<'a>(event: &'a BufferEvent, into: &mut Vec<WindowName<'a>>) {
    match event {
        BufferEvent::Store { name, .. }
        | BufferEvent::Load { name, .. }
        | BufferEvent::Extent { name, .. }
        | BufferEvent::Release { name }
        | BufferEvent::Lend { name, .. } => into.push(WindowName::Binding(name.as_str())),
        // Card `OWNED-FIELD-ROUTE-REACH`: a copy bind **names its source**, so
        // it is a use of that name and a loan's window runs through it. Pushed
        // rather than skipped in the refusing direction: a copy of a loan is
        // refused at `Q055` by the value half, and a window that ended before
        // the statement naming the loan would be the one mistake this function
        // exists to make impossible. Carried onto `COMPOSED-WINDOW-REACH`'s
        // classified key space by the integrator at the wave-41 merge: a copy
        // bind names a plain binding, never a field path, so it is a
        // `WindowName::Binding` and the two cards' rules compose unchanged.
        BufferEvent::CopyBind { source, .. } => into.push(WindowName::Binding(source.as_str())),
        // Card `W48-QR-SCALAR-TYPES-R3`: a conversion of a name **names its
        // source**, so it is a use of that name, pushed in the refusing
        // direction for the copy bind's reason: a conversion of a loan is
        // refused at `Q055` by the value half, and a window ending before the
        // statement naming the loan would be the one mistake to avoid.
        BufferEvent::Convert {
            operand: BufferOperand::Scalar(source),
            ..
        } => into.push(WindowName::Binding(source.as_str())),
        BufferEvent::Convert {
            operand: BufferOperand::Literal(_),
            ..
        } => {}
        // Card `W48-QR-SCALAR-TYPES-R5`: a value statement names each of its
        // operands, pushed for the conversion's reason.
        BufferEvent::Compute { operation, .. } => {
            for name in operation.names() {
                into.push(WindowName::Binding(name));
            }
        }
        BufferEvent::Repeat { body, .. } => {
            for event in body {
                custody_names_at_any_depth(event, into);
            }
        }
        // Card `AGGREGATE-OWNED-FIELD`'s four field-path events address a
        // buffer the window's key space cannot name. Reported rather than
        // dropped; see [`WindowName::FieldPath`].
        BufferEvent::FieldStore { record, path, .. }
        | BufferEvent::FieldLoad { record, path, .. }
        | BufferEvent::FieldExtent { record, path, .. }
        | BufferEvent::FieldRelease { record, path } => into.push(WindowName::FieldPath {
            record: record.as_str(),
            path: path.as_slice(),
        }),
        // Card `W48-QR-USABLE-RECORDS`: a field write **names its value** when
        // that is a name, pushed for the copy bind's reason — a loan written
        // into a field is refused at `Q055` by the value half, and a window
        // ending before the statement naming the loan would be the mistake
        // this function exists to prevent. Its target is a record's field: at
        // a scalar field that is no buffer and no loan, and at an owned field
        // — the refill — the record is one with an owned field, whose
        // construction the owned-field gate refuses beside any lend before a
        // window is computed (`OWNED-FIELD-LEND-WINDOW`), so no window is ever
        // computed over a refill.
        BufferEvent::FieldWrite { value, .. } => {
            if let FieldOperand::Name(name) = value {
                into.push(WindowName::Binding(name.as_str()));
            }
        }
        // These name no buffer and no loan. `Acquire` names a buffer, but a
        // freshly acquired one: a name the entry has not bound before, so it
        // is no loan of this window map (a rebinding is `Q012`, refused by
        // name resolution before any route sees the program), and an
        // acquisition inside a live window is refused separately
        // (`WRITE-LEND-NESTED-ACQUIRE`). `Construct` and `Field` name a
        // **record** and a field of it; for a scalar field the value is an
        // ordinary integer of this route's environment, and for an owned one
        // the statement is refused before any window is computed.
        BufferEvent::Bind { .. }
        | BufferEvent::Acquire { .. }
        | BufferEvent::Construct { .. }
        | BufferEvent::Field { .. }
        | BufferEvent::Line(_)
        | BufferEvent::Return => {}
    }
}

/// The top-level statements whose events name a buffer the window's key space
/// **cannot express**, each with the field path it names (card
/// `COMPOSED-WINDOW-REACH`, wave 41).
///
/// This is the fail-closed half of [`custody_names_at_any_depth`]'s
/// classification. A window computed over a program containing one of these
/// would silently omit that name, giving a **shorter** window than the program
/// deserves — and a shorter window is a *longer* stretch in which the owner may
/// write bytes a loan holds, which is the admitting direction invariant 1
/// constrains.
///
/// Today no source program reaches it: `check_buffer_program` refuses an
/// owned-field record beside a lend (`OWNED-FIELD-LEND-WINDOW`) and an
/// owned-field statement inside a loop body (`OWNED-FIELD-LOOP-BODY`), both
/// before [`lend_windows`] runs. That ordering is exactly what made the old
/// empty arm *look* safe. The refusal here does not depend on it: it is the
/// window function's own gate, reached by any caller — a model-level one
/// included — so lifting either of those two refusals cannot shorten a window
/// by accident. Whoever lifts them must give the four arms real windows and
/// retire this gate deliberately, which is what `WINDOW-FIELD-ARM-UNREACHED`
/// asked for.
#[must_use]
pub fn window_unnameable_events(events: &[BufferEvent]) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (index, event) in events.iter().enumerate() {
        let mut names = Vec::new();
        custody_names_at_any_depth(event, &mut names);
        for name in names {
            if let WindowName::FieldPath { record, path } = name {
                found.push((index, source_record::spell_path(record, path)));
            }
        }
    }
    found
}

/// **The window function over iterations** (card `LOOPS-ON-COMPOSED-ROUTES`):
/// for each of `bindings`, the index of the last **top-level** event that names
/// it at any depth.
///
/// Stated on its own so the theorem about it names one function. An event
/// inside a `for` body at top-level index `k` is read as occurring at every
/// iteration of that loop, so a loan the body touches is live across the whole
/// loop — its window runs through `k`, and every owner access inside the
/// body, at whatever body position, is judged against it. That is the
/// refusing direction relative to the unrolled program, and it is exactly
/// where the top-level-only computation this replaced was **admitting**: a
/// loan used only inside a loop closed before the loop opened, and every
/// owner write in the body was admitted as "outside every live loan"
/// (`STABILITY-REEVAL-3` §4).
///
/// `QCore.LoopWindows.every_use_lies_inside_the_window` proves the window
/// covers every use of the unrolled program, and
/// `a_top_level_only_window_misses_a_body_use` that the replaced computation
/// did not; `the_unrolling_is_loop_v1s` ties the unrolling those theorems are
/// stated over to `QCore.SourceLoop.unroll`, `LOOP-V1`'s own.
#[must_use]
pub fn last_uses<'a>(
    events: &'a [BufferEvent],
    bindings: &BTreeSet<&str>,
) -> BTreeMap<&'a str, usize> {
    let mut last_use = BTreeMap::new();
    for (index, event) in events.iter().enumerate() {
        let mut names = Vec::new();
        custody_names_at_any_depth(event, &mut names);
        for name in names {
            match name {
                WindowName::Binding(name) => {
                    if bindings.contains(name) {
                        last_use.insert(name, index);
                    }
                }
                // A name this map's key space cannot hold. Skipping it here is
                // sound **only** because [`window_unnameable_events`] refuses
                // the program before a window built from this map is used
                // (card `COMPOSED-WINDOW-REACH`); it is not skipped on the
                // ground that the construct is unreachable, which is what
                // `WINDOW-FIELD-ARM-UNREACHED` measured and objected to.
                WindowName::FieldPath { .. } => {}
            }
        }
    }
    last_use
}

/// Resolve every lend of an entry to its window and footprint.
///
/// The second component collects the **tier refusals**: a bound outside the
/// lowering-visible constant tier (a byte read, a field read) cannot be stated
/// by the custody projection and is refused by name
/// (`WRITE-LEND-BOUND-FROM-BYTE`). A lend whose bounds do not resolve, or whose
/// buffer this entry never acquired, still receives a window — over-approximated
/// to the buffer's whole extent, the maximally refusing direction — so the
/// projection below is total whatever the value half goes on to say.
///
/// Only a **top-level** lend receives a window. A lend inside a `for` body is
/// a per-iteration loan, a construct this revision refuses by name before this
/// function is consulted (`LOOP-PER-ITERATION-LOAN`, `check_buffer_program`);
/// a direct model-level caller handing one in gets no window for it, which
/// cannot admit anything — the projection opens no borrow node for a lend it
/// has no window for, and every later use of its binding is then an unknown
/// name to the inherited checker.
#[must_use]
pub fn lend_windows(events: &[BufferEvent]) -> (Vec<LendWindow>, Vec<BufferError>) {
    let mut scalars: BTreeMap<String, i32> = BTreeMap::new();
    let mut extents: BTreeMap<String, u32> = BTreeMap::new();
    let mut windows = Vec::new();
    let mut errors = Vec::new();

    // Card `COMPOSED-WINDOW-REACH` (wave 41): a name the window's key space
    // cannot express is refused **here**, by the window function's own gate,
    // rather than dropped on the ground that an earlier refusal makes the
    // construct unreachable. See [`window_unnameable_events`].
    for (index, path) in window_unnameable_events(events) {
        errors.push(BufferError::Lend {
            event: index,
            refusal: LendRefusal::Shape(format!(
                "this statement addresses the buffer `{path}` through a record's field path, \
                 and a lend's window is an interval of statement indices keyed by the plain \
                 binding a lend opens; the window function cannot name a field path, so a \
                 window computed over this entry would silently omit every use of it and run \
                 shorter than the program deserves — the admitting direction — and the entry \
                 is refused by name instead (`WINDOW-FIELD-ARM-UNREACHED`)"
            )),
        });
    }

    // Card `LOOPS-ON-COMPOSED-ROUTES`: the last top-level event naming each
    // lend binding **at any depth** — `last_uses` is the window function.
    let lend_bindings: BTreeSet<&str> = events
        .iter()
        .filter_map(|event| match event {
            BufferEvent::Lend { binding, .. } => Some(binding.as_str()),
            _ => None,
        })
        .collect();
    let last_use = last_uses(events, &lend_bindings);

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
///
/// The windows are intervals of **top-level** indices, and since card
/// `LOOPS-ON-COMPOSED-ROUTES` a `through` may be the index of a `for` whose
/// body is the loan's last use. Two loans last used inside one loop therefore
/// share a `through` and read as **nested**, whatever order their body uses
/// come in: both are live across the whole loop, the enclosing borrow node
/// holds the whole unrolled body, and the encoding is faithful for the
/// iteration windows — which over-approximate the unrolled program's exact
/// windows in the refusing direction (`QCore.LoopWindows`, and the IR's
/// decision is monotone in the live loans,
/// `QCore.LoopWindows.a_longer_live_set_refuses_no_less`).
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
                    BufferEvent::Store { .. }
                    | BufferEvent::Load { .. }
                    | BufferEvent::Extent { .. } => self.access(event, &mut operations),
                    BufferEvent::Release { name } => {
                        if self.acquired.contains(name) {
                            operations.push(OwnershipOp::Drop(name.clone()));
                        }
                    }
                    // Card `LOOPS-ON-COMPOSED-ROUTES`: a `for` is projected by
                    // **unrolling its body under the loans live here** — the
                    // same loans map, `count` times, with the index bound —
                    // so every body access states its footprint against every
                    // loan whose window the loop sits inside, which is what
                    // makes the IR's decision over this list the decision
                    // over `LOOP-V1`'s unrolled program.
                    BufferEvent::Repeat {
                        variable,
                        count,
                        body,
                        ..
                    } => self.repeat(variable, *count, body, &mut operations),
                    // Unreachable from the route: `check_buffer_program` —
                    // card `AGGREGATE-OWNED-FIELD` — refuses an owned-field
                    // record beside a lend before this projection is
                    // consulted (`OWNED-FIELD-LEND-WINDOW`). A direct
                    // model-level caller gets no operation for any of them,
                    // and the refusal it already received stands. Card
                    // `W48-QR-USABLE-RECORDS`: a field write is in this class
                    // — on a record of scalars it contributes nothing, and a
                    // record with an owned field is refused beside a lend.
                    BufferEvent::Construct { .. }
                    | BufferEvent::Field { .. }
                    | BufferEvent::FieldWrite { .. }
                    | BufferEvent::FieldStore { .. }
                    | BufferEvent::FieldLoad { .. }
                    | BufferEvent::FieldExtent { .. }
                    | BufferEvent::FieldRelease { .. }
                    | BufferEvent::Line(_)
                    | BufferEvent::Return => {}
                    // Card `W48-QR-SCALAR-TYPES-R3`: a conversion contributes
                    // no operation and **no constant to this tier**: a bound or
                    // an index spelled through one stays rangeless here, the
                    // refusing direction, as a copy of a byte read does.
                    BufferEvent::Convert { .. } => {}
                    // Card `W48-QR-SCALAR-TYPES-R5`: a value statement likewise.
                    BufferEvent::Compute { .. } => {}
                    // Card `OWNED-FIELD-ROUTE-REACH`: the copy bind reaches
                    // this projection too, because an entry may both lend and
                    // copy. The question is the same one the flat walk asks —
                    // is the source a name this projection holds a binding for
                    // — and the answer is built from this walk's own
                    // environment: an acquired buffer is asked, a loan is
                    // asked (the IR answers `MoveOfBorrow`), and a scalar is
                    // not in either map so it contributes nothing.
                    BufferEvent::CopyBind { binding, source } => {
                        if self.acquired.contains(source) || self.loans.contains_key(source) {
                            operations.push(OwnershipOp::Copy(source.clone()));
                        }
                        // A copied **scalar** carries its value forward in this
                        // walk's own environment, so an index written through
                        // the copy resolves to the constant it is rather than
                        // falling back to the rangeless form. Strictly a
                        // sharpening: the fallback refuses.
                        if let Some(value) = self.scalars.get(source).copied() {
                            self.scalars.insert(binding.clone(), value);
                        }
                    }
                }
                index += 1;
            }
            operations
        }

        /// One byte access — a store, a load, or an extent read — as the
        /// footprint-stating operation the IR decides, through a loan or on
        /// the owner. Shared by the flat segment and the unrolled loop body,
        /// so an access inside a `for` is projected by the one rule a
        /// top-level one is.
        fn access(&mut self, event: &BufferEvent, operations: &mut Vec<OwnershipOp>) {
            match event {
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
                _ => {}
            }
        }

        /// Project one `for` by running its body `count` times under the loans
        /// live at the loop, with `variable` bound to the iteration number in
        /// the lowering-visible tier — so `part[i] = v` states the footprint
        /// `low + i` of the iteration it runs in, exactly as the value half
        /// and the emitted C decide it (card `LOOPS-ON-COMPOSED-ROUTES`).
        ///
        /// A body's bindings are body-local and are scoped out afterwards,
        /// the value half's own rule. The body contributes **no custody
        /// operation and opens no borrow node**: an acquisition, a release or
        /// a lend inside a `for` of an entry that lends is refused by name
        /// before this projection is consulted (`LOOP-WINDOW-BODY-CUSTODY`,
        /// `LOOP-PER-ITERATION-LOAN`), and a direct model-level caller handing
        /// one in gets no operation for it, which cannot admit anything — an
        /// acquisition the checker never saw makes every later use an unknown
        /// name, and a release it never saw leaves the owner live at the
        /// normal exit.
        fn repeat(
            &mut self,
            variable: &str,
            count: u32,
            body: &[BufferEvent],
            operations: &mut Vec<OwnershipOp>,
        ) {
            let outer: BTreeSet<String> = self.scalars.keys().cloned().collect();
            for iteration in 0..count {
                self.scalars.insert(
                    variable.to_owned(),
                    i32::try_from(iteration).expect("MAX_ITERATIONS is far inside i32"),
                );
                for event in body {
                    match event {
                        BufferEvent::Bind { binding, value } => {
                            self.scalars.insert(binding.clone(), *value);
                        }
                        BufferEvent::Store { .. }
                        | BufferEvent::Load { .. }
                        | BufferEvent::Extent { .. } => self.access(event, operations),
                        // Card `OWNED-FIELD-ROUTE-REACH`: the flat segment's
                        // arm, in a body — the same question, the same answer.
                        BufferEvent::CopyBind { binding, source } => {
                            if self.acquired.contains(source) || self.loans.contains_key(source) {
                                operations.push(OwnershipOp::Copy(source.clone()));
                            }
                            if let Some(value) = self.scalars.get(source).copied() {
                                self.scalars.insert(binding.clone(), value);
                            }
                        }
                        BufferEvent::Repeat {
                            variable,
                            count,
                            body,
                            ..
                        } => self.repeat(variable, *count, body, operations),
                        // An owned-field statement inside a body is refused
                        // by name before this projection runs
                        // (`OWNED-FIELD-LOOP-BODY`, card `AGGREGATE-OWNED-FIELD`).
                        BufferEvent::Acquire { .. }
                        | BufferEvent::Release { .. }
                        | BufferEvent::Lend { .. }
                        | BufferEvent::Construct { .. }
                        | BufferEvent::Convert { .. }
                        | BufferEvent::Compute { .. }
                        | BufferEvent::Field { .. }
                        | BufferEvent::FieldWrite { .. }
                        | BufferEvent::FieldStore { .. }
                        | BufferEvent::FieldLoad { .. }
                        | BufferEvent::FieldExtent { .. }
                        | BufferEvent::FieldRelease { .. }
                        | BufferEvent::Line(_)
                        | BufferEvent::Return => {}
                    }
                }
            }
            self.scalars.retain(|name, _| outer.contains(name));
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
    ///
    /// Card `W48-QR-SCALAR-TYPES-R3`: every name here is an **`i32`**. The
    /// environment is typed — each name has exactly one §G02t type — and this
    /// map is its `i32` part, the only part a composed route can ever fill
    /// (their own values are `i32`s and they refuse the record and conversion
    /// statements that are the only sources of another type).
    pub scalars: BTreeMap<String, i32>,
    /// The environment's part of every other §G02t type: the name's type and
    /// its number (`bool` `0`/`1`), bound by a read of a field of that type,
    /// by a conversion to it, or by a copy of such a name (card
    /// `W48-QR-SCALAR-TYPES-R3`). A name is in at most one of the two maps —
    /// a duplicate `let` is `Q012` before any route runs.
    pub typed: BTreeMap<String, (ScalarType, i128)>,
    /// The builder's recorded target, which alone gives `isize`/`usize` a
    /// range (card `W48-QR-SCALAR-TYPES-R3`). `None` refuses every value of
    /// either type.
    pub target: Option<RecordedTarget>,
    /// The record types the module declared, by name (card `AGGREGATES-V1`).
    ///
    /// Installed **before** the run and never written by it, because a record
    /// type is a declaration: a statement can construct a value of one and read
    /// a field of one, and neither can change what a type is.
    pub record_types: BTreeMap<String, RecordType>,
    /// The record values this entry constructed, by binding name. A record of
    /// scalars is a plain value, so for one this map is the whole of what
    /// holding it costs — there is no ledger entry beside it.
    pub records: BTreeMap<String, RecordValue>,
    /// The owned fields of every composed record — record binding → field →
    /// the buffer it holds and whether it still does (card
    /// `AGGREGATE-OWNED-FIELD`). The value half's mirror of what the ownership
    /// IR decides: it carries the bytes and the extent so a store through a
    /// field is checked against the same bounds rule a store through a name
    /// is, and it records liveness only so that a whole-record `drop` can
    /// **publish** which fields it released. It refuses no custody question;
    /// those are the IR's, at `Q054`.
    ///
    /// Card `W48-QR-USABLE-RECORDS`: keyed by the owned leaf's **path** below
    /// the record (`["inner", "buf"]`), so a record holding a record holding a
    /// buffer carries that buffer under its own binding.
    pub owned_fields: BTreeMap<String, BTreeMap<Vec<String>, OwnedField>>,
    /// The live write lends, by binding name (card
    /// `SLICE-PERMISSIONS-WRITE-LEND`). A loan never leaves this map, and that
    /// is not a leak: the loan's window runs to the last statement naming it,
    /// so by construction there is no later use for an expired entry to
    /// mislead — the same self-consistency the read card's `through` rests on.
    pub loans: BTreeMap<String, LoanShape>,
    /// The float comparison effects the entry's row permits (card
    /// `W48-QR-FLOAT-BUFFER-PAYLOADS`). Empty by default, so a route that
    /// does not set them admits no float comparison.
    pub float_effects: st::FloatEffects,
    /// The float comparison effects the decided value statements used.
    pub float_comparisons: st::FloatEffects,
}

/// One owned field of a composed record, as the value half carries it (card
/// `AGGREGATE-OWNED-FIELD`): the moved-in buffer's extent and bytes, and
/// whether the field still holds them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedField {
    pub extent: u32,
    pub contents: Vec<u8>,
    pub state: OwnedFieldState,
}

/// Whether an owned field still holds its buffer. A field that does not is a
/// **hole**: the partial move or the field release left it, and a later
/// whole-record `drop` skips it. The value half records this to publish what a
/// drop released; every *refusal* about a hole — a use after the move, a
/// second release — is the ownership IR's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnedFieldState {
    Live,
    Moved,
    Released,
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
    /// The field of a type other than `i32` this event read (card
    /// `W48-QR-SCALAR-TYPES-R3`).
    pub typed_projection: Option<TypedProjection>,
    /// The owned field this event moved out (card `AGGREGATE-OWNED-FIELD`).
    pub field_move: Option<FieldMove>,
    /// The owned fields this event released, by path, in declaration order:
    /// one for `drop RECORD.FIELD;`, every live one for `drop RECORD;`.
    pub field_releases: Vec<String>,
    /// The field this event wrote (card `W48-QR-USABLE-RECORDS`).
    pub write: Option<RecordWrite>,
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
    /// is [`BufferRun::repeat`]'s, which owns the index binding, the body-local
    /// scoping and the shadowing refusal. Deciding it here would put a second
    /// iteration rule beside that one. **A caller that cannot recurse must
    /// therefore refuse a `Repeat` rather than pass it here** — silently
    /// stepping over a loop would drop its body's writes, which is the one
    /// fail-open shape this composition could have had. A caller that *can*
    /// recurse does so through [`BufferRun`], which card
    /// `LOOPS-ON-COMPOSED-ROUTES` published for exactly that reason.
    #[must_use]
    pub fn apply(&mut self, index: usize, event: &BufferEvent) -> BufferStep {
        let mut step = BufferStep::default();
        match event {
            BufferEvent::Bind { binding, value } => {
                self.scalars.insert(binding.clone(), *value);
            }
            // --- Card `OWNED-FIELD-ROUTE-REACH`: the copy bind's value half. ---
            //
            // Three cases, and the split is the whole rule. A **scalar** and a
            // **record of scalars** are plain copyable values and are copied
            // here; that is constraint 15k's "a v1 record is a plain copyable
            // value" made checkable rather than asserted, and it is the only
            // thing this card admits that was refused before. Everything the
            // **ledger** holds — an acquired buffer, a moved-out binding, a
            // record with an owned field, a write loan — is refused by the
            // ownership IR at `Q054`, and this half deliberately says
            // *nothing* about any of them: the custody refusals of a copy
            // arrive from the IR exactly as a lend's do (constraint 15h), and
            // reporting the same fault twice would put a second copy rule
            // beside the one the card exists to reach.
            //
            // The order is total: names are unique (a duplicate `let` is
            // `Q012`, before this route), so at most one arm matches.
            BufferEvent::CopyBind { binding, source } => {
                if let Some(value) = self.scalars.get(source).copied() {
                    self.scalars.insert(binding.clone(), value);
                } else if let Some(value) = self.typed.get(source).copied() {
                    // Card `W48-QR-SCALAR-TYPES-R3`: a copy keeps its type.
                    self.typed.insert(binding.clone(), value);
                } else if let Some(value) = self.records.get(source).cloned() {
                    // Card `W48-QR-USABLE-RECORDS`: a record owns when an owned
                    // field lies anywhere below it.
                    if !self.owns(&value.type_name) {
                        self.records.insert(binding.clone(), value);
                    }
                } else if !self.extents.contains_key(source) && !self.loans.contains_key(source) {
                    // Not a scalar, not a record, not a buffer, not a loan:
                    // nothing this route bound. `Q013` refuses an unknown name
                    // before the route runs, so this arm is the fail-closed
                    // backstop rather than a reachable message.
                    step.errors.push(BufferError::NotAScalar {
                        event: index,
                        name: source.clone(),
                    });
                }
            }
            // Card `W48-QR-SCALAR-TYPES-R3`: the one explicit conversion. The
            // source is an `i32` literal or a name of any type; the result is
            // §G02t's `convert` — the same number at the target, or no value —
            // and since every operand here is a check-time constant, "no
            // value" is a refusal now, never a run-time event.
            BufferEvent::Convert {
                binding,
                operand,
                to,
            } => {
                let source = match operand {
                    BufferOperand::Literal(value) => Ok((ScalarType::I32, i128::from(*value))),
                    BufferOperand::Scalar(name) => self.scalar_value(index, name),
                };
                match source.and_then(|(from, value)| self.convert(index, from, *to, value)) {
                    Ok(value) => self.bind(binding, *to, value),
                    Err(error) => step.errors.push(error),
                }
            }
            // Card `W48-QR-SCALAR-TYPES-R5`: the value statement, decided by
            // the shared decision over this route's typed environment. Every
            // operand here is known, so the decision folds: its value is
            // bound, and a fault is a refusal now.
            BufferEvent::Compute { binding, operation } => match self.compute(index, operation) {
                Ok((ty, value)) => self.bind(binding, ty, value),
                Err(error) => step.errors.push(error),
            },
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
                } else if let Some(type_name) = self
                    .records
                    .get(name)
                    .map(|value| value.type_name.clone())
                    .filter(|type_name| self.owns(type_name))
                {
                    // Card `AGGREGATE-OWNED-FIELD`: `drop RECORD;` of a record
                    // with an owned field. The value half publishes which
                    // fields the drop-through releases — the live ones, in
                    // declaration order — and marks them released; the custody
                    // of the drop, the tolerated hole and the double drop are
                    // the IR's. A record of scalars falls through to the
                    // "is not a buffer" refusal it always had. Card
                    // `W48-QR-USABLE-RECORDS`: the owned **leaves**, at every
                    // depth, in the IR's own order.
                    let mut released = Vec::new();
                    for leaf in self.owned_leaves(&type_name) {
                        if let Some(owned) = self
                            .owned_fields
                            .get_mut(name)
                            .and_then(|fields| fields.get_mut(&leaf))
                            && owned.state == OwnedFieldState::Live
                        {
                            owned.state = OwnedFieldState::Released;
                            released.push(source_record::spell_path(name, &leaf));
                        }
                    }
                    step.field_releases = released;
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
                    || self.typed.contains_key(binding)
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
                    // Card `AGGREGATE-OWNED-FIELD`: a field declared `bytes`
                    // takes an acquired buffer's **name** and moves it in;
                    // every other field takes this route's operand exactly as
                    // before — so a buffer at an `i32` field still refuses at
                    // the crossing rule (`Q055`), unchanged, and only the
                    // declared owned field is the lift.
                    let value = match (declared.field(field), operand) {
                        (Some(FieldType::Bytes), FieldOperand::Name(name))
                            if self.extents.contains_key(name) =>
                        {
                            Ok(FieldValue::Bytes {
                                moved_from: name.clone(),
                            })
                        }
                        (Some(FieldType::Bytes), FieldOperand::Name(name))
                            if self.loans.contains_key(name) =>
                        {
                            // A loan cannot cross into a record: the existing
                            // lend crossing condition, reached at a field.
                            Err(BufferError::Lend {
                                event: index,
                                refusal: LendRefusal::CrossesBoundary { name: name.clone() },
                            })
                        }
                        (Some(FieldType::Bytes), _) => Err(BufferError::Record {
                            event: index,
                            refusal: RecordRefusal::FieldTakesABuffer {
                                type_name: type_name.clone(),
                                field: field.clone(),
                            },
                        }),
                        // Card `W48-QR-USABLE-RECORDS`: a record field takes a
                        // record this entry bound — the whole value, copied
                        // for a record of scalars and moved for one that owns
                        // (the move is the IR's `ComposeNested`). Its type is
                        // checked by the construction rule itself.
                        (Some(FieldType::Record(_)), FieldOperand::Name(name))
                            if self.records.contains_key(name) =>
                        {
                            Ok(FieldValue::Record(self.records[name].clone()))
                        }
                        (Some(FieldType::Record(declared)), _) => Err(BufferError::Record {
                            event: index,
                            refusal: RecordRefusal::FieldTakesARecord {
                                type_name: type_name.clone(),
                                field: field.clone(),
                                declared,
                                supplied: None,
                            },
                        }),
                        // Card `W48-QR-SCALAR-TYPES-R3`: a scalar field takes a
                        // value of exactly its declared type.
                        (Some(FieldType::Scalar(ty)), operand) => {
                            self.field_value(index, type_name, field, ty, operand)
                        }
                        // A field the type does not declare: the operand is
                        // read as this route's `i32` operand was before the
                        // card, so the construction's own refusal
                        // (`UndeclaredField`) is the one reported for it.
                        (None, FieldOperand::Name(name)) => self
                            .operand(index, &BufferOperand::Scalar(name.clone()))
                            .map(FieldValue::Scalar),
                        (
                            None,
                            FieldOperand::Literal(_)
                            | FieldOperand::Bool(_)
                            | FieldOperand::Float(_),
                        ) => Ok(FieldValue::Scalar(0)),
                    };
                    match value {
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
                                    .scalar(&field.name)
                                    .map(|value| (field.name.clone(), value))
                            })
                            .collect();
                        let typed = declared
                            .fields
                            .iter()
                            .filter_map(|field| {
                                value
                                    .typed(&field.name)
                                    .map(|(ty, value)| (field.name.clone(), ty, value))
                            })
                            .collect();
                        // The owned fields, in declared order, each taking the
                        // moved-in buffer's extent and bytes. The source
                        // binding is left as it was: every later use of it is
                        // refused by the IR as a use after move (`Q054`), the
                        // one refusal and the true one — the precedent is a
                        // use after `drop`, which the value half likewise
                        // leaves to the checker.
                        let mut owned_fields = BTreeMap::new();
                        let mut owned = Vec::new();
                        let mut records = Vec::new();
                        for field in &declared.fields {
                            match value.project(&field.name) {
                                Some(FieldValue::Bytes { moved_from }) => {
                                    let extent = self.extents.get(moved_from).copied().unwrap_or(0);
                                    let contents =
                                        self.contents.get(moved_from).cloned().unwrap_or_default();
                                    owned_fields.insert(
                                        vec![field.name.clone()],
                                        OwnedField {
                                            extent,
                                            contents,
                                            state: OwnedFieldState::Live,
                                        },
                                    );
                                    owned.push((field.name.clone(), moved_from.clone()));
                                }
                                // Card `W48-QR-USABLE-RECORDS`: a record moved
                                // in brings its owned leaves, in their states,
                                // under the field — the value half's mirror of
                                // the IR's `ComposePart::Record`.
                                Some(FieldValue::Record(_)) => {
                                    let from = fields
                                        .iter()
                                        .find_map(|(name, operand)| match operand {
                                            FieldOperand::Name(from) if *name == field.name => {
                                                Some(from.clone())
                                            }
                                            _ => None,
                                        })
                                        .unwrap_or_default();
                                    if let Some(inner) = self.owned_fields.get(&from) {
                                        for (leaf, held) in inner {
                                            let mut path = vec![field.name.clone()];
                                            path.extend(leaf.iter().cloned());
                                            owned_fields.insert(path, held.clone());
                                        }
                                    }
                                    records.push((field.name.clone(), from));
                                }
                                _ => {}
                            }
                        }
                        if !owned_fields.is_empty() {
                            self.owned_fields.insert(binding.clone(), owned_fields);
                        }
                        self.records.insert(binding.clone(), value);
                        step.construction = Some(RecordConstruction {
                            event: index,
                            binding: binding.clone(),
                            type_name: type_name.clone(),
                            fields: published,
                            typed,
                            owned,
                            records,
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
                path,
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
                let type_name = value.type_name.clone();
                // Card `W48-QR-USABLE-RECORDS`: the path is typed first, so a
                // component the type does not declare and a component reached
                // through a field that has no fields are refused by name.
                if let Err(refusal) = source_record::field_type_at(
                    &|name: &str| self.record_types.get(name),
                    record,
                    &type_name,
                    path,
                ) {
                    step.errors.push(BufferError::Record {
                        event: index,
                        refusal,
                    });
                    return step;
                }
                let spelling = path.join(".");
                match value.project_path(path) {
                    Some(FieldValue::Scalar(read)) => {
                        let read = *read;
                        step.projection = Some(RecordProjection {
                            event: index,
                            record: record.clone(),
                            type_name,
                            field: spelling,
                            value: read,
                        });
                        self.scalars.insert(binding.clone(), read);
                    }
                    // Card `W48-QR-SCALAR-TYPES-R3`: a field of another type
                    // binds a value of exactly that type.
                    Some(FieldValue::Typed { ty, value: read }) => {
                        let (ty, read) = (*ty, *read);
                        step.typed_projection = Some(TypedProjection {
                            event: index,
                            record: record.clone(),
                            type_name,
                            field: spelling,
                            ty,
                            value: read,
                        });
                        self.typed.insert(binding.clone(), (ty, read));
                    }
                    // Card `W48-QR-USABLE-RECORDS`: a sub-record of scalars is
                    // a plain value and is copied out; one with an owned field
                    // below it would be a partial move of a whole aggregate,
                    // which this revision does not spell — it is read field by
                    // field instead.
                    Some(FieldValue::Record(inner)) => {
                        if self.owns(&inner.type_name) {
                            step.errors.push(BufferError::Record {
                                event: index,
                                refusal: RecordRefusal::SubRecordOwns {
                                    path: source_record::spell_path(record, path),
                                    declared: inner.type_name.clone(),
                                },
                            });
                            return step;
                        }
                        let inner = inner.clone();
                        self.records.insert(binding.clone(), inner);
                    }
                    // Card `AGGREGATE-OWNED-FIELD`: the partial move. The
                    // binding owns the field's buffer from here on — its
                    // extent and bytes are the field's — and the field is a
                    // hole. A move out of a hole is the IR's use-after-move at
                    // `Q054`; the value half still binds the name so that the
                    // one refusal is the custody one.
                    Some(FieldValue::Bytes { .. }) => {
                        let Some(owned) = self
                            .owned_fields
                            .get_mut(record)
                            .and_then(|fields| fields.get_mut(path))
                        else {
                            step.errors.push(BufferError::Record {
                                event: index,
                                refusal: RecordRefusal::NoSuchField {
                                    name: record.clone(),
                                    type_name,
                                    field: spelling,
                                },
                            });
                            return step;
                        };
                        if owned.state == OwnedFieldState::Live {
                            owned.state = OwnedFieldState::Moved;
                        }
                        let extent = owned.extent;
                        let contents = owned.contents.clone();
                        self.extents.insert(binding.clone(), extent);
                        self.contents.insert(binding.clone(), contents);
                        step.field_move = Some(FieldMove {
                            event: index,
                            record: record.clone(),
                            field: spelling,
                            binding: binding.clone(),
                            extent,
                        });
                    }
                    None => step.errors.push(BufferError::Record {
                        event: index,
                        refusal: RecordRefusal::NoSuchField {
                            name: record.clone(),
                            type_name,
                            field: spelling,
                        },
                    }),
                }
            }
            // --- The field write (card `W48-QR-USABLE-RECORDS`). ---
            //
            // A scalar field takes a value of exactly its declared type, by the
            // construction's own typing, and the write replaces that field and
            // nothing else. An owned field takes a buffer this entry owns —
            // the refill — whose custody (into a hole only) is the IR's. A
            // record field is refused by name. The record must be one this
            // entry bound; whether a record with an owned field is still live
            // is the IR's question too (`AccessRecord`), at `Q054`.
            BufferEvent::FieldWrite {
                record,
                path,
                value: operand,
            } => {
                let Some(type_name) = self.records.get(record).map(|v| v.type_name.clone()) else {
                    step.errors.push(BufferError::Record {
                        event: index,
                        refusal: RecordRefusal::NotARecord {
                            name: record.clone(),
                        },
                    });
                    return step;
                };
                let declared = match source_record::field_type_at(
                    &|name: &str| self.record_types.get(name),
                    record,
                    &type_name,
                    path,
                ) {
                    Ok(declared) => declared,
                    Err(refusal) => {
                        step.errors.push(BufferError::Record {
                            event: index,
                            refusal,
                        });
                        return step;
                    }
                };
                let spelling = path.join(".");
                let holder =
                    source_record::spell_path(&type_name, &path[..path.len().saturating_sub(1)]);
                let last = path.last().cloned().unwrap_or_default();
                match declared {
                    FieldType::Scalar(ty) => {
                        match self.field_value(index, &holder, &last, ty, operand) {
                            Ok(written) => {
                                let number = match &written {
                                    FieldValue::Scalar(number) => i128::from(*number),
                                    FieldValue::Typed { value, .. } => *value,
                                    FieldValue::Bytes { .. } | FieldValue::Record(_) => 0,
                                };
                                if let Some(value) = self.records.get_mut(record) {
                                    value.write_path(path, written);
                                }
                                step.write = Some(RecordWrite {
                                    event: index,
                                    record: record.clone(),
                                    field: spelling,
                                    value: WrittenValue::Scalar { ty, value: number },
                                });
                            }
                            Err(error) => step.errors.push(error),
                        }
                    }
                    FieldType::Bytes => match operand {
                        FieldOperand::Name(name) if self.extents.contains_key(name) => {
                            let extent = self.extents[name];
                            let contents = self.contents.get(name).cloned().unwrap_or_default();
                            if let Some(value) = self.records.get_mut(record) {
                                value.write_path(
                                    path,
                                    FieldValue::Bytes {
                                        moved_from: name.clone(),
                                    },
                                );
                            }
                            self.owned_fields.entry(record.clone()).or_default().insert(
                                path.clone(),
                                OwnedField {
                                    extent,
                                    contents,
                                    state: OwnedFieldState::Live,
                                },
                            );
                            step.write = Some(RecordWrite {
                                event: index,
                                record: record.clone(),
                                field: spelling,
                                value: WrittenValue::Refill {
                                    buffer: name.clone(),
                                    extent,
                                },
                            });
                        }
                        FieldOperand::Name(name) if self.loans.contains_key(name) => {
                            step.errors.push(BufferError::Lend {
                                event: index,
                                refusal: LendRefusal::CrossesBoundary { name: name.clone() },
                            });
                        }
                        _ => step.errors.push(BufferError::Record {
                            event: index,
                            refusal: RecordRefusal::FieldTakesABuffer {
                                type_name: holder,
                                field: last,
                            },
                        }),
                    },
                    FieldType::Record(inner) => step.errors.push(BufferError::Record {
                        event: index,
                        refusal: RecordRefusal::WriteOfRecordField {
                            path: source_record::spell_path(record, path),
                            declared: inner,
                        },
                    }),
                }
            }
            // --- The field-path custody positions (card `AGGREGATE-OWNED-FIELD`). ---
            //
            // Each is the plain-name arm above at `RECORD.FIELD`: the same
            // bounds rule, the same byte-range rule, the same published access
            // — with the name published as the path. Which fields are owned,
            // and whether the field still holds its buffer, is asked of the
            // declared type and of the IR respectively; this half asks only
            // "which bytes".
            BufferEvent::FieldStore {
                record,
                path,
                index: at,
                value,
            } => {
                let spelling = source_record::spell_path(record, path);
                let place = self.owned_field(index, record, path).and_then(|owned| {
                    field_place(index, &spelling, owned.extent, self.operand(index, at))
                });
                let value = self.operand(index, value);
                match (place, value) {
                    (Ok(at), Ok(value)) => {
                        let Ok(byte) = u8::try_from(value) else {
                            step.errors.push(BufferError::ByteOutOfRange {
                                event: index,
                                value,
                            });
                            return step;
                        };
                        if let Some(owned) = self
                            .owned_fields
                            .get_mut(record)
                            .and_then(|fields| fields.get_mut(path))
                        {
                            owned.contents[at] = byte;
                        }
                        step.access = Some(BufferAccess {
                            event: index,
                            name: spelling,
                            index: at as u32,
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
            BufferEvent::FieldLoad {
                binding,
                record,
                path,
                index: at,
            } => {
                let spelling = source_record::spell_path(record, path);
                match self.owned_field(index, record, path).and_then(|owned| {
                    field_place(index, &spelling, owned.extent, self.operand(index, at))
                }) {
                    Ok(at) => {
                        let byte = self
                            .owned_fields
                            .get(record)
                            .and_then(|fields| fields.get(path))
                            .and_then(|owned| owned.contents.get(at).copied())
                            .unwrap_or(0);
                        self.scalars.insert(binding.clone(), i32::from(byte));
                        step.access = Some(BufferAccess {
                            event: index,
                            name: spelling,
                            index: at as u32,
                            byte,
                            write: false,
                        });
                    }
                    Err(error) => step.errors.push(error),
                }
            }
            BufferEvent::FieldExtent {
                binding,
                record,
                path,
            } => match self.owned_field(index, record, path) {
                Ok(owned) => {
                    self.scalars.insert(binding.clone(), owned.extent as i32);
                }
                Err(error) => step.errors.push(error),
            },
            BufferEvent::FieldRelease { record, path } => {
                match self.owned_field(index, record, path) {
                    Ok(_) => {
                        // Published only while live: a release of a hole is
                        // the IR's double drop or use after move, the one
                        // refusal, at `Q054`.
                        if let Some(owned) = self
                            .owned_fields
                            .get_mut(record)
                            .and_then(|fields| fields.get_mut(path))
                            && owned.state == OwnedFieldState::Live
                        {
                            owned.state = OwnedFieldState::Released;
                            step.field_releases = vec![source_record::spell_path(record, path)];
                        }
                    }
                    Err(error) => step.errors.push(error),
                }
            }
            BufferEvent::Line(_) | BufferEvent::Return | BufferEvent::Repeat { .. } => {}
        }
        step
    }

    /// Bind `binding` to `value` at `ty`, in the environment's part for that
    /// type (card `W48-QR-SCALAR-TYPES-R3`).
    fn bind(&mut self, binding: &str, ty: ScalarType, value: i128) {
        if ty == ScalarType::I32 {
            let value = i32::try_from(value).expect("a checked i32 value fits i32");
            self.scalars.insert(binding.to_owned(), value);
        } else {
            self.typed.insert(binding.to_owned(), (ty, value));
        }
    }

    /// One value statement's result, decided now by the shared decision (card
    /// `W48-QR-SCALAR-TYPES-R5`): its type and its number, or the refusal.
    pub fn compute(
        &mut self,
        event: usize,
        operation: &ValueOperation,
    ) -> Result<(ScalarType, i128), BufferError> {
        use crate::scalar_values::{Decider, Operand, Operation};
        let operand = |operand: &ValueOperand| -> Result<Operand, BufferError> {
            Ok(match operand {
                ValueOperand::Name(name) => {
                    let (ty, value) = self.scalar_value(event, name)?;
                    Operand::Typed {
                        ty,
                        known: Some(value),
                    }
                }
                ValueOperand::Integer(value) => Operand::Integer(*value),
                ValueOperand::Bool(value) => Operand::Bool(*value),
                ValueOperand::Float(literal) => Operand::Float(*literal),
            })
        };
        let shared = match operation {
            ValueOperation::Binary(op, a, b) => Operation::Binary(*op, operand(a)?, operand(b)?),
            ValueOperation::Compare(cmp, a, b) => {
                Operation::Compare(*cmp, operand(a)?, operand(b)?)
            }
            ValueOperation::Negate(a) => Operation::Negate(operand(a)?),
            ValueOperation::Convert(a, to) => Operation::Convert(operand(a)?, *to),
        };
        let refused = |refusal| BufferError::Typed {
            event,
            refusal: TypedRefusal::Value(refusal),
        };
        // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: a float comparison needs its
        // effect in the entry's row, never inferred.
        let mut decider = Decider {
            permitted: self.float_effects,
            ..Decider::default()
        };
        let decision = decider
            .decide(shared, false, self.target.as_ref())
            .map_err(refused)?;
        for effect in st::FloatEffect::ALL {
            if decider.used.contains(effect) {
                self.float_comparisons.insert(effect);
            }
        }
        // Every operand is known, so the decision folded; a statement it left
        // unfolded would be a value this route cannot hold, refused rather
        // than bound.
        let value = decision.known.ok_or_else(|| {
            refused(crate::scalar_values::ValueRefusal::NeedsPermission {
                ty: decision.operand_ty,
            })
        })?;
        Ok((decision.ty, value))
    }

    /// `value as to`, from `from`, decided now (card `W48-QR-SCALAR-TYPES-R3`):
    /// §G02t's one conversion, refused when the target width is unrecorded,
    /// when either side is `bool`, and when the number does not fit.
    fn convert(
        &self,
        event: usize,
        from: ScalarType,
        to: ScalarType,
        value: i128,
    ) -> Result<i128, BufferError> {
        for ty in [from, to] {
            if ty.is_target_width() && self.target.is_none() {
                return Err(BufferError::Typed {
                    event,
                    refusal: TypedRefusal::NoRecordedTarget { ty },
                });
            }
        }
        // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: a float converts to and from
        // every integer and the other float (§G02t's rules: rounding to a
        // float, truncation to an integer); `bool` still converts to nothing.
        if from == ScalarType::Bool || to == ScalarType::Bool {
            return Err(BufferError::Typed {
                event,
                refusal: TypedRefusal::NoConversion { from, to },
            });
        }
        st::convert(from, to, value, self.target.as_ref()).ok_or(BufferError::Typed {
            event,
            refusal: TypedRefusal::ConversionOutOfRange { from, to, value },
        })
    }

    /// The value a construction supplies at a field declared the scalar type
    /// `ty` (card `W48-QR-SCALAR-TYPES-R3`): a literal that is a value of `ty`,
    /// `true`/`false` at a `bool` field, or a name of exactly type `ty`.
    fn field_value(
        &self,
        event: usize,
        type_name: &str,
        field: &str,
        ty: ScalarType,
        operand: &FieldOperand,
    ) -> Result<FieldValue, BufferError> {
        let at = |value: i128| {
            if ty == ScalarType::I32 {
                FieldValue::Scalar(i32::try_from(value).expect("a checked i32 value fits i32"))
            } else {
                FieldValue::Typed { ty, value }
            }
        };
        let out_of_range = |literal: String| BufferError::Record {
            event,
            refusal: RecordRefusal::FieldLiteralOutOfRange {
                type_name: type_name.to_owned(),
                field: field.to_owned(),
                declared: ty,
                literal,
            },
        };
        if ty.is_target_width() && self.target.is_none() {
            return Err(BufferError::Record {
                event,
                refusal: RecordRefusal::FieldNeedsRecordedTarget {
                    type_name: type_name.to_owned(),
                    field: field.to_owned(),
                    declared: ty,
                },
            });
        }
        let kind = |float: bool| BufferError::Typed {
            event,
            refusal: TypedRefusal::Value(crate::scalar_values::ValueRefusal::LiteralKind {
                ty,
                float,
            }),
        };
        match operand {
            // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: an integer literal is
            // never a float, and a float literal never an integer.
            FieldOperand::Literal(_) if ty.is_float() => Err(kind(false)),
            FieldOperand::Float(_) if !ty.is_float() => Err(kind(true)),
            FieldOperand::Float(literal) => {
                literal.at(ty).map(at).map_err(|error| BufferError::Typed {
                    event,
                    refusal: TypedRefusal::Value(
                        crate::scalar_values::ValueRefusal::FloatLiteral { ty, error },
                    ),
                })
            }
            FieldOperand::Literal(value) => {
                if ty.is_integer() && ty.contains(*value, self.target.as_ref()) {
                    Ok(at(*value))
                } else {
                    Err(out_of_range(value.to_string()))
                }
            }
            FieldOperand::Bool(value) => {
                if ty == ScalarType::Bool {
                    Ok(at(i128::from(*value)))
                } else {
                    Err(out_of_range(value.to_string()))
                }
            }
            FieldOperand::Name(name) => {
                let (found, value) = self.scalar_value(event, name)?;
                if found == ty {
                    Ok(at(value))
                } else {
                    Err(BufferError::Record {
                        event,
                        refusal: RecordRefusal::FieldTakesAType {
                            type_name: type_name.to_owned(),
                            field: field.to_owned(),
                            declared: ty,
                            supplied: found,
                        },
                    })
                }
            }
        }
    }

    /// The type and number a scalar name denotes (card
    /// `W48-QR-SCALAR-TYPES-R3`): the environment's typing function. A loan, a
    /// buffer and a record are refused here by name, in the order and with the
    /// refusals [`Self::operand`] has always given them.
    pub fn scalar_value(
        &self,
        event: usize,
        name: &str,
    ) -> Result<(ScalarType, i128), BufferError> {
        if self.loans.contains_key(name) {
            // A lend at a scalar position (card
            // `SLICE-PERMISSIONS-WRITE-LEND`), refused for the reason
            // a buffer is and at the same code: neither is a number,
            // and the reader needs to be told which they wrote.
            return Err(BufferError::Lend {
                event,
                refusal: LendRefusal::CrossesBoundary {
                    name: name.to_owned(),
                },
            });
        }
        if self.extents.contains_key(name) {
            // A buffer at a scalar position. Reported as the boundary
            // refusal rather than the generic one, because the reader
            // needs to know *why* a name they bound is not usable here.
            return Err(BufferError::CrossesBoundary {
                event,
                name: name.to_owned(),
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
                refusal: RecordRefusal::RecordAtScalarPosition {
                    name: name.to_owned(),
                },
            });
        }
        if let Some(value) = self.scalars.get(name) {
            return Ok((ScalarType::I32, i128::from(*value)));
        }
        self.typed
            .get(name)
            .copied()
            .ok_or_else(|| BufferError::NotAScalar {
                event,
                name: name.to_owned(),
            })
    }

    /// Resolve `RECORD.PATH` to the owned field it names, or refuse by name:
    /// not a record this entry constructed, a path the type does not declare,
    /// or a field that is a scalar or a record rather than `bytes` (cards
    /// `AGGREGATE-OWNED-FIELD`, `W48-QR-USABLE-RECORDS`). Whether the field
    /// still holds its buffer is not asked here — that is the IR's question,
    /// and its answer arrives at `Q054`.
    fn owned_field(
        &self,
        event: usize,
        record: &str,
        path: &[String],
    ) -> Result<&OwnedField, BufferError> {
        let Some(value) = self.records.get(record) else {
            return Err(BufferError::Record {
                event,
                refusal: RecordRefusal::NotARecord {
                    name: record.to_owned(),
                },
            });
        };
        let refused = |refusal| BufferError::Record { event, refusal };
        match source_record::field_type_at(
            &|name: &str| self.record_types.get(name),
            record,
            &value.type_name,
            path,
        )
        .map_err(refused)?
        {
            FieldType::Bytes => {}
            declared @ (FieldType::Scalar(_) | FieldType::Record(_)) => {
                return Err(refused(RecordRefusal::FieldIsNotABuffer {
                    name: record.to_owned(),
                    type_name: value.type_name.clone(),
                    field: path.join("."),
                    declared: declared.spelling().to_owned(),
                }));
            }
        }
        self.owned_fields
            .get(record)
            .and_then(|fields| fields.get(path))
            .ok_or_else(|| {
                refused(RecordRefusal::NoSuchField {
                    name: record.to_owned(),
                    type_name: value.type_name.clone(),
                    field: path.join("."),
                })
            })
    }

    /// Whether a value of `type_name` owns anything at any depth (card
    /// `W48-QR-USABLE-RECORDS`).
    #[must_use]
    pub fn owns(&self, type_name: &str) -> bool {
        source_record::owns(&|name: &str| self.record_types.get(name), type_name)
    }

    /// The owned leaves of `type_name`, in the IR's order (card
    /// `W48-QR-USABLE-RECORDS`).
    #[must_use]
    pub fn owned_leaves(&self, type_name: &str) -> Vec<Vec<String>> {
        source_record::owned_leaves(&|name: &str| self.record_types.get(name), type_name)
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

    /// An operand at a position that takes an **`i32`** — an index, a stored
    /// byte, a lend bound. Card `W48-QR-SCALAR-TYPES-R3`: a name of another
    /// type is refused by name (`NotAnI32`), never converted.
    pub fn operand(&self, event: usize, operand: &BufferOperand) -> Result<i32, BufferError> {
        match operand {
            BufferOperand::Literal(value) => Ok(*value),
            BufferOperand::Scalar(name) => match self.scalar_value(event, name)? {
                (ScalarType::I32, value) => {
                    Ok(i32::try_from(value).expect("an i32 binding holds an i32"))
                }
                (found, _) => Err(BufferError::Typed {
                    event,
                    refusal: TypedRefusal::NotAnI32 {
                        name: name.clone(),
                        found,
                    },
                }),
            },
        }
    }

    /// Resolve `NAME[INDEX]` to a validated `(extent, index)` pair. The bounds
    /// rule lives here and nowhere else, so a read and a write cannot disagree
    /// about it — [`field_place`] below is the same rule at a field path,
    /// stated over the extent the field holds.
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

/// The bounds rule at a field path (card `AGGREGATE-OWNED-FIELD`): the index
/// is inside `[0, extent)` of the buffer the field holds, or the statement is
/// refused at `Q053` naming the path. Written beside [`BufferEvaluation::place`]
/// with the same comparison so the two cannot drift.
fn field_place(
    event: usize,
    spelling: &str,
    extent: u32,
    index: Result<i32, BufferError>,
) -> Result<usize, BufferError> {
    let index = index?;
    if index < 0 || u32::try_from(index).is_ok_and(|index| index >= extent) {
        return Err(BufferError::OutOfBounds {
            event,
            name: spelling.to_owned(),
            index,
            extent,
        });
    }
    Ok(index as usize)
}

/// Whether this event is one of card `AGGREGATE-OWNED-FIELD`'s — a
/// construction of a type declaring an owned field, a field-path custody
/// statement, or a whole-record release of a composed record — at any depth.
/// The two window gates in [`check_buffer_program`] ask it.
fn owned_field_event(event: &BufferEvent, types: &BTreeMap<String, RecordType>) -> bool {
    match event {
        // Card `W48-QR-USABLE-RECORDS`: a type owns when an owned field lies
        // anywhere below it, a record field's included.
        BufferEvent::Construct { type_name, .. } => {
            source_record::owns(&|name: &str| types.get(name), type_name)
        }
        BufferEvent::FieldStore { .. }
        | BufferEvent::FieldLoad { .. }
        | BufferEvent::FieldExtent { .. }
        | BufferEvent::FieldRelease { .. } => true,
        _ => false,
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
    check_buffer_program_for_target(program, None)
}

/// Whether this program names `isize` or `usize` anywhere — a field type or a
/// conversion target (card `W48-QR-SCALAR-TYPES-R3`). Exactly such a program's
/// result depends on the recorded target.
#[must_use]
pub fn names_target_width(program: &BufferProgram) -> bool {
    program.records.iter().any(|record| {
        record.fields.iter().any(|field| {
            field
                .field_type
                .scalar()
                .is_some_and(ScalarType::is_target_width)
        })
    }) || any_event(&program.events, |event| match event {
        BufferEvent::Convert { to, .. } => to.is_target_width(),
        // Card `W48-QR-SCALAR-TYPES-R5`: a loop whose index is `isize` or
        // `usize` depends on the target too.
        BufferEvent::Repeat { index, .. } => index.ty().is_target_width(),
        _ => false,
    })
}

/// Whether this program holds an `f32` or `f64` value anywhere (card
/// `W48-QR-FLOAT-BUFFER-PAYLOADS`): a record field declared with a float type,
/// a conversion to one, or a value statement with a float literal or
/// converting to a float. Every float value of the route originates at one of
/// those, so exactly such a program's result carries the IEEE premise
/// (`scalar_floats::ASSUMPTION`) and its C the float environment.
#[must_use]
pub fn uses_floats(program: &BufferProgram) -> bool {
    program.records.iter().any(|record| {
        record
            .fields
            .iter()
            .any(|field| field.field_type.scalar().is_some_and(ScalarType::is_float))
    }) || any_event(&program.events, |event| match event {
        BufferEvent::Convert { to, .. } => to.is_float(),
        BufferEvent::Compute { operation, .. } => {
            matches!(operation, ValueOperation::Convert(_, to) if to.is_float())
                || operation
                    .operands()
                    .iter()
                    .any(|operand| matches!(operand, ValueOperand::Float(_)))
        }
        _ => false,
    })
}

/// [`check_buffer_program`] for the builder's recorded target (card
/// `W48-QR-SCALAR-TYPES-R3`), which gives `isize`/`usize` their width. A
/// program naming neither is checked exactly as without one and carries no
/// target assumption; a program naming either with no target is refused.
pub fn check_buffer_program_for_target(
    program: &BufferProgram,
    target: Option<&RecordedTarget>,
) -> Result<BufferCertificate, Vec<BufferError>> {
    let mut errors = Vec::new();
    let target_assumption = if names_target_width(program) {
        target.cloned()
    } else {
        None
    };

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
    // Card `W48-QR-SCALAR-TYPES-R3`: a field declared `isize` or `usize` is
    // a declaration this module can state only for a recorded target. Refused
    // at the declaration, whether or not any value of it is constructed.
    if target_assumption.is_none() {
        for record in &program.records {
            for field in &record.fields {
                if let Some(declared) = field.field_type.scalar()
                    && declared.is_target_width()
                {
                    return Err(vec![BufferError::Record {
                        event: program.events.len(),
                        refusal: RecordRefusal::FieldNeedsRecordedTarget {
                            type_name: record.name.clone(),
                            field: field.name.clone(),
                            declared,
                        },
                    }]);
                }
            }
        }
    }

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

    // --- Half zero-and-a-quarter: the owned field's two window premises
    // (card `AGGREGATE-OWNED-FIELD`), each refused by name. ---
    //
    // A record with an owned field beside a **lend**: the lend projection's
    // borrow nodes are built by a segment walk that addresses owners by
    // plain name, and giving it field paths would put a second custody
    // projection beside the first. Refused rather than approximated, the
    // `SLICE-PERMISSIONS-LOOP-WINDOW` shape (`OWNED-FIELD-LEND-WINDOW`).
    let composes_owned_field = any_event(&program.events, |event| {
        owned_field_event(event, &record_types)
    });
    if composes_owned_field
        && let Some(position) = program
            .events
            .iter()
            .position(|event| matches!(event, BufferEvent::Lend { .. }))
    {
        return Err(vec![BufferError::Shape {
            event: position,
            reason: "a lend is not a statement an entry that composes a record with an owned \
                     field models; the lend's custody projection addresses owners by plain \
                     name, and a projection that also addressed fields would be a second one \
                     beside it (`OWNED-FIELD-LEND-WINDOW`)"
                .to_owned(),
        }]);
    }
    // An owned-field statement inside a **loop body**: the unrolled custody
    // walk gives each iteration's acquisitions their own scope, and a composed
    // record — whose fields are addressed by path — would need the same
    // scoping for its aggregate and every field. Refused by name rather than
    // left to surface as a rebound name in the second iteration
    // (`OWNED-FIELD-LOOP-BODY`). A field read of an **outer** composed record
    // inside a body is refused for the same reason: the move would recur.
    {
        let composed_outside: BTreeSet<&str> = program
            .events
            .iter()
            .filter_map(|event| match event {
                BufferEvent::Construct {
                    binding, type_name, ..
                } if source_record::owns(&|name: &str| record_types.get(name), type_name) => {
                    Some(binding.as_str())
                }
                _ => None,
            })
            .collect();
        for (position, event) in program.events.iter().enumerate() {
            let BufferEvent::Repeat { body, .. } = event else {
                continue;
            };
            let offending = any_event(body, |inner| {
                owned_field_event(inner, &record_types)
                    || matches!(inner, BufferEvent::Field { record, .. }
                        | BufferEvent::Release { name: record }
                        // Card `W48-QR-USABLE-RECORDS`: a write of an outer
                        // composed record in a body — a refill would recur,
                        // and a scalar write keeps the reads' company.
                        | BufferEvent::FieldWrite { record, .. }
                        if composed_outside.contains(record.as_str()))
            });
            if offending {
                return Err(vec![BufferError::Shape {
                    event: position,
                    reason: "a record with an owned field is not a statement a `for` body \
                             models; the unrolled custody walk scopes each iteration's \
                             acquisitions by name, and a composed record's fields are addressed \
                             by path, so the aggregate and its fields would each need that \
                             scoping (`OWNED-FIELD-LOOP-BODY`)"
                        .to_owned(),
                }]);
            }
        }
    }

    // --- Half zero-and-a-half: the lend's window premises (card
    // `SLICE-PERMISSIONS-WRITE-LEND`), decided before the custody projection
    // is consulted because the projection's borrow nodes rest on them. ---
    let mut lend_window_by_event: BTreeMap<usize, usize> = BTreeMap::new();
    if any_event(&program.events, |event| {
        matches!(event, BufferEvent::Lend { .. })
    }) {
        // Card `LOOPS-ON-COMPOSED-ROUTES`: a `for` in an entry that lends is
        // admitted, with the lend's window computed **over iterations**
        // (`last_uses`) rather than refused for being an index interval —
        // the wall `SLICE-PERMISSIONS-LOOP-WINDOW` recorded, lifted where
        // `QCore.LoopWindows` covers it. Two body shapes stay outside the
        // theorem and are refused by name, **before** the window function
        // runs, so the refusals below cannot be reordered behind it:
        //
        // * a lend inside a body is a **per-iteration loan** — a loan that
        //   opens and closes inside one iteration, whose window is not an
        //   interval of top-level indices at all (`LOOP-PER-ITERATION-LOAN`);
        // * an acquisition or a release inside a body would put a custody
        //   step in a loop of an entry whose projection opens borrow nodes
        //   over top-level windows, where neither the per-iteration identity
        //   gate nor the one-pass peak walker sees it
        //   (`LOOP-WINDOW-BODY-CUSTODY`).
        for (position, event) in program.events.iter().enumerate() {
            let BufferEvent::Repeat { .. } = event else {
                continue;
            };
            if event.any(&|inner| matches!(inner, BufferEvent::Lend { .. })) {
                return Err(vec![BufferError::Lend {
                    event: position,
                    refusal: LendRefusal::Shape(
                        "this `for` body opens a lend, which is a per-iteration loan: its window \
                         opens and closes inside one iteration and is not an interval of \
                         statement indices, so `source.buffer.v1` refuses it by name rather \
                         than approximating it; a loan a loop body uses is opened before the \
                         loop (`LOOP-PER-ITERATION-LOAN`)"
                            .to_owned(),
                    ),
                }]);
            }
            if event.any(&|inner| {
                matches!(
                    inner,
                    BufferEvent::Acquire { .. } | BufferEvent::Release { .. }
                )
            }) {
                return Err(vec![BufferError::Lend {
                    event: position,
                    refusal: LendRefusal::Shape(
                        "this `for` body acquires or releases a buffer; in an entry that lends, \
                         a loop body neither acquires nor releases, because the lend-structured \
                         custody projection opens its borrow nodes over top-level windows and a \
                         custody step inside an iteration would be seen by neither the \
                         per-iteration identity gate nor the one-pass peak walker \
                         (`LOOP-WINDOW-BODY-CUSTODY`)"
                            .to_owned(),
                    ),
                }]);
            }
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
        // (`WRITE-LEND-NESTED-ACQUIRE`). Scanned **at any depth** since card
        // `LOOPS-ON-COMPOSED-ROUTES`: a window may now contain a `for`, and
        // an acquisition inside its body is inside the window too — already
        // refused by name above, and kept here so the scan is total on its
        // own terms rather than by the order the refusals happen to run in.
        for window in &windows {
            for (index, event) in program.events.iter().enumerate() {
                if index > window.event
                    && index <= window.through
                    && event.any(&|inner| matches!(inner, BufferEvent::Acquire { .. }))
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
    let mut run = BufferRun::new(BufferEvaluation {
        record_types,
        target: target_assumption.clone(),
        float_effects: program.float_effects,
        ..BufferEvaluation::new()
    });
    run.lend_windows = lend_window_by_event;
    run.walk(&program.events, 0, true);
    let BufferRun {
        evaluation,
        acquisitions,
        releases,
        moves,
        lends,
        accesses,
        constructions,
        projections,
        typed_projections,
        writes,
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
            moves,
            lends,
            peak_live_bytes,
            accesses,
            constructions,
            projections,
            typed_projections,
            writes,
            scalars: evaluation.scalars,
            typed_scalars: evaluation.typed,
            target_assumption,
            lines,
            flushes,
            renderings,
            float_comparisons: evaluation.float_comparisons,
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
///
/// **Published since card `LOOPS-ON-COMPOSED-ROUTES`**, for the one reason
/// [`BufferEvaluation::apply`]'s header states: a caller that cannot recurse
/// must refuse a `Repeat` rather than step over it, and the composed
/// string/buffer route now admits one. It runs the body through **this**
/// runner — [`BufferRun::repeat`], which owns the index binding, the
/// body-local scoping and the shadowing refusal — rather than through a
/// second iteration rule written beside it, so a loop body's events on that
/// route are decided by the one implementation a loop body's events on this
/// route are.
pub struct BufferRun {
    pub evaluation: BufferEvaluation,
    pub acquisitions: Vec<(String, u32)>,
    pub releases: Vec<String>,
    /// Every field move this run published (card `AGGREGATE-OWNED-FIELD`).
    pub moves: Vec<FieldMove>,
    /// Each lend's window end by its event index, precomputed by
    /// [`lend_windows`] — `through` is a whole-program fact the stepper cannot
    /// know (card `SLICE-PERMISSIONS-WRITE-LEND`).
    pub lend_windows: BTreeMap<usize, usize>,
    pub lends: Vec<LendRecord>,
    pub accesses: Vec<BufferAccess>,
    pub constructions: Vec<RecordConstruction>,
    pub projections: Vec<RecordProjection>,
    /// Card `W48-QR-SCALAR-TYPES-R3`: reads of fields of other types.
    pub typed_projections: Vec<TypedProjection>,
    /// Card `W48-QR-USABLE-RECORDS`: every field write.
    pub writes: Vec<RecordWrite>,
    pub lines: Vec<ObservedLine>,
    pub flushes: usize,
    pub renderings: BTreeSet<Rendering>,
    pub returned: bool,
    pub errors: Vec<BufferError>,
}

impl BufferRun {
    /// A run over `evaluation` that has published nothing yet.
    #[must_use]
    pub fn new(evaluation: BufferEvaluation) -> Self {
        Self {
            evaluation,
            acquisitions: Vec::new(),
            releases: Vec::new(),
            moves: Vec::new(),
            lend_windows: BTreeMap::new(),
            lends: Vec::new(),
            accesses: Vec::new(),
            constructions: Vec::new(),
            projections: Vec::new(),
            typed_projections: Vec::new(),
            writes: Vec::new(),
            lines: Vec::new(),
            flushes: 0,
            renderings: BTreeSet::new(),
            returned: false,
            errors: Vec::new(),
        }
    }

    /// Run `events` in order.
    ///
    /// `attributed` is the index of the **top-level** event every refusal and
    /// every published access of this walk is attributed to, and `top_level`
    /// says whether this call is the entry's own sequence. A refusal inside a
    /// loop body is therefore reported at the `for` statement: the body has no
    /// top-level index of its own, the statement is what the reader has to
    /// change, and `LOOP-BODY-ATTRIBUTION` records the finer span as a
    /// follow-on rather than guessing one.
    pub fn walk(&mut self, events: &[BufferEvent], attributed: usize, top_level: bool) {
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
            index: loop_index,
            body,
        } = event
        {
            self.repeat_typed(variable, *count, loop_index, body, index);
            return;
        }
        let Self {
            evaluation,
            acquisitions,
            releases,
            moves,
            lend_windows,
            lends,
            accesses,
            constructions,
            projections,
            typed_projections,
            writes,
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
            | BufferEvent::Field { .. }
            // Card `AGGREGATE-OWNED-FIELD`: the four field-path positions are
            // value-bearing exactly as their plain-name twins are, and reach
            // the same stepper.
            | BufferEvent::FieldStore { .. }
            | BufferEvent::FieldLoad { .. }
            | BufferEvent::FieldExtent { .. }
            | BufferEvent::FieldRelease { .. }
            // Card `W48-QR-USABLE-RECORDS`: a field write is value-bearing and
            // reaches the same stepper.
            | BufferEvent::FieldWrite { .. }
            // Card `OWNED-FIELD-ROUTE-REACH`: the copy bind is value-bearing —
            // it binds a scalar or a record value — and reaches the same
            // stepper, so a copy inside a `for` body is decided by the one
            // implementation a top-level one is.
            | BufferEvent::CopyBind { .. }
            // Card `W48-QR-SCALAR-TYPES-R3`: a conversion binds a value and
            // reaches the same stepper.
            | BufferEvent::Convert { .. }
            // Card `W48-QR-SCALAR-TYPES-R5`: so does a value statement.
            | BufferEvent::Compute { .. } => {
                let step = evaluation.apply(index, event);
                errors.extend(step.errors);
                acquisitions.extend(step.acquisition);
                releases.extend(step.release);
                releases.extend(step.field_releases);
                moves.extend(step.field_move);
                accesses.extend(step.access);
                constructions.extend(step.construction);
                projections.extend(step.projection);
                typed_projections.extend(step.typed_projection);
                writes.extend(step.write);
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
                    // Card `W48-QR-SCALAR-TYPES-R3`: the operand is rendered
                    // at its own type. An `i32` takes the path every program
                    // before the card took, byte for byte; every other type
                    // is §G02t's `std.fmt` rendering, framed by the one line
                    // framing every route shares.
                    match evaluation.typed_line_operand(index, operand) {
                        Ok((ScalarType::I32, value)) => lines.push(
                            rendered_line(
                                line,
                                i32::try_from(value).expect("an i32 binding holds an i32"),
                            )
                            .expect("a print and a log each produce exactly one line"),
                        ),
                        Ok((ty, value)) => {
                            let level = match line {
                                StdIoEvent::Log { level, .. } => Some(*level),
                                _ => None,
                            };
                            match st::render(
                                *rendering == Rendering::Hex,
                                ty,
                                value,
                                evaluation.target.as_ref(),
                            ) {
                                Some(body) => lines.push(rendered_text_line(level, &body)),
                                None => errors.push(BufferError::Typed {
                                    event: index,
                                    refusal: TypedRefusal::NoHexRendering {
                                        name: match operand {
                                            StdIoOperand::Scalar(name) => name.clone(),
                                            StdIoOperand::Literal(value) => value.to_string(),
                                            StdIoOperand::Borrowed(name) => name.clone(),
                                        },
                                    },
                                }),
                            }
                        }
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
    pub fn repeat(&mut self, variable: &str, count: u32, body: &[BufferEvent], index: usize) {
        self.repeat_typed(variable, count, &LoopIndex::I32, body, index);
    }

    /// [`Self::repeat`] with the index at its own §G02t type (card
    /// `W48-QR-SCALAR-TYPES-R5`). The count is checked against the index type
    /// again here — [`source_loop::index_admits`] — so a loop whose index
    /// cannot hold every value it takes, or its final step, is refused rather
    /// than iterated; each iteration binds the index at that type.
    pub fn repeat_typed(
        &mut self,
        variable: &str,
        count: u32,
        loop_index: &LoopIndex,
        body: &[BufferEvent],
        index: usize,
    ) {
        let index_type = loop_index.ty();
        // Card `W48-QR-SCALAR-TYPES-R3`: the environment is typed, and both of
        // its parts are scoped and shadow-checked by the one rule — a scalar
        // name is a scalar name whatever its type.
        let outer_scalars: BTreeSet<String> = self
            .evaluation
            .scalars
            .keys()
            .chain(self.evaluation.typed.keys())
            .cloned()
            .collect();
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
                        value: i128::from(count),
                    }
                }),
            });
            return;
        }
        if let Err(refusal) =
            source_loop::index_admits(count, index_type, self.evaluation.target.as_ref())
        {
            self.errors.push(BufferError::Loop {
                event: index,
                refusal: LoopRefusal::Bound(refusal),
            });
            return;
        }
        // Card `W48-QR-SCALAR-TYPES-R5`: a typed bound is a name, and the
        // lowering decided the count from its value in the lowering's tier.
        // The checker's own environment must agree — same type, same number
        // — or the loop is refused: the count is decided in the lowering, and
        // this is the second place that re-derives it rather than trusts it.
        if let LoopIndex::Typed { ty, bound } = loop_index {
            match self.evaluation.scalar_value(index, bound) {
                Ok((found, value)) if found == *ty && value == i128::from(count) => {}
                Ok(_) | Err(_) => {
                    self.errors.push(BufferError::Loop {
                        event: index,
                        refusal: LoopRefusal::Bound(BoundRefusal::BoundDisagrees {
                            bound: bound.clone(),
                            count,
                            ty: *ty,
                        }),
                    });
                    return;
                }
            }
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
            if index_type == ScalarType::I32 {
                self.evaluation.scalars.insert(
                    variable.to_owned(),
                    i32::try_from(iteration).expect("MAX_ITERATIONS is far inside i32"),
                );
            } else {
                self.evaluation
                    .typed
                    .insert(variable.to_owned(), (index_type, i128::from(iteration)));
            }
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
            .typed
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
                // both names are body-local. Card `AGGREGATE-OWNED-FIELD`: so
                // do a load and an extent read through a field — though an
                // owned-field statement in a body refuses before this matters
                // (`OWNED-FIELD-LOOP-BODY`).
                | BufferEvent::Construct { binding, .. }
                | BufferEvent::Field { binding, .. }
                | BufferEvent::FieldLoad { binding, .. }
                | BufferEvent::FieldExtent { binding, .. }
                // Card `OWNED-FIELD-ROUTE-REACH`: a copy bind binds a name,
                // and it is body-local exactly as every other binding is.
                | BufferEvent::CopyBind { binding, .. }
                // Card `W48-QR-SCALAR-TYPES-R3`: so does a conversion.
                | BufferEvent::Convert { binding, .. }
                // Card `W48-QR-SCALAR-TYPES-R5`: and a value statement.
                | BufferEvent::Compute { binding, .. } => {
                    names.insert(binding.clone());
                }
                BufferEvent::Acquire { name, .. } => {
                    names.insert(name.clone());
                }
                // Card `SLICE-PERMISSIONS-WRITE-LEND`: a lend binds a name,
                // and it is body-local exactly as every other binding is —
                // though a lend inside a body is refused by name before this
                // can matter (`LOOP-PER-ITERATION-LOAN`).
                BufferEvent::Lend { binding, .. } => {
                    names.insert(binding.clone());
                }
                BufferEvent::Repeat { variable, body, .. } => {
                    names.insert(variable.clone());
                    walk(body, names);
                }
                BufferEvent::Store { .. }
                | BufferEvent::Release { .. }
                | BufferEvent::FieldStore { .. }
                | BufferEvent::FieldRelease { .. }
                // Card `W48-QR-USABLE-RECORDS`: a field write binds nothing.
                | BufferEvent::FieldWrite { .. }
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

    /// [`Self::line_operand`] at the operand's own type (card
    /// `W48-QR-SCALAR-TYPES-R3`): a rendering takes a scalar of **any** §G02t
    /// type and renders it at that type. A literal is an `i32`.
    pub fn typed_line_operand(
        &self,
        event: usize,
        operand: &StdIoOperand,
    ) -> Result<(ScalarType, i128), BufferError> {
        match operand {
            StdIoOperand::Literal(value) => Ok((ScalarType::I32, i128::from(*value))),
            StdIoOperand::Scalar(name) => self.scalar_value(event, name),
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
    Moves,
    Lends,
    PeakLiveBytes,
    Accesses,
    Constructions,
    Projections,
    TypedProjections,
    Writes,
    Scalars,
    TypedScalars,
    TargetAssumption,
    Lines,
    Flushes,
    Renderings,
    /// Card `W48-QR-FLOAT-BUFFER-PAYLOADS`.
    FloatComparisons,
    Refused,
}

impl fmt::Display for BufferRecheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Self::Entry => "the entry name",
            Self::Effects => "the derived effect row",
            Self::Acquisitions => "the acquisitions and the extents they declare",
            Self::Releases => "the releases",
            Self::Moves => "the partial moves out of owned fields",
            Self::Lends => "the admitted write lends, their footprints and their windows",
            Self::PeakLiveBytes => "the peak live byte total",
            Self::Accesses => "the checked byte accesses",
            Self::Constructions => "the constructed record values and the fields they supply",
            Self::Projections => "the checked field reads",
            Self::TypedProjections => "the checked reads of fields of other scalar types",
            Self::Writes => "the checked field writes",
            Self::Scalars => "the scalar environment",
            Self::TypedScalars => "the environment of every other scalar type",
            Self::TargetAssumption => "the recorded-target assumption",
            Self::Lines => "the produced lines",
            Self::Flushes => "the barrier count",
            Self::Renderings => "the used renderings",
            Self::FloatComparisons => "the float comparison effects the statements used",
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
    recheck_buffer_certificate_for_target(program, certificate, None)
}

/// [`recheck_buffer_certificate`] against the builder's recorded target (card
/// `W48-QR-SCALAR-TYPES-R3`). The target is the builder's, passed in — never
/// read back out of the certificate being rechecked.
pub fn recheck_buffer_certificate_for_target(
    program: &BufferProgram,
    certificate: &BufferCertificate,
    target: Option<&RecordedTarget>,
) -> Result<(), BufferRecheckFailure> {
    let rederived = check_buffer_program_for_target(program, target)
        .map_err(|_| BufferRecheckFailure::Refused)?;
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
    if rederived.moves != certificate.moves {
        return Err(BufferRecheckFailure::Moves);
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
    if rederived.typed_projections != certificate.typed_projections {
        return Err(BufferRecheckFailure::TypedProjections);
    }
    if rederived.writes != certificate.writes {
        return Err(BufferRecheckFailure::Writes);
    }
    if rederived.scalars != certificate.scalars {
        return Err(BufferRecheckFailure::Scalars);
    }
    if rederived.typed_scalars != certificate.typed_scalars {
        return Err(BufferRecheckFailure::TypedScalars);
    }
    if rederived.target_assumption != certificate.target_assumption {
        return Err(BufferRecheckFailure::TargetAssumption);
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
    if rederived.float_comparisons != certificate.float_comparisons {
        return Err(BufferRecheckFailure::FloatComparisons);
    }
    Ok(())
}
