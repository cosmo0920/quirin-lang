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

//! Predefined-macro arms for the configured census reading (card
//! **W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS**, gateway rule 10p's amendment).
//!
//! The configured reading ([`super::BuildConfiguration`]) owns the
//! compile commands' `-D` sets and `FLB_`-prefixed macros; a group guarded by a
//! macro the compiler itself predefines (`_WIN32`, `__linux__`, `__APPLE__`)
//! stays undecided and both arms are read. This module decides such a macro
//! **from the recorded compiler, never from an assumption**:
//!
//! * [`derive_facts`] runs each recorded compile command's own compiler
//!   (its first word) with that command's flags. It records the compiler's
//!   identity (path, resolved path, SHA-256, `--version`), the macros
//!   `-dM -E -x LANG` over an empty unit defines (the compiler's predefined set for
//!   the command's target and flags, `-D`/`-U` applied), and, for each
//!   *candidate* name (every reserved identifier a read file's conditional
//!   directive names), whether a probe `#ifdef NAME` is taken at the start
//!   of a translation unit under those flags. It also preprocesses the
//!   command's own file with `-E -dD`: every `#define`/`#undef` outside the
//!   `<built-in>` and `<command line>` segments (in any header, system
//!   headers included) and every file the unit reads.
//! * [`PredefinedMacros::derive`] checks the facts against the database the
//!   census reads (its SHA-256, and each command's file and compiler, in
//!   order) and against the recorded build's compiler (`--version` equal to
//!   the build record's, every derivation run succeeded, every `-dM` name
//!   probed as defined). Any mismatch decides nothing.
//! * [`PredefinedMacros::decide`] decides a reserved candidate name for one
//!   read file: defined when every command of that file defines it at the
//!   start of the unit and no preprocessed file `#undef`s it, undefined when
//!   none does and no preprocessed file `#define`s it. A file without a
//!   command is decided over every command (the configured reading's
//!   intersection rule, with the union for "undefined"), and only when some
//!   preprocessed unit read it. Anything else is undecided.
//!
//! Only names reserved to the implementation (C11 7.1.3: `__X`, or `_` and
//! an uppercase letter) are decided here: other macros keep the configured
//! reading's rules. An arithmetic comparison of a predefined value is never
//! evaluated (the directive reader leaves it undecided), and a read file's
//! own `#define`/`#undef` still wins (the census's `written` and
//! `undefined_somewhere` sets).
//!
//! Premises stated, not proved (trust boundary 28): the recorded commands
//! are the program's (as rule 10p's configured reading states); the compiler
//! at the recorded path, with the recorded version, behaves on the probe and
//! the preprocessed units as it did in the build; Clang's `-dD` output prints
//! every `#define`/`#undef` it processes, with line markers naming the file
//! it reads.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

/// The facts format this module writes and reads.
pub const SCHEMA: u32 = 1;

/// A compiler as the derivation found and ran it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompilerIdentity {
    /// The first word of the recorded commands.
    pub path: String,
    /// The path with every symbolic link resolved, empty when unavailable.
    pub resolved: String,
    /// SHA-256 of the resolved binary, empty when unavailable.
    pub sha256: String,
    /// Its `--version` output, empty when it could not be run.
    pub version: String,
    /// Why it could not be identified, when it could not.
    pub failure: Option<String>,
}

/// What one compile command's compiler says.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandFacts {
    /// The command's file: its path under the source root, else absolute.
    pub key: String,
    /// The command's first word.
    pub compiler: String,
    /// The language the unit is compiled as (`c`, `c++`,
    /// `assembler-with-cpp`, or `assembler`: never preprocessed).
    pub language: String,
    /// `-dM -E -x LANG` over an empty unit under the command's flags.
    pub predefined: BTreeSet<String>,
    /// The candidates a probe `#ifdef` finds defined at a unit's start.
    pub probe_defined: BTreeSet<String>,
    /// The candidates a `#define` in some file of the preprocessed unit
    /// (`-E -dD`, not the built-in or command-line segment) names.
    pub file_defined: BTreeSet<String>,
    /// The candidates an `#undef` there names.
    pub file_undefined: BTreeSet<String>,
    /// Why a derivation run failed, when one did: the command's file is then
    /// undecided, and so is every file without a command.
    pub failure: Option<String>,
}

