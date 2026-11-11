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

//! Card REASON-CODES-B2: `ChunkSourceResult::Unknown` carries a typed category.
//!
//! §L21.1 named this enum as a reason site and the REASON-CODES card left it as
//! free text only, so every consumer had to re-derive the category from the
//! prose — which is how a second, uncarried vocabulary starts. The row now
//! carries the classification the importer made, and these tests pin the two
//! fail-closed directions the vocabulary requires.
//!
//! No Clang invocation is needed: the constructor is a pure function of the
//! reason text, which is exactly why it can never disagree with the text.

use qcore_c_gateway::boolean_source::ChunkSourceResult;
use qcore_schemas::reason::{ReasonCategory, ReasonSubject};

fn code(reason: &str) -> Option<ReasonCategory> {
    ChunkSourceResult::unknown("f", reason).reason_code()
}

#[test]
fn a_recognised_importer_reason_carries_its_closed_category() {
    // The three highest-frequency reasons in the archived 185-unit sidecars.
    assert_eq!(
        code("outside single borrowed chunk / int-or-void-result source profile"),
        Some(ReasonCategory::UnsupportedSyntax)
    );
    assert_eq!(
        code("unmodeled statement DeclStmt"),
        Some(ReasonCategory::UnsupportedSyntax)
    );
    assert_eq!(
        code("expected one selected definition after preprocessing, found 0"),
        Some(ReasonCategory::IncompleteObligationCoverage)
    );
}

#[test]
fn the_row_keeps_its_exact_function_and_reason_bytes() {
    let reason = "unmodeled expression CallExpr";
    let ChunkSourceResult::Unknown {
        function,
        reason: carried,
        reason_code,
    } = ChunkSourceResult::unknown("cio_chunk_open", reason)
    else {
        panic!("the constructor must build an Unknown row");
    };
    assert_eq!(function, "cio_chunk_open");
    assert_eq!(carried, reason);
    assert_eq!(reason_code, Some(ReasonCategory::UnsupportedSyntax));
}

#[test]
fn an_unrecognised_reason_asserts_no_category_rather_than_a_source_defect() {
    // Fail-closed: no fallback category, and in particular no fallback onto a
    // subject-AnalysedSource category. `None` asserts nothing.
    assert_eq!(code("No such file or directory (os error 2)"), None);
    assert_eq!(code(""), None);
    assert_eq!(code("something nobody has ever emitted"), None);
}

#[test]
fn an_environment_reason_is_never_typed_as_a_statement_about_the_source() {
    // §L21.1 rule 1 across this carrier. `Clang identity changed during
    // Boolean source import` is minted by this very module.
    for reason in [
        "Clang identity changed during Boolean source import",
        "Clang stdin unavailable",
    ] {
        let category = code(reason).expect("an environment reason classifies");
        assert_eq!(category, ReasonCategory::EnvironmentUnavailable);
        assert_eq!(category.subject(), ReasonSubject::Environment);
        assert!(!category.describes_analysed_source());
    }
}

#[test]
fn a_stale_identity_reason_is_not_read_as_a_source_defect_either() {
    let category = code("ChunkIO source revision does not match the pinned revision")
        .expect("a pinned-revision mismatch classifies");
    assert_eq!(category, ReasonCategory::StaleEvidence);
    assert!(!category.describes_analysed_source());
}
