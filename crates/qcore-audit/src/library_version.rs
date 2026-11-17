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

//! Library version ranges at audit time (card **OPENSSL-PROVIDER-PLUGIN**,
//! user decisions of 2026-11-16; contract spec §C06.2, "Library version
//! ranges").
//!
//! A declaration may state the versions of an external library it holds for
//! (`requires resource_library_version(LIB, since "V"[, removed "W"]);`,
//! `qcore_schemas::library_version`). The library's **version source** is
//! declared once per crate, beside its `contracts.lock`, in
//! `library-versions.json`:
//!
//! ```json
//! { "schema_version": 1,
//!   "libraries": { "openssl": { "header": "openssl/opensslv.h",
//!                               "major": "OPENSSL_VERSION_MAJOR",
//!                               "minor": "OPENSSL_VERSION_MINOR",
//!                               "patch": "OPENSSL_VERSION_PATCH" } } }
//! ```
//!
//! The **probe** preprocesses `#include <HEADER>` with Clang
//! (`qcore_c_gateway::clang_command`) under each include configuration of the
//! audited build and reads the three macros, each a decimal integer. The
//! configurations are the distinct `-I`/`-isystem`/`-iquote`/`-idirafter`
//! lists of the compile databases the run analyses ([`ProbeDatabases`],
//! card LIBRARY-VERSION-PROBE-ROUTE-DATABASE): the configured and current
//! routes (and the pinned route) hand the probe their own databases, and
//! `quirin-audit pin` takes the same `--compile-database`. Precedence: the
//! database `QCORE_LIBRARY_VERSION_COMPILE_DATABASE` names, when it is set,
//! is an explicit override of the run's own (and the record says so); else
//! the run's databases; else, for a run with no database (the contracts
//! route, a pin without `--compile-database`), one configuration with Clang's
//! default search path (which is the build's exactly when the build passes
//! the library no include directory, as the prepared Fluent Bit build does
//! for OpenSSL). A database that cannot be read, holds no compile command,
//! or carries a command whose include configuration cannot be read
//! ([`include_flags`]) never falls back to the default search path: every
//! probe under it is [`LibraryVersionFailure::Unreadable`], and a pin
//! refuses.
//!
//! `quirin-audit pin` records the probed version in the crate's lock
//! (`library_versions`); every audit probes again ([`evaluate`]) before any
//! unit is analysed. A declaration is in force only when the version is
//! readable, unambiguous (every configuration resolves one header reporting
//! one version), within its range and equal to the pinned one. Otherwise it
//! is withheld (`ResourceContracts::withhold_for_pin_mismatch`): a version
//! that only differs from the pinned one is `contract-pin-mismatch`
//! (`Pending`, a review and re-pin resolve it); out of range, unreadable or
//! ambiguous holds every call `Unknown`. Without any declaration that states a
//! range nothing here runs, so every report is byte-identical.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::contract_revision::PinMismatchReason;
use qcore_schemas::library_version::{LibraryVersion, LibraryVersionFailure};
use qcore_schemas::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The version-source file's name, beside a crate's `contracts.lock`.
pub const SOURCE_FILE: &str = "library-versions.json";

/// The one schema of [`SOURCE_FILE`].
pub const SOURCE_SCHEMA: u32 = 1;

/// The variable naming a compile database whose include configurations the
/// probe reads, overriding the run's own databases.
pub const PROBE_DATABASE_VARIABLE: &str = "QCORE_LIBRARY_VERSION_COMPILE_DATABASE";

/// The configurations source of a run with no compile database: one
/// configuration, Clang's default search path.
pub const DEFAULT_SEARCH_PATH: &str = "clang-default-search-path";

/// What the report record means, carried in it.
pub const MEANING: &str = "library version ranges (card OPENSSL-PROVIDER-PLUGIN, contract spec C06.2): a declaration stating `resource_library_version(LIB, since|documented V[, removed W])` is in force only where the version the audited build's headers report (read from the crate's library-versions.json source by preprocessing under every include configuration) is readable, unambiguous, within [V, W) and equal to the version its lock pins; a version that only differs from the pinned one is held as contract-pin-mismatch (Pending: review and re-pin), and out of range, unreadable or ambiguous holds every call Unknown";

