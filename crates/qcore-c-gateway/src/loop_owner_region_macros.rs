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

//! Macro invocations on the list-head region, expanded as the unit's own
//! preprocessor expands them (card **W47-PREMISES-HOLDER-REGION**, gateway
//! rule 10p's amendment; follow-up `W47-LOOP-OWNER-REGION-MACRO-CALLS`).
//!
//! Step B's list-head region (`loop_owner_instance.rs`) reads the test up to
//! the list-head read, the context maker, the context constructor and what it
//! returns null through, and a keyed setter's two bodies, token by token. A
//! function-like macro invoked there is only a name to that reading: the calls
//! its replacement list makes were not counted, and expanding it by name
//! across the read files picked a definition the unit does not include
//! (`lib/monkey`'s acutest for the test's `TEST_CHECK`). This module reads
//! **the expansion the unit's own preprocessor produced**:
//!
//! * [`derive_facts`] runs each selected compile command (and each extra unit
//!   borrowing one) with its own recorded compiler and flags under `-E`'s
//!   token dump (`-fsyntax-only -Xclang -dump-tokens`). Every token Clang
//!   emits carries its expansion location and, when it came from a macro, its
//!   spelling location. The tokens whose expansion location is in the unit's
//!   own file and that came from a macro are grouped by that location (the
//!   outermost invocation's name): an **invocation**, with each token marked
//!   as spelled in a replacement list (`body`) or in the invocation's own
//!   arguments. File-scope function definitions of the unit are read from the
//!   same stream (a definition's file is its name's), and for every one a
//!   region name may reach outside the census's files, the names its body
//!   uses. The compiler's identity is recorded as the predefined-macro facts
//!   record it, and nothing is decided unless its `--version` is the build
//!   record's and the database is the census's.
//! * [`RegionMacros`] is the census's view: per census path, the units that
//!   compiled it, each with the SHA-256 of the text it read.
//! * [`StepB::region_macro_calls`] reads every invocation on a region part's
//!   lines: a replacement list's `return`, `goto`, unmatched
//!   `break`/`continue`, a field dispatch, a listed list member named or the
//!   context's `config` written refuses; each name the replacement list uses
//!   is resolved **in the unit** (a function the unit defines in a read file
//!   is that file's definition; one it defines elsewhere, such as the test's
//!   `tests/lib/acutest/acutest.h`, is read through the names its body uses;
//!   any other name is an external definition of the read files); an argument
//!   name is resolved as the lexical region resolves it. A resolved function
//!   that reaches a hazard refuses (a call a replacement list makes, or one
//!   an expansion may drop, repeat or put in a loop); every resolved function
//!   joins the region's roots. A unit function outside the source root is
//!   library code (rule 10p's premise that nothing outside the read files
//!   names a read function); one under it without a typed reading, or with an
//!   indirect call, is an unmodelled call of the closure.
//!
//! A region part in a file without facts, a unit whose derivation failed, a
//! text other than the one the facts read, or a function the unit defines in
//! a read file that the census did not read there refuses: an unresolvable
//! macro keeps the region refused.
//!
//! Premises stated, not proved (trust boundary 28): P-RECORDED-COMPILER (as
//! the predefined-macro facts state it) and **P-DUMP-TOKENS-COMPLETE**:
//! Clang's token dump prints every token the unit's preprocessor produces, in
//! order, each with its expansion location and, for a token from a macro, a
//! spelling location in a replacement list (or the scratch buffer) or at the
//! invocation's argument.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use super::StepB;
use super::predefined_macros::{
    CompilerIdentity, ExtraUnit, NOT_PREPROCESSED, database_entries, derivation_flags, identify,
    same_version,
};
use crate::legacy_c::is_identifier;

/// The facts format this module writes and reads.
pub const SCHEMA: u32 = 1;

/// One token of an invocation's expansion.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpansionToken {
    pub text: String,
    /// Spelled in a replacement list (or pasted), not in the invocation's
    /// own arguments.
    pub body: bool,
    /// An identifier, as the preprocessor lexed it (keywords are not).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub identifier: bool,
}

/// One outermost macro invocation in a census file the unit read.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invocation {
    /// The census file it stands in (the unit's own file, or a header).
    pub file: String,
    pub line: usize,
    pub column: usize,
    /// The macro's name as the source spells it there.
    pub name: String,
    /// Its whole expansion, in order.
    pub tokens: Vec<ExpansionToken>,
}

/// A function the unit defines.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitDefinition {
    /// Its file: the path under the source root, else absolute.
    pub file: String,
    /// The file is one of the census's: the census reads its definition.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub census: bool,
    /// For a definition outside the census's files: the identifiers its body
    /// uses (not after `.` or `->`).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub names: BTreeSet<String>,
}

/// What one unit's preprocessor produced.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitExpansions {
    /// The unit's file: its census path (an extra's key).
    pub key: String,
    pub compiler: String,
    /// SHA-256 of the text the unit read.
    pub source_sha256: String,
    /// Every other census file the unit read, with the SHA-256 of its text.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    pub invocations: Vec<Invocation>,
    /// Every function a region name may reach through the unit's own
    /// definitions (closed over the names of those outside the census).
    pub definitions: BTreeMap<String, UnitDefinition>,
    /// Why the derivation failed, when it did.
    pub failure: Option<String>,
}

