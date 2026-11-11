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

//! Card `W48-QR-USABLE-RECORDS` (card 2 of the `.qr` language track), Step 0:
//! the record survey.
//!
//! One fixture per (route, operation) cell under
//! `compiler/tests/fixtures/usable_records/survey/`, written by
//! `docs/reports/w48-qr-usable-records/make_survey.py` before any source change
//! of the card. Each is checked through the library API with what its route
//! needs — the extern cells with `survey_scale.qv` selected, the FFI cells with
//! the recorded target of the Clang that runs — and its outcome is pinned:
//! admitted, or refused with the first diagnostic's code. The table below is
//! the survey at the card's tip; the report publishes it beside the one
//! measured at the base.
//!
//! `QCORE_W48_RECORDS_SURVEY=<absolute path>` writes the measured table as
//! JSON (the report's `artifacts/survey-*.json`).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget};
use qcore_compiler::{CompileOptions, SourceFile, compile_files_for_target};

#[path = "../../tests/support/tracked_files.rs"]
mod tracked_files;

fn survey_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/usable_records/survey")
}

fn host_target() -> RecordedTarget {
    let (compiler, triple) = qcore_c_gateway::ffi_layout::running_compiler()
        .expect("Clang is required for the FFI survey cells");
    RecordedTarget {
        pointer_width: if cfg!(target_pointer_width = "64") {
            PointerWidth::W64
        } else {
            PointerWidth::W32
        },
        triple,
        compiler,
    }
}

const ROUTES: [&str; 5] = ["buffer", "scalar", "extern", "task", "ffi"];

/// The pinned outcome of every cell: `None` admitted, `Some(code)` refused
/// with that code first.
const EXPECTED: &[(&str, Option<&str>)] = &[
    ("buffer_construct.qr", None),
    ("buffer_copy.qr", None),
    ("buffer_lend.qr", None),
    ("buffer_lend_owned.qr", Some("Q051")),
    ("buffer_loop.qr", None),
    ("buffer_loop_owned.qr", Some("Q051")),
    ("buffer_move.qr", Some("Q054")),
    ("buffer_nested.qr", None),
    ("buffer_param.qr", None),
    ("buffer_read.qr", None),
    ("buffer_result.qr", None),
    ("buffer_view_field.qr", Some("Q051")),
    ("buffer_write.qr", None),
    ("buffer_write_owned.qr", None),
    ("buffer_write_released.qr", Some("Q054")),
    ("scalar_construct.qr", None),
    ("scalar_copy.qr", None),
    ("scalar_lend.qr", Some("Q051")),
    ("scalar_loop.qr", Some("Q020")),
    ("scalar_move.qr", Some("Q020")),
    ("scalar_nested.qr", None),
    ("scalar_param.qr", None),
    ("scalar_read.qr", None),
    ("scalar_result.qr", None),
    ("scalar_write.qr", Some("Q020")),
    ("extern_construct.qr", Some("Q025")),
    ("extern_copy.qr", Some("Q025")),
    ("extern_lend.qr", Some("Q025")),
    ("extern_loop.qr", Some("Q026")),
    ("extern_move.qr", None),
    ("extern_nested.qr", Some("Q025")),
    ("extern_param.qr", Some("Q026")),
    ("extern_read.qr", Some("Q025")),
    ("extern_result.qr", Some("Q026")),
    ("extern_write.qr", Some("Q025")),
    // Card `W48-QR-TASK-RECORD-CAPTURES` moved the task row off the record
    // declaration's `Q500` (`AGGREGATE-TASK-CAPTURE`): a record of scalars is
    // constructed, read, copied, nested and passed; `result` now refuses at the
    // rendering of a whole record (`Q050`), `write` for its missing `mut`
    // (`Q300`), `loop` at the task route's `for` (`Q500`) and `move` at its
    // `bytes` field (`Q500`, `WRITE-LEND-TASKS-EXTENT`).
    ("task_construct.qr", None),
    ("task_copy.qr", None),
    ("task_lend.qr", None),
    ("task_loop.qr", Some("Q500")),
    ("task_move.qr", Some("Q500")),
    ("task_nested.qr", None),
    ("task_param.qr", None),
    ("task_read.qr", None),
    ("task_result.qr", Some("Q050")),
    ("task_write.qr", Some("Q300")),
    ("ffi_construct.qr", None),
    ("ffi_copy.qr", None),
    ("ffi_lend.qr", Some("Q026")),
    ("ffi_loop.qr", Some("Q026")),
    ("ffi_move.qr", Some("Q051")),
    ("ffi_nested.qr", Some("Q051")),
    ("ffi_param.qr", None),
    ("ffi_read.qr", None),
    ("ffi_result.qr", None),
    ("ffi_write.qr", None),
];

