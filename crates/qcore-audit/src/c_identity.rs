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

//! Normalised source identity of C functions (card
//! **W45-CONTRACT-REVISION-MODE**): the "identity a pinned contract was
//! reviewed against", recorded in a lock and compared at audit time.
//!
//! # What the identity is
//!
//! For one function name, every top-level **site** in a source tree: each
//! declaration (a prototype) and each definition, with the file it is in, the
//! SHA-256 of its declaration text (return type, name, parameter list and any
//! suffix before `;` or `{`) and, for a definition, the SHA-256 of its body
//! (`{` to the matching `}`). Text is **normalised** to C tokens joined by one
//! space: comments and layout do not count, every other change does (a renamed
//! local, a reordered statement, a changed directive inside the body). The
//! sites are sorted, so the identity is a value: two trees give equal
//! identities exactly when every site is textually equal after
//! normalisation. A name with no site has the empty identity.
//!
//! # What the scanner does and does not do
//!
//! It is a lexical scanner, not a parser, and it never preprocesses: a site
//! guarded by `#if` counts whatever the configuration, so the identity covers
//! every configuration's text. Braces are balanced per conditional group the
//! way ctags does it: each `#elif`/`#else` branch restarts from the depth the
//! group opened at, and `#endif` resumes from the end of the first branch. An
//! `extern "C" {` block is transparent. A body that never closes runs to the
//! end of its file, which covers more text, never less. The scanner reads only
//! `.c` and `.h` files; for a Git checkout it reads exactly the blobs of
//! `HEAD` (never the working tree), and otherwise a sorted walk that skips
//! dot-directories and symbolic links.
//!
//! A false mismatch refuses a pinned contract (the fail-closed direction);
//! the scanner is built so that a change inside a function's text always
//! changes its identity.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use qcore_schemas::sha256_hex;
use serde::{Deserialize, Serialize};

/// The normalisation's name, recorded in every lock.
pub const NORMALISATION: &str = "c-tokens-v1";

/// One declaration or definition of a function.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceSite {
    /// The file, relative to the tree's root, with `/` separators.
    pub path: String,
    /// `declaration` or `definition`.
    pub kind: SiteKind,
    /// SHA-256 of the normalised declaration text.
    pub declaration_sha256: String,
    /// SHA-256 of the normalised body, for a definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_sha256: Option<String>,
}

/// Whether a site declares or defines its function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteKind {
    Declaration,
    Definition,
    /// Card **SUPPLIED-OWNED-RETURN-CODE-HANDOFF** (rule 13l): the top-level
    /// enumeration declaring an `@` name a declaration spells (keyed `@NAME`;
    /// `declaration_sha256` covers the whole statement, `typedef enum {…} T;`
    /// or `enum T {…};`), so the enumerator's value is part of the reviewed
    /// identity.
    Enumeration,
}

/// Card **SUPPLIED-OWNED-RETURN-CODE-HANDOFF** (rule 13l): the names to scan
/// for `functions`: each function and each `@` name its declaration spells
/// (`c_names`, from `declaration_c_names`), the latter keyed `@NAME`.
#[must_use]
pub fn identity_names(
    functions: &BTreeSet<String>,
    c_names: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut names = functions.clone();
    for function in functions {
        for name in c_names.get(function).into_iter().flatten() {
            names.insert(format!("@{name}"));
        }
    }
    names
}

/// The identity of `function`: its own sites and the enumeration sites of
/// every `@` name its declaration spells, sorted. `Err` names an `@` name
/// with no enumeration site in the tree (unresolvable: the pin refuses it).
///
/// # Errors
/// An `@` name the tree declares in no enumeration.
pub fn identity_sites(
    found: &BTreeMap<String, Vec<SourceSite>>,
    function: &str,
    c_names: &BTreeMap<String, BTreeSet<String>>,
) -> Result<Vec<SourceSite>, String> {
    let mut sites = found.get(function).cloned().unwrap_or_default();
    for name in c_names.get(function).into_iter().flatten() {
        let enumerations = found.get(&format!("@{name}")).cloned().unwrap_or_default();
        if enumerations.is_empty() {
            return Err(format!(
                "`@{name}` names no enumerator of the source tree (an unresolvable name)"
            ));
        }
        sites.extend(enumerations);
    }
    sites.sort();
    Ok(sites)
}

