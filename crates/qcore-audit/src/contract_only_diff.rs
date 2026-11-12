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

//! Card **MEASURE-LOOKUP-RECORD-REUSE** (R8 option A): accepting the base
//! side's analyzer build for a diff confined to contract data.
//!
//! A contract-only card leaves every analyzer source unchanged, so its tip
//! can be measured by the **base** binary run over the tip's contract data:
//! that binary's identity (SHA-256, build commit, and the build commit every
//! published row's evidence version names) is then the base side's, and a
//! base-side group checkpoint can stand for a tip-side group whenever the
//! rest of its identity matches. [`classify`] decides whether a diff allows
//! it, fail-closed:
//!
//! * every path `git diff --name-only --no-renames BASE TIP` names is
//!   contract data: under `contract/`, under a provider's `contracts/`
//!   directory, or one of a provider's contract locks
//!   (`contracts.lock`, `bounds-contracts.lock`, `outer-contracts.lock`,
//!   `contract-inventory.lock`), and is not Rust source, a manifest or a
//!   build script;
//! * no such path is compiled into an analyzer: no `include_str!`,
//!   `include_bytes!` or `include!` of the base tree's non-test Rust sources
//!   names it (resolved relative to the including file, or to its crate
//!   for `concat!(env!("CARGO_MANIFEST_DIR"), ..)`), and an include whose
//!   argument is neither form refuses the whole classification, because what
//!   it compiles in cannot be enumerated.
//!
//! [`Acceptance::verify_run`] then checks the run that claims it: the running
//! binary's build commit is the base commit, the repository's `HEAD` is the
//! tip with its contract data unmodified, and the contract inputs the run
//! reads (manifest, locks, source profile, source-contract configuration)
//! are the repository's. The checked acceptance is recorded in the run
//! record and in every checkpoint decision; it changes no identity.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// A checked acceptance of the base side's analyzer for a contract-only diff.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acceptance {
    /// The tip checkout the run reads its contract data from.
    pub repository: String,
    /// The base commit (the analyzer's build commit).
    pub base: String,
    /// The tip commit (the repository's `HEAD`).
    pub tip: String,
    /// Every path the diff names, all contract data.
    pub paths: Vec<String>,
    /// The base tree's compiled-in paths the classification checked against.
    pub compiled_in: usize,
}

impl Acceptance {
    /// The decision every checkpoint record carries.
    #[must_use]
    pub fn decision(&self) -> Value {
        json!({
            "accepted_from": "base",
            "reason": "the diff is confined to contract data none of which is compiled into an analyzer",
            "base": self.base,
            "tip": self.tip,
            "repository": self.repository,
            "paths": self.paths,
            "compiled_in_paths_checked": self.compiled_in,
        })
    }

    /// Check the run that claims this acceptance: `build_commit` is the
    /// running analyzer's, `inputs` the contract inputs it reads.
    ///
    /// # Errors
    ///
    /// The first failed condition.
    pub fn verify_run(&self, build_commit: Option<&str>, inputs: &[&Path]) -> Result<(), String> {
        let repository = Path::new(&self.repository);
        let Some(commit) = build_commit else {
            return Err("the analyzer records no build commit, so it cannot be the base side's".into());
        };
        let commit = rev_parse(repository, commit)?;
        if commit != self.base {
            return Err(format!(
                "the analyzer was built at {commit}, not at the base {}",
                self.base
            ));
        }
        let head = rev_parse(repository, "HEAD")?;
        if head != self.tip {
            return Err(format!(
                "{} is at {head}, not at the accepted tip {}",
                self.repository, self.tip
            ));
        }
        let mut status = vec!["status", "--porcelain", "--untracked-files=all", "--"];
        status.extend(CONTRACT_PATHSPECS);
        let modified = git(repository, &status)?;
        if !modified.trim().is_empty() {
            return Err(format!(
                "{} has uncommitted contract data: {}",
                self.repository,
                modified.lines().next().unwrap_or_default()
            ));
        }
        for input in inputs {
            let canonical = input
                .canonicalize()
                .map_err(|error| format!("{}: {error}", input.display()))?;
            if !canonical.starts_with(repository) {
                return Err(format!(
                    "contract input {} is not read from the accepted tip checkout {}",
                    canonical.display(),
                    self.repository
                ));
            }
        }
        Ok(())
    }
}

