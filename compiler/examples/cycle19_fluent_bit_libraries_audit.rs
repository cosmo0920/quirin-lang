use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[path = "support/legacy_audit_report.rs"]
mod legacy_audit_report;
use legacy_audit_report::AuditReport;

use qcore_compiler::chunk_contract::{check_chunk_frame_contract, check_chunk_source_contract};
use qcore_compiler::contract_loader::load_resource_contract_manifest;
use qcore_compiler::profile_loader::{LoadedProfile, load_audited_bundle, verify_pin};
use qcore_compiler::source_profiles::{FLUENT_BIT_SOURCE_SET, LIBEVENT_SOURCE_SET, load_profile};
use qcore_compiler::verifier_plugins::chunkio::{
    ChunkFrameSummary, ChunkSourceResult, import_chunk_frame_helper,
    import_chunk_source_with_helpers,
};
use qcore_compiler::verifier_plugins::libevent::LIBEVENT_SUBMODULE_PATH;
use qcore_compiler::verifier_plugins::libevent::monkey_source::{
    MonkeySourceResult, import_monkey_callbacks,
};
use qcore_compiler::{
    C11TranslationUnit, CCompilerDialect, LegacyFunctionScope, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, classify_libevent_symbol,
    derive_library_summary_profile, legacy_c_source_functions, validate_unrestricted_c11,
    verify_pinned_libevent_source,
};

/// W2: the pinned revision, the clean-path list and the audited-file
/// inventories below come from `contract/profiles/<source-set>/profile.json`.
/// An unloadable profile audits nothing; it never falls back to a built-in
/// table. The ChunkIO provider modules still carry their own revision
/// constant (their migration is not part of this change), so the checks that
/// take a revision string take it from there; a test asserts the profile pin
/// and that constant agree.
use qcore_compiler::PINNED_FLUENT_BIT_REVISION as PINNED_FLUENT_BIT_COMMIT;

#[derive(Clone, Debug)]
struct CompileEntry {
    directory: PathBuf,
    command: String,
    file: PathBuf,
}

