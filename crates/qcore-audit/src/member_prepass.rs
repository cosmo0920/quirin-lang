//! Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (gateway rule 12n, amended in
//! place): the member-facts pre-pass of the pinned route.
//!
//! USER DECISION 2026-10-14 ("Build the agent's recommended set"), **R1** and
//! **F1**: before any unit is analysed, member facts are read from every
//! unit of Fluent Bit's production compile database (and the Monkey units the
//! run analyses), so a unit decides a member invocation over the run's set;
//! the fold stays demote-only.
//!
//! Every unit is preprocessed. A unit whose preprocessed text names no token
//! of any invoked member's closure graph (the member's record tags and path
//! segments, the setters whose formals the graph reaches, every function a
//! formal of the graph belongs to) is skipped: C reaches a member only through
//! its record's declaration and a function only through its name, so such a
//! unit holds no store, registration or address that the closure reads
//! (`QCore.AppRkCallbacks.skipping_a_silent_unit_keeps_the_run_set`). Every
//! other C unit is read whole for its facts
//! ([`qcore_c_gateway::callback_member_closure::read_unit`]); reading may add
//! tokens (a formal's function), so the selection is iterated to a fixed
//! point. A unit that names a token but cannot be read as C (a C++ or
//! assembly unit, a failed preprocessing or Clang run) is unread and opens
//! every member, except where the profile's `member-facts-scope.json` names
//! it outside the program with recorded evidence.
//!
//! Token matching uses 32-bit hashes of the unit's identifiers: a collision
//! only reads one more unit, never skips one.
//!
//! Card **CALLBACK-MEMBER-NON-C-UNITS** (rule 12n, amended in place): an
//! unread C++ unit whose `-E` text is the whole unit (no precompiled header
//! or module in its command or text) opens only the members whose own
//! closure-graph tokens its text names: the skipping reading above, applied
//! to each member's graph. An assembly unit, a failed preprocessing and a C
//! unit whose AST is not read still open every member.
//!
//! Card **MEMBER-PREPASS-OTHER-ROUTES** (rule 12n, amended in place): the
//! current and no-build routes run the same pre-pass over their own compile
//! databases, and the scope file's exclusions are re-checked at run time. The
//! scope file names the revision its evidence was taken at, and every entry a
//! typed evidence ([`Evidence`]) the pre-pass re-verifies against the run's
//! own sources inside the token fixed point, against the final tokens. An
//! exclusion whose evidence does not hold, or cannot be checked, is dropped:
//! the unit is preprocessed and read like any other, and a unit that then
//! cannot be read opens every member
//! (`QCore.MemberPrepassRoutes.a_stale_exclusion_never_closes_a_member`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use qcore_c_gateway::callback_member_closure::{MemberFacts, closure_tokens, read_unit};
use qcore_c_gateway::legacy_c::{C11TranslationUnit, LegacyFunctionScope};
use qcore_c_gateway::member_run_facts::{RunMemberFacts, declarations_unit};
use qcore_schemas::contract::ResourceContracts;
use serde_json::{Value, json};

use crate::compile_db::{CompileEntry, compiler_options, shell_words};

/// The profile file naming units outside the program.
pub const SCOPE_FILE: &str = "member-facts-scope.json";

/// The scope file's schema: 2 names the evidence revision and each entry's
/// typed evidence (card MEMBER-PREPASS-OTHER-ROUTES).
pub const SCOPE_SCHEMA: u64 = 2;

/// How an exclusion is re-verified at run time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evidence {
    /// The unit's raw text (when it exists), every file Clang resolves for it
    /// under its own command (`-M -MG`), and the named generator `sources`
    /// (files, read with the files Clang resolves for their preprocessor
    /// directives under the unit's command, or directories, every file under
    /// them) name no token of the pre-pass; every include Clang leaves
    /// unresolved is one of `generated`, generated at build time from the
    /// sources. A unit absent from the inputs needs at least one source.
    TokensAbsent {
        sources: Vec<String>,
        generated: BTreeSet<String>,
    },
    /// The program's link lines (`links`, relative to the CMake build root
    /// holding `CMakeCache.txt` above the unit's compile directory) name
    /// `linked` and never `unlinked`: the unit's target is not linked into the
    /// audited program.
    NotLinked {
        links: Vec<String>,
        linked: String,
        unlinked: String,
    },
}

/// One unit the profile names outside the program: a path fragment, the
/// recorded evidence and how it is re-verified.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outside {
    pub fragment: String,
    pub why: String,
    pub evidence: Evidence,
}

/// The profile's scope file: the revision its evidence was taken at, and its
/// entries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    pub evidence_revision: Option<String>,
    pub outside: Vec<Outside>,
}

fn text_list(entry: &Value, key: &str, path: &Path) -> Result<Vec<String>, String> {
    match &entry[key] {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .filter(|text| !text.is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{}: `{key}` holds a non-text item", path.display()))
            })
            .collect(),
        _ => Err(format!("{}: `{key}` is not a list", path.display())),
    }
}

