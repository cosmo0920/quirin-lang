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

//! `source.std-io.v1` — the checked model behind the `std.fmt`, `std.io` and
//! `std.logger` bundled modules (card `STD-MODULES`).
//!
//! # What this model is, stated before anything else
//!
//! A program on this route is a flat, ordered list of line-producing events in
//! one entry function. Each event renders **one scalar** and writes **one line**
//! to one of two streams, or hands the logger's stream to its sink.
//!
//! # The v1 effect discipline, and why it is sound by construction
//!
//! [`StdIoOperand::Literal`] and [`StdIoOperand::Scalar`] **cannot denote a
//! resource**. There is no operand form there for an owned buffer, a loan share
//! or a handle, so a rendering of either cannot consume, release, store or
//! retain custody of anything: it is not that the checker refuses those uses,
//! it is that the model cannot express them. That is why [`derived_effects`] is
//! exactly `{Io}` for every program this module accepts — never `Alloc`, never
//! `Mut`.
//!
//! # The borrowed render, and what it is exactly (card `STD-FMT-BORROW`)
//!
//! [`StdIoOperand::Borrowed`] is the third constructor and the one place this
//! route names a resource at all. It is spelled `borrow NAME` at the operand
//! position, and it renders **one thing and one thing only: the model identity
//! of that resource** — the same number the emitted C's own call trace spells
//! `rN` and the same number the checked custody model carries. It renders no
//! byte of the object, no address, no extent and no descriptor: the census
//! behind this card measured that no such projection exists in any model a
//! rendering can reach (report §2).
//!
//! Three properties make it sound, and none of them is decided here:
//!
//! 1. **Liveness is a precondition.** The number in [`StdIoProgram::identities`]
//!    is supplied by a checked custody model, and that model refuses to supply
//!    one for an identity it does not hold live at the rendering's own step.
//!    This route has no custody model of its own, so `identities` is **empty**
//!    on `source.std-io.v1` and every `borrow NAME` there refuses at `Q028` —
//!    structurally, exactly as an unbound scalar does.
//! 2. **Obligations are unchanged.** Rendering appends a line; it opens,
//!    discharges and moves nothing. The identity is as live after the rendering
//!    as before it, and its cleanup obligation is the same one.
//! 3. **A rendering is not a publication.** Standing constraint 10 refuses a
//!    *store* of a tracked identity into an object that outlives the call,
//!    because that creates a second reachable path to the resource and
//!    falsifies `Borrowed`. A rendering creates none: the destination is a byte
//!    stream and not an object of the custody graph, the bytes are a **copy of
//!    the identity number** rather than the address, and no operation of any
//!    model in this repository reads a stream back. There is no path from a
//!    written line to a custody identity, so nothing the caller owes changed.
//!
//! # What is not claimed
//!
//! * No interleaving order between the two streams is modeled or claimed. Each
//!   stream's own line order is checked; their relative order is not.
//! * [`StdIoEvent::Flush`] models **sink handoff**, not durability: the C
//!   rendering is `fflush`, and no `fsync`, no device write and no crash
//!   survival is claimed or measured (the O02/O03 boundary discipline).
//! * Rendering agreement between the reference execution and the emitted C is
//!   checked by comparison over the fixed forms below, not proved from a model
//!   of C's `printf` conversion specifiers.
//! * There is no format-string language here. A rendering is one of two closed
//!   constructors; nothing in a source program supplies a template.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::typed_errors::{Effect, EffectRow};

/// The `std.fmt` vocabulary: how one scalar becomes the bytes of a line.
///
/// Both forms are total on `i32`, so a rendering never fails and never has an
/// error exit. `Dec` is the signed decimal the sequential backend already
/// renders results with; `Hex` renders the unsigned reinterpretation in a fixed
/// eight-digit field, which is the one construct this card adds to the
/// backend's existing repertoire — it exists so the selector is non-vacuous and
/// so a wrong-rendering refusal is testable in both directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rendering {
    Dec,
    Hex,
}