#[derive(Default)]
struct AuditCounts {
    units: usize,
    data_only_units: usize,
    source_functions: usize,
    projected_functions: usize,
    safe_units: usize,
    unsafe_units: usize,
    unknown_units: usize,
    gateway_failures: usize,
}

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let root = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../fluent-bit"));
    let monkey_database = arguments.next().map(PathBuf::from);
    let monkey_source_report = arguments.next().map(PathBuf::from);
    let diagnostic_report = arguments.next().map(PathBuf::from);
    let binding_directory = arguments.next().map(PathBuf::from);
    let contract_manifest = arguments.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../contract/cycle19-resource-contracts.json")
    });
    if arguments.next().is_some() {
        eprintln!(
            "usage: cycle19_fluent_bit_libraries_audit [fluent-bit-root] \
             [monkey-libevent-compile_commands.json] [monkey-callback-source-report.json] \
             [resource-diagnostics.json] [native-observation-directory] [resource-contract-manifest.json]"
        );
        return ExitCode::FAILURE;
    }
    match run(
        &root,
        monkey_database.as_deref(),
        monkey_source_report.as_deref(),
        diagnostic_report.as_deref(),
        binding_directory.as_deref(),
        &contract_manifest,
    ) {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("Fluent Bit library audit failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(
    root: &Path,
    monkey_database: Option<&Path>,
    monkey_source_report: Option<&Path>,
    diagnostic_report: Option<&Path>,
    binding_directory: Option<&Path>,
    contract_manifest: &Path,
) -> Result<bool, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("could not resolve {}: {error}", root.display()))?;
    let database_path = root.join("build/compile_commands.json");
    let database = fs::read_to_string(&database_path)
        .map_err(|error| format!("could not read {}: {error}", database_path.display()))?;
    let entries = compile_entries(&database)?;
    let profile = load_profile(FLUENT_BIT_SOURCE_SET)
        .map_err(|error| format!("Fluent Bit source-set profile unavailable: {error}"))?;
    let pin = profile
        .manifest()
        .pins
        .get(FLUENT_BIT_SOURCE_SET)
        .ok_or("the fluent-bit profile declares no `fluent-bit` pin")?;
    let revision = verify_pin(FLUENT_BIT_SOURCE_SET, pin, &root)
        .map_err(|error| format!("Fluent Bit pin check failed: {error}"))?;
    println!("Fluent Bit {revision}");
    println!(
        "source profile assumptions: {}",
        serde_json::to_string(&profile.assumptions())
            .map_err(|error| format!("could not serialize profile assumptions: {error}"))?
    );
    let mut report = AuditReport::new(&revision);
    let mut native_results = Vec::new();
    let mut teardown_results = Vec::new();
    audit_library_summary_profiles(&root, &revision, &profile)?;

    let contracts = load_resource_contract_manifest(contract_manifest)?;
    if let Some(path) = diagnostic_report {
        let sidecar = path.with_extension("contracts.json");
        let inputs = serde_json::json!({
            "schema_version": 1,
            "trust": "declared resource-projection assumptions, not verified C behavior",
            "manifest": fs::read_to_string(contract_manifest).map_err(|e| e.to_string())?,
            "sources": contracts.source_assumptions(),
            "source_profile": profile.assumptions(),
        });
        let bytes = serde_json::to_vec_pretty(&inputs).map_err(|e| e.to_string())?;
        fs::write(&sidecar, bytes).map_err(|e| format!("{}: {e}", sidecar.display()))?;
    }
    let mut failed = false;
    for library in ["cmetrics", "ctraces"] {
        let source_root = root.join("lib").join(library).join("src");
        let mut selected = entries
            .iter()
            .filter(|entry| {
                entry.file.parent() == Some(source_root.as_path())
                    && entry
                        .file
                        .extension()
                        .is_some_and(|extension| extension == "c")
            })
            .cloned()
            .collect::<Vec<_>>();
        selected.sort_by(|left, right| left.file.cmp(&right.file));
        if selected.is_empty() {
            return Err(format!(
                "compile database contains no top-level {library} sources"
            ));
        }
        let counts = audit_library(
            library,
            &source_root,
            &selected,
            &contracts,
            &mut report,
            &mut native_results,
            &mut teardown_results,
        )?;
        failed |= counts.gateway_failures != 0;
        println!(
            "{library}: {} unit(s), {} source function(s), {} projected; \
             {} data-only, {} Safe, {} Unsafe, {} Unknown, {} gateway failure(s)",
            counts.units,
            counts.source_functions,
            counts.projected_functions,
            counts.data_only_units,
            counts.safe_units,
            counts.unsafe_units,
            counts.unknown_units,
            counts.gateway_failures
        );
    }
    for (library, relative_root) in [
        ("librdkafka", "lib/librdkafka-2.15.0/src"),
        ("chunkio", "lib/chunkio/src"),
    ] {
        let source_root = root.join(relative_root);
        let selected = select_recursive_units(&entries, &source_root);
        if selected.is_empty() {
            return Err(format!(
                "compile database contains no built {library} sources"
            ));
        }
        let counts = audit_library(
            library,
            &source_root,
            &selected,
            &contracts,
            &mut report,
            &mut native_results,
            &mut teardown_results,
        )?;
        failed |= counts.gateway_failures != 0;
        println!(
            "{library}: {} unit(s), {} source function(s), {} projected; \
             {} data-only, {} Safe, {} Unsafe, {} Unknown, {} gateway failure(s)",
            counts.units,
            counts.source_functions,
            counts.projected_functions,
            counts.data_only_units,
            counts.safe_units,
            counts.unsafe_units,
            counts.unknown_units,
            counts.gateway_failures
        );
    }
    let input_chunk_path = root.join("src/flb_input_chunk.c");
    let selected = select_exact_unit(&entries, &input_chunk_path);
    if selected.is_empty() {
        return Err("compile database contains no src/flb_input_chunk.c command".into());
    }
    let counts = audit_library(
        "fluent-bit-chunk",
        &root.join("src"),
        &selected,
        &contracts,
        &mut report,
        &mut native_results,
        &mut teardown_results,
    )?;
    failed |= counts.gateway_failures != 0;
    println!(
        "fluent-bit-chunk: {} unit(s), {} source function(s), {} projected; \
         {} data-only, {} Safe, {} Unsafe, {} Unknown, {} gateway failure(s)",
        counts.units,
        counts.source_functions,
        counts.projected_functions,
        counts.data_only_units,
        counts.safe_units,
        counts.unsafe_units,
        counts.unknown_units,
        counts.gateway_failures
    );
    audit_monkey_libevent_contract(&root)?;
    if let Some(database_path) = monkey_database {
        let database_path = database_path
            .canonicalize()
            .map_err(|error| format!("could not resolve {}: {error}", database_path.display()))?;
        let database = fs::read_to_string(&database_path)
            .map_err(|error| format!("could not read {}: {error}", database_path.display()))?;
        let monkey_entries = compile_entries(&database)?;
        let source_root = monkey_source_root(&monkey_entries)?;
        let mut selected = monkey_entries
            .into_iter()
            .filter(|entry| is_monkey_unit(&source_root, &entry.file))
            .collect::<Vec<_>>();
        selected.sort_by(|left, right| left.file.cmp(&right.file));
        selected.dedup_by(|left, right| left.file == right.file);
        verify_monkey_source_copy(&root.join("lib/monkey"), &source_root, &selected)?;
        audit_monkey_callback_source(&selected, monkey_source_report, binding_directory)?;
        let counts = audit_library(
            "monkey-libevent",
            &source_root,
            &selected,
            &contracts,
            &mut report,
            &mut native_results,
            &mut teardown_results,
        )?;
        failed |= counts.gateway_failures != 0;
        println!(
            "monkey-libevent: {} unit(s), {} source function(s), {} projected; \
             {} data-only, {} Safe, {} Unsafe, {} Unknown, {} gateway failure(s)",
            counts.units,
            counts.source_functions,
            counts.projected_functions,
            counts.data_only_units,
            counts.safe_units,
            counts.unsafe_units,
            counts.unknown_units,
            counts.gateway_failures
        );
    } else {
        println!(
            "monkey-libevent: source contract checked; pass its forced-libevent \
             compile_commands.json to run the C gateway projection"
        );
    }
    if let Some(path) = diagnostic_report {
        report.write(path)?;
        let sidecar = path.with_extension("chunk-contracts.json");
        let inputs = serde_json::json!({
            "schema_version": 1, "source_revision": revision,
            "scope": "bounded field contracts; live/exclusive/Boolean domains and source/build/ABI identity assumed; no whole-unit lifecycle certificate",
            "results": native_results,
            "backend_teardown": teardown_results,
            "source_policy_contract": include_str!("../../contract/qcore/library/source_projection.qcore"),
            "frame_helper_contract": include_str!("../../contract/qcore/chunkio/error_state.qcore"),
        });
        fs::write(
            &sidecar,
            serde_json::to_vec_pretty(&inputs).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("Resource diagnostics: {}", path.display());
    }
    Ok(failed)
}

fn audit_library_summary_profiles(
    root: &Path,
    revision: &str,
    profile: &LoadedProfile,
) -> Result<(), String> {
    for name in ["librdkafka", "cfl", "chunkio"] {
        let library = profile
            .library(name)
            .ok_or_else(|| format!("the fluent-bit profile declares no `{name}` library"))?;
        let bundle = load_audited_bundle(library, root, revision).map_err(|diagnostics| {
            format!("{name} audited source inventory failed: {diagnostics:?}")
        })?;
        let certificate = derive_library_summary_profile(name, library, revision, &bundle)
            .map_err(|diagnostics| {
                format!("{name} source-summary contract failed: {diagnostics:?}")
            })?;
        println!(
            "{name} source summaries: {} contract(s), {} file(s), {:?}",
            certificate.checked_contracts.len(),
            certificate.checked_files.len(),
            certificate.profile.summaries
        );
    }
    Ok(())
}

fn select_recursive_units(entries: &[CompileEntry], source_root: &Path) -> Vec<CompileEntry> {
    let mut selected = entries
        .iter()
        .filter(|entry| {
            entry.file.starts_with(source_root)
                && entry
                    .file
                    .extension()
                    .is_some_and(|extension| extension == "c")
        })
        .cloned()
        .collect::<Vec<_>>();
    selected.sort_by(|left, right| left.file.cmp(&right.file));
    selected.dedup_by(|left, right| left.file == right.file);
    selected
}

fn select_exact_unit(entries: &[CompileEntry], source: &Path) -> Vec<CompileEntry> {
    entries
        .iter()
        .find(|entry| entry.file == source)
        .cloned()
        .into_iter()
        .collect()
}