/// Check one cell; the outcome and the first diagnostic's message.
fn measure(route: &str, file: &str) -> (Option<String>, String) {
    let path = survey_dir().join(file);
    let source = std::fs::read_to_string(&path).unwrap();
    let contracts = (route == "extern").then(|| {
        BTreeMap::from([(
            "compiler/tests/fixtures/usable_records/survey/survey_scale.qv".to_owned(),
            std::fs::read_to_string(survey_dir().join("survey_scale.qv")).unwrap(),
        )])
    });
    let target = (route == "ffi").then(host_target);
    match compile_files_for_target(
        &[SourceFile::new(path.display().to_string(), source)],
        &CompileOptions {
            extern_contracts: contracts,
            ..CompileOptions::default()
        },
        target.as_ref(),
    ) {
        Ok(module) => (None, format!("admitted: {:?}", module.execution_profile)),
        Err(errors) => (
            Some(errors[0].code.clone()),
            errors[0].message.lines().next().unwrap_or("").to_owned(),
        ),
    }
}

fn cells() -> Vec<(String, String, String)> {
    let mut cells = Vec::new();
    // The tracked files only (constraint 29): an untracked or ignored file
    // left in the directory is not a cell.
    let names: Vec<String> = tracked_files::tracked_files(&survey_dir())
        .into_iter()
        .map(|path| {
            let relative = path.strip_prefix(survey_dir()).unwrap();
            relative.to_str().unwrap().to_owned()
        })
        .collect();
    let (names, support): (Vec<String>, Vec<String>) =
        names.into_iter().partition(|name| name.ends_with(".qr"));
    assert_eq!(
        support,
        ["survey_rec.h", "survey_scale.qv"],
        "the survey directory holds its cells and two support files only"
    );
    for name in names {
        let stem = name.trim_end_matches(".qr");
        let (route, op) = stem.split_once('_').unwrap();
        assert!(ROUTES.contains(&route), "{name}");
        cells.push((route.to_owned(), op.to_owned(), name.clone()));
    }
    cells
}

#[test]
fn every_survey_cell_has_its_pinned_outcome() {
    // The tracked files of this test's own fixture directory, each a cell or
    // one of the two support files: an extra tracked file fails the
    // inventory (constraint 29).
    let mut rows = Vec::new();
    let mut mismatches = Vec::new();
    let expected: BTreeMap<&str, Option<&str>> = EXPECTED.iter().copied().collect();
    for (route, op, file) in cells() {
        let (code, message) = measure(&route, &file);
        let pinned = expected.get(file.as_str()).copied();
        if pinned != Some(code.as_deref()) {
            mismatches.push(format!("(\"{file}\", {code:?}), // {message}"));
        }
        rows.push(format!(
            "  {{\"route\": \"{route}\", \"op\": \"{op}\", \"file\": \"{file}\", \"admitted\": {}, \"code\": {}, \"message\": {:?}}}",
            code.is_none(),
            code.as_ref()
                .map_or_else(|| "null".to_owned(), |code| format!("\"{code}\"")),
            message
        ));
    }
    if let Ok(out) = std::env::var("QCORE_W48_RECORDS_SURVEY") {
        std::fs::write(out, format!("[\n{}\n]\n", rows.join(",\n"))).unwrap();
    }
    assert_eq!(expected.len(), rows.len(), "every pinned cell is a fixture");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
