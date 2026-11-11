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

//! GNU `ld` scripts used as libraries: `libm.a` and `libm.so` on glibc are
//! text files such as `GROUP ( /usr/lib/x86_64-linux-gnu/libm-2.43.a
//! /usr/lib/x86_64-linux-gnu/libmvec.a )`.
//!
//! The linker follows the script, so the manifest pins the script **and every
//! file it names**. The subset read is exactly what such a library script
//! uses: `/* … */` comments, `OUTPUT_FORMAT(…)` (recorded, ignored), `GROUP(…)`
//! and `INPUT(…)` with file operands, and `AS_NEEDED(…)` inside them.
//!
//! Refused rather than guessed, each by name: any other command (`SEARCH_DIR`,
//! `INCLUDE`, `SECTIONS`, …), an `-lNAME` operand (it names a search, not a
//! file), and a relative path (`ld` resolves it against the working directory
//! and then the search path, which the manifest cannot pin).

use super::bytes::ReadError;

/// One file a script names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptInput {
    /// The path exactly as the script spells it.
    pub path: String,
    /// Inside `AS_NEEDED ( … )`: linked only if it satisfies a reference.
    pub as_needed: bool,
}

/// Whether `data` looks like an `ld` script rather than a binary artifact:
/// UTF-8 text whose first command is a comment or one of the commands read
/// here.
#[must_use]
pub fn is_ld_script(data: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(data) else {
        return false;
    };
    let Ok(stripped) = strip_comments(text) else {
        return false;
    };
    let first = stripped.trim_start();
    ["GROUP", "INPUT", "OUTPUT_FORMAT"]
        .iter()
        .any(|command| first.starts_with(command))
}

fn strip_comments(text: &str) -> Result<String, ReadError> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        out.push(' ');
        let after = &rest[start + 2..];
        let end = after
            .find("*/")
            .ok_or_else(|| ReadError::new("ld script", "an unterminated `/*` comment"))?;
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        match ch {
            '(' | ')' | ',' => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
                if ch != ',' {
                    out.push(ch.to_string());
                }
            }
            ch if ch.is_whitespace() => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            ch => current.push(ch),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Read a library script: the files it names, in order.
///
/// # Errors
///
/// A [`ReadError`] for text that is not UTF-8, an unterminated comment or
/// parenthesis, a command outside the subset, an `-l` operand, a relative
/// path, or a script that names no file.
pub fn read(data: &[u8]) -> Result<Vec<ScriptInput>, ReadError> {
    let text = std::str::from_utf8(data)
        .map_err(|_| ReadError::new("ld script", "the script is not UTF-8"))?;
    let tokens = tokens(&strip_comments(text)?);
    let mut inputs = Vec::new();
    let mut index = 0;
    let expect_open = |index: usize, command: &str| -> Result<(), ReadError> {
        if tokens.get(index).map(String::as_str) == Some("(") {
            Ok(())
        } else {
            Err(ReadError::new(
                "ld script",
                format!("`{command}` is not followed by `(`"),
            ))
        }
    };
    while index < tokens.len() {
        let command = tokens[index].as_str();
        match command {
            "OUTPUT_FORMAT" => {
                expect_open(index + 1, command)?;
                let close = tokens[index + 1..]
                    .iter()
                    .position(|token| token == ")")
                    .ok_or_else(|| {
                        ReadError::new("ld script", "an unterminated `OUTPUT_FORMAT(`")
                    })?;
                index += close + 2;
            }
            "GROUP" | "INPUT" => {
                expect_open(index + 1, command)?;
                index += 2;
                let mut as_needed = false;
                loop {
                    let token = tokens.get(index).ok_or_else(|| {
                        ReadError::new("ld script", format!("an unterminated `{command}(`"))
                    })?;
                    match token.as_str() {
                        ")" if as_needed => as_needed = false,
                        ")" => {
                            index += 1;
                            break;
                        }
                        "AS_NEEDED" => {
                            if as_needed {
                                return Err(ReadError::new("ld script", "a nested `AS_NEEDED`"));
                            }
                            expect_open(index + 1, "AS_NEEDED")?;
                            as_needed = true;
                            index += 1;
                        }
                        "(" => {
                            return Err(ReadError::new(
                                "ld script",
                                format!("an unexpected `(` inside `{command}`"),
                            ));
                        }
                        path if path.starts_with("-l") => {
                            return Err(ReadError::new(
                                "ld script",
                                format!(
                                    "`{path}` names a library search, not a file; the manifest cannot pin a search"
                                ),
                            ));
                        }
                        path if !path.starts_with('/')
                            && !std::path::Path::new(path).is_absolute() =>
                        {
                            return Err(ReadError::new(
                                "ld script",
                                format!(
                                    "`{path}` is a relative path, which `ld` resolves against the working directory and then the search path; only an absolute path is pinned"
                                ),
                            ));
                        }
                        path => inputs.push(ScriptInput {
                            path: path.to_owned(),
                            as_needed,
                        }),
                    }
                    index += 1;
                }
            }
            other => {
                return Err(ReadError::new(
                    "ld script",
                    format!(
                        "`{other}` is not a command of the library-script subset read here (`GROUP`, `INPUT`, `AS_NEEDED`, `OUTPUT_FORMAT`)"
                    ),
                ));
            }
        }
    }
    if inputs.is_empty() {
        return Err(ReadError::new("ld script", "the script names no file"));
    }
    Ok(inputs)
}