fn audit_library(
    library: &str,
    source_root: &Path,
    entries: &[CompileEntry],
    contracts: &ResourceContracts,
    report: &mut AuditReport,
    native_results: &mut Vec<serde_json::Value>,
    teardown_results: &mut Vec<serde_json::Value>,
) -> Result<AuditCounts, String> {
    let mut counts = AuditCounts::default();
    let mut chunk_helpers = Vec::new();
    let mut memory_backend = None;
    let mut filesystem_deps = None;
    let mut filesystem_close = None;
    let mut nested_memory = None;
    let mut chunk_traversal = None;
    let mut stream_traversal = None;
    let mut stream_parent = None;
    let mut context_parent = None;
    let mut filesystem_chunk = None;
    if library == "chunkio" {
        let entry = entries
            .iter()
            .find(|entry| entry.file.ends_with("cio_error.c"))
            .ok_or("missing compiled ChunkIO error helper")?;
        let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
        let options = compiler_options(entry)?;
        let unit = C11TranslationUnit {
            source: &source,
            include_paths: options.include_paths,
            definitions: options.definitions,
            dialect: options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
        };
        let helper = import_chunk_frame_helper(&unit, PINNED_FLUENT_BIT_COMMIT, "cio_error_reset")?;
        let postconditions = check_chunk_frame_contract(
            &helper,
            include_str!("../../contract/qcore/chunkio/error_state.qcore"),
        )?;
        println!(
            "qcore-contract cio_error_reset: Checked ({postconditions} postconditions; complete constant-field helper body, live/exclusive premises assumed)"
        );
        chunk_helpers.push(helper);
        println!(
            "chunk-source dependency cio_error_reset: source-validated constant field writes; revision/build/ABI supplied by pinned audit"
        );
    }
    if library == "chunkio" {
        let entry = entries
            .iter()
            .find(|entry| entry.file.ends_with("cio_memfs.c"))
            .ok_or("missing compiled memory backend")?;
        let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
        let options = compiler_options(entry)?;
        let unit = C11TranslationUnit {
            source: &source,
            include_paths: options.include_paths,
            definitions: options.definitions,
            dialect: options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
        };
        memory_backend = Some(qcore_compiler::verifier_plugins::chunkio::teardown_source::check_memory_backend_source(&unit, PINNED_FLUENT_BIT_COMMIT)?);
    }
    if library == "chunkio" {
        let load = |file: &str| -> Result<(String, CompilerOptions), String> {
            let entry = entries
                .iter()
                .find(|entry| entry.file.ends_with(file))
                .ok_or_else(|| format!("missing compiled dependency {file}"))?;
            Ok((
                fs::read_to_string(&entry.file).map_err(|error| error.to_string())?,
                compiler_options(entry)?,
            ))
        };
        let (native_source, native_options) = load("cio_file_unix.c")?;
        let native_unit = C11TranslationUnit {
            source: &native_source,
            include_paths: native_options.include_paths,
            definitions: native_options.definitions,
            dialect: native_options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
        };
        let native =
            qcore_compiler::verifier_plugins::chunkio::file_outcomes::check_file_outcome_sources(
                &native_unit,
                PINNED_FLUENT_BIT_COMMIT,
            )?
            .into_iter()
            .find(|c| c.source().function() == "cio_file_native_unmap")
            .ok_or("missing native unmap certificate")?;
        let close =
            qcore_compiler::verifier_plugins::chunkio::file_outcomes::check_file_outcome_sources(
                &native_unit,
                PINNED_FLUENT_BIT_COMMIT,
            )?
            .into_iter()
            .find(|c| c.source().function() == "cio_file_native_close")
            .ok_or("missing native close certificate")?;
        let (counter_source, counter_options) = load("cio_chunk.c")?;
        let counter_unit = C11TranslationUnit {
            source: &counter_source,
            include_paths: counter_options.include_paths,
            definitions: counter_options.definitions,
            dialect: counter_options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
        };
        let counter =
            qcore_compiler::verifier_plugins::chunkio::teardown_source::check_counter_sources(
                &counter_unit,
                PINNED_FLUENT_BIT_COMMIT,
            )?
            .into_iter()
            .find(|c| c.field() == "total_chunks_up" && c.delta() == -1)
            .ok_or("missing up counter certificate")?;
        let delete =
            qcore_compiler::verifier_plugins::chunkio::file_delete::check_native_delete_source(
                &native_unit,
                PINNED_FLUENT_BIT_COMMIT,
            )?;
        let (file_source, file_options) = load("cio_file.c")?;
        let file_unit = C11TranslationUnit {
            source: &file_source,
            include_paths: file_options.include_paths,
            definitions: file_options.definitions,
            dialect: file_options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
        };
        let unmap =
            qcore_compiler::verifier_plugins::chunkio::unmap_wrapper::check_unmap_wrapper_source(
                &file_unit,
                PINNED_FLUENT_BIT_COMMIT,
                &native,
                &counter,
            )?;
        filesystem_close = Some(qcore_compiler::verifier_plugins::chunkio::file_close::check_file_close_deletion_source(
            &file_unit, PINNED_FLUENT_BIT_COMMIT, &unmap, &close, &delete)?);
        filesystem_deps = Some((native, counter, close, delete));
    }
    for entry in entries {
        let source = fs::read_to_string(&entry.file)
            .map_err(|error| format!("could not read {}: {error}", entry.file.display()))?;
        let source_functions = legacy_c_source_functions(&source);
        let file = entry
            .file
            .strip_prefix(source_root)
            .map_err(|_| format!("source is outside audit root: {}", entry.file.display()))?
            .to_string_lossy();
        let options = compiler_options(entry)?;
        let unit = C11TranslationUnit {
            source: &source,
            include_paths: options.include_paths,
            definitions: options.definitions,
            dialect: options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
        };
        if library == "chunkio" && matches!(file.as_ref(), "cio_file.c" | "cio_file_unix.c") {
            let evidence = qcore_compiler::verifier_plugins::chunkio::file_attempts::check_file_attempt_sources(&unit, PINNED_FLUENT_BIT_COMMIT, file == "cio_file_unix.c")?;
            for function in &evidence {
                println!(
                    "qcore-result-use {}: Checked ({} source paths; OS cleanup success and custody Unknown)",
                    function.source().function(),
                    function.source().paths().len()
                );
            }
            teardown_results.push(serde_json::json!({"kind":"filesystem_call_results", "file":file, "evidence":evidence}));
        }
        if library == "chunkio" && file == "cio_file.c" {
            let (native, counter, close, delete) = filesystem_deps
                .as_ref()
                .ok_or("missing filesystem dependencies")?;
            let certificate = qcore_compiler::verifier_plugins::chunkio::unmap_wrapper::check_unmap_wrapper_source(&unit,PINNED_FLUENT_BIT_COMMIT,native,counter)?;
            println!(
                "qcore-outcomes munmap_file: Checked ({} source cases; sync/log frames assumed, OS custody conditional)",
                certificate.cases().len()
            );
            let close_evidence =
                qcore_compiler::verifier_plugins::chunkio::file_close::check_file_close_source(
                    &unit,
                    PINNED_FLUENT_BIT_COMMIT,
                    &certificate,
                    close,
                )?;
            println!(
                "qcore-custody cio_file_close: {} conditional no-delete cases, {} unresolved nested cleanup cases; deletion paths uncovered",
                close_evidence.cases().len(),
                close_evidence
                    .cases()
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| close_evidence.require_nested_discharge(*i).is_err())
                    .count()
            );
            let deleting = qcore_compiler::verifier_plugins::chunkio::file_close::check_file_close_deletion_source(&unit,PINNED_FLUENT_BIT_COMMIT,&certificate,close,delete)?;
            println!(
                "qcore-custody cio_file_close delete: {} conditional cases, {} uncovered source paths across both modes",
                deleting.deleting_cases().len(),
                deleting.uncovered_paths().len()
            );
            teardown_results.push(
                serde_json::json!({"kind":"filesystem_close_with_delete", "evidence":deleting}),
            );
            teardown_results.push(
                serde_json::json!({"kind":"filesystem_close_no_delete", "evidence":close_evidence}),
            );
            teardown_results.push(
                serde_json::json!({"kind":"filesystem_unmap_wrapper", "certificate":certificate}),
            );
        }
        if library == "chunkio" && file == "cio_file_unix.c" {
            let deletion =
                qcore_compiler::verifier_plugins::chunkio::file_delete::check_native_delete_source(
                    &unit,
                    PINNED_FLUENT_BIT_COMMIT,
                )?;
            println!(
                "qcore-outcomes cio_file_native_delete: Checked ({} eligibility/outcome cases; unlink framing assumed)",
                deletion.cases().len()
            );
            teardown_results.push(
                serde_json::json!({"kind":"filesystem_delete_outcomes", "certificate":deletion}),
            );
            let delete =
                qcore_compiler::verifier_plugins::chunkio::file_attempts::check_file_delete_source(
                    &unit,
                    PINNED_FLUENT_BIT_COMMIT,
                )?;
            println!(
                "qcore-result-use cio_file_native_delete: Checked ({} short-circuit source paths; deletion custody Unknown)",
                delete.source().paths().len()
            );
            teardown_results
                .push(serde_json::json!({"kind":"filesystem_delete_attempt", "evidence":delete}));
            let certificates = qcore_compiler::verifier_plugins::chunkio::file_outcomes::check_file_outcome_sources(&unit, PINNED_FLUENT_BIT_COMMIT)?;
            for certificate in &certificates {
                println!(
                    "qcore-outcomes {}: Checked ({} null/absent/failure/success cases; OS custody conditional)",
                    certificate.source().function(),
                    certificate.cases().len()
                );
            }
            teardown_results.push(serde_json::json!({"kind":"filesystem_native_outcomes", "certificates":certificates}));
        }
        if library == "chunkio" && file == "cio_chunk.c" {
            let unlink = qcore_compiler::verifier_plugins::chunkio::teardown_source::check_list_unlink_source(&unit, PINNED_FLUENT_BIT_COMMIT)?;
            println!(
                "qcore-contract mk_list_del + __mk_list_del: Checked ({} neighbor alias cases; live reciprocal/exclusive links assumed)",
                unlink.cases().len()
            );
            teardown_results.push(serde_json::json!({"kind":"list_unlink", "status":"checked_source_links", "certificate":unlink}));
            let certificates =
                qcore_compiler::verifier_plugins::chunkio::teardown_source::check_counter_sources(
                    &unit,
                    PINNED_FLUENT_BIT_COMMIT,
                )?;
            println!(
                "qcore-contract ChunkIO counters: Checked ({} complete unsigned unit updates; nonwrapping entry bounds and ABI binding assumed)",
                certificates.len()
            );
            teardown_results.push(serde_json::json!({"kind": "unsigned_counters", "status": "checked_source_updates", "certificates": certificates}));
            let dependencies =
                qcore_compiler::verifier_plugins::chunkio::memory_close::MemoryCloseDependencies {
                    reset: chunk_helpers[0].clone(),
                    backend: memory_backend
                        .as_ref()
                        .ok_or("missing memory backend dependency")?
                        .clone(),
                    unlink: unlink.clone(),
                    counter: certificates
                        .iter()
                        .find(|certificate| {
                            certificate.field() == "total_chunks" && certificate.delta() == -1
                        })
                        .ok_or("missing total decrement dependency")?
                        .clone(),
                };
            let close =
                qcore_compiler::verifier_plugins::chunkio::memory_close::check_memory_close_source(
                    &unit,
                    PINNED_FLUENT_BIT_COMMIT,
                    &dependencies,
                )?;
            println!(
                "qcore-contract cio_chunk_close memory scope: Checked ({} null/memory paths; {} source paths uncovered; enclosing stream/context teardown open)",
                close.checked_paths().len(),
                close.uncovered_paths().len()
            );
            nested_memory = Some(close.clone());
            teardown_results.push(serde_json::json!({"kind":"memory_chunk_close", "status":"checked_null_and_memory_paths", "certificate":close}));
            let file = qcore_compiler::verifier_plugins::chunkio::filesystem_chunk_close::check_filesystem_chunk_close_source(
                &unit, PINNED_FLUENT_BIT_COMMIT, &dependencies,
                filesystem_close.as_ref().ok_or("missing filesystem close dependency")?)?;
            println!(
                "qcore-custody cio_chunk_close filesystem: {} conditional cases, {} unresolved nested cases, {} combined uncovered paths (unsupported backend kinds)",
                file.cases().len(),
                (0..file.cases().len())
                    .filter(|i| file.require_nested_discharge(*i).is_err())
                    .count(),
                file.combined_uncovered_paths().len()
            );
            filesystem_chunk = Some(file.clone());
            let unsupported = qcore_compiler::verifier_plugins::chunkio::unsupported_close::check_unsupported_close_source(&file)?;
            if file.combined_uncovered_paths() != [unsupported.source_path()] {
                return Err("chunk close path partition has additional unclassified paths".into());
            }
            println!(
                "qcore-custody cio_chunk_close unsupported backend: Unknown (complete symbolic int domain excluding {:?}; outer release preserves unresolved backend custody)",
                unsupported.excluded_kinds()
            );
            teardown_results.push(
                serde_json::json!({"kind":"unsupported_chunk_close", "evidence":unsupported}),
            );
            teardown_results
                .push(serde_json::json!({"kind":"filesystem_chunk_close", "evidence":file}));
        }
        if library == "chunkio" && matches!(file.as_ref(), "cio_chunk.c" | "cio_stream.c") {
            use qcore_compiler::verifier_plugins::chunkio::traversal::{
                TraversalKind, check_traversal_source,
            };
            let kind = if file == "cio_chunk.c" {
                TraversalKind::Chunks
            } else {
                TraversalKind::Streams
            };
            let evidence = check_traversal_source(&unit, PINNED_FLUENT_BIT_COMMIT, kind)?;
            match kind {
                TraversalKind::Chunks => chunk_traversal = Some(evidence.clone()),
                TraversalKind::Streams => stream_traversal = Some(evidence.clone()),
            }
            println!(
                "qcore-traversal {}: complete typed reference body checked; ring provenance and child custody remain premises",
                evidence.source().function()
            );
            teardown_results.push(
                serde_json::json!({"kind":"teardown_traversal", "file":file, "evidence":evidence}),
            );
        }
        if library == "chunkio" && matches!(file.as_ref(), "cio_stream.c" | "chunkio.c") {
            use qcore_compiler::verifier_plugins::chunkio::parent_teardown::{
                ParentKind, check_parent_source,
            };
            let kind = if file == "cio_stream.c" {
                ParentKind::Stream
            } else {
                ParentKind::Context
            };
            let evidence = check_parent_source(&unit, PINNED_FLUENT_BIT_COMMIT, kind)?;
            match kind {
                ParentKind::Stream => stream_parent = Some(evidence.clone()),
                ParentKind::Context => context_parent = Some(evidence.clone()),
            }
            println!(
                "qcore-parent {}: complete allocation-release body checked; child effects and allocation provenance remain premises",
                evidence.source().function()
            );
            teardown_results.push(
                serde_json::json!({"kind":"parent_teardown", "file":file, "evidence":evidence}),
            );
        }
        if library == "chunkio" && file == "cio_memfs.c" {
            let certificate = memory_backend
                .as_ref()
                .ok_or("missing memory backend certificate")?;
            println!(
                "qcore-contract cio_memfs_close: Checked ({} nullable custody states; borrowed root/provenance/libc premises assumed; enclosing chunk teardown open)",
                certificate.cases().len()
            );
            teardown_results.push(serde_json::json!({"function": "cio_memfs_close", "status": "checked_field_custody", "certificate": certificate}));
        }
        if library == "fluent-bit-chunk" || (library == "chunkio" && file == "cio_chunk.c") {
            audit_chunk_source(&unit, library, &file, &chunk_helpers, native_results)?;
        }
        counts.units += 1;
        if source_functions.is_empty() {
            counts.data_only_units += 1;
            let result = validate_unrestricted_c11(&unit);
            report.record_data_only(library, &file, result.as_ref().err());
            match result {
                Ok(_) => println!("  {library}/{file}: data-only gateway pass"),
                Err(error) => {
                    counts.gateway_failures += 1;
                    println!("  {library}/{file}: data-only gateway failure: {error}");
                }
            }
            continue;
        }
        counts.source_functions += source_functions.len();
        let result = analyze_unrestricted_c11(&unit, &source_functions[0], contracts);
        let detail = report.record(library, &file, &source_functions, &result);
        match result {
            Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
                counts.safe_units += 1;
                counts.projected_functions += certificate.checked_functions.len();
                println!(
                    "  {library}/{file}: Safe ({}/{})",
                    certificate.checked_functions.len(),
                    source_functions.len()
                );
            }
            Ok(UnrestrictedLegacyResult::Unsafe(summary)) => {
                counts.unsafe_units += 1;
                counts.projected_functions += summary.functions.len();
                println!(
                    "  {library}/{file}: Unsafe ({}/{})",
                    summary.functions.len(),
                    source_functions.len()
                );
            }
            Ok(UnrestrictedLegacyResult::Unknown {
                reason, partial, ..
            }) => {
                counts.unknown_units += 1;
                let projected = partial
                    .as_ref()
                    .map_or(0, |summary| summary.functions.len());
                counts.projected_functions += projected;
                println!(
                    "  {library}/{file}: Unknown ({projected}/{}): {reason}",
                    source_functions.len()
                );
            }
            Err(error) => {
                counts.gateway_failures += 1;
                println!("  {library}/{file}: gateway failure: {error}");
            }
        }
        for line in detail {
            println!("    {line}");
        }
    }
    if library == "chunkio" {
        let certificate =
            qcore_compiler::verifier_plugins::chunkio::nested_memory::check_nested_memory_source(
                chunk_traversal.as_ref().ok_or("missing chunk traversal")?,
                stream_traversal
                    .as_ref()
                    .ok_or("missing stream traversal")?,
                stream_parent.as_ref().ok_or("missing stream parent")?,
                context_parent.as_ref().ok_or("missing context parent")?,
                nested_memory.as_ref().ok_or("missing memory close")?,
            )?;
        println!(
            "qcore-nested-memory: complete source dependency chain checked; native caller/ring provenance remains Unknown; filesystem children rejected"
        );
        teardown_results
            .push(serde_json::json!({"kind":"nested_memory_source", "certificate":certificate}));
        use qcore_compiler::verifier_plugins::chunkio::nested_cleanup::{
            ChildCase, ConditionalStream, check_nested_cleanup_source,
        };
        let filesystem = filesystem_chunk
            .as_ref()
            .ok_or("missing filesystem chunk close")?;
        let cleanup = check_nested_cleanup_source(&certificate, filesystem)?;
        let mut conditional_examples = Vec::new();
        for (case, c) in filesystem
            .cases()
            .iter()
            .enumerate()
            .filter(|(_, c)| c.chunk_present && !c.delete)
        {
            conditional_examples.push(cleanup.project(
                1,
                &[ConditionalStream {
                    stream: 1,
                    chunks: vec![
                        ChildCase::Memory { chunk: 1 },
                        ChildCase::Filesystem { chunk: 2, case },
                    ],
                }],
                2,
                c.up_counter_decrements,
            )?);
        }
        println!(
            "qcore-nested-cleanup: {} mixed-backend conditional cases, {} retain child OS/counter obligations after parent free; native entry/outcome binding remains Unknown",
            conditional_examples.len(),
            conditional_examples
                .iter()
                .filter(|c| c.require_conditional_discharge().is_err())
                .count()
        );
        teardown_results.push(serde_json::json!({"kind":"nested_cleanup_source", "source":cleanup, "conditional_examples":conditional_examples}));
        let unsupported = qcore_compiler::verifier_plugins::chunkio::unsupported_close::check_unsupported_close_source(filesystem)?;
        let extended = cleanup.with_unsupported(&unsupported)?;
        let mut examples = Vec::new();
        for kind in [i32::MIN, -1, 2, i32::MAX] {
            let case = extended.project(
                1,
                &[
                    ConditionalStream {
                        stream: 10,
                        chunks: vec![ChildCase::Memory { chunk: 11 }],
                    },
                    ConditionalStream {
                        stream: 20,
                        chunks: vec![ChildCase::Unsupported { chunk: 21, kind }],
                    },
                ],
                2,
                0,
            )?;
            if case.require_conditional_discharge().is_ok() || case.opaque_residuals().len() != 1 {
                return Err("unsupported parent teardown erased opaque child custody".into());
            }
            examples.push(case);
        }
        println!(
            "qcore-nested-unsupported: {} conditional examples retain exact opaque child identities after parent release; constructor reachability Unknown",
            examples.len()
        );
        teardown_results.push(serde_json::json!({"kind":"nested_unsupported_source", "source":extended, "conditional_examples":examples}));
    }
    Ok(counts)
}

