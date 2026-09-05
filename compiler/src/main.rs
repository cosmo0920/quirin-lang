use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use qcore_compiler::{CompileOptions, SourceFile, compile_files, format_diagnostic};

const USAGE: &str = "qcorec 0.1.0\n\nUsage:\n  qcorec check [--entry NAME] FILE...\n  qcorec emit [--entry NAME] [-o FILE] FILE...\n";

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
        println!(
            "checked {} function(s); entry `{}`",
            module.checked.functions.len(),
            module.entry
        );
    } else if let Some(path) = output {
        fs::write(&path, module.c_source)
            .map_err(|error| format!("{}: {error}", path.display()))?;
    } else {
        print!("{}", module.c_source);
    }
    Ok(())
}