/// The facts one derivation wrote.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredefinedFacts {
    pub schema: u32,
    /// SHA-256 of the compile database the commands were read from.
    pub database_sha256: String,
    /// The recorded build's compiler `--version` text.
    pub recorded_compiler_version: String,
    pub compilers: BTreeMap<String, CompilerIdentity>,
    /// One per database entry, in database order.
    pub commands: Vec<CommandFacts>,
    /// Units the build does not compile (a test), read with another
    /// command's flags.
    pub extras: Vec<CommandFacts>,
    /// The reserved names read files' conditional directives name.
    pub candidates: BTreeSet<String>,
    /// Files under the source root some preprocessed unit read.
    pub covered: BTreeSet<String>,
}

/// A name reserved to the implementation (C11 7.1.3).
#[must_use]
pub fn is_reserved(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() >= 2
        && bytes[0] == b'_'
        && (bytes[1] == b'_' || bytes[1].is_ascii_uppercase())
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

/// The reserved identifiers a file's conditional directives (`#if`,
/// `#elif`, `#ifdef`, `#ifndef`, `#elifdef`, `#elifndef`) name, continuation
/// lines joined and comments dropped.
#[must_use]
pub fn condition_names(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut logical = String::new();
    let mut open = false;
    for line in text.lines() {
        if !open {
            let Some(rest) = line.trim_start().strip_prefix('#') else {
                continue;
            };
            let word = rest.trim_start();
            let end = word
                .find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(word.len());
            if !matches!(
                &word[..end],
                "if" | "elif" | "ifdef" | "ifndef" | "elifdef" | "elifndef"
            ) {
                continue;
            }
            open = true;
        }
        if let Some(stripped) = line.strip_suffix('\\') {
            logical.push_str(stripped);
            logical.push(' ');
            continue;
        }
        logical.push_str(line);
        open = false;
        while let Some(start) = logical.find("/*") {
            let end = logical[start + 2..]
                .find("*/")
                .map_or(logical.len(), |e| start + 2 + e + 2);
            logical.replace_range(start..end, " ");
        }
        if let Some(start) = logical.find("//") {
            logical.truncate(start);
        }
        names.extend(
            logical
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .filter(|word| is_reserved(word))
                .map(str::to_owned),
        );
        logical.clear();
    }
    names
}

/// The names a `-dM` listing defines.
#[must_use]
pub fn listed_macros(output: &str) -> BTreeSet<String> {
    output
        .lines()
        .filter_map(|line| directive_name(line, "define"))
        .collect()
}

/// `#WORD NAME…` → `NAME`.
fn directive_name(line: &str, word: &str) -> Option<String> {
    let rest = line.strip_prefix('#')?.trim_start().strip_prefix(word)?;
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let rest = rest.trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    (end > 0).then(|| rest[..end].to_owned())
}

/// A preprocessed unit's (`-E -dD`) directives, by segment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Preprocessed {
    /// Names a `#define` in a file (not the built-in or command-line
    /// segment) names.
    pub file_defined: BTreeSet<String>,
    pub file_undefined: BTreeSet<String>,
    /// Every file a line marker names (not `<built-in>`/`<command line>`).
    pub files: BTreeSet<String>,
}

/// A line marker `# N "FILE" FLAGS…` → `FILE`.
fn line_marker(line: &str) -> Option<String> {
    let rest = line.strip_prefix("# ")?;
    let digits = rest.find(|c: char| !c.is_ascii_digit())?;
    if digits == 0 {
        return None;
    }
    let rest = rest[digits..].strip_prefix(" \"")?;
    let mut name = String::new();
    let mut chars = rest.chars();
    loop {
        match chars.next()? {
            '"' => break,
            '\\' => name.push(chars.next()?),
            c => name.push(c),
        }
    }
    let flags = chars.as_str();
    flags
        .split_whitespace()
        .all(|flag| flag.chars().all(|c| c.is_ascii_digit()))
        .then_some(name)
}