fn audit_chunk_source(
    unit: &C11TranslationUnit<'_>,
    library: &str,
    file: &str,
    helpers: &[ChunkFrameSummary],
    native_results: &mut Vec<serde_json::Value>,
) -> Result<(), String> {
    let contract_source = match library {
        "fluent-bit-chunk" => include_str!("../../contract/qcore/fluent_bit/busy_state.qcore"),
        "chunkio" => include_str!("../../contract/qcore/chunkio/lock_state.qcore"),
        _ => return Err("missing native QCore chunk contract file".into()),
    };
    let results = import_chunk_source_with_helpers(unit, PINNED_FLUENT_BIT_COMMIT, helpers)?;
    let mut summaries = 0;
    let mut checked_contracts = 0;
    for result in &results {
        match result {
            ChunkSourceResult::Summary(summary) => {
                summaries += 1;
                println!(
                    "  chunk-source {library}/{file}/{}: field-summary {:?}; {:?}",
                    summary.function(),
                    summary.field(),
                    summary.paths()
                );
                match check_chunk_source_contract(summary, contract_source) {
                    Ok(certificate) => {
                        checked_contracts += 1;
                        native_results.push(serde_json::json!({
                            "library": library, "file": file, "function": summary.function(),
                            "status": "checked", "summary": summary,
                            "contract_source": certificate.contract_source(),
                            "checked_states": certificate.checked_states(),
                        }));
                        println!(
                            "  qcore-contract {}: Checked ({} Boolean pre-states; live/exclusive/domain preconditions assumed)",
                            certificate.function(),
                            certificate.checked_states()
                        );
                    }
                    Err(reason) => {
                        return Err(format!(
                            "native QCore contract check failed for {}: {reason}",
                            summary.function()
                        ));
                    }
                }
            }
            ChunkSourceResult::Unknown { function, reason } => {
                native_results.push(serde_json::json!({ "library": library, "file": file, "function": function, "status": "unknown", "reason": reason }));
                println!("  chunk-source {library}/{file}/{function}: Unknown: {reason}")
            }
        }
    }
    println!(
        "chunk-source {library}/{file}: {} function(s), {summaries} field summary(s), {checked_contracts} QCore contract(s) checked, {} Unknown; no whole-unit lifecycle certificate",
        results.len(),
        results.len() - summaries
    );
    Ok(())
}

