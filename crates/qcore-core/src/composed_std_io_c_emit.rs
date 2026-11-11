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
//! program order, exactly the shape `source.std-io.v1` publishes. Only
//! source-authored `print` and `log` records contribute. The extern projection
//! retains its structured trace as diagnostic evidence, but routes it through
//! the default-null diagnostic logger rather than a program stream.
//!
//! A raised error ends the trace: the extern projection's own interpretation
//! stops at the raise, and every composed step after it — renderings included —
//! produces nothing, because the emitted C has returned.

use std::fmt::Write as _;

use crate::c_program::{self, Fragment, Section};
use crate::composed_std_io::{ComposedCertificate, ComposedProgram, ComposedStep};
use crate::diagnostic_log_c_emit;
use crate::extern_c_emit::{
    self, ExternEmissionRefusal, emit_extern_statements, trace_extern_events,
};
use crate::scalar_types as st;
use crate::std_io::{Rendering, StdIoEvent, StdIoOperand};
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
                let Some((_, _lines)) = extern_trace.iter().find(|(at, _)| at == index) else {
                    // Past the raise: this event is in neither trace nor run.
                    stopped = true;
                    break;
                };
                if raised_at == Some(*index) {
                    stopped = true;
                    break;
                }
            }
            ComposedStep::Io(index) => {
                let Some(event) = program.io.events.get(*index) else {
                    return Err(ComposedEmissionRefusal::ObservationDisagrees);
                };
                if matches!(
                    event,
                    StdIoEvent::LogLevel { .. } | StdIoEvent::Flush | StdIoEvent::Return
                ) {
                    continue;
                }
                let Some(line) = io_lines.next() else {
                    return Err(ComposedEmissionRefusal::ObservationDisagrees);
                };
                observation.push(line.clone());
            }
            ComposedStep::Return => {
                let last = program.externs.events.len() - 1;
                let Some((_, _lines)) = extern_trace.iter().find(|(at, _)| *at == last) else {
                    stopped = true;
                    break;
                };
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

    // Two routes, one entry point (card `C-EMIT-MAIN-UNIFY`). The fragments the
    // two halves need are contributed under shared keys — the typed logger and
    // the IO prelude are each named once no matter how many routes want them —
    // and the two halves' statements interleave into the single entry's body
    // rather than into a second `main`.
    let mut fragments = vec![Fragment::new(
        Section::Banner,
        "composed.banner",
        "/* Generated by qcore `source.std-io-extern.v1`. */\n",
    )];
    fragments.push(Fragment::new(
        Section::Includes,
        "composed.includes",
        INCLUDES,
    ));
    if let Some(level) = certificate.io.log_level {
        fragments.push(diagnostic_log_c_emit::adapter_declaration_fragment(level));
    }
    fragments.push(diagnostic_log_c_emit::diagnostic_log_fragment());
    if certificate.io.log_level.is_some() {
        fragments.push(diagnostic_log_c_emit::adapter_definition_fragment());
    }
    fragments.push(crate::io_emit::io_prelude_fragment());
    fragments.push(extern_c_emit::prototypes_fragment(&program.externs));
    // Card `W48-QR-SCALAR-TYPES-R3`: the extern model's value statements bring
    // their headers, width assertions and helpers — nothing for a program
    // without one.
    fragments.extend(
        extern_c_emit::scalar_fragments(&program.externs, &certificate.externs)
            .map_err(ComposedEmissionRefusal::Extern)?,
    );

    let mut source = String::new();
    // Card `W48-QR-FLOATING-POINT`: the float renderers the lines use.
    let mut renderers = std::collections::BTreeSet::new();
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
                // Card `W48-QR-SCALAR-TYPES-R3`: a value of another §G02t type
                // renders at its type — §G02t's conversion, over the extern
                // model's own variable cast to the type's C11 spelling (the
                // ABI's `intptr_t`/`uintptr_t` and `_Bool` are the same values
                // at the same widths), framed like every line.
                if let StdIoEvent::Print {
                    rendering,
                    operand: StdIoOperand::Scalar(name),
                }
                | StdIoEvent::Log {
                    rendering,
                    operand: StdIoOperand::Scalar(name),
                    ..
                } = event
                    && let Some((ty, _)) = program.io.typed_scalars.get(name)
                {
                    let Some(variable) = expressions.get(name) else {
                        return Err(ComposedEmissionRefusal::UnboundOperand {
                            step: *index,
                            name: name.clone(),
                        });
                    };
                    let level = match event {
                        StdIoEvent::Log { level, .. } => Some(*level),
                        _ => None,
                    };
                    let (stream, prefix) = crate::std_io::line_framing(level);
                    if ty.is_float() {
                        renderers.insert((*rendering == Rendering::Hex, *ty));
                    }
                    let cast = format!("({})({variable})", st::c_type(*ty));
                    let Some((conversion, operand)) = st::c_rendering(
                        *rendering == Rendering::Hex,
                        *ty,
                        &cast,
                        program.io.target.as_ref(),
                    ) else {
                        return Err(ComposedEmissionRefusal::UnboundOperand {
                            step: *index,
                            name: name.clone(),
                        });
                    };
                    let _ = writeln!(
                        source,
                        "    {}",
                        crate::io_emit::write_line(
                            stream,
                            &format!("{prefix}{conversion}"),
                            &[&operand]
                        )
                    );
                    continue;
                }
                match std_io_c_emit::event_statement(event, &expressions, &program.io.identities) {
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
    if !renderers.is_empty() {
        fragments.push(Fragment::new(
            Section::Prelude,
            "scalar-floats.renderers",
            format!("\n{}", crate::scalar_floats::c_renderers(&renderers)),
        ));
    }
    Ok(ComposedCEmission {
        source: c_program::assemble(&fragments, &c_program::Entry::source_route(source)),
    })
}