/// Read a unit's `-E -dD` output.
#[must_use]
pub fn read_preprocessed(output: &str) -> Preprocessed {
    let mut read = Preprocessed::default();
    let mut in_file = false;
    for line in output.lines() {
        if let Some(name) = line_marker(line) {
            in_file = name != "<built-in>" && name != "<command line>";
            if in_file {
                read.files.insert(name);
            }
            continue;
        }
        if !in_file {
            continue;
        }
        if let Some(name) = directive_name(line, "define") {
            read.file_defined.insert(name);
        } else if let Some(name) = directive_name(line, "undef") {
            read.file_undefined.insert(name);
        }
    }
    read
}

/// A command's words as a POSIX shell splits them: single quotes, double
/// quotes (with `\` escaping `"`, `\`, `$` and `` ` ``) and a bare `\`.
#[must_use]
pub fn shell_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut chars = command.chars();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                started = true;
                for q in chars.by_ref() {
                    if q == '\'' {
                        break;
                    }
                    word.push(q);
                }
            }
            '"' => {
                started = true;
                while let Some(q) = chars.next() {
                    match q {
                        '"' => break,
                        '\\' => match chars.next() {
                            Some(e @ ('"' | '\\' | '$' | '`')) => word.push(e),
                            Some(e) => {
                                word.push('\\');
                                word.push(e);
                            }
                            None => word.push('\\'),
                        },
                        _ => word.push(q),
                    }
                }
            }
            '\\' => {
                started = true;
                if let Some(e) = chars.next() {
                    word.push(e);
                }
            }
            c if c.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            c => {
                started = true;
                word.push(c);
            }
        }
    }
    if started {
        words.push(word);
    }
    words
}

/// One compile database entry, as the derivation and the census read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseEntry {
    pub directory: PathBuf,
    /// The absolute file.
    pub file: PathBuf,
    /// Every word, the compiler first.
    pub words: Vec<String>,
}

impl DatabaseEntry {
    /// The file's key: its path under `root`, else the absolute path.
    #[must_use]
    pub fn key(&self, root: &Path) -> String {
        key_of(&self.file, root)
    }
}

fn key_of(file: &Path, root: &Path) -> String {
    file.strip_prefix(root).map_or_else(
        |_| file.to_string_lossy().replace('\\', "/"),
        |relative| relative.to_string_lossy().replace('\\', "/"),
    )
}

/// Read a compile database's entries in order.
///
/// # Errors
/// The text is not a JSON array of entries with a `file` and a command.
pub fn database_entries(text: &str) -> Result<Vec<DatabaseEntry>, String> {
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(text).map_err(|e| format!("compile database: {e}"))?;
    entries
        .iter()
        .map(|entry| {
            let directory = PathBuf::from(entry["directory"].as_str().unwrap_or(""));
            let file = PathBuf::from(
                entry["file"]
                    .as_str()
                    .ok_or("compile database entry without a file")?,
            );
            let file = if file.is_absolute() {
                file
            } else {
                directory.join(file)
            };
            let words = match entry.get("arguments").and_then(|a| a.as_array()) {
                Some(arguments) => arguments
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_owned))
                    .collect(),
                None => shell_words(
                    entry["command"]
                        .as_str()
                        .ok_or("compile database entry without a command")?,
                ),
            };
            if words.is_empty() {
                return Err(format!("{}: an empty command", file.display()));
            }
            Ok(DatabaseEntry {
                directory,
                file,
                words,
            })
        })
        .collect()
}

/// The language a unit is compiled as: `assembler` (`.s`) is never
/// preprocessed, so it reads no header and defines no macro.
pub const NOT_PREPROCESSED: &str = "assembler";