/// One library's version source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionSource {
    /// The header to include, as `#include <HEADER>` spells it.
    pub header: String,
    pub major: String,
    pub minor: String,
    pub patch: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    schema_version: u32,
    libraries: BTreeMap<String, VersionSource>,
}

/// The version a lock pins for one library (`ContractLock::library_versions`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedLibraryVersion {
    /// `MAJOR.MINOR.PATCH`.
    pub version: String,
    /// The header the probe resolved, as provenance.
    pub header: String,
    /// Its SHA-256, as provenance (a different digest with an equal version
    /// is not a mismatch: the identity is the version, user decision
    /// 2026-11-16).
    pub header_sha256: String,
}

/// Read a crate's version sources. `Ok(None)` when the crate has none.
///
/// # Errors
///
/// A file that does not parse, names another schema, or carries a header or
/// macro name that is not a plain spelling.
pub fn read_sources(crate_root: &Path) -> Result<Option<BTreeMap<String, VersionSource>>, String> {
    let path = crate_root.join(SOURCE_FILE);
    if !path.is_file() {
        return Ok(None);
    }
    let file: SourceFile = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?,
    )
    .map_err(|error| format!("{}: {error}", path.display()))?;
    if file.schema_version != SOURCE_SCHEMA {
        return Err(format!(
            "{}: schema {} is not {SOURCE_SCHEMA}",
            path.display(),
            file.schema_version
        ));
    }
    let identifier = |text: &str| {
        !text.is_empty()
            && !text.starts_with(|c: char| c.is_ascii_digit())
            && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    for (name, source) in &file.libraries {
        let header_ok = !source.header.is_empty()
            && !source.header.starts_with('/')
            && !source.header.contains("..")
            && source
                .header
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-./".contains(c));
        if !header_ok
            || !identifier(&source.major)
            || !identifier(&source.minor)
            || !identifier(&source.patch)
        {
            return Err(format!(
                "{}: library `{name}` must name a relative header and three macro names",
                path.display()
            ));
        }
    }
    Ok(Some(file.libraries))
}

/// What a probe read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Probed {
    pub version: LibraryVersion,
    pub header: String,
    pub header_sha256: String,
    /// How many include configurations were probed.
    pub configurations: usize,
}

/// The compile databases a run analyses units from, whose include
/// configurations the probe reads (card LIBRARY-VERSION-PROBE-ROUTE-DATABASE).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ProbeDatabases {
    /// A run with no compile database (the contracts route, a pin without
    /// `--compile-database`): Clang's default search path.
    #[default]
    None,
    /// The run's own databases, in the order it reads them.
    Route(Vec<PathBuf>),
}

impl ProbeDatabases {
    /// One route database.
    #[must_use]
    pub fn route(database: &Path) -> Self {
        Self::Route(vec![database.to_path_buf()])
    }
}

/// The include configurations of the audited build and where they came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Configurations {
    /// The record's `configurations` value.
    pub source: String,
    /// The distinct include-flag lists, or why they cannot be read (every
    /// probe under them is then unreadable, never the default search path).
    pub sets: Result<Vec<Vec<String>>, String>,
}

/// The include configurations of the audited build: the database
/// [`PROBE_DATABASE_VARIABLE`] names (an explicit override, recorded as one),
/// else `databases` (the run's own), else Clang's default search path.
#[must_use]
pub fn include_configurations(databases: &ProbeDatabases) -> Configurations {
    if let Some(path) = std::env::var_os(PROBE_DATABASE_VARIABLE) {
        let path = PathBuf::from(path);
        return Configurations {
            source: format!(
                "override {PROBE_DATABASE_VARIABLE}: compile-database:{}",
                path.display()
            ),
            sets: database_configurations(std::slice::from_ref(&path)),
        };
    }
    match databases {
        ProbeDatabases::None => Configurations {
            source: DEFAULT_SEARCH_PATH.into(),
            sets: Ok(vec![Vec::new()]),
        },
        ProbeDatabases::Route(paths) => Configurations {
            source: paths
                .iter()
                .map(|path| format!("compile-database:{}", path.display()))
                .collect::<Vec<_>>()
                .join(" + "),
            sets: database_configurations(paths),
        },
    }
}

