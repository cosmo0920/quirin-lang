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

use crate::c_program::{self, Fragment, Section};
use crate::io_emit;
use crate::std_io::{StdIoCertificate, StdIoEvent, StdIoOperand, StdIoProgram, Stream};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StdIoEmissionRefusal {
    /// An operand the certificate's program does not bind. The checker already
    /// refuses this; the backend refuses it again rather than rendering a
    /// value it cannot justify.
    UnboundScalar { event: usize, name: String },
    /// A `borrow NAME` operand for which no custody model published an identity
    /// (card `STD-FMT-BORROW`). Refused again here for the same reason: a
    /// backend must never render a number no model supplied.
    UnborrowableIdentity { event: usize, name: String },
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
        StdIoOperand::Borrowed(name) => program.identities.get(name).copied().ok_or_else(|| {
            StdIoEmissionRefusal::UnborrowableIdentity {
                event: index,
                name: name.clone(),
            }
        }),
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
        // Card `W48-QR-SCALAR-TYPES-R3`: an operand of another §G02t type,
        // rendered at its type by §G02t's rendering and this route's framing.
        if let StdIoEvent::Print {
            rendering,
            operand: StdIoOperand::Scalar(name),
        }
        | StdIoEvent::Log {
            rendering,
            operand: StdIoOperand::Scalar(name),
            ..
        } = event
            && let Some((ty, value)) = program.typed_scalars.get(name)
        {
            let level = match event {
                StdIoEvent::Log { level, .. } => Some(*level),
                _ => None,
            };
            let body = crate::scalar_types::render(
                *rendering == crate::std_io::Rendering::Hex,
                *ty,
                *value,
                program.target.as_ref(),
            )
            .ok_or(StdIoEmissionRefusal::CertificateDisagrees)?;
            let line = crate::std_io::rendered_text_line(level, &body);
            observation.push(format!("{}: {}", line.stream.label(), line.text));
            continue;
        }
        match event {
            StdIoEvent::LogLevel { .. } => {}
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

/// The C statement one standard-IO event lowers to, or `None` for an event
/// that emits nothing.
///
/// `expressions` maps each scalar name to the C expression that denotes it, so
/// the caller decides where a rendered value lives: this route declares its own
/// `s_NAME` copies of the literals it binds, and the composed route
/// (card `STD-IO-COMPOSE`) points at the extern model's `vN` variables instead.
/// The rendering itself — the conversion, the level tag, the stream, the
/// `fflush` — is decided here and nowhere else, so both routes render one way.
///
/// `identities` maps each borrowed name to the model identity a custody model
/// published for it (card `STD-FMT-BORROW`). A borrowed render emits that
/// identity as a **literal**, and it is worth saying plainly what that means:
/// the emitted program reads nothing out of the object. The identity is a
/// number the checker determined, exactly as this route's `let`-bound integers
/// are; what the borrow buys is the *checked precondition* that the resource
/// was live at this statement, not a run-time read. No address is emitted, and
/// none could be — an address is not model-determined, so a line carrying one
/// could not be compared against a reference run at all.
///
/// `Err(name)` is a name the caller supplied no expression for. The checker
/// already refuses that; refusing again is what keeps a backend from naming a
/// variable it never declared.
pub fn event_statement(
    event: &StdIoEvent,
    expressions: &std::collections::BTreeMap<String, String>,
    identities: &std::collections::BTreeMap<String, i32>,
) -> Result<Option<String>, String> {
    let expression = |operand: &StdIoOperand| -> Result<String, String> {
        match operand {
            StdIoOperand::Literal(value) => Ok(format!("INT32_C({value})")),
            StdIoOperand::Scalar(name) => {
                expressions.get(name).cloned().ok_or_else(|| name.clone())
            }
            StdIoOperand::Borrowed(name) => identities
                .get(name)
                .map(|identity| format!("INT32_C({identity})"))
                .ok_or_else(|| name.clone()),
        }
    };
    match event {
        StdIoEvent::LogLevel { .. } => Ok(None),
        StdIoEvent::Print { rendering, operand } => Ok(Some(io_emit::write_line(
            Stream::Out,
            rendering.c_conversion(),
            &[&rendering.c_operand(&expression(operand)?)],
        ))),
        StdIoEvent::Log {
            level,
            rendering,
            operand,
        } => Ok(Some(io_emit::write_line(
            Stream::Err,
            &format!("[{}] {}", level.tag(), rendering.c_conversion()),
            &[&rendering.c_operand(&expression(operand)?)],
        ))),
        StdIoEvent::Flush => Ok(Some(io_emit::flush(Stream::Err))),
        StdIoEvent::Return => Ok(None),
    }
}

const INCLUDES: &str = "\
#include <inttypes.h>\n\
#include <stdint.h>\n\
#include <stdio.h>\n";

const PRELUDE: &str = "\
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

    // Every scalar this route binds is a literal known before the first event,
    // so the whole set is declared in the entry's prologue.
    let mut prologue = String::new();
    for (name, value) in &program.scalars {
        let _ = writeln!(prologue, "    int32_t s_{name} = INT32_C({value});");
    }
    let mut source = String::new();

    // Each name denotes this route's own declared copy of the literal it was
    // bound to. The rendering itself is `event_statement`'s, which the composed
    // route shares (card `STD-IO-COMPOSE`).
    let expressions = program
        .scalars
        .keys()
        .map(|name| (name.clone(), format!("s_{name}")))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (index, event) in program.events.iter().enumerate() {
        // Rechecked here so the backend never renders a name the program does
        // not bind.
        match event {
            StdIoEvent::LogLevel { .. } => {}
            StdIoEvent::Print { operand, .. } | StdIoEvent::Log { operand, .. } => {
                let _ = operand_value(program, index, operand)?;
            }
            StdIoEvent::Flush | StdIoEvent::Return => {}
        }
        match event_statement(event, &expressions, &program.identities) {
            Ok(Some(statement)) => {
                let _ = writeln!(source, "    {statement}");
            }
            Ok(None) => {}
            Err(name) => {
                return Err(StdIoEmissionRefusal::UnboundScalar { event: index, name });
            }
        }
    }

    Ok(StdIoCEmission {
        source: c_program::assemble(
            &preamble(),
            &c_program::Entry {
                prologue,
                ..c_program::Entry::source_route(source)
            },
        ),
    })
}

/// The file-scope text `source.std-io.v1` contributes to a program it is part
/// of, whether alone or composed (card `C-EMIT-MAIN-UNIFY`).
///
/// The IO prelude is named through `io_emit`'s own constructor rather than
/// spelled here, so a program that also takes another printing route installs
/// exactly one copy.
#[must_use]
pub fn preamble() -> Vec<Fragment> {
    vec![
        Fragment::new(Section::Includes, "std_io.includes", INCLUDES),
        crate::io_emit::io_prelude_fragment(),
        Fragment::new(Section::Prelude, "std_io.prelude", PRELUDE),
    ]
}
