use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use qcore_compiler::{
    C11TranslationUnit, CCompilerDialect, LegacyFunctionScope, ResourceApiKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, legacy_c_source_functions,
};

#[derive(Clone, Debug)]
struct CompileEntry {
    directory: PathBuf,
    command: String,
    file: PathBuf,
}

#[derive(Default)]
struct AuditCounts {
    units: usize,
    source_functions: usize,
    projected_functions: usize,
    safe_units: usize,
    unsafe_units: usize,
    unknown_units: usize,
    gateway_failures: usize,
}

fn main() -> ExitCode {
    let root = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../fluent-bit"));
    match run(&root) {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("Fluent Bit library audit failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(root: &Path) -> Result<bool, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("could not resolve {}: {error}", root.display()))?;
    let database_path = root.join("build/compile_commands.json");
    let database = fs::read_to_string(&database_path)
        .map_err(|error| format!("could not read {}: {error}", database_path.display()))?;
    let entries = compile_entries(&database)?;
    let revision = Command::new("git")
        .args([
            "-C",
            root.to_str().ok_or("non-UTF-8 Fluent Bit root")?,
            "rev-parse",
            "HEAD",
        ])
        .output()
        .map_err(|error| format!("could not run git: {error}"))?;
    if !revision.status.success() {
        return Err("could not identify the Fluent Bit revision".into());
    }
    println!(
        "Fluent Bit {}",
        String::from_utf8_lossy(&revision.stdout).trim()
    );

    let contracts = ResourceContracts::new([
        ("malloc", ResourceApiKind::Acquire),
        ("calloc", ResourceApiKind::Acquire),
        ("free", ResourceApiKind::Release),
    ]);
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
        let counts = audit_library(library, &selected, &contracts)?;
        failed |= counts.gateway_failures != 0;
        println!(
            "{library}: {} unit(s), {} source function(s), {} projected; \
             {} Safe, {} Unsafe, {} Unknown, {} gateway failure(s)",
            counts.units,
            counts.source_functions,
            counts.projected_functions,
            counts.safe_units,
            counts.unsafe_units,
            counts.unknown_units,
            counts.gateway_failures
        );
    }
    Ok(failed)
}

fn audit_library(
    library: &str,
    entries: &[CompileEntry],
    contracts: &ResourceContracts,
) -> Result<AuditCounts, String> {
    let mut counts = AuditCounts::default();
    for entry in entries {
        let source = fs::read_to_string(&entry.file)
            .map_err(|error| format!("could not read {}: {error}", entry.file.display()))?;
        let source_functions = legacy_c_source_functions(&source);
        if source_functions.is_empty() {
            continue;
        }
        let options = compiler_options(entry)?;
        let unit = C11TranslationUnit {
            source: &source,
            include_paths: options.include_paths,
            definitions: options.definitions,
            dialect: options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
        };
        counts.units += 1;
        counts.source_functions += source_functions.len();
        let file = entry
            .file
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("non-UTF-8 source path: {}", entry.file.display()))?;
        match analyze_unrestricted_c11(&unit, &source_functions[0], contracts) {
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
    }
    Ok(counts)
}

struct CompilerOptions {
    include_paths: Vec<String>,
    definitions: Vec<String>,
    dialect: CCompilerDialect,
}

fn compiler_options(entry: &CompileEntry) -> Result<CompilerOptions, String> {
    let words = shell_words(&entry.command)?;
    let mut include_paths = Vec::new();
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