/// The distinct include configurations of `paths`, read by the routes' own
/// compile-record reader ([`crate::compile_db::compile_entries`]). Total or
/// refused: no database, an unreadable database, one with no compile
/// command, a record the reader refuses, or a command whose include
/// configuration cannot be read ([`include_flags`]) is an error, never the
/// default search path.
fn database_configurations(paths: &[PathBuf]) -> Result<Vec<Vec<String>>, String> {
    if paths.is_empty() {
        return Err("the run names no compile database".into());
    }
    let mut sets = BTreeSet::new();
    for path in paths {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("compile database {}: {error}", path.display()))?;
        let entries = crate::compile_db::compile_entries(&text)
            .map_err(|error| format!("compile database {}: {error}", path.display()))?;
        if entries.is_empty() {
            return Err(format!(
                "compile database {} holds no compile command",
                path.display()
            ));
        }
        for entry in &entries {
            let flags = crate::compile_db::shell_words(&entry.command)
                .and_then(|words| include_flags(&words, &entry.directory))
                .map_err(|error| {
                    format!(
                        "compile database {}: the include configuration of {} cannot be read: \
                         {error}",
                        path.display(),
                        entry.file.display()
                    )
                })?;
            sets.insert(flags);
        }
    }
    Ok(sets.into_iter().collect())
}

/// Arguments that change where `#include <HEADER>` resolves other than by an
/// include directory the probe passes on, matched as the whole argument or as
/// its prefix (a joined value): a command carrying one has an include
/// configuration the probe cannot read, so it never guesses one.
pub const UNREAD_SEARCH_FLAGS: [&str; 11] = [
    "-nostdinc",
    "-nostdlibinc",
    "-nobuiltininc",
    "--sysroot",
    "-isysroot",
    "-iprefix",
    "-iwithprefix",
    "-I-",
    "--include-directory",
    "-iframework",
    "-F",
];

/// The include-path arguments of one compile command, each directory made
/// absolute against the command's directory (joined with the platform's
/// separator).
///
/// # Errors
///
/// An include flag naming no directory, or an argument of
/// [`UNREAD_SEARCH_FLAGS`].
pub fn include_flags(arguments: &[String], directory: &Path) -> Result<Vec<String>, String> {
    const FLAGS: [&str; 4] = ["-I", "-isystem", "-iquote", "-idirafter"];
    let absolute = |value: &str| {
        let path = Path::new(value);
        if path.is_absolute() {
            value.to_owned()
        } else {
            directory.join(path).display().to_string()
        }
    };
    let mut flags = Vec::new();
    let mut index = 0;
    // The first word is the compiler, never a flag.
    if !arguments.is_empty() {
        index = 1;
    }
    while index < arguments.len() {
        let argument = &arguments[index];
        if let Some(flag) = UNREAD_SEARCH_FLAGS
            .iter()
            .find(|flag| argument.starts_with(**flag))
        {
            return Err(format!(
                "`{argument}` changes the header search (`{flag}`) in a way the probe does not \
                 reproduce"
            ));
        }
        if let Some(flag) = FLAGS.iter().find(|flag| argument == *flag) {
            let value = arguments
                .get(index + 1)
                .ok_or_else(|| format!("`{flag}` names no directory"))?;
            flags.push((*flag).to_owned());
            flags.push(absolute(value));
            index += 2;
            continue;
        }
        if let Some(flag) = FLAGS
            .iter()
            .find(|flag| argument.starts_with(*flag) && argument.len() > flag.len())
        {
            flags.push((*flag).to_owned());
            flags.push(absolute(&argument[flag.len()..]));
        }
        index += 1;
    }
    Ok(flags)
}

