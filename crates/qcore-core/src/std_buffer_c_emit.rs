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

//! Reference execution and C11 emission for `source.buffer.v1`
//! (card `STD-BUFFER`).
//!
//! # Self-contained by construction
//!
//! The emitted program links nothing. There is no extern declaration, no
//! library manifest and no boundary: `calloc`, `free` and the two accessors
//! below are the whole mechanism, and they are defined in the emitted file.
//! That is what distinguishes this tier from the native-library-backed
//! `std.buffer` of `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` §6, which is a
//! different tier of the same name and is untouched here.
//!
//! # The trap discipline, stated rather than implied
//!
//! [`BUFFER_PRELUDE`] aborts on an out-of-range index and on an allocation
//! failure. Both are **witnesses, not checks**:
//!
//! * Every index in an admitted program is a check-time constant this route's
//!   checker already proved in range, so the emitted comparison is provably
//!   never taken. It is emitted anyway because it costs nothing, because it
//!   makes the generated C readable as a standalone artifact, and — the point
//!   that matters — because **if it ever fired, that would be a defect in the
//!   checker**. A sanitized run that completes is therefore evidence about the
//!   checker and not only about the program.
//! * `calloc` can fail, and v1 aborts rather than modelling a failure arm,
//!   which makes every claim on this route a **partial-correctness** statement:
//!   *if the emitted program produces an observation at all, it is the
//!   reference observation.* An aborting run produces none and contradicts
//!   none. Modelling the arm needs `source.branch.v1`'s join, which is a
//!   different pass's construct (`STD-BUFFER-ALLOC-FAILURE`).
//!
//! `abort()` is not catchable and produces no QCore error value. Neither trap
//! is a recovery mechanism and neither makes a varying index admissible.
//!
//! # Why `calloc` and not `malloc`
//!
//! The model's buffer is zero at acquisition, so a read before a write has a
//! defined answer. `malloc` would leave the bytes indeterminate, which is
//! undefined behaviour to read natively and unpredictable in the reference —
//! the per-line comparison this route rests on would be unwritable. `calloc`
//! is what makes the two agree, and it is not an optimisation detail.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::c_program::{self, Fragment, Section};
use crate::io_emit;
use crate::scalar_types::{self as st, ScalarType};
use crate::source_record::{FieldType, RecordType};
use crate::std_buffer::{
    self, BufferCertificate, BufferEvent, BufferOperand, BufferProgram, FieldOperand, MAX_EXTENT,
};
use crate::std_io::{Rendering, StdIoEvent, StdIoOperand};
use crate::std_io_c_emit;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BufferEmissionRefusal {
    /// A name the certificate's program does not bind. The checker already
    /// refuses this; the backend refuses it again rather than emitting an
    /// expression it cannot justify.
    Unbound { event: usize, name: String },
    /// An access the checker admitted but the backend cannot place. Unreachable
    /// for a checked program, and refused rather than emitted.
    UnplaceableAccess { event: usize },
    /// A `for` statement reached a caller that cannot emit one (the
    /// `LOOP-V1` × `STRING-CONCAT` composition).
    ///
    /// Emitting a loop needs the **recursion** [`emit_events`] has, so
    /// [`buffer_event_statement`] — which returns one statement's text and has
    /// no way to emit a block — refuses rather than returning `None`. Returning
    /// `None` would have been the one fail-open shape available here: a caller
    /// that could not recurse would have silently emitted nothing for the loop
    /// and dropped every write its body performs.
    UnemittableLoop { event: usize },
    /// A write lend reached a caller that cannot emit one (card
    /// `SLICE-PERMISSIONS-WRITE-LEND`) — the [`Self::UnemittableLoop`] shape
    /// and for its reason: a lend and every access through it need the loan
    /// **environment** [`emit_events`] carries (which owner, which
    /// displacement, which length), and [`buffer_event_statement`] has none.
    /// The one caller without it is the composed string/buffer route, whose
    /// checker refuses a lend before emission (`SLICE-PERMISSIONS-COMPOSED-LEND`).
    UnemittableLend { event: usize },
    /// A record construction or a field read reached a caller that cannot emit
    /// one (card `AGGREGATES-V1`).
    ///
    /// Exactly the [`Self::UnemittableLoop`] shape and for its reason. Emitting
    /// either needs the record **environment** [`emit_events`] carries — which
    /// C object a binding names and which declared type it has — and
    /// [`buffer_event_statement`] has no such environment. Returning `None`
    /// would have been the one fail-open shape available here: a caller that
    /// could not emit the construction would have silently emitted nothing for
    /// it and left every later field read addressing an object that was never
    /// declared. The composed string/buffer route is such a caller, and this is
    /// the refusal it receives (`AGGREGATE-STRING-COMPOSE`).
    UnemittableRecord { event: usize },
    /// A conversion reached a caller that cannot emit one (card
    /// `W48-QR-SCALAR-TYPES-R3`): the [`Self::UnemittableRecord`] shape, for
    /// its reason — a conversion's source may be a name of any §G02t type,
    /// which only [`emit_events`]'s typed environment knows. The one caller
    /// without it is the composed string/buffer route, which lowers none.
    UnemittableConversion { event: usize },
    /// A value statement reached a caller that cannot emit one (card
    /// `W48-QR-SCALAR-TYPES-R5`), for the conversion's reason: its operands
    /// may be names of any §G02t type, which only [`emit_events`]'s typed
    /// environment knows. The composed string/buffer route refuses one first.
    UnemittableValueStatement { event: usize },
    /// A certificate whose lines disagree with the program's.
    CertificateDisagrees,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferCEmission {
    pub source: String,
}

/// The header of this route's C fragment: what the mechanism is, and what the
/// two `abort()`s in it are and are not.
pub const BUFFER_PRELUDE_HEADER: &str = r#"
/* std.buffer / source.buffer.v1 (card STD-BUFFER): the whole mechanism of an
   owned byte buffer, defined here rather than linked. `calloc` and not
   `malloc`, because the model's buffer is zero at acquisition and an
   indeterminate byte would be undefined behaviour to read.

   The bounds comparison and the allocation check are WITNESSES, not checks.
   Every index in an admitted program is a check-time constant the checker
   already proved in range, so the comparisons below are provably never taken;
   if one ever fired, that would be a defect in the checker. `abort()` is not
   catchable and yields no QCore error value: a failing run produces no
   observation, which is why every claim on this route is a partial-correctness
   statement.

   Only the accessors this program uses are emitted. These are file-scope
   `static` functions and the emitted C is built with -Wall -Wextra -Werror
   (and /W4 /WX on MSVC), where an unused one is an error — so a program that
   never reads a byte must not carry a reader. */
"#;

/// The acquisition and release helpers. Every buffer program has both, because
/// the inherited ownership checker refuses a program that acquires without
/// releasing and there is nothing else to acquire.
pub const BUFFER_PRELUDE_LIFETIME: &str = r#"static unsigned char *qcore_bytes_new(size_t extent)
{
    unsigned char *bytes = (unsigned char *) calloc(extent, 1u);
    if (bytes == NULL) {
        abort();
    }
    return bytes;
}

static void qcore_bytes_drop(unsigned char *bytes)
{
    free(bytes);
}
"#;

/// The byte writer, emitted only for a program that writes one.
pub const BUFFER_PRELUDE_STORE: &str = r#"
static void qcore_bytes_store(unsigned char *bytes, size_t extent, size_t index, int32_t value)
{
    if (index >= extent) {
        abort();
    }
    bytes[index] = (unsigned char) value;
}
"#;

/// The byte reader, emitted only for a program that reads one.
pub const BUFFER_PRELUDE_LOAD: &str = r#"
static int32_t qcore_bytes_load(const unsigned char *bytes, size_t extent, size_t index)
{
    if (index >= extent) {
        abort();
    }
    return (int32_t) bytes[index];
}
"#;

/// The C fragment for a program holding buffers: the header, the lifetime pair,
/// and each accessor the program actually uses.
///
/// Published since card `STRING-CONCAT` so the composed string/buffer route
/// assembles **this** prelude rather than a second copy of it. The two booleans
/// are the two conditional accessors, and they are the caller's because a
/// composed program's events are not all [`BufferEvent`]s.
///
/// Every caller must compute them over **every depth**, which is card
/// `LOOP-V1`'s rule: a program whose only byte write is inside a `for` body
/// still writes one, and emitting no writer for it would not compile. Both
/// callers do — this route through [`std_buffer::any_event`] below, and the
/// composed string/buffer route through the same function — so the scan is
/// depth-recursive wherever the prelude is assembled.
#[must_use]
pub fn buffer_prelude_for(stores: bool, loads: bool) -> String {
    let mut prelude = String::from(BUFFER_PRELUDE_HEADER);
    prelude.push_str(BUFFER_PRELUDE_LIFETIME);
    if stores {
        prelude.push_str(BUFFER_PRELUDE_STORE);
    }
    if loads {
        prelude.push_str(BUFFER_PRELUDE_LOAD);
    }
    prelude
}

fn buffer_prelude(program: &BufferProgram) -> String {
    // Card `AGGREGATES-V1`: a program that acquires **no** buffer carries no
    // buffer prelude at all, not even the lifetime pair.
    //
    // This is byte-neutral for every program admitted before that card, and
    // that is a measurement rather than a hope: the prelude's helpers are
    // `static`, the emitted C is built with `-Wall -Wextra -Werror` where an
    // unused one is an error, and among *admitted* programs "reads or writes a
    // byte" implies "acquires a buffer" — a store, a load or an extent read
    // naming nothing this entry acquired is already refused at `Q052`. So no
    // program that reached this function before could have failed the test
    // below. What the card adds is the first program that can: a module that
    // declares a record and prints, holding no buffer at all.
    if !std_buffer::any_event(&program.events, |event| {
        matches!(event, BufferEvent::Acquire { .. })
    }) {
        return String::new();
    }
    // `any_event` and not a flat `iter().any(…)`: the scan is over every event
    // at **every depth** (card `LOOP-V1`), because a program whose only byte
    // write is inside a `for` body still writes one and emitting no writer for
    // it would not compile.
    // Card `AGGREGATE-OWNED-FIELD`: a store or a load **through a field** is
    // a store or a load, and needs its accessor exactly as the plain-name one
    // does. Neither can occur in a program that acquires nothing: a field can
    // hold only a buffer `bytes(N)` acquired, so the scan above still decides
    // whether the prelude exists at all.
    buffer_prelude_for(
        std_buffer::any_event(&program.events, |event| {
            matches!(
                event,
                BufferEvent::Store { .. } | BufferEvent::FieldStore { .. }
            )
        }),
        std_buffer::any_event(&program.events, |event| {
            matches!(
                event,
                BufferEvent::Load { .. } | BufferEvent::FieldLoad { .. }
            )
        }),
    )
}

const INCLUDES: &str = "\
#include <inttypes.h>\n\
#include <stdint.h>\n\
#include <stdio.h>\n\
#include <stdlib.h>\n";

/// The C name of a buffer binding. Published since card `STRING-CONCAT`: a
/// borrowed string view addresses this object, so the composed route must spell
/// the same name this one does.
#[must_use]
pub fn buffer_name(name: &str) -> String {
    format!("b_{name}")
}

/// The C name of a scalar binding.
fn scalar_name(name: &str) -> String {
    format!("s_{name}")
}

/// The C name of a write lend's binding (card `SLICE-PERMISSIONS-WRITE-LEND`).
///
/// A fourth prefix beside `b_`, `s_` and `r_`, so a loan, a buffer, a scalar
/// and a record of one source name are distinct C objects. The object is a
/// `const` **pointer** into the owner's block, displaced by the lend's low
/// bound — the rebasing arithmetic, emitted once at the lend rather than at
/// every access.
#[must_use]
pub fn lend_name(name: &str) -> String {
    format!("l_{name}")
}

/// The C name of a record binding (card `AGGREGATES-V1`).
///
/// A third prefix beside `b_` and `s_`, so a record, a buffer and a scalar of
/// one source name are three distinct C objects and no emitted expression can
/// read one for another.
#[must_use]
pub fn record_binding_name(name: &str) -> String {
    format!("r_{name}")
}

/// The C tag of a declared record type.
///
/// Prefixed rather than spelled through, because a QCore type name is a QCore
/// word and nothing stops it being a C keyword. The prefix is what makes
/// `type int { … }` emit a compilable struct instead of a syntax error.
#[must_use]
pub fn record_type_tag(type_name: &str) -> String {
    format!("qcore_record_{type_name}")
}

/// The C member name of a declared field, prefixed for the reason
/// [`record_type_tag`] is.
#[must_use]
pub fn record_field_name(field: &str) -> String {
    format!("f_{field}")
}

/// The C type definitions the module's record declarations contribute.
///
/// Emitted in **declaration order**, with each field in its own declared order,
/// so the emitted struct is the type the source wrote rather than a
/// reconstruction of it. A module declaring no record contributes the empty
/// string, which is what keeps every program written before card
/// `AGGREGATES-V1` byte-identical: [`c_program::assemble`] drops an empty
/// fragment.
#[must_use]
pub fn record_types_text(records: &[RecordType]) -> String {
    if records.is_empty() {
        return String::new();
    }
    let mut text = String::from(RECORD_PRELUDE_HEADER);
    // Card `AGGREGATE-OWNED-FIELD`: the paragraph about owned members is
    // written only for a module that declares one, so a module of scalar
    // records emits the bytes it always emitted.
    if records.iter().any(RecordType::has_owned_field) {
        text.push_str(RECORD_PRELUDE_OWNED);
    }
    // Card `W48-QR-SCALAR-TYPES-R3`: likewise the paragraph about members of
    // the other fixed-width types, written only for a module declaring one.
    if records.iter().any(declares_typed_field) {
        text.push_str(RECORD_PRELUDE_TYPED);
    }
    // Card `W48-QR-USABLE-RECORDS`: likewise the paragraph about record
    // members, written only for a module declaring one.
    if records.iter().any(declares_record_field) {
        text.push_str(RECORD_PRELUDE_NESTED);
    }
    for record in emission_order(records) {
        let _ = writeln!(text, "struct {} {{", record_type_tag(&record.name));
        for field in &record.fields {
            let member = record_field_name(&field.name);
            match &field.field_type {
                // `int32_t` for an `i32`, exactly as before the card, and each
                // other type's `<stdint.h>`/`<stdbool.h>`/`<stddef.h>` spelling.
                FieldType::Scalar(ty) => {
                    let _ = writeln!(text, "    {} {member};", st::c_type(*ty));
                }
                FieldType::Bytes => {
                    let _ = writeln!(text, "    unsigned char *{member};");
                }
                // Card `W48-QR-USABLE-RECORDS`: a record member by value.
                FieldType::Record(inner) => {
                    let _ = writeln!(text, "    struct {} {member};", record_type_tag(inner));
                }
            }
        }
        let _ = writeln!(text, "}};");
    }
    text
}

/// The records in the order C must define them (card
/// `W48-QR-USABLE-RECORDS`): declaration order, except that a record held by
/// value by another is defined first, since a member of struct type needs a
/// complete type. For a module declaring no record field this **is**
/// declaration order, so its emitted bytes are unchanged. The graph is acyclic
/// (`source_record::admit_graph`), so the walk ends.
fn emission_order(records: &[RecordType]) -> Vec<&RecordType> {
    fn visit<'a>(
        records: &'a [RecordType],
        record: &'a RecordType,
        done: &mut Vec<&'a str>,
        order: &mut Vec<&'a RecordType>,
    ) {
        if done.contains(&record.name.as_str()) {
            return;
        }
        done.push(&record.name);
        for field in &record.fields {
            if let FieldType::Record(inner) = &field.field_type
                && let Some(held) = records.iter().find(|candidate| candidate.name == *inner)
            {
                visit(records, held, done, order);
            }
        }
        order.push(record);
    }
    let mut done = Vec::new();
    let mut order = Vec::new();
    for record in records {
        visit(records, record, &mut done, &mut order);
    }
    order
}

