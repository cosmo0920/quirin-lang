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

//! Card CONTRACT-SUPERSESSION (contract spec §C06.6, user decision 9a):
//! lock format version 4 (pin stamps and the lock set's Fluent Bit revision),
//! reading versions 1 to 3 for history, the stamps an audit reads from the
//! locks, the loader's supersession through them, and the lock-set window.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_audit::contract_revision::{ContractLock, LOCK_VERSION, LockStamps, ModuleOwners};
use qcore_audit::pin_window::{self, PinLockWindow};
use qcore_contract::contract_loader::{
    check_source_assumptions, load_resource_contract_manifest_text_stamped,
};
use qcore_contract::contract_revision::module_declaration_digests;
use qcore_contract::contract_supersession::DeclarationStamps;
use qcore_schemas::contract_supersession::{ConflictReason, StampStanding};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "qcore-contract-supersession-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

const FB: &str = "046accf6aded7906f66a1baf0202da349e521736";

fn module(function: &str, mode: &str) -> String {
    format!(
        "module contract;

extern fn {function}(h: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal {mode};
    ensures normal resource_return_none();
}};
"
    )
}

fn digest(text: &str) -> String {
    module_declaration_digests("m", text).unwrap()[0]
        .contract_sha256
        .clone()
}

/// A lock document of `version` with one pinned entry for `function`.
fn lock(version: u32, function: &str, sha: &str, stamp: Option<&str>, tie: Option<&str>) -> String {
    let mut entry = serde_json::json!({
        "function": function,
        "module": "contracts/m.qv",
        "contract_sha256": sha,
        "identities": [{
            "reviewed_at": [{"kind": "git", "revision": FB}],
            "sites": []
        }]
    });
    if let Some(stamp) = stamp {
        entry["pinned_at"] = stamp.into();
    }
    let mut lock = serde_json::json!({
        "lock_version": version,
        "generated_by": "quirin-audit pin",
        "note": "",
        "owner": "crate",
        "normalisation": "c-tokens-v1",
        "pinned": [entry],
        "flexible": []
    });
    if let Some(tie) = tie {
        lock["fluent_bit_revision"] = tie.into();
    }
    serde_json::to_string_pretty(&lock).unwrap()
}

/// A crate directory: `Cargo.toml`, `contracts/m.qv` and, when given, a lock.
fn crate_dir(root: &Path, name: &str, text: &str, lock: Option<String>) -> PathBuf {
    let directory = root.join(name);
    fs::create_dir_all(directory.join("contracts")).unwrap();
    fs::write(directory.join("Cargo.toml"), "[package]\n").unwrap();
    fs::write(directory.join("contracts/m.qv"), text).unwrap();
    if let Some(lock) = lock {
        fs::write(directory.join("contracts.lock"), lock).unwrap();
    }
    directory.join("contracts/m.qv")
}

#[test]
fn version_four_round_trips_and_older_versions_are_read_without_stamps() {
    let root = scratch("versions");
    let sha = "a".repeat(64);
    let path = root.join("contracts.lock");
    // Version 4 with a stamp and a tie.
    fs::write(
        &path,
        lock(4, "f", &sha, Some("2026-10-25T18:10:00Z"), Some(FB)),
    )
    .unwrap();
    let read = ContractLock::read(&path).unwrap();
    assert_eq!(read.lock_version, LOCK_VERSION);
    assert_eq!(read.fluent_bit_revision.as_deref(), Some(FB));
    assert_eq!(
        read.pinned[0].pinned_at.as_deref(),
        Some("2026-10-25T18:10:00Z")
    );
    fs::write(&path, read.to_text().unwrap()).unwrap();
    assert_eq!(ContractLock::read(&path).unwrap(), read);
    // Version 2 is read for history, with no stamp, and written as 4.
    fs::write(&path, lock(2, "f", &sha, None, None)).unwrap();
    let old = ContractLock::read(&path).unwrap();
    assert_eq!(old.lock_version, 2);
    assert_eq!(old.pinned[0].pinned_at, None);
    assert!(old.to_text().unwrap().contains("\"lock_version\": 4"));
    // A version-2 lock cannot carry a stamp or a tie.
    for text in [
        lock(2, "f", &sha, Some("2026-10-25T18:10:00Z"), None),
        lock(2, "f", &sha, None, Some(FB)),
    ] {
        fs::write(&path, text).unwrap();
        assert!(ContractLock::read(&path).unwrap_err().contains("version 4"));
    }
    // A tie must be a full revision; a version beyond 4 is refused.
    fs::write(&path, lock(4, "f", &sha, None, Some("046accf6"))).unwrap();
    assert!(ContractLock::read(&path).is_err());
    fs::write(&path, lock(5, "f", &sha, None, None)).unwrap();
    assert!(ContractLock::read(&path).is_err());
    // Version 1 is still migrated.
    fs::write(
        &path,
        serde_json::to_string(&serde_json::json!({
            "lock_version": 1, "generated_by": "quirin-audit pin", "note": "", "owner": "crate",
            "source": {"kind": "git", "revision": FB, "normalisation": "c-tokens-v1"},
            "pinned": [{"function": "f", "module": "m.qv", "contract_sha256": sha, "sites": []}],
            "flexible": []
        }))
        .unwrap(),
    )
    .unwrap();
    let migrated = ContractLock::read(&path).unwrap();
    assert_eq!(migrated.pinned[0].pinned_at, None);
    assert_eq!(migrated.fluent_bit_revision, None);
}