/// The command's flags with its output, compilation step, dependency
/// outputs and source file removed, and the language its file is compiled
/// as; `None` for a file of any other kind.
#[must_use]
pub fn derivation_flags(entry: &DatabaseEntry) -> Option<(Vec<String>, &'static str)> {
    let language = match entry.file.extension().and_then(|e| e.to_str()) {
        Some("c") => "c",
        Some("cc" | "cpp" | "cxx" | "C") => "c++",
        Some("S") => "assembler-with-cpp",
        Some("s") => NOT_PREPROCESSED,
        _ => return None,
    };
    let mut flags = Vec::new();
    let mut words = entry.words.iter().skip(1);
    while let Some(word) = words.next() {
        match word.as_str() {
            "-o" | "-MF" | "-MT" | "-MQ" => {
                words.next();
            }
            "-c" | "-MD" | "-MMD" | "-MP" | "-M" | "-MM" => {}
            _ if word.starts_with("-o") && word.len() > 2 => {}
            _ if Path::new(word) == entry.file || entry.directory.join(word) == entry.file => {
                // The source file.
            }
            _ => flags.push(word.clone()),
        }
    }
    Some((flags, language))
}

/// A unit the build does not compile, read with another command's flags.
#[derive(Clone, Debug)]
pub struct ExtraUnit {
    pub file: PathBuf,
    /// Its key in the census (`tests/runtime/in_tail.c`).
    pub key: String,
    /// The database entry (by key) whose command it borrows.
    pub like: String,
    /// Include directories put before the borrowed flags.
    pub includes: Vec<String>,
}

/// What [`derive_facts`] reads.
#[derive(Clone, Debug)]
pub struct FactsRequest {
    pub database_text: String,
    pub root: PathBuf,
    pub recorded_compiler_version: String,
    pub candidates: BTreeSet<String>,
    pub extras: Vec<ExtraUnit>,
    /// A directory the probe file is written to.
    pub scratch: PathBuf,
    pub jobs: usize,
}

fn run(command: &mut Command) -> Result<String, String> {
    let output = command
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not run: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "exit {:?}: {}",
            output.status.code(),
            stderr.lines().next().unwrap_or("")
        ));
    }
    String::from_utf8(output.stdout).map_err(|_| "output is not UTF-8".to_owned())
}

/// Identify a compiler: resolve its path, hash it, run `--version`.
#[must_use]
pub fn identify(path: &str) -> CompilerIdentity {
    let mut identity = CompilerIdentity {
        path: path.to_owned(),
        ..CompilerIdentity::default()
    };
    let resolved = match std::fs::canonicalize(path) {
        Ok(resolved) => resolved,
        Err(error) => {
            identity.failure = Some(format!("unavailable: {error}"));
            return identity;
        }
    };
    identity.resolved = resolved.to_string_lossy().into_owned();
    match std::fs::read(&resolved) {
        Ok(bytes) => identity.sha256 = qcore_schemas::digest::sha256_hex(&bytes),
        Err(error) => {
            identity.failure = Some(format!("unreadable: {error}"));
            return identity;
        }
    }
    match run(Command::new(path).arg("--version")) {
        Ok(version) => identity.version = version,
        Err(error) => identity.failure = Some(format!("`--version`: {error}")),
    }
    identity
}

/// Two `--version` texts name the same compiler: equal but for trailing
/// white space (a build record stores the text without its last newline).
#[must_use]
pub fn same_version(run: &str, recorded: &str) -> bool {
    !recorded.trim().is_empty() && run.trim_end() == recorded.trim_end()
}

/// The probe unit: one `#ifdef` per candidate (in name order), printing its
/// index when taken. The name itself is never printed: a builtin
/// function-like macro (`__has_attribute`) would be expanded.
#[must_use]
pub fn probe_text(candidates: &BTreeSet<String>) -> String {
    let mut text = String::new();
    for (index, name) in candidates.iter().enumerate() {
        text.push_str(&format!(
            "#ifdef {name}\nqcore_probe_defined_{index}\n#endif\n"
        ));
    }
    text
}