/// Whether a record declares a field whose type is a record (card
/// `W48-QR-USABLE-RECORDS`).
fn declares_record_field(record: &RecordType) -> bool {
    record
        .fields
        .iter()
        .any(|field| matches!(field.field_type, FieldType::Record(_)))
}

/// The paragraph a module declaring a record member adds to
/// [`RECORD_PRELUDE_HEADER`] (card `W48-QR-USABLE-RECORDS`), appended rather
/// than edited in so the header's bytes are unchanged for every other module.
pub const RECORD_PRELUDE_NESTED: &str = r#"
/* W48-QR-USABLE-RECORDS: a member declared with a record type holds that
   record by value, as a member of its struct type, defined above it. A
   construction copies the named record into it (for a record holding an
   owned buffer, the pointers move with it and the source binding is dead),
   a source path `a.b.c` is the member access `r_a.f_b.f_c`, and a field
   write is one member assignment of a value the checker already proved to
   be a value of the member's type. */
"#;

/// Whether a record declares a scalar field of a type other than `i32` (card
/// `W48-QR-SCALAR-TYPES-R3`).
fn declares_typed_field(record: &RecordType) -> bool {
    record
        .fields
        .iter()
        .any(|field| matches!(field.field_type, FieldType::Scalar(ty) if ty != ScalarType::I32))
}

/// Whether a program names a §G02t type other than `i32` anywhere — a field
/// type or a conversion target — and so needs `<stdbool.h>`/`<stddef.h>`
/// (card `W48-QR-SCALAR-TYPES-R3`). False for every program before the card,
/// which is what keeps their includes byte-identical.
#[must_use]
pub fn uses_typed_scalars(program: &BufferProgram) -> bool {
    program.records.iter().any(declares_typed_field)
        || std_buffer::any_event(&program.events, |event| match event {
            BufferEvent::Convert { to, .. } => *to != ScalarType::I32,
            // Card `W48-QR-SCALAR-TYPES-R5`: a loop with an index of another
            // type declares it at that type's C spelling.
            BufferEvent::Repeat { index, .. } => index.ty() != ScalarType::I32,
            // Card `W48-QR-SCALAR-TYPES-R5`: a value statement may produce a
            // `bool` (a comparison).
            BufferEvent::Compute { .. } => true,
            _ => false,
        })
}

/// The paragraph a module declaring a field of another fixed-width type adds
/// to [`RECORD_PRELUDE_HEADER`] (card `W48-QR-SCALAR-TYPES-R3`), appended
/// rather than edited in so the header's bytes are unchanged for every module
/// of `i32` records.
pub const RECORD_PRELUDE_TYPED: &str = r#"
/* W48-QR-SCALAR-TYPES-R3: a member may be any fixed-width scalar of the
   source's table, declared at its own C11 type (`int8_t` ... `uint64_t`,
   `bool`, and `ptrdiff_t`/`size_t` for isize/usize at the recorded target's
   width). The paragraph above about `i32` members holds for every member of
   each type: a distinct scalar object, owning nothing, initialized from a
   value the checker already proved to be a value of that type. */
"#;

/// What the emitted struct is, and what it is not.
pub const RECORD_PRELUDE_HEADER: &str = r#"
/* source.record.v1 (card AGGREGATES-V1): one C struct per declared record type.

   Every field of a v1 record is an `i32`, so each member is a distinct object
   of the same scalar type and the struct owns nothing: there is no allocation,
   no destructor and no custody obligation anywhere in this file on its account.
   A value is constructed with a designated initializer naming every declared
   field exactly once — the checker already refused a construction that does
   not — so no member is left indeterminate and no field read is undefined.

   The member names carry an `f_` prefix and the tags a `qcore_record_` prefix
   so that a QCore field or type name which happens to be a C keyword still
   emits a compilable struct. */
