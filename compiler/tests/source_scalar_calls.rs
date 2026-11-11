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

use qcore_compiler::{
    CheckedSourceModule, CompileOptions, SourceExecutionProfile, SourceFile, compile_files,
};
use qcore_core::{
    c_program::{self, Entry},
    io_emit, scalar_calls_c_emit,
};
#[cfg(windows)]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

fn native_run(source: &str, tag: &str) -> std::process::Output {
    #[cfg(windows)]
    {
        let _ = tag;
        windows_c::run_msvc(source, None, &[])
    }
    #[cfg(unix)]
    {
        use std::{fs, process::Command};
        let dir = std::env::temp_dir().join(format!("qcore-scalar-{tag}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("main.c"), source).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
        let built = Command::new(compiler)
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-fsanitize=address,undefined",
                "-fno-sanitize-recover=all",
            ])
            .arg(dir.join("main.c"))
            .arg("-o")
            .arg(dir.join("main"))
            .output()
            .expect("Clang is required for this native check");
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        let output = Command::new(dir.join("main")).output().unwrap();
        fs::remove_dir_all(dir).unwrap();
        output
    }
}
use qcore_core::{
    cfg::ExecutionOutcome,
    scalar_calls::{self, Op},
};
fn compile(s: &str) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("scalar.qcore", s)],
        &CompileOptions::default(),
    )
}
fn value(s: &str) -> i32 {
    let m = compile(s).unwrap();
    assert_eq!(m.execution_profile, SourceExecutionProfile::ScalarCallsV1);
    match m.reference_observation.unwrap().outcome {
        ExecutionOutcome::Ok { value, .. } => value,
        _ => panic!("normal scalar outcome"),
    }
}
fn observation(s: &str) -> scalar_calls::Observation {
    let m = compile(s).unwrap();
    assert_eq!(m.execution_profile, SourceExecutionProfile::ScalarCallsV1);
    let CheckedSourceModule::ScalarCalls(checked) = &m.checked else {
        panic!("scalar-call checker result")
    };
    let entry = checked.program().functions.len() - 1;
    checked.observe(entry, &[]).unwrap()
}
fn refuse(s: &str, needle: &str) {
    let e = compile(s).unwrap_err();
    assert!(e.iter().any(|e| e.message.contains(needle)), "{e:?}");
}
#[test]
fn arguments_and_return_values_are_not_erased() {
    assert_eq!(value(include_str!("../../examples/scalar/calls.qr")), 42);
    for n in [-50, 0, 10, 41] {
        assert_eq!(
            value(&format!(
                "fn inc(x: i32) -> i32 {{ return x+1; }} fn main() -> i32 {{ return inc({n}); }}"
            )),
            n + 1
        );
    }
}
#[test]
fn arithmetic_precedence_associativity_and_signed_division() {
    for (expr, expected) in [
        ("2+3*4", 14),
        ("(2+3)*4", 20),
        ("20-5-3", 12),
        ("-7/3", -2),
        ("-7%3", -1),
        ("-(2+3)", -5),
        ("-2147483648", i32::MIN),
        ("2147483647", i32::MAX),
    ] {
        assert_eq!(
            value(&format!(
                "fn id(x: i32) -> i32 {{ return x; }} fn main() -> i32 {{ return id({expr}); }}"
            )),
            expected
        );
    }
}
#[test]
fn arithmetic_faults_refuse_the_closed_entry() {
    for expr in [
        "2147483647+1",
        "-2147483648-1",
        "50000*50000",
        "1/0",
        "1%0",
        "-2147483648/-1",
        "-2147483648%-1",
        "-(-2147483648)",
    ] {
        refuse(
            &format!("fn main() -> i32 {{ return {expr}; }}"),
            "arithmetic",
        );
    }
}
#[test]
fn malformed_call_and_result_types_refuse() {
    for (s, n) in [
        (
            "fn f(x:i32)->i32{return x;} fn main()->i32{return f();}",
            "argument count",
        ),
        (
            "fn f()->unit{return;} fn main()->i32{return f();}",
            "return type",
        ),
        (
            "fn f()->unit{return;} fn main()->i32{return f()+1;}",
            "expected i32",
        ),
        (
            "fn f(x:i32)->i32{return x;} fn main()->i32{return f(1,2);}",
            "argument count",
        ),
        (
            "fn f()->unit{return;} fn main()->i32{let x=f();return 0;}",
            "locals require i32",
        ),
        (
            "fn f()->i32{return 1;} fn main()->i32{let f:i32=2;return f();}",
            "local cannot be called",
        ),
        (
            "fn main()->i32{let x:i32=1;x=2;return x;}",
            "only calls may be discarded",
        ),
        (
            "fn main()->i32{let x:i32=1;return x;let y=2;}",
            "RightBrace",
        ),
        ("fn main()->i32{let x:i32=1;}", "final explicit return"),
    ] {
        refuse(s, n);
    }
}
#[test]
fn every_function_is_checked_including_uncalled_bodies() {
    refuse(
        "fn bad()->i32{return bad()+1;} fn main()->i32{return 0;}",
        "recursive",
    );
    refuse(
        "fn a(x:i32)->i32{return b(x);} fn b(x:i32)->i32{return a(x);} fn main()->i32{return 0;}",
        "recursive",
    );
    refuse(
        "fn bad(x:i32)->unit{return x;} fn main()->i32{return 0;}",
        "return type",
    );
}
#[test]
fn unit_calls_and_empty_annotations_are_explicit() {
    assert_eq!(
        value(
            "fn f(x:i32)->unit effects {} contract {} {let y=x+1;return;} fn main()->unit {f(4);return;}"
        ),
        0
    );
    refuse(
        "fn f(x:i32)->i32 effects {alloc} {return x;} fn main()->i32{return f(0);}",
        "only the io effect",
    );
    refuse(
        "fn f(x:i32)->i32 contract {ensures normal true;} {return x;} fn main()->i32{return f(0);}",
        "no scalar-call consumer",
    );
}
#[test]
fn print_requires_std_io_and_an_i32_operand() {
    refuse("fn main()->i32{let f=42;print(f);return f;}", "std.io");
    refuse(
        "#import std.io; fn unit()->unit{return;} fn main()->i32{print(unit());return 0;}",
        "print requires i32",
    );
    refuse(
        "#import std.io; fn main()->i32{print(missing);return 0;}",
        "unknown name",
    );
}
#[test]
fn print_effect_is_inferred_or_declared_and_propagates_to_callers() {
    let inferred =
        "#import std.io; fn show(x:i32)->i32{print(x);return x;} fn main()->i32{return show(42);}";
    let observed = observation(inferred);
    assert_eq!(observed.value, Some(42));
    assert_eq!(observed.stdout, vec![42]);

    assert_eq!(
        value(
            "#import std.io; fn show(x:i32)->i32 effects {io}{print(x);return x;} fn main()->i32 effects {io}{return show(42);}"
        ),
        42
    );
    refuse(
        "#import std.io; fn show(x:i32)->i32{print(x);return x;} fn main()->i32 effects {}{return show(42);}",
        "io effect required",
    );
    refuse(
        "#import std.io; fn show(x:i32)->i32 effects {}{print(x);return x;} fn main()->i32{return show(42);}",
        "io effect required",
    );
}
#[test]
fn print_observation_preserves_argument_and_statement_order() {
    let observed = observation(
        "#import std.io; fn show(x:i32)->i32{print(x);return x;} fn add(a:i32,b:i32)->i32{return a+b;} fn main()->i32{let f=add(show(20),show(22));print(f);return f;}",
    );
    assert_eq!(observed.value, Some(42));
    assert_eq!(observed.stdout, vec![20, 22, 42]);
}
#[test]
fn entry_free_library_checks_types_without_claiming_arithmetic_safety() {
    let m = compile("module library; fn inc(x:i32)->i32{return x+1;}").unwrap();
    assert!(m.c_source.is_none());
    assert!(m.reference_observation.is_none());
    let CheckedSourceModule::ScalarCalls(c) = m.checked else {
        panic!()
    };
    assert_eq!(c.run(0, &[41]).unwrap(), Some(42));
    assert!(c.run(0, &[i32::MAX]).is_err());
}
#[test]
fn old_profiles_stay_selected() {
    let m = compile("fn main()->i32{return 0;}").unwrap();
    assert_eq!(
        m.execution_profile,
        SourceExecutionProfile::SequentialCleanupV1
    );
    let m=compile("error Failure { Failed; } fn main()->i32{let p=qcore_acquire(1,true,Failure);drop p;return 0;}").unwrap();
    assert_eq!(
        m.execution_profile,
        SourceExecutionProfile::SequentialCleanupV1
    );
}
#[test]
fn multi_file_calls_and_explicit_entry() {
    let m = compile_files(
        &[
            SourceFile::new(
                "lib.qcore",
                "module library; fn inc(x:i32)->i32{return x+1;}",
            ),
            SourceFile::new("app.qcore", "fn start()->i32{return inc(8);}"),
        ],
        &CompileOptions {
            entry: Some("start".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(m.checked.function_count(), 2);
    assert_eq!(m.entry.as_deref(), Some("start"));
}
#[test]
fn bounded_arithmetic_matches_widened_reference() {
    let xs = [
        i32::MIN,
        i32::MIN + 1,
        -65536,
        -7,
        -1,
        0,
        1,
        3,
        65536,
        i32::MAX,
    ];
    for a in xs {
        for b in xs {
            for op in [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Rem] {
                let wide = match op {
                    Op::Add => Some(i64::from(a) + i64::from(b)),
                    Op::Sub => Some(i64::from(a) - i64::from(b)),
                    Op::Mul => Some(i64::from(a) * i64::from(b)),
                    Op::Div | Op::Rem if b == 0 || (a == i32::MIN && b == -1) => None,
                    Op::Div => Some(i64::from(a) / i64::from(b)),
                    Op::Rem => Some(i64::from(a) % i64::from(b)),
                };
                assert_eq!(
                    scalar_calls::arithmetic(op, a, b),
                    wide.and_then(|n| i32::try_from(n).ok())
                );
            }
        }
    }
}
#[test]
fn missing_names_and_boolean_operands_refuse() {
    assert!(compile("fn main()->i32{return missing+1;}").is_err());
    assert!(compile("fn main()->i32{return true+1;}").is_err());
}
#[test]
fn native_calls_match_reference() {
    let m = compile(include_str!("../../examples/scalar/calls.qr")).unwrap();
    let source = m
        .c_source
        .unwrap()
        .replace("int main(void)", "int checked_entry(void)")
        + "\nint main(void) { return q_scalar_fn_2() == 42 ? 0 : 87; }\n";
    assert!(source.contains("q_scalar_fn_0(v") || source.contains("q_scalar_fn_0(t"));
    let o = native_run(&source, "calls");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(o.stdout.is_empty());
    assert!(o.stderr.is_empty());
}

#[test]
fn composed_printing_emits_one_main_and_uses_the_shared_io_path() {
    let m = compile(
        "#import std.io; fn show(x:i32)->i32{print(x);return x;} fn add(a:i32,b:i32)->i32{return a+b;} fn main()->i32{let f=add(show(20),show(22));print(f);return f;}",
    )
    .unwrap();
    let CheckedSourceModule::ScalarCalls(checked) = &m.checked else {
        panic!("scalar-call checker result")
    };
    let fragments = scalar_calls_c_emit::fragments(checked);
    assert!(
        fragments
            .iter()
            .all(|fragment| c_program::entry_definitions(&fragment.text).is_empty())
    );
    let shared_io = [
        io_emit::stdio_include_fragment(),
        io_emit::io_prelude_fragment(),
    ];
    let merged = c_program::merge(&[&fragments, &shared_io]);
    assert!(c_program::conflicting_keys(&merged).is_empty());
    assert_eq!(
        merged
            .iter()
            .filter(|fragment| fragment.key == io_emit::IO_PRELUDE_KEY)
            .count(),
        1
    );
    let mut body = scalar_calls_c_emit::entry_body(checked, 2).unwrap();
    body.push_str("    ");
    body.push_str(&io_emit::write_line(
        qcore_core::std_io::Stream::Out,
        "%\" PRId32 \"",
        &["INT32_C(99)"],
    ));
    body.push('\n');
    assert!(c_program::entry_definitions(&body).is_empty());
    let source = c_program::assemble(&merged, &Entry::source_route(body));
    assert_eq!(c_program::entry_definitions(&source).len(), 1);
    assert!(io_emit::direct_stdio_calls(&source).is_empty());

    let o = native_run(&source, "compose");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(o.stdout, b"20\n22\n42\n99\n");
    assert!(o.stderr.is_empty());
}

#[test]
fn expression_call_graph_and_execution_bounds_refuse() {
    let expression = (0..70).map(|_| "1").collect::<Vec<_>>().join("+");
    refuse(
        &format!("fn main()->i32{{return {expression};}}"),
        "depth bound",
    );
    let functions = (0..65)
        .map(|i| {
            format!(
                "fn f{i}(x:i32)->i32{{return {};}}",
                if i == 64 {
                    "x".into()
                } else {
                    format!("f{}(x)", i + 1)
                }
            )
        })
        .collect::<String>();
    refuse(
        &format!("{functions}fn main()->i32{{return f0(1);}}"),
        "call depth",
    );
    let functions = (0..18)
        .map(|i| {
            format!(
                "fn f{i}(x:i32)->i32{{return {};}}",
                if i == 17 {
                    "x".into()
                } else {
                    format!("f{}(x)+f{}(x)", i + 1, i + 1)
                }
            )
        })
        .collect::<String>();
    refuse(
        &format!("{functions}fn main()->i32{{return f0(0);}}"),
        "step bound",
    );
}
#[test]
fn lean_arithmetic_vectors_match_the_rust_kernel() {
    let text = include_str!("../../proof/QCore/ScalarCalls.lean");
    let table = text
        .split("def agreement :")
        .nth(1)
        .unwrap()
        .split("theorem agreement_holds")
        .next()
        .unwrap();
    let mut count = 0;
    for row in table.split("(.").skip(1) {
        let fields = row.split(',').take(4).collect::<Vec<_>>();
        let op = match fields[0].trim() {
            "add" => Op::Add,
            "sub" => Op::Sub,
            "mul" => Op::Mul,
            "div" => Op::Div,
            "rem" => Op::Rem,
            _ => panic!("unknown Lean operation"),
        };
        let a = fields[1].trim().parse().unwrap();
        let b = fields[2].trim().parse().unwrap();
        let raw = fields[3].trim();
        let expected = if raw.starts_with("none") {
            None
        } else {
            let raw = raw.strip_prefix("some ").unwrap();
            let digits = raw
                .trim_start_matches('(')
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '-')
                .collect::<String>();
            Some(digits.parse().unwrap())
        };
        assert_eq!(scalar_calls::arithmetic(op, a, b), expected);
        count += 1;
    }
    assert_eq!(count, 8);
}
#[test]
fn intrinsic_prefix_is_not_a_new_reserved_namespace() {
    assert_eq!(
        value("fn qcore_user(x:i32)->i32{return x+1;} fn main()->i32{return qcore_user(8);}"),
        9
    );
}
#[test]
fn pure_extension_does_not_claim_existing_branch_or_usage_syntax() {
    let e = compile("fn main()->i32{while(true){return 1;}}").unwrap_err();
    assert_eq!(e[0].code, "Q500");
    let m =
        compile("fn discard(x:i32 use affine)->i32{return 0;} fn main()->i32{return 0;}").unwrap();
    assert_eq!(
        m.execution_profile,
        SourceExecutionProfile::SequentialCleanupV1
    );
    let e =
        compile("fn discard(x:i32 use one)->i32{return 0;} fn main()->i32{return 0;}").unwrap_err();
    assert_eq!(e[0].code, "Q100");
}

#[test]
fn native_arithmetic_guards_refuse_faults_in_uncalled_helpers() {
    for (index, (body, args)) in [
        ("x+1", "INT32_MAX"),
        ("x/-1", "INT32_MIN"),
        ("x%-1", "INT32_MIN"),
        ("1/x", "0"),
        ("-x", "INT32_MIN"),
    ]
    .into_iter()
    .enumerate()
    {
        let m = compile(&format!(
            "fn helper(x:i32)->i32{{return {body};}} fn main()->i32{{return helper(1);}}"
        ))
        .unwrap();
        // A negative native harness enters the emitted helper with inputs the
        // closed program did not supply; the backend must still avoid UB.
        let source = m
            .c_source
            .unwrap()
            .replace("int main(void)", "int checked_entry(void)")
            + &format!("\nint main(void) {{ (void)q_scalar_fn_0({args}); return 0; }}\n");
        let o = native_run(&source, &format!("guard-{index}"));
        assert_eq!(o.status.code(), Some(86));
        assert!(o.stderr.is_empty());
        assert!(o.stdout.is_empty());
    }
}
