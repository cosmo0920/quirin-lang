//! Compilation-database ingestion and the inherited bounded option adapter.
use qcore_c_gateway::legacy_c::{
    CCompilerDialect, CFrontendFlag, CPreprocessorOption, CSourceInput,
};
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
    pub preprocessor_options: Vec<CPreprocessorOption>,
    /// How the gateway must hand this entry's source to Clang.
    pub input: CSourceInput,
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
        preprocessor_options: Vec::new(),
        // The pinned Cycle 19 route keeps its archived standard-input
        // projection, including `<stdin>` file names. It is a declared
        // assumption of those artifacts, not an equivalence claim.
        input: CSourceInput::HistoricalStdin,
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
    let mut started = false;
    let mut characters = command.chars().peekable();
    while let Some(character) = characters.next() {
        match quote {
            Some('\'') => {
                if character == '\'' {
                    quote = None;
                } else {
                    word.push(character);
                }
            }
            Some('"') => match character {
                '"' => quote = None,
                '\\' => match characters.peek().copied() {
                    Some(next @ ('$' | '`' | '"' | '\\' | '\n')) => {
                        characters.next();
                        if next != '\n' {
                            word.push(next);
                        }
                    }
                    Some(_) => word.push('\\'),
                    None => return Err("unterminated quoting in compile command".into()),
                },
                '$' | '`' => return Err("shell expansion in compile command is unsupported".into()),
                _ => word.push(character),
            },
            None => match character {
                '\'' | '"' => {
                    quote = Some(character);
                    started = true;
                }
                '\\' => {
                    let next = characters
                        .next()
                        .ok_or("unterminated quoting in compile command")?;
                    if next != '\n' {
                        word.push(next);
                        started = true;
                    }
                }
                '$' | '`' | ';' | '|' | '&' | '<' | '>' | '(' | ')' | '*' | '?' | '[' => {
                    return Err("shell expression in compile command is unsupported".into());
                }
                value if value.is_whitespace() => {
                    if started {
                        words.push(std::mem::take(&mut word));
                        started = false;
                    }
                }
                _ => {
                    word.push(character);
                    started = true;
                }
            },
            _ => unreachable!(),
        }
    }
    if quote.is_some() {
        return Err("unterminated quoting in compile command".into());
    }
    if started {
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

// Resolve names without executing the compile-database executable. Only the
// gateway's validated Clang command is probed, by its existing discovery API.
fn executable_path(program: &str, directory: &Path) -> Result<PathBuf, String> {
    let path = Path::new(program);
    let selected = if path.is_absolute() {
        path.to_owned()
    } else if path.components().count() > 1 {
        directory.join(path)
    } else {
        let search = std::env::var_os("PATH").ok_or("PATH is unavailable")?;
        std::env::split_paths(&search)
            .flat_map(|search_directory| {
                let search_directory = if search_directory.is_absolute() {
                    search_directory
                } else {
                    directory.join(search_directory)
                };
                let mut candidates = vec![search_directory.join(path)];
                if cfg!(windows) && path.extension().is_none() {
                    candidates.push(search_directory.join(format!("{program}.exe")));
                }
                candidates
            })
            .find(|path| {
                #[cfg(unix)]
                let executable = {
                    use std::os::unix::fs::PermissionsExt;
                    path.metadata()
                        .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
                };
                #[cfg(not(unix))]
                let executable = true;
                path.is_file() && executable
            })
            .ok_or_else(|| format!("compiler executable `{program}` cannot be resolved"))?
    };
    selected.canonicalize().map_err(|error| error.to_string())
}

/// The configurable driver accepts only options whose preprocessing effects
/// this gateway can preserve. Existing pinned adapters retain their explicitly
/// bounded historical projection via `compiler_options_compatibility`.
pub fn compiler_options(entry: &CompileEntry) -> Result<CompilerOptions, String> {
    if !entry.directory.is_absolute() || !entry.file.is_absolute() {
        return Err("compile directory and source must be absolute".into());
    }
    let words = shell_words(&entry.command)?;
    if words.first().is_none_or(String::is_empty) {
        return Err("empty compiler executable".into());
    }
    // Clang is given this entry's own input word, run from this entry's own
    // directory, so `__FILE__` and the includer-directory rule for quoted
    // includes answer exactly as they do for the recorded compile command. The
    // gateway no longer approximates the includer directory with `-iquote`:
    // that synthesized path loses to whatever directory a standard-input run
    // happens to start in.
    let mut options = CompilerOptions {
        include_paths: Vec::new(),
        definitions: Vec::new(),
        dialect: CCompilerDialect::Gnu17,
        preprocessor_options: Vec::new(),
        input: CSourceInput::AnonymousText,
    };
    let mut cursor = 1;
    let mut spelled_input = None;
    let mut inputs = 0;
    let mut compile = 0;
    let mut outputs = 0;
    let mut explicit_dialect = false;
    let mut frontend_flags = false;
    while cursor < words.len() {
        let word = &words[cursor];
        let split = ["-isystem", "-iquote", "-I", "-D", "-U"]
            .into_iter()
            .find_map(|flag| word.strip_prefix(flag).map(|suffix| (flag, suffix)));
        if let Some((flag, suffix)) = split {
            let value = if suffix.is_empty() {
                cursor += 1;
                words
                    .get(cursor)
                    .ok_or("missing compile option argument")?
                    .as_str()
            } else {
                suffix
            };
            if value.is_empty() || value.contains('$') || (flag == "-I" && value == "-") {
                return Err("empty or unexpanded compiler option is unsupported".into());
            }
            if matches!(flag, "-I" | "-isystem" | "-iquote") && value.starts_with('=') {
                return Err("sysroot-relative include paths are unsupported".into());
            }
            let option = match flag {
                "-I" => CPreprocessorOption::Include(resolve_path(&entry.directory, value)),
                "-isystem" => {
                    CPreprocessorOption::SystemInclude(resolve_path(&entry.directory, value))
                }
                "-iquote" => {
                    CPreprocessorOption::QuoteInclude(resolve_path(&entry.directory, value))
                }
                "-D" => CPreprocessorOption::Define(value.into()),
                "-U" => CPreprocessorOption::Undefine(value.into()),
                _ => unreachable!(),
            };
            options.preprocessor_options.push(option);
        } else if let Some(flag) = match word.as_str() {
            "-Wall" => Some(CFrontendFlag::AllWarnings),
            "-g" => Some(CFrontendFlag::DebugInformation),
            "-fPIC" => Some(CFrontendFlag::PositionIndependentCode),
            "-Wa,--noexecstack" => Some(CFrontendFlag::AssemblerNoExecStack),
            _ => None,
        } {
            frontend_flags = true;
            options
                .preprocessor_options
                .push(CPreprocessorOption::FrontendFlag(flag));
        } else if let Some(standard) = word.strip_prefix("-std=") {
            explicit_dialect = true;
            options.dialect = match standard {
                "c11" => CCompilerDialect::C11,
                "gnu99" => CCompilerDialect::Gnu99,
                "gnu11" => CCompilerDialect::Gnu11,
                "gnu17" => CCompilerDialect::Gnu17,
                other => return Err(format!("unsupported compilation dialect `{other}`")),
            };
        } else if word == "-o" {
            cursor += 1;
            let value = words.get(cursor).ok_or("missing compile output argument")?;
            if value.is_empty() || value.starts_with('-') || value.contains('$') {
                return Err("invalid compile output argument".into());
            }
            outputs += 1;
        } else if word == "-c" {
            compile += 1;
        } else if !word.starts_with('-')
            && Path::new(&resolve_path(&entry.directory, word)) == entry.file
        {
            inputs += 1;
            spelled_input = Some(word.clone());
        } else {
            return Err(format!(
                "compiler option `{word}` cannot be preserved by the bounded gateway"
            ));
        }
        cursor += 1;
    }
    if inputs != 1 || compile != 1 || outputs > 1 {
        return Err(
            "compile command requires one -c, one source input and at most one output".into(),
        );
    }
    options.input = CSourceInput::DeclaredFile {
        spelled: spelled_input.ok_or("compile command has no recorded source input")?,
        directory: entry.directory.clone(),
    };
    if frontend_flags {
        let driver_name = Path::new(&words[0])
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if driver_name.contains("++") || driver_name.starts_with("clang-cl") {
            return Err(
                "frontend build flags require the Clang C driver, not a C++ or CL driver".into(),
            );
        }
        if !explicit_dialect {
            return Err("frontend build flags require an explicit supported -std= dialect".into());
        }
        let command = qcore_c_gateway::clang_command()?;
        let actual = command
            .get_program()
            .to_str()
            .ok_or("non-UTF-8 Clang executable")?;
        let actual = executable_path(actual, &std::env::current_dir().map_err(|e| e.to_string())?)?;
        if executable_path(&words[0], &entry.directory)? != actual {
            return Err("frontend build flags require the configured Clang executable; compiler equivalence is unproven".into());
        }
    }
    Ok(options)
}
