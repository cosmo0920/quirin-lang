//! Actual held cJSON source -> typed projection + live native heap observations.
//! Usage: SOURCE_C HEADER OUTPUT_DIRECTORY (fresh directory required).
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, CCompilerDialect, CSourceInput, prepare_translation_unit,
};
use qcore_schemas::hypothesis::{Context, Graph};
use quirin_source_analysis::cjson_local_binding::{self as binding, NativeCase};
use serde_json::json;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn main() -> Result<(), String> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        return Err("SOURCE_C HEADER FRESH_OUTPUT_DIRECTORY".into());
    }
    let out = PathBuf::from(&args[3]);
    fs::create_dir(&out).map_err(|e| e.to_string())?;
    let out = fs::canonicalize(out).map_err(|e| e.to_string())?;
    let original = fs::read(&args[1]).map_err(|e| e.to_string())?;
    let header = fs::read(&args[2]).map_err(|e| e.to_string())?;
    fs::write(out.join("cJSON.c"), &original).map_err(|e| e.to_string())?;
    fs::write(out.join("cJSON.h"), &header).map_err(|e| e.to_string())?;
    let harness =
        include_bytes!("../../../docs/reports/premise-local-source-bindings/native/cjson_heap.c");
    fs::write(out.join("heap.c"), harness).map_err(|e| e.to_string())?;
    let source = std::str::from_utf8(&original).map_err(|e| e.to_string())?;
    let unit = C11TranslationUnit {
        source,
        include_paths: vec![out.to_string_lossy().into()],
        dialect: CCompilerDialect::C11,
        input: CSourceInput::DeclaredFile {
            spelled: "cJSON.c".into(),
            directory: out.clone(),
        },
        ..Default::default()
    };
    let prepared = prepare_translation_unit(&unit);
    let pp = prepared.preprocessed_source().map_err(|e| e.to_string())?;
    fs::write(out.join("prepared.c"), pp).map_err(|e| e.to_string())?;
    let clang = qcore_c_gateway::clang_command()?;
    let compiler = clang.get_program().to_string_lossy().into_owned();
    let compiler_version = Command::new(&compiler)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;
    fs::write(out.join("clang-version.txt"), &compiler_version.stdout)
        .map_err(|e| e.to_string())?;
    if !compiler_version.status.success() {
        return Err("compiler version failed".into());
    }
    let compile_args = [
        "-std=c11",
        "-O0",
        "-fsanitize=undefined",
        "-fno-sanitize-recover=all",
        "-I.",
        "cJSON.c",
        "heap.c",
        "-lm",
        "-o",
        "heap",
    ];
    let command = json!({"program":compiler,"args":compile_args,"directory":out,"source_sha256":qcore_schemas::sha256_hex(&original),"header_sha256":qcore_schemas::sha256_hex(&header),"harness_sha256":qcore_schemas::sha256_hex(harness),"compiler_version_sha256":qcore_schemas::sha256_hex(&compiler_version.stdout)});
    let context = Context {
        artifact: "native/cjson/detach#cJSON_DetachItemViaPointer".into(),
        property: "resource-custody.v1".into(),
        semantic_profile: binding::ENVIRONMENT.into(),
        scope: "lengths0..4;all-valid-detach-indices;11cases".into(),
        bounds: "single-thread;finite-live-ledger;no-callback-hooks".into(),
        assumptions: "native-execution-and-ledger-observed;allocator-and-extraction-not-proved"
            .into(),
        source_identity: qcore_schemas::sha256_hex(pp.as_bytes()),
        build_identity: qcore_schemas::sha256_hex(&serde_json::to_vec(&command).unwrap()),
        evidence_version: "premise-local-source-bindings.v1".into(),
    };
    let graph: Graph = serde_json::from_str(include_str!(
        "../../../docs/reports/premise-evidence-contracts/expected-graph.json"
    ))
    .map_err(|e| e.to_string())?;
    let model = include_bytes!("../../../proof/QCore/CjsonRepresentationPremises.lean");
    let checked_source = binding::check_source(
        &unit,
        &qcore_schemas::sha256_hex(&original),
        &context,
        &context,
        &graph,
        model,
        binding::ENVIRONMENT,
    )?;
    let compiled = Command::new(&compiler)
        .args(compile_args)
        .current_dir(&out)
        .output()
        .map_err(|e| e.to_string())?;
    fs::write(out.join("compile.stdout"), &compiled.stdout).map_err(|e| e.to_string())?;
    fs::write(out.join("compile.stderr"), &compiled.stderr).map_err(|e| e.to_string())?;
    let mut receipt =
        json!({"compile":command,"compile_exit":compiled.status.code(),"context":context});
    if !compiled.status.success() {
        fs::write(
            out.join("commands.json"),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
        return Err("native compilation refused".into());
    }
    let executed = Command::new(out.join("heap"))
        .current_dir(&out)
        .output()
        .map_err(|e| e.to_string())?;
    fs::write(out.join("native.json"), &executed.stdout).map_err(|e| e.to_string())?;
    fs::write(out.join("native.stderr"), &executed.stderr).map_err(|e| e.to_string())?;
    receipt["native_exit"] = json!(executed.status.code());
    receipt["executable_sha256"] = json!(qcore_schemas::sha256_hex(
        &fs::read(out.join("heap")).unwrap()
    ));
    receipt["native_stdout_sha256"] = json!(qcore_schemas::sha256_hex(&executed.stdout));
    fs::write(
        out.join("commands.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    if !executed.status.success() || !executed.stderr.is_empty() {
        return Err("native execution refused or sanitizer reported".into());
    }
    for (path, bytes) in [
        ("cJSON.c", original.as_slice()),
        ("cJSON.h", header.as_slice()),
        ("heap.c", harness.as_slice()),
    ] {
        if fs::read(out.join(path)).map_err(|e| e.to_string())? != bytes {
            return Err("compiler input changed during run".into());
        }
    }
    let observed: Vec<NativeCase> =
        serde_json::from_slice(&executed.stdout).map_err(|e| e.to_string())?;
    let required: BTreeSet<String> = [
        "native-driver-executed-held-source",
        "allocation-ledger-live-until-snapshot",
        "single-thread-harness-no-callback-hooks",
    ]
    .map(str::to_owned)
    .into_iter()
    .collect();
    let checked = binding::check_native_domain(checked_source, &observed, &observed, &required)?;
    fs::write(
        out.join("checked.json"),
        serde_json::to_vec_pretty(&checked).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    assert_eq!(checked.case_count(), 11);
    println!(
        "checked 11 native heap cases; universal atoms and no-writer trace remain Open; {}",
        Path::new(&args[3]).display()
    );
    Ok(())
}
