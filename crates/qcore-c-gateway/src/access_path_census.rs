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

//! Card **W46-ACCESS-PATH-CENSUS**: the typed access-path census's unit stream.
//!
//! A measurement, not a rule, built like the other `indirect-census` streams:
//! this module and its two call sites in [`crate::legacy_c`] are behind the
//! off-by-default `indirect-census` feature, it is written after preprocessing
//! has already been decided and before any analysis reads the unit, and
//! nothing reads what it writes but the card's scripts. With the feature on,
//! the pinned corpus audit must still reproduce its artifacts byte for byte.
//!
//! For every translation-unit analysis it writes, into the directory named by
//! [`ACCESS_PATH_CENSUS_PATH`], the exact preprocessed text the analysis reads
//! (`unit-<seq>.c`, whose line numbers are the leaves' internal `line`
//! identities, constraint 12a) and one JSON line in `units.jsonl` with the unit
//! ordinal (the grouping key every census stream shares), the dialect and the
//! frontend flags. The card's scripts give that text to Clang with exactly the
//! argument vector [`crate::typed_c_ast::translation_unit_ast`] builds, so the
//! typed AST they classify is the gateway's own typed-AST transport of the
//! analysed text, not a second preprocessing of the original files.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// The environment variable naming the census directory.
pub const ACCESS_PATH_CENSUS_PATH: &str = "QCORE_ACCESS_PATH_CENSUS";

static SINK: Mutex<()> = Mutex::new(());

#[derive(serde::Serialize)]
struct UnitRecord<'a> {
    unit_seq: u64,
    file: String,
    dialect: &'a str,
    frontend_flags: &'a [String],
    bytes: usize,
    lines: usize,
}

/// Record one unit's analysed text. A no-op unless [`ACCESS_PATH_CENSUS_PATH`]
/// names a directory; a census that cannot write is silent rather than fatal.
pub(crate) fn record_unit(preprocessed_source: &str, dialect: &str, frontend_flags: &[String]) {
    let Some(directory) = std::env::var_os(ACCESS_PATH_CENSUS_PATH) else {
        return;
    };
    let Ok(_guard) = SINK.lock() else {
        return;
    };
    let directory = PathBuf::from(directory);
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    let unit_seq = crate::indirect_census::unit_seq();
    let file = format!("unit-{unit_seq}.c");
    let Ok(mut text) = File::create(directory.join(&file)) else {
        return;
    };
    if text.write_all(preprocessed_source.as_bytes()).is_err() {
        return;
    }
    let record = UnitRecord {
        unit_seq,
        file,
        dialect,
        frontend_flags,
        bytes: preprocessed_source.len(),
        lines: preprocessed_source.lines().count(),
    };
    let Ok(mut index) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("units.jsonl"))
    else {
        return;
    };
    if let Ok(line) = serde_json::to_string(&record) {
        let _ = writeln!(index, "{line}");
    }
}
