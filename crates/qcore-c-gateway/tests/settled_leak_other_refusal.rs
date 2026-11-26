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

//! Card SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE (gateway rule 84, amended): a
//! leak whose settled exit marks the identity by any refusal, not only a
//! refused release, is decided from the settled states alone.
//!
//! Each fixture comes in two schedules of the same shape. Either the arm that
//! avoids the refusal reaches the exit first (an iterate sees only that arm
//! and `report_leaks` claims the site), or the refusing arm does (its marker
//! joins first and no iterate claims the site). At `98c51878f` the first
//! order was `Unsafe` and the second `Unknown`: rule 6's hazard. Both orders
//! now decide the same. The fixtures run the real Clang-backed gateway over
//! the generic `k_*` surface (`fixtures/contracts/descriptor_generic.qv`) and
//! uncontracted `u_*` calls. Each `Unsafe` is provisional, over synthetic
//! trusted test assumptions.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
int k_open(const char *path); int k_close(int fd); \
char *k_fopen(const char *path); int k_fclose(char *stream); \
void u_keep(char *stream); int u_read(void); int u_ok(char *stream); \
void u_log(const char *format, ...);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "descriptor_generic.qcore".to_owned(),
        include_str!("fixtures/contracts/descriptor_generic.qv").to_owned(),
    )]))
    .expect("the fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
}

impl Run {
    fn leaks(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .count()
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("these fixtures project partially")
        }
    }
}

/// `body` is `int w(const char *path, int flag, int n, char **out)`'s body.
fn analyze(body: &str) -> Run {
    let source = format!(
        "{DECLARATIONS}\nint w(const char *path, int flag, int n, char **out) {{\n{body}\n}}\n"
    );
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let result = &summary_of(&outcome).functions["w"];
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    }
}

impl Run {
    fn refused(&self, description: &str) -> bool {
        self.diagnostics.iter().any(|kind| match kind {
            LegacyDiagnosticKind::Unsupported { description: d } => d == description,
            _ => false,
        })
    }
}

const HAND_ON: &str = "call to uncontracted function `u_keep`";
const ESCAPE: &str = "store through `*out` without an out-parameter contract";
const TEST: &str = "call to uncontracted function `u_ok` in branch condition";

/// `(kind, avoiding arm first, refusing arm first, the refusal)`: the
/// refusing arm marks the stream and the other arm leaves it open.
const SCHEDULES: [(&str, &str, &str, &str); 3] = [
    // A refused hand-on (rule W44).
    (
        "hand-on",
        "char *s = k_fopen(path); if (flag) { n = n + 1; u_keep(s); } return 0;",
        "char *s = k_fopen(path); if (flag) { u_keep(s); } else { n = n + 1; n = n + 2; } return 0;",
        HAND_ON,
    ),
    // A refused escape: a store through an uncontracted out-parameter.
    (
        "escape",
        "char *s = k_fopen(path); if (flag) { n = n + 1; *out = s; } return 0;",
        "char *s = k_fopen(path); if (flag) { *out = s; } else { n = n + 1; n = n + 2; } return 0;",
        ESCAPE,
    ),
    // A refused test: an uncontracted call in a branch condition.
    (
        "test",
        "char *s = k_fopen(path); if (flag) { n = n + 1; if (u_ok(s)) { n = 2; } } return 0;",
        "char *s = k_fopen(path); if (flag) { if (u_ok(s)) { n = 2; } } else { n = n + 1; n = n + 2; } return 0;",
        TEST,
    ),
];

/// The probe at `98c51878f`: the verdict reads the schedule. With the
/// avoiding arm first an iterate claims the leak and it is published; with
/// the refusing arm first no iterate claims it.
#[test]
fn the_verdict_reads_the_schedule_at_base() {
    if !clang_available() {
        return;
    }
    for (kind, first, second, refusal) in SCHEDULES {
        let run = analyze(first);
        assert_eq!(run.status, AnalysisStatus::Unsafe, "{kind}, first: {run:?}");
        assert_eq!(run.leaks(), 1, "{kind}, first: {run:?}");
        assert!(run.refused(refusal), "{kind}, first: {run:?}");
        let run = analyze(second);
        assert_eq!(
            run.status,
            AnalysisStatus::Unknown,
            "{kind}, second: {run:?}"
        );
        assert_eq!(run.leaks(), 0, "{kind}, second: {run:?}");
        assert!(run.refused(refusal), "{kind}, second: {run:?}");
    }
}

/// The refusal on every path: nothing is witnessed in either schedule, the
/// function stays `Unknown` with no leak and the refusal is kept.
#[test]
fn a_refusal_on_every_path_is_withdrawn_in_both_schedules() {
    if !clang_available() {
        return;
    }
    for (kind, first, second, refusal) in [
        (
            "hand-on",
            "char *s = k_fopen(path); if (flag) { n = n + 1; u_keep(s); } else { u_keep(s); } return 0;",
            "char *s = k_fopen(path); if (flag) { u_keep(s); } else { n = n + 1; n = n + 2; u_keep(s); } return 0;",
            HAND_ON,
        ),
        (
            "escape",
            "char *s = k_fopen(path); if (flag) { n = n + 1; *out = s; } else { *out = s; } return 0;",
            "char *s = k_fopen(path); if (flag) { *out = s; } else { n = n + 1; n = n + 2; *out = s; } return 0;",
            ESCAPE,
        ),
        (
            "test",
            "char *s = k_fopen(path); if (u_ok(s)) { n = 2; } return 0;",
            "char *s = k_fopen(path); n = n + 1; if (u_ok(s)) { n = 2; } else { n = 3; } return 0;",
            TEST,
        ),
    ] {
        for (order, body) in [("first", first), ("second", second)] {
            let run = analyze(body);
            assert_eq!(
                run.status,
                AnalysisStatus::Unknown,
                "{kind}, {order}: {run:?}"
            );
            assert_eq!(run.leaks(), 0, "{kind}, {order}: {run:?}");
            assert!(run.refused(refusal), "{kind}, {order}: {run:?}");
        }
    }
}
