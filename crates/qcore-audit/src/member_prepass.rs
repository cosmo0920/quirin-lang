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

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
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

/// One unit the profile names outside the program: a path fragment and the
/// recorded evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outside {
    pub fragment: String,
    pub why: String,
}

/// Read the profile's scope file (`[]` when absent).
pub fn load_scope(directory: &Path) -> Result<Vec<Outside>, String> {
    let path = directory.join(SCOPE_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let value: Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    value["outside_program"]
        .as_array()
        .ok_or_else(|| format!("{}: no `outside_program` list", path.display()))?
        .iter()
        .map(|entry| {
            let text = |key: &str| {
                entry[key]
                    .as_str()
                    .filter(|text| !text.is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{}: an entry has no `{key}`", path.display()))
            };
            Ok(Outside {
                fragment: text("fragment")?,
                why: text("why")?,
            })
        })
        .collect()
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
    /// command with `-E`.
    Other(Vec<u32>, String),
    /// Preprocessing failed: why.
    Failed(String),
    /// The profile names it outside the program: why.
    Outside(String),
}

fn c_unit<'a>(entry: &CompileEntry, source: &'a str) -> Result<C11TranslationUnit<'a>, String> {
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

/// Preprocess a unit with its own recorded command: `-c` and `-o FILE`
/// dropped, `extra` added, run from its directory. The command the build
/// compiled the unit with, so what it includes is what the program includes.
fn preprocess_own(entry: &CompileEntry, extra: &[&str]) -> Result<String, String> {
    let words = shell_words(&entry.command)?;
    let (program, rest) = words.split_first().ok_or("empty command")?;
    let mut args = Vec::new();
    let mut skip = false;
    for word in rest {
        if skip {
            skip = false;
            continue;
        }
        match word.as_str() {
            "-c" => {}
            "-o" => skip = true,
            _ if word.starts_with("-o") && word.len() > 2 => {}
            _ => args.push(word.clone()),
        }
    }
    args.extend(extra.iter().map(|word| (*word).to_owned()));
    let output = Command::new(program)
        .args(&args)
        .current_dir(&entry.directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
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

fn read_identifiers(entry: &CompileEntry, scope: &[Outside]) -> Reading {
    let path = entry.file.to_string_lossy();
    if let Some(outside) = scope
        .iter()
        .find(|outside| path.contains(&outside.fragment))
    {
        return Reading::Outside(outside.why.clone());
    }
    match preprocess_own(entry, &["-E"]) {
        Ok(text) if is_c(entry) => Reading::C(identifiers(&text)),
        Ok(text) => Reading::Other(
            identifiers(&text),
            format!(
                "a unit that is not C (`{}`)",
                entry
                    .file
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("?")
            ),
        ),
        Err(error) => Reading::Failed(format!("its preprocessing failed: {error}")),
    }
}

/// The C dialect a command names (`-std=`), for reading preprocessed text.
fn dialect_of(entry: &CompileEntry) -> qcore_c_gateway::legacy_c::CCompilerDialect {
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
    let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
    if let Ok(unit) = c_unit(entry, &source)
        && let Ok(preprocessed) = qcore_c_gateway::legacy_c::preprocess_unrestricted_c11(&unit)
    {
        return read_unit(&preprocessed, &unit);
    }
    let preprocessed = preprocess_own(entry, &["-E", "-P"])?;
    read_unit(
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
/// `None` when no declaration invokes a member.
pub fn run(
    entries: &[CompileEntry],
    contracts: &ResourceContracts,
    scope: &[Outside],
    jobs: usize,
) -> Option<Prepass> {
    let members = crate::callback_member_fold::member_declarations(contracts)
        .into_values()
        .flatten()
        .collect::<BTreeSet<_>>();
    if members.is_empty() {
        return None;
    }
    let declared = crate::callback_member_fold::declared_stores(contracts);
    let declarations = declarations_unit(&declared);
    let label = |entry: &CompileEntry| entry.file.to_string_lossy().into_owned();
    // Every unit's identifiers, once (in parallel, consumed in order).
    let mut readings = Vec::with_capacity(entries.len());
    let _ = crate::parallel::run_ordered(
        entries.iter().collect::<Vec<_>>(),
        jobs,
        None,
        |entry| read_identifiers(entry, scope),
        |reading| {
            readings.push(reading);
            Ok::<(), ()>(())
        },
    );
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
    loop {
        rounds += 1;
        let mut units = read.values().cloned().collect::<Vec<_>>();
        units.push(declarations.clone());
        let views = units.iter().collect::<Vec<_>>();
        tokens = BTreeSet::new();
        for member in &members {
            tokens.extend(closure_tokens(&views, member));
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
            break;
        }
        let mut results = Vec::with_capacity(selected.len());
        let _ = crate::parallel::run_ordered(
            selected,
            jobs,
            None,
            |index| (index, read_facts(&entries[index])),
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
    for (index, reading) in readings.iter().enumerate() {
        let key = label(&entries[index]);
        match reading {
            Reading::Other(ids, why) if names(ids) => {
                unread.insert(key, why.clone());
            }
            Reading::Failed(why) => {
                unread.insert(key, why.clone());
            }
            Reading::Outside(why) => {
                outside.insert(key, why.clone());
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
    let facts = Arc::new(RunMemberFacts::new(units, unread));
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
    });
    Some(Prepass { facts, section })
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