/// A source tree read for identities: every `.c` and `.h` file, with how it
/// was read.
#[derive(Clone, Debug)]
pub struct SourceTree {
    /// `git` (the blobs of `HEAD`) or `directory` (a walk).
    pub kind: &'static str,
    /// The Git revision of `HEAD`, for a Git checkout.
    pub revision: Option<String>,
    /// Relative path to text, sorted by path.
    pub files: BTreeMap<String, String>,
}

fn is_c_file(path: &str) -> bool {
    path.ends_with(".c") || path.ends_with(".h")
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("git {}: {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "git {} in {}: {}",
            args.join(" "),
            root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

impl SourceTree {
    /// Read `root`: the blobs of `HEAD` when it is the top of a Git checkout,
    /// otherwise a walk of the directory.
    ///
    /// # Errors
    ///
    /// A Git command or a read that fails refuses the whole tree.
    pub fn read(root: &Path) -> Result<Self, String> {
        if root.join(".git").exists() {
            Self::read_git(root, "HEAD")
        } else {
            Self::read_directory(root)
        }
    }

    /// Read the blobs of `revision` (any commit spelling Git resolves) from
    /// the object store of the checkout at `root`, never its working tree.
    /// Every command used only reads (`rev-parse`, `ls-tree`, `cat-file`).
    ///
    /// # Errors
    ///
    /// A root that is not a Git checkout, an unknown revision, or a failing
    /// Git command refuses.
    pub fn read_revision(root: &Path, revision: &str) -> Result<Self, String> {
        if !root.join(".git").exists() {
            return Err(format!(
                "{} is not a Git checkout, so it has no revision {revision}",
                root.display()
            ));
        }
        Self::read_git(root, revision)
    }

    fn read_git(root: &Path, spelling: &str) -> Result<Self, String> {
        let mut files = BTreeMap::new();
        let revision = visit_git(root, spelling, &mut |path, text| {
            files.insert(path.to_owned(), text);
            Ok(())
        })?;
        Ok(Self {
            kind: "git",
            revision: Some(revision),
            files,
        })
    }

    fn read_directory(root: &Path) -> Result<Self, String> {
        let mut files = BTreeMap::new();
        visit_directory(root, &mut |path, text| {
            files.insert(path.to_owned(), text);
            Ok(())
        })?;
        Ok(Self {
            kind: "directory",
            revision: None,
            files,
        })
    }

    /// The sites of every name in `names`, as [`Self::read`] then
    /// [`Self::sites`] give them, reading one file at a time and never
    /// holding the tree (card **W45-AUDIT-CHECKPOINTS**: the audit's setup
    /// used to hold every `.c`/`.h` blob of the checkout twice, which set the
    /// process's peak memory). The sites of each name are sorted, so the
    /// order the files are read in does not matter.
    ///
    /// # Errors
    ///
    /// As [`Self::read`].
    pub fn read_sites(root: &Path, names: &BTreeSet<String>) -> Result<SiteScan, String> {
        let mut sites: BTreeMap<String, Vec<SourceSite>> = names
            .iter()
            .map(|name| (name.clone(), Vec::new()))
            .collect();
        let mut visit = |path: &str, text: String| {
            for (name, site) in file_sites(path, &text, names) {
                sites.entry(name).or_default().push(site);
            }
            Ok(())
        };
        let (kind, revision) = if root.join(".git").exists() {
            ("git", Some(visit_git(root, "HEAD", &mut visit)?))
        } else {
            visit_directory(root, &mut visit)?;
            ("directory", None)
        };
        for list in sites.values_mut() {
            list.sort();
        }
        Ok(SiteScan {
            kind,
            revision,
            sites,
        })
    }

    /// Card **CONTRACT-PIN-TIP-MODE**: the sites of every name in `names` as
    /// the blobs of `revision` (a commit spelling Git resolves) in the
    /// repository at `root` give them, one blob at a time, never reading a
    /// working tree. Every command used only reads. The caller decides that
    /// `root` is the repository it means.
    ///
    /// # Errors
    ///
    /// A failing Git command or an unreadable blob refuses.
    pub fn read_sites_at(
        root: &Path,
        revision: &str,
        names: &BTreeSet<String>,
    ) -> Result<SiteScan, String> {
        let mut sites: BTreeMap<String, Vec<SourceSite>> = names
            .iter()
            .map(|name| (name.clone(), Vec::new()))
            .collect();
        let mut visit = |path: &str, text: String| {
            for (name, site) in file_sites(path, &text, names) {
                sites.entry(name).or_default().push(site);
            }
            Ok(())
        };
        let resolved = visit_git(root, revision, &mut visit)?;
        for list in sites.values_mut() {
            list.sort();
        }
        Ok(SiteScan {
            kind: "git",
            revision: Some(resolved),
            sites,
        })
    }

    /// The sorted sites of every name in `names`; a name with no site maps to
    /// an empty list.
    #[must_use]
    pub fn sites(&self, names: &BTreeSet<String>) -> BTreeMap<String, Vec<SourceSite>> {
        let mut sites: BTreeMap<String, Vec<SourceSite>> = names
            .iter()
            .map(|name| (name.clone(), Vec::new()))
            .collect();
        for (path, text) in &self.files {
            for (name, site) in file_sites(path, text, names) {
                sites.entry(name).or_default().push(site);
            }
        }
        for list in sites.values_mut() {
            list.sort();
        }
        sites
    }
}

/// What [`SourceTree::read_sites`] found: how the tree was read and the sites.
#[derive(Clone, Debug)]
pub struct SiteScan {
    /// As [`SourceTree::kind`].
    pub kind: &'static str,
    /// As [`SourceTree::revision`].
    pub revision: Option<String>,
    /// As [`SourceTree::sites`].
    pub sites: BTreeMap<String, Vec<SourceSite>>,
}

/// Visit every `.c`/`.h` blob of `spelling` in the checkout at `root`, one at a
/// time, streaming `git cat-file --batch`; returns the resolved revision.
fn visit_git(
    root: &Path,
    spelling: &str,
    visit: &mut dyn FnMut(&str, String) -> Result<(), String>,
) -> Result<String, String> {
    let commit = format!("{spelling}^{{commit}}");
    let revision = String::from_utf8_lossy(&git(root, &["rev-parse", "--verify", &commit])?)
        .trim()
        .to_owned();
    let listing = git(root, &["ls-tree", "-r", "-z", "--full-tree", &revision])?;
    let mut blobs = Vec::new();
    for entry in listing.split(|byte| *byte == 0).filter(|e| !e.is_empty()) {
        let entry = String::from_utf8_lossy(entry);
        let Some((meta, path)) = entry.split_once('\t') else {
            return Err(format!("unreadable git tree entry `{entry}`"));
        };
        let mut fields = meta.split(' ');
        let (_, kind, object) = (fields.next(), fields.next(), fields.next());
        if kind == Some("blob") && is_c_file(path) {
            let object = object.ok_or_else(|| format!("unreadable git tree entry `{entry}`"))?;
            blobs.push((path.to_owned(), object.to_owned()));
        }
    }
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("git cat-file: {error}"))?;
    let mut stdin = child.stdin.take().ok_or("git cat-file: no stdin")?;
    let request = blobs
        .iter()
        .map(|(_, object)| format!("{object}\n"))
        .collect::<String>();
    let writer = std::thread::spawn(move || stdin.write_all(request.as_bytes()));
    let mut output = BufReader::new(child.stdout.take().ok_or("git cat-file: no stdout")?);
    let read = (|| -> Result<(), String> {
        let mut header = Vec::new();
        for (path, object) in &blobs {
            header.clear();
            output
                .read_until(b'\n', &mut header)
                .map_err(|error| format!("git cat-file: {error}"))?;
            if header.pop() != Some(b'\n') {
                return Err("git cat-file: truncated output".into());
            }
            let header = String::from_utf8_lossy(&header).into_owned();
            let mut fields = header.split(' ');
            if fields.next() != Some(object.as_str()) || fields.next() != Some("blob") {
                return Err(format!("git cat-file: unexpected header `{header}`"));
            }
            let size = fields
                .next()
                .and_then(|size| size.parse::<usize>().ok())
                .ok_or_else(|| format!("git cat-file: unexpected header `{header}`"))?;
            let mut body = vec![0u8; size + 1];
            output
                .read_exact(&mut body)
                .map_err(|_| "git cat-file: truncated output".to_owned())?;
            if body.pop() != Some(b'\n') {
                return Err("git cat-file: truncated output".into());
            }
            visit(path, String::from_utf8_lossy(&body).into_owned())?;
        }
        Ok(())
    })();
    // Drain whatever is left so the child can exit, whatever happened above.
    let _ = std::io::copy(&mut output, &mut std::io::sink());
    let written = writer
        .join()
        .map_err(|_| "git cat-file: writer panicked".to_owned())?;
    let status = child
        .wait()
        .map_err(|error| format!("git cat-file: {error}"))?;
    read?;
    written.map_err(|error| format!("git cat-file: {error}"))?;
    if !status.success() {
        return Err("git cat-file failed".into());
    }
    Ok(revision)
}

/// Visit every `.c`/`.h` file under `root` in a sorted walk that skips
/// dot-entries and symbolic links, one file at a time.
fn visit_directory(
    root: &Path,
    visit: &mut dyn FnMut(&str, String) -> Result<(), String>,
) -> Result<(), String> {
    fn walk(
        root: &Path,
        directory: &Path,
        visit: &mut dyn FnMut(&str, String) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut entries = fs::read_dir(directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let kind = entry
                .file_type()
                .map_err(|error| format!("{}: {error}", entry.path().display()))?;
            if kind.is_symlink() || name.starts_with('.') {
                continue;
            }
            let path = entry.path();
            if kind.is_dir() {
                walk(root, &path, visit)?;
            } else if kind.is_file() && is_c_file(&name) {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| format!("{} escapes the tree", path.display()))?
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                let bytes =
                    fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
                visit(&relative, String::from_utf8_lossy(&bytes).into_owned())?;
            }
        }
        Ok(())
    }
    walk(root, root, visit)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Kind {
    Identifier,
    Directive,
    Literal,
    Punctuator,
}

#[derive(Clone, Debug)]
struct Token {
    kind: Kind,
    text: String,
}

const PUNCTUATORS: [&str; 24] = [
    "...", "<<=", ">>=", "->", "++", "--", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "+=",
    "-=", "*=", "/=", "%=", "&=", "^=", "|=", "##", "::",
];

/// Lex C text into normalised tokens: comments dropped, a preprocessing
/// directive (continuation lines included) one token with its whitespace
/// collapsed, string and character literals verbatim.
fn tokens(text: &str) -> Vec<Token> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    let mut line_start = true;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\n' {
            line_start = true;
            index += 1;
            continue;
        }
        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if text[index..].starts_with("//") {
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if text[index..].starts_with("/*") {
            index = text[index + 2..]
                .find("*/")
                .map_or(bytes.len(), |end| index + 2 + end + 2);
            continue;
        }
        if byte == b'#' && line_start {
            let mut directive = String::new();
            while index < bytes.len() && bytes[index] != b'\n' {
                if text[index..].starts_with("\\\n") {
                    index += 2;
                    directive.push(' ');
                    continue;
                }
                if text[index..].starts_with("/*") {
                    index = text[index + 2..]
                        .find("*/")
                        .map_or(bytes.len(), |end| index + 2 + end + 2);
                    directive.push(' ');
                    continue;
                }
                if text[index..].starts_with("//") {
                    while index < bytes.len() && bytes[index] != b'\n' {
                        index += 1;
                    }
                    break;
                }
                let character = text[index..].chars().next().expect("in bounds");
                directive.push(character);
                index += character.len_utf8();
            }
            out.push(Token {
                kind: Kind::Directive,
                text: directive.split_whitespace().collect::<Vec<_>>().join(" "),
            });
            continue;
        }
        line_start = false;
        if byte == b'"' || byte == b'\'' {
            let start = index;
            index += 1;
            while index < bytes.len() && bytes[index] != byte && bytes[index] != b'\n' {
                if bytes[index] == b'\\' {
                    index += 1;
                }
                index += 1;
            }
            index = (index + 1).min(bytes.len());
            out.push(Token {
                kind: Kind::Literal,
                text: String::from_utf8_lossy(&bytes[start..index]).into_owned(),
            });
            continue;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = index;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            out.push(Token {
                kind: Kind::Identifier,
                text: text[start..index].to_owned(),
            });
            continue;
        }
        if byte.is_ascii_digit()
            || (byte == b'.' && bytes.get(index + 1).is_some_and(u8::is_ascii_digit))
        {
            // A preprocessing number: digits, letters, `_`, `.` and a sign
            // after an exponent letter.
            let start = index;
            index += 1;
            while index < bytes.len() {
                let current = bytes[index];
                let part = current.is_ascii_alphanumeric() || current == b'_' || current == b'.';
                let exponent_sign = (current == b'+' || current == b'-')
                    && matches!(bytes[index - 1], b'e' | b'E' | b'p' | b'P');
                if !(part || exponent_sign) {
                    break;
                }
                index += 1;
            }
            out.push(Token {
                kind: Kind::Literal,
                text: text[start..index].to_owned(),
            });
            continue;
        }
        let punctuator = PUNCTUATORS
            .iter()
            .find(|candidate| text[index..].starts_with(**candidate))
            .map_or_else(
                || text[index..].chars().next().expect("in bounds").to_string(),
                |candidate| (*candidate).to_owned(),
            );
        index += punctuator.len();
        out.push(Token {
            kind: Kind::Punctuator,
            text: punctuator,
        });
    }
    out
}

