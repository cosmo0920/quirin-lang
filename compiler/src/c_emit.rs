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
        "static void q_trace(char kind, uint64_t identity)\n\
         {\n\
             if (q_trace_length >= (sizeof q_trace_events / sizeof q_trace_events[0])) {\n\
                 abort();\n\
             }\n\
             q_trace_events[q_trace_length].kind = kind;\n\
             q_trace_events[q_trace_length].identity = identity;\n\
             q_trace_length += 1;\n\
         }\n\n\
         static bool q_acquire(void **slot, bool succeeds, uint64_t identity)\n\
         {\n\
             if (!succeeds) {\n\
                 return false;\n\
             }\n\
             *slot = malloc(1);\n\
             if (*slot == NULL) {\n\
                 return false;\n\
             }\n\
             q_trace('A', identity);\n\
             return true;\n\
         }\n\n\
         static void q_release(void **slot, uint64_t identity)\n\
         {\n\
             if (*slot == NULL) {\n\
                 abort();\n\
             }\n\
             free(*slot);\n\
             *slot = NULL;\n\
             q_trace('R', identity);\n\
         }\n\n",
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
        "    if (q_result.ok) {\n\
             printf(\"OK %\" PRId32 \"\\n\", q_result.value);\n\
         } else {\n\
             printf(\"ERR %\" PRIu32 \"\\n\", (uint32_t)q_result.error.tag);\n\
         }\n\
         for (size_t index = 0; index < q_trace_length; index += 1) {\n\
             printf(\"%c %\" PRIu64 \"\\n\", q_trace_events[index].kind,\n\
                    q_trace_events[index].identity);\n\
         }\n",
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
mod tests {
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::cfg::interpret_cfg;
    use crate::cleanup_lowering::{SourceOp, SourceProgram, interpret_source, lower_cleanup_cfg};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn corpus() -> Vec<SourceProgram> {
        vec![
            SourceProgram::new(
                "normal_transfer",
                vec![1],
                vec!["IoError".into()],
                vec![
                    SourceOp::Acquire {
                        resource: 1,
                        succeeds: true,
                        error: "IoError".into(),
                    },
                    SourceOp::TransferToResult(1),
                    SourceOp::Return(23),
                ],
            )
            .unwrap(),
            SourceProgram::new(
                "partial_failure",
                vec![1, 2],
                vec!["IoError".into()],
                vec![
                    SourceOp::Acquire {
                        resource: 1,
                        succeeds: true,
                        error: "IoError".into(),
                    },
                    SourceOp::Acquire {
                        resource: 2,
                        succeeds: false,
                        error: "IoError".into(),
                    },
                    SourceOp::Drop(2),
                    SourceOp::Drop(1),
                    SourceOp::Return(0),
                ],
            )
            .unwrap(),
            SourceProgram::new(
                "lifo_failure",
                vec![7, 9],
                vec!["IoError".into(), "ParseError".into()],
                vec![
                    SourceOp::Acquire {
                        resource: 7,
                        succeeds: true,
                        error: "IoError".into(),
                    },
                    SourceOp::Acquire {
                        resource: 9,
                        succeeds: true,
                        error: "IoError".into(),
                    },
                    SourceOp::Check {
                        passes: false,
                        error: "ParseError".into(),
                    },
                    SourceOp::Drop(9),
                    SourceOp::Drop(7),
                    SourceOp::Return(0),
                ],
            )
            .unwrap(),
            SourceProgram::new(
                "signed_value",
                vec![],
                vec!["IoError".into()],
                vec![SourceOp::Return(i32::MIN)],
            )
            .unwrap(),
        ]
    }

    #[test]
    fn emitted_c_is_readable_and_has_tagged_abi_and_cleanup_labels() {
        let cfg = lower_cleanup_cfg(&corpus().remove(1)).unwrap();
        let c = emit_c11(&cfg).unwrap();
        assert!(c.contains("enum qcore_error_tag"));
        assert!(c.contains("union qcore_error_payload"));
        assert!(c.contains("q_cleanup_"));
        assert!(c.contains("q_release(&q_res_1"));
    }

    #[test]
    fn compiled_instrumented_c_matches_source_and_cfg() {
        for source in corpus() {
            let cfg = lower_cleanup_cfg(&source).unwrap();
            let reference = interpret_source(&source).unwrap();
            assert_eq!(interpret_cfg(&cfg).unwrap(), reference);
            let c_observation = compile_and_run(&cfg);
            assert_eq!(c_observation, reference);
        }
    }

    fn compile_and_run(cfg: &TypedCfg) -> ExecutionObservation {
        let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "qcore-cycle7-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        let source_path = directory.join("generated.c");
        let binary_path = directory.join(if cfg!(target_env = "msvc") {
            "generated.exe"
        } else {
            "generated"
        });
        fs::write(&source_path, emit_c11(cfg).unwrap()).unwrap();
        let flags: &[&str] = if cfg!(target_env = "msvc") {
            // /RTC1 enables the native debug runtime's stack-frame and
            // uninitialized-use checks. /sdl adds Microsoft's stricter
            // security diagnostics and runtime checks. The cc configuration
            // below adds /W4 and /WX without first injecting /W0.
            &["/std:c11", "/TC", "/sdl", "/RTC1"]
        } else {
            &[
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-fsanitize=address,undefined",
                "-fno-omit-frame-pointer",
            ]
        };
        let rust_target = rust_host_target();
        let mut build = cc::Build::new();
        build
            .host(&rust_target)
            .target(&rust_target)
            .file(&source_path)
            .out_dir(&directory)
            .opt_level(0)
            .debug(false)
            .cargo_metadata(false)
            .flags(flags);
        if cfg!(target_env = "msvc") {
            build.warnings(true).warnings_into_errors(true);
        } else {
            build.warnings(false);
        }
        let compiler = build.get_compiler();
        if cfg!(target_env = "msvc") {
            assert!(
                compiler.is_like_msvc() && !compiler.is_like_clang_cl(),
                "Cycle 7.5 requires the native MSVC compiler selected by cc, got {}",
                compiler.path().display()
            );
        }
        build.try_compile("qcore_generated").unwrap();

        let mut compile_command = compiler.to_command();
        compile_command.current_dir(&directory).arg(&source_path);
        if compiler.is_like_msvc() {
            compile_command.arg(format!("/Fe:{}", binary_path.display()));
        } else {
            compile_command.arg("-o").arg(&binary_path);
        }
        let compile = compile_command.output().unwrap();
        assert!(
            compile.status.success(),
            "C compiler rejected generated code:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );
        let mut run_command = Command::new(&binary_path);
        if !cfg!(target_env = "msvc") {
            // LeakSanitizer cannot run under the process supervisor used by
            // some CI sandboxes; AddressSanitizer and UBSan remain enabled.
            run_command
                .env("ASAN_OPTIONS", "detect_leaks=0:halt_on_error=1")
                .env("UBSAN_OPTIONS", "halt_on_error=1");
        }
        let run = run_command.output().unwrap();
        assert!(
            run.status.success(),
            "generated code failed under runtime instrumentation:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let observation =
            parse_c_observation(std::str::from_utf8(&run.stdout).unwrap(), cfg.error_abi())
                .unwrap();
        fs::remove_dir_all(directory).unwrap();
        observation
    }

    fn rust_host_target() -> String {
        let version = Command::new("rustc").arg("-vV").output().unwrap();
        assert!(version.status.success());
        std::str::from_utf8(&version.stdout)
            .unwrap()
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .expect("rustc -vV must report its host target")
            .into()
    }
}