"#;

/// The paragraph a module with an owned field adds to the header above (card
/// `AGGREGATE-OWNED-FIELD`). Appended rather than edited into
/// [`RECORD_PRELUDE_HEADER`], so the header's bytes are unchanged for every
/// module of scalar records.
pub const RECORD_PRELUDE_OWNED: &str = r#"
/* AGGREGATE-OWNED-FIELD: a member declared `bytes` in the source holds the
   pointer of an owned byte buffer, MOVED into the struct by the designated
   initializer that names it. The struct then owes that buffer's release:
   `drop RECORD;` in the source is one `qcore_bytes_drop` per member still
   held, in declaration order, and `let NAME = RECORD.FIELD;` hands the
   pointer to a plain binding that owes it instead. The checker decided every
   move and every release before emission; the paragraph above about scalar
   members is unchanged for them. */
"#;

/// The reference observation, as `"<stream>: <line>"` records in program order.
/// A consumer projects one stream by filtering on the prefix.
///
/// The lines are the **certificate's** lines: the checker already computed
/// every rendered value out of the modelled bytes, so re-deriving them here
/// would be a second implementation of the same arithmetic rather than a check
/// of the first.
/// How many lines this sequence produces when it runs — a `for` body's lines
/// counted once per iteration, because that is how many times they are
/// written. Published since card `LOOPS-ON-COMPOSED-ROUTES` so the composed
/// string route counts an embedded loop's lines by the same rule.
#[must_use]
pub fn produced_lines(events: &[BufferEvent]) -> usize {
    events
        .iter()
        .map(|event| match event {
            BufferEvent::Line(StdIoEvent::Print { .. } | StdIoEvent::Log { .. }) => 1,
            BufferEvent::Repeat { count, body, .. } => {
                (*count as usize).saturating_mul(produced_lines(body))
            }
            _ => 0,
        })
        .sum()
}

pub fn interpret_buffer_program(
    program: &BufferProgram,
    certificate: &BufferCertificate,
) -> Result<Vec<String>, BufferEmissionRefusal> {
    let produced = produced_lines(&program.events);
    if produced != certificate.lines.len() {
        return Err(BufferEmissionRefusal::CertificateDisagrees);
    }
    Ok(certificate
        .lines
        .iter()
        .map(|line| format!("{}: {}", line.stream.label(), line.text))
        .collect())
}

/// The C expression for a value a construction supplies at a scalar field of
/// type `ty` (card `W48-QR-SCALAR-TYPES-R3`). An `i32` field is written exactly
/// as before the card; a literal of another type is that type's C constant,
/// and a name is its C object, whose type the checker proved to be `ty`.
fn field_expression(
    scalars: &BTreeMap<String, String>,
    typed: &BTreeMap<String, (String, ScalarType)>,
    event: usize,
    ty: ScalarType,
    operand: &FieldOperand,
) -> Result<String, BufferEmissionRefusal> {
    match (ty, operand) {
        (ScalarType::I32, FieldOperand::Literal(value)) => Ok(format!("INT32_C({value})")),
        (_, FieldOperand::Literal(value)) => Ok(st::c_literal(ty, *value)),
        (_, FieldOperand::Bool(value)) => Ok(st::c_literal(ty, i128::from(*value))),
        // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: a float literal is its bits at
        // the field's type, the value the checker read once.
        (_, FieldOperand::Float(literal)) => literal
            .at(ty)
            .map(|bits| st::c_literal(ty, bits))
            .map_err(|_| BufferEmissionRefusal::UnemittableRecord { event }),
        (_, FieldOperand::Name(name)) => scalars
            .get(name)
            .cloned()
            .or_else(|| typed.get(name).map(|(emitted, _)| emitted.clone()))
            .ok_or_else(|| BufferEmissionRefusal::Unbound {
                event,
                name: name.clone(),
            }),
    }
}

/// The C name of the checked conversion helper from `from` to `to` (card
/// `W48-QR-SCALAR-TYPES-R3`).
fn conversion_helper_name(from: ScalarType, to: ScalarType) -> String {
    format!("qcore_buffer_as_{from}_{to}")
}

/// The definition of one checked conversion helper. Its test is §G02t's shared
/// one (`scalar_types::c_conversion_test`), decided on the operand before the
/// cast, and it **aborts** — this route's witness discipline: every conversion
/// in an admitted program is one the checker already proved to fit, so the
/// test is provably never taken, and if it ever fired that would be a defect
/// in the checker.
fn conversion_helper(
    from: ScalarType,
    to: ScalarType,
    target: Option<&st::RecordedTarget>,
) -> Option<String> {
    let test = st::c_conversion_test(from, to, target)?;
    let (from_c, to_c) = (st::c_type(from), st::c_type(to));
    Some(format!(
        "\nstatic {to_c} {}({from_c} v)\n{{\n    if ({test}) {{\n        abort();\n    }}\n    return ({to_c}) v;\n}}\n",
        conversion_helper_name(from, to)
    ))
}

/// The C expression for one index or stored byte.
fn operand_expression(
    scalars: &BTreeMap<String, String>,
    event: usize,
    operand: &BufferOperand,
) -> Result<String, BufferEmissionRefusal> {
    match operand {
        BufferOperand::Literal(value) => Ok(format!("INT32_C({value})")),
        BufferOperand::Scalar(name) => {
            scalars
                .get(name)
                .cloned()
                .ok_or_else(|| BufferEmissionRefusal::Unbound {
                    event,
                    name: name.clone(),
                })
        }
    }
}

/// The C statement one **value-bearing** buffer event emits, without its
/// indentation.
///
/// Published since card `STRING-CONCAT` so the composed string/buffer route
/// emits these statements rather than a second implementation of them: a
/// composed program's `bytes(8)` is the *same* `qcore_bytes_new` call this
/// route's is, with the same C names, so the two backends cannot drift.
///
/// [`BufferEvent::Line`] and [`BufferEvent::Return`] return `None` — a line is
/// `source.std-io.v1`'s own emitter's and a return is the assembler's.
pub fn buffer_event_statement(
    index: usize,
    event: &BufferEvent,
    scalars: &mut BTreeMap<String, String>,
    extents: &mut BTreeMap<String, u32>,
) -> Result<Option<String>, BufferEmissionRefusal> {
    let statement = match event {
        BufferEvent::Bind { binding, value } => {
            let emitted = scalar_name(binding);
            scalars.insert(binding.clone(), emitted.clone());
            format!("int32_t {emitted} = INT32_C({value});")
        }
        // Card `OWNED-FIELD-ROUTE-REACH`: the **scalar** half of the copy
        // bind. A copy of a record needs the record environment this function
        // does not carry, so it is refused here and emitted by `emit_events`
        // — the `Construct` rule, for the `Construct` reason. A copy of
        // anything the ledger holds never reaches emission: the checker
        // refused it at `Q054`.
        BufferEvent::CopyBind { binding, source } => {
            let Some(from) = scalars.get(source).cloned() else {
                return Err(BufferEmissionRefusal::UnemittableRecord { event: index });
            };
            let emitted = scalar_name(binding);
            scalars.insert(binding.clone(), emitted.clone());
            format!("int32_t {emitted} = {from};")
        }
        BufferEvent::Acquire { name, extent } => {
            debug_assert!(*extent > 0 && *extent <= MAX_EXTENT);
            extents.insert(name.clone(), *extent);
            format!(
                "unsigned char *{} = qcore_bytes_new((size_t) {extent}u);",
                buffer_name(name)
            )
        }
        BufferEvent::Store {
            name,
            index: at,
            value,
        } => {
            let Some(extent) = extents.get(name).copied() else {
                return Err(BufferEmissionRefusal::UnplaceableAccess { event: index });
            };
            let at = operand_expression(scalars, index, at)?;
            let value = operand_expression(scalars, index, value)?;
            format!(
                "qcore_bytes_store({}, (size_t) {extent}u, (size_t) ({at}), {value});",
                buffer_name(name)
            )
        }
        BufferEvent::Load {
            binding,
            name,
            index: at,
        } => {
            let Some(extent) = extents.get(name).copied() else {
                return Err(BufferEmissionRefusal::UnplaceableAccess { event: index });
            };
            let at = operand_expression(scalars, index, at)?;
            let emitted = scalar_name(binding);
            scalars.insert(binding.clone(), emitted.clone());
            format!(
                "int32_t {emitted} = qcore_bytes_load({}, (size_t) {extent}u, (size_t) ({at}));",
                buffer_name(name)
            )
        }
        BufferEvent::Extent { binding, name } => {
            let Some(extent) = extents.get(name).copied() else {
                return Err(BufferEmissionRefusal::UnplaceableAccess { event: index });
            };
            // The extent is a number the checker determined out of the
            // ownership ledger, so it is emitted as a literal exactly as
            // this route's `let`-bound integers are. The emitted program
            // reads no length out of the object, and there is none to read:
            // a `calloc`ed block carries no queryable size in portable C.
            let emitted = scalar_name(binding);
            scalars.insert(binding.clone(), emitted.clone());
            format!("int32_t {emitted} = INT32_C({extent});")
        }
        BufferEvent::Release { name } => {
            format!("qcore_bytes_drop({});", buffer_name(name))
        }
        BufferEvent::Line(_) | BufferEvent::Return => return Ok(None),
        // **Refused, not skipped** — see
        // [`BufferEmissionRefusal::UnemittableConversion`].
        BufferEvent::Convert { .. } => {
            return Err(BufferEmissionRefusal::UnemittableConversion { event: index });
        }
        // **Refused, not skipped** — see
        // [`BufferEmissionRefusal::UnemittableValueStatement`].
        BufferEvent::Compute { .. } => {
            return Err(BufferEmissionRefusal::UnemittableValueStatement { event: index });
        }
        // **Refused, not skipped.** A loop is a block, not a statement, and
        // this function returns one statement's text. `emit_events` handles a
        // `Repeat` in its own arm and never reaches here; a caller that cannot
        // recurse gets a refusal rather than an empty emission.
        BufferEvent::Repeat { .. } => {
            return Err(BufferEmissionRefusal::UnemittableLoop { event: index });
        }
        // **Refused, not skipped** — the same rule, for the same reason. See
        // [`BufferEmissionRefusal::UnemittableRecord`]. Card
        // `AGGREGATE-OWNED-FIELD`'s field-path statements need the same record
        // environment and are refused here for the same reason.
        BufferEvent::Construct { .. }
        | BufferEvent::Field { .. }
        | BufferEvent::FieldStore { .. }
        | BufferEvent::FieldLoad { .. }
        | BufferEvent::FieldExtent { .. }
        | BufferEvent::FieldRelease { .. }
        // Card `W48-QR-USABLE-RECORDS`: a field write likewise.
        | BufferEvent::FieldWrite { .. } => {
            return Err(BufferEmissionRefusal::UnemittableRecord { event: index });
        }
        // **Refused, not skipped** — see [`BufferEmissionRefusal::UnemittableLend`].
        BufferEvent::Lend { .. } => {
            return Err(BufferEmissionRefusal::UnemittableLend { event: index });
        }
    };
    Ok(Some(statement))
}

