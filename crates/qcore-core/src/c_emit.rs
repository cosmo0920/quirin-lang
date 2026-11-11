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

//! C11 emission and translation-validation support for cleanup-explicit CFGs.

use std::collections::BTreeSet;
use std::fmt::{self, Write as _};

use crate::ResourceId;
use crate::c_program::{self, Fragment, Section};
use crate::cfg::{
    BasicBlock, CfgInstruction, CfgTerminator, ErrorAbi, ErrorTag, ExecutionObservation,
    ExecutionOutcome, TraceEvent, TypedCfg,
};
use crate::cleanup_lowering::{BranchCondition, IntExpr};
use crate::diagnostic_log_c_emit;

/// Emit a standalone, strictly conforming C11 translation unit.
///
/// The generated program is quiet unless the source program explicitly uses a
/// standard-output module.  Runtime custody bookkeeping is not a user-visible
/// logging facility and therefore never writes a synthesized trace.
pub fn emit_c11(cfg: &TypedCfg) -> Result<String, CEmitError> {
    emit_c11_with_log_level(cfg, None)
}

/// Emit C11 with an explicitly selected hosted system-event adapter.
pub fn emit_c11_with_log_level(
    cfg: &TypedCfg,
    log_level: Option<crate::std_io::Level>,
) -> Result<String, CEmitError> {
    validate_c_symbols(cfg)?;
    let fragments = preamble(log_level);
    // The error ABI, the result structure, the runtime helpers and every
    // emitted function body: one file-scope fragment, built here.
    let mut output = String::from("\n");

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

    output.push_str(
        r"static void q_trace(char kind, uint64_t identity)
{
    enum qcore_diagnostic_kind diagnostic_kind;
    switch (kind) {
    case 'A': diagnostic_kind = QCORE_DIAG_RESOURCE_ACQUIRE; break;
    case 'R': diagnostic_kind = QCORE_DIAG_RESOURCE_RELEASE; break;
    case 'M': diagnostic_kind = QCORE_DIAG_RESOURCE_MOVE; break;
    default: abort();
    }
    qcore_diagnostic_log(QCORE_LOG_TRACE, diagnostic_kind, identity,
        UINT64_C(0), INT64_C(0), (const char *)0);
}

static bool q_acquire(void **slot, bool succeeds, uint64_t identity)
{
    if (!succeeds) {
        qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_RESOURCE_ACQUIRE_FAILED,
            identity, UINT64_C(0), INT64_C(0), (const char *)0);
        return false;
    }
    *slot = malloc(1);
    if (*slot == NULL) {
        qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_RESOURCE_ACQUIRE_FAILED,
            identity, UINT64_C(0), INT64_C(0), (const char *)0);
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

    let mut body = String::new();
    writeln!(
        body,
        "    struct qcore_result q_result = qcore_fn_{}();",
        cfg.function()
    )
    .unwrap();
    writeln!(body, "    (void) q_result.value;").unwrap();
    writeln!(body, "    (void) q_result.error;").unwrap();
    writeln!(body, "    qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_RUN_OUTCOME, UINT64_C(0), UINT64_C(0), q_result.ok ? (int64_t)q_result.value : (int64_t)q_result.error.tag, q_result.ok ? \"ok\" : \"error\");").unwrap();
    for resource in cfg.resources() {
        writeln!(body, "    if (q_result.owned_{resource} != NULL) {{").unwrap();
        writeln!(body, "        free(q_result.owned_{resource});").unwrap();
        writeln!(body, "        q_result.owned_{resource} = NULL;").unwrap();
        writeln!(body, "    }}").unwrap();
    }

    let mut definitions = fragments;
    definitions.push(Fragment::new(
        Section::FileScope,
        "sequential.definitions",
        output,
    ));
    Ok(c_program::assemble(
        &definitions,
        &c_program::Entry {
            // No leading blank line: this route's file-scope text already ends
            // in one, from the blank line after the entry function's body.
            lead: String::new(),
            brace: c_program::BraceStyle::Allman,
            prologue: String::new(),
            body,
            epilogue: "    return 0;\n}\n".to_owned(),
        },
    ))
}

/// The file-scope text the sequential route contributes to a program it is part
/// of (card `C-EMIT-MAIN-UNIFY`).
///
/// The hosted stdout adapter's declaration and definition are contributed only
/// for a module that selected one, exactly as before: ordinary emission selects
/// the null sink and needs no stdio.
#[must_use]
pub fn preamble(log_level: Option<crate::std_io::Level>) -> Vec<Fragment> {
    let mut fragments = vec![Fragment::new(
        Section::Includes,
        "sequential.includes",
        "#include <inttypes.h>\n#include <stdbool.h>\n#include <stdint.h>\n#include <stdlib.h>\n",
    )];
    if let Some(level) = log_level {
        fragments.push(diagnostic_log_c_emit::adapter_declaration_fragment(level));
    }
    fragments.push(diagnostic_log_c_emit::diagnostic_log_fragment());
    if log_level.is_some() {
        fragments.push(diagnostic_log_c_emit::adapter_definition_fragment());
    }
    fragments
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
        CfgTerminator::Branch {
            condition,
            then_block,
            else_block,
        } => {
            // The emitted program computes the condition itself rather than
            // receiving the decided literal: that makes the reference / CFG /
            // compiled-C observation agreement a real cross-check of the Rust
            // evaluator against a C compiler instead of a tautology.
            writeln!(output, "    if ({}) {{", c_condition(condition)).unwrap();
            writeln!(output, "        goto {};", target_label(cfg, *then_block)?).unwrap();
            writeln!(output, "    }} else {{").unwrap();
            writeln!(output, "        goto {};", target_label(cfg, *else_block)?).unwrap();
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

/// The C11 spelling of a `source.branch.v1` condition (card `IF-STATEMENT`).
fn c_condition(condition: &BranchCondition) -> String {
    match condition {
        BranchCondition::Literal(value) => c_bool(*value).to_owned(),
        BranchCondition::Compare {
            left,
            operator,
            right,
        } => format!(
            "{} {} {}",
            c_int_expr(left),
            operator.c_operator(),
            c_int_expr(right)
        ),
    }
}

/// The C11 spelling of one comparison operand (card `BITWISE-CONDITIONS`).
///
/// A bit operation is emitted **fully parenthesized**, which is what keeps the
/// grouping this tier declared from depending on the C compiler's own
/// precedence table.  A plain literal is emitted exactly as card `IF-STATEMENT`
/// emitted it, so a program that writes no bit operation moves no byte.
fn c_int_expr(expression: &IntExpr) -> String {
    match expression {
        IntExpr::Literal(value) => c_i32(*value),
        IntExpr::Bitwise {
            operator,
            left,
            right,
        } => format!(
            "({} {} {})",
            c_int_expr(left),
            operator.c_operator(),
            c_int_expr(right)
        ),
    }
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
