//! Card **W45-FB-SYSTEM-SELECTION**: an explicit translation-unit selection for
//! `quirin-audit current` (`--selection FILE`, or `selection` in a current
//! configuration).
//!
//! A selection file replaces the source profile's `audit.json` library groups
//! with named groups of **exact files**. The profile itself is unchanged: its
//! pins, its library entries and its declared manifest still apply, so a unit
//! selected both here and by the profile's own groups is analysed with the
//! same inputs and the same compile-task identity (the selection is not a
//! verdict dependency). Only the choice of units differs.
//!
//! Fail-closed and denominator-preserving:
//!
//! * the file is validated before any unit runs: schema version 1, a nonempty
//!   name, nonempty groups with distinct libraries and labels, and nonempty,
//!   distinct, relative `.c` paths with no `.`/`..`/empty component;
//! * every group's library must exist in the profile;
//! * a selected file keeps **every** distinct compilation configuration the
//!   database holds for it (static and shared variants stay separate units);
//! * a selected file with **no** compilation record is not dropped and does not
//!   refuse the run: it stays in the denominator as one `Unknown` unit whose
//!   reason is `EnvironmentUnavailable` (no record, or no such file in the
//!   checkout).
//!
//! The selection's path, digest and contents are recorded in the run's
//! `.invocation.json` and `.contracts.json` sidecars.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use qcore_contract::profile_loader::sha256_hex;
use serde::Deserialize;
use serde_json::{Value, json};

/// One group of a selection: a profile library, an optional report label and
/// the exact files, relative to the checkout root, in audit order.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedGroup {
    pub library: String,
    #[serde(default)]
    pub label: Option<String>,
    pub files: Vec<String>,
}

impl SelectedGroup {
    /// The report label: the declared label, or the library name.
    #[must_use]
    pub fn label(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.library)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionDocument {
    schema_version: u32,
    name: String,
    #[serde(default)]
    description: Option<String>,
    groups: Vec<SelectedGroup>,
}

/// A validated selection file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitSelection {
    pub path: PathBuf,
    pub sha256: String,
    pub name: String,
    pub description: Option<String>,
    pub groups: Vec<SelectedGroup>,
}

impl UnitSelection {
    /// Read and validate a selection file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        Self::parse(path, &bytes)
    }

    /// Validate a selection document read from `path`.
    pub fn parse(path: &Path, bytes: &[u8]) -> Result<Self, String> {
        let at = |message: String| format!("{}: {message}", path.display());
        let document: SelectionDocument =
            serde_json::from_slice(bytes).map_err(|error| at(error.to_string()))?;
        if document.schema_version != 1 {
            return Err(at(format!(
                "unsupported unit selection version {}",
                document.schema_version
            )));
        }
        if document.name.trim().is_empty() {
            return Err(at("a unit selection needs a nonempty name".into()));
        }
        if document.groups.is_empty() {
            return Err(at("a unit selection needs at least one group".into()));
        }
        let mut libraries = BTreeSet::new();
        let mut labels = BTreeSet::new();
        let mut files = BTreeSet::new();
        for group in &document.groups {
            if group.library.trim().is_empty()
                || group
                    .label
                    .as_ref()
                    .is_some_and(|label| label.trim().is_empty())
            {
                return Err(at(
                    "a selection group needs a nonempty library and, when given, a nonempty label"
                        .into(),
                ));
            }
            if !libraries.insert(group.library.as_str()) || !labels.insert(group.label()) {
                return Err(at(format!(
                    "selection groups need distinct libraries and labels; `{}` repeats",
                    group.library
                )));
            }
            if group.files.is_empty() {
                return Err(at(format!(
                    "selection group `{}` selects no file",
                    group.label()
                )));
            }
            for file in &group.files {
                check_relative_source(file).map_err(&at)?;
                if !files.insert(file.as_str()) {
                    return Err(at(format!("`{file}` is selected twice")));
                }
            }
        }
        Ok(Self {
            path: path.to_path_buf(),
            sha256: sha256_hex(bytes),
            name: document.name,
            description: document.description,
            groups: document.groups,
        })
    }

    /// The record the sidecars carry.
    #[must_use]
    pub fn record(&self) -> Value {
        json!({
            "path": self.path,
            "sha256": self.sha256,
            "name": self.name,
            "description": self.description,
            "groups": self.groups.iter().map(|group| json!({
                "library": group.library, "label": group.label(), "files": group.files,
            })).collect::<Vec<_>>(),
            "scope": "an explicit unit choice over the unchanged source profile; it is not a verdict dependency, and a selected file without a compilation record stays in the denominator as Unknown",
        })
    }
}

