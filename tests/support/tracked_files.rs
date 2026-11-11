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

//! Tracked-file discovery for tests that find their inputs by walking the
//! repository (card W45-TEST-HYGIENE; the rule is "Tracked test inputs" in
//! `docs/agents/validation.md`).
//!
//! A test that walked the working tree read whatever lay there: ignored
//! `private/`, `build/` and `target/` content, untracked exports and other
//! worktrees. That failed a correct tree on 2026-09-26 (an export under
//! `private/` gave `std_socket_inet` two more address-host claimants) and
//! could as well let a stale copy satisfy a check. This lists what `git`
//! tracks instead, so ignored and untracked content never changes a result.
//!
//! Fail-closed: when `git` cannot run, or `directory` is not inside a git
//! checkout (an exported tarball), or nothing is tracked there, the calling
//! test panics with the reason. It never falls back to walking the
//! filesystem. Included with `#[path]`; this directory is not a test target.
use std::path::{Path, PathBuf};
use std::process::Command;

/// Every file `git ls-files` lists under `directory`, as `directory` joined
/// with its path, sorted. The index is the source, so a file added with
/// `git add` and not yet committed is listed; submodule entries are skipped
/// (no caller needs submodule content), and so is a tracked path deleted from
/// the working tree, exactly as a walk would have missed it.
#[allow(dead_code)]
pub fn tracked_files(directory: &Path) -> Vec<PathBuf> {
    let checkout = directory
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .unwrap_or_else(|| {
            panic!(
                "{} is not inside a git checkout (W45-TEST-HYGIENE)",
                directory.display()
            )
        });
    let canonical_checkout = checkout.canonicalize().unwrap_or_else(|error| {
        panic!(
            "cannot canonicalize git checkout {}: {error} (W45-TEST-HYGIENE)",
            checkout.display()
        )
    });
    let canonical_text = canonical_checkout.to_string_lossy();
    let safe_directory = canonical_text
        .strip_prefix(r"\\?\")
        .unwrap_or(&canonical_text)
        .replace('\\', "/");
    let output = Command::new("git")
        .args(["-c", &format!("safe.directory={safe_directory}")])
        .arg("-C")
        .arg(directory)
        .args(["ls-files", "-z", "--cached", "--", "."])
        // A caller's own git context (for example a hook) must not redirect
        // the listing to another repository or index.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "this test discovers its inputs with `git ls-files`, which could not run \
                 ({error}); it needs `git` and a git checkout, and never walks the \
                 filesystem instead (W45-TEST-HYGIENE)"
            )
        });
    assert!(
        output.status.success(),
        "`git ls-files` failed under {} ({}): this test discovers its inputs from \
         tracked files only and must run in a git checkout, not an export \
         (W45-TEST-HYGIENE):\n{}",
        directory.display(),
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let mut files: Vec<PathBuf> = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            directory.join(std::str::from_utf8(entry).unwrap_or_else(|_| {
                panic!("a tracked path under {} is not UTF-8", directory.display())
            }))
        })
        .filter(|path| {
            path.symlink_metadata()
                .is_ok_and(|metadata| !metadata.is_dir())
        })
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "git tracks no file under {}: an untracked or ignored directory, or the \
         wrong path (W45-TEST-HYGIENE)",
        directory.display()
    );
    files
}