/// The git pathspecs of contract data, for the cleanliness check.
const CONTRACT_PATHSPECS: [&str; 6] = [
    "contract",
    ":(glob)providers/*/contracts/**",
    ":(glob)providers/*/contracts.lock",
    ":(glob)providers/*/bounds-contracts.lock",
    ":(glob)providers/*/outer-contracts.lock",
    ":(glob)providers/*/contract-inventory.lock",
];

const LOCKS: [&str; 4] = [
    "contracts.lock",
    "bounds-contracts.lock",
    "outer-contracts.lock",
    "contract-inventory.lock",
];

/// Whether `path` (repository-relative, `/`-separated) is contract data.
#[must_use]
pub fn is_contract_data(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    if name.ends_with(".rs") || name == "Cargo.toml" || name == "Cargo.lock" || name == "build.rs"
    {
        return false;
    }
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.iter().any(|part| part.is_empty() || *part == "." || *part == "..") {
        return false;
    }
    match parts.as_slice() {
        ["contract", _, ..] => true,
        ["providers", _, "contracts", _, ..] => true,
        ["providers", _, lock] => LOCKS.contains(lock),
        _ => false,
    }
}

/// Whether a Rust source is test-only by its path (an integration test,
/// benchmark or example, or a `tests.rs`/`tests/` module), so not compiled
/// into an analyzer binary.
fn test_only(path: &str) -> bool {
    path.split('/')
        .any(|part| matches!(part, "tests" | "benches" | "examples" | "tests.rs"))
}

fn git(repository: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .output()
        .map_err(|error| format!("could not run git: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| error.to_string())
}

fn rev_parse(repository: &Path, revision: &str) -> Result<String, String> {
    Ok(git(
        repository,
        &["rev-parse", "--verify", &format!("{revision}^{{commit}}")],
    )?
    .trim()
    .to_owned())
}

/// Normalize `base/relative` lexically (no filesystem access).
fn join_normalized(base: &str, relative: &str) -> Option<String> {
    let mut parts = base
        .split('/')
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?.to_owned()),
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(parts.join("/"))
}

/// The repository-relative paths the include macros of `text` (the file
/// `path`, in crate `crate_root`) compile in.
///
/// # Errors
///
/// An include whose argument is neither a string literal nor
/// `concat!(env!("CARGO_MANIFEST_DIR"), "literal")`.
pub fn includes_of(path: &str, crate_root: &str, text: &str) -> Result<Vec<String>, String> {
    let directory = path.rsplit_once('/').map_or("", |(directory, _)| directory);
    let mut out = Vec::new();
    let mut rest = text;
    let mut offset = 0;
    while let Some(found) = rest.find("include") {
        let after = &rest[found + "include".len()..];
        let suffix = ["_str!", "_bytes!", "!"]
            .into_iter()
            .find(|suffix| after.starts_with(suffix));
        let Some(suffix) = suffix else {
            offset += found + 1;
            rest = &text[offset..];
            continue;
        };
        // An identifier that merely ends in `include` is not the macro.
        let before = text[..offset + found].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            offset += found + 1;
            rest = &text[offset..];
            continue;
        }
        let argument = after[suffix.len()..].trim_start();
        let Some(argument) = argument.strip_prefix('(') else {
            offset += found + 1;
            rest = &text[offset..];
            continue;
        };
        let argument = argument.trim_start();
        let (anchor, literal) = if let Some(literal) = argument.strip_prefix('"') {
            (directory, literal)
        } else if let Some(concat) = argument.strip_prefix("concat!") {
            let concat = concat.trim_start().strip_prefix('(').unwrap_or("").trim_start();
            let Some(tail) = concat
                .strip_prefix("env!")
                .map(str::trim_start)
                .and_then(|tail| tail.strip_prefix('('))
                .map(str::trim_start)
                .and_then(|tail| tail.strip_prefix("\"CARGO_MANIFEST_DIR\""))
                .map(str::trim_start)
                .and_then(|tail| tail.strip_prefix(')'))
                .map(str::trim_start)
                .and_then(|tail| tail.strip_prefix(','))
                .map(str::trim_start)
                .and_then(|tail| tail.strip_prefix('"'))
            else {
                return Err(format!(
                    "{path}: an include whose argument cannot be enumerated"
                ));
            };
            (crate_root, tail)
        } else {
            return Err(format!(
                "{path}: an include whose argument cannot be enumerated"
            ));
        };
        let Some(end) = literal.find('"') else {
            return Err(format!("{path}: an unterminated include literal"));
        };
        let spelled = literal[..end].trim_start_matches('/');
        if literal[..end].contains('\\') {
            return Err(format!("{path}: an include literal with an escape"));
        }
        let resolved = join_normalized(anchor, spelled)
            .ok_or_else(|| format!("{path}: include `{spelled}` leaves the repository"))?;
        out.push(resolved);
        offset += found + 1;
        rest = &text[offset..];
    }
    Ok(out)
}

