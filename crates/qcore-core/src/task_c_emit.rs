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

//! Native C11 threads for checked finite copy/owned-buffer task modules.
//! Start records are private until fully initialized, retained through native
//! join, and never reclaimed after an unexpected join failure. Worker code has
//! no source-shared memory and writes only its own completion record.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use crate::c_program::{self, Fragment, Section};
use crate::diagnostic_log_c_emit;
use crate::io_emit;
use crate::scalar_calls_c_emit::{Helper, scalar_helper_definition};
use crate::scalar_types::{self as st, ArithmeticError};
use crate::task_execution::{
    CheckedTaskModule, ErrorOperand, ScalarOperand, ScalarType,
    TaskInstructionKind as InstructionKind, TaskSpan, TaskTerminatorKind as TerminatorKind,
    TaskValueOperand, TaskValueOperation,
};

/// The copy-scalar slot types of the function being emitted (card
/// `W48-QR-SCALAR-TYPES-R4`), read from the checker's own record.
type SlotTypes = BTreeMap<usize, ScalarType>;

/// The member of the fixed-width task value's union that holds a value of
/// `ty` (card `W48-QR-SCALAR-TYPES-R4`). `bool` is a macro in `<stdbool.h>`,
/// so every member carries the `v_` prefix.
fn member(ty: st::ScalarType) -> String {
    format!("v_{ty}")
}

/// The C lvalue of a copy-scalar slot: an `i32` lives in the task value's
/// `scalar` field exactly as before, every other type in its union member.
fn slot_expression(slot: usize, types: &SlotTypes) -> String {
    match types.get(&slot) {
        Some(ScalarType::Fixed(ty)) => format!("s_{slot}.value.typed.{}", member(*ty)),
        _ => format!("s_{slot}.value.scalar"),
    }
}

/// The C expression of a value-statement operand.
fn value_operand(operand: &TaskValueOperand, types: &SlotTypes) -> String {
    match operand {
        TaskValueOperand::Slot(slot) => slot_expression(*slot, types),
        TaskValueOperand::Literal { ty, value } => st::c_literal(*ty, *value),
    }
}

/// The type of a value-statement operand.
fn value_operand_type(operand: &TaskValueOperand, types: &SlotTypes) -> st::ScalarType {
    match operand {
        TaskValueOperand::Literal { ty, .. } => *ty,
        TaskValueOperand::Slot(slot) => match types.get(slot) {
            Some(ScalarType::Fixed(ty)) => *ty,
            _ => st::ScalarType::I32,
        },
    }
}

/// The operation's own (operand) type.
fn operation_type(operation: &TaskValueOperation, types: &SlotTypes) -> st::ScalarType {
    match operation {
        TaskValueOperation::Binary(_, a, _)
        | TaskValueOperation::Compare(_, a, _)
        | TaskValueOperation::Negate(a)
        | TaskValueOperation::Convert(a, _) => value_operand_type(a, types),
    }
}

/// The shared checked helper a value statement calls, if any: the exiting
/// form (`q_scalar_OP_T`, a witness the checker proved never fires) when the
/// statement cannot raise, the tag-returning form (`q_scalar_try_OP_T`) when
/// it can. A comparison and a total conversion call none.
fn operation_helper(
    operation: &TaskValueOperation,
    types: &SlotTypes,
    raises: bool,
    target: Option<&st::RecordedTarget>,
) -> Option<Helper> {
    let ty = operation_type(operation, types);
    match operation {
        TaskValueOperation::Binary(op, ..) => Some(if raises {
            Helper::TryOp(*op, ty)
        } else {
            Helper::Op(*op, ty)
        }),
        TaskValueOperation::Negate(_) => Some(if raises {
            Helper::TryNeg(ty)
        } else {
            Helper::Neg(ty)
        }),
        TaskValueOperation::Convert(_, to) => {
            if st::conversion_is_total(ty, *to, target).unwrap_or(false) {
                None
            } else if raises {
                Some(Helper::TryConvert(ty, *to))
            } else {
                Some(Helper::Convert(ty, *to))
            }
        }
        // Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-INLINE-COMPARISON-WERROR`): a
        // comparison is a two-parameter function, so `x == x` compiles under
        // `-Werror`.
        TaskValueOperation::Compare(cmp, ..) => Some(Helper::Compare(*cmp, ty)),
    }
}

/// The C expression of a value statement that cannot raise.
fn value_expression(
    operation: &TaskValueOperation,
    types: &SlotTypes,
    target: Option<&st::RecordedTarget>,
) -> String {
    let operands = operation
        .operands()
        .iter()
        .map(|operand| value_operand(operand, types))
        .collect::<Vec<_>>();
    match (operation_helper(operation, types, false, target), operation) {
        (Some(helper), _) => format!("{}({})", helper.name(), operands.join(", ")),
        (None, TaskValueOperation::Convert(_, to)) => {
            format!("({})({})", st::c_type(*to), operands[0])
        }
        (None, _) => unreachable!("an arithmetic statement always calls a helper"),
    }
}

/// The `result.kind` of a worker result of fixed-width type `ty`: `10` plus
/// its position in the §G02t table, clear of the historical `0`/`1`/`2`; and
/// (card `W48-QR-FLOATING-POINT`) `f32`/`f64` after the integer table, `21`
/// and `22`.
fn fixed_kind(ty: st::ScalarType) -> usize {
    10 + st::ALL
        .iter()
        .chain(st::FLOATS.iter())
        .position(|candidate| *candidate == ty)
        .unwrap_or(0)
}