/// Emit C11 for a checked program. The emitted lines must reproduce the
/// reference observation exactly, per stream.
pub fn emit_buffer_c11(
    program: &BufferProgram,
    certificate: &BufferCertificate,
) -> Result<BufferCEmission, BufferEmissionRefusal> {
    // Refuse before emitting anything if the certificate is not this program's.
    let _ = interpret_buffer_program(program, certificate)?;

    let mut fragments = preamble(program);
    let mut source = String::new();

    // Every binding is declared at its own statement, in program order: unlike
    // `source.std-io.v1`, whose scalars are all literals known before the first
    // event, a scalar here can be the result of a byte read and therefore has
    // no value until its event runs.
    let mut scalars: BTreeMap<String, String> = BTreeMap::new();
    let mut extents: BTreeMap<String, u32> = BTreeMap::new();
    let mut records = RecordEnvironment {
        types: program
            .records
            .iter()
            .map(|record| (record.name.clone(), record.clone()))
            .collect(),
        bound: BTreeMap::new(),
        field_extents: BTreeMap::new(),
        live_fields: BTreeMap::new(),
        typed: BTreeMap::new(),
        conversions: BTreeSet::new(),
        helpers: BTreeSet::new(),
        comparisons: BTreeSet::new(),
        target: certificate.target_assumption.clone(),
        discard_unused: true,
        renderers: BTreeSet::new(),
        floats: std_buffer::uses_floats(program),
    };

    let lends: BTreeMap<usize, &std_buffer::LendRecord> = certificate
        .lends
        .iter()
        .map(|record| (record.event, record))
        .collect();
    let mut loans: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    emit_events(
        &program.events,
        "    ",
        0,
        true,
        &mut scalars,
        &mut extents,
        &mut loans,
        &lends,
        &mut records,
        &mut source,
    )?;

    // Card `W48-QR-SCALAR-TYPES-R3`: the recorded target's width assertions
    // and the conversion helpers this program used — both empty, and so no
    // fragment at all, for every program written before the card.
    let mut typed_prelude = String::new();
    if let Some(target) = &certificate.target_assumption {
        typed_prelude.push('\n');
        typed_prelude.push_str(&st::c_target_assertions(target));
    }
    for (from, to) in &records.conversions {
        match conversion_helper(*from, *to, certificate.target_assumption.as_ref()) {
            Some(helper) => typed_prelude.push_str(&helper),
            None => return Err(BufferEmissionRefusal::UnemittableConversion { event: 0 }),
        }
    }
    // Card `W48-QR-SCALAR-TYPES-R5`: the shared checked helpers the value
    // statements call — §G02t's own definitions, whose operand tests are
    // witnesses here: every value statement on this route was folded by the
    // checker, which refused each one that faults, so the `exit(86)` inside
    // is provably never taken.
    for helper in &records.helpers {
        typed_prelude.push('\n');
        typed_prelude.push_str(&crate::scalar_calls_c_emit::scalar_helper_definition(
            *helper,
            certificate.target_assumption.as_ref(),
        ));
    }
    for (cmp, ty) in &records.comparisons {
        typed_prelude.push_str(&comparison_helper(*cmp, *ty));
    }
    typed_prelude.push_str(&records.float_prelude());
    if !typed_prelude.is_empty() {
        fragments.push(Fragment::new(
            Section::Prelude,
            "buffer.scalar-types",
            typed_prelude,
        ));
    }

    Ok(BufferCEmission {
        source: c_program::assemble(&fragments, &c_program::Entry::source_route(source)),
    })
}

/// Emit one `for` block — the loop and its body, at the entry's own indent —
/// for a route that embeds this route's statements but not its lends or
/// records (card `LOOPS-ON-COMPOSED-ROUTES`: the composed string/buffer route).
///
/// The text is [`emit_events`]'s own, byte for byte, with an empty loan and
/// record environment: a lend or a record inside the body is refused by that
/// route before emission, and this function refuses one too rather than
/// emitting nothing for it. The body's bindings are scoped out of `scalars`
/// and `extents` on return, exactly as the block scopes them in C.
pub fn buffer_loop_block(
    index: usize,
    event: &BufferEvent,
    scalars: &mut BTreeMap<String, String>,
    extents: &mut BTreeMap<String, u32>,
) -> Result<String, BufferEmissionRefusal> {
    EmbeddedTyped::new(None).loop_block(index, event, scalars, extents)
}

/// The typed half of this route's emission, for a route that **embeds** this
/// route's statements across a whole entry (card `W48-QR-SCALAR-TYPES-R6`,
/// `W48-QR-TYPES-COMPOSED-STRING`: the composed string/buffer route).
///
/// [`buffer_loop_block`] emits one `for` with a fresh environment, which is
/// exactly right for an `i32` program and is why card R5 refused the typed
/// tier on the composed route: a conversion or a value statement inside a body
/// called a helper nothing defined. This keeps **one** environment for the
/// whole entry — the typed names and their C objects, the checked
/// conversions, §G02t's shared helpers and the comparison functions the
/// statements use — so a top-level statement and a loop body see the same
/// typed names, and [`EmbeddedTyped::prelude`] defines every helper once.
/// With no typed statement it is empty and emits nothing, so every program
/// before the card keeps its bytes.
pub struct EmbeddedTyped {
    records: RecordEnvironment,
}

impl EmbeddedTyped {
    #[must_use]
    pub fn new(target: Option<st::RecordedTarget>) -> Self {
        Self {
            records: RecordEnvironment {
                types: BTreeMap::new(),
                bound: BTreeMap::new(),
                field_extents: BTreeMap::new(),
                live_fields: BTreeMap::new(),
                typed: BTreeMap::new(),
                conversions: BTreeSet::new(),
                helpers: BTreeSet::new(),
                comparisons: BTreeSet::new(),
                target,
                discard_unused: true,
                renderers: BTreeSet::new(),
                floats: false,
            },
        }
    }

    /// Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: the composing entry holds a float
    /// value, so [`EmbeddedTyped::prelude`] adds the IEEE environment.
    pub fn hold_floats(&mut self, floats: bool) {
        self.records.floats = floats;
    }

    /// Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: a composing route's own line
    /// renders a float name, so the prelude defines that renderer.
    pub fn use_renderer(&mut self, hex: bool, ty: ScalarType) {
        self.records.renderers.insert((hex, ty));
    }

    /// One `for` block at the entry's indent, in this environment.
    pub fn loop_block(
        &mut self,
        index: usize,
        event: &BufferEvent,
        scalars: &mut BTreeMap<String, String>,
        extents: &mut BTreeMap<String, u32>,
    ) -> Result<String, BufferEmissionRefusal> {
        if !matches!(event, BufferEvent::Repeat { .. }) {
            return Err(BufferEmissionRefusal::UnemittableLoop { event: index });
        }
        self.events(index, event, scalars, extents)
    }

    /// One top-level conversion or value statement, in this environment.
    pub fn statement(
        &mut self,
        index: usize,
        event: &BufferEvent,
        scalars: &mut BTreeMap<String, String>,
        extents: &mut BTreeMap<String, u32>,
    ) -> Result<String, BufferEmissionRefusal> {
        if !matches!(
            event,
            BufferEvent::Convert { .. } | BufferEvent::Compute { .. }
        ) {
            return Err(BufferEmissionRefusal::UnemittableConversion { event: index });
        }
        // The composing route decides whether a later statement of its own
        // reads the binding, and discards it itself when none does.
        self.records.discard_unused = false;
        let text = self.events(index, event, scalars, extents);
        self.records.discard_unused = true;
        text
    }

    fn events(
        &mut self,
        index: usize,
        event: &BufferEvent,
        scalars: &mut BTreeMap<String, String>,
        extents: &mut BTreeMap<String, u32>,
    ) -> Result<String, BufferEmissionRefusal> {
        let lends = BTreeMap::new();
        let mut loans = BTreeMap::new();
        let mut source = String::new();
        emit_events(
            std::slice::from_ref(event),
            "    ",
            index,
            false,
            scalars,
            extents,
            &mut loans,
            &lends,
            &mut self.records,
            &mut source,
        )?;
        Ok(source)
    }

    /// The C object and type of a typed (non-`i32`) name, if it is one.
    #[must_use]
    pub fn typed(&self, name: &str) -> Option<(String, ScalarType)> {
        self.records.typed.get(name).cloned()
    }