impl Rendering {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "dec" => Some(Self::Dec),
            "hex" => Some(Self::Hex),
            _ => None,
        }
    }

    pub fn keyword(self) -> &'static str {
        match self {
            Self::Dec => "dec",
            Self::Hex => "hex",
        }
    }

    /// The exact bytes of the rendered field. The C backend must produce this
    /// same text; the two are compared, never assumed equal.
    pub fn render(self, value: i32) -> String {
        match self {
            Self::Dec => format!("{value}"),
            Self::Hex => format!("0x{:08X}", value as u32),
        }
    }

    /// The `printf` conversion the C backend uses for this rendering.
    pub fn c_conversion(self) -> &'static str {
        match self {
            Self::Dec => "%\" PRId32 \"",
            Self::Hex => "0x%08\" PRIX32 \"",
        }
    }

    /// The C expression that presents the operand to [`Self::c_conversion`].
    pub fn c_operand(self, expression: &str) -> String {
        match self {
            Self::Dec => expression.to_owned(),
            Self::Hex => format!("(uint32_t)({expression})"),
        }
    }
}

/// The ordered `std.logger` severity vocabulary. On a source-authored `log`
/// event it is a tag and filters nothing; on the distinct entry-level
/// `log_level` event it is the hosted system-trace threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl Level {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "trace" => Some(Self::Trace),
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            "fatal" => Some(Self::Fatal),
            _ => None,
        }
    }

    pub fn keyword(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Fatal => "fatal",
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
        }
    }
}

/// The two streams this route writes. They are separate observations: no order
/// between them is modeled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stream {
    /// `std.io` writes here.
    Out,
    /// `std.logger` writes here.
    Err,
}

impl Stream {
    pub fn label(self) -> &'static str {
        match self {
            Self::Out => "out",
            Self::Err => "err",
        }
    }

    pub fn c_handle(self) -> &'static str {
        match self {
            Self::Out => "stdout",
            Self::Err => "stderr",
        }
    }
}

/// The composable observable value of source-authored I/O.
///
/// Each stream retains its own order. Composition concatenates each projection
/// independently and deliberately states no order between distinct streams.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Streams {
    lines: BTreeMap<Stream, Vec<String>>,
}

impl Streams {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn singleton(stream: Stream, line: impl Into<String>) -> Self {
        let mut streams = Self::empty();
        streams.push(stream, line);
        streams
    }

    pub fn push(&mut self, stream: Stream, line: impl Into<String>) {
        self.lines.entry(stream).or_default().push(line.into());
    }

    /// Pointwise ordered concatenation.
    #[must_use]
    pub fn compose(mut self, other: Self) -> Self {
        for (stream, mut lines) in other.lines {
            self.lines.entry(stream).or_default().append(&mut lines);
        }
        self
    }