#[test]
fn an_audit_reads_each_declaration_stamp_from_its_lock() {
    let root = scratch("stamps");
    let text = module("f", "resource_borrowed(h)");
    let sha = digest(&text);
    let owners = ModuleOwners::default();
    let cases = [
        (
            Some(lock(4, "f", &sha, Some("2026-10-25T18:10:00Z"), Some(FB))),
            StampStanding::Stamped("2026-10-25T18:10:00Z".into()),
        ),
        (Some(lock(2, "f", &sha, None, None)), StampStanding::Missing),
        (
            Some(lock(4, "f", &sha, Some("today"), Some(FB))),
            StampStanding::Malformed("today".into()),
        ),
        (
            Some(lock(
                4,
                "f",
                &"b".repeat(64),
                Some("2026-10-25T18:10:00Z"),
                Some(FB),
            )),
            StampStanding::Stale,
        ),
        (
            Some(lock(4, "g", &sha, Some("2026-10-25T18:10:00Z"), Some(FB))),
            StampStanding::Unpinned,
        ),
        (None, StampStanding::Unpinned),
    ];
    for (index, (lock, expected)) in cases.into_iter().enumerate() {
        let module = crate_dir(&root, &format!("c{index}"), &text, lock);
        let stamps = LockStamps::new(&owners);
        assert_eq!(
            stamps.standing(&module.display().to_string(), "f", &sha),
            expected,
            "case {index}"
        );
    }
}

/// Two crates declare `f`; a manifest selects both modules.
fn two_bundles(root: &Path, a: Option<&str>, b: Option<&str>) -> (PathBuf, String) {
    let releasing = module("f", "resource_released(h, nonnull)");
    let borrowing = module("f", "resource_borrowed(h)");
    crate_dir(
        root,
        "a",
        &releasing,
        Some(lock(4, "f", &digest(&releasing), a, Some(FB))),
    );
    crate_dir(
        root,
        "b",
        &borrowing,
        Some(lock(4, "f", &digest(&borrowing), b, Some(FB))),
    );
    let manifest = root.join("manifest.json");
    let text = serde_json::json!({
        "schema_version": 1,
        "modules": ["a/contracts/m.qv", "b/contracts/m.qv"],
    })
    .to_string();
    fs::write(&manifest, &text).unwrap();
    (manifest, text)
}