    /// The recorded target's width assertions and every helper the entry's
    /// typed statements used, each once — the empty string when none did.
    pub fn prelude(&self) -> Result<String, BufferEmissionRefusal> {
        let target = self.records.target.as_ref();
        let mut text = String::new();
        if let Some(target) = target {
            text.push('\n');
            text.push_str(&st::c_target_assertions(target));
        }
        for (from, to) in &self.records.conversions {
            match conversion_helper(*from, *to, target) {
                Some(helper) => text.push_str(&helper),
                None => return Err(BufferEmissionRefusal::UnemittableConversion { event: 0 }),
            }
        }
        for helper in &self.records.helpers {
            text.push('\n');
            text.push_str(&crate::scalar_calls_c_emit::scalar_helper_definition(
                *helper, target,
            ));
        }
        for (cmp, ty) in &self.records.comparisons {
            text.push_str(&comparison_helper(*cmp, *ty));
        }
        text.push_str(&self.records.float_prelude());
        Ok(text)
    }
}

/// Emit one sequence of events at `indent`.
///
/// Recursive because of the bounded counted loop (card `LOOP-V1`), and the
/// recursion is the whole of what emitting a loop costs: a `for` body is
/// emitted **once**, inside a real C `for` statement, rather than once per
/// iteration.
///
/// That is a deliberate asymmetry with the *checking* route, which runs the body
/// once per iteration, and both halves are right for their own reason. The
/// checker needs each iteration's exact values, because the certificate
/// publishes them and the evidence is a byte comparison. The emitted C needs
/// none of that: it needs to be the program a C reader would have written, and
/// it needs to stay one program rather than growing linearly in a number that
/// can reach [`qcore_core::source_loop::MAX_ITERATIONS`]. Emitting the loop also
/// makes the native run a **check** of the unrolling rather than a restatement
/// of it: the C computes the index itself, so an unrolled reference observation
/// that disagreed with a rolled native one would show up as a byte difference.
/// The record half of the emitter's environment (card `AGGREGATES-V1`): the
/// module's declared types, and which type each live record binding has.
///
/// A struct rather than two more parameters, because the two are only ever used
/// together and the walk already carries the maximum number of arguments a
/// reader can follow.
struct RecordEnvironment {
    types: BTreeMap<String, RecordType>,
    bound: BTreeMap<String, String>,
    /// `(record binding, field path)` → the extent of the buffer moved into
    /// that owned field (card `AGGREGATE-OWNED-FIELD`; a path since card
    /// `W48-QR-USABLE-RECORDS`), so an access through the field emits the same
    /// bounds witness a plain access does.
    field_extents: BTreeMap<(String, Vec<String>), u32>,
    /// Record binding → the owned fields (paths) that still hold their buffer,
    /// so a whole-record release frees exactly those, in the IR's leaf order
    /// — the emitter's mirror of what the checker decided and the certificate
    /// published.
    live_fields: BTreeMap<String, BTreeSet<Vec<String>>>,
    /// Card `W48-QR-SCALAR-TYPES-R3`: every scalar binding of a type other
    /// than `i32`, with its C object — the emitter's half of the typed
    /// environment (`i32` names stay in `scalars`, as before the card).
    typed: BTreeMap<String, (String, ScalarType)>,
    /// The checked conversions the program uses, each emitted as one helper.
    conversions: BTreeSet<(ScalarType, ScalarType)>,
    /// Card `W48-QR-SCALAR-TYPES-R5`: §G02t's shared checked helpers the
    /// program's value statements call, each emitted once.
    helpers: BTreeSet<crate::scalar_calls_c_emit::Helper>,
    /// Card `W48-QR-SCALAR-TYPES-R5`: the comparisons the program's value
    /// statements make, each emitted once as a function (see
    /// [`comparison_helper`]).
    comparisons: BTreeSet<(st::Cmp, ScalarType)>,
    /// The recorded target, when the certificate depends on one.
    target: Option<st::RecordedTarget>,
    /// Whether [`emit_events`] discards a scalar its sequence never reads
    /// (card `W48-QR-SCALAR-TYPES-R6`). Off only for a composing route's single
    /// top-level statement, whose reads that route's own discard decides.
    discard_unused: bool,
    /// Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: the float renderings the
    /// program's lines use, `(hex, type)`, each renderer emitted once.
    renderers: BTreeSet<(bool, ScalarType)>,
    /// Whether the program holds a float value, so its C carries the IEEE
    /// environment ([`crate::scalar_floats::c_environment`]).
    floats: bool,
}

impl RecordEnvironment {
    /// The IEEE environment and the float renderers the program's lines use
    /// (card `W48-QR-FLOAT-BUFFER-PAYLOADS`) — the scalar route's own text,
    /// asserted when the C is compiled; empty for a program holding no float.
    fn float_prelude(&self) -> String {
        if !self.floats {
            return String::new();
        }
        let mut text = String::from("\n");
        text.push_str(&crate::scalar_floats::c_environment());
        text.push_str(&crate::scalar_floats::c_renderers(&self.renderers));
        text
    }

    /// The C object and type of a scalar name of any type.
    fn scalar(
        &self,
        scalars: &BTreeMap<String, String>,
        event: usize,
        name: &str,
    ) -> Result<(String, ScalarType), BufferEmissionRefusal> {
        if let Some(emitted) = scalars.get(name) {
            return Ok((emitted.clone(), ScalarType::I32));
        }
        self.typed
            .get(name)
            .cloned()
            .ok_or_else(|| BufferEmissionRefusal::Unbound {
                event,
                name: name.to_owned(),
            })
    }
}

impl RecordEnvironment {
    /// The C expression for the member `record.path` — `r_a.f_b.f_c` for the
    /// source path `a.b.c` (card `W48-QR-USABLE-RECORDS`; one level before).
    fn member(record: &str, path: &[String]) -> String {
        let mut text = record_binding_name(record);
        for field in path {
            text.push('.');
            text.push_str(&record_field_name(field));
        }
        text
    }

    fn field_extent(
        &self,
        event: usize,
        record: &str,
        path: &[String],
    ) -> Result<u32, BufferEmissionRefusal> {
        self.field_extents
            .get(&(record.to_owned(), path.to_vec()))
            .copied()
            .ok_or(BufferEmissionRefusal::UnplaceableAccess { event })
    }

    /// The declared type at `record.path` of a bound record.
    fn field_type(&self, record: &str, path: &[String]) -> Option<FieldType> {
        let type_name = self.bound.get(record)?;
        crate::source_record::field_type_at(
            &|name: &str| self.types.get(name),
            record,
            type_name,
            path,
        )
        .ok()
    }
}

/// The C scalar a binding event declares, if it declares one.
fn declared_scalar(event: &BufferEvent) -> Option<String> {
    match event {
        BufferEvent::Bind { binding, .. }
        | BufferEvent::Load { binding, .. }
        | BufferEvent::Extent { binding, .. }
        | BufferEvent::Convert { binding, .. }
        | BufferEvent::Compute { binding, .. }
        | BufferEvent::CopyBind { binding, .. } => Some(scalar_name(binding)),
        _ => None,
    }
}

/// Whether `name` occurs in `text` as a whole C identifier.
fn names_identifier(text: &str, name: &str) -> bool {
    let is_identifier = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(name).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + name.len()..].chars().next();
        !before.is_some_and(is_identifier) && !after.is_some_and(is_identifier)
    })
}

