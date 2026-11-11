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

//! Card **AUDIT-CANONICAL-PATHS** (user decision 7, 2026-10-24): unit
//! selection compares canonical paths, so a moved or symlinked checkout keeps
//! working.
//!
//! A compilation database spells its paths as they were when it was written.
//! After the checkout moved, the pinned databases spelled
//! `/home/cosmo/Documents/GitHub/qcore-c/...`, a symlink to the real checkout,
//! while every route resolves its declared root to the real path; comparing
//! the two strings selected no unit (`PINNED-INPUTS-PATH-AFTER-MIGRATION`).
//! Every selection now compares **located** forms:
//!
//! * a path that resolves is replaced by its canonical form (symlinks
//!   resolved, `.`/`..` applied as the kernel applies them, so a
//!   `compile_commands.json` `directory` that is itself a symlink and a
//!   relative `file` with `..` resolve as Clang would open them);
//! * a path that does not resolve is located through its deepest existing
//!   ancestor, canonicalised, with the unresolved tail kept component for
//!   component. It is never treated as resolving: if it is selected it stays
//!   in the denominator and its unit is refused when it is read, exactly as a
//!   dangling path was before.
//!
//! The located form is for **matching only**. Every selected
//! [`CompileEntry`] keeps its spelled `directory`, `file` and `command`, so
//! Clang reads the recorded spelling (constraint 11) and every published path,
//! compile-task identity and digest is the one the route published before.
//! A relative path a row publishes is the spelled one wherever the spelling
//! lies under the root, and the canonical relative path otherwise.
//!
//! Fail-closed, never admitted:
//!
//! * an entry spelled under a root that resolves outside it is refused with
//!   the route's existing outside-root diagnostic ([`Placement::Escapes`]);
//! * two selected records that spell one source file two ways (or one
//!   configuration's directory two ways) are refused with the named
//!   diagnostic [`ALIAS_DIAGNOSTIC`], never merged and never counted twice.
//!
//! Source identity is untouched: pinned-byte checks and contract pins keep
//! comparing content digests.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::compile_db::CompileEntry;

/// The named diagnostic for two spellings of one selected source file.
pub const ALIAS_DIAGNOSTIC: &str = "input-path-alias";

/// Where a spelled path lies on disk, for matching only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Location {
    /// The path resolves; its canonical form.
    Resolved(PathBuf),
    /// The path does not resolve; its deepest existing ancestor, canonical,
    /// joined with the unresolved tail.
    Unresolved(PathBuf),
}

impl Location {
    /// The located path.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Resolved(path) | Self::Unresolved(path) => path,
        }
    }

    /// Whether the path resolved.
    #[must_use]
    pub fn is_resolved(&self) -> bool {
        matches!(self, Self::Resolved(_))
    }
}

/// Locate one spelled path ([module documentation](self)).
#[must_use]
pub fn locate(path: &Path) -> Location {
    if let Ok(canonical) = path.canonicalize() {
        return Location::Resolved(canonical);
    }
    let components = path.components().collect::<Vec<_>>();
    for split in (1..components.len()).rev() {
        let ancestor = components[..split].iter().collect::<PathBuf>();
        if let Ok(canonical) = ancestor.canonicalize() {
            let tail = components[split..].iter().collect::<PathBuf>();
            return Location::Unresolved(canonical.join(tail));
        }
    }
    Location::Unresolved(path.to_path_buf())
}

/// Whether two spelled paths locate to the same path.
#[must_use]
pub fn same_located(left: &Path, right: &Path) -> bool {
    left == right || locate(left) == locate(right)
}

/// A path under a root: the canonical relative path selection decides on, and
/// the relative path a row publishes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    /// The located path relative to the located root.
    pub canonical: PathBuf,
    /// The spelled relative path when the spelling lies under the spelled
    /// root, else [`Self::canonical`].
    pub spelled: PathBuf,
}

/// How a spelled path stands against a declared root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Placement {
    /// The located path lies under the located root.
    Inside(Placed),
    /// The spelling lies under the root but the located path does not.
    Escapes,
    /// Neither the spelling nor the located path lies under the root.
    Unrelated,
}

/// Place `file` against `root` by their located forms.
#[must_use]
pub fn place(file: &Path, root: &Path) -> Placement {
    let located_root = locate(root);
    let located_file = locate(file);
    let spelled = file.strip_prefix(root).ok();
    match located_file.path().strip_prefix(located_root.path()) {
        Ok(canonical) => Placement::Inside(Placed {
            canonical: canonical.to_path_buf(),
            spelled: spelled.unwrap_or(canonical).to_path_buf(),
        }),
        Err(_) if spelled_through(file, root, located_root.path()).is_some() => Placement::Escapes,
        Err(_) => Placement::Unrelated,
    }
}

