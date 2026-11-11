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

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_compiler::{load_first_party_manifests, load_first_party_registry};

#[path = "../../tests/support/tracked_files.rs"]
mod tracked_files;

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        // Card W45-TEST-HYGIENE: the fixture is the tracked profile tree; an
        // untracked or ignored file under the profiles root is not copied.
        fn copy(source: &Path, target: &Path) {
            for path in tracked_files::tracked_files(source) {
                let destination = target.join(path.strip_prefix(source).unwrap());
                fs::create_dir_all(destination.parent().unwrap()).unwrap();
                fs::copy(&path, destination).unwrap();
            }
        }
        let root = std::env::temp_dir().join(format!(
            "qcore-h6-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        copy(&qcore_compiler::source_profiles::profiles_root(), &root);
        Self(root)
    }
    fn mutate(&self, relative: &str, change: impl FnOnce(&mut serde_json::Value)) {
        let path = self.0.join(relative);
        let mut value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        change(&mut value);
        fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn provider_metadata_is_discovered_in_plugin_id_order() {
    let fixture = Fixture::new();
    fixture.mutate("libevent/providers/qcore.libevent.json", |manifest| {
        manifest["package_version"] = "0.1.1".into();
    });
    let manifests = load_first_party_manifests(&fixture.0).unwrap();
    let ids = manifests
        .iter()
        .map(|m| m.plugin_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            "qcore.chunkio",
            "qcore.cmetrics",
            "qcore.ctraces",
            "qcore.libevent",
            "qcore.librdkafka"
        ]
    );
    assert_eq!(manifests[3].package_version, "0.1.1");
    let registry = load_first_party_registry(&fixture.0).unwrap();
    assert_eq!(registry.manifests(), manifests.iter().collect::<Vec<_>>());
}

#[test]
fn malformed_or_missing_provider_rejects_the_entire_registry() {
    let fixture = Fixture::new();
    fixture.mutate("libevent/providers/qcore.libevent.json", |manifest| {
        manifest["unrecognized"] = true.into();
    });
    assert!(load_first_party_registry(&fixture.0).is_err());
    fs::remove_file(fixture.0.join("libevent/providers/qcore.libevent.json")).unwrap();
    assert!(load_first_party_registry(&fixture.0).is_err());
    assert!(load_first_party_registry(&fixture.0.join("absent")).is_err());
}

#[test]
fn duplicate_provider_across_source_sets_is_an_error() {
    let fixture = Fixture::new();
    fs::copy(
        fixture.0.join("fluent-bit/providers/qcore.cmetrics.json"),
        fixture.0.join("libevent/providers/duplicate.json"),
    )
    .unwrap();
    fixture.mutate("libevent/profile.json", |profile| {
        profile["provider_manifests"]
            .as_array_mut()
            .unwrap()
            .push("providers/duplicate.json".into());
    });
    let error = load_first_party_registry(&fixture.0)
        .err()
        .expect("duplicates reject");
    assert!(
        error
            .message
            .contains("duplicate provider `qcore.cmetrics`")
    );
}

#[test]
fn provider_packages_keep_their_manifest_directory() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.0.join("libevent/providers/nested")).unwrap();
    fs::rename(
        fixture.0.join("libevent/providers/qcore.libevent.json"),
        fixture
            .0
            .join("libevent/providers/nested/qcore.libevent.json"),
    )
    .unwrap();
    fixture.mutate("libevent/profile.json", |profile| {
        profile["provider_manifests"] = serde_json::json!(["providers/nested/qcore.libevent.json"]);
    });
    let profile =
        qcore_compiler::profile_loader::load_profile_directory(&fixture.0.join("libevent"))
            .unwrap();
    let id = qcore_compiler::PluginId::new("qcore.libevent").unwrap();
    assert_eq!(
        profile.provider_directory(&id),
        Some(fixture.0.join("libevent/providers/nested").as_path())
    );
    assert!(load_first_party_registry(&fixture.0).is_ok());
}
