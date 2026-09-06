//! Compilation-database ingestion and the inherited bounded option adapter.
use qcore_c_gateway::legacy_c::CCompilerDialect;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct CompileEntry {
    pub directory: PathBuf,
    pub command: String,
    pub file: PathBuf,
}

pub struct CompilerOptions {
    pub include_paths: Vec<String>,
    pub definitions: Vec<String>,
    pub dialect: CCompilerDialect,
}

pub fn compiler_options_compatibility(entry: &CompileEntry) -> Result<CompilerOptions, String> {
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

pub fn shell_words(command: &str) -> Result<Vec<String>, String> {
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

/// Read actual compilation-database records atomically, rather than aligning
/// independent textual searches for similarly named fields.
pub fn compile_entries(database: &str) -> Result<Vec<CompileEntry>, String> {
    #[derive(serde::Deserialize)]
    struct Record {
        directory: PathBuf,
        file: PathBuf,
        command: Option<String>,
        arguments: Option<Vec<String>>,
    }
    let records: Vec<Record> = serde_json::from_str(database).map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    for record in records {
        if !record.directory.is_absolute() {
            return Err("compilation directory must be absolute".into());
        }
        let command =
            match (record.command, record.arguments) {
                (Some(command), None) => command,
                (None, Some(arguments)) if !arguments.is_empty() => arguments
                    .into_iter()
                    .map(|word| format!("'{}'", word.replace('\'', "'\\''")))
                    .collect::<Vec<_>>()
                    .join(" "),
                _ => return Err(
                    "each compile record must carry exactly one nonempty command or arguments list"
                        .into(),
                ),
            };
        if shell_words(&command)?.is_empty() {
            return Err("empty compile command".into());
        }
        let file = if record.file.is_absolute() {
            record.file
        } else {
            record.directory.join(record.file)
        };
        entries.push(CompileEntry {
            directory: record.directory,
            file,
            command,
        });
    }
    Ok(entries)
}

/// The configurable driver accepts only options whose preprocessing effects
/// this gateway can preserve. Existing pinned adapters retain their explicitly
/// bounded historical projection via `compiler_options_compatibility`.
pub fn compiler_options(entry: &CompileEntry) -> Result<CompilerOptions, String> {
    let words = shell_words(&entry.command)?;
    let mut cursor = 1;
    while cursor < words.len() {
        let word = &words[cursor];
        if matches!(word.as_str(), "-I" | "-isystem" | "-D" | "-o") {
            cursor += 1;
            let value = words.get(cursor).ok_or("missing compile option argument")?;
            if value.contains('$') {
                return Err("unexpanded compiler option is unsupported".into());
            }
        } else if word.starts_with("-I") || word.starts_with("-D") || word.starts_with("-std=") {
            if word.contains('$') {
                return Err("unexpanded compiler option is unsupported".into());
            }
        } else if word != "-c" && Path::new(&resolve_path(&entry.directory, word)) != entry.file {
            return Err(format!(
                "compiler option `{word}` cannot be preserved by the bounded gateway"
            ));
        }
        cursor += 1;
    }
    compiler_options_compatibility(entry)
}