fn audit_monkey_callback_source(
    entries: &[CompileEntry],
    report: Option<&Path>,
    binding_directory: Option<&Path>,
) -> Result<(), String> {
    let entry = entries
        .iter()
        .find(|entry| entry.file.ends_with("mk_core/mk_event.c"))
        .ok_or("missing Monkey adapter translation unit")?;
    let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
    let options = compiler_options(entry)?;
    let unit = C11TranslationUnit {
        source: &source,
        include_paths: options.include_paths,
        definitions: options.definitions,
        dialect: options.dialect,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
    };
    let results = import_monkey_callbacks(&unit, PINNED_FLUENT_BIT_COMMIT)?;
    for result in &results {
        match result {
            MonkeySourceResult::Summary(summary) => println!(
                "  monkey-libevent/{}: {} source access path(s); custody, failure paths, and runtime access refinement Unknown",
                summary.function(),
                summary.paths().len()
            ),
            MonkeySourceResult::Unknown { function, reason } => {
                println!("  monkey-libevent/{function}: callback source Unknown: {reason}")
            }
        }
    }
    let wrappers =
        qcore_compiler::verifier_plugins::libevent::monkey_source::import_monkey_wrappers(
            &unit,
            PINNED_FLUENT_BIT_COMMIT,
        )?;
    for result in &wrappers {
        match result {
            MonkeySourceResult::Summary(summary) if summary.has_scan_loop() => println!(
                "  monkey-wrapper/{}: {} structured source branch(es) with scan loop; loop bounds, arithmetic, termination and custody Unknown",
                summary.function(),
                summary.paths().len()
            ),
            MonkeySourceResult::Summary(summary) => println!(
                "  monkey-wrapper/{}: {} complete symbolic source path(s); opaque calls and custody Unknown",
                summary.function(),
                summary.paths().len()
            ),
            MonkeySourceResult::Unknown { function, reason } => {
                println!("  monkey-wrapper/{function}: source Unknown: {reason}")
            }
        }
    }
    if let Some(directory) = binding_directory {
        let read =
            |name| fs::read_to_string(directory.join(name)).map_err(|error| error.to_string());
        let bound =
            qcore_compiler::verifier_plugins::libevent::monkey_binding::check_monkey_bindings(
                &results,
                &read("monkey_observation.jsonl")?,
                &read("monkey_bindings.jsonl")?,
                include_str!("../../contract/qcore/monkey/composition.qcore"),
            )?;
        for callback in &bound.callbacks {
            println!(
                "  monkey-libevent/{}: {} bound native execution(s), checked paths {:?}, Unknown paths {:?}; {:?}",
                callback.function,
                callback.checked_executions,
                callback.checked_paths,
                callback.unknown_paths,
                callback.unknown_reasons
            );
        }
        fs::write(
            directory.join("monkey_binding_result.json"),
            serde_json::to_string_pretty(&bound).map_err(|e| e.to_string())? + "\n",
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(report) = report {
        let output = serde_json::json!({
            "schema_version": 2,
            "scan_loop_semantics": "Each iteration evaluates header, then matches; a true match executes early_return and exits the function. Otherwise store increment in the induction local and repeat. A false header continues outside the loop. Iteration symbols are scoped to one iteration. Bounds, arithmetic, termination and memory safety remain obligations.",
            "source_revision": PINNED_FLUENT_BIT_COMMIT,
            "compile_entry": { "directory": entry.directory, "command": entry.command, "file": entry.file },
            "source_projection_only": true,
            "source_policy_contract": include_str!("../../contract/qcore/library/source_projection.qcore"),
            "wrapper_behavior_specification": include_str!("../../contract/qcore/monkey/wrapper_paths.qcore"),
            "wrapper_behavior_specification_status": "syntax-preserved source findings; custody and behavioral predicate replay not certified",
            "callbacks": results,
            "wrappers": wrappers,
        });
        fs::write(
            report,
            serde_json::to_string_pretty(&output).map_err(|error| error.to_string())? + "\n",
        )
        .map_err(|error| format!("could not write {}: {error}", report.display()))?;
    }
    Ok(())
}

fn monkey_source_root(entries: &[CompileEntry]) -> Result<PathBuf, String> {
    let event_entry = entries
        .iter()
        .find(|entry| entry.file.ends_with("mk_core/mk_event.c"))
        .ok_or("Monkey compile database has no mk_core/mk_event.c entry")?;
    let options = compiler_options(event_entry)?;
    if !options
        .definitions
        .iter()
        .any(|definition| definition == "MK_EVENT_LOOP_LIBEVENT")
    {
        return Err("Monkey mk_event.c was not compiled with -DMK_EVENT_LOOP_LIBEVENT".into());
    }
    event_entry
        .file
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            format!(
                "could not derive Monkey source root from {}",
                event_entry.file.display()
            )
        })
}