/// The candidates a probe's preprocessed output shows taken.
#[must_use]
pub fn probe_defined(output: &str, candidates: &BTreeSet<String>) -> BTreeSet<String> {
    let names = candidates.iter().collect::<Vec<_>>();
    output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("qcore_probe_defined_"))
        .filter_map(|index| index.parse::<usize>().ok())
        .filter_map(|index| names.get(index).map(|name| (*name).clone()))
        .collect()
}

/// The derivation's own inputs: the probe unit, the empty unit and the
/// candidates the probe tests.
struct Probe<'a> {
    probe: &'a Path,
    empty: &'a Path,
    candidates: &'a BTreeSet<String>,
}

/// Run the three derivations of one command (its flags, its language and
/// the file it preprocesses).
fn derive_command(
    compiler: &str,
    directory: &Path,
    flags: &[String],
    language: &str,
    file: &Path,
    inputs: &Probe<'_>,
) -> Result<(BTreeSet<String>, BTreeSet<String>, Preprocessed), String> {
    let Probe {
        probe,
        empty,
        candidates,
    } = *inputs;
    let listing = run(Command::new(compiler)
        .current_dir(directory)
        .args(flags)
        .args(["-dM", "-E", "-x", language])
        .arg(empty))
    .map_err(|e| format!("`-dM`: {e}"))?;
    let predefined = listed_macros(&listing);
    let probed = run(Command::new(compiler)
        .current_dir(directory)
        .args(flags)
        .args(["-E", "-P", "-x", language])
        .arg(probe))
    .map_err(|e| format!("probe: {e}"))?;
    let defined = probe_defined(&probed, candidates);
    // Every listed candidate must probe as defined: the two runs read the
    // same start of a unit.
    if let Some(name) = predefined
        .iter()
        .find(|name| candidates.contains(*name) && !defined.contains(*name))
    {
        return Err(format!(
            "`{name}` is listed by `-dM` but its probe is not taken"
        ));
    }
    let preprocessed = run(Command::new(compiler)
        .current_dir(directory)
        .args(flags)
        .args(["-E", "-dD", "-x", language])
        .arg(file))
    .map_err(|e| format!("`-E -dD`: {e}"))?;
    Ok((predefined, defined, read_preprocessed(&preprocessed)))
}