/// The facts one derivation wrote.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionMacroFacts {
    pub schema: u32,
    pub database_sha256: String,
    pub recorded_compiler_version: String,
    pub compilers: BTreeMap<String, CompilerIdentity>,
    /// The selected database units in database order, then the extras.
    pub units: Vec<UnitExpansions>,
}

/// What [`derive_facts`] reads.
#[derive(Clone, Debug)]
pub struct FactsRequest {
    pub database_text: String,
    pub root: PathBuf,
    pub recorded_compiler_version: String,
    /// The database files (census paths) to derive.
    pub only: BTreeSet<String>,
    pub extras: Vec<ExtraUnit>,
    /// The census's files (paths under the root): a definition there is the
    /// census's to read.
    pub census_files: BTreeSet<String>,
    pub jobs: usize,
}

// ---------------------------------------------------------------------------
// Reading Clang's token dump.

/// A location `FILE:LINE:COL`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Place {
    file: String,
    line: usize,
    column: usize,
}

fn place(text: &str) -> Option<Place> {
    let mut parts = text.rsplitn(3, ':');
    let column = parts.next()?.parse().ok()?;
    let line = parts.next()?.parse().ok()?;
    let file = parts.next()?.to_owned();
    Some(Place { file, line, column })
}

/// One dumped token: its kind, spelling, expansion location and, for a token
/// from a macro, its spelling location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DumpedToken {
    pub kind: String,
    pub text: String,
    expansion: Place,
    spelling: Option<Place>,
}

/// Read one entry of `-Xclang -dump-tokens`:
/// `KIND 'SPELLING'\tFLAGS\tLoc=<FILE:L:C[ <Spelling=FILE:L:C>]>`. An entry
/// whose token was spelled across a line splice carries its raw spelling
/// (`[UnClean='…']`) over several lines; the reader joins them first.
#[must_use]
pub fn dumped_token(line: &str) -> Option<DumpedToken> {
    let at = line.rfind("\tLoc=<")?;
    let (left, location) = (&line[..at], &line[at + 6..]);
    let location = location.strip_suffix('>')?;
    let (kind, rest) = left.split_once(' ')?;
    let rest = rest.strip_prefix('\'')?;
    // The spelling ends at the quote before the flags' tab.
    let close = rest.find("'\t")?;
    let text = rest[..close].to_owned();
    let (expansion, spelling) = match location.split_once(" <Spelling=") {
        Some((expansion, spelling)) => (expansion, Some(spelling.strip_suffix('>')?)),
        None => (location, None),
    };
    Some(DumpedToken {
        kind: kind.to_owned(),
        text,
        expansion: place(expansion)?,
        spelling: match spelling {
            Some(spelling) => Some(place(spelling)?),
            None => None,
        },
    })
}

/// A path with `.` and `..` removed lexically.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// A dumped file name as the census names it: the path under `root`, else
/// the normalized absolute path (relative names resolve against `directory`).
fn census_name(name: &str, directory: &Path, root: &Path) -> String {
    let path = Path::new(name);
    let full = if path.is_absolute() {
        normalize(path)
    } else {
        normalize(&directory.join(path))
    };
    full.strip_prefix(normalize(root)).map_or_else(
        |_| full.to_string_lossy().replace('\\', "/"),
        |relative| relative.to_string_lossy().replace('\\', "/"),
    )
}

/// The identifier the source spells at `line:column` (1-based bytes).
fn name_at(text: &str, line: usize, column: usize) -> String {
    text.lines()
        .nth(line.wrapping_sub(1))
        .and_then(|l| l.get(column.wrapping_sub(1)..))
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect()
        })
        .unwrap_or_default()
}