/// Every repository-relative path the base tree's non-test Rust sources
/// compile in.
///
/// # Errors
///
/// A git failure or an include that cannot be enumerated.
pub fn compiled_in(repository: &Path, base: &str) -> Result<BTreeSet<String>, String> {
    let listed = git(
        repository,
        &[
            "grep",
            "-l",
            "-E",
            "include(_str|_bytes)?!",
            base,
            "--",
            "*.rs",
            ":(exclude)third_party",
        ],
    )
    .or_else(|error| {
        // `git grep -l` exits 1 when nothing matches.
        if error.contains("failed: ") && error.ends_with("failed: ") {
            Ok(String::new())
        } else {
            Err(error)
        }
    })?;
    let prefix = format!("{base}:");
    let files = listed
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .filter(|path| !test_only(path))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not run git cat-file: {error}"))?;
    let mut stdin = child.stdin.take().ok_or("git cat-file has no input")?;
    let requests = files
        .iter()
        .map(|path| format!("{base}:{path}\n"))
        .collect::<String>();
    let writer = std::thread::spawn(move || stdin.write_all(requests.as_bytes()));
    let mut reader = BufReader::new(child.stdout.take().ok_or("git cat-file has no output")?);
    let mut out = BTreeSet::new();
    for path in &files {
        let mut header = String::new();
        reader
            .read_line(&mut header)
            .map_err(|error| error.to_string())?;
        let size = header
            .split_whitespace()
            .nth(2)
            .and_then(|size| size.parse::<usize>().ok())
            .ok_or_else(|| format!("git cat-file: unexpected header for {path}: {header}"))?;
        let mut body = vec![0; size + 1];
        reader
            .read_exact(&mut body)
            .map_err(|error| error.to_string())?;
        body.pop();
        let text = String::from_utf8(body).map_err(|error| format!("{path}: {error}"))?;
        let crate_root = crate_root_of(path);
        out.extend(includes_of(path, &crate_root, &text)?);
    }
    writer
        .join()
        .map_err(|_| "git cat-file writer panicked".to_owned())?
        .map_err(|error| error.to_string())?;
    let status = child.wait().map_err(|error| error.to_string())?;
    if !status.success() {
        return Err("git cat-file failed".into());
    }
    Ok(out)
}

/// The directory of the crate a source belongs to: the path before its
/// `src/` component (the manifest directory of a conventional layout).
fn crate_root_of(path: &str) -> String {
    let parts = path.split('/').collect::<Vec<_>>();
    match parts.iter().rposition(|part| *part == "src") {
        Some(index) => parts[..index].join("/"),
        None => path.rsplit_once('/').map_or("", |(d, _)| d).to_owned(),
    }
}