/// The logger arm of one float result (card `W48-QR-FLOATING-POINT`): the
/// value's bit pattern, copied with `memcpy` into the event's unsigned
/// `related`, under the name `=F32`/`=F64` — never a conversion to an
/// integer, which would be undefined for a NaN or an out-of-range value.
fn float_result_log(ty: st::ScalarType, field: &str, indent: &str) -> String {
    let label = ty.name().to_ascii_uppercase();
    let (bits, c) = if ty == st::ScalarType::F32 {
        ("uint32_t", "float")
    } else {
        ("uint64_t", "double")
    };
    format!(
        "{{\n{indent}    {c} q_value = {field}; {bits} q_bits = 0; memcpy(&q_bits, &q_value, sizeof q_bits);\n{indent}    qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_RESULT, identity,\n{indent}        (uint64_t)q_bits, INT64_C(0), \"={label}\");\n{indent}}}"
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskCMapping {
    pub generated_start_line: usize,
    pub generated_end_line: usize,
    pub span: TaskSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskCEmission {
    pub source: String,
    pub source_map: Vec<TaskCMapping>,
}

fn scalar(value: &ScalarOperand) -> String {
    match value {
        ScalarOperand::Unit => "0".into(),
        // Reached only at a position the checker gave a fixed-width type.
        ScalarOperand::Fixed { ty, value } => st::c_literal(*ty, *value),
        ScalarOperand::I32(i32::MIN) => "(-INT32_C(2147483647) - INT32_C(1))".into(),
        ScalarOperand::I32(value) if *value < 0 => format!("(-INT32_C({}))", -value),
        ScalarOperand::I32(value) => format!("INT32_C({value})"),
        ScalarOperand::Slot(slot) => format!("s_{slot}.value.scalar"),
        ScalarOperand::Move(_)
        | ScalarOperand::Loan { .. }
        | ScalarOperand::Lease { .. }
        | ScalarOperand::Record(_) => {
            unreachable!("checked scalar operand is neither owned nor lent nor a record")
        }
    }
}

/// The `result.kind` of a worker result of record type `record` (card
/// `W48-QR-TASK-RECORD-CAPTURES`): `100` plus the record's key, clear of the
/// historical `0`/`1`/`2` and of the fixed-width `10`..`20`.
fn record_kind(record: usize) -> usize {
    100 + record
}

/// The C expression of a copy-scalar operand at its checked type (card
/// `W48-QR-SCALAR-TYPES-R6`): a slot of a fixed-width type is its union
/// member, every other operand what [`scalar`] spells.
fn typed_operand(operand: &ScalarOperand, types: &SlotTypes) -> String {
    match operand {
        ScalarOperand::Slot(slot) => slot_expression(*slot, types),
        other => scalar(other),
    }
}

/// A move, a loan and a lease all hand the callee the same pointer value —
/// which is what makes two borrowers of one allocation two readers of one
/// object rather than two copies, and two lease holders two writers-in-waiting
/// of one object. They differ only in what the *caller* keeps, which
/// `consume` decides.
fn self_value(operand: &ScalarOperand, types: &SlotTypes, tail: &str) -> String {
    match operand {
        // Card `W48-QR-SCALAR-TYPES-R4`: a fixed-width value travels in its
        // union member; an `i32` keeps the historical spelling below.
        ScalarOperand::Fixed { ty, value } => format!(
            "(struct qcore_task_value){{.typed.{} = {}}}",
            member(*ty),
            st::c_literal(*ty, *value)
        ),
        ScalarOperand::Slot(slot) if matches!(types.get(slot), Some(ScalarType::Fixed(_))) => {
            format!("s_{slot}.value")
        }
        // A lease additionally hands the worker its **base**, in the task
        // value's scalar field — which a lease slot has no other use for,
        // since no scalar operation admits one. Without it a lease-relative
        // byte index would have nothing to count from inside the worker, and
        // the base is exactly the fact the holder must not be able to choose
        // (card `WRITE-LEND-TASKS-EXERCISE`).
        ScalarOperand::Lease { slot, low, .. } => {
            format!("(struct qcore_task_value){{INT32_C({low}), s_{slot}.value.buffer{tail}}}")
        }
        ScalarOperand::Move(slot) | ScalarOperand::Loan { slot, .. } => {
            format!("s_{slot}.value")
        }
        _ => format!(
            "(struct qcore_task_value){{{}, NULL{tail}}}",
            scalar(operand)
        ),
    }
}

/// A byte index reaches emission only as the check-time constant the checker
/// demanded, so there is no run-time index expression to emit. An operand of
/// any other shape never passes `check_task_module`, and reaching one here is
/// a defect in this crate rather than in the program.
fn byte_index(operand: &ScalarOperand) -> u64 {
    match operand {
        ScalarOperand::I32(value) if *value >= 0 => *value as u64,
        _ => unreachable!("checked byte index is a non-negative constant"),
    }
}

fn consume(source: &mut String, operand: &ScalarOperand) {
    match operand {
        ScalarOperand::Move(slot) => {
            writeln!(source, "    s_{slot}.value.buffer = NULL;").unwrap();
        }
        // Card `W48-QR-TASK-RECORD-CAPTURES`: each owned leaf of a returned
        // record leaves its binding exactly as a moved Buffer does.
        ScalarOperand::Record(leaves) => {
            for leaf in leaves {
                consume(source, leaf);
            }
        }
        _ => {}
    }
}

fn c_string(value: &str) -> String {
    let mut result = String::from("\"");
    for byte in value.bytes() {
        // Fixed-width octal escapes cannot absorb the next byte.
        write!(result, "\\{byte:03o}").unwrap();
    }
    result.push('"');
    result
}

pub fn emit_task_c11(checked: &CheckedTaskModule) -> TaskCEmission {
    emit_task_c11_with_source_lines(checked, &[])
}

/// The C statements of the source lines standing immediately before position
/// `index` of `block` of `function` (cards `COMPOSE-CONCURRENCY` and
/// `OBS-REORDERING`).
///
/// The rendering itself — the conversion, the level tag, the stream, the
/// `fflush` — is `std_io_c_emit::event_statement`'s, which is
/// `source.std-io.v1`'s own and which the extern composition already shares. The
/// only thing decided here is the *expression*: a composed rendering reads
/// `s_N.value.scalar`, the object this backend already declared for that slot,
/// never a fresh copy.
///
/// A site in a **worker** function emits inside that worker's own body, so the
/// write is performed by that worker's thread. That is what card
/// `OBS-REORDERING` admits, and it is why such a program's streams are compared
/// up to the declared reordering relation rather than byte for byte. A whole
/// line still arrives whole: one `QCORE_IO_WRITE` is one `fprintf`, performed
/// under the stream's own C11 lock (premise `C11StreamLock`).
fn source_lines(
    sites: &[crate::composed_tasks::TaskIoSite],
    function: usize,
    block: usize,
    index: usize,
    types: &SlotTypes,
    target: Option<&st::RecordedTarget>,
) -> Vec<String> {
    let mut statements = Vec::new();
    for site in sites
        .iter()
        .filter(|site| site.function == function && site.block == block && site.index == index)
    {
        // Card `W48-QR-SCALAR-TYPES-R4`: a fixed-width operand renders at its
        // type — §G02t's conversion over the slot's union member, framed like
        // every line (the composed extern route's precedent).
        if let crate::std_io::StdIoEvent::Print {
            rendering,
            operand: crate::std_io::StdIoOperand::Scalar(_),
        }
        | crate::std_io::StdIoEvent::Log {
            rendering,
            operand: crate::std_io::StdIoOperand::Scalar(_),
            ..
        } = &site.event
            && let Some(slot) = site.operand
            && let Some(ScalarType::Fixed(ty)) = types.get(&slot)
        {
            let level = match &site.event {
                crate::std_io::StdIoEvent::Log { level, .. } => Some(*level),
                _ => None,
            };
            let (stream, prefix) = crate::std_io::line_framing(level);
            // A checked program has a rendering here; the silent case is the
            // same fail-closed backstop as below.
            if let Some((conversion, operand)) = st::c_rendering(
                *rendering == crate::std_io::Rendering::Hex,
                *ty,
                &slot_expression(slot, types),
                target,
            ) {
                statements.push(io_emit::write_line(
                    stream,
                    &format!("{prefix}{conversion}"),
                    &[&operand],
                ));
            }
            continue;
        }
        let expressions = site
            .operand
            .map(|slot| {
                let name = match &site.event {
                    crate::std_io::StdIoEvent::Print { operand, .. }
                    | crate::std_io::StdIoEvent::Log { operand, .. } => match operand {
                        crate::std_io::StdIoOperand::Scalar(name) => name.clone(),
                        _ => String::new(),
                    },
                    _ => String::new(),
                };
                BTreeMap::from([(name, format!("s_{slot}.value.scalar"))])
            })
            .unwrap_or_default();
        // The two silent cases are unreachable on a checked program: `Ok(None)`
        // is the entry's return, which is never a placed event, and `Err` is a
        // name the checker already refused. Emitting nothing for them is the
        // fail-closed backstop — never an `unwrap` and never a guessed line.
        if let Ok(Some(statement)) =
            crate::std_io_c_emit::event_statement(&site.event, &expressions, &BTreeMap::new())
        {
            statements.push(statement);
        }
    }
    statements
}

/// Emit a checked task module, optionally with `source.std-io.v1` lines placed
/// in its entry's CFG (card `COMPOSE-CONCURRENCY`).
///
/// The **empty** site list is [`emit_task_c11`] above, so a task-route module
/// travels this identical code path and its emitted C is byte-identical: there
/// is no second backend to drift.
pub fn emit_task_c11_with_source_lines(
    checked: &CheckedTaskModule,
    sites: &[crate::composed_tasks::TaskIoSite],
) -> TaskCEmission {
    let module = checked.module();
    let errors: BTreeMap<_, _> = module
        .functions
        .values()
        .flat_map(|f| f.errors.iter().cloned())
        .chain(["SpawnError".to_owned(), "AllocError".to_owned()])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .enumerate()
        .map(|(index, name)| (name, index + 1))
        .collect();
    let capacity = module
        .functions
        .values()
        .map(|f| f.parameters.len())
        .max()
        .unwrap_or(0)
        .max(1);
    let event_capacity = module
        .functions
        .values()
        .map(|f| {
            f.blocks
                .values()
                .map(|b| b.instructions.len() + 1)
                .sum::<usize>()
        })
        .max()
        .unwrap_or(1)
        .max(1);
    let log_level = sites.iter().find_map(|site| match site.event {
        crate::std_io::StdIoEvent::LogLevel { level } => Some(level),
        _ => None,
    });
    let mut fragments = preamble(capacity, event_capacity, log_level, !sites.is_empty());
    // Card `W48-QR-SCALAR-TYPES-R4`: a module using the fixed-width surface
    // gets the typed runtime; every other module the historical bytes.
    let fixed = crate::task_execution::fixed_types(module);
    let fixed_width = crate::task_execution::uses_fixed_width(module);
    // Card `W48-QR-FLOATING-POINT`: a module holding a float value.
    let floats = fixed.iter().any(|ty| ty.is_float());
    // A module whose task value carries the fixed-width union initializes it
    // explicitly in every positional initializer (`-Wmissing-field-initializers`);
    // every other module keeps its historical bytes.
    let tail = if fixed.is_empty() { "" } else { ", {0}" };
    let target = checked.target_assumption().or(module.target.as_ref());
    let fixed_results = module
        .functions
        .values()
        .filter_map(|function| match function.result {
            ScalarType::Fixed(ty) => Some(ty),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    // Card `W48-QR-SCALAR-TYPES-R6`: every payload type other than `i32` a
    // Buffer of this module has. Such a type is in `fixed` too, so a module
    // with a typed payload always takes the fixed-width runtime.
    let payloads = checked
        .payload_types()
        .into_iter()
        .filter(|payload| *payload != st::ScalarType::I32)
        .collect::<BTreeSet<_>>();
    // Card `W48-QR-TASK-RECORD-CAPTURES`: the record types this module's
    // workers return. A module returning none keeps every byte it had.
    let record_results = module
        .functions
        .values()
        .filter_map(|function| match function.result {
            ScalarType::Record(record) => module
                .records
                .get(&record)
                .map(|declared| (record, declared)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    // Every positional result initializer of a module whose result carries
    // record leaves initializes them too (`-Wmissing-field-initializers`).
    let result_tail = if record_results.is_empty() {
        ""
    } else {
        ", {{0}}"
    };
    // A record result logs its header under `=RECORD`, which only the
    // fixed-width result arm of the stdout adapter prints.
    let fixed_arm = !fixed_results.is_empty() || !record_results.is_empty();
    let mut helpers = BTreeSet::new();
    for fragment in &mut fragments {
        if fixed_width && fragment.key == "task.runtime" {
            fragment.text = fixed_width_runtime(&fixed, &fixed_results, &payloads);
        }
        if !record_results.is_empty() && fragment.key == "task.runtime" {
            fragment.text = record_runtime(&fragment.text, &record_results);
        }
        // A typed payload implies the fixed-width surface, and so does a
        // fixed-width result, so without a record result this is exactly the
        // replacement that surface always made.
        if fragment.key == diagnostic_log_c_emit::ADAPTER_DEFINITION_KEY
            && (fixed_arm || !payloads.is_empty())
        {
            fragment.text = diagnostic_log_c_emit::diagnostic_stdout_adapter_definition_for(
                fixed_arm,
                !payloads.is_empty(),
            );
            // Card `W48-QR-FLOATING-POINT`: a float result's arm, only in a
            // program that can log one.
            if floats && fixed_arm {
                fragment.text = diagnostic_log_c_emit::with_float_results(&fragment.text);
            }
            // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: a float payload's arm, only
            // in a program holding one.
            if payloads.iter().any(|payload| payload.is_float()) {
                fragment.text = diagnostic_log_c_emit::with_float_payloads(&fragment.text);
            }
        }
    }

    // The runtime's own helpers and this module's function bodies, built into
    // one file-scope fragment below.
    let mut source = String::new();
    source.push_str("static const char *qcore_task_error_name(int tag) {\n    switch (tag) {\n");
    for (name, tag) in &errors {
        writeln!(source, "    case {tag}: return {};", c_string(name)).unwrap();
    }
    source.push_str("    default: abort();\n    }\n}\n");
    for id in module.functions.keys() {
        writeln!(source, "static struct qcore_task_result qcore_task_fn_{id}(const struct qcore_task_value *, struct qcore_task_context *);").unwrap();
    }
    let mut source_map = Vec::new();
    for (id, function) in &module.functions {
        let start_line = source.lines().count() + 1;
        writeln!(source, "static struct qcore_task_result qcore_task_fn_{id}(const struct qcore_task_value *args, struct qcore_task_context *context) {{").unwrap();
        source.push_str("    (void)args; (void)context;\n");
        let types: SlotTypes = checked.function_slot_types(*id);
        // Card `W48-QR-SCALAR-TYPES-R6`: the payload type of a Buffer, loan
        // or lease slot when it is not `i32`, which selects the payload
        // type's own runtime helpers.
        let typed_payload = |slot: usize| {
            checked
                .buffer_payload(*id, slot)
                .filter(|payload| *payload != st::ScalarType::I32)
        };
        let typed_suffix = |slot: usize| {
            typed_payload(slot).map_or_else(String::new, |payload| format!("_{payload}"))
        };
        let mut slots = function
            .parameters
            .iter()
            .map(|(slot, _)| *slot)
            .collect::<BTreeSet<_>>();
        for block in function.blocks.values() {
            for instruction in &block.instructions {
                match &instruction.kind {
                    InstructionKind::ReadBuffer {
                        value_slot: slot, ..
                    }
                    | InstructionKind::BindScalar { slot, .. }
                    | InstructionKind::Compute { slot, .. } => {
                        slots.insert(*slot);
                    }
                    InstructionKind::UnwrapInfallible { value_slot, .. }
                    | InstructionKind::ReadBufferByte { value_slot, .. } => {
                        slots.insert(*value_slot);
                    }
                    InstructionKind::TakeErrorPayload { buffer_slot, .. } => {
                        slots.insert(*buffer_slot);
                    }
                    InstructionKind::Join {
                        outcome_slot: Some(slot),
                        ..
                    } => {
                        slots.insert(*slot);
                    }
                    // Card `W48-QR-TASK-RECORD-CAPTURES`.
                    InstructionKind::UnpackRecord { leaves, .. } => {
                        slots.extend(leaves.iter().copied());
                    }
                    InstructionKind::RefillField { slot, .. } => {
                        slots.insert(*slot);
                    }
                    _ => {}
                }
            }
            match &block.terminator.kind {
                TerminatorKind::TryAllocateBuffer {
                    value_slot: handle_slot,
                    ..
                }
                | TerminatorKind::TrySpawn { handle_slot, .. } => {
                    slots.insert(*handle_slot);
                }
                TerminatorKind::TryOutcome {
                    value_slot,
                    error_slot,
                    ..
                }
                | TerminatorKind::TryCompute {
                    slot: value_slot,
                    error_slot,
                    ..
                } => {
                    slots.insert(*value_slot);
                    slots.insert(*error_slot);
                }
                _ => {}
            }
        }
        for slot in slots {
            writeln!(
                source,
                "    struct qcore_task_slot s_{slot} = {{0}}; (void)&s_{slot};"
            )
            .unwrap();
        }
        for (index, (slot, _)) in function.parameters.iter().enumerate() {
            writeln!(source, "    s_{slot}.value = args[{index}];").unwrap();
        }
        // A byte access counts from its authority's base: a lease's own `low`,
        // which the creator wrote into the task value's scalar field, or 0 for
        // an owner. A lease is only ever a parameter — no operand rebinds one,
        // and there is no transitive leasing — so this set is exactly the
        // lease bindings this function has.
        let lease_slots = function
            .parameters
            .iter()
            .filter(|(_, kind)| matches!(kind, ScalarType::WriteLease(_)))
            .map(|(slot, _)| *slot)
            .collect::<BTreeSet<_>>();
        let base = |slot: &usize| {
            if lease_slots.contains(slot) {
                format!("(uint64_t)(uint32_t)s_{slot}.value.scalar")
            } else {
                "UINT64_C(0)".to_string()
            }
        };
        // Slots are single-assignment, so a handle slot names exactly one
        // creation and therefore exactly the loans that creation opened.
        let loans_by_handle = function
            .blocks
            .values()
            .filter_map(|block| match &block.terminator.kind {
                TerminatorKind::TrySpawn {
                    arguments,
                    handle_slot,
                    ..
                } => Some((
                    *handle_slot,
                    arguments
                        .iter()
                        .filter_map(|argument| match argument {
                            ScalarOperand::Loan { slot, .. } => Some(*slot),
                            _ => None,
                        })
                        .collect::<Vec<_>>(),
                )),
                _ => None,
            })
            .collect::<BTreeMap<_, _>>();
        // The write-side mirror (card `WRITE-LEND-TASKS`): the leases each
        // creation opened, so the join can announce their return.
        let leases_by_handle = function
            .blocks
            .values()
            .filter_map(|block| match &block.terminator.kind {
                TerminatorKind::TrySpawn {
                    arguments,
                    handle_slot,
                    ..
                } => Some((
                    *handle_slot,
                    arguments
                        .iter()
                        .filter_map(|argument| match argument {
                            ScalarOperand::Lease { slot, .. } => Some(*slot),
                            _ => None,
                        })
                        .collect::<Vec<_>>(),
                )),
                _ => None,
            })
            .collect::<BTreeMap<_, _>>();
        // Card `W48-QR-TASK-RECORD-CAPTURES`: the Result slot each collected
        // record was bound out of. Slots are single-assignment, so the value
        // slot names exactly one collection, whose leaves are still in that
        // Result's `leaves` when the unpack that follows it runs.
        let record_sources = function
            .blocks
            .values()
            .flat_map(|block| {
                block
                    .instructions
                    .iter()
                    .filter_map(|instruction| match &instruction.kind {
                        InstructionKind::UnwrapInfallible {
                            result_slot,
                            value_slot,
                        } => Some((*value_slot, *result_slot)),
                        _ => None,
                    })
                    .chain(match &block.terminator.kind {
                        TerminatorKind::TryOutcome {
                            result_slot,
                            value_slot,
                            ..
                        } => Some((*value_slot, *result_slot)),
                        _ => None,
                    })
            })
            .collect::<BTreeMap<_, _>>();
        writeln!(source, "    goto qcore_block_{};", function.entry).unwrap();
        for (block_id, block) in &function.blocks {
            writeln!(source, "qcore_block_{block_id}: ;").unwrap();
            for (index, instruction) in block.instructions.iter().enumerate() {
                for statement in source_lines(sites, *id, *block_id, index, &types, target) {
                    writeln!(source, "    {statement}").unwrap();
                }
                let start = source.lines().count() + 1;
                match &instruction.kind {
                    InstructionKind::BindScalar { slot, value } => {
                        writeln!(
                            source,
                            "    s_{slot}.value = {};",
                            self_value(value, &types, tail)
                        )
                        .unwrap();
                        consume(&mut source, value);
                    }
                    InstructionKind::Compute { slot, operation } => {
                        if let Some(helper) = operation_helper(operation, &types, false, target) {
                            helpers.insert(helper);
                        }
                        writeln!(
                            source,
                            "    {} = {};",
                            slot_expression(*slot, &types),
                            value_expression(operation, &types, target)
                        )
                        .unwrap();
                    }
                    InstructionKind::UnwrapInfallible {
                        result_slot,
                        value_slot,
                    } => {
                        writeln!(
                            source,
                            "    s_{value_slot}.value = s_{result_slot}.result.value; s_{result_slot}.result.value.buffer = NULL;"
                        )
                        .unwrap();
                    }
                    InstructionKind::TakeErrorPayload {
                        error_slot,
                        buffer_slot,
                    } => {
                        // The payload leaves the error value in the same
                        // statement it enters the buffer binding, so no live C
                        // state ever names it twice.
                        writeln!(
                            source,
                            "    s_{buffer_slot}.value = s_{error_slot}.value; s_{error_slot}.value.buffer = NULL;"
                        )
                        .unwrap();
                    }
                    // Card `W48-QR-SCALAR-TYPES-R6`: a typed payload's
                    // operations are its own type's helpers; an `i32` payload
                    // keeps the historical calls byte for byte.
                    InstructionKind::ReadBuffer { buffer, value_slot } => {
                        if let Some(payload) = typed_payload(*buffer) {
                            writeln!(
                                source,
                                "    {} = qcore_buffer_read_{payload}(s_{buffer}.value.buffer, context);",
                                slot_expression(*value_slot, &types)
                            )
                            .unwrap();
                        } else {
                            writeln!(source, "    s_{value_slot}.value.scalar = qcore_buffer_read(s_{buffer}.value.buffer, context);").unwrap();
                        }
                    }
                    InstructionKind::WriteBuffer { buffer, value } => {
                        if let Some(payload) = typed_payload(*buffer) {
                            writeln!(
                                source,
                                "    qcore_buffer_write_{payload}(s_{buffer}.value.buffer, {}, context);",
                                typed_operand(value, &types)
                            )
                            .unwrap();
                        } else {
                            writeln!(
                                source,
                                "    qcore_buffer_write(s_{buffer}.value.buffer, {}, context);",
                                scalar(value)
                            )
                            .unwrap();
                        }
                    }
                    InstructionKind::WriteBufferByte {
                        authority,
                        index,
                        value,
                    } => {
                        writeln!(
                            source,
                            "    qcore_buffer_byte_write{}(s_{authority}.value.buffer, \
                             {} + UINT64_C({}), {}, context);",
                            typed_suffix(*authority),
                            base(authority),
                            byte_index(index),
                            scalar(value)
                        )
                        .unwrap();
                    }
                    InstructionKind::ReadBufferByte {
                        authority,
                        index,
                        value_slot,
                    } => {
                        writeln!(
                            source,
                            "    s_{value_slot}.value.scalar = \
                             qcore_buffer_byte_read{}(s_{authority}.value.buffer, \
                             {} + UINT64_C({}), context);",
                            typed_suffix(*authority),
                            base(authority),
                            byte_index(index)
                        )
                        .unwrap();
                    }
                    InstructionKind::DropBuffer { buffer } => {
                        writeln!(source, "    qcore_buffer_drop(s_{buffer}.value.buffer, context); s_{buffer}.value.buffer = NULL;").unwrap();
                    }
                    // Card `W48-QR-TASK-RECORD-CAPTURES`: each leaf leaves the
                    // collected Result in the statement it enters its slot, so
                    // no live C state names an owned leaf twice.
                    InstructionKind::UnpackRecord { record, leaves } => {
                        let result = record_sources
                            .get(record)
                            .expect("a checked unpack follows the collection that bound it");
                        for (index, leaf) in leaves.iter().enumerate() {
                            writeln!(
                                source,
                                "    s_{leaf}.value = s_{result}.result.leaves[{index}]; s_{result}.result.leaves[{index}].buffer = NULL;"
                            )
                            .unwrap();
                        }
                    }
                    InstructionKind::RefillField { slot, value, .. } => {
                        writeln!(
                            source,
                            "    s_{slot}.value = {};",
                            self_value(value, &types, tail)
                        )
                        .unwrap();
                        consume(&mut source, value);
                    }
                    InstructionKind::EnterScope { scope } => {
                        writeln!(source, "    /* checked scope {scope} begins */").unwrap()
                    }
                    InstructionKind::ExitScope { scope } => writeln!(
                        source,
                        "    /* checked scope {scope} exits without tasks */"
                    )
                    .unwrap(),
                    InstructionKind::Join {
                        handle,
                        outcome_slot,
                    } => {
                        // The borrower's identity has to be read before
                        // collection, which reclaims the start record. The
                        // return line is emitted only after a successful
                        // collection: a join that does not return closes no
                        // loan, here or in the model.
                        let lent = loans_by_handle.get(handle).cloned().unwrap_or_default();
                        let leased = leases_by_handle.get(handle).cloned().unwrap_or_default();
                        source.push_str("    {\n");
                        if !lent.is_empty() || !leased.is_empty() {
                            writeln!(
                                source,
                                "        uint64_t borrower = s_{handle}.handle->identity;"
                            )
                            .unwrap();
                        }
                        if let Some(slot) = outcome_slot {
                            writeln!(
                                source,
                                "        s_{slot}.result = qcore_task_collect(s_{handle}.handle);"
                            )
                            .unwrap();
                        } else {
                            writeln!(
                                source,
                                "        (void)qcore_task_collect(s_{handle}.handle);"
                            )
                            .unwrap();
                        }
                        writeln!(source, "        s_{handle}.handle = NULL;").unwrap();
                        for lent_slot in lent {
                            let generation = format!("s_{lent_slot}.value.buffer->generation");
                            writeln!(
                                source,
                                "        qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_TASK_RETURN, borrower, {generation}, INT64_C(0), (const char *)0);"
                            )
                            .unwrap();
                        }
                        for leased_slot in leased {
                            let generation = format!("s_{leased_slot}.value.buffer->generation");
                            writeln!(
                                source,
                                "        qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_TASK_LEASE_RETURN, borrower, {generation}, INT64_C(0), (const char *)0);"
                            )
                            .unwrap();
                        }
                        source.push_str("    }\n");
                    }
                }
                source_map.push(TaskCMapping {
                    generated_start_line: start,
                    generated_end_line: source.lines().count(),
                    span: instruction.span,
                });
            }
            for statement in source_lines(
                sites,
                *id,
                *block_id,
                block.instructions.len(),
                &types,
                target,
            ) {
                writeln!(source, "    {statement}").unwrap();
            }
            let start = source.lines().count() + 1;
            match &block.terminator.kind {
                TerminatorKind::Goto { target } => {
                    writeln!(source, "    goto qcore_block_{target};").unwrap()
                }
                TerminatorKind::TrySpawn {
                    worker,
                    arguments,
                    handle_slot,
                    success,
                    spawn_error,
                    ..
                } => {
                    source.push_str(
                        "    {\n        struct qcore_task_value captures[QCORE_TASK_ARGUMENT_CAPACITY] = {{0}};\n",
                    );
                    for (index, value) in arguments.iter().enumerate() {
                        writeln!(
                            source,
                            "        captures[{index}] = {};",
                            self_value(value, &types, tail)
                        )
                        .unwrap();
                    }
                    writeln!(source, "        s_{handle_slot}.handle = qcore_task_start(qcore_task_fn_{worker}, captures, {}, context, {worker});", arguments.len()).unwrap();
                    writeln!(source, "        if (s_{handle_slot}.handle == NULL) goto qcore_block_{spawn_error};").unwrap();
                    for argument in arguments {
                        consume(&mut source, argument);
                    }
                    // A creation that failed jumped away above, so no loan is
                    // ever announced for a child that does not exist.
                    for argument in arguments {
                        if let ScalarOperand::Loan { slot, .. } = argument {
                            let borrower = format!("s_{handle_slot}.handle->identity");
                            let generation = format!("s_{slot}.value.buffer->generation");
                            writeln!(
                                source,
                                "        qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_TASK_LOAN, {borrower}, {generation}, INT64_C(0), (const char *)0);"
                            )
                            .unwrap();
                        }
                        // A lease line carries its byte range packed into the
                        // event's value field: `low` in the high 32 bits,
                        // `high` in the low 32 — both bounds are check-time
                        // constants no larger than the payload's 4 bytes, so
                        // the packing is exact and the adapter unpacks it for
                        // the `QCORE_BUFFER_LEASE` rendering.
                        if let ScalarOperand::Lease { slot, low, high } = argument {
                            let borrower = format!("s_{handle_slot}.handle->identity");
                            let generation = format!("s_{slot}.value.buffer->generation");
                            let packed = (low << 32) | high;
                            writeln!(
                                source,
                                "        qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_TASK_LEASE, {borrower}, {generation}, INT64_C({packed}), (const char *)0);"
                            )
                            .unwrap();
                        }
                    }
                    writeln!(source, "        goto qcore_block_{success};\n    }}").unwrap();
                }
                TerminatorKind::TryAllocateBuffer {
                    initial,
                    value_slot,
                    success,
                    allocation_error,
                } => {
                    let (suffix, operand) = match typed_payload(*value_slot) {
                        Some(payload) => (format!("_{payload}"), typed_operand(initial, &types)),
                        None => (String::new(), scalar(initial)),
                    };
                    writeln!(source, "    s_{value_slot}.value.buffer = qcore_buffer_new{suffix}({operand}, context);\n    if (s_{value_slot}.value.buffer == NULL) goto qcore_block_{allocation_error};\n    goto qcore_block_{success};").unwrap();
                }
                TerminatorKind::TryOutcome {
                    result_slot,
                    value_slot,
                    error_slot,
                    success,
                    error,
                } => {
                    // A failing Result hands its payload, when it has one, to
                    // the error binding and clears the Result's own copy, so
                    // both edges move the pointer exactly once.
                    writeln!(source, "    if (s_{result_slot}.result.error != 0) {{\n        s_{error_slot}.error = s_{result_slot}.result.error;\n        s_{error_slot}.value = s_{result_slot}.result.value; s_{result_slot}.result.value.buffer = NULL;\n        goto qcore_block_{error};\n    }}\n    s_{value_slot}.value = s_{result_slot}.result.value; s_{result_slot}.result.value.buffer = NULL;\n    goto qcore_block_{success};").unwrap();
                }
                TerminatorKind::TryCompute {
                    slot,
                    operation,
                    error_slot,
                    success,
                    error,
                } => {
                    let helper = operation_helper(operation, &types, true, target)
                        .expect("a raising value statement calls a checked helper");
                    helpers.insert(helper);
                    let operands = operation
                        .operands()
                        .iter()
                        .map(|operand| value_operand(operand, &types))
                        .collect::<Vec<_>>();
                    writeln!(
                        source,
                        "    {{\n        int q_tag = {}({}, &{});\n        if (q_tag != 0) {{\n            switch (q_tag) {{",
                        helper.name(),
                        operands.join(", "),
                        slot_expression(*slot, &types)
                    )
                    .unwrap();
                    // The helper's tag is §G02t's; the task value carries the
                    // task route's own tag for the same `ArithmeticError`
                    // variant, which is the one every task error travels as.
                    for variant in ArithmeticError::ALL {
                        if let Some(tag) = errors.get(&variant.to_string()) {
                            writeln!(
                                source,
                                "            case {}: s_{error_slot}.error = {tag}; break;",
                                variant.tag()
                            )
                            .unwrap();
                        }
                    }
                    writeln!(
                        source,
                        "            default: abort();\n            }}\n            goto qcore_block_{error};\n        }}\n        goto qcore_block_{success};\n    }}"
                    )
                    .unwrap();
                }
                // Card `W48-QR-TASK-RECORD-CAPTURES`: a record result travels
                // in the result's `leaves`, one task value per leaf, each
                // spelled exactly as a single result of its type would be.
                TerminatorKind::Return {
                    value: value @ ScalarOperand::Record(leaves),
                } => {
                    let ScalarType::Record(record) = function.result else {
                        unreachable!("a checked record return is a record result's")
                    };
                    let leaves = leaves
                        .iter()
                        .map(|leaf| self_value(leaf, &types, tail))
                        .collect::<Vec<_>>();
                    writeln!(
                        source,
                        "    {{ struct qcore_task_result returned = {{{}, {{0, NULL{tail}}}, 0, {{{}}}}};",
                        record_kind(record),
                        leaves.join(", ")
                    )
                    .unwrap();
                    consume(&mut source, value);
                    source.push_str("    return returned; }\n");
                }
                TerminatorKind::Return { value } => {
                    let kind = match function.result {
                        ScalarType::Unit => 0,
                        ScalarType::I32 => 1,
                        ScalarType::Fixed(ty) => fixed_kind(ty),
                        ScalarType::OwnedBuffer(_) => 2,
                        ScalarType::ReadLoan(_) => {
                            unreachable!("a checked result is never a read loan")
                        }
                        ScalarType::WriteLease(_) => {
                            unreachable!("a checked result is never a write lease")
                        }
                        ScalarType::Record(_) => {
                            unreachable!("a checked record result returns a whole record")
                        }
                    };
                    writeln!(
                        source,
                        "    {{ struct qcore_task_result returned = {{{kind}, {}, 0{result_tail}}};",
                        self_value(value, &types, tail)
                    )
                    .unwrap();
                    consume(&mut source, value);
                    source.push_str("    return returned; }\n");
                }
                TerminatorKind::Raise { error, payload } => {
                    let tag = match error {
                        ErrorOperand::Tag(name) => errors[name].to_string(),
                        ErrorOperand::Slot(slot) => format!("s_{slot}.error"),
                    };
                    match payload {
                        None => writeln!(
                            source,
                            "    return (struct qcore_task_result){{0, {{0, NULL{tail}}}, {tag}{result_tail}}};"
                        )
                        .unwrap(),
                        Some(operand) => {
                            writeln!(
                                source,
                                "    {{ struct qcore_task_result raised = {{0, {}, {tag}{result_tail}}};",
                                self_value(operand, &types, tail)
                            )
                            .unwrap();
                            consume(&mut source, operand);
                            source.push_str("    return raised; }\n");
                        }
                    }
                }
            }
            source_map.push(TaskCMapping {
                generated_start_line: start,
                generated_end_line: source.lines().count(),
                span: block.terminator.span,
            });
        }
        source.push_str("}\n");
        source_map.push(TaskCMapping {
            generated_start_line: start_line,
            generated_end_line: source.lines().count(),
            span: function.span,
        });
    }
    // The generated lines this module's mappings name are counted from the top
    // of the emitted file, and the preamble is no longer spliced into the same
    // `String` they were measured against. Shifting them by the preamble's line
    // count once, here, reproduces exactly the numbers the old splice produced —
    // every fragment ends in a newline, so no line is spliced at the boundary.
    // Card `W48-QR-SCALAR-TYPES-R4`: the fixed-width headers — beside the
    // runtime's own, since the widened task value names `bool` and `size_t` —
    // the recorded target's width assertions and each shared checked helper
    // once, after the runtime. Added before the line offset is measured, so
    // the source map counts them.
    if fixed_width {
        let position = fragments
            .iter()
            .position(|fragment| fragment.key == "task.includes")
            .map_or(fragments.len(), |index| index + 1);
        fragments.insert(
            position,
            Fragment::new(
                Section::Includes,
                "task.scalar-types.includes",
                if payloads.is_empty() && !floats {
                    "#include <stdbool.h>\n#include <stddef.h>\n"
                } else if !floats {
                    // The typed payload's unsigned event bits are copied with
                    // `memcpy` (card `W48-QR-SCALAR-TYPES-R6`).
                    "#include <stdbool.h>\n#include <stddef.h>\n#include <string.h>\n"
                } else {
                    // Card `W48-QR-FLOATING-POINT`: `<float.h>` for the
                    // environment assertions, `memcpy` for a float's bits.
                    "#include <stdbool.h>\n#include <stddef.h>\n#include <string.h>\n#include <float.h>\n"
                },
            ),
        );
        let mut text = String::new();
        if let Some(target) = checked.target_assumption() {
            text.push('\n');
            text.push_str(&st::c_target_assertions(target));
        }
        // Card `W48-QR-FLOATING-POINT`: the IEEE environment, asserted when the
        // C is compiled, and the renderers the module's lines use.
        if floats {
            text.push('\n');
            text.push_str(&crate::scalar_floats::c_environment());
            let mut renderers = BTreeSet::new();
            for site in sites {
                if let crate::std_io::StdIoEvent::Print { rendering, .. }
                | crate::std_io::StdIoEvent::Log { rendering, .. } = &site.event
                    && let Some(slot) = site.operand
                    && let Some(ScalarType::Fixed(ty)) = checked
                        .function_slot_types(site.function)
                        .get(&slot)
                        .copied()
                    && ty.is_float()
                {
                    renderers.insert((*rendering == crate::std_io::Rendering::Hex, ty));
                }
            }
            text.push_str(&crate::scalar_floats::c_renderers(&renderers));
        }
        // Card `W48-QR-SCALAR-TYPES-R6`: every typed payload's byte extent is
        // the size the checker decided leases against, asserted to the C
        // compiler — for `bool`, `ptrdiff_t` and `size_t` the one check that
        // turns an implementation-defined size into a build refusal.
        if !payloads.is_empty() {
            text.push('\n');
            for payload in &payloads {
                if let Some(assertion) = st::c_size_assertion(*payload, target) {
                    text.push_str(&assertion);
                }
            }
        }
        if !helpers.is_empty() {
            text.push('\n');
        }
        for helper in &helpers {
            text.push_str(&scalar_helper_definition(*helper, target));
        }
        if !text.is_empty() {
            fragments.push(Fragment::new(Section::Prelude, "task.scalar-helpers", text));
        }
    }
    let offset = c_program::preamble_text(&fragments).lines().count();
    for mapping in &mut source_map {
        mapping.generated_start_line += offset;
        mapping.generated_end_line += offset;
    }

    let mut prologue = String::from(
        "    (void)&qcore_task_start; (void)&qcore_task_collect; (void)&qcore_buffer_new; (void)&qcore_buffer_read; (void)&qcore_buffer_write; (void)&qcore_buffer_byte_write; (void)&qcore_buffer_byte_read; (void)&qcore_buffer_drop;\n",
    );
    if !payloads.is_empty() {
        prologue.push_str("    (void)&qcore_buffer_typed_event; (void)&qcore_buffer_acquire; (void)&qcore_buffer_bits;\n");
        for payload in &payloads {
            write!(
                prologue,
                "    (void)&qcore_buffer_new_{payload}; (void)&qcore_buffer_read_{payload}; (void)&qcore_buffer_write_{payload}; (void)&qcore_buffer_byte_read_{payload};"
            )
            .unwrap();
            if *payload != st::ScalarType::Bool {
                write!(prologue, " (void)&qcore_buffer_byte_write_{payload};").unwrap();
            }
            prologue.push('\n');
        }
    }
    for id in module.functions.keys() {
        writeln!(prologue, "    (void)&qcore_task_fn_{id};").unwrap();
    }
    let mut definitions = fragments;
    definitions.push(Fragment::new(
        Section::FileScope,
        "task.definitions",
        source,
    ));
    let source = c_program::assemble(
        &definitions,
        &c_program::Entry {
            lead: String::new(),
            brace: c_program::BraceStyle::KAndR,
            prologue,
            body: String::new(),
            epilogue: format!(
                "    struct qcore_task_context context = {{0}}; context.next_task = 1; context.next_buffer = 1;\n    struct qcore_task_result result = qcore_task_fn_{}(NULL, &context);\n    qcore_task_log_result(0, result);\n    return 0;\n}}\n",
                module.entry
            ),
        },
    );
    TaskCEmission { source, source_map }
}

/// The file-scope text the checked task route contributes to a program it is
/// part of (card `C-EMIT-MAIN-UNIFY`).
///
/// `capacity` and `event_capacity` are read by the compatibility header and the
/// runtime below it, so they sit in [`Section::Config`] — ahead even of the
/// compatibility header, which is the only section whose reason is that a later
/// header reads it.
///
/// The `<stdio.h>` include is placed among this route's preludes rather than
/// beside its other includes. That is this repository's one section inversion,
/// and it is reproduced rather than repaired: lifting the include above
/// `DIAGNOSTIC_LOG_PRELUDE` is inert in C — the logger prelude uses no stdio —
/// but it is still a byte move in a program the two F02 emission digests pin,
/// and this card moves no pinned byte. `c_program::out_of_order_fragments`
/// reports it, and a test pins that it is the only one.
#[must_use]
pub fn preamble(
    capacity: usize,
    event_capacity: usize,
    log_level: Option<crate::std_io::Level>,
    writes_lines: bool,
) -> Vec<Fragment> {
    let mut fragments = vec![
        Fragment::new(
            Section::Config,
            "task.capacities",
            format!(
                "#define QCORE_TASK_ARGUMENT_CAPACITY {capacity}\n#define QCORE_BUFFER_EVENT_CAPACITY {event_capacity}\n"
            ),
        ),
        Fragment::new(
            Section::Compat,
            "c11_threads.compat",
            include_str!("c11_threads_compat.h"),
        ),
        Fragment::new(Section::Includes, "task.includes", RUNTIME_INCLUDES),
    ];
    if let Some(level) = log_level {
        fragments.push(diagnostic_log_c_emit::adapter_declaration_fragment(level));
    }
    fragments.push(diagnostic_log_c_emit::diagnostic_log_fragment());
    if log_level.is_some() {
        fragments.push(diagnostic_log_c_emit::adapter_definition_fragment());
    }
    if writes_lines {
        fragments.push(io_emit::stdio_include_fragment());
        fragments.push(io_emit::io_prelude_fragment());
    }
    fragments.push(Fragment::new(Section::Prelude, "task.runtime", RUNTIME));
    fragments
}

/// The task runtime of a module using the fixed-width surface (card
/// `W48-QR-SCALAR-TYPES-R4`): [`RUNTIME`] with the task value widened by a
/// union holding one member per fixed-width type the module names, and the
/// result logger given one arm per fixed-width worker result. An `i32` still
/// travels in `scalar`, so every historical operation is unchanged; and a
/// module that names no such type never reaches this function, so its bytes
/// are [`RUNTIME`]'s.
fn fixed_width_runtime(
    fixed: &BTreeSet<st::ScalarType>,
    results: &BTreeSet<st::ScalarType>,
    payloads: &BTreeSet<st::ScalarType>,
) -> String {
    let value = "struct qcore_task_value { int32_t scalar; struct qcore_buffer *buffer; };\n";
    let tail = "    } else abort();\n}\nstatic int qcore_task_trampoline";
    assert_eq!(
        RUNTIME.matches(value).count(),
        1,
        "the task value is declared once"
    );
    assert_eq!(
        RUNTIME.matches(tail).count(),
        1,
        "the result logger ends once"
    );
    let mut widened = String::new();
    if !fixed.is_empty() {
        widened.push_str("union qcore_task_typed {");
        for ty in fixed {
            write!(widened, " {} {};", st::c_type(*ty), member(*ty)).unwrap();
        }
        widened.push_str(" };\n");
        widened.push_str(
            "struct qcore_task_value { int32_t scalar; struct qcore_buffer *buffer; union qcore_task_typed typed; };\n",
        );
    } else {
        widened.push_str(value);
    }
    let mut arms = String::from("    }");
    for ty in results {
        let label = ty.name().to_ascii_uppercase();
        let field = format!("result.value.typed.{}", member(*ty));
        if ty.is_float() {
            write!(
                arms,
                " else if (result.kind == {}) {}\n   ",
                fixed_kind(*ty),
                float_result_log(*ty, &field, "    ")
            )
            .unwrap();
            continue;
        }
        // A signed value (and `bool`) travels in the event's signed `value`,
        // an unsigned one in its unsigned `related`: both are lossless for
        // every width of the table, so no conversion is implementation-defined.
        let (related, signed) = if ty.is_integer() && !ty.is_signed() {
            (format!("(uint64_t){field}"), "INT64_C(0)".to_owned())
        } else {
            ("UINT64_C(0)".to_owned(), format!("(int64_t){field}"))
        };
        write!(
            arms,
            " else if (result.kind == {})\n        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_RESULT, identity,\n            {related}, {signed}, \"={label}\");\n   ",
            fixed_kind(*ty)
        )
        .unwrap();
    }
    arms.push_str(" else abort();\n}\nstatic int qcore_task_trampoline");
    let runtime = RUNTIME.replace(value, &widened).replace(tail, &arms);
    if payloads.is_empty() {
        runtime
    } else {
        typed_payload_runtime(&runtime, payloads)
    }
}

/// The task runtime of a module whose workers return records (card
/// `W48-QR-TASK-RECORD-CAPTURES`): the task result gains a `leaves` array of
/// task values — one per leaf of the largest record, `QCORE_TASK_RECORD_CAPACITY`
/// — and the result logger an arm per record result kind. The arm logs the
/// header under `=RECORD`, whose number is the leaf count, and then each leaf
/// exactly as a single result of that leaf's type is logged, so the trace of a
/// record result is `QCORE_TASK_RESULT TASK RECORD N` followed by one result
/// line per leaf in leaf order (the reference's own rendering). Applied only
/// to a module that returns a record; every other runtime keeps its bytes.
fn record_runtime(
    runtime: &str,
    records: &BTreeMap<usize, &crate::task_execution::TaskRecord>,
) -> String {
    let result =
        "struct qcore_task_result { int kind; struct qcore_task_value value; int error; };\n";
    let tail = " else abort();\n}\nstatic int qcore_task_trampoline";
    assert_eq!(
        runtime.matches(result).count(),
        1,
        "the task result is declared once"
    );
    assert_eq!(
        runtime.matches(tail).count(),
        1,
        "the result logger ends once"
    );
    let capacity = records
        .values()
        .map(|record| record.leaves.len())
        .max()
        .unwrap_or(1)
        .max(1);
    let mut arms = String::new();
    for (record, declared) in records {
        write!(
            arms,
            " else if (result.kind == {}) {{\n        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_RESULT, identity,\n            UINT64_C(0), INT64_C({}), \"=RECORD\");\n",
            record_kind(*record),
            declared.leaves.len()
        )
        .unwrap();
        for (index, leaf) in declared.leaves.iter().enumerate() {
            let value = format!("result.leaves[{index}]");
            match leaf.ty {
                ScalarType::I32 => write!(
                    arms,
                    "        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_RESULT, identity,\n            UINT64_C(0), (int64_t){value}.scalar, \"i32\");\n"
                )
                .unwrap(),
                ScalarType::Fixed(ty) if ty.is_float() => {
                    let field = format!("{value}.typed.{}", member(ty));
                    writeln!(arms, "        {}", float_result_log(ty, &field, "        ")).unwrap();
                }
                ScalarType::Fixed(ty) => {
                    let label = ty.name().to_ascii_uppercase();
                    let field = format!("{value}.typed.{}", member(ty));
                    let (related, signed) = if ty.is_integer() && !ty.is_signed() {
                        (format!("(uint64_t){field}"), "INT64_C(0)".to_owned())
                    } else {
                        ("UINT64_C(0)".to_owned(), format!("(int64_t){field}"))
                    };
                    write!(
                        arms,
                        "        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_RESULT, identity,\n            {related}, {signed}, \"={label}\");\n"
                    )
                    .unwrap();
                }
                ScalarType::OwnedBuffer(_) => write!(
                    arms,
                    "        if ({value}.buffer == NULL) abort();\n        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_RESULT, identity,\n            {value}.buffer->generation, INT64_C(0), \"buffer\");\n"
                )
                .unwrap(),
                _ => unreachable!("a checked record leaf is a copy scalar or an owned Buffer"),
            }
        }
        arms.push_str("    }");
    }
    arms.push_str(tail);
    runtime
        .replace(
            result,
            &format!(
                "#define QCORE_TASK_RECORD_CAPACITY {capacity}\nstruct qcore_task_result {{ int kind; struct qcore_task_value value; int error; struct qcore_task_value leaves[QCORE_TASK_RECORD_CAPACITY]; }};\n"
            ),
        )
        .replace(tail, &arms)
}

/// The task runtime of a module whose Buffers carry typed payloads (card
/// `W48-QR-SCALAR-TYPES-R6`, `W48-QR-TYPES-PAYLOADS`).
///
/// `struct qcore_buffer` gains a union with one member per payload type the
/// module names beside the historical `int32_t payload`, so an `i32` Buffer
/// is unchanged and a typed one lives in its member; each payload type gets
/// its own allocation, read, write and byte-access helpers, whose byte extent
/// is `sizeof` of the member — the type's size, asserted equal to the extent
/// the checker decided leases against — never the `int32_t`'s four. A typed
/// event carries its bits and a `=TYPE` name through the worker's private
/// event array, which the maintained sink's typed-payload arm renders; a
/// `bool` payload has no byte-write helper, because the checker refuses every
/// byte write into one.
fn typed_payload_runtime(runtime: &str, payloads: &BTreeSet<st::ScalarType>) -> String {
    let buffer = "struct qcore_buffer { uint64_t generation; int32_t payload; };\n";
    let event = "struct qcore_buffer_event { int kind; uint64_t generation; int32_t payload; };\n";
    let event_init = "    struct qcore_buffer_event event = {kind, generation, payload};\n";
    let logged = "        (int64_t)event.payload, (const char *)0);\n";
    for (needle, what) in [
        (buffer, "the Buffer"),
        (event, "the Buffer event"),
        (event_init, "the event initializer"),
        (logged, "the event log call"),
    ] {
        assert_eq!(runtime.matches(needle).count(), 1, "{what} is written once");
    }
    let mut union = String::from("union qcore_buffer_typed {");
    for payload in payloads {
        write!(union, " {} {};", st::c_type(*payload), member(*payload)).unwrap();
    }
    union.push_str(" };\n");
    let mut text = runtime
        .replace(
            buffer,
            &format!(
                "{union}struct qcore_buffer {{ uint64_t generation; int32_t payload; union qcore_buffer_typed typed; }};\n"
            ),
        )
        .replace(
            event,
            "struct qcore_buffer_event { int kind; uint64_t generation; int32_t payload; int64_t bits; const char *name; };\n",
        )
        .replace(
            event_init,
            "    struct qcore_buffer_event event = {kind, generation, payload, INT64_C(0), (const char *)0};\n",
        )
        .replace(
            logged,
            "        event.name != NULL ? event.bits : (int64_t)event.payload, event.name);\n",
        );
    text.push_str(TYPED_PAYLOAD_COMMON);
    for payload in payloads {
        let c = st::c_type(*payload);
        let name = payload.name();
        let label = name.to_ascii_uppercase();
        let field = format!("buffer->typed.{}", member(*payload));
        // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: a float payload's event carries
        // its bit pattern, copied with `memcpy` — never a conversion to an
        // integer, which is undefined for a NaN or an out-of-range value.
        let bits = if payload.is_float() {
            let (unsigned, widen) = if *payload == st::ScalarType::F32 {
                ("uint32_t", "(int64_t)q_bits")
            } else {
                ("uint64_t", "qcore_buffer_bits(q_bits)")
            };
            write!(
                text,
                "static int64_t qcore_buffer_bits_{name}({c} value) {{\n    \
                 {unsigned} q_bits;\n    \
                 memcpy(&q_bits, &value, sizeof q_bits);\n    \
                 return {widen};\n}}\n"
            )
            .unwrap();
            format!("qcore_buffer_bits_{name}(payload)")
        } else if payload.is_integer() && !payload.is_signed() {
            "qcore_buffer_bits((uint64_t)payload)".to_owned()
        } else {
            "(int64_t)payload".to_owned()
        };
        write!(
            text,
            "static struct qcore_buffer *qcore_buffer_new_{name}({c} payload, struct qcore_task_context *context) {{\n    \
             struct qcore_buffer *buffer = qcore_buffer_acquire(context);\n    \
             if (buffer == NULL) return NULL;\n    \
             {field} = payload;\n    \
             qcore_buffer_typed_event(context, 0, buffer->generation, {bits}, \"={label}\");\n    \
             return buffer;\n}}\n\
             static {c} qcore_buffer_read_{name}(const struct qcore_buffer *buffer, struct qcore_task_context *context) {{\n    \
             if (buffer == NULL) abort();\n    \
             {c} payload = {field};\n    \
             qcore_buffer_typed_event(context, 2, buffer->generation, {bits}, \"={label}\");\n    \
             return payload;\n}}\n\
             static void qcore_buffer_write_{name}(struct qcore_buffer *buffer, {c} payload, struct qcore_task_context *context) {{\n    \
             if (buffer == NULL) abort();\n    \
             {field} = payload;\n    \
             qcore_buffer_typed_event(context, 4, buffer->generation, {bits}, \"={label}\");\n}}\n\
             static int32_t qcore_buffer_byte_read_{name}(const struct qcore_buffer *buffer, uint64_t index, struct qcore_task_context *context) {{\n    \
             if (buffer == NULL || index >= sizeof {field}) abort();\n    \
             unsigned char byte = ((const unsigned char *)&{field})[index];\n    \
             qcore_buffer_event(context, 6, buffer->generation, (int32_t)((index << 8) | byte));\n    \
             return (int32_t)byte;\n}}\n"
        )
        .unwrap();
        if *payload != st::ScalarType::Bool {
            write!(
                text,
                "static void qcore_buffer_byte_write_{name}(struct qcore_buffer *buffer, uint64_t index, int32_t value, struct qcore_task_context *context) {{\n    \
                 if (buffer == NULL || index >= sizeof {field}) abort();\n    \
                 unsigned char byte = (unsigned char)(value & INT32_C(255));\n    \
                 ((unsigned char *)&{field})[index] = byte;\n    \
                 qcore_buffer_event(context, 5, buffer->generation, (int32_t)((index << 8) | byte));\n}}\n"
            )
            .unwrap();
        }
    }
    text
}

/// The helpers every typed payload shares (card `W48-QR-SCALAR-TYPES-R6`):
/// the typed event (the historical event's dispatch, carrying bits and a
/// name), the allocation without a payload store (the historical allocation's
/// identity and failure discipline, verbatim), and the unsigned bit copy — a
/// `memcpy`, because converting a `uint64_t` above `INT64_MAX` to `int64_t` is
/// implementation-defined in C11 while copying its object representation is
/// not.
const TYPED_PAYLOAD_COMMON: &str = r#"/* Card W48-QR-SCALAR-TYPES-R6: typed Buffer payloads. */
static void qcore_buffer_typed_event(struct qcore_task_context *context, int kind, uint64_t generation, int64_t bits, const char *name) {
    struct qcore_buffer_event event = {kind, generation, INT32_C(0), bits, name};
    if (context->identity == 0) qcore_buffer_log_event(0, event);
    else {
        if (context->event_count >= QCORE_BUFFER_EVENT_CAPACITY) abort();
        context->events[context->event_count++] = event;
    }
}
static int64_t qcore_buffer_bits(uint64_t value) {
    int64_t bits;
    memcpy(&bits, &value, sizeof bits);
    return bits;
}
static struct qcore_buffer *qcore_buffer_acquire(struct qcore_task_context *context) {
    if (context->identity > UINT32_MAX || context->next_buffer == 0 ||
        context->next_buffer > UINT32_MAX || context->allocation_attempt == UINT64_MAX) abort();
    ++context->allocation_attempt;
    struct qcore_buffer *buffer = QCORE_BUFFER_MALLOC_FOR_TASK(
        sizeof(*buffer), context->identity, context->allocation_attempt);
    if (buffer == NULL) {
        qcore_buffer_event(context, 1, 0, 0);
        return NULL;
    }
    buffer->generation = context->identity * UINT64_C(4294967296) + context->next_buffer++;
    buffer->payload = INT32_C(0);
    return buffer;
}
"#;

/// The runtime's headers, kept separate from [`RUNTIME`] so the single printing
/// route can be installed between them.
const RUNTIME_INCLUDES: &str = "#include <inttypes.h>\n\
#include <stdint.h>\n\
#include <stdlib.h>\n";

const RUNTIME: &str = r#"#ifndef QCORE_TASK_CALLOC
#define QCORE_TASK_CALLOC calloc
#endif
#ifndef QCORE_TASK_FREE
#define QCORE_TASK_FREE free
#endif
#ifndef QCORE_TASK_CREATE
#define QCORE_TASK_CREATE thrd_create
#endif
#ifndef QCORE_TASK_JOIN
#define QCORE_TASK_JOIN thrd_join
#endif
#ifndef QCORE_BUFFER_MALLOC
#define QCORE_BUFFER_MALLOC malloc
#endif
#ifndef QCORE_BUFFER_MALLOC_FOR_TASK
#define QCORE_BUFFER_MALLOC_FOR_TASK(size, task, attempt) QCORE_BUFFER_MALLOC(size)
#endif
#ifndef QCORE_BUFFER_FREE
#define QCORE_BUFFER_FREE free
#endif
/* Only checked move and read-share operations expose this private allocation to
 * workers. The generation never changes after successful allocation; the
 * payload changes only through a checked exclusive-authority write. */
struct qcore_buffer { uint64_t generation; int32_t payload; };
struct qcore_task_value { int32_t scalar; struct qcore_buffer *buffer; };
struct qcore_task_result { int kind; struct qcore_task_value value; int error; };
struct qcore_buffer_event { int kind; uint64_t generation; int32_t payload; };
struct qcore_task_context {
    uint64_t identity;
    uint64_t next_task;
    uint64_t next_buffer;
    uint64_t allocation_attempt;
    size_t event_count;
    struct qcore_buffer_event events[QCORE_BUFFER_EVENT_CAPACITY];
};
typedef struct qcore_task_result (*qcore_task_worker)(const struct qcore_task_value *, struct qcore_task_context *);
struct qcore_task_start_record {
    thrd_t thread;
    uint64_t identity;
    qcore_task_worker worker;
    struct qcore_task_value arguments[QCORE_TASK_ARGUMENT_CAPACITY];
    struct qcore_task_result result;
    struct qcore_task_context context;
};
struct qcore_task_slot {
    struct qcore_task_value value;
    struct qcore_task_result result;
    struct qcore_task_start_record *handle;
    int error;
};
static const char *qcore_task_error_name(int);
static void qcore_buffer_log_event(uint64_t task, struct qcore_buffer_event event) {
    enum qcore_diagnostic_kind kind;
    switch (event.kind) {
    case 0: kind = QCORE_DIAG_RESOURCE_ACQUIRE; break;
    case 1: kind = QCORE_DIAG_RESOURCE_ACQUIRE_FAILED; break;
    case 2: kind = QCORE_DIAG_RESOURCE_READ; break;
    case 3: kind = QCORE_DIAG_RESOURCE_RELEASE; break;
    case 4: kind = QCORE_DIAG_RESOURCE_WRITE; break;
    case 5: kind = QCORE_DIAG_BUFFER_BYTE_WRITE; break;
    case 6: kind = QCORE_DIAG_BUFFER_BYTE_READ; break;
    default: abort();
    }
    qcore_diagnostic_log(QCORE_LOG_TRACE, kind, task, event.generation,
        (int64_t)event.payload, (const char *)0);
}
static void qcore_buffer_event(struct qcore_task_context *context, int kind, uint64_t generation, int32_t payload) {
    struct qcore_buffer_event event = {kind, generation, payload};
    if (context->identity == 0) qcore_buffer_log_event(0, event);
    else {
        /* Each leaf worker owns its event storage. Only its parent reads this
         * array, after successful native join. No shared trace counter exists. */
        if (context->event_count >= QCORE_BUFFER_EVENT_CAPACITY) abort();
        context->events[context->event_count++] = event;
    }
}
static struct qcore_buffer *qcore_buffer_new(int32_t payload, struct qcore_task_context *context) {
    /* Finite leaf tasks have disjoint identity ranges and private counters.
     * Failed allocations consume an attempt, not a resource generation. */
    if (context->identity > UINT32_MAX || context->next_buffer == 0 ||
        context->next_buffer > UINT32_MAX || context->allocation_attempt == UINT64_MAX) abort();
    ++context->allocation_attempt;
    struct qcore_buffer *buffer = QCORE_BUFFER_MALLOC_FOR_TASK(
        sizeof(*buffer), context->identity, context->allocation_attempt);
    if (buffer == NULL) {
        qcore_buffer_event(context, 1, 0, 0);
        return NULL;
    }
    buffer->generation = context->identity * UINT64_C(4294967296) + context->next_buffer++;
    buffer->payload = payload;
    qcore_buffer_event(context, 0, buffer->generation, payload);
    return buffer;
}
static int32_t qcore_buffer_read(const struct qcore_buffer *buffer, struct qcore_task_context *context) {
    if (buffer == NULL) abort();
    int32_t payload = buffer->payload;
    qcore_buffer_event(context, 2, buffer->generation, payload);
    return payload;
}
/* Only a checked exclusive-authority holder reaches this: no read share may be
 * outstanding, so no other thread is reading this object while it is written. */
static void qcore_buffer_write(struct qcore_buffer *buffer, int32_t payload, struct qcore_task_context *context) {
    if (buffer == NULL) abort();
    buffer->payload = payload;
    qcore_buffer_event(context, 4, buffer->generation, payload);
}
/* One byte of the payload, through a checked range write lease. The object
 * written is `((unsigned char *)&buffer->payload)[index]`: a distinct memory
 * location in the sense of C11 6.2.6.1/4 and 3.14, so two workers holding
 * disjoint leases write distinct objects and are not in a data race. The
 * checker has already decided that `index` lies inside the writer's own lease
 * and that no two live leases overlap; the extent is re-decided here so a
 * miscompiled index cannot reach memory. */
static void qcore_buffer_byte_write(struct qcore_buffer *buffer, uint64_t index, int32_t value, struct qcore_task_context *context) {
    if (buffer == NULL || index >= sizeof buffer->payload) abort();
    unsigned char byte = (unsigned char)(value & INT32_C(255));
    ((unsigned char *)&buffer->payload)[index] = byte;
    qcore_buffer_event(context, 5, buffer->generation, (int32_t)((index << 8) | byte));
}
static int32_t qcore_buffer_byte_read(const struct qcore_buffer *buffer, uint64_t index, struct qcore_task_context *context) {
    if (buffer == NULL || index >= sizeof buffer->payload) abort();
    unsigned char byte = ((const unsigned char *)&buffer->payload)[index];
    qcore_buffer_event(context, 6, buffer->generation, (int32_t)((index << 8) | byte));
    return (int32_t)byte;
}
static void qcore_buffer_drop(struct qcore_buffer *buffer, struct qcore_task_context *context) {
    if (buffer == NULL) abort();
    qcore_buffer_event(context, 3, buffer->generation, 0);
    QCORE_BUFFER_FREE(buffer);
}
static void qcore_task_log_result(uint64_t identity, struct qcore_task_result result) {
    if (result.error != 0 && result.value.buffer != NULL)
        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_RESULT, identity,
            result.value.buffer->generation, result.error, qcore_task_error_name(result.error));
    else if (result.error != 0)
        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_RESULT, identity,
            UINT64_C(0), result.error, qcore_task_error_name(result.error));
    else if (result.kind == 0)
        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_RESULT, identity,
            UINT64_C(0), INT64_C(0), "unit");
    else if (result.kind == 1)
        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_RESULT, identity,
            UINT64_C(0), (int64_t)result.value.scalar, "i32");
    else if (result.kind == 2) {
        if (result.value.buffer == NULL) abort();
        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_RESULT, identity,
            result.value.buffer->generation, INT64_C(0), "buffer");
    } else abort();
}
static int qcore_task_trampoline(void *pointer) {
    struct qcore_task_start_record *record = pointer;
    record->result = record->worker(record->arguments, &record->context);
    return 0;
}
static struct qcore_task_start_record *qcore_task_start(qcore_task_worker worker,
        const struct qcore_task_value *arguments, size_t count, struct qcore_task_context *context, size_t worker_id) {
    if (context->next_task == 0 || context->next_task > UINT32_MAX) abort();
    struct qcore_task_start_record *record = QCORE_TASK_CALLOC(1, sizeof(*record));
    if (record == NULL) {
        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_CREATE_FAILED,
            (uint64_t)worker_id, UINT64_C(0), INT64_C(0), (const char *)0);
        return NULL;
    }
    record->worker = worker;
    record->identity = context->next_task;
    record->context.identity = record->identity;
    record->context.next_buffer = 1;
    for (size_t i = 0; i < count; ++i) record->arguments[i] = arguments[i];
    if (QCORE_TASK_CREATE(&record->thread, qcore_task_trampoline, record) != thrd_success) {
        /* Creation failure never transfers the buffer; discard only the
         * environment's non-owning copies. The parent still owns each slot. */
        QCORE_TASK_FREE(record);
        qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_CREATE_FAILED,
            (uint64_t)worker_id, UINT64_C(0), INT64_C(0), (const char *)0);
        return NULL;
    }
    ++context->next_task;
    qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_SPAWN,
        record->identity, (uint64_t)worker_id, INT64_C(0), (const char *)0);
    return record;
}
static struct qcore_task_result qcore_task_collect(struct qcore_task_start_record *record) {
    int status;
    /* An unsuccessful join gives no permission to read completion data or
     * reclaim either the environment or an owned completion payload. */
    if (QCORE_TASK_JOIN(record->thread, &status) != thrd_success) abort();
    if (status != 0) abort();
    struct qcore_task_result result = record->result;
    for (size_t i = 0; i < record->context.event_count; ++i)
        qcore_buffer_log_event(record->identity, record->context.events[i]);
    qcore_task_log_result(record->identity, result);
    qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_TASK_JOIN,
        record->identity, UINT64_C(0), INT64_C(0), (const char *)0);
    QCORE_TASK_FREE(record);
    return result;
}
"#;