#[test]
fn the_loader_decides_duplicates_by_the_lock_stamps() {
    let owners = ModuleOwners::default();
    // The newer stamp wins, whichever bundle holds it.
    for (a, b, winner) in [
        ("2026-10-02T00:00:00Z", "2026-10-01T00:00:00Z", "a"),
        ("2026-10-01T00:00:00Z", "2026-10-02T00:00:00Z", "b"),
    ] {
        let root = scratch("decide");
        let (manifest, text) = two_bundles(&root, Some(a), Some(b));
        let contracts = load_resource_contract_manifest_text_stamped(
            &manifest,
            &text,
            &LockStamps::new(&owners),
        )
        .unwrap();
        check_source_assumptions(&contracts).unwrap();
        let superseded = &contracts.superseded()["f"];
        assert_eq!(superseded.len(), 1);
        assert!(
            superseded[0].by_module.contains(&format!("/{winner}/")),
            "{superseded:?}"
        );
        assert!(contracts.contract_conflict().is_empty());
        assert!(contracts.contract("f").is_some());
    }
    // Equal stamps and a missing stamp withhold the function.
    for (a, b, reason) in [
        (
            Some("2026-10-01T00:00:00Z"),
            Some("2026-10-01T00:00:00Z"),
            ConflictReason::EqualStamps,
        ),
        (
            Some("2026-10-01T00:00:00Z"),
            None,
            ConflictReason::MissingStamp,
        ),
    ] {
        let root = scratch("conflict");
        let (manifest, text) = two_bundles(&root, a, b);
        let contracts = load_resource_contract_manifest_text_stamped(
            &manifest,
            &text,
            &LockStamps::new(&owners),
        )
        .unwrap();
        check_source_assumptions(&contracts).unwrap();
        assert_eq!(contracts.contract_conflict()["f"].reason, reason);
        assert!(contracts.contract("f").is_none());
        assert!(contracts.superseded().is_empty());
    }
}

/// A provider tree with one crate's live lock tied to `tie`.
fn provider_tree(tie: &str) -> PathBuf {
    let root = scratch("window");
    fs::create_dir_all(root.join("crate")).unwrap();
    fs::write(
        root.join("crate/contracts.lock"),
        lock(
            4,
            "f",
            &"a".repeat(64),
            Some("2026-10-25T18:10:00Z"),
            Some(tie),
        ),
    )
    .unwrap();
    root
}

fn revision(n: u8) -> String {
    format!("{n:x}").repeat(40)
}

fn tie_of(root: &Path) -> String {
    ContractLock::read(&root.join("crate/contracts.lock"))
        .unwrap()
        .fluent_bit_revision
        .unwrap()
}

#[test]
fn the_window_keeps_at_most_three_lock_sets_on_disk() {
    let root = provider_tree(&revision(1));
    assert!(
        pin_window::create(&root, &revision(2)).is_err(),
        "tied elsewhere"
    );
    pin_window::create(&root, &revision(1)).unwrap();
    assert!(pin_window::check(&root).unwrap().is_empty());
    assert!(pin_window::retire(&root).is_err(), "the live set stays");
    pin_window::add(&root, &revision(2), false).unwrap();
    pin_window::add(&root, &revision(3), false).unwrap();
    assert_eq!(tie_of(&root), revision(3));
    assert!(pin_window::check(&root).unwrap().is_empty());
    assert!(
        root.join(format!(
            "pin-lock-sets/{}/crate/contracts.lock",
            revision(1)
        ))
        .is_file()
    );
    // A fourth set is refused, and nothing is written.
    let before = fs::read(pin_window::manifest_path(&root)).unwrap();
    let error = pin_window::add(&root, &revision(4), false).unwrap_err();
    assert!(error.contains("retire the oldest"), "{error}");
    assert_eq!(fs::read(pin_window::manifest_path(&root)).unwrap(), before);
    assert_eq!(tie_of(&root), revision(3));
    // The re-base step retires the oldest and adds the new set.
    let (window, retired) = pin_window::add(&root, &revision(4), true).unwrap();
    assert_eq!(retired.unwrap().fluent_bit_revision, revision(1));
    assert_eq!(window.latest(), revision(4));
    assert!(!root.join(format!("pin-lock-sets/{}", revision(1))).exists());
    assert!(pin_window::check(&root).unwrap().is_empty());
    let read = PinLockWindow::read(&pin_window::manifest_path(&root)).unwrap();
    assert_eq!(
        read.sets
            .iter()
            .map(|set| set.fluent_bit_revision.clone())
            .collect::<Vec<_>>(),
        vec![revision(2), revision(3), revision(4)]
    );
    // An edited archive, a stray archive and a live lock tied elsewhere are
    // reported.
    let archived = root.join(format!(
        "pin-lock-sets/{}/crate/contracts.lock",
        revision(2)
    ));
    fs::write(&archived, "{}").unwrap();
    fs::create_dir_all(root.join(format!("pin-lock-sets/{}", revision(9)))).unwrap();
    fs::write(
        root.join("crate/contracts.lock"),
        lock(4, "f", &"a".repeat(64), None, Some(&revision(2))),
    )
    .unwrap();
    let problems = pin_window::check(&root).unwrap();
    assert_eq!(problems.len(), 3, "{problems:?}");
}