fn macro_value(defines: &str, name: &str) -> Option<u32> {
    let line = defines
        .lines()
        .find_map(|line| line.strip_prefix(&format!("#define {name} ")))?;
    let mut text = line.trim();
    while let Some(inner) = text.strip_prefix('(').and_then(|t| t.strip_suffix(')')) {
        text = inner.trim();
    }
    let text = text.trim_end_matches(['L', 'l', 'U', 'u']);
    text.parse().ok()
}

/// Probe one library's version under the run's include configurations; a
/// configuration set that cannot be read is
/// [`LibraryVersionFailure::Unreadable`], never the default search path.
///
/// # Errors
///
/// As [`probe`], and unreadable configurations.
pub fn probe_under(
    source: &VersionSource,
    configurations: &Configurations,
) -> Result<Probed, (LibraryVersionFailure, String)> {
    match &configurations.sets {
        Ok(sets) => probe(source, sets),
        Err(detail) => Err((LibraryVersionFailure::Unreadable, detail.clone())),
    }
}

/// Probe one library's version under every configuration.
///
/// # Errors
///
/// [`LibraryVersionFailure::Unreadable`] when Clang is missing or fails, the
/// header is not found, or a macro is not a decimal integer;
/// [`LibraryVersionFailure::Ambiguous`] when two configurations resolve
/// different headers or versions. The string says which.
pub fn probe(
    source: &VersionSource,
    configurations: &[Vec<String>],
) -> Result<Probed, (LibraryVersionFailure, String)> {
    let unreadable = |detail: String| (LibraryVersionFailure::Unreadable, detail);
    let input = format!("#include <{}>\n", source.header);
    let mut found = BTreeSet::new();
    for flags in configurations {
        let run =
            |extra: &[&str]| -> Result<std::process::Output, (LibraryVersionFailure, String)> {
                let mut command = qcore_c_gateway::clang_command().map_err(unreadable)?;
                command
                    .args(["-E", "-x", "c"])
                    .args(extra)
                    .args(flags)
                    .arg("-")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                let mut child = command
                    .spawn()
                    .map_err(|error| unreadable(format!("clang: {error}")))?;
                child
                    .stdin
                    .take()
                    .expect("piped")
                    .write_all(input.as_bytes())
                    .map_err(|error| unreadable(format!("clang: {error}")))?;
                let output = child
                    .wait_with_output()
                    .map_err(|error| unreadable(format!("clang: {error}")))?;
                if output.status.success() {
                    Ok(output)
                } else {
                    Err(unreadable(format!(
                        "`#include <{}>` does not preprocess: {}",
                        source.header,
                        String::from_utf8_lossy(&output.stderr)
                            .lines()
                            .next()
                            .unwrap_or_default()
                    )))
                }
            };
        let defines = String::from_utf8_lossy(&run(&["-dM"])?.stdout).into_owned();
        // `-H` writes the include chain to stderr; the preprocessed text goes
        // to the piped stdout and is dropped (no `-o /dev/null`, which
        // Windows does not have).
        let chain = String::from_utf8_lossy(&run(&["-H"])?.stderr).into_owned();
        let header = chain
            .lines()
            .find_map(|line| line.strip_prefix(". "))
            .map(str::to_owned)
            .ok_or_else(|| unreadable(format!("no header resolved for `{}`", source.header)))?;
        let component = |name: &str| {
            macro_value(&defines, name)
                .ok_or_else(|| unreadable(format!("`{name}` is not a decimal integer macro")))
        };
        let version = LibraryVersion {
            major: component(&source.major)?,
            minor: component(&source.minor)?,
            patch: component(&source.patch)?,
        };
        found.insert((version, header));
    }
    let mut found = found.into_iter();
    let (version, header) = found
        .next()
        .ok_or_else(|| unreadable("no include configuration".into()))?;
    if let Some((other_version, other_header)) = found.next() {
        return Err((
            LibraryVersionFailure::Ambiguous,
            format!(
                "include configurations disagree: `{header}` reports {version}, `{other_header}` \
                 reports {other_version}"
            ),
        ));
    }
    let header_sha256 = fs::read(&header)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| unreadable(format!("{header}: {error}")))?;
    Ok(Probed {
        version,
        header,
        header_sha256,
        configurations: configurations.len(),
    })
}