/// A selected file must be a plain relative `.c` path under the checkout.
fn check_relative_source(file: &str) -> Result<(), String> {
    let path = Path::new(file);
    // Checked on the spelling, not on `Path::components`, which silently drops
    // an interior `.` and would admit `src/./a.c` as `src/a.c`.
    let plain = !file.contains('\\')
        && file
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if !plain {
        return Err(format!(
            "`{file}` is not a plain relative path under the checkout (no absolute, `.`, `..` or \
             empty component)"
        ));
    }
    if path.extension().is_none_or(|extension| extension != "c") {
        return Err(format!("`{file}` is not a C source file"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(value: &Value) -> Result<UnitSelection, String> {
        UnitSelection::parse(
            Path::new("/sel.json"),
            serde_json::to_string(value).unwrap().as_bytes(),
        )
    }

    fn group(files: &[&str]) -> Value {
        json!({"library": "lib", "label": "slice", "files": files})
    }

    #[test]
    fn a_valid_selection_keeps_its_order_label_and_digest() {
        let value = json!({"schema_version": 1, "name": "slice", "groups": [
            group(&["src/b.c", "plugins/x/a.c"]),
            {"library": "other", "files": ["src/c.c"]},
        ]});
        let selection = parse(&value).unwrap();
        assert_eq!(selection.groups[0].files, ["src/b.c", "plugins/x/a.c"]);
        assert_eq!(selection.groups[0].label(), "slice");
        assert_eq!(selection.groups[1].label(), "other");
        assert_eq!(
            selection.sha256,
            sha256_hex(serde_json::to_string(&value).unwrap().as_bytes())
        );
        assert_eq!(selection.record()["groups"][1]["label"], "other");
    }

    #[test]
    fn malformed_selections_refuse() {
        for (value, expected) in [
            (
                json!({"schema_version": 2, "name": "s", "groups": [group(&["a.c"])]}),
                "unsupported unit selection version",
            ),
            (
                json!({"schema_version": 1, "name": " ", "groups": [group(&["a.c"])]}),
                "nonempty name",
            ),
            (
                json!({"schema_version": 1, "name": "s", "groups": []}),
                "at least one group",
            ),
            (
                json!({"schema_version": 1, "name": "s", "groups": [group(&[])]}),
                "selects no file",
            ),
            (
                json!({"schema_version": 1, "name": "s", "groups": [group(&["a.c"]), group(&["b.c"])]}),
                "distinct libraries and labels",
            ),
            (
                json!({"schema_version": 1, "name": "s", "groups": [
                    group(&["a.c"]), {"library": "other", "label": "slice", "files": ["b.c"]}]}),
                "distinct libraries and labels",
            ),
            (
                json!({"schema_version": 1, "name": "s", "groups": [group(&["a.c", "a.c"])]}),
                "selected twice",
            ),
            (
                json!({"schema_version": 1, "name": "s", "groups": [group(&["a.h"])]}),
                "not a C source file",
            ),
            (
                json!({"schema_version": 1, "name": "s", "groups": [{"library": "", "files": ["a.c"]}]}),
                "nonempty library",
            ),
            (
                json!({"schema_version": 1, "name": "s", "groups": [group(&["a.c"])], "extra": 1}),
                "unknown field",
            ),
        ] {
            let error = parse(&value).unwrap_err();
            assert!(error.contains(expected), "{expected}: {error}");
        }
    }

    #[test]
    fn only_plain_relative_paths_are_selectable() {
        for file in [
            "/abs/a.c",
            "../a.c",
            "src/../a.c",
            "./a.c",
            "src/./a.c",
            "src//a.c",
            "src\\a.c",
            "src/",
            "",
        ] {
            assert!(check_relative_source(file).is_err(), "{file:?}");
        }
        assert!(check_relative_source("plugins/in_storage_backlog/sb.c").is_ok());
    }
}