/// Classify `base..tip` in `repository`.
///
/// # Errors
///
/// A path that is not contract data, a contract path compiled into an
/// analyzer, an include that cannot be enumerated, or a git failure.
pub fn classify(repository: &Path, base: &str, tip: &str) -> Result<Acceptance, String> {
    let repository: PathBuf = repository
        .canonicalize()
        .map_err(|error| format!("{}: {error}", repository.display()))?;
    let base = rev_parse(&repository, base)?;
    let tip = rev_parse(&repository, tip)?;
    let diff = git(
        &repository,
        &["diff", "--name-only", "--no-renames", &base, &tip],
    )?;
    let paths = diff.lines().map(str::to_owned).collect::<Vec<_>>();
    if let Some(path) = paths.iter().find(|path| !is_contract_data(path)) {
        return Err(format!(
            "the diff is not confined to contract data: `{path}` (the analyzer build is not accepted from the base side)"
        ));
    }
    let compiled = compiled_in(&repository, &base)?;
    if let Some(path) = paths.iter().find(|path| compiled.contains(*path)) {
        return Err(format!(
            "`{path}` is compiled into an analyzer, so the base binary does not read the tip's version"
        ));
    }
    Ok(Acceptance {
        repository: repository.display().to_string(),
        base,
        tip,
        paths,
        compiled_in: compiled.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_data_is_exactly_the_contract_trees_and_provider_locks() {
        for path in [
            "contract/profiles/fluent-bit/audit.json",
            "providers/quirin-provider-libc/contracts/libc/memory.qv",
            "providers/qcore-provider-chunkio/contracts/chunkio/contracts.lock",
            "providers/qcore-provider-cmetrics/contracts.lock",
            "providers/qcore-provider-cmetrics/bounds-contracts.lock",
            "providers/quirin-provider-libc/contract-inventory.lock",
            "providers/quirin-provider-libc/outer-contracts.lock",
        ] {
            assert!(is_contract_data(path), "{path}");
        }
        for path in [
            "crates/qcore-audit/src/lib.rs",
            "providers/Cargo.lock",
            "providers/pin-lock-window.json",
            "providers/contract-load-path.json",
            "providers/quirin-provider-libc/src/lib.rs",
            "providers/quirin-provider-libc/Cargo.toml",
            "providers/quirin-provider-libc/contracts/build.rs",
            "contract/x/../../crates/a.qv",
            "contract",
            "compiler/tests/fixtures/contracts/alloc_acquire.qv",
        ] {
            assert!(!is_contract_data(path), "{path}");
        }
    }

    #[test]
    fn includes_resolve_relative_to_the_file_or_the_crate_and_refuse_the_rest() {
        let text = r#"
            const A: &str = include_str!("../../../providers/p/contracts/a.qv");
            const B: &[u8] = include_bytes!(
                "fixtures/b.bin"
            );
            const C: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/c.json"));
            fn my_include(x: u8) {}
        "#;
        assert_eq!(
            includes_of("crates/k/src/m.rs", "crates/k", text).unwrap(),
            vec![
                "providers/p/contracts/a.qv".to_owned(),
                "crates/k/src/fixtures/b.bin".to_owned(),
                "crates/k/data/c.json".to_owned(),
            ]
        );
        assert!(includes_of("crates/k/src/m.rs", "crates/k", "include_str!(PATH)").is_err());
        assert!(
            includes_of(
                "crates/k/src/m.rs",
                "crates/k",
                r#"include_str!(concat!("a", "b"))"#
            )
            .is_err()
        );
        assert!(includes_of("m.rs", "", r#"include_str!("../../x")"#).is_err());
        assert!(test_only("crates/k/tests/t.rs"));
        assert!(test_only("crates/k/src/route/checkpoints/tests.rs"));
        assert!(!test_only("crates/k/src/lib.rs"));
        assert_eq!(crate_root_of("crates/k/src/a/b.rs"), "crates/k");
    }
}