    #[must_use]
    pub fn project(&self, stream: Stream) -> &[String] {
        self.lines.get(&stream).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn from_lines(lines: &[ObservedLine]) -> Self {
        let mut streams = Self::empty();
        for line in lines {
            streams.push(line.stream, line.text.clone());
        }
        streams
    }
}

/// What a rendering may be applied to.
///
/// **This enum is the v1 effect discipline.** Two of its three constructors
/// cannot denote a resource at all, and the third denotes one only through the
/// identity a checked custody model published for it — never through the
/// object, its address, its extent or its contents. Adding a constructor that
/// carries an object is a semantic change requiring its own custody argument,
/// not a convenience.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StdIoOperand {
    /// An integer literal written at the site.
    Literal(i32),
    /// A name the entry bound to an integer literal with `let`.
    Scalar(String),
    /// `borrow NAME` — the **model identity** of a resource a checked custody
    /// model holds live at this rendering (card `STD-FMT-BORROW`).
    ///
    /// Resolved against [`StdIoProgram::identities`], which this route never
    /// fills: a `source.std-io.v1` module has no custody model, so every
    /// `borrow NAME` refuses there. See the module header for the custody
    /// argument.
    Borrowed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StdIoEvent {
    /// `log_level LEVEL;` — explicitly install the enclosing profile's
    /// system-event adapter at this minimum level. It emits no line itself.
    LogLevel { level: Level },
    /// `print RENDERING OPERAND;` — one line on [`Stream::Out`].
    Print {
        rendering: Rendering,
        operand: StdIoOperand,
    },
    /// `log LEVEL RENDERING OPERAND;` — one line on [`Stream::Err`].
    Log {
        level: Level,
        rendering: Rendering,
        operand: StdIoOperand,
    },
    /// `flush;` — hand every previously accepted logger line to the sink.
    /// Produces no line of its own, so it never moves either stream's
    /// observation.
    Flush,
    /// The entry's return. Exactly one, last.
    Return,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdIoProgram {
    pub entry: String,
    /// The integer values the entry bound with `let`, by name.
    pub scalars: BTreeMap<String, i32>,
    /// The **model identities** a checked custody model published for the names
    /// this entry renders with `borrow NAME` (card `STD-FMT-BORROW`).
    ///
    /// Deliberately a second map rather than more entries in `scalars`: the two
    /// namespaces never merge, a borrowed name is never a scalar and a scalar
    /// name is never borrowable. Empty on `source.std-io.v1`, which has no
    /// custody model — which is what makes the borrowed render refuse there
    /// structurally rather than by a rule.
    pub identities: BTreeMap<String, i32>,
    /// The values of every §G02t type other than `i32` a composing model bound
    /// for the names this entry renders (card `W48-QR-SCALAR-TYPES-R3`): the
    /// type and the number, rendered at that type by the §G02t `std.fmt` rules.
    /// A third map, disjoint from `scalars` — an `i32` keeps the rendering this
    /// route always gave it — and empty on `source.std-io.v1` itself, whose own
    /// `let` binds an `i32` literal only.
    pub typed_scalars: BTreeMap<String, (crate::scalar_types::ScalarType, i128)>,
    /// The builder's recorded target, which gives `isize`/`usize` their `hex`
    /// width (card `W48-QR-SCALAR-TYPES-R3`).
    pub target: Option<crate::scalar_types::RecordedTarget>,
    /// The entry's declared `effects { … }` row, verbatim from source.
    pub declared_effects: EffectRow,
    pub events: Vec<StdIoEvent>,
}

/// The derived effect row: exactly `{Io}` when the program writes or flushes,
/// and empty otherwise. `Alloc` and `Mut` are unreachable on this route by
/// construction — see the module header.
pub fn derived_effects(program: &StdIoProgram) -> EffectRow {
    let writes = program.events.iter().any(|event| {
        matches!(
            event,
            StdIoEvent::LogLevel { .. }
                | StdIoEvent::Print { .. }
                | StdIoEvent::Log { .. }
                | StdIoEvent::Flush
        )
    });
    if writes {
        EffectRow::new([Effect::Io])
    } else {
        EffectRow::default()
    }
}

/// One accepted line of the reference observation, before it is split by
/// stream. `Flush` contributes none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedLine {
    pub stream: Stream,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdIoCertificate {
    pub entry: String,
    /// The derived row, recomputed by the recheck and compared.
    pub effects: EffectRow,
    /// Every line this program produces, in program order across both streams.
    /// A consumer that needs one stream projects it; the cross-stream order
    /// here is the *program's* order, which is not claimed to be the order the
    /// two streams are observed in.
    pub lines: Vec<ObservedLine>,
    /// How many `flush` barriers the program performs.
    pub flushes: usize,
    /// The renderings actually used, so a certificate names its own `std.fmt`
    /// surface rather than the whole vocabulary.
    pub renderings: BTreeSet<Rendering>,
    /// `(name, identity)` of every borrowed render, in program order (card
    /// `STD-FMT-BORROW`). Published beside `lines` rather than folded into
    /// them, because it is the one field of this certificate that names a
    /// custody identity: a reader can see at a glance which lines are about a
    /// resource and which are data.
    pub borrowed_renders: Vec<(String, i32)>,
    /// Explicit system-event threshold, absent for the ordinary null sink.
    pub log_level: Option<Level>,
}

impl StdIoCertificate {
    /// The source-authored output as independently ordered stream projections.
    #[must_use]
    pub fn streams(&self) -> Streams {
        Streams::from_lines(&self.lines)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StdIoError {
    /// `Q028`: the operand does not denote a scalar this entry bound.
    UnboundScalar { event: usize, name: String },
    /// `Q028`: a `borrow NAME` operand for which no checked custody model
    /// published a live identity (card `STD-FMT-BORROW`). On
    /// `source.std-io.v1` this is *every* borrowed render, because that route
    /// has no custody model at all.
    UnborrowableIdentity { event: usize, name: String },
    /// `Q029`: the declared effect row does not admit the derived one.
    EffectNotDeclared { event: usize, effect: Effect },
    /// `Q030`: the flat entry shape this profile models is violated.
    Shape { event: usize, reason: &'static str },
}

impl StdIoError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnboundScalar { .. } | Self::UnborrowableIdentity { .. } => "Q028",
            Self::EffectNotDeclared { .. } => "Q029",
            Self::Shape { .. } => "Q030",
        }
    }

    pub fn event(&self) -> usize {
        match self {
            Self::UnboundScalar { event, .. }
            | Self::UnborrowableIdentity { event, .. }
            | Self::EffectNotDeclared { event, .. }
            | Self::Shape { event, .. } => *event,
        }
    }
}

impl fmt::Display for StdIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnboundScalar { name, .. } => write!(
                f,
                "`{name}` is not a scalar this entry bound; `source.std-io.v1` renders integer \
                 literals and `let`-bound integers only, and has no operand form that could \
                 denote a resource"
            ),
            Self::UnborrowableIdentity { name, .. } => write!(
                f,
                "`borrow {name}` renders the model identity of a live resource, and no checked \
                 custody model published one here; `source.std-io.v1` has no custody model of its \
                 own, so a borrowed render needs a route that composes this one with a model that \
                 holds the identity live at that statement"
            ),
            Self::EffectNotDeclared { effect, .. } => write!(
                f,
                "this entry performs `{effect:?}` but its declared `effects {{ … }}` row does not \
                 admit it"
            ),
            Self::Shape { reason, .. } => write!(f, "{reason}"),
        }
    }
}

