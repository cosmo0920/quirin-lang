mod plugin_config;
mod qrlib;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use qcore_compiler::verifier_plugin::ExecutionPolicy;
use qcore_compiler::{
    CompileOptions, PluginId, SourceFile, compile_files_with_filesystem_for_target,
    compile_files_with_filesystem_with_registry_for_target, format_diagnostic,
    load_first_party_registry,
};
use qcore_contract::contract_loader::{PublishedModuleRef, load_published_resource_contracts};
use qcore_contract::load_path_manifest::{LoadPathManifest, load_contract_load_path_manifest};

const USAGE: &str = "quirin 0.1.0 — the Quirin language driver\n\nUsage:\n  quirin check [--entry NAME] [--plugins CONFIG] [--native-library MANIFEST]... [--recorded-target FILE] FILE...\n  quirin check --contract-load-path MANIFEST FILE.qv\n  quirin emit [--entry NAME] [--plugins CONFIG] [--native-library MANIFEST]... [--recorded-target FILE] [-o FILE] FILE...\n  quirin qrlib generate --library NAME --version V --linkage static|dynamic --artifact REF [--header REF]\n      --contracts FILE.qv... --recorded-target FILE --unwinding abort --allocator-custody closed\n      --thread-posture single_caller [--os-build BUILD] [--runtime-dir DIR] -o OUT.qrlib\n  quirin qrlib check STORED.qrlib (the generate flags, without -o)\n  quirin build --recorded-target FILE --contracts FILE.qv... --native-library M.qrlib... [--entry NAME]\n      [--runtime-dir DIR] -o EXE FILE...\n  A .qrlib is generated from the artifact on every build; REF resolves against the manifest's\n  directory; check and build refuse a stored manifest the regenerated one differs from.\n  Source modes: module contract; | module library; | module executable;\n  Extensions: .qv contracts; .qr runtime/library; .qrlib native ABI manifests.\n  Legacy .qcore/.qcorelib inputs remain supported with outdated-name warnings.\n  Contract load-path check validates trusted resource-v1 assumptions and published\n  includes; it does not check C bodies or establish a safety verdict.\n  quirin plugins list [--config CONFIG]\n  quirin plugins profile [--config CONFIG]\n  quirin plugins inspect PLUGIN_ID [--config CONFIG]\n  quirin plugins verify PACKAGE_DIR\n  quirin plugins enable PACKAGE_DIR --config CONFIG\n  quirin plugins disable PLUGIN_ID --config CONFIG\n\n  verify/enable read the package without executing it: manifest syntax, path\n  containment, the declared executable digest, and every other declaration\n  against the template embedded in the executable's bytes. That is a snapshot\n  binding, not a signature or an immutable-execution guarantee.\n";

