//! Reference execution and C11 emission for `source.std-io-extern.v1`
//! (card `STD-IO-COMPOSE`).
//!
//! # One lowering per model, joined by order
//!
//! Neither half of a composed program is lowered here. The extern call
//! statements come from [`extern_c_emit::emit_extern_statements`] and the
//! rendering statements from [`std_io_c_emit::event_statement`] — the same
//! functions the two single-model routes use — and this module only places them
//! in the composed order and writes the prologue that both need. A composed
//! module's call statements are therefore byte-identical to the extern route's,
//! and its rendering statements byte-identical to the standard-IO route's, by
//! construction rather than by comparison.
//!
//! # The observation, per stream
//!
//! The composed observation is a list of `"<stream>: <line>"` records in
//! program order, exactly the shape `source.std-io.v1` publishes. Standard
//! output carries two kinds of line — this route's `print` lines and the extern
//! route's synthesized call trace — and their order *on that stream* is real:
//! one `FILE *`, one sequence. The order **between** stdout and stderr is not
//! modeled and not claimed, which is the boundary `source.std-io.v1` already
//! stated.
//!
//! A raised error ends the trace: the extern projection's own interpretation
//! stops at the raise, and every composed step after it — renderings included —
//! produces nothing, because the emitted C has returned.

use std::fmt::Write as _;