/// Derive the predefined-macro facts of one build's compile database.
///
/// # Errors
/// The database cannot be read, or the probe file cannot be written. A
/// command whose compiler or derivation fails is recorded with its failure
/// (the facts then decide nothing), never an error.
pub fn derive_facts(request: &FactsRequest) -> Result<PredefinedFacts, String> {
    let entries = database_entries(&request.database_text)?;
    let probe = request.scratch.join("qcore-predefined-probe.c");
    std::fs::write(&probe, probe_text(&request.candidates))
        .map_err(|e| format!("{}: {e}", probe.display()))?;
    // `-dM` over an empty unit (in place of `/dev/null`, which not every
    // host has).
    let empty = request.scratch.join("qcore-predefined-empty.c");
    std::fs::write(&empty, "").map_err(|e| format!("{}: {e}", empty.display()))?;
    let mut compilers = BTreeMap::new();
    for entry in &entries {
        compilers
            .entry(entry.words[0].clone())
            .or_insert_with(|| identify(&entry.words[0]));
    }
    // Every unit to derive: (key, compiler, directory, flags, language,
    // file), the extras after the database's entries.
    let mut units = Vec::new();
    for entry in &entries {
        let flags = derivation_flags(entry);
        units.push((
            entry.key(&request.root),
            entry.words[0].clone(),
            entry.directory.clone(),
            flags,
            entry.file.clone(),
        ));
    }
    for extra in &request.extras {
        let template = entries
            .iter()
            .find(|entry| entry.key(&request.root) == extra.like);
        let flags = template
            .and_then(derivation_flags)
            .map(|(flags, language)| {
                let mut all = extra
                    .includes
                    .iter()
                    .map(|include| format!("-I{include}"))
                    .collect::<Vec<_>>();
                all.extend(flags);
                (all, language)
            });
        units.push((
            extra.key.clone(),
            template.map_or_else(String::new, |t| t.words[0].clone()),
            template.map_or_else(PathBuf::new, |t| t.directory.clone()),
            flags,
            extra.file.clone(),
        ));
    }
    let next = AtomicUsize::new(0);
    let results = Mutex::new(vec![None; units.len()]);
    std::thread::scope(|scope| {
        for _ in 0..request.jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some((key, compiler, directory, flags, file)) = units.get(index) else {
                        break;
                    };
                    let identity = compilers.get(compiler);
                    let language = flags.as_ref().map_or("", |(_, language)| *language);
                    let outcome = match (flags, identity) {
                        (None, _) => Err("not a C, C++ or assembler unit".to_owned()),
                        // Never preprocessed: no macro state, no header read.
                        (Some((_, NOT_PREPROCESSED)), _) => Ok((
                            BTreeSet::new(),
                            BTreeSet::new(),
                            Preprocessed::default(),
                        )),
                        (_, None) => Err("no database command to borrow".to_owned()),
                        (_, Some(identity)) if identity.failure.is_some() => Err(format!(
                            "compiler `{compiler}`: {}",
                            identity.failure.as_deref().unwrap_or("")
                        )),
                        (_, Some(identity))
                            if !same_version(&identity.version, &request.recorded_compiler_version) =>
                        {
                            Err(format!(
                                "compiler `{compiler}` is not the recorded build's (`--version` differs)"
                            ))
                        }
                        (Some((flags, language)), Some(_)) => derive_command(
                            compiler,
                            directory,
                            flags,
                            language,
                            file,
                            &Probe {
                                probe: &probe,
                                empty: &empty,
                                candidates: &request.candidates,
                            },
                        ),
                    };
                    let facts = match outcome {
                        Ok((predefined, probe_defined, preprocessed)) => (
                            CommandFacts {
                                key: key.clone(),
                                compiler: compiler.clone(),
                                language: language.to_owned(),
                                predefined,
                                probe_defined,
                                file_defined: preprocessed
                                    .file_defined
                                    .intersection(&request.candidates)
                                    .cloned()
                                    .collect(),
                                file_undefined: preprocessed
                                    .file_undefined
                                    .intersection(&request.candidates)
                                    .cloned()
                                    .collect(),
                                failure: None,
                            },
                            Some(preprocessed),
                        ),
                        Err(failure) => (
                            CommandFacts {
                                key: key.clone(),
                                compiler: compiler.clone(),
                                language: language.to_owned(),
                                failure: Some(failure),
                                ..CommandFacts::default()
                            },
                            None,
                        ),
                    };
                    results
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)[index] = Some(facts);
                }
            });
        }
    });
    let results = results
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut facts = PredefinedFacts {
        schema: SCHEMA,
        database_sha256: qcore_schemas::digest::sha256_hex(request.database_text.as_bytes()),
        recorded_compiler_version: request.recorded_compiler_version.clone(),
        compilers,
        candidates: request.candidates.clone(),
        ..PredefinedFacts::default()
    };
    for (index, result) in results.into_iter().enumerate() {
        let (command, preprocessed) = result.unwrap_or_default();
        if let Some(preprocessed) = preprocessed {
            facts.covered.extend(
                preprocessed
                    .files
                    .iter()
                    .filter_map(|file| {
                        Path::new(file)
                            .strip_prefix(&request.root)
                            .ok()
                            .map(|r| r.to_string_lossy().replace('\\', "/"))
                    })
                    .chain(
                        // An extra is read from a copy; it covers its key.
                        (index >= entries.len()).then(|| command.key.clone()),
                    ),
            );
        }
        if index < entries.len() {
            facts.commands.push(command);
        } else {
            facts.extras.push(command);
        }
    }
    Ok(facts)
}

/// What a set of preprocessed units says about the candidates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct UnitsView {
    /// Defined at the start of every unit.
    all: BTreeSet<String>,
    /// Defined at the start of some unit.
    any: BTreeSet<String>,
    /// `#define`d / `#undef`d by some file of some unit.
    file_defined: BTreeSet<String>,
    file_undefined: BTreeSet<String>,
    units: usize,
}