/// The unit's invocations and definitions, from its dumped tokens. `main` is
/// the unit's own file as the dump names it; `source` its text; `census` the
/// census's files. Definitions are kept only where a region name may reach
/// them: the names of every invocation, closed over the names of definitions
/// outside the census's files.
#[must_use]
pub fn read_dump(
    tokens: &[DumpedToken],
    names: &dyn Fn(&str) -> String,
    census: &BTreeSet<String>,
    text: &dyn Fn(&str) -> Option<String>,
) -> (Vec<Invocation>, BTreeMap<String, UnitDefinition>) {
    let mut invocations: Vec<Invocation> = Vec::new();
    let mut last: Option<Place> = None;
    let mut named: BTreeMap<String, String> = BTreeMap::new();
    let mut texts: BTreeMap<String, Option<String>> = BTreeMap::new();
    for token in tokens {
        let Some(spelling) = &token.spelling else {
            last = None;
            continue;
        };
        let file = named
            .entry(token.expansion.file.clone())
            .or_insert_with(|| names(&token.expansion.file))
            .clone();
        if !census.contains(&file) {
            last = None;
            continue;
        }
        // A token spelled in the invocation's own file after its name is one
        // of its arguments; anything else is a replacement list's.
        let argument = spelling.file == token.expansion.file
            && (spelling.line, spelling.column) > (token.expansion.line, token.expansion.column);
        let expansion = ExpansionToken {
            text: token.text.clone(),
            body: !argument,
            identifier: token.kind == "identifier",
        };
        if last.as_ref() == Some(&token.expansion)
            && let Some(current) = invocations.last_mut()
        {
            current.tokens.push(expansion);
            continue;
        }
        last = Some(token.expansion.clone());
        let source = texts.entry(file.clone()).or_insert_with(|| text(&file));
        invocations.push(Invocation {
            file,
            line: token.expansion.line,
            column: token.expansion.column,
            name: source.as_deref().map_or_else(String::new, |source| {
                name_at(source, token.expansion.line, token.expansion.column)
            }),
            tokens: vec![expansion],
        });
    }
    // File-scope definitions: `NAME ( … ) [attributes] {` at brace depth 0.
    let mut all: BTreeMap<String, (String, BTreeSet<String>)> = BTreeMap::new();
    let mut depth = 0usize;
    let mut k = 0;
    while k < tokens.len() {
        match tokens[k].kind.as_str() {
            "l_brace" => depth += 1,
            "r_brace" => depth = depth.saturating_sub(1),
            "identifier"
                if depth == 0 && tokens.get(k + 1).is_some_and(|t| t.kind == "l_paren") =>
            {
                if let Some(close) = matching_paren(tokens, k + 1) {
                    let mut after = close + 1;
                    while tokens.get(after).is_some_and(|t| {
                        matches!(
                            t.text.as_str(),
                            "__attribute__" | "__asm__" | "asm" | "__asm"
                        )
                    }) && tokens.get(after + 1).is_some_and(|t| t.kind == "l_paren")
                    {
                        match matching_paren(tokens, after + 1) {
                            Some(end) => after = end + 1,
                            None => break,
                        }
                    }
                    if tokens.get(after).is_some_and(|t| t.kind == "l_brace")
                        && let Some(end) = matching_brace(tokens, after)
                    {
                        let file = names(&tokens[k].expansion.file);
                        let body = body_names(&tokens[after..=end]);
                        let entry = all
                            .entry(tokens[k].text.clone())
                            .or_insert_with(|| (file, BTreeSet::new()));
                        entry.1.extend(body);
                        k = end + 1;
                        continue;
                    }
                }
            }
            _ => {}
        }
        k += 1;
    }
    let mut stack = invocations
        .iter()
        .flat_map(|i| i.tokens.iter())
        .filter(|t| t.identifier)
        .map(|t| t.text.clone())
        .collect::<Vec<_>>();
    let mut definitions = BTreeMap::new();
    while let Some(name) = stack.pop() {
        if definitions.contains_key(&name) {
            continue;
        }
        let Some((file, body)) = all.get(&name) else {
            continue;
        };
        let census_file = census.contains(file);
        definitions.insert(
            name.clone(),
            UnitDefinition {
                file: file.clone(),
                census: census_file,
                names: if census_file {
                    BTreeSet::new()
                } else {
                    body.clone()
                },
            },
        );
        if !census_file {
            stack.extend(body.iter().cloned());
        }
    }
    (invocations, definitions)
}