/// Read the profile's scope file (empty when absent). A file of another
/// schema, without its evidence revision, or with an entry whose evidence is
/// missing or unknown is refused: an exclusion that cannot be re-verified is
/// never taken.
pub fn load_scope(directory: &Path) -> Result<Scope, String> {
    let path = directory.join(SCOPE_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(Scope::default());
    };
    let value: Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    if value["schema_version"].as_u64() != Some(SCOPE_SCHEMA) {
        return Err(format!(
            "{}: schema_version must be {SCOPE_SCHEMA} (the evidence revision and each \
             entry's re-checked evidence)",
            path.display()
        ));
    }
    let evidence_revision = value["evidence_revision"]
        .as_str()
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            format!(
                "{}: no `evidence_revision` (the revision its evidence was taken at)",
                path.display()
            )
        })?;
    let outside = value["outside_program"]
        .as_array()
        .ok_or_else(|| format!("{}: no `outside_program` list", path.display()))?
        .iter()
        .map(|entry| {
            let text = |value: &Value, key: &str| {
                value[key]
                    .as_str()
                    .filter(|text| !text.is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{}: an entry has no `{key}`", path.display()))
            };
            let fragment = text(entry, "fragment")?;
            let evidence = &entry["evidence"];
            let evidence = match evidence["kind"].as_str() {
                Some("tokens-absent") => Evidence::TokensAbsent {
                    sources: text_list(evidence, "sources", &path)?,
                    generated: text_list(evidence, "generated", &path)?
                        .into_iter()
                        .collect(),
                },
                Some("not-linked") => {
                    let links = text_list(evidence, "links", &path)?;
                    if links.is_empty() {
                        return Err(format!(
                            "{}: `{fragment}`: a not-linked evidence names no link line",
                            path.display()
                        ));
                    }
                    Evidence::NotLinked {
                        links,
                        linked: text(evidence, "linked")?,
                        unlinked: text(evidence, "unlinked")?,
                    }
                }
                other => {
                    return Err(format!(
                        "{}: `{fragment}`: evidence kind {other:?} is not `tokens-absent` or \
                         `not-linked`",
                        path.display()
                    ));
                }
            };
            Ok(Outside {
                why: text(entry, "why")?,
                fragment,
                evidence,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Scope {
        evidence_revision: Some(evidence_revision),
        outside,
    })
}

fn hash(word: &str) -> u32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    word.hash(&mut hasher);
    let value = hasher.finish();
    (value ^ (value >> 32)) as u32
}

/// The identifiers of `text`, hashed, sorted and deduplicated. Words inside
/// literals are kept: an extra word only reads one more unit.
fn identifiers(text: &str) -> Vec<u32> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte == b'_' || byte.is_ascii_alphabetic() {
            let start = at;
            while at < bytes.len() && (bytes[at] == b'_' || bytes[at].is_ascii_alphanumeric()) {
                at += 1;
            }
            out.push(hash(&text[start..at]));
        } else if byte.is_ascii_digit() {
            while at < bytes.len() && (bytes[at] == b'_' || bytes[at].is_ascii_alphanumeric()) {
                at += 1;
            }
        } else {
            at += 1;
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// How one unit was preprocessed.
enum Reading {
    /// A C unit: its identifiers.
    C(Vec<u32>),
    /// A unit that is not C (C++, assembly): its identifiers, from its own
    /// command with `-E`, why it is not read, and (card
    /// CALLBACK-MEMBER-NON-C-UNITS) whether its reading is scoped to the
    /// members whose tokens it names: a C++ unit whose text is the whole
    /// unit ([`scoped_cxx`]).
    Other(Vec<u32>, String, bool),
    /// Preprocessing failed: why.
    Failed(String),
    /// The profile names it outside the program: the scope entry's index.
    Outside(usize),
}

pub(crate) fn c_unit<'a>(
    entry: &CompileEntry,
    source: &'a str,
) -> Result<C11TranslationUnit<'a>, String> {
    let options = compiler_options(entry)?;
    Ok(C11TranslationUnit {
        source,
        include_paths: options.include_paths,
        definitions: options.definitions,
        preprocessor_options: options.preprocessor_options,
        dialect: options.dialect,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        input: options.input,
    })
}

fn is_c(entry: &CompileEntry) -> bool {
    entry.file.extension().and_then(|ext| ext.to_str()) == Some("c")
}

/// A unit's own recorded command as a program and its arguments: `-c` and
/// `-o FILE` dropped and, with `drop_input`, the unit's own input as well
/// (card MEMBER-PREPASS-OTHER-ROUTES: and every dependency-file flag, so a
/// dependency listing prints to standard output and writes nothing beside
/// the build).
fn own_arguments(entry: &CompileEntry, drop_input: bool) -> Result<(String, Vec<String>), String> {
    let words = shell_words(&entry.command)?;
    let (program, rest) = words.split_first().ok_or("empty command")?;
    let input = |word: &str| {
        let path = Path::new(word);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            entry.directory.join(path)
        };
        crate::compile_db::same_existing_path(&path, &entry.file)
    };
    let mut args = Vec::new();
    let mut skip = false;
    for word in rest {
        if skip {
            skip = false;
            continue;
        }
        match word.as_str() {
            "-c" | "-MD" | "-MMD" | "-MP" | "-M" | "-MM" | "-MG" => {}
            "-o" | "-MF" | "-MT" | "-MQ" => skip = true,
            _ if word.starts_with("-o") && word.len() > 2 => {}
            _ if ["-MF", "-MT", "-MQ"]
                .iter()
                .any(|flag| word.starts_with(flag) && word.len() > 3) => {}
            _ if drop_input && input(word) => {}
            _ => args.push(word.clone()),
        }
    }
    Ok((program.clone(), args))
}

/// Run `program args` from the unit's directory, `stdin` fed when given;
/// standard output, or the first line of standard error.
fn run_own(
    entry: &CompileEntry,
    program: &str,
    args: &[String],
    stdin: Option<&str>,
) -> Result<String, String> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(&entry.directory)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    if let Some(text) = stdin {
        let mut pipe = child.stdin.take().ok_or("no standard input")?;
        pipe.write_all(text.as_bytes())
            .map_err(|error| error.to_string())?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr)
            .lines()
            .next()
            .unwrap_or("preprocessing failed")
            .to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Preprocess a unit with its own recorded command: `-c` and `-o FILE`
