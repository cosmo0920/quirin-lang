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

//! Card **SCALAR-STATEMENT-FORMS** (round 26 E), measurement only: the
//! statements the classifier refused with one of the five scalar-statement
//! refusal families, as the classifier itself saw them.
//!
//! Behind the off-by-default cargo feature `scalar-form-census`, which no
//! workspace member enables: with it off this module and its call sites are
//! compiled out. It moves no verdict; it is written after the decision.
//!
//! Each record is one JSON line to `QCORE_SCALAR_FORM_CENSUS`: the function,
//! the statement's internal line, its token texts, the refusal, and the stage
//! (`classified` from [`crate::legacy_c`]'s `classify_simple`, or `fallback`
//! with whether a statement fallback answered the same statement).

use std::io::Write;
use std::sync::Mutex;

/// The stream's environment variable.
pub const SCALAR_FORM_CENSUS_PATH: &str = "QCORE_SCALAR_FORM_CENSUS";

/// The five refusal families the card censuses.
pub(crate) const FAMILIES: [&str; 5] = [
    "unsupported assignment expression",
    "unrecognized assignment destination",
    "braced initializer, expression, or declaration is not modeled",
    "unmodeled increment or decrement expression",
    "C operator `sizeof` requires type/operand analysis",
];

static STREAM: Mutex<Option<std::fs::File>> = Mutex::new(None);

pub(crate) fn record(
    function: &str,
    line: usize,
    texts: &[&str],
    description: &str,
    stage: &str,
    answered: Option<bool>,
) {
    if !FAMILIES.contains(&description) {
        return;
    }
    let Some(path) = std::env::var_os(SCALAR_FORM_CENSUS_PATH) else {
        return;
    };
    let mut stream = STREAM.lock().unwrap_or_else(|error| error.into_inner());
    if stream.is_none() {
        *stream = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok();
    }
    if let Some(file) = stream.as_mut() {
        let row = serde_json::json!({
            "function": function,
            "line": line,
            "texts": texts,
            "description": description,
            "stage": stage,
            "answered": answered,
        });
        let _ = writeln!(file, "{row}");
    }
}
