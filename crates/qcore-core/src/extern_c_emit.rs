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
    AbiType, CallParameter, ExternArgument, ExternCallCertificate, ExternCallee, ExternEvent,
    ExternProgram,
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
    /// An argument names an identity no earlier adapter in this trace bound.
    /// The backend declares a C variable per adapter-bound identity and has
    /// nothing to name otherwise; emitting the name anyway would produce a
    /// translation unit that only the C compiler could reject. The reference
    /// interpreter has no such requirement — it prints identities symbolically
    /// — so this refusal is deliberately emission-only.
    UnboundIdentity { event: usize, resource: ResourceId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternCEmission {
    pub source: String,
}

fn resource_name(resource: ResourceId) -> String {
    format!("r{resource}")
}

/// One argument's rendering. The prefix names the *declared mode* the argument
/// bridged to, so an observation records which custody rule ran rather than
/// only which identity was passed.
fn argument_line(argument: &ExternArgument, out: ResourceId) -> String {
    match argument {
        ExternArgument::Data(value) => format!("d:{value}"),
        ExternArgument::Borrow(resource) => format!("b:{}", resource_name(*resource)),
        ExternArgument::Consume(resource) => format!("c:{}", resource_name(*resource)),
        ExternArgument::Store(resource) => format!("s:{}", resource_name(*resource)),
        ExternArgument::Owner(resource) => format!("w:{}", resource_name(*resource)),
        ExternArgument::OutSlot => format!("o:{}", resource_name(out)),
        // `f` freed (discharged by a declared `resource_released`), `g` the
        // subject a declared reallocation replaces. Both are distinct from
        // `c:`, so an observation still records which custody rule ran.
        ExternArgument::Release(resource) => format!("f:{}", resource_name(*resource)),
        ExternArgument::Realloc(resource) => format!("g:{}", resource_name(*resource)),
    }
}

fn argument_lines(arguments: &[ExternArgument], out: ResourceId) -> String {
    arguments
        .iter()
        .map(|argument| format!(" {}", argument_line(argument, out)))
        .collect()
}

fn call_observation(callee: &ExternCallee, arguments: &[ExternArgument]) -> String {
    format!("call {}{}", callee.name, argument_lines(arguments, 0))
}

/// A call that establishes fresh custody through a declared out slot, without
/// its outcome word. Both arms share this prefix, so the emitted `printf`
/// formats it once and appends the observed nullness.
fn out_prefix(callee: &ExternCallee, arguments: &[ExternArgument], resource: ResourceId) -> String {
    format!("out {}{}", callee.name, argument_lines(arguments, resource))
}

/// A declared reallocation, without its outcome word. The subject appears among
/// the arguments as `g:`, so the observation records both identities and which
/// arm ran — the two facts a reader needs to tell a replacement from a
/// preserved subject.
fn realloc_prefix(
    callee: &ExternCallee,
    arguments: &[ExternArgument],
    resource: ResourceId,
) -> String {
    format!(
        "realloc {} {}{}",
        callee.name,
        resource_name(resource),
        argument_lines(arguments, resource)
    )
}

/// The prefix of whichever acquisition source this callee declares. An owned
/// return names only the identity it binds; an out slot and a reallocation both
/// name their arguments, because in both the identity's provenance is one of
/// them.
fn acquisition_prefix(
    callee: &ExternCallee,
    arguments: &[ExternArgument],
    resource: ResourceId,
) -> String {
    if callee.out_parameter.is_some() {
        out_prefix(callee, arguments, resource)
    } else if callee.realloc_parameter.is_some() {
        realloc_prefix(callee, arguments, resource)
    } else {
        format!("acquire {} {}", callee.name, resource_name(resource))
    }
}

fn acquisition_observation(
    callee: &ExternCallee,
    arguments: &[ExternArgument],
    resource: ResourceId,
    succeeds: bool,
) -> String {
    format!(
        "{} {}",
        acquisition_prefix(callee, arguments, resource),
        if succeeds { "ok" } else { "null" }
    )
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
                match (&call.adapter, callee.acquires()) {
                    (Some(adapter), true) => {
                        trace.push(acquisition_observation(
                            callee,
                            &call.arguments,
                            adapter.resource,
                            adapter.succeeds,
                        ));
                        if !adapter.succeeds {
                            // The error arm's dispositions are real calls and
                            // are observed exactly as plain calls are, in the
                            // order the checked handler names them, before the
                            // raise ends the trace.
                            for release in &adapter.releases {
                                let released =
                                    program.callees.get(&release.callee).ok_or_else(|| {
                                        ExternEmissionRefusal::UnknownCallee {
                                            event: index,
                                            name: release.callee.clone(),
                                        }
                                    })?;
                                trace.push(call_observation(released, &release.arguments));
                            }
                            trace.push(format!("raise {}", adapter.error));
                            return Ok(trace);
                        }
                    }
                    (None, false) => {
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
                    CallParameter::Consumed
                    | CallParameter::Stored { .. }
                    | CallParameter::Owner
                    | CallParameter::Released
                    | CallParameter::ReallocSubject => AbiType::PtrMutU8,
                    CallParameter::OutSlot => AbiType::PtrMutPtrMutU8,
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

/// The C expression for one argument. `out` is the identity the adapter binds,
/// which is the only identity an out slot can name.
fn argument_expression(argument: &ExternArgument, out: ResourceId) -> String {
    match argument {
        ExternArgument::Data(value) => format!("{value}"),
        ExternArgument::Borrow(resource)
        | ExternArgument::Consume(resource)
        | ExternArgument::Store(resource)
        | ExternArgument::Owner(resource)
        | ExternArgument::Release(resource)
        | ExternArgument::Realloc(resource) => resource_name(*resource),
        ExternArgument::OutSlot => format!("&{}", resource_name(out)),
    }
}

/// The caller identity an argument reads, if any. An out slot reads none: it
/// writes the identity the adapter binds.
const fn argument_identity(argument: &ExternArgument) -> Option<ResourceId> {
    match argument {
        ExternArgument::Data(_) | ExternArgument::OutSlot => None,
        ExternArgument::Borrow(resource)
        | ExternArgument::Consume(resource)
        | ExternArgument::Store(resource)
        | ExternArgument::Owner(resource)
        | ExternArgument::Release(resource)
        | ExternArgument::Realloc(resource) => Some(*resource),
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
    // Every identity the emitted body may name is one an earlier adapter bound
    // to a C variable. Anything else is refused rather than printed.
    let mut bound = std::collections::BTreeSet::<ResourceId>::new();
    for (index, event) in program.events.iter().enumerate() {
        match event {
            ExternEvent::Call(call) => {
                let callee = program.callees.get(&call.callee).ok_or_else(|| {
                    ExternEmissionRefusal::UnknownCallee {
                        event: index,
                        name: call.callee.clone(),
                    }
                })?;
                for argument in &call.arguments {
                    if let Some(resource) = argument_identity(argument)
                        && !bound.contains(&resource)
                    {
                        return Err(ExternEmissionRefusal::UnboundIdentity {
                            event: index,
                            resource,
                        });
                    }
                }
                let out = call.adapter.as_ref().map_or(0, |adapter| adapter.resource);
                let arguments = call
                    .arguments
                    .iter()
                    .map(|argument| argument_expression(argument, out))
                    .collect::<Vec<_>>()
                    .join(", ");
                match (&call.adapter, callee.acquires()) {
                    (Some(adapter), true) => {
                        let binding = resource_name(adapter.resource);
                        if callee.out_parameter.is_some() {
                            // The slot starts null, so a callee that does not
                            // write it is observed as the null arm and caught
                            // by the declared-outcome check below rather than
                            // silently believed.
                            let _ = writeln!(source, "    uint8_t *{binding} = NULL;");
                            if callee.result_abi == AbiType::Unit {
                                let _ = writeln!(source, "    {}({arguments});", callee.name);
                            } else {
                                let _ =
                                    writeln!(source, "    (void) {}({arguments});", callee.name);
                            }
                        } else {
                            let _ = writeln!(
                                source,
                                "    uint8_t *{binding} = {}({arguments});",
                                callee.name
                            );
                        }
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
                        let observed =
                            acquisition_prefix(callee, &call.arguments, adapter.resource);
                        let _ = writeln!(
                            source,
                            "    printf(\"{observed} %s\\n\", {binding} != NULL ? \"ok\" : \"null\");"
                        );
                        // The error arm renders this acquisition's cleanup edge:
                        // the checked dispositions are emitted as real calls
                        // inside the failure branch, so a leak checker sees the
                        // releases the checker required rather than taking them
                        // on trust.
                        let mut arm = String::new();
                        for release in &adapter.releases {
                            let released =
                                program.callees.get(&release.callee).ok_or_else(|| {
                                    ExternEmissionRefusal::UnknownCallee {
                                        event: index,
                                        name: release.callee.clone(),
                                    }
                                })?;
                            for argument in &release.arguments {
                                if let Some(resource) = argument_identity(argument)
                                    && !bound.contains(&resource)
                                {
                                    return Err(ExternEmissionRefusal::UnboundIdentity {
                                        event: index,
                                        resource,
                                    });
                                }
                            }
                            let released_arguments = release
                                .arguments
                                .iter()
                                .map(|argument| argument_expression(argument, 0))
                                .collect::<Vec<_>>()
                                .join(", ");
                            if released.result_abi == AbiType::Unit {
                                let _ = writeln!(
                                    arm,
                                    "        {}({released_arguments});",
                                    released.name
                                );
                            } else {
                                let _ = writeln!(
                                    arm,
                                    "        (void) {}({released_arguments});",
                                    released.name
                                );
                            }
                            let _ = writeln!(
                                arm,
                                "        printf(\"{}\\n\");",
                                call_observation(released, &release.arguments)
                            );
                        }
                        let _ = writeln!(
                            source,
                            "    if ({binding} == NULL) {{\n{arm}        \
                             printf(\"raise {}\\n\");\n        return 0;\n    }}",
                            adapter.error
                        );
                        bound.insert(adapter.resource);
                    }
                    (None, false) => {
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
