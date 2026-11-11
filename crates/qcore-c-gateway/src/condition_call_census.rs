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

//! Card **W44-BRANCH-CONDITION-CALLS**: the instrumented condition-call census.
//!
//! A measurement, not a rule, built like card W43-INDEX-TRAVERSAL's traversal
//! stream: this module and its one call site are behind the off-by-default
//! `indirect-census` feature, every record is written **after** the pre-existing
//! routes refused the controlling expression, and nothing reads the stream but
//! the card's scripts. With the feature on, the pinned corpus audit must still
//! reproduce its artifacts byte for byte.
//!
//! One JSON line per refused call-bearing controlling expression, to the path
//! named by [`CONDITION_CALL_CENSUS_PATH`], carrying the desugaring rule's own
//! answer (`admitted`, or its refusal code) and a lexical inventory of the
//! expression's callees (contract in force, local definition, result mode).
//! The analyzer parses a function several times (derivation rounds and the
//! final analysis); the scripts keep the last record per key, which is the
//! final analysis's.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;

/// The environment variable naming the condition-call census stream.
pub const CONDITION_CALL_CENSUS_PATH: &str = "QCORE_CONDITION_CALL_CENSUS";

static SINK: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

/// One callee of a recorded expression.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Callee {
    pub name: String,
    /// A contract for this name is in force in the environment of this parse.
    pub contracted: bool,
    /// This translation unit defines the name.
    pub local: bool,
    /// The in-force contract's result mode.
    pub result: Option<&'static str>,
}

/// One refused call-bearing controlling expression and the rule's answer.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Record {
    pub unit_seq: u64,
    pub function: String,
    pub line: usize,
    pub position: &'static str,
    pub texts: Vec<String>,
    /// `admitted` or the refusal code.
    pub outcome: &'static str,
    /// The callee or name the refusal is about, where it names one.
    pub subject: Option<String>,
    /// The classifier's own refusal for a refused call.
    pub reason: Option<String>,
    /// The number of plan nodes, for an admitted expression.
    pub nodes: usize,
    pub callees: Vec<Callee>,
}

/// Whether the stream is being collected.
pub fn collecting() -> bool {
    std::env::var_os(CONDITION_CALL_CENSUS_PATH).is_some()
}

/// Append one record.
pub fn record(record: Record) {
    let Ok(mut guard) = SINK.lock() else {
        return;
    };
    let opened = guard.get_or_insert_with(|| {
        let path = std::env::var_os(CONDITION_CALL_CENSUS_PATH)?;
        File::create(path).ok().map(BufWriter::new)
    });
    let Some(writer) = opened.as_mut() else {
        return;
    };
    if let Ok(line) = serde_json::to_string(&record) {
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}
