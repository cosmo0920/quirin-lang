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

//! Test-only discovery of the single canonical source for each logical module.
//!
//! Card CONTRACT-INHERIT-ADOPTION: `contract/corpus.json` schema 2 names the
//! published load path beside its roots. Roots resolve exactly as in schema 1;
//! the load path lets a consumer read a module that uses `#include` /
//! `contract inherit` as its elaboration ([`text`]), which is the text every
//! loader parses and retains, instead of the written form no parser admits.
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use qcore_contract::contract_inclusion::elaborate_contract_module;

#[path = "../../../tests/support/tracked_files.rs"]
mod tracked_files;
use qcore_contract::load_path_manifest::{LoadPathManifest, load_contract_load_path_manifest};

fn index() -> (PathBuf, PathBuf, serde_json::Value) {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let index = repository.join("contract/corpus.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&index).unwrap()).unwrap();
    let keys: Vec<_> = manifest.as_object().unwrap().keys().cloned().collect();
    match manifest["schema_version"].as_u64() {
        Some(1) => assert_eq!(keys, ["roots", "schema_version"]),
        Some(2) => assert_eq!(keys, ["load_path", "roots", "schema_version"]),
        other => panic!("unsupported corpus index schema {other:?}"),
    }
    (repository, index, manifest)
}

#[allow(dead_code)]
pub fn sources() -> BTreeMap<String, PathBuf> {
    let (repository, index, manifest) = index();
    let mut sources = BTreeMap::new();
    for root in manifest["roots"].as_array().unwrap() {
        let root = index.parent().unwrap().join(root.as_str().unwrap());
        assert!(
            root.is_dir() && !root.is_symlink(),
            "invalid corpus root: {}",
            root.display()
        );
        assert!(
            root.canonicalize()
                .unwrap()
                .starts_with(repository.canonicalize().unwrap()),
            "corpus root escapes repository: {}",
            root.display()
        );
        // Card W45-TEST-HYGIENE: the corpus is the tracked tree under each
        // root; an untracked or ignored `.qcore` there is not a module.
        for path in tracked_files::tracked_files(&root) {
            assert!(
                !path.is_symlink(),
                "symbolic link in corpus: {}",
                path.display()
            );
            if path
                .extension()
                .is_some_and(|ext| ext == "qcore" || ext == "qv")
            {
                let logical = path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                assert!(
                    sources.insert(logical.clone(), path).is_none(),
                    "duplicate logical contract: {logical}"
                );
            }
        }
    }
    sources
}

/// The load path the corpus index names (schema 2), read and validated.
#[allow(dead_code)]
pub fn load_path() -> Option<LoadPathManifest> {
    let (_, index, manifest) = index();
    let relative = manifest.get("load_path")?.as_str().unwrap().to_owned();
    Some(
        load_contract_load_path_manifest(&index.parent().unwrap().join(relative))
            .unwrap_or_else(|e| panic!("{e}")),
    )
}

/// The crate owning the corpus module at `path` (logical path `logical`), if
/// its root is a load-path crate's contract directory.
#[allow(dead_code)]
pub fn owning_crate(load_path: &LoadPathManifest, logical: &str, path: &Path) -> Option<String> {
    let mut root = path.to_path_buf();
    for _ in logical.split('/') {
        root.pop();
    }
    load_path.crate_for_root(&root).map(str::to_owned)
}

/// The text a loader reads for a corpus module: the written text, or, for a
/// module that uses contract inclusion, its elaboration on the corpus load
/// path. A module using inclusion outside a load-path crate panics: no
/// consumer could read it.
#[allow(dead_code)]
pub fn text(logical: &str, path: &Path) -> String {
    let written = fs::read_to_string(path).unwrap();
    if !qcore_frontend::contract_inclusion::contains_contract_inclusion_syntax(&written) {
        return written;
    }
    let load_path = load_path().expect("a module uses inclusion but the corpus names no load path");
    let krate = owning_crate(&load_path, logical, path)
        .unwrap_or_else(|| panic!("{logical}: inclusion outside a load-path crate"));
    elaborate_contract_module(&load_path.load_path, &krate, logical)
        .unwrap_or_else(|e| panic!("{logical}: {e}"))
        .text
}
