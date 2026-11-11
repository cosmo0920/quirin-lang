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

//! Reference execution and C11 emission for `source.string.v1`
//! (card `STRING-SLICE`).
//!
//! # Self-contained by construction, and allocation-free by construction
//!
//! The emitted program links nothing and **allocates nothing**. There is no
//! `malloc`, no `calloc`, no `strdup` and no `memcpy` anywhere in what this
//! module can write, because there is no event of the model that could ask for
//! one: a literal is a file-scope `static const` array, a view is a two-field
//! aggregate held by value, and a slice is pointer addition and length
//! subtraction. That is amendment §4's "no allocation and no copy" as a property
//! of the emitter rather than a property of a particular program, and
//! [`allocator_calls`] is the tripwire a test points at it.
//!
//! # No `strlen`, anywhere
//!
//! Amendment §3.1 requires the length to be available without scanning and §3.7
//! forbids recovering it with `strlen`. The emitted `len` reads the aggregate's
//! own `len` field. A trailing zero **is** emitted after each literal's payload,
//! for toolchain convenience and so a debugger shows something sensible, but it
//! is outside `len` and outside the invariant: a payload containing an embedded
//! NUL emits that NUL inside `len` and the terminator after it, and the emitted
//! program's behaviour does not change.
//!
//! # The trap discipline, stated rather than implied
//!
//! [`STRING_PRELUDE`]'s slice helper aborts on a bound pair the checker should
//! have refused. That is a **witness, not a check**: every bound in an admitted
//! program is a check-time constant this route's checker already proved
//! satisfies `0 <= low <= high <= len`, so the comparison is provably never
//! taken, and if it ever fired that would be a defect in the checker. There is
//! no allocation that could fail, so — unlike `source.buffer.v1` — this route's
//! claim is not weakened to partial correctness by a failing allocator arm.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::c_program::{self, Fragment, Section};
use crate::io_emit;
use crate::std_buffer::{BufferEvent, BufferOperand, FieldOperand};
use crate::std_buffer_c_emit;
use crate::std_io::{StdIoEvent, line_framing};
use crate::std_io_c_emit;
use crate::std_string::{
    StringCertificate, StringEvent, StringOperand, StringProgram, StringStorage, TextOperand,
    text_slice_name,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StringEmissionRefusal {
    /// A name the certificate's program does not bind. The checker already
    /// refuses this; the backend refuses it again rather than emitting an
    /// expression it cannot justify.
    Unbound { event: usize, name: String },
    /// A view the checker admitted but the backend cannot place.
    UnplaceableView { event: usize },
    /// A certificate whose lines disagree with the program's.
    CertificateDisagrees,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringCEmission {
    pub source: String,
}

/// The header of this route's C fragment.
pub const STRING_PRELUDE_HEADER: &str = r#"
/* std.string / source.string.v1 (card STRING-SLICE): the whole mechanism of an
   immutable UTF-8 string view, defined here rather than linked.

   A view is `{ ptr, len }` and nothing else. There is deliberately no `cap`:
   an immutable string has no append operation, so capacity would grow the
   value and its ABI size without enabling anything. There is deliberately no
   allocation and no copy: a literal is static storage and a slice is pointer
   addition plus length subtraction, so `len` and slicing are O(1) in the
   number of bytes.

   `len` is a field, never a scan. `strlen` appears nowhere in this program and
   could not be correct if it did: a payload may contain an embedded NUL, which
   is inside `len` and is an ordinary byte of the text. Each literal array does
   carry a trailing zero after its payload, for toolchain convenience only; it
   is outside `len` and outside the language invariant.

   The bounds comparison in the slice helper is a WITNESS, not a check. Every
   bound in an admitted program is a check-time constant the checker already
   proved satisfies 0 <= low <= high <= len, so the comparison is provably never
   taken; if it ever fired, that would be a defect in the checker. */
"#;

/// The view type: amendment §6's aggregate, verbatim.
pub const STRING_PRELUDE_VIEW: &str = r#"typedef struct {
    const unsigned char *ptr;
    size_t len;
} qcore_string_view;
"#;

/// The slice helper: §4's arithmetic, verbatim.
///
/// Emitted **only for a program that slices at run time** (card
/// `STRING-CONCAT`). A `concat` piece written as a slice is composed at check
/// time and calls nothing, so a program whose only slices are pieces would
/// otherwise carry a helper it never uses — and the emitted C is built with
/// `-Wall -Wextra -Werror`, where an unused `static` function is an error. This
/// is `source.buffer.v1`'s own selective-prelude constraint, which that route
/// recorded in its prelude comment for exactly this reason.
pub const STRING_PRELUDE_SLICE: &str = r#"
static qcore_string_view qcore_string_slice(qcore_string_view s, size_t low, size_t high)
{
    qcore_string_view derived;
    if (!(low <= high && high <= s.len)) {
        abort();
    }
    /* { ptr = p + low, len = high - low }. `s.ptr + low` is in range for every
       low <= s.len, one-past-the-end included, and is never dereferenced when
       the derived length is zero. */
    derived.ptr = s.ptr + low;
    derived.len = high - low;
    return derived;
}
"#;

const INCLUDES: &str = "\
#include <inttypes.h>\n\
#include <stddef.h>\n\
#include <stdint.h>\n\
#include <stdio.h>\n\
#include <stdlib.h>\n";

/// The C name of a view binding.
fn view_name(name: &str) -> String {
    format!("v_{name}")
}

/// The C name of a scalar binding.
fn scalar_name(name: &str) -> String {
    format!("s_{name}")
}

/// The C name of a literal's static byte array.
fn literal_name(index: usize) -> String {
    format!("qcore_string_literal_{index}")
}

/// Mark every name an embedded `source.buffer.v1` event reads, **at every
/// depth**.
///
/// Recursive for the `LOOP-V1` × `STRING-CONCAT` composition. A `for` is
/// refused on this route, so this never meets one in practice; it recurses
/// anyway because the alternative is the fail-open one. This map decides which
/// bindings get an explicit `(void)` discard, so a name a loop body reads and
/// this scan missed would be discarded and then used, which is a C compile
/// error rather than a refusal.
fn buffer_consumers(event: &BufferEvent, consumed: &mut BTreeMap<String, bool>) {
    match event {
        BufferEvent::Store { name, index, value } => {
            consumed.insert(name.clone(), true);
            for operand in [index, value] {
                if let BufferOperand::Scalar(scalar) = operand {
                    consumed.insert(scalar.clone(), true);
                }
            }
        }
        BufferEvent::Load { name, index, .. } => {
            consumed.insert(name.clone(), true);
            if let BufferOperand::Scalar(scalar) = index {
                consumed.insert(scalar.clone(), true);
            }
        }
        BufferEvent::Extent { name, .. } | BufferEvent::Release { name } => {
            consumed.insert(name.clone(), true);
        }
        // Card `SLICE-PERMISSIONS-WRITE-LEND`: unreachable here — the composed
        // route refuses a lend before emission — and total anyway, marking
        // exactly what the statement reads.
        BufferEvent::Lend {
            name, low, high, ..
        } => {
            consumed.insert(name.clone(), true);
            for operand in [low, high] {
                if let BufferOperand::Scalar(scalar) = operand {
                    consumed.insert(scalar.clone(), true);
                }
            }
        }
        BufferEvent::Repeat { body, .. } => {
            for event in body {
                buffer_consumers(event, consumed);
            }
        }
        BufferEvent::Bind { .. } | BufferEvent::Acquire { .. } => {}
        // Card `AGGREGATES-V1`: a record construction reads its field values
        // and a field read reads the record it names. Both are marked
        // **consumed** here even though this route refuses a record statement
        // before emission ever runs, for the same reason the `Repeat` arm above
        // recurses: the alternative is the fail-open one, and a name this scan
        // missed would be `(void)`-discarded and then used, which is a C
        // compile error rather than a refusal.
        BufferEvent::Construct { fields, .. } => {
            for (_, operand) in fields {
                if let FieldOperand::Name(scalar) = operand {
                    consumed.insert(scalar.clone(), true);
                }
            }
        }
        // Card `W48-QR-SCALAR-TYPES-R3`: a conversion reads its operand —
        // unreachable here (this route lowers none) and total anyway.
        BufferEvent::Convert {
            operand: BufferOperand::Scalar(scalar),
            ..
        } => {
            consumed.insert(scalar.clone(), true);
        }
        BufferEvent::Convert {
            operand: BufferOperand::Literal(_),
            ..
        } => {}
        // Card `W48-QR-SCALAR-TYPES-R5`: a value statement reads its operands
        // — refused by this route before emission, and total anyway.
        BufferEvent::Compute { operation, .. } => {
            for name in operation.names() {
                consumed.insert(name.to_owned(), true);
            }
        }
        BufferEvent::Field { record, .. } => {
            consumed.insert(record.clone(), true);
        }
        // Card `AGGREGATE-OWNED-FIELD`: the field-path statements read the
        // record they name and their operands — marked consumed for the
        // reason the arms above are, though this route refuses them before
        // emission.
        BufferEvent::FieldStore {
            record,
            index,
            value,
            ..
        } => {
            consumed.insert(record.clone(), true);
            for operand in [index, value] {
                if let BufferOperand::Scalar(scalar) = operand {
                    consumed.insert(scalar.clone(), true);
                }
            }
        }
        BufferEvent::FieldLoad { record, index, .. } => {
            consumed.insert(record.clone(), true);
            if let BufferOperand::Scalar(scalar) = index {
                consumed.insert(scalar.clone(), true);
            }
        }
        BufferEvent::FieldExtent { record, .. } | BufferEvent::FieldRelease { record, .. } => {
            consumed.insert(record.clone(), true);
        }
        // Card `W48-QR-USABLE-RECORDS`: a field write reads its record and its
        // value, marked for the record arms' reason.
        BufferEvent::FieldWrite { record, value, .. } => {
            consumed.insert(record.clone(), true);
            if let crate::std_buffer::FieldOperand::Name(name) = value {
                consumed.insert(name.clone(), true);
            }
        }
        // Card `OWNED-FIELD-ROUTE-REACH`: a copy bind **reads its source**, so
        // the source is consumed — marked here for the reason the record arms
        // above are, though this route lowers no copy bind at all
        // (`buffer_use` does not claim the shape).
        BufferEvent::CopyBind { source, .. } => {
            consumed.insert(source.clone(), true);
        }
        BufferEvent::Line(_) | BufferEvent::Return => {}
    }
}

/// The C name of a **composed** payload's static byte array (card
/// `STRING-CONCAT`).
///
/// Deliberately a second pool rather than more entries in the first: a literal
/// is a payload the source spelled and a composed payload is one the compiler
/// derived, and a reader of the emitted C should be able to tell which is
/// which without consulting the certificate.
fn composed_name(index: usize) -> String {
    format!("qcore_string_composed_{index}")
}

/// The reference observation, as `"<stream>: <line>"` records in program order.
///
/// The lines are the **certificate's** lines: the checker already computed every
/// rendered value, so re-deriving them here would be a second implementation of
/// the same arithmetic rather than a check of the first.
pub fn interpret_string_program(
    program: &StringProgram,
    certificate: &StringCertificate,
) -> Result<Vec<String>, StringEmissionRefusal> {
    // A line inside an embedded `for` body is counted once per iteration, by
    // `source.buffer.v1`'s own count (card `LOOPS-ON-COMPOSED-ROUTES`).
    let produced: usize = program
        .events
        .iter()
        .map(|event| match event {
            StringEvent::Line(StdIoEvent::Print { .. } | StdIoEvent::Log { .. })
            | StringEvent::Text { .. } => 1,
            StringEvent::Buffer(buffer_event) => {
                std_buffer_c_emit::produced_lines(std::slice::from_ref(buffer_event))
            }
            _ => 0,
        })
        .sum();
    if produced != certificate.lines.len() {
        return Err(StringEmissionRefusal::CertificateDisagrees);
    }
    Ok(certificate
        .lines
        .iter()
        .map(|line| format!("{}: {}", line.stream.label(), line.text))
        .collect())
}

/// The C expression for one slice bound.
fn operand_expression(
    scalars: &BTreeMap<String, String>,
    event: usize,
    operand: &StringOperand,
) -> Result<String, StringEmissionRefusal> {
    match operand {
        StringOperand::Literal(value) => Ok(format!("INT32_C({value})")),
        StringOperand::Scalar(name) => {
            scalars
                .get(name)
                .cloned()
                .ok_or_else(|| StringEmissionRefusal::Unbound {
                    event,
                    name: name.clone(),
                })
        }
    }
}

/// One payload's static byte array, written as explicit hexadecimal bytes.
///
/// Byte-wise and not as a C string literal, deliberately: a payload may contain
/// an embedded NUL, and a C string literal would make that byte look like the
/// end of the data to every reader and tool. The trailing `0x00` after the
/// payload is the toolchain terminator and is outside the declared length.
fn payload_definition(name: &str, payload: &[u8]) -> String {
    let mut bytes: Vec<String> = payload.iter().map(|byte| format!("0x{byte:02x}")).collect();
    bytes.push("0x00".to_owned());
    format!(
        "static const unsigned char {}[{}] = {{ {} }};\n",
        name,
        bytes.len(),
        bytes.join(", ")
    )
}

/// Emit C11 for a checked program. The emitted lines must reproduce the
/// reference observation exactly, per stream.
pub fn emit_string_c11(
    program: &StringProgram,
    certificate: &StringCertificate,
) -> Result<StringCEmission, StringEmissionRefusal> {
    // Refuse before emitting anything if the certificate is not this program's.
    let _ = interpret_string_program(program, certificate)?;

    let mut fragments = preamble(program, certificate);
    let mut source = String::new();
    // Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-TYPES-COMPOSED-STRING`): one
    // typed environment for the whole entry — `source.buffer.v1`'s own — so a
    // top-level conversion or value statement and every loop body see the
    // same typed names, and each helper they use is defined once below. Empty,
    // and so invisible, for every program before the card.
    let mut embedded = std_buffer_c_emit::EmbeddedTyped::new(certificate.target_assumption.clone());
    // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: an entry holding a float carries
    // the IEEE environment and the renderers its lines use.
    let floats = crate::std_string::uses_floats(program);
    embedded.hold_floats(floats);

    // Which bindings a later event actually consumes. A binding nothing
    // consumes is still emitted — the program said to bind it — and then
    // discarded with an explicit `(void)`, because the emitted C is built with
    // -Wall -Wextra -Werror (and /W4 /WX on MSVC), where an unused local is an
    // error. Discarding it is honest: the value exists and nothing reads it.
    let mut consumed: BTreeMap<String, bool> = BTreeMap::new();
    for event in &program.events {
        match event {
            StringEvent::Slice {
                name, low, high, ..
            } => {
                consumed.insert(name.clone(), true);
                for operand in [low, high] {
                    if let StringOperand::Scalar(scalar) = operand {
                        consumed.insert(scalar.clone(), true);
                    }
                }
            }
            StringEvent::Len { name, .. } => {
                consumed.insert(name.clone(), true);
            }
            StringEvent::Line(
                StdIoEvent::Print { operand, .. } | StdIoEvent::Log { operand, .. },
            ) => {
                if let crate::std_io::StdIoOperand::Scalar(scalar) = operand {
                    consumed.insert(scalar.clone(), true);
                }
            }
            // A rendered view is a consumed view, and a rendered inline slice
            // consumes both its source view and any name bounding it. Without
            // this the emitter would discard a binding the program reads and
            // the C compiler would then see a use of a `(void)`-discarded value.
            StringEvent::Text { operand, .. } => match operand {
                TextOperand::Literal(_) => {}
                TextOperand::Name(name) => {
                    consumed.insert(name.clone(), true);
                }
                TextOperand::Slice { name, low, high } => {
                    consumed.insert(name.clone(), true);
                    for operand in [low, high] {
                        if let StringOperand::Scalar(scalar) = operand {
                            consumed.insert(scalar.clone(), true);
                        }
                    }
                }
            },
            // Card `STRING-CONCAT`. A `concat` piece is **deliberately not a
            // consumer**: the composition happens at check time, so the emitted
            // program never reads the piece's view or scalar — it reads the
            // composed static array. A binding whose only reader is a
            // composition is therefore `(void)`-discarded, which is the honest
            // record of what the emitted program does with it. An embedded
            // buffer statement, by contrast, is emitted, so it consumes the
            // buffer it names and the operands it indexes with.
            StringEvent::Concat { .. } => {}
            StringEvent::Borrow { buffer, .. } => {
                consumed.insert(buffer.clone(), true);
            }
            StringEvent::Buffer(buffer_event) => {
                buffer_consumers(buffer_event, &mut consumed);
            }
            StringEvent::Bind { .. }
            | StringEvent::Literal { .. }
            | StringEvent::Line(_)
            | StringEvent::Return => {}
        }
    }

    let mut scalars: BTreeMap<String, String> = BTreeMap::new();
    let mut views: BTreeMap<String, String> = BTreeMap::new();
    // The declared extents of the buffers a composed module holds, carried for
    // `source.buffer.v1`'s own emitter (card `STRING-CONCAT`).
    let mut extents: BTreeMap<String, u32> = BTreeMap::new();
    let discard = |source: &mut String, binding: &str, emitted: &str| {
        if !consumed.contains_key(binding) {
            let _ = writeln!(source, "    (void) {emitted};");
        }
    };

    for (index, event) in program.events.iter().enumerate() {
        match event {
            StringEvent::Bind { binding, value } => {
                let emitted = scalar_name(binding);
                let _ = writeln!(source, "    int32_t {emitted} = INT32_C({value});");
                scalars.insert(binding.clone(), emitted.clone());
                discard(&mut source, binding, &emitted);
            }
            // Card `STRING-CONCAT`: the composed payload is a file-scope
            // `static const` array exactly as a literal's is, so the view that
            // names it is built the same way and **no allocation and no
            // run-time copy exists anywhere in this route's output**. The bytes
            // were computed by the checker; the emitted program only addresses
            // them.
            StringEvent::Concat { binding, .. } => {
                let Some(record) = certificate
                    .concats
                    .iter()
                    .find(|record| record.event == index)
                else {
                    return Err(StringEmissionRefusal::UnplaceableView { event: index });
                };
                let StringStorage::Composed(slot) = &record.result.storage else {
                    return Err(StringEmissionRefusal::UnplaceableView { event: index });
                };
                let emitted = view_name(binding);
                let _ = writeln!(
                    source,
                    "    qcore_string_view {emitted} = {{ {}, (size_t) {}u }};",
                    composed_name(*slot),
                    record.result.len
                );
                views.insert(binding.clone(), emitted.clone());
                discard(&mut source, binding, &emitted);
            }
            // Card `STRING-CONCAT`: a borrow is a view of the buffer's own
            // storage — the pointer `qcore_bytes_new` returned and the extent
            // the checker proved. No copy, no allocation, no validation at run
            // time: the UTF-8 decision was made at check time out of the
            // buffer's write history.
            StringEvent::Borrow {
                binding, buffer, ..
            } => {
                let Some(record) = certificate
                    .borrows
                    .iter()
                    .find(|record| record.event == index)
                else {
                    return Err(StringEmissionRefusal::UnplaceableView { event: index });
                };
                let emitted = view_name(binding);
                // Card `CT07-SLICE-PERMISSIONS`: a sub-extent loan is the same
                // pointer displaced by the offset the checker decided, and the
                // displacement is written **only** when there is one — so a
                // whole-extent borrow emits the byte-identical initialiser it
                // emitted before ranges existed, and no program written before
                // this card changes by a character.
                let base = std_buffer_c_emit::buffer_name(buffer);
                let pointer = if record.view.offset == 0 {
                    base
                } else {
                    format!("{base} + {}u", record.view.offset)
                };
                let _ = writeln!(
                    source,
                    "    qcore_string_view {emitted} = {{ {pointer}, (size_t) {}u }};",
                    record.view.len
                );
                views.insert(binding.clone(), emitted.clone());
                discard(&mut source, binding, &emitted);
            }
            // Card `STRING-CONCAT`: an embedded buffer statement is emitted by
            // `source.buffer.v1`'s **own** emitter, with that route's own C
            // names, so a composed module's `bytes(8)` is the same call the
            // uncomposed route writes.
            // Card `LOOPS-ON-COMPOSED-ROUTES`: an embedded `for` is emitted by
            // `source.buffer.v1`'s **own** block emitter — a real C `for`
            // whose body is written once — so the loop a composed module
            // writes is the loop the uncomposed route writes, character for
            // character.
            StringEvent::Buffer(buffer_event @ BufferEvent::Repeat { .. }) => {
                match embedded.loop_block(index, buffer_event, &mut scalars, &mut extents) {
                    Ok(block) => source.push_str(&block),
                    Err(_) => {
                        return Err(StringEmissionRefusal::UnplaceableView { event: index });
                    }
                }
            }
            // Card `W48-QR-SCALAR-TYPES-R6`: a top-level conversion or value
            // statement, written by `source.buffer.v1`'s own statement emitter
            // in the entry's typed environment, and discarded when nothing
            // reads it, as every other binding of this route is.
            StringEvent::Buffer(
                buffer_event @ (BufferEvent::Convert { binding, .. }
                | BufferEvent::Compute { binding, .. }),
            ) => match embedded.statement(index, buffer_event, &mut scalars, &mut extents) {
                Ok(statement) => {
                    source.push_str(&statement);
                    discard(&mut source, binding, &scalar_name(binding));
                }
                Err(_) => {
                    return Err(StringEmissionRefusal::UnplaceableView { event: index });
                }
            },
            StringEvent::Buffer(buffer_event) => {
                match std_buffer_c_emit::buffer_event_statement(
                    index,
                    buffer_event,
                    &mut scalars,
                    &mut extents,
                ) {
                    Ok(Some(statement)) => {
                        let _ = writeln!(source, "    {statement}");
                        if let BufferEvent::Bind { binding, .. }
                        | BufferEvent::Load { binding, .. }
                        | BufferEvent::Extent { binding, .. } = buffer_event
                        {
                            let emitted = scalar_name(binding);
                            discard(&mut source, binding, &emitted);
                        }
                    }
                    Ok(Option::None) => {}
                    Err(_) => {
                        return Err(StringEmissionRefusal::UnplaceableView { event: index });
                    }
                }
            }
            StringEvent::Literal { binding, literal } => {
                let Some(payload) = program.literals.get(*literal) else {
                    return Err(StringEmissionRefusal::UnplaceableView { event: index });
                };
                let emitted = view_name(binding);
                // The length is the payload's own, written as a constant: it is
                // the number the checker determined, and the array's declared
                // size is one larger because of the terminator that is not part
                // of it. No `sizeof` and no scan appears here for exactly that
                // reason — either would silently include or exclude the wrong
                // byte.
                let _ = writeln!(
                    source,
                    "    qcore_string_view {emitted} = {{ {}, (size_t) {}u }};",
                    literal_name(*literal),
                    payload.len()
                );
                views.insert(binding.clone(), emitted.clone());
                discard(&mut source, binding, &emitted);
            }
            StringEvent::Slice {
                binding,
                name,
                low,
                high,
            } => {
                let Some(from) = views.get(name).cloned() else {
                    return Err(StringEmissionRefusal::UnplaceableView { event: index });
                };
                let low = operand_expression(&scalars, index, low)?;
                let high = operand_expression(&scalars, index, high)?;
                let emitted = view_name(binding);
                let _ = writeln!(
                    source,
                    "    qcore_string_view {emitted} = qcore_string_slice({from}, (size_t) \
                     ({low}), (size_t) ({high}));"
                );
                views.insert(binding.clone(), emitted.clone());
                discard(&mut source, binding, &emitted);
            }
            StringEvent::Len { binding, name } => {
                let Some(from) = views.get(name).cloned() else {
                    return Err(StringEmissionRefusal::UnplaceableView { event: index });
                };
                let emitted = scalar_name(binding);
                // **The whole `len` implementation.** A field read, O(1), no
                // scan and no `strlen`.
                let _ = writeln!(source, "    int32_t {emitted} = (int32_t) {from}.len;");
                scalars.insert(binding.clone(), emitted.clone());
                discard(&mut source, binding, &emitted);
            }
            // Card `W48-QR-SCALAR-TYPES-R6`: a name of the typed environment
            // renders at its own type — §G02t's conversion over its C object,
            // framed like every line (the composed extern and task routes'
            // precedent).
            StringEvent::Line(
                line @ (StdIoEvent::Print {
                    rendering,
                    operand: crate::std_io::StdIoOperand::Scalar(name),
                }
                | StdIoEvent::Log {
                    rendering,
                    operand: crate::std_io::StdIoOperand::Scalar(name),
                    ..
                }),
            ) if embedded.typed(name).is_some() => {
                let (object, ty) = embedded.typed(name).expect("a typed name");
                let level = match line {
                    StdIoEvent::Log { level, .. } => Some(*level),
                    _ => None,
                };
                let (stream, prefix) = line_framing(level);
                if ty.is_float() {
                    embedded.use_renderer(*rendering == crate::std_io::Rendering::Hex, ty);
                }
                let Some((conversion, operand)) = crate::scalar_types::c_rendering(
                    *rendering == crate::std_io::Rendering::Hex,
                    ty,
                    &object,
                    certificate.target_assumption.as_ref(),
                ) else {
                    return Err(StringEmissionRefusal::Unbound {
                        event: index,
                        name: name.clone(),
                    });
                };
                let _ = writeln!(
                    source,
                    "    {}",
                    io_emit::write_line(stream, &format!("{prefix}{conversion}"), &[&operand])
                );
            }
            StringEvent::Line(line) => {
                // The rendering itself — the conversion, the level tag, the
                // stream, the `fflush` — is decided by `source.std-io.v1`'s own
                // emitter and nowhere else.
                let identities = BTreeMap::new();
                match std_io_c_emit::event_statement(line, &scalars, &identities) {
                    Ok(Some(statement)) => {
                        let _ = writeln!(source, "    {statement}");
                    }
                    Ok(Option::None) => {}
                    Err(name) => {
                        return Err(StringEmissionRefusal::Unbound { event: index, name });
                    }
                }
            }
            // Card `STRING-SLICE-IO`. The stream, the `[TAG] ` framing and the
            // single trailing newline are `source.std-io.v1`'s `line_framing`
            // and nowhere else; the body is the view's own bytes, written by a
            // length-bounded `fwrite` so an embedded NUL is an ordinary byte of
            // the text rather than the end of it.
            StringEvent::Text { level, operand } => {
                let (stream, prefix) = line_framing(*level);
                let (pointer, length) = match operand {
                    TextOperand::Literal(literal) => {
                        let Some(payload) = program.literals.get(*literal) else {
                            return Err(StringEmissionRefusal::UnplaceableView { event: index });
                        };
                        (
                            literal_name(*literal),
                            format!("(size_t) {}u", payload.len()),
                        )
                    }
                    TextOperand::Name(name) => {
                        let Some(view) = views.get(name).cloned() else {
                            return Err(StringEmissionRefusal::UnplaceableView { event: index });
                        };
                        (format!("{view}.ptr"), format!("{view}.len"))
                    }
                    TextOperand::Slice { name, low, high } => {
                        let Some(from) = views.get(name).cloned() else {
                            return Err(StringEmissionRefusal::UnplaceableView { event: index });
                        };
                        let low = operand_expression(&scalars, index, low)?;
                        let high = operand_expression(&scalars, index, high)?;
                        // The same helper a bound slice calls, so the emitted
                        // arithmetic of an inline slice is not a second
                        // implementation of §4. The temporary is named for the
                        // event, which is also the name the certificate
                        // publishes the derived view under.
                        let emitted = text_slice_name(index);
                        let _ = writeln!(
                            source,
                            "    qcore_string_view {emitted} = qcore_string_slice({from}, (size_t) \
                             ({low}), (size_t) ({high}));"
                        );
                        (format!("{emitted}.ptr"), format!("{emitted}.len"))
                    }
                };
                let _ = writeln!(
                    source,
                    "    {}",
                    io_emit::write_text_bytes(stream, &prefix, &pointer, &length)
                );
            }
            StringEvent::Return => {}
        }
    }

    // Card `W48-QR-SCALAR-TYPES-R6`: the typed environment's headers and its
    // helpers, each once — no fragment at all for a program with none.
    let typed_prelude = embedded
        .prelude()
        .map_err(|_| StringEmissionRefusal::UnplaceableView { event: 0 })?;
    if uses_typed_scalars(program) {
        fragments.push(Fragment::new(
            Section::Includes,
            "scalar-types.includes",
            "#include <stdbool.h>\n#include <stddef.h>\n",
        ));
    }
    if floats {
        fragments.push(Fragment::new(
            Section::Includes,
            "scalar-floats.includes",
            crate::scalar_floats::C_INCLUDES,
        ));
    }
    if !typed_prelude.is_empty() {
        fragments.push(Fragment::new(
            Section::Prelude,
            "string.scalar-types",
            typed_prelude,
        ));
    }
    let source = c_program::assemble(&fragments, &c_program::Entry::source_route(source));
    Ok(StringCEmission { source })
}

/// Whether the composed entry uses the fixed-width tier at all (card
/// `W48-QR-SCALAR-TYPES-R6`): `source.buffer.v1`'s own test over its embedded
/// statements.
fn uses_typed_scalars(program: &StringProgram) -> bool {
    std_buffer_c_emit::uses_typed_scalars(&embedded_buffer(program))
}

/// The composed entry's embedded `source.buffer.v1` statements as one
/// program, for that route's own tests over them (cards
/// `W48-QR-SCALAR-TYPES-R6` and `W48-QR-FLOAT-BUFFER-PAYLOADS`).
fn embedded_buffer(program: &StringProgram) -> crate::std_buffer::BufferProgram {
    crate::std_buffer::BufferProgram {
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
    }
}

/// The file-scope text `source.string.v1` contributes to a program it is part
/// of (card `C-EMIT-MAIN-UNIFY`).
///
/// The text prelude is contributed **only** for a program that renders text,
/// exactly as before: `STRING-SLICE-IO`'s byte-identity claim — a program that
/// renders none is unchanged by that card — is a property of this conditional,
/// and moving the condition into the assembler would have replaced a measured
/// fact with an unconditional fragment the assembler then had to remove again.
///
/// The **buffer** prelude is contributed on the same terms since card
/// `STRING-CONCAT`: only a program that acquires a buffer carries `calloc`,
/// `free` and their accessors, so every program written before that card — and
/// every program that only composes text — is byte-identical to what it was.
#[must_use]
pub fn preamble(program: &StringProgram, certificate: &StringCertificate) -> Vec<Fragment> {
    let mut fragments = vec![
        Fragment::new(Section::Includes, "string.includes", INCLUDES),
        io_emit::io_prelude_fragment(),
    ];
    if program
        .events
        .iter()
        .any(|event| matches!(event, StringEvent::Text { .. }))
    {
        fragments.push(io_emit::text_prelude_fragment());
    }
    fragments.push(Fragment::new(
        Section::Prelude,
        "string.prelude_header",
        STRING_PRELUDE_HEADER,
    ));
    fragments.push(Fragment::new(
        Section::Prelude,
        "string.prelude",
        STRING_PRELUDE_VIEW,
    ));
    // Only a program that derives a view at run time carries the helper that
    // does it. Both forms that call it are here, and a `concat` piece is
    // deliberately not one of them.
    if program.events.iter().any(|event| {
        matches!(
            event,
            StringEvent::Slice { .. }
                | StringEvent::Text {
                    operand: TextOperand::Slice { .. },
                    ..
                }
        )
    }) {
        fragments.push(Fragment::new(
            Section::Prelude,
            "string.prelude_slice",
            STRING_PRELUDE_SLICE,
        ));
    }
    // Scanned **at any depth** since card `LOOPS-ON-COMPOSED-ROUTES`: a store
    // whose only occurrence is inside a `for` body still needs the writer in
    // the prelude — `LOOP-V1`'s depth-recursive scan, surviving the
    // composition. Missing it would be a C compile error rather than a
    // refusal, but a compile error is not the fail-closed answer either.
    let buffer_event = |matcher: fn(&BufferEvent) -> bool| {
        program.events.iter().any(|event| match event {
            StringEvent::Buffer(buffer_event) => buffer_event.any(&matcher),
            _ => false,
        })
    };
    if buffer_event(|event| matches!(event, BufferEvent::Acquire { .. })) {
        fragments.push(Fragment::new(
            Section::Prelude,
            "string.buffer_prelude",
            std_buffer_c_emit::buffer_prelude_for(
                buffer_event(|event| matches!(event, BufferEvent::Store { .. })),
                buffer_event(|event| matches!(event, BufferEvent::Load { .. })),
            ),
        ));
    }
    // Only the literals the emitted program **addresses**. A literal written as
    // a `concat` piece is read by the checker and folded into the composed
    // payload, so the emitted program never names it — and an unreferenced
    // file-scope `static const` array is an error under `-Wall -Wextra
    // -Werror`. The array names carry their pool index, so skipping one
    // renumbers nothing and every surviving name is the name it always had.
    let mut referenced = std::collections::BTreeSet::new();
    for event in &program.events {
        match event {
            StringEvent::Literal { literal, .. }
            | StringEvent::Text {
                operand: TextOperand::Literal(literal),
                ..
            } => {
                referenced.insert(*literal);
            }
            _ => {}
        }
    }
    let mut payloads = String::from("\n");
    for (index, payload) in program.literals.iter().enumerate() {
        if referenced.contains(&index) {
            payloads.push_str(&payload_definition(&literal_name(index), payload));
        }
    }
    for (index, payload) in certificate.composed.iter().enumerate() {
        payloads.push_str(&payload_definition(&composed_name(index), payload));
    }
    fragments.push(Fragment::new(
        Section::FileScope,
        "string.literals",
        payloads,
    ));
    fragments
}

/// The single-emission-route check, applied to this backend: an emitted program
/// must contain no standard-output call outside [`IO_PRELUDE`]'s own
/// definitions (card `TRACE-STDIO`).
#[must_use]
pub fn direct_stdio_calls(source: &str) -> Vec<(usize, &'static str)> {
    io_emit::direct_stdio_calls(source)
}

/// Replace every `/* … */` and `//` comment body with spaces, keeping newlines
/// so line numbers survive.
///
/// Needed because [`allocator_calls`] must measure the emitted **code** and this
/// route's prelude comment names the very functions the tripwire looks for, in
/// the course of saying that they are absent. Scanning the prose would make the
/// explanation of the property falsify it.
fn without_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut stripped = String::with_capacity(source.len());
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if bytes[cursor..].starts_with(b"/*") {
            let end = source[cursor + 2..]
                .find("*/")
                .map_or(bytes.len(), |offset| cursor + 2 + offset + 2);
            for byte in &bytes[cursor..end] {
                stripped.push(if *byte == b'\n' { '\n' } else { ' ' });
            }
            cursor = end;
            continue;
        }
        if bytes[cursor..].starts_with(b"//") {
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                stripped.push(' ');
                cursor += 1;
            }
            continue;
        }
        // The emitted C is ASCII, so a byte is a character here.
        stripped.push(char::from(bytes[cursor]));
        cursor += 1;
    }
    stripped
}

/// Every allocator, copier or scanner call in an emitted program.
///
/// **The zero-allocation, zero-copy tripwire.** Amendment §4 requires slicing to
/// perform no allocation and no copy, §3.1 requires the length without scanning
/// and §3.7 forbids `strlen`; this function is what a test points at an emitted
/// program to check all three at once, rather than asserting them about the
/// emitter's intentions. It must return empty for every program this backend
/// writes.
#[must_use]
pub fn allocator_calls(source: &str) -> Vec<(usize, &'static str)> {
    const FORBIDDEN: [&str; 8] = [
        "malloc", "calloc", "realloc", "strdup", "memcpy", "memmove", "strcpy", "strlen",
    ];
    let mut found = Vec::new();
    for (number, line) in without_comments(source).lines().enumerate() {
        for name in FORBIDDEN {
            if line.contains(name) {
                found.push((number + 1, name));
            }
        }
    }
    found
}