/// The **one** place that decides what an accepted event's line looks like.
///
/// Both routes that produce these lines call it: `check_std_io_program` below,
/// and the composed task route (card `COMPOSE-CONCURRENCY`), whose operand
/// values are computed by a checked task model rather than bound by a `let` and
/// therefore cannot travel through [`StdIoProgram::scalars`]. Keeping the byte
/// decision here is what makes "the source lines of a composed program are
/// `source.std-io.v1`'s lines" a statement about code rather than about two
/// implementations that happen to agree.
///
/// `None` for an event that produces no line: [`StdIoEvent::Flush`] hands the
/// stream to its sink and [`StdIoEvent::Return`] is the entry's exit.
pub fn rendered_line(event: &StdIoEvent, value: i32) -> Option<ObservedLine> {
    match event {
        StdIoEvent::Print { rendering, .. } => Some(ObservedLine {
            stream: Stream::Out,
            text: rendering.render(value),
        }),
        StdIoEvent::Log {
            level, rendering, ..
        } => Some(ObservedLine {
            stream: Stream::Err,
            text: format!("[{}] {}", level.tag(), rendering.render(value)),
        }),
        StdIoEvent::LogLevel { .. } | StdIoEvent::Flush | StdIoEvent::Return => None,
    }
}

/// The stream a line goes to, and the framing that precedes its body.
///
/// `None` is `print` and `Some(level)` is `log LEVEL`, exactly as they are in
/// [`rendered_line`]. Published as one function so the composing routes cannot
/// invent a second framing: the `[TAG] ` prefix, the choice of stream and the
/// one trailing newline are decided here for **every** line this repository
/// writes, whether its body is a rendered scalar or a view's own bytes.
#[must_use]
pub fn line_framing(level: Option<Level>) -> (Stream, String) {
    match level {
        Option::None => (Stream::Out, String::new()),
        Some(level) => (Stream::Err, format!("[{}] ", level.tag())),
    }
}

