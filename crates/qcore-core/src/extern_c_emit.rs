//! Native C11 backend and reference execution for checked
//! `source.extern-call.v1` modules.
//!
//! Scope, stated before anything else. This backend prints a prototype for each
//! admitted callee from the C04 ABI projection and calls it directly. It is
//! therefore evidence that one checked trace, linked against a real C
//! implementation of its callees, performs those calls in that order, takes the
//! declared arm of every nullable acquisition, and leaves no allocation behind
//! under a leak checker. It is **not** evidence about the callee's
//! implementation, its buffer extents, its aliasing, or which object file
//! supplies the symbol: linkage is the builder's premise, recorded on every
//! certificate as `ExternAssumption::LinkageSuppliedByBuilder`.
//!
//! [`interpret_extern_program`] computes the same observation from the checked
//! event list. The two are written against the model, not against each other's
//! output; agreement is a translation check for the selected inputs in the
//! sense of `docs/QCORE_SPEC_r0_3.md` §L15, not a general source refinement.
//!
//! The emitted program does not trust its own literal outcome controls. Every
//! adapter emits a runtime comparison of the real result's nullness against the
//! declared control and exits with status 70 if they disagree, so a fixture
//! whose C implementation stops matching its declared arm fails loudly instead
//! of silently observing the other branch.
use std::fmt::Write;

use crate::ResourceId;
use crate::extern_call::{
    AbiType, CallParameter, CallResult, ExternArgument, ExternCallCertificate, ExternCallee,
    ExternEvent, ExternProgram,
};

/// One line of the execution observation. The reference interpreter and the
/// emitted C must produce byte-identical sequences.
pub type ExternObservation = Vec<String>;