fn main() -> ExitCode {
    // Card RULE-SWITCH-ENTRY-POINTS: `quirin` records no gateway rule set.
    qcore_compiler::gateway_rules::exit_if_unrecorded("quirin");
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<String>) -> Result<(), String> {
    if arguments.is_empty()
        || arguments
            .iter()
            .any(|argument| argument == "--help" || argument == "-h")
    {
        print!("{USAGE}");
        return Ok(());
    }
    if arguments.len() == 1 && arguments[0] == "--version" {
        println!("quirin 0.1.0");
        return Ok(());
    }
    let command = arguments[0].as_str();
    if command == "plugins" {
        return plugins_command(&arguments[1..]);
    }
    // Card `W48-QRLIB-LINKAGE`: the generated `.qrlib` and the build that
    // regenerates it, checks the program and links exactly the pinned artifact.
    if command == "qrlib" {
        return qrlib::qrlib_command(&arguments[1..]);
    }
    if command == "build" {
        return qrlib::build_command(&arguments[1..]);
    }
    if !matches!(command, "check" | "emit") {
        return Err(format!("unknown command `{command}`\n\n{USAGE}"));
    }
    let mut entry = None;
    let mut plugins = None::<PathBuf>;
    let mut contract_load_path = None::<PathBuf>;
    let mut output = None::<PathBuf>;
    let mut manifests = Vec::<PathBuf>::new();
    let mut recorded_target = None::<PathBuf>;
    let mut inputs = Vec::new();
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--plugins" => {
                index += 1;
                if plugins.is_some() {
                    return Err("--plugins specified more than once".into());
                }
                plugins = Some(PathBuf::from(
                    arguments
                        .get(index)
                        .ok_or("--plugins requires a config path")?,
                ));
            }
            "--contract-load-path" if command == "check" => {
                index += 1;
                if contract_load_path.is_some() {
                    return Err("--contract-load-path specified more than once".into());
                }
                contract_load_path = Some(PathBuf::from(
                    arguments
                        .get(index)
                        .ok_or("--contract-load-path requires a manifest path")?,
                ));
            }
            "--native-library" => {
                index += 1;
                manifests.push(PathBuf::from(
                    arguments
                        .get(index)
                        .ok_or("--native-library requires a manifest path")?,
                ));
            }
            // Card `W48-QR-SCALAR-TYPES`: the builder's recorded target, which
            // alone supplies the width of `isize`/`usize`. Selecting none is
            // the default and changes nothing for a program naming neither.
            "--recorded-target" => {
                index += 1;
                if recorded_target.is_some() {
                    return Err("--recorded-target specified more than once".into());
                }
                recorded_target = Some(PathBuf::from(
                    arguments
                        .get(index)
                        .ok_or("--recorded-target requires a file path")?,
                ));
            }
            "--entry" => {
                index += 1;
                entry = Some(
                    arguments
                        .get(index)
                        .ok_or("--entry requires a name")?
                        .clone(),
                );
            }
            "-o" | "--output" if command == "emit" => {
                index += 1;
                output = Some(PathBuf::from(
                    arguments.get(index).ok_or("-o requires a path")?,
                ));
            }
            argument if argument.starts_with('-') => {
                return Err(format!("unknown option `{argument}`"));
            }
            path => inputs.push(PathBuf::from(path)),
        }
        index += 1;
    }
    if inputs.is_empty() {
        return Err("no input files were provided".into());
    }
    if let Some(path) = contract_load_path {
        if inputs.len() != 1
            || entry.is_some()
            || plugins.is_some()
            || !manifests.is_empty()
            || recorded_target.is_some()
        {
            return Err(
                "--contract-load-path requires exactly one contract input and cannot be combined with --entry, --plugins, --native-library or --recorded-target"
                    .into(),
            );
        }
        return check_published_resource_contract(&path, &inputs[0]);
    }
    let files = inputs
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .map(|source| SourceFile::new(path.display().to_string(), source))
                .map_err(|error| format!("{}: {error}", path.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    // The CLI selects no extern contract environment. A source file that calls
    // a declared extern function is therefore refused at `Q023` through this
    // entry point; a selection flag is a named follow-on, not a default.
    //
    // It *does* select native-library manifests, one per `--native-library`
    // (card `UNSAFE-LINKAGE-ARTIFACT`). Selecting none is the default and keeps
    // every existing invocation byte-identical; selecting one sharpens the
    // waiver premises of the symbols it names, and the sharpened premise is
    // printed below. A manifest that does not load refuses the module rather
    // than being ignored.
    let native_libraries = if manifests.is_empty() {
        None
    } else {
        let mut selected = std::collections::BTreeMap::new();
        for path in &manifests {
            let text =
                fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
            if selected.insert(path.display().to_string(), text).is_some() {
                return Err(format!(
                    "the manifest `{}` is selected more than once",
                    path.display()
                ));
            }
        }
        Some(selected)
    };
    // Card `NATIVE-LIBRARY-HEADER`. A manifest that names an exported header
    // needs that header's text, and the CLI is the layer that performs I/O. The
    // reference is resolved against the **manifest's own directory** — one
    // candidate path, no search, no fallback — because the manifest is the only
    // anchor this binary has for a reference the manifest itself spelled, and a
    // search would make two byte-identical manifests mean different things
    // depending on where they were invoked from (`HEADER-CLI-RELATIVE`).
    //
    // A manifest naming no header supplies nothing here and is unchanged. A
    // header that cannot be read is an error rather than an omission: the driver
    // would refuse at `Q063` anyway, and reporting the `open` failure names the
    // real cause.
    let native_library_headers = match native_libraries.as_ref() {
        None => None,
        Some(_) => {
            let mut headers = std::collections::BTreeMap::new();
            for path in &manifests {
                let text = fs::read_to_string(path)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
                let Ok(manifest) =
                    qcore_compiler::native_library::load_native_library_manifest(&text)
                else {
                    // A manifest that does not load is refused by the driver,
                    // with its own named refusal. Nothing is guessed here.
                    continue;
                };
                let Some(binding) = manifest.header else {
                    continue;
                };
                let directory = path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .to_owned();
                // Card `NATIVE-LIBRARY-HEADER-DECORATION`: the texts the header's
                // own `#include`s are read from resolve the same way, against the
                // manifest's own directory and with no search — one reference,
                // one candidate path. They travel in the same map as the header,
                // keyed by the same kind of reference, because they are the same
                // kind of input: a text this level reads and identifies.
                for include in &binding.includes {
                    let resolved = directory.join(&include.reference);
                    let text = fs::read_to_string(&resolved)
                        .map_err(|error| format!("{}: {error}", resolved.display()))?;
                    headers.insert(include.reference.clone(), text);
                }
                let resolved = directory.join(&binding.reference);
                let header = fs::read_to_string(&resolved)
                    .map_err(|error| format!("{}: {error}", resolved.display()))?;
                headers.insert(binding.reference, header);
            }
            (!headers.is_empty()).then_some(headers)
        }
    };
    let options = CompileOptions {
        entry,
        extern_contracts: None,
        native_libraries,
        native_library_headers,
    };
    // A record that does not load is an error rather than an omission: a
    // program naming `isize`/`usize` would otherwise be checked for no width
    // at all, and nothing here guesses one.
    let target = match &recorded_target {
        None => None,
        Some(path) => {
            let text =
                fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
            Some(
                qcore_compiler::scalar_types::load_recorded_target(&text)
                    .map_err(|error| format!("{}: {error}", path.display()))?,
            )
        }
    };
    let compiled = if let Some(path) = plugins {
        let registry = plugin_config::registry(&path)?;
        compile_files_with_filesystem_with_registry_for_target(
            &files,
            &options,
            &registry,
            target.as_ref(),
        )
    } else {
        compile_files_with_filesystem_for_target(&files, &options, target.as_ref())
    };
    let module = compiled.map_err(|diagnostics| {
        diagnostics
            .iter()
            .map(format_diagnostic)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    if command == "check" {
        if let Some(entry) = &module.entry {
            let profile = match module.execution_profile {
                qcore_compiler::SourceExecutionProfile::ScalarCallsV1 => match &module.checked {
                    qcore_compiler::CheckedSourceModule::ScalarCalls(checked)
                        if checked.is_widened() =>
                    {
                        "; source.scalar-calls.v1: fixed-width scalar calls; checked arithmetic; explicit conversions; std.fmt/std.io/std.logger lines; closed entry observation"
                    }
                    _ => {
                        "; source.scalar-calls.v1: i32/unit calls; checked arithmetic; explicit std.io; closed entry observation"
                    }
                },
                qcore_compiler::SourceExecutionProfile::ResourceCallsV1 => {
                    "; source.resource-calls.v1: acyclic owned bytes moves and whole-buffer read/write loans"
                }
                qcore_compiler::SourceExecutionProfile::ScopedReadSharesV1 => {
                    "; scoped owned-buffer tasks with counted read shares (Linux/macOS/Windows threads)"
                }
                qcore_compiler::SourceExecutionProfile::ScopedErrorPayloadV1 => {
                    "; scoped owned-buffer tasks with one typed error payload (Linux/macOS/Windows threads)"
                }
                qcore_compiler::SourceExecutionProfile::ScopedReadLoansV1 => {
                    "; scoped owned-buffer tasks with one read loan (Linux/macOS/Windows threads)"
                }
                qcore_compiler::SourceExecutionProfile::ScopedOwnedTasksV1 => {
                    "; scoped owned-buffer tasks (Linux/macOS/Windows threads)"
                }
                qcore_compiler::SourceExecutionProfile::ScopedCopyTasksV1 => {
                    "; scoped copy tasks (Linux/macOS/Windows threads)"
                }
                qcore_compiler::SourceExecutionProfile::CheckedMutexGuardsV1 => {
                    "; checked mutex guards (Linux/macOS/Windows threads)"
                }
                qcore_compiler::SourceExecutionProfile::FlatEpochBoundV1 => {
                    "; flat-epoch quantitative bound (checked accounting only, no emission)"
                }
                qcore_compiler::SourceExecutionProfile::ComposedEpochTasksV1 => {
                    "; scoped tasks with an agreeing flat-epoch bound (Linux/macOS/Windows threads)"
                }
                // Card `W48-QR-C-STRUCT-IMPORT`: the file-wide trust mark is
                // part of the verdict line, never only a premise below it.
                qcore_compiler::SourceExecutionProfile::FfiV1 => {
                    "; source.ffi.v1: imported C structs checked against the recorded compiler; calls through #c_ffi contracts; std.fmt/std.io/std.logger lines; Safe under the unsafe-ffi assumption, never plain Safe"
                }
                qcore_compiler::SourceExecutionProfile::ComposedStdIoExternV1 => {
                    "; declared extern calls composed with rendered lines (both models checked)"
                }
                qcore_compiler::SourceExecutionProfile::ComposedStdIoTasksV1 => {
                    "; scoped tasks composed with rendered lines in the parent and in \
                     spawned workers (Linux/macOS/Windows threads)"
                }
                qcore_compiler::SourceExecutionProfile::EventV1 => {
                    if module.c_source.is_some() {
                        "; one checked event context over program-induced readiness \
                         (poll(2)/WSAPoll)"
                    } else {
                        // The `accept_check_only` arm. Saying so here rather
                        // than silently emitting nothing is the point: a reader
                        // sees that the module was checked in full and that its
                        // execution is outside v1's determinism boundary.
                        "; one checked event context (checked only: a write \
                         registration's readiness is not program-induced, so no C is emitted)"
                    }
                }
                _ => "",
            };
            if let qcore_compiler::CheckedSourceModule::BitfieldTasks(checked) = &module.checked {
                println!(
                    "checked field-footprint independence for {} worker write(s); entry `{entry}`; \
                     property-only successful spawn/join window; no scoped-task lifetime proof or C emission",
                    checked.lowered.writes.len()
                );
            } else {
                println!(
                    "checked {} function(s); entry `{entry}`{profile}",
                    module.checked.function_count()
                );
            }
            print_declared_assumptions(&module);
        } else {
            println!(
                "checked {:?} module: {} declaration(s), {} function body/bodies; no entry point",
                module.mode,
                module.declaration_count,
                module.checked.function_count()
            );
        }
    } else {
        let source = module.c_source.as_ref().ok_or(
            "library/contract mode supports checking declarations and bodies; standalone C emission requires an executable module"
        )?;
        if let Some(path) = output {
            fs::write(&path, source).map_err(|error| format!("{}: {error}", path.display()))?;
        } else {
            print!("{source}");
        }
    }
    Ok(())
}

/// This route validates one owned contract input as a trusted resource-v1
/// assumption. The existing compiler route still rejects unresolved inclusion.
fn check_published_resource_contract(manifest_path: &Path, input: &Path) -> Result<(), String> {
    let manifest = load_contract_load_path_manifest(manifest_path).map_err(|e| e.to_string())?;
    let (crate_name, module) = selected_owned_module(&manifest, input)?;
    let selected = manifest
        .load_path
        .crate_named(&crate_name)
        .and_then(|krate| krate.modules.get(&module))
        .ok_or_else(|| format!("{}: module is not owned by the load path", input.display()))?;
    let input_text =
        fs::read_to_string(input).map_err(|error| format!("{}: {error}", input.display()))?;
    if &input_text != selected {
        return Err(format!(
            "{}: input differs from the load path's owned source snapshot",
            input.display()
        ));
    }
    load_published_resource_contracts(
        &manifest.load_path,
        &[PublishedModuleRef::new(&crate_name, &module)],
    )
    .map_err(|error| format!("{}: {error}", input.display()))?;
    println!(
        "validated resource-v1 contract assumptions from `{}` ({crate_name}:{module}); published includes resolved; no C bodies checked or safety verdict established",
        input.display()
    );
    Ok(())
}

/// Canonical paths establish membership only. The user-spelled `input` is kept
/// for diagnostics and the result, never replaced with the canonical spelling.
fn selected_owned_module(
    manifest: &LoadPathManifest,
    input: &Path,
) -> Result<(String, String), String> {
    let actual = input
        .canonicalize()
        .map_err(|error| format!("{}: {error}", input.display()))?;
    let mut selected = None;
    for (crate_name, root) in &manifest.contract_roots {
        let canonical_root = root
            .canonicalize()
            .map_err(|error| format!("{}: {error}", root.display()))?;
        let Ok(relative) = actual.strip_prefix(&canonical_root) else {
            continue;
        };
        let module = relative
            .to_str()
            .ok_or_else(|| format!("{}: contract path is not UTF-8", input.display()))?
            .replace('\\', "/");
        if manifest
            .load_path
            .crate_named(crate_name)
            .is_some_and(|krate| krate.modules.contains_key(&module))
        {
            if input.file_name() != relative.file_name() {
                return Err(format!(
                    "{}: input filename must match its owned module path; no alias substitution is allowed",
                    input.display()
                ));
            }
            if selected.replace((crate_name.clone(), module)).is_some() {
                return Err(format!(
                    "{}: input is owned by more than one load-path crate",
                    input.display()
                ));
            }
        }
    }
    selected.ok_or_else(|| {
        format!(
            "{}: input is not an owned contract module on the load path",
            input.display()
        )
    })
}

/// Print the declared premises a checked module's result holds relative to
/// (card `UNSAFE-LINKAGE`).
///
/// A module that linked an uncontracted symbol is checked *under* that waiver,
/// so this is the surface at which a reader sees it. Printing nothing when
/// there is nothing to print keeps every existing route's output byte-identical.
fn print_declared_assumptions(module: &qcore_compiler::CompiledModule) {
    use qcore_compiler::extern_call::ExternAssumption;
    use qcore_compiler::native_library::{
        AllocatorCustody, DataModel, ThreadPosture, UnwindingPosture,
    };

    let assumptions = module.declared_assumptions();
    // Card `W48-QR-SCALAR-TYPES`: a program naming `isize`/`usize` holds only
    // for the recorded target's width, and says so here beside the others.
    let target = module.target_assumption();
    // Card `W48-QR-C-STRUCT-IMPORT`: an FFI file's premises, the unsafe-FFI
    // trust mark first. Empty on every other route, so no existing output moves.
    let ffi = module.ffi_assumptions();
    // Card `W48-QR-FLOATING-POINT`: a program holding an `f32`/`f64` value
    // holds only for an IEEE evaluation, and says so here too.
    let float = module.float_assumption();
    if assumptions.is_empty() && target.is_none() && ffi.is_empty() && float.is_none() {
        return;
    }
    if let Some(verdict) = module.published_verdict() {
        println!("verdict: {}", verdict.render());
    }
    println!(
        "under {} declared assumption(s):",
        assumptions.len()
            + usize::from(target.is_some())
            + ffi.len()
            + usize::from(float.is_some())
    );
    for assumption in ffi {
        println!("  - {assumption}");
    }
    if let Some(target) = target {
        println!("  - recorded target: {target}");
    }
    if let Some(float) = float {
        println!("  - IEEE floating point: {float}");
    }
    for assumption in assumptions {
        let line = match assumption {
            ExternAssumption::UncheckedLinkage { callee, signature } => format!(
                "unchecked linkage: `{callee}` is linked as `{signature}` with no contract; nothing about its behavior is checked"
            ),
            ExternAssumption::ManifestUncheckedLinkage {
                callee,
                signature,
                residence,
            } => {
                let identity = &residence.identity;
                let premises = &residence.premises;
                format!(
                    "unchecked linkage: `{callee}` is linked as `{signature}` with no contract, from artifact `{}` of library `{} {}` at {}, under stated ABI premises ({}, {}, {}, {}); the artifact identity is declared, not verified here, and nothing about the callee's behavior is checked{}",
                    identity.artifact,
                    identity.library,
                    identity.version,
                    identity.hash,
                    match premises.data_model {
                        DataModel::Lp64 => "data model lp64",
                        DataModel::Llp64 => "data model llp64",
                    },
                    match premises.unwinding {
                        UnwindingPosture::Abort => "no unwinding across the boundary",
                    },
                    match premises.allocator_custody {
                        AllocatorCustody::Closed => "closed allocator custody",
                    },
                    match premises.thread_posture {
                        ThreadPosture::SingleCaller => "single-threaded caller",
                    },
                    header_clause(residence),
                )
            }
            ExternAssumption::DeclaredContract(callee) => {
                format!("declared contract: `{callee}` behaves as its selected contract states")
            }
            // Card `NATIVE-LIBRARY-CONTRACT-SLICE`. Printed **beside** the
            // declared-contract line above, never in place of it: the two
            // premises stack, and a reader who saw only this one would have
            // been shown which build without being shown that its behaviour is
            // a declaration.
            //
            // Unreachable through this binary today, and deliberately present
            // anyway: `quirin` selects no extern contract environment (follow-on
            // `UNSAFE-LINKAGE-CLI`), so no contracted callee is admitted here at
            // all. Without the arm the premise would render through the
            // `{other:?}` fallback the moment that follow-on lands, which is how
            // a premise stops being read.
            ExternAssumption::ManifestResidentContract {
                callee,
                contract_module,
                residence,
            } => {
                let identity = &residence.identity;
                let premises = &residence.premises;
                format!(
                    "contracted linkage: `{callee}` is read through the declared contract `{contract_module}`, from artifact `{}` of library `{} {}` at {}, under stated ABI premises ({}, {}, {}, {}); the artifact identity is declared, not verified here{}",
                    identity.artifact,
                    identity.library,
                    identity.version,
                    identity.hash,
                    match premises.data_model {
                        DataModel::Lp64 => "data model lp64",
                        DataModel::Llp64 => "data model llp64",
                    },
                    match premises.unwinding {
                        UnwindingPosture::Abort => "no unwinding across the boundary",
                    },
                    match premises.allocator_custody {
                        AllocatorCustody::Closed => "closed allocator custody",
                    },
                    match premises.thread_posture {
                        ThreadPosture::SingleCaller => "single-threaded caller",
                    },
                    header_clause(residence),
                )
            }
            // Card `NATIVE-LIBRARY-SOURCE-BINDING`. The file's own binding act,
            // printed beside the callee premises and never in place of one: a
            // reader who saw only this line would know which build the file
            // named without knowing what any symbol was read as, and a reader
            // who saw only the callee lines would not know the file had said
            // anything at all.
            //
            // **Reachable through this binary**, unlike the contracted arm
            // above: `quirin` selects no extern *contract* environment
            // (`UNSAFE-LINKAGE-CLI`) but it does select manifests, one per
            // `--native-library`, so a file that binds one and declares waived
            // callees prints this line here.
            ExternAssumption::SourceBoundLibrary(binding) => {
                let identity = &binding.identity;
                let premises = &binding.premises;
                format!(
                    "source library binding: this file declared `extern library {}`, which resolved through the manifest `{}` to artifact `{}` of library `{} {}` at {}, under stated ABI premises ({}, {}, {}, {}); the artifact identity is declared, not verified here, and this premise says which build the file's extern surface was resolved against — not what any symbol does{}",
                    binding.named,
                    binding.manifest,
                    identity.artifact,
                    identity.library,
                    identity.version,
                    identity.hash,
                    // The **same prose** the two arms above render, not the
                    // manifest's own spellings: a reader of one premise list
                    // must not be shown two vocabularies for one vocabulary.
                    match premises.data_model {
                        DataModel::Lp64 => "data model lp64",
                        DataModel::Llp64 => "data model llp64",
                    },
                    match premises.unwinding {
                        UnwindingPosture::Abort => "no unwinding across the boundary",
                    },
                    match premises.allocator_custody {
                        AllocatorCustody::Closed => "closed allocator custody",
                    },
                    match premises.thread_posture {
                        ThreadPosture::SingleCaller => "single-threaded caller",
                    },
                    match binding.header.as_ref() {
                        None => String::new(),
                        Some(header) => format!(
                            "; its exported header `{}` was read at {} — that digest is verified, not declared",
                            header.reference, header.digest
                        ),
                    },
                )
            }
            ExternAssumption::NoExtentTransport => {
                "no extent transport: no declaration says how many bytes a callee touches".into()
            }
            ExternAssumption::LinkageSuppliedByBuilder => {
                "linkage: which object file supplies each symbol is the builder's premise".into()
            }
            other => format!("{other:?}"),
        };
        println!("  - {line}");
    }
}

/// Card `NATIVE-LIBRARY-HEADER`. The clause a manifest-resident premise gains
/// when its manifest bound an exported header, and the empty string when it did
/// not — which keeps every premise printed before this card byte-identical.
///
/// The wording says **verified**, and it must: this is the one identity in the
/// record whose text was read and whose digest was measured over exactly the
/// bytes the decision was taken on. The library, version and artifact hash
/// beside it are declared and nothing opens them. A reader who could not tell
/// the two apart would have been handed a stronger-looking statement than the
/// one that was checked.
fn header_clause(residence: &qcore_compiler::native_library::ManifestResidence) -> String {
    match residence.header.as_ref() {
        None => String::new(),
        Some(header) => {
            // Card `NATIVE-LIBRARY-HEADER-DECORATION`: the spliced texts are
            // named too, and for the same reason the header is. The reading of a
            // decorated declaration is a function of *their* bytes as much as of
            // the header's, so a certificate that named only the header would
            // publish a statement a recheck could not reproduce.
            if header.includes.is_empty() {
                format!(
                    "; its exported header `{}` was read at {} and every bound declaration was compared against it — that digest is verified, not declared",
                    header.reference, header.digest
                )
            } else {
                format!(
                    "; its exported header `{}` was read at {}, reading {}, and every bound declaration was compared against it — those digests are verified, not declared",
                    header.reference,
                    header.digest,
                    header
                        .includes
                        .iter()
                        .map(|include| format!(
                            "`{}` ({}) at {}",
                            include.reference, include.spelling, include.digest
                        ))
                        .collect::<Vec<_>>()
                        .join(" and ")
                )
            }
        }
    }
}

fn plugins_command(arguments: &[String]) -> Result<(), String> {
    use qcore_compiler::verifier_plugin::{VerifierProvider, verify_executable_package};
    match arguments {
        [command, package] if command == "verify" => {
            let provider = verify_executable_package(package).map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(provider.manifest()).map_err(|e| e.to_string())?
            );
            return Ok(());
        }
        [command, value, flag, config] if flag == "--config" && command == "enable" => {
            plugin_config::enable(std::path::Path::new(config), std::path::Path::new(value))?;
            println!("enabled package in {config}");
            return Ok(());
        }
        [command, value, flag, config] if flag == "--config" && command == "disable" => {
            plugin_config::disable(std::path::Path::new(config), value)?;
            println!("disabled {value} in {config}");
            return Ok(());
        }
        _ => {}
    }
    let (arguments, config) =
        if arguments.len() >= 2 && arguments[arguments.len() - 2] == "--config" {
            (
                &arguments[..arguments.len() - 2],
                Some(&arguments[arguments.len() - 1]),
            )
        } else {
            (arguments, None)
        };
    let registry = if let Some(path) = config {
        plugin_config::registry(std::path::Path::new(path))?
    } else {
        load_first_party_registry(&qcore_compiler::source_profiles::profiles_root())
            .map_err(|error| format!("provider discovery failed: {error}"))?
    };
    match arguments {
        [command] if command == "list" => {
            for manifest in registry.manifests() {
                let capabilities = manifest
                    .capabilities
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                println!(
                    "{}\t{}\t{}",
                    manifest.plugin_id, manifest.package_version, capabilities
                );
            }
            Ok(())
        }
        // The configured execution profile, stated rather than implied. With no
        // `--config` there is no configuration to read, so the answer is the
        // recorded default; a configuration answers with what it selected.
        [command] if command == "profile" => {
            let policy = match config {
                Some(path) => plugin_config::execution_policy(std::path::Path::new(path))?,
                None => ExecutionPolicy::default(),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&policy).map_err(|error| error.to_string())?
            );
            Ok(())
        }
        [command, raw_id] if command == "inspect" => {
            let id = PluginId::new(raw_id).map_err(|error| error.to_string())?;
            let manifest = registry
                .manifest(&id)
                .ok_or_else(|| format!("plugin `{id}` is not installed"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(manifest).map_err(|error| error.to_string())?
            );
            Ok(())
        }
        _ => Err(format!("invalid plugins command\n\n{USAGE}")),
    }
}