use crate::composed_std_io::{ComposedCertificate, ComposedProgram, ComposedStep};
use crate::extern_c_emit::{
    self, ExternEmissionRefusal, emit_extern_statements, extern_prototypes, trace_extern_events,
};
use crate::io_emit::IO_PRELUDE;
use crate::std_io::{StdIoEvent, Stream};
use crate::std_io_c_emit::{self, StdIoEmissionRefusal};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposedEmissionRefusal {
    /// The certificate does not describe this program.
    UncheckedProgram,
    /// The extern projection refused to emit or to run.
    Extern(ExternEmissionRefusal),
    /// The standard-IO projection refused to emit or to run.
    StandardIo(StdIoEmissionRefusal),
    /// A rendering names a scalar the composed program binds no value for. The
    /// checker already refuses this; the backend refuses it again rather than
    /// naming a C variable it never declared.
    UnboundOperand { step: usize, name: String },
    /// The composed observation does not reproduce a projection's own.
    ObservationDisagrees,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedCEmission {
    pub source: String,
}

/// The C expression each renderable scalar name denotes: the extern model's own
/// value variable, `vN`, never a fresh copy. A composed rendering reads exactly
/// the object the call bound.
fn operand_expressions(program: &ComposedProgram) -> std::collections::BTreeMap<String, String> {
    program
        .operand_values
        .iter()
        .map(|(name, value)| (name.clone(), extern_c_emit::value_variable(*value)))
        .collect()
}

/// The reference observation, as `"<stream>: <line>"` records in composed
/// program order.
pub fn interpret_composed_program(
    program: &ComposedProgram,
    certificate: &ComposedCertificate,
) -> Result<Vec<String>, ComposedEmissionRefusal> {
    if certificate.entry != program.entry {
        return Err(ComposedEmissionRefusal::UncheckedProgram);
    }
    let extern_trace = trace_extern_events(&program.externs, &certificate.externs)
        .map_err(ComposedEmissionRefusal::Extern)?;
    let io_trace = std_io_c_emit::interpret_std_io_program(&program.io, &certificate.io)
        .map_err(ComposedEmissionRefusal::StandardIo)?;
    // A raised error ends the *whole* composed run, not only the extern
    // projection's share of it: the emitted C returns inside the failure arm,
    // so every later step — a rendering included — writes nothing. The extern
    // projection reports that by truncating its own trace, and this is the
    // composed step it truncated after. Getting this wrong would publish
    // rendered lines the program never writes, which is the one way a per-stream
    // comparison could disagree with a real run.
    let raised_at = (extern_trace.len() < program.externs.events.len())
        .then(|| extern_trace.last().map(|(index, _)| *index))
        .flatten();
    let mut observation = Vec::new();
    let mut io_lines = io_trace.iter();
    let mut stopped = false;
    for step in &program.order {
        match step {
            ComposedStep::Extern(index) => {
                let Some((_, lines)) = extern_trace.iter().find(|(at, _)| at == index) else {
                    // Past the raise: this event is in neither trace nor run.
                    stopped = true;
                    break;
                };
                for line in lines {
                    observation.push(format!("{}: {line}", Stream::Out.label()));
                }
                if raised_at == Some(*index) {
                    stopped = true;
                    break;
                }
            }
            ComposedStep::Io(index) => {
                let Some(event) = program.io.events.get(*index) else {
                    return Err(ComposedEmissionRefusal::ObservationDisagrees);
                };
                if matches!(event, StdIoEvent::Flush | StdIoEvent::Return) {
                    continue;
                }
                let Some(line) = io_lines.next() else {
                    return Err(ComposedEmissionRefusal::ObservationDisagrees);
                };
                observation.push(line.clone());
            }
            ComposedStep::Return => {
                let last = program.externs.events.len() - 1;
                let Some((_, lines)) = extern_trace.iter().find(|(at, _)| *at == last) else {
                    stopped = true;
                    break;
                };
                for line in lines {
                    observation.push(format!("{}: {line}", Stream::Out.label()));
                }
            }
        }
    }
    // Nothing is dropped at the join: every line of the standard-IO
    // projection's own observation was placed — unless the run raised, in which
    // case the lines after the raise are exactly the ones the program does not
    // write.
    if !stopped && io_lines.next().is_some() {
        return Err(ComposedEmissionRefusal::ObservationDisagrees);
    }
    Ok(observation)
}

const INCLUDES: &str = "\
#include <inttypes.h>\n\
#include <stdint.h>\n\
#include <stdio.h>\n";

/// Emit one composed module as a self-contained C11 translation unit. The
/// extern callees are declared, never defined: a build must supply them.
pub fn emit_composed_c11(
    program: &ComposedProgram,
    certificate: &ComposedCertificate,
) -> Result<ComposedCEmission, ComposedEmissionRefusal> {
    // Refuse before emitting anything if the certificate is not this program's.
    let _ = interpret_composed_program(program, certificate)?;
    let statements = emit_extern_statements(&program.externs, &certificate.externs)
        .map_err(ComposedEmissionRefusal::Extern)?;
    let expressions = operand_expressions(program);

    let mut source = String::new();
    source.push_str("/* Generated by qcore `source.std-io-extern.v1`. */\n");
    source.push_str(INCLUDES);
    source.push_str(IO_PRELUDE);
    source.push('\n');
    source.push_str(&extern_prototypes(&program.externs));
    source.push_str("\nint main(void)\n{\n");
    for step in &program.order {
        match step {
            ComposedStep::Extern(index) => {
                if let Some((_, text)) = statements.iter().find(|(at, _)| at == index) {
                    source.push_str(text);
                }
            }
            ComposedStep::Io(index) => {
                let Some(event) = program.io.events.get(*index) else {
                    return Err(ComposedEmissionRefusal::ObservationDisagrees);
                };
                match std_io_c_emit::event_statement(event, &expressions) {
                    Ok(Some(statement)) => {
                        let _ = writeln!(source, "    {statement}");
                    }
                    Ok(None) => {}
                    Err(name) => {
                        return Err(ComposedEmissionRefusal::UnboundOperand { step: *index, name });
                    }
                }
            }
            ComposedStep::Return => {
                let last = program.externs.events.len() - 1;
                if let Some((_, text)) = statements.iter().find(|(at, _)| *at == last) {
                    source.push_str(text);
                }
            }
        }
    }
    source.push_str("    return 0;\n}\n");
    Ok(ComposedCEmission { source })
}