fn is_monkey_unit(source_root: &Path, file: &Path) -> bool {
    let Ok(relative) = file.strip_prefix(source_root) else {
        return false;
    };
    if file.extension().is_none_or(|extension| extension != "c") {
        return false;
    }
    let relative = relative.to_string_lossy();
    (relative.starts_with("mk_core/") && !relative.starts_with("mk_core/deps/"))
        || relative.starts_with("mk_server/")
        || relative.starts_with("tls/")
}

fn verify_monkey_source_copy(
    canonical_root: &Path,
    compiled_root: &Path,
    entries: &[CompileEntry],
) -> Result<(), String> {
    for entry in entries {
        let relative = entry
            .file
            .strip_prefix(compiled_root)
            .map_err(|_| format!("source is outside Monkey root: {}", entry.file.display()))?;
        let canonical = canonical_root.join(relative);
        let compiled = fs::read(&entry.file)
            .map_err(|error| format!("could not read {}: {error}", entry.file.display()))?;
        let expected = fs::read(&canonical)
            .map_err(|error| format!("could not read {}: {error}", canonical.display()))?;
        if compiled != expected {
            return Err(format!(
                "compiled Monkey source differs from pinned Fluent Bit source: {}",
                relative.display()
            ));
        }
    }
    // The adapter is textually included, so it is absent from the standalone
    // command list. Bind its body, layout and core macro headers explicitly.
    for relative in [
        "mk_core/mk_event_libevent.c",
        "include/monkey/mk_core/mk_event.h",
        "include/monkey/mk_core/mk_event_libevent.h",
        "include/monkey/mk_core/mk_macros.h",
        "include/monkey/mk_core/mk_memory.h",
        "include/monkey/mk_core/mk_limits.h",
        "include/monkey/mk_core/mk_list.h",
    ] {
        let compiled = fs::read(compiled_root.join(relative)).map_err(|error| error.to_string())?;
        let expected =
            fs::read(canonical_root.join(relative)).map_err(|error| error.to_string())?;
        if compiled != expected {
            return Err(format!(
                "compiled Monkey callback source/layout differs from pinned source: {relative}"
            ));
        }
    }
    Ok(())
}