/// One declaration's standing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    pub function: String,
    pub clause: String,
    pub library: String,
    pub audited: Option<String>,
    pub pinned: Option<String>,
    pub failure: Option<LibraryVersionFailure>,
    pub detail: String,
}

/// Decide one declaration from what was probed and pinned (pure; the
/// in-force predicate `QCore.LibraryVersionRange.inForce`).
#[must_use]
pub fn decide(
    bound: &qcore_schemas::library_version::LibraryVersionBound,
    probed: &Result<Probed, (LibraryVersionFailure, String)>,
    pinned: Option<&LockedLibraryVersion>,
) -> (Option<LibraryVersionFailure>, String) {
    let probed = match probed {
        Ok(probed) => probed,
        Err((failure, detail)) => return (Some(*failure), detail.clone()),
    };
    if !bound.admits(probed.version) {
        return (
            Some(LibraryVersionFailure::OutOfRange),
            format!(
                "the audited version {} is outside `{}`",
                probed.version,
                bound.clause()
            ),
        );
    }
    match pinned {
        None => (
            Some(LibraryVersionFailure::Changed),
            format!(
                "the lock pins no version of `{}`; the audited headers report {}",
                bound.library, probed.version
            ),
        ),
        Some(pinned) if pinned.version != probed.version.to_string() => (
            Some(LibraryVersionFailure::Changed),
            format!(
                "the audited headers report {} but the lock pins {}",
                probed.version, pinned.version
            ),
        ),
        Some(_) => (None, String::new()),
    }
}

/// The run's version evaluation: every declaration's standing and every
/// library probed, for the report.
#[derive(Clone, Debug, Default)]
pub struct Evaluation {
    pub standings: Vec<Standing>,
    pub probes: BTreeMap<String, Value>,
    pub configurations_source: Option<String>,
}

impl Evaluation {
    /// The report record, `None` when no declaration states a range.
    #[must_use]
    pub fn record(&self) -> Option<Value> {
        if self.standings.is_empty() {
            return None;
        }
        Some(json!({
            "meaning": MEANING,
            "configurations": self.configurations_source,
            "libraries": self.probes,
            "declarations": self.standings.iter().map(|standing| json!({
                "function": standing.function,
                "range": standing.clause,
                "library": standing.library,
                "audited_version": standing.audited,
                "pinned_version": standing.pinned,
                "in_force": standing.failure.is_none(),
                "reason": standing.failure.map(LibraryVersionFailure::as_str),
                "detail": standing.detail,
            })).collect::<Vec<_>>(),
        }))
    }
}

