use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use qcore_compiler::{
    CompileOptions, PluginId, SourceFile, compile_files, first_party_registry, format_diagnostic,
};

const USAGE: &str = "qcorec 0.1.0\n\nUsage:\n  qcorec check [--entry NAME] FILE...\n  qcorec emit [--entry NAME] [-o FILE] FILE...\n  Source modes: module contract; | module library; | module executable;\n  qcorec plugins list\n  qcorec plugins inspect PLUGIN_ID\n";

fn main() -> ExitCode {
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
        println!("qcorec 0.1.0");
        return Ok(());
    }
    let command = arguments[0].as_str();
    if command == "plugins" {
        return plugins_command(&arguments[1..]);
    }
    if !matches!(command, "check" | "emit") {
        return Err(format!("unknown command `{command}`\n\n{USAGE}"));
    }
    let mut entry = None;
    let mut output = None::<PathBuf>;
    let mut inputs = Vec::new();
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
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
    let files = inputs
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .map(|source| SourceFile::new(path.display().to_string(), source))
                .map_err(|error| format!("{}: {error}", path.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let module = compile_files(&files, &CompileOptions { entry }).map_err(|diagnostics| {
        diagnostics
            .iter()
            .map(format_diagnostic)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    if command == "check" {
        if let Some(entry) = &module.entry {
            println!(
                "checked {} function(s); entry `{entry}`",
                module.checked.functions.len()
            );
        } else {
            println!(
                "checked {:?} module: {} declaration(s), {} function body/bodies; no entry point",
                module.mode,
                module.checked.source.declarations.len(),
                module.checked.functions.len()
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

fn plugins_command(arguments: &[String]) -> Result<(), String> {
    let registry = first_party_registry();
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
