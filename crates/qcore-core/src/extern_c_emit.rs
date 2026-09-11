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
//!
//! Card `PIPE-SCALAR-RESULT` adds one statement shape and one argument
//! rendering, both in a namespace disjoint from the identities: a bound scalar
//! is declared at its own ABI spelling under a `v` name beside the `r` names of
//! resources.
//!
//! Card `EXTERN-SCALAR-OUTCOME` (trace half) then makes that binding's *number*
//! observable — but only where a `succeeds INTEGER` outcome is declared, and
//! only because of the check card `STD-IO-COMPOSE` already emits. The reference
//! interpreter still cannot compute a C function's result; what it can do is
//! print the **declared** number, and the emitted program compares the real
//! result against that same declared number and exits 70 before reaching the
//! observation write. So on every run that reaches the line, the declared
//! number *is* the returned number, and the two observations agree by the
//! run-time comparison rather than by assumption. A binding with no declared
//! outcome keeps card `PIPE-SCALAR-RESULT`'s line byte for byte: nothing
//! computes a number for it, so observing one would be publishing fiction.
use std::fmt::Write;

use crate::ResourceId;
use crate::c_program::{self, Fragment, Section};
use crate::extern_call::{
    AbiType, CallParameter, ExternArgument, ExternCallCertificate, ExternCallee, ExternEvent,
    ExternProgram, ValueId,
};

fn c_string(value: &str) -> String {
    let mut result = String::from("\"");
    for byte in value.bytes() {
        write!(result, "\\{byte:03o}").expect("writing to a String cannot fail");
    }
    result.push('"');
    result
}

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
    /// An argument names a scalar value no earlier call in this trace bound
    /// (card `PIPE-SCALAR-RESULT`). The mirror of [`Self::UnboundIdentity`] for
    /// the value namespace, and emission-only for the same reason.
    UnboundValue { event: usize, value: ValueId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternCEmission {
    pub source: String,
}

fn resource_name(resource: ResourceId) -> String {
    format!("r{resource}")
}

/// The C variable of a bound scalar (card `PIPE-SCALAR-RESULT`). A separate
/// prefix from [`resource_name`] on purpose: the two id spaces are disjoint but
/// numerically overlapping, and a reader of the emitted C or of an observation
/// must be able to tell a value from an identity by its name alone.
fn value_name(value: ValueId) -> String {
    format!("v{value}")
}

/// The same name, for a composing backend (card `STD-IO-COMPOSE`). A composed
/// rendering must read exactly the object this backend declared, so the naming
/// rule is published rather than reproduced.
#[must_use]
pub fn value_variable(value: ValueId) -> String {
    value_name(value)
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
        // `v` a bound scalar read as data. Distinct from `d:` — a literal —
        // because the two are the same *mode* and different provenances, and
        // an observation that could not tell them apart would hide which call
        // produced the number the next call received.
        ExternArgument::Value(value) => format!("v:{}", value_name(*value)),
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

/// A call whose scalar result is bound (card `PIPE-SCALAR-RESULT`), with the
/// declared outcome appended where the site declares one (card
/// `EXTERN-SCALAR-OUTCOME`).
///
/// The observation names the callee, the value the binding introduces, the
/// arguments and — only for a `succeeds INTEGER` binding — the number. The
/// trailing `-> N` field is spelled with an arrow rather than as a bare token,
/// so that the argument list stays exactly the space-separated `x:y` tokens it
/// was and a reader cannot mistake a number for a positional argument; the
/// arrow is this backend's own existing rendering of a call result, in the
/// `declared outcome violated: NAME -> N` message.
///
/// **Why printing the declared number is an observation and not a claim.** The
/// emitted program compares the real result against this same number and exits
/// 70 before it reaches the write, so control arrives at the line only on a run
/// where the two are equal. The reference interpreter prints the declared
/// number; the native run prints the declared number having just proved it is
/// the returned one. A binding with no declared outcome appends nothing — the
/// card `PIPE-SCALAR-RESULT` line, byte for byte — because nothing in either
/// model computes a number for it.
fn value_observation(
    callee: &ExternCallee,
    arguments: &[ExternArgument],
    value: ValueId,
    outcome: Option<i32>,
) -> String {
    format!(
        "value {} {}{}{}",
        callee.name,
        value_name(value),
        argument_lines(arguments, 0),
        outcome.map_or_else(String::new, |outcome| format!(" -> {outcome}"))
    )
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
    Ok(trace_extern_events(program, certificate)?
        .into_iter()
        .flat_map(|(_, lines)| lines)
        .collect())
}

/// The reference observation of each event, in trace order, paired with the
/// event index that produced it.
///
/// Split out of [`interpret_extern_program`] for the composed route (card
/// `STD-IO-COMPOSE`), which interleaves these lines with its own per-stream
/// records. A raised error still ends the trace: the events after it produce
/// nothing, exactly as they do in the flat observation and in the emitted C.
pub fn trace_extern_events(
    program: &ExternProgram,
    certificate: &ExternCallCertificate,
) -> Result<Vec<(usize, Vec<String>)>, ExternEmissionRefusal> {
    if certificate.entry != program.entry {
        return Err(ExternEmissionRefusal::UncheckedProgram);
    }
    let mut parts: Vec<(usize, Vec<String>)> = Vec::new();
    for (index, event) in program.events.iter().enumerate() {
        let mut trace = Vec::<String>::new();
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
                            parts.push((index, trace));
                            return Ok(parts);
                        }
                    }
                    (None, false) => match call.binding {
                        Some(binding) => {
                            trace.push(value_observation(
                                callee,
                                &call.arguments,
                                binding.value,
                                binding.outcome,
                            ));
                        }
                        None => trace.push(call_observation(callee, &call.arguments)),
                    },
                    _ => return Err(ExternEmissionRefusal::UnsupportedResult { event: index }),
                }
            }
            // Card `EXTERN-SCALAR-LITERAL`: a literal binding calls nothing and
            // is observed as nothing. The number is in the source and on the
            // certificate; printing it would add an observation no call made.
            ExternEvent::Bind(_) => {}
            ExternEvent::Return => trace.push("return".to_owned()),
        }
        parts.push((index, trace));
    }
    Ok(parts)
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
        ExternArgument::Value(value) => value_name(*value),
    }
}