/// The part of `file`'s spelling below the root, when the spelling passes
/// through the root: it lies under the spelled or located root as written,
/// or one of its directories locates at or under the located root (a record
/// spelled through a symlinked checkout whose last component, or a directory
/// below the root, is a symlink leading out).
fn spelled_through(file: &Path, root: &Path, located_root: &Path) -> Option<PathBuf> {
    if let Ok(relative) = file
        .strip_prefix(root)
        .or_else(|_| file.strip_prefix(located_root))
    {
        return Some(relative.to_path_buf());
    }
    file.ancestors()
        .skip(1)
        .filter(|ancestor| !ancestor.as_os_str().is_empty())
        .find_map(|ancestor| {
            let located = locate(ancestor);
            let below = located.path().strip_prefix(located_root).ok()?;
            Some(below.join(file.strip_prefix(ancestor).ok()?))
        })
}

/// The relative path a row publishes for `file` under `root`, or `None` when
/// the located file lies outside the located root.
#[must_use]
pub fn relative_within(file: &Path, root: &Path) -> Option<PathBuf> {
    match place(file, root) {
        Placement::Inside(placed) => Some(placed.spelled),
        Placement::Escapes | Placement::Unrelated => None,
    }
}

/// Select the entries under `root` that `wanted` accepts, in database order.
///
/// `wanted` sees the entry and a relative path: the canonical one for an
/// entry inside the root, and the spelled one when deciding whether an
/// escaping entry would have been selected by its spelling. Such an entry is
/// refused with `outside(entry)`, the route's existing diagnostic. The
/// selected entries are then checked for aliases ([`refuse_aliases`]).
pub fn select_under<'a>(
    entries: &'a [CompileEntry],
    root: &Path,
    wanted: impl Fn(&CompileEntry, &Path) -> bool,
    outside: impl Fn(&CompileEntry) -> String,
) -> Result<Vec<&'a CompileEntry>, String> {
    let located_root = locate(root);
    let mut selected = Vec::new();
    for entry in entries {
        let located = locate(&entry.file);
        if let Ok(canonical) = located.path().strip_prefix(located_root.path()) {
            if wanted(entry, canonical) {
                selected.push(entry);
            }
            continue;
        }
        if let Some(spelled) = spelled_through(&entry.file, root, located_root.path())
            && wanted(entry, &spelled)
        {
            return Err(outside(entry));
        }
    }
    refuse_aliases(selected.iter().copied())?;
    Ok(selected)
}

/// Select the entries whose file locates to `target`, in database order,
/// refusing aliases.
pub fn select_exact<'a>(
    entries: &'a [CompileEntry],
    target: &Path,
) -> Result<Vec<&'a CompileEntry>, String> {
    let located_target = locate(target);
    let selected = entries
        .iter()
        .filter(|entry| entry.file == target || locate(&entry.file) == located_target)
        .collect::<Vec<_>>();
    refuse_aliases(selected.iter().copied())?;
    Ok(selected)
}

/// Refuse selected records that spell one source file two ways, or one
/// configuration's compile directory two ways.
///
/// Records repeating one spelling are not aliases: each route keeps its own
/// rule for those (the pinned route keeps the first, the configured routes
/// keep each distinct command). Two spellings of one file would otherwise be
/// two units of one source, or be merged by whichever spelling came first.
pub fn refuse_aliases<'a>(
    entries: impl IntoIterator<Item = &'a CompileEntry>,
) -> Result<(), String> {
    let mut files = BTreeMap::<PathBuf, BTreeSet<&Path>>::new();
    let mut directories = BTreeMap::<(PathBuf, PathBuf, &str), BTreeSet<&Path>>::new();
    for entry in entries {
        let file = locate(&entry.file).path().to_path_buf();
        files.entry(file.clone()).or_default().insert(&entry.file);
        directories
            .entry((
                file,
                locate(&entry.directory).path().to_path_buf(),
                &entry.command,
            ))
            .or_default()
            .insert(&entry.directory);
    }
    if let Some((file, spellings)) = files.iter().find(|(_, spellings)| spellings.len() > 1) {
        return Err(alias_error("source file", file, spellings));
    }
    if let Some(((file, _, _), spellings)) = directories
        .iter()
        .find(|(_, spellings)| spellings.len() > 1)
    {
        return Err(alias_error("compile directory of", file, spellings));
    }
    Ok(())
}