/// Why this backend refused to emit. Refusal is the only alternative to
/// emission; there is no partial or approximate lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternEmissionRefusal {
    /// The certificate does not describe this program.
    UncheckedProgram,
    /// A call naming a callee the program does not declare.
    UnknownCallee { event: usize, name: String },
    /// A `unit` result cannot be bound; only `resource_return_none()` callees
    /// may declare one, and those bind nothing.
    UnsupportedResult { event: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternCEmission {
    pub source: String,
}

fn resource_name(resource: ResourceId) -> String {
    format!("r{resource}")
}

fn argument_line(argument: &ExternArgument) -> String {
    match argument {
        ExternArgument::Data(value) => format!("d:{value}"),
        ExternArgument::Borrow(resource) => format!("b:{}", resource_name(*resource)),
        ExternArgument::Consume(resource) => format!("c:{}", resource_name(*resource)),
    }
}

fn call_observation(callee: &ExternCallee, arguments: &[ExternArgument]) -> String {
    let mut line = format!("call {}", callee.name);
    for argument in arguments {
        line.push(' ');
        line.push_str(&argument_line(argument));
    }
    line
}

/// Reference execution of a checked trace. Mirrors the emitted C statement for
/// statement; neither is derived from the other's output.
pub fn interpret_extern_program(
    program: &ExternProgram,
    certificate: &ExternCallCertificate,
) -> Result<ExternObservation, ExternEmissionRefusal> {
    if certificate.entry != program.entry {
        return Err(ExternEmissionRefusal::UncheckedProgram);
    }
    let mut trace = ExternObservation::new();
    for (index, event) in program.events.iter().enumerate() {
        match event {
            ExternEvent::Call(call) => {
                let callee = program.callees.get(&call.callee).ok_or_else(|| {
                    ExternEmissionRefusal::UnknownCallee {
                        event: index,
                        name: call.callee.clone(),
                    }
                })?;
                match (&call.adapter, callee.result) {
                    (Some(adapter), CallResult::OwnedNullable) => {
                        trace.push(format!(
                            "acquire {} {} {}",
                            callee.name,
                            resource_name(adapter.resource),
                            if adapter.succeeds { "ok" } else { "null" }
                        ));
                        if !adapter.succeeds {
                            trace.push(format!("raise {}", adapter.error));
                            return Ok(trace);
                        }
                    }
                    (None, CallResult::Data(_)) => {
                        trace.push(call_observation(callee, &call.arguments));
                    }
                    _ => return Err(ExternEmissionRefusal::UnsupportedResult { event: index }),
                }
            }
            ExternEvent::Return => trace.push("return".to_owned()),
        }
    }
    Ok(trace)
}

fn prototype(callee: &ExternCallee) -> String {
    let parameters = if callee.parameters.is_empty() {
        "void".to_owned()
    } else {
        callee
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                let abi = match parameter {
                    CallParameter::Data(abi) | CallParameter::Borrowed(abi) => *abi,
                    CallParameter::Consumed => AbiType::PtrMutU8,
                };
                let spelling = abi.c_spelling();
                if spelling.ends_with('*') {
                    format!("{spelling}p{index}")
                } else {
                    format!("{spelling} p{index}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let result = callee.result_abi.c_spelling();
    if result.ends_with('*') {
        format!("{result}{}({parameters});", callee.name)
    } else {
        format!("{result} {}({parameters});", callee.name)
    }
}

fn argument_expression(argument: &ExternArgument) -> String {
    match argument {
        ExternArgument::Data(value) => format!("{value}"),
        ExternArgument::Borrow(resource) | ExternArgument::Consume(resource) => {
            resource_name(*resource)
        }
    }
}

/// Emit one checked module as a self-contained C11 translation unit whose
/// `main` performs the trace. The callees are declared, never defined: a build
/// must supply them.
pub fn emit_extern_c11(
    program: &ExternProgram,
    certificate: &ExternCallCertificate,
) -> Result<ExternCEmission, ExternEmissionRefusal> {
    if certificate.entry != program.entry {
        return Err(ExternEmissionRefusal::UncheckedProgram);
    }
    let mut source = String::new();
    source.push_str("/* Generated by qcore `source.extern-call.v1`. */\n");
    source.push_str("#include <stdint.h>\n#include <stdio.h>\n\n");
    for callee in program.callees.values() {
        let _ = writeln!(source, "{}", prototype(callee));
    }
    source.push_str("\nint main(void)\n{\n");
    for (index, event) in program.events.iter().enumerate() {
        match event {
            ExternEvent::Call(call) => {
                let callee = program.callees.get(&call.callee).ok_or_else(|| {
                    ExternEmissionRefusal::UnknownCallee {
                        event: index,
                        name: call.callee.clone(),
                    }
                })?;
                let arguments = call
                    .arguments
                    .iter()
                    .map(argument_expression)
                    .collect::<Vec<_>>()
                    .join(", ");
                match (&call.adapter, callee.result) {
                    (Some(adapter), CallResult::OwnedNullable) => {
                        let binding = resource_name(adapter.resource);
                        let _ = writeln!(
                            source,
                            "    uint8_t *{binding} = {}({arguments});",
                            callee.name
                        );
                        // The declared literal outcome control is checked
                        // against the real result rather than assumed.
                        let _ = writeln!(
                            source,
                            "    if (({binding} != NULL) != {}) {{\n        \
                             fprintf(stderr, \"declared outcome violated: {}\\n\");\n        \
                             return 70;\n    }}",
                            u8::from(adapter.succeeds),
                            callee.name
                        );
                        let _ = writeln!(
                            source,
                            "    printf(\"acquire {} {binding} %s\\n\", {binding} != NULL ? \"ok\" : \"null\");",
                            callee.name
                        );
                        let _ = writeln!(
                            source,
                            "    if ({binding} == NULL) {{\n        \
                             printf(\"raise {}\\n\");\n        return 0;\n    }}",
                            adapter.error
                        );
                    }
                    (None, CallResult::Data(_)) => {
                        if callee.result_abi == AbiType::Unit {
                            let _ = writeln!(source, "    {}({arguments});", callee.name);
                        } else {
                            let _ = writeln!(source, "    (void) {}({arguments});", callee.name);
                        }
                        let _ = writeln!(
                            source,
                            "    printf(\"{}\\n\");",
                            call_observation(callee, &call.arguments)
                        );
                    }
                    _ => return Err(ExternEmissionRefusal::UnsupportedResult { event: index }),
                }
            }
            ExternEvent::Return => source.push_str("    printf(\"return\\n\");\n"),
        }
    }
    source.push_str("    return 0;\n}\n");
    Ok(ExternCEmission { source })
}