fn audit_monkey_libevent_contract(fluent_root: &Path) -> Result<(), String> {
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("compiler crate must be inside the QCore-C repository")?;
    let reference_root = repository_root.join(LIBEVENT_SUBMODULE_PATH);
    let libevent = load_profile(LIBEVENT_SOURCE_SET)
        .map_err(|error| format!("Libevent source-set profile unavailable: {error}"))?;
    let libevent_library = libevent
        .library(LIBEVENT_SOURCE_SET)
        .ok_or("the libevent profile declares no `libevent` library")?;
    let libevent_pin = libevent
        .pin_for(LIBEVENT_SOURCE_SET)
        .ok_or("the libevent library binds no pin")?;
    verify_pinned_libevent_source(&reference_root, libevent_pin, libevent_library)
        .map_err(|error| format!("could not verify Cycle 18 Libevent source: {error}"))?;

    let bundled_root = fluent_root.join("lib/monkey/mk_core/deps/libevent");
    let trusted_files = [
        "include/event2/event.h",
        "event.c",
        "bufferevent.c",
        "include/event2/thread.h",
        "include/event2/util.h",
        "evutil.c",
        "epoll.c",
    ];
    for relative in trusted_files {
        let reference = fs::read(reference_root.join(relative))
            .map_err(|error| format!("could not read reference {relative}: {error}"))?;
        let bundled = fs::read(bundled_root.join(relative))
            .map_err(|error| format!("could not read bundled {relative}: {error}"))?;
        if bundled != reference {
            return Err(format!(
                "Monkey's bundled Libevent differs in Cycle 18 contract file {relative}"
            ));
        }
    }

    let adapter_path = fluent_root.join("lib/monkey/mk_core/mk_event_libevent.c");
    let adapter = fs::read_to_string(&adapter_path)
        .map_err(|error| format!("could not read {}: {error}", adapter_path.display()))?;
    let symbols = prefixed_call_symbols(&adapter);
    let mut accepted = Vec::new();
    let mut unknown = Vec::new();
    for symbol in symbols {
        match classify_libevent_symbol(&libevent_library.api_dispositions, &symbol) {
            qcore_compiler::ProfileDisposition::Accepted => accepted.push(symbol),
            qcore_compiler::ProfileDisposition::Unknown => unknown.push(symbol),
        }
    }
    println!(
        "monkey-libevent adapter: {} accepted Cycle 18 API symbol(s): {}",
        accepted.len(),
        accepted.join(", ")
    );
    println!(
        "monkey-libevent adapter: {} symbol(s) remain Unknown: {}",
        unknown.len(),
        unknown.join(", ")
    );
    println!(
        "monkey bundled Libevent: {} Cycle 18 contract file(s) byte-identical",
        trusted_files.len()
    );
    Ok(())
}