fn directive_name(text: &str) -> &str {
    text.trim_start_matches('#')
        .split_whitespace()
        .next()
        .unwrap_or_default()
}

/// The brace structure of one file: each token's depth and each counted
/// opening brace's matching close.
struct Structure {
    depth: Vec<usize>,
    closing: BTreeMap<usize, usize>,
    /// A conditional group closed a brace opened before the group. The brace
    /// structure is then not trusted: every body in the file runs to its end.
    unbalanced: bool,
}

fn structure(tokens: &[Token]) -> Structure {
    struct Group {
        opened: Vec<(usize, bool)>,
        after_first: Option<Vec<(usize, bool)>>,
    }
    // Open braces: (token index, transparent).
    let mut open: Vec<(usize, bool)> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    let mut depth = Vec::with_capacity(tokens.len());
    let mut closing = BTreeMap::new();
    let mut unbalanced = false;
    let current = |open: &[(usize, bool)]| open.iter().filter(|(_, t)| !t).count();
    for (index, token) in tokens.iter().enumerate() {
        depth.push(current(&open));
        match (&token.kind, token.text.as_str()) {
            (Kind::Directive, text) => match directive_name(text) {
                "if" | "ifdef" | "ifndef" => groups.push(Group {
                    opened: open.clone(),
                    after_first: None,
                }),
                "elif" | "else" | "elifdef" | "elifndef" => {
                    if let Some(group) = groups.last_mut() {
                        if group.after_first.is_none() {
                            group.after_first = Some(open.clone());
                        }
                        open.clone_from(&group.opened);
                    }
                }
                "endif" => {
                    if let Some(after_first) = groups.pop().and_then(|group| group.after_first) {
                        open = after_first;
                    }
                }
                _ => {}
            },
            (Kind::Punctuator, "{") => {
                let transparent = index >= 2
                    && tokens[index - 2].text == "extern"
                    && tokens[index - 1].kind == Kind::Literal
                    && tokens[index - 1].text.starts_with('"');
                open.push((index, transparent));
            }
            (Kind::Punctuator, "}") => {
                // The `#ifdef __cplusplus` close of an `extern "C" {` block is
                // the one outer brace a group may close.
                let transparent = open.last().is_some_and(|(_, transparent)| *transparent);
                if !transparent && groups.iter().any(|group| open.len() <= group.opened.len()) {
                    unbalanced = true;
                }
                if let Some((opened, _)) = open.pop() {
                    closing.insert(opened, index);
                }
                // A closing brace stands at the depth it returns to, so the
                // `}` ending a top-level body is itself top-level.
                depth[index] = current(&open);
            }
            _ => {}
        }
    }
    Structure {
        depth,
        closing,
        unbalanced,
    }
}

