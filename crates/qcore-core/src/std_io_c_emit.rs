//! Reference execution and C11 emission for `source.std-io.v1`
//! (card `STD-MODULES`).
//!
//! The reference execution and the emitted program are compared line for line,
//! **per stream**. No order between the two streams is claimed: a run's stdout
//! lines are compared with the reference's stdout projection and its stderr
//! lines with the reference's stderr projection. Interleaving is not modeled,
//! because C gives no order between two separately buffered streams and this
//! card does not measure one.
//!
//! `flush` produces no line, so it never moves either projection. It emits a
//! real `fflush` on the logger's stream and nothing else — sink handoff, not
//! durability.

use std::fmt::Write as _;

use crate::std_io::{StdIoCertificate, StdIoEvent, StdIoOperand, StdIoProgram, Stream};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StdIoEmissionRefusal {
    /// An operand the certificate's program does not bind. The checker already
    /// refuses this; the backend refuses it again rather than rendering a
    /// value it cannot justify.
    UnboundScalar { event: usize, name: String },
    /// A certificate whose line count disagrees with the program's.
    CertificateDisagrees,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdIoCEmission {
    pub source: String,
}

fn operand_value(
    program: &StdIoProgram,
    index: usize,
    operand: &StdIoOperand,
) -> Result<i32, StdIoEmissionRefusal> {
    match operand {
        StdIoOperand::Literal(value) => Ok(*value),
        StdIoOperand::Scalar(name) => {
            program
                .scalars
                .get(name)
                .copied()
                .ok_or_else(|| StdIoEmissionRefusal::UnboundScalar {
                    event: index,
                    name: name.clone(),
                })
        }
    }
}

/// The reference observation, as `"<stream>: <line>"` records in program order.
/// A consumer projects one stream by filtering on the prefix.
pub fn interpret_std_io_program(
    program: &StdIoProgram,
    certificate: &StdIoCertificate,
) -> Result<Vec<String>, StdIoEmissionRefusal> {
    let mut observation = Vec::new();
    for (index, event) in program.events.iter().enumerate() {
        match event {
            StdIoEvent::Print { rendering, operand } => {
                let value = operand_value(program, index, operand)?;
                observation.push(format!(
                    "{}: {}",
                    Stream::Out.label(),
                    rendering.render(value)
                ));
            }
            StdIoEvent::Log {
                level,
                rendering,
                operand,
            } => {
                let value = operand_value(program, index, operand)?;
                observation.push(format!(
                    "{}: [{}] {}",
                    Stream::Err.label(),
                    level.tag(),
                    rendering.render(value)
                ));
            }
            StdIoEvent::Flush | StdIoEvent::Return => {}
        }
    }
    // The certificate is the checker's own record of the same lines. Rendering
    // a program whose certificate disagrees with it would publish an
    // observation no check covered.
    if observation.len() != certificate.lines.len() {
        return Err(StdIoEmissionRefusal::CertificateDisagrees);
    }
    for (rendered, line) in observation.iter().zip(&certificate.lines) {
        if *rendered != format!("{}: {}", line.stream.label(), line.text) {
            return Err(StdIoEmissionRefusal::CertificateDisagrees);
        }
    }
    Ok(observation)
}

const PRELUDE: &str = "\
#include <inttypes.h>\n\
#include <stdint.h>\n\
#include <stdio.h>\n\
\n\
/* source.std-io.v1 (card STD-MODULES). Every line below is one admitted\n\
   `print`, `log` or `flush` event. `fflush` is sink handoff, not durability:\n\
   no fsync is performed and no crash survival is claimed. */\n";

/// Emit C11 for a checked program. The emitted lines must reproduce the
/// reference observation exactly, per stream.
pub fn emit_std_io_c11(
    program: &StdIoProgram,
    certificate: &StdIoCertificate,
) -> Result<StdIoCEmission, StdIoEmissionRefusal> {
    // Refuse before emitting anything if the certificate is not this program's.
    let _ = interpret_std_io_program(program, certificate)?;

    let mut source = String::from(PRELUDE);
    source.push_str("\nint main(void)\n{\n");

    for (name, value) in &program.scalars {
        let _ = writeln!(source, "    int32_t s_{name} = INT32_C({value});");
    }

    for (index, event) in program.events.iter().enumerate() {
        match event {
            StdIoEvent::Print { rendering, operand } => {
                let expression = match operand {
                    StdIoOperand::Literal(value) => format!("INT32_C({value})"),
                    StdIoOperand::Scalar(name) => {
                        // Rechecked here so the backend never renders a name
                        // the program does not bind.
                        let _ = operand_value(program, index, operand)?;
                        format!("s_{name}")
                    }
                };
                let _ = writeln!(
                    source,
                    "    printf(\"{}\\n\", {});",
                    rendering.c_conversion(),
                    rendering.c_operand(&expression)
                );
            }
            StdIoEvent::Log {
                level,
                rendering,
                operand,
            } => {
                let expression = match operand {
                    StdIoOperand::Literal(value) => format!("INT32_C({value})"),
                    StdIoOperand::Scalar(name) => {
                        let _ = operand_value(program, index, operand)?;
                        format!("s_{name}")
                    }
                };
                let _ = writeln!(
                    source,
                    "    fprintf(stderr, \"[{}] {}\\n\", {});",
                    level.tag(),
                    rendering.c_conversion(),
                    rendering.c_operand(&expression)
                );
            }
            StdIoEvent::Flush => {
                let _ = writeln!(source, "    (void) fflush({});", Stream::Err.c_handle());
            }
            StdIoEvent::Return => {}
        }
    }

    source.push_str("    return 0;\n}\n");
    Ok(StdIoCEmission { source })
}
