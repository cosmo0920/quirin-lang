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

//! The process, path and text operations of the libevent preparation, each
//! where it reaches a prepared byte: running a command for its stripped
//! output or into a merged log, program lookup on `PATH`, path resolution,
//! the final `.`-suffix of a name, strict UTF-8 decoding with universal
//! newlines, and the CMake cache reader. `quirin-audit prepare` has the same
//! operations
//! (`crates/qcore-audit/src/prepare.rs`, card W44-QUIRIN-AUDIT-PREPARE).

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

pub fn text(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| format!("non-UTF-8 path {}", path.display()))
}

pub fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read(path)
        .map_err(|error| format!("{}: {error}", path.display()))
        .and_then(decode)
}

pub fn write_text(path: &Path, contents: &str) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

/// SHA-256 of the file's bytes.
pub fn digest(path: &Path) -> Result<String, String> {
    std::fs::read(path)
        .map(|bytes| qcore_schemas::sha256_hex(&bytes))
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// Strict UTF-8 with universal newlines (`\r\n` and `\r` read as `\n`).
pub fn decode(bytes: Vec<u8>) -> Result<String, String> {
    let text = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    Ok(if text.contains('\r') {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text
    })
}

/// Trim Unicode whitespace and the ASCII information separators U+001C–U+001F
/// from both ends.
pub fn strip_whitespace(text: &str) -> &str {
    text.trim_matches(|character: char| {
        character.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&character)
    })
}

/// Run a command to completion and return its standard output, decoded and
/// stripped; a nonzero exit is an error carrying its standard error. Standard
/// input is closed; none of the commands reads it.
pub fn capture(program: &str, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("{program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "command {:?} failed ({}): {}",
            std::iter::once(program)
                .chain(arguments.iter().copied())
                .collect::<Vec<_>>(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    decode(output.stdout).map(|output| strip_whitespace(&output).to_owned())
}

/// Run a command with standard output and error through one pipe into
/// `path`, written whatever the exit status; a nonzero exit is then an error.
pub fn logged(command: &[String], path: &Path) -> Result<(), String> {
    let (mut reader, writer) = std::io::pipe().map_err(|error| error.to_string())?;
    let error_writer = writer.try_clone().map_err(|error| error.to_string())?;
    let mut child = {
        // The command owns both write ends; it is dropped once the child holds
        // them, so the read below ends with the child's output.
        let mut process = Command::new(&command[0]);
        process
            .args(&command[1..])
            .stdin(Stdio::null())
            .stdout(writer)
            .stderr(error_writer);
        process
            .spawn()
            .map_err(|error| format!("{}: {error}", command[0]))?
    };
    let mut bytes = Vec::new();
    let read = reader.read_to_end(&mut bytes);
    let status = child.wait().map_err(|error| error.to_string())?;
    read.map_err(|error| error.to_string())?;
    write_text(path, &decode(bytes)?)?;
    if !status.success() {
        let code = status
            .code()
            .map_or_else(|| status.to_string(), |code| code.to_string());
        return Err(format!("command failed ({code}); see {}", path.display()));
    }
    Ok(())
}

/// Resolve a path without requiring it to exist: symbolic links resolved for
/// the components that exist, the rest applied lexically, a relative path
/// against `base`.
pub fn resolve(path: &Path, base: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    };
    let mut resolved = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => resolved.push(component),
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            Component::Normal(name) => {
                let candidate = resolved.join(name);
                resolved = candidate.canonicalize().unwrap_or(candidate);
            }
        }
    }
    resolved
}

/// The final component's last `.`-suffix, empty for a name that starts with
/// its only dot or ends with a dot.
#[must_use]
pub fn suffix(path: &Path) -> &str {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return "";
    };
    match name.rfind('.') {
        Some(index) if index > 0 && index < name.len() - 1 => &name[index..],
        _ => "",
    }
}

/// Find a program: a name with a separator is checked as given; otherwise the
/// first executable, non-directory entry of `PATH` (duplicates skipped),
/// spelled as the directory joined with the name.
pub fn which(name: &str) -> Option<String> {
    let usable = |path: &Path| path.exists() && executable(path) && !path.is_dir();
    if name.chars().any(std::path::is_separator) {
        return usable(Path::new(name)).then(|| name.to_owned());
    }
    let search: OsString = std::env::var_os("PATH").unwrap_or_else(|| "/bin:/usr/bin".into());
    if search.is_empty() {
        return None;
    }
    let mut seen = BTreeSet::new();
    for directory in std::env::split_paths(&search) {
        if !seen.insert(directory.clone()) {
            continue;
        }
        let candidate = directory.join(name);
        if usable(&candidate) {
            return candidate.to_str().map(str::to_owned);
        }
    }
    None
}

#[cfg(unix)]
fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable(path: &Path) -> bool {
    path.is_file()
}

/// The `KEY=VALUE` entries of a CMake cache, comments (`#`, `//`) skipped;
/// the last value of a repeated key wins.
#[must_use]
pub fn cmake_cache(text: &str) -> Vec<(String, String)> {
    let mut values: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match values.iter_mut().find(|(name, _)| name == key) {
            Some((_, slot)) => value.clone_into(slot),
            None => values.push((key.to_owned(), value.to_owned())),
        }
    }
    values
}

/// The value of `key`, if present.
#[must_use]
pub fn lookup<'a>(values: &'a [(String, String)], key: &str) -> Option<&'a str> {
    values
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_is_the_final_dot_suffix() {
        assert_eq!(suffix(Path::new("/a/buffer.c")), ".c");
        assert_eq!(suffix(Path::new("/a/.c")), "");
        assert_eq!(suffix(Path::new("/a/x.")), "");
        assert_eq!(suffix(Path::new("/a/x.tar.c")), ".c");
        assert_eq!(suffix(Path::new("/a/x")), "");
    }

    #[test]
    fn the_cache_keeps_the_last_value_and_skips_comments() {
        let values = cmake_cache("# c\n//d=e\nA:STRING=1\nB=2\nA:STRING=3\nnoequals\n");
        assert_eq!(lookup(&values, "A:STRING"), Some("3"));
        assert_eq!(lookup(&values, "B"), Some("2"));
        assert_eq!(lookup(&values, "//d"), None);
    }

    #[test]
    fn strip_and_decode_normalise_text() {
        assert_eq!(
            strip_whitespace("\u{1c} clang version 21\n"),
            "clang version 21"
        );
        assert_eq!(decode(b"a\r\nb\rc".to_vec()).unwrap(), "a\nb\nc");
    }
}
