//! Native C11 threads for checked finite copy/owned-buffer task modules.
//! Start records are private until fully initialized, retained through native
//! join, and never reclaimed after an unexpected join failure. Worker code has
//! no source-shared memory and writes only its own completion record.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use crate::task_execution::{
    CheckedTaskModule, ErrorOperand, ScalarOperand, ScalarType,
    TaskInstructionKind as InstructionKind, TaskSpan, TaskTerminatorKind as TerminatorKind,
};

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
        ScalarOperand::I32(i32::MIN) => "(-INT32_C(2147483647) - INT32_C(1))".into(),
        ScalarOperand::I32(value) if *value < 0 => format!("(-INT32_C({}))", -value),
        ScalarOperand::I32(value) => format!("INT32_C({value})"),
        ScalarOperand::Slot(slot) => format!("s_{slot}.value.scalar"),
        ScalarOperand::Move(_) => unreachable!("checked scalar operand is not owned"),
    }
}

fn self_value(operand: &ScalarOperand) -> String {
    if let ScalarOperand::Move(slot) = operand {
        format!("s_{slot}.value")
    } else {
        format!("(struct qcore_task_value){{{}, NULL}}", scalar(operand))
    }
}

fn consume(source: &mut String, operand: &ScalarOperand) {
    if let ScalarOperand::Move(slot) = operand {
        writeln!(source, "    s_{slot}.value.buffer = NULL;").unwrap();
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
    let mut source = format!(
        "#define QCORE_TASK_ARGUMENT_CAPACITY {capacity}\n#define QCORE_BUFFER_EVENT_CAPACITY {event_capacity}\n"
    );
    source.push_str(RUNTIME);
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
                    | InstructionKind::BindScalar { slot, .. } => {
                        slots.insert(*slot);
                    }
                    InstructionKind::UnwrapInfallible { value_slot, .. } => {
                        slots.insert(*value_slot);
                    }
                    InstructionKind::Join {
                        outcome_slot: Some(slot),
                        ..
                    } => {
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
        writeln!(source, "    goto qcore_block_{};", function.entry).unwrap();
        for (block_id, block) in &function.blocks {
            writeln!(source, "qcore_block_{block_id}: ;").unwrap();
            for instruction in &block.instructions {
                let start = source.lines().count() + 1;
                match &instruction.kind {
                    InstructionKind::BindScalar { slot, value } => {
                        writeln!(source, "    s_{slot}.value = {};", self_value(value)).unwrap();
                        consume(&mut source, value);
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
                    InstructionKind::ReadBuffer { buffer, value_slot } => {
                        writeln!(source, "    s_{value_slot}.value.scalar = qcore_buffer_read(s_{buffer}.value.buffer, context);").unwrap();
                    }
                    InstructionKind::DropBuffer { buffer } => {
                        writeln!(source, "    qcore_buffer_drop(s_{buffer}.value.buffer, context); s_{buffer}.value.buffer = NULL;").unwrap();
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
                        if let Some(slot) = outcome_slot {
                            writeln!(
                                source,
                                "    s_{slot}.result = qcore_task_collect(s_{handle}.handle);"
                            )
                            .unwrap();
                        } else {
                            writeln!(source, "    (void)qcore_task_collect(s_{handle}.handle);")
                                .unwrap();
                        }
                        writeln!(source, "    s_{handle}.handle = NULL;").unwrap();
                    }
                }
                source_map.push(TaskCMapping {
                    generated_start_line: start,
                    generated_end_line: source.lines().count(),
                    span: instruction.span,
                });
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
                        writeln!(source, "        captures[{index}] = {};", self_value(value))
                            .unwrap();
                    }
                    writeln!(source, "        s_{handle_slot}.handle = qcore_task_start(qcore_task_fn_{worker}, captures, {}, context, {worker});", arguments.len()).unwrap();
                    writeln!(source, "        if (s_{handle_slot}.handle == NULL) goto qcore_block_{spawn_error};").unwrap();
                    for argument in arguments {
                        consume(&mut source, argument);
                    }
                    writeln!(source, "        goto qcore_block_{success};\n    }}").unwrap();
                }
                TerminatorKind::TryAllocateBuffer {
                    initial,
                    value_slot,
                    success,
                    allocation_error,
                } => {
                    writeln!(source, "    s_{value_slot}.value.buffer = qcore_buffer_new({}, context);\n    if (s_{value_slot}.value.buffer == NULL) goto qcore_block_{allocation_error};\n    goto qcore_block_{success};", scalar(initial)).unwrap();
                }
                TerminatorKind::TryOutcome {
                    result_slot,
                    value_slot,
                    error_slot,
                    success,
                    error,
                } => {
                    writeln!(source, "    if (s_{result_slot}.result.error != 0) {{\n        s_{error_slot}.error = s_{result_slot}.result.error;\n        goto qcore_block_{error};\n    }}\n    s_{value_slot}.value = s_{result_slot}.result.value; s_{result_slot}.result.value.buffer = NULL;\n    goto qcore_block_{success};").unwrap();
                }
                TerminatorKind::Return { value } => {
                    let kind = match function.result {
                        ScalarType::Unit => 0,
                        ScalarType::I32 => 1,
                        ScalarType::OwnedBuffer => 2,
                    };
                    writeln!(
                        source,
                        "    {{ struct qcore_task_result returned = {{{kind}, {}, 0}};",
                        self_value(value)
                    )
                    .unwrap();
                    consume(&mut source, value);
                    source.push_str("    return returned; }\n");
                }
                TerminatorKind::Raise { error } => {
                    let tag = match error {
                        ErrorOperand::Tag(name) => errors[name].to_string(),
                        ErrorOperand::Slot(slot) => format!("s_{slot}.error"),
                    };
                    writeln!(
                        source,
                        "    return (struct qcore_task_result){{0, {{0, NULL}}, {tag}}};"
                    )
                    .unwrap();
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
    source.push_str("int main(void) {\n    (void)&qcore_task_start; (void)&qcore_task_collect; (void)&qcore_buffer_new; (void)&qcore_buffer_read; (void)&qcore_buffer_drop;\n");
    for id in module.functions.keys() {
        writeln!(source, "    (void)&qcore_task_fn_{id};").unwrap();
    }
    writeln!(source, "    struct qcore_task_context context = {{0}}; context.next_task = 1; context.next_buffer = 1;\n    struct qcore_task_result result = qcore_task_fn_{}(NULL, &context);\n    qcore_task_print_result(0, result);\n    return 0;\n}}", module.entry).unwrap();
    TaskCEmission { source, source_map }
}

const RUNTIME: &str = r#"#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <threads.h>
#if defined(__STDC_NO_THREADS__)
#error "The scoped task profile requires C11 threads"
#endif
#ifndef QCORE_TASK_CALLOC
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
/* Only checked move operations expose this private allocation to workers.
 * Payload and generation never change after successful allocation. */
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
static void qcore_buffer_print_event(uint64_t task, struct qcore_buffer_event event) {
    switch (event.kind) {
    case 0: printf("QCORE_BUFFER_ALLOC %" PRIu64 " %" PRIu64 " %" PRId32 "\n", task, event.generation, event.payload); break;
    case 1: printf("QCORE_BUFFER_ALLOC_FAILED %" PRIu64 "\n", task); break;
    case 2: printf("QCORE_BUFFER_READ %" PRIu64 " %" PRIu64 " %" PRId32 "\n", task, event.generation, event.payload); break;
    case 3: printf("QCORE_BUFFER_DROP %" PRIu64 " %" PRIu64 "\n", task, event.generation); break;
    default: abort();
    }
}
static void qcore_buffer_event(struct qcore_task_context *context, int kind, uint64_t generation, int32_t payload) {
    struct qcore_buffer_event event = {kind, generation, payload};
    if (context->identity == 0) qcore_buffer_print_event(0, event);
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
static void qcore_buffer_drop(struct qcore_buffer *buffer, struct qcore_task_context *context) {
    if (buffer == NULL) abort();
    qcore_buffer_event(context, 3, buffer->generation, 0);
    QCORE_BUFFER_FREE(buffer);
}
static void qcore_task_print_result(uint64_t identity, struct qcore_task_result result) {
    if (result.error != 0)
        printf("QCORE_TASK_RESULT %" PRIu64 " ERROR %s\n", identity, qcore_task_error_name(result.error));
    else if (result.kind == 0)
        printf("QCORE_TASK_RESULT %" PRIu64 " UNIT\n", identity);
    else if (result.kind == 1)
        printf("QCORE_TASK_RESULT %" PRIu64 " I32 %" PRId32 "\n", identity, result.value.scalar);
    else if (result.kind == 2) {
        if (result.value.buffer == NULL) abort();
        printf("QCORE_TASK_RESULT %" PRIu64 " BUFFER %" PRIu64 "\n", identity, result.value.buffer->generation);
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
        printf("QCORE_TASK_CREATE_FAILED %zu\n", worker_id);
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
        printf("QCORE_TASK_CREATE_FAILED %zu\n", worker_id);
        return NULL;
    }
    ++context->next_task;
    printf("QCORE_TASK_SPAWN %" PRIu64 " %zu\n", record->identity, worker_id);
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
        qcore_buffer_print_event(record->identity, record->context.events[i]);
    qcore_task_print_result(record->identity, result);
    printf("QCORE_TASK_JOIN %" PRIu64 "\n", record->identity);
    QCORE_TASK_FREE(record);
    return result;
}
"#;