/// One line whose body is **text supplied by the caller** rather than a rendered
/// scalar (card `STRING-SLICE-IO`).
///
/// The body is written verbatim: no re-encoding, no escaping, no terminator, and
/// no newline beyond the one the framing already defines. That is the whole
/// rendering rule for a string, and it is stated here rather than in the string
/// route so that the two line kinds provably share one framing —
/// [`rendered_line`] is this function applied to a rendered number, which
/// `a_rendered_scalar_line_is_a_text_line_of_its_digits` pins.
#[must_use]
pub fn rendered_text_line(level: Option<Level>, body: &str) -> ObservedLine {
    let (stream, prefix) = line_framing(level);
    ObservedLine {
        stream,
        text: format!("{prefix}{body}"),
    }
}

fn operand_value(
    program: &StdIoProgram,
    index: usize,
    operand: &StdIoOperand,
) -> Result<i32, StdIoError> {
    match operand {
        StdIoOperand::Literal(value) => Ok(*value),
        StdIoOperand::Scalar(name) => {
            program
                .scalars
                .get(name)
                .copied()
                .ok_or_else(|| StdIoError::UnboundScalar {
                    event: index,
                    name: name.clone(),
                })
        }
        // The two namespaces are read from two maps and never from one: a
        // scalar name is not borrowable and a borrowed name is not a scalar,
        // whatever either map happens to contain.
        StdIoOperand::Borrowed(name) => {
            program
                .identities
                .get(name)
                .copied()
                .ok_or_else(|| StdIoError::UnborrowableIdentity {
                    event: index,
                    name: name.clone(),
                })
        }
    }
}

/// Record a borrowed render in the certificate's own list. A literal or scalar
/// operand records nothing: the list names custody identities and only those.
fn record_borrow(operand: &StdIoOperand, value: i32, into: &mut Vec<(String, i32)>) {
    if let StdIoOperand::Borrowed(name) = operand {
        into.push((name.clone(), value));
    }
}