fn prefixed_call_symbols(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut symbols = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let start = cursor;
        if bytes[cursor].is_ascii_alphabetic() || bytes[cursor] == b'_' {
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            let symbol = &source[start..cursor];
            let mut following = cursor;
            while bytes.get(following).is_some_and(u8::is_ascii_whitespace) {
                following += 1;
            }
            if (symbol.starts_with("event_") || symbol.starts_with("evutil_"))
                && bytes.get(following) == Some(&b'(')
            {
                symbols.push(symbol.to_owned());
            }
        } else {
            cursor += 1;
        }
    }
    symbols.sort();
    symbols.dedup();
    symbols
}

struct CompilerOptions {
    include_paths: Vec<String>,
    definitions: Vec<String>,
    dialect: CCompilerDialect,
}

fn compiler_options(entry: &CompileEntry) -> Result<CompilerOptions, String> {
    let words = shell_words(&entry.command)?;
    // The gateway preprocesses through stdin, so Clang cannot infer the normal
    // quoted-include search directory from the source filename.  Preserve that
    // part of the compilation-database contract explicitly.
    let mut include_paths = entry
        .file
        .parent()
        .map(|path| vec![path.to_string_lossy().into_owned()])
        .unwrap_or_default();
    let mut definitions = Vec::new();
    let mut dialect = CCompilerDialect::Gnu17;
    let mut cursor = 1;
    while cursor < words.len() {
        let word = &words[cursor];
        if word == "-I" || word == "-isystem" {
            cursor += 1;
            let path = words.get(cursor).ok_or("missing compiler include path")?;
            include_paths.push(resolve_path(&entry.directory, path));
        } else if let Some(path) = word.strip_prefix("-I") {
            include_paths.push(resolve_path(&entry.directory, path));
        } else if word == "-D" {
            cursor += 1;
            let definition = words.get(cursor).ok_or("missing compiler definition")?;
            if !definition.contains('$') {
                definitions.push(definition.clone());
            }
        } else if let Some(definition) = word.strip_prefix("-D") {
            if !definition.contains('$') {
                definitions.push(definition.to_owned());
            }
        } else if let Some(standard) = word.strip_prefix("-std=") {
            dialect = match standard {
                "c11" => CCompilerDialect::C11,
                "gnu99" => CCompilerDialect::Gnu99,
                "gnu11" => CCompilerDialect::Gnu11,
                "gnu17" => CCompilerDialect::Gnu17,
                other => return Err(format!("unsupported compilation dialect `{other}`")),
            };
        }
        cursor += 1;
    }
    Ok(CompilerOptions {
        include_paths,
        definitions,
        dialect,
    })
}

fn resolve_path(directory: &Path, path: &str) -> String {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_string_lossy().into_owned()
    } else {
        directory.join(path).to_string_lossy().into_owned()
    }
}

fn compile_entries(database: &str) -> Result<Vec<CompileEntry>, String> {
    let directories = json_string_fields(database, "directory")?;
    let commands = json_string_fields(database, "command")?;
    let files = json_string_fields(database, "file")?;
    if directories.len() != commands.len() || commands.len() != files.len() {
        return Err("compile database fields are not aligned".into());
    }
    Ok(directories
        .into_iter()
        .zip(commands)
        .zip(files)
        .map(|((directory, command), file)| CompileEntry {
            directory: PathBuf::from(directory),
            command,
            file: PathBuf::from(file),
        })
        .collect())
}

fn json_string_fields(input: &str, field: &str) -> Result<Vec<String>, String> {
    let marker = format!("\"{field}\"");
    let mut cursor = 0;
    let mut values = Vec::new();
    while let Some(relative) = input[cursor..].find(&marker) {
        cursor += relative + marker.len();
        let colon = input[cursor..]
            .find(':')
            .ok_or_else(|| format!("missing colon after `{field}`"))?;
        cursor += colon + 1;
        while input
            .as_bytes()
            .get(cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            cursor += 1;
        }
        if input.as_bytes().get(cursor) != Some(&b'"') {
            return Err(format!("`{field}` is not a JSON string"));
        }
        cursor += 1;
        let (value, next) = decode_json_string(input, cursor)?;
        values.push(value);
        cursor = next;
    }
    Ok(values)
}

fn decode_json_string(input: &str, mut cursor: usize) -> Result<(String, usize), String> {
    let bytes = input.as_bytes();
    let mut value = String::new();
    while let Some(&byte) = bytes.get(cursor) {
        match byte {
            b'"' => return Ok((value, cursor + 1)),
            b'\\' => {
                cursor += 1;
                let escaped = *bytes.get(cursor).ok_or("unterminated JSON escape")?;
                value.push(match escaped {
                    b'"' => '"',
                    b'\\' => '\\',
                    b'/' => '/',
                    b'b' => '\u{0008}',
                    b'f' => '\u{000c}',
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    _ => return Err("unsupported JSON escape in compile database".into()),
                });
            }
            _ if byte.is_ascii() => value.push(char::from(byte)),
            _ => return Err("non-ASCII compile database value is unsupported".into()),
        }
        cursor += 1;
    }
    Err("unterminated JSON string".into())
}

fn shell_words(command: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in command.chars() {
        if escaped {
            word.push(character);
            escaped = false;
        } else if character == '\\' && quote != Some('\'') {
            escaped = true;
        } else if let Some(active) = quote {
            if character == active {
                quote = None;
            } else {
                word.push(character);
            }
        } else if character == '\'' || character == '"' {
            quote = Some(character);
        } else if character.is_whitespace() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(character);
        }
    }
    if escaped || quote.is_some() {
        return Err("unterminated quoting in compile command".into());
    }
    if !word.is_empty() {
        words.push(word);
    }
    Ok(words)
}