/// Emit one sequence of events at `indent`, then discard every scalar the
/// sequence declares and nothing after it in the sequence reads (card
/// `W48-QR-SCALAR-TYPES-R6`, `W48-QR-LOOP-BOUND-NAME-UNUSED-C`).
///
/// The emitted C is built with `-Wall -Wextra -Werror`, where an unused local
/// is an error; before the card a binding the program never read — a loop
/// bound whose loop is written against its literal count, a body binding
/// nothing in the body reads — emitted C that did not compile. The decision is
/// made on the **emitted text itself**: a declared `s_NAME` that no later text
/// of its own sequence (its C block) names as an identifier is followed by
/// `(void) s_NAME;`, exactly the string route's discard. A program whose every
/// binding is read gets no discard, so its bytes are unchanged.
#[allow(clippy::too_many_arguments)]
fn emit_events(
    events: &[BufferEvent],
    indent: &str,
    attributed: usize,
    top_level: bool,
    scalars: &mut BTreeMap<String, String>,
    extents: &mut BTreeMap<String, u32>,
    loans: &mut BTreeMap<String, (u32, u32)>,
    lends: &BTreeMap<usize, &std_buffer::LendRecord>,
    records: &mut RecordEnvironment,
    source: &mut String,
) -> Result<(), BufferEmissionRefusal> {
    let mut declared: Vec<(usize, String)> = Vec::new();
    for (position, event) in events.iter().enumerate() {
        let index = if top_level { position } else { attributed };
        let start = source.len();
        emit_event_sequence(
            std::slice::from_ref(event),
            indent,
            index,
            false,
            scalars,
            extents,
            loans,
            lends,
            records,
            source,
        )?;
        // Only a scalar this event's own text declares: a copy of a record,
        // say, binds another C object and is never discarded here.
        if let Some(name) = declared_scalar(event)
            && names_identifier(&source[start..], &name)
        {
            declared.push((source.len(), name));
        }
    }
    if records.discard_unused {
        for (offset, name) in declared.into_iter().rev() {
            if !names_identifier(&source[offset..], &name) {
                source.insert_str(offset, &format!("{indent}(void) {name};\n"));
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_event_sequence(
    events: &[BufferEvent],
    indent: &str,
    attributed: usize,
    top_level: bool,
    scalars: &mut BTreeMap<String, String>,
    extents: &mut BTreeMap<String, u32>,
    loans: &mut BTreeMap<String, (u32, u32)>,
    lends: &BTreeMap<usize, &std_buffer::LendRecord>,
    records: &mut RecordEnvironment,
    source: &mut String,
) -> Result<(), BufferEmissionRefusal> {
    for (position, event) in events.iter().enumerate() {
        let index = if top_level { position } else { attributed };
        // Card `SLICE-PERMISSIONS-WRITE-LEND`: the lend and every access
        // through a loan are emitted here, where the loan environment lives —
        // which owner, which displacement, which length. The displacement is
        // written **once**, at the lend (from the bounds the certificate's
        // `LendRecord` resolved), so every later access is written in the
        // loan's own coordinates exactly as the source spells it, and the
        // emitted bounds witness is the loan's own `[0, len)`.
        match event {
            BufferEvent::Lend { binding, name, .. } => {
                let Some(record) = lends.get(&index) else {
                    return Err(BufferEmissionRefusal::UnemittableLend { event: index });
                };
                let len = record.high - record.low;
                loans.insert(binding.clone(), (record.low, len));
                let emitted = lend_name(binding);
                let owner = buffer_name(name);
                if record.low == 0 {
                    let _ = writeln!(source, "{indent}unsigned char *const {emitted} = {owner};");
                } else {
                    let _ = writeln!(
                        source,
                        "{indent}unsigned char *const {emitted} = {owner} + {}u;",
                        record.low
                    );
                }
                continue;
            }
            BufferEvent::Store {
                name,
                index: at,
                value,
            } if loans.contains_key(name) => {
                let (_, len) = loans[name];
                let at = operand_expression(scalars, index, at)?;
                let value = operand_expression(scalars, index, value)?;
                let _ = writeln!(
                    source,
                    "{indent}qcore_bytes_store({}, (size_t) {len}u, (size_t) ({at}), {value});",
                    lend_name(name)
                );
                continue;
            }
            BufferEvent::Load {
                binding,
                name,
                index: at,
            } if loans.contains_key(name) => {
                let (_, len) = loans[name];
                let at = operand_expression(scalars, index, at)?;
                let emitted = scalar_name(binding);
                scalars.insert(binding.clone(), emitted.clone());
                let _ = writeln!(
                    source,
                    "{indent}int32_t {emitted} = qcore_bytes_load({}, (size_t) {len}u, (size_t) ({at}));",
                    lend_name(name)
                );
                continue;
            }
            BufferEvent::Extent { binding, name } if loans.contains_key(name) => {
                let (_, len) = loans[name];
                let emitted = scalar_name(binding);
                scalars.insert(binding.clone(), emitted.clone());
                let _ = writeln!(source, "{indent}int32_t {emitted} = INT32_C({len});");
                continue;
            }
            _ => {}
        }
        match event {
            // Card `AGGREGATE-OWNED-FIELD`: `drop RECORD;` of a composed
            // record frees every owned member still held, in declaration
            // order — the emitter walking the same declared list the checker
            // published its releases from. A record of scalars is never in
            // `live_fields`, so its `drop` — refused by the checker — never
            // reaches this arm.
            BufferEvent::Release { name } if records.live_fields.contains_key(name) => {
                let Some(type_name) = records.bound.get(name) else {
                    return Err(BufferEmissionRefusal::Unbound {
                        event: index,
                        name: name.clone(),
                    });
                };
                // Card `W48-QR-USABLE-RECORDS`: the owned **leaves**, at every
                // depth, in the IR's order.
                let declared = crate::source_record::owned_leaves(
                    &|name: &str| records.types.get(name),
                    type_name,
                );
                let live = records.live_fields.remove(name).unwrap_or_default();
                for leaf in declared {
                    if live.contains(&leaf) {
                        let _ = writeln!(
                            source,
                            "{indent}qcore_bytes_drop({});",
                            RecordEnvironment::member(name, &leaf)
                        );
                    }
                }
            }
            // **The composition of the two restructures** (cards `LOOP-V1`
            // and `STRING-CONCAT`). The recursion and the indent are
            // `LOOP-V1`'s and are unchanged; what one value-bearing event
            // emits is `STRING-CONCAT`'s published
            // [`buffer_event_statement`], so a byte write inside a `for` body
            // is emitted by the same function — with the same C names — that
            // the composed string/buffer route emits a top-level one with.
            // The indent is applied here rather than inside that function,
            // because a statement's text does not depend on its depth.
            // Card `OWNED-FIELD-ROUTE-REACH`: a copy of a **record** value is
            // one C struct assignment — the aggregate is a plain value, so the
            // copy is the language's own, with no member walk and no custody.
            // Handled here rather than in `buffer_event_statement` because it
            // needs the record environment, exactly as `Construct` does. The
            // copy is **by value**, so the two bindings are two C objects and
            // a later read of either names its own member; only a record whose
            // type declares no owned field reaches emission, the rest having
            // been refused at `Q054`.
            BufferEvent::CopyBind {
                binding,
                source: from,
            } if records.bound.contains_key(from) => {
                let type_name = records.bound[from].clone();
                let emitted = record_binding_name(binding);
                records.bound.insert(binding.clone(), type_name.clone());
                let _ = writeln!(
                    source,
                    "{indent}struct {} {emitted} = {};",
                    record_type_tag(&type_name),
                    record_binding_name(from)
                );
            }
            // Card `W48-QR-SCALAR-TYPES-R3`: a copy of a name of another type
            // keeps its type — the scalar copy of the arm below, at that type.
            BufferEvent::CopyBind {
                binding,
                source: from,
            } if records.typed.contains_key(from) => {
                let (from_c, ty) = records.typed[from].clone();
                let emitted = scalar_name(binding);
                records.typed.insert(binding.clone(), (emitted.clone(), ty));
                let _ = writeln!(source, "{indent}{} {emitted} = {from_c};", st::c_type(ty));
            }
            // Card `W48-QR-SCALAR-TYPES-R5`: the value statement. Arithmetic
            // and negation call §G02t's shared checked helper, a comparison is
            // plain C; the result is declared at its own type.
            BufferEvent::Compute { binding, operation } => {
                let (ty, expression) =
                    value_statement_expression(records, scalars, index, operation)?;
                let emitted = scalar_name(binding);
                if ty == ScalarType::I32 {
                    scalars.insert(binding.clone(), emitted.clone());
                } else {
                    records.typed.insert(binding.clone(), (emitted.clone(), ty));
                }
                let _ = writeln!(
                    source,
                    "{indent}{} {emitted} = {expression};",
                    st::c_type(ty)
                );
            }
            // Card `W48-QR-SCALAR-TYPES-R3`: the conversion. Total, it is a
            // plain cast; checked, it calls the helper whose test the checker
            // proved never taken.
            BufferEvent::Convert {
                binding,
                operand,
                to,
            } => {
                let (value, from) = match operand {
                    BufferOperand::Literal(value) => (format!("INT32_C({value})"), ScalarType::I32),
                    BufferOperand::Scalar(name) => records.scalar(scalars, index, name)?,
                };
                let total = st::conversion_is_total(from, *to, records.target.as_ref())
                    .map_err(|_| BufferEmissionRefusal::UnemittableConversion { event: index })?;
                let expression = if total {
                    format!("({})({value})", st::c_type(*to))
                } else {
                    records.conversions.insert((from, *to));
                    format!("{}({value})", conversion_helper_name(from, *to))
                };
                let emitted = scalar_name(binding);
                if *to == ScalarType::I32 {
                    scalars.insert(binding.clone(), emitted.clone());
                } else {
                    records
                        .typed
                        .insert(binding.clone(), (emitted.clone(), *to));
                }
                let _ = writeln!(
                    source,
                    "{indent}{} {emitted} = {expression};",
                    st::c_type(*to)
                );
            }
            BufferEvent::Bind { .. }
            | BufferEvent::CopyBind { .. }
            | BufferEvent::Acquire { .. }
            | BufferEvent::Store { .. }
            | BufferEvent::Load { .. }
            | BufferEvent::Extent { .. }
            | BufferEvent::Release { .. } => {
                if let Some(statement) = buffer_event_statement(index, event, scalars, extents)? {
                    let _ = writeln!(source, "{indent}{statement}");
                }
            }
            // Card `AGGREGATE-OWNED-FIELD`: the four field-path positions are
            // the plain-name statements with the member in place of the
            // binding and the field's extent as the bounds witness.
            BufferEvent::FieldStore {
                record,
                path,
                index: at,
                value,
            } => {
                let extent = records.field_extent(index, record, path)?;
                let at = operand_expression(scalars, index, at)?;
                let value = operand_expression(scalars, index, value)?;
                let _ = writeln!(
                    source,
                    "{indent}qcore_bytes_store({}, (size_t) {extent}u, (size_t) ({at}), {value});",
                    RecordEnvironment::member(record, path)
                );
            }
            BufferEvent::FieldLoad {
                binding,
                record,
                path,
                index: at,
            } => {
                let extent = records.field_extent(index, record, path)?;
                let at = operand_expression(scalars, index, at)?;
                let emitted = scalar_name(binding);
                scalars.insert(binding.clone(), emitted.clone());
                let _ = writeln!(
                    source,
                    "{indent}int32_t {emitted} = qcore_bytes_load({}, (size_t) {extent}u, (size_t) ({at}));",
                    RecordEnvironment::member(record, path)
                );
            }
            BufferEvent::FieldExtent {
                binding,
                record,
                path,
            } => {
                let extent = records.field_extent(index, record, path)?;
                let emitted = scalar_name(binding);
                scalars.insert(binding.clone(), emitted.clone());
                let _ = writeln!(source, "{indent}int32_t {emitted} = INT32_C({extent});");
            }
            BufferEvent::FieldRelease { record, path } => {
                records.field_extent(index, record, path)?;
                if let Some(live) = records.live_fields.get_mut(record) {
                    live.remove(path);
                }
                let _ = writeln!(
                    source,
                    "{indent}qcore_bytes_drop({});",
                    RecordEnvironment::member(record, path)
                );
            }
            // Card `W48-QR-USABLE-RECORDS`: the field write. A scalar field is
            // one member assignment of a value the checker proved a value of
            // the member's type; an owned field — the refill — takes the
            // buffer's pointer, and the struct owes it again.
            BufferEvent::FieldWrite {
                record,
                path,
                value,
            } => {
                let Some(field_type) = records.field_type(record, path) else {
                    return Err(BufferEmissionRefusal::UnplaceableAccess { event: index });
                };
                let member = RecordEnvironment::member(record, path);
                match field_type {
                    FieldType::Scalar(ty) => {
                        let value = field_expression(scalars, &records.typed, index, ty, value)?;
                        let _ = writeln!(source, "{indent}{member} = {value};");
                    }
                    FieldType::Bytes => {
                        let FieldOperand::Name(from) = value else {
                            return Err(BufferEmissionRefusal::UnplaceableAccess { event: index });
                        };
                        let Some(extent) = extents.get(from).copied() else {
                            return Err(BufferEmissionRefusal::Unbound {
                                event: index,
                                name: from.clone(),
                            });
                        };
                        records
                            .field_extents
                            .insert((record.clone(), path.clone()), extent);
                        records
                            .live_fields
                            .entry(record.clone())
                            .or_default()
                            .insert(path.clone());
                        let _ = writeln!(source, "{indent}{member} = {};", buffer_name(from));
                    }
                    FieldType::Record(_) => {
                        return Err(BufferEmissionRefusal::UnplaceableAccess { event: index });
                    }
                }
            }
            // Card `AGGREGATES-V1`. Emitted here rather than in
            // `buffer_event_statement` because both need the record
            // environment, which that function does not carry; it refuses them
            // rather than silently emitting nothing.
            BufferEvent::Construct {
                binding,
                type_name,
                fields,
            } => {
                let Some(declared) = records.types.get(type_name) else {
                    return Err(BufferEmissionRefusal::Unbound {
                        event: index,
                        name: type_name.clone(),
                    });
                };
                // Emitted in **declared** order with a designated initializer
                // per field. The checker already refused a construction that
                // does not supply every declared field exactly once, so this
                // lookup is total for an admitted program — and the `else`
                // below refuses rather than leaving a member indeterminate if
                // it ever were not.
                let mut initializers = Vec::new();
                let mut live = BTreeSet::new();
                let mut moved_in = Vec::new();
                for field in &declared.fields {
                    let Some((_, operand)) = fields.iter().find(|(name, _)| *name == field.name)
                    else {
                        return Err(BufferEmissionRefusal::Unbound {
                            event: index,
                            name: format!("{type_name}.{}", field.name),
                        });
                    };
                    let value = match &field.field_type {
                        FieldType::Scalar(ty) => {
                            field_expression(scalars, &records.typed, index, *ty, operand)?
                        }
                        // Card `W48-QR-USABLE-RECORDS`: a record member takes
                        // the named record by value. For one holding an owned
                        // buffer the pointers move with it: its live members
                        // and their extents are re-keyed under this field.
                        FieldType::Record(_) => {
                            let FieldOperand::Name(from) = operand else {
                                return Err(BufferEmissionRefusal::UnplaceableAccess {
                                    event: index,
                                });
                            };
                            if !records.bound.contains_key(from) {
                                return Err(BufferEmissionRefusal::Unbound {
                                    event: index,
                                    name: from.clone(),
                                });
                            }
                            if let Some(inner) = records.live_fields.remove(from) {
                                for leaf in inner {
                                    let mut path = vec![field.name.clone()];
                                    path.extend(leaf.iter().cloned());
                                    if let Some(extent) =
                                        records.field_extents.get(&(from.clone(), leaf)).copied()
                                    {
                                        moved_in.push(((binding.clone(), path.clone()), extent));
                                    }
                                    live.insert(path);
                                }
                            }
                            record_binding_name(from)
                        }
                        // Card `AGGREGATE-OWNED-FIELD`: the owned member takes
                        // the buffer's pointer — the move, in C, is the
                        // initializer naming the old binding — and the struct
                        // owes it from here on.
                        FieldType::Bytes => {
                            let FieldOperand::Name(from) = operand else {
                                return Err(BufferEmissionRefusal::UnplaceableAccess {
                                    event: index,
                                });
                            };
                            let Some(extent) = extents.get(from).copied() else {
                                return Err(BufferEmissionRefusal::Unbound {
                                    event: index,
                                    name: from.clone(),
                                });
                            };
                            moved_in.push(((binding.clone(), vec![field.name.clone()]), extent));
                            live.insert(vec![field.name.clone()]);
                            buffer_name(from)
                        }
                    };
                    initializers.push(format!(".{} = {value}", record_field_name(&field.name)));
                }
                let emitted = record_binding_name(binding);
                records.bound.insert(binding.clone(), type_name.clone());
                if !live.is_empty() {
                    records.live_fields.insert(binding.clone(), live);
                    records.field_extents.extend(moved_in);
                }
                let _ = writeln!(
                    source,
                    "{indent}struct {} {emitted} = {{ {} }};",
                    record_type_tag(type_name),
                    initializers.join(", ")
                );
            }
            BufferEvent::Field {
                binding,
                record,
                path,
            } => {
                // The declared type is consulted rather than trusted: a path
                // the type does not declare has no member to read, and the
                // checker already refused it. Card `W48-QR-USABLE-RECORDS`: the
                // member is the whole path, `r_a.f_b.f_c`.
                let Some(field_type) = records.field_type(record, path) else {
                    return Err(BufferEmissionRefusal::UnplaceableAccess { event: index });
                };
                let member = RecordEnvironment::member(record, path);
                match field_type {
                    FieldType::Scalar(ScalarType::I32) => {
                        let emitted = scalar_name(binding);
                        scalars.insert(binding.clone(), emitted.clone());
                        let _ = writeln!(source, "{indent}int32_t {emitted} = {member};");
                    }
                    // Card `W48-QR-SCALAR-TYPES-R3`: a field of another type
                    // binds a scalar of exactly that type.
                    FieldType::Scalar(ty) => {
                        let emitted = scalar_name(binding);
                        records.typed.insert(binding.clone(), (emitted.clone(), ty));
                        let _ =
                            writeln!(source, "{indent}{} {emitted} = {member};", st::c_type(ty));
                    }
                    // Card `AGGREGATE-OWNED-FIELD`: the partial move. The plain
                    // binding takes the member's pointer and owes it; the
                    // struct no longer does, so the whole-record release below
                    // skips this member.
                    FieldType::Bytes => {
                        let extent = records.field_extent(index, record, path)?;
                        if let Some(live) = records.live_fields.get_mut(record) {
                            live.remove(path);
                        }
                        extents.insert(binding.clone(), extent);
                        let _ = writeln!(
                            source,
                            "{indent}unsigned char *{} = {member};",
                            buffer_name(binding)
                        );
                    }
                    // Card `W48-QR-USABLE-RECORDS`: a sub-record of scalars is
                    // copied out whole — one struct assignment. One holding an
                    // owned buffer was refused by the checker.
                    FieldType::Record(inner) => {
                        let emitted = record_binding_name(binding);
                        records.bound.insert(binding.clone(), inner.clone());
                        let _ = writeln!(
                            source,
                            "{indent}struct {} {emitted} = {member};",
                            record_type_tag(&inner)
                        );
                    }
                }
            }
            BufferEvent::Repeat {
                variable,
                count,
                index: loop_index,
                body,
            } => {
                let index_type = loop_index.ty();
                // A real C `for`. The index is a C variable the emitted program
                // computes itself, so every index expression inside the body is
                // written in terms of it rather than of a number the checker
                // substituted.
                let name = scalar_name(variable);
                if let std_buffer::LoopIndex::Typed { ty, bound } = loop_index {
                    // Card `W48-QR-SCALAR-TYPES-R5`: the index at the bound's
                    // own type. The checker proved the count a value of that
                    // type, so the index's values `0 … count-1` and the final
                    // step's `count` are all values of it: `+= 1` never
                    // overflows and its conversion back to a narrow type never
                    // truncates (`QCore.ScalarLoopBounds`). The bound is the
                    // name's own C object, of the index's type, so the native
                    // run compares against the number the emitted program
                    // computed — a check of the lowering's fold, not a
                    // restatement of it.
                    let (bound_c, bound_ty) = records.scalar(scalars, index, bound)?;
                    if bound_ty != *ty {
                        return Err(BufferEmissionRefusal::Unbound {
                            event: index,
                            name: bound.clone(),
                        });
                    }
                    let _ = writeln!(
                        source,
                        "{indent}for ({} {name} = {}; {name} < {bound_c}; {name} += {}) {{",
                        st::c_type(*ty),
                        st::c_literal(*ty, 0),
                        st::c_literal(*ty, 1)
                    );
                } else {
                    let _ = writeln!(
                        source,
                        "{indent}for (int32_t {name} = INT32_C(0); {name} < INT32_C({count}); \
                         {name} += INT32_C(1)) {{"
                    );
                }
                let outer_scalars: Vec<String> = scalars.keys().cloned().collect();
                let outer_extents: Vec<String> = extents.keys().cloned().collect();
                let outer_records: Vec<String> = records.bound.keys().cloned().collect();
                let outer_typed: Vec<String> = records.typed.keys().cloned().collect();
                if index_type == ScalarType::I32 {
                    scalars.insert(variable.clone(), name);
                } else {
                    records.typed.insert(variable.clone(), (name, index_type));
                }
                let inner = format!("{indent}    ");
                emit_events(
                    body, &inner, index, false, scalars, extents, loans, lends, records, source,
                )?;
                // The body's declarations do not outlive the C block either, so
                // the emitter's environment is scoped exactly as the model's is.
                scalars.retain(|key, _| outer_scalars.contains(key));
                extents.retain(|key, _| outer_extents.contains(key));
                records.bound.retain(|key, _| outer_records.contains(key));
                records.typed.retain(|key, _| outer_typed.contains(key));
                let _ = writeln!(source, "{indent}}}");
            }
            // Handled — or refused — by the loan environment's own match
            // above, which never falls through for a lend.
            BufferEvent::Lend { .. } => {}
            // Card `W48-QR-SCALAR-TYPES-R3`: a line rendering a name of another
            // type renders it at that type — §G02t's conversion and operand
            // for the type, framed by the one line framing every route shares.
            // An `i32` operand never takes this arm, so its bytes are the ones
            // the standard-IO emitter below has always written.
            BufferEvent::Line(
                StdIoEvent::Print {
                    rendering,
                    operand: StdIoOperand::Scalar(name),
                }
                | StdIoEvent::Log {
                    rendering,
                    operand: StdIoOperand::Scalar(name),
                    ..
                },
            ) if records.typed.contains_key(name) => {
                let level = match event {
                    BufferEvent::Line(StdIoEvent::Log { level, .. }) => Some(*level),
                    _ => None,
                };
                let (emitted, ty) = records.typed[name].clone();
                let (stream, prefix) = crate::std_io::line_framing(level);
                // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: a float renders through
                // the language's own renderer, defined once in the prelude.
                if ty.is_float() {
                    records.renderers.insert((*rendering == Rendering::Hex, ty));
                }
                let Some((conversion, operand)) = st::c_rendering(
                    *rendering == Rendering::Hex,
                    ty,
                    &emitted,
                    records.target.as_ref(),
                ) else {
                    return Err(BufferEmissionRefusal::Unbound {
                        event: index,
                        name: name.clone(),
                    });
                };
                let _ = writeln!(
                    source,
                    "{indent}{}",
                    io_emit::write_line(stream, &format!("{prefix}{conversion}"), &[&operand])
                );
            }
            BufferEvent::Line(line) => {
                // The rendering itself — the conversion, the level tag, the
                // stream, the `fflush` — is decided by `source.std-io.v1`'s own
                // emitter and nowhere else, so this route renders one way with
                // it rather than in a way that happens to agree.
                let identities = BTreeMap::new();
                match std_io_c_emit::event_statement(line, scalars, &identities) {
                    Ok(Some(statement)) => {
                        let _ = writeln!(source, "{indent}{statement}");
                    }
                    Ok(None) => {}
                    Err(name) => {
                        return Err(BufferEmissionRefusal::Unbound { event: index, name });
                    }
                }
            }
            BufferEvent::Return => {}
        }
    }
    Ok(())
}

/// The C name of the comparison function for `cmp` at `ty` (card
/// `W48-QR-SCALAR-TYPES-R5`).
fn comparison_helper_name(cmp: st::Cmp, ty: ScalarType) -> String {
    let name = match cmp {
        st::Cmp::Eq => "eq",
        st::Cmp::Ne => "ne",
        st::Cmp::Lt => "lt",
        st::Cmp::Le => "le",
        st::Cmp::Gt => "gt",
        st::Cmp::Ge => "ge",
    };
    format!("qcore_buffer_{name}_{ty}")
}

/// One comparison as a function of two parameters (card
/// `W48-QR-SCALAR-TYPES-R5`). A comparison written inline would let the C
/// compiler see a comparison of a name with itself (`x == x`) and warn
/// that its result is fixed (`-Wtautological-compare`) —
/// an error under `-Werror` for a statement the source legitimately writes;
/// through parameters the compiler sees two values of the type and nothing
/// else, and the result is the same `bool`.
fn comparison_helper(cmp: st::Cmp, ty: ScalarType) -> String {
    let c = st::c_type(ty);
    format!(
        "\nstatic bool {}({c} a, {c} b)\n{{\n    return a {} b;\n}}\n",
        comparison_helper_name(cmp, ty),
        cmp.c_operator()
    )
}

/// The result type and C expression of one value statement (card
/// `W48-QR-SCALAR-TYPES-R5`). The operands' one type is a named operand's
/// (the checker proved every named operand of one type), `bool` for a `bool`
/// literal, `i32` for two integer literals; a literal is that type's C
/// constant.
fn value_statement_expression(
    records: &mut RecordEnvironment,
    scalars: &BTreeMap<String, String>,
    event: usize,
    operation: &std_buffer::ValueOperation,
) -> Result<(ScalarType, String), BufferEmissionRefusal> {
    use crate::scalar_calls_c_emit::Helper;
    use std_buffer::{ValueOperand, ValueOperation};
    let mut ty = None;
    let mut float_literal = false;
    for operand in operation.operands() {
        match operand {
            ValueOperand::Name(name) => {
                let (_, found) = records.scalar(scalars, event, name)?;
                ty.get_or_insert(found);
            }
            ValueOperand::Bool(_) => {
                ty.get_or_insert(ScalarType::Bool);
            }
            ValueOperand::Integer(_) => {}
            ValueOperand::Float(_) => float_literal = true,
        }
    }
    // A literal takes the other operand's type; two literals are `i32`, or
    // `f64` when they are float literals (card `W48-QR-FLOAT-BUFFER-PAYLOADS`,
    // the shared decision's rule — a mixed pair was refused by the checker).
    let ty = ty.unwrap_or(if float_literal {
        ScalarType::F64
    } else {
        ScalarType::I32
    });
    let unemittable = BufferEmissionRefusal::UnemittableValueStatement { event };
    let c = |operand: &ValueOperand, at: ScalarType| -> Result<String, BufferEmissionRefusal> {
        Ok(match operand {
            ValueOperand::Name(name) => records.scalar(scalars, event, name)?.0,
            ValueOperand::Integer(value) => st::c_literal(at, *value),
            ValueOperand::Bool(value) => st::c_literal(ScalarType::Bool, i128::from(*value)),
            ValueOperand::Float(literal) => {
                st::c_literal(at, literal.at(at).map_err(|_| unemittable.clone())?)
            }
        })
    };
    // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: the conversion of a float literal
    // (and the literal binding, `FLOAT as f64`). To a float type the literal is
    // read once at that type and is its constant; to an integer type it is the
    // `f64` literal passed through the checked conversion helper, whose test
    // the checker proved never taken.
    if let ValueOperation::Convert(operand, to) = operation {
        let from = match operand {
            ValueOperand::Float(_) if to.is_float() => *to,
            ValueOperand::Float(_) => ScalarType::F64,
            _ => ty,
        };
        let value = c(operand, from)?;
        if from == *to {
            return Ok((*to, value));
        }
        let total = st::conversion_is_total(from, *to, records.target.as_ref())
            .map_err(|_| unemittable.clone())?;
        return Ok((
            *to,
            if total {
                format!("({})({value})", st::c_type(*to))
            } else {
                records.conversions.insert((from, *to));
                format!("{}({value})", conversion_helper_name(from, *to))
            },
        ));
    }
    let c = |operand: &ValueOperand| c(operand, ty);
    Ok(match operation {
        ValueOperation::Binary(op, a, b) => {
            let (a, b) = (c(a)?, c(b)?);
            let helper = Helper::Op(*op, ty);
            records.helpers.insert(helper);
            (ty, format!("{}({a}, {b})", helper.name()))
        }
        ValueOperation::Negate(a) => {
            let a = c(a)?;
            let helper = Helper::Neg(ty);
            records.helpers.insert(helper);
            (ty, format!("{}({a})", helper.name()))
        }
        ValueOperation::Compare(cmp, a, b) => {
            let (a, b) = (c(a)?, c(b)?);
            records.comparisons.insert((*cmp, ty));
            (
                ScalarType::Bool,
                format!("{}({a}, {b})", comparison_helper_name(*cmp, ty)),
            )
        }
        ValueOperation::Convert(..) => unreachable!("decided above"),
    })
}

/// The file-scope text `source.buffer.v1` contributes to a program it is part
/// of (card `C-EMIT-MAIN-UNIFY`).
///
/// The buffer prelude is program-dependent — it declares only the helpers this
/// program's events reach — so it is built here rather than being a constant.
#[must_use]
pub fn preamble(program: &BufferProgram) -> Vec<Fragment> {
    let mut fragments = vec![Fragment::new(
        Section::Includes,
        "buffer.includes",
        INCLUDES,
    )];
    // Card `W48-QR-SCALAR-TYPES-R3`: `bool` and `ptrdiff_t`/`size_t` need
    // their headers, under the scalar route's own fragment key so a merged
    // program includes them once. Absent for every program before the card.
    if uses_typed_scalars(program) {
        fragments.push(Fragment::new(
            Section::Includes,
            "scalar-types.includes",
            "#include <stdbool.h>\n#include <stddef.h>\n",
        ));
    }
    // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: `<float.h>` for the environment's
    // assertions and `<string.h>` for the renderers, under the scalar route's
    // own fragment key so a merged program includes them once.
    if std_buffer::uses_floats(program) {
        fragments.push(Fragment::new(
            Section::Includes,
            "scalar-floats.includes",
            crate::scalar_floats::C_INCLUDES,
        ));
    }
    fragments.extend([
        io_emit::io_prelude_fragment(),
        // Card `AGGREGATES-V1`, and **before** the buffer prelude: a record
        // type is a type definition and nothing in the buffer prelude reads it,
        // so the order is readability rather than dependency. A module
        // declaring no record contributes the empty string, and an empty
        // fragment is dropped — which is what keeps every program written
        // before this card byte-identical.
        Fragment::new(
            Section::Prelude,
            "record.types",
            record_types_text(&program.records),
        ),
        Fragment::new(Section::Prelude, "buffer.prelude", buffer_prelude(program)),
    ]);
    fragments
}

/// The single-emission-route check, applied to this backend: an emitted program
/// must contain no standard-output call outside [`IO_PRELUDE`]'s own
/// definitions (card `TRACE-STDIO`).
#[must_use]
pub fn direct_stdio_calls(source: &str) -> Vec<(usize, &'static str)> {
    io_emit::direct_stdio_calls(source)
}