impl UnitsView {
    fn add(&mut self, command: &CommandFacts) {
        if self.units == 0 {
            self.all.clone_from(&command.probe_defined);
        } else {
            self.all = self
                .all
                .intersection(&command.probe_defined)
                .cloned()
                .collect();
        }
        self.any.extend(command.probe_defined.iter().cloned());
        self.file_defined
            .extend(command.file_defined.iter().cloned());
        self.file_undefined
            .extend(command.file_undefined.iter().cloned());
        self.units += 1;
    }

    /// Defined when every unit defines it at its start and no file of any
    /// unit `#undef`s it; undefined when no unit does and no file `#define`s
    /// it.
    fn decide(&self, name: &str) -> Option<bool> {
        if self.units == 0 {
            None
        } else if self.all.contains(name) && !self.file_undefined.contains(name) {
            Some(true)
        } else if !self.any.contains(name) && !self.file_defined.contains(name) {
            Some(false)
        } else {
            None
        }
    }
}

/// The census's view of the derived facts: what decides a reserved name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PredefinedMacros {
    /// Each read file with a command: its commands' view, `None` when one of
    /// them failed.
    units: BTreeMap<String, Option<UnitsView>>,
    /// Every preprocessed unit's (the database's and the extras'), `None`
    /// when any failed: a failed unit may read any header.
    every: Option<UnitsView>,
    candidates: BTreeSet<String>,
    covered: BTreeSet<String>,
    /// The compilers the facts ran, for the census's report.
    pub compilers: Vec<CompilerIdentity>,
    /// Each failed derivation, by the command's file (the census's report).
    pub failures: BTreeMap<String, String>,
}

impl PredefinedMacros {
    /// Check `facts` against the database the census reads and the recorded
    /// build, and build the view.
    ///
    /// # Errors
    /// Why nothing is decided: a schema or database mismatch, a command that
    /// is not the database's, or a compiler that is unavailable or is not the
    /// recorded build's.
    pub fn derive(
        facts: &PredefinedFacts,
        database_text: &str,
        root: &Path,
    ) -> Result<Self, String> {
        if facts.schema != SCHEMA {
            return Err(format!("facts schema {} is not {SCHEMA}", facts.schema));
        }
        let digest = qcore_schemas::digest::sha256_hex(database_text.as_bytes());
        if facts.database_sha256 != digest {
            return Err("the facts were derived from another compile database".to_owned());
        }
        if facts.recorded_compiler_version.trim().is_empty() {
            return Err("no recorded compiler version".to_owned());
        }
        for identity in facts.compilers.values() {
            if let Some(failure) = &identity.failure {
                return Err(format!("compiler `{}`: {failure}", identity.path));
            }
            if !same_version(&identity.version, &facts.recorded_compiler_version) {
                return Err(format!(
                    "compiler `{}` is not the recorded build's (`--version` differs)",
                    identity.path
                ));
            }
        }
        let entries = database_entries(database_text)?;
        if entries.len() != facts.commands.len() {
            return Err(format!(
                "{} database commands, {} derived",
                entries.len(),
                facts.commands.len()
            ));
        }
        for (entry, command) in entries.iter().zip(&facts.commands) {
            if entry.key(root) != command.key || entry.words[0] != command.compiler {
                return Err(format!(
                    "the derived command for `{}` is not the database's",
                    command.key
                ));
            }
        }
        let mut view = Self {
            candidates: facts.candidates.clone(),
            covered: facts.covered.clone(),
            compilers: facts.compilers.values().cloned().collect(),
            ..Self::default()
        };
        let mut every = Some(UnitsView::default());
        let commands = facts
            .commands
            .iter()
            .map(|c| (c, false))
            .chain(facts.extras.iter().map(|c| (c, true)));
        for (command, extra) in commands {
            if !facts.compilers.contains_key(&command.compiler) {
                return Err(format!(
                    "compiler `{}` was not identified",
                    command.compiler
                ));
            }
            if let Some(failure) = &command.failure {
                view.failures.insert(command.key.clone(), failure.clone());
                every = None;
                if !extra {
                    view.units.insert(command.key.clone(), None);
                }
                continue;
            }
            if command.language == NOT_PREPROCESSED {
                // Assembled without the preprocessor: it reads no header.
                continue;
            }
            if let Some(every) = every.as_mut() {
                every.add(command);
            }
            if !extra {
                let unit = view
                    .units
                    .entry(command.key.clone())
                    .or_insert_with(|| Some(UnitsView::default()));
                if let Some(unit) = unit.as_mut() {
                    unit.add(command);
                }
            }
        }
        view.every = every.filter(|every| every.units > 0);
        Ok(view)
    }

