//! C11 emission and translation-validation support for cleanup-explicit CFGs.

use std::collections::BTreeSet;
use std::fmt::{self, Write as _};

use crate::ResourceId;
use crate::cfg::{
    BasicBlock, CfgInstruction, CfgTerminator, ErrorAbi, ErrorTag, ExecutionObservation,
    ExecutionOutcome, TraceEvent, TypedCfg,
};

/// Emit a standalone, strictly conforming C11 translation unit.  The small
/// harness prints the observable result and resource trace, allowing compiled
/// C to be compared directly with the reference interpreter.
pub fn emit_c11(cfg: &TypedCfg) -> Result<String, CEmitError> {
    validate_c_symbols(cfg)?;
    let mut output = String::new();
    writeln!(output, "#include <inttypes.h>").unwrap();
    writeln!(output, "#include <stdbool.h>").unwrap();
    writeln!(output, "#include <stdint.h>").unwrap();
    writeln!(output, "#include <stdio.h>").unwrap();
    writeln!(output, "#include <stdlib.h>\n").unwrap();

    writeln!(output, "enum qcore_error_tag {{").unwrap();
    writeln!(output, "    QERR_NONE = 0,").unwrap();
    for variant in cfg.error_abi().variants() {
        writeln!(
            output,
            "    QERR_{} = {},",
            upper_identifier(variant.name()),
            variant.tag()
        )
        .unwrap();
    }
    writeln!(output, "}};\n").unwrap();

    writeln!(output, "union qcore_error_payload {{").unwrap();
    for variant in cfg.error_abi().variants() {
        writeln!(output, "    int32_t {};", lower_identifier(variant.name())).unwrap();
    }
    writeln!(output, "}};\n").unwrap();
    writeln!(output, "struct qcore_error {{").unwrap();
    writeln!(output, "    enum qcore_error_tag tag;").unwrap();
    writeln!(output, "    union qcore_error_payload payload;").unwrap();
    writeln!(output, "}};\n").unwrap();

    writeln!(output, "struct qcore_result {{").unwrap();
    writeln!(output, "    bool ok;").unwrap();
    writeln!(output, "    int32_t value;").unwrap();
    writeln!(output, "    struct qcore_error error;").unwrap();
    for resource in cfg.resources() {
        writeln!(output, "    void *owned_{resource};").unwrap();
    }
    writeln!(output, "}};\n").unwrap();

    let trace_capacity = trace_capacity(cfg);
    writeln!(output, "struct qcore_trace_event {{").unwrap();
    writeln!(output, "    char kind;").unwrap();
    writeln!(output, "    uint64_t identity;").unwrap();
    writeln!(output, "}};").unwrap();
    writeln!(
        output,
        "static struct qcore_trace_event q_trace_events[{trace_capacity}];"
    )
    .unwrap();
    writeln!(output, "static size_t q_trace_length = 0;\n").unwrap();
    output.push_str(
        r"static void q_trace(char kind, uint64_t identity)
{
    if (q_trace_length >= (sizeof q_trace_events / sizeof q_trace_events[0])) {
        abort();
    }
    q_trace_events[q_trace_length].kind = kind;
    q_trace_events[q_trace_length].identity = identity;
    q_trace_length += 1;
}

static bool q_acquire(void **slot, bool succeeds, uint64_t identity)
{
    if (!succeeds) {
        return false;
    }
    *slot = malloc(1);
    if (*slot == NULL) {
        return false;
    }
    q_trace('A', identity);
    return true;
}

static void q_release(void **slot, uint64_t identity)
{
    if (*slot == NULL) {
        abort();
    }
    free(*slot);
    *slot = NULL;
    q_trace('R', identity);
}

",
    );

    writeln!(
        output,
        "static struct qcore_result qcore_fn_{}(void)",
        cfg.function()
    )
    .unwrap();
    writeln!(output, "{{").unwrap();
    writeln!(output, "    struct qcore_result q_result = {{0}};").unwrap();
    writeln!(output, "    (void)&q_acquire;").unwrap();
    writeln!(output, "    (void)&q_release;").unwrap();
    for resource in cfg.resources() {
        writeln!(output, "    void *q_res_{resource} = NULL;").unwrap();
    }
    writeln!(
        output,
        "    goto {};",
        block_label(cfg.block(cfg.entry()).unwrap())
    )
    .unwrap();

    for block in cfg.blocks() {
        writeln!(output, "{}:", block_label(block)).unwrap();
        for instruction in block.instructions() {
            match instruction {
                CfgInstruction::Release(resource) => {
                    writeln!(
                        output,
                        "    q_release(&q_res_{resource}, UINT64_C({resource}));"
                    )
                    .unwrap();
                }
                CfgInstruction::TransferToResult(resource) => {
                    writeln!(output, "    q_result.owned_{resource} = q_res_{resource};").unwrap();
                    writeln!(output, "    q_res_{resource} = NULL;").unwrap();
                    writeln!(output, "    q_trace('M', UINT64_C({resource}));").unwrap();
                }
            }
        }
        emit_terminator(&mut output, cfg, block.terminator())?;
    }
    writeln!(output, "}}\n").unwrap();

    output.push_str("int main(void)\n{\n");
    writeln!(
        output,
        "    struct qcore_result q_result = qcore_fn_{}();",
        cfg.function()
    )
    .unwrap();
    output.push_str(
        r#"    if (q_result.ok) {
        printf("OK %" PRId32 "\n", q_result.value);
    } else {
        printf("ERR %" PRIu32 "\n", (uint32_t)q_result.error.tag);
    }
    for (size_t index = 0; index < q_trace_length; index += 1) {
        printf("%c %" PRIu64 "\n", q_trace_events[index].kind,
               q_trace_events[index].identity);
    }
"#,
    );
    for resource in cfg.resources() {
        writeln!(output, "    if (q_result.owned_{resource} != NULL) {{").unwrap();
        writeln!(output, "        printf(\"P {resource}\\n\");").unwrap();
        writeln!(output, "        free(q_result.owned_{resource});").unwrap();
        writeln!(output, "        q_result.owned_{resource} = NULL;").unwrap();
        writeln!(output, "    }}").unwrap();
    }
    output.push_str("    return 0;\n}\n");
    Ok(output)
}

