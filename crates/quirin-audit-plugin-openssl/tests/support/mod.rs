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

//! Shared test support: Clang and scratch directories.

#![allow(dead_code)]

use std::path::PathBuf;

/// The Clang the tests run: `QCORE_CLANG` (an empty value is an error, never
/// a silent fallback), else `clang` on `PATH`. A Clang that does not run fails
/// the test: these tests need it, as every existing-C analysis test does.
pub fn clang() -> String {
    let clang = match std::env::var("QCORE_CLANG") {
        Ok(value) if value.is_empty() => panic!("QCORE_CLANG is set and empty"),
        Ok(value) => value,
        Err(_) => "clang".to_owned(),
    };
    let status = std::process::Command::new(&clang)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .status();
    assert!(
        status.is_ok_and(|status| status.success()),
        "Clang `{clang}` does not run; set QCORE_CLANG"
    );
    clang
}

/// A fresh directory for one test.
pub fn scratch(name: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("openssl-plugin")
        .join(format!(
            "{}-{name}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    directory.canonicalize().unwrap()
}
