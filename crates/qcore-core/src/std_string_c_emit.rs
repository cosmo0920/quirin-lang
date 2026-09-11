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

use crate::io_emit::{self, IO_PRELUDE};
use crate::std_io::StdIoEvent;
use crate::std_io_c_emit;
use crate::std_string::{StringCertificate, StringEvent, StringOperand, StringProgram};

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

/// The view type and the slice helper: amendment §6's aggregate, verbatim, and
/// §4's arithmetic, verbatim.
pub const STRING_PRELUDE: &str = r#"typedef struct {
    const unsigned char *ptr;
    size_t len;
} qcore_string_view;

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

/// The reference observation, as `"<stream>: <line>"` records in program order.
///
/// The lines are the **certificate's** lines: the checker already computed every
/// rendered value, so re-deriving them here would be a second implementation of
/// the same arithmetic rather than a check of the first.
pub fn interpret_string_program(
    program: &StringProgram,
    certificate: &StringCertificate,
) -> Result<Vec<String>, StringEmissionRefusal> {
    let produced = program
        .events
        .iter()
        .filter(|event| {
            matches!(
                event,
                StringEvent::Line(StdIoEvent::Print { .. } | StdIoEvent::Log { .. })
            )
        })
        .count();
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

/// One literal's static byte array, written as explicit hexadecimal bytes.
///
/// Byte-wise and not as a C string literal, deliberately: a payload may contain
/// an embedded NUL, and a C string literal would make that byte look like the
/// end of the data to every reader and tool. The trailing `0x00` after the
/// payload is the toolchain terminator and is outside the declared length.
fn literal_definition(index: usize, payload: &[u8]) -> String {
    let mut bytes: Vec<String> = payload.iter().map(|byte| format!("0x{byte:02x}")).collect();
    bytes.push("0x00".to_owned());
    format!(
        "static const unsigned char {}[{}] = {{ {} }};\n",
        literal_name(index),
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

    let mut source = String::from(INCLUDES);
    source.push_str(IO_PRELUDE);
    source.push_str(STRING_PRELUDE_HEADER);
    source.push_str(STRING_PRELUDE);
    source.push('\n');
    for (index, payload) in program.literals.iter().enumerate() {
        source.push_str(&literal_definition(index, payload));
    }
    source.push_str("\nint main(void)\n{\n");

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
            StringEvent::Bind { .. }
            | StringEvent::Literal { .. }
            | StringEvent::Line(_)
            | StringEvent::Return => {}
        }
    }

    let mut scalars: BTreeMap<String, String> = BTreeMap::new();
    let mut views: BTreeMap<String, String> = BTreeMap::new();
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
            StringEvent::Return => {}
        }
    }

    source.push_str("    return 0;\n}\n");
    Ok(StringCEmission { source })
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