/// The scalar value an argument reads, if any. The mirror of
/// [`argument_identity`] over the disjoint value namespace: every argument
/// answers exactly one of the two with `Some`, or neither.
const fn argument_value(argument: &ExternArgument) -> Option<ValueId> {
    match argument {
        ExternArgument::Value(value) => Some(*value),
        ExternArgument::Data(_)
        | ExternArgument::OutSlot
        | ExternArgument::Borrow(_)
        | ExternArgument::Consume(_)
        | ExternArgument::Store(_)
        | ExternArgument::Owner(_)
        | ExternArgument::Release(_)
        | ExternArgument::Realloc(_) => None,
    }
}

/// The caller identity an argument reads, if any. An out slot reads none: it
/// writes the identity the adapter binds.
const fn argument_identity(argument: &ExternArgument) -> Option<ResourceId> {
    match argument {
        ExternArgument::Data(_) | ExternArgument::OutSlot | ExternArgument::Value(_) => None,
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
    let mut body = String::new();
    for (_, statements) in emit_extern_statements(program, certificate)? {
        body.push_str(&statements);
    }
    Ok(ExternCEmission {
        source: c_program::assemble(&preamble(program), &c_program::Entry::source_route(body)),
    })
}

/// The file-scope text `source.extern-call.v1` contributes to a program it is
/// part of, whether alone or composed (card `C-EMIT-MAIN-UNIFY`).
///
/// The composed standard-IO route contributed exactly this list by hand before
/// this card, with its own banner in place of this one; both now name the
/// prototypes and the typed logger through the same keys, so a composition
/// declares each callee once.
#[must_use]
pub fn preamble(program: &ExternProgram) -> Vec<Fragment> {
    vec![
        Fragment::new(
            Section::Banner,
            "extern.banner",
            "/* Generated by qcore `source.extern-call.v1`. */\n",
        ),
        Fragment::new(
            Section::Includes,
            "extern.includes",
            "#include <stddef.h>\n#include <stdint.h>\n",
        ),
        crate::diagnostic_log_c_emit::diagnostic_log_fragment(),
        prototypes_fragment(program),
    ]
}

/// The declared prototypes as one file-scope fragment, with the blank line that
/// separates them from the prelude above.
#[must_use]
pub fn prototypes_fragment(program: &ExternProgram) -> Fragment {
    Fragment::new(
        Section::FileScope,
        "extern.prototypes",
        format!("\n{}", extern_prototypes(program)),
    )
}

/// The declared prototype of every admitted callee, one per line.
///
/// Split out of [`emit_extern_c11`] for the composed route (card
/// `STD-IO-COMPOSE`), which builds its own prologue around exactly these
/// declarations rather than reproducing them.
#[must_use]
pub fn extern_prototypes(program: &ExternProgram) -> String {
    let mut prototypes = String::new();
    for callee in program.callees.values() {
        let _ = writeln!(prototypes, "{}", prototype(callee));
    }
    prototypes
}

/// The C statements of each event of a checked trace, in trace order, paired
/// with the event index they came from.
///
/// This is the whole body of [`emit_extern_c11`]'s `main`, and the composed
/// route (card `STD-IO-COMPOSE`) interleaves these same strings with its
/// `print`/`log`/`flush` statements. Both routes therefore emit one call the
/// one way: there is no second lowering of an extern call site, so a composed
/// module's call statements are byte-identical to the extern route's by
/// construction rather than by comparison.
pub fn emit_extern_statements(
    program: &ExternProgram,
    certificate: &ExternCallCertificate,
) -> Result<Vec<(usize, String)>, ExternEmissionRefusal> {
    if certificate.entry != program.entry {
        return Err(ExternEmissionRefusal::UncheckedProgram);
    }
    let mut parts: Vec<(usize, String)> = Vec::new();
    // Every identity the emitted body may name is one an earlier adapter bound
    // to a C variable. Anything else is refused rather than printed.
    let mut bound = std::collections::BTreeSet::<ResourceId>::new();
    // The same rule over the disjoint value namespace (card
    // `PIPE-SCALAR-RESULT`): the backend declares a C variable per bound scalar
    // and refuses to print a name it never declared.
    let mut bound_values = std::collections::BTreeSet::<ValueId>::new();
    for (index, event) in program.events.iter().enumerate() {
        let mut source = String::new();
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
                    if let Some(value) = argument_value(argument)
                        && !bound_values.contains(&value)
                    {
                        return Err(ExternEmissionRefusal::UnboundValue {
                            event: index,
                            value,
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
                             qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_EXTERN_OUTCOME, {}, UINT64_C(0), INT64_C(-1), {});\n        \
                             return 70;\n    }}",
                            u8::from(adapter.succeeds), adapter.resource,
                            c_string(&format!("declared outcome violated: {}", callee.name))
                        );
                        let observed = acquisition_observation(
                            callee,
                            &call.arguments,
                            adapter.resource,
                            adapter.succeeds,
                        );
                        let _ = writeln!(
                            source,
                            "    qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_EXTERN_OUTCOME, {}, UINT64_C(0), {binding} != NULL ? INT64_C(1) : INT64_C(0), {});",
                            adapter.resource, c_string(&observed)
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
                                if let Some(value) = argument_value(argument)
                                    && !bound_values.contains(&value)
                                {
                                    return Err(ExternEmissionRefusal::UnboundValue {
                                        event: index,
                                        value,
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
                            let observed = call_observation(released, &release.arguments);
                            let _ = writeln!(arm, "        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_EXTERN_CALL, UINT64_C(0), UINT64_C(0), INT64_C(0), {});", c_string(&observed));
                        }
                        let _ = writeln!(
                            source,
                            "    if ({binding} == NULL) {{\n{arm}        \
                             qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_RUN_OUTCOME, {}, UINT64_C(0), INT64_C(1), {});\n        return 0;\n    }}",
                            adapter.resource, c_string(&format!("raise {}", adapter.error))
                        );
                        bound.insert(adapter.resource);
                    }
                    (None, false) => match call.binding {
                        // Card `PIPE-SCALAR-RESULT`. The result is declared at
                        // its own ABI spelling and immediately discarded to
                        // `void`, so the translation unit stays warning-clean
                        // under `-Wall -Wextra -Werror` whether or not a later
                        // call reads the value. The number is printed only
                        // where a declared outcome makes it well-defined, and
                        // only *after* the comparison below — see
                        // `value_observation`.
                        Some(binding) => {
                            let value = binding.value;
                            let name = value_name(value);
                            let _ = writeln!(
                                source,
                                "    {} {name} = {}({arguments});",
                                callee.result_abi.c_spelling(),
                                callee.name
                            );
                            let _ = writeln!(source, "    (void) {name};");
                            // Card `STD-IO-COMPOSE`. The emitted program does
                            // not trust a declared outcome any more than it
                            // trusts a declared `succeeds` arm: it compares the
                            // real result and exits 70 rather than going on to
                            // render a number the run did not produce.
                            //
                            // Card `EXTERN-SCALAR-OUTCOME` makes this
                            // comparison load-bearing for the observation as
                            // well as for the rendering, so the order of these
                            // two writes is not incidental: the check is
                            // emitted **before** the observation, and a
                            // disagreeing run therefore exits without writing
                            // the line at all.
                            if let Some(outcome) = binding.outcome {
                                let _ = writeln!(
                                    source,
                                    "    if ({name} != {outcome}) {{\n        \
                                     qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_EXTERN_OUTCOME, {}, UINT64_C(0), (int64_t){name}, {});\n        return 70;\n    }}",
                                    value, c_string(&format!("declared outcome violated: {}", callee.name))
                                );
                            }
                            let observed = value_observation(
                                callee,
                                &call.arguments,
                                value,
                                binding.outcome,
                            );
                            let _ = writeln!(
                                source,
                                "    qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_EXTERN_OUTCOME, {}, UINT64_C(0), (int64_t){name}, {});",
                                value, c_string(&observed)
                            );
                            bound_values.insert(value);
                        }
                        None => {
                            if callee.result_abi == AbiType::Unit {
                                let _ = writeln!(source, "    {}({arguments});", callee.name);
                            } else {
                                let _ =
                                    writeln!(source, "    (void) {}({arguments});", callee.name);
                            }
                            let observed = call_observation(callee, &call.arguments);
                            let _ = writeln!(source, "    qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_EXTERN_CALL, UINT64_C(0), UINT64_C(0), INT64_C(0), {});", c_string(&observed));
                        }
                    },
                    _ => return Err(ExternEmissionRefusal::UnsupportedResult { event: index }),
                }
            }
            // Card `EXTERN-SCALAR-LITERAL`. Declared at the fixed `int32_t`
            // spelling the value namespace's literals carry, and discarded to
            // `void` for the same `-Werror` reason a bound call result is.
            ExternEvent::Bind(literal) => {
                let name = value_name(literal.value);
                let _ = writeln!(source, "    int32_t {name} = INT32_C({});", literal.literal);
                let _ = writeln!(source, "    (void) {name};");
                bound_values.insert(literal.value);
            }
            ExternEvent::Return => source.push_str("    qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_RUN_OUTCOME, UINT64_C(0), UINT64_C(0), INT64_C(0), \"return\");\n"),
        }
        parts.push((index, source));
    }
    Ok(parts)
}