/// dropped, `extra` added, run from its directory. The command the build
/// compiled the unit with, so what it includes is what the program includes.
pub(crate) fn preprocess_own(entry: &CompileEntry, extra: &[&str]) -> Result<String, String> {
    let (program, mut args) = own_arguments(entry, false)?;
    args.extend(extra.iter().map(|word| (*word).to_owned()));
    run_own(entry, &program, &args, None)
}

/// The files of a Make dependency rule (`-M` output), target dropped.
fn dependency_files(rule: &str) -> Vec<String> {
    let joined = rule.replace("\\\r\n", " ").replace("\\\n", " ");
    let mut out = Vec::new();
    for line in joined.lines() {
        // The target ends at the first `:` followed by whitespace or the end.
        let bytes = line.as_bytes();
        let Some(colon) = (0..bytes.len())
            .find(|&at| bytes[at] == b':' && bytes.get(at + 1).is_none_or(u8::is_ascii_whitespace))
        else {
            continue;
        };
        let mut word = String::new();
        let mut chars = line[colon + 1..].chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\\' if chars.peek() == Some(&' ') => {
                    word.push(' ');
                    chars.next();
                }
                ch if ch.is_whitespace() => {
                    if !word.is_empty() {
                        out.push(std::mem::take(&mut word));
                    }
                }
                ch => word.push(ch),
            }
        }
        if !word.is_empty() {
            out.push(word);
        }
    }
    out
}

/// Split a dependency listing into the files Clang resolved (from the unit's
/// directory) and the names it left unresolved (`-MG`).
fn resolve_dependencies(entry: &CompileEntry, rule: &str) -> (Vec<PathBuf>, BTreeSet<String>) {
    let mut found = Vec::new();
    let mut missing = BTreeSet::new();
    for name in dependency_files(rule) {
        let path = Path::new(&name);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            entry.directory.join(path)
        };
        if path.is_file() {
            found.push(path);
        } else if name != "-" {
            missing.insert(name);
        }
    }
    (found, missing)
}

/// The preprocessor directive lines of a generator source (a grammar, a
/// header template), continuation lines kept: what its generated C includes.
fn directive_lines(text: &str) -> String {
    let mut out = String::new();
    let mut continued = false;
    for line in text.lines() {
        if continued || line.trim_start().starts_with('#') {
            out.push_str(line);
            out.push('\n');
            continued = line.ends_with('\\');
        }
    }
    out
}

