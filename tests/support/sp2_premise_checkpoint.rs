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

//! Card **SP2-SYMBOLIC-LIBRDKAFKA-PREMISES**: the converted librdkafka
//! modules' bytes at the card's base, recovered from their current bytes.
//!
//! The card rewrote each premise's string statement as its formula
//! (C04.18.9) and refreshed each binding marker whose manifest pins the
//! declaration's graph; nothing else in those modules changed. A frozen
//! checkpoint test that compares a converted module with a historical copy
//! compares the module as it stood before the card instead: [`restore`]
//! writes every formula back as its prose and every refreshed marker back as
//! its previous digest, and [`restored_file`] also checks both byte digests
//! the card's record (`docs/reports/sp2-symbolic-librdkafka-premises/migration.json`)
//! states, so an unrecorded change to a converted module still fails.

#![allow(dead_code)]

const RECORD: &str =
    include_str!("../../docs/reports/sp2-symbolic-librdkafka-premises/migration.json");

fn record() -> serde_json::Value {
    serde_json::from_str(RECORD).expect("the SP2 migration record is JSON")
}

/// `text` with every SP2 formula statement written back as its prose and
/// every refreshed binding marker written back as its previous digest.
pub fn restore(text: &str) -> String {
    let record = record();
    let mut out = text.to_owned();
    for statement in record["statements"].as_array().unwrap() {
        let name = statement["name"].as_str().unwrap();
        let formula = format!(
            "resource_premise({name}, {})",
            statement["formula"].as_str().unwrap()
        );
        let prose = format!(
            "resource_premise({name}, \"{}\")",
            statement["prose"].as_str().unwrap()
        );
        out = out.replace(&formula, &prose);
    }
    for marker in record["markers"].as_array().unwrap() {
        out = out.replace(
            marker["manifest_sha256"].as_str().unwrap(),
            marker["previous_manifest_sha256"].as_str().unwrap(),
        );
    }
    out
}

/// The bytes of the converted module at `path` (repository-relative) before
/// the card: `current` must be the module's recorded current bytes and the
/// restoration its recorded base bytes. A module the card did not convert is
/// returned unchanged.
pub fn restored_file(path: &str, current: &str, sha256: fn(&[u8]) -> String) -> String {
    let record = record();
    let Some(file) = record["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| path.ends_with(file["file"].as_str().unwrap()))
    else {
        return current.to_owned();
    };
    assert_eq!(
        sha256(current.as_bytes()),
        file["after_sha256"].as_str().unwrap(),
        "unregistered change to {path} after SP2-SYMBOLIC-LIBRDKAFKA-PREMISES"
    );
    let restored = restore(current);
    assert_eq!(
        sha256(restored.as_bytes()),
        file["before_sha256"].as_str().unwrap(),
        "{path}: the SP2 restoration is not the module's base bytes"
    );
    restored
}
