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

//! The retained cJSON frames are Linux (LP64) preprocessor output, so they
//! spell `size_t` as `long unsigned int`. Clang's MSVC-compatible Windows
//! target (LLP64) predeclares `size_t` as `unsigned long long` and rejects
//! that redefinition, so on Windows the one typedef is respelled as that
//! target's preprocessor prints it (the precedent is the library's
//! `checked_indexed_child_lookup_keeps_borrowed_result_summary`). Everywhere
//! else every frame is used byte for byte.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The typedef the retained frames carry.
pub const LP64_SIZE_T: &str = "typedef long unsigned int size_t;";
const LLP64_SIZE_T: &str = "typedef unsigned long long size_t;";

/// The `size_t` typedef the target's preprocessor prints.
pub fn size_t_typedef() -> &'static str {
    if cfg!(windows) {
        LLP64_SIZE_T
    } else {
        LP64_SIZE_T
    }
}

/// The same typedef made signed, at the target's width.
pub fn signed_size_t_typedef() -> &'static str {
    if cfg!(windows) {
        "typedef long long size_t;"
    } else {
        "typedef long int size_t;"
    }
}

/// `frame` as the target's preprocessor prints it: unchanged except for its
/// single `size_t` typedef, which must be present.
pub fn for_target(frame: &str) -> String {
    assert_eq!(
        frame.matches(LP64_SIZE_T).count(),
        1,
        "a retained frame carries exactly one LP64 `size_t` typedef"
    );
    frame.replace(LP64_SIZE_T, size_t_typedef())
}

/// The include directory for `names` in `fixtures`: the fixture directory
/// itself where the target is LP64, otherwise a fresh directory holding each
/// named file, `for_target` where it carries the typedef and verbatim where it
/// does not.
pub fn include_root(fixtures: &Path, names: &[&str]) -> PathBuf {
    if !cfg!(windows) {
        return fixtures.to_path_buf();
    }
    // Several tests build include roots at once, and Windows' clock can hand
    // two of them the same nanosecond reading; the counter keeps each
    // directory its own.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "qcore-retained-frame-{}-{}-{nonce}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).unwrap();
    for name in names {
        let source = std::fs::read_to_string(fixtures.join(name)).unwrap();
        let source = if source.contains(LP64_SIZE_T) {
            for_target(&source)
        } else {
            source
        };
        std::fs::write(directory.join(name), source).unwrap();
    }
    directory
}