fn digest(tokens: &[Token]) -> String {
    sha256_hex(
        tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
            .as_bytes(),
    )
}

/// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 13l): the top-level
/// enumeration statements of one file that declare a wanted `@` name.
fn enumeration_sites(
    path: &str,
    tokens: &[Token],
    structure: &Structure,
    wanted: &BTreeSet<&str>,
) -> Vec<(String, SourceSite)> {
    let mut sites = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != Kind::Identifier || token.text != "enum" || structure.depth[index] != 0 {
            continue;
        }
        // `enum {` or `enum TAG {`.
        let open = match tokens.get(index + 1) {
            Some(next) if next.text == "{" => index + 1,
            Some(next)
                if next.kind == Kind::Identifier
                    && tokens.get(index + 2).is_some_and(|brace| brace.text == "{") =>
            {
                index + 2
            }
            _ => continue,
        };
        let Some(&close) = structure.closing.get(&open) else {
            continue;
        };
        let declared = (open + 1..close)
            .filter(|at| {
                tokens[*at].kind == Kind::Identifier
                    && matches!(tokens[at - 1].text.as_str(), "{" | ",")
                    && tokens
                        .get(at + 1)
                        .is_some_and(|next| matches!(next.text.as_str(), "=" | "," | "}"))
            })
            .map(|at| tokens[at].text.as_str())
            .filter(|name| wanted.contains(name))
            .collect::<BTreeSet<_>>();
        if declared.is_empty() {
            continue;
        }
        let mut start = index;
        while start > 0 {
            let previous = &tokens[start - 1];
            if previous.kind == Kind::Directive
                || (structure.depth[start - 1] == 0
                    && previous.kind == Kind::Punctuator
                    && matches!(previous.text.as_str(), ";" | "}" | "{"))
            {
                break;
            }
            start -= 1;
        }
        let end = (close + 1..tokens.len())
            .find(|at| structure.depth[*at] == 0 && tokens[*at].text == ";")
            .unwrap_or(tokens.len() - 1);
        let digest = digest(&tokens[start..=end]);
        for name in declared {
            sites.push((
                format!("@{name}"),
                SourceSite {
                    path: path.to_owned(),
                    kind: SiteKind::Enumeration,
                    declaration_sha256: digest.clone(),
                    body_sha256: None,
                },
            ));
        }
    }
    sites
}

