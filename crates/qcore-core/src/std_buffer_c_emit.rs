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

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::io_emit::{self, IO_PRELUDE};
use crate::std_buffer::{BufferCertificate, BufferEvent, BufferOperand, BufferProgram, MAX_EXTENT};
use crate::std_io::{StdIoEvent, StdIoOperand, Stream};
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

/// The C fragment for one program: the header, the lifetime pair, and each
/// accessor the program actually uses.
fn buffer_prelude(program: &BufferProgram) -> String {
    let mut prelude = String::from(BUFFER_PRELUDE_HEADER);
    prelude.push_str(BUFFER_PRELUDE_LIFETIME);
    if program
        .events
        .iter()
        .any(|event| matches!(event, BufferEvent::Store { .. }))
    {
        prelude.push_str(BUFFER_PRELUDE_STORE);
    }
    if program
        .events
        .iter()
        .any(|event| matches!(event, BufferEvent::Load { .. }))
    {
        prelude.push_str(BUFFER_PRELUDE_LOAD);
    }
    prelude
}

const INCLUDES: &str = "\
#include <inttypes.h>\n\
#include <stdint.h>\n\
#include <stdio.h>\n\
#include <stdlib.h>\n";

/// The C name of a buffer binding.
fn buffer_name(name: &str) -> String {
    format!("b_{name}")
}

/// The C name of a scalar binding.
fn scalar_name(name: &str) -> String {
    format!("s_{name}")
}

/// The reference observation, as `"<stream>: <line>"` records in program order.
/// A consumer projects one stream by filtering on the prefix.
///
/// The lines are the **certificate's** lines: the checker already computed
/// every rendered value out of the modelled bytes, so re-deriving them here
/// would be a second implementation of the same arithmetic rather than a check
/// of the first.
pub fn interpret_buffer_program(
    program: &BufferProgram,
    certificate: &BufferCertificate,
) -> Result<Vec<String>, BufferEmissionRefusal> {
    let produced = program
        .events
        .iter()
        .filter(|event| {
            matches!(
                event,
                BufferEvent::Line(StdIoEvent::Print { .. } | StdIoEvent::Log { .. })
            )
        })
        .count();
    if produced != certificate.lines.len() {
        return Err(BufferEmissionRefusal::CertificateDisagrees);
    }
    Ok(certificate
        .lines
        .iter()
        .map(|line| format!("{}: {}", line.stream.label(), line.text))
        .collect())
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

/// Emit C11 for a checked program. The emitted lines must reproduce the
/// reference observation exactly, per stream.
pub fn emit_buffer_c11(
    program: &BufferProgram,
    certificate: &BufferCertificate,
) -> Result<BufferCEmission, BufferEmissionRefusal> {
    // Refuse before emitting anything if the certificate is not this program's.
    let _ = interpret_buffer_program(program, certificate)?;

    let mut source = String::from(INCLUDES);
    source.push_str(IO_PRELUDE);
    source.push_str(&buffer_prelude(program));
    source.push_str("\nint main(void)\n{\n");

    // Every binding is declared at its own statement, in program order: unlike
    // `source.std-io.v1`, whose scalars are all literals known before the first
    // event, a scalar here can be the result of a byte read and therefore has
    // no value until its event runs.
    let mut scalars: BTreeMap<String, String> = BTreeMap::new();
    let mut extents: BTreeMap<String, u32> = BTreeMap::new();

    for (index, event) in program.events.iter().enumerate() {
        match event {
            BufferEvent::Bind { binding, value } => {
                let _ = writeln!(
                    source,
                    "    int32_t {} = INT32_C({value});",
                    scalar_name(binding)
                );
                scalars.insert(binding.clone(), scalar_name(binding));
            }
            BufferEvent::Acquire { name, extent } => {
                debug_assert!(*extent > 0 && *extent <= MAX_EXTENT);
                let _ = writeln!(
                    source,
                    "    unsigned char *{} = qcore_bytes_new((size_t) {extent}u);",
                    buffer_name(name)
                );
                extents.insert(name.clone(), *extent);
            }
            BufferEvent::Store {
                name,
                index: at,
                value,
            } => {
                let Some(extent) = extents.get(name).copied() else {
                    return Err(BufferEmissionRefusal::UnplaceableAccess { event: index });
                };
                let at = operand_expression(&scalars, index, at)?;
                let value = operand_expression(&scalars, index, value)?;
                let _ = writeln!(
                    source,
                    "    qcore_bytes_store({}, (size_t) {extent}u, (size_t) ({at}), {value});",
                    buffer_name(name)
                );
            }
            BufferEvent::Load {
                binding,
                name,
                index: at,
            } => {
                let Some(extent) = extents.get(name).copied() else {
                    return Err(BufferEmissionRefusal::UnplaceableAccess { event: index });
                };
                let at = operand_expression(&scalars, index, at)?;
                let _ = writeln!(
                    source,
                    "    int32_t {} = qcore_bytes_load({}, (size_t) {extent}u, (size_t) ({at}));",
                    scalar_name(binding),
                    buffer_name(name)
                );
                scalars.insert(binding.clone(), scalar_name(binding));
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
                let _ = writeln!(
                    source,
                    "    int32_t {} = INT32_C({extent});",
                    scalar_name(binding)
                );
                scalars.insert(binding.clone(), scalar_name(binding));
            }
            BufferEvent::Release { name } => {
                let _ = writeln!(source, "    qcore_bytes_drop({});", buffer_name(name));
            }
            BufferEvent::Line(line) => {
                // The rendering itself — the conversion, the level tag, the
                // stream, the `fflush` — is decided by `source.std-io.v1`'s own
                // emitter and nowhere else, so this route renders one way with
                // it rather than in a way that happens to agree.
                let identities = BTreeMap::new();
                match std_io_c_emit::event_statement(line, &scalars, &identities) {
                    Ok(Some(statement)) => {
                        let _ = writeln!(source, "    {statement}");
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

    source.push_str("    return 0;\n}\n");
    Ok(BufferCEmission { source })
}

/// Every operand name a line-producing event renders, so a caller can check the
/// backend named nothing it did not declare. Published for tests rather than
/// used by the emitter, which resolves names through its own map.
#[must_use]
pub fn rendered_names(program: &BufferProgram) -> Vec<String> {
    program
        .events
        .iter()
        .filter_map(|event| match event {
            BufferEvent::Line(
                StdIoEvent::Print { operand, .. } | StdIoEvent::Log { operand, .. },
            ) => match operand {
                StdIoOperand::Scalar(name) | StdIoOperand::Borrowed(name) => Some(name.clone()),
                StdIoOperand::Literal(_) => None,
            },
            _ => None,
        })
        .collect()
}

/// The streams this route can write, for a consumer that needs the pair without
/// depending on `std_io`'s enum directly.
#[must_use]
pub fn streams() -> [Stream; 2] {
    [Stream::Out, Stream::Err]
}

/// A convenience for a caller that wants only one stream's projection of the
/// reference observation.
#[must_use]
pub fn project(observation: &[String], stream: Stream) -> Vec<String> {
    let prefix = format!("{}: ", stream.label());
    observation
        .iter()
        .filter_map(|line| line.strip_prefix(&prefix).map(ToOwned::to_owned))
        .collect()
}

/// The single-emission-route check, applied to this backend: an emitted program
/// must contain no standard-output call outside [`IO_PRELUDE`]'s own
/// definitions (card `TRACE-STDIO`).
#[must_use]
pub fn direct_stdio_calls(source: &str) -> Vec<(usize, &'static str)> {
    io_emit::direct_stdio_calls(source)
}