fn matching_paren(tokens: &[DumpedToken], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token.kind.as_str() {
            "l_paren" => depth += 1,
            "r_paren" => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn matching_brace(tokens: &[DumpedToken], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token.kind.as_str() {
            "l_brace" => depth += 1,
            "r_brace" => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// The identifiers a body uses, not after `.` or `->`.
fn body_names(tokens: &[DumpedToken]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != "identifier" {
            continue;
        }
        let member = index > 0 && matches!(tokens[index - 1].kind.as_str(), "period" | "arrow");
        if !member {
            names.insert(token.text.clone());
        }
    }
    names
}

/// Run one unit's token dump and read it.
fn derive_unit(
    (key, compiler): (&str, &str),
    directory: &Path,
    flags: &[String],
    language: &str,
    file: &Path,
    root: &Path,
    census: &BTreeSet<String>,
) -> Result<UnitExpansions, String> {
    let source = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let digest = qcore_schemas::digest::sha256_hex(&source);
    let source = String::from_utf8_lossy(&source).into_owned();
    let mut child = Command::new(compiler)
        .current_dir(directory)
        .args(flags)
        .args([
            "-w",
            "-fsyntax-only",
            "-Xclang",
            "-dump-tokens",
            "-x",
            language,
        ])
        .arg(file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run: {e}"))?;
    let stderr = child.stderr.take().ok_or("no stderr")?;
    let mut tokens = Vec::new();
    let mut unread = None;
    // An entry ends with its location; a line without one continues onto
    // the next (a raw spelling across a line splice).
    let mut entry = String::new();
    for line in BufReader::new(stderr).lines() {
        let line = line.map_err(|e| format!("dump: {e}"))?;
        if !entry.is_empty() {
            entry.push('\n');
        }
        entry.push_str(&line);
        if !line.contains("\tLoc=<") {
            continue;
        }
        match dumped_token(&entry) {
            Some(token) => tokens.push(token),
            None => {
                if unread.is_none() {
                    unread = Some(entry.clone());
                }
            }
        }
        entry.clear();
    }
    if !entry.is_empty() && unread.is_none() {
        unread = Some(entry);
    }
    let status = child.wait().map_err(|e| format!("dump: {e}"))?;
    if !status.success() {
        return Err(format!(
            "exit {:?}: {}",
            status.code(),
            unread.unwrap_or_default()
        ));
    }
    if let Some(line) = unread {
        return Err(format!("a dump line this reader does not read: {line}"));
    }
    let main = tokens
        .iter()
        .map(|t| &t.expansion.file)
        .find(|f| normalize(&directory.join(f)) == normalize(file))
        .cloned()
        .unwrap_or_else(|| file.to_string_lossy().into_owned());
    let names = |name: &str| {
        if name == main {
            key.to_owned()
        } else {
            census_name(name, directory, root)
        }
    };
    let text = |path: &str| -> Option<String> {
        if path == key {
            return Some(source.clone());
        }
        std::fs::read(root.join(path))
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    };
    let (invocations, definitions) = read_dump(&tokens, &names, census, &text);
    let mut headers = BTreeMap::new();
    for file in tokens
        .iter()
        .map(|t| &t.expansion.file)
        .collect::<BTreeSet<_>>()
    {
        let path = names(file);
        if path != key && census.contains(&path) {
            let bytes = std::fs::read(root.join(&path)).map_err(|e| format!("{path}: {e}"))?;
            headers.insert(path, qcore_schemas::digest::sha256_hex(&bytes));
        }
    }
    Ok(UnitExpansions {
        key: key.to_owned(),
        compiler: compiler.to_owned(),
        source_sha256: digest,
        headers,
        invocations,
        definitions,
        failure: None,
    })
}

/// Derive the region-macro facts of the selected commands and the extras.
///
/// # Errors
/// The database cannot be read. A command whose compiler or derivation fails
/// is recorded with its failure, never an error.
pub fn derive_facts(request: &FactsRequest) -> Result<RegionMacroFacts, String> {
    let entries = database_entries(&request.database_text)?;
    let mut compilers = BTreeMap::new();
    for entry in &entries {
        compilers
            .entry(entry.words[0].clone())
            .or_insert_with(|| identify(&entry.words[0]));
    }
    // (key, compiler, directory, flags, file)
    let mut units = Vec::new();
    for entry in &entries {
        let key = entry.key(&request.root);
        if !request.only.contains(&key) {
            continue;
        }
        units.push((
            key,
            entry.words[0].clone(),
            entry.directory.clone(),
            derivation_flags(entry),
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
                    let outcome = match (flags, identity) {
                        (None, _) => Err("not a C or C++ unit".to_owned()),
                        (Some((_, NOT_PREPROCESSED)), _) => Err("not preprocessed".to_owned()),
                        (_, None) => Err("no database command to borrow".to_owned()),
                        (_, Some(identity)) if identity.failure.is_some() => Err(format!(
                            "compiler `{compiler}`: {}",
                            identity.failure.as_deref().unwrap_or("")
                        )),
                        (_, Some(identity))
                            if !same_version(
                                &identity.version,
                                &request.recorded_compiler_version,
                            ) =>
                        {
                            Err(format!(
                                "compiler `{compiler}` is not the recorded build's (`--version` differs)"
                            ))
                        }
                        (Some((flags, language)), Some(_)) => derive_unit(
                            (key, compiler),
                            directory,
                            flags,
                            language,
                            file,
                            &request.root,
                            &request.census_files,
                        ),
                    };
                    let unit = match outcome {
                        Ok(unit) => unit,
                        Err(failure) => UnitExpansions {
                            key: key.clone(),
                            compiler: compiler.clone(),
                            failure: Some(failure),
                            ..UnitExpansions::default()
                        },
                    };
                    results
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)[index] = Some(unit);
                }
            });
        }
    });
    let units = results
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .into_iter()
        .map(Option::unwrap_or_default)
        .collect();
    Ok(RegionMacroFacts {
        schema: SCHEMA,
        database_sha256: qcore_schemas::digest::sha256_hex(request.database_text.as_bytes()),
        recorded_compiler_version: request.recorded_compiler_version.clone(),
        compilers,
        units,
    })
}

// ---------------------------------------------------------------------------
// The census's view.

/// The census's view of the facts: each census path's units.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegionMacros {
    units: BTreeMap<String, Vec<UnitExpansions>>,
    /// The compilers the facts ran (the census's report).
    pub compilers: Vec<CompilerIdentity>,
}