/// Check a lowered program and derive its certificate. Every refusal is
/// reported; the certificate is produced only when there are none.
pub fn check_std_io_program(program: &StdIoProgram) -> Result<StdIoCertificate, Vec<StdIoError>> {
    let mut errors = Vec::new();
    let mut lines = Vec::new();
    let mut flushes = 0;
    let mut renderings = BTreeSet::new();
    let mut borrowed_renders = Vec::new();
    let mut log_level = None;
    let mut returned = false;

    for (index, event) in program.events.iter().enumerate() {
        if returned {
            errors.push(StdIoError::Shape {
                event: index,
                reason: "this entry continues after its return; `source.std-io.v1` models one \
                         flat sequence ending in exactly one return",
            });
            break;
        }
        match event {
            StdIoEvent::LogLevel { level } => {
                if log_level.replace(*level).is_some() {
                    errors.push(StdIoError::Shape {
                        event: index,
                        reason: "`source.std-io.v1` accepts at most one `log_level LEVEL;` declaration",
                    });
                }
            }
            StdIoEvent::Print { rendering, operand }
            | StdIoEvent::Log {
                rendering, operand, ..
            } => {
                renderings.insert(*rendering);
                // Card `W48-QR-SCALAR-TYPES-R3`: a name a composing model bound
                // at another §G02t type renders at that type, framed by the one
                // line framing every route shares.
                if let StdIoOperand::Scalar(name) = operand
                    && let Some((ty, value)) = program.typed_scalars.get(name)
                {
                    let level = match event {
                        StdIoEvent::Log { level, .. } => Some(*level),
                        _ => None,
                    };
                    match crate::scalar_types::render(
                        *rendering == Rendering::Hex,
                        *ty,
                        *value,
                        program.target.as_ref(),
                    ) {
                        Some(body) => lines.push(rendered_text_line(level, &body)),
                        None => errors.push(StdIoError::Shape {
                            event: index,
                            reason: "this operand has no rendering here: a `bool` renders \
                                     `true`/`false` under `dec` and has no `hex`, and \
                                     `isize`/`usize` render only for a recorded target",
                        }),
                    }
                    continue;
                }
                match operand_value(program, index, operand) {
                    Ok(value) => {
                        record_borrow(operand, value, &mut borrowed_renders);
                        lines.push(
                            rendered_line(event, value)
                                .expect("a print and a log each produce exactly one line"),
                        );
                    }
                    Err(error) => errors.push(error),
                }
            }
            StdIoEvent::Flush => flushes += 1,
            StdIoEvent::Return => returned = true,
        }
    }

    if !returned {
        errors.push(StdIoError::Shape {
            event: program.events.len(),
            reason: "this entry never returns; `source.std-io.v1` requires exactly one return",
        });
    }

    let derived = derived_effects(program);
    for effect in derived.iter() {
        if !program.declared_effects.contains(effect) {
            errors.push(StdIoError::EffectNotDeclared {
                event: program.events.len(),
                effect: effect.clone(),
            });
        }
    }

    if errors.is_empty() {
        Ok(StdIoCertificate {
            entry: program.entry.clone(),
            effects: derived,
            lines,
            flushes,
            renderings,
            borrowed_renders,
            log_level,
        })
    } else {
        Err(errors)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StdIoRecheckFailure {
    Entry,
    Effects,
    Lines,
    Flushes,
    Renderings,
    BorrowedRenders,
    LogLevel,
    Refused,
}

impl fmt::Display for StdIoRecheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Self::Entry => "the entry name",
            Self::Effects => "the derived effect row",
            Self::Lines => "the produced lines",
            Self::Flushes => "the barrier count",
            Self::Renderings => "the used renderings",
            Self::BorrowedRenders => "the borrowed renders and the identities they name",
            Self::LogLevel => "the explicit system-event logger threshold",
            Self::Refused => "the program itself, which the recheck refuses",
        };
        write!(f, "{what} does not survive an independent rederivation")
    }
}

/// Rederive the certificate from the program alone and compare it field for
/// field. A published certificate is never one the driver did not recompute.
pub fn recheck_std_io_certificate(
    program: &StdIoProgram,
    certificate: &StdIoCertificate,
) -> Result<(), StdIoRecheckFailure> {
    let rederived = check_std_io_program(program).map_err(|_| StdIoRecheckFailure::Refused)?;
    if rederived.entry != certificate.entry {
        return Err(StdIoRecheckFailure::Entry);
    }
    if rederived.effects != certificate.effects {
        return Err(StdIoRecheckFailure::Effects);
    }
    if rederived.lines != certificate.lines {
        return Err(StdIoRecheckFailure::Lines);
    }
    if rederived.flushes != certificate.flushes {
        return Err(StdIoRecheckFailure::Flushes);
    }
    if rederived.renderings != certificate.renderings {
        return Err(StdIoRecheckFailure::Renderings);
    }
    if rederived.borrowed_renders != certificate.borrowed_renders {
        return Err(StdIoRecheckFailure::BorrowedRenders);
    }
    if rederived.log_level != certificate.log_level {
        return Err(StdIoRecheckFailure::LogLevel);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/std_io.rs"]
mod tests;
