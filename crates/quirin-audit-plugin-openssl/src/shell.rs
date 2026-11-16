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

//! Reading the compile line the pinned build prints (`make -n OBJECT`).
//!
//! OpenSSL's generated Makefile prints each compile as one shell command line.
//! [`words`] splits it as the shell would for the constructs such a line
//! uses — blanks, single quotes, double quotes with backslash escapes, and
//! backslashes — and refuses anything else that would make the shell do more
//! than split (expansions, substitutions, redirections, separators), so a
//! line it accepts is the argument vector the build ran.

/// Split a command line into words.
///
/// # Errors
///
/// An unterminated quote, a trailing backslash, or a character outside quotes
/// that would make the shell expand, substitute, redirect or separate.
pub fn words(line: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    let mut characters = line.chars();
    while let Some(character) = characters.next() {
        match character {
            ' ' | '\t' => {
                if in_word {
                    words.push(std::mem::take(&mut current));
                    in_word = false;
                }
            }
            '\'' => {
                in_word = true;
                loop {
                    match characters.next() {
                        Some('\'') => break,
                        Some(inner) => current.push(inner),
                        None => return Err("an unterminated single quote".into()),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match characters.next() {
                        Some('"') => break,
                        Some('\\') => match characters.next() {
                            Some(escaped @ ('"' | '\\' | '$' | '`')) => current.push(escaped),
                            Some(other) => {
                                current.push('\\');
                                current.push(other);
                            }
                            None => return Err("an unterminated double quote".into()),
                        },
                        Some(inner @ ('$' | '`')) => {
                            return Err(format!("`{inner}` inside double quotes"));
                        }
                        Some(inner) => current.push(inner),
                        None => return Err("an unterminated double quote".into()),
                    }
                }
            }
            '\\' => {
                in_word = true;
                match characters.next() {
                    Some(escaped) => current.push(escaped),
                    None => return Err("a trailing backslash".into()),
                }
            }
            '$' | '`' | ';' | '&' | '|' | '<' | '>' | '(' | ')' | '*' | '?' | '[' | '~' | '#'
            | '\n' => {
                return Err(format!("`{character}` outside quotes"));
            }
            other => {
                in_word = true;
                current.push(other);
            }
        }
    }
    if in_word {
        words.push(current);
    }
    Ok(words)
}

/// The flags of a compile of `source` to `object` printed by the pinned
/// build: the line's words without the compiler, `-MMD`, `-MF FILE`, `-c`,
/// `-o OBJECT` and the source.
///
/// # Errors
///
/// No line of `printed` compiles `source` to `object`, or the line does not
/// split, or it names another output or source.
pub fn compile_flags(printed: &str, object: &str, source: &str) -> Result<Vec<String>, String> {
    let marker = format!(" -c -o {object} {source}");
    let line = printed
        .lines()
        .find(|line| line.trim_end().ends_with(&marker))
        .ok_or_else(|| format!("the build prints no compile of {source} to {object}"))?;
    let words = words(line.trim())?;
    let mut flags = Vec::new();
    let mut index = 1;
    let mut seen_output = false;
    while index < words.len() {
        let word = words[index].as_str();
        match word {
            "-MMD" | "-c" => index += 1,
            "-MF" => index += 2,
            "-o" => {
                if words.get(index + 1).map(String::as_str) != Some(object) {
                    return Err(format!("the compile line's output is not {object}"));
                }
                seen_output = true;
                index += 2;
            }
            _ if index + 1 == words.len() => {
                if word != source {
                    return Err(format!("the compile line's source is not {source}"));
                }
                index += 1;
            }
            _ => {
                flags.push(word.to_owned());
                index += 1;
            }
        }
    }
    if !seen_output {
        return Err(format!("the compile line names no output {object}"));
    }
    Ok(flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pinned_compile_line_splits_as_the_shell_would() {
        let line = "/usr/lib/llvm-21/bin/clang  -I. -Iinclude  -fPIC -m64 \
                    -DOPENSSLDIR=\"\\\"/usr/local/ssl\\\"\" -DNDEBUG  -MMD -MF \
                    crypto/err/libcrypto-lib-err.d.tmp -c -o crypto/err/libcrypto-lib-err.o \
                    crypto/err/err.c";
        let printed = format!("if true; then\n{line}\nfi\n");
        let flags =
            compile_flags(&printed, "crypto/err/libcrypto-lib-err.o", "crypto/err/err.c").unwrap();
        assert_eq!(
            flags,
            [
                "-I.",
                "-Iinclude",
                "-fPIC",
                "-m64",
                "-DOPENSSLDIR=\"/usr/local/ssl\"",
                "-DNDEBUG"
            ]
        );
    }

    #[test]
    fn a_line_the_shell_would_do_more_with_is_refused() {
        assert!(words("cc -DX=$HOME a.c").is_err());
        assert!(words("cc a.c; rm -rf x").is_err());
        assert!(words("cc \"unterminated").is_err());
        assert!(words("cc -DX=\"$(id)\"").is_err());
        assert_eq!(words("a 'b c' d\\ e").unwrap(), ["a", "b c", "d e"]);
        assert!(compile_flags("cc -c -o x.o y.c\n", "z.o", "y.c").is_err());
    }
}
