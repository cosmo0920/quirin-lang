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

//! Digests, file trees, JSON records and child processes.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

/// SHA-256 of a file's bytes.
///
/// # Errors
///
/// An unreadable file.
pub fn sha_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(qcore_schemas::sha256_hex(&bytes))
}

/// Whether a relative path lies under version-control metadata (a `.git`
/// directory or file at any depth). The audit's own `git` refreshes a fresh
/// clone's index during a route (MEASURE-SNAPSHOT-HAZARDS item 3), so VCS
/// metadata is never part of an input identity.
#[must_use]
pub fn is_vcs_metadata(relative: &Path) -> bool {
    relative.components().any(|part| part.as_os_str() == ".git")
}

/// Every regular file under `root` (not following symbolic links), relative,
/// sorted, skipping VCS metadata and any top-level directory in `skip_top`.
///
/// # Errors
///
/// An unreadable directory.
pub fn tree_files(root: &Path, skip_top: &[&str]) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        let directory = root.join(&relative);
        let entries = fs::read_dir(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
            let name = entry.file_name();
            let child = relative.join(&name);
            if is_vcs_metadata(&child) {
                continue;
            }
            if relative.as_os_str().is_empty()
                && skip_top.iter().any(|skip| name.as_os_str() == *skip)
            {
                continue;
            }
            let kind = entry
                .file_type()
                .map_err(|error| format!("{}: {error}", child.display()))?;
            if kind.is_dir() {
                pending.push(child);
            } else if kind.is_file() {
                found.push(child);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// The identity of a tree: relative path to SHA-256, VCS metadata excluded.
///
/// # Errors
///
/// An unreadable file or directory.
pub fn tree_identity(root: &Path, skip_top: &[&str]) -> Result<BTreeMap<String, String>, String> {
    tree_files(root, skip_top)?
        .into_iter()
        .map(|relative| {
            let digest = sha_file(&root.join(&relative))?;
            Ok((relative.display().to_string(), digest))
        })
        .collect()
}

/// One digest over a tree identity: SHA-256 of `path NUL sha256 LF` lines in
/// path order. The same formula reads a git tree ([`crate::baseline`]).
#[must_use]
pub fn identity_digest(identity: &BTreeMap<String, String>) -> String {
    let mut text = Vec::new();
    for (path, digest) in identity {
        text.extend_from_slice(path.as_bytes());
        text.push(0);
        text.extend_from_slice(digest.as_bytes());
        text.push(b'\n');
    }
    qcore_schemas::sha256_hex(&text)
}

/// Read and parse a JSON file.
///
/// # Errors
///
/// An unreadable or malformed file.
pub fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
    deserializer.disable_recursion_limit();
    let value = serde::Deserialize::deserialize(&mut deserializer)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    deserializer
        .end()
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(value)
}

/// Write a JSON record: two-space indentation, sorted keys (serde_json's
/// ordered map), a final newline.
///
/// # Errors
///
/// An unwritable file.
pub fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let mut text = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    text.push('\n');
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(path, text).map_err(|error| format!("{}: {error}", path.display()))
}

/// The current UTC time, as `date -u +%Y-%m-%dT%H:%M:%SZ` prints it.
#[must_use]
pub fn now() -> String {
    Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map_or_else(
            || "unrecorded".to_owned(),
            |output| String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        )
}

/// Run a command to completion, capturing its standard output; standard
/// error goes to `log` when given.
///
/// # Errors
///
/// A spawn failure or a nonzero exit, with the command and its stderr.
pub fn capture(
    program: &str,
    arguments: &[&str],
    directory: Option<&Path>,
) -> Result<String, String> {
    let mut command = Command::new(program);
    command.args(arguments).stdin(Stdio::null());
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    let output = command
        .output()
        .map_err(|error| format!("{program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "`{program} {}` failed ({}): {}",
            arguments.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("{program}: {error}"))
}

/// Run a command with its standard output and error appended to `log`.
/// Returns its exit code (`-1` for a signal).
///
/// # Errors
///
/// A spawn failure or an unopenable log.
pub fn logged(command: &mut Command, log: &Path) -> Result<i32, String> {
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .map_err(|error| format!("{}: {error}", log.display()))?;
    let error_file = file
        .try_clone()
        .map_err(|error| format!("{}: {error}", log.display()))?;
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(error_file))
        .status()
        .map_err(|error| format!("{:?}: {error}", command.get_program()))?;
    Ok(status.code().unwrap_or(-1))
}

/// Whether `text` is 64 lowercase hex digits.
#[must_use]
pub fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Whether `text` is a full 40-digit git object name.
#[must_use]
pub fn is_full_revision(text: &str) -> bool {
    text.len() == 40
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The longest common path suffix of a spelled path and its canonical form,
/// returned as the two differing prefixes `(spelled, canonical)`, or `None`
/// when the spelling is already canonical.
#[must_use]
pub fn differing_prefixes(spelled: &Path, canonical: &Path) -> Option<(PathBuf, PathBuf)> {
    if spelled == canonical {
        return None;
    }
    let left: Vec<_> = spelled.components().collect();
    let right: Vec<_> = canonical.components().collect();
    let mut shared = 0;
    while shared < left.len()
        && shared < right.len()
        && left[left.len() - 1 - shared] == right[right.len() - 1 - shared]
    {
        shared += 1;
    }
    let spelled_prefix: PathBuf = left[..left.len() - shared].iter().collect();
    let canonical_prefix: PathBuf = right[..right.len() - shared].iter().collect();
    Some((spelled_prefix, canonical_prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vcs_metadata_is_any_git_component() {
        assert!(is_vcs_metadata(Path::new(
            "third_party/libevent/.git/index"
        )));
        assert!(is_vcs_metadata(Path::new(".git")));
        assert!(!is_vcs_metadata(Path::new("src/git.c")));
    }

    #[test]
    fn differing_prefixes_strip_the_shared_suffix() {
        assert_eq!(
            differing_prefixes(
                Path::new("/home/u/old/build/cycle19/x"),
                Path::new("/media/d/new/build/cycle19/x")
            ),
            Some((PathBuf::from("/home/u/old"), PathBuf::from("/media/d/new")))
        );
        assert_eq!(
            differing_prefixes(Path::new("/a/b"), Path::new("/a/b")),
            None
        );
    }

    #[test]
    fn the_identity_digest_is_order_and_content_sensitive() {
        let mut one = BTreeMap::new();
        one.insert("a".to_owned(), "1".repeat(64));
        let mut two = one.clone();
        two.insert("b".to_owned(), "2".repeat(64));
        assert_ne!(identity_digest(&one), identity_digest(&two));
        assert_eq!(identity_digest(&one), identity_digest(&one.clone()));
    }
}