fn emit_terminator(
    output: &mut String,
    cfg: &TypedCfg,
    terminator: &CfgTerminator,
) -> Result<(), CEmitError> {
    match terminator {
        CfgTerminator::Goto(target) => {
            writeln!(output, "    goto {};", target_label(cfg, *target)?).unwrap();
        }
        CfgTerminator::TryAcquire {
            resource,
            succeeds,
            success,
            failure,
        } => {
            writeln!(
                output,
                "    if (q_acquire(&q_res_{resource}, {}, UINT64_C({resource}))) {{",
                c_bool(*succeeds)
            )
            .unwrap();
            writeln!(output, "        goto {};", target_label(cfg, *success)?).unwrap();
            writeln!(output, "    }} else {{").unwrap();
            writeln!(output, "        goto {};", target_label(cfg, *failure)?).unwrap();
            writeln!(output, "    }}").unwrap();
        }
        CfgTerminator::Check {
            passes,
            success,
            failure,
        } => {
            writeln!(output, "    if ({}) {{", c_bool(*passes)).unwrap();
            writeln!(output, "        goto {};", target_label(cfg, *success)?).unwrap();
            writeln!(output, "    }} else {{").unwrap();
            writeln!(output, "        goto {};", target_label(cfg, *failure)?).unwrap();
            writeln!(output, "    }}").unwrap();
        }
        CfgTerminator::ReturnOk(value) => {
            writeln!(output, "    q_result.ok = true;").unwrap();
            writeln!(output, "    q_result.value = {};", c_i32(*value)).unwrap();
            writeln!(output, "    return q_result;").unwrap();
        }
        CfgTerminator::ReturnError(tag) => {
            let variant = cfg
                .error_abi()
                .variant_by_tag(*tag)
                .ok_or(CEmitError::UnknownErrorTag(*tag))?;
            writeln!(output, "    q_result.ok = false;").unwrap();
            writeln!(
                output,
                "    q_result.error.tag = QERR_{};",
                upper_identifier(variant.name())
            )
            .unwrap();
            writeln!(
                output,
                "    q_result.error.payload.{} = INT32_C(0);",
                lower_identifier(variant.name())
            )
            .unwrap();
            writeln!(output, "    return q_result;").unwrap();
        }
    }
    Ok(())
}

fn target_label(cfg: &TypedCfg, id: usize) -> Result<String, CEmitError> {
    cfg.block(id)
        .map(block_label)
        .ok_or(CEmitError::MissingBlock(id))
}