/// Evaluate every declaration in force that states a library version range,
/// and withhold each one not in force.
///
/// `crate_root` gives the crate root of a module key; `pinned` the lock's
/// recorded versions under a crate root; `databases` the run's own compile
/// databases ([`include_configurations`]).
///
/// # Errors
///
/// A malformed version-source file, or a withholding the environment
/// refuses. An unreadable compile database is not an error: it holds every
/// ranged declaration `Unknown` (unreadable).
pub fn evaluate(
    contracts: &mut ResourceContracts,
    module_of: &BTreeMap<String, String>,
    crate_root: &dyn Fn(&str) -> Option<PathBuf>,
    pinned: &dyn Fn(&Path) -> Option<BTreeMap<String, LockedLibraryVersion>>,
    databases: &ProbeDatabases,
) -> Result<Evaluation, String> {
    let ranges = contracts.library_versions().clone();
    let mut evaluation = Evaluation::default();
    if ranges.is_empty() {
        return Ok(evaluation);
    }
    let configurations = include_configurations(databases);
    evaluation.configurations_source = Some(configurations.source.clone());
    let mut probes: BTreeMap<(PathBuf, String), Result<Probed, (LibraryVersionFailure, String)>> =
        BTreeMap::new();
    for (name, bound) in ranges {
        let in_force = contracts.contract(&name).is_some()
            || contracts.relational_contract(&name).is_some()
            || contracts.retaining_contract(&name).is_some();
        if !in_force {
            continue;
        }
        let module = module_of.get(&name).cloned().unwrap_or_default();
        let root = crate_root(&module);
        let probed = match &root {
            None => Err((
                LibraryVersionFailure::Unreadable,
                format!("`{module}` lies in no crate with a {SOURCE_FILE}"),
            )),
            Some(root) => {
                let key = (root.clone(), bound.library.clone());
                if !probes.contains_key(&key) {
                    let result = match read_sources(root)? {
                        Some(sources) => match sources.get(&bound.library) {
                            Some(source) => probe_under(source, &configurations),
                            None => Err((
                                LibraryVersionFailure::Unreadable,
                                format!("{SOURCE_FILE} declares no `{}`", bound.library),
                            )),
                        },
                        None => Err((
                            LibraryVersionFailure::Unreadable,
                            format!("{} has no {SOURCE_FILE}", root.display()),
                        )),
                    };
                    evaluation.probes.insert(
                        format!("{}:{}", root.display(), bound.library),
                        match &result {
                            Ok(probed) => json!({
                                "version": probed.version.to_string(),
                                "header": probed.header,
                                "header_sha256": probed.header_sha256,
                                "configurations": probed.configurations,
                            }),
                            Err((failure, detail)) => {
                                json!({"failure": failure.as_str(), "detail": detail})
                            }
                        },
                    );
                    probes.insert(key.clone(), result);
                }
                probes[&key].clone()
            }
        };
        let locked = root
            .as_deref()
            .and_then(pinned)
            .and_then(|versions| versions.get(&bound.library).cloned());
        let (failure, detail) = decide(&bound, &probed, locked.as_ref());
        if let Some(failure) = failure {
            contracts.withhold_for_pin_mismatch(
                &name,
                module.clone(),
                PinMismatchReason::from_library_version(failure),
                None,
                detail.clone(),
            )?;
        }
        evaluation.standings.push(Standing {
            function: name,
            clause: bound.clause(),
            library: bound.library.clone(),
            audited: probed.as_ref().ok().map(|p| p.version.to_string()),
            pinned: locked.map(|l| l.version),
            failure,
            detail,
        });
    }
    Ok(evaluation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::library_version::{LibraryVersionBound, LowerBoundSource};

    fn v(text: &str) -> LibraryVersion {
        LibraryVersion::parse(text).unwrap()
    }

    fn probed(version: &str) -> Result<Probed, (LibraryVersionFailure, String)> {
        Ok(Probed {
            version: v(version),
            header: "/usr/include/openssl/opensslv.h".into(),
            header_sha256: "0".repeat(64),
            configurations: 1,
        })
    }

    fn locked(version: &str) -> LockedLibraryVersion {
        LockedLibraryVersion {
            version: version.into(),
            header: "/usr/include/openssl/opensslv.h".into(),
            header_sha256: "1".repeat(64),
        }
    }

    /// The five fixtures of the user decision: in range -> in force; below
    /// `since` -> held; past removal -> held; unreadable -> held; header
    /// version != lock version -> mismatch (Pending).
    #[test]
    fn the_decision_fixtures() {
        let bound = LibraryVersionBound {
            library: "openssl".into(),
            lower: v("1.1.0"),
            lower_source: LowerBoundSource::Since,
            removed: Some(v("4.0.0")),
        };
        let pin = locked("3.5.5");
        assert_eq!(decide(&bound, &probed("3.5.5"), Some(&pin)).0, None);
        assert_eq!(
            decide(&bound, &probed("1.0.2"), Some(&locked("1.0.2"))).0,
            Some(LibraryVersionFailure::OutOfRange)
        );
        assert_eq!(
            decide(&bound, &probed("4.0.0"), Some(&locked("4.0.0"))).0,
            Some(LibraryVersionFailure::OutOfRange)
        );
        let unreadable = Err((LibraryVersionFailure::Unreadable, "no header".into()));
        assert_eq!(
            decide(&bound, &unreadable, Some(&pin)).0,
            Some(LibraryVersionFailure::Unreadable)
        );
        let ambiguous = Err((LibraryVersionFailure::Ambiguous, "two".into()));
        assert_eq!(
            decide(&bound, &ambiguous, Some(&pin)).0,
            Some(LibraryVersionFailure::Ambiguous)
        );
        assert_eq!(
            decide(&bound, &probed("3.5.4"), Some(&pin)).0,
            Some(LibraryVersionFailure::Changed)
        );
        assert_eq!(
            decide(&bound, &probed("3.5.5"), None).0,
            Some(LibraryVersionFailure::Changed)
        );
        // A different header digest at the pinned version is provenance, not
        // a mismatch.
        assert_eq!(decide(&bound, &probed("3.5.5"), Some(&pin)).1, "");
        for (failure, pending) in [
            (LibraryVersionFailure::Changed, true),
            (LibraryVersionFailure::OutOfRange, false),
            (LibraryVersionFailure::Unreadable, false),
            (LibraryVersionFailure::Ambiguous, false),
        ] {
            assert_eq!(
                PinMismatchReason::from_library_version(failure).is_human_resolvable(),
                pending
            );
        }
    }

    #[test]
    fn macros_and_include_flags_read_as_written() {
        let defines = "#define A 3\n#define B (5)\n#define C 5L\n#define D (1<<4)\n";
        assert_eq!(macro_value(defines, "A"), Some(3));
        assert_eq!(macro_value(defines, "B"), Some(5));
        assert_eq!(macro_value(defines, "C"), Some(5));
        assert_eq!(macro_value(defines, "D"), None);
        assert_eq!(macro_value(defines, "E"), None);
        let flags = include_flags(
            &[
                "cc".into(),
                "-Iinc".into(),
                "-isystem".into(),
                "/opt/ssl".into(),
                "-DX".into(),
                "-c".into(),
            ],
            Path::new("/b"),
        )
        .unwrap();
        // A relative directory is joined with the platform's separator.
        let joined = Path::new("/b").join("inc").display().to_string();
        assert_eq!(flags, ["-I", joined.as_str(), "-isystem", "/opt/ssl"]);
    }

    /// Card LIBRARY-VERSION-PROBE-ROUTE-DATABASE: a configuration the probe
    /// cannot reproduce is refused, never read as the default search path.
    #[test]
    fn an_include_configuration_is_read_total_or_refused() {
        let words =
            |text: &str| -> Vec<String> { text.split_whitespace().map(str::to_owned).collect() };
        let directory = Path::new("/b");
        // The compiler word is never a flag, whatever its spelling.
        assert_eq!(
            include_flags(&words("-F/usr/bin/cc -c x.c"), directory).unwrap(),
            Vec::<String>::new()
        );
        for refused in [
            "cc -I",
            "cc -c x.c -isystem",
            "cc -nostdinc -Iinc",
            "cc --sysroot=/s -c x.c",
            "cc --sysroot /s -c x.c",
            "cc -isysroot /s -c x.c",
            "cc -iprefix /p -iwithprefix inc",
            "cc -I- -Iinc",
            "cc --include-directory=inc",
            "cc -iframework /f",
            "cc -F/f",
        ] {
            assert!(
                include_flags(&words(refused), directory).is_err(),
                "{refused}"
            );
        }
    }

    #[test]
    fn a_route_database_never_falls_back_to_the_default_search_path() {
        // The run's own databases are named in the record; with none the
        // record is the default search path, exactly as before.
        if std::env::var_os(PROBE_DATABASE_VARIABLE).is_some() {
            return;
        }
        let none = include_configurations(&ProbeDatabases::None);
        assert_eq!(none.source, DEFAULT_SEARCH_PATH);
        assert_eq!(none.sets, Ok(vec![Vec::new()]));
        let root = std::env::temp_dir().join(format!("qcore-libver-db-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let database = root.join("compile_commands.json");
        let directory = root.display().to_string();
        let write = |value: Value| fs::write(&database, value.to_string()).unwrap();
        let read = || include_configurations(&ProbeDatabases::route(&database));
        // Unreadable, empty, a record the routes' reader refuses (a relative
        // directory, no command), and a configuration the probe cannot
        // reproduce: each is an error, held as unreadable.
        assert!(read().sets.is_err());
        for bad in [
            json!([]),
            json!([{"directory": "relative", "file": "a.c", "arguments": ["cc", "-c", "a.c"]}]),
            json!([{"directory": directory, "file": "a.c"}]),
            json!([{"directory": directory, "file": "a.c",
                    "arguments": ["cc", "--sysroot=/s", "-c", "a.c"]}]),
            json!([{"directory": directory, "file": "a.c", "arguments": ["cc", "-c", "a.c"]},
                   {"directory": directory, "file": "b.c", "arguments": ["cc", "-c", "b.c", "-I"]}]),
        ] {
            write(bad.clone());
            let configurations = read();
            assert!(configurations.sets.is_err(), "{bad}");
            let source = VersionSource {
                header: "fakelib/fakelibv.h".into(),
                major: "A".into(),
                minor: "B".into(),
                patch: "C".into(),
            };
            assert_eq!(
                probe_under(&source, &configurations).unwrap_err().0,
                LibraryVersionFailure::Unreadable
            );
        }
        // A readable database: its distinct configurations, a command with no
        // include flag among them (that unit is built on the default search
        // path), the record naming the database.
        write(json!([
            {"directory": directory, "file": "a.c", "command": "cc -Ibundled -c a.c"},
            {"directory": directory, "file": "b.c", "arguments": ["cc", "-Ibundled", "-c", "b.c"]},
            {"directory": directory, "file": "c.c", "arguments": ["cc", "-c", "c.c"]},
        ]));
        let configurations = read();
        assert_eq!(
            configurations.source,
            format!("compile-database:{}", database.display())
        );
        let bundled = root.join("bundled").display().to_string();
        assert_eq!(
            configurations.sets.unwrap(),
            vec![Vec::new(), vec!["-I".to_owned(), bundled]]
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_version_source_is_read_total_or_refuse() {
        let root = std::env::temp_dir().join(format!("qcore-libver-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        assert_eq!(read_sources(&root).unwrap(), None);
        fs::write(
            root.join(SOURCE_FILE),
            r#"{"schema_version":1,"libraries":{"openssl":{"header":"openssl/opensslv.h","major":"OPENSSL_VERSION_MAJOR","minor":"OPENSSL_VERSION_MINOR","patch":"OPENSSL_VERSION_PATCH"}}}"#,
        )
        .unwrap();
        assert_eq!(read_sources(&root).unwrap().unwrap().len(), 1);
        for bad in [
            r#"{"schema_version":2,"libraries":{}}"#,
            r#"{"schema_version":1,"libraries":{"x":{"header":"/abs.h","major":"A","minor":"B","patch":"C"}}}"#,
            r#"{"schema_version":1,"libraries":{"x":{"header":"../x.h","major":"A","minor":"B","patch":"C"}}}"#,
            r#"{"schema_version":1,"libraries":{"x":{"header":"x.h","major":"1A","minor":"B","patch":"C"}}}"#,
            r#"{"schema_version":1,"libraries":{},"extra":1}"#,
        ] {
            fs::write(root.join(SOURCE_FILE), bad).unwrap();
            assert!(read_sources(&root).is_err(), "{bad}");
        }
        fs::remove_dir_all(&root).unwrap();
    }
}