impl RegionMacros {
    /// Check `facts` against the database the census reads and the recorded
    /// build, and build the view.
    ///
    /// # Errors
    /// A schema or database mismatch, a unit whose command is not the
    /// database's, or a compiler that is unavailable or not the recorded one.
    pub fn derive(
        facts: &RegionMacroFacts,
        database_text: &str,
        root: &Path,
    ) -> Result<Self, String> {
        if facts.schema != SCHEMA {
            return Err(format!("facts schema {} is not {SCHEMA}", facts.schema));
        }
        if facts.database_sha256 != qcore_schemas::digest::sha256_hex(database_text.as_bytes()) {
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
        let mut view = Self {
            compilers: facts.compilers.values().cloned().collect(),
            ..Self::default()
        };
        for unit in &facts.units {
            if unit.failure.is_none() && !facts.compilers.contains_key(&unit.compiler) {
                return Err(format!("compiler `{}` was not identified", unit.compiler));
            }
            // A database file's unit ran one of that file's commands' compiler.
            let commands = entries
                .iter()
                .filter(|e| e.key(root) == unit.key)
                .collect::<Vec<_>>();
            if !commands.is_empty() && !commands.iter().any(|e| e.words[0] == unit.compiler) {
                return Err(format!(
                    "the derived unit for `{}` is not the database's",
                    unit.key
                ));
            }
            view.units
                .entry(unit.key.clone())
                .or_default()
                .push(unit.clone());
        }
        // Every command of a derived database file is derived: a file compiled
        // twice is read as the union of its units.
        for (key, units) in &view.units {
            let commanded = entries.iter().filter(|e| e.key(root) == *key).count();
            if commanded > units.len() {
                return Err(format!(
                    "`{key}` has {commanded} database commands, {} derived",
                    units.len()
                ));
            }
        }
        Ok(view)
    }

    /// The units of one census path, or why it cannot be read.
    ///
    /// # Errors
    /// No unit read the path, or one of its units failed.
    pub fn units(&self, path: &str) -> Result<Vec<(&UnitExpansions, &str)>, String> {
        let mut found = Vec::new();
        for unit in self.units.values().flatten() {
            let digest = if unit.key == path {
                Some(unit.source_sha256.as_str())
            } else {
                unit.headers.get(path).map(String::as_str)
            };
            // A failed unit of the file, or any failed unit for a header (it
            // may have read it), leaves the path unread.
            if unit.failure.is_some() && (unit.key == path || !self.units.contains_key(path)) {
                return Err(format!(
                    "the expansion of {} failed: {}",
                    unit.key,
                    unit.failure.as_deref().unwrap_or("")
                ));
            }
            if let Some(digest) = digest {
                found.push((unit, digest));
            }
        }
        if found.is_empty() {
            return Err(format!("no unit's expansion facts read {path}"));
        }
        Ok(found)
    }

    /// The census paths with facts.
    #[must_use]
    pub fn paths(&self) -> Vec<String> {
        self.units.keys().cloned().collect()
    }
}

/// Facts for a fixture: `units` as derived, the recorded compiler `cc`.
#[must_use]
pub fn fixture_facts(database_text: &str, units: Vec<UnitExpansions>) -> RegionMacroFacts {
    RegionMacroFacts {
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
        units,
    }
}

// ---------------------------------------------------------------------------
// Reading the region's invocations.

/// What the region's invocations add to the list-head derivation.
#[derive(Debug, Default)]
pub(super) struct RegionMacroReach {
    /// Functions a replacement list or an argument names.
    pub roots: BTreeSet<usize>,
    /// Unit functions outside the census whose calls are not modelled.
    pub unmodelled: Vec<String>,
    /// Invocations read.
    pub invocations: usize,
    /// Macro names read (for the note).
    pub names: BTreeSet<String>,
}

/// Per macro name, the fields any read file's definition of it calls as a
/// member: `-> F (` or `. F (` in its replacement list (continuation lines
/// joined, comments dropped).
fn member_calls_by_macro(files: &[super::CensusFile]) -> BTreeMap<String, BTreeSet<String>> {
    let mut calls: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for file in files {
        let mut logical = String::new();
        for line in file.text.lines() {
            if logical.is_empty() {
                let trimmed = line.trim_start();
                let Some(rest) = trimmed.strip_prefix('#') else {
                    continue;
                };
                if !rest.trim_start().starts_with("define") {
                    continue;
                }
            }
            if let Some(stripped) = line.strip_suffix('\\') {
                logical.push_str(stripped);
                logical.push(' ');
                continue;
            }
            logical.push_str(line);
            while let Some(start) = logical.find("/*") {
                let end = logical[start + 2..]
                    .find("*/")
                    .map_or(logical.len(), |e| start + 2 + e + 2);
                logical.replace_range(start..end, " ");
            }
            if let Some(start) = logical.find("//") {
                logical.truncate(start);
            }
            let body = logical.trim_start().trim_start_matches('#').trim_start();
            let body = body.strip_prefix("define").unwrap_or(body).trim_start();
            let name_end = body
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(body.len());
            let name = body[..name_end].to_owned();
            let (tokens, _) = crate::legacy_c::tokenize_with_conditionals(&body[name_end..]);
            let texts = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
            for k in 1..texts.len() {
                let member =
                    texts[k - 1] == "." || (texts[k - 1] == ">" && k >= 2 && texts[k - 2] == "-");
                if member && is_identifier(texts[k]) && texts.get(k + 1) == Some(&"(") {
                    calls
                        .entry(name.clone())
                        .or_default()
                        .insert(texts[k].to_owned());
                }
            }
            logical.clear();
        }
    }
    calls
}

/// The census graph with its macro-made calls (card
/// W47-PREMISES-HOLDER-REGION): the functions the region's roots reach and
/// those that reach a hazard.
#[derive(Debug, Default)]
pub(super) struct WidenedReach {
    pub note: String,
    pub closure: BTreeSet<usize>,
    pub reaching: BTreeSet<usize>,
}

/// One region part: a file, a token range and the ranges it skips.
pub(super) type RegionPart = (usize, (usize, usize), Vec<(usize, usize)>);

const CONTROL: &[&str] = &["return", "goto"];
const LOOPS: &[&str] = &["for", "while", "do", "switch"];
const JUMPS: &[&str] = &["break", "continue"];

impl StepB<'_> {
    fn census_definitions(&self, file: &str, name: &str) -> Vec<usize> {
        self.definitions
            .iter()
            .filter(|d| d.name == name && self.files[d.file].path == file)
            .map(|d| self.index[&d.census_name])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// The census functions `name` names in `unit`, with the unit's own
    /// functions outside the census read through their bodies' names.
    fn resolve_in_unit(
        &self,
        unit: &UnitExpansions,
        name: &str,
        hidden: &BTreeMap<String, String>,
        found: &mut BTreeSet<usize>,
        unmodelled: &mut Vec<String>,
        seen: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if !seen.insert(name.to_owned()) {
            return Ok(());
        }
        let Some(definition) = unit.definitions.get(name) else {
            // Not the unit's: an external definition of the read files.
            let ids = self
                .definitions
                .iter()
                .filter(|d| d.name == name && !d.census_name.contains("::"))
                .map(|d| self.index[&d.census_name])
                .collect::<BTreeSet<_>>();
            if ids.is_empty()
                && let Some(file) = hidden.get(name)
            {
                return Err(format!(
                    "`{name}` may be defined only in {file}, a file this reader cannot structure"
                ));
            }
            found.extend(ids);
            return Ok(());
        };
        if definition.census {
            let ids = self.census_definitions(&definition.file, name);
            if ids.is_empty() {
                return Err(format!(
                    "the unit defines `{name}` in {}, where this reading has no definition of it",
                    definition.file
                ));
            }
            found.extend(ids);
            return Ok(());
        }
        // Outside the census's files: under the source root, the typed
        // reading must have read it without an indirect call; library code
        // outside the root is rule 10p's premise.
        if !Path::new(&definition.file).is_absolute() {
            match self.typed {
                Some(typed)
                    if typed
                        .functions
                        .contains(&format!("{}|{name}", definition.file)) =>
                {
                    let key = format!("{}|{name}|", definition.file);
                    if typed
                        .indirect_calls
                        .range(key.clone()..)
                        .next()
                        .is_some_and(|site| site.starts_with(&key))
                    {
                        unmodelled.push(format!(
                            "`{name}` ({}) makes an indirect call",
                            definition.file
                        ));
                    }
                }
                _ => unmodelled.push(format!(
                    "`{name}` ({}) has no typed reading",
                    definition.file
                )),
            }
        }
        for inner in &definition.names {
            self.resolve_in_unit(unit, inner, hidden, found, unmodelled, seen)?;
        }
        Ok(())
    }

    /// Every invocation on the region's parts, read from its unit's
    /// expansion: the functions it names, or why the region is refused.
    pub(super) fn region_macro_calls(
        &self,
        view: &RegionMacros,
        parts: &[RegionPart],
        reaching: &BTreeSet<usize>,
        witness: &dyn Fn(usize) -> String,
    ) -> Result<RegionMacroReach, String> {
        let hidden = self.hidden_definitions();
        let tables = self.flow;
        let mut reach = RegionMacroReach::default();
        // Every part's file must have been read: name them all.
        let unread = parts
            .iter()
            .map(|(file, _, _)| self.files[*file].path.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter_map(|path| view.units(path).err())
            .collect::<Vec<_>>();
        if !unread.is_empty() {
            return Err(unread.join("; "));
        }
        for (file, range, skipped) in parts {
            let path = &self.files[*file].path;
            let units = view.units(path)?;
            let digest = qcore_schemas::digest::sha256_hex(self.files[*file].text.as_bytes());
            let tokens = &self.scans[*file].tokens;
            let last = range.1.min(tokens.len().saturating_sub(1));
            // The part's lines, but a line every token of which lies in a
            // skipped range.
            let mut lines: BTreeMap<usize, bool> = BTreeMap::new();
            for (k, token) in tokens.iter().enumerate().take(last + 1).skip(range.0) {
                let inside = skipped.iter().any(|(a, b)| *a <= k && k <= *b);
                let entry = lines.entry(token.line).or_insert(true);
                *entry &= inside;
            }
            for (unit, read) in units {
                if read != digest {
                    return Err(format!("the expansion facts of {path} read another text"));
                }
                for invocation in &unit.invocations {
                    if invocation.file != *path || lines.get(&invocation.line) != Some(&false) {
                        continue;
                    }
                    reach.invocations += 1;
                    reach.names.insert(invocation.name.clone());
                    let site = format!(
                        "the macro `{}` invoked at {path}:{}",
                        invocation.name, invocation.line
                    );
                    let expansion = &invocation.tokens;
                    let body = |t: &ExpansionToken| t.body;
                    let has = |words: &[&str]| {
                        expansion
                            .iter()
                            .any(|t| t.body && words.contains(&t.text.as_str()))
                    };
                    if has(CONTROL) {
                        return Err(format!(
                            "{site} expands to a jump this reading does not model"
                        ));
                    }
                    if has(JUMPS) && !has(LOOPS) {
                        return Err(format!(
                            "{site} expands to a `break` or `continue` of an enclosing statement"
                        ));
                    }
                    for (i, t) in expansion.iter().enumerate() {
                        let member = i > 0 && matches!(expansion[i - 1].text.as_str(), "." | "->");
                        if member && t.identifier {
                            let from_body = body(t) || body(&expansion[i - 1]);
                            let called = expansion.get(i + 1).is_some_and(|n| n.text == "(");
                            if called && (from_body || expansion.get(i + 1).is_some_and(body)) {
                                return Err(format!("{site} expands to a field dispatch"));
                            }
                            let listed =
                                t.text == tables.list_field || t.text == tables.registrations_field;
                            let rewrites = t.text == tables.context_config_field
                                && expansion.get(i + 1).is_some_and(|n| n.text == "=");
                            if from_body && (listed || rewrites) {
                                return Err(format!(
                                    "{site} expands to a use of `{}` this reading does not \
                                     classify",
                                    t.text
                                ));
                            }
                            continue;
                        }
                        if !t.identifier || !is_identifier(&t.text) {
                            continue;
                        }
                        let mut found = BTreeSet::new();
                        if t.body {
                            let mut seen = BTreeSet::new();
                            self.resolve_in_unit(
                                unit,
                                &t.text,
                                &hidden,
                                &mut found,
                                &mut reach.unmodelled,
                                &mut seen,
                            )
                            .map_err(|why| format!("{site}: {why}"))?;
                        } else {
                            found.extend(self.resolve_name(*file, &t.text));
                        }
                        for id in &found {
                            if reaching.contains(id) {
                                return Err(format!(
                                    "{site} {} `{}`, which reaches a hazard ({})",
                                    if t.body { "names" } else { "carries" },
                                    self.functions[*id],
                                    witness(*id)
                                ));
                            }
                        }
                        reach.roots.extend(found);
                    }
                }
            }
        }
        Ok(reach)
    }

    /// The calls the census graph misses: each function's macro invocations
    /// (from its unit's expansion where a derived unit read its file, else
    /// every read file's definition of the invoked macro, through nested
    /// macros) as edges. A region root that reaches a hazard only through one
    /// refuses; a name a unit's expansion cannot resolve counts as reaching.
    pub(super) fn closure_macro_calls(
        &self,
        view: &RegionMacros,
        roots: &BTreeSet<usize>,
        reaching: &BTreeSet<usize>,
    ) -> Result<WidenedReach, String> {
        let hidden = self.hidden_definitions();
        // Every invocation by the census file it stands in.
        let mut by_file: BTreeMap<&str, Vec<(&UnitExpansions, &Invocation)>> = BTreeMap::new();
        for path in view.paths() {
            let Ok(units) = view.units(&path) else {
                continue;
            };
            for (unit, _) in units {
                for invocation in &unit.invocations {
                    by_file
                        .entry(invocation.file.as_str())
                        .or_default()
                        .push((unit, invocation));
                }
            }
        }
        let read_files = view
            .units
            .values()
            .flatten()
            .filter(|u| u.failure.is_none())
            .flat_map(|u| {
                std::iter::once(u.key.as_str()).chain(u.headers.keys().map(String::as_str))
            })
            .collect::<BTreeSet<_>>();
        let mut extra: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        let mut poisoned: BTreeMap<usize, String> = BTreeMap::new();
        let mut memo: BTreeMap<(usize, String), BTreeSet<usize>> = BTreeMap::new();
        let mut edges = 0usize;
        let mut dispatches = 0usize;
        let member_calls = member_calls_by_macro(self.files);
        for definition in self.definitions {
            let id = self.index[&definition.census_name];
            let path = self.files[definition.file].path.as_str();
            let tokens = &self.scans[definition.file].tokens;
            let mut found = BTreeSet::new();
            if read_files.contains(path) {
                let lines = tokens[definition.body.0].line..=tokens[definition.body.1].line;
                for (unit, invocation) in by_file.get(path).into_iter().flatten() {
                    if !lines.contains(&invocation.line) {
                        continue;
                    }
                    // A field dispatch a replacement list makes reaches every
                    // registrant of the field.
                    let expansion = &invocation.tokens;
                    for (i, t) in expansion.iter().enumerate() {
                        let member = i > 0 && matches!(expansion[i - 1].text.as_str(), "." | "->");
                        let called = expansion.get(i + 1).is_some_and(|n| n.text == "(");
                        if member
                            && called
                            && t.identifier
                            && (t.body || expansion[i - 1].body || expansion[i + 1].body)
                        {
                            found.extend(
                                self.registrants
                                    .get(&t.text)
                                    .into_iter()
                                    .flat_map(BTreeMap::keys)
                                    .copied(),
                            );
                            dispatches += 1;
                        }
                    }
                    for t in invocation.tokens.iter().filter(|t| t.body && t.identifier) {
                        let mut seen = BTreeSet::new();
                        let mut unmodelled = Vec::new();
                        if let Err(why) = self.resolve_in_unit(
                            unit,
                            &t.text,
                            &hidden,
                            &mut found,
                            &mut unmodelled,
                            &mut seen,
                        ) {
                            poisoned.entry(id).or_insert(why);
                        }
                    }
                }
            } else {
                let texts = self.texts(definition.file);
                for k in definition.body.0..=definition.body.1 {
                    let text = texts[k];
                    if !self.macro_bodies.contains_key(text) || matches!(texts[k - 1], "." | ">") {
                        continue;
                    }
                    let callees = memo
                        .entry((definition.file, text.to_owned()))
                        .or_insert_with(|| {
                            let mut callees = self.macro_callees(definition.file, text);
                            // Every read file's definition of the macro, every
                            // arm: a target no compiled unit defines is not
                            // part of the program.
                            callees.retain(|id| !self.compiled_by_no_unit(*id));
                            // A field a replacement list calls (`->F (` or
                            // `.F (`): every registrant of it.
                            for word in self.macro_member_calls(text, &member_calls) {
                                callees.extend(
                                    self.registrants
                                        .get(&word)
                                        .into_iter()
                                        .flat_map(BTreeMap::keys)
                                        .copied(),
                                );
                            }
                            callees
                        });
                    found.extend(callees.iter().copied());
                }
            }
            found.remove(&id);
            let direct = self.reach.next.get(&id);
            edges += found
                .iter()
                .filter(|to| direct.is_none_or(|d| !d.contains(to)))
                .count();
            if !found.is_empty() {
                extra.entry(id).or_default().extend(found);
            }
        }
        // Reaching over the census graph and the macro-made calls.
        let mut back: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (from, tos) in self.reach.next.iter().chain(extra.iter()) {
            for to in tos {
                back.entry(*to).or_default().push(*from);
            }
        }
        let mut widened = reaching.clone();
        widened.extend(poisoned.keys().copied());
        let mut stack = widened.iter().copied().collect::<Vec<_>>();
        while let Some(at) = stack.pop() {
            for from in back.get(&at).into_iter().flatten() {
                if widened.insert(*from) {
                    stack.push(*from);
                }
            }
        }
        if let Some(root) = roots
            .iter()
            .find(|r| !reaching.contains(r) && widened.contains(r))
        {
            let why = poisoned
                .iter()
                .map(|(id, why)| format!("`{}`: {why}", self.functions[*id]))
                .next()
                .unwrap_or_default();
            // A shortest path to a hazard or an unresolved function, the
            // macro-made steps marked.
            let mut parent: BTreeMap<usize, (usize, bool)> = BTreeMap::new();
            let mut queue = std::collections::VecDeque::from([*root]);
            let mut seen = BTreeSet::from([*root]);
            let mut end = None;
            while let Some(at) = queue.pop_front() {
                if reaching.contains(&at) || poisoned.contains_key(&at) {
                    end = Some(at);
                    break;
                }
                let steps = self
                    .reach
                    .next
                    .get(&at)
                    .into_iter()
                    .flatten()
                    .map(|to| (*to, false))
                    .chain(extra.get(&at).into_iter().flatten().map(|to| (*to, true)));
                for (to, made) in steps {
                    if widened.contains(&to) && seen.insert(to) {
                        parent.insert(to, (at, made));
                        queue.push_back(to);
                    }
                }
            }
            let mut path = Vec::new();
            let mut cursor = end;
            while let Some(at) = cursor {
                let step = parent.get(&at);
                path.push(format!(
                    "{}{}",
                    if step.is_some_and(|(_, made)| *made) {
                        "(macro) "
                    } else {
                        ""
                    },
                    self.functions[at]
                ));
                cursor = step.map(|(from, _)| *from);
            }
            path.reverse();
            return Err(format!(
                "`{}` reaches a hazard through a macro-made call the census graph does not \
                 hold ({}){}",
                self.functions[*root],
                path.join(" > "),
                if why.is_empty() {
                    String::new()
                } else {
                    format!(" (or an unresolved one, first {why})")
                }
            ));
        }
        // The functions the roots reach over both.
        let mut closure = BTreeSet::new();
        let mut stack = roots.iter().copied().collect::<Vec<_>>();
        while let Some(at) = stack.pop() {
            if closure.insert(at) {
                stack.extend(self.reach.next.get(&at).into_iter().flatten().copied());
                stack.extend(extra.get(&at).into_iter().flatten().copied());
            }
        }
        Ok(WidenedReach {
            note: format!(
                "closure macro calls: {edges} macro-made call(s) of the read files ({dispatches} \
                 field dispatch(es) of the units' expansions to every registrant; the units' \
                 expansions for the {} file(s) they read, every read file's definition and \
                 every registrant of a field it calls elsewhere) added to the census graph; no \
                 call of the region reaches a hazard through one",
                read_files.len()
            ),
            closure,
            reaching: widened,
        })
    }

    /// The fields the read files' definitions of macro `name` call as
    /// members (`->F (` / `.F (`), through nested macros.
    fn macro_member_calls(
        &self,
        name: &str,
        member_calls: &BTreeMap<String, BTreeSet<String>>,
    ) -> BTreeSet<String> {
        let mut fields = BTreeSet::new();
        let mut seen = BTreeSet::new();
        let mut stack = vec![name.to_owned()];
        while let Some(macro_name) = stack.pop() {
            if !seen.insert(macro_name.clone()) {
                continue;
            }
            fields.extend(member_calls.get(&macro_name).into_iter().flatten().cloned());
            for word in self.macro_bodies.get(&macro_name).into_iter().flatten() {
                if self.macro_bodies.contains_key(word) {
                    stack.push(word.clone());
                }
            }
        }
        fields
    }

    /// Every definition of `id` is in a `.c` file each of whose units the
    /// typed reading read without failure, and none of them defines it: no
    /// compiled unit holds it.
    fn compiled_by_no_unit(&self, id: usize) -> bool {
        let Some(typed) = self.typed else {
            return false;
        };
        let definitions = self
            .definitions
            .iter()
            .filter(|d| self.index[&d.census_name] == id)
            .collect::<Vec<_>>();
        !definitions.is_empty()
            && definitions.iter().all(|d| {
                let path = &self.files[d.file].path;
                path.ends_with(".c")
                    && typed.units.contains(path)
                    && !typed.failures.contains_key(path)
                    && !typed.functions.contains(&format!("{path}|{}", d.name))
            })
    }

    /// The indirect calls the typed reading records in a function read in
    /// place (spelled in it or in a macro's replacement list it invokes).
    pub(super) fn own_indirect_calls(
        &self,
        typed: &crate::typed_dispatch_census::TypedFacts,
        own: &BTreeSet<usize>,
    ) -> Vec<String> {
        let mut found = Vec::new();
        for id in own {
            let name = &self.functions[*id];
            let plain = name.rsplit_once("::").map_or(name.as_str(), |(_, p)| p);
            for site in &typed.indirect_calls {
                let mut parts = site.split('|');
                let (Some(file), Some(function), Some(line)) =
                    (parts.next(), parts.next(), parts.next())
                else {
                    continue;
                };
                if function == plain {
                    found.push(format!(
                        "`{name}`, read in place, makes an indirect call at {file}:{line}"
                    ));
                }
            }
        }
        found
    }
}