fn block_label(block: &BasicBlock) -> String {
    if block.is_cleanup() {
        format!("q_cleanup_{}", block.id())
    } else {
        format!("q_bb_{}", block.id())
    }
}

fn trace_capacity(cfg: &TypedCfg) -> usize {
    let instructions = cfg
        .blocks()
        .map(|block| block.instructions().len())
        .sum::<usize>();
    let acquisitions = cfg
        .blocks()
        .filter(|block| matches!(block.terminator(), CfgTerminator::TryAcquire { .. }))
        .count();
    instructions.saturating_add(acquisitions).saturating_add(8)
}

fn validate_c_symbols(cfg: &TypedCfg) -> Result<(), CEmitError> {
    let mut enum_names = BTreeSet::new();
    let mut field_names = BTreeSet::new();
    for variant in cfg.error_abi().variants() {
        let enum_name = upper_identifier(variant.name());
        let field_name = lower_identifier(variant.name());
        if !enum_names.insert(enum_name.clone()) {
            return Err(CEmitError::CollidingErrorSymbol(enum_name));
        }
        if !field_names.insert(field_name.clone()) {
            return Err(CEmitError::CollidingErrorSymbol(field_name));
        }
    }
    Ok(())
}

fn upper_identifier(name: &str) -> String {
    name.chars()
        .map(|character| character.to_ascii_uppercase())
        .collect()
}

fn lower_identifier(name: &str) -> String {
    name.chars()
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

const fn c_bool(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn c_i32(value: i32) -> String {
    if value == i32::MIN {
        "INT32_MIN".into()
    } else if value < 0 {
        format!("(-INT32_C({}))", -value)
    } else {
        format!("INT32_C({value})")
    }
}

pub fn parse_c_observation(
    stdout: &str,
    error_abi: &ErrorAbi,
) -> Result<ExecutionObservation, CEmitError> {
    let mut lines = stdout.lines();
    let outcome_line = lines.next().ok_or(CEmitError::MalformedOutput)?;
    let mut outcome_parts = outcome_line.split_whitespace();
    let kind = outcome_parts.next().ok_or(CEmitError::MalformedOutput)?;
    let number = outcome_parts.next().ok_or(CEmitError::MalformedOutput)?;
    if outcome_parts.next().is_some() {
        return Err(CEmitError::MalformedOutput);
    }
    let mut payload = Vec::new();
    let mut trace = Vec::new();
    for line in lines {
        let mut parts = line.split_whitespace();
        let event = parts.next().ok_or(CEmitError::MalformedOutput)?;
        let resource = parts
            .next()
            .ok_or(CEmitError::MalformedOutput)?
            .parse::<ResourceId>()
            .map_err(|_| CEmitError::MalformedOutput)?;
        if parts.next().is_some() {
            return Err(CEmitError::MalformedOutput);
        }
        match event {
            "A" => trace.push(TraceEvent::Acquire(resource)),
            "R" => trace.push(TraceEvent::Release(resource)),
            "M" => trace.push(TraceEvent::Transfer(resource)),
            "P" => payload.push(resource),
            _ => return Err(CEmitError::MalformedOutput),
        }
    }
    let outcome = match kind {
        "OK" => ExecutionOutcome::Ok {
            value: number
                .parse::<i32>()
                .map_err(|_| CEmitError::MalformedOutput)?,
            payload,
        },
        "ERR" => {
            if !payload.is_empty() {
                return Err(CEmitError::PayloadOnErrorOutput);
            }
            let tag = number
                .parse::<ErrorTag>()
                .map_err(|_| CEmitError::MalformedOutput)?;
            let variant = error_abi
                .variant_by_tag(tag)
                .ok_or(CEmitError::UnknownErrorTag(tag))?;
            ExecutionOutcome::Error {
                name: variant.name().into(),
                tag,
            }
        }
        _ => return Err(CEmitError::MalformedOutput),
    };
    Ok(ExecutionObservation { outcome, trace })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CEmitError {
    MissingBlock(usize),
    UnknownErrorTag(ErrorTag),
    CollidingErrorSymbol(String),
    MalformedOutput,
    PayloadOnErrorOutput,
}

impl fmt::Display for CEmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CEmitError {}

#[cfg(test)]
#[path = "../tests/unit/c_emit.rs"]
mod tests;
