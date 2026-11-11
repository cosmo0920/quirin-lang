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

//! Scratch generator for the provider-side projection fixtures. Not part of
//! the gate; run explicitly with QCORE_EMIT_CLEANUP_FIXTURES=1.
#![cfg(not(target_os = "windows"))]

#[path = "fixtures/cleanup_replay_source.rs"]
mod fixture;
use fixture::*;

#[test]
fn emit() {
    if std::env::var_os("QCORE_EMIT_CLEANUP_FIXTURES").is_none() || !clang_available() {
        return;
    }
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../providers/quirin-provider-shared/fixtures");
    for (source, function, records, name) in [
        (
            OUTCOMES_SOURCE,
            "cio_file_native_close",
            &OUTCOMES_RECORDS[..],
            "native_close_cleanup_replay_projection.json",
        ),
        (
            OUTCOMES_SOURCE,
            "cio_file_native_unmap",
            &OUTCOMES_RECORDS[..],
            "native_unmap_cleanup_replay_projection.json",
        ),
        (
            DELETE_SOURCE,
            "cio_file_native_delete",
            &DELETE_RECORDS[..],
            "native_delete_cleanup_replay_projection.json",
        ),
    ] {
        let (import, _) = route(source, function, records, "cio_file_unix.c");
        std::fs::write(
            root.join(name),
            serde_json::to_string_pretty(&import.source_input()).unwrap() + "\n",
        )
        .unwrap();
    }
}