/// The top-level sites of `names` in one file (an `@NAME` its enumeration).
fn file_sites(path: &str, text: &str, names: &BTreeSet<String>) -> Vec<(String, SourceSite)> {
    // Cheap rejection: a file that never spells a wanted name has no site.
    if !names
        .iter()
        .any(|name| text.contains(name.trim_start_matches('@')))
    {
        return Vec::new();
    }
    let tokens = tokens(text);
    let structure = structure(&tokens);
    let wanted = names
        .iter()
        .filter_map(|name| name.strip_prefix('@'))
        .collect::<BTreeSet<_>>();
    let mut sites = if wanted.is_empty() {
        Vec::new()
    } else {
        enumeration_sites(path, &tokens, &structure, &wanted)
    };
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        let is_site = token.kind == Kind::Identifier
            && structure.depth[index] == 0
            && names.contains(&token.text)
            && tokens.get(index + 1).is_some_and(|next| next.text == "(")
            && !(index > 0 && matches!(tokens[index - 1].text.as_str(), "." | "->"));
        if !is_site {
            index += 1;
            continue;
        }
        // The statement starts after the previous top-level `;`, `}` or
        // directive, or at the file's start.
        let mut start = index;
        while start > 0 {
            let previous = &tokens[start - 1];
            if previous.kind == Kind::Directive
                || (structure.depth[start - 1] == 0
                    && previous.kind == Kind::Punctuator
                    && matches!(previous.text.as_str(), ";" | "}" | "{"))
            {
                break;
            }
            start -= 1;
        }
        // The parameter list's closing parenthesis.
        let mut parentheses = 0usize;
        let mut close = None;
        for (offset, candidate) in tokens[index + 1..].iter().enumerate() {
            match candidate.text.as_str() {
                "(" if candidate.kind == Kind::Punctuator => parentheses += 1,
                ")" if candidate.kind == Kind::Punctuator => {
                    parentheses -= 1;
                    if parentheses == 0 {
                        close = Some(index + 1 + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(close) = close else {
            // An unclosed parameter list: the rest of the file is its text.
            sites.push((
                token.text.clone(),
                SourceSite {
                    path: path.to_owned(),
                    kind: SiteKind::Declaration,
                    declaration_sha256: digest(&tokens[start..]),
                    body_sha256: None,
                },
            ));
            break;
        };
        // The suffix up to the terminator: `{` (a definition) or `;`/`,`/`=`
        // (a declaration), skipping balanced parentheses (attributes).
        let mut parentheses = 0usize;
        let mut end = close + 1;
        while end < tokens.len() {
            match tokens[end].text.as_str() {
                "(" => parentheses += 1,
                ")" => parentheses = parentheses.saturating_sub(1),
                "{" | ";" | "," | "="
                    if parentheses == 0 && tokens[end].kind == Kind::Punctuator =>
                {
                    break;
                }
                _ => {}
            }
            end += 1;
        }
        let declaration = &tokens[start..end.min(tokens.len())];
        if tokens.get(end).is_some_and(|t| t.text == "{") {
            // A body whose close is unknown, or in a file whose conditional
            // groups do not balance, runs to the end of the file: the digest
            // then covers more text, never less.
            let body_end = structure
                .closing
                .get(&end)
                .copied()
                .filter(|_| !structure.unbalanced)
                .unwrap_or(tokens.len() - 1);
            sites.push((
                token.text.clone(),
                SourceSite {
                    path: path.to_owned(),
                    kind: SiteKind::Definition,
                    declaration_sha256: digest(declaration),
                    body_sha256: Some(digest(&tokens[end..=body_end])),
                },
            ));
            index = body_end + 1;
        } else {
            sites.push((
                token.text.clone(),
                SourceSite {
                    path: path.to_owned(),
                    kind: SiteKind::Declaration,
                    declaration_sha256: digest(declaration),
                    body_sha256: None,
                },
            ));
            index = end.max(index + 1);
        }
    }
    sites
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use std::fs;
    use std::process::Command;

    use super::{SiteKind, SourceTree, file_sites};

    fn names(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    /// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 13l): an `@` name's
    /// site is its whole top-level enumeration; a changed enumerator value
    /// changes the digest, a use of the name is no site, and a name no
    /// enumeration declares has none (the pin refuses it).
    #[test]
    fn an_enumerator_is_pinned_by_its_whole_enumeration() {
        let text = "typedef enum {\n A_PASS, /* x */ A_HANDLED,\n A_KEEP = 7 } a_res_t;\n\
                    enum other { A_OTHER };\nint f(void) { return A_KEEP; }\n";
        let sites = file_sites("a.h", text, &names(&["@A_KEEP", "@A_OTHER", "@A_NONE"]));
        let listed = sites
            .iter()
            .map(|(name, site)| (name.as_str(), site.kind))
            .collect::<Vec<_>>();
        assert_eq!(
            listed,
            vec![
                ("@A_KEEP", SiteKind::Enumeration),
                ("@A_OTHER", SiteKind::Enumeration)
            ]
        );
        let changed = file_sites(
            "a.h",
            &text.replace("A_KEEP = 7", "A_KEEP = 8"),
            &names(&["@A_KEEP"]),
        );
        assert_ne!(
            changed[0].1.declaration_sha256,
            sites[0].1.declaration_sha256
        );
        let found = std::collections::BTreeMap::from([
            ("f".to_owned(), Vec::new()),
            ("@A_KEEP".to_owned(), vec![sites[0].1.clone()]),
        ]);
        let wanted = std::collections::BTreeMap::from([("f".to_owned(), names(&["A_KEEP"]))]);
        assert_eq!(
            super::identity_sites(&found, "f", &wanted).expect("resolves"),
            vec![sites[0].1.clone()]
        );
        let unresolved = std::collections::BTreeMap::from([("f".to_owned(), names(&["A_NONE"]))]);
        assert!(super::identity_sites(&found, "f", &unresolved).is_err());
        assert!(
            super::identity_names(&names(&["f"]), &wanted).contains("@A_KEEP"),
            "the scan asks for the enumeration"
        );
    }

    #[test]
    fn declarations_and_definitions_are_sites_and_calls_are_not() {
        let text = "#include <x.h>\nint f(int a);\n/* c */ static int g(void) { return f(1); }\n\
                    int f(int a)\n{\n    return a + 1;\n}\n";
        let sites = file_sites("a.c", text, &names(&["f", "g"]));
        let kinds = sites
            .iter()
            .map(|(name, site)| (name.as_str(), site.kind))
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            [
                ("f", SiteKind::Declaration),
                ("g", SiteKind::Definition),
                ("f", SiteKind::Definition)
            ]
        );
    }

    #[test]
    fn layout_and_comments_do_not_count_and_every_token_does() {
        let site = |text: &str| file_sites("a.c", text, &names(&["f"]))[0].1.clone();
        let base = site("int f(int a) { int x = a; return x; }");
        assert_eq!(
            base,
            site("int\nf (int a)\n{\n  /* same */ int x = a;\n  return x; // done\n}\n")
        );
        let renamed = site("int f(int a) { int y = a; return y; }");
        assert_eq!(base.declaration_sha256, renamed.declaration_sha256);
        assert_ne!(base.body_sha256, renamed.body_sha256);
        assert_ne!(
            base.declaration_sha256,
            site("long f(int a) { int x = a; return x; }").declaration_sha256
        );
    }

    #[test]
    fn a_group_that_closes_an_outer_brace_extends_every_body_to_the_end() {
        let text = "int f(void) {\n  x();\n#ifdef A\n}\nint g(void) {\n#endif\n  y();\n}\n";
        let changed = text.replace("y();", "z();");
        let body = |text: &str| {
            file_sites("a.c", text, &names(&["f"]))[0]
                .1
                .body_sha256
                .clone()
        };
        // `y()` is inside `f` when `A` is undefined, so it is in `f`'s identity.
        assert_ne!(body(text), body(&changed));
    }

    #[test]
    fn conditional_groups_and_extern_c_keep_the_top_level() {
        let text = "#ifdef __cplusplus\nextern \"C\" {\n#endif\nint f(int a);\n\
                    #ifdef A\nint g(int a) {\n#else\nint g(long a) {\n#endif\n  return 0;\n}\n\
                    int h(void) { return f(2); }\n#ifdef __cplusplus\n}\n#endif\n";
        let sites = file_sites("a.h", text, &names(&["f", "g", "h"]));
        let listed = sites
            .iter()
            .map(|(name, site)| (name.as_str(), site.kind))
            .collect::<Vec<_>>();
        assert_eq!(listed[0], ("f", SiteKind::Declaration));
        assert!(listed.contains(&("h", SiteKind::Definition)));
        assert!(listed.iter().filter(|(name, _)| *name == "g").count() >= 1);
        // `f(2)` inside `h` is a call, not a site.
        assert_eq!(listed.iter().filter(|(name, _)| *name == "f").count(), 1);
    }

    /// Card W45-AUDIT-CHECKPOINTS: reading one file at a time finds exactly
    /// the sites the whole-tree read finds, for a Git checkout (whose working
    /// tree differs from `HEAD`) and for a plain directory.
    #[test]
    fn streamed_sites_equal_the_whole_tree_sites() {
        let root =
            std::env::temp_dir().join(format!("qcore-identity-stream-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let files = [
            ("a.c", "int f(int a) { return a; }\nstatic int g(void);\n"),
            ("sub/b.h", "int f(int a);\nint g(void);\n"),
            ("sub/deep/c.c", "int g(void) { return f(1); }\n"),
            ("notes.txt", "int f(int a) { return 0; }\n"),
        ];
        for (path, text) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        let wanted = names(&["f", "g", "absent"]);
        let directory = SourceTree::read(&root).unwrap();
        let streamed = SourceTree::read_sites(&root, &wanted).unwrap();
        assert_eq!(streamed.kind, "directory");
        assert_eq!(streamed.sites, directory.sites(&wanted));
        assert_eq!(streamed.sites["f"].len(), 2);
        let run = |args: &[&str]| {
            let output = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
        };
        run(&["init", "--quiet"]);
        run(&["add", "."]);
        run(&[
            "-c",
            "user.name=Identity Fixture",
            "-c",
            "user.email=identity@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ]);
        // The working tree moves away from HEAD; both reads use HEAD's blobs.
        fs::write(root.join("a.c"), "int f(int a) { return a + 1; }\n").unwrap();
        let tree = SourceTree::read(&root).unwrap();
        let streamed = SourceTree::read_sites(&root, &wanted).unwrap();
        assert_eq!(streamed.kind, "git");
        assert_eq!(streamed.revision, tree.revision);
        assert_eq!(streamed.sites, tree.sites(&wanted));
        assert_eq!(streamed.sites, directory.sites(&wanted));
        let _ = fs::remove_dir_all(&root);
    }
}