    /// Whether `name` is defined (`Some(true)`), undefined (`Some(false)`)
    /// or undecided in the read file `path`, by the recorded compiler alone.
    /// The census's own `written`/`undefined_somewhere` sets still apply.
    #[must_use]
    pub fn decide(&self, path: &str, name: &str) -> Option<bool> {
        if !is_reserved(name) || !self.candidates.contains(name) {
            return None;
        }
        match self.units.get(path) {
            Some(unit) => unit.as_ref()?.decide(name),
            None if self.covered.contains(path) => self.every.as_ref()?.decide(name),
            None => None,
        }
    }

    /// The read files with a derived command, and those with a failed one.
    #[must_use]
    pub fn unit_counts(&self) -> (usize, usize) {
        let failed = self.units.values().filter(|u| u.is_none()).count();
        (self.units.len() - failed, failed)
    }

    /// Whether a file without a command can be decided (no derivation
    /// failed).
    #[must_use]
    pub fn decides_uncommanded_files(&self) -> bool {
        self.every.is_some()
    }

    /// The candidate names.
    #[must_use]
    pub fn candidates(&self) -> &BTreeSet<String> {
        &self.candidates
    }
}

/// One command's facts for a fixture: its file, what is defined at its
/// start, and what its preprocessed files `#define` and `#undef`.
#[must_use]
pub fn fixture_command(
    key: &str,
    defined: &[&str],
    file_defined: &[&str],
    file_undefined: &[&str],
) -> CommandFacts {
    let set = |names: &[&str]| names.iter().map(|n| (*n).to_owned()).collect();
    CommandFacts {
        key: key.to_owned(),
        compiler: "cc".to_owned(),
        language: "c".to_owned(),
        predefined: set(defined),
        probe_defined: set(defined),
        file_defined: set(file_defined),
        file_undefined: set(file_undefined),
        failure: None,
    }
}

/// Facts for a fixture: `commands` in database order (see
/// [`fixture_database`]), the reserved candidates, and the files some unit
/// read. The recorded compiler is `cc` at version `cc version 1`.
#[must_use]
pub fn fixture_facts(
    database_text: &str,
    commands: Vec<CommandFacts>,
    candidates: &[&str],
    covered: &[&str],
) -> PredefinedFacts {
    PredefinedFacts {
        schema: SCHEMA,
        database_sha256: qcore_schemas::digest::sha256_hex(database_text.as_bytes()),
        recorded_compiler_version: "cc version 1".to_owned(),
        compilers: BTreeMap::from([(
            "cc".to_owned(),
            CompilerIdentity {
                path: "cc".to_owned(),
                version: "cc version 1\n".to_owned(),
                ..CompilerIdentity::default()
            },
        )]),
        commands,
        extras: Vec::new(),
        candidates: candidates.iter().map(|n| (*n).to_owned()).collect(),
        covered: covered.iter().map(|n| (*n).to_owned()).collect(),
    }
}

/// A compile database compiling each of `files` (paths under `root`) once
/// with `cc`.
#[must_use]
pub fn fixture_database(root: &str, files: &[&str]) -> String {
    let entries = files
        .iter()
        .map(|file| {
            serde_json::json!({
                "directory": root,
                "file": format!("{root}/{file}"),
                "command": format!("cc -c {root}/{file} -o {file}.o"),
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&entries).unwrap_or_default()
}