/// Every regular file under `directory`, sorted, without following links.
fn files_under(directory: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", directory.display()))?;
    entries.sort();
    for path in entries {
        let kind = fs::symlink_metadata(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?
            .file_type();
        if kind.is_dir() {
            files_under(&path, out)?;
        } else if kind.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

/// The CMake build root above a unit's compile directory: the nearest
/// ancestor holding `CMakeCache.txt`.
fn cmake_root(directory: &Path) -> Option<&Path> {
    directory
        .ancestors()
        .find(|dir| dir.join("CMakeCache.txt").is_file())
}

/// Card MEMBER-PREPASS-OTHER-ROUTES: re-verify one excluded unit's evidence
/// against the run's sources, up to the token test: the files whose text
/// must name no token (none for a linkage evidence), or why the evidence
/// cannot hold whatever the tokens.
fn exclusion_files(
    entry: &CompileEntry,
    outside: &Outside,
    source_root: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    match &outside.evidence {
        Evidence::NotLinked {
            links,
            linked,
            unlinked,
        } => {
            let root = cmake_root(&entry.directory).ok_or_else(|| {
                format!(
                    "no CMakeCache.txt above {} to read the link lines from",
                    entry.directory.display()
                )
            })?;
            for link in links {
                let path = root.join(link);
                let text = fs::read_to_string(&path)
                    .map_err(|error| format!("link line {}: {error}", path.display()))?;
                if !text.contains(linked.as_str()) {
                    return Err(format!(
                        "link line {} does not name `{linked}`",
                        path.display()
                    ));
                }
                if text.contains(unlinked.as_str()) {
                    return Err(format!("link line {} names `{unlinked}`", path.display()));
                }
            }
            Ok(Vec::new())
        }
        Evidence::TokensAbsent { sources, generated } => {
            let mut files = Vec::new();
            let mut missing = BTreeSet::new();
            if entry.file.is_file() {
                let rule = preprocess_own(entry, &["-M", "-MG"])
                    .map_err(|why| format!("its includes could not be listed: {why}"))?;
                let (found, unresolved) = resolve_dependencies(entry, &rule);
                files.push(entry.file.clone());
                files.extend(found);
                missing.extend(unresolved);
            } else if sources.is_empty() {
                return Err(
                    "the unit is absent and the scope names no source it is generated from".into(),
                );
            }
            if !sources.is_empty() {
                let root = source_root
                    .ok_or("the scope names generator sources but the route has no source root")?;
                let (program, mut base) = own_arguments(entry, true)?;
                base.extend(["-M", "-MG"].map(str::to_owned));
                for source in sources {
                    let path = root.join(source);
                    if path.is_dir() {
                        files_under(&path, &mut files)?;
                        continue;
                    }
                    let text = fs::read(&path)
                        .map_err(|error| format!("source {}: {error}", path.display()))?;
                    files.push(path.clone());
                    let directives = directive_lines(&String::from_utf8_lossy(&text));
                    if directives.is_empty() {
                        continue;
                    }
                    let mut args = base.clone();
                    if let Some(parent) = path.parent() {
                        args.push("-iquote".into());
                        args.push(parent.display().to_string());
                    }
                    args.extend(["-x", "c", "-"].map(str::to_owned));
                    let rule =
                        run_own(entry, &program, &args, Some(&directives)).map_err(|why| {
                            format!(
                                "the includes of source {} could not be listed: {why}",
                                path.display()
                            )
                        })?;
                    let (found, unresolved) = resolve_dependencies(entry, &rule);
                    files.extend(found);
                    missing.extend(unresolved);
                }
            }
            if let Some(name) = missing.iter().find(|name| !generated.contains(*name)) {
                return Err(format!(
                    "`{name}` is not resolved and the scope does not name it as generated"
                ));
            }
            if let (Some(name), true) = (missing.iter().next(), sources.is_empty()) {
                return Err(format!(
                    "`{name}` is generated but the scope names no source it is generated from"
                ));
            }
            files.sort();
            files.dedup();
            Ok(files)
        }
    }
}

fn read_identifiers(entry: &CompileEntry, scope: &[Outside]) -> Reading {
    let path = entry.file.to_string_lossy().replace('\\', "/");
    if let Some(index) = scope
        .iter()
        .position(|outside| path.contains(&outside.fragment))
    {
        return Reading::Outside(index);
    }
    match preprocess_own(entry, &["-E"]) {
        Ok(text) if is_c(entry) => Reading::C(identifiers(&text)),
        Ok(text) => {
            let mut why = format!(
                "a unit that is not C (`{}`)",
                entry
                    .file
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("?")
            );
            let scoped = match scoped_cxx(entry, &text) {
                Ok(scoped) => scoped,
                Err(reason) => {
                    why.push_str(&format!("; {reason}"));
                    false
                }
            };
            Reading::Other(identifiers(&text), why, scoped)
        }
        Err(error) => Reading::Failed(format!("its preprocessing failed: {error}")),
    }
}

/// The extensions of a C++ unit (the driver's own).
const CXX_EXTENSIONS: &[&str] = &["cc", "cp", "cxx", "cpp", "CPP", "c++", "C"];

/// The flags under which a unit's `-E` text may not be the whole unit: a
/// precompiled header or a module stands in for text, and `-Xclang` may pass
/// either.
const NOT_TEXTUAL_FLAGS: &[&str] = &[
    "-include",
    "-fmodule",
    "-fcxx-modules",
    "-fprebuilt-module-path",
    "-fimplicit-module",
    "-fbuiltin-module-map",
    "-emit-pch",
    "-emit-module",
    "--precompile",
    "-Xclang",
];

/// Card CALLBACK-MEMBER-NON-C-UNITS (rule 12n): whether an unread unit's
/// reading is scoped to the members whose closure-graph tokens its
/// preprocessed text names. `Ok(true)` for a C++ unit whose `-E` text is the
/// whole unit: C++, like C, reaches a member only through its record's
/// definition (a member access, an initializer and a whole-object write all
/// need the complete type, so its tag or a typedef the text spells) and a
/// function only through its name. `Ok(false)` for an assembly or other unit,
/// which writes through offsets and symbols no record names; `Err` for a C++
/// unit whose command reads a precompiled header or a module, or whose text
/// declares or imports a module (the text is then not the whole unit). Every
/// unit that is not scoped opens every member when it names a token.
fn scoped_cxx(entry: &CompileEntry, text: &str) -> Result<bool, String> {
    let cxx = entry
        .file
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| CXX_EXTENSIONS.contains(&ext));
    if !cxx {
        return Ok(false);
    }
    let words = shell_words(&entry.command)?;
    if let Some(flag) = words.iter().skip(1).find(|word| {
        NOT_TEXTUAL_FLAGS
            .iter()
            .any(|flag| word.as_str() == *flag || word.starts_with(flag))
    }) {
        return Err(format!(
            "its command may read a precompiled header or module (`{flag}`)"
        ));
    }
    if let Some(line) = text.lines().find(|line| module_line(line)) {
        return Err(format!(
            "its text declares or imports a module (`{}`)",
            line.trim()
        ));
    }
    Ok(true)
}

/// A line of preprocessed C++ that declares or imports a module, or a
/// module pragma Clang leaves in `-E` output.
fn module_line(line: &str) -> bool {
    let line = line.trim_start();
    if line.starts_with("#pragma") {
        return line
            .split_whitespace()
            .nth(1)
            .is_some_and(|word| word == "clang")
            && line.split_whitespace().nth(2) == Some("module");
    }
    let line = line.strip_prefix("export").map_or(line, str::trim_start);
    ["module", "import"].iter().any(|word| {
        line.strip_prefix(word).is_some_and(|rest| {
            rest.chars()
                .next()
                .is_none_or(|next| !(next == '_' || next.is_ascii_alphanumeric()))
        })
    })
}

/// The C dialect a command names (`-std=`), for reading preprocessed text.
pub(crate) fn dialect_of(entry: &CompileEntry) -> qcore_c_gateway::legacy_c::CCompilerDialect {
    use qcore_c_gateway::legacy_c::CCompilerDialect;
    let words = shell_words(&entry.command).unwrap_or_default();
    match words
        .iter()
        .rev()
        .find_map(|word| word.strip_prefix("-std="))
    {
        Some("c11") => CCompilerDialect::C11,
        Some("gnu99" | "c99") => CCompilerDialect::Gnu99,
        Some("gnu11") => CCompilerDialect::Gnu11,
        _ => CCompilerDialect::Gnu17,
    }
}

/// A unit's facts: read through the gateway's own preprocessing where its
/// compile command is one the gateway reads (the analysis's input), and
/// otherwise from the text its recorded command preprocesses (`-E -P`).
pub fn read_facts(entry: &CompileEntry) -> Result<MemberFacts, String> {
    read_facts_in(entry, None)
}

fn read_facts_in(
    entry: &CompileEntry,
    lifetime: Option<&ResourceContracts>,
) -> Result<MemberFacts, String> {
    let read = |source: &str, unit: &C11TranslationUnit<'_>| -> Result<MemberFacts, String> {
        let mut facts = read_unit(source, unit)?;
        if let Some(contracts) = lifetime {
            facts.lifetime_source = Some(qcore_c_gateway::loop_lifetime_source::read_unit(
                source, unit, contracts,
            )?);
        }
        Ok(facts)
    };
    let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
    if let Ok(unit) = c_unit(entry, &source)
        && let Ok(preprocessed) = qcore_c_gateway::legacy_c::preprocess_unrestricted_c11(&unit)
    {
        return read(&preprocessed, &unit);
    }
    let preprocessed = preprocess_own(entry, &["-E", "-P"])?;
    read(
        &preprocessed,
        &C11TranslationUnit {
            source: &preprocessed,
            dialect: dialect_of(entry),
            ..Default::default()
        },
    )
}

/// What the pre-pass read, and its evidence for the sidecar.
pub struct Prepass {
    pub facts: Arc<RunMemberFacts>,
    pub section: Value,
}

/// Run the pre-pass over `entries` for the members `contracts` invoke;
/// `None` when no declaration invokes a member. `source_root` resolves the
/// scope's generator sources (card MEMBER-PREPASS-OTHER-ROUTES).
pub fn run(
    entries: &[CompileEntry],
    contracts: &ResourceContracts,
    scope: &Scope,
    source_root: Option<&Path>,
    jobs: usize,
) -> Option<Prepass> {
    let members = crate::callback_member_fold::member_declarations(contracts)
        .into_values()
        .flatten()
        .collect::<BTreeSet<_>>();
    let lifetime = contracts
        .lifetime_declarations()
        .filter_map(|(_, d)| d.lifetime.as_ref())
        .collect::<Vec<_>>();
    let mut lifetime_tokens = BTreeSet::new();
    for declared in &lifetime {
        for p in declared
            .registered
            .iter()
            .chain(declared.blocks.iter().flat_map(|b| {
                b.clauses.iter().filter_map(|c| {
                    if let qcore_schemas::loop_lifetime::LocalClause::Registered(p) = c {
                        Some(p)
                    } else {
                        None
                    }
                })
            }))
        {
            lifetime_tokens.insert(p.record.clone());
            lifetime_tokens.extend(p.interior.iter().chain(&p.holder).cloned());
        }
        for b in &declared.bindings {
            lifetime_tokens.extend(b.members.iter().cloned());
        }
    }
    if members.is_empty() && lifetime.is_empty() {
        return None;
    }
    let declared = crate::callback_member_fold::declared_stores(contracts);
    let declarations = declarations_unit(&declared);
    let label = |entry: &CompileEntry| entry.file.to_string_lossy().replace('\\', "/");
    // Every unit's identifiers, once (in parallel, consumed in order).
    let mut readings = Vec::with_capacity(entries.len());
    let _ = crate::parallel::run_ordered(
        entries.iter().collect::<Vec<_>>(),
        jobs,
        None,
        |entry| read_identifiers(entry, &scope.outside),
        |reading| {
            readings.push(reading);
            Ok::<(), ()>(())
        },
    );
    // Card MEMBER-PREPASS-OTHER-ROUTES: every excluded unit's evidence,
    // re-verified against the run's sources up to the token test, and the
    // identifiers of every file the test reads (once each).
    let excluded = readings
        .iter()
        .enumerate()
        .filter_map(|(index, reading)| match reading {
            Reading::Outside(entry) => Some((index, *entry)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut evidence: BTreeMap<usize, Result<Vec<PathBuf>, String>> = BTreeMap::new();
    let _ = crate::parallel::run_ordered(
        excluded,
        jobs,
        None,
        |(index, outside)| {
            (
                index,
                exclusion_files(&entries[index], &scope.outside[outside], source_root),
            )
        },
        |(index, files)| {
            evidence.insert(index, files);
            Ok::<(), ()>(())
        },
    );
    let paths = evidence
        .values()
        .filter_map(|files| files.as_ref().ok())
        .flatten()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut file_ids: BTreeMap<PathBuf, Result<Vec<u32>, String>> = BTreeMap::new();
    let _ = crate::parallel::run_ordered(
        paths.into_iter().collect::<Vec<_>>(),
        jobs,
        None,
        |path| {
            let ids = fs::read(&path)
                .map(|bytes| identifiers(&String::from_utf8_lossy(&bytes)))
                .map_err(|error| format!("{}: {error}", path.display()));
            (path, ids)
        },
        |(path, ids)| {
            file_ids.insert(path, ids);
            Ok::<(), ()>(())
        },
    );
    let mut dropped: BTreeMap<String, String> = BTreeMap::new();
    let mut read: BTreeMap<usize, MemberFacts> = BTreeMap::new();
    let mut unread: BTreeMap<String, String> = BTreeMap::new();
    let mut outside = BTreeMap::new();
    let mut tokens;
    let mut rounds = 0usize;
    let mut failed_asts = BTreeMap::new();
    // Read every C unit naming a token of the closures over what is read so
    // far, until no unread C unit names one. The tokens are recomputed each
    // round (the records read refine a member's keys, which may drop a
    // member name the bootstrap round used); the read set only grows, so the
    // loop ends, and at its end every C unit naming a final token is read.
    // Card MEMBER-PREPASS-OTHER-ROUTES: at each fixed point of the reading
    // (no unread C unit names a token), every remaining exclusion is
    // re-checked against the tokens of that fixed point (the bootstrap
    // round's tokens, before the records refine the members' keys, are never
    // used); a dropped exclusion is read like any other unit, the reading
    // resumes, and an exclusion is never restored, so the loop still ends,
    // and at its end every exclusion holds against the final tokens.
    loop {
        rounds += 1;
        tokens = lifetime_tokens.clone();
        {
            // Retain only references while computing this round's closure.
            // No duplicate facts remain live during the next unit reads.
            let mut views = read.values().collect::<Vec<_>>();
            views.push(&declarations);
            for member in &members {
                tokens.extend(closure_tokens(&views, member));
            }
        }
        let hashes = tokens
            .iter()
            .map(|token| hash(token))
            .collect::<BTreeSet<_>>();
        let names = |ids: &[u32]| hashes.iter().any(|token| ids.binary_search(token).is_ok());
        let selected = readings
            .iter()
            .enumerate()
            .filter(|(index, reading)| {
                !read.contains_key(index) && matches!(reading, Reading::C(ids) if names(ids))
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if selected.is_empty() {
            // The fixed point of the reading: re-check every remaining
            // exclusion against these (final) tokens; a dropped one is read
            // like any other unit and the reading resumes.
            let stale = readings
                .iter()
                .enumerate()
                .filter_map(|(index, reading)| {
                    let Reading::Outside(_) = reading else {
                        return None;
                    };
                    let why = match &evidence[&index] {
                        Err(why) => why.clone(),
                        Ok(files) => {
                            let named = files.iter().find_map(|file| match &file_ids[file] {
                                Err(why) => Some(why.clone()),
                                Ok(ids) if names(ids) => Some(format!(
                                    "{} names a token of the pre-pass ({})",
                                    file.display(),
                                    tokens
                                        .iter()
                                        .filter(|token| ids.binary_search(&hash(token)).is_ok())
                                        .take(3)
                                        .map(|token| format!("`{token}`"))
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                )),
                                Ok(_) => None,
                            });
                            named?
                        }
                    };
                    Some((index, why))
                })
                .collect::<Vec<_>>();
            if stale.is_empty() {
                break;
            }
            let mut fresh = Vec::with_capacity(stale.len());
            let _ = crate::parallel::run_ordered(
                stale,
                jobs,
                None,
                |(index, why)| (index, why, read_identifiers(&entries[index], &[])),
                |result| {
                    fresh.push(result);
                    Ok::<(), ()>(())
                },
            );
            for (index, why, reading) in fresh {
                dropped.insert(label(&entries[index]), why);
                readings[index] = reading;
            }
            continue;
        }
        let mut results = Vec::with_capacity(selected.len());
        let _ = crate::parallel::run_ordered(
            selected,
            jobs,
            None,
            |index| {
                (
                    index,
                    read_facts_in(&entries[index], (!lifetime.is_empty()).then_some(contracts)),
                )
            },
            |result| {
                results.push(result);
                Ok::<(), ()>(())
            },
        );
        for (index, result) in results {
            match result {
                Ok(facts) => {
                    read.insert(index, facts);
                }
                Err(why) => {
                    // Never selected again; unread below.
                    failed_asts.insert(index, format!("its AST is not read: {why}"));
                    read.insert(index, MemberFacts::default());
                }
            }
        }
    }
    // The units the final tokens make relevant but that are not read: a
    // unit that is not C naming a token, a unit whose preprocessing failed
    // (its tokens are unknown), and a C unit whose AST failed.
    let hashes = tokens
        .iter()
        .map(|token| hash(token))
        .collect::<BTreeSet<_>>();
    let names = |ids: &[u32]| hashes.iter().any(|token| ids.binary_search(token).is_ok());
    // Card CALLBACK-MEMBER-NON-C-UNITS: a scoped unread unit's final tokens
    // (a hash collision only adds a token, so only opens more).
    let mut scoped: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (index, reading) in readings.iter().enumerate() {
        let key = label(&entries[index]);
        match reading {
            Reading::Other(ids, why, scope) if names(ids) => {
                if *scope {
                    scoped.insert(
                        key.clone(),
                        tokens
                            .iter()
                            .filter(|token| ids.binary_search(&hash(token)).is_ok())
                            .cloned()
                            .collect(),
                    );
                }
                unread.insert(key, why.clone());
            }
            Reading::Failed(why) => {
                unread.insert(key, why.clone());
            }
            Reading::Outside(entry) => {
                outside.insert(key, scope.outside[*entry].why.clone());
            }
            _ => {}
        }
    }
    for (index, why) in failed_asts {
        unread.insert(label(&entries[index]), why);
        read.remove(&index);
    }
    let mut units = read.into_values().collect::<Vec<_>>();
    units.push(declarations);
    let read_count = units.len() - 1;
    let facts = Arc::new(RunMemberFacts::new(units, unread).with_scoped(tokens.clone(), scoped));
    // Card CALLBACK-MEMBER-NON-C-UNITS: which unread units open each invoked
    // member (evidence; the closure reads the same `opens`).
    let non_c = json!({
        "meaning": "rule 12n, card CALLBACK-MEMBER-NON-C-UNITS: an unread C++ unit whose preprocessed text (its own command, -E) is the whole unit opens only an invoked member whose closure-graph tokens it names (or one with a graph token it was not checked against); every other unread unit (a failed preprocessing, an assembly unit, a C unit whose AST was not read, a C++ unit reading a precompiled header or module) opens every member",
        "scoped": facts.scoped,
        "opened_by": members
            .iter()
            .map(|member| (member.clone(), facts.opening(member)))
            .collect::<BTreeMap<_, _>>(),
    });
    let section = json!({
        "meaning": "the member-facts pre-pass (rule 12n, card OP-APP-RK-CALLBACKS-VIA-POLL-CB, user decisions R1 and F1): every unit of the production compile database was preprocessed; a unit naming a token of an invoked member's closure graph was read whole for its facts, and a unit that names one but could not be read opens every member; a unit naming none holds no store the closure reads",
        "units": entries.len(),
        "read": read_count,
        "rounds": rounds,
        "tokens": tokens,
        "declared_stores": declared.iter().map(|store| format!("{store:?}")).collect::<Vec<_>>(),
        "unread": facts.unread,
        "unread_reasons": facts.unread.values().fold(BTreeMap::<String, usize>::new(), |mut counts, why| {
            *counts.entry(why.chars().take(80).collect()).or_default() += 1;
            counts
        }),
        "outside_program": outside,
        "non_c_units": non_c,
        // Card MEMBER-PREPASS-OTHER-ROUTES: the scope file's exclusions, each
        // re-verified against this run's sources and the final tokens.
        "scope": {
            "meaning": "every exclusion of the profile's member-facts-scope.json, re-verified against this run's own sources (rule 12n): a tokens-absent exclusion holds when the unit's raw text, every file Clang resolves for it under its own command and the generator sources the entry names name no final token and every unresolved include is one the entry names as generated; a not-linked exclusion holds when the named link lines link the archive and not the wrapper. A dropped exclusion's unit is preprocessed and read like any other unit, and opens every member when it cannot be read",
            "evidence_revision": scope.evidence_revision,
            "entries": scope.outside.len(),
            "held": outside.len(),
            "dropped": dropped,
        },
    });
    let mut section = section;
    if !lifetime.is_empty() {
        section["lifetime_protocols"] = serde_json::to_value(
            qcore_c_gateway::loop_lifetime_summary::run_evidence(&facts, contracts),
        )
        .expect("S3 lifetime evidence contains only JSON-compatible fields");
    }
    Some(Prepass { facts, section })
}

/// Card MEMBER-PREPASS-OTHER-ROUTES: SHA-256 of the run's member facts (every
/// read unit's facts, the declarations' unit and the unread units), the
/// identity a checkpoint records for them.
///
/// # Errors
/// A serialisation failure.
pub fn facts_sha256(facts: &RunMemberFacts) -> Result<String, String> {
    // Card CALLBACK-MEMBER-NON-C-UNITS: the scoped units and their checked
    // tokens are part of the facts (a run without any keeps its digest).
    let value = if facts.scoped.is_empty() {
        json!({"units": facts.units, "unread": facts.unread})
    } else {
        json!({
            "units": facts.units,
            "unread": facts.unread,
            "scoped": facts.scoped,
            "checked": facts.checked,
        })
    };
    let bytes = serde_json::to_vec(&value).map_err(|error| error.to_string())?;
    Ok(qcore_contract::profile_loader::sha256_hex(&bytes))
}

/// Card MEMBER-PREPASS-OTHER-ROUTES: what every route does with a pre-pass
/// once it is read, before any unit is analysed: the evidence section to
/// `QCORE_MEMBER_PREPASS_SECTION` when that names a file, the console line,
/// and the hand-off constants' resolution (rule 13l).
///
/// Returns the console line, already printed.
///
/// # Errors
/// A file write, or a hand-off rejection.
pub fn announce(prepass: &Prepass, contracts: &ResourceContracts) -> Result<String, String> {
    if let Some(path) = std::env::var_os("QCORE_MEMBER_PREPASS_SECTION") {
        fs::write(
            &path,
            serde_json::to_vec_pretty(&prepass.section).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("{}: {error}", Path::new(&path).display()))?;
    }
    let line = format!(
        "member facts pre-pass: {} unit(s), {} read, {} unread",
        prepass.section["units"],
        prepass.section["read"],
        prepass.facts.unread.len()
    );
    println!("{line}");
    check_handoff_constants(contracts, &prepass.facts)?;
    Ok(line)
}

/// Card **SUPPLIED-OWNED-RETURN-CODE-HANDOFF** (rule 13l): every hand-off's
/// `@` name resolves, through the units the pre-pass read with Clang, to an
/// enumerator of its candidate's own result type in every read unit that
/// declares the candidate. A typo or a constant of another type rejects the
/// run's contracts: the loader's module rejection, made where Clang has read
/// the units (the loader itself runs no Clang). A candidate no read unit
/// declares is decided nowhere through the run's facts and refuses at each
/// use.
///
/// # Errors
/// The rejection, naming the declaration.
pub fn check_handoff_constants(
    contracts: &ResourceContracts,
    facts: &RunMemberFacts,
) -> Result<(), String> {
    let units = facts.units.iter().collect::<Vec<_>>();
    for (name, declared) in contracts.retaining() {
        let Some(handoff) = &declared.handoff else {
            continue;
        };
        if !units
            .iter()
            .any(|unit| unit.result_types.contains_key(name))
        {
            continue;
        }
        qcore_c_gateway::handoff_branch::resolve(&units, name, &handoff.guard.constant).map_err(
            |why| {
                format!(
                    "the hand-off of `{name}` is rejected (rule 13l: `@` names resolve through \
                     Clang): {why}"
                )
            },
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dependency_rule_lists_its_files_without_the_target() {
        let rule = "out.o: /a/b.c /a/c.h \\\n  d.h with\\ space.h \\\n  /usr/include/stdio.h\n";
        assert_eq!(
            dependency_files(rule),
            [
                "/a/b.c",
                "/a/c.h",
                "d.h",
                "with space.h",
                "/usr/include/stdio.h"
            ]
        );
        assert_eq!(dependency_files("-: x.h\n"), ["x.h"]);
    }

    #[test]
    fn a_generator_sources_directives_are_kept_with_their_continuations() {
        let text =
            "%{\n#include <stdio.h>\n  #define X(a) \\\n   (a)\nint y;\n%}\n%%\nrule: 'x';\n";
        assert_eq!(
            directive_lines(text),
            "#include <stdio.h>\n  #define X(a) \\\n   (a)\n"
        );
    }

    #[test]
    fn a_scope_file_must_name_its_evidence_revision_and_each_evidence() {
        let directory = std::env::temp_dir().join(format!(
            "qcore-member-scope-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let write = |value: Value| {
            fs::write(directory.join(SCOPE_FILE), value.to_string()).unwrap();
            load_scope(&directory)
        };
        let entry = json!({"fragment": "/x.c", "why": "w", "evidence": {"kind": "tokens-absent"}});
        // Schema 1 and a file with no evidence revision are refused.
        assert!(write(json!({"schema_version": 1, "outside_program": [entry]})).is_err());
        assert!(write(json!({"schema_version": 2, "outside_program": [entry]})).is_err());
        // An entry with no or an unknown evidence kind is refused.
        for evidence in [json!(null), json!({"kind": "trusted"})] {
            let bad = json!({"fragment": "/x.c", "why": "w", "evidence": evidence});
            assert!(
                write(
                    json!({"schema_version": 2, "evidence_revision": "r", "outside_program": [bad]})
                )
                .is_err()
            );
        }
        // A not-linked evidence names its link lines.
        let unlinked = json!({"fragment": "/x.cpp", "why": "w", "evidence": {
            "kind": "not-linked", "links": [], "linked": "a.a", "unlinked": "b"}});
        assert!(
            write(
                json!({"schema_version": 2, "evidence_revision": "r", "outside_program": [unlinked]})
            )
            .is_err()
        );
        let scope = write(json!({"schema_version": 2, "evidence_revision": "r", "outside_program": [
            entry,
            {"fragment": "/y.cpp", "why": "v", "evidence": {
                "kind": "not-linked", "links": ["bin/link.txt"], "linked": "a.a", "unlinked": "b"}},
        ]}))
        .unwrap();
        assert_eq!(scope.evidence_revision.as_deref(), Some("r"));
        assert_eq!(
            scope.outside[0].evidence,
            Evidence::TokensAbsent {
                sources: Vec::new(),
                generated: BTreeSet::new()
            }
        );
        assert!(matches!(
            &scope.outside[1].evidence,
            Evidence::NotLinked { links, .. } if links == &["bin/link.txt"]
        ));
        // No file: nothing excluded.
        fs::remove_file(directory.join(SCOPE_FILE)).unwrap();
        assert_eq!(load_scope(&directory).unwrap(), Scope::default());
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn a_not_linked_exclusion_holds_only_while_its_link_lines_say_so() {
        let root = std::env::temp_dir().join(format!(
            "qcore-member-link-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let unit = root.join("lib/cpp");
        fs::create_dir_all(&unit).unwrap();
        fs::create_dir_all(root.join("bin")).unwrap();
        let entry = CompileEntry {
            directory: unit.clone(),
            command: "c++ -c w.cpp".into(),
            file: unit.join("w.cpp"),
        };
        let outside = Outside {
            fragment: "/cpp/".into(),
            why: "w".into(),
            evidence: Evidence::NotLinked {
                links: vec!["bin/link.txt".into()],
                linked: "libk.a".into(),
                unlinked: "k++".into(),
            },
        };
        // No CMakeCache.txt above the unit: cannot hold.
        assert!(exclusion_files(&entry, &outside, None).is_err());
        fs::write(root.join("CMakeCache.txt"), "").unwrap();
        fs::write(root.join("bin/link.txt"), "cc main.o libk.a -o bin\n").unwrap();
        assert_eq!(exclusion_files(&entry, &outside, None), Ok(Vec::new()));
        fs::write(
            root.join("bin/link.txt"),
            "cc main.o libk.a libk++.a -o bin\n",
        )
        .unwrap();
        assert!(exclusion_files(&entry, &outside, None).is_err());
        fs::write(root.join("bin/link.txt"), "cc main.o -o bin\n").unwrap();
        assert!(exclusion_files(&entry, &outside, None).is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn identifiers_are_every_word_hashed_once() {
        let ids = identifiers("int f(int x) { return g(x) + 12u + x; } struct s_t y;");
        for word in ["int", "f", "x", "return", "g", "struct", "s_t", "y"] {
            assert!(ids.binary_search(&hash(word)).is_ok(), "{word}");
        }
        assert!(ids.binary_search(&hash("12u")).is_err());
        assert!(ids.binary_search(&hash("u")).is_err());
        let mut sorted = ids.clone();
        sorted.dedup();
        assert_eq!(sorted, ids);
    }
}
