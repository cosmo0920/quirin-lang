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

//! Card W45-TEST-HYGIENE: the tracked-file helper every tree-walking test
//! uses (`tests/support/tracked_files.rs`) lists exactly what `git` tracks,
//! including a file only added to the index, and fails closed outside a git
//! checkout instead of walking the filesystem.
use std::fs;
use std::panic;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "../../tests/support/tracked_files.rs"]
mod tracked_files;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-tracked-inputs-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.0)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn relative(root: &Path, files: &[PathBuf]) -> Vec<String> {
    files
        .iter()
        .map(|path| {
            path.strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

fn panic_text(result: std::thread::Result<Vec<PathBuf>>) -> String {
    let payload = result.expect_err("the helper must fail the test");
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
        .unwrap_or_default()
}

/// Tracked and index-only files are listed; untracked, ignored and deleted
/// ones are not, whatever their extension or depth.
#[test]
fn only_tracked_files_are_listed() {
    let scratch = Scratch::new("listing");
    scratch.git(&["init", "-q"]);
    scratch.write(".gitignore", "/private/\n");
    scratch.write("examples/a.qcore", "a\n");
    scratch.write("examples/nested/b.qcore", "b\n");
    scratch.write("examples/gone.qcore", "gone\n");
    scratch.git(&["add", "."]);
    // Added to the index after the others, never committed: still tracked.
    scratch.write("examples/staged.qcore", "staged\n");
    scratch.git(&["add", "examples/staged.qcore"]);
    fs::remove_file(scratch.0.join("examples/gone.qcore")).unwrap();
    scratch.write("examples/untracked.qcore", "untracked\n");
    scratch.write("examples/export/copy.qr", "untracked copy\n");
    scratch.write("private/export/examples/a.qcore", "ignored copy\n");

    let root = tracked_files::tracked_files(&scratch.0);
    assert_eq!(
        relative(&scratch.0, &root),
        [
            ".gitignore",
            "examples/a.qcore",
            "examples/nested/b.qcore",
            "examples/staged.qcore"
        ]
    );
    let examples = scratch.0.join("examples");
    assert_eq!(
        relative(&examples, &tracked_files::tracked_files(&examples)),
        ["a.qcore", "nested/b.qcore", "staged.qcore"]
    );
}

/// Outside a git checkout (an exported tree) the helper fails the test with
/// its reason; it never walks the filesystem instead.
#[test]
fn an_export_fails_closed() {
    let scratch = Scratch::new("export");
    scratch.write("examples/a.qcore", "a\n");
    // The scratch directory is under the system temporary directory, outside
    // any checkout, like a tarball unpacked there. (Were the temporary
    // directory inside a checkout, nothing would be tracked here and the
    // helper would fail closed on that instead; either way it fails.)
    let text = panic_text(panic::catch_unwind(|| {
        tracked_files::tracked_files(&scratch.0)
    }));
    assert!(text.contains("W45-TEST-HYGIENE"), "{text}");
    assert!(
        text.contains("not inside a git checkout")
            || text.contains("`git ls-files` failed")
            || text.contains("git tracks no file"),
        "{text}"
    );
}

/// A directory with nothing tracked (an ignored export, a wrong path) fails
/// rather than answering with an empty census.
#[test]
fn a_directory_with_nothing_tracked_fails_closed() {
    let scratch = Scratch::new("empty");
    scratch.git(&["init", "-q"]);
    scratch.write(".gitignore", "/private/\n");
    scratch.git(&["add", ".gitignore"]);
    scratch.write("private/export/a.qcore", "ignored\n");
    let text = panic_text(panic::catch_unwind(|| {
        tracked_files::tracked_files(&scratch.0.join("private"))
    }));
    assert!(text.contains("git tracks no file"), "{text}");
}

/// The repository itself: this file is tracked, and nothing under an ignored
/// directory is ever listed.
#[test]
fn the_repository_listing_holds_this_file_and_no_ignored_tree() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let files = relative(root, &tracked_files::tracked_files(root));
    assert!(
        files
            .iter()
            .any(|file| file == "compiler/tests/tracked_inputs.rs")
    );
    for ignored in [
        "target/",
        "private/",
        "measure/",
        "build/cycle19/",
        ".claude/",
    ] {
        assert!(
            !files.iter().any(|file| file.starts_with(ignored)),
            "{ignored} is not tracked"
        );
    }
}