fn alias_error(what: &str, file: &Path, spellings: &BTreeSet<&Path>) -> String {
    format!(
        "{ALIAS_DIAGNOSTIC}: the selected compile records spell the {what} {} {} ways ({}); the audit neither merges nor double-counts them",
        file.display(),
        spellings.len(),
        spellings
            .iter()
            .map(|path| format!("`{}`", path.display()))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "qcore-input-paths-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn entry(directory: &Path, file: &Path) -> CompileEntry {
        CompileEntry {
            directory: directory.to_path_buf(),
            command: format!("clang -c {}", file.display()),
            file: file.to_path_buf(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_spelling_locates_to_the_real_root() {
        let scratch = Scratch::new("link");
        let real = scratch.0.join("real");
        fs::create_dir_all(real.join("src")).unwrap();
        fs::write(real.join("src/unit.c"), "int x;\n").unwrap();
        let old = scratch.0.join("old");
        std::os::unix::fs::symlink(&real, &old).unwrap();
        let spelled = old.join("src/unit.c");
        assert_eq!(
            locate(&spelled),
            Location::Resolved(real.join("src/unit.c"))
        );
        assert_eq!(
            place(&spelled, &real),
            Placement::Inside(Placed {
                canonical: "src/unit.c".into(),
                spelled: "src/unit.c".into(),
            })
        );
        assert_eq!(relative_within(&spelled, &real), Some("src/unit.c".into()));
        // A dangling file under the old spelling is located, not resolved.
        let missing = old.join("src/missing.c");
        assert_eq!(
            locate(&missing),
            Location::Unresolved(real.join("src/missing.c"))
        );
        assert!(!locate(&missing).is_resolved());
    }

    #[cfg(unix)]
    #[test]
    fn an_escaping_spelling_is_refused_with_the_callers_diagnostic() {
        let scratch = Scratch::new("escape");
        let root = scratch.0.join("root");
        let outside = scratch.0.join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("evil.c"), "int x;\n").unwrap();
        std::os::unix::fs::symlink(outside.join("evil.c"), root.join("evil.c")).unwrap();
        assert_eq!(place(&root.join("evil.c"), &root), Placement::Escapes);
        assert_eq!(place(&outside.join("evil.c"), &root), Placement::Unrelated);
        let entries = [entry(&root, &root.join("evil.c"))];
        let error =
            select_under(&entries, &root, |_, _| true, |_| "outside root".into()).unwrap_err();
        assert_eq!(error, "outside root");
    }

    #[cfg(unix)]
    #[test]
    fn two_spellings_of_one_file_are_an_alias_and_one_spelling_twice_is_not() {
        let scratch = Scratch::new("alias");
        let real = scratch.0.join("real");
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("unit.c"), "int x;\n").unwrap();
        let old = scratch.0.join("old");
        std::os::unix::fs::symlink(&real, &old).unwrap();
        let repeated = [
            entry(&real, &real.join("unit.c")),
            entry(&real, &real.join("unit.c")),
        ];
        assert_eq!(
            select_under(&repeated, &real, |_, _| true, |_| unreachable!())
                .unwrap()
                .len(),
            2
        );
        let aliased = [
            entry(&real, &real.join("unit.c")),
            entry(&real, &old.join("unit.c")),
        ];
        let error = select_under(&aliased, &real, |_, _| true, |_| unreachable!()).unwrap_err();
        assert!(error.starts_with(ALIAS_DIAGNOSTIC), "{error}");
        assert!(select_exact(&aliased, &real.join("unit.c")).is_err());
        // One configuration whose directory is spelled two ways.
        let mut moved = entry(&old, &real.join("unit.c"));
        moved.command = repeated[0].command.clone();
        let error = refuse_aliases([&repeated[0], &moved]).unwrap_err();
        assert!(error.contains("compile directory"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn a_relative_file_resolves_against_a_symlinked_directory_as_the_kernel_does() {
        let scratch = Scratch::new("relative");
        let real = scratch.0.join("real");
        fs::create_dir_all(real.join("build")).unwrap();
        fs::create_dir_all(real.join("src")).unwrap();
        fs::write(real.join("src/unit.c"), "int x;\n").unwrap();
        let old = scratch.0.join("old");
        std::os::unix::fs::symlink(&real, &old).unwrap();
        // What `compile_entries` makes of {"directory": OLD/build, "file": "../src/unit.c"}.
        let joined = old.join("build").join("../src/unit.c");
        assert_eq!(locate(&joined), Location::Resolved(real.join("src/unit.c")));
        let entries = [entry(&old.join("build"), &joined)];
        let selected =
            select_under(&entries, &real.join("src"), |_, _| true, |_| unreachable!()).unwrap();
        assert_eq!(selected.len(), 1);
        // The selected record keeps its spelling.
        assert_eq!(selected[0].file, joined);
        // A missing component before `..` does not resolve and is not
        // normalised away: the kernel would refuse that spelling too.
        let dangling = real.join("src/missing/../unit.c");
        assert_eq!(
            locate(&dangling),
            Location::Unresolved(real.join("src/missing/../unit.c"))
        );
    }
}
