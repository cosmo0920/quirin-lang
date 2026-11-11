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

#![cfg(unix)]

use qcore_c_gateway::boolean_source::{
    ChunkFrameSummary, ChunkSourceResult, ChunkSourceSummary, import_chunk_frame_helper,
    import_chunk_source_with_profile,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::TargetProfile;
use qcore_verification::source_effect::{BOOLEAN_FIELD_SEMANTIC_PROFILE, BooleanFieldSourceTask};
use std::path::{Path, PathBuf};
use std::process::Command;

// The wrapper is written by a separate process
// (`tests/support/process_fixture.rs`). This binary has one test, so no
// sibling spawns while it is written; the shared writer keeps it that way
// if a test is added.
#[path = "../../../tests/support/process_fixture.rs"]
mod process_fixture;

const CHILD_ROOT: &str = "QCORE_SOURCE_EFFECT_HELPER_TARGET_CHILD";
const RULES: &str = include_str!(
    "../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"
);
const CONTRACT: &str =
    include_str!("../../../providers/qcore-provider-chunkio/contracts/chunkio/lock_state.qv");
const HELPER: &str = "struct cio_chunk { int lock; int error_n; }; void cio_error_reset(struct cio_chunk *p) { p->error_n = 0; }";
const SOURCE: &str = "struct cio_chunk { int lock; int error_n; }; void cio_error_reset(struct cio_chunk *); int cio_chunk_unlock(struct cio_chunk *p) { cio_error_reset(p); if (p->lock == 0) { return -1; } p->lock = 0; return 0; }";

struct TemporaryDirectory(PathBuf);
impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[test]
fn helper_target_mismatch_is_rejected_in_isolated_process() {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        exercise_child(Path::new(&root));
        return;
    }
    // All environment selection is confined to the child. A phase file switches
    // that child's stable wrapper between two real Clang targets, allowing both
    // opaque imports to coexist without serializing or forging either result.
    let clang = qcore_c_gateway::clang_command().expect("Clang is required for source evidence");
    let root = TemporaryDirectory(std::env::temp_dir().join(format!(
        "qcore-helper-target-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )));
    std::fs::create_dir_all(&root.0).unwrap();
    let marker = root.0.join("native");
    let wrapper = root.0.join("clang-target-wrapper");
    let program = shell_quote(clang.get_program().to_str().unwrap());
    process_fixture::write_executable(
        &wrapper,
        format!(
            "#!/bin/sh\nif [ -f {} ]; then\nexec {program} \"$@\"\nfi\nexec {program} --target=wasm32-unknown-unknown \"$@\"\n",
            shell_quote(marker.to_str().unwrap())
        )
        .as_bytes(),
    );
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "helper_target_mismatch_is_rejected_in_isolated_process",
            "--nocapture",
        ])
        .env(CHILD_ROOT, &root.0)
        .env("QCORE_CLANG", &wrapper)
        .env("RUST_TEST_THREADS", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "isolated target test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn exercise_child(root: &Path) {
    let profile = load_profile_directory(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contract/profiles/fluent-bit"),
    )
    .unwrap();
    let revision = &profile.pin_for("chunkio").unwrap().git_commit;
    let helper_unit = C11TranslationUnit {
        source: HELPER,
        ..Default::default()
    };
    let alternate =
        import_chunk_frame_helper(&helper_unit, revision, revision, "cio_error_reset", RULES)
            .unwrap();
    assert_eq!(
        alternate.input_identity().compiler_target(),
        "wasm32-unknown-unknown"
    );
    std::fs::write(root.join("native"), b"native Clang from this point").unwrap();
    let native =
        import_chunk_frame_helper(&helper_unit, revision, revision, "cio_error_reset", RULES)
            .unwrap();
    assert_ne!(
        alternate.input_identity().compiler_target(),
        native.input_identity().compiler_target()
    );
    assert_eq!(
        serde_json::to_value(&alternate).unwrap(),
        serde_json::to_value(&native).unwrap(),
        "textual helper layout and paths alone cannot identify its target ABI"
    );
    let import = |helper: ChunkFrameSummary| -> ChunkSourceSummary {
        let mut results = import_chunk_source_with_profile(
            &C11TranslationUnit {
                source: SOURCE,
                ..Default::default()
            },
            revision,
            &profile,
            &["chunkio"],
            &[helper],
            RULES,
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        let ChunkSourceResult::Summary(summary) = results.remove(0) else {
            panic!("expected source paths for the complete supported function");
        };
        *summary
    };
    let compatible = import(native);
    let incompatible = import(alternate);
    assert_eq!(
        serde_json::to_value(&compatible).unwrap(),
        serde_json::to_value(&incompatible).unwrap()
    );
    let target = TargetProfile {
        library: "chunkio".into(),
        revision: revision.clone(),
        profile: BOOLEAN_FIELD_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    };
    let valid =
        BooleanFieldSourceTask::derive(&compatible, "fixture.c", CONTRACT, target.clone()).unwrap();
    assert_eq!(valid.evaluate().unwrap().checked_states(), 4);
    let rejected = BooleanFieldSourceTask::derive(&incompatible, "fixture.c", CONTRACT, target);
    assert!(
        matches!(rejected, Err(reason) if reason == "source-effect helper compiler target differs from the importing unit")
    );
}
